# Test fixture review: work to rust (2026-10-21, Luna-40)

User decision: `resume-20261017-diagnosis/user-decisions-20261021T1205Z.json`. Before the PR to `rust`, review the test fixtures added on `work` for performance, completeness and overlap, and cut their number where possible. Evidence stays on `work`.

Scope: the 29,092 test files added on `work` (f5b18f5049) relative to `origin/rust` (fcb7927f02). 29,053 of them are in 466 `tests/data` families, and 39 are test sources and harness files. Only the review files in this directory were written. No source or fixture was changed, and no commit was made.

Outputs in this directory:
- `inventory.json`: one record per family. Each record gives the consumer, load mechanism, cases, files and bytes, files read and unread, file kinds, what generates the expectations, captured and asserted releases, duplicates, findings, the recommendation and the estimated file count afterwards. The file also lists the 39 non-data files.
- `duplicates.txt`: byte-identical groups, with each copy marked as read (`R`) or unread (`-`), empty files, families copied between runtime and tcl-vm, near-duplicate scripts with a verdict, and copies of files already in the repository.
- `keep-paths.txt`: the 2,514 added files that some test reads, plus each family README and the 39 added test sources. If these are the only added test files carried to `rust`, every current assertion still holds byte for byte.

## Totals

| | Files | Size |
|---|---:|---:|
| Before (added on work) | 29,092 | 1,465 MB |
| Read by some test (plus 39 test sources) | 2,434 | 9.2 MB |
| **After step 1: carry only the files tests read (no harness change)** | **2,518** | **~9.3 MB** |
| **After steps 2-3: merge and dedupe, then table-driven families** | **~1,332** | **~9 MB** |
| Moved to work-only evidence | 26,574 to 27,760 | ~1,455 MB |

Test runtime does not change in any step. Every kept row and assertion stays. What drops is checkout and clone size (-1.46 GB), the corpus hashing in `check-name-resolution-proofs`, and review noise. Details are under "Performance" below.

## Method and limits

- **Read set.** A fixture counts as read when a Rust file in the repository names its path in a string literal and the path resolves to that file. The literal can sit in `include!`, `include_str!`, `include_bytes!`, `concat!`, `Path::join` or `format!` (with placeholders matched against the files). The path is resolved against the consuming file or `CARGO_MANIFEST_DIR`. When a test names only a family directory, the file-name literals in the same consumer resolve it. No consumer runs `read_dir` over a new family; the only `read_dir` loader is the upstream `tclsh_parity`. One family (`native_procedure_recompile`) uses a macro that builds its file names, and it was resolved by hand. Families with generic names (`body_execution`, `resolution`, `diagnostics`) are over-attributed, so their read counts are upper bounds. They are small and are all kept.
- Duplicates come from git blob hashes. Near-duplicates are `.tcl` inputs that match after whitespace and comment normalisation, or after renaming identifiers. Release coverage comes from release tokens in paths and in the tables that tests read.
- Expectation source is taken from each family's capture files (receipts, stderr, `manifest.json`) and from the `software_controls.rs` / `cases.rs` headers.
- **Spot checks.** About 25 consumers were read by hand. These include `native_jim_inventory_tests.rs`, `native_holder_routing.rs`, `native_observation_tests.rs` (vwait), `native_original_numeric_controls.rs`, `native_procedure.rs`, `value_ops.rs`, `cmd_dict.rs`, `execution_conformance.rs` and `body_execution_conformance.rs`.
- **No build.** Free disk was 2.9-3.0 GB against the 2.5 GB floor. A fresh `CARGO_PROFILE_TEST_DEBUG=0` build of tcl-vm or runtime/rust needs its own copy of every dependency, which does not fit in the remaining headroom. Runtime evidence comes from the existing `target/debug/deps/tcl_compiler-ef22ef87d0bd65a7` lib-test binary, built at 12:36 today from the shared working tree with Sol-49's edits. It was run read-only, serially, on shared CPUs. See "Performance".
- `origin/rust` has no `docs/design/analysis/name-resolution-proofs/` and no `scripts/dev/check-name-resolution-proofs.py`, so neither the proof records nor the corpus census are on the target branch today.

## Findings (ordered by files affected)

### F1. 92% of the files are evidence that no test reads
26,658 of the 29,053 data files (1,455.6 MB of 1,464.8 MB) are not read by any test. By kind:

| Kind | Files |
|---|---:|
| stderr captures | 6,570 |
| stdout captures not asserted (superseded attempts, other cases, provenance) | 5,350 |
| receipts | 5,102 |
| unread tables and text | 2,143 |
| JSON (manifests, requests, interpretations, closures) | 1,444 |
| C sources (`probe.c` and vendored `tclBasic.c`, `tclNamesp.c`, `jim.c` and others) | 1,278 |
| binaries and archives | 1,204 |
| unread `.tcl` inputs | 1,049 |
| tooling (`capture.py`, `run.py`, `launch.py`, `prepare.py`, `collector.py`, `Makefile.observer`) | 363 |
| `.argv`, `.source`, `.msg`, `.enc`, `.map` and other | ~2,000 |

The binaries and archives are probe ELF files, `version.elf.gz`, static `libtcl8.6.a`, `libtcl9.0.a` and `libtcl9.1.a`, `baseline-probe`, and `pinned-input-bytes.tar.gz`. They are **1,044 MB, 71% of all bytes**. 282 copies of 11 identical `probe.elf` builds alone account for 413 MB of redundant bytes.

Some families also carry provenance copies of repository and vendor files, 1,499 files under `original-inputs/` or `repo-inputs/`. Examples are `runtime/rust/build.rs`, `.cargo/config.toml`, `rust/tcl-compiler/tests/common/wasm_link.rs` and `runtime/rust/vendor/tcl_library/tcltest/tcltest.tcl`.

None of these files is a test. They are the proof corpus behind `docs/design/analysis/name-resolution-proofs` (`corpus-inventory.json` lists 32,025 files). The commit messages that added them say so: "retain cross-runtime proof corpus", "Record ... evidence". The user decision keeps them on `work`.

### F2. 148 families have no test consumer at all (8,297 files, 378 MB)
Proof records and replay or generator scripts name these families, but no test does. The scripts are `scripts/dev/replay-native-naming-corners.py`, `replay-grammar-native-source.py` and `bigip-probes/.../generate-variable-lifecycle-fixtures.py`, and CI runs none of them. The biggest are:

