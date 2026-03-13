# KCS: Compiler systems overview (contracts by subsystem)

## Symptom

Contributors know a bug is "in compiler" but cannot quickly determine which subsystem owns the relevant fact contract.

## Operational context

Compiler work spans lowering, CFG/SSA, core analyses, interprocedural summaries, pass-specific findings, diagnostics integration, and codegen boundaries.

## Decision rules / contracts

1. Prefer changing the earliest subsystem that can produce a reusable fact safely.
2. Preserve explicit producer/consumer ownership when adding new facts.
3. Update subsystem-specific KCS notes whenever contract semantics change.

## File-path anchors

- `core/compiler/lowering.py`
- `core/compiler/cfg.py`
- `core/compiler/ssa.py`
- `core/compiler/core_analyses.py`
- `core/compiler/interprocedural.py`
- `core/compiler/optimiser/`
- `core/compiler/codegen/` (package: `__init__.py`, `opcodes.py`, `layout.py`, `format.py`)

## Failure modes

- Fixes applied in late consumers when root cause is earlier fact production.
- New facts added without ownership/docs leading to duplicated pass logic.
- Divergence between diagnostics and codegen expectations for same IR shape.

## Test anchors

- `tests/test_diagnostics.py`
- `tests/test_optimiser.py`
- `tests/test_gvn.py`
- `tests/test_bytecode_identity.py`

## Discoverability

- [compiler KCS index](README.md)
- [pass/fact ownership matrix](kcs-pass-fact-ownership-matrix.md)
- [compiler pipeline overview](kcs-compiler-pipeline-overview.md)
