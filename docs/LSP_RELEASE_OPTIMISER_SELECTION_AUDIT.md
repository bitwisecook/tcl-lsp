# Optimiser selection audit: #2426

This source audit starts at
`010452b74b79db2fe2ff0412b7259e4cb8c9bc1f`. The next candidate is local and
unverified. No heavy build or Tcl execution was run for this work.

The candidate is parked for priority #940 work in stash
`e2fd5783914859c032ff4aebc66a4dbf15b790b9`; the patch and pending measurement
receipt remain at `/tmp/lsp-2426-optimiser-selection-candidate.patch` and
`/tmp/lsp-2426-optimiser-selection-evidence.json`. No workload counts are
available and no product change from this candidate has landed.

## Existing facts and equality

`optimiser::Optimisation` retains typed code, source span, group partition and
hint status, plus rendered message and replacement text. Its private issued
batch is absent on raw rows. No per-operation identity or original IR/AST
occurrence travels with each raw candidate. `PassId` belongs to the dispatcher,
but is not a retained candidate identity.

| Consumer | Equality or ownership purpose |
|---|---|
| `optimiser/mod.rs::Optimisation` | Public Eq/Hash covers the six visible payload fields, including message and replacement, excluding the private issued batch. |
| `OriginalRawOptimiserReport::eq` | Exact member vector plus prepared worker unit/origin. |
| `OriginalBodyOptimiserReport::eq` | Exact member vector plus neutral capsule, dialect, issuance and analysis context. |
| `OriginalOptimiserRewriteBatch` / private `OptimisationPayload` | Whole final source/input/registry/realm/origin plus exact ordered payloads; issued after overlap arbitration. |
| `original_rewrite_batch` / `is_original_rewrite_group` | Exact payload membership and complete same-batch groups. Changing wording or an edit cannot borrow the old receipt. |
| DB `CheckSolve::eq` and `CompilerDiagnostics::eq` | Visible optimisation equality plus separate original-provenance comparison for Salsa early cutoff. |
| Core `FindingData::IssuedOptimiser` / `Finding` | Derived equality includes rendered output and the boxed optimisation. |

The scoped compiler/Core/database/server search found no production
`HashSet<Optimisation>` or `HashMap` keyed by `Optimisation`. Group, source-span,
code and body-ID collections are separate typed keys. Tests also compare whole
optimisation vectors for memo/whole-unit parity. Global Eq/Hash and sealed
payload equality remain unchanged: removing message there could suppress
display updates or weaken receipt integrity.

## Narrow candidate policy

The current canonical sorter orders messages before replacement text, then
the overlap selector preserves that order for equally ranked rewrites. Thus
two rows with the same code and span can choose different edits when only
their wording changes. Replacement sorting or hash-map emission order is not
a substitute for missing operation identity.

The local candidate withdraws every actionable class containing more than one
row with the same typed code and span. This is an unresolved arbitration rank,
not a claim that the rows denote the same operation. It also withdraws every
member of each affected all-or-nothing group, including hint members. Standalone
hints and unique typed ranks remain available.

Withdrawal happens before canonicalisation of actual worker output, before
raw/body report equality can observe it; it also runs at whole-manager ingress
for assembled or manually supplied vectors, and after tail-generated candidates.
No message, replacement or global row Eq/Hash participates in this decision.
Canonical presentation ordering remains separate from choosing an executable
rewrite. The policy intentionally also declines identical duplicate edits and
duplicate zero-width insertions when no operation identity distinguishes them.

Four full-manager controls are prepared for equal/different/swapped wording,
equal and different replacement payloads, zero-width duplicates, affected group
mates, and unique/higher-ranked positive controls. They use one retained source
unit per test. Manufactured proposals test selection policy only; they do not
claim that a production pass emits the proposal or grants it source authority.

A separate fixed workload probe prepares actual retained analysis inputs and
runs all passes before withdrawal: 12 core Tcl source cases under Tcl 8.4, 8.5,
8.6, 9.0, 9.1 and Jim (72 cells). It reports raw/actionable rows, unresolved
classes, affected groups and rows withdrawn. At least one actual actionable
worker row is required; individual zero-row cells remain visible. This is Rust
implementation measurement, not Tcl execution evidence.

**No impact counts are available yet.** Do not land the candidate until the
workload has run and common-duplicate losses have been reviewed. Formatting and
`git diff --check` pass; focused tests, production Clippy and full gates are
unrun. #2426 remains open. No external comment or closure was made.
