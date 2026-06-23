#!/usr/bin/env bash
# Build a SINGLE self-contained .wasm from a Tcl script: the AOT-compiled
# `::top` (+ `_start`) merged with the real Rust runtime into one core module
# that runs under `wasmtime merged.wasm` with no host orchestration and no
# dynamic linking. The merged module is a plain core wasm (one memory, WASI
# imports only) — so it also instantiates in a browser given a WASI shim.
#
# Usage: build_standalone.sh <script.tcl> [out.wasm]
#
# Requires: the repo `stable` Rust toolchain with `wasm32-wasip1`, `zig` (the
# runtime build.rs cross-compiles libtommath for the numeric tower), and
# `wasm-merge` (binaryen). `wasm-opt` is used to shrink the result if present.
set -euo pipefail

script="${1:?usage: build_standalone.sh <script.tcl> [out.wasm]}"
out="${2:-merged.wasm}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
rt_wasm="$repo_root/runtime/rust/target/wasm32-wasip1/release/tcl_runtime.wasm"
top_wasm="$(mktemp -t aot_top.XXXX.wasm)"
trap 'rm -f "$top_wasm"' EXIT

echo "==> 1/3 build the runtime (wasm32-wasip1, with the numeric tower)"
( cd "$repo_root/runtime/rust" && cargo build --target wasm32-wasip1 --lib --release )

echo "==> 2/3 emit the AOT module (standalone: ::top + _start)"
( cd "$repo_root" && cargo run -q -p tcl-compiler --example emit_wasm -- \
    --standalone "$script" "$top_wasm" )

echo "==> 3/3 merge runtime + module into one self-contained wasm"
# Name the runtime input "tcl" so the emitted module's `(import "tcl" ...)`
# (the codegen ABI + memory + interp bootstrap) resolves to the runtime's
# exports; -all lets the merge fuse the imported memory into the runtime's.
wasm-merge -all "$rt_wasm" tcl "$top_wasm" user -o "$out"
if command -v wasm-opt >/dev/null 2>&1; then
    wasm-opt -all -O2 "$out" -o "$out"
fi

echo "==> wrote $out ($(wc -c < "$out") bytes)"
echo "    run it:  wasmtime $out"
