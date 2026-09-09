# Compilation unit contracts and incremental cache

What a pass may assume about the `CompilationUnit` it consumes, and the
per-procedure memo that keeps repeated edits cheap. Read this before adding a
pass input, so that diagnostics stay consistent between top-level and
procedure scopes as a document is edited.

`CompilationUnit::build_for` and its siblings (`build_for_dialect`,
`build_with_options`, `build_for_memoized`) are the compiler's integration
boundary for editor features: one `CompilationUnit` carries IR, CFG,
per-function SSA/core facts (`FunctionUnit`), and, after
`with_interprocedural`, the interprocedural summary — reused by diagnostics
and every downstream pass.

This path runs on every edit, so the per-procedure lattice memo is part of the
contract: `build_for_memoized` normalises each procedure body to offset 0 and
hands it to a `ProcLatticeCache` — in the server, salsa's `function_lattice`
query keyed by `FnLatticeKey` — then rebases the returned `FunctionUnit`
through `base_offset`. The result must be byte-identical to an unmemoised
build.

## What a pass may assume

1. **Single source of truth.** New pass inputs come from `CompilationUnit` /
   `FunctionUnit` facts, never from a pass-local parse/lower pipeline. TclOO
   method bodies are first-class: `CompilationUnit::methods` holds a
   per-method `FunctionUnit` and `interproc.methods` a `MethodSummary`.
2. **Memo safety.** The memo key is the offset-0 body IR, qualified name,
   parameters, module `CfgContext`, dialect, interprocedural parameter seeds,
   known classes, and trace facts — never the body's position. A reused unit
   must preserve range correctness (`abs_span` / `abs_pos`) and
   dialect-sensitive behaviour.
3. **Top-level parity.** Top-level and procedure units keep the same fact
   shape (`cfg`, `ssa`, `sccp`, `types`, …) so consumers need no
   mode-specific paths.
4. **Interprocedural dependency.** A change to call edges, purity, or
   constant-return modelling must revalidate proc folding (O103) and taint
   propagation (T-series) consumers.

## Anchors

- `rust/tcl-compiler/src/compilation_unit.rs` — `CompilationUnit`,
  `FunctionUnit`, `ProcLatticeCache`, `LatticeRequest`, `UnitBuildOptions`
- `rust/tcl-compiler/src/interprocedural.rs` —
  `build_interprocedural_analysis`
- `rust/tcl-lsp-db/src/lib.rs` — `compilation_unit`, `function_lattice`,
  `FnLatticeKey`, `CfgContext`
- `rust/tcl-compiler/src/analyser/` — CU-assisted semantic diagnostics

## Failure modes

- A memoised unit reused after text drift, giving wrong ranges or messages.
- A pass rebuilding local IR/SSA and diverging from the CU-backed result.
- Interprocedural summaries stale relative to per-proc updates.
- Missing top-level/proc parity, so a diagnostic works in only one scope.

## Tests

- `rust/tcl-compiler/src/compilation_unit.rs` unit tests.
- `rust/tcl-lsp-db/src/lib.rs` incremental-versus-fresh differential tests.
- `rust/tcl-lsp-server/tests/e2e/` — the diagnostic end-to-end suites.

## See also

- [cfg-ssa-fact-model.md](cfg-ssa-fact-model.md)
- [compiler architecture overview](architecture.md)
