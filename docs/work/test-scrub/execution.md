# Test scrub: execution

Failing tests of `tcl-codegen`, `tcl-vm`, `tcl-runtime`, the engines, `tcl-cshim` and `tcl-spec-hooks` as of 17 November 2026 evening (latest runs: tcl-vm 3,208 tests, runtime/rust 2,726, the selection 9,479), less the 51 routed to dataflow and values and the tests fixed since. One row per test in `ledger/execution.tsv`. Grouped by cause, then the first pass's undecided rows read test by test (part 2): part 3 then wrote one concrete observable per REWRITE group and turned each group into MERGE (a survivor already holds that script and answer), REWRITE (no test holds it yet) or RETIRE (no script or C caller can see the difference). None remain `UNDECIDED`.

## Counts by verdict

| Verdict | Tests |
| --- | ---: |
| `KEEP-ORACLE` | 110 |
| `KEEP-CONTRACT` | 56 |
| `MERGE` | 49 |
| `REWRITE` | 33 |
| `RETIRE` | 8 |
| `FIX-EXPECTATION` | 3 |
| `FIX-FIXTURE` | 3 |
| `KEEP-F5` | 2 |
| total kept here | 264 |
| routed: live-inlining (inliner, `ledger/live-inlining.tsv`) | 23 |

## Status (the ledger's `status` column)

257 open, 5 fixed, 2 repaired, as of 17 November 2026 night:
- Fixed: `errorstack-inner-uplevel-and-fold-options` (8c92829a, the folded syntax options), `bad-option-wording-below-8.6` (ec05d9fc: `encoding` below 8.6 was a product fault, the expectation half right) and the three `optimiser-drops-effectful-foreach-list` rows (fixed upstream).
- Repaired: `delete-proc-runs-in-delete-command` (42cde941) and `interp-option-list` (f4452996).
- Re-read: `switch_exact_return_arms_jump_table` is `KEEP-ORACLE` (`switch-exact-jump-table-declined`), not `FIX-EXPECTATION`. tclsh 8.5-9.1 compile the test's own `switch -exact -- $cmd {...}` to a `jumpTable`; only the form without `--` is invoked. The codegen invokes it.
- `runtime-compile-admission-refused`: every refusal is a tcl-compiler preflight or tcl-registry selection `require_provider()` (clock 18, set/variable command target 10, `string wordstart|wordend` 2, five singles), routed with fixtures.

## Causes ranked by tests blocked

