# Sol-41: tclvm/runtime ports deferred by the origin/rust merge

Branch `work`, merged HEAD efc58507c1. Nothing committed, staged, stashed or reset.
Changes are in the working tree. `sol-41-tclvm-ports.patch` is
`git diff -- rust/tcl-vm rust/tcl-vm-cli rust/tcl-cmd-core` (only my files; `runtime/rust/`
holds another agent's unrelated cfg/clippy edits, and I made no runtime/rust edits).
Gates: `CARGO_INCREMENTAL=0 cargo check -p tcl-vm -p tcl-vm-cli -p tcl-cmd-core -p tcl-engine-tclvm --tests`
gives 0 warnings. `cargo fmt` is clean for these crates. Not run: clippy (work already
carries tcl-vm clippy debt) and the runtime/rust workspace (it consumes the changed
`tcl_cmd_core::return_options` 8.5 behaviour; see item 3).

## A. Systemic failure clusters on merged HEAD (coordinator priority)

| Cluster | Symptom | Root cause | Before the merge? |
|---|---|---|---|
| A | `run_module` panics `module requires a genuine native compiler preflight provider: HostCommandRefusal("native command name protocol is unavailable")`. Also: `Vm::new()` cannot define a proc. | `Vm::new()`/`Vm::with_output` never set `actual_engine_profile`. `actual_native_execution_profile()` then falls back to `DialectProfile::plain_tcl()`, which has no execution point, so `name_policy_protocol()` is `None` and every command-name projection and lookup refuses. Only `Vm::with_native_core` installs the engine profile. No production host called it: the `tclvm` CLI used `Vm::with_output` (fixed below). | **Yes.** Introduced on work by 6fa086c780 (2026-10-16). `name_policy_protocol`, `actual_native_execution_profile`, `with_shared_output_bootstrap`, `run_module` and `validate_foreign_native_compilation_entry` are byte-identical at 03c943a52c and HEAD. This explains most of CI's builtins_e2e, run_script, cmd_control, namespace_surface, cmd_math_expr, cmd_oo, opcode_*, command_mutation_deopt, embed_api, cmd_event and safe_interp failures. It also explains most failing tcl-vm lib unit tests in cmd_package/return_options/exec/cmd_namespace/subst: they all build `Vm::new()`/`with_output`. |
| B | `BytecodeCompileService::compile_for_profile` → `NativeCompilationAdmission(NativePreflightRequired)`, even for `set x 1` | Work's compiler marks every whole-module compile without an entry as `ProviderRequired`. Only the VM's own compile provider (`try_eval_source`/`eval_source*`) supplies an entry. | **Yes** (validation code identical before and after the merge). It affects upstream tests and some work tests (switch_integer_e2e, cross_version_vars_e2e). |
| C | On a native-core VM, `string toupper/index/range/map/…`, `regsub -command` (9.0), encoding/format paths, and calling a proc that has an execution trace are refused: `CompilationAdmission(NativePreflightRequired)`. With `TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC=1` the log shows `NATIVE_CODEGEN_ADMISSION argv=["string","index",…] selection=Some(Unknown)` at `rust/tcl-compiler/src/codegen/mod.rs:877` (also `statements.rs:2641`). `string length` is admitted. | The compiler/registry native-compilation selection is `Unknown` for these invocations. Candidates: the merge's `tcl-registry/src/commands/tcl/string_.rs` changes (`inline_codegen_hook_windows`, the `StringMatch` hook) or `release_compilation.rs`, or Sol-42's uncommitted compiler WIP in this shared tree (all my builds include it). | **Undetermined.** Owner: compiler/registry (Sol-42). Only a clean-HEAD build can decide it; there was not enough disk to do one. |
| D | Error codes set only as completion options never reach the interpreter's error code (`native_errors.primitive_error_code` is the authority in work) | Each site sets the code itself | Fixed where found (below). |

Measured effect of fixing A and B in a harness: moving only `run_script.rs`'s two helpers
to `Vm::with_native_core` + `try_eval_source` took run_script from 0/98 to **63/98** passing.
Cluster C accounts for 22 of the 35 still failing. That experiment is reverted.
**Decision needed** (work's VM naming owner):
1. Make `Vm::new`/`with_output` native-core interpreters at the pinned profile, or
2. Move every work test harness to `with_native_core`.

The compiler owner must also resolve cluster C.

What I did change for A/B:
- **`tclvm` CLI** (`rust/tcl-vm-cli/src/main.rs::build_vm`) now constructs
  `Vm::with_native_core(output, NativeHost, profile-of-release, empty bootstrap inputs)`.
  Before this, the product CLI refused every command. tcl-vm-cli bin unit tests: 8/9 pass.
  The one failure is `{*}`/lassign under `--tcl-version`; it prints empty results and looks like cluster C.
- **Upstream test harness helpers only** now use work's entry:
  `with_native_core` + `try_eval_source` (or `eval_source_directly`) instead of
  `compile_for_profile` + `Vm::new()` + `run_module`. Results are read with checked
  `ValueOps::native_string_bytes`. No expectation was changed. Files:
  - c_error_paths
  - cmd_info_prefix
  - cmd_string
  - coro_error_trace
  - cross_version_info_surface
  - cross_version_lset
  - encoding
  - error_info_text
  - package_coro
  - package_diagnostics
  - regexp (`run_for_version` only)
  - release_number_semantics (reads its result through `puts`, because a numeric list
    result needs the interpreter's numeric operation)
  - return_code_boundary
  - tailcall_uplevel
  - uplevel_coro
  - literal_index_encoding
  - release_running_bytecode
  - rename_return_source
  - tclsh_parity: native core, and it now reports every mismatching vector/release/entry
    instead of stopping at the first; `TCLSH_PARITY_VECTOR=<name>` filters to one vector.
- Two unit-test helpers that call `cmd_package` directly now settle the deferred discovery
  script through `Vm::settle_native_invoke`: `cmd_package::tests::package` and
  `cmd_package/original/tests.rs::call`.

## B. Ports, per item

### Item 1: yieldable `package require` (4a641f4b18). DONE
- `cmd_package.rs`: `pkg_require` is now resumable phases (`PackageRequire`, `RequireStep::{Unknown, Loader}`, `SuspendedRequire`).
  - `defer_package_script` runs a `package unknown`/`ifneeded` script at level 0 as an `EvalReq` activation (`select_execution_frame_checked(0)`). The activation carries the suspended require.
  - `advance_require` resumes it: `resume_require`, `after_package_unknown`, `finish_loader`.
  - A script with nothing to run continues at once.
  - Diagnostics spell their requirements per release (`requirement_text`: `can't find package nope 1.2`, 8.4 `need 1.1`, `exactly V` from 8.5).
  - A `package present` conflict uses the release's conflict code (8.5 `NONE`, 8.6+ `TCL PACKAGE VERSIONCONFLICT`).
- `exec.rs`:
  - `EvalReq.package` / `Frame.package`.
  - `settle_control_activation` resumes the require. It pushes the next phase (loader after unknown), or delivers the result and logs the `package require` command.
  - `abandon_parked_requires` runs when a suspended coroutine is retired or deleted (`cmd_coro.rs`): the marker is ended and the loader's provided version is withdrawn.
  - A host-refused loader activation retires its marker (`end_abandoned_package_loading`).
- `interp.rs`: `end_package_loading_using` removes the matching marker wherever it sits, so loaders that finish out of order retire their own markers (upstream's `PackageLoadToken`).
- `cmd_package/original.rs`: `OriginalPackage.authored`.
- Tests:
  - package_coro_e2e **12/12** (was 1/12)
  - package_diagnostics_e2e **5/5**
  - lib `cmd_package` 10/34: the 24 failures all build `Vm::new()` (cluster A). `original_package_unknown_and_loader_restore…` passes.

### Item 3: return/tailcall semantics and C error reporting (9aa982ac2a). Mostly done
- `exec.rs`:
  - `logged_command`: upstream `error_info_text`/`error_info_enclosing` as bytes over work's `SourceImage`.
  - `log_command_with_enclosing` and `log_enclosing_commands` are used at every errorInfo site: settle_completion, `try_cmd`, the three invoke sites, and parent logging after body/uplevel/proc.
- TIP 348 `INNER` for invokes: `invoke_inner_context` / `compiled_error_context` (`invokeStk1 …`, 9.1 `invokeStk`, `returnImm` for compiled `error`/`throw`, text for a direct command).
- Script words the parent evaluates (`INST_EVAL_STK`, `catch $s`) log the parent command. This was verified against tclsh.
- Procedure frames name the invoking word (`(procedure "::r" line 1)`). OO method exits are excluded.
- Proc-boundary `break`/`continue` takes the command-error path: errorCode `NONE` before 8.6, `TCL RESULT UNEXPECTED` from 8.6. C8.4 global errorInfo/errorCode are seeded (`Vm::seed_unexpected_completion_error`).
- `update_return_info` (both the merged-options path and the options-dict path): a `-code return` that reaches its level becomes a plain `return` one level further out (`-code 0 -level 1`), as C's `TclUpdateReturnInfo` does.
- Tailcall replacements log their own words (`Frame.tailcall_invocation`, `log_tailcall_invocation`), including synchronous replacement errors (`tailcall set`).
- Top-level settle (`interp.rs::settle_level_zero`):
  - `PendingControl.top_level_command` records the command the outermost activation ran, and C's `TclEvalEx` logs it at `numLevels == 0`, unless it was already logged.
  - An unexpected code starts a fresh trace (`while executing "break"`).
  - From 8.5 a `return -errorinfo` counts as logged.
  - `eval_file(top_level)` adds the `(file …)` frame even for an error a `return` settled to. Measured: 8.4/8.5 `return -code error`, `return -level 2`.
  - `eval_source_commands` no longer re-publishes `errorInfo` from the completion snapshot over the trace `settle_top_level` published.
- `command.rs::merged_completion_options_snapshot`: once logging has extended the interpreter's trace, `-errorinfo` is that trace, not the stale `return -errorinfo` object.
- `return_options.rs` (VM): 8.4 `return -errorcode` stores the global code at once.
- `tcl-cmd-core/return_options.rs`: 8.5 `TclMergeReturnOptions` errors carry errorCode `NONE`. An invalid or odd `-options` fails with the result word as message (measured on 8.5.19: `return -options {-code ok -level} x` gives `1 x NONE`). `native_return_merge.rs::process_stack` does the same for the bytecode `returnStk` path. This also changes runtime/rust's 8.5 behaviour through the shared core; that was not run there.
- `cmd_list.rs` lsort `-command` (C path): keeps the comparator's own trace and logs the comparator command (`invoked from within "cmp b a"`).
- `subst.rs`: `subst_word_bytes_as(word, vm, evaluation)`. A command substitution in a directly evaluated command is itself evaluated directly (`eval_source_direct_internal`), as each nested `TclEvalEx` level is.
- Tests:
  - error_info_text **5/5** (0/5)
  - tailcall_uplevel **7/7**
  - uplevel_coro **15/15**
  - return_code_boundary **5/5** (0/5)
  - c_error_paths **15/20** (0/20). The 5 left:
    - 4 are cluster C (`string`, `regsub -command`, the execution-trace callback).
    - 1 is `lsort -command`: the comparator's inner `(procedure "cmp")` frames are still missing, and `lsort -command nosuch` hits a `retired native object header` value refusal.
  - tcl-vm-cli: script_errors 3/5, top_level_e2e 6/10, bin unit tests 8/9. Before this they were 0 (cluster A). The rest:
    - top-level coroutine settle (item 9b)
    - 8.4 direct substitution enclosing log (`string repeat [e] 2`: work evaluates that statement through a path that bypasses PUSH-word substitution)
    - 8.5 stdin errorInfo and 8.5 `return -code error` file trace
    - two script_errors cases (`expr {[p]}` nesting, `if {1} {break}`)

### Item 4: errorInfo levels / `ErrorLogged::ThroughFrames` (fcb7927f02). Partial
- Done: enclosing-command logging (above). Directly evaluated substitutions log each level through direct nested evaluation.
- Not ported: the tri-state `ErrorLogged::ThroughFrames` (8.4 `Tcl_ExprObj` keeping a substitution error logged up to a procedure boundary). Also not ported: `Frame.script_line_base` / `errorLine` counting for multi-command substitutions. Both need work's `NativeErrorState.error_logged` (bool, swapped in parked flows and native C error flags) to become a three-state value. Their parity vectors (condition_errors, constant_fold_errors) are blocked by cluster C, so I could not verify them, and I left the field unchanged.

### Item 5: compiled loop/switch/try/dict frames via `release_compiled` (ca698d87c8). Base done
- `PendingControl.release_compiled` is set around INVOKE_STK with upstream's filter: `in_procedure_only`, and `Vm::frame_compiled_locals_named` over work's `CallFrame::compiled_locals`. It is cleared after dispatch.
- Consumers:
  - `cmd_catch`: a compiled catch that evaluates its script word logs itself (`CatchReq/CatchCtx.evaluates_word`, `log_compiled_catch`).
  - `cmd_if`, `cmd_while`, `cmd_for` (`ControlState.compiled`, `ControlState::fail`):
    - a compiled command's inline clause logs once;
    - its script word logs the command;
    - a run command adds `("while" body line N)`, `("for" initial command)` or `("for" loop-end command)`, then the command logs (`logs_invoking_command`).
  - All of this was verified against tclsh 8.4/8.6 for `if 1 $s`, `while 1 $s`, `for $s …` and `for {} 1 $s {}`.
- Not ported: the switch, try and dict consumers (`cmd_switch` arm frames, try fall-through binding, dict compiled-body logging). Also not ported: `Value::compile_as_script` for the script word (item 8). The loop_frames, switch_compile and top_level_* parity vectors are dominated by cluster C refusals and by the compiled-vs-direct model (below), so I could not use them as evidence.

### Item 6: expr word substitution and the 8.4 numeric model (0c7cb8f64b). Not done
- release_number_semantics_e2e is 6/14 after the harness fix.
  - 3 failures are cluster C.
  - 2 are a VM gap: a list holding 8.4 wrapped wide integers has no string updater (`ValueAccessRefusal(StringAccess(Unavailable(StringUpdater)))`). This was a **panic** inside `cmd_puts`; I changed `cmd_puts` to read its words through checked `ValueOps::native_string_bytes`, so it is now a host refusal and no longer a panic.
  - 3 are semantics to port: 8.4 double divide-by-zero / underflow, the 8.4 operand wording for an empty string, and the `tcl_precision` write-trace frame.
- The upstream `cmd_math`/`value_ops` 8.4 model needs work's native numeric object model (string updater for 8.4 wide ints). That is a broader change than a port.

### Item 7: tcl-cmd-core rewrites (8c2138282f, ca698d87c8). Partial
- `index.rs::resolve_for_ops_with`: a caller naming 8.4 numerals reads 8.4's index grammar and wording (`must be integer or end?-integer?`).
- Error-code separators were missing in `encoding/original_names.rs` (`TCL LOOKUP ENCODING name`, `POSIX ENOENT {…}`) and `native_frame_error.rs` (`TCL LOOKUP LEVEL name`). These were pre-existing on work.
- tcl-cmd-core unit tests: **257/257** (was 253/256).
- Not ported: lseq `OPERATIONS`/`check_reads`, the `lsearch`/`lsort` `release` field plus `drill_in_release`, and the `string.rs` identity shortcuts. Their e2e evidence is blocked: cmd_string_e2e is 6/51 and all 45 failures are cluster C; string_surface parity is likewise.

### Item 8: literal binding / representation shimmer. Not done
- `LiteralValues`, `CompiledUnit.literal_values` and `literal_word_value` stay `#[expect(dead_code)]`.
- literal_index_encoding_e2e (harness migrated) fails with cluster C before reaching any literal behaviour. There is no verifiable bar until C is fixed.

### Item 9: namespace frame text and top-level coroutine bodies. Partial
- Done: `cmd_namespace.rs::eval_in_ns` adds `(in namespace eval|inscope "FQN" script line N)` through `tcl_cmd_core::namespace::script_frame_text`. Output is identical to tclsh 8.4 and 8.6 for eval and inscope.
- Done (coroutine error traces): `exec.rs` logs a coroutine body command's words (`Frame.dispatched_invocation`). `interp.rs::adopt_flow_error_trace` carries the native errorInfo object across flows. This fixed `native private errorInfo result` refusals. coro_error_trace_e2e **7/8** (1/8); the remaining failure is cluster C.
- Not done: a coroutine created at top level settling as top level (`{0 x 0 0}`). Work's frames carry no top-level role, so this needs a decision on how to mark top-level entry activations (upstream `ActivationRole::TopLevel`).

### Item 2: 8.4 running-bytecode release units. Not done
- `release_units.rs` is still undeclared. release_running_bytecode_e2e and rename_return_source_e2e (harness migrated) first hit cluster C (`string` cases) and an `original C86 split worker` value refusal. The model (command-table history per running unit) is a large port into work's INVOKE and command-table mutation paths. It is not started, because none of its tests can run until C is fixed.

### Item 10: stdin history. Not done
- Not modelled. It needs the item 8 literal lifetimes.

### Sol-40 deferrals
- `LiteralValues` dead_code: unchanged (item 8).
- `ErrorInfoCommand.text: String`: kept. Work's VM now reads it as bytes (`logged_command`); a non-UTF-8 command keeps its `source_cmd_text` bytes.
- runtime `info exists` exception-state restore: not touched (runtime/rust).

### Other fixes (cluster D and robustness)
- `interp.rs::present_variable_trace_failure`: the access's `TCL READ|WRITE VARNAME` becomes the interpreter's error code (`restore_guest_error_code`). variable_trace_semantics_e2e 14 → 16/26. The rest are cluster C or other value refusals.

## C. Evidence summary (tests run one binary at a time; binaries deleted afterwards)
| Binary | Before | After |
|---|---|---|
| error_info_text_e2e | 0/5 | 5/5 |
| uplevel_coro_e2e | 0/15 | 15/15 |
| tailcall_uplevel_e2e | 0/7 | 7/7 |
| return_code_boundary_e2e | 0/5 | 5/5 |
| c_error_paths_e2e | 0/20 | 15/20 |
| coro_error_trace_e2e | 0/8 | 7/8 |
| package_coro_e2e | 0/12 | 12/12 |
| package_diagnostics_e2e | 0/5 | 5/5 |
| variable_trace_semantics_e2e | 14/26 | 16/26 |
| release_number_semantics_e2e | 0/14 | 6/14 |
| tcl-cmd-core (lib + doc) | 253/256 | 257/257 |
| tcl-vm-cli bin / script_errors / top_level_e2e | 0 / 0 / 0 (A) | 8/9, 3/5, 6/10 |
| tclsh_parity_e2e | all vectors mismatch | top_level_return_code/level pass; the rest are cluster C or compiled-vs-direct |
| tcl-vm lib filters (cmd_package, cmd_coro, return_options, cmd_control, cmd_list, cmd_namespace, subst, exec) | — | failures are Vm::new (A) or C; none from these ports |

"Before" means the merged HEAD harness. Most 0s were harness clusters A/B; the after
numbers include the ported VM behaviour.

## D. Shared-tree notes
- runtime/rust has uncommitted cfg/clippy edits from another agent. They are not mine and are not in the patch.
- `target/` disk stayed at 0.2–2 GB free during this run. Test binaries were built one at a time and deleted after use. One build hit ENOSPC; I freed space and rebuilt it, so no result here comes from that build.
