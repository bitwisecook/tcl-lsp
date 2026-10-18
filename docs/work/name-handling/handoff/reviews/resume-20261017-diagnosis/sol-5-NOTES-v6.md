# Sol-5: same-document order, v6 delta (against e46e88617)

Patch: `same-document-order-v6-delta.patch` (sha256 7dfe7acc3f6c506da68fbae09acb5842586ace24fd2e54ee0eab8f6bde4b7a0e, `diff -ruN a b`; a/ is `git show e46e88617:<path>`).
Files changed: `rust/tcl-lsp-core/src/source_graph.rs` and `docs/design/analysis/import-order-source-graph.md` (§8.1).

Checks:
- `git apply --check` passes against the working tree, and `--check --cached` passes against the index (HEAD e6cb0aecd).
- `rustfmt --edition 2024 --check` is clean.
- No `#[allow]`, and no `.orig` or `.rej` files.
- Not compiled: there is no cargo in this task.

Diagnosis method:
- A copy of the build39 binary (`dbg/tcl_lsp_core`, sha 3a0d50de…) run under gdb, one process at a time at `nice -n 19`.
- Probes: patched-literal fixtures run through the guard test (`dbg/probe2.sh`, `probe_emu.sh`), plus Registry source tags and obligations read from memory.

## Cause

There are three independent causes, plus two failures that are not order failures.

### A. The v5 `Selected` gate (404 and the Modeled-path tests)

In a non-global `namespace eval` body, the compiler models `namespace export`, `import` and `forget` as execution-unknown:
- That command reports `compiled_execution_unknown`.
- That command and every later command report `execution_is_unknown()`, so `RegistryInvocationAssistance.unknown_residual` is set.
- `unanimous_command_words()` returns None, so no later command gets `Selected` words.

Probe (`namespace eval ::a {namespace import ::m::*}`, then `namespace eval ::a {namespace forget ::m::h}`, then `h`, then `set z 1`):