| Cause | Verdict | Tests | Crates | Survivor |
| --- | --- | ---: | --- | --- |
| `native-conformance-vector` | KEEP-ORACLE | 22 | tcl-runtime, tcl-vm |  |
| `runtime-compile-admission-refused` | KEEP-ORACLE | 22 | tcl-engine-tclvm, tcl-runtime, tcl-vm |  |
| `per-release-script-divergence` | KEEP-ORACLE | 20 | tcl-runtime, tcl-vm |  |
| `obs-native-tier-report` | MERGE | 17 | tcl-codegen | tcl-explorer lib serialise::tests::wasm_view_exposes_the_native_tier_lowering_record ; tcl-cli compile_verbs compwasm_codegen_passes_natively_lower_a_hot_loop |
| `obs-redefinition-seen-at-next-command` | MERGE | 17 | tcl-codegen, tcl-runtime, tcl-vm | tcl-vm it_5 command_mutation_deopt_e2e::binding_revalidation::active_proc_frame_redispatches_after_nested_command_mutation |
| `native-compiler-selection-declined` | KEEP-ORACLE | 14 | tcl-codegen |  |
| `obs-wasm-module-answers-as-tclsh` | REWRITE | 12 | tcl-codegen | tcl-codegen it_5 wasm_execute::emitted_modules_run_under_wasmtime |
| `embedding-contract` | KEEP-CONTRACT | 10 | tcl-cshim, tcl-engine-tclvm, tcl-runtime, tcl-vm |  |
| `oracle-corpus-divergence` | KEEP-ORACLE | 10 | tcl-runtime, tcl-vm |  |
| `obs-disassembly-equals-tclsh` | REWRITE | 8 | tcl-codegen |  |
| `c-extension-vectors` | KEEP-ORACLE | 6 | tcl-cshim, tcl-runtime |  |
| `contract-authored-operand-currency` | KEEP-CONTRACT | 6 | tcl-vm |  |
| `obs-result-representation` | REWRITE | 6 | tcl-runtime, tcl-vm | tcl-vm it_3 cross_version_info_surface_e2e::representation_reports_the_value_as_each_release_does |
| `wasm-module-fails-to-instantiate` | KEEP-CONTRACT | 6 | tcl-codegen |  |
| `contract-append-boundary` | KEEP-CONTRACT | 5 | tcl-runtime |  |
| `contract-body-artifact-boundary` | KEEP-CONTRACT | 5 | tcl-runtime |  |
| `live-splicing-off-by-ruling` | KEEP-CONTRACT | 5 | tcl-codegen |  |
| `obs-switch-dispatch` | MERGE | 5 | tcl-codegen | tcl-codegen codegen_depth switch_exact_return_arms_jump_table |
| `analysis-record-no-execution-observable` | RETIRE | 4 | tcl-codegen |  |
| `contract-name-consumer-withdrawal` | KEEP-CONTRACT | 3 | tcl-runtime, tcl-vm |  |
| `contract-scalar-carrier-issuer` | KEEP-CONTRACT | 3 | tcl-engine-tclvm, tcl-runtime |  |
| `fixture-body-identity-lookup` | FIX-FIXTURE | 3 | tcl-codegen |  |
| `fold-survives-unit-rename` | KEEP-ORACLE | 3 | tcl-codegen |  |
| `obs-8.4-direct-evaluation` | MERGE | 3 | tcl-codegen | tcl-vm it_3 literal_index_encoding_e2e::a_literal_index_reads_as_the_release_compiles_it |
| `obs-loop-control-dispatch` | MERGE | 3 | tcl-codegen | tcl-vm it_3 run_script::control_flow::loop_body_redefining_continue_is_honoured |
| `obs-namespace-introspection-opcodes` | REWRITE | 3 | tcl-codegen |  |
| `obs-profile-and-dispatch-mode-answers` | MERGE | 3 | tcl-codegen | tcl-vm it_4 cross_version_command_surface_e2e::profile_mutation::profile_mutation_recompiles_cached_bodies_and_rejects_live_continuations |
| `optimiser-drops-effectful-foreach-list` | KEEP-ORACLE | 3 | tcl-codegen |  |
| `c-api-completion` | KEEP-ORACLE | 2 | tcl-cshim |  |
| `c-api-delete-proc` | KEEP-ORACLE | 2 | tcl-cshim |  |
| `c-api-visibility` | KEEP-CONTRACT | 2 | tcl-cshim |  |
| `engine-value-bridge` | KEEP-CONTRACT | 2 | tcl-engine-tclvm |  |
| `runtime-engine-host-metadata-list-reader` | KEEP-CONTRACT | 2 | tcl-spec-hooks |  |
| `bad-option-wording-below-8.6` | FIX-EXPECTATION | 1 | tcl-runtime |  |
| `boolean-nan-wording-9.0` | KEEP-ORACLE | 1 | tcl-runtime |  |
| `bridge-header-sharing-unobservable` | RETIRE | 1 | tcl-engine-tclvm |  |
| `c-abi-layout` | KEEP-CONTRACT | 1 | tcl-cshim |  |
| `c-api-load` | KEEP-ORACLE | 1 | tcl-cshim |  |
| `c-api-object-lifetime` | KEEP-CONTRACT | 1 | tcl-cshim |  |
| `clock-read-compilation` | KEEP-ORACLE | 1 | tcl-vm |  |
| `contract-child-completion-transport` | KEEP-CONTRACT | 1 | tcl-runtime |  |
| `contract-dictionary-read-refusal-order` | KEEP-CONTRACT | 1 | tcl-runtime |  |
| `contract-namespace-cleanup-retention` | KEEP-CONTRACT | 1 | tcl-runtime |  |
| `contract-stock-installation-refusal` | KEEP-CONTRACT | 1 | tcl-runtime |  |
| `delete-proc-runs-in-delete-command` | FIX-EXPECTATION | 1 | tcl-runtime |  |
| `errorstack-inner-uplevel-and-fold-options` | KEEP-ORACLE | 1 | tcl-vm |  |
| `f5-iapps-evaluation-point` | KEEP-F5 | 1 | tcl-spec-hooks |  |
| `f5-matches-operator` | KEEP-F5 | 1 | tcl-runtime |  |
| `foreign-issuer-unreachable` | RETIRE | 1 | tcl-cshim |  |
| `interp-option-list` | FIX-EXPECTATION | 1 | tcl-runtime |  |
| `jim-module-purpose-premise` | REWRITE | 1 | tcl-codegen |  |
| `obs-8.4-expr-syntax-errorcode` | REWRITE | 1 | tcl-runtime |  |
| `obs-c-index-after-retirement` | REWRITE | 1 | tcl-cshim |  |
| `obs-errorinfo-command-extent` | MERGE | 1 | tcl-codegen | tcl-vm it_3 c_error_paths_e2e::procedure_frame_names_the_invoking_word |
| `obs-namespace-value-resolves-by-name` | REWRITE | 1 | tcl-engine-tclvm |  |
| `representation-command-before-8.6` | KEEP-ORACLE | 1 | tcl-runtime |  |
| `self-referential-golden` | RETIRE | 1 | tcl-codegen |  |
| `switch-return-arms-invoked` | FIX-EXPECTATION | 1 | tcl-codegen |  |
| `wasm-framing-budget` | KEEP-CONTRACT | 1 | tcl-codegen |  |
| `withdrawn-world-unreachable` | RETIRE | 1 | tcl-vm |  |

