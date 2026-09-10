#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract test for the binary-aware five-way Rust test fan-out. The matrix keeps
# shard 1 eligible for the one-physical-host Tank lane; shards 2–5 are always
# hosted. The assertions below parse job and step structure, then exercise the
# mocked selector API contract so comments or unrelated jobs cannot satisfy it.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
WORKFLOW=$REPO_ROOT/.github/workflows/ci.yml

rust_tests_job=$(awk '
    /^  rust-tests-shard:/ { in_job = 1 }
    in_job && /^  [A-Za-z0-9_-]+:/ && $1 != "rust-tests-shard:" { exit }
    in_job { print }
' "$WORKFLOW")
aggregate=$(awk '
    /^  rust-tests:/ { in_job = 1 }
    in_job && /^  [A-Za-z0-9_-]+:/ && $1 != "rust-tests:" { exit }
    in_job { print }
' "$WORKFLOW")
doctest_job=$(awk '
    /^  rust-tests-doctest:/ { in_job = 1 }
    in_job && /^  [A-Za-z0-9_-]+:/ && $1 != "rust-tests-doctest:" { exit }
    in_job { print }
' "$WORKFLOW")

if [ -z "$rust_tests_job" ]; then
    echo "ci.yml must define the rust-tests-shard job" >&2
    exit 1
fi
if [ -z "$aggregate" ]; then
    echo "ci.yml must define the rust-tests aggregate job" >&2
    exit 1
fi
if [ -z "$doctest_job" ]; then
    echo "ci.yml must define the rust-tests-doctest job" >&2
    exit 1
fi

# Parse the workflow without PyYAML or another unprovisioned dependency. This
# is a strict parser for the YAML subset used by the contract: mappings,
# sequences, scalar values, and block scalars. It records paths only after
# walking indentation and list structure, so assertions are structural.
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
function flush_block() {
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
    # A flow sequence may continue on the line after its key (`needs:`).
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
    shard_job = "jobs.rust-tests-shard"
    aggregate_job = "jobs.rust-tests"
    doctest_job = "jobs.rust-tests-doctest"
    server_job = "jobs.build-tcl-lsp-server"
    need(values[shard_job ".name"] == "rust-tests-shard (${{ matrix.shard }})", "Rust shards must have an explicit matrix job name")
    # An already-green *tag* must not skip the matrix at job level: a skipped
    # ancestor propagates through the needs graph and takes create-release and
    # every release producer with it (v2.2.5 tagged, reported success, and
    # published nothing). A tag forces rust_tests_changed true, so the job
    # stays in the graph there and step-skips its work instead.
    need(values[shard_job ".if"] == "${{ needs.channel.outputs.rust_tests_changed == '\''true'\'' }}", "unaffected changes must skip the shard matrix, and an already-green tag must not")
    need(values_seen[shard_job ".runs-on"], "rust-tests-shard must define runs-on")
    need(values_seen[aggregate_job ".needs"], "rust-tests aggregate must define needs")
    tank_jobs = 0
    for (path in values) {
        if (path ~ /^jobs\.[^.]+\.runs-on$/ && index(values[path], "tank") != 0) {
            split(path, parts, ".")
            tank_jobs++
            tank_job = parts[2]
        }
    }
    need(tank_jobs == 1 && tank_job == "rust-tests-shard", "only rust-tests-shard may reach Tank")
    need(nodes["jobs.channel"], "ci.yml must define jobs.channel as a mapping")
    need(values["jobs.channel.outputs.runner_policy_changed"] == "${{ steps.paths.outputs.runner_policy_changed }}",
         "channel must publish runner_policy_changed from the paths step")
    need(values["jobs.channel.outputs.rust_tests_runner"] == "${{ steps.rust-runner.outputs.runner }}",
         "channel must publish the broad Rust runner decision")

    paths_step = step("jobs.channel", "id", "paths")
    need(paths_step >= 0, "channel must define the paths step")
    paths_run = "jobs.channel.steps." paths_step ".run"
    path_list = ".github/workflows/ci.yml | .github/dependabot.yml | scripts/dev/changed-paths.sh | scripts/dev/rust-tests-path.sh | scripts/dev/rust-tests-input-paths.txt | scripts/dev/rust-tests-package-paths.txt | scripts/dev/rust-test-binary-shard.sh | scripts/dev/rust-test-binary-shards.tsv | scripts/dev/verify-nextest-binary-shards.py | scripts/dev/test-nextest-binary-shards.sh | scripts/dev/select-rust-tests-runner.sh | scripts/dev/persistent-cargo-target.sh | scripts/dev/test-rust-tests-paths.sh | scripts/dev/test-rust-tests-runner.sh | scripts/dev/test-persistent-cargo-target.sh)"
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
    contains(runner_run, routing, "trust routing must stay outside the mutable selector")
    contains(runner_run, "runner=hosted", "trust routing must force hosted capacity")

    need(values["on.workflow_dispatch.inputs.rust_tests_runner.type"] == "choice", "manual dispatch must choose a Rust runner")
    need(values["on.workflow_dispatch.inputs.rust_tests_runner.options.0"] == "tank" &&
         values["on.workflow_dispatch.inputs.rust_tests_runner.options.1"] == "hosted" &&
         seq_count["on.workflow_dispatch.inputs.rust_tests_runner.options"] == 2,
         "manual dispatch must offer exactly tank and hosted Rust runner choices")

    # Matrix and actual-mode facts must be explicit. In particular, job-level
    # env is valid for steps but is not a valid source for runs-on/concurrency.
    need(values[shard_job ".strategy.fail-fast"] == "false", "Rust shard matrix must disable fail-fast")
    need(seq_count[shard_job ".strategy.matrix.include"] == 5, "Rust shard matrix must define five entries")
    for (i = 0; i < 5; i++) {
        need(values[shard_job ".strategy.matrix.include." i ".shard"] == (i + 1) "/5", "Rust shard matrix has incorrect shard names")
        need(values[shard_job ".strategy.matrix.include." i ".id"] == (i + 1) "-5", "Rust shard matrix has incorrect artifact ids")
        need(values[shard_job ".strategy.matrix.include." i ".tank_capable"] == (i == 0 ? "true" : "false"), "only shard 1 may be Tank-capable")
    }
    need(index(values[shard_job ".runs-on"], "matrix.tank_capable") != 0 && index(values[shard_job ".runs-on"], "needs.channel.outputs.rust_tests_runner") != 0 && index(values[shard_job ".runs-on"], "tank") != 0,
         "runs-on must derive actual mode directly from matrix and channel")
    need(index(values[shard_job ".runs-on"], "github.event_name == '\''pull_request'\''") != 0 && index(values[shard_job ".runs-on"], "github.event.pull_request.head.repo.full_name") != 0 && index(values[shard_job ".runs-on"], "runner_policy_changed") != 0,
         "runs-on must retain direct fork, Dependabot, and policy trust guards")
    need(index(values[shard_job ".concurrency.group"], "matrix.tank_capable") != 0 && index(values[shard_job ".concurrency.group"], "matrix.shard") != 0,
         "concurrency must derive directly from matrix mode and shard")
    need(index(values[shard_job ".concurrency.group"], "github.event_name == '\''pull_request'\''") != 0 && index(values[shard_job ".concurrency.group"], "runner_policy_changed") != 0,
         "concurrency must retain direct trust guards")
    need(index(values[shard_job ".env.RUNNER_MODE"], "matrix.tank_capable") != 0 && index(values[shard_job ".env.RUNNER_MODE"], "needs.channel.outputs.rust_tests_runner") != 0,
         "job steps must receive the actual matrix runner mode")
    need(index(values[shard_job ".runs-on"], "env.RUNNER_MODE") == 0, "runs-on must not use job-level env")
    need(index(values[shard_job ".concurrency.group"], "env.RUNNER_MODE") == 0, "concurrency must not use job-level env")
    need(values[shard_job ".concurrency.queue"] == "max" && values[shard_job ".concurrency.cancel-in-progress"] == "false", "Tank concurrency must retain every pending suite without cancellation")
    need(values[shard_job ".env.CARGO_HOME"] == "/home/runner/.cargo" && values[shard_job ".env.RUSTUP_HOME"] == "/home/runner/.rustup", "Rust shards must use canonical Cargo and rustup homes")
    need(!nodes[shard_job ".env.CARGO_TARGET_DIR"] && !nodes[shard_job ".env.RUSTC_WRAPPER"], "target and wrapper must not be correctness-critical at job scope")
    need(values[shard_job ".env.SCCACHE_GHA_ENABLED"] == "true", "Rust shards must enable GHA sccache")

    changed = "needs.channel.outputs.rust_tests_changed == '\''true'\''"
    active = changed " && (needs.channel.outputs.already_green != '\''true'\'' || matrix.shard == '\''1/5'\'')"
    preflight = step(shard_job, "name", "Verify canonical Rust homes")
    need(preflight >= 0 && values[shard_job ".steps." preflight ".if"] == active, "canonical-home preflight must be path- and warm-path-gated")
    contains(shard_job ".steps." preflight ".run", "test ! -L \"$root\"", "canonical-home preflight must reject a symlinked root")
    contains(shard_job ".steps." preflight ".run", "readlink -f \"$root\"", "canonical-home preflight must resolve the trusted root")

    setup = step(shard_job, "name", "Set up Rust")
    need(setup >= 0 && index(values[shard_job ".steps." setup ".uses"], "actions-rust-lang/setup-rust-toolchain@") == 1 && values[shard_job ".steps." setup ".if"] == active, "Rust setup must remain path- and warm-path-gated")
    need(values[shard_job ".steps." setup ".with.cache-key"] == "rust-tests-v2" && values[shard_job ".steps." setup ".with.cache-targets"] == "false", "Rust target archives must remain disabled")
    target_step = step(shard_job, "name", "Prepare persistent Tank Cargo target")
    need(target_step >= 0 && values[shard_job ".steps." target_step ".if"] == active " && env.RUNNER_MODE == '\''tank'\''", "persistent target setup must use actual Tank mode")
    contains(shard_job ".steps." target_step ".run", "persistent-cargo-target.sh prepare tank", "Tank setup must use the persistent target helper")
    contains(shard_job ".steps." target_step ".run", "TCL_LSP_TANK_REGISTRATION_ID", "target identity must prefer explicit registration id")
    contains(shard_job ".steps." target_step ".run", "RUNNER_NAME", "target identity must have a stable runner fallback")
    sccache = step(shard_job, "name", "Set up sccache")
    need(sccache >= 0 && index(values[shard_job ".steps." sccache ".uses"], "mozilla-actions/sccache-action@") == 1 && values[shard_job ".steps." sccache ".with.version"] == "v0.17.0" && values[shard_job ".steps." sccache ".with.disable_annotations"] == "true", "Rust shards must retain pinned resilient sccache setup")
    need(values[shard_job ".steps." sccache ".id"] == "sccache" && values[shard_job ".steps." sccache ".if"] == active && values[shard_job ".steps." sccache ".continue-on-error"] == "true", "sccache setup must be observable, gated, and non-fatal")
    enable = step(shard_job, "name", "Enable sccache when available")
    need(enable > sccache && values[shard_job ".steps." enable ".if"] == active && values[shard_job ".steps." enable ".env.SCCACHE_SETUP_OUTCOME"] == "${{ steps.sccache.outcome }}", "wrapper enablement must follow optional setup")
    contains(shard_job ".steps." enable ".run", "RUSTC_WRAPPER=sccache", "successful setup must enable compiler cache")
    contains(shard_job ".steps." enable ".run", "continuing with uncached compilation", "failed setup must report uncached fallback")
    nextest_setup = step(shard_job, "name", "Install cargo-nextest")
    need(nextest_setup > enable && values[shard_job ".steps." nextest_setup ".if"] == active, "cargo-nextest setup must be warm-path-gated")

    nextest = step(shard_job, "name", "cargo nextest run (binary-aware workspace shard)")
    need(nextest >= 0, "Rust shards must retain the broad nextest step")
    contains(shard_job ".steps." nextest ".run", "if [ \"$RUST_TESTS_RUNNER\" = tank ]; then", "nextest must branch on the actual per-shard runner mode")
    contains(shard_job ".steps." nextest ".run", "rust-test-binary-shard.sh run \"$SHARD\" $extra", "nextest must use the reviewed binary-aware selection")
    contains(shard_job ".steps." nextest ".run", "cargo nextest run --no-run --workspace --exclude tcl-irule-test --exclude f5-cli --exclude tcl-lsp-server --exclude tcl-fuzz --all-features", "already-green must warm the complete root workspace")
    contains(shard_job ".steps." nextest ".run", "persistent-cargo-target.sh with-lock", "Tank nextest must hold the persistent target lock")
    need(index(values[shard_job ".steps." nextest ".if"], "matrix.shard == '\''1/5'\''") != 0, "already-green warm compilation must be restricted to shard 1")
    shard_upload = step(shard_job, "name", "Upload selected shard listing")
    metadata_upload = step(shard_job, "name", "Upload authoritative Cargo metadata")
    need(shard_upload > nextest && values[shard_job ".steps." shard_upload ".with.overwrite"] == "true", "shard listing uploads must be replaceable on job reruns")
    need(metadata_upload > shard_upload && values[shard_job ".steps." metadata_upload ".with.overwrite"] == "true", "authoritative metadata uploads must be replaceable on job reruns")
    doctest = step(doctest_job, "name", "cargo test --doc")
    need(doctest >= 0, "doctests must run in their own job")
    need(values[doctest_job ".if"] == values[shard_job ".if"], "doctests must retain the shard matrix unchanged/exact-green job skips")
    need(values[doctest_job ".runs-on"] == "ubuntu-26.04", "doctests must run concurrently on hosted capacity")
    need(values[doctest_job ".needs"] == "[channel]", "doctests must be independently concurrent with shards")
    need(values[doctest_job ".steps." doctest ".if"] == changed " && needs.channel.outputs.docs_only != '\''true'\'' && needs.channel.outputs.already_green != '\''true'\''", "doctests must retain exact changed-path and exact-green skip semantics")
    contains(doctest_job ".steps." doctest ".run", "cargo test --workspace --all-features --doc --no-fail-fast", "doctests must retain whole-workspace coverage")
    report = step(shard_job, "name", "Report final Tank Cargo target")
    need(report >= 0 && values[shard_job ".steps." report ".if"] == "always() && " changed " && env.RUNNER_MODE == '\''tank'\''" && values[shard_job ".steps." report ".continue-on-error"] == "true", "final target telemetry must be Tank-only, always-run, and non-fatal")
    contains(shard_job ".steps." report ".run", "persistent-cargo-target.sh report", "final target telemetry must report the retained target")
    stats = step(shard_job, "name", "Report sccache statistics")
    need(stats > enable && values[shard_job ".steps." stats ".if"] == "always() && " active, "sccache reuse must be measured after every active attempt")
    contains(shard_job ".steps." stats ".run", "sccache --show-stats", "sccache reuse must be observable")
    contains(shard_job ".steps." stats ".run", "cache write errors", "degraded cache writes must emit a warning")
    contains(shard_job ".steps." stats ".run", "exit 0", "cache-statistics failures must not change correctness")
    for (path in nodes) if (path ~ /^jobs\.rust-tests-shard\.env\./ && path ~ /SCCACHE_BASEDIRS$/) fail("do not claim cross-checkout Rust remapping without pinned-source support")

    need(values[server_job ".env.SCCACHE_GHA_ENABLED"] == "true", "extension server build must enable the GitHub Actions sccache backend")
    need(!nodes[server_job ".env.RUSTC_WRAPPER"], "extension server compiler cache must not be correctness-critical at job scope")
    server_sccache = step(server_job, "name", "Set up sccache")
    need(server_sccache >= 0 && values[server_job ".steps." server_sccache ".id"] == "sccache", "extension server sccache setup must expose its outcome")
    need(values[server_job ".steps." server_sccache ".if"] == "env.RUN_EXT == '\''true'\''" && values[server_job ".steps." server_sccache ".continue-on-error"] == "true", "extension server sccache setup must be gated and non-fatal")
    need(index(values[server_job ".steps." server_sccache ".uses"], "mozilla-actions/sccache-action@") == 1 && values[server_job ".steps." server_sccache ".with.version"] == "v0.17.0" && values[server_job ".steps." server_sccache ".with.disable_annotations"] == "true", "extension server build must retain pinned resilient sccache setup")
    server_enable = step(server_job, "name", "Enable sccache when available")
    need(server_enable > server_sccache && values[server_job ".steps." server_enable ".if"] == "env.RUN_EXT == '\''true'\''" && values[server_job ".steps." server_enable ".env.SCCACHE_SETUP_OUTCOME"] == "${{ steps.sccache.outcome }}", "extension server wrapper enablement must follow optional setup")
    contains(server_job ".steps." server_enable ".run", "RUSTC_WRAPPER=sccache", "successful extension server setup must enable compiler cache")
    contains(server_job ".steps." server_enable ".run", "continuing with uncached compilation", "failed extension server setup must report uncached fallback")
    server_build = step(server_job, "name", "Build tcl-lsp-server (ci profile)")
    need(server_build > server_enable && values[server_job ".steps." server_build ".run"] == "cargo build -p tcl-lsp-server --profile ci", "extension server build command and profile must remain unchanged")
    server_upload = step(server_job, "name", "Upload tcl-lsp-server binary")
    need(server_upload > server_build && values[server_job ".steps." server_upload ".with.path"] == "target/ci/tcl-lsp-server", "extension tests must receive the unchanged server artefact")
    server_stats = step(server_job, "name", "Report sccache statistics")
    need(server_stats > server_upload && values[server_job ".steps." server_stats ".if"] == "always() && env.RUN_EXT == '\''true'\''" && values[server_job ".steps." server_stats ".continue-on-error"] == "true", "extension server cache statistics must be resilient and run after the build")
    contains(server_job ".steps." server_stats ".run", "sccache --show-stats", "extension server compiler-cache reuse must be observable")
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
    occupied:*'/runs/41/jobs'*) printf '%s\n' '[{"jobs":[{"name":"rust-tests-shard (1/5)","status":"in_progress","labels":["tank"]}]}]' ;;
    hosted:*'/runs/42/jobs'*) printf '%s\n' '[{"jobs":[{"name":"rust-tests-shard (1/5)","status":"in_progress","labels":["ubuntu-26.04"]}]}]' ;;
    waiting:*'/runs/43/jobs'*) printf '%s\n' '[{"jobs":[{"name":"rust-tests-shard (1/5)","status":"waiting","labels":["tank"]}]}]' ;;
    page2:*'/runs/44/jobs'*) printf '%s\n' '[{"jobs":[]},{"jobs":[{"name":"rust-tests-shard (1/5)","status":"pending","labels":["tank"]}]}]' ;;
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
        echo "rust-tests-shard expected $required gated steps, found $actual: $expected" >&2
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
    *) echo "the load detector must cover and paginate every active workflow state" >&2; exit 1 ;;
