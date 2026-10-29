# CI run 5248 (WORK, head 83a0b2351) vs origin/rust CI 5203 baseline: per-test comparison

Analysis only. Source: GitHub Actions run 40739740045 job logs plus run artifacts (terminal-capture, nextest listings, lsp-e2e result bundles). Baseline: 24,312 named PASS identities (binary_id, selector). Join key: exact (binary_id, selector). A missing or unobserved outcome is never counted as a pass.

## Headline

| Baseline PASS identity outcome on WORK | Count |
|---|---:|
| pass -> pass | 10,962 |
| **pass -> fail** (WORK terminal status failed) | **2,835** |
| unobserved (listed and selected on WORK, no outcome recorded; jobs killed or hung) | 10,299 |
| absent (in no WORK listing at all: renamed, moved or deleted) | 216 |
| now `#[ignore]`d on WORK | 0 |
| total | 24,312 |

- All 2,835 pass->fail rows are nextest `failed` terminal statuses (panics/assertions). No timeout or abort status was detected for any of them (console artifacts show only PASS/FAIL/SLOW lines; for shards 1 and 4 only the 128-byte output sample in the terminal record was available to check). Sources: {'job-log capture': 2539, 'console stderr artifact (2 MiB cap)': 296}.
- fail -> pass: 0. The 5203 receipt has 0 named FAIL rows (only 4,625 unnamed summary skips), so no baseline failure exists to flip.
- New on WORK (not in the baseline receipt): 2,722 pass, 285 fail, 18 ignored, 6,740 listed but unobserved. Some of these may have been baseline skips (unnamed in the receipt), so they are not necessarily new source tests.
- WORK listing universe: 33,843 selected tests across the 5 rust shards, rust-tests-heavy and 3 lsp-e2e partitions (nextest listing artifacts match the log `selected` records exactly: True); 26 `#[ignore]`d tests listed. WORK outcomes observed: 16,822.

## Per job (nextest-backed jobs)

| Job | Conclusion | Selected | Outcomes recorded | Pass | Fail | Ignored | Selected, no outcome | Baseline P2F |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| rust-tests-shard (1/5) | failure | 10,781 | 1,464 | 1,118 | 346 | 0 | 9,317 | 278 |
| rust-tests-shard (2/5) | cancelled | 7,954 | 1,194 | 896 | 298 | 0 | 6,760 | 296 |
| rust-tests-shard (3/5) | failure | 3,917 | 3,919 | 3,295 | 622 | 2 | 0 | 599 |
| rust-tests-shard (4/5) | failure | 3,029 | 2,080 | 1,551 | 519 | 10 | 959 | 465 |
| rust-tests-shard (5/5) | failure | 5,243 | 5,244 | 4,642 | 601 | 1 | 0 | 548 |
| rust-tests-heavy | failure | 356 | 356 | 327 | 29 | 0 | 0 | 0 |
| lsp-e2e-partition (1/3) | cancelled | 882 | 885 | 639 | 241 | 5 | 2 | 225 |
| lsp-e2e-partition (2/3) | cancelled | 840 | 844 | 614 | 225 | 0 | 1 | 207 |
| lsp-e2e-partition (3/3) | failure | 841 | 846 | 602 | 239 | 0 | 0 | 217 |

rust-tests-shard (2/5): 0 terminal records in the job log; its 1,194 outcomes (console ordinals 1..1194, contiguous) come from the 2 MiB-capped nextest-stderr artifact. Pass/Fail/Ignored include all observed outcomes (log plus console); Ignored counts are those recorded as terminal records (the listings show 26 ignored tests overall).

Console cross-check: where both the job-log terminal records and the nextest console stderr artifact cover a test (shard 3, shard 5, heavy, lsp-e2e 1-3), 0 disagreements: rust-tests-shard (3/5): 3845 agree, rust-tests-shard (5/5): 1993 agree, rust-tests-heavy: 356 agree, lsp-e2e-partition (1/3): 70 agree, lsp-e2e-partition (2/3): 839 agree, lsp-e2e-partition (3/3): 841 agree.

## Unobserved baseline PASS and why

| WORK job | Baseline PASS unobserved | Reason |
|---|---:|---|
| rust-tests-shard (1/5) | 5,436 | job cancelled: runner received shutdown signal at 18:15:42Z, ~11 min into nextest; no terminal-capture artifact (upload step skipped); only 1464 of 10781 selected tests had terminal records in the job log |
| rust-tests-shard (2/5) | 4,071 | job cancelled after ~5h34m with nextest never completing (SLOW > 120s: tcl-cli::value_transfers_cli opt_* tests); capture terminal.jsonl is empty (0 bytes), 0 terminal records in log; only the first 1194 console ordinals survive in the 2 MiB-capped nextest-stderr artifact |
| rust-tests-shard (4/5) | 789 | job lost: step "cargo nextest run" never recorded a conclusion, job log ends at 18:57:19Z (job marked failure 19:42:04Z); no terminal-capture artifact; 2080 of 3029 selected tests had terminal records in the job log |
| lsp-e2e-partition (1/3) | 2 | job cancelled at 00:10Z after nextest hung (last terminal record 19:15Z); remaining tests never reported |
| lsp-e2e-partition (2/3) | 1 | job cancelled at 00:09Z after nextest hung (last terminal record 20:15Z); remaining tests never reported |
| total | 10,299 | |