## Distinct failing behaviours (what is worth fixing)

Each line is one behaviour tclsh, Jim or a contract shows that the product gets wrong today, with the tests it blocks.

1. **Run-time compile under the VM's own entry refuses** (`runtime-compile-admission-refused`, 21 tests). Scripts tclsh runs fail before execution with `NativePreflightRequired`: the compile service cannot settle a native compiler selection under the live entry. Known roots: `upvar` with a level other than 0/1 (fixed in cc8649e8), the `clock` ensemble's uncompiled subcommands (with analyser-binding), `{*}` expansion and trace gates under pinned profiles. Many vectors suites (`vm_matches_the_pinned_*`, `regexp_cmd_corpus`, `cross_version_command_surface_e2e`) are blocked whole by one refusal each.
2. **Compiled instruction selection declined** (`native-compiler-selection-declined`, 13). `foreach`, `lindex`, `info exists`, three-word `catch` and `string cat` emit `invokeStk` where tclsh 8.6/9.0 emit `foreach_start`, `listIndexImm`/`lindexMulti`, `existScalar`/`existStk`, `pushReturnOpts` and a folded `push1`.
3. **Native conformance vectors** (`native-conformance-vector`, `per-release-script-divergence`, `oracle-corpus-divergence`, about 45). Scripts whose tclsh/jimsh answers the tests carry: Jim binary/string/expr rows answer `invalid command name`, trace and observer orders differ on named releases, `lsort` comparator errorInfo on 8.4, literal index encoding per release.
4. **WASM**: modules that fail to instantiate under wasmtime (`wasm-module-fails-to-instantiate`, 6); the native tier's records (`wasm-native-tier-plan-record`, 29, REWRITE onto the emitted module).
5. **Live splicing** is off by ruling; 5 claim tests return with the SpliceRange exit.
6. Single behaviours: `representation` exists before 8.6 (runtime), `if {NaN}` wording on 9.0 (runtime), errorstack INNER for an uplevel'd invoke and the constant-fold options (vm), C-extension vectors (pkga/doors: errorCode, NUL bytes, leaks), the optimiser dropping an effectful `foreach` list (dataflow), fold invalidation after `rename list`/`proc format`/`rename dict`.

## Where the verdict says the expectation, not the product, is wrong

`FIX-EXPECTATION` (3): `binary d`/`encoding d` below 8.6 say `bad option`; `Tcl_DeleteCommandFromToken` runs the delete proc before returning; `interp bogus` lists cancel/share/transfer on 9.0. The last three are runtime/rust tests whose corrections I hold locally (applied after the coordinator samples this ledger, per the rules).

## Open questions for the coordinator

- `inliner-unshipped` (23): routed to live-inlining by the coordinator's ruling (read per test: answers as the original → KEEP-ORACLE; inliner shape → REWRITE to the differential or MERGE into a `live_differential` row; visible `__inline_N__` names → FIX-EXPECTATION).
- Part 2 read the native-artefact rows (superseded in part, see the groups below): C-API and extension behaviours are KEEP-ORACLE or KEEP-CONTRACT (`c-api-*`, `engine-value-bridge`, `c-extension-vectors`); the rest assert a refusal or ordering point through receipts or pointers and are REWRITE (`native-artefact-ordering`): the behaviour is real, the observable is the script result and the absence of later effects. Still undecided: `callback_namespace_primary_retains_private_scope_and_descriptor` (engine), `measured_bare_matches_runtime_requires_independent_authored_policy` (runtime), `unrepresented_native_byte_literal_is_a_host_refusal` (vm); the two `families_e2e::*::runtime` pack-family rows are KEEP-CONTRACT (G16 paragraph).
- `codegen-internal-record` (37): REWRITE, each re-pointed at tclsh disassembly or the replay/deopt behaviour it guards.

## Observables by group (part 3)

Each REWRITE, MERGE and RETIRE row names its group; the group states the script (or C-API sequence) and the answer tclsh or Jim gives, and the survivor where one already holds it.

