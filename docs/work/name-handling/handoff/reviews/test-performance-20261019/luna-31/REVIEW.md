# tcl_explorer slow-test review (luna-31)

Read-only review. No cargo commands were run. Timings come from the sol-26 runs: one process per test, `--exact --test-threads=1`, 3 at a time (`agents/sol-26/{head,patched}/run/explorer`). The proposals change fixtures and selectors, so each one needs user sign-off.

## 1. Systemic cause

Each slow test pays a fixed cost of about 17 s per process the first time the tcl_spectcl bundled pack set loads. run_pipeline (lib.rs:213-224) calls registry_for_dialect = tcl_spectcl::bundled::active_registry_for_dialect (bundled.rs:471). That calls registry_for_dialect -> registry_with_packs(profile, packs()) (bundled.rs:495-504). packs() (bundled.rs:404-411) is a per-process OnceLock that loads and evaluates all 8 EDA .tclspec packs (35,136 lines, 2.0 MB; eda_xilinx is 20,902 lines) from specs/ (discovery.rs:306-311, debug only) in an opt-level-0 build. nextest runs each test in its own process (CI ci.yml:1415, make test-rust, and the sol-26 harness), so the OnceLock never amortises and all 105 tests pay it.

Call chain for every slow test:

- Each test calls `run_pipeline`, directly or through a helper: `data()` in render.rs and view_tree.rs, `wasm_entries()` in wasm_explorer.rs, or a direct call in asm.rs, serialise.rs and lib.rs.
- `run_pipeline` (rust/tcl-explorer/src/lib.rs:213-224) calls `registry_for_dialect`, which is `tcl_spectcl::bundled::active_registry_for_dialect` (lib.rs:60; tcl-spectcl/src/bundled.rs:471-476).
- That calls `registry_for_dialect`, which calls `install::registry_with_packs(profile, packs())` (bundled.rs:495-504; install.rs:69-89).
- `packs()` is a per-process `OnceLock` (bundled.rs:404-411): `core_surfaces::ensure()` followed by `load_from(specs/)`. In debug builds, `specs/` is found through `CARGO_MANIFEST_DIR/../../specs` (discovery.rs:306-311). It holds 8 packs: 35,136 lines and 2.0 MB, with eda_xilinx alone at 20,902 lines.
- The overlay is cached once per profile (tcl-registry/src/cache.rs:244-271), and the 9-dialect test costs about the same as 1-dialect tests, so the per-profile overlay is cheap. The once-per-process `packs()` load is the cost.
- nextest (CI `rust-tests-shard`, ci.yml:1415; `make test-rust`) runs each test in its own process, so 105 tests pay this cost 105 times. Plain `cargo test -p tcl-explorer --lib` would pay it once.

Evidence (all measured, none inferred):

- tests::supplied_analysis_input_controls_the_document_pipeline runs the same run_pipeline_with_inputs (declared surface, interprocedural, memory SSA, deep semantic) on "set value 1" but with an ingress registry, not packs: 0.29-0.31 s for the whole process.
- retained_explorer_source_tests::fixture() builds the unit with the pack-free default_context_registry: retained_explorer_source_keeps_actual_profiles... builds 6 dialect pipelines in 1.94-2.0 s, and the other fixture tests take 0.34-0.56 s.
- bounds_withdraws_if_a_function_loses_its_actual_module_metadata = fixture() (cheap) plus one crate::run_pipeline call: 18.2 s.
- original_document_pipeline_keeps_root_and_later_bom_roles calls run_pipeline for 9 dialects (9 registry overlays) in 17.6 s, about the same as single-call tests (16.9-18). So the per-profile overlay (tcl-registry cache.rs:244-271) is cheap and the once-per-process packs() load dominates.
- Tests that never call run_pipeline (meta, segments, cst, formatters, view_tree unavailable-* tests) take 0.01-0.02 s, so harness and process start-up cost is negligible.
- Regression lead: frozen091/explorer-event091 (docs/design/analysis/name-resolution-proofs/rust-validation/frozen091/explorer-event091/tests.log:69) ran original_event_semantic_view_preserves_conditional_descriptor_without_worker_grant, which calls run_pipeline twice, in 2.21 s in its own debug process. Today that test takes 17.4-17.7 s.

**Fixed cost per test is about 16.9 s.** The input-dependent part is about 0.3 s for a trivial pipeline and about 1-3 s for tests that serialise every view (`serialise_result` includes the optimised re-pipeline and WASM codegen). The pipeline itself (lower, CFG, SSA, interprocedural, memory SSA, deep semantic) is not the problem.

## 2. Proposals (each needs sign-off)

- **P1 — FIX FIXTURE SETUP**: Factor run_pipeline into run_pipeline_with_registry(source, dialect, registry). run_pipeline then just passes &registry_for_dialect(dialect). Add a #[cfg(test)] pub(crate) fn run_pipeline_core(source, dialect) that passes tcl_registry::model::ingress::static_context_for_profile(profile).commands(), which is exactly what tcl_spectcl::install::registry_with_packs returns for an empty pack set (install.rs:76-79). In the test modules (serialise.rs, render.rs, view_tree.rs, wasm_explorer.rs, asm.rs and the lib.rs tests not listed under P2), change `use crate::run_pipeline` to `use crate::run_pipeline_core as run_pipeline`. Assertion text, inputs and dialects stay the same. Every test input is a non-EDA dialect (tcl8.6 x54, f5-irules x8, tcl9.0/9.1 x3 each, jim, tk, unknown), and every EDA pack command is behind required_package. wasm_explorer::tests::wasm_entries already compiles WASM against static_context_for(...).commands(), so this also makes the unit and codegen use the same registry.
- **P2 — KEEP on the real entry point (deep tier)**: Keep 3 tests on the real run_pipeline/pack path so the shipped entry-point wiring stays covered: tests::run_pipeline_top_level_only, tests::unparseable_dialect_does_not_panic and tests::original_document_pipeline_keeps_root_and_later_bom_roles (the 9-dialect sweep). Optional, with sign-off: put them in `mod bundled_pack_path` so a local light run can exclude them with a nextest filter.
- **P3 — FIX PRODUCT COST**: Measure tcl_spectcl::bundled::packs() alone in a debug binary and split the time into read/disk cache, evaluate_pack_cached and merge. Then choose between: (a) [profile.dev.package.<hot crate>] opt-level, following the precedent at Cargo.toml:233-242; (b) a lazy per-pack or per-profile load, so a non-EDA dialect does not evaluate 35k lines of vendor specs (this needs a product owner decision because the bundled tier deliberately carries all 8 packs for every dialect); (c) bisect against frozen091 (2.2 s) to see whether this is a regression. This also speeds up tcl explore, tcl-cli explorer_gui and tcl-mcp in debug.
- **P4 — perf guard**: Add a .config/nextest.toml override: [[profile.default.overrides]] filter = "package(tcl-explorer)", slow-timeout = { period = "20s", terminate-after = 3 }. This uses the existing mechanism; a test that becomes slower than today's pack-path cost is then marked SLOW in CI and killed at 60 s, with no wall-clock assertions in the tests.
- **P5 — smoke (light tier)**: tcl-explorer has no smoke test today. Add one smoke_ test that uses run_pipeline_core on a small proc + if script and checks that serialise_result has non-null ir, cfgPreSsa, sccp and stats with a ::p procedure row. Add its row to scripts/dev/smoke-targets.tsv (the cargo xtask smoke-targets check drift gate requires it). It runs under make smoke / make smoke-p P=tcl-explorer.

