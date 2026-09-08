#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract for carrying a green pull-request result onto its merge push or
# release tag. The optimization is safe only while it resolves one merged PR,
# compares exact Git tree identities, is bounded in time, and fails closed on
# every API or local-Git error.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
WORKFLOW=$REPO_ROOT/.github/workflows/ci.yml
HELPER=$REPO_ROOT/scripts/dev/already-green.sh

green_step=$(awk '
    /      - name: Check whether this exact code is already green/ { in_step = 1 }
    in_step && /      - name: Classify changed paths/ { exit }
    in_step { print }
' "$WORKFLOW")

test -n "$green_step" || {
    echo "ci.yml must define the exact-code already-green step" >&2
    exit 1
}

require_text() {
    description=$1
    wanted=$2
    case "$green_step" in
        *"$wanted"*) ;;
        *)
            echo "already-green contract lost $description" >&2
            exit 1
            ;;
    esac
}

require_helper_text() {
    description=$1
    wanted=$2
    case "$(cat "$HELPER")" in
        *"$wanted"*) ;;
        *)
            echo "already-green helper lost $description" >&2
            exit 1
            ;;
    esac
}

case "$(cat "$WORKFLOW")" in
    *'pull-requests: read # resolve a squash commit back to its PR head'*) ;;
    *)
        echo "the channel job needs read-only PR permission for squash resolution" >&2
        exit 1
        ;;
esac

require_text "the helper invocation" 'bash scripts/dev/already-green.sh'
require_helper_text "the 24-hour freshness bound" "date -u -d '24 hours ago'"
require_helper_text "two-parent merge-head consistency" 'git rev-parse -q --verify HEAD^2'
require_helper_text "squash-to-PR resolution" 'commits/$GITHUB_SHA/pulls'
require_helper_text "the exact merged-commit association" '.merge_commit_sha == \"$GITHUB_SHA\"'
require_helper_text "the protected base-branch restriction" '.base.ref == \"rust\"'
require_helper_text "fail-closed ambiguous association handling" 'pr_count" -eq 1'
require_helper_text "the PR-head tree lookup" 'git/commits/$pr_head'
require_helper_text "local current-tree identity" "git rev-parse 'HEAD^{tree}'"
require_helper_text "quiet verified local PR-head tree lookup" 'git rev-parse -q --verify "$pr_head^{tree}"'
require_helper_text "exact tree equality" '[ "$current_tree" = "$pr_tree" ]'
require_helper_text "successful PR workflow lookup" 'head_sha=$pr_head&event=pull_request&status=success'
require_helper_text "workflow response shape validation" '.workflow_runs | type'
require_helper_text "workflow timestamp type validation" 'def canonical_utc'
require_helper_text "workflow timestamp epoch validation" 'fromdateiso8601'
require_helper_text "workflow head SHA validation" '.head_sha == \"$expected_sha\"'
require_helper_text "workflow conclusion validation" '.conclusion == \"success\"'
require_helper_text "protected merge-push ref" 'refs/heads/rust'
require_helper_text "association response shape validation" 'invalid pull-request association response'
require_helper_text "association merged timestamp validation" '.merged_at | canonical_utc'
require_helper_text "association head SHA validation" '.head.sha | test'
require_helper_text "fail-closed PR resolution" '|| return 1'
require_helper_text "fail-closed tree resolution" '|| true)'

# Deterministic fixtures for the decision itself. These fake only the small
# Git/GitHub surface used by the helper; no network, repository state, or
# Cargo build is involved.
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
fake_bin=$fixture/bin
mkdir -p "$fake_bin"

