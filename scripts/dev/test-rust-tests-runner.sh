#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract test for the one-physical-host `tank` runner pool. Multiple runner
# registrations must not turn into concurrent heavyweight workspace suites,
# and bursts must not displace an older suite while it is waiting for the host.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
WORKFLOW=$REPO_ROOT/.github/workflows/ci.yml

rust_tests_job=$(awk '
    /^  rust-tests:/ { in_job = 1 }
    in_job && /^  [A-Za-z0-9_-]+:/ && $1 != "rust-tests:" { exit }
    in_job { print }
' "$WORKFLOW")

if [ -z "$rust_tests_job" ]; then
    echo "ci.yml must define the rust-tests job" >&2
    exit 1
fi

# Parse the workflow without PyYAML or another unprovisioned dependency.  This
# is a strict parser for the YAML subset used by the contract: mappings,
# sequences, scalar values, and literal block scalars.  It records paths only
# after walking indentation and list structure, so assertions cannot be
# satisfied by a comment or an unrelated job/step with the same text.
awk '
function fail(message) {
    print "ci.yml contract: " message > "/dev/stderr"
    failed = 1
    exit 1
}
function trim(value) {
    sub(/^[ \t]+/, "", value)
    sub(/[ \t]+$/, "", value)
    return value
}
function scalar(value,    i, c, quote, out) {
    value = trim(value)
    quote = ""
    out = ""
    for (i = 1; i <= length(value); i++) {
        c = substr(value, i, 1)
        if ((c == "\"" || c == "\047") && (i == 1 || substr(value, i - 1, 1) != "\\")) {
            if (quote == "") quote = c
            else if (quote == c) quote = ""
        }
        if (c == "#" && quote == "" && (i == 1 || substr(value, i - 1, 1) ~ /[ \t]/)) break
        out = out c
    }
    out = trim(out)
    if (length(out) >= 2 && ((substr(out, 1, 1) == "\"" && substr(out, length(out), 1) == "\"") ||
                             (substr(out, 1, 1) == "\047" && substr(out, length(out), 1) == "\047")))
        out = substr(out, 2, length(out) - 2)
    return out
}
function put(path, value) {
    if (values_seen[path]) fail("duplicate mapping key " path)
    values_seen[path] = 1
    seen[path] = 1
    values[path] = value
}
function flush_block(    value) {
    if (!block_active) return
    sub(/\n$/, "", block_value)
    put(block_path, block_value)
    block_active = 0
    block_path = ""
    block_value = ""
}
function step(job, field, wanted,    parent, i, path) {
    parent = job ".steps"
    for (i = 0; i < seq_count[parent]; i++) {
        path = parent "." i "." field
        if (values[path] == wanted) return i
    }
    return -1
}
function need(condition, message) {
    if (!condition) fail(message)
}
function contains(path, text, message) {
    need(seen[path] && index(values[path], text) != 0, message)
}
{
    line = $0
    if (block_active) {
        line_indent = 0
        while (substr(line, line_indent + 1, 1) == " ") line_indent++
        if (line ~ /^[ \t]*$/ || line_indent > block_indent) {
            block_value = block_value line "\n"
            next
        }
        flush_block()
    }
    if (line ~ /^[ \t]*$/ || line ~ /^[ \t]*#/) next
    if (line ~ /\t/) fail("tabs are not valid indentation (line " NR ")")
    indent = 0
    while (substr(line, indent + 1, 1) == " ") indent++
    while (stack_count > 0 && stack_indent[stack_count] >= indent) stack_count--
    content = substr(line, indent + 1)
    # YAML permits a flow sequence to continue on the line after its key
    # (`needs:` in this workflow). It is a scalar for our purposes; consume it
    # as the value of the pending mapping key rather than mistaking it for a
    # malformed top-level line.
    if (content ~ /^\[/ && stack_count > 0) {
        path = stack_path[stack_count]
        need(!values_seen[path], "duplicate mapping key " path)
        values_seen[path] = 1
        seen[path] = 1
        values[path] = scalar(content)
        stack_count--
        next
    }
    if (content ~ /^-[ \t]*/) {
        parent = (stack_count > 0 ? stack_path[stack_count] : "")
        need(parent != "", "sequence without a mapping parent (line " NR ")")
        sub(/^-/, "", content)
        content = trim(content)
        item = parent "." seq_count[parent]++
        stack_count++
        stack_indent[stack_count] = indent
        stack_path[stack_count] = item
        if (content == "") next
        colon = index(content, ":")
        if (colon == 0) {
            put(item, scalar(content))
            next
        }
        key = trim(substr(content, 1, colon - 1))
        value = trim(substr(content, colon + 1))
        path = item "." key
    } else {
        colon = index(content, ":")
        need(colon != 0, "expected a mapping or sequence item (line " NR ")")
        parent = (stack_count > 0 ? stack_path[stack_count] : "")
        key = trim(substr(content, 1, colon - 1))
        need(key != "", "empty mapping key (line " NR ")")
        value = trim(substr(content, colon + 1))
        path = (parent == "" ? key : parent "." key)
    }
    need(!nodes[path], "duplicate mapping key " path)
    nodes[path] = 1
    if (value == "") {
        stack_count++
        stack_indent[stack_count] = indent
        stack_path[stack_count] = path
    } else if (value == "|") {
        block_active = 1
        block_indent = indent
        block_path = path
        block_value = ""
    } else if (value ~ /^[>|][+-]?$/) {
        # Preserve folded scalars line-for-line. The contract only searches
        # literal shell blocks, but accepting both YAML block styles keeps an
        # unrelated workflow field from making this structural parser brittle.
        block_active = 1
        block_indent = indent
        block_path = path
        block_value = ""
    } else {
        put(path, scalar(value))
    }
}
END {
    if (failed) exit 1
    flush_block()
    need(values_seen["jobs.rust-tests.runs-on"], "ci.yml must define jobs.rust-tests as a mapping")
    tank_jobs = 0
    for (path in values) {
        if (path ~ /^jobs\.[^.]+\.runs-on$/ && index(values[path], "tank") != 0) {
            split(path, parts, ".")
            tank_jobs++
            tank_job = parts[2]
        }
    }
    need(tank_jobs == 1 && tank_job == "rust-tests", "rust-tests must remain the only job that can reach Tank")
    need(nodes["jobs.channel"], "ci.yml must define jobs.channel as a mapping")
    need(values["jobs.channel.outputs.runner_policy_changed"] == "${{ steps.paths.outputs.runner_policy_changed }}",
         "channel must publish runner_policy_changed from the paths step")
    need(values["jobs.channel.outputs.rust_tests_runner"] == "${{ steps.rust-runner.outputs.runner }}",
         "channel must publish the broad Rust runner decision")

    paths_step = step("jobs.channel", "id", "paths")
    need(paths_step >= 0, "channel must define the paths step")
    paths_run = "jobs.channel.steps." paths_step ".run"
    path_list = ".github/workflows/ci.yml | .github/dependabot.yml | scripts/dev/changed-paths.sh | scripts/dev/rust-tests-path.sh | scripts/dev/rust-tests-input-paths.txt | scripts/dev/rust-tests-package-paths.txt | scripts/dev/select-rust-tests-runner.sh | scripts/dev/persistent-cargo-target.sh | scripts/dev/test-rust-tests-paths.sh | scripts/dev/test-rust-tests-runner.sh | scripts/dev/test-persistent-cargo-target.sh)"
    contains(paths_run, path_list, "runner and dependency-policy changes must classify themselves for hosted proof")

    runner_step = step("jobs.channel", "id", "rust-runner")
    need(runner_step >= 0, "channel must define the rust-runner step")
    runner_run = "jobs.channel.steps." runner_step ".run"
    contains(runner_run, "\"$PR_HEAD_REPOSITORY\" != \"$GITHUB_REPOSITORY\"", "fork pull requests must force hosted Rust tests")
    contains(runner_run, "\"$PR_AUTHOR\" == '\''dependabot[bot]'\''", "Dependabot pull requests must force hosted Rust tests")
    contains(runner_run, "\"$RUNNER_POLICY_CHANGED\" == true", "runner-policy pull requests must force hosted Rust tests")
    contains(runner_run, "elif [[ \"$EVENT_NAME\" == workflow_dispatch ]]", "manual dispatch must remain an explicit runner override")
    contains(runner_run, "runner=\"$DISPATCH_RUNNER\"", "manual dispatch must remain an explicit runner override")
    routing = "if [[ \"$EVENT_NAME\" == pull_request && (\"$PR_HEAD_REPOSITORY\" != \"$GITHUB_REPOSITORY\" || \"$PR_AUTHOR\" == '\''dependabot[bot]'\'' || \"$RUNNER_POLICY_CHANGED\" == true) ]]"
    contains(runner_run, routing, "fork, Dependabot, policy-change, and manual-dispatch routing must stay outside the mutable selector")
    contains(runner_run, "runner=hosted", "fork, Dependabot, policy-change, and manual-dispatch routing must stay outside the mutable selector")

    need(values["on.workflow_dispatch.inputs.rust_tests_runner.type"] == "choice", "manual dispatch must choose a Rust runner")
    need(values["on.workflow_dispatch.inputs.rust_tests_runner.options.0"] == "tank" &&
         values["on.workflow_dispatch.inputs.rust_tests_runner.options.1"] == "hosted" &&
         seq_count["on.workflow_dispatch.inputs.rust_tests_runner.options"] == 2,
         "manual dispatch must offer exactly tank and hosted Rust runner choices")

    hosted = "(github.event_name == '\''pull_request'\'' && (github.event.pull_request.head.repo.full_name != github.repository || github.event.pull_request.user.login == '\''dependabot[bot]'\'' || needs.channel.outputs.runner_policy_changed == '\''true'\'')) || needs.channel.outputs.rust_tests_runner == '\''hosted'\''"
    contains("jobs.rust-tests.runs-on", hosted, "runs-on must include all hosted guards")
    contains("jobs.rust-tests.concurrency.group", hosted, "concurrency group must include all hosted guards")
    contains("jobs.rust-tests.runs-on", "&& '\''ubuntu-26.04'\'' || '\''tank'\''", "rust-tests must retain the Tank fallback")
    contains("jobs.rust-tests.concurrency.group", "format('\''hosted-{0}'\'', github.run_id) || '\''tank'\''", "hosted overflow must be unique while Tank remains one logical lane")
    need(values["jobs.rust-tests.concurrency.queue"] == "max", "Tank concurrency must retain every pending suite")
    need(values["jobs.rust-tests.concurrency.cancel-in-progress"] == "false", "a new Tank job must not cancel an already-running suite")
    need(values["jobs.rust-tests.env.CARGO_HOME"] == "/home/runner/.cargo", "rust-tests must use the canonical Cargo home")
    need(values["jobs.rust-tests.env.RUSTUP_HOME"] == "/home/runner/.rustup", "rust-tests must use the canonical rustup home")
    need(!nodes["jobs.rust-tests.env.CARGO_TARGET_DIR"], "rust-tests must not share CARGO_TARGET_DIR")
    need(!nodes["jobs.rust-tests.env.RUSTC_WRAPPER"], "sccache must not be correctness-critical at job scope")
    need(values["jobs.rust-tests.env.SCCACHE_GHA_ENABLED"] == "true", "rust-tests must enable GHA sccache")

    changed = "needs.channel.outputs.rust_tests_changed == '\''true'\''"
    preflight = step("jobs.rust-tests", "name", "Verify canonical Rust homes")
    need(preflight >= 0 && values["jobs.rust-tests.steps." preflight ".if"] == changed,
         "canonical-home preflight must skip with the unaffected root Rust archive")
    contains("jobs.rust-tests.steps." preflight ".run", "test ! -L \"$root\"",
             "canonical-home preflight must reject a symlinked root")
    contains("jobs.rust-tests.steps." preflight ".run", "readlink -f \"$root\"",
             "canonical-home preflight must resolve to the literal trusted root")

    setup = step("jobs.rust-tests", "name", "Set up Rust")
    need(setup >= 0 && index(values["jobs.rust-tests.steps." setup ".uses"], "actions-rust-lang/setup-rust-toolchain@") == 1, "rust-tests must use setup-rust-toolchain")
    need(values["jobs.rust-tests.steps." setup ".if"] == changed,
         "Rust setup must skip with the unaffected root Rust archive")
    need(values["jobs.rust-tests.steps." setup ".with.cache-key"] == "rust-tests-v2", "Rust cache key must reject old target-heavy archives")
    need(values["jobs.rust-tests.steps." setup ".with.cache-targets"] == "false", "Rust target archives must remain disabled")
    target_step = step("jobs.rust-tests", "name", "Prepare persistent Tank Cargo target")
    need(target_step >= 0 && values["jobs.rust-tests.steps." target_step ".if"] == changed " && needs.channel.outputs.rust_tests_runner == '\''tank'\''",
         "persistent Cargo target setup must be Tank-only")
    contains("jobs.rust-tests.steps." target_step ".run", "persistent-cargo-target.sh prepare tank",
             "Tank setup must use the persistent Cargo target helper")
    contains("jobs.rust-tests.steps." target_step ".run", "TCL_LSP_TANK_REGISTRATION_ID",
             "target identity must prefer the explicit registration id")
    contains("jobs.rust-tests.steps." target_step ".run", "RUNNER_NAME",
             "target identity must use only the stable runner-name fallback")
    sccache = step("jobs.rust-tests", "name", "Set up sccache")
    need(sccache >= 0 && index(values["jobs.rust-tests.steps." sccache ".uses"], "mozilla-actions/sccache-action@") == 1 && values["jobs.rust-tests.steps." sccache ".with.version"] == "v0.17.0", "rust-tests must retain the pinned sccache setup")
    need(values["jobs.rust-tests.steps." sccache ".with.disable_annotations"] == "true",
         "the resilient statistics step must own cache-degradation warnings")
    need(values["jobs.rust-tests.steps." sccache ".id"] == "sccache" &&
         values["jobs.rust-tests.steps." sccache ".if"] == changed &&
         values["jobs.rust-tests.steps." sccache ".continue-on-error"] == "true",
         "sccache setup must be observable, gated, and non-fatal")
    enable = step("jobs.rust-tests", "name", "Enable sccache when available")
    need(enable > sccache && values["jobs.rust-tests.steps." enable ".if"] == changed,
         "the compiler wrapper must only be enabled after optional sccache setup")
    need(values["jobs.rust-tests.steps." enable ".env.SCCACHE_SETUP_OUTCOME"] == "${{ steps.sccache.outcome }}",
         "wrapper enablement must inspect the sccache setup outcome")
    contains("jobs.rust-tests.steps." enable ".run", "RUSTC_WRAPPER=sccache",
             "successful optional setup must enable the compiler cache")
    contains("jobs.rust-tests.steps." enable ".run", "continuing with uncached compilation",
             "failed optional setup must report the uncached fallback")
    nextest = step("jobs.rust-tests", "name", "cargo nextest run (workspace minus VM-sim heavies and lsp-e2e)")
    need(nextest >= 0, "rust-tests must retain the broad nextest step")
    contains("jobs.rust-tests.steps." nextest ".run", "persistent-cargo-target.sh with-lock",
             "Tank nextest must hold the persistent target lock")
    doctest = step("jobs.rust-tests", "name", "cargo test --doc")
    need(doctest >= 0, "rust-tests must retain the workspace doctest step")
    contains("jobs.rust-tests.steps." doctest ".run", "persistent-cargo-target.sh with-lock",
             "Tank doctests must hold the persistent target lock")
    report = step("jobs.rust-tests", "name", "Report final Tank Cargo target")
    need(report >= 0 && values["jobs.rust-tests.steps." report ".if"] == "always() && " changed " && needs.channel.outputs.rust_tests_runner == '\''tank'\''",
         "final target telemetry must be Tank-only and always run")
    need(values["jobs.rust-tests.steps." report ".continue-on-error"] == "true",
         "final target telemetry must not change test correctness")
    contains("jobs.rust-tests.steps." report ".run", "persistent-cargo-target.sh report",
             "final target telemetry must report the retained target")
    stats = step("jobs.rust-tests", "name", "Report sccache statistics")
    need(stats > enable && values["jobs.rust-tests.steps." stats ".if"] == "always() && " changed,
         "sccache reuse must be measured after every affected test attempt")
    contains("jobs.rust-tests.steps." stats ".run", "sccache --show-stats",
             "cross-registration sccache reuse must be observable")
    contains("jobs.rust-tests.steps." stats ".run", "cache write errors",
             "degraded cache writes must emit a workflow warning")
    contains("jobs.rust-tests.steps." stats ".run", "exit 0",
             "cache-statistics failures must not change test correctness")
    for (path in nodes) if (path ~ /^jobs\.rust-tests\.env\./ && path ~ /SCCACHE_BASEDIRS$/) fail("do not claim cross-checkout Rust remapping without pinned-source support")
}
' "$WORKFLOW"

selector=$REPO_ROOT/scripts/dev/select-rust-tests-runner.sh
tmp=${TMPDIR:-/tmp}/tcl-lsp-runner-policy.$$
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
mkdir -p "$tmp"

cat >"$tmp/gh" <<'EOF'
#!/bin/sh
set -eu
endpoint=${2:-}
case "${SCENARIO:-idle}:$endpoint" in
    fail:*) exit 1 ;;
    occupied:*'status=in_progress'*) printf '%s\n' '[{"workflow_runs":[{"id":41,"status":"in_progress"}]}]' ;;
    hosted:*'status=in_progress'*) printf '%s\n' '[{"workflow_runs":[{"id":42,"status":"in_progress"}]}]' ;;
    current:*'status=in_progress'*) printf '%s\n' '[{"workflow_runs":[{"id":99,"status":"in_progress"}]}]' ;;
    waiting:*'status=waiting'*) printf '%s\n' '[{"workflow_runs":[{"id":43,"status":"waiting"}]}]' ;;
    page2:*'status=pending'*) printf '%s\n' '[{"workflow_runs":[]},{"workflow_runs":[{"id":44,"status":"pending"}]}]' ;;
    *:*'status='*) printf '%s\n' '[{"workflow_runs":[]}]' ;;
    occupied:*'/runs/41/jobs'*) printf '%s\n' '[{"jobs":[{"name":"rust-tests","status":"in_progress","labels":["tank"]}]}]' ;;
    hosted:*'/runs/42/jobs'*) printf '%s\n' '[{"jobs":[{"name":"rust-tests","status":"in_progress","labels":["ubuntu-26.04"]}]}]' ;;
    waiting:*'/runs/43/jobs'*) printf '%s\n' '[{"jobs":[{"name":"rust-tests","status":"waiting","labels":["tank"]}]}]' ;;
    page2:*'/runs/44/jobs'*) printf '%s\n' '[{"jobs":[]},{"jobs":[{"name":"rust-tests","status":"pending","labels":["tank"]}]}]' ;;
    *) printf '%s\n' '[{"jobs":[]}]' ;;