Hung tests that were started and never finished (kept unobserved, not counted as pass or fail): lsp-e2e 1/3 `tcl-lsp-server tests::an_edit_during_a_config_apply_re_resolves_its_diagnostics_inputs` and `tests::default_dialect_change_retries_after_concurrent_edit_1854`; lsp-e2e 2/3 `tcl-lsp-server tests::a_late_resolve_cannot_stamp_the_slot_backwards` (console shows SLOW > 9960s; the job was cancelled at 00:09Z). Shard 2 console shows `tcl-cli::value_transfers_cli` opt_* tests SLOW > 120s as the last activity before the job was cancelled after 5h34m.

## Absent baseline PASS (216)

Not present in any WORK nextest listing (matching or ignored). Likely renamed/moved/deleted tests; 1 has the identical selector listed under a different binary_id, and 7 tcl-registry `resolved_invocation::tests::*` identities share only their leaf test name with new `resolved_invocation::logical_frame_source_role_tests::*` tests (likely module move; not asserted). Not counted as pass or fail. By binary: tcl-compiler 131, tcl-lsp-core 21, tcl-diagram 11, tcl-lsp-core::code_actions_depth 9, tcl-registry 8, tcl-irules 6, tcl-vm 5, tcl-compiler::inlining 4, tcl-lsp-server 3, tcl-lsp-server::e2e 3, tcl-lsp-core::lsp_edit_workspace 2, tcl-lsp-db 2. Full list in compare.json under baseline_pass_absent.

## Per-binary table (binaries with pass->fail > 0, sorted by pass->fail)

