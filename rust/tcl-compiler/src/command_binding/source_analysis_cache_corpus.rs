const ORIGINAL_CORPUS_INPUTS: &[(&str, &str, &str)] = &[
    (
        "samples/for_screenshots/27-if-to-switch-before.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/27-if-to-switch-before.tcl"
        )),
    ),
    (
        "samples/for_screenshots/20-folding.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/20-folding.tcl"
        )),
    ),
    (
        "samples/for_screenshots/32-document-links.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/32-document-links.tcl"
        )),
    ),
    (
        "samples/for_screenshots/07-formatting-side-by-side.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/07-formatting-side-by-side.tcl"
        )),
    ),
    (
        "samples/for_screenshots/27-if-to-switch-after.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/27-if-to-switch-after.tcl"
        )),
    ),
    (
        "samples/for_screenshots/16-references-long.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/16-references-long.tcl"
        )),
    ),
    (
        "samples/for_screenshots/22-optimiser-before.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/22-optimiser-before.tcl"
        )),
    ),
    (
        "samples/for_screenshots/15-definition-long.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/15-definition-long.tcl"
        )),
    ),
    (
        "samples/proc_arg_traits_examples.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/proc_arg_traits_examples.tcl"
        )),
    ),
    (
        "samples/tclpkg/tclpkg.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/tclpkg/tclpkg.tcl"
        )),
    ),
    (
        "samples/wasm/t4-scopes/42_arrays.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t4-scopes/42_arrays.tcl"
        )),
    ),
    (
        "samples/wasm/t4-scopes/43_namespaces.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t4-scopes/43_namespaces.tcl"
        )),
    ),
    (
        "samples/wasm/t4-scopes/40_global_variable.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t4-scopes/40_global_variable.tcl"
        )),
    ),
    (
        "samples/wasm/t6-tcloo/62_mixin_filter_forward.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t6-tcloo/62_mixin_filter_forward.tcl"
        )),
    ),
    (
        "samples/wasm/t6-tcloo/63_objects_in_data.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t6-tcloo/63_objects_in_data.tcl"
        )),
    ),
    (
        "samples/wasm/t6-tcloo/61_inheritance_next.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t6-tcloo/61_inheritance_next.tcl"
        )),
    ),
    (
        "samples/wasm/t6-tcloo/60_class_basic.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t6-tcloo/60_class_basic.tcl"
        )),
    ),
    (
        "samples/wasm/t7-dynamic/70_var_traces.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t7-dynamic/70_var_traces.tcl"
        )),
    ),
    (
        "samples/wasm/t7-dynamic/72_uplevel_eval.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t7-dynamic/72_uplevel_eval.tcl"
        )),
    ),
    (
        "samples/wasm/t7-dynamic/71_exec_traces_rename.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t7-dynamic/71_exec_traces_rename.tcl"
        )),
    ),
    (
        "samples/wasm/t5-completion/51_try_finally.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t5-completion/51_try_finally.tcl"
        )),
    ),
    (
        "samples/wasm/t5-completion/52_loop_completion.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t5-completion/52_loop_completion.tcl"
        )),
    ),
    (
        "samples/wasm/t5-completion/50_catch_error.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t5-completion/50_catch_error.tcl"
        )),
    ),
    (
        "samples/wasm/t1-expr-control/15_switch.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t1-expr-control/15_switch.tcl"
        )),
    ),
    (
        "samples/wasm/t1-expr-control/13_expr_ops.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t1-expr-control/13_expr_ops.tcl"
        )),
    ),
    (
        "samples/wasm/t1-expr-control/16_conditional_arms.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t1-expr-control/16_conditional_arms.tcl"
        )),
    ),
    (
        "samples/wasm/t2-values/23_dicts.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t2-values/23_dicts.tcl"
        )),
    ),
    (
        "samples/wasm/t2-values/21_foreach.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t2-values/21_foreach.tcl"
        )),
    ),
    (
        "samples/wasm/t2-values/20_lists.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t2-values/20_lists.tcl"
        )),
    ),
    (
        "samples/wasm/t2-values/22_strings.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t2-values/22_strings.tcl"
        )),
    ),
    (
        "samples/wasm/t3-procs/35_native_dispatch_matrix.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t3-procs/35_native_dispatch_matrix.tcl"
        )),
    ),
    (
        "samples/wasm/t3-procs/37_return_state.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t3-procs/37_return_state.tcl"
        )),
    ),
    (
        "samples/wasm/t3-procs/31_recursion.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t3-procs/31_recursion.tcl"
        )),
    ),
    (
        "samples/wasm/t3-procs/36_proc_error_info.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t3-procs/36_proc_error_info.tcl"
        )),
    ),
    (
        "samples/wasm/t3-procs/33_string_builder.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t3-procs/33_string_builder.tcl"
        )),
    ),
    (
        "samples/wasm/t3-procs/32_defaults_and_args.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/wasm/t3-procs/32_defaults_and_args.tcl"
        )),
    ),
    (
        "samples/dialect_stubs_inline_example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/dialect_stubs_inline_example.tcl"
        )),
    ),
    (
        "samples/optimiser/profile_standard.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/optimiser/profile_standard.tcl"
        )),
    ),
    (
        "samples/optimiser/profile_aggressive.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/optimiser/profile_aggressive.tcl"
        )),
    ),
    (
        "samples/optimiser/deep_pipeline.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/optimiser/deep_pipeline.tcl"
        )),
    ),
    (
        "samples/optimiser/profile_readability.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/optimiser/profile_readability.tcl"
        )),
    ),
    (
        "samples/optimiser/profile_full.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/optimiser/profile_full.tcl"
        )),
    ),
    (
        "samples/optimiser/input.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/optimiser/input.tcl"
        )),
    ),
    (
        "samples/diagnostics/W211_set_never_used/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W211_set_never_used/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W105_unbraced_eval/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W105_unbraced_eval/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/S101_shimmer_merge/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/S101_shimmer_merge/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/O110_canonicalise_expr/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/O110_canonicalise_expr/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W102_subst_injection/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W102_subst_injection/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W307_non_literal_command/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W307_non_literal_command/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W301_uplevel_unbraced/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W301_uplevel_unbraced/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W313_file_delete_path/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W313_file_delete_path/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W210_read_before_set/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W210_read_before_set/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W220_assignment_never_read/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W220_assignment_never_read/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W314_no_absolute_name/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W314_no_absolute_name/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W111_long_line/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W111_long_line/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W002_disabled_command/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W002_disabled_command/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W112_trailing_whitespace/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W112_trailing_whitespace/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W304_missing_double_dash/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W304_missing_double_dash/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W001_unknown_subcommand/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W001_unknown_subcommand/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/S102_type_thunking/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/S102_type_thunking/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/O116_fold_constant_list/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/O116_fold_constant_list/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/O106_loop_invariant/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/O106_loop_invariant/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W103_open_pipeline/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W103_open_pipeline/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W101_eval_injection/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W101_eval_injection/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W115_backslash_newline_comment/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W115_backslash_newline_comment/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W214_unused_parameter/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W214_unused_parameter/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/E003_proc_call_arity/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/E003_proc_call_arity/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W306_regexp_substitution/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W306_regexp_substitution/example.tcl"
        )),
    ),
    (
        "samples/diagnostics/W108_non_ascii/example.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/diagnostics/W108_non_ascii/example.tcl"
        )),
    ),
    (
        "samples/tcl/10_format_strings.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/tcl/10_format_strings.tcl"
        )),
    ),
    (
        "samples/tcl/11_minifier_demo.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/tcl/11_minifier_demo.tcl"
        )),
    ),
    (
        "samples/tcl/compiler_explorer_demo.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/tcl/compiler_explorer_demo.tcl"
        )),
    ),
    (
        "samples/tcl/08_tricky_edge_cases.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/tcl/08_tricky_edge_cases.tcl"
        )),
    ),
    (
        "samples/tcl/06_security_smells.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/tcl/06_security_smells.tcl"
        )),
    ),
    (
        "samples/tcl/13_incr_dialect_difference.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/tcl/13_incr_dialect_difference.tcl"
        )),
    ),
    (
        "samples/tcl/12_safe_on_uninit.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/tcl/12_safe_on_uninit.tcl"
        )),
    ),
    (
        "samples/tcl/05_warning_examples.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/tcl/05_warning_examples.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_original_lifecycle_providers/source.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_original_lifecycle_providers/source.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_signature_scope.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_signature_scope.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_namespace_variable_identity/original.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_namespace_variable_identity/original.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_expanded_head/probe.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_expanded_head/probe.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/name-consumers/variables.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/name-consumers/variables.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/name-consumers/aliases.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/name-consumers/aliases.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_source_normal_results/source.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_source_normal_results/source.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_namespace_unknown_lookup/probe.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_namespace_unknown_lookup/probe.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_namespace_unknown_lookup/path_stop.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_namespace_unknown_lookup/path_stop.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_double_completion/source.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_double_completion/source.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_deep_array_source.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_deep_array_source.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_integer_contents_results/source.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_integer_contents_results/source.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/diagnostics/tcltest-shared-frame/3.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/diagnostics/tcltest-shared-frame/3.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/diagnostics/tcltest-shared-frame/4.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/diagnostics/tcltest-shared-frame/4.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/diagnostics/tcltest-shared-frame/2.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/diagnostics/tcltest-shared-frame/2.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/diagnostics/tcltest-shared-frame/1.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/diagnostics/tcltest-shared-frame/1.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_alias_target_context.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_alias_target_context.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/native_itcl/protocols.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/native_itcl/protocols.tcl"
        )),
    ),
    (
        "tests/fixtures/spec-packs/tiny-project/lib/tcl_lsp_fixture.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/spec-packs/tiny-project/lib/tcl_lsp_fixture.tcl"
        )),
    ),
    (
        "tests/fixtures/tcl9/completion-options-control.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/tcl9/completion-options-control.tcl"
        )),
    ),
    (
        "tests/fixtures/tcl9/completion-options-alias.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/tcl9/completion-options-alias.tcl"
        )),
    ),
    (
        "tests/fixtures/tcl9/array-get-read-traces.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/tcl9/array-get-read-traces.tcl"
        )),
    ),
    (
        "tests/fixtures/tcl9/expanded-command-head.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/tcl9/expanded-command-head.tcl"
        )),
    ),
    (
        "tests/external/backend_constraints.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/external/backend_constraints.tcl"
        )),
    ),
    (
        "rust/tcl-compiler/tests/data/name-consumers/classes.tcl",
        "tcl8.6",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-compiler/tests/data/name-consumers/classes.tcl"
        )),
    ),
    (
        "samples/for_screenshots/32-extract-datagroup-mapping-after.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/32-extract-datagroup-mapping-after.irul"
        )),
    ),
    (
        "samples/for_screenshots/31-extract-datagroup-ip-before.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/31-extract-datagroup-ip-before.irul"
        )),
    ),
    (
        "samples/for_screenshots/31-extract-datagroup-ip-after.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/31-extract-datagroup-ip-after.irul"
        )),
    ),
    (
        "samples/for_screenshots/24-template-fallback.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/24-template-fallback.irul"
        )),
    ),
    (
        "samples/for_screenshots/32-extract-datagroup-mapping-before.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/32-extract-datagroup-mapping-before.irul"
        )),
    ),
    (
        "samples/for_screenshots/05-security-taint-before.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/05-security-taint-before.irul"
        )),
    ),
    (
        "samples/for_screenshots/30-extract-datagroup-before.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/30-extract-datagroup-before.irul"
        )),
    ),
    (
        "samples/for_screenshots/05-security-taint-after.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/05-security-taint-after.irul"
        )),
    ),
    (
        "samples/for_screenshots/30-extract-datagroup-after.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/30-extract-datagroup-after.irul"
        )),
    ),
    (
        "samples/for_screenshots/ai-scene.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/for_screenshots/ai-scene.irul"
        )),
    ),
    (
        "samples/irules/cookbook_http_collect.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/irules/cookbook_http_collect.irul"
        )),
    ),
    (
        "samples/irules/diagnostics_event_flow.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/irules/diagnostics_event_flow.irul"
        )),
    ),
    (
        "samples/irules/cookbook_tcp_collect.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/irules/cookbook_tcp_collect.irul"
        )),
    ),
    (
        "samples/irules/diagnostics_security.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/irules/diagnostics_security.irul"
        )),
    ),
    (
        "samples/irules/diagnostics_style_perf.irul",
        "f5-irules",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../samples/irules/diagnostics_style_perf.irul"
        )),
    ),
];

