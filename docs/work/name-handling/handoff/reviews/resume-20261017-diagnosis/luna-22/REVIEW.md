# Luna-22 review of the Sol-18 v2 patches

**Scope.** Repo `/home/user/tcl-lsp` at HEAD `bfa47846e`, whose source matches `9cc6b4089`. The review was static. I ran no cargo and wrote nothing to the repo.

**Checks run:**
- Each v2 patch passes `git apply --check` against the working tree and with `--cached`.
- I applied each patch to scratch copies of HEAD files, in `luna-22/c221/` and `luna-22/v/`. `rustfmt --edition 2024` on stdin reports every changed `.rs` file clean.
- I re-ran the oracles on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. Scripts and outputs are in `luna-22/probes/`.

| Patch | Verdict |
|---|---|
| compiler221-v2.patch | **ADMIT-WITH-CONDITIONS** |
| vsatisfies-84-v2.patch | **ADMIT-WITH-CONDITIONS** |
| forced-import-assertion-v2.patch | **ADMIT-WITH-CONDITIONS** |

## 1. compiler221-v2 (proof doc and `source_class.rs:1081`)

The v2 test hunk is byte-identical to v1. The doc hunks reword lines 18 and 140 as Luna-20 asked.

**Hash gates. No gate covers the doc's bytes, so nothing needs regenerating.**
- **The only live gate is `make check-name-resolution-proofs`** (`scripts/dev/check-name-resolution-proofs.py`, part of `xtask-check`).
  - For each manifest record it requires these fields to appear in the doc as normalised substrings: `problem_statement`, `question`, `conclusion`, `scope`, each provider `answer`, and the stable ID. It also requires each bound test's body to contain the ID and doc-path comment.
  - It does not hash the doc.
  - I reproduced those checks for `naming.source.original-class-constructor-call` against the HEAD and patched doc and `source_class.rs`. Both report zero errors: line 15 (the conclusion) is unchanged, and the test still carries its comment. The script is `luna-22/c221/check.py`.
- **`corpus-inventory.json` SHA rows** cover only `rust/*/tests/data/{native,authored}*` and the BIG-IP appliance directory. They do not cover docs.
- **Other files mention the doc path, but none is a gate.** No script, xtask or Makefile reads any of them:
  - `rust-validation/current-proof-bindings621.json`: its `source_sha256` is of the test sources, not the doc.
  - `frozen347/.../compiler-focused-consumer-admission347-request.json`: path only.
  - `replay-storage/.../archive-base-files-inventory.json`: a historical 2607-byte snapshot.
  - `storage225/inactive-publication-staging225.jsonl`.
  - `experiments/.../lsp-readonly-cache-source-review/combined-manifest.json`: its `after_sha256` `6e72f466…` equals today's doc SHA, but it is a provenance record of an earlier edit.

  These are retained evidence. Do not regenerate or edit them.
- **Caveat.** `update-name-resolution-proof-index.py --render` does not produce either edited paragraph from the manifest; the manifest's binding text for this test is the generic "Authored source/API control … linked". The doc carries hand-maintained prose, so a future `--render` of this record would drop it. This does not block the patch.

**Oracle.** On 8.6.18, 9.0.4 and 9.1.0 the script returns code 0 and creates `::outer::inner::Widget` with class `::oo::class` and methods `m`. The cited versions are the installed patchlevels.

**Conditions:**
1. Execute the test. If a later assertion fails, it is a separate defect; do not revert the assertion.
2. Run `make check-name-resolution-proofs` in the gated tree. It is Python and cheap.

## 2. vsatisfies-84-v2

**Three-state arity holds.**
- `argv_accepted_in` returns `Option<bool>`. `release_arity` keeps only the releases where the registry decided, through `filter_map`.
- `Condition::holds` maps the result: `Some(true)` falls through, `Some(false)` gives `No`, and `None` gives `Inert`.
- A missing `authoring_query()` produces no entry, so the lookup returns `None` and the condition is `Inert`. That case cannot occur today, because `for_version` always sets the `Tcl` family.
- No path turns `None` into `Unavailable`.

