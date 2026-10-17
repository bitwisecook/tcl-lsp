# Sol-9 review: Luna-6 `test-corrections-v1.patch`

- **Reviewed:** `scratchpad/agents/luna-6/test-corrections-v1.patch`, along with its `NOTES.md` and `probes/`.
- **Base:** repo HEAD `bb7277d82`, which includes the integration37 candidate `60051c3c2`. The worktree is clean.
- **Authority:** `user-decisions-20261017T2030Z.json`, decisions `document_links` and `non_tcl_tests`. Test 136 is untouched.
- **Method:** static reading, plus fresh `nice tclshX.Y` runs of every probe in `sol-9/probes/` (copied from Luna-6; the `.out` files there are mine).
- **Extra evidence:** I wrote extra probes in `sol-9/probes/extra/` and `sol-9/probes/classmethod-only.tcl`.
- **HEAD behaviour:** read from the running capture37 logs (`scratchpad/w/lsp-37/original-tcl_lsp_core/test-NNN.log`) and from targeted-37a. Nothing was compiled or run.

## Verdict

**ADMIT v1, with one comment-only condition (D1) and a gate run before anything counts as green.**

Every changed assertion and fixture reproduces under the stated tclsh versions, and each conclusion follows from its probe. Each change is the minimal tclsh-faithful one. No unrelated assertion changes. The fixture changes are limited to cases where tclsh rejects the fixture or contradicts the test's own stated intent, and every assertion value is unchanged.

## Mechanical checks

| Check | Result |
|---|---|
| `git apply --check` against HEAD worktree | pass (the definition.rs hunk applies at offset +19) |
| `git apply --check --cached` | pass |
| Patched copies (`sol-9/b/`, built from `git show HEAD:` and `patch -p1`): `rustfmt 1.10.0 --edition 2024 --check` | clean on all four files |
| `#[allow]` in the patch | none |
| `analyse_tcl9` | exists at `call_hierarchy.rs:2502`, inside the same `mod tests` (lines 1784–2801) as both edited tests |
| `definition(src,u32,u32,&a) -> Vec<LspRange>` and `.start_line` | match |
| `{locs:?}` / `{links:?}` | already used in the same modules |
| The 3 new test names | unique in `rust/` |
| Fixtures | re-extracted from my own patched copy with `extract_fixtures.py`; byte-identical to Luna's `probes/fixtures/` |
| Hunk scope | the hunks touch only the 9 tests named in NOTES. Diff is 73 `+` / 19 `-` lines |
| Spelling | no US spellings in added lines |

## Per-test verdicts

| # | Test | Change | tclsh re-run | HEAD (i37) today | Verdict |
|---|---|---|---|---|---|
| 1 | document_links `original_source_links_use_actual_selected_navigation_and_decline_shadows_or_stale_input` (124) | The rename fixture moves to the negative loop | 8.4–9.1: `can't rename to "load": command already exists` (8.6+ `TCL OPERATION RENAME TARGET_EXISTS`). The target is never sourced. Lone `load x.tcl` gives `file too short`. `::source` and the alias form do source the file | Failed at the rename fixture, `left 0` (test-124) | **ADMIT**. Gate: the positive loop now reaches the alias fixture on HEAD for the first time, and that outcome is unverified |
| 2 | `double_dash_terminator_skipped` (140) | `len==1`/target becomes `is_empty()` | 8.4 `wrong # args: should be "source fileName"`. 8.5/8.6 `…?-encoding name? fileName`. 9.0/9.1 `bad option "--": must be -nopkg` (`tclCmdMZ.c` 9.0.4: objc==3 accepts only `-nopkg`, checked). The control `source $t` sources | Returns 0 links (test-140) | **ADMIT**. Predicted green |
| 3 | definition `jump_to_nextto_namespaced_class_method` | Becomes `is_empty()`. Fixture and cursor unchanged | 8.6/9.0/9.1: `A does not refer to an object` (`TCL LOOKUP OBJECT A`). Method body namespace/path `::oo::ObjN ::oo::Helpers` | Returns `[]` (targeted-37a test-003, `left 0`) | **ADMIT**. Predicted green |
| 4 | new `jump_to_nextto_relative_qualified_class_method` | Expects line 2 | Returns `hi`, and still `hi` when B returns `from-B`, so dispatch reaches `::Ns::A::greet` | `canonicalise_class`: `::oo::Helpers::Ns::A` is not a class, so there is no shadow. The root lookup of `Ns::A` gives `::Ns::A` | **ADMIT** |
| 5 | new `jump_to_nextto_fully_qualified_class_method` | Expects line 2 | Same as #4 (`hi`/`hi`) | An absolute name skips the shadow check | **ADMIT** |
| 6 | new `nextto_refuses_a_class_name_reached_through_the_helpers_path` | Expects `is_empty()` | Returns `helpers` (also from a nested eval, and in the helpers-only variant). The tclsh answer would be line 2, never line 6 | `shadowed` gives `None` | **ADMIT**. This is the only test covering i37's shadow branch. If the analyser does not retain `::oo::Helpers::A`, the test fails with the unsound line-6 answer, which is the right failure |
| 7 | call_hierarchy `same_named_procs_in_different_namespaces_disambiguate_by_span` | Line 0 gains `namespace eval a {}; namespace eval b {}; ` | 8.4–9.1 original: `can't create procedure "::a::helper": unknown namespace`, and none of the 4 procs exist. Corrected: all exist, `::b::helper` enters `::bCallee` only. Line 1 is byte-identical and char 12 is `l` of `helper` | `[]` at `items.len()` (test-571) | **ADMIT**. A gate run is needed; the outcome on HEAD is unverified |
| 8 | `prepare_resolves_classmethod_over_same_named_method_by_cursor` | `analyse` becomes `analyse_tcl9` | 8.6: `invalid command name "classmethod"`, and `::C` is absent. 9.0/9.1: `::C` exists, the delegate holds `greet`, `C greet` runs the classmethod | `[]` at 8.6 (test-567) | **ADMIT**, but **predicted to stay red** (F2) |
| 9 | `outgoing_calls_does_not_conflate_method_and_classmethod_sharing_a_name` | `analyse_tcl9`. The classmethod moves to line 0 and line 2 is left blank | 8.6: the class is never created. 9.0/9.1 with the original order: `C greet` is `FORWARD to {myclass greet}` and `[C new] twice` enters the **classmethod**. With the new order: a real `method greet`, `[C new] twice` enters `instance-method`, and `C greet` enters `classmethod` | **Currently GREEN** at 8.6 (test-556 exit 0) | **ADMIT after D1** and a gate run (F1) |
| 10 | code_lens `lens_bare_word_in_method_body_is_not_a_self_dispatch` (057) | `"0 references"` becomes `"References unavailable"` | 8.6/9.0/9.1: `invalid command name "greet"` and 0 entries. With an `::unknown` or `::greet` embedding: 2 entries. **New, stock tclsh, document unchanged:** with `TCLLIBPATH` pointing at a tclIndex that autoloads `greet`, the default `::unknown` reaches the method 2× | `"References unavailable"`, Partial (test-057) | **ADMIT** (see (a)) |
| 11 | `lens_unknown_instance_method_is_not_counted` (067) | Same | 8.6/9.0/9.1: `can't read "x"` and 0 entries. With a proxy preset: 1. **New:** an ordinary caller that sets `x` and `source`s the unchanged document: `bark` is entered 1× | `"References unavailable"`, Partial (test-067) | **ADMIT** (see (a)) |

