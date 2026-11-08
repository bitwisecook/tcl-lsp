#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# build_interp_engines.sh <checkout> <out-dir>: build the VM CLI and the runtime
# script runner from <checkout> with fixed release settings, so a base and a
# head build measured by interp_instructions.py differ only in their source.
set -euo pipefail
checkout=$(cd "$1" && pwd)
mkdir -p "$2"
out=$(cd "$2" && pwd)
export CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$checkout/target}"
tommath="${TCL_TOMMATH_DIR:-$checkout/tmp/tcl9.0.4/libtommath}"
test -f "$tommath/tommath.h" || { echo "libtommath not found at $tommath" >&2; exit 1; }
(cd "$checkout" && cargo build --release -p tcl-vm-cli)
cp "$CARGO_TARGET_DIR/release/tclvm" "$out/tclvm"
(cd "$checkout/runtime/rust" && TCL_TOMMATH_DIR="$tommath" cargo build --release --example run_script)
cp "$CARGO_TARGET_DIR/release/examples/run_script" "$out/runtime"
