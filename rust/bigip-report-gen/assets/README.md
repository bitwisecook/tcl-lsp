# Generated report assets

`f5query_wasm.js` and `f5query_wasm_bg.wasm` are generated from the reviewed
Rust sources in `rust/bigip-query-wasm` by:

```bash
bash rust/bigip-query-wasm/build-wasm.sh
```

They are committed because the native and Python report renderers embed them
to produce a self-contained report whose query console works offline. Keeping
the generated module in the source distribution also lets those renderers
build without a WebAssembly toolchain. The build script pins the Rust crate
graph through `Cargo.lock`, patches the generated glue deterministically, and
runs `wasm-opt -Os` before replacing these two files.

The other files in this directory are reviewable JavaScript and SVG assets.
