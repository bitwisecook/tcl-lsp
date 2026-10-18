# Sol-14: auto_import fixtures and H2 (unpositioned consumers read the document end)

Base: HEAD `0e1fa1670`, which already contains Integration41 (`c0437decf`, layer-a / named-ensemble seal premises). Every scratch `a/` file is byte-identical to `git show HEAD:<path>`.

Note: while I was working, the live worktree and index at /home/user/tcl-lsp were changed by someone else. They now hold a staged reversal of Integration41 and are no longer equal to HEAD. I did not touch the repository. All apply checks below ran against exact copies of HEAD's files.

## Deliverables

| File | sha256 | Lines |
|---|---|---|
| `auto-import-fixtures.patch` | `e0b9112c0fc7a6b3da7930593bb1022e6518d9d07bd36e60c45415765408d85d` | 189 |
| `h2-end-of-load.patch` | `bc127ea62a3412e4724235ccc1691497212012e89da1c39ef2ef6eadb043d023` | 280 |
| `auto-import-fixture-025-optional.patch` | `50af0eb2dd75396f7209c9caa2d2007b6abae465df9827078f75e25f31be27b4` | 12 |

The optional patch is outside the user's list; see "Test 025" below.

### Checks

All checks are static. Nothing was compiled or run under cargo.

- **Apply.** All three patches are `diff -ruN a b`. Against a copy of HEAD, each passes both `git apply --check` and `git apply --check --cached`, and the three stack cleanly in any combination.
- **Format.** rustfmt (edition 2024, formatting via stdin compared with the original) is clean on all five touched `.rs` files. Note that `rustfmt --check` reading from stdin always exits 0, so it proves nothing here.
- **Lints.** No `#[allow]` added and no Registry change: neither task needed one.
- **tclsh.** tclsh 8.6.18 and 9.0.4, run under `nice`. Every result below was identical on both.

Evidence files:
- `tcl/evidence-task-a.txt`
- `tcl/evidence-task-a-variants.txt`
- `tcl/h2/evidence.txt`

Harness: `tcl/state.tcl`, `run_a.py`, `run_a2.py`, `tcl/h2/shapes.tcl` and `tcl/h2/ctl.tcl`. The fixtures were extracted from the Rust literals and unescaped, not retyped.

## Task A: `auto-import-fixtures.patch`

### The change

Every change is the same edit: the importing document's literal gains the prefix `rename auto_import savedPrelude; `. That is the exact spelling of the existing positive fixtures (for example `original_namespace_binding.rs:823` and `source_command_world.rs:3117`).

- The 20 hunks are pure prefix insertions. A script verified this by checking that each `-` line equals its `+` line with the prefix removed.
- No assertion value changed.
- No cursor or offset constant changed, and none needed to:
  - **Line numbers:** the prefix sits on line 0 and is joined with `; `, so every line number stays valid. This covers `definition(src, 10, 4)`, `definition(src, 11, 4)` and `start_line`.
  - **Byte offsets:** every one is computed from a needle on the same string (`at_first`, `after_last`, `app_src_offset`, `rfind` in `source_proof`), or is `u32::MAX` or `src.len()`. None of the needles occurs inside the prefix.

### What tclsh shows

In every fixture without the rename, tclsh calls stock `auto_import` with the import pattern (for example `::src::*` or `::mymod::*`; the execution trace records it), and it loads nothing. With the rename it is not called. The resulting command table (`namespace origin` for every command) and each probe result are identical with and without the rename. So the rename does not change what any fixture means.

### Changed tests

All 20 tests, with the state each asserts and what tclsh shows (8.6 and 9.0 identical):

