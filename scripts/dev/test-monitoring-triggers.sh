#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Fail closed if a future cadence edit removes a release benchmark, narrows the
# repository-wide Scorecard push scan, or makes cache retention unbounded.
# Binary-Artifacts examines file content, including extensionless executables,
# so filename filters are not a complete representation of its input surface.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
PERF=$REPO_ROOT/.github/workflows/perf.yml
SCORECARD=$REPO_ROOT/.github/workflows/scorecard.yml
CACHE_CLEANUP=$REPO_ROOT/.github/workflows/cache-cleanup.yml

perf_push=$(awk '
    /^  push:/ { in_push = 1 }
    in_push && /^  pull_request:/ { exit }
    in_push { print }
' "$PERF")

printf '%s\n' "$perf_push" | grep -Fq 'tags: ["v*"]' || {
    echo "performance workflow must benchmark release tags" >&2
    exit 1
}
if printf '%s\n' "$perf_push" | grep -Fq 'branches: [rust]'; then
    echo "performance workflow must not duplicate nightly benchmarks on every merge" >&2
    exit 1
fi

scorecard_push=$(awk '
    /^  push:/ { in_push = 1 }
    in_push && /^permissions:/ { exit }
    in_push { print }
' "$SCORECARD")

printf '%s\n' "$scorecard_push" | grep -Fq 'branches: [rust]' || {
    echo "Scorecard must scan every rust push" >&2
    exit 1
}
if printf '%s\n' "$scorecard_push" | grep -Eq '^[[:space:]]+paths(-ignore)?:'; then
    echo "Scorecard push scan must not use an incomplete filename filter" >&2
    exit 1
fi

grep -Fq 'branch_protection_rule:' "$SCORECARD" || {
    echo "Scorecard must rescan branch-protection changes" >&2
    exit 1
}
grep -Fq 'cron: "23 4 * * 2"' "$SCORECARD" || {
    echo "Scorecard must retain its weekly ecosystem-drift scan" >&2
    exit 1
}

[ "$(grep -c -- '--paginate' "$CACHE_CLEANUP")" -eq 2 ] || {
    echo "cache cleanup must paginate both PR-close and weekly inventories" >&2
    exit 1
}
grep -Fq 'SCCACHE_MAX_IDLE_DAYS: 7' "$CACHE_CLEANUP" || {
    echo "default-branch sccache retention must stay bounded" >&2
    exit 1
}
grep -Fq '(.key | startswith(\"sccache/\"))' "$CACHE_CLEANUP" || {
    echo "weekly cleanup must identify content-addressed sccache objects" >&2
    exit 1
}
grep -Fq '.last_accessed_at < \"$cutoff\"' "$CACHE_CLEANUP" || {
    echo "weekly sccache retention must use last-access time" >&2
    exit 1
}

echo "monitoring workflow trigger contract passed"
