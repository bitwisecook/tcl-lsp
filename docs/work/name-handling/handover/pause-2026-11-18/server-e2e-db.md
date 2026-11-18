# server-e2e-db handover, pause 2026-11-18

Session `server-e2e-db` (id session_017u5g3KHQpTL4RNf7ZN9wiQ), branch `work`, area: `rust/tcl-lsp-server` (e2e non-navigation and lib), all of `rust/tcl-lsp-db`.

## (a) Landed today (all on `origin/work`)
- Declaration-side rehomed identity fix: `release_workspace_symbols` names the index's procedures whose `name_span` covers the cursor (3 lib tests fixed).
- Open-pack-buffer notices (`PackNoticePublication` files/trust, `buffer_pack_notices`, `check_buffer`).
- Source-view publication: a second sibling `source` of the same file is not recursion. `OriginalSourceViewDraft::call_chain_executes` (view_drafts.rs) replaces the ancestor-uri check in `publication.rs`; fixes `multi_seeded_declaration_unions_every_runtime_identity_945` references half. Commit message starts "Source views: a second sibling source".
- Expectation/fixture repairs: `rehoming_gate_dedups_concurrent_reconciliation_passes` (solo-pass generation delta), `rehoming_never_waits_for_live_publication_that_needs_its_gate_1854` (60 s bound, named ConditionalAdvice refusal), fix_all layering uses W100, `ws_index` test helper installs the source resolver (`new_workspace_index`), `a_constants_only_edit_reschedules_both_children` precondition compares authored rows, recovery fixture role `ParamList`, db `cross_file_call_sites` x3 and e2e I230 x7 seed flips, flaky repairs (reload, issue954, mathfunc).
- clippy 1.100 `assert_is_empty` lints fixed in three server test files.
- e2e `open_ready_lang_timeout` returns the latest version-1 publish after the settle barrier (measured: 9/9 re-run failures identical with and without; it fixes nothing by itself).
- **Temp-root fix `740ea60a0`**: `ScratchDir` guards (e2e common, server lib `unique_scratch_dir`, preview_tickets_e2e, one db test), shared ref-counted harness root, tests for drop/panic/file.
- Ledger: `docs/work/test-scrub/ledger/server-e2e-db.tsv` (status vocabulary checked by `ledger_status.py --check`), `-routed`, `-clusters`, `-flaky`, `-passing.tsv`; failing list `docs/work/name-handling/lists/server-e2e-db-failing-2026-11-18.txt`.

## (b) In flight
Nothing is half done in code. Not finished: tracing about 12 untraced original-world-seed rows (proposal_endpoint_tests, original_document_metadata_tests x4, closed_files, pull, code_actions, closed_scan_input_custody autoload recapture), the `policy_truth_table::every_row_offers_fixes_for_shown_findings_only` currency refusal, the `.irulex` variant of `claimed_extensions`, and the 5 untriaged e2e rows added as UNTRIAGED.

## (c) Queue, in order
1. `policy_truth_table` (3 rows): the code-action response transaction returns CurrencyRefused on every retry for an opened, indexed document; `ReadonlySourceDocument.matches_current_snapshot` has `database_revision` and `external_call_sites` false while capture reads both from the db. The writer between capture and commit is not identified. Fixture needs `did_open` plus a converged cross-file-evidence sync (`sync_cross_file_evidence` loop until `changed` empty) first. Reason first: it blocks every code-action fixture that opens through the harness.
2. Remaining original-world-seed rows (above), each to its first wrong value.
3. `.irulex` `claimed_extensions` variant (tooling pushed 0b7c633f for the dialect-less claim; the f5-irules closed-file definition is still null).
4. db integration failures (below) to first wrong values; route the slow ones to analysis-performance.
5. FIX-FIXTURE / FIX-EXPECTATION rows by cause, then MERGE deletions whose survivors pass (one is unblocked: `diagnostic_matrix::s102_fires_for_numeric_shimmer_masked_by_int_entry`; `union_lattice_three_way_merge_reports_every_type` needs a new named survivor).
6. Passing-test second pass beyond `server-e2e-db-passing.tsv` (only `hover::proc_no_comment_bleed` VACUOUS recorded; the cluster groups are in `irules-registry-tooling-passing.tsv`).

## (d) Waiting on / waited on
- analyser-binding: library merge (`source_logical_command_candidates_at` None, position 6), path-constant namespace keys + `file join $dir` folding (position 10), `package require $computed` not dynamic (position 11).
- taint area: `w201-detector-silent` (W201 rows); diagnostics checking W216 rows.
- lsp-features: rename Binding purpose abstains in `release_workspace_symbols` (multi_seeded rename half, rename::* rows); `$`-completion core witnesses; claimed_extensions.
- tooling: UPF body resolver; fix of the leaked `/tmp/tcl-spectcl-cache-test-*` source.
- others wait on me: nothing outstanding.

