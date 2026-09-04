#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Fail closed if a future cadence edit removes a release benchmark or an
# immediate Scorecard scan for a repository input that affects the score.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
PERF=$REPO_ROOT/.github/workflows/perf.yml
SCORECARD=$REPO_ROOT/.github/workflows/scorecard.yml

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

for required in \
    'branches: [rust]' \
    'paths:' \
    '".github/**"' \
    '"**/Cargo.toml"' \
    '"**/Cargo.lock"' \
    '"**/package.json"' \
    '"**/package-lock.json"' \
    '"**/pyproject.toml"' \
    '"**/build.gradle*"' \
    '"**/settings.gradle*"' \
    '"editors/jetbrains/gradle/**"' \
    '"editors/jetbrains/gradle.properties"' \
    '"editors/jetbrains/gradlew*"' \
    '"rust-toolchain.toml"' \
    '"deny.toml"' \
    '"LICENSE"' \
    '"SECURITY.md"' \
    '"rust/tcl-fuzz/**"'
do
    printf '%s\n' "$scorecard_push" | grep -Fq -- "$required" || {
        echo "Scorecard push filter lost required input: $required" >&2
        exit 1
    }
done

grep -Fq 'branch_protection_rule:' "$SCORECARD" || {
    echo "Scorecard must rescan branch-protection changes" >&2
    exit 1
}
grep -Fq 'cron: "23 4 * * 2"' "$SCORECARD" || {
    echo "Scorecard must retain its weekly ecosystem-drift scan" >&2
    exit 1
}

echo "monitoring workflow trigger contract passed"