// Test-only current-source execution of the existing complete-report corpus contract.
thread_local! {
    static CORPUS_ORIGINAL_STRONG: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

struct CorpusRetentionMode(bool);
impl CorpusRetentionMode {
    fn enter(original: bool) -> Self {
        Self(CORPUS_ORIGINAL_STRONG.with(|mode| mode.replace(original)))
    }
}
impl Drop for CorpusRetentionMode {
    fn drop(&mut self) {
        CORPUS_ORIGINAL_STRONG.with(|mode| mode.set(self.0));
    }
}

pub(super) fn retain_original_strong_for_corpus(
    key: &SourceAnalysisCacheKey,
    bindings: &SourceCommandBindings,
) -> bool {
    if !CORPUS_ORIGINAL_STRONG.with(std::cell::Cell::get) || key.entry.source_point_entry.is_none()
    {
        return false;
    }
    let mut snapshot = bindings.clone();
    snapshot.source_point_cache_lease = None;
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain(|old| !old.matches(key));
        if cache.len() == MAX_ENTRIES {
            cache.pop_front();
        }
        cache.push_back(CacheEntry::Root {
            key: key.clone(),
            bindings: Arc::new(snapshot),
        });
    });
    true
}

fn corpus_source_point(dialect: &str) -> super::super::OriginalSourcePointEntry {
    let expected = tcl_registry::model::ingress::resolve_known_environment(dialect)
        .expect("every original corpus profile must be genuinely known");
    let source = "source child.tcl";
    let parent = crate::analyser::Analyser::new().analyse(source, dialect);
    let input = parent
        .resolved_input
        .as_ref()
        .expect("genuine corpus parent input");
    assert_eq!(
        input.analyser_profile().name,
        expected.analyser_profile().name
    );
    assert_eq!(input.unit_profile().name, expected.unit_profile().name);
    assert_eq!(
        input.availability_context().environment.id.as_str(),
        expected.id()
    );
    let point = crate::registry_invocation::source_structure::original_project_source_load_at(
        source, &parent, 0,
    )
    .expect("genuine per-profile source load")
    .source_point_entry(&parent)
    .expect("genuine per-profile source point");
    assert!(
        point.input() == input,
        "complete per-profile issued parent input differs"
    );
    point
}