## Focus questions

### (a) code_lens `"0 references"` → `"References unavailable"`: required by tclsh, or a weakening?

**Required.** `"0 references"` is the complete-census title, a claim that no site in the document reaches the member. tclsh refutes that claim for both fixtures without any change to the document text:
- **057:** the bare `greet` is an unknown command. Stock tclsh's default `unknown` auto-loads a `::greet` from `$auto_path` (I set it with the `TCLLIBPATH` env var only), and that procedure enters the method twice.
- **067:** the receiver `$x` is a global the document never sets. A sourcing caller that sets it makes the site enter `Dog::bark`.

So the effect of each site is not determined by the document. A fail-closed lens must not report absence. Partial with zero hits renders `"References unavailable"` (`code_lens.rs:77-92`).

**Not a weakening of the FP guard.** Under Partial, a count above zero renders `"at least N references"`. So if a regression counted the bare `greet` words or the `$x bark` site, both tests would still fail.

**Residual.** `"References unavailable"` is also the title for `Unavailable` coverage at any count, so the title alone does not pin `Partial`.
- **Optional hardening (non-blocking):** add `assert_eq!(lens.reference_coverage, Some(ReadonlyReferenceCoverage::Partial))`. This adds a strengthening assertion; it changes no existing value.

**Consistency with the census work.** Sol-6's census V2 draft also leaves both tests incomplete (NOTES lines 128–129: "unknown head" / "dynamic head"). It moves only 056/058/059/060 to Complete, matching decision `method_census_contradiction`. The new expected values therefore will not churn when V2 lands. The orchestrator already raised this with the user (`progress-report-20261017T2058Z.json` `scope_flags_for_user[0]`).

### (b) classmethod fixture reorder in `outgoing_calls_does_not_conflate…`

**Correct and minimal.** Under 9.0/9.1 the original order makes `my greet` enter the classmethod: a later `classmethod greet` replaces the earlier `method greet` with the forward `myclass greet`. That contradicts the test's line-1 assertion and its intent. The alternatives do not work:
- Keeping the order means asserting the classmethod (line 2), which inverts the intent.
- Swapping lines 1 and 2 changes the asserted `start_line` to 2.

Luna's layout (classmethod on the header line, line 2 blank) keeps every value: cursor `(3,11)` on `twice`, `::C::twice`, `len 1`, `from_ranges 1`, `start_line 1`. My re-runs confirm `[C new] twice` → `instance-method` and `C greet` → `classmethod` on 9.0.4 and 9.1.0.

