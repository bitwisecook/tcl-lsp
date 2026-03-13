# KCS: Downstream pass contracts (optimiser/taint/shimmer/gvn/irules-flow)

## Symptom

A pass update introduces duplicate or contradictory diagnostics, code-family drift, or non-deterministic output ordering across runs.

## Operational context

After CU assembly, specialised passes consume shared facts and emit typed findings. `get_diagnostics()` then applies suppression and LSP conversion. This stage changes frequently as new warning families and optimisation heuristics are added.

## Decision rules / contracts

1. **CU-first inputs**
   - Pass entrypoints should accept `CompilationUnit` (or CU-derived function facts) and avoid private lowering/SSA rebuilds.
2. **Typed findings by family**
   - Findings must carry stable diagnostic code families (`O*`, `S*`, `T*`, `IRULE*`) and precise ranges.
3. **Deterministic ordering**
   - Outputs should be stable for unchanged input to avoid diagnostic flicker and flaky integration tests.
4. **No duplicate semantics**
   - If two passes can flag the same issue shape, define canonical ownership and related-info linking rather than duplicate emissions.
5. **Shared word/value-shape helpers**
   - Passes should use shared helper modules for Tcl word/value parsing (`value_shapes.py`, `var_refs.py`) instead of embedding pass-local mini-parsers.

## File-path anchors

- `core/compiler/optimiser/` (`find_optimisations`)
- `core/compiler/taint/` (`find_taint_warnings`)
- `core/compiler/shimmer.py` (`find_shimmer_warnings`)
- `core/compiler/gvn.py` (`find_redundant_computations`)
- `core/compiler/irules_flow.py` (`find_irules_flow_warnings`)
- `core/compiler/value_shapes.py`
- `core/compiler/var_refs.py`
- `lsp/features/diagnostics.py` (pass aggregation and suppression)

## Failure modes

- Diagnostic duplication between optimiser and GVN outputs.
- Pass-specific severity assumptions leaking past diagnostics normalisation.
- Range drift from pass-local source reconstruction.
- Non-deterministic set/dict iteration surfacing as unstable finding order.

## Ownership map

- [kcs-pass-fact-ownership-matrix.md](kcs-pass-fact-ownership-matrix.md)

## Test anchors

- `tests/test_optimiser.py`
- `tests/test_taint.py`
- `tests/test_shimmer.py`
- `tests/test_gvn.py`
- `tests/test_irules_checks.py`
- `tests/test_diagnostics.py`
- `tests/test_compiler_helpers.py`


## Discoverability

- [compiler KCS index](README.md)
- [compiler architecture overview](../../compiler-architecture.md)
- [shared utility contracts](../kcs-core-lsp-shared-utility-contracts.md)
