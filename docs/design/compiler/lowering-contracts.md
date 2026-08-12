# IR lowering contracts

What lowering guarantees about the IR it produces — ranges, token snapshots,
degradation for dynamic constructs, and name qualification — so that CFG
construction, SSA, the passes, and codegen all read a command the same way.

`lower_to_ir()` (`rust/tcl-compiler/src/lowering/mod.rs`) translates segmented
Tcl commands to a typed IR — a `Module` of `Statement` trees. This is the
first stable semantic layer many passes rely on. Its variants
(`lower_to_ir_with_dialect`, `lower_to_ir_with_config`,
`lower_to_ir_with_body_cache`, `lower_to_ir_for_bytecode`,
`lower_to_ir_traced`) differ only in the dialect, body cache, codegen mode, or
trace facts they thread through; the contract below holds for all of them.

## Contract expectations

- IR node ranges must remain precise and point to user-authored source spans.
- Command-level token snapshots (`CommandTokens`) should preserve enough lexical context for downstream diagnostics.
- Unknown/dynamic constructs should degrade to explicit barrier/call shapes rather than silent assumptions.
- Namespace/proc qualification should be normalised consistently.
- TclOO method bodies (`oo::class create` / `oo::define`) are lifted to
  `Module::methods` (`MethodDef`, keyed `{class_qname}::{method_name}`) by
  `extract_oo_methods_pass` — a post-pass over the *assembled* module, run from
  `lower_to_ir` after every chunk has been walked, so it is independent of
  incremental chunk caching. Replacement bodies of a redefined method are
  retained in `Module::redefined_methods`. The pass also merges each class's
  whole-class instance-variable union into every one of its methods, so a
  method extracted from an early block still sees state a later `oo::define`
  block declares. The class command itself still lowers to its existing
  barrier and codegen never reads `Module::methods`, so bytecode is
  unaffected; method extraction is an analysis-only side artefact.

## Operational guidance

- When adding new lowering behaviour, document:
  - emitted IR node shape,
  - fallback shape for ambiguous/dynamic inputs,
  - range source and limitations.
- Add unit tests alongside `rust/tcl-compiler/src/lowering/` and downstream pass tests for the same script shape.

## Related files

- `rust/tcl-compiler/src/lowering/`
- `rust/tcl-compiler/src/ir.rs`
- `rust/tcl-compiler/src/ir_helpers.rs`
