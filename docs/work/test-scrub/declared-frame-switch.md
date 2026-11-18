# Declared-frame switch: measured regressions

Measured 18 November 2026 on `work` at 2c016d0c (tcl-compiler, every target), with and without the switch:
`Lowerer::retain_declared_frame_bodies` replaces each named procedure's own body with the body lowered in its declared frame.

- Baseline: 13152 tests run, 1897 failed.
- Switch: 13152 tests run, 1957 failed.
- New: 65 in tcl-compiler, plus 1 in the tcl-lsp-core proof groups (`refactor::inline_variable::tests::inline_keeps_the_following_line_indentation`).
- Fixed: 6.

Every called body is bound under a per-activation frame identity (`::setter@0/implementation:1:0:First/source:1/site:76` against the declared `::setter/source:1/site:0`), so the switch replaces every called body, not only joined ones.
The declared-frame view then loses facts that hold for every entry; for example `proc ::setter {} {uplevel 1 {uplevel 1 {set x 1}}}` loses `beyond_caller = Caller(2)`.
These are gaps in what `declared_frame_walk` retains (dataflow).
The switch lands after the collector keeps them, re-measured to new 0.

Narrow fix landed instead: `ExpressionMathBindings::for_module_statement` reads the declared-frame body, which removes the `add` → `return 7` rewrite.
The `info exists` fold (`value_transfer_witnesses::try_and_errors::a_body_that_ends_in_an_error_may_have_failed_at_its_start`) stays open, blocked on this.

## Frames a body reaches outside its own (uplevel / upvar / global targets) (9)

- `tcl-compiler cfg_builder::global_write_info::tests::nested_relative_zero_inside_global_upframe_stays_global`
- `tcl-compiler cfg_builder::global_write_info::tests::original_global_source_write_keeps_empty_scalar_and_array_root`
- `tcl-compiler cfg_builder::global_write_info::tests::uplevel_hash_zero_literal_body_records_global_writes`
- `tcl-compiler cfg_builder::global_write_info::tests::uplevel_hash_zero_literal_body_resolves_late_alias_prefix_write`
- `tcl-compiler cfg_builder::upvar_info::tests::nested_relative_one_reaches_past_the_caller`
- `tcl-compiler cfg_builder::upvar_info::tests::nested_relative_zero_preserves_the_caller_frame`
- `tcl-compiler::it_3 optimiser::nested_reads_and_writes::a_condition_substitution_reads_the_frames_variables`
- `tcl-compiler::it_3 optimiser::nested_reads_and_writes::a_structural_body_is_not_the_enclosing_statements_surface`
- `tcl-compiler::it_4 caller_frame_effects::uplevel_of_an_unreadable_script_blinds_the_callers_frame_not_its_own`

## Cell and alias identity across retargeting (11)

- `tcl-compiler def_use::tests::return_reads_preserve_both_producers_across_alias_retargeting`
- `tcl-compiler sccp::original_value_facts::tests::semantic_projection::point_expression_environment_preserves_alias_spelling_and_retargeting`
- `tcl-compiler sccp::original_value_facts::tests::value_solver::global_alias_contents_are_precise_until_an_actual_reentry_dependency`
- `tcl-compiler ssa::cell_resolution_tests::exact_reads_keep_alias_retargeting_within_one_word_separate`
- `tcl-compiler ssa::cell_resolution_tests::incoming_slot_projection_keeps_distinct_call_activations`
- `tcl-compiler ssa::cell_resolution_tests::name_only_read_projection_requires_reached_consensus`
- `tcl-compiler ssa::cell_resolution_tests::nested_contents_store_does_not_borrow_an_earlier_ssa_version`
- `tcl-compiler ssa::cell_resolution_tests::scalar_alias_write_after_unset_reaches_both_spellings`
- `tcl-compiler ssa::cell_resolution_tests::terminator_read_consensus_declines_retargeted_source_spelling`
- `tcl-compiler type_infer::tests::expression_types::written_container_reads_keep_raw_and_braced_variable_identity`
- `tcl-compiler::it_4 name_lookup_issue_witnesses::an_unknown_array_key_observes_possible_elements`

## Interprocedural taint sinks and sources (12)

- `tcl-compiler taint::source_context::authored_body::controls::original_sinking_helper_uses_only_current_authored_may_schema`
- `tcl-compiler::it_3 taint::object_instance_sources::object_instance_taint_sources::a_caught_early_call_does_not_withdraw_the_receiver_from_a_later_call`
- `tcl-compiler::it_3 taint::object_instance_sources::object_instance_taint_sources::rename_moves_local_and_global_instance_class_facts`
- `tcl-compiler::it_3 taint::object_instance_sources::object_instance_taint_sources::top_level_widget_is_a_source_inside_callback_procedure`
- `tcl-compiler::it_3 taint::propagation::interprocedural_colours::helper_list_wrapper_eval_still_fires`
- `tcl-compiler::it_3 taint::propagation::interprocedural_taint::tainted_argument_into_sinking_helper_warns`
- `tcl-compiler::it_3 taint::sources::tcl_taint_sources::escaped_static_command_head_obeys_release_grammar`
- `tcl-compiler::it_3 taint::sources::tcl_taint_sources::eval_braced_nested_command_substitution_reaches_inner_sink`
- `tcl-compiler::it_3 taint::sources::tcl_taint_sources::namespace_shadowed_catch_cannot_hijack_nested_analysis_scaffolding`
- `tcl-compiler::it_3 taint::sources::tcl_taint_sources::quoted_whole_command_substitution_reaches_inner_sink`
- `tcl-compiler::it_3 taint::sources::tcl_taint_sources::whole_command_substitution_classifies_each_executed_inner_sink`
- `tcl-compiler::it_4 taint_depth::interproc_depth::tainted_arg_into_helper_with_internal_sink_warns`