Effect on the order of tests and on assertions: none. P1 swaps only the registry argument, and the code path is otherwise identical to production (`run_pipeline_with_inputs`). Coverage of the bundled-pack entry point stays in P2. One gap remains, and it exists today too: no explorer test asserts that an EDA pack command actually resolves through `run_pipeline`. If you want that covered, it would be a new deep test, for example a Vivado dialect resolving one xilinx command.

## 3. Tier placement (user direction, 2026-10-19 10:00Z)

- **smoke**: P5 smoke_ test (<1 s), selected by the nextest smoke profile filter test(/(::|^)smoke/) and smoke-targets.tsv.
- **deep**: All 140 tcl_explorer lib tests, unchanged in number: rust-tests-shard 3 (scripts/dev/rust-test-binary-shards.tsv:142) and make test-rust / make test before a PR. 137 run sub-second to about 3 s on run_pipeline_core; the 3 P2 tests stay on the real pack path (about 17 s each until P3).
- **exhaustive**: none. Do not #[ignore] the pack-path tests: the exhaustive tier never runs before a PR.
- **local_light_iteration**: `cargo nextest run -p tcl-explorer -E "package(tcl-explorer) - test(/bundled_pack_path/)"` after P2, or plain `cargo test -p tcl-explorer --lib`, which uses a single process, so packs() loads once (about 17 s plus the remainder) even today.

All original assertions stay in the deep tier, which runs before every PR. No test moves to the exhaustive tier.

## 4. Totals

- The 105 slow tests take **1903.5 s** today, summed. Estimated after P1 and P2: **180.8 s**. That is a conservative upper bound (each test is estimated as max(0.3, s - 16.9), and the 3 P2 tests stay at 17.5 s). The evidence-based lower bound is about 100 s.
- Whole tcl_explorer target: 1908.2 s summed and 10m39s at P=3 wall today. Estimated after: about 185.5 s summed, under 1 min wall at P=3.
- If P3 brings `packs()` down to about 2 s in debug (the frozen091 level), the total drops to about 134.3 s.

## 5. Per-test classification (105 slow tests)

Every test is either KEEP on the real pack path (P2, 3 tests) or FIX FIXTURE SETUP (P1, 102 tests). None is REDUNDANT enough to delete: once each costs about 0.3-1 s, removing overlaps saves almost nothing, and each test asserts a distinct view or property. SHRINK INPUT does not apply, because every input is already 1-5 lines. The wasm_explorer and serialise_result tests keep about 2-3 s each of real WASM-codegen and all-view cost.

