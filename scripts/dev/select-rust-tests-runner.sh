#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Print the preferred runner for a trusted broad Rust suite. Tank remains the
# default while its logical lane is idle; ambiguous API state fails over to a
# fresh hosted runner instead of risking an unbounded self-hosted queue. This
# is deliberately best-effort: the job-level concurrency group remains the
# final guard if two channel jobs observe the lane as idle simultaneously.

set -eu

repository=${1:-}
current_run_id=${2:-}
gh_bin=${GH_BIN:-gh}

case "$repository" in
    */*) ;;
    *)
        echo hosted
        exit 0
        ;;
esac

case "$current_run_id" in
    '' | *[!0-9]*)
        echo hosted
        exit 0
        ;;
esac

active_runs=
for status in in_progress queued requested waiting pending; do
    if ! response=$(
        "$gh_bin" api \
            "repos/$repository/actions/workflows/ci.yml/runs?status=$status&per_page=100" \
            --paginate --slurp \
            2>/dev/null
    ); then
        echo hosted
        exit 0
    fi
    if ! run_ids=$(printf '%s\n' "$response" | jq -er \
        --argjson current "$current_run_id" '
        if type != "array"
            or any(.[]; type != "object" or (.workflow_runs | type) != "array")
            or any(.[].workflow_runs[];
                type != "object" or (.id | type) != "number"
                or (.status != "in_progress" and .status != "queued"
                    and .status != "requested" and .status != "waiting"
                    and .status != "pending"))
        then error("malformed workflow-run response")
        else [.[].workflow_runs[] | select(.id != $current) | .id] | @tsv
        end
        ' 2>/dev/null); then
        echo hosted
        exit 0
    fi
    active_runs="$active_runs $run_ids"
done

for run_id in $active_runs; do
    if ! response=$(
        "$gh_bin" api \
            "repos/$repository/actions/runs/$run_id/jobs?filter=latest&per_page=100" \
            --paginate --slurp \
            2>/dev/null
    ); then
        echo hosted
        exit 0
    fi
    if ! occupied=$(printf '%s\n' "$response" | jq -er '
        if type != "array"
            or any(.[]; type != "object" or (.jobs | type) != "array")
            or any(.[].jobs[];
                type != "object" or (.name | type) != "string"
                or (.labels | type) != "array"
                or any(.labels[]; type != "string")
                or (.status != "in_progress" and .status != "queued"
                    and .status != "requested" and .status != "waiting"
                    and .status != "pending" and .status != "completed"))
        then error("malformed workflow-job response")
        else [.[].jobs[] | select(
                .name == "rust-tests-shard (1/5)"
                and (.labels | index("tank"))
                and .status != "completed"
            )] | length
        end
        ' 2>/dev/null); then
        echo hosted
        exit 0
    fi
    if [ "$occupied" -gt 0 ]; then
        echo hosted
        exit 0
    fi
done

echo tank