**Alignment with the owner.** `arity_accepts_under` lives in the same `impl ResolvedInvocation` block as the owner.
- It starts from `authored_source_arity()?`, which declines for an unresolved or ambiguous selector, for `Traits::STRUCTURALLY_CHECKED_ARITY`, and when the fallback count is `None`.
- It returns `None` when `source.windows` (`arity_windows`) is not empty.
- Each admitted form recounts through `authored_source_count_for_arity(form.arity)`, and an indeterminate count gives `None`. This is what that function's doc requires.
- When no form is admitted, it uses the spec's common arity, as `pick_form` does.
- Across forms, any `Some(true)` wins; otherwise any `None` gives `None`; otherwise `Some(false)`.
- `TclVersion::ALL` is `[Self; 5]`, so the by-value `filter_map` compiles.

**No regression in practice.** The registry's `VSATISFIES_FORMS` are unchanged: `tcl8.4` is `exact(2)` and `tcl8.5+` is `at_least(2)`.

**Existing assertions are unchanged.**
- The only edit to existing test text is the doc comment of `the_tcllib_head_guard_admits_both_supported_releases`. Its 8.4 `Unavailable` assertion is untouched and is now reached through the argv path.
- The receipt test gains exact `Some(false)` / `Some(true)` assertions.
- Every new row matches my re-probe. On 8.4, `package vsatisfies 8.4 8.4 8.5` raises `wrong # args`. On 8.5 through 9.1 it returns 1.

**Conditions:**
1. Run `reachability::tests`, the registry `package_` tests and the `package_target_survey` tests. They have never run.
2. **Recommended, not blocking.** The helper is public and generic. `pick_form` honours each form's `literal_argument_prefix` and its own `options` (`invocation_options(spec, sub, Some(form), …)`). The helper ignores both and recounts with the selected `semantics.options`. A wrong `Some(false)` would become a definite `Unavailable`. The fail-closed fix is to return `None` when an admitted form has a `literal_argument_prefix` or its own options. Alternatively, say so in the helper's doc. The vsatisfies forms have neither.

**Unrelated, outside the patch.** At `package_.rs:576-585`, the `vsatisfies` comment says `at_least(2)` but the code is `Arity::exact(2)`. The working tree still carries the other agent's debug `eprintln!("CONTROL …")` and a `zz_probe` test in `source_structure/package_target_survey.rs`. They must not ship.

## 3. forced-import-assertion-v2

**(a) Verified.**
- `AnalysisResult::allows_retained_logical_declaration_advice()` (`analyser/types.rs:3134-3136`) is exactly `self.allows_lexical_declaration_advice()`. It reads plain fields.
- In `resolve_called_proc` (`definition.rs:3918`), the first branch returns when advice is disallowed (`:3926-3933`) and the second returns when it is allowed (`:3934-3937`).
- So `definition.rs:3938-4010` is unreachable. That covers `invocation_head_at`/`resolved_definition`, `forced_import_shadows`, `proc_visible_from_namespace`, `proc_visible_via_wildcard_import`, `fallback_proc_by_simple_name` and `only_route_is_a_dead_import`.
- The dead code is only this tail. The helpers stay live elsewhere: `forced_import_shadows` from `references.rs:388`, and `live_import_at` from `definition.rs:3302`. Outside tests, `proc_visible_from_namespace`'s only caller is the dead `:3964`.
- The doc comment at `:3889-3917` describes the dead path.
- Luna-20's resurrect-the-local concern is therefore unreachable through `resolve_called_proc`.

**(b) The vacuity claim holds, but Sol-18's mechanism is slightly wrong.**
- `after_last(src, "    p\n")` is byte 269, the closing `}` of the last `namespace eval dst {…}`. The source is 271 bytes.
- No `command_invocations` range contains byte 269. Those ranges are head and name-word spans (`SignatureCommandInvocation::written(cmd_name, cmd_tok.span, …)`, `commands.rs:906`); the last `namespace eval` head is bytes [242, 251). No declaration name span contains it either.
- So `original_declaration::select` returns `ControlFlow::Continue(())` (`original_declaration.rs:753-758`). It does not pick the `namespace eval` invocation, as Sol-18 says. `resolve_called_proc`'s let-else maps `Continue` to `None`.
- The test therefore returned `None` without making any selection, whatever the implementation did.
- `after_last(…) - 2` is byte 267, the `p` head word, whose invocation range is [267, 268).

**The Native trace at `p` is plausible.**
- A forced import with a full export match does not extend `prior` (`original_namespace_binding.rs:250-254`).
- The forget turns that `Imported` binding into `Missing` (`:279-316`).
- `matching_source_advice_publications` then picks the `::dst::p` header, the bindings `{Missing}` do not agree, nothing is admitted, and the result is `None`.

**Oracle.** All five releases print `origin-after-force=::src::p` and an empty `::dst::*`, and the call raises `invalid command name "p"`.

