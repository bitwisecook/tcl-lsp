#!/usr/bin/env bash
# ============================================================================
# SPIKE -- throwaway proof-of-concept. NOT the final design.
# ============================================================================
#
# Build and run the DYNAMIC side-module C-extension spike.
#
#   runtime/  (Rust cdylib) --rustc--> tcl_ext_spike_dyn_runtime.wasm
#                                       (exports memory + table + Tcl C API)
#   ../ext/pkga.c (unmodified) --clang/-fPIC--> pkga_pic.o
#                              --wasm-ld -shared--> build/pkga.side.wasm
#   loader.py: instantiate runtime, load side module at allocated bases, run it.
#
# Exit 0 on SPIKE PASS.
set -euo pipefail
cd "$(dirname "$0")"
export RUSTUP_TOOLCHAIN=stable
mkdir -p build

echo ">> build runtime cdylib (wasm32-unknown-unknown, exported + growable table)"
# --export-table exposes __indirect_function_table; --growable-table drops its
# fixed max so the loader can append the side module's functions.
RUSTFLAGS="-C link-arg=--export-table -C link-arg=--growable-table" \
    cargo build --quiet --manifest-path runtime/Cargo.toml --target wasm32-unknown-unknown
RT=runtime/target/wasm32-unknown-unknown/debug/tcl_ext_spike_dyn_runtime.wasm

echo ">> build side module from UNMODIFIED ../ext/pkga.c (clang -fPIC, wasm-ld -shared)"
clang --target=wasm32-wasi -ffreestanding -fPIC -fvisibility=default -O2 \
    -c -I ../include ../ext/pkga.c -o build/pkga_pic.o
wasm-ld --experimental-pic -shared --no-entry --import-memory --import-table \
    build/pkga_pic.o -o build/pkga.side.wasm

echo ">> run loader"
echo
exec uv run --with wasmtime python loader.py "$RT" build/pkga.side.wasm
