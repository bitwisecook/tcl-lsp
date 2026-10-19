# Luna-30: slow-test review, target `tcl_lsp_server` (54 tests)

This review is read-only. Nothing was run: no cargo, no test binaries. Evidence comes from capture47 per-test logs, the sol-26 head and patched runs, the sol-26 Server108 fixture and timing runs, and the sol-28 gdb samples of Server108. The per-test rows are in `REVIEW.json` (same directory). Everything below is a **proposal**. The project rule holds: original assertions, selectors, budgets and fixtures change only with user sign-off. Every original assertion stays reachable in the deep tier.

## Totals

| | seconds |
|---|---|
| Before (sum of the input rows; 30 rows are 180 s TIMEOUTs, so this is a lower bound) | **6,374** |
| After the recommended fixes (static estimate per row, see `est_new_s`) | **~420** |

Almost all of the saving comes from two systemic causes (S1, S2). Neither needs an assertion change. One is a fixture fix and the other is a product fix.

## Systemic causes (ranked by seconds)

### S1: the test backend discovers and surveys the *host* Tcl installation on every workspace scan (~4,860 s of input time, 27 tests)

- **Mechanism.** `test_backend_over` (`rust/tcl-lsp-server/src/lib.rs:46602`) leaves `discovered_tcl` as an empty `OnceLock` (`lib.rs:46637`). Every `scan_workspace_folders` (`lib.rs:28710`), even one with no roots, runs `core_tcl_install::discover(default_search_bases())` (`lib.rs:28949-28953`; `rust/tcl-lsp-core/src/tcl_install.rs:59,117`).
  - On this host, discovery finds `/usr/share/tcltk/tcl8.6` and `/usr/lib/blt2.5`. Their `auto_path` therefore includes `/usr/lib`, `/usr/share/tcltk` and `/usr/share/tcltk/tcl8.6`.
  - `build_package_resolver` (`lib.rs:37693`) then reaches `scan_single_dir` (`package_resolver.rs:907`) for each of those directories.
  - `scan_single_dir` surveys every `pkgIndex.tcl` and `tclIndex` it finds once plainly, then **once per `TclVersion::ALL` (5)**. Each survey is a whole-document `source_structure::analyse_document` (`package_resolver/original_names.rs:185,484,634,809`). Among the files surveyed is `tk8.6/tclIndex`, which has 253 lines.
- **Evidence.**
  - All 20 tests that call `scan_workspace_folders()` time out at 180 s in both the sol-26 head and patched runs. Server040 also times out in capture47.
  - The workspace fixtures are 1-6 tiny files.
  - Sibling tests that reach the same index and reindex code without a scan finish in 0.3-2.6 s. For example, `closed_disk_index_keeps_workspace_roles_and_withdraws_changed_pack` takes 0.34 s against Server040's 180+ s.
  - Surveying a single one-line `pkgIndex.tcl` already costs about 3 s in debug (`workspace_scan_resolver_publication_requires_current_inputs`: 2 × `scan_path` = 6.4 s).
  - Another 7 tests reach the scan indirectly:
    - `apply_global_config` with `packages.provides` sets `rescan_workspace` (`lib.rs:24809-24829` → `24763`).
    - `initialized` scans at `lib.rs:30184`.
    - `did_change_workspace_folders` scans at `lib.rs:30852`.
- **Fixture fix (no test edits).** `test_backend_over` presets `discovered_tcl` to `OnceLock::from(Vec::new())`, and the user-config read in `effective_auto_path` (`lib.rs:37887`) gets a test override. Discovery stays covered hermetically by the existing explicit-`TclInstallation` tests (`build_package_resolver_picks_up_a_discovered_installation_package`, `effective_auto_path_layers_editor_project_and_discovery`). This also makes the suite **independent of the machine**: today these tests are fast or slow depending on whether `/usr/share/tcltk` exists.
- **Product cost (user-facing).** Every rescan re-surveys the unchanged host library indexes 6 times. Rescans happen at startup, on folder add, on `libraryPaths`/`packages.provides`/`maxFiles` changes, and when a watched `pkgIndex` changes. Proposal: cache per-directory index surveys across scans, keyed by path+len+mtime or content digest. The core review should also look at the 5× per-release survey; I call that S6.
- **Verification probe, no code change** (for when the tree is free): run one scan test under a mount namespace that hides the host install:
  ```
  unshare -m sh -c 'mount -t tmpfs none /usr/share/tcltk; mount -t tmpfs none /usr/lib/blt2.5; <tcl_lsp_server test bin> tests::scan_workspace_folders_indexes_unopened_files --exact'
  ```
  The prediction is that it drops from 180+ s to seconds.