| binary_id | baseline PASS | pass->pass | **pass->fail** | unobserved | absent |
|---|---:|---:|---:|---:|---:|
| tcl-lsp-server::e2e | 1629 | 1101 | 525 | 0 | 3 |
| tcl-compiler | 6760 | 915 | 278 | 5436 | 131 |
| tcl-compiler::analyser | 506 | 334 | 172 | 0 | 0 |
| tcl-lsp-server | 622 | 501 | 115 | 3 | 3 |
| tcl-compiler::codegen | 167 | 61 | 106 | 0 | 0 |
| tcl-compiler::taint | 373 | 294 | 79 | 0 | 0 |
| tcl-compiler::optimiser | 105 | 42 | 63 | 0 | 0 |
| tcl-compiler::core_analyses | 109 | 48 | 61 | 0 | 0 |
| tcl-compiler::optimiser_coverage | 93 | 38 | 55 | 0 | 0 |
| tcl-compiler::var_escape_typeinfer | 70 | 19 | 51 | 0 | 0 |
| tcl-compiler::compiler_analysis_residual | 119 | 74 | 45 | 0 | 0 |
| tcl-compiler::inlining | 72 | 23 | 45 | 0 | 4 |
| tcl-compiler::checks | 274 | 232 | 42 | 0 | 0 |
| tcl-compiler::inlining_interproc_residual | 59 | 20 | 39 | 0 | 0 |
| tcl-compiler::type_propagation | 39 | 0 | 39 | 0 | 0 |
| tcl-compiler::var_escape_residual2 | 51 | 15 | 36 | 0 | 0 |
| tcl-compiler::dataflow | 45 | 10 | 35 | 0 | 0 |
| tcl-compiler::codegen_depth | 55 | 20 | 34 | 0 | 1 |
| tcl-explorer | 114 | 83 | 31 | 0 | 0 |
| tcl-vm::opcode_c_parity | 86 | 55 | 31 | 0 | 0 |
| tcl-cli::value_transfers_cli | 39 | 8 | 30 | 1 | 0 |
| tcl-compiler::optimiser_codegen_residual | 67 | 37 | 30 | 0 | 0 |
| tcl-vm::command_mutation_deopt_e2e | 84 | 55 | 29 | 0 | 0 |
| tcl-cli::cli | 68 | 41 | 27 | 0 | 0 |
| tcl-lsp-core::command_prefix_integration | 34 | 7 | 27 | 0 | 0 |
| tcl-compiler::taint_depth | 130 | 104 | 26 | 0 | 0 |
| tcl-vm::builtins_e2e | 126 | 101 | 25 | 0 | 0 |
| tcl-compiler::pipeline_coverage | 61 | 37 | 24 | 0 | 0 |
| tcl-compiler::compiler_residual | 59 | 36 | 23 | 0 | 0 |
| tcl-compiler::inlining_rename | 26 | 3 | 23 | 0 | 0 |
| tcl-spectcl::codegen_stamps | 29 | 6 | 23 | 0 | 0 |
| tcl-lsp-core::stub_arg_roles | 35 | 15 | 20 | 0 | 0 |
| tcl-diagram | 36 | 6 | 19 | 0 | 11 |
| tcl-compiler::var_escape_cfg_catch | 24 | 6 | 18 | 0 | 0 |
| tcl-lsp-core::command_table_mutation | 37 | 20 | 17 | 0 | 0 |
| tcl-compiler::wasm_codegen | 46 | 30 | 16 | 0 | 0 |
| tcl-mcp::bin/tcl-mcp | 115 | 98 | 16 | 0 | 1 |
| tcl-compiler::fp_depth | 71 | 56 | 15 | 0 | 0 |
| tcl-compiler::optimiser_depth | 20 | 5 | 15 | 0 | 0 |
| tcl-irules | 63 | 42 | 15 | 0 | 6 |
| tcl-lsp-db | 111 | 37 | 15 | 57 | 2 |
| tcl-compiler::cfg | 23 | 9 | 14 | 0 | 0 |
| tcl-compiler::tcl_expr_eval | 73 | 59 | 14 | 0 | 0 |
| tcl-lsp-core::minify_residual | 39 | 25 | 14 | 0 | 0 |
| tcl-vm::namespace_surface_e2e | 123 | 109 | 14 | 0 | 0 |
| tcl-lsp-core::code_actions_depth | 46 | 24 | 13 | 0 | 9 |
| tcl-spec-studio | 242 | 228 | 13 | 0 | 1 |
| tcl-compiler::side_effects_binding | 68 | 56 | 12 | 0 | 0 |
| tcl-lsp-core::completion_residual | 37 | 25 | 12 | 0 | 0 |
| tcl-lsp-core::oo_mro_parity | 15 | 3 | 12 | 0 | 0 |
| tcl-lsp-core::semantic_tokens_residual | 63 | 51 | 12 | 0 | 0 |
| tcl-vm::run_script | 98 | 86 | 12 | 0 | 0 |
| tcl-vm::variable_trace_semantics_e2e | 26 | 15 | 11 | 0 | 0 |
| tcl-vm::cmd_oo_e2e | 73 | 63 | 10 | 0 | 0 |
| tcl-compiler::caller_frame_effects | 21 | 12 | 9 | 0 | 0 |
| tcl-cshim | 77 | 68 | 9 | 0 | 0 |
| tcl-lsp-core::lsp_navigation | 42 | 33 | 9 | 0 | 0 |
| tcl-lsp-server::preview_tickets_e2e | 24 | 15 | 9 | 0 | 0 |
| tcl-vm::cmd_control_e2e | 108 | 98 | 9 | 0 | 1 |
| f5-xc::model | 24 | 16 | 8 | 0 | 0 |
| tcl-compiler::alias_scoping | 30 | 22 | 8 | 0 | 0 |
| tcl-compiler::mro_lattice_adversarial | 9 | 1 | 8 | 0 | 0 |
| tcl-lsp-core::formatting_keywords | 29 | 21 | 8 | 0 | 0 |
| tcl-lsp-core::mathfunc_and_word_recognition | 38 | 30 | 8 | 0 | 0 |
| tcl-spec-hooks::families_e2e | 40 | 32 | 8 | 0 | 0 |
| tcl-compiler::var_escape_cfg_tree_arms | 7 | 0 | 7 | 0 | 0 |
| tcl-lsp-core::lsp_providers | 51 | 44 | 7 | 0 | 0 |
| tcl-vm::opcode_catch_parity | 17 | 10 | 7 | 0 | 0 |
| tcl-vm::release_number_semantics_e2e | 14 | 7 | 7 | 0 | 0 |
| bpf-tcl::profiles | 10 | 4 | 6 | 0 | 0 |
| tcl-compiler::unused_var_tricky_features | 28 | 22 | 6 | 0 | 0 |
| tcl-irules::event_emission | 8 | 2 | 6 | 0 | 0 |
| tcl-lsp-core::lsp_lens_links_symbols | 45 | 39 | 6 | 0 | 0 |
| tcl-lsp-core::name_resolution | 75 | 69 | 6 | 0 | 0 |
| bpf-tcl::kernel_codegen | 10 | 5 | 5 | 0 | 0 |
| tcl-compiler::dialect_threading | 13 | 7 | 5 | 1 | 0 |
| tcl-compiler::slot_allocation | 5 | 0 | 5 | 0 | 0 |
| tcl-vm::c_error_paths_e2e | 20 | 15 | 5 | 0 | 0 |
| bpf-tcl-ir | 50 | 46 | 4 | 0 | 0 |
| tcl-cli::spec_verbs | 48 | 44 | 4 | 0 | 0 |
| tcl-compiler::codegen_integration | 18 | 14 | 4 | 0 | 0 |
| tcl-compiler::fix_safety | 15 | 11 | 4 | 0 | 0 |
| tcl-compiler::span_char_boundaries | 14 | 10 | 4 | 0 | 0 |
| tcl-compiler::tcl_namespace_probe | 7 | 3 | 4 | 0 | 0 |
| tcl-cshim::pkga_e2e | 11 | 7 | 4 | 0 | 0 |
| tcl-lsp-core::force_import_shadow_consumers | 10 | 6 | 4 | 0 | 0 |
| tcl-lsp-core::lsp_core_residual | 49 | 45 | 4 | 0 | 0 |
| tcl-registry | 1061 | 1049 | 4 | 0 | 8 |
| tcl-registry::registry_sweep | 43 | 38 | 4 | 0 | 1 |
| tcl-spec-hooks::containment_e2e | 12 | 8 | 4 | 0 | 0 |
| bigip-report-gen-rust::certs | 17 | 14 | 3 | 0 | 0 |
| bpf-tcl::composition | 4 | 1 | 3 | 0 | 0 |
| bpf-tcl::diagnostics | 7 | 4 | 3 | 0 | 0 |
| bpf-tcl::hardening | 18 | 15 | 3 | 0 | 0 |
| bpf-tcl::socket_filter | 8 | 5 | 3 | 0 | 0 |
| tcl-bigip | 303 | 300 | 3 | 0 | 0 |
| tcl-compiler::differential_segment | 4 | 1 | 3 | 0 | 0 |
| tcl-compiler::release_units | 5 | 2 | 3 | 0 | 0 |
| tcl-cshim::doors_e2e | 4 | 1 | 3 | 0 | 0 |
| tcl-irules::script_bearing_args | 5 | 2 | 3 | 0 | 0 |
| tcl-lsp-core::hover_residual | 36 | 33 | 3 | 0 | 0 |
| tcl-registry::registry_commands | 105 | 102 | 3 | 0 | 0 |
| tcl-vm::command_traces_e2e | 5 | 2 | 3 | 0 | 0 |
| tcl-vm::cross_version_info_surface_e2e | 6 | 3 | 3 | 0 | 0 |
| tcl-vm::embed_api_e2e | 22 | 19 | 3 | 0 | 0 |
| tcl-vm::step_trace_inline_e2e | 4 | 1 | 3 | 0 | 0 |
| tcl-vm::variable_name_resolution_e2e | 7 | 4 | 3 | 0 | 0 |
| bigip-report-gen-rust | 70 | 68 | 2 | 0 | 0 |
| bpf-tcl-codegen | 28 | 26 | 2 | 0 | 0 |
| bpf-tcl::templates | 8 | 6 | 2 | 0 | 0 |
| bpf-tcl::xdp | 7 | 5 | 2 | 0 | 0 |
| tcl-bigip::integration | 42 | 40 | 2 | 0 | 0 |
| tcl-cli | 51 | 49 | 2 | 0 | 0 |
| tcl-cli::compile_verbs | 11 | 9 | 2 | 0 | 0 |
| tcl-cli::pkg_verbs | 16 | 14 | 2 | 0 | 0 |
| tcl-compiler::recovery_positions | 5 | 3 | 2 | 0 | 0 |
| tcl-compiler::wasm_tiers | 7 | 5 | 2 | 0 | 0 |
| tcl-irules::object_refs | 2 | 0 | 2 | 0 | 0 |
| tcl-lsp-core::datagroup_residual | 61 | 59 | 2 | 0 | 0 |
| tcl-lsp-core::folding | 16 | 14 | 2 | 0 | 0 |
| tcl-lsp-db::interned_gc | 3 | 1 | 2 | 0 | 0 |
| tcl-spec-studio::schema_coverage | 8 | 6 | 2 | 0 | 0 |
| tcl-spectcl::workspace_packs | 15 | 13 | 2 | 0 | 0 |
| tcl-vm::cmd_math_expr_e2e | 102 | 100 | 2 | 0 | 0 |
| tcl-vm::cross_interp_alias_e2e | 4 | 2 | 2 | 0 | 0 |
| tcl-vm::mathfunc_derivation_e2e | 9 | 7 | 2 | 0 | 0 |
| tcl-vm::safe_interp_e2e | 13 | 11 | 2 | 0 | 0 |
| bigip-report-gen-rust::report | 18 | 17 | 1 | 0 | 0 |
| bpf-tcl::deploy | 8 | 7 | 1 | 0 | 0 |
| bpf-tcl::loops | 6 | 5 | 1 | 0 | 0 |
| f5-xc::differential | 1 | 0 | 1 | 0 | 0 |
| tcl-cli::explorer_gui | 3 | 2 | 1 | 0 | 0 |
| tcl-compiler::codegen_golden | 1 | 0 | 1 | 0 | 0 |
| tcl-compiler::command_resolution_conformance | 1 | 0 | 1 | 0 | 0 |
| tcl-compiler::complexity_guard | 11 | 10 | 1 | 0 | 0 |
| tcl-compiler::differential_codegen | 9 | 8 | 1 | 0 | 0 |
| tcl-compiler::differential_incremental | 2 | 1 | 1 | 0 | 0 |
| tcl-compiler::per_item_corpus | 1 | 0 | 1 | 0 | 0 |
| tcl-compiler::real_job_fixtures | 2 | 1 | 1 | 0 | 0 |
| tcl-compiler::wasm_api_architecture | 7 | 6 | 1 | 0 | 0 |
| tcl-cshim::completion | 6 | 5 | 1 | 0 | 0 |
| tcl-irules::walker | 1 | 0 | 1 | 0 | 0 |
| tcl-lexer::lexer_depth | 65 | 64 | 1 | 0 | 0 |
| tcl-lsp-core::authoring_dialect_completion | 7 | 6 | 1 | 0 | 0 |
| tcl-lsp-core::lsp_edit_workspace | 37 | 34 | 1 | 0 | 2 |
| tcl-registry::sslictcl_pack | 9 | 8 | 1 | 0 | 0 |
| tcl-spec-studio::spectcl_ports | 11 | 10 | 1 | 0 | 0 |
| tcl-vm-cli::bin/tclvm | 9 | 8 | 1 | 0 | 0 |
| tcl-vm-cli::script_errors | 5 | 4 | 1 | 0 | 0 |
| tcl-vm::cross_version_escapes_e2e | 3 | 2 | 1 | 0 | 0 |
| tcl-vm::literal_index_encoding_e2e | 2 | 1 | 1 | 0 | 0 |
| tcl-vm::release_running_bytecode_e2e | 1 | 0 | 1 | 0 | 0 |
| tcl-vm::rename_return_source_e2e | 3 | 2 | 1 | 0 | 0 |
| tcl-vm::tclsh_parity_e2e | 2 | 1 | 1 | 0 | 0 |