**Judgment on the offset move: a legitimate narrow correction, not a selector or fixture change.**
- The test path (the selector) is unchanged. The fixture bytes and the `is_none()` assertion are unchanged.
- Only the query coordinate moves, onto the call that the test name and the `call` variable denote. That is the coordinate production callers pass: a cursor on the head word, as in hover, `signature_help` and `code_actions`.
- The original offset was chosen when `call_off` was a lexical position horizon, where any offset after the call worked. On the cursor-selection route, a `None` at `}` is a selection that never happened being reported as a result. That breaks the no-absence-as-completeness rule.
- The new comment documents the move.

**Conditions:**
1. Execute the test.
2. In the tracking doc or commit, record the offset move as a test-input correction, separate from the tclsh assertion correction.
3. Non-vacuity still depends on a positive control on the same route. The `None` at `p` could also come from a refusal: a missing `original_lookup`, or a positioned-table refusal.

## Sibling tests with the same defect (`definition.rs`)

**Group A: `after_last(src, "    <word>\n")` fed to `resolve_called_proc`.** The offset lands on the final `}`, so no selection is possible. The FAILED status comes from `sol-5/dbg/base_def.txt`, an earlier baseline.

| Test (line) | Offset line → query line | Expected | Baseline |
|---|---|---|---|
| `a_forced_import_shadows_the_local_command` (6350) | 6360 → 6361 | `Some(::src::p)` | FAILED |
| `a_forced_import_shadows_when_this_file_holds_no_export_for_the_source` (6583) | 6593 → 6600 | `is_none` | vacuous pass |
| `a_forced_import_does_not_shadow_when_this_file_holds_the_export_list` (6606) | 6614 → 6618 | `Some(::app::helper)` | FAILED |
| `partly_observable_target` helper (6673) | 6675 → 6681 | used by the three rows below | — |
| ↳ `a_forced_import_shadows_when_another_file_holds_the_covering_export` (6692) | | `Some(::src::helper)` | FAILED |
| ↳ `a_forced_import_does_not_shadow_when_the_whole_program_exports_nothing` (6707) | | `Some(::app::helper)` | FAILED |
| ↳ `a_forced_import_from_a_namespace_the_program_cannot_see_still_shadows` (6722) | | `None` | vacuous pass |
| `the_same_document_without_an_oracle_keeps_the_document_only_answer` (6733) | 6739 → 6743 | `Some(::app::helper)` | FAILED |

In `PARTLY_OBSERVABLE`, `after_last(…, "    helper\n")` is byte 266, which is `}`. The `forced_import_shadows` calls at 6595, 6615, 6680 and 6740 use the offset as a horizon, so this defect does not affect them.

**Group B: a different defect.** These tests query `resolve_called_proc` where the fixture has no call site at that position, because the offset was used as a horizon. Moving the offset cannot fix them; they need a fixture change or a different API.

| Test (line) | Query line | Offset | Baseline |
|---|---|---|---|
| `an_unforced_import_onto_an_existing_command_installs_nothing` (6369) | 6381 | `u32::MAX` | FAILED |
| `a_local_defined_after_the_import_is_not_a_conflict_but_does_end_the_alias` (6389) | 6412 → 6417 | `after_last("return LOCAL }")` | FAILED |
| `a_redefinition_ends_a_forced_import_shadow` (6425) | 6432 → 6435, 6440 → 6441 | — (no call at all) | FAILED |

**Not this defect.** `the_real_oracle_does_not_resurrect_an_export_written_after_the_import` (6767) uses the same `after_last`, but only with `proc_visible_via_wildcard_import`, which is horizon-based. Its FAILED status has some other cause, which I did not examine.

**Correction to Luna-20.** Sol-18 is right that the positive control at 6350 does not keep the suite from being vacuous. It also may not pass once its offset is moved to the head. In the matcher's agreement check, an `Imported` binding agrees only when the origin's allocation site equals the selected `::dst::p` header's site, so `::src::p` may be unselectable through that slot. This is unverified and needs a run.

**The sibling sweep needs a user ruling.** It would correct the coordinates of six queries in Group A. Two currently "passing" vacuous tests would then become real checks, and some tests may expose Native-route gaps that need implementation work. Xfails never ship. Group B needs fixture changes.

**Cleanup task, not part of these patches.** Remove or route the dead `resolve_called_proc` tail (`definition.rs:3938-4010`) and its stale doc comment (`:3889-3917`).
