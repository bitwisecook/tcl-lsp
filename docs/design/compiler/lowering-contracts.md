# KCS: IR lowering contracts

## Symptom

Later passes disagree on command/argument interpretation, especially around substitutions and variable forms.

## Context

`lower_to_ir()` translates segmented Tcl commands to a typed IR. This is the first stable semantic layer many passes rely on.

## Contract expectations

- IR node ranges must remain precise and point to user-authored source spans.
- Command-level token snapshots (`CommandTokens`) should preserve enough lexical context for downstream diagnostics.
- Unknown/dynamic constructs should degrade to explicit barrier/call shapes rather than silent assumptions.
- Namespace/proc qualification should be normalized consistently.
- TclOO method bodies (`oo::class create` / `oo::define`) are lifted to `IRModule.methods` (`IRMethodDef`) by `extract_oo_methods_pass` — a post-pass over the *assembled* module so it is independent of incremental chunk caching. The class command itself still lowers to its existing barrier (codegen-unaffected); method extraction is an analysis-only side artefact.

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