Binaries with zero pass->fail but unobserved or absent baseline PASS:

| binary_id | baseline PASS | pass->pass | unobserved | absent |
|---|---:|---:|---:|---:|
| tcl-bigip-query::integration | 97 | 96 | 0 | 1 |
| tcl-cmd-core | 181 | 180 | 0 | 1 |
| tcl-compiler::differential_word_expr | 6 | 1 | 5 | 0 |
| tcl-compiler::entry_contract_abstention | 2 | 0 | 2 | 0 |
| tcl-compiler::intervals | 74 | 0 | 74 | 0 |
| tcl-compiler::irules_event_context | 4 | 0 | 4 | 0 |
| tcl-compiler::phi_undef_memo | 14 | 0 | 14 | 0 |
| tcl-compiler::regex_source_oracle | 4 | 0 | 4 | 0 |
| tcl-compiler::value_transfer_witnesses | 175 | 0 | 175 | 0 |
| tcl-compiler::var_escape_cfg | 44 | 0 | 44 | 0 |
| tcl-debugger | 15 | 14 | 1 | 0 |
| tcl-engine-tclvm | 23 | 0 | 23 | 0 |
| tcl-engine-tclvm::doors | 15 | 0 | 15 | 0 |
| tcl-irules::resolve_args | 1 | 0 | 1 | 0 |
| tcl-lexer::differential_boundaries | 3 | 0 | 3 | 0 |
| tcl-lexer::f5_lexical_conformance | 1 | 0 | 1 | 0 |
| tcl-lexer::parse_expr | 49 | 0 | 49 | 0 |
| tcl-lexer::parse_lexer | 63 | 0 | 63 | 0 |
| tcl-lsp-core | 2353 | 0 | 2332 | 21 |
| tcl-lsp-core::bigip_code_actions | 14 | 0 | 14 | 0 |
| tcl-lsp-core::docstring | 37 | 0 | 37 | 0 |
| tcl-lsp-core::file_ops_bigip_residual | 34 | 0 | 34 | 0 |
| tcl-lsp-core::references_rename | 98 | 0 | 98 | 0 |
| tcl-lsp-core::references_residual | 23 | 0 | 23 | 0 |
| tcl-lsp-core::semantic_tokens | 82 | 0 | 82 | 0 |
| tcl-lsp-db::class_factory_fixpoint | 13 | 0 | 13 | 0 |
| tcl-lsp-db::compiler_check_corpus | 5 | 0 | 5 | 0 |
| tcl-lsp-db::dialect_seam | 5 | 0 | 5 | 0 |
| tcl-lsp-db::overlay_generations | 4 | 0 | 4 | 0 |
| tcl-pkg | 56 | 0 | 56 | 0 |
| tcl-pkg-model | 45 | 0 | 45 | 0 |
| tcl-regex::collating_names_oracle | 1 | 0 | 1 | 0 |
| tcl-regex::dos_bounds | 12 | 0 | 12 | 0 |
| tcl-regex::lazy_alt_backref | 5 | 0 | 5 | 0 |
| tcl-regex::nocase_posix_class | 1 | 0 | 1 | 0 |
| tcl-regex::precision_oracle | 4 | 0 | 4 | 0 |
| tcl-regex::reg_test | 2 | 0 | 2 | 0 |
| tcl-regex::z_anchor | 4 | 0 | 4 | 0 |
| tcl-registry::analyser_hooks | 4 | 0 | 4 | 0 |
| tcl-registry::dialect_oracle | 3 | 0 | 3 | 0 |
| tcl-registry::differential_fold | 27 | 0 | 27 | 0 |
| tcl-registry::keyword_abbreviations | 6 | 0 | 6 | 0 |
| tcl-registry::library_axis | 7 | 0 | 7 | 0 |
| tcl-registry::provider_relation | 5 | 0 | 5 | 0 |
| tcl-registry::registry | 7 | 0 | 7 | 0 |
| tcl-registry::snapshot | 7 | 0 | 7 | 0 |
| tcl-registry::spec_pack_provenance | 5 | 0 | 5 | 0 |
| tcl-registry::spectcl_pack | 9 | 0 | 9 | 0 |
| tcl-registry::tcltest_specs | 10 | 0 | 10 | 0 |
| tcl-registry::value_transfers | 80 | 0 | 80 | 0 |
| tcl-runtime-api | 49 | 48 | 0 | 1 |
| tcl-spec-hooks | 10 | 0 | 10 | 0 |
| tcl-spec-hooks::const_fold_e2e | 6 | 0 | 6 | 0 |
| tcl-spec-studio::option_row_editing | 6 | 0 | 6 | 0 |
| tcl-spec-studio::pack_store | 7 | 0 | 7 | 0 |
| tcl-spec-studio::reference_doc | 1 | 0 | 1 | 0 |
| tcl-spec-studio::render_sweep | 2 | 0 | 2 | 0 |
| tcl-spec-studio::spectcl_roundtrip | 12 | 0 | 12 | 0 |
| tcl-spectcl | 246 | 244 | 0 | 2 |
| tcl-spectcl::disabled_command_wording | 5 | 0 | 5 | 0 |
| tcl-spectcl::eval_loader | 27 | 0 | 27 | 0 |
| tcl-spectcl::export | 10 | 0 | 10 | 0 |
| tcl-spectcl::i6_security_floor | 10 | 0 | 10 | 0 |
| tcl-spectcl::jim_document | 26 | 0 | 26 | 0 |
| tcl-spectcl::jim_generation_zero | 3 | 0 | 3 | 0 |
| tcl-spectcl::jim_own_surface | 10 | 0 | 10 | 0 |
| tcl-spectcl::jim_seed_after_publish | 1 | 0 | 1 | 0 |
| tcl-spectcl::jim_surface_roster | 4 | 0 | 4 | 0 |
| tcl-spectcl::pack_is_real_tcl | 1 | 0 | 1 | 0 |
| tcl-spectcl::pack_source_e2e | 6 | 0 | 6 | 0 |
| tcl-spectcl::pack_torture | 39 | 0 | 39 | 0 |
| tcl-spectcl::roster_reload_static_view | 2 | 0 | 2 | 0 |
| tcl-spectcl::spec_corpus | 5 | 0 | 5 | 0 |
| tcl-spectcl::spectcl_dialect_analysis | 3 | 0 | 3 | 0 |
| tcl-sslictcl | 73 | 0 | 73 | 0 |
| tcl-sslictcl::golden | 2 | 0 | 2 | 0 |
| tcl-sslictcl::registry_pack_drift | 8 | 0 | 8 | 0 |
| tcl-syntax::command_resolution_conformance | 3 | 0 | 3 | 0 |
| tcl-syntax::expr_parser | 21 | 0 | 21 | 0 |
| tcl-syntax::expr_parser_coverage | 98 | 0 | 98 | 0 |
| tcl-syntax::expr_parser_depth | 24 | 0 | 24 | 0 |
| tcl-syntax::namespace_op_conformance | 2 | 0 | 2 | 0 |
| tcl-userdirs | 2 | 0 | 2 | 0 |
| tcl-vm | 116 | 0 | 111 | 5 |
| tcl-vm-cli::channel_output | 12 | 0 | 12 | 0 |
| tcl-vm-cli::static_extensions | 4 | 0 | 4 | 0 |
| tcl-vm-cli::top_level_e2e | 10 | 0 | 10 | 0 |
| tcl-vm::capability | 6 | 0 | 6 | 0 |
| tcl-vm::cmd_collections_e2e | 67 | 0 | 67 | 0 |
| tcl-vm::cmd_coro_e2e | 56 | 0 | 56 | 0 |
| tcl-vm::cmd_info_prefix_e2e | 83 | 0 | 83 | 0 |
| tcl-vm::cmd_string_e2e | 51 | 0 | 51 | 0 |
| tcl-vm::cmd_thread_e2e | 26 | 0 | 26 | 0 |
| tcl-vm::command_resolution_conformance | 1 | 0 | 1 | 0 |
| tcl-vm::cross_interp_reentry_e2e | 2 | 0 | 2 | 0 |
| tcl-vm::cross_version_command_surface_e2e | 38 | 0 | 38 | 0 |
| tcl-vm::cross_version_expr_surface_e2e | 6 | 0 | 6 | 0 |
| tcl-vm::cross_version_numerals_e2e | 4 | 0 | 4 | 0 |
| tcl-vm::cross_version_vars_e2e | 28 | 0 | 28 | 0 |
| tcl-vm::element_recovery_traces_e2e | 3 | 0 | 3 | 0 |
| tcl-vm::encoding_e2e | 1 | 0 | 1 | 0 |
| tcl-vm::env_host_sync_e2e | 13 | 0 | 13 | 0 |
| tcl-vm::expanded_invoke_e2e | 2 | 0 | 2 | 0 |
| tcl-vm::expr_surface_e2e | 2 | 0 | 2 | 0 |
| tcl-vm::integer_argument_reads_e2e | 15 | 0 | 15 | 0 |
| tcl-vm::language_e2e | 39 | 0 | 39 | 0 |
| tcl-vm::legacy_variable_traces_e2e | 3 | 2 | 0 | 1 |
| tcl-vm::namespace_colon_runs_e2e | 22 | 0 | 22 | 0 |
| tcl-vm::numeric_tower_e2e | 18 | 0 | 18 | 0 |
| tcl-vm::opcode_dispatch_coverage | 3 | 0 | 3 | 0 |
| tcl-vm::package_coro_e2e | 12 | 0 | 12 | 0 |
| tcl-vm::package_diagnostics_e2e | 5 | 0 | 5 | 0 |
| tcl-vm::regexp_cmd_corpus | 1 | 0 | 1 | 0 |
| tcl-vm::regexp_e2e | 8 | 0 | 8 | 0 |
| tcl-vm::switch_integer_e2e | 3 | 0 | 3 | 0 |
| tcl-vm::tcltest_load | 13 | 0 | 13 | 0 |
| tcl-vm::wasm_coro_e2e | 3 | 0 | 3 | 0 |
| tcl-vm::word_parts_owner_e2e | 4 | 0 | 4 | 0 |
| xtask::bin/xtask | 275 | 0 | 275 | 0 |

