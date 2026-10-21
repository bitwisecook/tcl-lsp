# Sol-48: TclOO/VM stack and parity work (parked), runtime P0, SpecTcl execution

Branch `work`. I made no commits myself. Every change listed here since e34d8a427e is
already in the coordinator's WIP snapshots (0fd92a22b6, 363a9f89de) and in 0cffd94ecf (the
runtime P0 fix). The only changes after 0cffd94ecf are in `sol-48.patch`
(`git diff 0cffd94ecf -- rust/tcl-vm rust/tcl-vm-cli rust/tcl-vm-wasm rust/tcl-engine-tclvm rust/tcl-cmd-core runtime/rust rust/tcl-spectcl`):

- `rust/tcl-spectcl/src/loader/eval.rs`: the continuation fix (§3.2).
- `rust/tcl-spectcl/tests/eval_loader.rs`: removal of my temporary `zz_s48_*` probe tests,
  which the snapshot had committed by accident. The file is now byte-identical to e34d8a427e,
  which also clears the CI fmt flag.

**Stopped on disk.** Free disk fell to 1.3–1.8 GB while Sol-49 was rebuilding tcl-compiler,
and the brief says to stop building below 2 GB. Two things were not run as a result:

- the "after" runs of the SpecTcl targets;
- the runtime string-member tests.

Both are listed under "Not run" below.

