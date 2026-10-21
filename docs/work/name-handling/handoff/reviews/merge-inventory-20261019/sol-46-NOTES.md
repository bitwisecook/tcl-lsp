# Sol-46: TclOO recursion stack (task 1); stopped on disk

Branch `work`, HEAD 273a21de70. Nothing is committed, staged, stashed or reset.
`sol-46.patch` is `git diff -- rust/tcl-vm rust/tcl-vm-cli rust/tcl-vm-wasm rust/tcl-engine-tclvm rust/tcl-cmd-core runtime/rust`.
Only `rust/tcl-vm` changed. Sol-45's `rust/tcl-compiler` and `rust/tcl-registry` edits were not touched.

**Why this stopped early.** Task 1 is done and verified. Tasks 2 and 3 were not started.
- Free disk fell from 3.1 GB to 0.7 GB, then settled at 1.2–1.3 GB. The brief says to stop building below 2 GB.
- Most of the drop is mine. A `cargo build -p tcl-vm --example eval` (dev profile, full debuginfo) wrote large rlibs into
  `target/debug/deps` between 23:05:00 and 23:08:10:
  - `libtcl_compiler-3875a7ddaa95dbe2.rlib` (1.04 GB) and `.rmeta` (47 MB)
  - `libtcl_registry-6849e370486ec6fe.rlib` (232 MB) and `.rmeta` (59 MB)
  - `libtcl_vm-8f3d8e1599423fd9.rlib` (211 MB) and `.rmeta` (7 MB)
  - fingerprints `.fingerprint/tcl-{compiler-3875a7ddaa95dbe2,registry-6849e370486ec6fe,vm-8f3d8e1599423fd9}`
- I deleted the example binary itself (677 MB). The permission classifier refused to let me delete the rlibs, because it
  could not tell that they were mine.
- Deleting them frees about 1.6 GB. Nothing in the test profile uses them, so a later dev-profile build would only rebuild
  them. Never build examples in the dev profile on this machine: they carry full debuginfo.
- I deleted my test executable (`cmd_oo_e2e-0dc419f3369c36b7`) and my 120 MB of scratch files.

## Task 1: TclOO method recursion stack (crash). Done

### Measurement
- Method: gdb on the stack overflow; for each frame, take the difference in `$rsp` from the frame below it.
  The script is `scratchpad/sp.py`.
- Static frame sizes: the prologue `sub rsp` plus the stack-probe loop, read from `objdump`.
- Build: debug test build, `CARGO_PROFILE_TEST_DEBUG=0`.

Native stack per `my go` recursion level:

| frame | before | after |
|---|---|---|
| `Vm::tick_in_original_scope` | 93 504 | 712 (+ `tick_invoke` 4 992) |
| `cmd_oo::run_step_inner` | 48 160 | 952 |
| `Vm::oo_run_method` | 17 856 | 1 816 |
| `Vm::dispatch_selected_words` | 15 600 | 1 048 |
| `Vm::run_activation` | 5 264 | 152 |
| rest of the cycle (drive/drive_loop/tick/dispatch_words_*/run_step/oo_invoke*/cmd_my) | ~16 000 | ~16 000 |
| **one recursion level** | **≈196 KB** | **≈25.8 KB** |

- origin/rust, for comparison: I did not build it (disk). Its `OO_DISPATCH_DEPTH_LIMIT` doc comment records a crash
  between depth 45 and 48 on a 2 MiB thread, which is about 44 KB per level. Work is now about 40 % below upstream's cost.
- Recursion to the limit (20 levels, then the refused 21st) now needs between 768 KiB and 1 MiB in total
  (`RUST_MIN_STACK` bisection). Before, depth 10 did not fit in 2 MiB.
- The deepest level also compiles the method body: about 261 KB of compiler frames. This is included in the figure above
  (see F1).