## Top 20 pass->fail clusters by parent module path (binary_id + selector minus test name)

| pass->fail | binary_id | selector prefix |
|---:|---|---|
| 122 | tcl-compiler | analyser::diagnostics::tests |
| 112 | tcl-lsp-server | tests |
| 106 | tcl-compiler::codegen | (flat integration target, no module path) |
| 84 | tcl-compiler::analyser | class_factories |
| 69 | tcl-lsp-server::e2e | diagnostics |
| 62 | tcl-compiler::optimiser | (flat integration target, no module path) |
| 55 | tcl-compiler::optimiser_coverage | (flat integration target, no module path) |
| 51 | tcl-compiler::var_escape_typeinfer | (flat integration target, no module path) |
| 45 | tcl-compiler::compiler_analysis_residual | (flat integration target, no module path) |
| 45 | tcl-compiler::inlining | (flat integration target, no module path) |
| 40 | tcl-lsp-server::e2e | tcloo_navigation |
| 39 | tcl-compiler::inlining_interproc_residual | (flat integration target, no module path) |
| 39 | tcl-compiler::type_propagation | (flat integration target, no module path) |
| 36 | tcl-compiler::var_escape_residual2 | (flat integration target, no module path) |
| 35 | tcl-compiler::dataflow | (flat integration target, no module path) |
| 34 | tcl-compiler::codegen_depth | (flat integration target, no module path) |
| 32 | tcl-compiler | analyser::diagnostics::fp::rbs |
| 31 | tcl-vm::opcode_c_parity | (flat integration target, no module path) |
| 31 | tcl-lsp-server::e2e | rename |
| 30 | tcl-cli::value_transfers_cli | (flat integration target, no module path) |

