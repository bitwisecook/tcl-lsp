#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CLASSIFIER=$SCRIPT_DIR/spectcl-compat-path.sh

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
