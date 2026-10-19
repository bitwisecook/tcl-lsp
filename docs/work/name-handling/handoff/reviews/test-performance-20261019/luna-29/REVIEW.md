# Slow-test review: tcl_compiler, tcl_lsp_core, tcl_lsp_db (Luna-29)

Scope: the 47 tests at 10 s or more in `slow/slow-tests.json` for these three targets (12 + 26 + 9).
Per-test rows are in `REVIEW.json` in this directory.

This review is static. I ran no cargo command and no test binary. Two kinds of evidence are used:
- Sol-28's gdb samples and timings in `agents/sol-28/` (read-only).
- Sol-26's per-test logs in `agents/sol-26/patched/run/*/results.jsonl`.

Caveats on the measured times:
- Rows labelled `sol-26` ran 3-way parallel, so they are upper bounds. Rows labelled `capture47` ran serially, one process per test.
- 18 of the 47 rows are FAIL or TIMEOUT in the current tree. A FAIL time is the time until the failing assertion.
- All builds are debug test builds.

Every proposal below is a proposal. Under the project rule, original assertions, selectors, budgets and fixtures change only with user sign-off. Each light version is proposed as a **new** test. The original keeps its selector and assertion text and stays in the deep tier. The few edits proposed to an original are listed in section 8.

## 1. Totals

| | seconds |
|---|---|
| Measured now (sum of the 47 rows) | **2258** (compiler 1009, core 821, db 427) |
| Quick tier after the proposal (light versions only; 16 originals have no light version until their product cost is fixed) | **~231** |
| Deep tier after test-only changes (originals + new light tests − memoised fresh baselines) | **~2282** |
| Deep tier after the product fixes in section 3 (estimated targets) | **~460** |

What these numbers mean:
- Test-only changes buy fast local iteration. They do not make CI cheaper.
- About 80% of the measured time is product cost. Groups S1–S3 alone are 1839 s.
- CI time comes down only when the product is fixed.

Measured seconds by systemic cause: S1 nesting re-cut 916; S2 interprocedural module cost 466; S3 Logical-path analysis and per-query recapture 456; S5 seeded edit storms 294; S4 pkgIndex parse and rescans 126.

## 2. How the tiers are selected (fits the existing contract)