## Top 20 pass->fail clusters by first module segment

| pass->fail | binary_id | selector prefix |
|---:|---|---|
| 278 | tcl-compiler | analyser |
| 112 | tcl-lsp-server | tests |
| 106 | tcl-compiler::codegen | (flat integration target, no module path) |
| 84 | tcl-compiler::analyser | class_factories |
| 69 | tcl-lsp-server::e2e | diagnostics |
| 62 | tcl-compiler::optimiser | (flat integration target, no module path) |
| 55 | tcl-compiler::optimiser_coverage | (flat integration target, no module path) |
| 51 | tcl-compiler::var_escape_typeinfer | (flat integration target, no module path) |
| 45 | tcl-compiler::compiler_analysis_residual | (flat integration target, no module path) |
| 45 | tcl-compiler::inlining | (flat integration target, no module path) |
| 40 | tcl-lsp-server::e2e | tcloo_navigation |
| 39 | tcl-compiler::inlining_interproc_residual | (flat integration target, no module path) |
| 39 | tcl-compiler::type_propagation | (flat integration target, no module path) |
| 36 | tcl-compiler::var_escape_residual2 | (flat integration target, no module path) |
| 35 | tcl-compiler::dataflow | (flat integration target, no module path) |
| 34 | tcl-compiler::codegen_depth | (flat integration target, no module path) |
| 31 | tcl-vm::opcode_c_parity | (flat integration target, no module path) |
| 31 | tcl-lsp-server::e2e | rename |
| 30 | tcl-cli::value_transfers_cli | (flat integration target, no module path) |
| 30 | tcl-compiler::optimiser_codegen_residual | (flat integration target, no module path) |

