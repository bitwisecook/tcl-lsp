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

# Parse the workflow rather than searching comments: the fork, Dependabot, and
# runner-policy guards must be present in the values Actions actually consumes.
python3 - "$WORKFLOW" <<'PY'
import sys

try:
    import yaml
except ModuleNotFoundError as error:
    raise SystemExit(f"PyYAML is required for the CI contract test: {error}")

with open(sys.argv[1], encoding="utf-8") as workflow_file:
    workflow = yaml.safe_load(workflow_file)


def require(condition, message):
    if not condition:
        raise SystemExit(message)


jobs = workflow.get("jobs", {})
rust_tests = jobs.get("rust-tests")
require(isinstance(rust_tests, dict), "ci.yml must define jobs.rust-tests as a mapping")
require(
    [name for name, job in jobs.items() if isinstance(job, dict) and "tank" in str(job.get("runs-on", ""))]
    == ["rust-tests"],
    "rust-tests must remain the only job that can reach Tank",
)
channel = jobs.get("channel")
require(isinstance(channel, dict), "ci.yml must define jobs.channel as a mapping")
outputs = channel.get("outputs", {})
require(outputs.get("runner_policy_changed") == "${{ steps.paths.outputs.runner_policy_changed }}",
        "channel must publish runner_policy_changed from the paths step")
require(outputs.get("rust_tests_runner") == "${{ steps.rust-runner.outputs.runner }}",
        "channel must publish the broad Rust runner decision")

channel_steps = channel.get("steps", [])
paths = next((step for step in channel_steps if step.get("id") == "paths"), {})
path_list = ".github/workflows/ci.yml | .github/dependabot.yml | scripts/dev/select-rust-tests-runner.sh | scripts/dev/test-rust-tests-runner.sh)"
require(path_list in paths.get("run", ""),
        "runner and dependency-policy changes must classify themselves for hosted proof")
runner = next((step for step in channel_steps if step.get("id") == "rust-runner"), {})
runner_script = runner.get("run", "")
require('"$PR_HEAD_REPOSITORY" != "$GITHUB_REPOSITORY"' in runner_script,
        "fork pull requests must force hosted Rust tests")
require('"$PR_AUTHOR" == \'dependabot[bot]\'' in runner_script,
        "Dependabot pull requests must force hosted Rust tests")
require('"$RUNNER_POLICY_CHANGED" == true' in runner_script,
        "runner-policy pull requests must force hosted Rust tests")
require('elif [[ "$EVENT_NAME" == workflow_dispatch ]]' in runner_script
        and 'runner="$DISPATCH_RUNNER"' in runner_script,
        "manual dispatch must remain an explicit runner override")

triggers = workflow.get("on") or workflow.get(True, {})
dispatch = triggers.get("workflow_dispatch", {})
runner_input = dispatch.get("inputs", {}).get("rust_tests_runner", {})
require(runner_input.get("type") == "choice", "manual dispatch must choose a Rust runner")
require(runner_input.get("options") == ["tank", "hosted"],
        "manual dispatch must offer exactly tank and hosted Rust runner choices")

hosted = ("(github.event_name == 'pull_request' && ("
          "github.event.pull_request.head.repo.full_name != github.repository || "
          "github.event.pull_request.user.login == 'dependabot[bot]' || "
          "needs.channel.outputs.runner_policy_changed == 'true')) || "
          "needs.channel.outputs.rust_tests_runner == 'hosted'")
runs_on = rust_tests.get("runs-on", "")
group = rust_tests.get("concurrency", {}).get("group", "")
require(hosted in runs_on, "runs-on must include all hosted guards")
require(hosted in group, "concurrency group must include all hosted guards")
require("&& 'ubuntu-26.04' || 'tank'" in runs_on, "rust-tests must retain the Tank fallback")
require("format('hosted-{0}', github.run_id) || 'tank'" in group,
        "hosted overflow must be unique while Tank remains one logical lane")

concurrency = rust_tests.get("concurrency", {})
require(concurrency.get("queue") == "max", "Tank concurrency must retain every pending suite")
require(concurrency.get("cancel-in-progress") is False,
        "a new Tank job must not cancel an already-running suite")

env = rust_tests.get("env", {})
require(env.get("CARGO_HOME") == "/home/runner/.cargo",
        "rust-tests must use the canonical Cargo home")
require(env.get("RUSTUP_HOME") == "/home/runner/.rustup",
        "rust-tests must use the canonical rustup home")
require("CARGO_TARGET_DIR" not in env, "rust-tests must not share CARGO_TARGET_DIR")
require(env.get("RUSTC_WRAPPER") == "sccache", "rust-tests must retain sccache")
require(env.get("SCCACHE_GHA_ENABLED") == "true", "rust-tests must enable GHA sccache")

setup = next((step for step in rust_tests.get("steps", []) if step.get("name") == "Set up Rust"), {})
require(setup.get("uses", "").startswith("actions-rust-lang/setup-rust-toolchain@"),
        "rust-tests must use setup-rust-toolchain")
require(setup.get("with", {}).get("cache-key") == "rust-tests",
        "Rust cache key must stay stable")
require(setup.get("with", {}).get("cache-targets") is False,
        "Rust target archives must remain disabled")
sccache = next((step for step in rust_tests.get("steps", []) if step.get("name") == "Set up sccache"), {})
require(sccache.get("uses", "").startswith("mozilla-actions/sccache-action@")
        and sccache.get("with", {}).get("version") == "v0.17.0",
        "rust-tests must retain the pinned sccache setup")
stats = next((step for step in rust_tests.get("steps", []) if step.get("name") == "Report sccache statistics"), {})
require(stats.get("if") == "always()" and "sccache --show-stats" in stats.get("run", ""),
        "cross-registration sccache reuse must be measured even after a test failure")
require("SCCACHE_BASEDIRS" not in env,
        "do not claim cross-checkout Rust remapping without pinned-source support")
PY

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
require_path_gate_count "        if: needs.channel.outputs.rust_tests_changed == 'true'" 3
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
