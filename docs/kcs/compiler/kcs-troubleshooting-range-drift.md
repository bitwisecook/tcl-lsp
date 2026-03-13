# KCS: Troubleshooting range drift across passes

## Symptom

A diagnostic code is correct in principle but highlights the wrong token/span, especially after transformations, concatenated words, or control-flow lowering.

## Operational context

Range fidelity depends on preserving source `Range` from lowering through CFG/SSA facts into pass findings and final diagnostics conversion.

## Decision rules / contracts

1. Producer passes own source ranges for emitted findings.
2. Aggregation layer should not reconstruct ranges from message text.
3. Range adjustments must be test-backed when pass logic changes.
4. Offset/position conversion should route through shared mappers (`core/common/source_map.py`) where possible to avoid duplicate converter drift.

## File-path anchors

- `core/compiler/lowering.py`
- `core/compiler/ir.py`
- `core/compiler/optimiser/`
- `core/compiler/gvn.py`
- `core/common/source_map.py`
- `lsp/features/diagnostics.py`

## Failure modes

- Pass emits synthetic ranges not mapped to original source spans.
- Multi-token word joins lose atomic range boundaries.
- Diagnostics layer re-derives ranges and drifts from producer anchor.

## Triage checklist

1. Capture expected vs actual `(line, col)` ranges from test failure.
2. Verify originating pass finding range before diagnostics conversion.
3. Check lowering-originated `Range` values for the underlying IR node.
4. Add regression case with minimal reproducer script.

## Test anchors

- `tests/test_diagnostics.py`
- `tests/test_optimiser.py`
- `tests/test_gvn.py`
- `tests/test_source_map.py`

## Discoverability

- [compiler KCS index](README.md)
- [diagnostics integration](kcs-diagnostics-integration.md)
- [pass/fact ownership matrix](kcs-pass-fact-ownership-matrix.md)
- [shared utility contracts](../kcs-core-lsp-shared-utility-contracts.md)
