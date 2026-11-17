# Diagnostics area: test scrub

Session `session_017u5g3HXzMVEcpUS6bxGcmA`, 17 November 2026. Ledger: [`ledger/diagnostics.tsv`](ledger/diagnostics.tsv) (397 failing tests of the 21:00 full run in this area's families, `it_4 analyser::class_factories` excluded: class-facts'). Clusters touching the area: [`ledger/diagnostics-clusters.tsv`](ledger/diagnostics-clusters.tsv). Passing tests the oracle contradicts: [`ledger/diagnostics-passing-wrong.tsv`](ledger/diagnostics-passing-wrong.tsv).

Every Tcl-behaviour row was run on tclsh 8.6.18 and 9.0.4, and on 8.4.20, 8.5.19 or 9.1.0 where the release decides; snit and report from tcllib 2.0. No Itcl and no jimsh here: the Itcl row rests on the W307 KCS page, the Jim-only rows are UNDECIDED.

## Counts by verdict

| Verdict | Tests |
| --- | ---: |
| KEEP-ORACLE | 143 |
| KEEP-CONTRACT | 94 |
| FIX-FIXTURE | 62 |
| REWRITE | 36 |
| FIX-EXPECTATION | 33 |
| KEEP-F5 | 17 |
| UNDECIDED | 6 |
| RETIRE | 5 |
| MERGE | 1 |
| total | 397 |

Seven `core_analyses` narrowing tests of the 21:00 run were renamed upstream with the corrected verdicts and are not listed.

## Causes ranked by tests blocked

All verdicts; the owner is the session that fixes the product (or the fixture).

