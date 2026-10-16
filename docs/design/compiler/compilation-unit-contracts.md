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
contract. In the server, `function_lattice` interns a `FnLatticeKey` whose
inputs include `BodyLatticeInputs`, qname and parameters, prepared CFG context,
exact lexer config, dialect and registry snapshot, parameter seeds, whole-module
facts, source-entry/command-trust state, and analysis context. `BodyLatticeInputs`
may be an uncertified IR body or an authentic retained-source input with a
checked declaration/source correspondence. Offset rebasing does not establish
that correspondence. Memo reuse also requires rebuilding under the current
context and matching the current procedure CFG; source-order or module-derived
effects can require a fresh build. The differential contract is that the
memoised result matches the full build for the supported reuse path.

## What a pass may assume

1. **Single source of truth.** New pass inputs come from `CompilationUnit` /
   `FunctionUnit` facts, never from a pass-local parse/lower pipeline. TclOO
   method bodies are first-class: `CompilationUnit::methods` holds a
   per-method `FunctionUnit` and `interproc.methods` a `MethodSummary`.
2. **Memo safety.** `FnLatticeKey` includes more than normalised body text and
   the closed CFG context: `BodyLatticeInputs`, qname/parameters, exact lexer
   config, dialect and registry snapshot, parameter constants, whole-module
   class/trace facts, source-entry and command-trust metadata, and the value
   transfer analysis context all participate. A retained-source
   `BodyLatticeInputs` carries a source correspondence certificate only when
   the current source, declaration and analysis inputs pass the retention
   checks; position rebasing alone cannot certify it. The database reconstructs
   the prepared CFG and only reuses a candidate when it matches the current
   procedure CFG. Module-derived or source-order effects that cannot be
   reconstructed from the key take the fresh-build path. Keep this owner check
   authoritative when describing reuse; a prose summary of selected fields is
   not an exhaustive identity contract.
3. **Top-level parity.** Top-level and procedure units keep the same fact
   shape (`cfg`, `ssa`, `sccp`, `types`, …) so consumers need no
   mode-specific paths.
4. **Interprocedural dependency.** A change to call edges, purity, or
   constant-return modelling must revalidate proc folding (O103) and taint
   propagation (T-series) consumers.

Source-order registry-effect projection keeps completed root-boundary states
separate from historical observations. A retained user call joins the closed
boundary effects rather than introducing arbitrary command-table mutation.
Independently callable method and body units replay from those boundaries, so
pre-definition missing candidates do not turn known helpers into unresolved
handlers. Actual missing targets and opaque mutations remain barriers.

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