### Changes (all in `rust/tcl-vm`)
- `exec.rs`, `tick_in_original_scope`: the opcode match is split three ways, so the frame on the recursion path is small.
  - `tick_bytecode_prologue` (`#[inline(never)]`): the command-boundary bookkeeping, stale-command replay and debugger
    seams, moved verbatim.
    - It returns `Result<Rc<FunctionAsm>, Tick>` with `f.pc` already advanced.
    - The helpers that move `pc` (`enter_native_operations`, `enter_native_compiler_selection`) only do so on a path that
      returns a `Tick`, so `asm.instructions[f.pc - 1]` is the same instruction the old code used.
  - `tick_invoke` (`#[inline(never)]`): only the arms that run a command — `INVOKE_STK1/4`, `INVOKE_EXPANDED`,
    `INVOKE_REPLACE` and the compiled `next`/`nextto` forms (`TCLOO_NEXT*`) — chosen by `is_invoke_op`.
  - `tick_instruction` (`#[inline(never)]`): every other arm, unchanged.
    - Its 80 KB frame is now live only while a non-invoking instruction runs.
    - The invoke arms delegate to `tick_invoke`, so the match stays exhaustive.
- `exec.rs`, `run_activation(Box<Frame>)`:
  - `Frame` is 5 096 bytes. Debug builds kept 2–4 copies of it on every level (argument, return slot, move temporaries).
  - The frame is now boxed. `push_boxed_frame` (`#[inline(never)]`) moves it into the activation `Vec` in a frame that
    has returned before `drive` runs.
  - All 11 call sites now box (`exec.rs`, `exec/native_jim_script.rs`).
- `exec.rs`, `oo_run_method`: entry is moved to `enter_oo_method` (`#[inline(never)]`).
  - `enter_oo_method` does `enter_proc`, the compiled instance-variable links and `push_method`.
  - It returns `Box<Frame>`, or the completion that `OriginalOoActivation::finish` must finish with.
  - The cleanup order on every error path is unchanged: `retire_*` in the helper, `finish` in the caller.
  - The activation is passed as `Box<PreparedProcedureActivation>` (2 168 bytes).
- `exec.rs`, `dispatch_selected_words`: the procedure arm and the three alias arms are moved to
  `dispatch_procedure_words` and `dispatch_alias_command_words` (`#[inline(never)]`).
  - The builtin, object, ensemble and unknown arms stay inline, since those are what a method call recurses through.
  - The alias helper's impossible arm returns a host refusal, not a panic.
- `cmd_oo.rs`, `run_step_inner`:
  - Everything that finishes before the body runs now lives in `prepare_method_step` (`#[inline(never)]`). It returns
    `MethodStep::{Finished(Completion), Procedure(Box<PreparedProcedureActivation>, links)}`.
  - The intrinsic, property and forward method bodies moved out further, to `run_intrinsic_method`,
    `run_property_method` and `run_forward_method`. The logic is the same; owned values are cloned where the old code
    moved them (`invoked`, `usage_prefix`, `method`).
- No behaviour change was intended or seen.

### Regression test
`tests/cmd_oo_e2e.rs::recursion_at_the_oo_dispatch_limit_fits_a_two_mib_thread`:
- Runs on an explicit 2 MiB `std::thread::Builder` thread, so `RUST_MIN_STACK` cannot mask it.
- `obj go 20` (exactly 20 nested method calls) returns `done`.
- `obj go 21` is the catchable `too many nested evaluations (infinite loop?)`.
- Takes 39 s in a debug build, because of F1.
- Before this change, the same script aborted with a stack overflow at about depth 9.

### Results (cmd_oo_e2e, debug, libtest, `--test-threads 2`)
- Before: the binary aborts under libtest. Under nextest it was 59/71 plus 2 stack overflows (Sol-44).
- After: **62/74, no abort.**
  - `moderately_recursive_method_still_runs` and `deeply_recursive_method_errors_instead_of_crashing` pass.
  - The new test passes.
  - The 12 failures are the ones Sol-44 already listed:
    - cluster B: `NativePreflightRequired` from the harness's standalone `run_module`, in `private_instance_variable_*`
      and `retained_*`;
    - `bodyless_forward_needs_no_runtime_procedure_compiler`;
    - `tcl_oo_package_and_version_vars`;
    - six `tip558_configurable_*` tests, failing with `can't set "oo::ObjN::x": parent namespace doesn't exist`
      (inside `native_properties::invoke_default`, which I did not change).
- I did not re-run other tcl-vm binaries before disk ran out. The change only moves code between frames, but it touches
  `exec.rs` dispatch, so the rerun set below is the first thing to run.

