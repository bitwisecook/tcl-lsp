# Compiler pipeline overview

The native Rust compiler builds reusable facts once. The language server,
Explorer, bytecode emitter, and WASM pipeline consume those facts through
generic interfaces; command-specific semantics remain in `tcl-registry`.

## Pipeline layers

1. **Lexing and syntax** — `rust/tcl-lexer/`, `rust/tcl-syntax/`, and
   `rust/tcl-compiler/src/parsing/` produce lossless tokens, a red-green CST,
   and segmented commands.
2. **Lowering** — `rust/tcl-compiler/src/lowering/` turns structured source
   into registry-driven IR while retaining source identity and word shape.
3. **CFG and SSA** — `cfg_builder/`, `cfg.rs`, `ssa.rs`, `def_use.rs`, and
   `memory_ssa.rs` build control-flow, scalar, def-use, and memory facts.
4. **Common semantic analysis** — `executable_ir.rs`, `semantic_analysis.rs`,
   `effect_ssa.rs`, `registry_invocation.rs`, and the analysis modules retain
   completion, dispatch, state, representation, and proof facts.
5. **Specialised analyses** — `interprocedural.rs`, `connection_scope.rs`,
   `taint.rs`, `shimmer/`, `intervals.rs`, `interval_bounds.rs`, and
   `optimiser/` produce diagnostics, transformations, and proof sidecars.
6. **Target emission** — bytecode uses `codegen/` and `tcl-bytecode`; WASM
   uses `codegen/wasm::compile_wasm` and the shared runtime ABI.
7. **Consumer publication** — `tcl-explorer`, `tcl-lsp-core`, and the native
   CLI serialise the retained artefacts without rebuilding command semantics.

## Ownership rule

Add a reusable fact at the earliest producer that can establish it soundly.
Consumers should read the typed fact or decline; they must not add a command
name switch or reconstruct a private analysis.

## Related

- [Common semantic compiler contract](common-semantic-compiler.md)
- [Compilation-unit contracts](compilation-unit-contracts.md)
- [Code-generation module map](codegen-module-map.md)
