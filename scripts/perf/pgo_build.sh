#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# pgo_build.sh <out-dir>: build the VM CLI (`tclvm`) and the runtime script
# runner (`runtime`) for this host with profile-guided optimisation, trained on
# every benchmark in scripts/perf/interp/ in both engines.
#
# Needs the toolchain's llvm-profdata (`rustup component add llvm-tools`) and
# libtommath (TCL_TOMMATH_DIR, default tmp/tcl9.0.4/libtommath). A profile is
# only valid for the target it was collected on, so this builds the host only.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$1"
out=$(cd "$1" && pwd)
iterations=${PGO_TRAIN_ITERATIONS:-20000}
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target-pgo}"
export TCL_TOMMATH_DIR="${TCL_TOMMATH_DIR:-$root/tmp/tcl9.0.4/libtommath}"
test -f "$TCL_TOMMATH_DIR/tommath.h" || { echo "libtommath not found at $TCL_TOMMATH_DIR" >&2; exit 1; }
profdata=$(find "$(rustc --print sysroot)/lib/rustlib" -name llvm-profdata -type f | head -n 1)
test -n "$profdata" || { echo "llvm-profdata not found: rustup component add llvm-tools" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

build() {
    (cd "$root" && RUSTFLAGS="$1" cargo build --release -p tcl-vm-cli)
    cp "$CARGO_TARGET_DIR/release/tclvm" "$2/tclvm"
    (cd "$root/runtime/rust" && RUSTFLAGS="$1" cargo build --release --example run_script)
    cp "$CARGO_TARGET_DIR/release/examples/run_script" "$2/runtime"
}

mkdir -p "$work/bin" "$work/profiles"
build "-Cprofile-generate=$work/profiles" "$work/bin"
for bench in "$root"/scripts/perf/interp/*.tcl; do
    sed "s/@N@/$iterations/" "$bench" > "$work/train.tcl"
    "$work/bin/tclvm" "$work/train.tcl" > /dev/null
    "$work/bin/runtime" --quiet "$work/train.tcl" > /dev/null
done
"$profdata" merge -o "$work/merged.profdata" "$work/profiles"
build "-Cprofile-use=$work/merged.profdata" "$out"
echo "PGO engines in $out: tclvm runtime"
