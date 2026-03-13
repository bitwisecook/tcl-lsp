# KCS: Diagnostics integration across analyser + compiler passes

## Symptom

Users report diagnostics that disagree by severity, source range, or suppression handling across warning families.

## Operational context

`get_diagnostics()` combines outputs from:

- semantic analyser diagnostics,
- style checks,
- downstream compiler passes (optimiser/taint/shimmer/gvn/iRules flow),
- optional compilation artefacts reused via `CompilationUnit`.

The diagnostics layer is the contract boundary for code-family mapping and suppression semantics seen by LSP clients.

## Decision rules / contracts

1. **Aggregation ownership lives in diagnostics layer**
   - Passes emit typed findings; conversion and final policy mapping happen centrally.
2. **Suppression is global and uniform**
   - `# noqa` and disabled-code filtering applies consistently regardless of finding origin.
3. **Prefer CU-backed consistency**
   - When `CompilationUnit` is available, pass/analyser consumers should use shared artefacts to avoid drift.
4. **Range trust boundary**
   - Publish ranges exactly from producer facts; avoid ad-hoc line/column reconstruction during aggregation.

## File-path anchors

- `lsp/features/diagnostics.py` (`get_diagnostics`, suppression, family aggregation)
- `core/analysis/analyser.py` (semantic warning production)
- `core/compiler/compilation_unit.py` (shared artefact generation)
- `lsp/async_diagnostics.py` (tiered publish integration)

## Failure modes

- Duplicate diagnostics from overlapping pass ownership.
- Mismatched severity defaults between pass-local assumptions and central mapping.
- Source-only fallback path producing different outcomes from CU-backed path.
- Broken suppression when code families are added without diagnostics-layer updates.

## Test anchors

- `tests/test_diagnostics.py`
- `tests/test_diagnostic_phases.py`
- `tests/test_async_diagnostics.py`

## Related KCS notes

- [kcs-downstream-pass-contracts.md](kcs-downstream-pass-contracts.md)
- [kcs-async-diagnostics-tiering.md](kcs-async-diagnostics-tiering.md)
- [kcs-compilation-unit-contracts.md](kcs-compilation-unit-contracts.md)


## Discoverability

- [compiler KCS index](README.md)
- [compiler architecture overview](../../compiler-architecture.md)
