# Compiler pipeline walkthrough

This is the current Rust path from Tcl source to reusable analysis facts and
target artefacts. It is a contract map, not a record of retired implementations.

```text
Tcl source
  -> lexer + red-green CST + segmented commands
  -> registry-driven lowering to source-faithful IR
  -> CFG construction and layout
  -> scalar SSA, def-use, memory SSA, and semantic sidecars
  -> diagnostics, interprocedural facts, taint, shimmer, and optimisation
  -> bytecode ModuleAsm or canonical WASM compile_wasm output
  -> LSP / CLI / Explorer serialisation
```

## Stage ownership

- **Lexing and syntax:** `rust/tcl-lexer`, `rust/tcl-syntax`, and
  `rust/tcl-compiler/src/parsing/` retain token and source identity.
- **Lowering:** `lowering/` and `lowering_hooks.rs` convert structured words
  and registry descriptors into IR. Dynamic or incomplete constructs remain
  explicit rather than being reparsed by a consumer.
- **CFG and dataflow:** `cfg_builder/`, `cfg.rs`, `cfg_layout.rs`, `ssa.rs`,
  `def_use.rs`, and `memory_ssa.rs` build control-flow and versioned facts.
- **Semantic analysis:** `executable_ir.rs`, `semantic_analysis.rs`,
  `effect_ssa.rs`, and `registry_invocation.rs` retain invocation, completion,
  dispatch, world-state, representation, and proof evidence.
- **Specialised passes:** `interprocedural.rs`, `connection_scope.rs`,
  `taint.rs`, `shimmer/`, `intervals.rs`, `interval_bounds.rs`, and
  `optimiser/` add diagnostics and sound transformations.
- **Targets:** `codegen/` emits bytecode artefacts from the common unit;
  `codegen/wasm::compile_wasm` selects a typed WASM plan and emits the shared
  target IR. The VM executes bytecode; it is not a second WASM compiler.

## Consumer contract

`CompilationUnit` is the durable hand-off between analysis and consumers.
LSP providers and Explorer serialisers read its retained facts and typed
declines. They do not infer command semantics from rendered output or add
command-name branches.

## Related

- [Compiler pipeline overview](compiler-pipeline-overview.md)
- [Common semantic compiler contract](common-semantic-compiler.md)
- [Code-generation internals](codegen-internals.md)
- [Explorer coverage contract](../contracts/explorer-compiler-coverage.md)