Environment for every run: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0`,
`TCL_LSP_REQUIRE_TCLSH=9.0,8.6`, `TCL_LSP_JIMSH=tmp/jimtcl-oracle/jimsh`. Binaries were built
and run one at a time and deleted afterwards.

Caveat for every before/after pair: Sol-49 rebuilt tcl-compiler several times during this
session, so each "after" also includes their compiler edits from that moment.

## 1. Runtime P0 (CI 39616304947): compile fix, committed as 0cffd94ecf

Sol-45 added `NativeInstructionPlan::StringOperand` (`string compare/index/first/last/toupper/tolower/totitle`),
but `runtime/rust` was never built against it.

- New `runtime/rust/src/interp/native_body_artifact/native_string_operand.rs`:
  - Each operand is the original word, read through `namespace_operand` and
    `body_namespace_operand`, exactly as `Scalar` reads its operands.
  - Each member then runs through the same shared core the `string` command uses:
    - `compare` → `tcl_cmd_core::string::compare` (`CompareMode::Compare`);
    - `index` → `string::index`;
    - `first` / `last` → `string::first` / `string::last` (no start index);
    - `toupper` / `tolower` / `totitle` → `case_convert` in Upper / Lower / Title mode.
  - The number syntax is that of the artifact's physical release.
- `native_body_artifact.rs`: wired in as follows.
  - A new `Operation::StringOperand` variant.
  - The plan arm that builds it.
  - The prepared-word drain.
  - Its operands added to the list of words the instruction does not emit.
  - The executor arm.
- `native_compiler_pass.rs`: `StringOperand` joins the no-hazard arm.
- Gates:
  - `cargo check --locked --features engine --all-targets` in `runtime/rust`: Finished, 0 warnings.
  - rustfmt is clean.
- Workspace grep: the only other exhaustive `NativeInstructionPlan` matches are in tcl-compiler,
  which already has the arm, and tcl-registry. Nothing else breaks.
- **Not run (disk):** the runtime tests that cover string members. Run them next with
  `cd runtime/rust && TCL_TOMMATH_DIR=../../tmp/tcl9.0.4/libtommath cargo test --locked --features engine`.
  The relevant ones are the lib `string` filter, `tests/integer_argument_reads.rs` and
  `tests/literal_index_encoding.rs`.

## 2. Scope change (user-decisions-20261021T1310Z): what was done, measured and parked

### 2.1 Done (VM/CLI; all in the snapshots)

- **Cluster P** (`command_mutation_deopt_e2e`): Sol-46's option (a) is applied, with a documented
  helper `assert_mid_run_switch_refused`.
  - Background: on a native core, an entered or parked operation's currency captures the
    profile and the compile service. A mid-run switch therefore reaches the host boundary
    first, as `TclError::Host(Execution(ValueAccessRefusal(CommandProtocolUnavailable("entered VM original execution context"))))`.
    That refusal comes before any stale bytecode step.
  - Every scenario and every "body ran once" / `info commands c` assertion is kept.
  - `fresh_try_return…` now uses `try_invoke_command`, because `invoke_command` panics on a
    host refusal.
  - 6 tests fixed: `terminal_profile_flips…`, `runtime_foreach_does_not_tag…`,
    `suspended_coroutine…`, `tailcall_non_ok…`, `stale_coroutine_ancestor…`, `fresh_try_return…`.
  - `final_native_compiler_swap…` and `stale_coroutine_teardown…` still stop first on
    cluster C, the NativePreflightRequired refusal from the compiler side (§4). Their P
    expectations were left untouched.
  - The rest of the binary is B/C plus "other". This is unchanged, and those failures belong
    to the compiler side.
- **F1 (TclOO recompile).**
  - `NativeProcedureCommand::new` moves `ProcDef.body` into `native_resources.foreign_body`.
  - `current_native_procedure_body` now accepts that retained body as a *native* candidate
    when it came from the current compile service.
  - It then applies the same currency checks as the per-proc bytecode cache:
    - native-cache stamp (namespace resolver epoch);
    - profile generation;
    - compiler generation;
    - namespace path;
    - interpreter;
    - command bindings and manifest;
    - no step trace.
  - The method record's `compiled_body` (per object namespace) is therefore reused until one of
    those changes.
  - Unit test `cmd_oo::lifecycle_tests::a_method_body_compiles_once_until_its_definition_or_command_world_changes`.
    It runs the sequence below; tclsh 9.0.4 gives `33 R 3 4`.
    - The second call reuses the compiled `asm`.
    - `rename expr` / `proc expr` recompiles, and the method returns `R`.
    - Restoring `expr` recompiles again.
    - `oo::define … method` recompiles.
  - Timings (debug test binary, 1 thread):

    | test | before | after |
    |---|---|---|
    | `obj go 10` (`moderately_recursive_method_still_runs`) | 13.5 s | 4.7 s (about 3.7 s of that is fixed per-test bootstrap; `basic_class_and_method` takes 3.7 s) |
    | 2 MiB recursion test | 41.0 s | 5.7 s |
    | `cmd_oo_e2e` whole binary | 107 s | 70 s |
- **F2 (frames).** Static sizes come from objdump of the debug test binary:

  | function | before | after |
  |---|---|---|
  | `tick_instruction` (non-invoking opcodes) | 80.7 KB | 54.4 KB |
  | `tick_variable_instruction` (new; routed from `tick_in_original_scope` like `tick_invoke`) | — | 19.4 KB |
  | `tick_scalar_variable_instruction` (new; LOAD/STORE/INCR scalar and element) | — | 7.3 KB |
  | `settle_native_invoke` | 101.9 KB | 0.6 KB |
  | `settle_native_dispatch` | 52.1 KB | 1.4 KB |
  | `eval_original_script_value` | 19.0 KB | 2.9 KB |

  - The old `settle_native_invoke` body moved out of line into `take_native_settlement`
    (99.7 KB), which builds a boxed activation and returns before the body runs.
  - The old `settle_native_dispatch` work moved into `take_pending_dispatch_tick` (49.1 KB);
    `deliver_sync` is now called with a small frame.
  - `eval_original_script_value` keeps its old interface; its preparation moved out of line
    into `prepare_original_script_run`.
  - Measured with gdb: recursion through a variable write trace costs about 123 KB per level
    before this change, about 88 KB after.
  - New test `cmd_oo_e2e::recursion_through_a_trace_or_math_function_fits_a_two_mib_thread`.
    tclsh 9.0.4 and 8.6.16 print `0 done 19`, `20` and `19`. On a 2 MiB thread it runs:
    - a method recursing through a write trace to `OO_DISPATCH_DEPTH_LIMIT` (20), with the 21st
      refused;
    - a user math function that calls a method, 19 levels;
    - proc recursion through a write trace, 19 levels.
    - Before this change the test overflowed the stack.
  - `install_tick` (83 KB) is transient, so it was left as it is.
- **Item 9, top-level coroutine settle.**
  - `PendingControl.{bottom_activation, top_level_invocation}` hold the "statement's own
    command at `numLevels == 0`" fact, read from `f.script_command`.
  - `coroutine` takes that fact, and its base frame settles through `settle_level_zero`
    (`Frame::settle_as_top_level`).
  - Fixes `top_level_e2e::coroutine_created_at_top_level_settles_as_top_level`.
- **Item 3, `if {1} {break}`.**
  - `Frame.script_command` records the compiler's `Instruction::script_command` boundary.
  - At the outermost unwind, the top-level settle logs that outermost statement, as C's
    `TclEvalEx` does.
  - Fixes the `if.tcl` case of `script_errors`.
- **CLI (`tcl-vm-cli/src/main.rs`).**
  - stdin: the stale `publish_caught_error` after `eval_interactive` is gone. In 8.5+ the
    completion's `-errorinfo` predates the trace, and `settle_top_level` already publishes
    the interpreter's trace. Fixes `stdin_is_read_a_command_at_a_time`.
  - `run_file` reports the published `errorInfo`. This fixes 8.5+ `return -code error` at file
    level, both `file_return_code_error_logs_the_return` and the `return.tcl` case.
- **Item 5.**
  - `switch` (runtime form): the arm frame is `("PATTERN" arm line N)`.
  - `dict for` / `dict map`: the body frame is `("dict for" body line N)`.
  - `dict with` / `dict update`: the frame is `(body of "dict with")`.
  - A release-compiled form logs nothing more, taking `pending.release_compiled`.
  - `try`: the compiled fall-through binds the matched clause's variables (`TclCompileTryCmd`).
  - All of these were checked against tclsh 8.5, 8.6 and 9.0.
- **Item 2, partial.**
  - `release_units` is now declared.
  - `Frame.release_units` records unit entries on 8.4.
  - `release_keeps_running` stops the boundary replay for a command the release compiled into
    a unit entered before the change.
  - Not done: unit inheritance into child activations, and `release_compiled_command` (the
    history-held builtin for an INVOKE). Those items carry a
    `#[cfg_attr(not(test), expect(dead_code, reason = …))]`.
  - The test bar (`release_running_bytecode_e2e`) still stops first on cluster C.