### S2: every `reload_spec_packs` re-parses the 2 MB bundled EDA spec set (~820 s of input time, 14 tests; initialized is shared with S1)

- **Mechanism.** `reload_spec_packs` (`lib.rs:27103`) runs `load_discovered_configuration_with_studio_sources_and_inputs_in` (`rust/tcl-spectcl/src/bundled.rs:292-377`). That function appends `embedded_sources()` (or reads `specs/`, 8 files, 2.0 MB) and calls `pack::load_sources` on every call. Only the bundled-only `packs()` is cached (`bundled.rs:405`).
- **Evidence.** Measured time divided by the number of reloads is a near-constant **18-19 s per reload** across all 9 `studio_source` tests and the 4 config-reload tests:
  - `the_last_generation_of_a_burst`: 4 reloads, 72 s.
  - `a_configuration_burst`: 1 reload, 21 s.
  - `cross_backend_same_uri...`: 3 reloads, 57 s.
  - Every accepted studio acquire, publish or retire triggers a reload (`studio_source.rs:505`).
  - `did_change_configuration_applies_inline_settings` **fails** at its own 10 s "should not hang" backstop (`Elapsed`), because one reload takes longer than 10 s. That failure is a performance symptom.
- **Product fix.** Memoise parsed bundled packs inside the configured loader, keyed by (tier, origin, path, content digest, trust). Every `didChangeConfiguration`, trust push and studio save pays this re-parse in release builds too. A fallback, if the product change is declined, is a `cfg(test)` knob that excludes the bundled tier for the studio tests. That changes fixture setup only.
- **Probe.** Sample `tests::the_last_generation_of_a_burst_still_runs_to_completion` with sol-28's `gdbsampler.py` to confirm that `pack::load_sources` dominates.

### S4: read-only request paths run a full fresh analysis per URI per request (Server108 and 6 smaller tests)

- `workspace/symbol` (`lib.rs:32626`) calls `capture_workspace_symbol_analysis` once per distinct hit URI (`lib.rs:32706`).
- The call hierarchy and lens paths call `capture_readonly_reference_document` (`readonly_reference_reports.rs:433-461`).
- Both paths end in `driver.analyse(...)` or `fresh_analysis_seed` (`lib.rs:13659`).
- So the Ctrl+T picker costs one full analysis per matching file per keystroke. That is exactly the cost the handler comment (`lib.rs:32641-32648`) says the design avoids.
- sol-28 samples of Server108: 297 of 330 samples are in `Backend::fresh_analysis_seed`, and 23 of 330 are in `WorkspaceIndex::add_document`.

### Amplifiers (no direct action proposed)

- **S5: debug analysis is superlinear.** sol-26 `fixture-synthetic` timings: 40, 80 and 160 body units take 6, 15 and 38 s. This makes S4 and `concurrent_snapshots` (300 procs) expensive. It is owned by the tcl_compiler review; the sampled hot frames are `emit_cfg_ssa_diagnostics`, `emit_w216_brace_then_paren → original_written_word_at_span_in_source`, and `source_registry_words_with_definition_parent`.
- **S3: tokio `Runtime` drop waits indefinitely for spawned blocking tasks.** In-flight pack loads and warm tasks therefore extend wall time after the last assertion. Example: `did_change_configuration...` panics at 10 s but exits at 18 s.
- **Not a cause: `test_backend()` construction.** Many `test_backend` tests run in 0.03 s.
- **Not a cause: `rt::timeout` or sleeps on success paths.** No slow test is waiting out a fixed timeout on its success path. The 5, 10 and 60 s timeouts are backstops. The only fixed waits are `CONFIG_RELOAD_DEBOUNCE` (150 ms), and they are part of the property under test. The client requests fail fast because the test `Client` is uninitialised (`tower-lsp-server` `send_request` → `not_initialized_error`).

