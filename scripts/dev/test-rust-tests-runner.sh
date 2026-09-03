#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract test for the one-physical-host `tank` runner pool. Multiple runner
# registrations must not turn into concurrent heavyweight workspace suites.

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
    *'.github/workflows/ci.yml | scripts/dev/test-rust-tests-runner.sh)'*) ;;
    *)
        echo "runner-policy changes must classify themselves for hosted proof" >&2
        exit 1
        ;;
esac

hosted_condition="github.event.pull_request.head.repo.full_name != github.repository || needs.channel.outputs.runner_policy_changed == 'true'"

case "$rust_tests_job" in
    *"runs-on:"*"$hosted_condition"*"&& 'ubuntu-26.04' || 'tank'"*) ;;
    *)
        echo "rust-tests must keep fork and runner-policy PRs off the self-hosted tank host" >&2
        exit 1
        ;;
esac

case "$rust_tests_job" in
    *"group: rust-tests-"*"$hosted_condition"*"format('hosted-{0}', github.run_id)"*"|| 'tank'"*) ;;
    *)
        echo "rust-tests must serialize trusted jobs in one tank group and keep hosted jobs unique" >&2
        exit 1
        ;;
esac

case "$rust_tests_job" in
    *"cancel-in-progress: false"*) ;;
    *)
        echo "queued tank jobs must not cancel an already-running workspace suite" >&2
        exit 1
        ;;
esac

echo "self-hosted Rust test scheduling contract passed"