## Top panic sites among pass->fail (from 128-byte terminal-capture samples; 1,714 rows have no site in the sample)

| count | site |
|---:|---|
| 57 | rust/tcl-compiler/tests/support/body_identity.rs:18 |
| 29 | rust/tcl-vm/src/exec.rs:1792 |
| 27 | rust/tcl-compiler/tests/type_propagation.rs:89 |
| 23 | rust/tcl-vm/src/exec.rs:1905 |
| 20 | rust/tcl-compiler/tests/var_escape_typeinfer.rs:164 |
| 19 | rust/tcl-compiler/tests/var_escape_typeinfer.rs:124 |
| 16 | rust/tcl-compiler/tests/var_escape_residual2.rs:143 |
| 8 | rust/tcl-compiler/tests/support/body_identity.rs:1 |
| 8 | rust/tcl-vm/src/value.rs:7237 |
| 8 | rust/tcl-lsp-server/tests/e2e/common/mod.rs:855 |
| 7 | rust/tcl-vm/tests/cmd_control_e2e.rs:75 |

Frequent panic messages across all failed tests in the console artifacts (shards 2,3,5, heavy, lsp-e2e 3): `fixture body "::p" absent` (tcl-compiler tests/support/body_identity.rs:18), `original authored fixture body` (var_escape_residual2.rs:143, compiler_residual.rs:1153), `CompilationAdmission(NativePreflightRequired)` (tcl-vm exec.rs:1792 and many e2e tests), `StringUpdater` checked child storage (tcl-vm value.rs:7237), `a completed original command world is unavailable` / `without original coverage` for rename (lsp-server e2e), `DSL phase requires its complete ...` (bpf-tcl).

## Jobs that did not complete or did not run tests

