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
- `rust/tcl-lsp-db/src/lib.rs` — `compiler_check_diagnostics` (aggregation)
- `rust/tcl-lsp-server/src/lib.rs` — `lift_compiler_diagnostics`
  (suppression and conversion)

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

- [compiler architecture overview](../compiler-architecture.md)
- [shared utility contracts](../contracts/shared-utility-contracts-rust.md)