| Family | Files |
|---|---:|
| `rust/tcl-syntax/.../native_generic_variable_consumers` | 1,008 |
| `rust/tcl-syntax/.../native_naming_corners` | 742 |
| `runtime/rust/.../native_constant_captured_cell_after_write_trace_original` | 508 |
| `rust/tcl-compiler/.../native_brace_expression` | 466 |
| `runtime/rust/.../native_source_original_counted_filename_extent` | 342 |
| `rust/tcl-lsp-core/.../native_inline_variable_semantics` | 323 |
| `runtime/rust/.../native_encoding_original_byte_name_source_order` | 277 |
| `rust/tcl-registry/.../native_literal_emitter_compiler_source` | 236 |
| `.../native_variable_container_columns189` | 200 |
| `.../native_tcltest_alias_caller_frame_original` | 191 |

T3 lists all 148. Eight more families are false matches: a Rust identifier contains the family name but nothing reads the data. They are `native_itcl`, `native_regexp_cache`, `native_index_cache`, `native_namespace_name_cache`, `native_regexp_jim`, `native_interpreter_paths`, the registry copy of `native_variable_array_write` (the tests read the runtime/rust copy), and `native_alias_target_context.tcl` (named only in a comment). **Recommendation: move to work-only evidence (0 files on rust).**

### F3. In 264 of the 318 consumed families the tests read only part of the files
- `native_naming_argument_getter_effect_order_original`: tests read 6 of 1,592 files. They are `499c/providers/<C release>/execute.stdout` and `499d/providers/jim/execute.stdout`. `499b` is a superseded attempt, and `original-inputs/` (1,288 files) holds provenance copies.
- The eight runtime/rust List and String families (`native_original_list_numeric645_647`, `_group_command651`, `_option_index636`, `_string_replacement629`, `_index_range635`, `_list_stride644`, `_binary_replacement633`, `_group_cli656`) each `include!` one `software_controls.rs`. That file is a derived projection table (56 to 385 `Row` literals) of the native capture, and the other 240 to 1,327 files per family are raw evidence.
- `native_info_commands_literal_original` (2 of 863), `native_event_original` (6 consolidated per-release TSVs of 621) and `native_explicit_error_code_header_setter_original` (5 of 534) follow the same pattern.

**Recommendation: carry the read subset (step 1).**

### F4. Exact duplicates
There are 1,699 groups of byte-identical non-empty files, with 5,121 redundant copies (531 MB). In addition, 7,573 files are empty (one blob), mostly empty stderr captures. Among the files tests read, 241 groups hold 643 redundant copies. Most are expected outputs shared across cases, such as an identical `NOT_APPLICABLE` or `UNAVAILABLE` marker for every release where a feature is absent. A table removes these. Step 1 removes the rest, because the duplicated binaries, sources and receipts are all unread. `duplicates.txt` gives the full list.

### F5. Overlap between families (merge or dedupe)
- **`runtime/rust/.../native_compiled_try` and `rust/tcl-registry/.../native_each_try_compilation`.** `8.6.18.tsv`, `9.0.4.tsv` and `9.1.0.tsv` are byte-identical, and both families' tests read them (`native_try.rs`, `native_each.rs`, `native_each_tests.rs`). Keep one table and point both tests at it.
- **`native_dict_update_write_error_lifetime398` and `native_dict_write_error_lifetime226`.** They share `request/original-source.tcl` and the observed case 0, 9 and 19 stderr expectations. Both are read by `rust/tcl-vm/src/cmd_dict.rs`. Merge into one table. `native_dict_with_write_error_lifetime405` is a separate behaviour (`dict with`): keep it, but in the same table format.
- **`native_jim_info_command_inventory254` and `native_jim_rooted_info_command_inventory256`.** The six sources are the same except for `info` versus `::info`. Jim expectations are identical and the C expectations differ. Merge into one table with a "head" column and keep all 84 rows.
- **Copies between runtime/rust and rust/tcl-vm.** `rust/tcl-vm/tests/data/{native_global_cache_token, native_jim_helper_literal_head, native_jim_namespace_parent, native_property_clientdata, native_property_opaque}` are byte-identical to the `runtime/rust/tests/data` families of the same name, and most of `native_alias_simple` is too (its `canonical/` folder differs). Keep one copy and include it across crates. This is already the norm: runtime/rust and tcl-vm unit tests include `rust/tcl-registry/tests/data/...`.
- **Same fixture asserted by both runtime/rust and rust/tcl-vm unit tests** (about 180 families). This is intentional, because they are two engines. Keep it.
- **Near-duplicate false positive.** `native_jim_catch_try_original_output_keys/global-call.tcl` and `native_jim_original_global_error_code_getter/call.tcl` differ in the command under test (`reset_selected_error_code` versus `read_selected_error_code`). Both are distinct.
- **Upstream overlap.** No added fixture is byte-identical to a test file on `origin/rust`. The 49 identical files are copies of vendored library or repository sources (F1). No family duplicates the upstream `rust/tcl-vm/tests/data/tclsh_parity` cases.

### F6. Trivial expectations
219 read expectations in 15 families are empty, or decode to `NOT_APPLICABLE` or `UNAVAILABLE`. The script prints a marker on releases where the behaviour does not apply, and the test asserts that the engine takes the same branch. That is a weak but real assertion, so keep it. Tables encode it once per family instead of once per case. The largest are:

| Family | Marker rows |
|---|---:|
| `native_jim_namespace_distribution236` | 44 |
| `native_jim_ensemble_rooted_result245` | 40 |
| `native_coroutine_public_injection255` | 32 |
| `native_jim_alias_introspection274` | 20 |

`native_w216_literal_replacement_value325` has 6 empty files, and in the WIP binary the test that reads them (`w216_fixed_proposals_match_captured_public_value_examples`) currently fails.

### F7. Completeness and coverage gaps
These were not created by this review, and no step below removes an existing assertion.
1. **Captured per-release expectations next to asserted ones, but not asserted.** There are 393 files in 40 families. 185 of them are absent-feature captures that are fine to leave out: for example TclOO, `try`, `apply` or coroutines on 8.4/8.5, or ensembles on 8.4. **208 are substantive:**
   - `native_info_inventory_original`: 138 C-release `exact-*/stdout.tsv` files, while only `jim/exact-*` and the vendored `tclCmdIL.c` are read.
   - `native_dict_update_write_error_lifetime398` and `native_dict_with_write_error_lifetime405`: 9.0.4 and 9.1.0 runs, 10 files each, while the test asserts 8.6.18 only.
   - `native_primitive_boolean_expression_truth_original`: C releases, 5 files; only Jim is asserted.
   - `native_jim_alias_publication194`: C releases, 5 files.
   - `native_concat`: `arithseries-8.4/8.5/8.6/jim`, 4 files.
   - `native_control_expression`: 8.4/8.5 contexts, 3 files.
   - `native_dictionary_search`: 8.4 and Jim, 3 files.
   - `native_equality`: 8.4/8.5, 2 files. `rust/tcl-vm/src/value_ops.rs` asserts 8.6/9.0/9.1/Jim only.
   - `native_substitution_owner` v6 8.4/8.5, 2 files.
   - `native_switch_compiler_sources` 8.4, 1 file.
   - `native_error_variables` 8.4 producers, 1 file.
   - `native_list_operations` `jim.txt`, 1 file.
   - Jim captures for `native_namespace_string_compilation` and `native_child_command_lifetime`, 1 file each.

   Each one needs a decision: wire it into the existing loop (this adds coverage, and the release list is already there), or leave it on `work`. `inventory.json` lists them under `unasserted_release_siblings`.
