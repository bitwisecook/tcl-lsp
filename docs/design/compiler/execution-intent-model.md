# KCS: execution-intent model (Phase 2)

## Symptom

Passes repeatedly parse command-substitution text to infer runtime intent (shape, substitution kind, and risk), leading to drift and duplicated heuristics.

## Context

`CompilationUnit` now carries `FunctionExecutionIntent` facts per function, built once from CFG statements during `compile_source()`.

## Current intent facets

For command substitutions (`set x [cmd ...]`), intent records:

- invocation shape,
- substitution categories per argument,
- side-effect class (`pure` vs `may_side_effect`),
- escape class (`no_escape` vs `may_escape`),
- shimmer-pressure score (coarse type-conversion pressure heuristic).

## How consumers should use this

1. Prefer `fu.execution_intent.command_substitutions[(block, idx)]` as the primary source.
2. Keep legacy fallback parsing only for robustness when intent is absent.
3. Treat side-effect/escape classes as conservative: unknown commands default to `may_*`.

## Practical use in this repo

- `shimmer.py` uses intent as the fast path for command-substitution shimmer checks.
- `optimiser.py` uses side-effect/escape intent to decide whether dead command-substitution stores are removable.

## Related files

- `server/compiler/execution_intent.py`
- `server/compiler/compilation_unit.py`
- `server/compiler/shimmer.py`
- `server/compiler/optimiser.py`
