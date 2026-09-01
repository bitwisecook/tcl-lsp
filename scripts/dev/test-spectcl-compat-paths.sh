#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CLASSIFIER=$SCRIPT_DIR/spectcl-compat-path.sh
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

expect_relevant() {
    if ! "$CLASSIFIER" "$1"; then
        echo "expected SpecTcl-relevant path: $1" >&2
        exit 1
    fi
}

expect_unrelated() {
    if "$CLASSIFIER" "$1"; then
        echo "expected unrelated path: $1" >&2
        exit 1
    else
        status=$?
        if [ "$status" -ne 1 ]; then
            echo "classifier failed for unrelated path $1 with status $status" >&2
            exit 1
        fi
    fi
}

expect_relevant specs/eda_cadence.tclspec
expect_relevant docs/design/spec-dsl-examples/foreach.tclspec
expect_relevant docs/design/spec-dsl-examples/external/tcllib.tclspec
expect_relevant rust/tcl-compiler/src/lib.rs
expect_relevant runtime/rust/src/lib.rs
expect_relevant rust/tcl-spectcl/data/shipped-pack-dirs.txt
expect_unrelated README.md
expect_unrelated editors/vscode/src/extension.ts

bad_manifest=$(mktemp)
trap 'rm -f "$bad_manifest"' EXIT HUP INT TERM
printf '%s\n' 'specs/' > "$bad_manifest"
if TCL_LSP_SPECTCL_PACK_DIRS_MANIFEST=$bad_manifest "$CLASSIFIER" README.md; then
    echo "malformed shipped-pack manifest unexpectedly passed" >&2
    exit 1
else
    status=$?
    if [ "$status" -ne 2 ]; then
        echo "malformed shipped-pack manifest returned $status, expected 2" >&2
        exit 1
    fi
fi

echo "SpecTcl compatibility path classifier tests passed"

# The owner target must include the shipped-pack corpus lane directly. The
# general workspace test job also reaches it today, but relying on that would
# let a future test partition silently make the focused merge-blocking lane
# vacuous for production hook installation and invocation.
spectcl_target=$(awk '
    /^test-spectcl-compat:/ { in_target = 1 }
    in_target && /^[A-Za-z0-9_.-]+:/ && $1 != "test-spectcl-compat:" { exit }
    in_target { print }
' "$REPO_ROOT/Makefile")

case "$spectcl_target" in
    *'--test eval_loader'*'--test golden_packs'*'--test pack_source_e2e'*'--test spec_corpus'*'--test pack_is_real_tcl'*) ;;
    *)
        echo "test-spectcl-compat must own loader, upgrade, live-hook, shipped-corpus, and real-Tcl cases" >&2
        exit 1
        ;;
esac

echo "SpecTcl compatibility target contract tests passed"

# The repository's existing required check is `pr-gate`, so the separate
# compatibility job must feed a real failure into that check. A plain `needs`
# edge is insufficient: Actions skips dependent jobs after a failed need.
# Keep this small textual contract beside the classifier contract so changing
# either half of the merge-blocking lane is caught by `make rust-check`.
pr_gate_block=$(awk '
    /^  pr-gate:/ { in_pr_gate = 1 }
    in_pr_gate && /^  [A-Za-z0-9_-]+:/ && $1 != "pr-gate:" { exit }
    in_pr_gate { print }
' "$REPO_ROOT/.github/workflows/ci.yml")

case "$pr_gate_block" in
    *'if: ${{ always() }}'*) ;;
    *)
        echo "pr-gate must run under always() so failed prerequisites are propagated" >&2
        exit 1
        ;;
esac
case "$pr_gate_block" in
    *'needs: [channel, spectcl-compat]'*) ;;
    *)
        echo "pr-gate must depend on channel and spectcl-compat" >&2
        exit 1
        ;;
esac
case "$pr_gate_block" in
    *'CHANNEL_RESULT: ${{ needs.channel.result }}'*'SPECTCL_COMPAT_RESULT: ${{ needs.spectcl-compat.result }}'*'if [ "$CHANNEL_RESULT" != success ] || [ "$SPECTCL_COMPAT_RESULT" != success ]; then'*'exit 1'*) ;;
    *)
        echo "pr-gate must explicitly fail when a prerequisite gate fails" >&2
        exit 1
        ;;
esac

echo "SpecTcl merge-blocking gate contract tests passed"