## Ranked list (saving, then action)

1. **S1 hermetic `discovered_tcl` in `test_backend_over`.** Fixture fix, about 4,700 s across 27 tests, all of which currently TIMEOUT. A small add-on is a scan-survey cache in product.
2. **S2 bundled-pack parse memo.** Product fix, about 800 s across 14 tests. It turns `did_change_configuration_applies_inline_settings` green and makes `initialized_reschedules...` cheap.
3. **Server108 `workspace_symbol_caps_its_answer`.** Product S4 fix plus a SPLIT: a light limit-seam test (~1 s), a counting budget (analyses per request), and the original kept in deep (185 s → ~20 s after the product fix).
4. **`concurrent_snapshots_share_one_text_and_index_allocation`.** Fixture: insert the 300-line document without `register()`'s analysis. The property is pointer identity, so 180+ s → ~1 s.
5. **`default_dialect_change_retries_after_concurrent_edit_1854`.** This is a **hang**, not slowness. `did_change` is awaited unbounded while the test holds the `default_dialect` guard (`lib.rs:61935` vs `61947`). Bound it with the same 5 s `rt::timeout` (proposal) and root-cause the lock order.
6. **S4 product fix** (shared with item 3). It roughly halves the readonly call, lens and incoming tests (11-18 s each).
7. **KEEP AS IS:**
   - `policy_truth_table` (27 s): table rows; currently failing.
   - The two O100 pull/deep tests (16 s each): failing, cost unattributed; sample them once they pass.
   - `library_publication...` (15 s), `pull_path_w120` (15 s), `no_code_lens` (11 s) and `malformed_recovery` (11 s). The last two are failing.

## Tiers: where each version lives and how it is selected

The tier contract is `docs/design/contracts/test-tiers-and-ci-gates.md`. The `tcl-lsp-server` lib unit tests run in the deep tier through the `lsp-e2e` nextest archive, default profile, hash partitions 1/3-3/3.

- **Deep (CI and pre-PR):** all 54 originals stay as they are, with assertion text unchanged. The new light and counting tests also go here as ordinary `#[tokio::test]`s, under 3 s each. With S1 and S2 in place, every original except Server108 (before its product fix) is in the 1-27 s range, so **no original needs to leave deep**.
- **Smoke (`make smoke`, in `prep-pr`):** optional, only if the user wants the workspace-symbol cap in the pre-push gate. Name the light test `smoke_workspace_symbol_cap_*`. Contract rule 8 then requires a `scripts/dev/smoke-targets.tsv` row: `rust/tcl-lsp-server/src/lib.rs  tcl-lsp-server  lib  tcl_lsp_server`. `cargo xtask smoke-targets check` enforces it, and no contract change is needed.
- **Exhaustive (`#[ignore = "...; run explicitly with --ignored"]`, `make test-exhaustive`):** optional *new* test only. It would be one environment-gated scan against the real host Tcl install, which rule 2 sanctions as environment-gated. No existing test moves here, because exhaustive does not run before a PR.
- **Quick iteration (needs a contract amendment if wanted):** a non-gating `[profile.quick]` in `.config/nextest.toml`:
  ```
  default-filter = 'not (test(=tests::workspace_symbol_caps_its_answer))'
  ```
  Developers would run `cargo nextest run --profile quick -p tcl-lsp-server`.
  - The exclusion list uses exact names, so no selector is renamed.
  - The amendment would add a "quick: local, never a gate, deep minus an explicit list" row to the Tiers table, plus a rule that every listed test must exist and must stay in deep. That needs a drift check, for example a new xtask or an extension to `smoke-targets check`.
  - Once S1, S2 and S4 land, the list is empty and the profile can be dropped. Without the amendment, developers can pass the same `-E` filter ad hoc.
- **Performance budgets:** I recommend **counting budgets, not wall-clock budgets** in the deep tier. Debug wall time on shared runners is noisy.
  - Analyses per `workspace/symbol` request: a `cfg(test)` counter on `FreshAnalysisDriver::analyse`.
  - Bundled-pack parses per reload.
  - Salsa queries per scan: the existing `TclDatabase::with_event_logger` precedent in `scan_workspace_folders_does_not_build_deep_tier_for_unopened_files`.
  These are deterministic, run in seconds, and fail on exactly the "performance has gone bad" class seen here.

