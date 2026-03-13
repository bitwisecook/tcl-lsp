# KCS: Phase 4 LSP consumers on shared compilation facts

## Goal

Ensure editor-facing consumers read from shared `CompilationUnit` facts instead of rebuilding local analysis pipelines.

## What changed

- Compiler explorer pipeline now resolves one shared `CompilationUnit` and reuses it for:
  - snapshots,
  - interprocedural summaries,
  - optimiser/shimmer/GVN/taint/iRules-flow consumers.
- Semantic graph helpers now accept optional shared `cu` / `analysis` inputs.
- Added `build_semantic_graph_bundle(source)` to build call/symbol/dataflow graphs through one shared compilation path.

## Consumer guidance

1. Prefer `ensure_compilation_unit(source, cu, ...)` at feature entry points.
2. Pass `cu` through to downstream passes (`find_*` functions) whenever supported.
3. When multiple views are requested in one operation, compute `cu` once and fan out.

## Validation expectations

- Add compile-once tests for any new multi-view or multi-pass entry point.
- Keep graceful degradation behaviour on transient compile failures.

## Related files

- `explorer/pipeline.py`
- `core/analysis/semantic_graph.py`
- `server/compiler/compilation_unit.py`
- `tests/test_semantic_graph.py`
- `tests/test_compiler_explorer_pipeline.py`