| test | before s | HEAD | property asserted | class | new s | fail bucket |
|---|---|---|---|---|---|---|
| `asm::tests::instructions_carry_per_op_source_range` | 17.5 | FAIL | asm view rows carry per-op sourceLine/range for top-level statements | FIX FIXTURE SETUP | 0.6 | F1 |
| `asm::tests::synthetic_ops_have_null_range` | 18.4 | FAIL | synthetic (compiler-inserted) ops carry a null range | FIX FIXTURE SETUP | 1.5 | F1 |
| `render::tests::a_mapped_line_points_at_the_span_its_row_shows` | 17.5 | PASS | render line map: a mapped text line points at the source span its row shows | FIX FIXTURE SETUP | 0.6 |  |
| `render::tests::aot_text_follows_the_pass_selection` | 19.3 | FAIL | AOT text output follows the semantic-optimisation pass selection | FIX FIXTURE SETUP | 2.4 | F1 |
| `render::tests::aot_text_lists_every_rejected_native_premise` | 19.0 | FAIL | AOT text lists every rejected native premise | FIX FIXTURE SETUP | 2.1 | F1 |
| `render::tests::colour_emits_ansi` | 17.7 | PASS | colour mode emits ANSI escapes | FIX FIXTURE SETUP | 0.8 |  |
| `render::tests::render_all_includes_headers_and_asm` | 18.0 | PASS | render_all emits ir/worldSsa/asm/wasm section headers | FIX FIXTURE SETUP | 1.1 |  |
| `render::tests::render_view_draws_a_tree` | 17.8 | PASS | render_view draws box-tree glyphs, no ANSI when colour off | FIX FIXTURE SETUP | 0.9 |  |
| `render::tests::sccp_text_prints_each_edge_refinement` | 18.5 | FAIL | SCCP text prints each edge refinement (value/type/range/unbound) | FIX FIXTURE SETUP | 1.6 | F1/F2 |
| `render::tests::sccp_text_prints_each_enumerated_loop` | 17.6 | FAIL | SCCP text prints each enumerated loop (exit block, final value) | FIX FIXTURE SETUP | 0.7 | F1/F2 |
| `render::tests::sccp_text_prints_the_route_of_each_cell_update` | 18.4 | FAIL | SCCP text prints the route/answer of each cell update | FIX FIXTURE SETUP | 1.5 | F1/F2 |
| `render::tests::show_filter_limits_views` | 17.5 | PASS | --show filter limits which views render | FIX FIXTURE SETUP | 0.6 |  |
| `render::tests::the_line_map_is_aligned_with_the_rendered_text` | 18.6 | PASS | line map has one entry per rendered text line | FIX FIXTURE SETUP | 1.7 |  |
| `render::tests::wasm_renders_wat_module` | 18.0 | FAIL | wasm view renders a WAT module naming $::top | FIX FIXTURE SETUP | 1.1 | F1 |
| `serialise::original_configured_index_tests::original_structural_completeness_keeps_module_input_and_unknown_null` | 18.5 | PASS | structural index keeps module input; unknown completeness is null, not false (tcl9.1 + jim) | FIX FIXTURE SETUP | 1.6 |  |
| `serialise::retained_explorer_source_tests::bounds_withdraws_if_a_function_loses_its_actual_module_metadata` | 18.2 | PASS | bounds view withdraws (null) when a function unit is swapped for a foreign one | FIX FIXTURE SETUP | 1.3 |  |
| `serialise::retained_explorer_source_tests::remaining_source_range_consumers_refuse_foreign_source_arguments` | 18.1 | PASS | asm/shimmer/gvn/taint/cfgPostSsa/irulesFlow return null for a foreign source argument | FIX FIXTURE SETUP | 1.2 |  |
| `serialise::retained_explorer_source_tests::retained_irules_warning_view_and_statistics_share_the_actual_count` | 17.8 | FAIL | iRules warning view and stats share one count, independent of display dialect label | FIX FIXTURE SETUP | 0.9 | F1 |
| `serialise::tests::annotations_collect_callouts_and_group_by_line` | 19.3 | FAIL | annotations collect callouts grouped by line | FIX FIXTURE SETUP | 2.4 | F1 |
| `serialise::tests::aot_view_carries_the_plan_and_every_rejected_native_premise` | 19.1 | FAIL | AOT view carries the plan and every rejected native premise | FIX FIXTURE SETUP | 2.2 | F1 |
| `serialise::tests::asm_emits_structured_instructions_for_top_level` | 17.6 | FAIL | asm view emits structured top-level instructions | FIX FIXTURE SETUP | 0.7 | F1 |
| `serialise::tests::asm_resolves_jump_targets` | 18.5 | FAIL | asm view resolves jump targets | FIX FIXTURE SETUP | 1.6 | F1 |
| `serialise::tests::bounds_emits_per_function_shape` | 18.3 | FAIL | bounds view emits per-function shape | FIX FIXTURE SETUP | 1.4 | F1 |
| `serialise::tests::cfg_post_ssa_has_phis_uses_defs_and_analysis` | 18.1 | FAIL | cfgPostSsa carries phis, uses/defs and analysis | FIX FIXTURE SETUP | 1.2 | F1 |
| `serialise::tests::cfg_post_ssa_optimised_present_when_source_changes` | 18.9 | FAIL | cfgPostSsaOptimised present when optimiser rewrites source | FIX FIXTURE SETUP | 2.0 | F1 |
| `serialise::tests::cfg_pre_ssa_has_entry_block_and_branch_terminator` | 18.6 | PASS | cfgPreSsa has entry block and branch terminator | FIX FIXTURE SETUP | 1.7 |  |
| `serialise::tests::completion_terminator_preserves_pending_native_return` | 19.3 | PASS | completion terminator preserves pending native return route | FIX FIXTURE SETUP | 2.4 |  |
| `serialise::tests::dataflow_emits_nodes_and_summary` | 18.1 | PASS | dataflow view emits nodes and def/use summary | FIX FIXTURE SETUP | 1.2 |  |
| `serialise::tests::dead_stores_surface_from_o109` | 18.5 | FAIL | O109 dead store surfaces in cfgPostSsa analysis, stats and callouts | FIX FIXTURE SETUP | 1.6 | F1/F2 |
| `serialise::tests::durable_compiler_views_are_present_and_cover_extra_units` | 17.8 | FAIL | durable views (dominators/sccp/liveness/semantic) present and cover methods/apply bodies; memory SSA for procs | FIX FIXTURE SETUP | 0.9 | F2 |
| `serialise::tests::event_order_applies_outer_priority_until_it_changes` | 18.9 | FAIL | iRules eventOrder applies outer priority until it changes | FIX FIXTURE SETUP | 2.0 | F1/F2 |
| `serialise::tests::event_order_empty_for_plain_tcl` | 19.4 | FAIL | eventOrder is [] (not null) for plain Tcl | FIX FIXTURE SETUP | 2.5 | F1 |
| `serialise::tests::event_order_ignores_non_braced_event_bodies` | 18.9 | FAIL | eventOrder ignores non-braced event bodies | FIX FIXTURE SETUP | 2.0 | F1/F2 |
| `serialise::tests::event_order_orders_when_handlers_by_firing_order` | 18.7 | FAIL | eventOrder orders when-handlers by firing order | FIX FIXTURE SETUP | 1.8 | F1/F2 |
| `serialise::tests::event_order_tie_break_increments_offset` | 18.9 | FAIL | eventOrder tie-break by source offset | FIX FIXTURE SETUP | 2.0 | F1/F2 |
| `serialise::tests::event_order_uses_rooted_normalised_top_level_handlers_only` | 18.6 | FAIL | eventOrder uses only rooted, normalised top-level handlers | FIX FIXTURE SETUP | 1.7 | F1/F2 |
| `serialise::tests::gvn_abstains_without_exact_common_invocation_provenance` | 19.5 | FAIL | GVN abstains without exact common invocation provenance | FIX FIXTURE SETUP | 2.6 | F1 |
| `serialise::tests::interproc_summarises_procs_with_arity_and_return_shape` | 18.9 | FAIL | interproc summarises procs with arity and return shape | FIX FIXTURE SETUP | 2.0 | F2 |
| `serialise::tests::interproc_view_shows_the_param_constant_seed_and_its_withdrawal` | 21.2 | FAIL | interproc view shows param-constant seed and its withdrawal | FIX FIXTURE SETUP | 4.3 | F2 |
| `serialise::tests::intervals_reports_bounded_loop_counter` | 18.3 | FAIL | intervals view reports bounded loop counter | FIX FIXTURE SETUP | 1.4 | F1/F2 |
| `serialise::tests::ir_expands_if_clauses_and_collects_procs` | 19.0 | FAIL | IR view expands if clauses and collects procs | FIX FIXTURE SETUP | 2.1 | F2 |
| `serialise::tests::ir_rows_keep_equal_names_and_distinct_source_bodies` | 18.8 | FAIL | IR rows keep duplicate proc names with distinct bodies/ranges | FIX FIXTURE SETUP | 1.9 | F2 |
| `serialise::tests::ir_widens_braced_condition_range_to_closer` | 18.3 | PASS | IR braced condition range widened to the closing brace | FIX FIXTURE SETUP | 1.4 |  |
| `serialise::tests::irules_flow_empty_for_plain_tcl` | 17.6 | FAIL | irulesFlow is [] (not null) for plain Tcl | FIX FIXTURE SETUP | 0.7 | F1 |
| `serialise::tests::loops_detects_for_loop_with_depth` | 19.2 | PASS | loops view detects a for loop with depth | FIX FIXTURE SETUP | 2.3 |  |
| `serialise::tests::loops_empty_for_straightline_code` | 18.3 | PASS | loops view empty for straight-line code | FIX FIXTURE SETUP | 1.4 |  |
| `serialise::tests::no_dead_stores_when_value_is_read` | 17.9 | FAIL | no dead store reported when the value is read | FIX FIXTURE SETUP | 1.0 | F1 |
| `serialise::tests::optimisations_reports_constant_fold_with_range` | 18.8 | FAIL | optimisations view reports constant fold with range | FIX FIXTURE SETUP | 1.9 | F1 |
| `serialise::tests::optimised_ir_and_cfg_present_when_source_changes` | 18.1 | FAIL | irOptimised/cfgPreSsaOptimised present when source changes | FIX FIXTURE SETUP | 1.2 | F1 |
| `serialise::tests::optimised_source_null_when_unchanged` | 18.7 | PASS | optimisedSource and double-pipeline keys null when unchanged | FIX FIXTURE SETUP | 1.8 |  |
| `serialise::tests::optimised_source_reflects_constant_fold` | 18.2 | FAIL | optimisedSource reflects constant fold (puts 3) | FIX FIXTURE SETUP | 1.3 | F1 |
| `serialise::tests::optimiser_passes_lists_every_pass_in_order` | 18.0 | FAIL | optimiserPasses lists every pass in order | FIX FIXTURE SETUP | 1.1 | F1 |
| `serialise::tests::optimiser_passes_preserve_set_only_tk_profile` | 18.0 | FAIL | optimiserPasses preserved under set-only tk profile | FIX FIXTURE SETUP | 1.1 | F1 |
| `serialise::tests::original_event_semantic_view_preserves_conditional_descriptor_without_worker_grant` | 17.7 | PASS | semantic view keeps conditional event descriptor; ordinary ::when proc has none (f5-irules x2) | FIX FIXTURE SETUP | 0.8 |  |
| `serialise::tests::rendered_properties_flags_slash_and_dash` | 17.5 | FAIL | rendered-properties flags STARTS_WITH_SLASH/DASH | FIX FIXTURE SETUP | 0.6 | F1 |
| `serialise::tests::sccp_reports_each_statements_completion_paths` | 18.7 | FAIL | SCCP view reports each statement completion paths | FIX FIXTURE SETUP | 1.8 | F2 |
| `serialise::tests::sccp_reports_each_statements_route_and_answer` | 18.0 | FAIL | SCCP view reports each statement route and answer | FIX FIXTURE SETUP | 1.1 | F2 |
| `serialise::tests::sccp_reports_folded_types` | 19.6 | FAIL | SCCP view reports folded types | FIX FIXTURE SETUP | 2.7 | F2 |
| `serialise::tests::sccp_reports_the_route_tally` | 19.9 | FAIL | SCCP view reports the per-function route tally | FIX FIXTURE SETUP | 3.0 | F2 |
| `serialise::tests::sccp_reports_the_selection` | 18.4 | FAIL | SCCP view reports the switch selection | FIX FIXTURE SETUP | 1.5 | F2 |
| `serialise::tests::sccp_reports_the_unreached_arms` | 18.2 | FAIL | SCCP view reports unreached arms | FIX FIXTURE SETUP | 1.3 | F2 |
| `serialise::tests::semantic_optimisations_view_carries_every_pass_and_its_state` | 17.8 | FAIL | semantic-optimisations view carries every pass and its state | FIX FIXTURE SETUP | 0.9 | F1 |
| `serialise::tests::serialise_result_includes_meta_and_ir` | 17.6 | PASS | serialise_result includes meta and ir | FIX FIXTURE SETUP | 0.7 |  |
| `serialise::tests::shimmer_reports_intrep_conversion` | 17.3 | FAIL | shimmer view reports intrep conversion | FIX FIXTURE SETUP | 0.4 | F1 |
| `serialise::tests::source_map_maps_lines_to_byte_ranges` | 17.9 | PASS | sourceMap maps lines to byte ranges | FIX FIXTURE SETUP | 1.0 |  |
| `serialise::tests::stats_reports_function_and_block_counts` | 18.2 | FAIL | stats reports function and block counts | FIX FIXTURE SETUP | 1.3 | F1 |
| `serialise::tests::structural_index_flags_incomplete_script` | 17.8 | PASS | structural index flags an incomplete script | FIX FIXTURE SETUP | 0.9 |  |
| `serialise::tests::structural_index_reports_boundaries_and_balance` | 17.4 | PASS | structural index reports boundaries and brace balance | FIX FIXTURE SETUP | 0.5 |  |
| `serialise::tests::taint_replay_availability_is_separate_from_empty_warning_arrays` | 17.5 | FAIL | taint replay availability distinct from empty warning arrays | FIX FIXTURE SETUP | 0.6 | F1 |
| `serialise::tests::taint_reports_eval_sink` | 18.1 | FAIL | taint view reports eval sink | FIX FIXTURE SETUP | 1.2 | F1 |
| `serialise::tests::taint_tracking_marks_exec_tainted_vars` | 17.6 | PASS | taint tracking marks exec-tainted vars | FIX FIXTURE SETUP | 0.7 |  |
| `serialise::tests::types_reports_known_int_for_constant_assignments` | 18.1 | FAIL | types view reports known int for constant assignments | FIX FIXTURE SETUP | 1.2 | F1 |
| `serialise::tests::unit_scope_reports_a_registry_declared_boundary` | 17.9 | PASS | unitScope reports PROVIDES_PACKAGE boundary; seeding declined | FIX FIXTURE SETUP | 1.0 |  |
| `serialise::tests::unit_scope_reports_uniform_literals_and_no_boundary` | 18.2 | FAIL | unitScope reports uniform literal seeds and no boundary | FIX FIXTURE SETUP | 1.3 | F2 |
| `serialise::tests::wasm_view_exposes_canonical_semantic_plan_and_serialised_output` | 17.6 | FAIL | wasm view exposes canonical semantic plan and serialised output | FIX FIXTURE SETUP | 0.7 | F1 |
| `serialise::tests::wasm_view_exposes_common_native_i64_selection_evidence` | 18.2 | FAIL | wasm view exposes native i64 selection evidence | FIX FIXTURE SETUP | 1.3 | F1 |
| `serialise::tests::wasm_view_exposes_the_native_tier_lowering_record` | 17.2 | FAIL | wasm view exposes native-tier lowering record | FIX FIXTURE SETUP | 0.3 | F1 |
| `serialise::tests::wasm_view_exposes_typed_semantic_decline` | 17.3 | FAIL | wasm view exposes typed semantic decline | FIX FIXTURE SETUP | 0.4 | F1 |
| `serialise::tests::wasm_view_serialises_multiple_mixed_regions_in_node_order` | 17.7 | FAIL | wasm view serialises mixed regions in node order | FIX FIXTURE SETUP | 0.8 | F1 |
| `serialise::tests::world_ssa_decline_preserves_transition_policy_evidence` | 17.9 | FAIL | worldSsa decline preserves transition-policy evidence | FIX FIXTURE SETUP | 1.0 | F1 |
| `serialise::tests::world_ssa_view_keeps_versions_transitions_and_typed_availability` | 17.7 | FAIL | worldSsa view keeps versions, transitions, typed availability | FIX FIXTURE SETUP | 0.8 | F1 |
| `tests::deep_pipeline_preserves_the_actual_source_metadata_owner` | 17.8 | FAIL | every snapshot (top/proc/body unit) keeps the source metadata owner; proc has world-state SSA | FIX FIXTURE SETUP | 0.9 | F2 |
| `tests::original_document_pipeline_keeps_config_withdrawal_terminal` | 17.5 | FAIL | retained source withdraws once lexer config or source text is altered | FIX FIXTURE SETUP | 0.6 | F1 |
| `tests::original_document_pipeline_keeps_root_and_later_bom_roles` | 17.6 | FAIL | 9-dialect sweep: root vs later BOM handling per grammar via run_pipeline | KEEP (real pack path, deep only) | 17.5 | F1 |
| `tests::run_pipeline_collects_procedures_in_order` | 17.7 | FAIL | run_pipeline snapshots: ::top first then procs sorted | FIX FIXTURE SETUP | 0.8 | F2 |
| `tests::run_pipeline_top_level_only` | 17.2 | PASS | run_pipeline on a proc-free script yields only ::top | KEEP (real pack path, deep only) | 17.5 |  |
| `tests::snapshots_retain_duplicate_procedures_and_named_top_separately` | 18.0 | FAIL | snapshots keep duplicate procs and a named ::top proc separately | FIX FIXTURE SETUP | 1.1 | F2 |
| `tests::unparseable_dialect_does_not_panic` | 16.9 | PASS | run_pipeline on an unknown dialect does not panic | KEEP (real pack path, deep only) | 17.5 |  |
| `view_tree::original_configured_completeness_tests::original_structural_view_keeps_unknown_completeness_distinct_from_false` | 17.9 | PASS | structural tree view keeps unknown completeness distinct from false | FIX FIXTURE SETUP | 1.0 |  |
| `view_tree::tests::a_hint_only_optimisation_is_not_rendered_as_a_rewrite` | 17.6 | FAIL | hint-only optimisation not rendered as a rewrite | FIX FIXTURE SETUP | 0.7 | F1 |
| `view_tree::tests::cfg_view_lists_blocks_with_entry_tag` | 17.5 | PASS | cfg tree view lists blocks with entry tag | FIX FIXTURE SETUP | 0.6 |  |
| `view_tree::tests::empty_analysis_views_have_a_dim_note` | 17.0 | PASS | empty analysis views carry a dim note | FIX FIXTURE SETUP | 0.3 |  |
| `view_tree::tests::every_tree_descriptor_has_a_shared_builder` | 17.4 | FAIL | every Tree descriptor renders non-empty for set x 1 | FIX FIXTURE SETUP | 0.5 | F1 |
| `view_tree::tests::ir_view_has_top_level_and_proc_nodes` | 17.4 | FAIL | IR tree view has top-level and ::f nodes | FIX FIXTURE SETUP | 0.5 | F2 |
| `view_tree::tests::loops_view_reports_a_loop` | 18.2 | PASS | loops tree view reports a loop | FIX FIXTURE SETUP | 1.3 |  |
| `view_tree::tests::rows_that_display_a_range_also_carry_the_structured_one` | 17.8 | FAIL | rows that display a range carry the structured range | FIX FIXTURE SETUP | 0.9 | F1 |
| `view_tree::tests::the_structured_range_matches_the_payload` | 17.5 | PASS | structured range matches payload | FIX FIXTURE SETUP | 0.6 |  |
| `view_tree::tests::unknown_view_is_empty` | 17.1 | PASS | unknown view id builds an empty tree | FIX FIXTURE SETUP | 0.3 |  |
| `view_tree::tests::world_ssa_view_explains_availability_and_transition_policy` | 17.4 | PASS | worldSsa tree explains availability/transition policy | FIX FIXTURE SETUP | 0.5 |  |
| `wasm_explorer::tests::branches_and_blocks_pair_up` | 18.1 | PASS | wasm explorer: br/block pairs match | FIX FIXTURE SETUP | 1.2 |  |
| `wasm_explorer::tests::calls_resolve_a_target` | 17.2 | PASS | wasm explorer: calls resolve a target | FIX FIXTURE SETUP | 0.3 |  |
| `wasm_explorer::tests::module_header_carries_every_contract_field` | 17.7 | PASS | wasm explorer: module header carries every contract field | FIX FIXTURE SETUP | 0.8 |  |
| `wasm_explorer::tests::module_header_then_functions` | 17.7 | PASS | wasm explorer: module header row precedes function rows | FIX FIXTURE SETUP | 0.8 |  |
| `wasm_explorer::tests::module_types_cover_imports_and_defined_functions` | 17.4 | PASS | wasm explorer: module types cover imports and defined functions | FIX FIXTURE SETUP | 0.5 |  |
| `wasm_explorer::tests::source_function_rows_preserve_duplicate_names_and_named_top` | 18.2 | FAIL | wasm explorer: duplicate proc names and named ::top kept as separate rows | FIX FIXTURE SETUP | 1.3 | F2 |