| cmd | source |
|---|---|
| root `namespace eval` #1 | Selected |
| `namespace import` | Conditional, no obligations |
| root `namespace eval` #2 | Conditional [UnknownCurrentLookup, UnknownLookupCoordinates, UnretainedHolder] |
| `namespace forget` (in body #2) | SourceTransitions, lineage 0, obligations [UnavailableActualLookup, UnknownEarlierMutation, CNamespace/CBody applicability] |
| `h` | no words |
| `set z 1` | Conditional [same three] |

The same pattern appears in the 404 fixture, in `exact_import_ignores_an_export_written_after_it` and in test-024's `app.tcl`. Because of these sources, v5 refused three things:
- entering the later root `namespace eval` bodies;
- every non-head position of the import and forget commands (their pattern words, where the events sit);
- the document end.

It is not the Realm barrier. A barrier hit returns no words at all, and that happens before the Selected/Conditional choice.

### B. Source-horizon advice never consulted the sequence (022, 024, 029)

`WorkspaceIndex::resolve_wildcard_import` reports `conditional_namespace_import_advice`, which uses `NamespaceStatementPurpose::SourceHorizon`. That purpose orders events through `RunOrder::source_advice_statement_precedence`, then `original_point`, then `SourceOrderContext::site`.

Since 1b45c540e, `SourceOrderContext::capture` keeps no site inside caller-context-unavailable bodies, and every `namespace eval` body is one of those.

gdb trace for `a_same_file_forget_after_the_import_still_revokes`:
- `site` returns None at 61 (the export), 44 (the import) and 101 (the forget);
- every `source_advice_statement_precedence` returns None;
- `removal_in_effect` returns true (an abstaining `unwrap_or(true)`);
- `after(forget, install)` returns false, so the forget revokes nothing.

v5 changed only the Modeled path, so even with `Selected` forced to true these tests stayed red.

### C. The guard test failure is the END assertion, not `set inner`

The failure at `source_graph.rs:3354` is `purpose("set after", "END")`. `contains(38)` (`set inner`) is true, and lines 3352–3353 pass.

Behind `if {$x}` (a variable read) plus `proc q`, the compiler gives `set after 6` no words at all. Under the v3 rule ("unresolved words withhold the end"), the end is therefore correctly withheld. With `if {1}` instead, every command is Selected. This is a fixture problem, not a code problem.

### Not order failures

- **406, `a_redefinition_ends_a_forced_import_shadow`:** fails with the same message as HEAD did before v1 ("between … source wins: None"). It stays red under emulation and is a separate defect.
- **`an_exact_import_link_dies_on_a_same_file_forget`:** `link_alias_live` has no query point. Since v2 (Luna-7 H2), an unpositioned removal applies nowhere. This test needs the open H2 decision ("no query = end of load?"), not an order fix.
- **`method_symbol … own_object` (test-008):** not mine.

### sol-11 (TclOO namespace-advice withdrawal)

My fixtures do not hit it. None of them has a TclOO class body.

`AdviceGraph::widen` (the step that leads to `withdraw_namespace_advice`) does fire for `namespace export` (via `apply_schema_transition`). But it fires identically for a root `namespace export h`, and there `set after 1` keeps `Selected` words. The widen is therefore not what tells the two cases apart. The trigger here is the namespace-body execution-unknown described in A.

## Fix (minimal)

1. **`SequenceCapture::selected` becomes `SequenceCapture::nominal`.** It is used in the same three places: entering, non-head positions, and the end. It accepts:
   - `Selected`;
   - `Conditional` whose obligations are only `UnknownCurrentLookup`, `UnknownLookupCoordinates` and `UnretainedHolder` (the command table is not closed);
   - `SourceTransitions` with empty alias/move lineage.

   It still refuses:
   - `EarlierBindingAlternative` and `SelectedBindingAlternative`;
   - an unknown executing namespace;
   - Scoped, DefinitionMember, ProducedPrefix, ConditionalNativeSource and Vendor.

   The declared-name guard, the refusal inside caller-context-unavailable bodies, and the barrier (no words) are all unchanged.

2. **Source-horizon advice uses the document sequence first.** `RunOrder::source_advice_statement_precedence`, `source_advice_statement_comparison` and `source_advice_after_document` answer through `document_sequence` (same relation, `in_effect_within` / `load_order`) for two straight-line positions of one document, or a position and an admitted end. This happens before the site check. Every other pair keeps the site-based answer, and where both answer, the relation is identical.

3. **Tests.**
   - The guard test's END assertion moves to a straight fixture without the `if {$x}` read: END is known for `set inner` and for `set after`.
   - New `document_sequence_enters_bodies_after_an_unmodelled_namespace_statement`. It orders export, import and forget, the import and forget argument positions, and the end, and it covers the three source-horizon relations.

4. **§8.1.** Adds the definition of nominal words and the source-horizon paragraph, replaces "Selected" wording, extends Known limits (see below), and adds a controls entry.

### Soundness

What protected against in-document rebinding in v5 was never `Selected`. It was:
- the barrier, which gives no words for aliased, shadowed, deleted or source-procedure cells (`rename myeval eval` gives no words; the existing control passes under emulation);
- the declared-name guard.

`Selected` does not close `interp alias`. In this probe, the next `namespace eval` is still Selected, while tclsh 8.6 defers its body (`0:after#0`):

```
interp alias {} namespace {} after 0
namespace eval ::a {...}
```

The nominal-but-not-Selected residual is exactly "a rebinding the compiler does not model". That is the documented cross-document/`interp alias` limit; §8.1 now says so explicitly.

## Evidence (emulation: `selected` forced to the v6 `nominal` predicate, read from memory)

The emulation covers fix 1 only.

**`definition::`** (248 tests):
- Baseline: 72 failures.
- Emulated: 59 failures, 13 fixed, 0 broken.
- Fixed includes 404 (`a_forget_after_the_import_stops_resolving_the_alias`), `a_glob_forget…`, `a_simple_forget…`, `a_forget_in_the_middle…`, `export_clear_before_the_import_does_revoke`, `export_clear_with_patterns…`, `import_site_snapshot_ignores_a_later_export`, `the_real_oracle_does_not_resurrect…`, `deleting_the_source_command_kills_the_alias` and `a_dynamic_forget_pattern_revokes_nothing`.

**`workspace_index::`, `source_graph::` and `namespace_import::`:**
- Baseline: 80 failures.
- Emulated: 76 failures, 4 fixed, 0 broken.
- Fixed: `exact_import_ignores_an_export_written_after_it`, `a_glob_import_conflicts_with_a_later_exact_import_of_the_same_name`, `a_top_level_import_is_not_conflicted_by_a_later_top_level_one` and `a_cross_file_exact_link_conflicts_with_an_earlier_different_source`.

**Controls still passing under emulation:**
- `document_sequence_refuses_shadowed_or_namespace_local_entering_commands`
- `document_end_is_after_normal_completion_only`
- `document_sequence_ends_at_a_possible_early_exit`
- `a_call_before_the_forget_still_resolves`

**Patched-literal checks of the new test's shapes:**

```
set before 1
namespace eval ::m {namespace export h}
namespace eval ::a {namespace forget {set inner}}
set after 1
```
before → inner, inner → after and after → END are all DocumentSequence.

The forget-argument → END shape and the new straight END fixture are also known. The unchanged guard fixture still withholds END, which confirms the fixture change is needed.

**Fix 2 cannot be emulated** because it is new code. Fix 1 admits the same shapes of the source-horizon tests on the Modeled path:
- 022 has the shape of 404;
- 029 has the shape of `exact_import_ignores…`;
- 024 has the probe shape above.

Fix 2 routes the source-horizon relations through that relation. Expected to flip: 022, 024 and 029, plus the guard test with its new fixture. 025 and 406 stay red, for the reasons given above.

Passing same-file source-horizon tests are unchanged in answer. Those positions either had sites, in which case the relation is identical, or the pair is cross-document or in a body, in which case the fallback applies. Examples:
- `resolve_wildcard_import_survives_a_later_export_clear_in_the_import_file` (the clear follows the import, so it is not in effect at the import);
- `a_forced_import_onto_an_existing_workspace_command_installs` (the redefinition precedes the forced import);
- `wildcard_import_inside_a_body_*` (body positions fall back).

## Ask of the capture

Run:
- `source_graph::original_source_navigation_order_tests::*`
- `definition::` and `workspace_index::` (definition 404 and the workspace tests 022, 024, 028, 029)
- `conditional_namespace_*`
- the 1b45c540e controls

Then run the clippy gate. The compile risk is limited to these paths, all `pub` at e46e88617:
- `tcl_compiler::command_binding::OriginalCatalogueSourceObligation`
- `OriginalConditionalRegistryMetadata::obligations`
- `OriginalSourceCommandTransitionAdvice::lineage`