**F1 (gate).** This test is **green on i37 today** (capture37 `test-556`, exit 0), because at 8.6 the analyser keeps the class and ignores the classmethod. The rewrite therefore risks turning a green test red, and a gate run must confirm it stays green.
- **Static evidence for green:** sibling `outgoing_calls_separate_same_named_instance_method_and_classmethod` uses the same order (classmethod, then method) at 9.0. On HEAD it resolves `my make` to the later instance method (line 2), as the test-565 log shows: `to ::C::make`, `selection_range.start_line 2`, `from (3,26)`. It fails only on the missing `C make` edge.

**D1 (required, comment-only).** The retained doc text "since `self` inside an instance method is the instance, whose method table never includes classmethods" and "TclOO keeps them in independent tables" is false for Tcl 9.
- `probes/classmethod-only.tcl`: `oo::class create D {classmethod greet …; method twice {} {my greet}}` makes `[D new] twice` return `classmethod` on 9.0.4/9.1.0. `info class forward C greet` returns `myclass greet`.
- Luna's appended sentence corrects the fixture but leaves the false claim above it. Suggested replacement for the doc block:

  ```
  /// Regression: a `method` and a `classmethod` sharing a name must not
  /// collide. In Tcl 9 `classmethod greet` also installs the instance forward
  /// `greet` -> `myclass greet`; the later `method greet` replaces it, so
  /// `my greet` from an instance method enters only the instance method
  /// (tclsh 9.0.4/9.1.0). Matching both by name alone double-counts the
  /// single real call site under one item key. `classmethod` is Tcl 9.0+,
  /// so this analyses at 9.0 and declares the classmethod first.
  ```

**N1 (NOTES accuracy, non-blocking).** NOTES says the `"tcl"` `outgoing_calls` profile matches "the passing sibling tests". However:
- Every `analyse_tcl9` sibling passes `"tcl9.0"`, including the green tests 532 and 533.
- Only the 8.6 siblings pass `"tcl"`.

Keeping `"tcl"` (the lenient profile) is the minimal choice and is acceptable, but the stated rationale is wrong. If the gate shows a difference, `"tcl9.0"` is the consistent alternative.

### (c) `analyse` → `analyse_tcl9`

- **Faithful and minimal.** `classmethod` is 9.0+, and 8.6 rejects the whole class. 9.0 is the lowest version that has the command, and the helper already exists in the module.
- **F2 (expectation, not blocking).** For `prepare_resolves_classmethod_over_same_named_method_by_cursor`, the switch probably does **not** make the test green:
  - `prepare_in_program` first takes `original_declaration::select`, which builds the item in `original_call_hierarchy::item` with `detail: Some("Source declaration")` (`original_call_hierarchy.rs:44`).
  - The asserted `"classmethod of ::C (0 params)"` comes only from the legacy `item_for_method` (`call_hierarchy.rs:440`).
  - Tcl-9 classmethod declarations do reach the original path: test-544's `prepare(…,1,16)` yields `::Factory::make`, and test-565's method item carries `"Source declaration"`.
  - The leftover mismatch is presentation that tclsh cannot observe, so it is outside the user's exception. It must not be "fixed" under this decision. Report it as a separate item.

### (d) The three new nextto tests vs HEAD's `canonicalise_class`

All three match the i37 code (`definition.rs:1333-1357`). The path is `TCLOO_MEMBER_BODY_NAMESPACE_PATH = ["::oo::Helpers"]` in member bodies (`definer.rs:4375`, `scope.rs:773`).

| Test | What HEAD does | Result |
|---|---|---|
| `Ns::A` | Unshadowed; root lookup gives `::Ns::A` | line 2 |
| `::Ns::A` | Absolute; root lookup gives `::Ns::A` | line 2 |
| Helpers fixture | `::oo::Helpers::A` is retained, so `shadowed` | `None` |

These are the positives and the refusal that Sol-4's review asked for. The existing test's original positive intent (namespaced nextto navigation to `::Ns::A::greet`, line 2) survives in #4 and #5. Cursors were verified on `nextto` (`(10,28)` and `(10,22)`).

The refusal test pins a fail-closed `None` where tclsh has a definite answer (line 2). Its comment says so, and any future decision to answer must assert line 2, never line 6. This is acceptable.

## Findings summary

- **D1 (required, comment only):** correct the false Tcl-9 claim in the `outgoing_calls_does_not_conflate…` doc block.
- **F1 (gate):** test-556 is green on i37; confirm the rewritten test stays green.
- **F2 (expectation):** test-567 is likely to stay red on `detail` (`"Source declaration"` vs the legacy string). This is not tclsh-observable and is out of scope for this decision.
- **F3 (gate):** test-124 now reaches the `interp alias` positive fixture for the first time, and test-571 depends on the analyser accepting `namespace eval a {}` creation. Both outcomes on HEAD are unverified.
- **H1 (optional):** pin `reference_coverage == Some(Partial)` in 057/067 so the FP guard cannot hide behind `Unavailable`.
- **N1:** NOTES misstates the sibling profile rationale.

The `NOTES.md` §5 items (Itcl, consumed name) change nothing and were not re-probed.