- **G1** `obs-redefinition-seen-at-next-command` → MERGE. proc p1 {} {set r [list a]; proc ::list args {return NEW}; lappend r [list b]; set r}; p1 -> `a NEW` (tclsh 8.6.18/9.0.4); the boundary/binding records exist only to give this answer Survivor: `tcl-vm it_5 command_mutation_deopt_e2e::binding_revalidation::active_proc_frame_redispatches_after_nested_command_mutation`.
- **G2** `obs-loop-control-dispatch` → MERGE. proc break {} {return NORMAL}; set r {}; foreach i {1 2} {lappend r $i; break}; set r -> `1 2` (tclsh 8.6.18/9.0.4); an unshadowed break ends the loop Survivor: `tcl-vm it_3 run_script::control_flow::loop_body_redefining_continue_is_honoured`.
- **G3** `obs-profile-and-dispatch-mode-answers` → MERGE. the same script answers the same in fast and plain dispatch and after a profile switch (the survivor runs 8.5->8.6 switches on cached bodies) Survivor: `tcl-vm it_4 cross_version_command_surface_e2e::profile_mutation::profile_mutation_recompiles_cached_bodies_and_rejects_live_continuations`.
- **G4** `obs-8.4-direct-evaluation` → MERGE. tclsh 8.4.20 evaluates an eval body directly: `eval {catch {set x 1} r o}` reports catch's usage error `while executing` ("eval" body line 1), the same command compiled (in a catch body or a proc) `while compiling`; 8.5+ accept the three-word catch (survivor holds these rows) Survivor: `tcl-vm it_3 literal_index_encoding_e2e::a_literal_index_reads_as_the_release_compiles_it`.
- **G5** `obs-errorinfo-command-extent` → MERGE. errorInfo names the failing command's exact text (`while executing "..."`), tclsh 8.4-9.1 rows in the survivor Survivor: `tcl-vm it_3 c_error_paths_e2e::procedure_frame_names_the_invoking_word`.
- **G6** `obs-namespace-introspection-opcodes` → REWRITE. tclsh 8.6.18/9.0.4 disassemble proc {namespace origin set} -> push1 originCmd; {namespace current} -> currentNamespace; {namespace code x} -> push1 push1 currentNamespace push1 list: re-point at disassembly equality
- **G7** `obs-disassembly-equals-tclsh` → REWRITE. the proc's disassembly equals tclsh 8.6/9.0's for the test's own source (bytecode-compare); e.g. a proc body and the same text at top level differ only by loadScalar vs loadStk as in tclsh
- **G8** `obs-switch-dispatch` → MERGE. tclsh 8.6.18/9.0.4 invoke switch with returning arms (proc {switch -glob $x {a* {return 1} default {return 2}}} -> invokeStk1); the IR shape has no other consumer Survivor: `tcl-codegen codegen_depth switch_exact_return_arms_jump_table`.
- **G9** `analysis-record-no-execution-observable` → RETIRE. compilation-unit callee records and Local/Frame value tags change no execution answer or emitted instruction; the analysis that consumes them is tested in tcl-compiler (analyser-binding/values)
- **G10** `obs-wasm-module-answers-as-tclsh` → REWRITE. the emitted module run under wasmtime answers as tclsh: proc set args {return REBOUND} then set x 1 -> REBOUND; set a 1; set b 1; if "$a == $b" {puts x} -> x (tclsh 8.6.18/9.0.4); re-point at wasm_execute rows Survivor: `tcl-codegen it_5 wasm_execute::emitted_modules_run_under_wasmtime`.
- **G11** `obs-native-tier-report` → MERGE. the native tier's plan/report is visible only as the explorer's lowering record and as emitted i64.add; both survivors hold it; no script answer depends on the premises Survivor: `tcl-explorer lib serialise::tests::wasm_view_exposes_the_native_tier_lowering_record ; tcl-cli compile_verbs compwasm_codegen_passes_natively_lower_a_hot_loop`.
- **G12** `self-referential-golden` → RETIRE. the golden is this compiler's earlier output; any answer change is caught by the KEEP-ORACLE opcode rows against tclsh disassembly; the corpus pins no external fact
- **G13** `obs-8.4-expr-syntax-errorcode` → REWRITE. tclsh 8.4.20 catch {expr {1 +}} -> syntax error in expression "1 +": premature end of expression, errorCode NONE
- **G14** `obs-c-index-after-retirement` → REWRITE. C caller: Tcl_GetIndexFromObj(table) answers the same index after the command owning the table is deleted (C API)
- **G15** `obs-result-representation` → REWRITE. tclsh 8.6.18 tcl::unsupported::representation [linsert [list a b] end X] -> value is a list ... no string representation (9.0: same); re-point the windows at representation rows; vm and runtime copies fold into one shared row Survivor: `tcl-vm it_3 cross_version_info_surface_e2e::representation_reports_the_value_as_each_release_does`.
- **G16** (redone test by test after the coordinator rejected one blanket RETIRE for 31 rows). 26 cite a naming key (`naming.embedding.original-host-publication-and-fact-transport`, `naming.error.nonempty-info-publication-order` or `naming.interpreter.original-child-host-refusal-transport`) and are KEEP-CONTRACT; each row's evidence carries the key and the assertion it fails on today, and the cause names the subsystem (`contract-append-boundary` 4, `contract-body-artifact-boundary` 6, `contract-authored-operand-currency` 6, `contract-name-consumer-withdrawal` 3, `contract-scalar-carrier-issuer` 3, one each for namespace cleanup, child completion transport, dictionary read order, stock installation). The 5 uncited rows: `callback_namespace_primary_retains_private_scope_and_descriptor` REWRITE to `namespace eval ::a {set ::n [namespace current]}; namespace delete ::a; namespace eval ::a {proc here {} {return HERE}}; namespace eval $::n {here}` -> `HERE` (tclsh 8.6.18/9.0.4); `publication_revision_tracks_moves_and_callback_rebound_rename_sources` MERGE onto G1's survivor (`proc a {} {return A}; proc call {} {a}; call; rename a b; proc a {} {return NEW}; call` -> `NEW`); RETIRE `zero_argument_callback_refuses_a_foreign_native_issuer_before_native_execution` (issuers are not in the public C API, so no extension can hand over a foreign one), `retained_list_bridge_keeps_actual_storage_and_native_header_sharing` (the engine API is value-based: shared header and copy read the same elements, no identity accessor) and `jim_info_current_namespace_getter_stops_at_the_withdrawn_consumer_world` (only the VM's own operation currency withdraws that world; Jim `namespace current` answers by name). The two `families_e2e::*::runtime` pack-family rows came back from irules-registry-tooling as KEEP-CONTRACT (`runtime-engine-host-metadata-list-reader`: RuntimeEngine leaves `Engine::host_metadata_list_reader` at None).

## tcl-vm and tcl-runtime: both copies

The same behaviour is held twice where the two engines are both product (the VM and the standalone runtime): `tclsh_parity`, `literal_index_encoding`, `clock_remains_callable_in_a_safe_child`, the info-commands 186-window artefact, `set_release_pins_the_numeral_grammar` (engine_cases shared by both). Two engines, two results: both copies earn their place where the script is the same and each engine can fail on its own. The artefact pair (186 windows) folds into one shared conformance row once rewritten onto its script observable.

## Exact-copy and overlap groups touching execution (passing tests included)

| Kind | Group | Size | Failing | Crates | Members (first four) |
| --- | --- | ---: | ---: | --- | --- |
| exact | `fa61f58ca6cd` | 3 | 0 | tcl-vm | tcl-vm::element_recovery_follows_the_selected_release ; tcl-vm::selected_runtime_surface_matches_the_cross_version_vectors ; tcl-vm::legacy_variable_trace_forms_follow_the_selected_release |
| exact | `03b969583e6b` | 2 | 0 | tcl-bytecode,tcl-codegen | tcl-bytecode::esc_null ; tcl-codegen::esc_nul_escaped_as_u_xxxx |
| exact | `0ee91035e6bf` | 2 | 0 | tcl-bytecode,tcl-codegen | tcl-bytecode::esc_quotes ; tcl-codegen::esc_quote_escaped |
| exact | `377e62d57dc5` | 2 | 0 | tcl-vm | tcl-vm::vectors_match_real_tclsh ; tcl-vm::vectors_match_real_tclsh |
| exact | `55205d9f9e85` | 2 | 2 | tcl-runtime,tcl-vm | tcl-runtime::original_range_and_assignment_match_84_native_completion_and_original_header_pairs ; tcl-vm::original_range_and_assignment_match_84_native_completion_and_original_header_pairs |
| exact | `5e5dfa14bc9e` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::original_upvar_and_exists_artifact_preserves_140_native_completions_and_20_name_headers ; tcl-vm::original_upvar_and_exists_preserve_140_native_completions_and_20_name_headers |
| exact | `5f7a7441bf0a` | 2 | 0 | tcl-vm | tcl-vm::info_complete_cases ; tcl-vm::info_complete_shared_core |
| exact | `6a1527cf0e06` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::the_vectors_match_each_tclsh ; tcl-vm::the_vectors_match_each_tclsh |
| exact | `7a325e48fe1f` | 2 | 2 | tcl-runtime,tcl-vm | tcl-runtime::generic_each_loops_match_all_406_original_native_physical_windows ; tcl-vm::generic_each_loops_match_all_406_original_native_physical_windows |
| exact | `8d95ae64a8e7` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::original_c9_insertion_matches_fourteen_native_result_and_original_header_pairs ; tcl-vm::original_c9_insertion_matches_fourteen_native_result_and_original_header_pairs |
| exact | `9ef7578fb736` | 2 | 0 | tcl-codegen,tcl-compiler | tcl-codegen::auto_path_nested_and_double_dirname ; tcl-compiler::nested_dirname |
| exact | `a5d0280d96e0` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::generic_each_loops_match_all_120_original_native_completions ; tcl-vm::generic_each_loops_match_all_120_original_native_completions |
| exact | `a9820cf15b32` | 2 | 0 | tcl-codegen,tcl-compiler | tcl-codegen::auto_path_unsupported_command_declines ; tcl-compiler::unsupported_command_is_none |
| exact | `b66239992f90` | 2 | 0 | tcl-bytecode,tcl-codegen | tcl-bytecode::esc_astral_is_byte_wise ; tcl-codegen::esc_supplementary_plane_is_bytewise_four_bytes |
| exact | `b969f09f5902` | 2 | 0 | tcl-codegen,tcl-compiler | tcl-codegen::auto_path_variable_subst_declines ; tcl-compiler::variable_substitution_is_unsupported |
| exact | `c19f6b7a102f` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::original_exists_quiet_trace_semantics_preserve_15_native_result_windows ; tcl-vm::original_exists_quiet_trace_semantics_preserve_15_native_result_windows |
| exact | `cc261c9785e2` | 2 | 2 | tcl-vm | tcl-vm::vm_matches_the_pinned_trace_vectors ; tcl-vm::vm_matches_the_pinned_step_trace_vectors |
| exact | `e605cb36ad10` | 2 | 0 | tcl-codegen | tcl-codegen::arity_mismatch_declines ; tcl-codegen::missing_arg_no_default_declines |
| exact | `ef2c0aae72eb` | 2 | 2 | tcl-vm | tcl-vm::workers_inherit_a_separate_host_command_surface ; tcl-vm::workers_inherit_a_separate_host_command_surface_in_release |
| exact | `ffd8b8c4ce98` | 2 | 0 | tcl-runtime | tcl-runtime::subst_reports_the_missing_bracket_after_running_the_earlier_one ; tcl-runtime::subst_is_not_a_command_parse_and_still_runs_the_earlier_bracket |
| overl | `43686a827d3b` | 6 | 0 | tcl-vm | tcl-vm::compiled_braced_var_close_rule_matches_real_tclsh ; tcl-vm::compiled_interpolated_and_switch_paths_match_real_tclsh ; tcl-vm::compiled_array_index_braced_var_matches_real_tclsh ; tcl-vm::compiled_backslash_braced_var_names_match_real_tclsh |
| overl | `b15beda2322f` | 6 | 0 | tcl-codegen | tcl-codegen::lassign_rejects_wrong_arity ; tcl-codegen::lassign_emits_expected_sequence ; tcl-codegen::llength_single_arg ; tcl-codegen::lrange_constant_indices |
| overl | `15f1260ac131` | 5 | 0 | tcl-codegen | tcl-codegen::dict_lappend_uses_dict_lappend_opcode ; tcl-codegen::lappend_scalar_single_uses_lappend_scalar1 ; tcl-codegen::lappend_multi_builds_list_then_lappend_list ; tcl-codegen::lappend_array_element_uses_lappend_array1 |
| overl | `59aa6941d776` | 4 | 4 | tcl-vm | tcl-vm::upstream_var_search_mutation_handlers_receive_errorinfo ; tcl-vm::upstream_error_shifted_context_definitions_pass_after_real_startup ; tcl-vm::upstream_error_stack_reset_preserves_shifted_context ; tcl-vm::upstream_invalid_error_stack_preserves_shifted_context |
| overl | `35b36b222704` | 3 | 0 | tcl-codegen | tcl-codegen::format_header ; tcl-codegen::format_instruction_offsets ; tcl-codegen::format_block_labels |
| overl | `6da95fa129dc` | 3 | 0 | tcl-codegen | tcl-codegen::an_emitted_module_carries_its_identity_manifest ; tcl-codegen::default_compilation_keeps_legacy_analysis_specialisations_disabled ; tcl-codegen::analysis_specialisation_requires_its_explicit_pass |
| overl | `e6b9d7494085` | 3 | 0 | tcl-vm | tcl-vm::prefix_all_bad_list_errors ; tcl-vm::prefix_match_error_option ; tcl-vm::prefix_longest_bad_list_errors |
| overl | `03f5f701f8ef` | 2 | 0 | tcl-vm | tcl-vm::named_command_surface_override_cannot_hide_compiled_release_commands ; tcl-vm::command_surface_override_rejects_a_same_release_narrower_profile |
| overl | `07553265f2ce` | 2 | 0 | tcl-vm | tcl-vm::jim_import_ignores_exports_overwrites_and_retains_missing_source_alias ; tcl-vm::jim_interpreter_handle_isolation |
| overl | `0c1fe6a72f1a` | 2 | 0 | tcl-runtime | tcl-runtime::guarded_intrinsic_string_length_hits_and_releases_exact_completion_and_token ; tcl-runtime::guarded_intrinsic_jim_character_identity_ignores_the_c_assistance_release |
| overl | `10bd7981d738` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::the_145th_draw_after_srand_1_matches_the_oracle ; tcl-vm::the_145th_draw_after_srand_1_matches_the_oracle |
| overl | `134752325fc3` | 2 | 0 | tcl-vm | tcl-vm::subst_braced_var_close_rule_follows_the_emulated_release ; tcl-vm::compiled_array_index_braced_var_follows_the_emulated_release |
| overl | `203b84c3c85d` | 2 | 0 | tcl-codegen | tcl-codegen::try_inline_list_without_target_emits_break_as_literal ; tcl-codegen::try_inline_list_with_break_emits_jump_to_target |
| overl | `259022099104` | 2 | 0 | tcl-vm | tcl-vm::info_complete_cases ; tcl-vm::info_complete_shared_core |
| overl | `2d2126df0fae` | 2 | 0 | tcl-engine-api | tcl-engine-api::a_door_without_the_extra_doors_declines_each_of_them ; tcl-engine-api::an_engine_without_the_extra_doors_declines_each_of_them |
| overl | `2ec75ca64e3d` | 2 | 0 | tcl-vm | tcl-vm::radix_invalid_numerals_are_barewords ; tcl-vm::bad_hex_numerals_are_barewords_without_a_hint |
| overl | `3e6899b2bada` | 2 | 0 | tcl-runtime | tcl-runtime::rejected_native_formal_binding_acquires_no_execution_reference ; tcl-runtime::c_interpreter_shutdown_retires_visible_and_hidden_bindings_despite_query_views |
| overl | `413541bd4210` | 2 | 0 | tcl-vm | tcl-vm::variable_read_diagnostics_match_five_original_source_controls ; tcl-vm::variable_read_input_preserves_current_jim_dictionary_results |
| overl | `4a8620619f4e` | 2 | 0 | tcl-codegen,tcl-compiler | tcl-codegen::auto_path_variable_subst_declines ; tcl-compiler::variable_substitution_is_unsupported |
| overl | `5180e70d092a` | 2 | 0 | tcl-codegen,tcl-compiler | tcl-codegen::auto_path_nested_and_double_dirname ; tcl-compiler::nested_dirname |
| overl | `540abc448f51` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::get_missing_key_errors ; tcl-vm::dict_get_missing_key_errors |
| overl | `585e3dced97a` | 2 | 0 | tcl-codegen | tcl-codegen::arity_mismatch_declines ; tcl-codegen::missing_arg_no_default_declines |
| overl | `58e1022f1db4` | 2 | 0 | tcl-codegen,tcl-compiler | tcl-codegen::auto_path_normpath_collapses_dotdot_and_dot ; tcl-compiler::parent_dir_join_idiom_collapses_dotdot |
| overl | `5a28c3ef35ed` | 2 | 0 | tcl-runtime | tcl-runtime::namespace_teardown_follows_command_table_hash_order ; tcl-runtime::namespace_teardown_hash_order_records_table_growth |
| overl | `5bf7b6015049` | 2 | 0 | tcl-runtime | tcl-runtime::subst_reports_the_missing_bracket_after_running_the_earlier_one ; tcl-runtime::subst_is_not_a_command_parse_and_still_runs_the_earlier_bracket |
| overl | `5e61b9aa9761` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::original_local_cache_getter_order_matches_all_25_native_paths ; tcl-vm::original_local_cache_getter_order_matches_all_25_native_paths |
| overl | `5ec0ed070e1e` | 2 | 0 | tcl-codegen | tcl-codegen::the_unseen_call_marker_keeps_the_next_command_boundary ; tcl-codegen::the_unseen_call_marker_under_start_cmd_does_not_advance_command_index |
| overl | `5fc7298f746b` | 2 | 0 | tcl-vm | tcl-vm::list_assignment_arity_matches_five_original_source_controls ; tcl-vm::list_assignment_arity_matches_current_jim_original_source_controls |
| overl | `68e7177b2270` | 2 | 0 | tcl-vm | tcl-vm::logical_eval_policy_withdrawal_prevents_fresh_word_execution ; tcl-vm::authored_uplevel_receipt_cannot_enter_after_its_invocation_policy_changes |
| overl | `6a311c39b9e1` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::compiled_and_dynamic_declared_variables_match_original_native_controls ; tcl-vm::compiled_and_dynamic_declared_variables_match_original_native_controls |
| overl | `6de086fcdb42` | 2 | 2 | tcl-vm | tcl-vm::vectors_match_real_tclsh_when_available ; tcl-vm::vectors_match_real_tclsh |
| overl | `7732f23a17ae` | 2 | 0 | tcl-codegen,tcl-compiler | tcl-codegen::place_whole_array_read_overlaps_every_element ; tcl-compiler::var_read_role_arg_resolves_whole_array |
| overl | `7dcb312e62d3` | 2 | 0 | tcl-runtime | tcl-runtime::array_root_write_diagnostics_match_five_original_source_controls ; tcl-runtime::array_root_write_preserves_current_jim_dictionary_replacement |
| overl | `915a18b5a34d` | 2 | 0 | tcl-vm | tcl-vm::array_root_write_diagnostics_match_five_original_source_controls ; tcl-vm::array_root_write_preserves_current_jim_dictionary_replacement |
| overl | `955c5b15287b` | 2 | 0 | tcl-runtime | tcl-runtime::variable_read_diagnostics_match_five_original_source_controls ; tcl-runtime::variable_read_input_preserves_current_jim_dictionary_results |
| overl | `a0c3e4d5265b` | 2 | 0 | tcl-runtime | tcl-runtime::list_assignment_arity_matches_five_original_source_controls ; tcl-runtime::list_assignment_arity_matches_current_jim_original_source_controls |
| overl | `ba57eacae515` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::lsearch_shared_core ; tcl-vm::lsearch_shared_core |
| overl | `bc12287e1901` | 2 | 0 | tcl-codegen,tcl-compiler | tcl-codegen::auto_path_unsupported_command_declines ; tcl-compiler::unsupported_command_is_none |
| overl | `ca34681bbf41` | 2 | 0 | tcl-runtime | tcl-runtime::ensemble_default_dispatches_to_namespace_commands ; tcl-runtime::ensemble_prefix_match_and_ambiguity |
| overl | `cedf726c9391` | 2 | 0 | tcl-codegen | tcl-codegen::foreach_emits_native_opcodes ; tcl-codegen::complex_foreach_body_emits_step_at_end |
| overl | `d2026d60b4e0` | 2 | 0 | tcl-vm | tcl-vm::format_pointer_width_modifiers_are_lp64_issue_2163 ; tcl-vm::format_pre_tcl9_long_width_and_unknown_modifiers_issue_2163 |
| overl | `d5adfdce6feb` | 2 | 0 | tcl-runtime | tcl-runtime::checked_original_duplicate_retains_first_cause_before_cleanup_and_refuses_incoming_admission ; tcl-runtime::checked_original_duplicate_transfers_exact_incoming_primary_and_cleanup_once |
| overl | `d73bc607c4eb` | 2 | 0 | tcl-codegen | tcl-codegen::global_multi_reuses_namespace ; tcl-codegen::global_qualified_or_toplevel_falls_back |
| overl | `dbd7e19b848a` | 2 | 0 | tcl-vm | tcl-vm::original_oo_name_declaring_facet_comes_from_the_genuine_entered_method_chain ; tcl-vm::original_oo_name_present_method_requires_its_actual_declaring_step |
| overl | `dd14b3a657ae` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::original_coroutine_probe_and_injection_match_counted_object_controls ; tcl-vm::original_coroutine_probe_and_injection_match_counted_object_controls |
| overl | `dd59e883da6d` | 2 | 0 | tcl-vm | tcl-vm::unset_array_stk_complain_flag_errors ; tcl-vm::unset_array_complain_flag_errors |
| overl | `e143c9d90904` | 2 | 0 | tcl-runtime,tcl-vm | tcl-runtime::original_static_member_objects_match_native_counted_names_and_value_capture ; tcl-vm::original_static_member_objects_match_native_counted_names_and_value_capture |
| overl | `e5840a4e16ad` | 2 | 0 | tcl-runtime | tcl-runtime::expression_arguments_use_native_concatenation_and_selected_cardinality ; tcl-runtime::captured_increment_receivers_match_actual_native_releases |
| overl | `e651c9e823d8` | 2 | 0 | tcl-codegen | tcl-codegen::hosted_literal_invocation_selects_executable_generic_argv_plan ; tcl-codegen::invalid_semantic_layout_declines_before_the_single_emitter_runs |
| overl | `edc49db10516` | 2 | 0 | tcl-codegen | tcl-codegen::catch_inline_emits_begin_end_catch ; tcl-codegen::catch_inline_stores_result_var |