fn corpus_report(
    label: &str,
    source: &str,
    dialect: &str,
    point: &super::super::OriginalSourcePointEntry,
    original: bool,
) -> crate::analyser::AuthenticatedAnalysisReport {
    let _mode = CorpusRetentionMode::enter(original);
    eprintln!("SOURCE_POINT_CORPUS_BEGIN input={label:?} dialect={dialect} original={original}");
    let report =
        crate::analyser::Analyser::new().source_point_report_for_cache_test(source, dialect, point);
    assert!(
        report.analysis().resolved_input.as_ref() == Some(point.input()),
        "complete corpus report input differs from issued point"
    );
    assert!(
        report
            .analysis()
            .retained_command_realm()
            .expect("actual corpus realm")
            .matches_resolved_analysis_input(point.input())
    );
    assert_eq!(point.input().analyser_profile().name, dialect);
    assert_eq!(point.input().unit_profile().name, dialect);
    report
}

fn compare_complete_corpus_input(label: &str, dialect: &str, source: &str) {
    for mixed in [false, true] {
        CACHE.with(|cache| cache.borrow_mut().clear());
        let point = corpus_source_point(dialect);
        let first = corpus_report(label, source, dialect, &point, true);
        // Clear policy-crossing entries; each observation uses the requested real retainer.
        CACHE.with(|cache| cache.borrow_mut().clear());
        let middle = corpus_report(label, source, dialect, &point, !mixed);
        if mixed {
            let warm = corpus_report(label, source, dialect, &point, false);
            assert_eq!(
                middle.compare(&warm),
                crate::analyser::AnalysisReportComparison::Equal,
                "complete warm corpus label={label}"
            );
            let middle_bindings = middle
                .analysis()
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref();
            if label == "calls:10" {
                assert!(
                    middle_bindings.source_point_cache_lease.is_some(),
                    "known eligible original calls10 must exercise the actual lease path"
                );
            }
            if let Some(lease) = &middle_bindings.source_point_cache_lease {
                assert!(
                    Arc::ptr_eq(lease, source_point_report_lease(&warm)),
                    "eligible corpus warm report must retain the same actual lease: {label}"
                );
            }
        }
        CACHE.with(|cache| cache.borrow_mut().clear());
        let last = corpus_report(label, source, dialect, &point, true);
        for (pair, outcome) in [
            ("old-old", first.compare(&last)),
            ("old-middle", first.compare(&middle)),
            ("middle-old", middle.compare(&last)),
        ] {
            assert_eq!(
                outcome,
                crate::analyser::AnalysisReportComparison::Equal,
                "complete corpus label={label} mixed={mixed} pair={pair}"
            );
        }
        eprintln!("SOURCE_POINT_CORPUS input={label:?} mixed={mixed} pairs=3 equal=3");
    }
    CACHE.with(|cache| cache.borrow_mut().clear());
}