2. **Families with complete tclsh/jimsh expectations that are never asserted** (in F2). These are:

| Family | What is captured |
|---|---|
| `native_naming_corners` | 40 sources × 5 C releases + Jim |
| `native_generic_variable_consumers` | 50 programs × 6 engines |
| `native_inline_variable_semantics` | |
| `native_brace_expression` | |
| `native_constant_captured_cell_after_write_trace_original` | |
| `native_source_original_counted_filename_extent` | |
| `native_encoding_original_byte_name_source_order` | |
| `native_variable_container_columns189` | |
| `native_tcltest_alias_caller_frame_original` | |
| `native_literal_emitter_compiler_source` | |

   Their READMEs say the implementation comparison lives in other tests. Moving them loses no assertion. If they should become tests, each one is a single table-driven conformance file (source plus per-release code and result).
3. **Oracle conformance families skip silently in the main CI.** `rust/tcl-syntax/.../body_execution`, `jim_name_resolution` and `resolution` run real tclsh or jimsh. Without `TCL_LSP_REQUIRE_*`, those tests return early when no oracle is installed; `resolution-oracles.yml` (`scripts/dev/run-resolution-oracles.sh`) is the job that enforces them. Keep them as files.
4. **Expectation provenance.** Of the 2,395 files tests read:
   - 2,051 are native tclsh or jimsh captures.
   - 76 are derived projection tables of a capture (`software_controls.rs`, `cases.rs`). These are hand-transformed or tool-transformed, so a re-capture does not update them automatically.
   - 268 are authored tables, for example `*_vectors.txt`, `controls.tsv` and `native_bootstrap/core-roots.tsv`.

   Some tests read only the `.tcl` inputs and keep the expected values in Rust. One example is `native_path_constant_scopes` in `rust/tcl-compiler/src/auto_path_eval/path_constants.rs`, where the per-release stdout files are unread. Those expectations were transcribed from the capture, so the capture is the evidence and stays on `work`.
5. **Releases.** Every consumed C-Tcl family covers 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 where the feature exists, plus Jim 0.84 where relevant, apart from the gaps in item 1. Release coverage per family is in `inventory.json` (`releases.captured` and `releases.asserted`).

### F8. Harness and structure notes
- Almost all consumers are `#[cfg(test)]` unit modules inside `runtime/rust/src` (224 family references) and `rust/tcl-vm/src` (181), not `tests/*.rs` targets. They reach other crates' data with paths such as `../../../../rust/tcl-registry/tests/data/...`. Moving or merging a family means editing those relative paths in both engines.
- `rust/tcl-syntax/src/execution_conformance.rs` is a `pub mod`, not `cfg(test)`. It `include_str!`s `tests/data/{command_binding,package_lookup,autoload}_vectors.txt` into the library build. These files must stay where they are, or the module must move behind a feature or `cfg(test)`.
- Nothing is regenerated in CI. The capture tooling lives inside each family (`capture.py`, `run.py`, `probe.c`) and needs native Tcl/Jim SDK builds. Integrity is checked only by `scripts/dev/check-name-resolution-proofs.py` (`make check-name-resolution-proofs`, part of `xtask-check`), which hashes all 32,025 corpus files and exists only on `work`.

## Performance

- **What drives cost.** The number of fixture files does not drive test runtime. Of the 318 consumed families, 312 are loaded at compile time with `include_str!`, `include_bytes!` or `include!` (6 of these also read through `std::fs` at runtime). One reads only through `std::fs`: `native_vwait_original`, with 90 small reads. For 5 the mechanism could not be classified. No loader walks or re-parses a directory per test, and there is no one-`#[test]`-per-file pattern: each test loops over its rows. Cost scales with rows times interpreter construction or compilation.
- **Measured.** The existing `tcl_compiler` lib-test binary was run serially, with debug info, on 4 CPUs shared with two agents. It ran 19 tests that read fixtures from 15 families:

| Duration | Tests |
|---|---|
| 0.02-0.2 s | 10 tests: `native_double_completion`, `native_integer_contents_results`, `native_namespace_unknown_lookup`, `native_formal_slots`, `native_expanded_head`, `f5-matches tsv`, `native_c84_parse_context`, `native_deep_array_*` (2), `w216` capture rows |
| 0.7-0.9 s | 3 tests: `native_path_constant_scopes` (2), `w216` proposals |
| 4.4-6.9 s | 3 tests: `native_original_jim_text` 4.4 s, `native_deep_array_source` 5.8 s, `native_activation_presence` 6.9 s |
| 10-18 s | `native_namespace_bindings` (100 windows) 10.1 s; `registered-switch56.tsv` (56 frontiers) 14.3 s; `native_switch_compiler_sources` preflight 17.8 s |

  The slow tests read 1 to 8 small files each, so their time is compilation work, not fixture I/O. Module-level runs with the same binary are similar: `command_binding::compiler_inventory` takes 42 s for 19 tests and `sccp::original_value_facts` 24 s for 92. Several tests fail in this binary because it was built from the WIP tree. The tcl-vm and runtime/rust consumers could not be timed (no binary, and no disk to build one). They build one `Interp::with_native_core` per row, so expect milliseconds per row.
- **Effect of the plan.**
  - Steps 1-3 leave test runtime unchanged.
  - Compile time is unchanged in steps 1-2, because unread files are never compiled.
  - The checkout loses 1.46 GB, which affects CI clone, cache restore and `check-name-resolution-proofs` hashing time on `work`.
- **Suggested compile-time saving (not measured).** Nine `software_controls.rs` tables total 700 KB of Rust with 1,251 `Row` literals. Each is `include!`d into both the runtime/rust and the tcl-vm lib-test crates, so rustc type-checks large const arrays twice in debug. Converting them to TSV parsed at test time would cut that cost without changing rows.
- The slow analysis tests above belong with the 2026-10-19 slow-test tiering (`test-performance-20261019`), not with fixture reduction.

## Reduction plan

