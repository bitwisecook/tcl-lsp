# Integration45 candidate (not admitted)

Base: e4a5aa600 (integration44 admitted at a3da51634). Composed and measured by Sol-26 (`integration45.patch`, hashes in SHA256SUMS):

| Piece | Review | Verdict |
|---|---|---|
| Sol-20 reachability test-helper deletion (user decision 16:00Z) | ../luna-27/ | ADMIT-WITH-CONDITIONS (doc sentences folded into the survey patch) |
| Sol-23 survey-scoped Logical conditional source control v2 (option (a)) | ../luna-27/ (survey-REVIEW-v2.md) | ADMIT-TO-BUILD |
| Sol-22 DB921 merged unit v2 (user decision 16:00Z) + E1/E2 text fixes | ../luna-25/REVIEW-v2.md | ADMIT-TO-BUILD |
| Sol-21 Server102/103 snapshot consistency v2 (user decision 16:00Z) | ../luna-26/ | conditions met |

Coordinator change: `depth_guard::tests::package_survey_conditional_bodies_fit_the_shared_stack_budget` (a new test from the survey patch, never admitted) asserted `over <= cap`; the walk retains the body one level past the cap and refuses its contents (`MAX_SOURCE_NEST_DEPTH.exceeded` is `depth > cap`), so the bound is `cap + 1`.

## Sol-26 measurement (one process per test, 180 s budget)

2,117 selectors compared against the integration44 capture, targeted run 45a, or HEAD a3da51634 run before patching: zero PASS->FAIL; 21 FAIL->PASS (15 core reachability/survey, 2 compiler survey, the DB921 test at tcl-lsp-db lib.rs ~12034, Server102 and Server103). Details: compare.txt, comparison.json.

Timings: `pathological_nesting_terminates` 4.9-5.4 s (2.5 s before; survey descent now reaches the cap); `parse_pkg_index_survives_deeply_nested_source_wrapper` 172.5 s (172.0 s before). DB921 synthetic build timing shows no regression (N=10..80 procs).

Server108 is not part of this candidate. The user-approved fixture change (`server108-fixture.patch`, Salsa-published documents) measured 368.9 / 367.5 / 376.8 s: 215 s of setup (25 publications at about 8.6 s) and 162 s of request, so it cannot meet 180 s; it is kept for the record and not applied.

Owed: `make rust-check` / clippy at the PR stage (user decision 2026-10-18T07:30Z); `cargo xtask owner-resolution` fails identically at HEAD and here (607 drift lines, none added); Explorer durable-field coverage fails at HEAD on an unrelated row.

## Capture47 (build47, HEAD aff99dc7e): NOT ADMITTED

1085/1085 executed with source/provider/ELF guards passing; summary in capture47-summary.json. Against capture46 (integration44): five FAIL->PASS (fixed_source_guard_retains_real_query_and_advertisement_dependencies, the_body_descent_leaves_top_level_declarations_alone, the DB921 test, Server102, Server103) and one PASS->FAIL: `package_resolver::tests::parse_pkg_index_survives_deeply_nested_source_wrapper` timed out at 180.0 s (172.0 s in capture46). Rerun alone on the build47 ELF: pass in 179.3 s. Integration45 adds about 7 s to this test, leaving under 1 s of budget; a profiling fix is required before admission.

## Amendment: nested-cut memo reuse (Sol-28)

`sol-28-nested-cut-memo.patch`: BodyWalk::script (NestedScript) and prepare_source_advice_tape (WholeInput) plan through the origin's existing per-origin NestedBodyCutMemo via `SourceOriginId::original_script_words_in`; the memo records a body only after its full recursive check found no cut, keyed on whole-image span, complete parse config and channel. Review ../luna-32/REVIEW.md: ADMIT-WITH-CONDITIONS, both met: (1) all 54 non-passing compiler selectors fail identically on HEAD and with the change after normalising thread ids, timings, addresses, the filtered count, the patch's own line shifts and one selector's per-process Arc addresses/fingerprint (sol-28-failure-output-compare.txt); (2) new test `one_nested_cut_memo_serves_whole_input_and_nested_script_plans_in_any_order` (5 grammars x Document/NativeValue, BOM cases, three orders) passes. Measured: the nested-wrapper test 182-185 s -> 9.0-9.1 s warm (14.6 s cold); integration45 itself adds no measurable time to it (back-to-back 182.9 s vs 185.0 s for the integration44 code); 318 selectors identical pass/fail.
