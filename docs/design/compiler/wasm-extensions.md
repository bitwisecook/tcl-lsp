# WASM extensions

> **Current truth:** the Rust runtime embeds optional Tcl scripts through the
> `wasm_stdlib` Cargo feature. The compiler does not yet select or bundle
> package-specific runtime extensions.

## What ships today

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

## Future desired state

> **Future desired state — written and reviewed 2026-08-11; not implemented.**

A package-aware extension system may later:

1. scan retained `package require` facts from `CompilationUnit` through generic
   package metadata;
2. resolve requested runtime features against an explicit extension registry;
3. build or select compatible runtime/extension artefacts; and
4. link them with the canonical `WasmModule` while preserving the shared memory,
   object ownership, completion, and capability contracts.

That work must add a Rust implementation and tests first. It must not add a
second compiler backend or an out-of-band bundle test path.

## Related

- [WASM code generation](wasm-codegen.md)
- [Semantic AOT optimisation](semantic-aot-optimisation.md)
- [WASM runtime boundary](wasm-runtime-primitives.md)
- [Package command oracle](command-oracle-audits.md)
