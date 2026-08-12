# Execution intent model

The per-function intent facts recorded for command substitutions — invocation
shape, substitution kind, side-effect and escape class, and shimmer pressure —
and how a consumer reads them instead of re-parsing substitution text.

`CompilationUnit` carries `FunctionExecutionIntent` facts per function, built once from CFG statements during `compile_source()`.

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

- `shimmer/` uses intent as the fast path for command-substitution shimmer checks.
- `optimiser/` uses side-effect/escape intent to decide whether dead command-substitution stores are removable.

## Related files

- `rust/tcl-compiler/src/execution_intent.rs`
- `rust/tcl-compiler/src/compilation_unit.rs`
- `rust/tcl-compiler/src/shimmer/`
- `rust/tcl-compiler/src/optimiser/`