| Version | Tier | Selected by |
|---|---|---|
| **Light** (new tests, plus the 7 unchanged originals that are already cheap) | new **quick** tier for local iteration, and also deep | a new nextest profile, `[profile.quick]` in `.config/nextest.toml` (draft: `quick-profile.proposal.toml`), with `default-filter = not (<deep-only originals>)`, run by a new `make test-quick` |
| **Deep** (all 47 originals, unchanged) | **deep**: CI `rust-tests-shard` on every PR. Before opening a PR, `make test-rust` locally (the user's "deeper testing just before opening a PR") | no change. CI runs every non-`#[ignore]` test, as it does today |
| Exhaustive | unchanged | only for the seeded edit storm, if the user picks option (a) in section 4 |

Mechanism details:
- **Source of truth for the deep-only set:** one manifest, `scripts/dev/deep-only-tests.tsv` (draft: `deep-only-tests.proposal.tsv`, 40 rows).
  - It holds `package <TAB> exact test path`, so original selectors are untouched.
  - A drift gate in the style of `cargo xtask smoke-targets check` would reject stale rows and regenerate the filter.
  - The smoke manifest is the precedent for "named-test manifest + xtask drift gate".
- **Without nextest**, `make test-quick` falls back to the full suite. That fallback is safe: it is slower, never weaker.
- **Mechanisms rejected:**
  - `#[ignore]`: rule 2 reserves it for permanently expensive or environment-gated tests, and exhaustive never runs before a PR.
  - An env-var gate inside test bodies: a skipped test reads as green.
  - A `cfg` or feature gate: it changes the feature graph and forces rebuilds, against the smoke-tier rule of never forcing a recompile.
- **Smoke tier: no change.** Smoke means one sanity check per crate in seconds, and none of these tests fits that.

Contract changes this needs (in `docs/design/contracts/test-tiers-and-ci-gates.md`):
1. Add a **quick** row to the Tiers table: "local iteration only, deep minus the deep-only manifest; never a gate. `prep-pr` stays smoke."
2. Add a decision rule: every deep-only original keeps its exact selector in the deep tier. A light version is a separate test. A row in the deep-only manifest never removes a test from CI.
3. **Rule 1 conflict, which needs a user decision:** the three `tcl_lsp_db` seeded-random edit storms are fuzz-shaped. Rule 1 says they belong in exhaustive (`#[ignore]`), but today they run in deep. See section 4. Moving them would take their assertions out of every tier that runs before a PR. Keeping them in deep needs an explicit exception to rule 1.

## 3. Systemic causes (ranked by measured seconds)

**S1 — Nested bodies are re-cut at every nesting level (916 s, 7 tests).**
- Each recursive source walker calls `native_script_words_in` on its region. The parse-cut check (`tcl_lexer::parse_cut::first_parse_cut_*` → `body_frame`/`command_part_frame`) descends the whole nested remainder below that region. The walker then recurses one level and repeats.
  - Walkers involved: source-transition `BodyWalk` (`command_binding/source_transition_advice/body_effects.rs`, `MAX_DEPTH 64`), the reachability `Walker`, `SourceStructure::capture_with_purpose` (`tcl-lsp-core/src/source_structure.rs:1090`), and lowering/bindings.
  - The cost is at least quadratic in depth, and is paid separately by each walker.
- Evidence (gdb samples of the nested-wrapper test, `agents/sol-28/samples/base`): 42/46 samples are in `BodyWalk`, and 45/46 in `first_parse_cut`.
- Sol-28's v2 fix shares a `NestedBodyCutMemo` through `SourceOriginId::original_script_words_in`. It takes the test from 182–186 s to 8.7 s with an identical result.
- A cap does not fix this. Below the cap (64 levels), the cost is real: 80 nested namespaces take 53 s, while 300 take 124 s. The time is spent in the analysed prefix, not in the tail beyond the cap. Real code nested 10–20 deep pays part of this cost.

**S2 — Interprocedural module cost grows super-linearly with procedure count (466 s, 8 tests).**
- Measured `logical_unit` (+ `with_interprocedural`) cost by module size:

  | Procedures | Time |
  |---|---|
  | 1 | ~1.4 s |
  | 7 | 31.7 s |
  | 11 | 116.8 s (then FAIL) |
  | 17 | >180 s |

  That is roughly 0.6–1 s × P².
- Code paths that run repeatedly, by reading:
  - `procedures_complete` (`interprocedural/completion.rs:212`) builds a `CompletionWalk::for_module` per procedure. That re-validates the module source owner on each procedure and runs the `original_*_for_module` provenance chain on each command.
  - `seedless_returns` (`interprocedural.rs:5025`) runs one SCCP solve per pure procedure.
  - Callee re-runs (`interprocedural/transfer.rs:800-960`) cost one SCCP solve per (callee, seed) key, bounded by `MAX_RERUN_DEPTH 32` and `MAX_RERUNS 4096`. A new `ModuleProcedures` (with a fresh memo) is built per unit build, per seedless stage and per O103 fold.
  - `global_instance_classes_with` re-captures the original source projection for every command. Sol-28's server samples show it in `with_interprocedural` → `capture_with_nested_cuts`.
- **Hypothesis worth one counter:** `a_name_argument_naming_an_outer_place_is_a_global_write` runs 116 s and then panics because `::c` has no summary (`transfer.rs:2715`). If `ModuleProcedures::spent` reaches `MAX_RERUNS`, the slowness and the missing summary are one defect.
- Next step: `TCL_LSP_TRACE_SOURCE_PHASES=1` (already in `compilation_unit.rs:990`) on 2/4/8/16-procedure modules attributes the phase.

**S3 — Logical-path analysis, compiler-check and per-query cost (456 s, 22 tests).**
- Analyses under the unversioned `"tcl"` dialect, an explicit lexical profile or a supplied `ResolvedAnalysisInput` cost about 4–7 s per tiny document. Versioned dialects cost about 0.9 s per analysis plus query (`native_conditional_publication_navigation_preserves_source_table_refusals`: 15 analyses in 13.3 s).
- The O100 compiler-check/optimiser path on a one-line `if {1}` procedure costs another ~5–10 s (`uncached_compiler_advice_preserves_original_diagnostic_context`: 1 analysis + 1 check pass = 15.9 s).
- Two amplifiers make it worse:
  - (a) **Repeated whole-document analysis inside one product call.** Minify `lexical_compact` re-analyses the source it was just given (`minify.rs:1269`), then re-analyses after the rename (`minify.rs:805`). The optimise loop re-analyses on every iteration.
  - (b) **Whole-document recapture on each cursor query.** `original_variable_reference_at` → `SourceStructure::capture` (`source_structure.rs:1018`), and `references` → `visit_checked_analysis_executable_commands_with_regions` (`references.rs:4398`). One analysis plus 3–4 queries costs 14–23 s.
- Both amplifiers are editor latency: every hover, definition or references request pays (b). The fix is to memoise per `AnalysisResult` (precedent: `capture_count()` predicates in `original_source_projection.rs`).

**S5 — Seeded edit storms recompute identical fresh baselines (294 s, 3 db tests).**
- In each storm, every round builds a fresh database for a text pair that has usually been seen before:
  - `project_diagnostics_incremental_matches_fresh_under_edits`: 6 distinct texts over 60 rounds.
  - `project_diagnostics_incremental_matches_fresh_both_files_edited`: 25 distinct pairs over 80 rounds.
  - `compiler_check_incremental_matches_fresh_under_edits`: at most 144 distinct states over 250 rounds.
- Memoising the fresh result by text keeps the assertion byte-identical and removes 55–90% of the fresh builds.
- These tests are also fuzz-shaped under contract rule 1 (section 2, item 3).

**S4 — pkgIndex parse cost and rescans (126 s, 8 core tests).**
- Each `parse_pkg_index` runs the reachability source survey, at about 1–1.5 s per one-line index in debug.
  - Integration45 doubled `pathological_nesting_terminates` (2.5 s to 4.9 s).
  - `compare.txt` shows reachability tests going from 0.47 s to 2.0 s.
- Tests make it worse by rescanning the same temp directories with fresh resolvers, or by parsing one index twice (`add_original_pkg_index` + `original_package_entries`).

**Stale-cap finding (correctness, not speed).**
- `tcl-lsp-core/src/lib.rs:43` `MAX_SCOPE_WALK_DEPTH = 256` and `declaration.rs:50` `MAX_BODY_DEPTH = 256` say they mirror the compiler analyser's cap. That cap is now `MAX_SOURCE_NEST_DEPTH = 64`.
- `inline_variable.rs:994` records that an unguarded `walk_scopes` overflows a 2 MiB stack at about 100 levels. A 256 cap therefore does not protect a scope tree "built or received some other way". It only works because the analyser stops at 64.
- Stale doc comments in tests say "256": `document_symbols.rs:1396`, `interprocedural.rs:8083`, `package_resolver/tests.rs:172`.
- Proposal: derive the lsp-core caps from `tcl_compiler`'s shared constant. This is a product change.

## 4. Decision needed: the seeded edit storms (rule 1)

`compiler_check_incremental_matches_fresh_under_edits`, `project_diagnostics_incremental_matches_fresh_under_edits` and `project_diagnostics_incremental_matches_fresh_both_files_edited` are seeded-random edit storms. Rule 1 says they belong in exhaustive.

- **Option (a), contract-conforming:**
  - Mark the originals `#[ignore = "seeded edit storm; run explicitly with --ignored"]`.
  - Add **deterministic** fixed-script tests in deep that visit every variant transition and the cascade edges (specified per row in `REVIEW.json`).
  - Cost: the storms' assertions no longer run before a PR.
- **Option (b), keep the originals in deep:**
  - Memoise the fresh baseline by text or state. The assertion text is unchanged.
  - Record an explicit rule-1 exception.
  - The deterministic light tests run in quick.

My recommendation is (b) now and (a) once the deterministic covers have caught a regression in CI at least once.

## 5. Performance properties worth an explicit budget

Use **work-counter budgets** (deterministic, CI-safe), not wall-clock budgets. Each takes two small inputs and asserts linear scaling, so it catches the super-linear regressions that made these tests slow while costing about 1 s each in deep. All four are new tests and need sign-off.
1. **S1 nesting budget.** Count parse-cut body frames entered (a test-visible counter beside the existing `capture_count()`) while parsing `package ifneeded` with 64 and 128 nested `[list …]` wrappers. Assert `count(128) <= 2.2 * count(64)`. Same check for 32 and 64 nested `namespace eval` through `Analyser::analyse`.
2. **S2 procedure-count budget.** For a `logical_unit` with 4 and with 8 independent one-line procedures, count SCCP solves and re-runs (`ModuleProcedures::spent` plus seedless solves). Assert `solves(8) <= 2.2 * solves(4)`.
3. **S3 per-query budget.** On one Logical analysis, run 4 cursor queries (definition and references). Assert at most 1 `SourceStructure`/executable-region capture per analysis.
4. **S3 analysis-count budget.** `minify_with_analysis` at the Compact tier runs at most 2 analyses beyond the caller's own (count analyses through a test-visible counter).

Avoid wall-clock budgets in CI; runner noise would make them flaky. If one is wanted, put a release-mode timing of the 2000-level nested wrapper (8.7 s debug after the fix) in exhaustive.

## 6. Ranked list (largest measured time first)

| # | target | test | measured s (status, source) | class | cause | light version (quick+deep) | light est s | deep after fix est s |
|---|---|---|---|---|---|---|---|---|
| 1 | tcl_compiler | `the_analyser_stops_at_the_shared_cap` | 180.1 (TIMEOUT, sol-26 ×3) | FIX PRODUCT COST | S1 | none: deep-only |  | 3.0 |
| 2 | tcl_compiler | `depth_exhaustion_blocks_dynamic_name_load_forwarding` | 180.1 (TIMEOUT, sol-26 ×3) | FIX PRODUCT COST | S1 | new | 4.0 | 5.0 |
| 3 | tcl_compiler | `a_body_that_may_raise_does_not_complete` | 180.0 (TIMEOUT, sol-26 ×3) | FIX PRODUCT COST | S2 | none: deep-only |  | 5.0 |
| 4 | tcl_lsp_core | `parse_pkg_index_survives_deeply_nested_source_wrapper` | 180.0 (TIMEOUT, capture47) | FIX PRODUCT COST | S1 | new | 1.0 | 8.7 |
| 5 | tcl_lsp_db | `compiler_check_incremental_matches_fresh_under_edits` | 180.0 (TIMEOUT, sol-26 ×3) | SPLIT | S5 | new | 6.0 | 60.0 |
| 6 | tcl_compiler | `deeply_nested_if_survives_interprocedural_scan` | 155.5 (PASS, sol-26 ×3) | SPLIT | S1 | new | 2.0 | 3.0 |
| 7 | tcl_lsp_core | `deeply_nested_namespaces_produce_a_symbol_tree` | 123.5 (PASS, capture47) | SPLIT | S1 | new | 0.5 | 10.0 |
| 8 | tcl_compiler | `a_name_argument_naming_an_outer_place_is_a_global_write` | 116.8 (FAIL, sol-26 ×3) | FIX PRODUCT COST | S2 | none: deep-only |  | 5.0 |
| 9 | tcl_lsp_db | `project_diagnostics_incremental_matches_fresh_both_files_edited` | 59.8 (PASS, sol-26 ×3) | FIX FIXTURE SETUP | S5 | new | 4.0 | 25.0 |
| 10 | tcl_lsp_core | `logical_formal_alpha_requires_isolation_and_closed_binding_observability` | 58.6 (PASS, capture47) | FIX PRODUCT COST | S3 | new | 25.0 | 40.0 |
| 11 | tcl_compiler | `a_callee_counts_as_defined_only_where_its_definition_surely_ran` | 55.6 (FAIL, sol-26 ×3) | SPLIT | S2 | new | 16.0 | 8.0 |
| 12 | tcl_lsp_db | `project_diagnostics_incremental_matches_fresh_under_edits` | 53.8 (PASS, sol-26 ×3) | FIX FIXTURE SETUP | S5 | new | 3.0 | 22.0 |
| 13 | tcl_lsp_core | `deeply_nested_namespaces_survive_scope_walk` | 53.0 (PASS, capture47) | SPLIT | S1 | new | 0.5 | 8.0 |
| 14 | tcl_lsp_core | `checked_potential_source_retains_depth_exhaustion_as_incomplete` | 44.2 (PASS, capture47) | SPLIT | S1 | new | 15.0 | 6.0 |
| 15 | tcl_compiler | `summaries_compose_through_two_callees` | 42.5 (FAIL, sol-26 ×3) | SPLIT | S2 | new | 17.0 | 6.0 |
| 16 | tcl_lsp_db | `summary_memo_observes_callee_edits_through_original_quoted_and_braced_heads` | 42.0 (PASS, sol-26 ×3) | KEEP AS IS | S3 | none: deep-only |  | 8.0 |
| 17 | tcl_lsp_core | `supplied_policy_rewrites_refuse_missing_unavailable_stale_and_foreign_input` | 35.1 (PASS, capture47) | REDUNDANT | S3 | new | 7.0 | 12.0 |
| 18 | tcl_compiler | `a_body_of_commands_that_complete_completes` | 31.7 (FAIL, sol-26 ×3) | FIX PRODUCT COST | S2 | none: deep-only |  | 3.0 |
| 19 | tcl_lsp_core | `exact_require_selects_that_release_or_nothing` | 27.9 (FAIL, sol-26 ×3) | FIX FIXTURE SETUP | S4 | original | 10.0 | 6.0 |
| 20 | tcl_compiler | `emit_cfg_ssa_diagnostics_w220_suppressed_for_substitution_hidden_reads` | 27.3 (FAIL, sol-26 ×3) | FIX PRODUCT COST | S3 | none: deep-only |  | 3.0 |
| 21 | tcl_lsp_core | `original_package_requirement_closure_keeps_versions_and_preference` | 26.9 (PASS, sol-26 ×3) | SPLIT | S4 | new | 9.0 | 12.0 |
| 22 | tcl_lsp_core | `supplied_policy_rewrites_keep_actual_source_config_and_refresh_directives` | 26.8 (FAIL, capture47) | FIX PRODUCT COST | S3 | none: deep-only |  | 5.0 |
| 23 | tcl_lsp_db | `tracked_source_units_share_checked_input_and_memoized_procedure_grammar` | 25.1 (FAIL, sol-26 ×3) | FIX PRODUCT COST | S3 | none: deep-only |  | 5.0 |
| 24 | tcl_lsp_core | `original_logical_declarations_join_global_aliases_without_local_root_stripping` | 23.8 (PASS, capture47) | KEEP AS IS | S3 | none: deep-only |  | 6.0 |
| 25 | tcl_lsp_core | `logical_qualified_workspace_coordinate_uses_the_same_source_scope_owner` | 23.2 (PASS, capture47) | FIX PRODUCT COST | S3 | none: deep-only |  | 4.0 |
| 26 | tcl_lsp_db | `project_source_roles_select_current_call_declaration_and_formals` | 22.4 (PASS, capture47) | SPLIT | S3 | new | 8.0 | 8.0 |
| 27 | tcl_lsp_core | `logical_scope_selector_current_image_and_domain_do_not_grant_edits` | 18.7 (PASS, capture47) | SPLIT | S3 | new | 10.0 | 5.0 |
| 28 | tcl_lsp_db | `callback_arity_foreign_library_source_contexts_do_not_select_local_headers` | 17.6 (PASS, sol-26 ×3) | SPLIT | S3 | new | 7.0 | 6.0 |
| 29 | tcl_lsp_db | `uncached_compiler_advice_preserves_original_diagnostic_context` | 15.9 (FAIL, sol-26 ×3) | FIX PRODUCT COST | S3 | none: deep-only |  | 3.0 |
| 30 | tcl_lsp_core | `logical_math_targets_keep_authentic_expression_and_header_allocations` | 15.4 (PASS, capture47) | KEEP AS IS | S3 | none: deep-only |  | 3.0 |
| 31 | tcl_lsp_core | `logical_scope_selector_preserves_root_current_and_unknown_caller_source_tables` | 15.3 (PASS, capture47) | REDUNDANT | S3 | new | 8.0 | 4.0 |
| 32 | tcl_compiler | `original_completion_keeps_selected_scalar_reference_and_element_obligations` | 15.2 (PASS, sol-26 ×3) | KEEP AS IS | S2 | none: deep-only |  | 4.0 |
| 33 | tcl_lsp_core | `the_result_cap_holds_and_truncates_by_document` | 14.8 (PASS, capture47) | SHRINK INPUT | S3 | new | 2.0 | 4.0 |
| 34 | tcl_lsp_core | `logical_recorded_reads_use_the_same_root_current_and_unknown_source_tables` | 14.5 (FAIL, capture47) | KEEP AS IS | S3 | none: deep-only |  | 3.0 |
| 35 | tcl_compiler | `a_barrier_has_no_summary` | 14.1 (FAIL, sol-26 ×3) | FIX PRODUCT COST | S2 | none: deep-only |  | 2.0 |
| 36 | tcl_lsp_core | `native_conditional_publication_navigation_preserves_source_table_refusals` | 13.3 (PASS, capture47) | SPLIT | S3 | new | 4.5 | 6.0 |
| 37 | tcl_lsp_core | `prefer_latest_selects_the_prerelease` | 12.9 (PASS, sol-26 ×3) | KEEP AS IS | S4 | original | 9.0 | 4.0 |
| 38 | tcl_lsp_core | `signature_subcommands_use_original_values_and_shared_prefix_rules` | 12.7 (PASS, capture47) | SPLIT | S3 | new | 5.0 | 6.0 |
| 39 | tcl_lsp_core | `transitive_closure_pulls_in_tk_through_a_wrapper_package` | 12.7 (PASS, sol-26 ×3) | KEEP AS IS | S4 | original | 9.0 | 4.0 |
| 40 | tcl_lsp_core | `isolated_logical_formal_alpha_uses_general_bindings_and_preserves_defaults` | 12.4 (PASS, capture47) | FIX PRODUCT COST | S3 | none: deep-only |  | 6.0 |
| 41 | tcl_lsp_core | `unconstrained_require_prefers_the_highest_stable_release` | 12.2 (PASS, sol-26 ×3) | KEEP AS IS | S4 | original | 8.0 | 4.0 |
| 42 | tcl_lsp_core | `equal_comparing_providers_all_contribute_their_files` | 11.8 (PASS, sol-26 ×3) | KEEP AS IS | S4 | original | 8.0 | 4.0 |
| 43 | tcl_lsp_core | `original_index_source_operand_grammar_and_real_producers_refuse_guessed_filenames` | 10.7 (FAIL, sol-26 ×3) | SPLIT | S4 | new | 4.5 | 4.0 |
| 44 | tcl_lsp_core | `logical_parameter_hints_keep_nested_and_prefix_ordinals_without_captured_anchors` | 10.6 (PASS, capture47) | KEEP AS IS | S3 | original | 10.6 | 5.0 |
| 45 | tcl_compiler | `a_rebound_or_declared_head_states_no_completion` | 10.5 (FAIL, sol-26 ×3) | KEEP AS IS | S2 | original | 8.0 | 3.0 |
| 46 | tcl_lsp_db | `legacy_project_advice_requires_positive_retained_logical_input` | 10.5 (PASS, sol-26 ×3) | SPLIT | S3 | new | 5.0 | 4.0 |
| 47 | tcl_lsp_core | `original_package_files_keep_source_ordinals_data_and_directory_hints_distinct` | 10.4 (PASS, sol-26 ×3) | SPLIT | S4 | new | 4.5 | 4.0 |

## 7. Per-test detail

Each entry gives the property, where the time goes, and the proposal. The original assertion text is preserved in every case; the only edits proposed to an original are listed in section 8.

### 1. `tcl_compiler::depth_guard::tests::the_analyser_stops_at_the_shared_cap` — 180.1 s, TIMEOUT
- **Location:** `rust/tcl-compiler/src/depth_guard.rs:325`
- **Property:** Analyser::analyse on nested foreach: cap-2 (62) levels => no E207; cap+2 (66) levels => E207 (analyser obeys shared MAX_SOURCE_NEST_DEPTH=64).
- **Cost driver:** Two whole analyses of ~64-deep nested braced bodies. Input is already the minimum (cap +/- 2). Time is super-linear nesting cost in the analyse pipeline (S1: every recursive body/source walk re-cuts the nested body text below it), not fixture size.
- **Classification:** FIX PRODUCT COST (systemic cause S1)
- **Proposal:** Keep assertions and input verbatim (already minimal). Deep-only until S1 is fixed; then it belongs in quick too. Optional perf budget after fix: none needed here; use the dedicated budget test in REVIEW section 5.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none (no test change)

### 2. `tcl_compiler::optimiser::propagation::tests::depth_exhaustion_blocks_dynamic_name_load_forwarding` — 180.1 s, TIMEOUT
- **Location:** `rust/tcl-compiler/src/optimiser/propagation.rs:4667`
- **Property:** 257 nested [list ...] (> MAX_BRACKET_TEXT_DEPTH 256) around `set $name 2`: FunctionUnit.dynamic_names == OPAQUE_SCRIPT and no O100/O102 rewrite to `return 1`.
- **Cost driver:** CompilationUnit::build_for + optimiser over 257-deep bracket nesting. Must exceed 256, so input is minimal for this cap. Each walker that descends a command substitution re-lexes/re-cuts the whole remaining nested text (S1), giving O(depth^2) or worse.
- **Classification:** FIX PRODUCT COST (systemic cause S1)
- **Proposal:** Keep original verbatim in deep. Light (new test, quick+deep): same two assertions with the hidden write behind MAX_SOURCE_NEST_DEPTH+2 (66) nested braced bodies instead of brackets, IF that route also yields OPAQUE_SCRIPT (verify first; if it does not, there is no light variant).
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~4.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** light variant covers a different cap route; bracket-cap regression only caught in deep

### 3. `tcl_compiler::interprocedural::completion::tests::a_body_that_may_raise_does_not_complete` — 180.0 s, TIMEOUT
- **Location:** `rust/tcl-compiler/src/interprocedural/completion.rs:600`
- **Property:** In one 17-procedure module, 16 procedures that may raise (expr, incr, unread var, element/qualified store, expansion, branch, upvar link, arity mismatch, raising callee, direct and mutual recursion) are !completes; `len` completes.
- **Cost driver:** One logical_unit build + with_interprocedural over 17 procs. Per-module interprocedural cost grows roughly quadratically with procedure count in this tree (1-proc builds ~1.4 s, 7 procs 31 s, 11 procs 117 s, 17 procs >180 s); fixture is not large.
- **Classification:** FIX PRODUCT COST (systemic cause S2)
- **Proposal:** Keep verbatim; deep-only until the interprocedural cost is fixed. Do not split now: under the current cost curve five 3-4-procedure modules still cost ~60 s. After the fix, keep as is (one build).
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 4. `tcl_lsp_core::package_resolver::tests::parse_pkg_index_survives_deeply_nested_source_wrapper` — 180.0 s, TIMEOUT
- **Location:** `rust/tcl-lsp-core/src/package_resolver/tests.rs:176`
- **Property:** parse_pkg_index returns (no overflow or hang) for a `package ifneeded` body wrapped in 2000 nested [list ...] words, past MAX_SOURCE_TARGET_SCAN_DEPTH (256) and MAX_BRACKET_TEXT_DEPTH (256).
- **Cost driver:** Reachability survey + source_transition_advice BodyWalk (body_effects.rs MAX_DEPTH 64) re-run native_script_words_in -> parse_cut over the whole nested remainder at each level (sol-28 gdb samples: 42/46 in BodyWalk, 45/46 in first_parse_cut). Sol-28's v2 memo (NestedBodyCutMemo shared through SourceOriginId::original_script_words_in) takes it 182-186 s -> 8.7 s, same result.
- **Classification:** FIX PRODUCT COST (systemic cause S1)
- **Proposal:** Land the sol-28 memo (product fix). Then light (new test, quick+deep): same body at DEPTH = 2*256+2 = 514 (past both caps with a 2x margin). Deep: original 2000 unchanged (it is what catches an uncapped walker whose overflow depth is between ~500 and 2000).
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~1.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** light misses an uncapped walker that overflows only between 514 and 2000 levels

### 5. `tcl_lsp_db::tests::compiler_check_incremental_matches_fresh_under_edits` — 180.0 s, TIMEOUT
- **Location:** `rust/tcl-lsp-db/src/lib.rs:8141`
- **Property:** After every one of 250 seeded-random edits to a 4-slot interprocedural module, memoised compiler_check_diagnostics (checks and optimisations) equals compiler_check_diagnostics_uncached.
- **Cost driver:** 250 iterations x (incremental recompute + full uncached build) of a <=4-procedure document. This is a seeded-random edit storm: rule 1 of the tier contract says it belongs in the exhaustive tier.
- **Classification:** SPLIT (systemic cause S5)
- **Proposal:** Deep and light (new deterministic test): a fixed edit script that visits every variant of every slot from the base state and the cascade edges (pass x caller: passthrough<->constant<->arity-2 under the tainted caller; leaf global<->pure under benign caller; calc variants) — about 30 edits. Memoise `want` per state string. Light = the 8 cascade-edge edits. Original seeded storm: either (a) #[ignore = "seeded edit storm; run explicitly with --ignored"] per contract rule 1 (moves its assertions to exhaustive; needs sign-off, because they would no longer run before a PR), or (b) keep it in deep with `want` memoised by state (identical assertions; <=144 distinct states).
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~6.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** deterministic cover may miss a stale edge that only a long random history reaches; (a) keeps the storm in exhaustive

### 6. `tcl_compiler::interprocedural::tests::deeply_nested_if_survives_interprocedural_scan` — 155.5 s, PASS
- **Location:** `rust/tcl-compiler/src/interprocedural.rs:8094`
- **Property:** build_interprocedural_analysis on a proc containing 1000 nested `if {1}` returns (no overflow) on a 64 MiB thread.
- **Cost driver:** CompilationUnit + interprocedural build over 1000-deep nesting. Lowering caps at MAX_SOURCE_NEST_DEPTH (64), so MAX_INTERPROCEDURAL_WALK_DEPTH (256) is unreachable from source (the doc comment's 256 is stale). Depth beyond 64 only feeds S1 re-cutting upstream.
- **Classification:** SPLIT (systemic cause S1)
- **Proposal:** Light (new test, quick+deep): identical body with DEPTH = 2*MAX_SOURCE_NEST_DEPTH+2 (130), still well past every cap source can reach. Deep: original 1000 unchanged. The 256 walker cap stays covered by the synthetic-IR test deeply_nested_interprocedural_scans_survive (0.37 s).
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~2.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** light misses an upstream walker that overflows only between 130 and 1000 levels; deep keeps it

### 7. `tcl_lsp_core::document_symbols::tests::deeply_nested_namespaces_produce_a_symbol_tree` — 123.5 s, PASS
- **Location:** `rust/tcl-lsp-core/src/document_symbols.rs:1412`
- **Property:** document_symbols returns a non-empty tree for 300 nested `namespace eval` on a 64 MiB thread.
- **Cost driver:** Analysis of 300-deep namespace nesting. The doc comment is stale: it says the analyser cap is 256, but MAX_SOURCE_NEST_DEPTH is 64, so the scope tree is at most ~64 deep and the lsp-core MAX_SCOPE_WALK_DEPTH (256) is never reached. 80 levels cost 53 s (inline_variable test) and 300 cost 124 s, so most of the time is in the first ~64 analysed levels.
- **Classification:** SPLIT (systemic cause S1)
- **Proposal:** Light (new test, ms): build a synthetic Scope tree deeper than MAX_SCOPE_WALK_DEPTH and call the scope_symbols walk directly; this tests the cap the doc says it defends. Deep: original unchanged. Also FIX PRODUCT COST for the ~64-level namespace analysis, and see the stale-cap finding (S5).
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~0.5 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** light does not exercise analyser->symbols integration; deep does

### 8. `tcl_compiler::interprocedural::transfer::tests::a_name_argument_naming_an_outer_place_is_a_global_write` — 116.8 s, FAIL
- **Location:** `rust/tcl-compiler/src/interprocedural/transfer.rs:2696`
- **Property:** Name-argument writes to outer places (::g, upvar #0, global, namespace variable, may-bind under if, unbind via reset) appear in the caller's TransferSummary.globals with the right ExistenceOutcome; ambiguous links have no summary.
- **Cost driver:** One 11-proc logical_unit build + ModuleProcedures::of_unit; currently FAILS at transfer.rs:2715 ('no entry found for key') after 116 s. Callee re-runs (transfer.rs:320 MAX_RERUN_DEPTH 32, :323 MAX_RERUNS 4096) are SCCP solves per (callee, seed) key. HYPOTHESIS: the 4096-rerun budget is being spent, which would explain both the time and the missing summary. Check the `spent` counter.
- **Classification:** FIX PRODUCT COST (systemic cause S2)
- **Proposal:** Keep verbatim. Instrument ModuleProcedures::spent; if the budget is spent, the slowness and the failure are one defect. Deep-only until fixed.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 9. `tcl_lsp_db::tests::project_diagnostics_incremental_matches_fresh_both_files_edited` — 59.8 s, PASS
- **Location:** `rust/tcl-lsp-db/src/lib.rs:9398`
- **Property:** Across 80 seeded edits to caller and callee files, a's cross-file project_diagnostics equals a fresh whole-project rebuild.
- **Cost driver:** 80 rounds x (incremental + fresh db build of 2 files via mk()). Only 25 (a,b) combinations exist, so 55+ fresh builds repeat an earlier result. Fuzz-shaped (rule 1).
- **Classification:** FIX FIXTURE SETUP (systemic cause S5)
- **Proposal:** Memoise mk(a,b) in a HashMap inside the test (assertion text unchanged: `assert_eq!(inc, fresh, ...)`). Light (new deterministic test): a fixed script that visits all 5 b variants with a in [0,1] plus 3 a-only edits. Tier: as for compiler_check_incremental (rule 1 decision).
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~4.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** low

### 10. `tcl_lsp_core::minify::original::tests::logical_formal_alpha_requires_isolation_and_closed_binding_observability` — 58.6 s, PASS
- **Location:** `rust/tcl-lsp-core/src/minify/original.rs:1175`
- **Property:** Logical formal alpha-renaming is refused (empty variable map, names kept) without isolation, and under isolation when an observer exists (unknown command, info locals, global, trace, reassignment).
- **Cost driver:** 6 cases x (analysis_for_profile + lexical_compact: compact_names re-analyses the same source it was just given (minify.rs:1269) + post-rename re-analysis (minify.rs:805) + lexical_default), so 3-4 full analyses per case under an f5-irules-derived profile with a fresh CommandRegistry::build_default() each time, plus an O(registry) builtin_names scan (minify.rs:1297).
- **Classification:** FIX PRODUCT COST (systemic cause S3)
- **Proposal:** Product: reuse the caller's matching AnalysisResult in compact_names instead of re-analysing (saves 1 of ~3 analyses per minify for users too). Test: keep verbatim in deep. Light (new test): the non-isolated case + bodies `mystery $longleft` and `trace add variable ...` (3 cases).
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~25.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** light misses info-locals/global/set observer regressions

### 11. `tcl_compiler::interprocedural::completion::tests::a_callee_counts_as_defined_only_where_its_definition_surely_ran` — 55.6 s, FAIL
- **Location:** `rust/tcl-compiler/src/interprocedural/completion.rs:483`
- **Property:** A callee counts as defined for a caller only when a direct top-level proc statement surely ran before the load may first run the caller (6 run routes, conditional and nested definitions negative; definitions first, callback-only and namespace-eval positive).
- **Cost driver:** ~13 logical_unit builds + with_interprocedural of 2-3 procedure sources (~4 s each in debug). Each source is a distinct load-ordering route.
- **Classification:** SPLIT (systemic cause S2)
- **Proposal:** Light (new test): run routes `main\n` and `set s main\neval $s\n`, the conditional case and the first defined-first case (4 builds). Deep: original unchanged.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~16.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** light misses route-specific regressions (catch, time, namespace eval, substitution)

### 12. `tcl_lsp_db::tests::project_diagnostics_incremental_matches_fresh_under_edits` — 53.8 s, PASS
- **Location:** `rust/tcl-lsp-db/src/lib.rs:9326`
- **Property:** Across 60 seeded edits of the defining file, a's project_diagnostics equals a fresh rebuild.
- **Cost driver:** 60 rounds x (incremental + fresh 2-file build). Only 6 distinct b texts, so 54 fresh builds are repeats. Fuzz-shaped (rule 1).
- **Classification:** FIX FIXTURE SETUP (systemic cause S5)
- **Proposal:** Memoise mk(b_text) (6 builds instead of 60, assertion unchanged). Light (new deterministic test): b cycles 0->1->2->3->4->5->0->3 (every variant plus a revisit and a namespace<->flat edge).
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~3.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** low

### 13. `tcl_lsp_core::refactor::inline_variable::tests::deeply_nested_namespaces_survive_scope_walk` — 53.0 s, PASS
- **Location:** `rust/tcl-lsp-core/src/refactor/inline_variable.rs:1000`
- **Property:** walk_scopes returns on the scope tree of 80 nested namespaces (2 MiB test thread).
- **Cost driver:** Analyser on 80-deep namespace nesting (~53 s; dominated by the first 64 analysed levels). The doc says 'unguarded overflows ~100 levels' and the cap is 256, so the cap would not stop that overflow. Only the analyser's 64 cap keeps the tree shallow (S5).
- **Classification:** SPLIT (systemic cause S1)
- **Proposal:** Light (new test, ms): a synthetic Scope tree of depth MAX_SOURCE_NEST_DEPTH+2 passed to walk_scopes (the deepest tree the analyser can produce) on the default 2 MiB thread. Deep: original unchanged. Fix S5 (align MAX_SCOPE_WALK_DEPTH with the compiler cap) so a synthetic tree deeper than the cap can be added safely.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~0.5 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** light skips the real analyser output shape

### 14. `tcl_lsp_core::executable_regions::checked_potential_source_tests::checked_potential_source_retains_depth_exhaustion_as_incomplete` — 44.2 s, PASS
- **Location:** `rust/tcl-lsp-core/src/executable_regions.rs:1357`
- **Property:** On 258 nested `if {1}` (past MAX_EXECUTABLE_REGION_DEPTH 256, and past the analyser cap 64), the checked visit sees at least one command and reports PotentialSourceVisit::Incomplete.
- **Cost driver:** One analysis of a 258-deep document + the region walk. Two caps can make the visit Incomplete. Which one fires is not pinned. Cost is mostly the analysed prefix (S1).
- **Classification:** SPLIT (systemic cause S1)
- **Proposal:** Light (new test): 66 levels (analyser-cap route); verify it returns Incomplete. Deep: original 258 (region-cap route). If 66 does not give Incomplete, there is no cheap light variant: keep deep-only.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~15.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** light covers only the analyser-cap route

### 15. `tcl_compiler::interprocedural::transfer::tests::summaries_compose_through_two_callees` — 42.5 s, FAIL
- **Location:** `rust/tcl-compiler/src/interprocedural/transfer.rs:2541`
- **Property:** The same composed ParamRole and completion results for bump/reset/twice/renew/clear/maybe/ensure/peek under tcl8.4..9.1.
- **Cost driver:** 5 environments x one 8-procedure build (~8.5 s each). The release axis matters only at 8.4 vs 8.5+ (incr on unset).
- **Classification:** SPLIT (systemic cause S2)
- **Proposal:** Light (new test): environments [tcl8.4, tcl9.1] (the behavioural edge and the newest). Deep: original five.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~17.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** a 8.5/8.6/9.0-only regression is caught in deep only

### 16. `tcl_lsp_db::tests::summary_memo_observes_callee_edits_through_original_quoted_and_braced_heads` — 42.0 s, PASS
- **Location:** `rust/tcl-lsp-db/src/lib.rs:8675`
- **Property:** Salsa proc_summary_cascade runs exactly twice initially, zero on re-query, twice after a callee edit, for callers reaching the callee through quoted and braced heads; summaries change accordingly.
- **Cost driver:** One 3-proc 'tcl' document, ~6 tracked summary probes + one edit. The query counts need the real memoised path, so the fixture is minimal.
- **Classification:** KEEP AS IS (systemic cause S3)
- **Proposal:** Keep verbatim; per-query product cost (S2) only.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 17. `tcl_lsp_core::diagnostic_report::tests::supplied_policy_rewrites_refuse_missing_unavailable_stale_and_foreign_input` — 35.1 s, PASS
- **Location:** `rust/tcl-lsp-core/src/diagnostic_report.rs:607`
- **Property:** optimise_under_policy_from_analysis applies rewrites for a valid supplied analysis and, for missing/unavailable/stale/foreign inputs, returns text unchanged, no applied rows, 0 iterations.
- **Cost driver:** Positive control (up to 3 re-analysis+optimiser iterations) + 2 supplied analyses (tcl8.6, tcl9.1 only to borrow resolved_input). The 4 refusals stop at iteration 0 and are cheap.
- **Classification:** REDUNDANT (systemic cause S3)
- **Proposal:** The positive `!applied.is_empty()` duplicates supplied_policy_rewrites_keep_actual_source_config_and_refresh_directives (which asserts O100 applied on the same source and policy). Light (new test): the 4 refusal checks only, with `foreign` built from ResolvedAnalysisInput::new over the tcl9.1 context (no second analysis). Deep: original unchanged.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~7.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** low (positive path kept by sibling)

### 18. `tcl_compiler::interprocedural::completion::tests::a_body_of_commands_that_complete_completes` — 31.7 s, FAIL
- **Location:** `rust/tcl-compiler/src/interprocedural/completion.rs:573`
- **Property:** Seven straight-line procedures whose commands complete (literal return, empty, string length of parameter/local, arity-correct calls) all complete.
- **Cost driver:** One 7-procedure build + with_interprocedural, 31.7 s. Each procedure is one case, so the fixture is minimal. Cost is the interprocedural per-module curve.
- **Classification:** FIX PRODUCT COST (systemic cause S2)
- **Proposal:** Keep verbatim; deep-only until the interprocedural cost is fixed.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 19. `tcl_lsp_core::package_resolver::tests::exact_require_selects_that_release_or_nothing` — 27.9 s, FAIL
- **Location:** `rust/tcl-lsp-core/src/package_resolver/tests.rs:472`
- **Property:** Oracle-backed `package require -exact` selection: exact release or nothing; trailing-zero spelling; ranged contrast; alpha does not satisfy -exact.
- **Cost driver:** 4 temp pkgIndex dirs; root scanned once, then subsets rescanned by fresh resolvers, ~7-8 parse_pkg_index runs at ~1-1.5 s each (reachability survey). Measured 3-way parallel, so inflated.
- **Classification:** FIX FIXTURE SETUP (systemic cause S4)
- **Proposal:** Parse each pkgIndex once per test: build subset resolvers from the root resolver's parsed entries (or add_original_pkg_index with the in-memory text, as original_names tests do) instead of rescanning. Assertions unchanged. Systemic per-parse cost: FIX PRODUCT COST (S4).
- **Tiers:** light: original (unchanged selector) runs in quick and deep (~10.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** low

### 20. `tcl_compiler::analyser::diagnostics::tests::emit_cfg_ssa_diagnostics_w220_suppressed_for_substitution_hidden_reads` — 27.3 s, FAIL
- **Location:** `rust/tcl-compiler/src/analyser/diagnostics/tests.rs:5676`
- **Property:** W220 (dead store) is suppressed for reads hidden in a command substitution (if condition, expr value, read-modify-write in [...]) and still fires for a genuine overwrite.
- **Cost driver:** 4 full Analyser::analyse calls on 1-3 line procs with the unversioned 'tcl' dialect, about 7 s each. Versioned-dialect analyses of similar size cost ~1 s (see native_conditional_publication_navigation_preserves_source_table_refusals). The full pipeline includes emit_cfg_ssa_diagnostics -> with_interprocedural (sol-28 server samples).
- **Classification:** FIX PRODUCT COST (systemic cause S3)
- **Proposal:** Keep verbatim (4 analyses are the minimum: 3 cases + control). After the fix it fits the quick tier.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 21. `tcl_lsp_core::package_resolver::original_names::tests::original_package_requirement_closure_keeps_versions_and_preference` — 26.9 s, PASS
- **Location:** `rust/tcl-lsp-core/src/package_resolver/original_names.rs:908`
- **Property:** Original requirement advice resolves exact/stable/latest/alternative selections and transitive closure (incl. unknown foreign keys and prefer-raise) identically under 8.6, 9.0, 9.1.
- **Cost driver:** 3 versions x (add_original_pkg_index + original_package_entries parse the same 5-line index twice) + resolutions + transitive walks.
- **Classification:** SPLIT (systemic cause S4)
- **Proposal:** Light (new test): version V9_1 only. Deep: original 3 versions. Also parse the index once per version (FIX FIXTURE SETUP) if the API allows.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~9.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** per-release regression caught in deep only

### 22. `tcl_lsp_core::diagnostic_report::tests::supplied_policy_rewrites_keep_actual_source_config_and_refresh_directives` — 26.8 s, FAIL
- **Location:** `rust/tcl-lsp-core/src/diagnostic_report.rs:557`
- **Property:** A supplied Logical analysis drives the real multipass optimiser (O100 applied, >=2 iterations, retained config/registry), and a `# noqa: O100` directive suppresses it only when the analysis is refreshed.
- **Cost driver:** optimise loop x2 (each iteration re-analyses + optimiser) + 1 extra analysis. One-line source. Per-iteration analysis+optimiser costs ~5 s debug.
- **Classification:** FIX PRODUCT COST (systemic cause S3)
- **Proposal:** Keep verbatim (core rewrite-loop contract). After the fix it belongs in quick.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 23. `tcl_lsp_db::original_tracked_metadata_tests::tracked_source_units_share_checked_input_and_memoized_procedure_grammar` — 25.1 s, FAIL
- **Location:** `rust/tcl-lsp-db/src/original_tracked_metadata_tests.rs:75`
- **Property:** Tracked document units share the checked analysis input, registry context and procedure grammar; memoized compiler checks retain O100 source context; a shifted declaration keeps the same input/grammar.
- **Cost driver:** One-line logical document: input + unit + analysis + compiler_check_diagnostics, then an edit and recompute. compiler_check_diagnostics on a one-line Logical procedure costs ~5-10 s debug (S2).
- **Classification:** FIX PRODUCT COST (systemic cause S3)
- **Proposal:** Keep verbatim.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 24. `tcl_lsp_core::declaration::tests::original_logical_declarations_join_global_aliases_without_local_root_stripping` — 23.8 s, PASS
- **Location:** `rust/tcl-lsp-core/src/declaration.rs:671`
- **Property:** Logical declaration navigation joins `global foo`/`global ::ns::foo` aliases to $foo and $::foo, and keeps a local `set foo` distinct from $::foo.
- **Cost driver:** 3 'tcl' logical analyses + 6 declaration_at_read queries. Per-query whole-document recapture (S3).
- **Classification:** KEEP AS IS (systemic cause S3)
- **Proposal:** Keep verbatim: three fixtures, three defect classes. Gains come from S2/S3 product fixes. Deep-only until then (23.8 s).
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 25. `tcl_lsp_core::definition::logical_variable_scope_owner_tests::logical_qualified_workspace_coordinate_uses_the_same_source_scope_owner` — 23.2 s, PASS
- **Location:** `rust/tcl-lsp-core/src/definition.rs:9582`
- **Property:** qualified_variable_cell_at maps $Target::x under uplevel #0 / 0 / 1 / $::Target::x to ::Target::x, ::N::Target::x, None, ::Target::x, and refuses a stale (edited) source.
- **Cost driver:** 1 'tcl' analysis + 4 live queries (the 4 stale-source queries exit at the byte-equality check in CurrentSourceContext::capture). ~4-5 s per live query. original_variable_reference_at -> SourceStructure::capture recaptures the whole document per query (source_structure.rs:1018).
- **Classification:** FIX PRODUCT COST (systemic cause S3)
- **Proposal:** Keep verbatim. Product: memoise SourceStructure/executable-region captures on the AnalysisResult (one capture per analysis, not per cursor). This is also an editor-latency issue: every hover/definition request pays it.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 26. `tcl_lsp_db::project_token_inputs::tests::project_source_roles_select_current_call_declaration_and_formals` — 22.4 s, PASS
- **Location:** `rust/tcl-lsp-db/src/project_token_inputs/tests.rs:420`
- **Property:** Project semantic tokens mark `retained` as a variable only when the call resolves to the library's upvar-binding `reader` (8 caller variants + duplicate library), in tcl, tcl8.6, jim.
- **Cost driver:** 3 dialects x 9 project semantic-token computations.
- **Classification:** SPLIT (systemic cause S3)
- **Proposal:** Light (new test): dialect `tcl` only (the logical path, which has the most refusal logic). Deep: original 3 dialects.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~8.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** dialect-specific regression caught in deep only

### 27. `tcl_lsp_core::definition::logical_variable_scope_owner_tests::logical_scope_selector_current_image_and_domain_do_not_grant_edits` — 18.7 s, PASS
- **Location:** `rust/tcl-lsp-core/src/definition.rs:9619`
- **Property:** Logical scope advice is refused for changed image, native channel, changed config, out-of-range offset, versioned/foreign dialects (tcl8.6, tcl9.0, jim, f5-iapps), detached input, and rename on stale source.
- **Cost driver:** 1 'tcl' analysis + 4 per-release analyses + rename. The refusals are cheap.
- **Classification:** SPLIT (systemic cause S3)
- **Proposal:** Light (new test): release loop reduced to [tcl8.6, f5-iapps] (one versioned Tcl, one foreign family). Deep: original unchanged.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~10.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** a 9.0- or jim-specific grant regression is caught in deep only

### 28. `tcl_lsp_db::tests::callback_arity_foreign_library_source_contexts_do_not_select_local_headers` — 17.6 s, PASS
- **Location:** `rust/tcl-lsp-db/src/lib.rs:10686`
- **Property:** Callback arity diagnostics never select local proc headers when the supplied input is a foreign closed (bpf) context, across 11 Tk/tcllib callback registrations (plain and package-required).
- **Cost driver:** 11 analyses + several project_callback_diagnostics calls each.
- **Classification:** SPLIT (systemic cause S3)
- **Proposal:** Light (new test): 4 rows: scale -command, smtp::sendmessage -tlspolicy, struct::graph walk, and the `package require struct::tree` row. Deep: original 11.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~7.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** per-library descriptor regression caught in deep only

### 29. `tcl_lsp_db::original_uncached_metadata_tests::uncached_compiler_advice_preserves_original_diagnostic_context` — 15.9 s, FAIL
- **Location:** `rust/tcl-lsp-db/src/original_uncached_metadata_tests.rs:118`
- **Property:** Uncached compiler checks from a supplied analysis retain the O100 original source context (image, lexer config, registry key), and Finding round-trips it.
- **Cost driver:** 1 analysis + one uncached compiler check pass on a one-line procedure. Minimal. The cost is all S2.
- **Classification:** FIX PRODUCT COST (systemic cause S3)
- **Proposal:** Keep verbatim.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 30. `tcl_lsp_core::expr_context::logical_math_source_target_tests::logical_math_targets_keep_authentic_expression_and_header_allocations` — 15.4 s, PASS
- **Location:** `rust/tcl-lsp-core/src/expr_context.rs:647`
- **Property:** math_function_target_at selects both tcl::mathfunc declarations (local, root) for `special(` from retained headers even with all_procs cleared, and refuses stale source and a non-occurrence offset.
- **Cost driver:** 1 'tcl' analysis + 4 queries. Minimal.
- **Classification:** KEEP AS IS (systemic cause S3)
- **Proposal:** Keep verbatim; S2/S3 product fixes are what will speed it up.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 31. `tcl_lsp_core::definition::logical_variable_scope_owner_tests::logical_scope_selector_preserves_root_current_and_unknown_caller_source_tables` — 15.3 s, PASS
- **Location:** `rust/tcl-lsp-core/src/definition.rs:9461`
- **Property:** For uplevel #0/0/1 cursors: definition selects root/current/none; references never include the namespace line 2 (nor root/current lines for unknown caller); eligible scopes exclude Namespace.
- **Cost driver:** 1 'tcl' analysis + 3 x (definition + references). References runs visit_checked_analysis_executable_commands_with_regions over the whole document per query (references.rs:4398).
- **Classification:** REDUNDANT (systemic cause S3)
- **Proposal:** Partial: its references assertions (`!contains(line 2)`; unknown caller has no 0/4) are implied by logical_recorded_reads_use_the_same_root_current_and_unknown_source_tables (same SCOPED fixture, same cursors: `!lines.contains(&2)` and `lines.is_empty()` for the unknown caller). Proposal (sign-off): drop the references block here and keep definition + scope eligibility. Light = that reduced form; deep = original.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~8.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** low

### 32. `tcl_compiler::interprocedural::completion::tests::original_completion_keeps_selected_scalar_reference_and_element_obligations` — 15.2 s, PASS
- **Location:** `rust/tcl-compiler/src/interprocedural/completion.rs:727`
- **Property:** Completion obligations from lexical reads: 5 scalar spellings complete, 4 element/unset reads do not; CompletionWalk::read grammar differs 8.6 vs 9.0 for ${a{b}.
- **Cost driver:** 11 one-procedure logical_unit builds, ~1.4 s each.
- **Classification:** KEEP AS IS (systemic cause S2)
- **Proposal:** Keep verbatim. Each source is one lexical case; merging them into one module would not save time under the current quadratic per-module cost. Revisit (merge positives / negatives into two modules) after the fix. Deep-only for now (15 s).
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 33. `tcl_lsp_core::workspace_symbols::tests::the_result_cap_holds_and_truncates_by_document` — 14.8 s, PASS
- **Location:** `rust/tcl-lsp-core/src/workspace_symbols.rs:645`
- **Property:** symbols_matching honours the result cap and truncates by document order (a capped answer is a prefix of documents).
- **Cost driver:** 10 analyses of 10-procedure documents to build a 100-symbol index. The cap logic needs about 3 documents.
- **Classification:** SHRINK INPUT (systemic cause S3)
- **Proposal:** Light (new test): 3 docs x 3 procs; assert symbols_matching("", 9).len()==9, capped(4).len()==4, uris == {d0,d1}. Deep: original unchanged (or replace it with the light form, with sign-off, since the larger index adds no defect class).
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~2.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none material

### 34. `tcl_lsp_core::definition::logical_variable_scope_owner_tests::logical_recorded_reads_use_the_same_root_current_and_unknown_source_tables` — 14.5 s, FAIL
- **Location:** `rust/tcl-lsp-core/src/definition.rs:9419`
- **Property:** references for uplevel #0/0/1 reads include the declaration and selected read for root/current, are empty for unknown caller, never the namespace line.
- **Cost driver:** 1 'tcl' analysis + 3 references queries (whole-document executable-region walk each).
- **Classification:** KEEP AS IS (systemic cause S3)
- **Proposal:** Keep verbatim (it is the covering test for the partial redundancy above). Quick tier once S3 is fixed.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 35. `tcl_compiler::interprocedural::transfer::tests::a_barrier_has_no_summary` — 14.1 s, FAIL
- **Location:** `rust/tcl-compiler/src/interprocedural/transfer.rs:2609`
- **Property:** Procedures reaching unknown code or frames (opaque, upvar 2, a caller of opaque) have no summary; a plain global write has a MayBind global.
- **Cost driver:** One 4-procedure build (14 s, FAIL now). Minimal fixture.
- **Classification:** FIX PRODUCT COST (systemic cause S2)
- **Proposal:** Keep verbatim; quick tier once the per-module cost is fixed.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 36. `tcl_lsp_core::definition::logical_variable_scope_owner_tests::native_conditional_publication_navigation_preserves_source_table_refusals` — 13.3 s, PASS
- **Location:** `rust/tcl-lsp-core/src/definition.rs:9371`
- **Property:** Definition of `C new` across 5 source-table cases (defined, future, renamed, proc-shadowed, unknown mutator) under tcl8.6/9.0/9.1.
- **Cost driver:** 15 analyses + 15 definition queries of 2-3 line documents (~0.9 s each, versioned dialects).
- **Classification:** SPLIT (systemic cause S3)
- **Proposal:** Light (new test): release tcl9.1 only (5 analyses). Deep: original 3 releases.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~4.5 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** release-specific regression caught in deep only

### 37. `tcl_lsp_core::package_resolver::tests::prefer_latest_selects_the_prerelease` — 12.9 s, PASS
- **Location:** `rust/tcl-lsp-core/src/package_resolver/tests.rs:602`
- **Property:** Oracle: `package prefer latest` selects 1.3b1 on 8.6; stable is untouched; no prerelease means no change.
- **Cost driver:** 2 temp dirs; 3 pkgIndex parses across 2 resolvers (3-way parallel figure).
- **Classification:** KEEP AS IS (systemic cause S4)
- **Proposal:** Keep verbatim; per-parse cost is systemic (S4). Optional: reuse the parsed `s` entry instead of rescanning.
- **Tiers:** light: original (unchanged selector) runs in quick and deep (~9.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 38. `tcl_lsp_core::signature_help::tests::signature_subcommands_use_original_values_and_shared_prefix_rules` — 12.7 s, PASS
- **Location:** `rust/tcl-lsp-core/src/signature_help.rs:650`
- **Property:** Unique prefix `le` (plain, quoted, escaped spellings) resolves `string length` help; ambiguous `l`, dynamic, NUL-containing spellings give none, on tcl8.4..9.1.
- **Cost driver:** 5 dialects x 7 one-line analyses = 35 analyses (~0.36 s each). Oracle (tclsh8.4..9.1, checked here): `string le` -> 3 and `string l` ambiguous on all five, so the release axis does not change the expected answers.
- **Classification:** SPLIT (systemic cause S3)
- **Proposal:** Light (new test): dialects [tcl8.4, tcl9.1] (pre-ensemble and newest). Deep: original five.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~5.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** a middle-release registry wiring regression is caught in deep only

### 39. `tcl_lsp_core::package_resolver::tests::transitive_closure_pulls_in_tk_through_a_wrapper_package` — 12.7 s, PASS
- **Location:** `rust/tcl-lsp-core/src/package_resolver/tests.rs:912`
- **Property:** transitive_available_packages pulls Tk in through a wrapper package's `package require Tk`, and not through a plain package.
- **Cost driver:** 2 resolvers x scan (the second rescans both dirs) + analysis of implementation files.
- **Classification:** KEEP AS IS (systemic cause S4)
- **Proposal:** Keep verbatim; optional fixture reuse (scan once, add the second dir).
- **Tiers:** light: original (unchanged selector) runs in quick and deep (~9.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 40. `tcl_lsp_core::minify::original::tests::isolated_logical_formal_alpha_uses_general_bindings_and_preserves_defaults` — 12.4 s, PASS
- **Location:** `rust/tcl-lsp-core/src/minify/original.rs:1140`
- **Property:** Isolated logical formal alpha renames both formals (incl. default referencing another formal), no refusals, defaults preserved.
- **Cost driver:** One case: analysis_for_profile + lexical_compact (3-4 full analyses, see logical_formal_alpha...).
- **Classification:** FIX PRODUCT COST (systemic cause S3)
- **Proposal:** Keep verbatim; the compact_names analysis reuse removes ~1/3 of the cost.
- **Tiers:** light: none: original runs in deep only until the product fix; deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 41. `tcl_lsp_core::package_resolver::tests::unconstrained_require_prefers_the_highest_stable_release` — 12.2 s, PASS
- **Location:** `rust/tcl-lsp-core/src/package_resolver/tests.rs:553`
- **Property:** Oracle: unconstrained require picks highest stable (1.2 over 1.3b1); prerelease-only loads 1.3b1; vsatisfies accepts 1.3b1 for 1.2.
- **Cost driver:** 3 pkgIndex parses across 2 resolvers.
- **Classification:** KEEP AS IS (systemic cause S4)
- **Proposal:** Keep verbatim (systemic S4).
- **Tiers:** light: original (unchanged selector) runs in quick and deep (~8.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 42. `tcl_lsp_core::package_resolver::tests::equal_comparing_providers_all_contribute_their_files` — 11.8 s, PASS
- **Location:** `rust/tcl-lsp-core/src/package_resolver/tests.rs:783`
- **Property:** Providers whose versions compare equal (1.0, 1.0.0) both contribute files in discovery order; a lower release is excluded; an exact lower constraint selects only it.
- **Cost driver:** 3 pkgIndex parses, one resolver. Minimal.
- **Classification:** KEEP AS IS (systemic cause S4)
- **Proposal:** Keep verbatim (systemic S4).
- **Tiers:** light: original (unchanged selector) runs in quick and deep (~8.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 43. `tcl_lsp_core::package_resolver::original_names::original_index_filename_data_tests::original_index_source_operand_grammar_and_real_producers_refuse_guessed_filenames` — 10.7 s, FAIL
- **Location:** `rust/tcl-lsp-core/src/package_resolver/original_names.rs:1342`
- **Property:** tclIndex `source` option grammar per release (-encoding from 8.5, -nopkg from 9.0, never both) and 9 non-producer/dynamic/shadowed forms refuse guessed filenames, for all TclVersion::ALL.
- **Cost driver:** 5 versions x 12 sources = 60 parses (~0.18 s each).
- **Classification:** SPLIT (systemic cause S4)
- **Proposal:** Light (new test): the 3 option rows on all 5 versions (the release truth table, 15 parses) + the 9 refusal rows on V9_1 only (9 parses). Deep: original unchanged.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~4.5 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** a release-specific refusal regression is caught in deep only

### 44. `tcl_lsp_core::inlay_hints::retained_hint_context_tests::logical_parameter_hints_keep_nested_and_prefix_ordinals_without_captured_anchors` — 10.6 s, PASS
- **Location:** `rust/tcl-lsp-core/src/inlay_hints.rs:2884`
- **Property:** charIndex: parameter hint placed exactly once for plain/nested/alias/escaped `string index`; none after alias-to-length, proc shadow, or dynamic subcommand.
- **Cost driver:** 7 analyses + hint passes on one-liners. Each is a distinct route.
- **Classification:** KEEP AS IS (systemic cause S3)
- **Proposal:** Keep verbatim; quick tier.
- **Tiers:** light: original (unchanged selector) runs in quick and deep (~10.6 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 45. `tcl_compiler::interprocedural::completion::tests::a_rebound_or_declared_head_states_no_completion` — 10.5 s, FAIL
- **Location:** `rust/tcl-compiler/src/interprocedural/completion.rs:652`
- **Property:** A shadowed, renamed or stub-declared `string` head makes `len` not complete.
- **Cost driver:** 4 two-procedure builds (~2.6 s each). Minimal.
- **Classification:** KEEP AS IS (systemic cause S2)
- **Proposal:** Keep verbatim; the quick tier can include it (10.5 s measured 3-way parallel, likely ~7 s serial).
- **Tiers:** light: original (unchanged selector) runs in quick and deep (~8.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** none

### 46. `tcl_lsp_db::original_project_callback_projection_tests::legacy_project_advice_requires_positive_retained_logical_input` — 10.5 s, PASS
- **Location:** `rust/tcl-lsp-db/src/original_project_callback_projection_tests.rs:230`
- **Property:** Project-owned source advice (E003) only for a positively retained Logical input; none for missing, unavailable, default, or any of 9 versioned or foreign dialects relabelled 'tcl'.
- **Cost driver:** 1 'tcl' analysis + 9 dialect analyses.
- **Classification:** SPLIT (systemic cause S3)
- **Proposal:** Light (new test): dialect loop reduced to [tcl8.4, tcl9.1, f5-irules]. Deep: original 9.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~5.0 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** dialect-specific grant regression caught in deep only

### 47. `tcl_lsp_core::package_resolver::original_names::file_candidates::tests::original_package_files_keep_source_ordinals_data_and_directory_hints_distinct` — 10.4 s, PASS
- **Location:** `rust/tcl-lsp-core/src/package_resolver/original_names/file_candidates.rs:334`
- **Property:** Package file candidates keep the real `source` operand (not data or a `puts source` word) and fall back to directory hints, for all TclVersion::ALL.
- **Cost driver:** 5 versions x (2 entry parses + 1 analysis).
- **Classification:** SPLIT (systemic cause S4)
- **Proposal:** Light (new test): versions [V8_4, V9_1]. Deep: original unchanged.
- **Tiers:** light: NEW light test runs in quick and deep; original runs in deep only (~4.5 s); deep: original selector and assertion text unchanged; deep tier (CI on every PR; `make test-rust` locally before opening a PR).
- **Risk of the reduced version:** middle-release regression caught in deep only

## 8. Items that need sign-off (they touch originals)

**Edits to originals (assertion text unchanged):**
1. `exact_require_selects_that_release_or_nothing`: build its subset resolvers from already-parsed entries instead of rescanning. Same applies, optionally, to `prefer_latest_*`, `transitive_closure_*` and `unconstrained_require_*`.
2. The three `tcl_lsp_db` storms: memoise the fresh baseline by text or state. Also the rule-1 decision in section 4.
3. `logical_scope_selector_preserves_root_current_and_unknown_caller_source_tables`: drop the references block, which `logical_recorded_reads_use_the_same_root_current_and_unknown_source_tables` covers (REDUNDANT, partial).
4. `supplied_policy_rewrites_refuse_missing_unavailable_stale_and_foreign_input`: the positive control is covered by `supplied_policy_rewrites_keep_actual_source_config_and_refresh_directives`. Keep it in deep. The light version omits it.
5. `the_result_cap_holds_and_truncates_by_document`: optionally replace it with the 3×3 form (the expected constants change from 100/25/{d0,d1,d2} to 9/4/{d0,d1}).

**New tests, tier and manifest changes:**
6. All new light tests and work-counter budget tests: new selectors.
7. The tier-contract additions in section 2: the quick tier and the deep-only manifest.

**Product changes (not test changes, listed for completeness):**
8. Product fixes S1–S3, and the lsp-core cap alignment.