{
    printf '%s\n' '#!/bin/sh'
    printf '%s\n' 'case "$*" in *"%s"*) printf "%s\\n" "1788782400" ;; *) printf "%s\\n" "2026-09-07T12:00:00Z" ;; esac'
} > "$fake_bin/date"
{
    printf '%s\n' '#!/bin/sh'
    printf '%s\n' 'set -eu'
    printf '%s\n' 'case "$*" in'
    printf '%s\n' '    *"HEAD^2"*) [ -n "${ALREADY_GREEN_MERGE_HEAD:-}" ] && printf "%s\\n" "$ALREADY_GREEN_MERGE_HEAD" || exit 1 ;;'
    printf '%s\n' '    *"HEAD^{tree}"*) printf "%s\\n" "$ALREADY_GREEN_CURRENT_TREE" ;;'
    printf '%s\n' '    *"^{tree}"*) if [ "${ALREADY_GREEN_LOCAL_PR_TREE:-}" = missing ]; then case "$*" in *"-q --verify"*) exit 1 ;; *) for arg do unresolved=$arg; done; printf "%s\\n" "$unresolved"; exit 128 ;; esac; else printf "%s\\n" "$ALREADY_GREEN_PR_TREE"; fi ;;'
    printf '%s\n' '    *) exit 1 ;;'
    printf '%s\n' 'esac'
} > "$fake_bin/git"
{
    printf '%s\n' '#!/bin/sh'
    printf '%s\n' 'set -eu'
    printf '%s\n' 'url=${2:-}'
    printf '%s\n' 'jq_filter=${4:-}'
    printf '%s\n' 'case "${ALREADY_GREEN_GH_MODE:-}" in api-failure) exit 1 ;; esac'
    printf '%s\n' 'case "$url" in'
    printf '%s\n' '    */commits/*/pulls) case "${ALREADY_GREEN_GH_MODE:-}" in malformed-association) printf "%s\\n" "{\"bad\":true}" | jq -r "$jq_filter" ;; *) if [ "${ALREADY_GREEN_GH_MODE:-}" = ambiguous ]; then json="[{\"merged_at\":\"2026-09-07T10:00:00Z\",\"merge_commit_sha\":\"$GITHUB_SHA\",\"base\":{\"ref\":\"rust\"},\"head\":{\"sha\":\"$ALREADY_GREEN_PR_SHA\"}},{\"merged_at\":\"2026-09-07T10:00:00Z\",\"merge_commit_sha\":\"$GITHUB_SHA\",\"base\":{\"ref\":\"rust\"},\"head\":{\"sha\":\"cccccccccccccccccccccccccccccccccccccccc\"}}]"; else json="[{\"merged_at\":\"2026-09-07T10:00:00Z\",\"merge_commit_sha\":\"$GITHUB_SHA\",\"base\":{\"ref\":\"rust\"},\"head\":{\"sha\":\"$ALREADY_GREEN_PR_SHA\"}}]"; fi; printf "%s\\n" "$json" | jq -r "$jq_filter" ;; esac ;;'
    printf '%s\n' '    */git/commits/*) printf "%s\\n" "$ALREADY_GREEN_PR_TREE" ;;'
    printf '%s\n' '    */actions/workflows/ci.yml/runs?*) case "${ALREADY_GREEN_GH_MODE:-}:$url" in push-failure:*"event=push"*) exit 1 ;; esac; case "$url" in *"event=push"*) count=${ALREADY_GREEN_PUSH_COUNT:-0}; head_sha=$GITHUB_SHA; event=push; branch=rust ;; *) count=${ALREADY_GREEN_PR_COUNT:-0}; head_sha=$ALREADY_GREEN_PR_SHA; event=pull_request; branch=release/v2.2.3 ;; esac; if [ "${ALREADY_GREEN_GH_MODE:-}" = malformed-schema ]; then printf "%s\\n" "{\"workflow_runs\":\"not-an-array\"}" | jq -r "$jq_filter"; elif [ "${ALREADY_GREEN_GH_MODE:-}" = malformed-timestamp ]; then printf "%s\\n" "{\"workflow_runs\":[{\"head_sha\":\"$head_sha\",\"event\":\"$event\",\"status\":\"completed\",\"conclusion\":\"success\",\"head_branch\":\"$branch\",\"updated_at\":\"9999-99-99T99:99:99Z\"}]}" | jq -r "$jq_filter"; elif [ "$count" -gt 0 ]; then printf "%s\\n" "{\"workflow_runs\":[{\"head_sha\":\"$head_sha\",\"event\":\"$event\",\"status\":\"completed\",\"conclusion\":\"success\",\"head_branch\":\"$branch\",\"updated_at\":\"2026-09-07T13:00:00Z\"}]}" | jq -r "$jq_filter"; else printf "%s\\n" "{\"workflow_runs\":[]}" | jq -r "$jq_filter"; fi ;;'
    printf '%s\n' '    *) exit 1 ;;'
    printf '%s\n' 'esac'
} > "$fake_bin/gh"
chmod 0755 "$fake_bin/date" "$fake_bin/git" "$fake_bin/gh"

