#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract test for the archived native lsp-e2e fan-out. Keep the required
# aggregate, producer, and consumers wired together; a textual check catches
# accidental job-level skips or a single unpartitioned fallback.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
WORKFLOW=$REPO_ROOT/.github/workflows/ci.yml

workflow=$(cat "$WORKFLOW")
job_block() {
    awk -v wanted="$1" '
        $0 == "  " wanted ":" { in_job = 1 }
        in_job && $0 ~ /^  [A-Za-z0-9_-]+:/ && $0 != "  " wanted ":" { exit }
        in_job { print }
    ' "$WORKFLOW"
}

aggregate=$(job_block lsp-e2e)
archive=$(job_block lsp-e2e-archive)
partition=$(job_block lsp-e2e-partition)

test -n "$aggregate" || { echo "ci.yml must define lsp-e2e aggregate" >&2; exit 1; }
test -n "$archive" || { echo "ci.yml must define lsp-e2e-archive" >&2; exit 1; }
test -n "$partition" || { echo "ci.yml must define lsp-e2e-partition" >&2; exit 1; }

printf '%s\n' "$aggregate" | grep -Fq 'if: ${{ always() }}' || {
    echo "lsp-e2e must be an always-running aggregate" >&2
    exit 1
}
printf '%s\n' "$aggregate" | grep -Fq 'needs: [channel, lsp-e2e-archive, lsp-e2e-partition]' || {
    echo "lsp-e2e must depend on both producer jobs" >&2
    exit 1
}
for required in 'ARCHIVE_RESULT: $' 'PARTITIONS_RESULT: $' 'exit 1'
do
    case "$aggregate" in *"$required"*) ;; *) echo "lsp-e2e aggregate is missing $required" >&2; exit 1 ;; esac
done

case "$archive" in
    *'if: $'*'needs: [channel]'*'tool: nextest@0.9.143'*'cargo nextest archive -p tcl-lsp-server --all-features'*) ;;
    *) echo "archive job lost its no-op, pinned nextest, or archive command" >&2; exit 1 ;;
esac

for required in 'hash:1/3' 'hash:2/3' 'hash:3/3' '--workspace-remap' \
    'cargo nextest list --archive-file' 'verify-nextest-partitions.py'
do
    case "$archive" in *"$required"*) ;; *) echo "archive job is missing partition proof component $required" >&2; exit 1 ;; esac
done

for required in 'lsp-e2e-result-1-3' 'lsp-e2e-result-2-3' 'lsp-e2e-result-3-3' 'third'
do
    case "$aggregate" in *"$required"*) ;; *) echo "lsp-e2e aggregate is missing $required" >&2; exit 1 ;; esac
done
case "$aggregate" in
    *'--verify-results'*'producer'*'first'*'second'*'third'*) ;;
    *) echo "lsp-e2e aggregate must verify exactly three consumer results" >&2; exit 1 ;;
esac

matrix_partitions=$(printf '%s\n' "$partition" | grep -Ec 'partition: "[123]/3"')
test "$matrix_partitions" -eq 3 || {
    echo "lsp-e2e matrix must define exactly three hash partitions" >&2
    exit 1
}
case "$partition$archive$aggregate" in
    *'1/2'*|*'2/2'*) echo "lsp-e2e workflow retains obsolete two-way partitioning" >&2; exit 1 ;;
esac

case "$partition" in
    *'strategy:'*'fail-fast: false'*'needs: [channel, lsp-e2e-archive]'*'tool: nextest@0.9.143'*'--archive-file'*'--workspace-remap'*'--partition "hash:'*) ;;
    *) echo "partition matrix lost fail-fast, archive remap, pinned nextest, or hash partitioning" >&2; exit 1 ;;
esac

for required in 'docs_only' 'already_green' 'Skip lsp-e2e' 'Warm already-green merge push once'
do
    case "$archive$partition" in *"$required"*) ;; *) echo "lsp-e2e no-op contract is missing $required" >&2; exit 1 ;; esac
done

case "$archive$partition$aggregate" in
    *'lsp_e2e_changed == '\''true'\'''*'lsp_e2e_changed != '\''true'\'''*) ;;
    *) echo "lsp-e2e archive, consumers, or proof transfer lost path-closure gating" >&2; exit 1 ;;
esac
if printf '%s\n' "$archive$partition$aggregate" | grep -Fq "needs.channel.outputs.docs_only != 'true' && needs.channel.outputs.already_green"; then
    echo "lsp-e2e has an expensive normal-path step that bypasses lsp_e2e_changed" >&2
    exit 1
fi

case "$workflow" in *'NEXTEST_BIN_EXE_tcl-lsp-server'*) ;; *) echo "workflow/docs do not name the runtime server path" >&2; exit 1 ;; esac
case "$(cat "$REPO_ROOT/rust/tcl-lsp-server/tests/e2e/common/mod.rs")" in
    *'NEXTEST_BIN_EXE_tcl-lsp-server'*'CARGO_BIN_EXE_tcl-lsp-server'*) ;;
    *) echo "e2e harness must retain Cargo fallback and use nextest runtime path" >&2; exit 1 ;;
esac

echo "lsp-e2e archive/partition workflow contract passed"