| # | Test | Asserted | tclsh |
|---|---|---|---|
| 1 | `command_binding::tests::source_c_import_follows_the_original_command_object` | target `::a::saved`, origin `::a::f` | `::b::f` returns OLD; `namespace origin ::b::f` is `::a::saved` |
| 2 | `definition::tests::a_forget_after_the_import_stops_resolving_the_alias` (404) | none | `info commands ::dst::*` is empty; `namespace eval ::dst {p}` gives `invalid command name "p"` |
| 3 | `definition::tests::a_glob_forget_removes_every_matching_alias` | p and q gone | `::dst` is empty |
| 4 | `definition::tests::a_simple_forget_pattern_removes_the_alias_too` | gone | `::dst` is empty |
| 5 | `definition::tests::a_dynamic_forget_pattern_revokes_nothing` | literal control: gone | literal spelling: `::dst` is empty |
| 6 | `definition::tests::a_forget_in_the_middle_of_a_chain_kills_the_whole_chain` | `::A::p` gone | only `::C::p` is left |
| 7 | `definition::tests::deleting_the_source_command_kills_the_alias` | gone | nothing is left |
| 8 | `definition::tests::import_site_snapshot_ignores_a_later_export` | none | `::dst` is empty |
| 9 | `definition::tests::the_real_oracle_does_not_resurrect_an_export_written_after_the_import` | none, then `::src::p` after the re-import | `invalid command name "p"` |
| 10 | `definition::tests::single_file_definition_refuses_direction_b_through_the_fallback` | definition empty | `namespace eval dst {p}` gives `invalid command name "p"` |
| 11 | `definition::tests::the_tail_match_refuses_a_forgotten_import_too` | definition empty | same: `invalid command name "p"` |
| 12 | `definition::tests::export_clear_before_the_import_does_revoke` | none | `::dst` is empty; `namespace export` is `{}` |
| 13 | `definition::tests::export_clear_with_patterns_on_the_same_call_keeps_only_the_new_ones` | p yes, a no | `::dst::p` comes from `::src::p`; no `::dst::a`; `namespace export` is `p` |
| 14 | `definition::tests::a_local_defined_after_the_import_is_not_a_conflict_but_does_end_the_alias` | between: `::src::p`; after: alias gone, local proc | between: SRC, origin `::src::p` |
| 15 | `definition::tests::a_redefinition_ends_a_forced_import_shadow` (406) | between: `::src::p`; after: `::dst::p` | between: SRC, origin `::src::p`; at the end: LOCAL2, origin `::dst::p` |
| 16 | `definition::tests::a_forget_lets_the_next_unforced_import_install` | `::B::p` | `::dst::p` returns BP, origin `::B::p` |
| 17 | `workspace_index::tests::a_same_file_forget_after_the_import_still_revokes` (022) | none | `::app::helper` is absent after the forget, and before it comes from `::mymod::helper` |
| 18 | `workspace_index::tests::a_top_level_call_before_its_own_import_does_not_resolve` (024) | before: none; after: `::mymod::helper` | before: `invalid command name "helper"`; after: `::app::helper` comes from `::mymod::helper` |
| 19 | `workspace_index::tests::exact_import_ignores_an_export_written_after_it` (028) | `::app::helper` does not exist | the import of the unexported name is silent and `::app::helper` is absent |
| 20 | `workspace_index::tests::resolve_wildcard_import_ignores_a_same_file_export_written_after_the_import` (029) | none | `::app::helper` is absent |

Test ids 022, 024, 028 and 029 follow the targeted-39a numbering, which matches sol-5 NOTES-v6.

Details that need more than one cell:

- **#5, the dynamic half.** This half is a static abstention, not a claim about what tclsh does. With `$name` unset, tclsh fails with `can't read "name"` and leaves the alias in place.
- **#9, the re-import half.** The fixture's own `p` call aborts the script before the appended re-import runs. With that call caught, the re-import leaves `::dst::p` from `::src::p` (returns SRC).
- **#14, after the redefinition.** The fixture's root `p` call aborts the load in tclsh, because that `p` is in the global namespace. With that call caught, the end state is `::dst::p` returning LOCAL with origin `::dst::p`, and `::src::p` still returns SRC.
- **#15** is a separate defect (sol-5) and may stay red regardless.
- **#17, the root call.** The final root `helper` call fails in tclsh because it runs in the global namespace. The test models the call through the candidate `::app::helper`.
- **#18, the root calls.** The first root `helper` call aborts the file in tclsh. The "after" figure comes from a variant with that call caught; the test models both calls through `::app::helper`.

Definition test #13 (`export_clear_with_patterns…`) also passes its `::dst::a` check.

### Not changed, and why

- **`definition::tests::a_forced_import_before_the_call_does_not_shadow_a_later_call_site`.**
  - It has the 404 shape, but its assertion is not what tclsh does. The test asserts that the call reaches `::dst::p`.
  - In tclsh, `namespace import -force` deletes the local `::dst::p`, and the later forget removes the alias. `info commands ::dst::*` is then empty and the call fails with `invalid command name "p"`, on both 8.6 and 9.0.
  - This is a pre-existing oracle error. It needs a decision on the assertion, which is outside this exception, so I left it alone.
