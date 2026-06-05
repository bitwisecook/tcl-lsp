#!/usr/bin/env bash
# ============================================================================
# SPIKE -- throwaway proof-of-concept. NOT the final design.
# ============================================================================
# Build and run the Rust-runtime C-extension spike under wasmtime.
#
#   clang  ext/pkga.c (unmodified)  --target=wasm32-wasi -c   ->  pkga.o
#   rustc  src/main.rs (Rust runtime) --target wasm32-wasip1  +  pkga.o
#          ---- linked by wasm-ld into ----                       tcl_ext_spike.wasm
#   wasmtime run tcl_ext_spike.wasm
#
# Exit 0 on SPIKE PASS, non-zero otherwise.
set -euo pipefail
cd "$(dirname "$0")"

TARGET=wasm32-wasip1
echo ">> cargo build --target $TARGET"
cargo build --quiet --target "$TARGET"

WASM="target/$TARGET/debug/tcl_ext_spike.wasm"
echo ">> wasmtime run $WASM"
echo
exec wasmtime run "$WASM"