## Per-test summary (seconds from the input; `+` means a timeout lower bound; details and file:line evidence are in REVIEW.json)

| selector | s | class | est. new s |
|---|---|---|---|
| `tests::workspace_symbol_caps_its_answer` | 180+ (TIMEOUT) | FIX PRODUCT COST + SPLIT | ~20 |
| `original_document_metadata_tests::startup_index_and_factory_reindex_keep_checked_document_roles` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~8 |
| `original_document_metadata_tests::source_rehoming_keeps_workspace_roles_in_the_actual_seed_namespace` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~8 |
| `tests::a_cross_file_metaclass_resolves_after_the_workspace_scan` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~8 |
| `tests::a_dynamic_metaclass_head_abstains_even_with_the_scan_index` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~8 |
| `tests::a_relative_auto_path_entry_anchors_on_the_document_not_the_server_cwd` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~4 |
| `tests::a_three_level_factory_scan_withdraws_descendants_when_the_source_root_disappears` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~12 |
| `tests::a_triple_nested_dirname_auto_path_feeds_the_package_database` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~4 |
| `tests::a_versioned_require_indexes_the_release_it_asks_for` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~4 |
| `tests::a_workspace_with_no_metaclass_never_sets_the_factory_oracle` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~8 |
| `tests::autoload_merge_is_dropped_when_the_package_database_rebuilds` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~4 |
| `tests::definition_lands_in_the_package_release_tcl_would_load` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~20 |
| `tests::definition_resolves_through_a_document_auto_path_package` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~8 |
| `tests::document_auto_path_mutation_feeds_the_package_database` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~8 |
| `tests::dropping_one_factory_reindexes_its_consumers_while_retaining_others` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~12 |
| `tests::removing_the_last_factory_reindexes_unopened_consumers_with_an_empty_oracle` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~12 |
| `tests::scan_workspace_folders_builds_resolver_with_no_roots` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~8 |
| `tests::scan_workspace_folders_does_not_build_deep_tier_for_unopened_files` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~4 |
| `tests::scan_workspace_folders_indexes_unopened_files` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~4 |
| `tests::scan_workspace_folders_skips_open_documents` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~4 |
| `tests::set_auto_path_puts_every_list_element_on_the_search_path` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) + FIX PRODUCT COST (host package-DB rebuilt per scan) | ~4 |
| `tests::apply_global_config_applies_every_knob` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) | ~2 |
| `tests::declared_package_provides_makes_tk_available_to_diagnostics` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) | ~5 |
| `tests::declared_package_provides_resolve_per_folder` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) | ~2 |
| `tests::declared_package_provides_survive_an_incomplete_buffer` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) | ~5 |
| `tests::resource_scoped_analyser_inputs_resolve_per_folder` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) | ~2 |
| `tests::workspace_folder_add_reschedules_non_xc_document_too` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (systemic S1) | ~2 |
| `tests::initialized_reschedules_open_documents_after_workspace_scan` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (S1) + FIX PRODUCT COST (S2) | ~4 |
| `tests::concurrent_snapshots_share_one_text_and_index_allocation` | 180+ (TIMEOUT) | FIX FIXTURE SETUP (SHRINK work, keep input) | ~1 |
| `tests::default_dialect_change_retries_after_concurrent_edit_1854` | 180+ (TIMEOUT) | FIX WAITS (hang; correctness, not perf) | ~1 |
| `studio_source::retirement_publication_tests::retire_and_shutdown_cannot_acknowledge_failed_withdrawal_and_can_retry` | 110 (PASS) | FIX PRODUCT COST (systemic S2) | ~15 |
| `studio_source::tests::active_saved_source_retires_without_deleting_and_restart_stays_closed` | 96 (PASS) | FIX PRODUCT COST (systemic S2) | ~12 |
| `tests::workspace_trust_comes_from_the_client_and_never_from_a_setting` | 91 (PASS) | FIX PRODUCT COST (S2) | ~10 |
| `studio_source::tests::original_provider_mapping_cannot_rebind_across_equal_byte_aba` | 91 (PASS) | FIX PRODUCT COST (systemic S2) | ~12 |
| `tests::the_last_generation_of_a_burst_still_runs_to_completion` | 72 (PASS) | FIX PRODUCT COST (S2) | ~8 |
| `studio_source::tests::acquire_before_creation_and_shutdown_retire_without_filesystem_cleanup` | 58 (PASS) | FIX PRODUCT COST (systemic S2) | ~8 |
| `studio_source::tests::retirement_invalidates_captured_reload_before_waiting_for_reload_lock` | 58 (PASS) | FIX PRODUCT COST (systemic S2) | ~8 |
| `studio_source::tests::cross_backend_same_uri_token_cannot_publish_or_retire_new_owner` | 57 (PASS) | FIX PRODUCT COST (systemic S2) | ~8 |
| `studio_source::tests::replaced_or_stale_revision_and_missing_entropy_never_donate_a_live_lease` | 56 (PASS) | FIX PRODUCT COST (systemic S2) | ~8 |
| `studio_source::tests::saved_image_changed_after_load_before_publication_is_unavailable` | 40 (PASS) | FIX PRODUCT COST (systemic S2) | ~5 |
| `studio_source::tests::saved_image_changed_while_reload_is_queued_cannot_acknowledge_publication` | 38 (PASS) | FIX PRODUCT COST (systemic S2) | ~5 |
| `policy_truth_table::every_row_publishes_its_shown_set` | 28 (FAIL) | KEEP AS IS | ~27 |
| `tests::a_configuration_burst_coalesces_into_one_reload` | 22 (PASS) | FIX PRODUCT COST (S2) | ~3 |
| `readonly_reference_reports::logical_call_backend_session_tests::fresh_backend_rejects_prior_session_call_on_both_real_endpoints` | 18 (PASS) | KEEP AS IS (test) / FIX PRODUCT COST (S4) | ~9 |
| `tests::did_change_configuration_applies_inline_settings` | 18 (FAIL) | FIX PRODUCT COST (S2); the failure is a perf symptom | ~2 |
| `original_document_metadata_tests::pull_compiler_and_code_actions_keep_supplied_diagnostic_context` | 17 (FAIL) | KEEP AS IS | ~16 |
| `original_document_metadata_tests::unindexed_deep_compiler_checks_borrow_the_shared_base_input` | 16 (FAIL) | KEEP AS IS | ~16 |
| `tests::library_publication_checks_closed_saved_source_and_commits_whole_batch` | 15 (PASS) | KEEP AS IS (minor FIX FIXTURE SETUP possible) | ~10 |
| `tests::pull_path_applies_w120_package_refinement` | 15 (PASS) | KEEP AS IS | ~9 |
| `tests::logical_incoming_target_keeps_procedure_member_side_and_colon_name_separate` | 14 (PASS) | KEEP AS IS (test) / FIX PRODUCT COST (S4) | ~7 |
| `readonly_reference_reports::logical_call_wire_tests::actual_call_handlers_use_opaque_current_source_selection_not_client_fields` | 12 (PASS) | KEEP AS IS (test) / FIX PRODUCT COST (S4) | ~6 |
| `readonly_reference_reports::logical_lens_wire_tests::logical_lens_equal_counter_from_another_backend_cannot_select_this_session` | 11 (PASS) | KEEP AS IS (test) / FIX PRODUCT COST (S4) | ~6 |
| `tests::no_code_lens_ever_carries_an_inert_empty_command` | 11 (FAIL) | KEEP AS IS (product failure + S4) | ~8 |
| `recovery_source::tests::malformed_recovery_keeps_exact_preference_and_unknown_provider_barriers` | 11 (FAIL) | KEEP AS IS | ~10 |
## Caveats

- Nothing was executed. S1 is inferred from a 20/20 correlation plus the code path, and S2 from arithmetic over reload counts; both have a no-code or low-cost probe above. The new-time estimates are static.
- 39 of the 54 rows come from sol-26 3-way-parallel runs on head and on a patched tree; 15 come from capture47 (serial). The 180 s TIMEOUT values are lower bounds, and the parallel figures are inflated.
- 6 rows are FAIL, not TIMEOUT. Five of them fail for non-performance reasons; the panic sites are in REVIEW.json. The sixth, `did_change_configuration_applies_inline_settings`, fails *because* of S2.
