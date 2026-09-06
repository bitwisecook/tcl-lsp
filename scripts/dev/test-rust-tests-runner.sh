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

case "$(cat "$WORKFLOW")" in
    *'.github/workflows/ci.yml | .github/dependabot.yml | scripts/dev/test-rust-tests-runner.sh)'*) ;;
    *)
        echo "runner and dependency-policy changes must classify themselves for hosted proof" >&2
        exit 1
        ;;
esac

hosted_condition="(github.event_name == 'pull_request' && (github.event.pull_request.head.repo.full_name != github.repository || needs.channel.outputs.runner_policy_changed == 'true')) || (github.event_name == 'workflow_dispatch' && inputs.rust_tests_runner == 'hosted')"

case "$(cat "$WORKFLOW")" in
    *'rust_tests_runner:'*'type: choice'*'- tank'*'- hosted'*) ;;
    *)
        echo "manual CI dispatch must offer tank and hosted Rust runner choices" >&2
        exit 1
        ;;
esac

case "$rust_tests_job" in
    *"runs-on:"*"($hosted_condition)"*"&& 'ubuntu-26.04' || 'tank'"*) ;;
    *)
        echo "rust-tests must keep fork, hosted dispatch, and runner-policy PRs off tank" >&2
        exit 1
        ;;
esac

case "$rust_tests_job" in
    *"group: rust-tests-"*"($hosted_condition)"*"format('hosted-{0}', github.run_id)"*"|| 'tank'"*) ;;
    *)
        echo "rust-tests must serialize tank jobs and keep hosted jobs unique" >&2
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

echo "self-hosted Rust test scheduling contract passed"