| Tests | Cause | Owner |
| ---: | --- | --- |
| 22 | `fixture-raises-before-subject` | diagnostics (fixtures) |
| 14 | `unknown-command-body-never-runs` | diagnostics (fixtures, #2427) |
| 14 | `native-direct-unit-omits-procedure` | dataflow-optimiser / analyser-binding |
| 12 | `oo-instance-w307` | analyser-binding / class-facts |
| 9 | `no-proc-transfer-summary` | dataflow-optimiser (interprocedural) |
| 9 | `param-traits-shallow-roles` | diagnostics |
| 8 | `walk-w123-presence` | analyser-binding |
| 8 | `param-traits-deep-descent` | diagnostics |
| 8 | `lenient-tcl-logical-source-records` | diagnostics |
| 7 | `alias-captured-prefix-anchor` | diagnostics |
| 7 | `w201-detector-silent` | taint |
| 7 | `unknown-proc-bounded-handler` | diagnostics |
| 6 | `registry-lenient-profile-words` | analyser-binding |
| 5 | `const-subst-head-fold` | diagnostics |
| 5 | `interp-eval-literal-domain` | diagnostics |
| 5 | `regex-pattern-record-unconsumed` | diagnostics |
| 4 | `uncalled-body-generic-entry` | diagnostics |
| 4 | `walk-unreached-site-presence` | analyser-binding |
| 4 | `vendor-profile-w123-silent` | diagnostics |
| 4 | `walk-loop-body-lookup` | analyser-binding |
| 4 | `walk-computed-callee` | analyser-binding |
| 4 | `o107-revert-window` | dataflow-optimiser |
| 4 | `manifest-env-not-applied` | diagnostics |
| 4 | `safe-interp-w129-reach` | diagnostics |
| 4 | `s100-shimmer-silent` | diagnostics |
| 3 | `materialized-footprint-empty` | diagnostics |
| 3 | `walk-zero-trip-loop` | analyser-binding |
| 3 | `irules-disabled-w002-missing` | diagnostics |
| 3 | `bounds-switch-arm-write-scan` | diagnostics |
| 3 | `option-scan-quoted-prefix` | analyser-binding (registry_invocation) |
| 3 | `logical-class-family-context` | diagnostics |
| 3 | `w210-narrowing-false-arm` | diagnostics |
| 3 | `interp-domain-key-internal` | diagnostics |
| 3 | `interp-create-value-words-helper` | diagnostics |

161 further causes block one or two tests each; see the ledger.

## Wrong answers (own slugs, first on the fix list)

- `i231-escaped-pattern-false-arm`: `set s "a\nb"; switch $s {a\nb {puts hit} default {puts miss}}` prints `hit` on every release; the product calls that arm always false.
- `i231-unpinned-release-option-subject`: under `tk` (no release named) the product claims a selected arm for a program tclsh 8.4 rejects with `bad option`.
- `w210-existence-guard-false-positive`: W210 on `x` after the `info exists` guard has set it.
- `w210-after-undefined-command` (`fp_rbs_19`): W210 at reads after `set data [getDataDict]` raises `invalid command name` (#2427); diagnostics fixes it with `entered_site`.
- `proc-name-raw-substitution-text`: a procedure literally named `::${c}` recorded after a `foreach`.
- `interp-domain-raw-var-key`: procedures made by `makeA` and `makeB` share `@interp@${s}`; navigation crosses between them.
- `dict-ensemble-map-reconfigure`: a false W001 on `dict is_dict` after `-map` is reconfigured, and a false W214 on a parameter `dict size` reads.
- `lexer-var-scan-debug-assert`: `w215_quiet_for_backslash_continued_param_list` panics on the `debug_assert` at `tcl-lexer/src/word_parts.rs:702` (a crash, naming owner).
- Passing tests the oracle contradicts: two `namespace unknown` rows in `it_4 unresolved_commands` ([`diagnostics-passing-wrong.tsv`](ledger/diagnostics-passing-wrong.tsv)).

## Distinct failing behaviours worth fixing

The causes of the kept, fixture-fixed and rewritten rows (one line per behaviour; tests per behaviour in brackets):

- `fixture-raises-before-subject` (22): e.g. `branch_joined_const_dispatch_records_every_may_target_945`
- `native-direct-unit-omits-procedure` (14): e.g. `info_exists_does_not_fold_unset_parameter`
- `unknown-command-body-never-runs` (13): e.g. `i230_never_reports_a_condition_across_a_catch_body_or_a_computed_head`
- `oo-instance-w307` (12): e.g. `fp_obj_05_snit_create_auto_no_w307`
- `no-proc-transfer-summary` (9): e.g. `a_call_the_module_cannot_see_keeps_a_loop_silent`
- `walk-w123-presence` (8): e.g. `w123_tp_a_dead_body_edge_does_not_lower_a_later_top_level_offset`
- `param-traits-deep-descent` (8): e.g. `analyse_with_deep_param_traits_surfaces_nested_eval`
- `param-traits-shallow-roles` (8): e.g. `deep_pass_skips_dynamic_body_args`
- `lenient-tcl-logical-source-records` (8): e.g. `logical_member_and_relation_views_keep_the_actual_source_owners`
- `alias-captured-prefix-anchor` (7): e.g. `original_option_diagnostics_share_alias_ordinals_and_stop_at_unknown_layout`
- `w201-detector-silent` (7): e.g. `fp_sty_16_genuine_path_concat_still_fires`
- `registry-lenient-profile-words` (6): e.g. `declared_variable_name_positions_keep_only_supplied_contract_roles`
- `const-subst-head-fold` (5): e.g. `a_constant_namespace_qualifiers_rhs_folds_and_resolves_the_head`
- `interp-eval-literal-domain` (5): e.g. `deleted_and_recreated_interp_is_a_fresh_domain_945`
- `uncalled-body-generic-entry` (4): e.g. `the_existence_branch_fact_is_stored_once`
- `walk-unreached-site-presence` (4): e.g. `a_read_after_code_the_module_cannot_see_is_no_read_before_set`
- `vendor-profile-w123-silent` (4): e.g. `tmsh_first_class_resolves_its_surface_and_gates_later_core`
- `walk-loop-body-lookup` (4): e.g. `fp_ds_10_dead_store_inside_dict_for_body_now_fires`
- `walk-computed-callee` (4): e.g. `fp_ds_10_proved_computed_set_still_reports_its_unused_destination`
- `o107-revert-window` (4): e.g. `fp_rch_04_infinite_loop_dead_code_fires`
- `safe-interp-w129-reach` (4): e.g. `safe_interp_w129_direct_nested_bracket_substitution_deep_1001`
- `s100-shimmer-silent` (4): e.g. `shimmer_not_double_counted_in_namespace_eval_body`
- `materialized-footprint-empty` (3): e.g. `expression_suppression_uses_each_original_child_lookup_horizon`
- `walk-zero-trip-loop` (3): e.g. `fp_rbs_18_guaranteed_for_defines_body_vars`
- `irules-disabled-w002-missing` (3): e.g. `irules_4b_disabled_split_pins_two_diagnostic_severities`
- `bounds-switch-arm-write-scan` (3): e.g. `a_command_an_arm_runs_that_ends_the_loop_keeps_w241_silent`
- `option-scan-quoted-prefix` (3): e.g. `fp_sty_02_concatenated_dollar_in_quoted_pattern_still_fires`
- `manifest-env-not-applied` (3): e.g. `manifest_directive_arity_checked`
- `logical-class-family-context` (3): e.g. `logical_class_family_refuses_defaults_missing_and_stale_source_context`
- `w210-narrowing-false-arm` (3): e.g. `failed_branch_is_ambiguous_and_still_flagged`
- `interp-domain-key-internal` (3): e.g. `real_world_safe_and_cross_interp_aliases_945`
- `interp-create-value-words-helper` (3): e.g. `interp_create_value_words_keep_a_nested_path_as_one_word_issue_1025`
- `w210-braced-span-drops-close-brace` (2): e.g. `literal_scalar_parentheses_do_not_borrow_array_root_parameters`
- `const-dispatch-indirect-record` (2): e.g. `catch_and_try_body_writes_join_never_last_write_945`
- `i231-opaque-switch-silent` (2): e.g. `an_opaque_switch_reports_the_arms_it_never_selects`
- `i230-message-condition-spelling` (2): e.g. `i230_message_keeps_braced_var_spelling`
- `w216-literal-replacement-missing` (2): e.g. `w216_fixed_proposals_match_captured_public_value_examples`
- `proven-append-container` (2): e.g. `a_nested_index_reads_the_proven_container`
- `braced-word-dead-store-silent` (2): e.g. `issue_1260_braced_loop_value_word_keeps_the_store_live`
- `dfo-info-exists-fold` (2): e.g. `fp_rch_05_dynamic_array_element_key_still_folds`
- `shimmer-s100-commit-tracking` (2): e.g. `fp_sh_22_merge_use_matching_neither_arm_fires`
- `uncalled-body-word-checks` (2): e.g. `w003_dialect_is_file_wide_regardless_of_namespace_or_proc_nesting`
- `foreach-list-cmd-constset` (2): e.g. `foreach_list_cmd_folds_to_constset`
- `list-built-script-body` (2): e.g. `tp_a_list_built_namespace_body_declares_in_the_namespace`
- `original-symbol-card-metadata` (2): e.g. `original_registry_symbol_advice_preserves_hosted_source_event_declarations`
- `if-static-branch-selection` (2): e.g. `original_if_branches_use_clause_order_and_complete_original_bodies`
- `lenient-tcl-value-provenance` (2): e.g. `logical_dynamic_word_keeps_exact_braced_name_and_interpolation_sources`
- `fixture-array-read-of-scalar-formal` (2): e.g. `compound_dynamic_array_name_marks_components`
- `original-variable-symbols-uncalled-body` (2): e.g. `original_variable_alias_reads_keep_derived_and_independent_local_spellings`
- `ghost-recovery-original-input` (2): e.g. `original_whole_source_recovery_keeps_file_config_and_typed_warning_fields`
- `oo-var-headed-constructor-instance` (2): e.g. `analyse_records_instance_class_through_a_var_headed_create`
- `irule-fixture-outside-event` (2): e.g. `run_all_checks_irule3101_gated_by_dialect`
- `existence-guard-retained-metadata` (1): e.g. `original_existence_guards_use_retained_availability_and_refuse_missing_foreign`
- `uncalled-proc-global-write-summary` (1): e.g. `procedure_namespace_effects_require_original_selected_cells`
- `const-dispatch-expanded-prefix-literal` (1): e.g. `expanded_head_list_prefix_keeps_the_exact_writable_component_945`
- `i230-switch-arm-callee-clobbers-local` (1): e.g. `i230_never_reports_a_condition_over_a_name_a_switch_arm_may_write`
- `i231-unpinned-release-option-subject` (1): e.g. `i231_never_reports_a_switch_over_a_subject_a_release_may_read_as_an_option`
- `i231-escaped-pattern-false-arm` (1): e.g. `i231_reads_a_switch_words_by_their_values`
- `w220-store-across-known-proc-call` (1): e.g. `a_store_a_call_the_module_cannot_see_may_read_draws_neither_w220_nor_w211`
- `w220-irules-event-local-silent` (1): e.g. `emit_cfg_ssa_diagnostics_w220_irules_proc_local_still_flagged`
- `index-entry-fact-not-file-name` (1): e.g. `w210_pkgindex_reports_other_reads_inside_command_substitutions`
- `irules-static-race-silent` (1): e.g. `analyse_irule4005_racy_static_emitted_for_per_request_writes`
- `walk-formal-values` (1): e.g. `w210_interproc_dict_with_caller_literal`
- `w210-possibly-empty-loop-fires` (1): e.g. `fp_rbs_17_guaranteed_foreach_defines_body_vars`
- `w210-existence-guard-false-positive` (1): e.g. `a_guard_holds_where_the_rung_refines_nothing`
- `w210-binary-scan-possible-target` (1): e.g. `var_binding_binary_scan_defines_targets`
- `w213-lazy-startup-trace` (1): e.g. `w210_startup_read_trace_respects_global_alias_and_unset_lifecycle`
- `w210-irules-startup-argv` (1): e.g. `w210_uses_registry_owned_startup_lifecycle_facts`
- `irules-policy-braced-eval-scan` (1): e.g. `irules_4b_policy_warning_follows_the_4c_lexical_scan`
- `declared-source-fix-safety` (1): e.g. `declared_source_diagnostics_use_only_the_declared_roles_and_signature`
- `alias-captured-option-relationship` (1): e.g. `original_source_arity_and_relationships_keep_effective_counts_and_written_suffixes`
- `index-statement-pass-reports-first` (1): e.g. `an_index_site_is_reported_once`
- `dfo-incr-variable-amount` (1): e.g. `w241_reads_a_proven_bound_and_step`
- `matcher-fixture-non-ascii-literal` (1): e.g. `original_condition_matcher_retains_multibyte_inclusive_extent_and_refuses_overflow`
- `matcher-no-match-w210-silent` (1): e.g. `logical_no_match_diagnostics_keep_prior_and_intervening_values_and_literal_names`
- `test-fixture-jim-profile-lookup` (1): e.g. `original_matcher_query_refuses_missing_foreign_and_stale_source_inputs`
- `barrier-body-local-store-purpose` (1): e.g. `original_reference_only_body_cannot_supply_local_store_suppression`
- `uplevel-braced-body-frame-shift` (1): e.g. `fp_ds_11_braced_body_only_read_is_a_frame_shifted_dead_store`
- `subst-literal-template-blinds-dead-store` (1): e.g. `fp_ds_12_literal_subst_template_is_not_a_dynamic_read`
- `braced-literal-name-read-w210` (1): e.g. `fp_ds_14_missing_literal_read_stops_before_the_later_stores`
- `irules-2004-cmdsub-head` (1): e.g. `fp_inj_04_loader_refusal_does_not_create_a_tainted_read_result`
- `dialect-blind-fixture` (1): e.g. `fp_inj_07_exec_encoding_does_not_mask_option_scan`
- `interproc-dict-with-propagation` (1): e.g. `fp_obj_18_interproc_dict_with_known_command_silent`
- `dfo-append-cmdsub-forward` (1): e.g. `fp_opt_06_o100_does_not_propagate_past_cmd_sub_write`
- `dfo-o114-loop-int-proof` (1): e.g. `fp_opt_10_provably_int_var_still_fires`
- `dfo-o103-preempts-o126` (1): e.g. `fp_opt_12_pure_user_proc_via_my_dispatch_handled_at_word_level`
- `dynamic-variable-link-tail` (1): e.g. `fp_rbs_04_unrelated_tail_still_fires`
- `w210-after-undefined-command` (1): e.g. `fp_rbs_19_reporter_foreach_accumulator_silent`
- `shimmer-s102-loop-oscillation` (1): e.g. `fp_sh_15_unaliased_method_local_still_fires`
- `shimmer-fixture-not-oscillating` (1): e.g. `fp_sh_18_numeric_shimmer_masked_by_int_entry_fires_s102`
- `irules-s110-payload-coercion` (1): e.g. `fp_sh_19_coercing_string_ops_on_payload_still_fire`
- `shimmer-expr-multiword-operand` (1): e.g. `fp_sh_24_braced_expr_operand_still_fires`
- `binary-scan-empty-input-targets` (1): e.g. `fp_sty_11_binary_scan_empty_input_does_not_define_its_targets`
- `fixture-missing-package-require` (1): e.g. `fp_tnt_05_tainted_url_fires_t104`
- `fixture-uncatalogued-quote-helper` (1): e.g. `original_quoting_child_keeps_bom_alias_and_complete_pattern_word`
- `reparse-nested-alias-w309` (1): e.g. `original_nested_reparse_keeps_both_aliases_and_declines_replaced_child_or_parent`
- `original-diagnostic-source-missing` (1): e.g. `variable_name_positions_are_registry_driven`
- `w212-dict-with-name-role` (1): e.g. `w212_covers_registry_name_positions`
- `registry-socket-server-arity` (1): e.g. `socket_server_form_counts_only_its_port_as_positional`
- `oo-instance-method-arity` (1): e.g. `tcloo_forward_arity_is_shifted_by_prepended_args`
- `tcllib-instance-method-options` (1): e.g. `w147_reports_a_cross_option_value_relation_on_an_instance_method`
- `fixture-ensemble-configure-shape` (1): e.g. `the_ensemble_option_scan_consumes_values_only_for_the_operations_own_options`
- `irules-log-arity` (1): e.g. `irules_arity_accepts_documented_pool_log_and_class_iteration_forms`
- `rename-constant-target-arity` (1): e.g. `same_file_dynamic_but_resolvable_rename_target_checks_arity`
- `namespace-relative-qualified-arity` (1): e.g. `same_file_relative_qualified_call_resolves_against_current_namespace`
- `fixture-self-dispatch-outside-method` (1): e.g. `analyse_w307_suppressed_for_my_self_dispatch`
- `w307-known-noncommand-value` (1): e.g. `w307_reads_element_and_body_bindings`
- `dispatch-table-nested-dict` (1): e.g. `dispatch_table_dictionary_updates_preserve_other_entries_and_original_spans`
- `fixture-dialect-lacks-operator` (1): e.g. `tip461_string_ordering_context_gets_no_fix`
- `irules-after-body-not-walked` (1): e.g. `analyser_recurses_into_irules_nesting_script_bodies`
- `w210-duplicate-report` (1): e.g. `tcltest_oracle_program_keeps_missing_reads_and_setup_completion`
- `provider-unselected-body-entered` (1): e.g. `tcltest_test_body_is_walked_when_imported`
- `presence-internal-record` (1): e.g. `actual_presence_suppresses_absence_without_a_target_allocation`
- `w114-emitter-direct-call` (1): e.g. `nested_expression_advice_uses_actual_renamed_descriptor_and_refuses_shadow_and_inert_data`
- `factory-metadata-context` (1): e.g. `factory_may_advice_uses_actual_availability_and_exact_supplied_source`
- `oo-self-return-literal` (1): e.g. `selected_original_method_body_requires_real_return_schema_and_static_operand`
- `irules-event-window-code` (1): e.g. `event_version_clause_follows_the_declared_range`
- `fixture-package-not-required` (1): e.g. `no_resolvable_floor_falls_back_to_the_plain_arity`
- `w220-braced-name-word-credit` (1): e.g. `tp_a_braced_name_word_does_not_credit_the_plain_cell`
- `w210-return-value-cmdsub-def` (1): e.g. `cmd_sub_set_in_return_value_is_a_definition`
- `w220-top-level-overwrite` (1): e.g. `dead_store_for_overwritten_assignment`
- `fixture-uncalled-caller-value` (1): e.g. `dict_update_harvests_keys_through_the_sccp_version`
- `w220-array-element-incr` (1): e.g. `array_element_incr_then_distinct_write_both_live`
- `taint-path-prefixed-copy` (1): e.g. `http_uri_via_copy_assignment_no_t102`
- `interp-path-handle-not-a-command` (1): e.g. `nested_path_value_flow_key_matches_the_direct_handler_issue_1025`
- `interp-domain-raw-var-key` (1): e.g. `two_procs_sharing_a_variable_name_never_collide`
- `walk-foreach-literal-binding` (1): e.g. `foreach_loop_variable_over_a_literal_list_is_constant_folded`
- `proc-name-raw-substitution-text` (1): e.g. `foreach_over_a_braced_list_element_is_not_mis_split_on_whitespace`
- `rename-in-uncalled-proc` (1): e.g. `two_concatenated_vars_in_same_straight_line_scope_resolve`
- `registry-object-command-w123` (1): e.g. `report_object_methods_resolve`
- `dict-ensemble-map-reconfigure` (1): e.g. `dynamically_mapped_dict_ensemble_fix_is_not_overfit_to_getnull`
- `w216-brace-dollar-index` (1): e.g. `w216_brace_array_with_dollar_index`
- `w216-fix-qualified-set` (1): e.g. `w216_funny_name_falls_back_to_set_indirection`
- `w214-dict-with-param-read` (1): e.g. `param_used_only_by_dict_with_or_update`
- `method-body-not-entered` (1): e.g. `an_unlinked_sibling_method_bareword_still_draws_w123`
- `release-specific-fixture-dialect` (1): e.g. `metaclass_variants`
- `oo-class-in-called-proc` (1): e.g. `per_item_analysis_agrees_with_whole_file_analysis`
- `oo-self-dispatch-w307` (1): e.g. `self_command_dispatch_to_a_subclass_method_still_draws_w308`
- `w123-did-you-mean-alias` (1): e.g. `alias_command_no_w123_and_alias_in_did_you_mean`
- `unknown-proc-bounded-handler` (1): e.g. `unknown_if_else_chains_original_suppresses`
- `reference-span-convention` (1): e.g. `tp_a_brace_quoted_name_word_reads_the_literal_cell`
- `w143-private-ensemble-fix` (1): e.g. `quick_fix_replaces_head_with_public_ensemble_call`
- `w210-uplevel-shifted-frame` (1): e.g. `uplevel_tn_shifted_frame_write_does_not_define_a_local_1051`
- `w212-raising-word-site` (1): e.g. `unset_dollar_vars_fire_through_options`
- `w211-call-by-name-dynamic-local` (1): e.g. `callee_local_dynamic_name_does_not_suppress_caller_dead_store`
- `nested-proc-defining-namespace` (1): e.g. `nested_proc_is_named_by_its_defining_namespace`
- `constructed-key-colon-run-holder` (1): e.g. `computed_creation_placeholder_preserves_constructed_procedure_holder`
- `conditional-registry-metadata-absent` (1): e.g. `original_conditional_registry_symbol_cards_retain_unknown_applicability`
- `retained-static-context-barrier` (1): e.g. `original_static_result_retains_native_and_hosted_advice_barriers`
- `dynamic-oo-define-synthetic-class` (1): e.g. `two_dynamic_oo_define_targets_sharing_a_variable_name_never_merge`
- `registry-installer-hook-drift` (1): e.g. `every_installer_spec_lands_on_a_live_redispatch_arm`
- `foreach-rename-reinstall` (1): e.g. `handle_foreach_rename_reinstall_idiom_resolves_every_literal_element`
- `computed-upvar-link-target` (1): e.g. `namespace_upvar_with_a_computed_word_keeps_the_cell_as_written`
- `fixture-upvar-at-global-level` (1): e.g. `upvar_links_an_array_other_var_to_its_base`
- `stray-close-bracket-recovery` (1): e.g. `analyse_top_level_repairs_stray_close_bracket`
- `fixture-invalid-formal` (1): e.g. `original_parameter_scalar_components_do_not_borrow_an_array_root_trait`
- `unavailable-context-abstain` (1): e.g. `definition_navigation_uses_retained_grammar_instead_of_display_dialect`
- `array-set-element-name-declaration` (1): e.g. `original_array_receiver_symbols_keep_combined_names_and_guest_error_independent`
- `jim-original-variable-symbols` (1): e.g. `original_conditional_variable_receivers_keep_absolute_source_geometry_and_unknown_coverage`
- `lexer-var-scan-debug-assert` (1): e.g. `w215_quiet_for_backslash_continued_param_list`
- `computed-proc-name-home` (1): e.g. `a_dynamically_named_procs_body_homes_to_the_lexical_namespace`
- `dead-store-dynamic-name-callee` (1): e.g. `dynamic_name_out_var_does_not_suppress_caller_dead_store`
- `w307-dict-with-unpacked-command` (1): e.g. `w307_interproc_dict_with_const_element_suppression`
- `w304-kcs-message-wording` (1): e.g. `analyse_emits_w304_for_regexp_pattern_variable`
- `fixture-escaped-separator` (1): e.g. `analyse_w101_rejects_multi_command_subscript`
- `w304-constant-propagation` (1): e.g. `analyse_w304_constant_propagation_dash_value_warns`
- `child-interp-receiver-body` (1): e.g. `original_source_receiver_body_retains_child_declaration_without_native_entry`
- `hosted-source-context-input` (1): e.g. `original_hosted_source_input_keeps_catalogue_naming_and_storage_separate`
- `o105-replay-silent` (1): e.g. `proof_gated_o105_is_a_hint_with_no_auto_fix`
- `s110-payload-roundtrip-silent` (1): e.g. `run_all_checks_reports_s110_payload_roundtrip_end_to_end`