- **Positive controls of the same block.** These were not changed: `import_site_snapshot_survives_a_later_export_clear`, `a_call_before_the_forget_still_resolves`, `a_forget_of_an_unrelated_source_leaves_the_alias_alone`, `a_re_import_after_a_forget_resolves_again`, `renaming_the_source_command_leaves_the_alias_alive`, `a_call_before_the_source_deletion_still_resolves`, `a_second_import_after_a_later_export_does_pick_the_name_up` and `a_redefinition_of_the_source_is_seen_through_the_link`.
  - Withholding keeps their candidates, so the sound rule does not withhold what they assert.
  - The family I changed is the set of tests whose assertion needs a same-document removal or export to be ordered against an import written in a root `namespace eval` body.
- **Workspace test 025 (`an_exact_import_link_dies_on_a_same_file_forget`).** It is not in the user's list; see below.

## Task B: `h2-end-of-load.patch`

### Rule

An unpositioned removal reads the end of its own document, using the existing DocumentSequence END from V5.

- **Code:** `NamespaceStatementPurpose::removal_in_effect` (Modeled purpose, source navigation) now uses `query.or_else(|| order.document_end(event.uri))`. It then takes the same `precedence == Some(true)` path as a positioned query.
- **`RunOrder::document_end`:** this new `pub(crate)` method returns `SourceOrderContext::sequence_end()`. That is the end, `DocumentSequence::end`, but only when `sequence_position(end)` holds, meaning no cutoff and no `end_withheld`, under the same image, input and Realm currency.
- **When a removal applies:**
  - It applies when the END is admitted and the removal itself is a sequence position. The fold then revokes an install only through `after(r, i)`, which needs both points to be in the same document's sequence.
  - It does not apply when the END is withheld, when the removal is in a body or a branch, or when it comes after a cutoff. That is the conditional answer from HEAD.
  - A removal in another document is still never ordered after the install, so it revokes nothing.
- **Unchanged:** installs (an unpositioned install still counts), SourceHorizon, constructed orders and `alias_certainly_live`.
- **Callers covered:**
  - `link_alias_live` is the only production caller that has no query point. A search for `None` queries across `alias_live_at`, `alias_for_purpose` and `exported_at_import_site` found no other.
  - Every other fold passes a query point. Export snapshots are positioned at the import site.
  - The rule sits in the shared gate, so any future unpositioned glob caller gets it too.

### Docs

- **§8.1 of `import-order-source-graph.md`:**
  - The END paragraph now names its one consumer.
  - The unpositioned paragraph states the rule, the withheld cases, the foreign-document case, and that end-only cutoffs are conservative.
  - The controls list is updated.
- **`docs/design/contracts/workspace-indexing.md`:** the link-tier paragraph said every removal counts as having run, which was already stale against HEAD. It now states the rule. The §8.1 cross-reference sentence is also updated.

### Doc comments

Updated on `removal_in_effect`, `alias_live_at` (the `query: None` paragraph) and `WildcardImportIndex::link_alias_live`.

### Tests

- **`source_graph::…::document_end_follows_every_straight_removal_but_no_foreign_one`.** Its pinned unpositioned answer flips, as the user decided:
  - the same-document removal now revokes (`!unpositioned_live`);
  - a foreign removal does not (`unpositioned_live`).
  - The helper `unpositioned_live` is new.
- **New `source_graph::…::an_unpositioned_removal_applies_only_before_an_admitted_document_end`.** It runs real `DocumentSequence` captures with synthetic install and remove points. Expected link state for each shape:

| Shape | Link |
|---|---|
| Straight at the root | dead |
| Straight in a root `namespace eval` body | dead |
| Removal after `if {$x} return` | live (cutoff) |
| Removal, then `source y.tcl` | live (END withheld) |
| Removal, then a user procedure that does `return -code return` | live (END withheld) |
| Removal in a `proc` body | live |
| Removal in an `if` branch | live |
| Removal before the install | live |

- **New `workspace_index::tests::an_exact_import_link_keeps_a_conditional_same_file_forget`.**
  - A same-file forget behind `if {$x} return`, or inside `proc teardown`, leaves `resolve_command_target("::app::helper") == "::mymod::helper"`.
  - Its fixtures carry the rename, so the only reason the answer stays conditional is the early exit or the body.