## 6. The 73 tests failing at HEAD, grouped by cause (static analysis of the HEAD panic sites)

HEAD and the patched run both fail the same 73 tests. The 68 slow failures give no signal until F1/F2 are fixed. After P1 they fail in under 1 s each instead of about 18 s, which makes fixing them cheaper.

- **F1** (41 tests): Retained-source withdrawal: run_pipeline output has no ir_module.source_metadata_input, so retained_explorer_source() (serialise.rs:102-130, the `?` at :117) returns None and every retained view serialises null/unavailable.
- **F2** (19 tests): Definitions not lowered: proc / oo::class method / iRules when bodies are lowered as plain IRCall (e.g. "call proc p  return FIRST"), so procedure/method/handler rows are missing. Also seen on the fast fixture-built path (retained_event_order_* returns 0 rows), so the cause is in tcl-compiler lowering rather than in run_pipeline alone.
- **F1/F2** (10 tests): Symptom fits both F1 and F2; which one fails first depends on the input.
- **F3** (1 tests): A supplied ResolvedAnalysisInput is still not kept on the module (lib.rs:320: source_metadata_input is None). This is the run_pipeline_with_analysis_input path that the LSP uses, so it is a likely product bug and probably the same root as F1.
- **F4** (2 tests): Inventory or contract drift that does not depend on the pipeline: ARTIFACT_COVERAGE lacks CompilationUnit::source_class_facts and related fields (luna-25 already flagged this), and remaining_actual_cu_views expects every view to be an array while taintReplays is now an object {scope,status,complete,...}.

