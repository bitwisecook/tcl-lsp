# KCS: Contributor runbook (IR -> CFG -> SSA -> diagnostics)

## Symptom

A contributor needs to trace one Tcl sample through the compiler pipeline to understand where a regression is introduced.

## Operational context

Most regressions can be localized by comparing outputs at boundaries: lowering, CFG, SSA/core analyses, then diagnostics aggregation.

## Decision rules / contracts

1. Reproduce with a minimal script first.
2. Inspect one boundary at a time; avoid mixing parser + pass changes in a single debug loop.
3. Lock expected diagnostics with a focused regression test once root cause is found.

## Repro workflow

1. **Create a minimal script fixture**
   - Prefer adding under existing test fixture conventions used by diagnostics/pass tests.
2. **Run diagnostics-focused test first**
   - Start with: `pytest tests/test_diagnostics.py -k <case>`
3. **Run pass-specific tests based on affected family**
   - e.g. `pytest tests/test_optimiser.py -k <case>` or `pytest tests/test_gvn.py -k <case>`.
4. **Check incremental/cancellation behaviour if edit-sequence sensitive**
   - `pytest tests/test_diagnostic_phases.py -k <case>`
   - `pytest tests/test_async_diagnostics.py -k <case>`
5. **Document contract update in relevant KCS note**
   - Update file anchors/failure modes/test anchors if behaviour contract changed.

## File-path anchors

- `core/compiler/lowering.py`
- `core/compiler/cfg.py`
- `core/compiler/ssa.py`
- `core/compiler/core_analyses.py`
- `lsp/features/diagnostics.py`

## Failure modes

- Fix applied at diagnostics layer when true bug is in fact producer.
- Test only covers one pass while integration path remains broken.
- Incremental/edit-sequence regressions missed by single-shot tests.

## Test anchors

- `tests/test_diagnostics.py`
- `tests/test_diagnostic_phases.py`
- `tests/test_async_diagnostics.py`
- `tests/test_optimiser.py`
- `tests/test_gvn.py`

## Discoverability

- [compiler KCS index](README.md)
- [diagnostics integration](kcs-diagnostics-integration.md)
- [pass authoring checklist](kcs-pass-authoring-checklist.md)