- **Item 6, partial.**
  - Before 9.0, an operand's description reads its text: `"x"` was described as
    `empty string` and is now `non-numeric string`.
  - An 8.4 double division by zero is `divide by zero` / `ARITH DIVZERO`.
  - From 8.5, a refused `tcl_precision` write starts errorInfo with the trace message and
    `(write trace on "tcl_precision")`.
- **lsort -command (item 3).**
  - Previously a comparator error under 8.4 was a silent host refusal: a retired
    error-code object was being restored.
  - Now a retired `-errorcode` object is not restored, because the interpreter already holds
    the error.
  - The `cmp` half of `lsort_comparator_error_logs_the_callback` now matches C. The test
    still fails, on the `nosuch` half: 8.4 leaks the previous errorInfo.
- **SpecTcl-relevant VM fix: `commands` budget.**
  - An armed commands limit now also charges a command compiled inline at its source
    boundary, as C does at `INST_START_CMD`. The check is `command_is_dispatched` (an
    `INVOKE_*` of the same span) and runs only when a limit is armed.
  - A `while {1} {set x [llength …]; incr i}` loop previously never charged a step.
  - Fixes `eval_loader::a_budget_blowing_loop_fails_closed_naming_the_axis`; see §3.

### 2.2 Measured: tcl-vm and tcl-vm-cli binaries (pass/total)

"Before" is the tree at e34d8a427e. Each per-test before/after comparison covers every
binary in this table:

- **Regressions:** none. No test that passed before fails after.
- **Newly passing:** the 6 P tests above; `script_errors::an_uncaught_error_reports_its_trace_and_where_in_the_file`;
  and three `top_level_e2e` tests (`coroutine_created_at_top_level…`, `file_return_code_error_logs_the_return`,
  `stdin_is_read_a_command_at_a_time`).

| binary | before | after |
|---|---|---|
| command_mutation_deopt_e2e | 32/84 | **38/84** |
| cmd_oo_e2e | 62/74 | **63/75** (+1 new test; 107 s → 70 s) |
| builtins_e2e | 100/126 | 100/126 |
| cmd_control_e2e | 61/114 | 61/114 |
| run_script | 74/98 | 74/98 |
| opcode_c_parity | 54/90 | 54/90 |
| c_error_paths_e2e | 15/20 | 15/20 |
| tcl-vm-cli script_errors | 3/5 | **4/5** |
| tcl-vm-cli top_level_e2e | 6/10 | **9/10** |
| tcl-vm-cli bin (tclvm unit) | 8/9 | 8/9 |
| tcl-vm-cli channel_output | — | 10/12 (Sol-44: 10/12) |
| tcl-vm-cli static_extensions | — | 2/2 |