**Step 1. Carry only what tests read** (no harness change): 29,092 to 2,518 files.
- Build the PR tree from `keep-paths.txt`, adding only those of the 29,092 added test files. Everything else stays on `work`, at its current paths, so the proof records keep resolving there.
- Every `include_*!` and `fs` path still resolves, so assertions are byte-for-byte unchanged.
- Leave `docs/design/analysis/name-resolution-proofs/{manifest,corpus-inventory}.json` and `scripts/dev/check-name-resolution-proofs.py` off the PR, because they validate the evidence corpus. If the proof `.md` files are carried as design docs, they will cite `tests/data` paths that are absent on `rust`. Either mark them as work-only evidence links, or exclude them as well.
- Verify by running the runtime/rust, tcl-vm, tcl-registry, tcl-syntax, tcl-compiler, tcl-cmd-core, tcl-lsp-core and tcl-test-support test targets on the reduced tree. A missing file is a compile error, so the build alone proves the keep list is complete.

**Step 2. Merge and dedupe** (small harness edits, same rows): about 66 more files. These are the 42 files of `256`, merged into `254`'s table; 15 tcl-vm copies; 6 files of `226`; and 3 of `compiled_try`.
1. `native_compiled_try` reads `rust/tcl-registry/tests/data/native_each_try_compilation/{8.6.18,9.0.4,9.1.0}.tsv`. Change the include paths in `runtime/rust/src/interp/native_body_artifact/native_try.rs`.
2. Fold `native_dict_write_error_lifetime226` into `native_dict_update_write_error_lifetime398`, with one table per family. Edit `rust/tcl-vm/src/cmd_dict.rs`.
3. Merge `native_jim_rooted_info_command_inventory256` into `native_jim_info_command_inventory254` as a table with a "head" column. Edit `runtime/rust/src/cmd_info/native_jim_inventory_tests.rs` and `rust/tcl-vm/src/cmd_info/native_jim_inventory_tests.rs`.
4. Have rust/tcl-vm include `runtime/rust/tests/data/{native_global_cache_token, native_jim_helper_literal_head, native_jim_namespace_parent, native_property_clientdata, native_property_opaque, native_alias_simple}`. Keep only `native_alias_simple/canonical/` in tcl-vm. Edit `rust/tcl-vm/src/interp/native_variable_names.rs`, `cmd_namespace/native_jim_helpers.rs`, `interp/native_namespace_names/jim_parent.rs` and `cmd_oo/native_properties.rs`.

**Step 3. Table-driven families** (harness change per family): 51 families, 2,518 to about 1,332 files.
- Replace the per-file arrays with one table per family. A family currently built from `include_str!` arrays or macros of per-release `stdout` files (6 releases times N cases) gets one TSV with columns `case`, `release`, `code`, `result-hex`, plus `source-hex` where the sources are binary.
- The test then `include_str!`s the table and checks the row count it already asserts (for example `assert_eq!(comparisons, 12)`).
- Keep a family's `.tcl` sources as files when there are 12 or fewer, because they are what a human edits. Otherwise fold them in as hex.
- Examples, read files to files after: `native_info_inventory_original` 57 to 2, `native_vwait_original` 90 to 1, `native_encoding_utf8_object306` 51 to 1, `native_jim_core_command_inventory257` 56 to 9, `native_namespace_holder_routing202` 39 to 10, `native_ensemble_holder_routing205` 30 to 7 and `native_array_owner_teardown347` 30 to 1.
- Every row, release and expected byte is preserved. `NOT_APPLICABLE` rows stay as rows. T4 gives the full list.

**Optional step 4. Coverage decisions** (F7): wire the 208 substantive unasserted per-release captures into the existing loops, or leave them on `work`. The highest-value items are the dict lifetime families for 9.0/9.1, `native_equality` for 8.4/8.5, the `native_info_inventory_original` C releases and `native_primitive_boolean_expression_truth_original` for C. Wiring them in adds rows.

**Step 5, also optional** (compile time): convert the nine `software_controls.rs` const tables to TSV.

### Harness changes, summarised
- **Step 1:** none in tests. Release tooling builds the PR from `keep-paths.txt`. Proof-corpus files are not carried, or `check-name-resolution-proofs` is scoped to `work`.
- **Step 2:** path or table edits in 8 consumer files, listed in step 2 above.
- **Step 3:** about 51 small table parsers. A shared helper in `tcl-test-support` (parse `case<TAB>release<TAB>code<TAB>hex`) keeps them to about 5 lines each. Edit both engines' consumers (runtime/rust and rust/tcl-vm) for each family.
- **`tcl-syntax`:** keep `tests/data/*_vectors.txt` in place, or move `execution_conformance` behind `cfg(test)` or a feature.