#[test]
fn source_point_cache_complete_original_corpus_old_and_mixed_reports_equal() {
    assert!(std::env::var_os("TCL_LSP_DIAGNOSTIC_SOURCE_POINT_MEMO_BYPASS").is_none());

    assert_eq!(ORIGINAL_CORPUS_INPUTS.len(), 117);
    for &(label, dialect, source) in ORIGINAL_CORPUS_INPUTS {
        compare_complete_corpus_input(label, dialect, source);
    }
}

fn generate(kind: &str, n: usize) -> String {
    let mut s = String::new();
    match kind {
        "calls" => {
            s.push_str("proc p {a} {return $a}\n");
            for _ in 0..n {
                s.push_str("p 1\n");
            }
        }
        "procs" => {
            for i in 0..n {
                s.push_str(&format!("proc capped_{i} {{}} {{}}\n"));
            }
        }
        "sets" => {
            for i in 0..n {
                s.push_str(&format!("set v{i} {i}\n"));
            }
        }
        "upvar" => {
            s.push_str("proc inc {name} {upvar 1 $name v; incr v}\n");
            for i in 0..n {
                s.push_str(&format!(
                    "proc c{i} {{}} {{set x{i} 1; inc x{i}; return $x{i}}}\nc{i}\n"
                ));
            }
        }
        _ => panic!("kind"),
    }
    s
}

#[test]
fn source_point_cache_original_generated_old_and_mixed_reports_equal() {
    assert!(std::env::var_os("TCL_LSP_DIAGNOSTIC_SOURCE_POINT_MEMO_BYPASS").is_none());
    let cases = [
        ("calls", 10),
        ("calls", 20),
        ("calls", 40),
        ("calls", 80),
        ("procs", 41),
        ("upvar", 10),
        ("upvar", 30),
    ];
    assert_eq!(cases.len(), 7);
    for (kind, size) in cases {
        compare_complete_corpus_input(&format!("{kind}:{size}"), "tcl8.6", &generate(kind, size));
    }
}