tag_sha=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
pr_sha=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
run_fixture() {
    name=$1
    expected=$2
    shift 2
    output=$fixture/$name.out
    : > "$output"
    if ! env PATH="$fake_bin:$PATH" \
        GITHUB_REPOSITORY=bitwisecook/tcl-lsp \
        GITHUB_REF=refs/tags/v2.2.3 GITHUB_EVENT_NAME=push GITHUB_SHA="$tag_sha" \
        GITHUB_OUTPUT="$output" ALREADY_GREEN_PR_TREE=cccccccccccccccccccccccccccccccccccccccc \
        ALREADY_GREEN_CURRENT_TREE=cccccccccccccccccccccccccccccccccccccccc \
        ALREADY_GREEN_MERGE_HEAD="$pr_sha" ALREADY_GREEN_LOCAL_PR_TREE=present ALREADY_GREEN_PR_SHA="$pr_sha" \
        "$@" "$HELPER" >/dev/null; then
        echo "$name: helper failed instead of failing closed" >&2
        exit 1
    fi
    if ! grep -Fxq "already_green=$expected" "$output"; then
        echo "$name: expected already_green=$expected" >&2
        cat "$output" >&2
        exit 1
    fi
}

run_fixture tag-after-green-pr true \
    env ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_ASSOCIATIONS="$pr_sha"
run_fixture exact-sha-push true \
    env ALREADY_GREEN_PUSH_COUNT=1 ALREADY_GREEN_PR_COUNT=0
run_fixture missing-local-pr-tree-uses-api true \
    env ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_ASSOCIATIONS="$pr_sha" ALREADY_GREEN_LOCAL_PR_TREE=missing
run_fixture changed-tree false \
    env ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_ASSOCIATIONS="$pr_sha" ALREADY_GREEN_CURRENT_TREE=dddddddddddddddddddddddddddddddddddddddd
run_fixture ambiguous-association false \
    env ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_GH_MODE=ambiguous
run_fixture stale false \
    env ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=0 \
        ALREADY_GREEN_ASSOCIATIONS="$pr_sha"
run_fixture push-api-failure false \
    env ALREADY_GREEN_GH_MODE=push-failure ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_ASSOCIATIONS="$pr_sha"
run_fixture api-failure false env ALREADY_GREEN_GH_MODE=api-failure
run_fixture malformed-schema false \
    env ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_ASSOCIATIONS="$pr_sha" ALREADY_GREEN_GH_MODE=malformed-schema
run_fixture malformed-timestamp false \
    env ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_ASSOCIATIONS="$pr_sha" ALREADY_GREEN_GH_MODE=malformed-timestamp
run_fixture malformed-association false \
    env ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_GH_MODE=malformed-association
run_fixture invalid-sha false \
    env GITHUB_SHA=not-a-commit ALREADY_GREEN_PUSH_COUNT=1

# The same PR proof must work for a merge push and remain fail-closed on each
# proof failure in the non-tag branch.
run_fixture merge-push-success true \
    env GITHUB_REF=refs/heads/rust ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_ASSOCIATIONS="$pr_sha"
run_fixture merge-push-changed-tree false \
    env GITHUB_REF=refs/heads/rust ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_ASSOCIATIONS="$pr_sha" ALREADY_GREEN_CURRENT_TREE=dddddddddddddddddddddddddddddddddddddddd
run_fixture merge-push-ambiguous false \
    env GITHUB_REF=refs/heads/rust ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_GH_MODE=ambiguous
run_fixture merge-push-stale false \
    env GITHUB_REF=refs/heads/rust ALREADY_GREEN_PUSH_COUNT=0 ALREADY_GREEN_PR_COUNT=0 \
        ALREADY_GREEN_ASSOCIATIONS="$pr_sha"
run_fixture merge-push-api-failure false \
    env GITHUB_REF=refs/heads/rust ALREADY_GREEN_GH_MODE=api-failure
run_fixture merge-push-parent-mismatch false \
    env GITHUB_REF=refs/heads/rust ALREADY_GREEN_MERGE_HEAD=cccccccccccccccccccccccccccccccccccccccc \
        ALREADY_GREEN_PR_COUNT=1 ALREADY_GREEN_ASSOCIATIONS="$pr_sha"
run_fixture wrong-merge-push-ref false \
    env GITHUB_REF=refs/heads/other ALREADY_GREEN_PR_COUNT=1 \
        ALREADY_GREEN_ASSOCIATIONS="$pr_sha"

# The supply-chain audit is intentionally outside every already-green gate.
cargo_deny_job=$(awk '
    /^  cargo-deny:/ { in_job = 1 }
    in_job && /^  [A-Za-z0-9_-]+:/ && $1 != "cargo-deny:" { exit }
    in_job { print }
' "$WORKFLOW")
case "$cargo_deny_job" in
    *already_green*)
        echo "cargo-deny must remain unconditional" >&2
        exit 1
        ;;
esac

echo "exact-tree green-result reuse contract passed"