Baseline only. The final sweep hit its 2 h limit before re-running these, so no "after" exists:

| binary | before |
|---|---|
| error_info_text_e2e | 5/5 |
| uplevel_coro_e2e | 15/15 |
| coro_error_trace_e2e | 8/8 |
| tailcall_uplevel_e2e | 7/7 |
| cmd_coro_e2e | 34/59 |
| namespace_surface_e2e | 109/123 |
| release_running_bytecode_e2e | 0/1 |
| release_number_semantics_e2e | 6/14 |
| literal_index_encoding_e2e | 1/2 |
| tclsh_parity_e2e | 1/2 |
| variable_trace_semantics_e2e | 16/26 |
| cmd_math_expr_e2e | 95/102 |
| tcl-vm lib | 1107/1387 (1327 s) |

Timing of the runs, relative to the later VM edits:

- cmd_oo_e2e, command_mutation and the CLI binaries ran before the `commands`-budget change
  and the expr operand changes.
- The CLI `after2` run straddled the expr edit.
- The remaining tcl-vm integration binaries were not run in this session.

**First thing to run when disk allows:** the tcl-vm lib, tcl-engine-tclvm lib (it has
commands-budget tests), embed_api_e2e and release_number_semantics_e2e.

The remaining clusters are as Sol-44 classified them: B/C (compiler native preflight),
H/S (harness), and V.

### 2.3 Parked (scope change)

- Items 2 (inheritance and history-held builtins), 6 (8.4 numeric model, StringUpdater
  refusals, literal underflow), 7, 8 and 10.
- Item 4 compiler metadata.
- The `lsort -command nosuch` 8.4 errorInfo leak.
- `tick_instruction` further splitting, and `install_tick`.

Probe of item 7: these vectors match tclsh 9.0 and 8.6 —

- every `lseq` case (prefix keywords `t`/`co`/`b`, by 0, count -1, big integers, `x`, wrong
  arity);
- `lsearch -start/-index/-stride/-integer`;
- `lsort -stride/-integer/-real/-index`;
- `string repeat a -1`.

Every other `string` vector tested (`replace`, `range … end+1`, `index x`, `first … x`,
`is int -strict`) dies with the cluster C host refusal. Also, the `tclvm` CLI prints
nothing for a host refusal; it exits 1 silently, which is a UX gap.

## 3. SpecTcl execution

Targets: `make test-spectcl-compat`, which runs
`eval_loader golden_packs pack_source_e2e spec_corpus pack_is_real_tcl`, plus
`codegen_stamps`. Environment: `TCL_LSP_TCLSH90`, `TCL_REQUIRE_SPECTCL_COMPAT=1`.

### 3.1 eval_loader

Before (tree with my VM work, before the SpecTcl fixes): **24/28**. The 4 failures, each
diagnosed with temporary probes:

1. `a_budget_blowing_loop_fails_closed_naming_the_axis`.
   - Symptom: wall clock ran out instead of command steps.
   - Root cause (tcl-vm): the commands limit charged only dispatched commands, so the loop's
     inline-compiled commands never counted.
   - Fix: the VM budget change in §2.1. A probe now reports `command steps` after 2 000 steps.
     The test passes; the wall-clock companion test also passes.
2. `from_frame_effect_derives_the_alias_pairs_resolver`.
3. `derived_frame_aliases_register_explicit_logical_source_dispatch`.
   - Symptom (2 and 3): `probe::plain` (`resolver from-frame-effect` without an `AliasPairs`
     `frame_effect`) was dropped on **both** routes.
   - Root cause (tcl-spectcl): `derive_transitions_from_frame_effect` logged the "needs the
     command's `frame_effect -layout AliasPairs`" notice in the Semantic class, and §6.1
     excludes any spec carrying a Semantic notice. Upstream (origin/rust) logged it as a
     plain notice. The README's `from-frame-effect` rule says "leaves nothing to derive, **a
     notice, and no resolver**".
   - Fix: the class is now `Assistance`. The spec loads degraded, without a resolver, and
     keeps the notice. This change is in `src/loader.rs` and is already in a snapshot.
   - Verified by probe: both routes load `probe::alias` and `probe::plain` with the notice.