Direct evidence for F1:
- lib.rs:403 and serialise.rs:3986 fail on `retained_explorer_source(..).is_some()`.
- lib.rs:369 fails on `source_metadata_input.unwrap()`.
- Views come back `Null` where `[]` is expected (event_order_empty_for_plain_tcl, irules_flow_empty_for_plain_tcl, stats_reports_function_and_block_counts, no_dead_stores_when_value_is_read).
- world_ssa_decline_preserves_transition_policy_evidence reports `context-unavailable` instead of `world-state-declined`.

Likely mechanism for F1: `run_pipeline` builds with `CompilationUnit::build_with_options` and no analysis input (lib.rs:278-281). That selects `OriginalSourceBuildIngress::DefaultAuthoring` (tcl-compiler/src/compilation_unit.rs:3842-3843) and the standalone lowerer (compilation_unit.rs:2777-2781). The module therefore never gets the `source_metadata_input` that `retained_explorer_source` requires.

F3 suggests that even a supplied input is dropped, so the root is probably in tcl-compiler, and the 313 tcl_compiler failures at HEAD may share it. This is a hypothesis. A cheap discriminating probe, once P1 makes each test take about 0.3 s: build `"proc p {} {}"` through `fixture()` and through `run_pipeline_core`, then compare `ir_module.procedures.len()` and `source_metadata_input.is_some()`.

