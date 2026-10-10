# Diagnostics integration across analyser + compiler passes

Where findings from the analyser, the style checks, and the compiler passes are
aggregated, and the policy boundary between the typed producers and the one
shared policy step below them. Read this when two producers disagree about
the same finding.

The server's deep pass combines:

- the semantic analyser (`tcl_lsp_db::file_analysis`);
- the style checks (`tcl_lsp_core::source_style::style_diagnostics`);
- the compiler checks and optimiser (`tcl_lsp_db::compiler_check_diagnostics`:
  `run_all_checks` and `optimise_unit` over the memoised `CompilationUnit`);
- cross-file resolution (`tcl_lsp_db::project_diagnostics`).

`rust/tcl-lsp-server/src/lib.rs` converts each producer's output into the
shared `Finding` shape (`analyser_findings`, `compiler_findings`,
`xc_findings`, `model_findings`), and `lifted_report` — `document_policy` +
`document_report` + `lift_report` — is the one call every publish path
makes: it builds the document's policy, joins the findings with the
producers `tcl-lsp-core` owns (the source-style pass, the byte-integrity
pass, the SslicTcl projection) through `diagnostic_report::document_report`,
decides with `diagnostic_policy::apply`, and lifts the result to LSP
diagnostics. That layer is the contract boundary for code-family mapping
and suppression semantics seen by LSP clients, and it is specified in full
in [diagnostic-policy.md](diagnostic-policy.md).

## Rules

1. **Aggregation lives in the report.** Producers emit typed findings;
   `diagnostic_report::document_report` joins them with the report's own
   producers and `diagnostic_policy::apply` decides; an adapter renders
   (`diagnostic-policy.md`).
2. **Policy has one owner.** `# noqa`, `# tcl-lsp: disable=`, the five
   configuration scopes, the seed, severity, the optimiser and shimmer
   switches, overlaps and abstention are applied by `apply` alone,
   identically whatever the finding's origin or the surface.
3. **One `CompilationUnit` per edit.** The analyser tail and the compiler
   checks share the salsa-memoised unit; no consumer rebuilds its own.
4. **Ranges come from producers.** Publish the producer's span; never
   reconstruct lines and columns during aggregation.
5. **One owner per overlap.** Where two producers flag the same site, the
   dialect's overlap table decides which is canonical: W110 over an O120
   whose span holds it, and in a SslicTcl document the loader over W123,
   document-wide (`dialect_overlaps`).
6. **Values come from the lattice, not from a diagnostic.** A producer that
   needs what a command computes reads the shared `FunctionUnit` lattice,
   which the registry's value-transfer declarations feed
   ([value-transfers.md](value-transfers.md)); it never re-derives a
   command's value by name, and a disabled rule never withholds a lattice
   fact from another.

## Failure modes

- Duplicate diagnostics from overlapping pass ownership.
- Severity defaults that differ between a pass and the central mapping.
- The no-salsa fallback (`compiler_check_diagnostics_uncached`) diverging from
  the memoised path.
- A new code family added without a conversion to `Finding`.

## Anchors

- `rust/tcl-lsp-db/src/lib.rs` — `file_analysis`,
  `compiler_check_diagnostics`, `project_diagnostics`
- `rust/tcl-lsp-server/src/lib.rs` — `lifted_report`, `lift_report`,
  `document_policy`
- `rust/tcl-lsp-core/src/diagnostic_policy.rs`,
  `rust/tcl-lsp-core/src/diagnostic_report.rs` — the policy step every
  surface calls
- `rust/tcl-compiler/src/analyser/` — semantic warning production
- `rust/tcl-compiler/src/compiler_checks.rs` — `run_all_checks`
- `rust/tcl-lsp-server/tests/e2e/` — the diagnostic end-to-end suites

## Related

- [diagnostic-policy.md](diagnostic-policy.md) — the policy step below
  every surface: the report every rule above hands to `apply`, and the
  adapters that render it for the LSP, the CLI, the MCP tools and the
  code actions
- [downstream-pass-contracts.md](downstream-pass-contracts.md)
- [async-diagnostics-tiering.md](async-diagnostics-tiering.md)
- [compilation-unit-contracts.md](compilation-unit-contracts.md)