### tclsh evidence (`tcl/h2/evidence.txt`)

Each run sources `mymod.tcl` and then the shape in a fresh interpreter, on 8.6 and 9.0, which agree.

| Shape | `::app::helper` after the load | Patched answer | Agrees with tclsh? |
|---|---|---|---|
| Straight | absent | dead | yes |
| Body-straight | absent | dead | yes |
| Early exit, `x=1` | present | live (conditional) | yes; the answer covers both |
| Early exit, `x=0` | absent | live (conditional) | yes; the answer covers both |
| `proc` uncalled | present | live (conditional) | yes; the answer covers both |
| `proc` called | absent | live (conditional) | yes; the answer covers both |
| Branch, `x=0` | present | live (conditional) | yes; the answer covers both |
| Branch, `x=1` | absent | live (conditional) | yes; the answer covers both |
| Removal before install | present | live | yes |
| Cross-document, forget file loaded first | present | live (conditional) | yes; the answer covers both orders |
| Cross-document, forget file loaded last | absent | live (conditional) | yes; the answer covers both orders |
| `source y.tcl` after the forget | absent | live (END withheld) | no: conservative |
| User `return -code return` after the forget | absent | live (END withheld) | no: conservative |
| 025, with or without the rename | absent | dead (expected; see below) | yes: `resolve_command_target` should be `::app::helper` and `workspace_command_exists` false |

- **The two "conservative" rows.** In both, a re-import placed after the cutoff did not run. The withholding follows the rule as given. A possible refinement: a removal that is a sequence position before every cutoff has run whenever the document finishes, so it could apply even when an END-only cutoff withholds the END. I did not implement this.
- **The new workspace control's fixtures with the rename:**
  - `x=1`, or `teardown` uncalled: the import comes from `::mymod::helper`.
  - `x=0`, or `teardown` called: `::app::helper` is absent.

### Test 025 needs more than H2

`an_exact_import_link_dies_on_a_same_file_forget` imports and forgets inside a root `namespace eval` body without the rename.

- **Why H2 alone does not flip it.** END admission requires `Selected` words on every command outside definition bodies, including those nested in immediately run bodies (`record_exits`: `!words.is_some_and(selected)`).
  - At pre-Integration41 HEADs, body namespace statements are not `Selected` (sol-5 v6, diagnosis A), so END is withheld.
  - Under layer-a (now in HEAD), a body import without `rename auto_import` stays withheld under the sound rule.
  - So H2 alone should leave 025 red.
- **Expected path to green:** H2 plus layer-a plus the rename in 025's fixture.
- **The optional patch.** `auto-import-fixture-025-optional.patch` adds that one prefix, which is a pure insertion with no assertion change. tclsh shows that with the rename `::app::helper` is absent, on 8.6 and 9.0.
- **Decision needed.** 025 is not in the user's list of fixtures that may change. The user, or you on their behalf, needs to decide whether to extend the exception. If it is not extended, 025 stays red under the sound rule.

## Risks to confirm in the next capture

These are unverified because nothing was compiled.

**Compile:**
- `query.or_else(|| order.document_end(event.uri))` mixes two elided lifetimes. `RunPoint` is covariant, so this should infer.
- `document_end` and `sequence_end` are new, `pub(crate)`/private, and `#[must_use]`.
- The new tests use only existing helpers and APIs (`point`, `SourceOrderContext::capture`, `RunOrder::for_source_navigation`, `analyse` and `WorkspaceIndex::from_documents`).

**Expectations:**
- `an_unpositioned_removal_applies_only_before_an_admitted_document_end` relies on `set` inside a root `namespace eval ::app` body being a sequence position with an admitted END. The existing guard test already asserts this for `::n`.
- Under layer-a, the changed fixtures should flip only where the sequence order is the remaining gap: 404 and its family, plus 022, 024, 028 and 029.
- 406 is a separate defect.
- `source_c_import_follows_the_original_command_object` should flip under layer-a's seal with the rename. That was sol-13's withheld item.

**Suggested selectors:**
- every test in the tables above, plus `a_forced_import_before_the_call_does_not_shadow_a_later_call_site`;
- `source_graph::original_source_navigation_order_tests::*`;
- the workspace link controls `an_exact_import_link_*` and `a_body_local_exact_import_survives_a_top_level_forget`;
- `namespace_import::tests::*`.
