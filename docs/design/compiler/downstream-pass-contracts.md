# Downstream pass contracts (optimiser/taint/shimmer/gvn/irules-flow)

What a specialised pass may assume about the facts it consumes and what it must
guarantee about the findings it emits — code families, ranges, ordering, and
ownership where two passes can flag the same issue.

After CU assembly, specialised passes consume shared facts and emit typed findings. `get_diagnostics()` then applies suppression and LSP conversion. This stage changes frequently as new warning families and optimisation heuristics are added.

## What a pass may assume, and must guarantee

1. **CU-first inputs**
   - Pass entrypoints should accept `CompilationUnit` (or CU-derived function facts) and avoid private lowering/SSA rebuilds.
2. **Typed findings by family**
   - Findings must carry stable diagnostic code families (`O*`, `S*`, `T*`, `IRULE*`) and precise ranges.
3. **Deterministic ordering**
   - Outputs should be stable for unchanged input to avoid diagnostic flicker and flaky integration tests.
4. **No duplicate semantics**
   - If two passes can flag the same issue shape, define canonical ownership and related-info linking rather than duplicate emissions.
5. **Shared word/value-shape helpers**
   - Passes should use shared helper modules for Tcl word/value parsing (`value_shapes.rs`, `var_refs.rs`) instead of embedding pass-local mini-parsers.

## File-path anchors

- `rust/tcl-compiler/src/optimiser/` (`find_optimisations`)
- `rust/tcl-compiler/src/taint.rs` (`find_taint_warnings`)
- `shimmer/` (`find_shimmer_warnings_for_cu`)
- `gvn.rs`
- `irules_checks.rs`
- `value_shapes.rs`
- `var_refs.rs`
- `rust/tcl-lsp-db/src/lib.rs` (pass aggregation and suppression)

## Failure modes

- Diagnostic duplication between optimiser and GVN outputs.
- Pass-specific severity assumptions leaking past diagnostics normalisation.
- Range drift from pass-local source reconstruction.
- Non-deterministic set/dict iteration surfacing as unstable finding order.

## Ownership map

- [kcs-pass-fact-ownership-matrix.md](../../../docs/design/compiler/pass-fact-ownership-matrix.md)

## Tests

The unit tests colocated with each producer module above, plus the LSP
end-to-end diagnostic suites in `rust/tcl-lsp-server/tests/e2e/`.


## See also

- [compiler KCS index](README.md)
- [compiler architecture overview](../../../docs/design/compiler-architecture.md)
- [shared utility contracts](../../../docs/design/contracts/shared-utility-contracts-rust.md)
