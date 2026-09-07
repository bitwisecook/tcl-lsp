#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Decide whether this ref may carry forward a recent green CI result. Every
# lookup is advisory: an API, schema, or local Git failure leaves the answer
# false so the caller runs the complete test surface.

set -euo pipefail

already=false
cutoff="$(date -u -d '24 hours ago' '+%Y-%m-%dT%H:%M:%SZ')" || cutoff=''
cutoff_epoch="$(date -u -d '24 hours ago' '+%s')" || cutoff_epoch=''
api="repos/$GITHUB_REPOSITORY/actions/workflows/ci.yml/runs"

recent_success() {
    local path=$1
    local expected_sha=$2
    local expected_event=$3
    local expected_branch=$4
    local branch_filter='true'
    local jq_filter
    local count

    [ -n "$cutoff" ] || return 2
    case "$cutoff_epoch" in
        '' | *[!0-9]*) return 2 ;;
    esac
    if [ -n "$expected_branch" ]; then
        branch_filter=".head_branch == \"$expected_branch\""
    fi
    # Validate the complete response before counting. In particular, jq's
    # normal cross-type comparison rules must not let a malformed timestamp
    # pass a freshness check. fromdateiso8601 also validates the calendar.
    jq_filter="
        def canonical_utc:
            (type == \"string\")
            and (test(\"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$\"))
            and ((try fromdateiso8601 catch null) | type == \"number\");
        if (type != \"object\" or (.workflow_runs | type) != \"array\") then
            error(\"invalid workflow-runs response\")
        else
            [.workflow_runs[] |
                if (type != \"object\"
                    or (.head_sha | type) != \"string\"
                    or (.event | type) != \"string\"
                    or (.status | type) != \"string\"
                    or (.conclusion | type) != \"string\"
                    or (.head_branch | type) != \"string\"
                    or ((.updated_at | canonical_utc) | not)) then
                    error(\"invalid workflow-run record\")
                else
                    select(.head_sha == \"$expected_sha\"
                        and .event == \"$expected_event\"
                        and .status == \"completed\"
                        and .conclusion == \"success\"
                        and $branch_filter
                        and (.updated_at | fromdateiso8601) > $cutoff_epoch)
                    | 1
                end
            ] | add // 0
        end"
    count="$(gh api "$path" --jq "$jq_filter" 2>/dev/null)" || return 2
    case "$count" in
        '' | *[!0-9]*) return 2 ;;
    esac
    [ "$count" -gt 0 ]
}

valid_sha() {
    [[ "$1" =~ ^[0-9a-f]{40}$ ]]
}

if ! valid_sha "${GITHUB_SHA:-}"; then
    printf 'already_green=false\n' >> "${GITHUB_OUTPUT:-/dev/null}"
    printf 'already_green=false (invalid GITHUB_SHA)\n'
    exit 0
fi

resolve_pr_heads() {
    local heads

    # The API association is authoritative even for a two-parent merge. The
    # local second parent is checked below, but is not itself proof that a
    # merged PR exists (an unrelated merge must not inherit a green result).
    heads="$(gh api "repos/$GITHUB_REPOSITORY/commits/$GITHUB_SHA/pulls" \
        --jq "
            def canonical_utc:
                (type == \"string\")
                and (test(\"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$\"))
                and ((try fromdateiso8601 catch null) | type == \"number\");
            if type != \"array\" then
                error(\"invalid pull-request association response\")
            else
                [.[] |
                    if (type != \"object\"
                        or (has(\"merged_at\") | not)
                        or (has(\"merge_commit_sha\") | not)
                        or (has(\"base\") | not)
                        or (has(\"head\") | not)
                        or (.merge_commit_sha | type) != \"string\"
                        or (.merge_commit_sha | test(\"^[0-9a-f]{40}$\") | not)
                        or ((.merged_at != null) and ((.merged_at | canonical_utc) | not))
                        or (.base | type) != \"object\"
                        or (.base.ref | type) != \"string\"
                        or (.head | type) != \"object\"
                        or (.head.sha | type) != \"string\"
                        or (.head.sha | test(\"^[0-9a-f]{40}$\") | not)) then
                        error(\"invalid pull-request association record\")
                    else
                        select(.merged_at != null
                            and .merge_commit_sha == \"$GITHUB_SHA\"
                            and .base.ref == \"rust\")
                        | .head.sha
                    end
                ] | .[]
            end" \
        2>/dev/null)" || return 1
    printf '%s\n' "$heads"
}

green_pr_for_current_tree() {
    local pr_heads pr_count pr_head merge_head current_tree pr_tree

    pr_heads="$(resolve_pr_heads)" || return 1
    pr_count="$(printf '%s\n' "$pr_heads" | awk 'NF { n += 1 } END { print n + 0 }')"
    [ "$pr_count" -eq 1 ] || return 1
    pr_head=$pr_heads
    valid_sha "$pr_head" || return 1

    # fetch-depth: 2 makes the PR head available for a normal merge commit.
    # It is only a consistency check; the API above remains the association
    # proof and is required for squash merges and for all tag refs.
    merge_head="$(git rev-parse -q --verify HEAD^2 2>/dev/null || true)"
    [ -z "$merge_head" ] || valid_sha "$merge_head" || return 1
    [ -z "$merge_head" ] || [ "$merge_head" = "$pr_head" ] || return 1

    current_tree="$(git rev-parse 'HEAD^{tree}' 2>/dev/null || true)"
    valid_sha "$current_tree" || return 1
    pr_tree="$(git rev-parse "$pr_head^{tree}" 2>/dev/null || true)"
    if [ -z "$pr_tree" ]; then
        pr_tree="$(gh api "repos/$GITHUB_REPOSITORY/git/commits/$pr_head" \
            --jq '.tree.sha' 2>/dev/null || true)"
    fi
    valid_sha "$pr_tree" && [ "$current_tree" = "$pr_tree" ] || return 1
    recent_success \
        "$api?head_sha=$pr_head&event=pull_request&status=success&per_page=20" \
        "$pr_head" pull_request ''
}

if [ -n "$cutoff" ] && [[ "$GITHUB_REF" == refs/tags/v* ]]; then
    # Preserve the cheap exact-SHA push proof for tags cut from an already
    # green rust tip. If that result is absent, use the merged-PR proof below.
    if recent_success \
        "$api?head_sha=$GITHUB_SHA&event=push&status=success&per_page=20" \
        "$GITHUB_SHA" push rust; then
        already=true
    else
        push_status=$?
        # An API/schema failure is not the same as a successful query with no
        # matching run: only the latter may fall through to the PR proof.
        if [ "$push_status" -eq 1 ] && green_pr_for_current_tree; then
            already=true
        fi
    fi
elif [ "$GITHUB_EVENT_NAME" = push ] && [ "$GITHUB_REF" = refs/heads/rust ]; then
    if green_pr_for_current_tree; then
        already=true
    fi
fi

printf 'already_green=%s\n' "$already" >> "${GITHUB_OUTPUT:-/dev/null}"
printf 'already_green=%s (ref=%s sha=%s)\n' "$already" "$GITHUB_REF" "$GITHUB_SHA"
