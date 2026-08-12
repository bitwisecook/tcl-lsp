# Compiler systems overview

Use this map to find the owner of a compiler fact. The authoritative
implementation is the Rust workspace under `rust/`.

## Fact ownership

| Concern | Rust owner |
|---|---|
| Lexing, syntax, and segmentation | `tcl-lexer`, `tcl-syntax`, `tcl-compiler/src/parsing/` |
| Lowering and registry dispatch | `tcl-compiler/src/lowering/`, `lowering_hooks.rs`, `tcl-registry` |
| CFG construction and layout | `tcl-compiler/src/cfg_builder/`, `cfg.rs`, `cfg_layout.rs` |
| Scalar SSA, def-use, and memory SSA | `ssa.rs`, `def_use.rs`, `memory_ssa.rs`, `place.rs` |
| Executable semantic and world-state facts | `executable_ir.rs`, `semantic_analysis.rs`, `effect_ssa.rs`, `registry_invocation.rs` |
| Interprocedural and cross-event facts | `interprocedural.rs`, `unit_scope.rs`, `connection_scope.rs` |
| Diagnostics and security checks | `analyser/`, `compiler_checks.rs`, `irules_checks.rs` |
| Optimisation and proof declines | `optimiser/`, `gvn.rs`, `sccp.rs`, `var_escape/` |
| Bytecode emission | `codegen/`, `tcl-bytecode` |
| WASM plan selection and emission | `codegen/wasm/`, `tcl-runtime-api`, `runtime/rust/` |

## Decision rules

Change the earliest producer that can establish a reusable fact. Keep command
semantics in the registry, expose typed evidence on compiler-owned results, and
make consumers abstain when a proof is incomplete. Update the relevant
contract and focused Rust tests with every ownership change.

## Related

- [Compiler pipeline overview](compiler-pipeline-overview.md)
- [Pass/fact ownership matrix](pass-fact-ownership-matrix.md)
- [Compilation-unit contracts](compilation-unit-contracts.md)
