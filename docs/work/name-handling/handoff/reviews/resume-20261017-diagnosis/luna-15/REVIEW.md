# Luna-15 review: Integration42 candidate (Sol-14 patches)

- **Commit:** `db5a1ba9a`, base `0e1fa1670`, branch `work`.
- **Method:** static review, rustfmt over stdin, and probes under `nice tclsh8.6` (8.6.18) and `nice tclsh9.0` (9.0.4). No cargo was run and the repository was not modified.
- **Where everything is:** all paths below are under `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/luna-15/`.

## Verdicts

| Patch | Verdict |
|---|---|
| `auto-import-fixtures.patch` | **ADMIT**. Record the scope note N2 and replace the receipt as described in N1. |
| `auto-import-fixture-025-optional.patch` | **ADMIT**, on condition that the scope extension is recorded (N3). |
| `h2-end-of-load.patch` | **ADMIT**. Follow-ups P2-1 and P3-1 to P3-3 are non-blocking. |

### Does the commit match the three patches?

Yes. I applied all three patches, in order, to copies of the `0e1fa1670` files. The result is byte-identical to `db5a1ba9a` for all 7 code and design-doc files (`diff -r p b`). The commit also adds coordination documents (the luna-14 worklist and sol-14 NOTES), which are outside this review.

## (1) Fixture patches

### The edits are pure prefixes

A script (`extract.py`) diffed `a/` against `b/` line by line. It found 21 changed literals: 20 from the main patch and 1 from the 025 patch. In every one, removing the prefix `rename auto_import savedPrelude; ` gives back the old line exactly.

There are no other edits in those hunks:
- no assertion, cursor or offset constant changed;
- `command_binding.rs` has 1 hunk;
- `definition.rs` has 15 single-line hunks;
- the `workspace_index.rs` fixture hunks are only the 5 literals.

### Positions are still valid

- **Line and column cursors.** `definition(src, 10, 4)` and `(11, 4)` stay valid because the prefix has no newline and sits on line 0.
- **Byte offsets.** Every one is derived from a needle:
  - `at_first`, `after_last`, `app_src_offset` (`find`) and `source_proof` (`rfind`);
  - or `u32::MAX` or `src.len()`.
- **No needle falls inside the prefix.** I checked `"\np\n"`, `"    p\n"`, `"helper\n"`, `"\nhelper\n"`, `"return LOCAL }"`, `"namespace import -force"`, `"::src::$name"` and `"::b::f"`.
- **Test 024's "before" offset.** It moves from 0 to 33, still on the first `helper` call. `"\nhelper\n"` still matches only the second call.

### Every edited fixture imports inside a `namespace eval` body

This holds for all 21. Test 024 places the prefix before a root `helper` call, and its import is still in a body.

### tclsh evidence: Sol's receipt does not cover the renamed fixtures (N1)

- **What is missing.** `sol-14/tcl/evidence-task-a.txt` prints `(b unchanged)` for every test. No run in it has `savedPrelude=1`.
- **The variant file has the same gap.** The `[b …]` rows in `evidence-task-a-variants.txt` show `auto_import=1 savedPrelude=0`, so they ran unprefixed text.
- **The real-oracle re-import is unrecorded.** The caught variant behind Sol's claim for test #9 (`b-real-oracle-src2-caught.tcl`) has no recorded output.
- **So the NOTES claim is unsupported.** "Identical with and without the rename" is not backed by Sol's files.

I re-ran everything independently. The harness is `probe/h.tcl`, driven by `run.py` and `caught.py`. It uses fixtures extracted from the actual Rust literals of both trees. Results are in `results-task-a.txt` and `results-caught.txt`.

- **All 21 fixtures, without and with the rename, on 8.6 and 9.0:**
  - The command table (`namespace origin` of every command in the touched namespaces) and every probe are identical across all four runs.
  - The only differences are the ones expected:
    - without the rename, `auto_import` is called with the import pattern;
    - with it, it is not called, and `auto_import=0 savedPrelude=1`.
  - Each asserted outcome is supported. For example:
    - 404 and its family: `info commands ::dst::*` is empty;
    - `export_clear_with_patterns…`: `::dst::p` comes from `::src::p`, and `namespace export` is `p`;
    - `a_forget_lets…`: BP, from `::B::p`;
    - 406 at the end: LOCAL2, from `::dst::p`;
    - the chain test: `::A::*` and `::B::*` are both empty;
    - `source_c_import`: OLD, `::a::saved`;
    - 022, 025 and 029: `::app::helper` is absent;
    - 028: only `::app::run`.

#### Calls that abort the load

Five calls abort the load in tclsh. I ran each with the call wrapped in `catch`, on both trees and both versions:

