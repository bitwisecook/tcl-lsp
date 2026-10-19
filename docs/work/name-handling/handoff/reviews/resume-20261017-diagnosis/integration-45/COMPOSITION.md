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
