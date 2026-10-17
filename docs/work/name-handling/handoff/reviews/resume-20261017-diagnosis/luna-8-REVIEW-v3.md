# Luna-8 delta review: member census V2 → V3 (`member-census-v3.patch`)

## Verdict

**ADMIT (static).** V3 closes the three soundness holes that caused the earlier
HOLD: H1, H2 and H3. Each fix uses registry data only, and each fail-closed test
is a real test of its hole rather than a vacuous one.

Admission depends on building and running the patch, in the order given in
REVIEW.md §"Required before ADMIT" items 2 and 3.

Non-blocking follow-ups:
- F1: the census is now incomplete for every `dict` call;
- F2: the itcl/snit control can pass vacuously;
- F3: the code-lens request now does more work and can fail where it did not
  before.

All of this is static. Nothing was compiled or run. The only commands run were
`rustfmt --check` on stdin and the earlier tclsh probes.

## Applies to HEAD

- `git apply --check` is clean at `bb7277d82`.
- Rebuilding from HEAD gives copies byte-identical to Sol's: my `a3/` matches
  HEAD, and `b3/` equals `sol-6/b3/` for all 9 files.
- Compared with V2, three files are byte-identical in V3:
  `source_scoped_body.rs`, `source_structure.rs` and `tcl-registry/src/lib.rs`.
- The V2→V3 delta is in `v2-v3.diff`, next to this file.

## H1, H2, H3: closed

All the checks below are in `original_declaration.rs` `MemberCensus::classify`,
around lines b3:1156–1177.

**H1.** A command is now treated as incomplete when it has
`traits.contains(PERFORMS_SUBSTITUTION)` or when
`semantics.body_execution == Some(BodyExecutionSpec::SubstitutionTemplate)`.
- Both facts are registry data. Only `subst` carries them (`subst_.rs:286` and
  `subst_.rs:308`).
- The fields check out: `InvocationSemantics::body_execution` is an
  `Option<BodyExecutionSpec>` (`resolved_invocation.rs:1180`), and
  `tcl_registry::body_execution` is a public module.
- The `traits` binding is already in scope from V2.

**H2 and H3.** A command is now treated as incomplete when
`authored_source_descriptors().command.implementation_namespace.is_some()`.
- This covers the qualified `::oo::Helpers::*` spellings (`oo_helpers.rs:131`).
- It is registry data, not a check on command names.

**The new tests exercise the holes.** They are in
`readonly_member_census_fails_closed_for_deferred_and_unproven_dispatch`:
- `subst {[my p\x69ng]}` and `set s [string cat pi ng]; subst {[my $s]}`, each on
  tcl8.6 and tcl9.0;
- `::oo::Helpers::next` in an overriding `Child::ping`, on 8.6 and 9.0;
- `::oo::Helpers::nextto Base` on 8.6;
- `::oo::Helpers::link [list set [string cat pi ng]]` on 9.0.

These are exactly the shapes I probed with tclsh. In V2, none of them was caught
by any other rule:
- none has a literal `ping` mention;
- none has an unknown or dynamic head;
- none has a callback trait.

So each new assertion is non-vacuous, as long as the base shape can reach
Complete. That is the same precondition as every fail-closed case, and the
positive controls cover it.

`member_report` selects the first class that declares `ping`. Declaration order
makes that `Base`, so the next/nextto cases test the `Base::ping` identity, as
intended.

The doc comment on `classify` and the contract text at
`original-method-candidates.md:18` now include both new conditions.

## Server guard: consistent on both lens paths

`single_document_coverage(coverage, n)` downgrades a census unless exactly one
document is retained (`readonly_reference_reports.rs`).
- The resolved report uses `retained.len()`. Its behaviour is the same as V2.
- The unresolved lens tag (`lib.rs` around line 33025) now gets its count from
  the same `original_declaration_documents(&uri)` inventory, so the two paths
  agree.
- An unavailable inventory has length 0, so its tag becomes `partial`.
- The count is computed only when some lens claims a census.
- Visibility and error conversion are fine: both helpers are `pub(super)`, and
  `?` uses the same `From` conversion as the existing `read_document(...).await?`
  in `code_lens`.

The new server assertions in `readonly_member_census_stays_partial_beside_other_documents`
check both the unresolved and the resolved tag for `"partial"` with two
documents. This is non-vacuous: without the change, the unresolved tag would
read `complete-source-census`.

## No other assertion changes

The only `-` assertion and coverage lines in the whole V3 patch are V2's agreed
set:
- test-096 coverage;
- the server wire test: coverage, the `"partial"` tag (×2), and the
  `"at least 2 references"` title.

The registry test dropped only the `my` callback expectations, as I suggested.
The non-`my` callback rows are unchanged. The new `my` rows assert only
`authored_source_self_dispatch`, and the non-`my` rows now assert `None` for it.

## Format and style

- All 8 patched `.rs` files pass `rustfmt --edition 2024 --check`. That includes
  the 60k-line server `lib.rs`, which also passes at base.
- No `#[allow]` or `expect(clippy…)`.
- Comments are plain and use UK spelling, with no process names.

## Follow-ups (non-blocking)

**F1 (medium, lost usefulness, not soundness).** The bare `dict` spec itself has
`implementation_namespace: Some("::tcl::dict")` (`commands/tcl/dict.rs:891`). The
same is true of the qualified `::tcl::dict::*` specs (`dict.rs:1137`).
- So any `dict …` call anywhere in a document keeps the census Partial.
- NOTES-v3 says only qualified `tcl::dict::*` calls are affected. That is
  inaccurate, and the contract phrase "namespaced implementation spelling"
  undersells it.
- The target tests are unaffected: 056–060, test-096 and the wire test use no
  `dict`.
- Fix it with the planned registry change: keep `TCLOO_NEXT_CHAIN` and
  `TCLOO_BINDS_METHOD_ALIAS` on the qualified `oo::Helpers` twins, then drop the
  interim `implementation_namespace` rule. Until then, correct the NOTES and
  contract wording.

**F2 (low).** `readonly_member_census_stays_partial_for_other_class_families`
passes vacuously in two cases:
- when `declarations()` is `None`;
- when it yields no `Method` identities for the itcl or snit sources.

Nothing checked statically shows that the analyser makes method identities for
`itcl::class` or `snit::type` under tcl8.6 with `package require`. Once built,
count the identities checked per source and assert the count is above zero. If a
family turns out to yield no identities, drop it with a comment.

**F3 (low).** `code_lens` now calls `original_declaration_documents`, which
analyses every workspace and open participating document. It does so only when a
census lens exists.
- This adds latency, although the analyses are cached.
- A document read error now fails the whole `textDocument/codeLens` request
  through `?`, where it previously could not.

Consider degrading to `partial` on error instead of propagating it.

**Carried from REVIEW.md, unchanged:**
- A Complete census with 0 locations is still non-clickable.
- The blast-radius test groups in REVIEW.md §5 still need to be run.
- Performance: one census per lens.