| Test | Abort | With the call caught |
|---|---|---|
| `a_local_defined…` | root `p` aborts before the redefinition | between: `::dst::p` comes from `::src::p` (SRC); end: LOCAL with origin `::dst::p`, and `::src::p` is SRC |
| `the_real_oracle…` (src2) | `p` in `dst` aborts before the re-import | uncaught: `::dst::*` is empty; caught: `::dst::p` comes from `::src::p` |
| 024 | the first root `helper` aborts before the import | before the import: `::app::*` is empty; at the end: `::app::helper` comes from `::mymod::helper` |
| 022 | the root `helper` fails after the forget | before the forget: from `::mymod::helper`; end: empty |
| 406 | (cut point, not an abort) | between: from `::src::p` (SRC) |

#### Is catching faithful to what the tests mean?

Yes. Each of these tests asserts the command-table state of a named namespace (`::dst`, or the `::app::helper` candidate) at a source position. The root call is only a position marker, and `catch` does not change the command table.

One test-modelling issue predates this patch:
- 024's second half asserts that a root `helper` resolves to `::mymod::helper`.
- In tclsh, a literal root `helper` fails even after the import, because the import goes into `::app`, not the global namespace.
- The test supplies the `::app::helper` candidate itself, so the assertion holds for the candidate, not for the literal call.
- The edit does not change this, but the oracle comment could say so.

### Scope

- **N2.** Five edited definition tests are not in Sol-5 v6's enumerated "404 family":
  - `single_file_definition_refuses_direction_b_through_the_fallback`
  - `the_tail_match_refuses_a_forgotten_import_too`
  - `a_local_defined_after_the_import_is_not_a_conflict_but_does_end_the_alias`
  - `a_redefinition_ends_a_forced_import_shadow` (406)
  - `a_forget_lets_the_next_unforced_import_install`

  All five fit the decision's class wording ("test fixtures that import inside a namespace eval body without `rename auto_import`"), and tclsh shows the edit preserves meaning. Record that the class reading was applied.
- **N3.** 025 (`an_exact_import_link_dies_on_a_same_file_forget`) is not in the 0915Z decision's list. Sol-14 flagged it as needing a decision. The commit applied it, and the 0943Z progress report calls it "user-authorised (20 tests + 025)". The edit is a pure, meaning-preserving prefix: tclsh gives `::app::helper` absent with and without it. Amend the decision record, or confirm with the user, so the authorisation is not implied.
- **Correctly left alone.** `a_forced_import_before_the_call_does_not_shadow_a_later_call_site` is wrong against tclsh: I re-ran it, and `info commands ::dst::*` is empty and `p` gives `invalid command name "p"` on both versions. Fixing it needs an assertion change, which this exception does not allow.

## (2) H2: `h2-end-of-load.patch`

### Code

- **`removal_in_effect`.** It now reads `query.or_else(|| order.document_end(event.uri))`, then takes the unchanged `precedence(..) == Some(true)` path.
- **`document_end`.**
  - It returns `SourceOrderContext::sequence_end()`.
  - That is `sequence.end`, only when `sequence_position(end)` holds.
  - That in turn needs `DocumentSequence::contains(end)`, which needs no cutoff and `!end_withheld`, plus the same image, input and Realm currency check as `site`.
- **The decision falls back conservatively.**
  - When either the removal or the END is not a sequence position, `navigation_precedence` falls back to `source_navigation_answer`, which is `Conditional` under source navigation.
  - So `modeled_source_exclusion` is None, and the removal is not applied.
  - The revocation then still needs `after(r, i)`, the same-document DocumentSequence order of removal after install.

### Soundness against C Tcl after `source`

I wrote my own shapes with the rename prelude: `h2/shapes.tcl`, with results in `h2/evidence-h2.txt`. Both versions agree.

**Where the patch answers "dead", tclsh is also dead.** In no case does the patch say dead where tclsh says live:

| Shape | tclsh | Patched |
|---|---|---|
| straight at the root | dead | dead |
| straight in a root `namespace eval` body (025 shape) | dead | dead |
| forget after `if {$x} return`, x=1 | live | live |
| forget after `if {$x} return`, x=0 | dead | live (cutoff, conservative) |
| early exit **after** the forget, x=0 or x=1 | dead | live (END cut off, conservative; Sol-14 had no such shape) |
| `if {$::x} return` inside the body before the forget | live | live |
| forget in a `proc` body, uncalled | live | live |
| forget in a `proc` body, called after the load | dead | live |
| forget in a branch, x=0 | live | live |
| forget in a branch, x=1 | dead | live |
| forget, then `source y.tcl` | dead | live (END withheld) |
| forget, then a user procedure | dead | live (END withheld) |
| forget before the import | live | live |
| cross-document, forget file sourced first | live | live |
| cross-document, forget file sourced last | dead | live (`after` is cross-document, so Conditional) |
| body-local import, `setup` uncalled, root forget | dead | live |
| body-local import, `setup` called, root forget | live | live |
| body-local import and forget in the same body, called | dead | live |

