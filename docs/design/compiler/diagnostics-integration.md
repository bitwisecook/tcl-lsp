# Diagnostics integration across analyser + compiler passes

Where findings from the analyser, the style checks, and the compiler passes are
aggregated, and the policy boundary that fixes severity, code family, ranges,
and suppression. Read this when two producers disagree about the same
finding.

The server's deep pass combines:

- the semantic analyser (`tcl_lsp_db::file_analysis`);
- the style checks (`tcl_lsp_core::source_style::style_diagnostics`);
- the compiler checks and optimiser (`tcl_lsp_db::compiler_check_diagnostics`:
  `run_all_checks` and `optimise_unit` over the memoised `CompilationUnit`);
- cross-file resolution (`tcl_lsp_db::project_diagnostics`).

The lifts in `rust/tcl-lsp-server/src/lib.rs` (`lift_analyser_diagnostics`,
`lift_source_style_diagnostics`, `lift_compiler_diagnostics`) convert typed
findings to LSP diagnostics, and `finalise_diagnostics` attaches tags and
applies severity overrides. That layer is the contract boundary for
code-family mapping and suppression semantics seen by LSP clients.

## Rules

1. **Aggregation lives in the lifts.** Passes emit typed findings; conversion
   and final policy mapping happen centrally.
2. **Suppression is uniform.** `# noqa`, `# tcl-lsp: disable=`, and disabled
   codes apply identically whatever the finding's origin.
3. **One `CompilationUnit` per edit.** The analyser tail and the compiler
   checks share the salsa-memoised unit; no consumer rebuilds its own.
4. **Ranges come from producers.** Publish the producer's span; never
   reconstruct lines and columns during aggregation.
5. **One owner per overlap.** Where two producers flag the same site, one is
   canonical at the LSP boundary — W110 over O120 (`suppress_duplicate_o120`).

## Failure modes

- Duplicate diagnostics from overlapping pass ownership.
- Severity defaults that differ between a pass and the central mapping.
- The no-salsa fallback (`compiler_check_diagnostics_uncached`) diverging from
  the memoised path.
- A new code family added without a lift.

## Anchors

- `rust/tcl-lsp-db/src/lib.rs` — `file_analysis`,
  `compiler_check_diagnostics`, `project_diagnostics`
- `rust/tcl-lsp-server/src/lib.rs` — the lifts, `finalise_diagnostics`,
  `line_suppressed`
- `rust/tcl-compiler/src/analyser/` — semantic warning production
- `rust/tcl-compiler/src/compiler_checks.rs` — `run_all_checks`
- `rust/tcl-lsp-server/tests/e2e/` — the diagnostic end-to-end suites

## Related

- [downstream-pass-contracts.md](downstream-pass-contracts.md)
- [async-diagnostics-tiering.md](async-diagnostics-tiering.md)
- [compilation-unit-contracts.md](compilation-unit-contracts.md)
