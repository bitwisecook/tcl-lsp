# v1.4.0

## New Features
- **W308 — TclOO method validation**: validates method calls on typed object
  variables. The type lattice tracks `$obj method` patterns and checks class
  hierarchies to flag unknown methods.
- **TclOO constructor type inference**: type propagation recognises
  user-defined class constructors (`ClassName new`, `ClassName create`) as
  returning typed OBJECT instances, enabling downstream method validation.

## Improvements
- **ProcessPoolExecutor parallelism**: analysis and deep diagnostics now run
  in a subprocess pool for true GIL-free parallelism, with strategic yield
  points between chunks, procedures, and fixed-point iterations.
- **Subprocess diagnostic computation**: basic diagnostics are computed in the
  analysis subprocess, eliminating a separate thread that held the GIL.
- **Non-blocking LSP handlers**: document symbols, code actions, folding
  ranges, inlay hints, and document links return partial or empty results
  while analysis is pending instead of blocking the event loop. Results
  refresh automatically when analysis completes.
- **Instant document symbols**: chunk-based symbols (events, procs,
  namespaces, classes) appear immediately on file open, before full analysis.
- **IRULE1005 enabled by default**: HTTP collect/release validation no longer
  requires explicit opt-in.
- **W307 refinement**: non-literal command name diagnostic now defers `$var`
  patterns to the post-analysis type-aware pass, only emitting immediately
  for command substitutions.

## Bug Fixes
- Fixed analysis being skipped when `didOpen` pre-creates document state,
  leaving code actions, folding ranges, inlay hints, and document links empty
  until the next edit.
- Fixed subprocess semantic token cache not being carried forward, causing
  re-computation on every request.
- Fixed trailing-edge event loop blocking after subprocess analysis return.
