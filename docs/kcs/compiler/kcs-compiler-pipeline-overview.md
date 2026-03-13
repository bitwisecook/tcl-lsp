# KCS: Compiler pipeline overview (LSP-oriented)

## Symptom

A contributor needs to add or debug analysis behaviour but cannot quickly determine where a fact should be produced and consumed.

## Context

The compiler pipeline spans parsing, lowering, control/data-flow analysis, specialised passes, and bytecode emission. LSP quality depends on producing reusable facts as early as practical.

## Pipeline layers

1. Parse and segment
   - source: `core/parsing/*`
2. Lowering to IR
   - source: `core/compiler/lowering.py`, `core/compiler/ir.py`
3. CFG + SSA + core analyses
   - source: `core/compiler/cfg.py`, `core/compiler/ssa.py`, `core/compiler/core_analyses.py`
4. Specialised passes (optimiser, shimmer, taint, etc.)
   - source: `core/compiler/optimiser/`, `core/compiler/shimmer.py`, `core/compiler/taint/`
5. Diagnostics composition for editor publication
   - source: `lsp/features/diagnostics.py`
6. Bytecode/disassembly generation
   - source: `core/compiler/codegen.py`

## Decision rule

If a fact can improve diagnostics/completions/quick fixes, prefer modelling it at IR/CFG/SSA/pass level, not as a codegen-only behaviour.

## Related docs

- [docs/compiler-architecture.md](../../compiler-architecture.md)


## Related compiler KCS

- [kcs-compilation-unit-contracts.md](kcs-compilation-unit-contracts.md)
- [kcs-downstream-pass-contracts.md](kcs-downstream-pass-contracts.md)
- [kcs-diagnostics-integration.md](kcs-diagnostics-integration.md)
- [kcs-async-diagnostics-tiering.md](kcs-async-diagnostics-tiering.md)