esac

case "$(cat "$WORKFLOW")" in
    *'if [[ "$EVENT_NAME" == pull_request && ("$PR_HEAD_REPOSITORY" != "$GITHUB_REPOSITORY" || "$PR_AUTHOR" == '\''dependabot[bot]'\'' || "$RUNNER_POLICY_CHANGED" == true) ]]'*'runner=hosted'*'elif [[ "$EVENT_NAME" == workflow_dispatch ]]'*'runner="$DISPATCH_RUNNER"'*) ;;
    *) echo "fork, Dependabot, policy-change, and manual-dispatch routing must stay outside the mutable selector" >&2; exit 1 ;;
esac

# The aggregate must not absorb a skipped shard on a tag. It did, which is how
# v2.2.5 reported success while create-release and every release producer were
# skipped out of the graph; the tag path now has no skip to absorb, and this
# job fails closed if one reappears. IS_TAG/ALREADY_GREEN stay in the env for
# the diagnostic line.
case "$aggregate" in
    *'if: ${{ always() }}'*'needs: [channel, rust-tests-shard, rust-tests-doctest]'*'SHARDS_RESULT'*'DOCTEST_RESULT'*'RUST_TESTS_CHANGED'*'ALREADY_GREEN'*'IS_TAG'*'if [ "$SHARDS_RESULT" = success ] && [ "$DOCTEST_RESULT" = success ]'*'"$SHARDS_RESULT" = skipped'*'"$DOCTEST_RESULT" = skipped'*'"$RUST_TESTS_CHANGED" != true'*) ;;
    *) echo "rust-tests aggregate must always run and fail closed over every shard plus doctests" >&2; exit 1 ;;