### No positioned consumer changes

- **Production callers.** `link_alias_live` (`workspace_index.rs:8788`) is the only production caller with `None`. The other callers pass `Some`:
  - `alias_live_at` at `workspace_index.rs:8590`;
  - `definition.rs:3520`;
  - `export_patterns_for_purpose` (`Some(import_site)`).
- **Unchanged paths:**
  - `alias_certainly_live` does not call `removal_in_effect`;
  - installs use `in_effect`;
  - SourceHorizon and constructed orders (`RunOrder::default()`, as in the `namespace_import` `unlinked()` tests) never reach the new branch.
- **Link-mask readers.** Definition, references and `workspace_command_exists*` read the link mask (`live_command_links`), so for an exact link with a straight same-file forget after it they now see the link as dead.
  - This is the effect the decision intends.
  - A load-time call of the imported name between the import and the forget is a non-Registry command, so it withholds END and keeps the link.
  - The only sites that lose the link are in deferred bodies, which run after the load, which matches C Tcl.

### The changed control test

In `document_end_follows_every_straight_removal_but_no_foreign_one`:
- the only flipped assertion is the unpositioned same-document one (`alias_live_at(None)` was true; `!unpositioned_live(before, after)` now);
- a foreign unpositioned assertion is added (`unpositioned_live(before, foreign)`);
- the positioned `exported_and_live` assertions are untouched.

This matches the user's decision.

### New tests

- **`an_unpositioned_removal_applies_only_before_an_admitted_document_end`.**
  - Expectations: the eight rows are consistent with the code.
  - Risk: the body-straight row relies on the root `namespace eval ::app {set…}` admitting END. The guard `document_sequence_orders_straight_line_bodies_and_withholds_branches` already asserts `set after`→END is KNOWN for a `namespace eval ::n` body.
- **`an_exact_import_link_keeps_a_conditional_same_file_forget`.** Its expectations match tclsh: when `x` is true or `teardown` is uncalled, `::app::helper` comes from `::mymod::helper`.

### Compile, format and lints (static)

- **Lifetimes.** `RunPoint<'a>` only holds `&'a str`, `u32` and `Option<Span>`, so it is covariant. NLL picks a common region for `Option<RunPoint<'q>>::or_else(|| Option<RunPoint<'e>>)`, so this should compile. Not built.
- **rustfmt.** `rustfmt --edition 2024 --emit stdout` over stdin, compared byte for byte, is clean for all 5 touched `.rs` files in both trees.
- **Lints.**
  - No `#[allow]` was added.
  - `document_end` is `pub(crate) #[must_use]`.
  - The private `sequence_end` does not need `#[must_use]`.
  - The workspace lints are pedantic only, so the new `unwrap()` in tests is fine.

### Docs

- **§8.1.** The end paragraph now names its one consumer. The unpositioned paragraph correctly states:
  - the admitted-END rule;
  - the withheld cases;
  - that a foreign removal never revokes;
  - that end-only cutoffs are conservative.
- **`workspace-indexing.md`.**
  - It fixes the stale claim that "every recorded removal counts as having run".
  - It says where the end is withheld and that branch and deferred removals do not count.
  - The cross-reference sentence ("…through the admitted document end, for the unpositioned link fold") is accurate.

### Findings (all non-blocking)

- **P2-1. `package require` after the forget.**
  - **The gap.** `package require` (`LOADS_EXTERNAL_UNIT`; neither `TRANSFERS_CONTROL` nor `SourceFile`) after the removal does not withhold END. External code can re-import the name.
  - **Probe** (`h2/pk`): import, forget, then `package require foo`, whose ifneeded script re-imports into `::app`. Result: `::app::helper` is live, from `::mymod::helper`, on 8.6 and 9.0. The patch marks this import site's link dead.
  - **Scope.** Positioned queries after the same point already have this limit. The re-import is a different import site, which is navigable when it is indexed.
  - **Action.** Add it to §8.1 Known limits, or make `LOADS_EXTERNAL_UNIT` withhold END.
- **P3-1. Error paths.** An error before the forget (`expr {1/0}`, a Selected command) leaves the alias live in tclsh if the caller catches the `source` error. The patch answers dead. This follows the decision's "normal completion" reading and §8.1's "an error path never reaches it". It is information only.
- **P3-2. Conservative refinement left out.** A removal that is a sequence position before every cutoff has run whenever the document completes. So "exit after the forget" and "forget then `source` or a user procedure" could be dead, matching tclsh. Not needed for soundness.
- **P3-3. Doc-comment nit.** The `link_alias_live` doc comment's list of reasons END is withheld omits `source` and control transfer; §8.1 has the full list.