## Value-transfer witnesses (try / catch / cell updates / expressions) (11)

- `tcl-compiler::it_2 value_transfer_witnesses::catch::a_catch_script_that_is_not_brace_quoted_text_is_not_the_script_it_runs`
- `tcl-compiler::it_2 value_transfer_witnesses::cell_updates::a_cell_update_reads_its_words_as_tcl_substitutes_them`
- `tcl-compiler::it_2 value_transfer_witnesses::cell_updates::value_position_words_are_read_as_tcl_substitutes_them`
- `tcl-compiler::it_2 value_transfer_witnesses::declared_routes::a_no_match_keeps_the_store_it_preserves`
- `tcl-compiler::it_2 value_transfer_witnesses::expressions::an_invalid_octal_condition_decides_nothing`
- `tcl-compiler::it_2 value_transfer_witnesses::expressions::format_folds_through_the_shared_core`
- `tcl-compiler::it_2 value_transfer_witnesses::preserving_outcomes::const_writes_only_an_absent_place`
- `tcl-compiler::it_2 value_transfer_witnesses::procedure_summaries::a_statement_that_certainly_raises_ends_the_exit_reading`
- `tcl-compiler::it_2 value_transfer_witnesses::try_and_errors::a_finally_resumes_what_it_interrupted`
- `tcl-compiler::it_2 value_transfer_witnesses::try_and_errors::a_handler_is_thrown_to_from_where_the_body_leaves`
- `tcl-compiler::it_2 value_transfer_witnesses::try_and_errors::try_handlers_run_as_the_registry_chain_reads_them`

## Formal and call-site proofs (AOT, object types, procedure facts) (9)

- `tcl-compiler analyser::diagnostics::helpers::exact_cell_diagnostic_tests::procedure_namespace_effects_require_original_selected_cells`
- `tcl-compiler analyser::diagnostics::tests::var_command::dispatch_tables::dispatch_table_navigation_does_not_borrow_another_frames_store`
- `tcl-compiler analyser::diagnostics::tests::var_command::method_dispatch::w308_tp_copy_propagated_handle_with_a_trailing_rename_still_flags`
- `tcl-compiler common_aot_plan::tests::add_call_and_integer_formals_receive_common_proofs`
- `tcl-compiler common_aot_plan::tests::declared_arguments_refuse_missing_owners_changed_contents_and_future_callers`
- `tcl-compiler object_types::tests::by_scope_cross_unit_call_edges_still_bind`
- `tcl-compiler object_types::tests::normal_callable_return_types_do_not_require_a_tcloo_class`
- `tcl-compiler object_types::tests::retained_procedure_factory_candidates_do_not_require_an_executable_unit`
- `tcl-compiler registry_invocation::source_structure::original_body_syntax::tests::actual_factorial_compiler_syntax_does_not_borrow_native_attachment`

## Optimiser and diagnostic folds (13)

- `tcl-compiler analyser::diagnostics::fp::ds::fp_ds_09_interproc_empty_dict_fires`
- `tcl-compiler analyser::diagnostics::fp::ds::fp_ds_09_interproc_mixed_callers_conservative_silent`
- `tcl-compiler analyser::diagnostics::fp::opt::fp_opt_01_genuine_simplification_still_fires`
- `tcl-compiler analyser::diagnostics::fp::opt::fp_opt_09_provably_numeric_var_still_fires`
- `tcl-compiler analyser::diagnostics::tests::dataflow::existence_folds::info_exists_does_not_fold_a_place_a_read_trace_may_bind`
- `tcl-compiler analyser::diagnostics::tests::dataflow::unused_bindings::w211_reads_an_existence_test_by_the_element_it_names`
- `tcl-compiler optimiser::elimination::tests::original_adce_keeps_actual_remote_stores_after_local_consumers_disappear`
- `tcl-compiler optimiser::manager::tests::a_stub_stating_its_own_frame_keeps_a_branch_foldable_across_its_call`
- `tcl-compiler optimiser::propagation::tests::proc_folds::o103_folds_implicit_return_zero_arg_proc`
- `tcl-compiler optimiser::structure_elimination::tests::original_sccp_environment_withdraws_rebound_source_names_despite_display_collisions`
- `tcl-compiler::it_3 fp_depth::opt_depth::o110_identity_on_provably_int_loop_counter_fires`
- `tcl-compiler::it_3 lsp_shimmer_conditional_source::a_compiled_literal_length_is_a_pure_string_before_9_1`
- `tcl-compiler::it_3 optimiser::dead_code::a_conditional_writer_does_not_kill_the_store_it_may_preserve`

## Fixed by the switch (6)

- `tcl-compiler analyser::diagnostics::tests::walked_bodies::tcltest_oracle_program_keeps_missing_reads_and_setup_completion`
- `tcl-compiler var_escape::api::tests::caller_of_impure_proc_is_downgraded`
- `tcl-compiler::it_2 value_transfer_witnesses::try_and_errors::a_body_that_ends_in_an_error_may_have_failed_at_its_start`
- `tcl-compiler::it_2 var_escape_cfg::cu_interprocedural_off_keeps_raw_per_proc_result`
- `tcl-compiler::it_5 var_escape_typeinfer::interprocedural_caller_inherits_upvar_pessimism`
- `tcl-compiler::it_5 var_escape_typeinfer::interprocedural_pure_leaf_downgrade_through_callees`

## The parked patch

Branch `park/declared-frame-switch` holds it locally. In `retain_declared_frame_bodies`, after the restored state and before `self.module.declared_frame_bodies = bodies;`:

```rust
for (id, procedure) in &bodies {
    if let Some(own) = self.module.procedures.get_mut(id) {
        own.body = procedure.body.clone();
    }
}
```
