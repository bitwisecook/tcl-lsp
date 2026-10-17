# Sol-6: member reference census V3

**Patch:** `member-census-v3.patch`
- Built as `diff -ruN a3 b3`, with the path headers rewritten to `a/`/`b/`. It touches 9 files.
- `git -C /home/user/tcl-lsp apply --check` passes at HEAD `bb7277d82`.
- None of the 9 files changed between `84cf7c884` and HEAD. The `a3/` copies are byte-identical to HEAD.
- Every changed `.rs` file is `rustfmt --edition 2024` clean (checked on stdin). The patch adds no `#[allow]`.

**Static only.** Nothing was compiled or run. The only commands were small `nice tclsh` probes. Itcl and snit are not installed for tclsh8.6 here, so `probe3.tcl` could not exercise them.

The V2 design, its fail-closed table and its risks are still described in `NOTES.md`. This file lists only what V3 adds in response to Luna-8's `REVIEW.md` (HOLD).

## Changes from V2

### H1: substitution templates (`original_declaration.rs`, `MemberCensus::classify`)
- A global command now leaves the census incomplete when either:
  - it carries `Traits::PERFORMS_SUBSTITUTION`; or
  - its selected `semantics.body_execution` is `BodyExecutionSpec::SubstitutionTemplate`.
- Both are registry data, and `subst` is the only command that has them.
- New fail-closed cases, each tested on tcl8.6 and tcl9.0:
  - `subst {[my p\x69ng]}`
  - `set s [string cat pi ng]; subst {[my $s]}`

### H2/H3: namespaced implementation spellings, interim fix
- A global command is now incomplete when its selected root descriptor has `implementation_namespace: Some(_)`. That covers:
  - `::oo::Helpers::{next, nextto, link, callback, mymethod, self, classvariable}`
  - `tcl::dict::*`
- New fail-closed cases:
  - `Child {superclass Base; method ping {} {::oo::Helpers::next}}`, for `Base::ping`, on tcl8.6 and tcl9.0.
  - `::oo::Helpers::nextto Base` (tcl8.6).
  - `constructor {} {::oo::Helpers::link [list set [string cat pi ng]]}; method body {} {set a 1}` (tcl9.0).
- **Follow-up (not done):** the fuller fix is to keep `TCLOO_NEXT_CHAIN` and `TCLOO_BINDS_METHOD_ALIAS` on the qualified twins.
  - `oo_helpers.rs` `QUALIFIED_SPELLING_EXCLUDED` would then strip only `TCLOO_METHOD_CONTEXT` and `TCLOO_REQUIRES_METHOD_FRAME`.
  - Before that, the consumers of those traits need an audit: `references.rs`, `call_hierarchy.rs`, `definition.rs`, `analyser/oo.rs` and `original_linked_member.rs`.
  - Once that lands, the interim check can be narrowed. `oo_helpers.rs` is unchanged here.

### Class-family control
- New test: `readonly_member_census_stays_partial_for_other_class_families`.
- It covers an itcl class that calls `ping` by its bare name, and a snit type that calls it through `$self ping`. Both are analysed as tcl8.6.
- For every method identity that `declarations()` yields, the report must not be `CompleteSourceOccurrenceCensus`. Both shapes fail closed: the itcl call is an unknown head, and the snit call is a dynamic head.
- The test passes vacuously if the analyser yields no method identities for either family.

### Registry doc and test (`resolved_invocation.rs`, `selected_script_timing.rs`)
- The doc comment on `authored_source_retains_callback` now says:
  - deferred script-valued options (Tk `-command`) are Body roles;
  - checked syntax coverage walks them when they are literal and refuses them when they are dynamic;
  - so any consumer must also require that coverage.
- The registry test now asserts only `authored_source_self_dispatch` for the `my` rows. Luna noted that the callback answer for an unlisted `my` selector depends on how its subcommand resolves. The callback rows (`set`, `proc`, `after`, `namespace code`, `trace add`, `string $x $y`, `lsort -command`) are unchanged.

### Server: the same guard on both lens paths
- New helpers in `readonly_reference_reports.rs`:
  - `is_source_census(coverage)`
  - `single_document_coverage(coverage, retained_documents)`, which downgrades Complete to Partial unless exactly one document is retained.
- The resolved report uses `single_document_coverage(report.coverage(), retained.len())`. Its behaviour is the same as in V2.
- The unresolved lens tag in `lib.rs` `code_lens` (around line 33027) now goes through the same helper.
  - `original_declaration_documents(&uri).len()` is computed only when some lens claims a census. Otherwise the count is taken as 1.
  - Resolved and unresolved lenses therefore agree.
  - If the inventory is unavailable (an empty vector), the tag degrades to `partial`.
- The test `readonly_member_census_stays_partial_beside_other_documents` now also checks that both lens tags read `"partial"` when two documents are open.

### Contract doc (`original-method-candidates.md:18`)
The completeness conditions now also require:
- no substitution template;
- no namespaced implementation spelling.

The server's single-document scope now names both the unresolved and the resolved lenses.

## Test expectation changes (unchanged from V2)
- **test-096:** only the coverage assertion changes, `Partial` → `CompleteSourceOccurrenceCensus`.
- **Server `readonly_member_wire_retains_owner_and_partial_coverage`:**
  - coverage → `CompleteSourceOccurrenceCensus`;
  - the tags `"partial"` → `"complete-source-census"` (×2);
  - the title `"at least 2 references"` → `"2 references"`.
- Its `original_declaration_reference_locations(...).is_empty()` assertion is not touched. The integration37 follow-up `i37/fix4b/my-edges-hierarchy-only.patch` restores it.

## Expected flips and remaining risks
These are as in `NOTES.md`, with the following updates:
- **NOTES.md Risks 5 is withdrawn.** It claimed qualified `oo::Helpers::next` was harmless because it names no member; it is now fail-closed.
- **New conservative loss.** Any `tcl::dict::*` qualified call and any `subst` in the document keep the census Partial.
- **Still unbuilt.** The order Luna-8 gave remains:
  1. the registry test, test-096, the server wire test, the new core and server tests;
  2. code_lens 056–060 and admission26;
  3. the §5 blast-radius groups (rename_safety, namespace_rename, call_hierarchy::logical_source, minify, server logical hierarchy).
- **Zero-count clickability (Luna §3).** A Complete census with 0 locations is still non-clickable. The e2e tests `test_method_lens_zero_when_uncalled` and `test_property_lens_zero_when_unused`, and the server test `code_lens_resolve_for_unused_method_is_clickable_with_zero_locations`, may still fail on the command id. This patch does not change that.
