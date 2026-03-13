# KCS: Troubleshooting duplicate diagnostics across passes

## Symptom

Users see near-identical diagnostics (same location or semantic issue) emitted under multiple code families.

## Operational context

Multiple downstream passes contribute findings that are merged in `get_diagnostics()`. Overlap can occur when ownership boundaries are unclear.

## Decision rules / contracts

1. Each semantic issue shape should have a canonical owning pass.
2. Secondary passes should attach related info rather than duplicate primary finding.
3. Diagnostics-layer dedupe is a safeguard, not a substitute for ownership clarity.

## File-path anchors

- `core/compiler/optimiser/`
- `core/compiler/gvn.py`
- `core/compiler/taint/`
- `core/compiler/irules_flow.py`
- `lsp/features/diagnostics.py`

## Failure modes

- Optimiser and GVN both flag equivalent redundancy shape.
- iRules flow and taint emit overlapping control-flow/security findings.
- Aggregation appends all findings without overlap reconciliation.

## Triage checklist

1. Group duplicate findings by range + normalized message family.
2. Identify originating pass for each duplicate.
3. Move ownership to canonical pass or convert one side to related info.
4. Add integration test proving no duplicate emission for reproducer.

## Test anchors

- `tests/test_diagnostics.py`
- `tests/test_diagnostic_phases.py`
- `tests/test_optimiser.py`
- `tests/test_gvn.py`
- `tests/test_irules_checks.py`

## Discoverability

- [compiler KCS index](README.md)
- [downstream pass contracts](kcs-downstream-pass-contracts.md)
- [pass/fact ownership matrix](kcs-pass-fact-ownership-matrix.md)
