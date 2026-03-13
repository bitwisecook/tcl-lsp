# KCS: Compilation unit contracts and incremental cache

## Symptom

A compiler change appears correct in an isolated pass, but diagnostics become stale, inconsistent between top-level and proc scopes, or noticeably slower after repeated edits.

## Operational context

`compile_source()` is the compiler pipeline integration boundary for editor features. It builds one `CompilationUnit` containing IR, CFG, SSA/core facts, and interprocedural summaries reused by diagnostics and downstream passes.

This path runs frequently during editing, so incremental proc reuse (`proc_cache`) is part of the contract, not an optional optimisation detail.

## Decision rules / contracts

1. **Single-source-of-truth artefact**
   - New pass inputs should come from `CompilationUnit` / `FunctionUnit` facts before introducing any pass-local parse/lower pipeline.
2. **Per-procedure cache safety**
   - Cache keys must include a stable procedure identity and source slice content hash.
   - Reused entries must preserve range correctness and dialect-sensitive behaviour.
3. **Top-level parity**
   - Top-level and procedure pipelines must keep equivalent fact shape (`cfg`, `ssa`, `analysis`) so downstream consumers do not need mode-specific code paths.
4. **Interprocedural dependency awareness**
   - Any change to call edges, purity, or constant-return modelling must revalidate proc folding and taint propagation consumers.

## File-path anchors

- `core/compiler/compilation_unit.py` (`compile_source`, `CompilationUnit`, `FunctionUnit`)
- `core/compiler/interprocedural.py` (`analyse_interprocedural_ir`)
- `lsp/features/diagnostics.py` (`get_diagnostics`, CU consumption)
- `core/analysis/analyser.py` (CU-assisted semantic diagnostics)

## Failure modes

- Cached procedure unit reused after text drift, causing incorrect ranges/messages.
- Passes rebuilding local IR/SSA and diverging from CU-backed results.
- Interprocedural summaries stale relative to per-proc updates, leading to incorrect O103/T-series outcomes.
- Missing top-level/proc parity causing diagnostics that only work in one scope.

## Test anchors

- `tests/test_compilation_cache.py`
- `tests/test_diagnostics.py`


## Discoverability

- [compiler KCS index](README.md)
- [compiler architecture overview](../../compiler-architecture.md)
