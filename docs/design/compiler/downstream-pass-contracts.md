# Downstream pass contracts (optimiser/taint/shimmer/gvn/irules-flow)

What a specialised pass may assume about the facts it consumes and what it must
guarantee about the findings it emits — code families, ranges, ordering, and
ownership where two passes can flag the same issue.

After CU assembly, specialised passes consume shared facts and emit typed
findings. `compiler_checks::run_all_checks` runs the checks, the optimiser's
`optimise_unit` runs the rewrites, and the server's lifts apply suppression
and LSP conversion.

## What a pass may assume, and must guarantee

1. **CU-first inputs.** Pass entry points accept a `CompilationUnit` (or its
   `FunctionUnit`s) and never rebuild lowering or SSA privately.
2. **Typed findings by family.** Findings carry stable code families (`O*`,
   `S*`, `T*`, `IRULE*`) and precise ranges.
3. **Deterministic ordering.** Output is stable for unchanged input, so
   diagnostics do not flicker and integration tests do not flake.
4. **No duplicate semantics.** Where two passes can flag the same shape, one
   owns it and the other links related information rather than emitting twice.
5. **Shared word/value-shape helpers.** Tcl word and value parsing goes through
   `value_shapes.rs` and `var_refs.rs`, never a pass-local mini-parser.

## Anchors

- `rust/tcl-compiler/src/compiler_checks.rs` — `run_all_checks`
- `rust/tcl-compiler/src/optimiser/manager.rs` — `optimise_unit`
- `rust/tcl-compiler/src/taint.rs` — `find_taint_warnings_for_cu` /
  `find_taint_warnings_for_function`
- `rust/tcl-compiler/src/shimmer/` — `find_shimmer_warnings_for_cu`
- `rust/tcl-compiler/src/gvn.rs` — `find_redundancies_for_cu`,
  `find_loop_invariants_for_function`,
  `find_partial_redundancies_for_function`
- `rust/tcl-compiler/src/irules_checks.rs`
- `rust/tcl-compiler/src/value_shapes.rs`, `var_refs.rs`
- `rust/tcl-compiler/src/value_transfer.rs` — the constant a pass reads from the lattice came through the registry's `CommandSemantics` declaration for the resolved invocation, never a pass-local command arm; a pass that needs a command's value asks the lattice, and a command that needs a value declares it in `tcl-registry` ([value-transfers.md](value-transfers.md)). A pass that needs to know whether a statement is a cell update asks the same resolution (`chain_fold`, the tail fold, `static_loops`, `intervals`, W231), never the command's spelling; a declined answer is `Overdefined`, and the statement's recorded explanation says why. Since slice 5: a pass that needs to know whether a definition's statement left a place untouched reads `SccpResult::preserved`, never a private no-match or partial-conversion prover of its own — W210 and W213 (`analyser/diagnostics/dataflow.rs`) are the model every future storage-writing command's preserve outcome follows; a pass that needs the existence branch a condition decided reads the stored fact's `BranchFactKind` rather than re-running the proof (I230, `compiler_checks.rs`). Since slice 8: a pass that needs to know whether a place is bound at a point reads the existence rung (`SccpResult::existence`, `FunctionUnit::existence`) rather than scanning the body for `set` / `unset` by spelling — W210, W211, W213, W214 (`analyser/diagnostics/dataflow.rs`), O108, O109 (`optimiser/elimination.rs`), I230, O101 (`compiler_checks.rs`) and S100 (`shimmer/phi.rs`) all read this one fact; a consumer with no SSA, or built below the deep tier, reads `Unavailable`, never `Unbound`
- `rust/tcl-compiler/src/analyser/diagnostics/proven.rs` — a literal-only check keeps its literal walk and records the `(span, check)` pairs a non-literal word abstains it on; the per-function pass alone re-runs exactly those checks over `value_transfer::proven_word_value`, so a finding never appears twice and never at a span the source does not spell — a check does not grow its own second, lattice-reading code path
- `rust/tcl-lsp-db/src/lib.rs` — `compiler_check_diagnostics` (aggregation)
- `rust/tcl-lsp-server/src/lib.rs` — `compiler_findings` (conversion)
- `rust/tcl-lsp-core/src/diagnostic_policy.rs` — `apply` (suppression)

## Failure modes

- Duplicate diagnostics between optimiser and GVN outputs.
- Pass-specific severity assumptions leaking past normalisation.
- Range drift from pass-local source reconstruction.
- Set/dict iteration order surfacing as unstable finding order.

## Ownership map

- [pass-fact-ownership-matrix.md](pass-fact-ownership-matrix.md)

## Tests

The unit tests colocated with each producer module above, plus the LSP
end-to-end diagnostic suites in `rust/tcl-lsp-server/tests/e2e/`.

## See also

- [compiler architecture overview](architecture.md)
- [shared utility contracts](../contracts/shared-utility-contracts-rust.md)
