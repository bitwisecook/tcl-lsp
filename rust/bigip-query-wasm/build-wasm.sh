#!/usr/bin/env bash
# Build the wasm32 query engine and vendor the artifacts into the f5report
# Python package, so `f5report` can embed a live in-browser query console
# without needing the wasm toolchain at report-generation time.
#
# Requires: rustup wasm32-unknown-unknown target, wasm-bindgen-cli (matching the
# wasm-bindgen crate version pinned in Cargo.toml), and wasm-opt (binaryen).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
vendor="$here/../bigip-query-py/python/f5report/vendor"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

echo "==> cargo build --target wasm32-unknown-unknown --release"
( cd "$here" && cargo build --target wasm32-unknown-unknown --release )
wasm="$here/target/wasm32-unknown-unknown/release/bigip_query_wasm.wasm"

echo "==> wasm-bindgen (no-modules)"
wasm-bindgen "$wasm" --out-dir "$out" --target no-modules --no-typescript

echo "==> wasm-opt -Os"
wasm-opt -Os "$out/bigip_query_wasm_bg.wasm" -o "$vendor/f5query_wasm_bg.wasm"
cp "$out/bigip_query_wasm.js" "$vendor/f5query_wasm.js"

echo "==> vendored:"
ls -la "$vendor"/f5query_wasm*
