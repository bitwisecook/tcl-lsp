#!/usr/bin/env bash
# Build a SINGLE self-contained .wasm that AOT-runs a Tcl script (or a tcltest
# `.test` file) against the **real embedded Tcl 9 standard library** — `init.tcl`,
# the `package`/Tcl-module machinery, and the `tcltest` package, all baked into
# the binary and seeded into an in-memory VFS. No host filesystem, no `--dir`
# preopen, no source files shipped alongside: `wasmtime merged.wasm` bootstraps
# the stdlib (`source init.tcl` → `package require tcltest`) then runs the
# AOT-compiled script.
#
# Usage: build_test.sh <script.tcl|file.test> [out.wasm]
#
# Differs from build_standalone.sh in two ways:
#   1. the runtime is built with `--features wasm_stdlib` (embeds the stdlib +
#      mounts the VFS + reports $TCL_LIBRARY), and
#   2. the emitted module's `_start` calls `tcl_runtime_init_library` before
#      `::top` (`emit_wasm --init`), so the compiled script runs against a fully
#      initialised interpreter.
#
# Requires: the repo `stable` Rust toolchain with `wasm32-wasip1`, `zig` (the
# runtime build.rs cross-compiles libtommath for the numeric tower), and
# `wasm-merge` (binaryen). `wasm-opt` is used to shrink the result if present.
set -euo pipefail

script="${1:?usage: build_test.sh <script.tcl|file.test> [out.wasm]}"
out="${2:-merged.wasm}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
rt_wasm="$repo_root/runtime/rust/target/wasm32-wasip1/release/tcl_runtime.wasm"
top_wasm="$(mktemp -t aot_top.XXXX.wasm)"
trap 'rm -f "$top_wasm"' EXIT

echo "==> 1/3 build the runtime (wasm32-wasip1, numeric tower + embedded stdlib)"
( cd "$repo_root/runtime/rust" \
    && cargo build --target wasm32-wasip1 --lib --release --features wasm_stdlib )

echo "==> 2/3 emit the AOT module (standalone: ::top + _start + init_library)"
( cd "$repo_root" && cargo run -q -p tcl-compiler --example emit_wasm -- \
    --init "$script" "$top_wasm" )

echo "==> 3/3 merge runtime + module into one self-contained wasm"
# Name the runtime input "tcl" so the emitted module's `(import "tcl" ...)`
# (the codegen ABI + memory + interp bootstrap + init_library) resolves to the
# runtime's exports; -all lets the merge fuse the imported memory into the runtime's.
wasm-merge -all "$rt_wasm" tcl "$top_wasm" user -o "$out"
if command -v wasm-opt >/dev/null 2>&1; then
    wasm-opt -all -O2 "$out" -o "$out"
fi

echo "==> wrote $out ($(wc -c < "$out") bytes)"
echo "    run it:  wasmtime $out"
