# WASM extensions

The Rust runtime embeds optional Tcl scripts through the `wasm_stdlib` Cargo
feature. The compiler does not select or bundle package-specific runtime
extensions.

## Embedded scripts

`runtime/rust/Cargo.toml` defines `wasm_stdlib`. When enabled,
`runtime/rust/src/embedded_stdlib.rs` seeds the in-memory filesystem with
`init.tcl`, package indices, and Tcl-level library scripts, including
`tcltest/tcltest.tcl`. `host_wasm.rs` exposes the VFS to the runtime. A
filesystem-backed interpreter can therefore load Tcl packages through the
normal `source` and `package require` machinery.

This is not a compiled tcltest extension: there is no tcltest Cargo feature,
variant runtime artefact, C-tier `test*` registration, or compiler scan that
selects an extension from `package require`. `compile_wasm` currently accepts
hosted or standalone packaging options and emits one module through the
canonical runtime ABI. The real-link coverage in
`rust/tcl-compiler/tests/wasm_real_link.rs` is a test helper, not automatic
package bundling.

## Runtime boundary

The compiler owns the user module. The runtime owns command registration,
package state, embedded files, and interpreter dispatch. Optional script files
are loaded by the runtime and do not create a second compiler backend.

## Package-driven extensions (not implemented)

A `package require` in compiled source does not select runtime features or
package-specific artefacts. Runtime package availability depends on the
commands and files supplied by the selected runtime and host.

## Related

- [WASM code generation](wasm-codegen.md)
- [Semantic AOT optimisation](semantic-aot-optimisation.md)
- [WASM runtime boundary](wasm-runtime-primitives.md)