## (e) Coordinator rulings not yet carried out
- Measure with the three REQUIRE flags (`source /tmp/claude-0/work/oracle.env`, i.e. `bash scripts/dev/ensure-jim-oracle.sh` prints the jimsh path).
- Push the failing list under `docs/work/name-handling/lists/` (done for 2026-11-18 except the db it_5 per-test measurement).
- Report counts only against a clean `origin/work` baseline; `cargo fmt` and clippy finish before a push.
- Coordinator asked each slow `tcl-lsp-db` integration test to be run with a 10-minute timeout and reported; done only partly (see f).

## (f) Last measured counts (commit a375d8656, flags set; `CARGO_INCREMENTAL=0`)
| target | command | run | failed |
|---|---|---|---|
| tcl-lsp-server lib | `cargo test -p tcl-lsp-server --lib` | 1006 | 92 |
| tcl-lsp-server e2e | `cargo test -p tcl-lsp-server --test e2e` | 1659 (+5 ignored) | 246 |
| tcl-lsp-db lib | `cargo test -p tcl-lsp-db --lib` | 206 | 55 |
| db class_factory_fixpoint | `--test class_factory_fixpoint` | 13 | 7 |
| db compiler_check_corpus | `--test compiler_check_corpus` | 11 (+2 ignored) | 2 |
| db interned_gc | `--test interned_gc` | 3 | 2 |
| db file_analysis_corpus | `--test file_analysis_corpus` | 2 (+1 ignored) | 0 |
| db file_decls_corpus | `--test file_decls_corpus` | 0 (1 ignored) | 0 |
| db it_2 | `--test it_2` | 10 | 0 |
| db it_5 | `--test it_5` | 11 | 3 (finished in 507 s) |
| db project_diagnostics_corpus | `--test project_diagnostics_corpus` | 0 (1 ignored, slow sweep, run with `--ignored`) | 0 |

Slow tests (in-process timings, opt-level 0): db lib `compiler_check_incremental_matches_fresh_under_edits` >60 s (lib 376-392 s total); compiler_check_corpus `compiler_check_memo_matches_uncached_graphops` >60 s (151 s binary); interned_gc `interned_slots_stay_bounded_across_an_edit_session` and `raising_input_durability_disables_the_collector` each >60 s (binary 948 s, both fail); it_5 `memory_growth::edit_session_memory_growth_plateaus` and `more_documents_than_the_memo_caps_reanalysed_under_alternating_configuration_plateau_2543` >60 s (binary 507 s). My earlier "ran for hours" came from a `--tests` invocation that also contained a stale cargo lock wait, not a non-terminating test: every binary finished. The per-test 600 s run of it_5 was killed at the pause after 7 of 11 (times: early_cutoff 0 s, memory_alias_graph x4 2-5 s one failing, memory_growth `..alternating_configuration_interns_nothing_new_2543` 9 s and 12 s); the other four it_5 memory_growth tests were not timed individually. None of these slow tests has been routed to analysis-performance yet.
db integration failures: compiler_check_memo_matches_uncached_for_irules_flow_checks, ..._for_hidden_writes; interned_gc x2; it_5 `actual_document_graphs_keep_native_jim_and_hosted_source_cards`, `many_revisions_between_configuration_flips_keep_the_lowered_module_2543`, `edit_session_memory_growth_plateaus` (all added to the ledger and the list as UNTRIAGED).

## (g) What a successor would get wrong
- Disk: `target/` grows past 60 GB; delete only old hash dirs under `target/debug/build/<crate>/` while nothing builds (a build in flight loses its fingerprint file otherwise), and `rm -rf /tmp/tcl-spectcl-cache-test-*`.
- A build with `CARGO_INCREMENTAL=0`; the e2e server binary is built at opt-level 0, so timings are about 10x analysis-performance's opt-level 1 numbers.
- Tests that "wait for the first publish" are the common flaw; wait for the converged state (`current_complete_diagnostics`, `sync_cross_file_evidence` fixpoint) not the first event.
- The compiler W201 detector is silent on the pushed tree (`run_all_checks_reports_w201_path_concat` fails): the W201 e2e rows are not a server filter, do not chase the server. A variant of `open_ready_lang_timeout` returning `current_complete_diagnostics` was tried and gave the same result; it is not saved.
- Code actions are offered only for a `did_open`ed, versioned, indexed document; directly inserting a `DocumentState` gives no actions (`capture_code_action_source` returns None for version None).
- A dialect-less pack `file_extension` was dropped by tcl-spectcl (fixed in 0b7c633f); the `.irulex`/f5-irules variant still returns null and may be a plain dialect barrier.
- `ledger_status.py --check` currently flags `values.tsv:220` (another session's `routed (analyser-binding)` status); not mine.
- Ledger rows say `routed` with a named reader; the first wrong value, not a hypothesis, is what the ledger notes record.