esac
case "$aggregate" in
    *'verify-nextest-binary-shards.py --partition-count 5'*'rust-tests-metadata.json'*'rust-test-binary-shards.tsv'*'rust-tests-5-5.json'*) ;;
    *) echo "rust-tests aggregate must prove complete five-way binary selection" >&2; exit 1 ;;
esac

# Keep the stable aggregate alive for unrelated changes and already-green tags.
# For an already-green merge push, the matrix remains allocated but only shard
# 1 may warm the cache; shards 2–5 must skip every expensive setup/report step.
require_path_gate_count "        if: needs.channel.outputs.rust_tests_changed == 'true' && needs.channel.outputs.docs_only != 'true' && needs.channel.outputs.already_green != 'true'" 2
require_path_gate_count "        if: needs.channel.outputs.rust_tests_changed == 'true' && (needs.channel.outputs.already_green != 'true' || matrix.shard == '1/5')" 5
require_path_gate_count "        if: needs.channel.outputs.rust_tests_changed == 'true' && needs.channel.outputs.docs_only != 'true' && !(startsWith(github.ref, 'refs/tags/') && needs.channel.outputs.already_green == 'true') && (needs.channel.outputs.already_green != 'true' || matrix.shard == '1/5')" 1
case "$(cat "$WORKFLOW")" in
    *'if [ "$rust_tests_changed" = "true" ]; then
              docs_only=false'*) ;;
    *) echo "root Rust closure must override the broad docs-only path shape" >&2; exit 1 ;;
esac

echo "binary-aware five-way Rust test scheduling contract passed"
