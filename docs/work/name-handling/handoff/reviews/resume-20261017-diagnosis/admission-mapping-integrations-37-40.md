# Admission mapping: integrations 37–40 (Luna-14)

This is a proposal only. Nothing under `/home/user/tcl-lsp` was modified, and no cargo or test command was run. The machine-readable form is `admission-mapping.json`, which sits beside this file. It quotes every catalogue row verbatim and pins the SHA-256 of the catalogue, annex and audit-status files it was built from.

## Result

**No catalogued unit is newly implemented, and no partial unit becomes full, in the source of 60051c3c2, 1433f2ffe, e46e88617, 5a8dfde48, a6e91d0a6 and d0cd9d73b.**

| Count | Before | Delta | After |
| --- | --- | --- | --- |
| Original LSP | 290/325 (228 full + 62 partial) | 0 | 290 + 0 = 290 (228 + 0, 62 + 0); 35 deferred BIG-IP unchanged |
| Baseline | 570/2158 (405 full + 165 partial) | 0 | 570 + 0 = 570 (405 + 0, 165 + 0); without integration 2158 − 570 = 1588 |
| Annex | 70/71 | 0 | 70 + 0 = 70; 1 still blocked |
| Effective | 640/2229 | 0 | 570 + 70 = 640 of 2158 + 71 = 2229; without integration 1589 |
| Verified complete | 0 | 0 | 0 |

The 25 existing Studio NONE rows are untouched.

## Why the result is zero

- **Only an uncounted row can raise the integrated totals.** The original LSP area has exactly 35 uncounted rows, all deferred BIG-IP. These pieces touch none of them. Every LSP-area row they touch is already counted.
- **None of the partial rows they touch closes.** Each one keeps a recorded open obligation that these pieces do not discharge:
  - **lsp-codelens-logical-source-identity:** captured and foreign-class census.
  - **lsp-server-logical-hierarchy-wire-currency:** foreign, inherited, anonymous and captured closure.
  - **lsp-server-code-lens-wire-currency:** unsupported candidate coverage.
  - **lsp-server-workspace-string-command-navigation:** string links and report popup.
  - **compiler.source.typed-member-kind-hierarchy:60:** MRO and physical dispatch.
  - **db.unit.db.project-source-role-hints:** blocked by the annex deferred-storage owner.
  - **db.unit.db.workspace-class-display-cards:** typed receipts not carried end to end.
- **The uncounted candidates were checked and rejected.** Reasons, in brief:
  - **`compiler.source.selected-definition-member-body:6436:object-body`:**
    - Integration 18 set a precedent: oo_objdefine counts once, under `db.unit.registry.tcloo-definition-source-body-layout`, with a separate count of 0.
    - The inline path still does not use the `OriginalSourceDefinitionMemberRegion` plan.
  - **`…:6436:inline-definition`:** this row is for `oo::define`, which no piece touches.
  - **`compiler.owner.source.definition-member-body-disposition:843`:** the tri-state disposition predates 2a3928bc6. Integration 38 only adds `checked_member_row`.
  - **`compiler.source.logical-selected-class-member-ledger:402`:** `source_class.rs` gains only a test.
  - **Annex `compiler.source.selected-definition-deferred-storage-effect-coverage-owner`:** integration 40 records uncertainty and closes no effects. This annex row stays blocked.
  - **`compiler.source.original-rewrite-statement-extent:61` and `audit:simulator-script-list-source-host-facet`:** both are unrelated to the nested memo.

## Per piece

Every piece maps to zero units.

| Piece | Commit | Kind | Existing rows touched (status unchanged) |
| --- | --- | --- | --- |
| db042 structural class factory | 60051c3c2 | causal repair (precedent: admission 34) | db.unit.db.project-source-role-hints (partial); db.unit.db.workspace-class-display-cards (partial) |
| Roster test setup | 60051c3c2 | test-only setup | — |
| Inline objdefine member (analyser) | 60051c3c2 | causal repair of the admission 22–24 regression (Sol G2) | db.unit.registry.tcloo-definition-source-body-layout (full) |
| Direct `my` call-hierarchy edges | 60051c3c2 | coverage addition | lsp-server-logical-hierarchy-wire-currency (partial) |
| Unknown-command debt on edges | 60051c3c2 | causal fail-closed repair | lsp-original-call-hierarchy-geometric-fallback (full) |
| TclOO-faithful nextto | 60051c3c2 | behaviour correction, still keyed on the report map | compiler.source.typed-member-kind-hierarchy:60 (partial) |
| Nested body cut memo | 60051c3c2 | performance memo / helper | — |
| `my` edges scoped to the hierarchy | 1433f2ffe | causal regression repair | — |
| tclsh test corrections | 1433f2ffe | test-only (user decisions at 20:30Z) | lsp.validation.20.00.fn3 / fn13 (full); see notes |
| Lifecycle next-chain references | 1433f2ffe | coverage addition | lsp-codelens-logical-source-identity (partial); typed-member-kind-hierarchy:60 (partial) |
| Member census V4 | 1433f2ffe | strengthens a post-baseline owner, plus the G1 repair and Registry helpers | lsp-codelens-logical-source-identity (partial); lsp-server-code-lens-wire-currency (partial); lsp-code-actions-package-presence-census (full); lsp-definition-member-unavailable-vs-nonmember (full) |
| Same-document order V5 | e46e88617 | stronger issuer inside a counted purpose | lsp-source-run-order-purpose (full); lsp-server-workspace-string-command-navigation (partial) |
| Inline objdefine own-object producer | e46e88617 | layout extension of a post-baseline producer | db.unit.registry.tcloo-definition-source-body-layout (full) |
| tcl-dialect doc lint | e46e88617 | lint | — |
| Clippy cleanup | 5a8dfde48 | lint-only, plus `include_str!` source copies | — |
| TclOO definition-body namespace advice | a6e91d0a6 | precision correction | — |
| v6 added, then withdrawn | a6e91d0a6 + d0cd9d73b | net-zero source | — |

## Bookkeeping notes (no count effect)

- **Stale implementation text on two validation rows.** The rows are `lsp.validation.20.00.fn3` and `lsp.validation.20.00.fn13`. Their text says "All original fixtures/assertions/budgets retained", but their expected title is now "References unavailable". That change is a user-authorised exception (20:30Z decision `non_tcl_tests`), and it should be recorded against these rows.
- **Partial-to-Complete assertion moves.** Core test-096 and the Server readonly-member wire test moved from Partial to Complete. This was authorised by the 20:30Z decision `method_census_contradiction` and is not unit credit.
- **Row `:843` needs its own reconciliation.** It remains NONE although the typed tri-state disposition exists in source. Those integrations earn no credit for it.
- **Integration 41 is out of scope.** c0437decf is source, not a docs-only commit, and it is outside capture41.
