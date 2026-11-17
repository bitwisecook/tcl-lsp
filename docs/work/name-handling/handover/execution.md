# Hand-over: execution (codegen, VM and runtimes), 17 November 2026

Written from a paused session. Its last code commit on `work` is `43c8d0b7` (16 November); its last commit of any kind is `f92ac0bb` (docs). Nothing was held locally at the pause: no local commits, stashes, uncommitted edits or side branches. The longer record, item by item, is the section "Execution area, 15 November 2026" of `docs/work/name-handling/current-state.md`; this file is the short form a successor starts from. All paths below are on the restructured tree (`da039ed1`), found with `grep -rn 'fn <name>'`; line numbers in `current-state.md` predate the file split.

## Scope

Owned: `rust/tcl-codegen`, `rust/tcl-vm`, `rust/tcl-vm-cli`, `rust/tcl-engine-api`, `rust/tcl-engine-tclvm`, `rust/tcl-engine-wasm`, `rust/tcl-cmd-core`, `rust/tcl-cshim`, `rust/tcl-bytecode`, `rust/tcl-runtime-api`, `rust/tcl-spec-hooks`, and the standalone `runtime/rust`.

Also worked, because the cause was in this area: the `tcl-spectcl` integration tests `it_5 codegen_stamps` (pack alias stamps and reference-body inlining live in `tcl-codegen`).

Not owned: `rust/tcl-compiler` (lowering, binding walk, common AOT proof, native byte compilation) and `rust/tcl-registry`. A cause there gets the smallest fix as a separate commit, or a record with cause and owner. Small registry commits were made earlier in the burn-down (prefix, clock readings); nothing in `tcl-compiler` was changed in the last stretch.

