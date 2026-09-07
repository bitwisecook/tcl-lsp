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

case "$(cat "$WORKFLOW")" in
    *'runner_policy_changed: ${{ steps.paths.outputs.runner_policy_changed }}'*) ;;
    *)
        echo "the channel job must publish its runner-policy path decision" >&2
        exit 1
        ;;
esac

hosted_condition="(github.event_name == 'pull_request' && (github.event.pull_request.head.repo.full_name != github.repository || github.event.pull_request.user.login == 'dependabot[bot]' || needs.channel.outputs.runner_policy_changed == 'true')) || needs.channel.outputs.rust_tests_runner == 'hosted'"

case "$(cat "$WORKFLOW")" in
    *'.github/workflows/ci.yml | .github/dependabot.yml | scripts/dev/select-rust-tests-runner.sh | scripts/dev/test-rust-tests-runner.sh)'*) ;;
    *)
        echo "runner and dependency-policy changes must classify themselves for hosted proof" >&2
        exit 1
        ;;
esac

case "$(cat "$WORKFLOW")" in
    *'rust_tests_runner:'*'type: choice'*'- tank'*'- hosted'*) ;;
    *)
        echo "manual CI dispatch must offer tank and hosted Rust runner choices" >&2
        exit 1
        ;;
esac

case "$(cat "$WORKFLOW")" in
    *'rust_tests_runner: ${{ steps.rust-runner.outputs.runner }}'*) ;;
    *)
        echo "the channel job must publish its broad Rust runner decision" >&2
        exit 1
        ;;
esac

case "$rust_tests_job" in
    *"runs-on:"*"($hosted_condition)"*"&& 'ubuntu-26.04' || 'tank'"*) ;;
    *)
        echo "rust-tests must use the channel job's runner decision" >&2
        exit 1
        ;;
esac

case "$rust_tests_job" in
    *"group: rust-tests-"*"($hosted_condition)"*"format('hosted-{0}', github.run_id)"*"|| 'tank'"*) ;;
    *)
        echo "rust-tests must serialize tank jobs and keep load-spilled hosted jobs unique" >&2
        exit 1
        ;;
esac

if ! printf '%s\n' "$rust_tests_job" | grep -Fqx '      cancel-in-progress: false'; then
    echo "a new tank job must not cancel an already-running workspace suite" >&2
    exit 1
fi

if ! printf '%s\n' "$rust_tests_job" | grep -Fqx '      queue: max'; then
    echo "the tank concurrency group must retain every pending workspace suite" >&2
    exit 1
fi

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

echo "self-hosted Rust test scheduling contract passed"
