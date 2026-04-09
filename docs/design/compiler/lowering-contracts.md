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

## Operational guidance

- When adding new lowering behaviour, document:
  - emitted IR node shape,
  - fallback shape for ambiguous/dynamic inputs,
  - range source and limitations.
- Add tests in `tests/test_ir_lowering.py` and downstream pass tests for the same script shape.

## Related files

- `core/compiler/lowering.py`
- `core/compiler/ir.py`
- `core/compiler/token_helpers.py`