| Job | Conclusion | Failure / first error |
|---|---|---|
| rust-tests-shard (1/5) | cancelled mid-run (job conclusion failure) | Runner received a shutdown signal at 18:15:42Z (11 min into nextest); step "cargo nextest run" cancelled; terminal-capture upload skipped. |
| rust-tests-shard (2/5) | cancelled | Nextest never finished; job cancelled at 23:56:55Z after 5h34m (runner "operation was canceled"). terminal.jsonl is 0 bytes. |
| rust-tests-shard (3/5) | failure | Completed, nextest exit 100 (622 tests failed of 3,919 recorded). |
| rust-tests-shard (4/5) | failure (no failed step) | Step "cargo nextest run" has no recorded conclusion; log ends 18:57:19Z; job marked failure at 19:42:04Z. Runner/job lost. |
| rust-tests-shard (5/5) | failure | Completed after 5h18m, nextest exit 100 (601 failed of 5,244). |
| rust-tests (aggregate) | failure | Require all Rust test shards (shards 1,2,3,4,5 not all green). |
| rust-tests-heavy | failure | Completed (4,320s for 356 tests), nextest exit 100: 327 pass, 29 fail. |
| lsp-e2e-partition (1/3) | cancelled | Hung test(s); cancelled at 00:10:06Z after ~6h. Last terminal record 19:15Z. |
| lsp-e2e-partition (2/3) | cancelled | Hung test `a_late_resolve_cannot_stamp_the_slot_backwards`; cancelled 00:09Z. Last terminal record 20:15Z. |
| lsp-e2e-partition (3/3) | failure | Completed, 602 pass, 239 fail, 5 ignored (nextest exit 100). |
| lsp-e2e (aggregate) | failure | Require all lsp-e2e partitions. |
| runtime-rust-tests | failure | `make runtime-rust-test`: 2,404 tests in tcl_runtime, 2,279 passed, 125 failed (Error 101). The preceding no-tommath-capability step passed (18 ok). Plain libtest output, not in the 5203 receipt. |
| spectcl-compat | failure | `make test-spectcl-compat`: eval_loader 28, golden_packs 3, pack_is_real_tcl 1, pack_source_e2e 10 all ok; spec_corpus 4 ok, 2 FAILED (`shipped_string_pack_replays_actual_corpus_first_gates`, `every_shipped_tclspec_loads_installs_and_analyses_against_corpus`), 600s. Not in the 5203 receipt. |
| rust-tests-doctest | success | 71 doctests ok, 2 ignored. Not in the 5203 receipt. |
| wasm-real-link | failure | `cargo test -p tcl-compiler --test wasm_real_link`: 24 ok, 1 FAILED (`emitted_modules_run_against_the_real_runtime`, wasm_real_link.rs:796). |
| rust-check | failure | `make check-rust-pr`: cargo fmt drift (diffs in rust/tcl-spectcl/tests/jim_document.rs and others) -> exit 2. No tests. |
| lsp-server-wasi | failure | WASI e2e: FAIL go-to-definition follows `source` into a file on the preopened disk; harness aborted on textDocument/rename "conditional source advice for file:///w/app.tcl is unavailable". |
| lsp-server-wasm | failure | Browser e2e: FAIL workspace/symbol and tclIndex go-to-definition; harness aborted on rename "incomplete-original-command-coverage". |
| python | failure | `make lint-py`: ruff format --check, 2 files would be reformatted. |
| cargo-deny | failure | vulnerability advisory (e.g. "Exponential event growth with ENABLE_WIKILINKS"); same baseline advisory failure as 5203. |
| test-ext-partition (3, three) | failure | Runner received shutdown signal 18:25:52Z (exit 143). Partitions 1 and 2 and multi-folder not part of the Rust comparison; multi-folder: "4 test(s) failed". |
| pr-gate / test-ext (aggregates) | failure | Propagated prerequisite failures. |

Skipped (not run): macos-wasm-check, native platform matrix, create-release, build-claude-skills, linux-release-portability, publish-*, build-zed/sublime/jetbrains/vsix, cleanup-*.

## Method notes

- Job logs for the nextest jobs carry `NEXTEST_CAPTURE_V1` records (base64 JSON: run-start, selected, terminal, run-end) written by scripts/dev/nextest-terminal-capture.py; the console reporter output is not in the job log. Records were accepted only if the nonce matched run-start and seq was contiguous (all 9 jobs: yes, 0 bad records). binary_id is nextest RustBinaryId as in the baseline (`tcl-compiler` lib, `tcl-compiler::taint` integration).
- Terminal status mapping: ok -> PASS, failed -> FAIL, ignored -> IGNORED. No retries occurred (retry_count null everywhere).
- rust-tests-shard (2/5) outcomes come from the nextest console lines (`PASS|FAIL [t] (n/N) binary selector`) in the uploaded nextest-stderr artifact, i.e. the same method as the baseline; only ordinals 1..1194 of 7,954 exist (2 MiB cap). Console and log agree on every overlapping test elsewhere.
- Shards 1 and 4 have only the terminal records that reached the job log before the runner died (1,464 and 2,080).
- Downloads and scripts: /tmp/claude-0/-home-user-tcl-lsp/ecf365c5-6571-597f-97b7-b7fbb7d1acfb/scratchpad/ci5248/ (raw logs in raw/, artifacts in art/); scripts in /tmp/claude-0/-home-user-tcl-lsp/ecf365c5-6571-597f-97b7-b7fbb7d1acfb/scratchpad/ci5248-scripts/ (run with python3 -I).
