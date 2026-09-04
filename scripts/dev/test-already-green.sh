#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract for carrying a green pull-request result onto its merge push. The
# optimization is safe only while it resolves the real PR head, compares exact
# Git tree identities, is bounded in time, and fails closed on every API error.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
WORKFLOW=$REPO_ROOT/.github/workflows/ci.yml

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

case "$(cat "$WORKFLOW")" in
    *'pull-requests: read # resolve a squash commit back to its PR head'*) ;;
    *)
        echo "the channel job needs read-only PR permission for squash resolution" >&2
        exit 1
        ;;
esac

require_text "the 24-hour freshness bound" "date -u -d '24 hours ago'"
require_text "two-parent merge-head resolution" 'git rev-parse -q --verify HEAD^2'
require_text "squash-to-PR resolution" 'commits/$GITHUB_SHA/pulls'
require_text "the exact merged-commit association" '.merge_commit_sha == \"$GITHUB_SHA\"'
require_text "the protected base-branch restriction" '.base.ref == \"rust\"'
require_text "the PR-head tree lookup" 'git/commits/$pr_head'
require_text "local current-tree identity" "git rev-parse 'HEAD^{tree}'"
require_text "exact tree equality" '[ "$current_tree" = "$pr_tree" ]'
require_text "successful PR workflow lookup" 'head_sha=$pr_head&event=pull_request&status=success'
require_text "fail-closed PR resolution" '|| pr_heads=""'
require_text "fail-closed tree resolution" '|| pr_tree=""'

echo "exact-tree green-result reuse contract passed"