### T1. Per crate
| Crate | Families | Files before | MB before | Files read by tests | After (read subset) | After (tables) |
|---|---:|---:|---:|---:|---:|---:|
| rust/tcl-registry | 226 | 15,024 | 849.1 | 1,532 | 1,558 | 547 |
| runtime/rust | 98 | 9,024 | 554.7 | 243 | 255 | 241 |
| rust/tcl-syntax | 40 | 2,463 | 3.1 | 293 | 308 | 214 |
| rust/tcl-vm | 49 | 1,043 | 4.7 | 210 | 230 | 181 |
| rust/tcl-compiler | 21 | 705 | 20.7 | 44 | 50 | 50 |
| rust/tcl-lsp-core | 4 | 377 | 0.2 | 12 | 12 | 12 |
| rust/tcl-cmd-core | 16 | 266 | 27.3 | 40 | 44 | 26 |
| rust/tcl-test-support | 3 | 49 | 1.8 | 18 | 18 | 18 |
| rust/tcl-cshim | 4 | 46 | 0.1 | 0 | 0 | 0 |
| rust/tcl-lexer | 2 | 36 | 0.0 | 0 | 0 | 0 |
| rust/tcl-core-types | 1 | 10 | 3.1 | 1 | 2 | 2 |
| rust/tcl-runtime-api | 1 | 7 | 0.0 | 1 | 1 | 1 |
| rust/tcl-regex | 1 | 3 | 0.0 | 1 | 1 | 1 |
| other added test sources (tests/*.rs etc.) | - | 39 | - | 39 | 39 | 39 |
| **Total** | 466 | 29,092 | 1464.8 | 2,434 | 2,518 | 1,332 |

### T2. Families ordered by files saved (top 40; all 466 in inventory.json)
| # | Family | Files | Read | After | Saved | MB | Recommendation |
|---:|---|---:|---:|---:|---:|---:|---|
| 1 | `rust/tcl-registry/…/native_naming_argument_getter_effect_order_original` | 1592 | 6 | 6 | 1586 | 29.0 | keep-read-subset |
| 2 | `runtime/rust/…/native_original_list_numeric645_647` | 1328 | 1 | 1 | 1327 | 44.0 | keep-read-subset |
| 3 | `rust/tcl-syntax/…/native_generic_variable_consumers` | 1008 | 0 | 0 | 1008 | 0.5 | move-to-work-evidence |
| 4 | `rust/tcl-registry/…/native_info_inventory_original` | 987 | 57 | 2 | 985 | 3.5 | keep-read-subset+table-driven |
| 5 | `rust/tcl-registry/…/native_info_commands_literal_original` | 863 | 2 | 2 | 861 | 20.8 | keep-read-subset |
| 6 | `rust/tcl-syntax/…/native_naming_corners` | 742 | 0 | 0 | 742 | 0.5 | move-to-work-evidence |
| 7 | `runtime/rust/…/native_original_list_group_command651` | 681 | 1 | 1 | 680 | 11.7 | keep-read-subset |
| 8 | `runtime/rust/…/native_original_option_index636` | 667 | 1 | 1 | 666 | 33.4 | keep-read-subset |
| 9 | `rust/tcl-registry/…/native_event_original` | 621 | 6 | 6 | 615 | 1.9 | keep-read-subset |
| 10 | `runtime/rust/…/native_explicit_error_code_header_setter_original` | 534 | 5 | 5 | 529 | 34.4 | keep-read-subset |
| 11 | `runtime/rust/…/native_constant_captured_cell_after_write_trace_original` | 508 | 0 | 0 | 508 | 24.8 | move-to-work-evidence |
| 12 | `runtime/rust/…/native_original_string_replacement629` | 469 | 1 | 1 | 468 | 32.5 | keep-read-subset |
| 13 | `rust/tcl-compiler/…/native_brace_expression` | 466 | 0 | 0 | 466 | 20.4 | move-to-work-evidence |
| 14 | `runtime/rust/…/native_original_index_range635` | 451 | 1 | 1 | 450 | 32.5 | keep-read-subset |
| 15 | `runtime/rust/…/native_method_cache_active_call_lifetime_original` | 435 | 3 | 3 | 432 | 26.0 | keep-read-subset |
| 16 | `runtime/rust/…/native_original_list_stride644` | 415 | 1 | 1 | 414 | 32.4 | keep-read-subset |
| 17 | `runtime/rust/…/native_method_rename_operands_records_original` | 403 | 3 | 3 | 400 | 32.8 | keep-read-subset |
| 18 | `runtime/rust/…/native_source_original_counted_filename_extent` | 342 | 0 | 0 | 342 | 51.5 | move-to-work-evidence |
| 19 | `runtime/rust/…/native_variable_introspection_getter_order_original` | 330 | 6 | 6 | 324 | 19.2 | keep-read-subset |
| 20 | `rust/tcl-lsp-core/…/native_inline_variable_semantics` | 323 | 0 | 0 | 323 | 0.1 | move-to-work-evidence |
| 21 | `rust/tcl-registry/…/native_vwait_original` | 293 | 90 | 1 | 292 | 0.2 | keep-read-subset+table-driven |
| 22 | `runtime/rust/…/native_encoding_original_byte_name_source_order` | 277 | 0 | 0 | 277 | 64.2 | move-to-work-evidence |
| 23 | `rust/tcl-registry/…/native_encoding_utf8_object306` | 252 | 51 | 1 | 251 | 12.0 | keep-read-subset+table-driven |
| 24 | `runtime/rust/…/native_original_list_group_cli656` | 252 | 2 | 2 | 250 | 7.2 | keep-read-subset |
| 25 | `runtime/rust/…/native_original_binary_replacement633` | 242 | 1 | 1 | 241 | 31.1 | keep-read-subset |
| 26 | `rust/tcl-registry/…/native_literal_emitter_compiler_source` | 236 | 0 | 0 | 236 | 0.4 | move-to-work-evidence |
| 27 | `rust/tcl-registry/…/native_capi_scalar_publication_original` | 217 | 12 | 1 | 216 | 22.3 | keep-read-subset+table-driven |
| 28 | `rust/tcl-registry/…/native_jim_ensemble_rooted_result245` | 224 | 60 | 13 | 211 | 20.3 | keep-read-subset+table-driven |
| 29 | `rust/tcl-registry/…/native_jim_namespace_distribution236` | 220 | 60 | 13 | 207 | 18.6 | keep-read-subset+table-driven |
| 30 | `rust/tcl-registry/…/native_coroutine_public_injection255` | 206 | 48 | 1 | 205 | 18.6 | keep-read-subset+table-driven |
| 31 | `rust/tcl-registry/…/native_variable_container_columns189` | 200 | 0 | 0 | 200 | 0.2 | move-to-work-evidence |
| 32 | `rust/tcl-registry/…/native_tcltest_alias_caller_frame_original` | 191 | 0 | 0 | 191 | 2.0 | move-to-work-evidence |
| 33 | `rust/tcl-registry/…/native_ensemble_map_prefix312` | 190 | 20 | 1 | 189 | 15.7 | keep-read-subset+table-driven |
| 34 | `rust/tcl-registry/…/native_jim_core_command_inventory257` | 191 | 56 | 9 | 182 | 10.0 | keep-read-subset+table-driven |
| 35 | `rust/tcl-registry/…/native_dict_update_write_error_lifetime398` | 187 | 6 | 6 | 181 | 47.9 | keep-read-subset |
| 36 | `rust/tcl-registry/…/native_dict_with_write_error_lifetime405` | 186 | 6 | 6 | 180 | 45.3 | keep-read-subset |
| 37 | `rust/tcl-registry/…/native_interpreter_option_tables210` | 180 | 48 | 1 | 179 | 9.4 | keep-read-subset+table-driven |
| 38 | `runtime/rust/…/native_property_slot_reference_order_original` | 175 | 1 | 1 | 174 | 14.4 | keep-read-subset |
| 39 | `rust/tcl-registry/…/native_child_alias_inventory298` | 174 | 42 | 7 | 167 | 10.7 | keep-read-subset+table-driven |
| 40 | `rust/tcl-registry/…/native_command_recreation_original` | 174 | 7 | 7 | 167 | 8.0 | keep-read-subset |

### T3. Evidence-only families (no test reads any file): move to work-only evidence
148 families, 8,297 files, 378.5 MB.

| Family | Files | MB | Referenced by |
|---|---:|---:|---|
| `rust/tcl-syntax/tests/data/native_generic_variable_consumers` | 1008 | 0.5 | proof records / scripts |
| `rust/tcl-syntax/tests/data/native_naming_corners` | 742 | 0.5 | proof records / scripts |
| `runtime/rust/tests/data/native_constant_captured_cell_after_write_trace_original` | 508 | 24.8 | nothing |
| `rust/tcl-compiler/tests/data/native_brace_expression` | 466 | 20.4 | nothing |
| `runtime/rust/tests/data/native_source_original_counted_filename_extent` | 342 | 51.5 | nothing |
| `rust/tcl-lsp-core/tests/data/native_inline_variable_semantics` | 323 | 0.1 | nothing |
| `runtime/rust/tests/data/native_encoding_original_byte_name_source_order` | 277 | 64.2 | nothing |
| `rust/tcl-registry/tests/data/native_literal_emitter_compiler_source` | 236 | 0.4 | nothing |
| `rust/tcl-registry/tests/data/native_variable_container_columns189` | 200 | 0.2 | nothing |
| `rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original` | 191 | 2.0 | nothing |
| `rust/tcl-registry/tests/data/native_namespace_source_controls` | 137 | 0.3 | nothing |
| `runtime/rust/tests/data/native_selected_grammar_complete_prefix_provider_execution_original` | 135 | 0.8 | nothing |
| `runtime/rust/tests/data/native_ordinary_cli_method_role_report_values_original` | 128 | 2.6 | nothing |
| `rust/tcl-registry/tests/data/native_active_object_command_deletion` | 124 | 5.3 | nothing |
| `rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original` | 123 | 45.1 | nothing |
| `rust/tcl-registry/tests/data/native_arithseries_multiple_index231` | 112 | 9.5 | nothing |
| `rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original` | 110 | 16.1 | nothing |
| `runtime/rust/tests/data/native_reset_result_error_header_retirement_original` | 105 | 7.8 | nothing |
| `rust/tcl-registry/tests/data/native_oo_context296` | 104 | 9.3 | nothing |
| `rust/tcl-vm/tests/data/native_oo_bootstrap_capability` | 100 | 0.3 | nothing |
| `rust/tcl-registry/tests/data/native_mathop_public_envelopes222` | 86 | 9.3 | nothing |
| `rust/tcl-registry/tests/data/native_nested_foreach_source214` | 84 | 9.3 | nothing |
| `rust/tcl-vm/tests/data/native_lassign_argv_evaluation` | 82 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_variable_container_envelope188` | 78 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/current_optimizer_boolean_result_production` | 67 | 5.5 | nothing |
| `rust/tcl-compiler/tests/data/native_tail_frame_release` | 65 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_wasi_libc_environment_original` | 53 | 3.4 | nothing |
| `runtime/rust/tests/data/original_entered_call_chain_report_source` | 52 | 0.2 | nothing |
| `rust/tcl-registry/tests/data/native_jim_core_alias_prefix273` | 52 | 10.2 | nothing |
| `rust/tcl-registry/tests/data/native_mathop_constructor_publication424` | 52 | 9.4 | nothing |
| `rust/tcl-syntax/tests/data/native_scalar_math` | 50 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_child_bootstrap_context303` | 49 | 9.2 | nothing |
| `rust/tcl-registry/tests/data/native_mathop_constructor_publication422` | 47 | 8.4 | nothing |
| `rust/tcl-registry/tests/data/native_source_error_context295` | 47 | 9.2 | nothing |
| `rust/tcl-registry/tests/data/native_oo_variable_header300` | 46 | 9.2 | nothing |
| `rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399` | 45 | 7.0 | nothing |
| `runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original` | 43 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_namespace_ensemble_options_original` | 43 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap` | 43 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_variable_trace_prefix` | 43 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_original_source_crlf330` | 41 | 0.0 | nothing |
| `rust/tcl-cmd-core/tests/data/native_info_functions_loaded` | 37 | 0.0 | nothing |
| `rust/tcl-lsp-core/tests/data/native_minifier_script_role` | 37 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_source_escape_getter_original` | 36 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_source_surrogate_replacement_escape_values` | 36 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_absolute_trace_source253` | 35 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_array_and_oo_source_controls` | 33 | 0.1 | proof records / scripts |
| `rust/tcl-registry/tests/data/native_callback_scope240` | 33 | 0.0 | nothing |
| `rust/tcl-vm/tests/data/native_interpreter_paths` | 33 | 0.1 | proof records / scripts |
| `runtime/rust/tests/data/native_selected_method_role_report_source_contract` | 30 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_child_colon_publication` | 29 | 0.4 | nothing |
| `rust/tcl-test-support/tests/data/native_resolution_reference_builds` | 28 | 1.7 | nothing |
| `rust/tcl-registry/tests/data/native_matcher_output_retention317` | 27 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_matcher_output_retention_alias322` | 27 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_w216_literal_replacement324` | 27 | 0.0 | nothing |
| `rust/tcl-syntax/tests/data/native_namespace_delete_callback_events` | 26 | 0.0 | proof records / scripts |
| `rust/tcl-registry/tests/data/native_conditional_expression_results` | 25 | 0.0 | proof records / scripts |
| `runtime/rust/tests/data/native_io_debug_public_conversion_side_effect_order_original` | 24 | 0.3 | nothing |
| `runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original` | 24 | 0.2 | nothing |
| `rust/tcl-registry/tests/data/native_empty_tcloo_lifecycle231` | 23 | 0.3 | nothing |
| `rust/tcl-registry/tests/data/native_original_array_observer` | 23 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_property_option_width281` | 23 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_setter_expression_original` | 23 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_implicit_function_presence_original` | 22 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_interpreter_created_parent_command` | 22 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_interpreter_created_parent_command_move` | 22 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_lambda_namespace_source` | 22 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_original_formal_count` | 22 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_source_escapes_original` | 22 | 0.0 | nothing |
| `runtime/rust/tests/data/native_namespace_call_levels` | 21 | 0.2 | nothing |
| `rust/tcl-lexer/tests/data/native_empty_array_root` | 21 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_active_read_unset_original` | 21 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_command_source_extent286` | 21 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_jim_class_initialisers271` | 21 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_jim_class_rooted_initialisers275` | 21 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_jim_class_source269` | 21 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_lassign_target_arity_original` | 21 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_recreated_element_read_original` | 21 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_return_evaluated_worker381` | 21 | 0.2 | nothing |
| `rust/tcl-registry/tests/data/native_return_evaluated_worker382` | 21 | 0.2 | nothing |
| `rust/tcl-registry/tests/data/native_return_option_pair_grammar378` | 21 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_safe_visibility_original` | 21 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_variable_array_write` | 21 | 0.0 | proof records / scripts |
| `rust/tcl-registry/tests/data/native_variable_container_runtime187` | 21 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_variable_read_diagnostic_original` | 21 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_callback_alias_source255` | 20 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_literal_set_compiler_source` | 20 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_package_future_frame` | 20 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_string_option_trailing_values` | 20 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_tcloo_stock_destroy` | 20 | 0.0 | nothing |
| `runtime/rust/tests/data/original_introspection_target_continuation_source` | 19 | 0.0 | nothing |
| `rust/tcl-compiler/tests/data/native_original_lifecycle_providers` | 17 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_info_coroutine_compiler_source317` | 17 | 0.3 | nothing |
| `rust/tcl-registry/tests/data/native_integer_formatter216` | 17 | 2.0 | nothing |
| `runtime/rust/tests/data/original_explicit_error_code_header_setter_source` | 15 | 0.1 | nothing |
| `rust/tcl-cshim/tests/data/native_callback_completions` | 15 | 0.0 | nothing |
| `rust/tcl-cshim/tests/data/native_command_publication` | 15 | 0.0 | nothing |
| `rust/tcl-lexer/tests/data/native_quoted_word_geometry` | 15 | 0.0 | proof records / scripts |
| `runtime/rust/tests/data/original_classvariable_preflight_link_owner_source` | 14 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_callback_lookup_scope` | 14 | 0.0 | proof records / scripts |
| `rust/tcl-compiler/tests/data/native_itcl` | 13 | 0.0 | proof records / scripts |
| `rust/tcl-registry/tests/data/native_ensemble_missing_selector_source297` | 13 | 0.6 | nothing |
| `rust/tcl-registry/tests/data/native_expression_compiler_name_effects` | 13 | 0.8 | nothing |
| `rust/tcl-registry/tests/data/native_jim_child_variable_extent306` | 13 | 0.4 | nothing |
| `rust/tcl-vm/tests/data/native_fixed_math_table` | 13 | 0.0 | nothing |
| `runtime/rust/tests/data/original_definition_getter_effect_owner_source` | 12 | 0.1 | nothing |
| `rust/tcl-registry/tests/data/native_compiled_local_formal_source272` | 11 | 0.5 | nothing |
| `rust/tcl-registry/tests/data/native_namespace_command_teardown227` | 11 | 0.8 | nothing |
| `rust/tcl-registry/tests/data/native_jim_dictionary_bootstrap191` | 10 | 0.2 | nothing |
| `rust/tcl-registry/tests/data/native_no_hook_source` | 10 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_selected_worker_names` | 10 | 0.0 | nothing |
| `runtime/rust/tests/data/native_regex_original` | 8 | 0.0 | nothing |
| `runtime/rust/tests/data/native_regexp_cache` | 8 | 0.0 | proof records / scripts |
| `rust/tcl-compiler/tests/data/native_source_normal_results` | 8 | 0.0 | nothing |
| `rust/tcl-cshim/tests/data/native_index_cache` | 8 | 0.0 | proof records / scripts |
| `rust/tcl-cshim/tests/data/native_original_objects` | 8 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_named_worker_rewrite` | 8 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_namespace_final_token` | 8 | 0.0 | nothing |
| `rust/tcl-vm/tests/data/native_body_bytecode` | 8 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_return_provider_executable_audit387` | 7 | 11.8 | nothing |
| `rust/tcl-vm/tests/data/jim_original_deferred_script_ownership` | 7 | 1.8 | nothing |
| `rust/tcl-vm/tests/data/native_namespace_name_cache` | 7 | 0.0 | proof records / scripts |
| `rust/tcl-cmd-core/tests/data/native_time_object_source` | 6 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_jim_flat_namespace_original` | 6 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_list_constructor_return` | 6 | 0.0 | nothing |
| `rust/tcl-cmd-core/tests/data/native_trace_selected_type_source` | 5 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_jim_package_hidden_list` | 5 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_procedure_usage_source285` | 5 | 0.6 | nothing |
| `rust/tcl-vm/tests/data/jim_original_concatenation_ownership` | 5 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_property_names` | 4 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_return_single_result` | 4 | 0.0 | proof records / scripts |
| `rust/tcl-registry/tests/data/native_substitution_token_source` | 4 | 0.1 | nothing |
| `runtime/rust/tests/data/native_regexp_jim` | 3 | 0.0 | proof records / scripts |
| `rust/tcl-cmd-core/tests/data/native_tcloo_object_eval_source` | 3 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source` | 3 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_tcloo_variable_info_source` | 3 | 0.0 | nothing |
| `runtime/rust/tests/data/native_jim_source_entry` | 2 | 0.0 | nothing |
| `rust/tcl-compiler/tests/data/native_namespace_variable_identity` | 2 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_corpus_naming_patterns` | 2 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_wiki_naming_patterns` | 2 | 0.1 | nothing |
| `rust/tcl-vm/tests/data/authored_package_runtime` | 2 | 0.0 | nothing |
| `rust/tcl-compiler/tests/data/native_alias_target_context.tcl` | 1 | 0.0 | proof records / scripts |
| `rust/tcl-compiler/tests/data/native_callable_static_source` | 1 | 0.0 | nothing |
| `rust/tcl-compiler/tests/data/native_signature_scope.tcl` | 1 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_coroutine_object_transport_source` | 1 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_coroutine_resume_source` | 1 | 0.0 | nothing |
| `rust/tcl-registry/tests/data/native_tcloo_own_method_source` | 1 | 0.0 | nothing |
| `rust/tcl-vm/tests/data/native_oo_frame_lifecycle.c` | 1 | 0.0 | nothing |

### T4. Table-driven conversions
51 families; read files 1,276 -> 167.

| Family | Read files | After | Cases | Consumers |
|---|---:|---:|---:|---|
| `rust/tcl-registry/…/native_vwait_original` | 90 | 1 | 15 | native_observation_tests.rs |
| `rust/tcl-syntax/…/native_scalar_getters` | 66 | 2 | 2529 | native_number_tests.rs, typed_value.rs, native_info_level.rs |
| `rust/tcl-registry/…/native_jim_ensemble_rooted_result245` | 60 | 13 | 12 | native_holder_routing.rs, native_command_holder_tests.rs |
| `rust/tcl-registry/…/native_jim_namespace_distribution236` | 60 | 13 | 12 | native_holder_routing.rs, native_scripted_distribution.rs, native_command_holder_tests.rs |
| `rust/tcl-registry/…/native_info_inventory_original` | 57 | 2 | 52 | native_jim.rs, info_.rs, info_jim.rs |
| `rust/tcl-registry/…/native_jim_core_command_inventory257` | 56 | 9 | 8 | native_jim_inventory_tests.rs, native_jim_inventory_tests.rs |
| `rust/tcl-registry/…/native_encoding_utf8_object306` | 51 | 1 | 10 | encoding_utf8.rs |
| `rust/tcl-registry/…/native_coroutine_public_injection255` | 48 | 1 | 8 | cmd_coro.rs |
| `rust/tcl-registry/…/native_interpreter_option_tables210` | 48 | 1 | 8 | native_option_tables.rs |
| `rust/tcl-registry/…/native_child_alias_inventory298` | 42 | 7 | 6 | native_child_alias_publication_tests.rs, native_child_alias_publication_tests.rs |
| `rust/tcl-registry/…/native_jim_info_command_inventory254` | 42 | 2 | 6 | native_jim_inventory_tests.rs, native_jim_inventory_tests.rs |
| `rust/tcl-registry/…/native_namespace_holder_routing202` | 39 | 10 | 9 | native_holder_routing.rs, native_command_holder_tests.rs |
| `rust/tcl-registry/…/native_array_owner_teardown347` | 30 | 1 | 5 | native_variable_teardown.rs |
| `rust/tcl-registry/…/native_ensemble_holder_routing205` | 30 | 7 | 6 | native_holder_routing.rs, native_command_holder_tests.rs |
| `rust/tcl-registry/…/native_jim_alias_introspection274` | 28 | 5 | 4 | native_jim_inventory_tests.rs, native_jim_inventory_tests.rs |
| `rust/tcl-registry/…/native_oo_original_holder248` | 28 | 5 | 4 | native_original_holder_tests.rs, native_original_holder_tests.rs |
| `rust/tcl-registry/…/native_procedure_usage_extents288` | 28 | 5 | 4 | native_jim_inventory_tests.rs, native_jim_inventory_tests.rs |
| `rust/tcl-registry/…/native_array_member_trace_horizon343` | 21 | 4 | 3 | member_trace_horizon.rs |
| `rust/tcl-registry/…/native_coroutine_original_holder250` | 21 | 4 | 3 | native_original_holder_tests.rs, native_original_holder_tests.rs |
| `rust/tcl-registry/…/native_dictionary_path_publication319` | 21 | 4 | 3 | cmd_dict.rs, cmd_dict.rs |
| `rust/tcl-registry/…/native_jim_info_helper_forwarding258` | 21 | 4 | 3 | native_jim_inventory_tests.rs, native_scripted_distribution.rs, native_jim_inventory_tests.rs |
| `rust/tcl-registry/…/native_precision_unset319` | 21 | 4 | 3 | native_precision.rs, native_precision_tests.rs |
| `rust/tcl-registry/…/native_ensemble_map_prefix312` | 20 | 1 | 5 | ensemble_map_prefix.rs |
| `rust/tcl-vm/…/native_procedure_recompile` | 20 | 2 | 140 | native_procedure.rs |
| `rust/tcl-registry/…/native_array_undefined_member350` | 18 | 1 | 3 | native_variable_teardown.rs |
| `rust/tcl-registry/…/native_array_operational_unset355` | 14 | 3 | 2 | native_variable_teardown.rs, native_unset_tests.rs |
| `rust/tcl-registry/…/native_array_read_destruction324` | 14 | 3 | 2 | cmd_var.rs |
| `rust/tcl-registry/…/native_array_unset_relookup356` | 14 | 3 | 2 | native_variable_teardown.rs, native_unset_tests.rs |
| `rust/tcl-registry/…/native_control_expression` | 14 | 1 | 361 | control_expression_tests.rs, native_literal_pool.rs, native_control_expression_tests.rs |
| `rust/tcl-registry/…/native_info_original_dispatch292` | 14 | 3 | 2 | native_jim_inventory_tests.rs, native_jim_inventory_tests.rs |
| `rust/tcl-registry/…/native_jim_alias_error_extent279` | 14 | 3 | 2 | native_jim_inventory_tests.rs, native_jim_inventory_tests.rs |
| `rust/tcl-registry/…/native_jim_command_table_keys340` | 14 | 3 | 2 | jim_table_key_tests.rs, jim_table_key_tests.rs |
| `rust/tcl-registry/…/native_oo_original_collision249` | 14 | 3 | 2 | native_original_holder_tests.rs, native_original_holder_tests.rs |
| `rust/tcl-registry/…/native_oo_original_copy251` | 14 | 3 | 2 | native_original_holder_tests.rs |
| `rust/tcl-registry/…/native_w216_literal_replacement_value325` | 13 | 2 | 1 | tests.rs |
| `runtime/rust/…/native_object_vector` | 12 | 1 | 120 | native_object_vector.rs |
| `rust/tcl-cmd-core/…/native_each_loop` | 12 | 2 | 526 | native_each_loop_tests.rs, native_each_loop_tests.rs |
| `rust/tcl-registry/…/native_array_frame_access349` | 12 | 1 | 2 | native_variable_teardown.rs |
| `rust/tcl-registry/…/native_array_frame_teardown348` | 12 | 1 | 2 | native_variable_teardown.rs |
| `rust/tcl-registry/…/native_capi_scalar_publication_original` | 12 | 1 | 3 | tests.rs, native_numeric.rs, native_numeric_float_tests.rs |
| `rust/tcl-registry/…/native_upvar_info_exists` | 12 | 1 | 175 | native_upvar_info_exists_tests.rs, native_info_exists_compilation.rs, native_upvar_compilation.rs |
| `rust/tcl-registry/…/native_variable_trace_refill351` | 12 | 1 | 2 | native_variable_teardown.rs |
| `rust/tcl-registry/…/native_error_variables` | 11 | 2 | 95 | native_error_variables.rs, native_object_vector.rs, special_vars.rs |
| `rust/tcl-syntax/…/native_array_search` | 11 | 3 | 1 | cmd_array.rs, native_array_search.rs |
| `rust/tcl-syntax/…/native_list_methods` | 11 | 2 | 214 | value_ops.rs, tests.rs |
| `rust/tcl-syntax/…/native_numeric_operand_conversions` | 11 | 2 | 247 | expr.rs, tests.rs, expr.rs |
| `rust/tcl-registry/…/native_package_ordinary` | 10 | 5 | 4 | cmd_package.rs, cmd_package.rs |
| `rust/tcl-registry/…/native_source_list_length241` | 9 | 4 | 3 | native_source_tests.rs, value_source_tests.rs |
| `rust/tcl-cmd-core/…/native_jim_switch` | 8 | 1 | 1229 | native_jim.rs, switch.rs, native_jim.rs |
| `rust/tcl-vm/…/native_borrowed_frame_slots` | 8 | 2 | 250 | native_compiled_locals.rs |
| `rust/tcl-vm/…/native_dictionary_search` | 8 | 2 | 135 | native_compiled_locals.rs |