F5 profiles are out of scope for the WASM native tier (owner's rule: F5 dialects are for F5 only).

## How to measure

One cargo build at a time on this machine, with `ulimit -v 14000000`. The per-session disk allowance is about 39 GB; incremental builds overflow it, so `CARGO_INCREMENTAL=0`.

Environment (the session kept this in a scratch `env.sh`):

```bash
cd /home/user/tcl-lsp
source scripts/dev/agent-build-env.sh
export CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 RUST_BACKTRACE=0
export TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh
export TCL_LSP_REQUIRE_TCLSH=1
export CARGO_INCREMENTAL=0
export PKGS="-p tcl-codegen -p tcl-vm -p tcl-vm-cli -p tcl-engine-api -p tcl-engine-tclvm -p tcl-cmd-core -p tcl-cshim -p tcl-bytecode -p tcl-runtime-api -p tcl-spec-hooks"
```

The selection run and the comparison:

```bash
(ulimit -v 14000000; cargo nextest run $PKGS --all-features --no-fail-fast \
   --status-level fail --final-status-level none --failure-output never > run.log 2>&1)
grep -E "^error" -A6 run.log | head -30        # a build error makes every test "fixed": look first
grep Summary run.log
grep -E "^\s+(FAIL|TIMEOUT|SIGABRT|SIGSEGV|ABORT)" run.log \
  | sed -E 's/^\s+\S+ \[[^]]*\] *(\([^)]*\))? *//' | sort -u > now.fail
comm -23 base.fail now.fail   # fixed
comm -13 base.fail now.fail   # new: must be empty before a push
```

A clean build of the selection takes 10 to 15 minutes; the tests themselves about 4.5 minutes (5,482 tests). Since the file split a test's path gained a module, so compare against the committed list by the last path segment, or take a fresh baseline on the restructured tree first and compare run to run.

`tcl-spectcl` (not in `PKGS`): `cargo nextest run -p tcl-spectcl --all-features --test it_5 --test it_3 --test it_2 --test it_4 -E 'not test(every_shape)'`. The excluded test, `codegen_stamps::every_shape_of_inlined_body_answers_as_its_definition_does_at_every_site`, takes about 14 minutes and was failing at the last full run of it.

`runtime/rust` is a separate workspace and was never run by this session (60 failing at the start of the burn-down, by the earlier record).

Diagnostics that paid off: `TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC=1` (prints `NATIVE_CODEGEN_ADMISSION argv=… selection=…` for the command that refuses a unit) and `TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC=first-decline-v1` (`CCG_…` lines with the refusing call site).

Gates before every push: `cargo fmt --all -- --check`; `cargo clippy -p <changed crate> --no-deps --all-features --tests -- -D warnings -A clippy::assert_is_empty` (without `--no-deps` a lint in another area's crate fails the run); `cargo xtask registry-axes`, `value-transfers --check`, `owner-resolution`, `callback-inventory --check`, `command-backing --check`, and now `file-sizes --check`.

## Current numbers

Last full run of the selection: 16 November 2026, on the tree that became `43c8d0b7`: 318 failing of 5,482. Two later adjustments were made by targeted re-runs, not a full run: `70460540` (tk-oo) fixed six, and the same commit broke `tcl-codegen::it_2 codegen::catch_in_while_condition`. That gives 313, which is the committed list. By crate at the full run: `tcl-codegen` 189, `tcl-vm` 102, `tcl-cshim` 15, `tcl-engine-tclvm` 8, `tcl-spec-hooks` 3, `tcl-vm-cli` 1.

Not included in that number: the dataflow session's structured-control change (landed after my last run; measured as a patch it fixed `compile_service::tests::cfg_edges_keep_ranges_but_are_not_runtime_recompile_sites` and three `codegen_stamps` tests, new 0), and anything the restructuring moved. The coordinator reports the VM library suite at 93 failing before and after the split.

`tcl-spectcl` `it_2`..`it_5` without `every_shape`: 21 failing of 220 at `43c8d0b7`; `codegen_stamps` 8 failing of 29 (5 after the structured-control change, by the patch measurement).

## Baseline list

`docs/work/name-handling/lists/execution-failing-2026-11-16.txt`: 313 lines, one test per line, in nextest's `crate[::binary] path::to::test` form, pre-split paths.

## Queue

In the order I would take it.

1. **Take a fresh baseline on the restructured tree.** Everything below assumes it.

2. **Reference bodies through the VM's own compile (3 `codegen_stamps` tests: `a_tcl_body_backed_command_is_inlined_and_admitted` at its last assertion, `a_host_native_backing_never_defines_a_proc`, `a_pack_text_body_that_diverges_turns_the_site_plain`).** The release-unit path works; the live-entry path does not inline. Three gaps: the byte-ingress procedure compile (`compile_procedure_bytes_target_with` in `rust/tcl-codegen/src/compile_service.rs`) never calls `inlining::inline_reference_bodies`; a live module has no catalogue metadata, so `called_descriptors` in `rust/tcl-codegen/src/inlining/reference.rs` finds no site (the source binding's `proved_execution_target()` names the command and worked in an experiment); and statements spliced from the simulated lowering carry no live compiler selection, so the unit is refused (`NativePreflightRequired`). Lowering `proc name …` under the live entry yields no procedure, because the body is not entered at definition time. The body has to be lowered as a procedure target under the live entry (`lower_procedure_target_module_for_bytecode_with_options`) and rebased, with eligibility still proved in the simulated module (`reference_inline_spec` in `inlining/mod.rs`). Fixture: `proc caller {n} {vdouble $n}` with the pack `proc vdouble {x} {expr {$x * 2}}`. An experiment was withdrawn; nothing of it is on `work`.

3. **Child interpreter deleted during its own evaluation (2 tests: `namespace_surface_e2e::command_trace_deleted_child_recreation_keeps_generations_distinct`, `…command_trace_retains_active_interpreter_during_callback_deletion`).** The result is refused with `retired native object header`. `in_interp_using` (`rust/tcl-vm/src/interp/interpreters.rs`) finalises a dying child before returning the closure's value, so a completion produced in the child is read after its objects are retired. Not confirmed which value is retired; the likely fix is to detach the completion's result to bytes inside the closure in `eval_values_in_interp_in_original_name` (`interp/interpreters/children.rs`) when the slot is dying and rebuild it in the caller, as `native_error_variables` already does for a dead error-result receipt. Smallest script to try: `interp create i; proc retire args {interp delete i}; interp alias i retire {} retire; list [catch {interp eval i {retire; set x 5}} m] $m`.

4. **`command_mutation_deopt_e2e` (8 open).** One inliner test (`active_inlined_namespaced_boundary_replays_in_its_defining_namespace`: bodies not spliced, the fixture has variable traces and procedures that define procedures); three manifest and pack tests report one more plain recompile than expected; `active_proc_frame_revalidates_a_cfg_consumed_eval_head` records no `eval` binding; `argument_substitution_mutation_keeps_entered_token…`, `escaped_head_stays_on_live_dispatch_after_replacement`, `fused_expr_keeps_its_entered_token_when_an_operand_mutates_expr`. None followed past the first assertion.

5. **`tcl-vm` library `interp::` (13, each its own cause).** One followed: `execution_name_policy::tests::counted_array_root_and_scalar_queries_use_normal_source_dispatch`: `[array exists {R\0T}]` under Tcl 9.0 gets `NativeByteCommandUnavailable::Recipe` from `native_byte_command_plan` (`rust/tcl-compiler/src/native_byte_compilation.rs`), because `native_array_compilation` (`rust/tcl-registry/src/native_array_compilation.rs`) returns nothing for a literal with a counted NUL; the test expects ordinary dispatch. Whether C compiles or dispatches that word is unmeasured: the oracle scripts cannot feed a NUL as text. The other twelve are listed with their first failing assertion in `current-state.md`.

6. **Ordinary inliner, in-crate (`tcl-codegen` `inlining::`).** Re-run after the structured-control change; several needed `if`/`while` to be structured. `inlining::tests::runtime_script_top_level_inlines_from_its_explicit_namespace` builds its module with the entry-less `lower_script_module_for_bytecode`, which has no source purpose; rebuilding the fixture as a fresh document did not make it pass (a script top level may be frame zero and the rewrite declines), so that change was withdrawn.

7. **WASM native tier (16 tests: `codegen::wasm::native_add` 5, `wasm::pipeline` 4, `wasm::backend` 6, `wasm_tiers` 1, plus `tcl-cli` `compwasm_codegen_passes_natively_lower_a_hot_loop`).** Both routes stop in `tcl-compiler`. Release unit: the sealed selector declines with `closed-program: uncovered-statement` and `direct-call: execution-trace`, because `original_procedure_has_execution_trace` (`registry_invocation/execution_trace.rs`) is true for every authored simulation. Live entry: the plan is the retained form (`select_retained`, not frame-elided), and the sealed selector declined because the body summary could not bind `expr` inside the procedure (`target_resolution_may_be_unknown`), which put a `<global-frame-script>` barrier before the call; not re-measured since `a4e9d10e`. Do not convert the `native_add` fixtures to a live entry as they stand: `a_selected_addition_records_no_decline` would pass on the retained form for the wrong reason. Fixture: `proc add {b c} {return [expr {$b + $c}]}\nset d 2\nset e 4\nputs [add $d $e]\n`.

8. **Remaining rows in `current-state.md`, unchanged in the last stretch:** bare CR in a braced script compiled inline becomes LF; registry compile gaps (8.6 `clock format/add/scan`, 9.1 clock readings, `list {*}` inside a command substitution, `upvar` level above 1, parts of `chan`, `dict`, `string`); Jim `binary` missing from the VM's scripted distribution (9 conformance tests); four `tcl-cshim` C-table rows; `set_release_pins_the_numeral_grammar` for tcl8.6; two `runtime`-engine `tcl-spec-hooks` rows; `cmd_thread_e2e::workers_inherit_a_separate_host_command_surface`.

9. **Not started at all:** `runtime/rust` (60), and collapsing duplicate `tcl-vm` and runtime tests into shared tables.

## Held work

None. No patch files were exported because there was nothing to export. Experiments that were withdrawn are described in the queue items they belong to.

## Open questions and rulings

Rulings I worked under:

- Owner: F5 dialects are for F5 only. No Tcl core stands in for an F5 dialect; F5 is out of scope for the WASM native tier.
- Coordinator, standing until the owner overrules it: an unpinned hook under an F5 profile is still told release 8.4. `tcl-spec-hooks::const_fold_e2e::a_versioned_pack_fold_is_told_the_profiles_evaluation_point` fails at its `f5-iapps` row (the hook is told nothing, the row expects 8.4); the row is unchanged and the failure is therefore a code question, not a fixture one.
- Coordinator: the CLI's WASM native tier goes through a live native entry and the WASM selector, not through lowering changes for authored simulations.
- Coordinator: `direct_body_decision` (`common_aot_plan/direct_calls.rs`) and `cover_closed_constant_store` (`common_aot_plan/closed_program_coverage.rs`) should use the token-owned trace check `original_invocation_has_execution_trace`, with two new differential tests (a body whose procedure has an execution trace, a constant store to a variable with a write trace, both must still decline). **Not applied**: it reaches neither decline that blocks the tier today and turns no test green. I asked whether to land it anyway and had no answer.
- Coordinator: structured `if`/`while` are lowered under the bytecode target where there is no native entry; under a live entry the call is kept, because native emission reads the control plan from the call's tokens (`emit_native_original_preparation_invocation` in `codegen/statements/native_emission.rs`, then `native_control_tasks` in `codegen/native_control.rs`).
- File split: no Rust file over 2,000 lines; new tests go in the module's test file; new code in a named module.

Unanswered: the trace-check question above. The F5 hook question was put to the owner and answered only by the coordinator's standing ruling.

## Hand-offs

Sent and waiting:

- tk-oo: `codegen::catch_in_while_condition` broke with `70460540` (a local revert restored it). They were to decide between changing the fixture and fixing the lowering when resumed.
- Dataflow: static `eval` and `lmap` bodies are still plain calls under the bytecode target (`codegen_stamps::an_inlined_body_inside_a_static_eval_keeps_its_value`, `…an_inlined_body_in_an_lmap_body_gives_the_list_each_iterations_value`); recorded in their area.
- Analyser: the `expr` binding in a procedure body under a live entry (WASM, item 7).

Received and not started: the trace-check ruling (above).

## Hazards

- A build error makes the comparison print "fixed N, new 0" with N equal to the whole baseline. Read the `^error` lines before the numbers. This happened once.
- After another session's commit lands, a "new" failure may be theirs. Re-run the test with your change stashed before reading anything into it, and confirm blame with `git revert --no-commit <sha>` then `git revert --abort`, never by inspection. Two regressions were attributed that way (`01568d90`, `70460540`).
- A measurement is not a result until the options are right. I reported twice that the WASM tier "works" or "fails in the selector" from runs that had the five required passes disabled, or that hit the retained form. Check `frame_elided` and `closed_program_statements` on the selection.
- A module lowered with no entry (`lower_script_module_for_bytecode`, `lower_to_ir`) has no source purpose: the inliner and several proofs are inert on it. A fixture that expects them to act must be lowered as the service lowers a fresh document (`lower_fresh_document` in `compile_service.rs`).
- The lenient `tcl` profile enters no procedure body; a test that needs one must name a release.
- `Script::from_transformed_parts` drops the namespace context, evaluated source and expression preparations; a rewrite must copy them or codegen skips the procedure.
- The clone is shallow. `git rebase origin/work` can fail at the shallow root when the local branch is far behind; with nothing local, `git checkout -B work origin/work` is the safe move.
- Cross-session messages can arrive hours late. Check the queued-at time before acting on one.
- Disk: free space with `cargo clean -p <crate>`, never by deleting under `target/` by age.
- `sleep` is blocked in the foreground; to wait on a background run use `timeout 580 tail --pid=<pid> -f /dev/null`.

## Oracle habits

`tclsh8.4`, `tclsh8.5`, `tclsh8.6`, `tclsh9.0`, `tclsh9.1` are on `PATH` (`/root/.local/bin`), at patch levels 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0; Jim 0.84 is `/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh`. The Tcl source trees are under `/home/user/tcl-lsp/tmp/tcl<version>/` for reading the C.

For an expectation: write the script to a file, run it on every release in a loop (`for v in 8.4 8.5 8.6 9.0 9.1; do echo "$v: $(tclsh$v f.tcl 2>&1)"; done`), and put the script and the answers in the test comment or the commit message. For bytecode: `puts [tcl::unsupported::disassemble script {…}]` or `… proc name` under `tclsh8.6` and later.

To see what the VM does with a one-line script, the quickest route was a throwaway test file in `rust/tcl-vm/tests/` that builds `tcl_vm::test_support::native_vm_for_profile(profile, …)`, sets `BytecodeCompileService::for_profile(profile)` as compiler and prints `vm.try_eval_source(src)`; delete it before committing.
