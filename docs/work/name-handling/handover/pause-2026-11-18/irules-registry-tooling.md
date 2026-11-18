# irules-registry-tooling: handover at the 18 November 2026 pause

Area slug `irules-registry-tooling`. Everything below is on `origin/work`; nothing is half-done in the tree. Written 18 November, last push `366cda3b` plus this commit.

## (a) Landed today

| Commit | What |
|---|---|
| `b7e03fa6`, `7660593b`, `cef0f6cd` | `rule_declaration` parts 1 and 2: `unproven_head_selection`, `walk_unknown_compiler_handler`, `SourceRuleDeclarationCandidate.procedure_name`, `retain_rule_loader_events` through `normalise_qualified_name` |
| `44f551ca` | `tcl_spectcl::pack::check_buffer` (buffer notices) |
| `7a0f2a3a` | red witness `the_analyser_enters_the_trailing_body_the_resolver_names` |
| `32ee6901` | pinning test `a_negative_return_level_completes_with_an_error` |
| `8f78abe3` | `subst_switches` reads `subst_::OPTIONS` |
| `00d66ac3` | iRules hosted-advice `lassign` row (BIG-IP check 32) |
| `a8361896` | explorer wasm names (`wasm_to_explorer_json_named`) |
| `d7fe6d5a` | spec_verbs and pkg_build find tclsh through `witness_tclsh` |
| `222c579c`, `11fc13d2` | `bigip-name-checks.md` start-here, Index A (74) and B (32), overlaps, checks 18 and 32 |
| `95a1567c`, `12272ed0`, `e6d54960` | unowned crates: ledger `unowned-passing.tsv` (60 rows), the 30-group pass, parser pair diffs |
| `3599d212` | a dialect-less pack `file_extension` claim publishes as a plain source extension (server-e2e-db finding) |
| `0393f348` | a Jim registration with no compiler descriptor declares its hook absent (execution's `namespace` request) |
| `e7e8246a`, `3008acde` | ledger: duplicate diagram row dropped, BIGIP-NAME-002 count, statuses after the 08:00 re-measure |
| `366cda3b` | `tcl-spectcl` unit tests no longer leave `tcl-spectcl-cache-test-<pid>` behind |

The coordinator signed off (not yet applied, needs the owner): the 11 parser MERGE rows, 4 weak-assertion REWRITE rows, the 5 unowned MERGE rows and the VACUOUS row, all in `ledger/unowned-passing.tsv`.

## (b) In flight

1. **`tcl-spectcl` test temp-directory guards.** Patch: `patches/irules-registry-tooling-spectcl-test-scratch-guards.patch` (578 lines, 12 integration sites in `tests/`, 8 unit-test sites in `src/`, new `tests/support/scratch.rs` and `src/test_scratch.rs`). **Never compiled.** Apply with `git apply` from the repository root, then `cargo nextest run -p tcl-spectcl` (every binary: `it_2`, `it_3`, `it_5`, `pack_source_e2e`), fix the by-value `PathBuf` uses the compiler reports (the guard derefs to `PathBuf`), run `make xtask-gates` and `cargo xtask file-sizes --check`, and push. It leaves 3 sites unguarded on purpose: `src/golden.rs` (creates nothing) and the two `variable_reference_boundaries` sites share one helper shape. Also unguarded: `tests/` of other crates; the coordinator asked server-e2e-db for the `tcl-lsp-e2e-*` roots.
2. **Unmeasured-crates sweep**: table pushed as `lists/unmeasured-crates-2026-11-18.md` (21 of 24 crates measured, 3 not run for disk space, 1 does not build). The script is `patches/unmeasured_sweep.py.txt`.
3. **Ledger truth pass**: started for `analyser-binding.tsv`, stopped before any result. The script is `patches/ledger_truth_pass.py.txt` (modes `run` and `apply`; it adds a `note` column where missing; treat an empty status as open). Nothing in any other session's ledger was touched.

## (c) Queue, in order

1. Apply and finish the `tcl-spectcl` scratch-directory guards (b1).
2. Rerun the sweep for `tcl-spec-studio-wasm`, `tcl-vm-wasm`, `bigip-query-wasm` (clear `rust/*/target` first); send the coordinator the table.
3. Ledger truth pass: `analyser-binding.tsv`, `class-facts.tsv`, `tk-oo.tsv`, in that order, per the coordinator's rules (batch per test binary, `fixed` plus note `passes on work <sha>, 18 Nov (truth pass)`, `not found on work <sha>` for vanished names, one commit per ledger, ledger file only, run `tools/ledger_status.py ledger --check` before each push, fetch and rebase immediately before pushing, tell each owner in one line, send the coordinator rows run / fixed / still failing / not found).
4. Re-run my blocked rows as the named blockers land (section d).

## (d) Waiting on

**The owner** (nothing edited; every item changes a test body or deletes or merges a test):
- FIX-EXPECTATION (5):
  - `tcl-spectcl` `spectcl_dialect_analysis::a_hover_key_written_outside_the_block_is_still_unknown`
  - `tcl-spectcl` `a_derived_body_runs_under_the_release_the_call_is_analysed_under`
  - `tcl-lsp-server` `spec_packs::pack_documents::an_open_pack_keeps_its_notices_while_it_is_edited`
  - `tcl-compiler` `command_binding::rule_declaration::tests::direct_loader_registration_preserves_the_later_original_procedure_handler`
  - `tcl-compiler` `command_binding::rule_declaration::tests::frozen_priority_retains_only_the_rule_loader_file_policy_value`
- FIX-FIXTURE (diagram fixtures read unset variables or sit after a certain exit; the Jim leg of `spec_test_presence...` uses a pkgIndex layout this Jim cannot load; `JIMLIB` plus a flat `demo.tcl` loads):
  - `tcl-diagram` `data::tests::dynamic_return_forms_keep_their_distinct_possible_completions`
  - `tcl-diagram` `data::tests::exact_custom_return_codes_keep_their_integer_payload`
  - `tcl-diagram` `data::tests::exit_statuses_project_release_specific_errors_and_typed_dynamic_outcomes`
  - `tcl-diagram` `data::tests::literal_return_options_and_exit_keep_distinct_try_outcomes`
  - `tcl-diagram` `data::tests::loop_nodes_carry_their_compiler_exit_reason`
  - `tcl-diagram` `data::tests::numeric_return_codes_project_as_their_canonical_completions`
  - `tcl-diagram` `data::tests::return_completion_preserves_source_word_shape`
  - `tcl-diagram` `data::tests::trap_patterns_are_projected_as_tcl_list_elements`
  - `tcl-cli` `spec_verbs::test_against_package::spec_test_presence_keeps_literal_colons_and_wildcards`
- MERGE (14 cli rows, all survivors still failing):
  - `tcl-cli` `decided_branches::diag_reads_the_decided_loop_header_and_the_dead_respond`
  - `tcl-cli` `decided_branches::explore_sccp_prints_the_selection`
  - `tcl-cli` `decided_branches::the_eleven_loop_witnesses`
  - `tcl-cli` `decided_branches::the_nested_equality_decides_through_diag_and_opt`
  - `tcl-cli` `ordered_state::explore_sccp_prints_what_a_catch_completed_with`
  - `tcl-cli` `ordered_state::explore_sccp_prints_what_a_flattened_catch_returned`
  - `tcl-cli` `ordered_state::explore_sccp_prints_x_exact_on_the_error_path`
  - `tcl-cli` `ordered_state::opt_forwards_what_a_nested_catch_completed_with`
  - `tcl-cli` `procedure_summaries::explore_interproc_prints_the_global_a_name_argument_writes`
  - `tcl-cli` `procedure_summaries::o103_summary_path_folds_a_computed_return`
  - `tcl-cli` `procedure_summaries::the_seven_summary_witnesses`
  - `tcl-cli` `transfer_routes::explore_sccp_folds_the_irules_cores_and_the_list_cell_updates`
  - `tcl-cli` `transfer_routes::explore_sccp_prints_the_route_of_each_statement`
  - `tcl-cli` `transfer_routes::explore_sccp_prints_what_each_completion_path_stores`
- the 6 MERGE and 7 REWRITE rows of `ledger/irules-registry-tooling-passing.tsv` and, from `ledger/unowned-passing.tsv`:
  - `tcl-lexer` `lexer::tests::expand_prefix_before_braced` (MERGE)
  - `tcl-lexer` `expr_string_is_opaque` (MERGE)
  - `tcl-lexer` `lexer_depth::line_index_line_at_is_encoding_independent` (MERGE)
  - `tcl-syntax` `expr_parser::command_substitution` (MERGE)
  - `tcl-lexer` `parse_lexer::t14_braces_suppress_substitution` (MERGE)
  - `tcl-lexer` `word_parts::tests::oracle_sheet_is_recorded` (VACUOUS)
  - `tcl-syntax` `jim_matches_its_independent_body_and_completion_contracts` (REWRITE)
  - `tcl-syntax` `jim_matches_shared_execution_vectors_with_measured_capabilities` (REWRITE)
  - `tcl-syntax` `jim_namespace_objects_and_text_helpers_match_native_operations` (REWRITE)
  - `tcl-syntax` `jim_publication_preserves_actual_flat_keys_and_procedure_namespaces` (REWRITE)
  - `tcl-syntax` `jim_lookup_searches_its_counted_flat_table_in_native_order` (REWRITE)
  - `tcl-syntax` `jim_global_alias_local_names_use_original_counted_operands` (REWRITE)
  - `tcl-syntax` `jim_scalar_and_combined_dictionary_names_match_actual_cells` (REWRITE)
  - `tcl-syntax` `expr::parser::tests::add` (MERGE)
  - `tcl-syntax` `expr::parser::tests::sub` (MERGE)
  - `tcl-syntax` `expr::parser::tests::mul` (MERGE)
  - `tcl-syntax` `expr::parser::tests::div` (MERGE)
  - `tcl-syntax` `expr::parser::tests::modulo` (MERGE)
  - `tcl-syntax` `expr::parser::tests::pow` (MERGE)
  - `tcl-syntax` `expr::parser::tests::equality` (MERGE)
  - `tcl-syntax` `expr::parser::tests::str_eq` (MERGE)
  - `tcl-syntax` `expr::parser::tests::logical_and` (MERGE)
  - `tcl-syntax` `expr::parser::tests::logical_or` (MERGE)
  - `tcl-syntax` `expr::parser::tests::in_operator` (MERGE)
  - `tcl-lexer` `parse_lexer::{t1_1_null_byte_in_word, t1_5_backslash_newline_leading_space, t3_3_semicolon_separator_words} + 17 other `texts` tests` (REWRITE)
  - `tcl-lexer` `span::tests::span_is_copy, tokens::tests::token_type_equality, tokens::tests::source_position_default_is_origin, parse_lexer bare-`$` tests` (REWRITE)
  - `tcl-lexer` `parse_expr::{t10_shift, t13_mul_div_mod, t2_9, t4_logical_and, t6_bitwise_xor} + 7 other `ops` tests` (REWRITE)
  - `tcl-lexer` `brace_backslash_does_not_close, brace_comment_skips, brace_empty, brace_mid_word_is_literal, brace_nested_balanced + 3` (REWRITE)

**Other sessions**: analyser-binding (UPF body barrier, arity, `native_normal_store`, `project_spec_packs`, procedures after `rename`), class-facts (`try` lowering with an invalid return), taint (`f5-xc` `switch` lowering, server quick-fix rows), execution (the 25 F5 string and name-protocol refusals, `namespace` and `array` selection, a coordinator ruling on sealing F5 script values with the host protocol), server-e2e-db (reload ordering; the `.irulex` / `f5-irules` variant of `a_closed_file_under_a_pack_extension_is_indexed` is not traced).

**Waiting on me**: execution re-runs its three `tcl-codegen` namespace rows after `0393f348` and may report back; the coordinator wanted the sweep table (this commit has it).

## (e) Rulings not yet carried out

- Coordinator: the `tcl-spectcl` integration temp guards (b1) and the sweep reruns (c2), the truth pass (c3). The `unowned-passing` "pass is closed", so no more reading there.

## (f) Last measured counts

Taken at `origin/work` after the 07:36 fix, 18 November 08:00 UTC, with
`TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_TEST_INCREMENTAL=false cargo nextest run -p <crate> --no-fail-fast`:
tcl-cli 37/281, tcl-mcp 1/146, tcl-diagram 13/36, tcl-explorer 11/142, tcl-spectcl 22/625, tcl-irules 1/104, tcl-irule-test 27/61, f5-xc 10/57, bigip-report-gen-rust 2/124, f5-cli 2/185; tcl-spec-studio 399, tcl-registry 2466 (after `0393f348`), tcl-sslictcl 89, tcl-bigip 362, tcl-bigip-io 28, tcl-bigip-query 179, tcl-f5mku 6 all pass. Server e2e groups `irules`, `spec_packs`, `jim`: 24 of 131 red (`-p tcl-lsp-server --test e2e -E 'test(/^(irules|spec_packs|sslictcl|jim)::/)'`). The failing list is `lists/irules-registry-tooling-failing-2026-11-18-0800.txt`. The registry change `0393f348` and `3599d212` were not followed by a full re-sweep of these crates.

## (g) What a successor would get wrong

- The disk fills: `target/debug` reaches 24 GB and each excluded wasm crate builds its own `rust/<crate>/target`. I deleted `target/debug` twice. Check `df -h /` before a sweep and delete `rust/*/target` after.
- Never `pgrep -f cargo` inside `bash -c` (it matches itself). Long runs: background plus a DONE marker, never `sleep` chains.
- The auto-mode classifier refuses test-body edits, deletions and rewrites; do not route around it. A helper-call edit with unchanged assertions and a single-assertion flip authorised by a ruling were accepted.
- `ledger_status.py --check` fails on other sessions' `values.tsv` words (`routed (owner)`); filter to your own ledger. An empty status counts as open.
- The installed `tcl-lsp` MCP binary is stale (built 17 November); use `target/debug/tcl explore --source ... --dialect ... --show ir --text --no-colour`.
- `make xtask-gates` is part of any push touching the registry, compiler, codegen, syntax or lexer. `file-sizes --check` fails on a new file over 2000 lines (I split a test into its own file once).
- Push loop: `git fetch --depth=200 origin work && git rebase origin/work && git push -u origin work`; a shallow fetch without `--depth=200` makes the remote look force-updated.
- The ledger `irules-registry-tooling.tsv` had 16 duplicate rows (two passes over the same test); I kept the later row. The 24 server rows are only as true as the 08:00 server run.