4. `every_shipped_pack_loads_identically_with_and_without_the_static_fast_path`.
   - Symptom: `eda_microchip` `add_library` is on line 67 by the fast path and on line 65 by
     the interpreter.
   - See §3.2.

### 3.2 Root cause of the microchip line drift (tcl-spectcl; fixed after 0cffd94ecf)

- A braced word's *value* folds each backslash-newline to one space, so a body evaluated
  from that value loses one line per continuation.
  - The microchip `command add_file_to_library` body has two `option … \` rows, which
    accounts for the 2-line drift.
- The loader is meant to evaluate the file's *verbatim* body text in that case
  (`VerbatimBody`, "so continuation collapse cannot shift line attribution").
  - But `original_for_capture` accepted the verbatim text only when `captured_text() ==
    actual_body`, plus a literal-statement match against the evaluated args.
  - Both comparisons fail exactly when the body holds a continuation, so the loader fell back
    to the folded value.
- Reproduced at the engine level:
  - the line watch with a speclib body holding a nested `\`-newline gives `command b` line 6
    instead of 7;
  - top-level source gives the correct line;
  - plain `tclvm` file, `eval`, `uplevel` and `namespace eval` lines are all correct.
  - So this is neither the VM nor the compiler.
- Fix in `loader/eval.rs`, `VerbatimBody::original_for_capture`:
  - the verbatim text corresponds when folding it (`collapse_brace_continuations_str_for`,
    `Folds`, which is the Tcl core rule pack evaluation runs under) gives the evaluated body;
  - the literal-statement match then compares the verbatim text in the body's place.
- **Not verified (disk):** I could not rebuild eval_loader after this fix. Expected result:
  28/28. This needs a run.

### 3.3 Not run (disk)

The "after" runs of all targets are missing, and so are "before" runs of everything except
eval_loader:

- eval_loader, after the continuation fix;
- codegen_stamps;
- golden_packs;
- pack_source_e2e;
- spec_corpus;
- pack_is_real_tcl.

For reference, Sol-44 had codegen_stamps at 1/29 (cluster B 22, plus "CompileService does
not expose entry-aware byte parsing" 3) and pack_source_e2e at 3/6.

## 4. For the compiler side (Sol-49); I made no edits there

- **Cluster C** (`CompilationAdmission(NativePreflightRequired)` through the VM's own entry)
  is still what stops these tests first:
  - `final_native_compiler_swap…` and `stale_coroutine_teardown…`;
  - `release_running_bytecode_e2e`;
  - `string replace/range/index/first/is` at run time;
  - c_error_paths: `error_stack_inner…`, `execution_trace…`, `regsub_command…` and `sourced_file…`;
  - most of cmd_control and cmd_coro.
- **Item 4 metadata.** These are still needed:
  - `ErrorInfoCommand.enclosing` for directly evaluated statements and conditions;
  - `in_direct_statement`;
  - the no-fold rule.

  They are needed by `script_errors::a_nested_error_logs_each_level_the_release_evaluates`
  (8.5 `set w [expr {[p]}]`) and by `top_level_e2e::file_substitution_errors_log_each_enclosing_command`
  (8.4 `string repeat [e] 2`).
- **Item 5.** The compiler must set `release_compiled` on the INVOKE of a `switch` / `try` /
  `dict for|map|with|update` that the release compiles but that the VM runs as a command. The
  VM side now consumes it. An example of what is missing: 8.5 compiles `dict with` in a proc
  body, which C does not, so the `(body of "dict with")` frame is missing.
- **Item 2.** `release_body` / `release_body_entry` / `release_hold` / `release_binding` are
  already emitted. The VM needs nothing more from the compiler.

## 5. Gates

- `cargo check -p tcl-vm --lib`: 0 warnings.
- The runtime crate (`--all-targets`): 0 warnings.
- tcl-spectcl test build: 0 warnings.
- rustfmt: clean on every file I touched, including `tcl-spectcl/tests/eval_loader.rs`.
- clippy: not run. It needs a build, and disk did not allow one.
- Scratch: under 500 MB of my own; the large scratch binaries were deleted.