Direct evidence for F2: `IRCall "call proc p  return FIRST"` in the ir_rows log, `procs.contains("::alpha")` failing, "the procedure's view" missing in all 6 sccp_reports_* tests, `::C::m` missing, and 0 rows for `when HTTP_REQUEST {}` even on the fixture path.

| test | HEAD s | bucket | panic site | message |
|---|---|---|---|---|
| `asm::tests::instructions_carry_per_op_source_range` | 18.357 | F1 | rust/tcl-explorer/src/asm.rs:389:14 | asm view is an array |
| `asm::tests::synthetic_ops_have_null_range` | 18.43 | F1 | rust/tcl-explorer/src/asm.rs:451:14 | called `Option::unwrap()` on a `None` value |
| `coverage::tests::every_durable_field_is_reviewed_in_the_coverage_table` | 0.101 | F4 | rust/tcl-explorer/src/coverage.rs:264:13 | assertion `left == right` failed: CompilationUnit::source_class_facts must have exactly one ARTIFACT_COVERAGE destination or exclusion |
| `render::tests::aot_text_follows_the_pass_selection` | 18.538 | F1 | rust/tcl-explorer/src/render.rs:358:14 | the native addition's node |
| `render::tests::aot_text_lists_every_rejected_native_premise` | 18.631 | F1 | rust/tcl-explorer/src/render.rs:370:9 | === aot === |
| `render::tests::sccp_text_prints_each_edge_refinement` | 18.02 | F1/F2 | rust/tcl-explorer/src/render.rs:311:9 | === sccp === |
| `render::tests::sccp_text_prints_each_enumerated_loop` | 17.74 | F1/F2 | rust/tcl-explorer/src/render.rs:341:9 | === sccp === |
| `render::tests::sccp_text_prints_the_route_of_each_cell_update` | 18.488 | F1/F2 | rust/tcl-explorer/src/render.rs:271:9 | === sccp === |
| `render::tests::wasm_renders_wat_module` | 18.12 | F1 | rust/tcl-explorer/src/render.rs:462:9 |   unavailable |
| `serialise::retained_explorer_source_tests::remaining_actual_cu_views_retain_profiles_and_ignore_display_labels` | 0.543 | F4 | rust/tcl-explorer/src/serialise.rs:5926:17 | tcl8.4/taintReplays: {"scope":"command-heads-and-stored-callbacks","status":"analysed","complete":true,"unavailableReplays":[]} |
| `serialise::retained_explorer_source_tests::retained_event_order_uses_the_custom_full_config_and_actual_command_store` | 0.389 | F2 | rust/tcl-explorer/src/serialise.rs:5691:9 | assertion `left == right` failed |
| `serialise::retained_explorer_source_tests::retained_explorer_views_refuse_missing_changed_and_foreign_source_owners` | 0.404 | F2 | rust/tcl-explorer/src/serialise.rs:5780:9 | assertion `left == right` failed |
| `serialise::retained_explorer_source_tests::retained_irules_warning_view_and_statistics_share_the_actual_count` | 17.514 | F1 | rust/tcl-explorer/src/serialise.rs:5961:14 | available warning inventory |
| `serialise::tests::annotations_collect_callouts_and_group_by_line` | 19.063 | F1 | rust/tcl-explorer/src/serialise.rs:5255:48 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::aot_view_carries_the_plan_and_every_rejected_native_premise` | 18.523 | F1 | rust/tcl-explorer/src/serialise.rs:4315:14 | the native record |
| `serialise::tests::asm_emits_structured_instructions_for_top_level` | 18.012 | F1 | rust/tcl-explorer/src/serialise.rs:5272:35 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::asm_resolves_jump_targets` | 17.34 | F1 | rust/tcl-explorer/src/serialise.rs:5306:35 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::bounds_emits_per_function_shape` | 18.315 | F1 | rust/tcl-explorer/src/serialise.rs:4980:38 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::cfg_post_ssa_has_phis_uses_defs_and_analysis` | 18.811 | F1 | rust/tcl-explorer/src/serialise.rs:5215:36 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::cfg_post_ssa_optimised_present_when_source_changes` | 18.029 | F1 | rust/tcl-explorer/src/serialise.rs:5233:9 | assertion failed: v["cfgPostSsaOptimised"].is_array() |
| `serialise::tests::dead_stores_surface_from_o109` | 18.752 | F1/F2 | rust/tcl-explorer/src/serialise.rs:4722:14 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::durable_compiler_views_are_present_and_cover_extra_units` | 19.031 | F2 | rust/tcl-explorer/src/serialise.rs:4623:9 | assertion failed: semantic.iter().any(\|f\| f["name"] == "::C::m") |
| `serialise::tests::event_order_applies_outer_priority_until_it_changes` | 18.444 | F1/F2 | rust/tcl-explorer/src/serialise.rs:4883:51 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::event_order_empty_for_plain_tcl` | 18.407 | F1 | rust/tcl-explorer/src/serialise.rs:4830:9 | assertion `left == right` failed |
| `serialise::tests::event_order_ignores_non_braced_event_bodies` | 18.514 | F1/F2 | rust/tcl-explorer/src/serialise.rs:4914:51 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::event_order_orders_when_handlers_by_firing_order` | 17.438 | F1/F2 | rust/tcl-explorer/src/serialise.rs:4843:37 | eventOrder is an array |
| `serialise::tests::event_order_tie_break_increments_offset` | 17.26 | F1/F2 | rust/tcl-explorer/src/serialise.rs:4868:37 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::event_order_uses_rooted_normalised_top_level_handlers_only` | 17.718 | F1/F2 | rust/tcl-explorer/src/serialise.rs:4900:51 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::gvn_abstains_without_exact_common_invocation_provenance` | 18.418 | F1 | rust/tcl-explorer/src/serialise.rs:4813:34 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::interproc_summarises_procs_with_arity_and_return_shape` | 18.04 | F2 | rust/tcl-explorer/src/serialise.rs:5474:9 | assertion `left == right` failed |
| `serialise::tests::interproc_view_shows_the_param_constant_seed_and_its_withdrawal` | 18.467 | F2 | rust/tcl-explorer/src/serialise.rs:4524:9 | assertion `left == right` failed |
| `serialise::tests::intervals_reports_bounded_loop_counter` | 17.844 | F1/F2 | rust/tcl-explorer/src/serialise.rs:5412:9 | assertion failed: entries.iter().any(\|e\| e["variable"] == "i" && e["lo"].is_number()) |
| `serialise::tests::ir_expands_if_clauses_and_collects_procs` | 17.614 | F2 | rust/tcl-explorer/src/serialise.rs:4666:14 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::ir_rows_keep_equal_names_and_distinct_source_bodies` | 18.533 | F2 | rust/tcl-explorer/src/serialise.rs:4048:9 | assertion `left == right` failed: {"topLevel":[{"kind":"IRCall","summary":"call proc p  return FIRST","colorClass":"ir-call","range":{"start |
| `serialise::tests::irules_flow_empty_for_plain_tcl` | 17.464 | F1 | rust/tcl-explorer/src/serialise.rs:4824:9 | assertion `left == right` failed |
| `serialise::tests::no_dead_stores_when_value_is_read` | 17.7 | F1 | rust/tcl-explorer/src/serialise.rs:4745:9 | assertion `left == right` failed |
| `serialise::tests::optimisations_reports_constant_fold_with_range` | 18.794 | F1 | rust/tcl-explorer/src/serialise.rs:5368:35 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::optimised_ir_and_cfg_present_when_source_changes` | 18.045 | F1 | rust/tcl-explorer/src/serialise.rs:4707:9 | assertion failed: v["irOptimised"]["topLevel"].is_array() |
| `serialise::tests::optimised_source_reflects_constant_fold` | 18.021 | F1 | rust/tcl-explorer/src/serialise.rs:4686:30 | optimised source string |
| `serialise::tests::optimiser_passes_lists_every_pass_in_order` | 18.724 | F1 | rust/tcl-explorer/src/serialise.rs:4790:37 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::optimiser_passes_preserve_set_only_tk_profile` | 18.497 | F1 | rust/tcl-explorer/src/serialise.rs:4214:14 | optimiser passes |
| `serialise::tests::rendered_properties_flags_slash_and_dash` | 17.857 | F1 | rust/tcl-explorer/src/serialise.rs:5395:9 | assertion failed: u["must"].as_array().unwrap().iter().any(\|f\| f == "STARTS_WITH_DASH") |
| `serialise::tests::sccp_reports_each_statements_completion_paths` | 18.49 | F2 | rust/tcl-explorer/src/serialise.rs:5038:18 | the procedure's view |
| `serialise::tests::sccp_reports_each_statements_route_and_answer` | 17.897 | F2 | rust/tcl-explorer/src/serialise.rs:5001:14 | the procedure's view |
| `serialise::tests::sccp_reports_folded_types` | 18.887 | F2 | rust/tcl-explorer/src/serialise.rs:5188:14 | the procedure's view |
| `serialise::tests::sccp_reports_the_route_tally` | 19.446 | F2 | rust/tcl-explorer/src/serialise.rs:5105:14 | the procedure's view |
| `serialise::tests::sccp_reports_the_selection` | 18.019 | F2 | rust/tcl-explorer/src/serialise.rs:5128:14 | the procedure's view |
| `serialise::tests::sccp_reports_the_unreached_arms` | 17.859 | F2 | rust/tcl-explorer/src/serialise.rs:5157:14 | the procedure's view |
| `serialise::tests::semantic_optimisations_view_carries_every_pass_and_its_state` | 17.478 | F1 | rust/tcl-explorer/src/serialise.rs:4402:35 | WASM entries |
| `serialise::tests::shimmer_reports_intrep_conversion` | 17.453 | F1 | rust/tcl-explorer/src/serialise.rs:5349:38 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::stats_reports_function_and_block_counts` | 17.723 | F1 | rust/tcl-explorer/src/serialise.rs:5240:9 | assertion `left == right` failed |
| `serialise::tests::taint_replay_availability_is_separate_from_empty_warning_arrays` | 17.518 | F1 | rust/tcl-explorer/src/serialise.rs:3986:9 | assertion failed: retained_explorer_source(&result, &source).is_some() |
| `serialise::tests::taint_reports_eval_sink` | 17.656 | F1 | rust/tcl-explorer/src/serialise.rs:5335:36 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::types_reports_known_int_for_constant_assignments` | 17.28 | F1 | rust/tcl-explorer/src/serialise.rs:5458:14 | called `Option::unwrap()` on a `None` value |
| `serialise::tests::unit_scope_reports_uniform_literals_and_no_boundary` | 17.949 | F2 | rust/tcl-explorer/src/serialise.rs:5500:14 | ::helper evidence present |
| `serialise::tests::wasm_view_exposes_canonical_semantic_plan_and_serialised_output` | 17.883 | F1 | rust/tcl-explorer/src/serialise.rs:4228:39 | WASM entries |
| `serialise::tests::wasm_view_exposes_common_native_i64_selection_evidence` | 18.285 | F1 | rust/tcl-explorer/src/serialise.rs:4279:37 | WASM entries |
| `serialise::tests::wasm_view_exposes_the_native_tier_lowering_record` | 17.661 | F1 | rust/tcl-explorer/src/serialise.rs:4431:40 | WASM entries |
| `serialise::tests::wasm_view_exposes_typed_semantic_decline` | 17.283 | F1 | rust/tcl-explorer/src/serialise.rs:4458:37 | WASM entries |
| `serialise::tests::wasm_view_serialises_multiple_mixed_regions_in_node_order` | 18.541 | F1 | rust/tcl-explorer/src/serialise.rs:4481:37 | WASM entries |
| `serialise::tests::world_ssa_decline_preserves_transition_policy_evidence` | 17.943 | F1 | rust/tcl-explorer/src/serialise.rs:4585:9 | assertion `left == right` failed |
| `serialise::tests::world_ssa_view_keeps_versions_transitions_and_typed_availability` | 18.173 | F1 | rust/tcl-explorer/src/serialise.rs:4550:9 | the availability payload must distinguish an absent graph from absent executable IR |
| `tests::deep_pipeline_preserves_the_actual_source_metadata_owner` | 18.0 | F2 | rust/tcl-explorer/src/lib.rs:445:9 | assertion failed: snapshots.iter().any(\|snapshot\| |
| `tests::original_document_pipeline_keeps_config_withdrawal_terminal` | 17.82 | F1 | rust/tcl-explorer/src/lib.rs:403:9 | assertion failed: crate::serialise::retained_explorer_source(&result, source).is_some() |
| `tests::original_document_pipeline_keeps_root_and_later_bom_roles` | 17.955 | F1 | rust/tcl-explorer/src/lib.rs:369:22 | called `Option::unwrap()` on a `None` value |
| `tests::run_pipeline_collects_procedures_in_order` | 18.134 | F2 | rust/tcl-explorer/src/lib.rs:429:9 | assertion failed: procs.contains(&"::alpha") |
| `tests::snapshots_retain_duplicate_procedures_and_named_top_separately` | 18.067 | F2 | rust/tcl-explorer/src/lib.rs:489:9 | assertion `left == right` failed |
| `tests::supplied_analysis_input_controls_the_document_pipeline` | 0.29 | F3 | rust/tcl-explorer/src/lib.rs:321:9 | assertion `left == right` failed |
| `view_tree::tests::a_hint_only_optimisation_is_not_rendered_as_a_rewrite` | 17.624 | F1 | rust/tcl-explorer/src/view_tree.rs:2092:14 | the opt payload is a list |
| `view_tree::tests::every_tree_descriptor_has_a_shared_builder` | 17.824 | F1 | rust/tcl-explorer/src/view_tree.rs:2220:13 | tree descriptor aot has no shared renderer |
| `view_tree::tests::ir_view_has_top_level_and_proc_nodes` | 18.732 | F2 | rust/tcl-explorer/src/view_tree.rs:2233:9 | assertion failed: nodes.iter().any(\|n\| n.label.starts_with("::f")) |
| `view_tree::tests::rows_that_display_a_range_also_carry_the_structured_one` | 17.986 | F1 | rust/tcl-explorer/src/view_tree.rs:2174:13 | ssa showed no range rows at all |
| `wasm_explorer::tests::source_function_rows_preserve_duplicate_names_and_named_top` | 19.598 | F2 | rust/tcl-explorer/src/wasm_explorer.rs:585:9 | assertion `left == right` failed: [Object {"name": String("(module)"), "kind": String("module"), "funcIdx": Null, "params": Array [], "resul |
