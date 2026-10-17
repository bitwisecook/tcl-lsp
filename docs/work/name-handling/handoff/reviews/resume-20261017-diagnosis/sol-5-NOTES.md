# Sol-5: same-document source order (decision `export_order`)

Patch: `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/sol-5/same-document-order-v1.patch`
(`diff -ruN a b`). The `a/` copies are byte-identical to HEAD `60051c3c2` ("Integration37
candidate"), which committed the earlier working-tree `definition.rs` edits. The patch's
sha256 is recorded in the final report.

Status:
- `git -C /home/user/tcl-lsp apply --check` passes.
- `rustfmt --edition 2024 --check` is clean on all four Rust files. It ran on scratch copies,
  so the patch parses.
- Not compiled and not run. The constraints rule out cargo and test binaries, so every
  expected flip below is a prediction.
- tclsh 8.6 and 9.0 probes (`scratchpad/agents/sol-5/probe.tcl`) agree with every Tcl claim
  the patch makes:
  - `namespace export p; namespace export -clear` gives an empty export list.
  - A forget leaves `info commands ::dst::*` empty.
  - `proc ::d2::p` after `import -force` gives `LOCAL2`, and `namespace origin` becomes `::d2::p`.
  - A `-clear` inside an `if` branch that is not taken leaves `q` exported.

## Design

Files: `rust/tcl-lsp-core/src/{source_graph,namespace_import,definition,workspace_index}.rs`
and `docs/design/analysis/import-order-source-graph.md` (new §8.1).

1. **New purpose** `SourceNavigationOrderPurpose::DocumentSequence`.
   - `modeled_source_exclusion` now admits it in the same way as `Constructed`.
   - `Conditional` is unchanged.
   - `has_run` and `cmp_run` still return `None` for workspace navigation.

2. **`DocumentSequence` inventory**, private to `source_graph.rs` and held in
   `SourceOrderContext.sequence`.
   - It is built in `SourceOrderContext::capture` from the same `SourceStructure` (scripts and
     commands) and the same `source_registry_words` as the sites.
   - The words are cached per command index, so each command is resolved only once and the
     site loop reuses it.
   - **Straight regions:** the document root, plus every body content span that a command in
     a straight region enters, under three conditions:
     - the command has validated words and complete roles;
     - its `script_body_flow_for_invocation` is `Sequence` or `ConcatenatedScript`. In Tcl
       that means `namespace eval`, `eval`, `uplevel` or `inscope` with a single complete
       script word, which `source_script_bodies` already requires for concatenating schemas;
     - `original_definition_body_span(body.start)` is `None`.
   - **Sequence position `at`:**
     - At or past the document end, `at` is always a sequence position. All straight
       statements ran before the end, so a query at `source.len()` or `u32::MAX` is known.
     - Otherwise, the innermost retained script region containing `at` must be straight.
     - Inside that region, the innermost command containing `at` (if any) must admit it in
       one of two ways:
       - `at` is in the command word, which is the call position even for an unresolved user
         command; or
       - the command has validated words with complete roles, and `at` is not inside any
         operand whose role may carry a script (`Body`, `Expr`, `LambdaLiteral`,
         `OpaqueScript`, `CommandPrefix`).
     - The operand-role rule means a script nested in a command but missing from the
       inventory can never take that command's position. A point between commands of a
       straight region is admitted.
     - An operand's extent is the union of its written token span and its whole native-word
       span. Span ends are exclusive, so a point just past a body's closing brace is not
       inside that operand.
   - **Never sequence positions:** procedure, method and class bodies, callbacks,
     `if`/`switch` branches, loop, `catch` and `try` bodies, command and expression
     substitutions, and argument words of unresolved commands.

3. **`RunOrder::navigation_precedence` / `navigation_comparison`.**
   - When both points share a URI and both are sequence positions of that URI's indexed
     context, they return `DocumentSequence` with:
     - `in_effect_within(e, q, None)` for precedence;
     - `load_order(_, false)` ordering for comparison.
     These are the relations the constructed model gives at load level.
   - Every other pair falls through to the existing Conditional or `None` path without change.
     This covers cross-URI pairs, unindexed points, deferred bodies and branches.
   - Supplied `enclosing_body` values are ignored; the retained inventory decides.

4. **Removal guard**, in `namespace_import.rs`, as
   `NamespaceStatementPurpose::removal_in_effect`.
   - On workspace navigation (`RunOrder::is_source_navigation`), a removal counts as in effect
     at the query only when its precedence is *known* to be true. Removals here are `-clear`
     tombstones and `AliasEventKind::Remove` events: forget, redefinition and deletion.
   - Without this guard, a known event-to-event order plus the abstain-to-true query gate
     would revoke for a body-local or cross-document query. That would drop candidates where
     the removal's timing against the query is unknown, which the guard forbids.
   - Patterns and installs keep the lenient gate.
   - Constructed models and SourceHorizon go through the old `in_effect`, unchanged.
   - Before this patch, no removal ever applied on source navigation. The guard therefore
     only ever *adds* removals, and only between straight-line positions in the same document.

5. **Unchanged:**
   - every call site of `ran_after`, `in_effect_at`, `definition::in_document_source_order`
     and the workspace folds; only doc comments change;
   - the `SourceHorizon` advice;
   - the package-bound and source-edge algorithms;
   - the legacy constructed `RunOrder::build` tests in `namespace_import::tests` and
     `source_graph::tests`.

## Control-test changes (justified by decision `export_order`)

The decision allows the 1b45c540e control to change only for the same-document shape and
requires its cross-document intent to stay. Commit 1b45c540e added three controls that
assert the same same-document shape, and all three would flip from pass to fail, so the
patch updates all three in the same way.

| control | same-document change | conditional answer kept by |
|---|---|---|
| `workspace_index::original_source_order_transport_tests::workspace_order_keeps_original_export_evidence_without_conditional_clear_completion` (the one the decision names) | purpose `DocumentSequence`, `modeled_source_exclusion() == Some(true)`, verdict `NotExported`; `has_run` still `None` | **new** `workspace_order_keeps_cross_document_clear_conditional`: pattern in `lib`, `-clear` in a second document; relation `None` or `Conditional` with no exclusion; verdict `Exported` |
| `source_graph::original_source_navigation_order_tests::original_workspace_order_retains_conditional_purpose_without_tombstone_authority` | purpose/exclusion assertions read `DocumentSequence`/`Some(..)`; the existing `exported`/`alias_live` assertions at query `after` are **unchanged** (the clear at the query point has not run yet); adds end-of-document revocation | same test: second indexed document `other` whose `-clear`/removal revokes nothing |
| `definition::original_document_source_order_tests::document_export_adopter_keeps_conditional_clear_and_missing_input_unknown` | verdict `NotExported` for the top-level clear | same test: `if {$x} {namespace export -clear}` keeps `Exported` (also asserts both exports are recorded); missing-input assertions unchanged |

These need sign-off. The decision text names only the WorkspaceIndex control. The other two
come from the same commit and contradict the decision in exactly the same way. The test names
stay as they are, so selection manifests such as cp09's 1024-test vector still match. Each
name stays accurate because each test still contains a conditional clear.

A new guard test,
`source_graph::original_source_navigation_order_tests::document_sequence_orders_straight_line_bodies_and_withholds_branches`,
checks three things:
- `namespace eval` body points are `DocumentSequence`;
- `if`-branch and `proc`-body points are not;
- the document end is known, and a foreign URI gives `None`.

## Expected flips (predicted, unexecuted)

**Fail → pass, confirmed red in cp09 `partial-lsp35r2`:**
- test-400 `workspace_index::original_namespace_export_index_tests::original_export_snapshot_joins_opaque_slots_and_rejects_ui_and_policy_substitutes`.
  Everything is top-level. The query is the document end. The `-clear` tombstone follows
  `p\uD800`'s pattern, so the verdict is `NotExported`; `p\uD801` comes after the tombstone,
  so it is `Exported`.
- test-404 `definition::tests::a_forget_after_the_import_stops_resolving_the_alias`.
  The forget follows the import inside two `namespace eval dst` bodies, and the query is
  `u32::MAX`.
- test-406 `definition::tests::a_redefinition_ends_a_forced_import_shadow`.
  - `between` falls between commands of the import's `namespace eval dst` body, so the
    `LOCAL2` declaration is known not yet in effect.
  - `after` falls after the `LOCAL2` proc, so the declaration is in effect and follows the
    install.

**Same-file workspace tests, red in luna's full `workspace_index` run** (agents/luna-workspace/wi-all.txt):
- Expected to pass if nothing else blocks them: `a_same_file_forget_after_the_import_still_revokes`,
  `an_exact_import_link_dies_on_a_same_file_forget`,
  `resolve_wildcard_import_ignores_a_same_file_export_written_after_the_import`,
  `a_top_level_call_before_its_own_import_does_not_resolve`,
  `exact_import_ignores_an_export_written_after_it`,
  `a_top_level_import_is_not_conflicted_by_a_later_top_level_one`,
  `a_glob_import_conflicts_with_a_later_exact_import_of_the_same_name`,
  `an_exact_import_conflicts_with_a_later_glob_import_of_the_same_name`.
- Some of these may also hit B3 (empty cross-document settlement). The ones that go through
  `resolve_command_target` or `workspace_command_exists` can stay red for that reason.

**Definition and references tests that need known same-document removal or order:**
- These were not in the cp09 subset. They are red at HEAD by the same mechanism as 404 and
  are expected to pass: `a_glob_forget_removes_every_matching_alias`,
  `a_dynamic_forget_pattern_revokes_nothing` (literal control half),
  `a_simple_forget_pattern_removes_the_alias_too`,
  `a_forget_lets_the_next_unforced_import_install`,
  `a_forced_import_before_the_call_does_not_shadow_a_later_call_site`,
  `the_tail_match_refuses_a_forgotten_import_too`,
  `a_forget_in_the_middle_of_a_chain_kills_the_whole_chain`,
  `deleting_the_source_command_kills_the_alias`,
  `a_local_defined_after_the_import_is_not_a_conflict_but_does_end_the_alias`,
  `export_clear_before_the_import_does_revoke`,
  `export_clear_with_patterns_on_the_same_call_keeps_only_the_new_ones`,
  `original_document_export_query_uses_retained_slot_after_reporting_rows_are_removed`
  (the definition-tier twin of test-400), and
  `references::…::references_drop_a_call_after_a_namespace_forget`.
- Their current status is not in any capture I could read. Confirm in the next capture.

**Stay red by design** (body-local straight-line order is withheld by the guard):
- `a_body_local_call_before_an_import_in_that_same_body_does_not_resolve`,
  `a_later_import_in_the_same_body_has_not_run_yet`,
  `a_body_local_import_conflicts_with_a_later_top_level_import`.
- Turning these green needs a separate decision on same-body sequence.

**Unaffected cross-document groups:**
- B1 tests 389, 390, 392, 516 and 517 stay red for their own reason, which is empty
  `resolved_source_edges`.
- Their intended answers are cross-document and stay Conditional. Test 517 in particular
  still expects `Some("::mymod::helper")` despite a same-file forget: the forget's position
  is known, but the install is in another document, so `after` stays `None`.

**Pass → fail risk checked:**
- Only the three controls above flip. The patch updates them.
- I traced these passing tests by hand, and they stay green:
  - `exact_import_survives_a_later_export_clear` and
    `resolve_wildcard_import_survives_a_later_export_clear_in_the_import_file`: the clear
    after the import is known *not* in effect.
  - `import_site_snapshot_survives_a_later_export_clear`.
  - `a_clear_after_a_dynamic_export_is_still_empty`: the unknown barrier is kept either way.
  - `a_load_level_forget_before_a_body_local_import_revokes_nothing` and
    `a_body_local_exact_import_survives_a_top_level_forget`: the install sits in a body, so
    the order is Conditional.
  - `a_body_local_call_resolves_through_an_import_written_later_in_its_file`: installs keep
    the lenient gate.
  - `abstention_2_a_statement_above_the_require_stays_unordered`: the clear-to-provider
    pattern order is cross-document under a package bound, so `None`.
  - `a_forget_in_another_file_revokes_nothing` and
    `a_cross_file_forget_cannot_revoke_on_unrelated_offsets`.
  - `indexed_order_context_is_replaced_and_missing_full_input_remains_unknown`.
  - The package-alternative and cycle/re-source source_graph controls, which are cross-URI.
  - SourceHorizon advice: `navigation_comparison` returning `Some` more often changes
    nothing, because `source_advice_statement_comparison` still requires sites.

## Risks

1. **Words inside `namespace eval` bodies.**
   - The patch assumes `source_registry_words` resolves commands inside a `namespace eval`
     body with complete roles. The commands involved are `namespace import|forget|export`,
     `proc` and `rename`.
   - The existing site capture *skips* these commands, because the body is in
     `caller_context_unavailable_bodies`. The sequence pass resolves them on its own.
   - If they do not resolve, or their roles come back incomplete, those points are
     withheld. The failure is closed: 404, 406 and the same-file workspace tests stay red,
     and no answer is wrong.
   - Test-400 is top-level and does not depend on this.
   - Fast check: in the new guard test, `set inner` inside `namespace eval ::n` must be
     `DocumentSequence`.
2. **Roles completeness.**
   - `namespace export` with a dynamic first word, or any command whose option value roles
     are unknown, has `roles() == None`. Only its command word is admitted.
   - This is conservative.
3. **Early exits are not modelled.** A top-level `return`, `exit` or error that stops the
   document before a later removal still counts that removal as having run. The constructed
   model has the same blind spot, and the decision accepts "source sequence is known".
4. **`uplevel` / `eval` count as straight-line.** Each has `ConcatenatedScript` with one
   literal script word. At top level, `uplevel 1` errors in real Tcl, which falls under the
   same early-exit caveat.
5. **Cost.**
   - `SourceOrderContext::capture` now also resolves words for every direct command of a
     straight region, which are cached and reused by the site loop. It also computes the
     innermost region per command, O(commands × regions).
   - `sequence_position` scans the regions and commands linearly, like `site()`.
   - `definition::in_document_source_order` still captures a fresh context for each query,
     as before. This is worth caching later if profiles show it.
6. **Docs outside §8 still describe "conditional only" without the exception.** I did not
   edit them, because they are proof records with manifest hashes or have uncommitted local
   edits:
   - `docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md`,
     section "Conditional workspace ordering boundary";
   - `docs/design/contracts/shared-utility-contracts-rust.md`, row "Conditional original
     source navigation order". It is modified in the working tree by someone else. The
     `xtask owner-resolution` row may want `RunOrder::is_source_navigation` and
     `NamespaceStatementPurpose::removal_in_effect` listed.
   - `docs/design/contracts/workspace-indexing.md:566`.
   The owner should add one sentence to each, pointing to §8.1.
7. **Tracking docs.** The `audit-status` and `current-state` entries for B2 must record that
   the 1b45c540e controls changed under decision `export_order`.