Gates:
- `cargo check -p tcl-vm`: 0 warnings.
- `rustfmt --check`: clean on the 4 touched files.
- clippy: not run (it needs a build). I removed one `needless_borrow` by hand.

## Findings
- **F1 (performance): a TclOO method body is recompiled on every call.**
  - `prepare_method_step` builds a fresh `Rc<ProcDef>` on each call.
  - `ensure_proc_traced` → `current_native_procedure_body` only accepts the `body_src` bytecode cache when
    `cache.procedure` is `ptr_eq` to *this* `ProcDef`. It never is, so every call lowers and compiles the body again.
  - The method record's own cache (`m.compiled_body`, also copied into `ProcDef.body`) is never consulted on that path.
  - Measured: `obj go 10` takes 12.6 s in a debug test build. At the deepest level the recompile costs 261 KB of
    compiler stack.
  - Fix options:
    - let `ensure_proc_traced` accept the method record's `compiled_body` when its stamps
      (`native_cache`, `profile_generation`, compiler, namespace, bindings) are current;
    - or keep one `ProcDef` per method record and carry the call-specific `usage_name`/`call_identity` elsewhere.
  - Not changed here: it is an owner-identity decision in the procedure-cache protocol.
- **F2**: `tick_instruction` is still an 80 KB frame. Any non-invoke opcode that runs a script synchronously still pays
  it per level. Examples: a variable-trace callback from a `STORE_*`, `CALL_FUNC1` reaching a user math function, and
  `settle_native_invoke` (101 KB) / `settle_native_dispatch` (52 KB) for natively settled `eval`/`catch $s` paths.
  - Example: a method that recurses through a write trace.
  - If those paths need the same guarantee, outline the arms that call back into the interpreter, in the same way.
- **F3**: `Frame` is 5 KB and `PreparedProcedureActivation` is 2 KB. Any by-value move of them costs that much stack in
  debug builds.

## Task 2 (cluster P): analysis only, no edits (blocked on disk)
The profile flips are in `tests/command_mutation_deopt_e2e.rs`:
- `terminal_profile_flips_fail_closed_in_catch_try_and_trace` (5 sources in one test);
- `runtime_foreach_does_not_tag_old_profile_assembly_as_fresh`.

Both register a native command that calls `vm.set_dialect_profile(tcl8.4)` on a running 9.0-core VM. Both expect
`cannot continue bytecode after dialect profile changed` (`Frame::stale_compilation_message`).

On a native core, the per-tick `NativeOperationScope` currency captures `profile` and `context`. It therefore refuses
first, with the retained host refusal `entered VM original execution context`
(`interp/native_operation_currency.rs::unavailable`). The run still fails closed; only the message differs.

These tests are not about per-release behaviour, and tclsh has no mid-run release switch, so rebuilding the VM for 8.4
does not apply. There are two options; the first is mine:
- (a) Keep the scenario, the `Code::Error` assertion and the "body ran once" assertion. Expect the currency refusal as
  the first refusal, and document that a mid-run pin change on a native core is a host refusal.
- (b) In `tick`, map a currency refusal caused only by a changed profile to the stale-bytecode message, so the
  existing text stands.

Neither option is applied or verified.

The `Repin`/`pin_context` tests in the same file change package floors under the same profile. That is the
manifest/pack mechanism, not cluster P, so they stay as they are.

Sol-44 counted 5 P failures. I could not run the binary to see which other tests make up that count.

## Task 3 (deferred upstream ports): not started
Items 2, the rest of 3, 5, 6, 7, 8, 9 and 10 are as Sol-44 left them.

## Rerun set when disk allows (one binary at a time)
- tcl-vm lib unit tests
- builtins_e2e
- cmd_control_e2e
- run_script
- command_mutation_deopt_e2e
- opcode_c_parity
- c_error_paths_e2e
- error_info_text_e2e
- uplevel_coro_e2e
- coro_error_trace_e2e
- tailcall_uplevel_e2e
- cmd_coro_e2e
- namespace_surface_e2e
- tcl-vm-cli (bin, script_errors, top_level_e2e)

Compare against Sol-44's table.