esac
EOF
chmod +x "$tmp/gh"

expect_selection() {
    scenario=$1
    expected=$2
    actual=$(SCENARIO=$scenario GH_BIN="$tmp/gh" "$selector" owner/repo 99)
    if [ "$actual" != "$expected" ]; then
        echo "$scenario: expected $expected, got $actual" >&2
        exit 1
    fi
}

require_path_gate_count() {
    expected=$1
    required=$2
    actual=$(printf '%s\n' "$rust_tests_job" | grep -Fxc "$expected" || true)
    if [ "$actual" -ne "$required" ]; then
        echo "rust-tests expected $required gated steps, found $actual: $expected" >&2
        exit 1
    fi
}

expect_selection idle tank
expect_selection occupied hosted
expect_selection hosted tank
expect_selection current tank
expect_selection waiting hosted
expect_selection page2 hosted
expect_selection fail hosted

case "$(cat "$selector")" in
    *'for status in in_progress queued requested waiting pending; do'*'--paginate --slurp'*) ;;
    *)
        echo "the load detector must cover and paginate every active workflow state" >&2
        exit 1
        ;;
esac

case "$(cat "$WORKFLOW")" in
    *'if [[ "$EVENT_NAME" == pull_request && ("$PR_HEAD_REPOSITORY" != "$GITHUB_REPOSITORY" || "$PR_AUTHOR" == '\''dependabot[bot]'\'' || "$RUNNER_POLICY_CHANGED" == true) ]]'*'runner=hosted'*'elif [[ "$EVENT_NAME" == workflow_dispatch ]]'*'runner="$DISPATCH_RUNNER"'*) ;;
    *)
        echo "fork, Dependabot, policy-change, and manual-dispatch routing must stay outside the mutable selector" >&2
        exit 1
        ;;
esac

# Keep the required job alive for unrelated changes, but do not provision
# toolchains, caches, interpreters, or test binaries when its archive is not
# in the changed-path closure. These are step-level gates deliberately: a
# job-level skip would leave the required status absent.
require_path_gate_count "        if: needs.channel.outputs.rust_tests_changed == 'true'" 5
require_path_gate_count "        if: needs.channel.outputs.rust_tests_changed == 'true' && needs.channel.outputs.docs_only != 'true' && needs.channel.outputs.already_green != 'true'" 2
require_path_gate_count "        if: needs.channel.outputs.rust_tests_changed == 'true' && needs.channel.outputs.docs_only != 'true' && !(startsWith(github.ref, 'refs/tags/') && needs.channel.outputs.already_green == 'true')" 1
case "$(cat "$WORKFLOW")" in
    *'if [ "$rust_tests_changed" = "true" ]; then
              docs_only=false'*) ;;
    *)
        echo "root Rust closure must override the broad docs-only path shape" >&2
        exit 1
        ;;
esac

echo "self-hosted Rust test scheduling contract passed"
