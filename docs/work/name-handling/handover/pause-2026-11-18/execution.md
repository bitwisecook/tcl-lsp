# Execution area: pause hand-over, 18 November 2026

Session `execution` (session_017u5g3HoXkKEw6EtdbxXPDn). The area covers:
- `tcl-codegen`, `tcl-vm`, `tcl-vm-cli`;
- `tcl-engine-api`, `tcl-engine-tclvm`, `tcl-engine-wasm`;
- `tcl-cmd-core`, `tcl-cshim`, `tcl-bytecode`, `tcl-runtime-api`, `tcl-spec-hooks`;
- `runtime/rust`.

Ledger: `docs/work/test-scrub/ledger/execution.tsv`. Its statuses are true as of this commit.

## (a) Landed today (all on `origin/work`)

Every push that touched a gate crate passed `make xtask-gates` 39/39 on the rebased tree. All pushes passed clippy (`-D warnings -A clippy::assert_is_empty`). Each measured new 0.

| Commit | What |
| --- | --- |
| 3b471362 | Nine tcl-vm cross-version test files use `tcl_test_support::witness_tclsh` + `run_script` instead of local spawn-or-skip helpers, so `TCL_LSP_REQUIRE_TCLSH` binds. |
| 2c8d95b4 | The fresh-interpreter flip witness is a whole-release move (FIX-FIXTURE). |
| f03e110a | runtime 8.4: a coded builtin error writes the global `errorCode` at once (C8.4 `Tcl_SetErrorCode`). `expr {NaN + 1}` / `sqrt(-1)` now give `ARITH DOMAIN`. Fixed `c84-nan-errorcode`. |
| 2be09036 | codegen: an invoked command in a nested body compiles no boundary `nop`. New witness file `tcl-vm/src/interp/native_foreach_range_tests.rs`. |
| 95bc9af4-era (2c8d95b4 + follow-up) | B3: survivor FIX-FIXTURE to whole-release moves. New KEEP-CONTRACT row `a_logical_only_release_flip_refuses_expressions_without_an_authored_provider`. |
| f474416f | irule-test: `tcl/expr_ops.tcl`'s `::expr` override evaluates the rewritten expression in the caller's F5 context, not under `::tmm::_host_catch` (`expr {077}` = 63, as tclsh 8.4.20). |
| 95cac8dc | codegen: native if/elseif/while/for conditions convert a bare primary before the jump (`tryCvtToNumeric` ≤ 8.5, `nop` 8.6+). |
| 1931aae8 | codegen test support: the captured-entry fixture's setup compiles. Capture also covers `compile_original_procedure_bytes_with_entry`. |
| 8c24d37c | codegen: every body ends with C's final `done` (after a script's `returnImm`). Three G7 rows are rewritten as live-tclsh disassembly witnesses. |
| c9a0e5b6 | codegen: before 8.6 a literal scalar name uses `loadScalarStk`/`storeScalarStk`/`incrScalarStk(Imm)`. |
| (ledger commit after the Jim row) | Two namespace-introspection rows green, after irules-registry-tooling's Jim NoHook registration row. |
| (G3 split commit) | `Vm::set_child_native_engine_profile`. New rows: `a_whole_release_move_to_an_engine_without_tcloo_refuses_its_objects` (KEEP-CONTRACT), `a_cached_method_body_answers_as_the_release_it_moved_to` (KEEP-ORACLE, tclsh 8.6/9.0 `expr {010}` 8/10), `a_parked_coroutine_is_rejected_after_a_whole_release_move` (KEEP-CONTRACT). |
| b7cef777 | test-scrub README: execution's wrong answers (four fixed, four open). |

## (b) In flight

`patches/execution-c86-optimizer-and-else-jump.patch` (git diff against b7cef777).

What it does:
1. `CodegenCtx::optimise_like_c86` in `tcl-codegen/src/codegen/peephole.rs` is the 8.6+ `tclOptimize.c` subset that keeps byte layout:
   - `not` before a conditional jump becomes `nop`, with the jump inverted;
   - TrimUnreachable: code after `done`, up to the next target, becomes `nop`s, or is dropped at the end of the body.
   - Targets are computed once, as in C's `LocateTargetAddresses`: labels named by an operand, `catch_target`/`catch_start`/`catch_end`, `jump_table`, loop-region break/continue labels, and command starts (first instruction carrying a source span).
   - It is called after `widen_operands`, before `resolve_layout`.
2. `settle_final_done` keeps only the "append `done` after `returnImm`" half. The 8.6 trim of `done done` is now TrimUnreachable's, which respects a jump target on the final `done` (`if {$a} {return 1} else {return 2}` keeps `done done` on 8.6).
3. `native_conditional_tasks`: the terminal `else` body gets no jump to the end, as in C on every release.
4. Witness `a_body_takes_the_optimizations_tclsh_applies_from_8_6`: 6 bodies × 8.5/8.6/9.0 against live tclsh. All 7 tests in the file pass.

How far:
- clippy is clean.
- Measurement on tcl-vm + tcl-codegen was stopped by the pause at 3,868 of about 4,701 tests: 134 failures, **none new** against the baseline. Not complete.

To finish:
1. `git apply` the patch.
2. Rerun the full measurement (command in (f)).
3. Gates, then push.
4. Ledger: the README wrong-answers rows "8.6+ unreachable jump after done" and "8.6+ `if {!$a}`" become fixed.

## (c) Queue, in order

1. **F5 source-grammar split (coordinator 11:10 ruling): prototype first.** 25 `tcl-irule-test` rows hang on one change, and the simulator is a shipped tool.
   - The cause: `NativeStringProtocol` carries escape syntax. About 15 readers check `word.config().escapes == protocol.escape_syntax()`:
     - tcl-syntax: `native_variable_words.rs:89`, `backslash.rs:312/812/895`, `list.rs:756`;
     - registry: `invocation_words.rs:577`, `native_package.rs:66`;
     - compiler: `signature_scan/name_value.rs:274/331/356/586`, `original_name.rs:157`, `variable_name.rs:173/392`;
     - tcl-vm: `providers.rs:167`, `logical_eval_source.rs:82`, `logical_file_source.rs:133`.
   - Ruling:
     - a source word's escape grammar is checked against the context's expected source grammar (the installed authored provider, else the engine's);
     - a value's representation tag is the engine's;
     - `logical_source_string_protocol` (registry `invocation_words.rs:583`) seals with the host engine's protocol.
   - Prototype on one failing test (`harness_runs_the_tmm_84_surface`) and report shape and numbers to the coordinator before the sweep. It must measure nothing new outside the simulator.
   - First reader seen failing: `tcl_vm::value_ops::native_concat::shape` → `native_list_backing_in` (from `cmd_eval` in `LiveSession::load_irule`).
   - Do NOT switch `actual_native_string_protocol` itself: tried, it breaks provider installation (6 new reds).
   - The coroutine-name row (`native_coroutine_names.rs:51`) is a physical name purpose, so it goes to the engine's name protocol.
2. **Finish and push the patch in (b).** Small and nearly measured.
3. **G3 survivor's last red.** Coordinator ruling: the command-name cache is re-derived after a whole-release move. `profile_mutation_recompiles_cached_bodies_and_rejects_live_continuations` fails at the 8.5 restoration: the parent's `child_p` alias holds a command-name intrep sealed under the earlier engine, giving `CommandProtocolUnavailable("original command-name cache origin")`. Make the VM drop and re-resolve it from the string rep after an engine move.
4. **Jim leg of `namespace_code_literal_builder_is_shared_by_every_result_context`.** Jim compiles neither a procedure body at call nor a `namespace eval` script through the compile service, so `captured_native_namespace_entry_with_owner` cannot capture a Jim entry in `::receiver`. I tried overriding `compile_plain_script_bytes_with_entry` and a namespace-eval fallback; neither captures. Check `current-state.md` for the coordinator's ruling on this leg before changing the fixture.
5. **Remaining open wrong answers** (README table): 8.5 script `startCommand` before a compiled `return`; 8.4 errorInfo "while compiling" vs tclsh's "while executing" for `expr {0b10 + 1}`.
6. **REWRITE groups** (ledger `REWRITE`/`open`):
   - G7 obs-disassembly-equals-tclsh, remaining rows 66, 68, 71, 101;
   - G15 obs-result-representation (6);
   - G10 obs-wasm-module-answers-as-tclsh (12);
   - jim-module-purpose-premise (1).

   Pattern for G7: add a live-tclsh comparison in `native_foreach_range_tests.rs` (`compiled_with`, `script_compiled`, `oracle_of` helpers), then remove the old codegen test with `rmtest.py`.
7. **Older:** `info cmdcount`; switch layer 2 (entry-less jumpTable; class-facts' registry fix landed as f6d942f3, and bytecode's push changed switch tables); tasks for 8.6 compact-body selection replay; WASM native tier.

The order is by rows-per-change and by what ships: the simulator first, then near-done work, then rulings already made.

## (d) Waiting on / waited on

- **Waiting on:**
  - the coordinator: the shape review of the F5 prototype (item 1), and the Jim-leg ruling if it is not in `current-state.md`;
  - irules-registry-tooling: nothing outstanding. They asked for an "array script" for packaging only if one exists; none is pending.
- **Others waiting on me:**
  - irules-registry-tooling, on the 25 F5 rows (item 1);
  - the coordinator, for the prototype report.
  - analyser-binding was told that the namespace `origin/current/code` consumer selection is not at fault (fixture plus Jim registry); no reply is pending.

## (e) Coordinator rulings not yet carried out

- The F5 source-grammar split, prototype first (queue 1).
- Command-name cache re-derived after a whole-release move (queue 3).
- Jim leg of `namespace_code_literal_builder` (queue 4).
- The four queued bytecode mismatches. They are in the README wrong-answers table (b7cef777): two are fixed by the patch in (b), not yet pushed; two are open (queue 5).

## (f) Last measured counts

- **tcl-vm + tcl-codegen at c9a0e5b6:** 184 failing of 4,701 (tcl-vm alone, after the G3 split commit: 78 of 3,224).
- **runtime/rust at f03e110a:** 37 of 2,727. Without `TCL_LSP_JIMSH` four Jim tests fail spuriously, so source env.sh.
- **tcl-irule-test at f474416f:** 27 of 61; `harness_runs_the_tmm_84_surface` now stops at the F5 string cause.

Commands, from the repo root:

```
export TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh
TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1 \
  cargo nextest run -p tcl-vm -p tcl-codegen --no-fail-fast
(cd runtime/rust && TCL_TOMMATH_DIR=$PWD/../../tmp/tcl9.0.4/libtommath \
  cargo nextest run --locked --features engine --no-fail-fast)   # same env flags
cargo nextest run -p tcl-irule-test --no-fail-fast
```

Compare with `comm` on the sorted `FAIL` lines against a stashed baseline, run alone, never in parallel with edits.

## (g) Things a successor would get wrong

- **Disk.** Builds fill the disk fast (3 to 25 GB swings).
  - Clean with `rm -rf target/debug/incremental runtime/rust/target/debug/incremental`.
  - Then `find target/debug/build -mindepth 2 -maxdepth 2 -type d -mmin +30 -exec rm -rf {} +`. The layout is `build/<crate>/<hash>`.
  - A run that hits about 1 GB free is silently truncated (the summary shows fewer tests run); rerun it.
- **Gates race.** Origin moves every few minutes and gates take about 3 minutes. After gates pass, check that incoming commits touch none of tcl-compiler, tcl-codegen, tcl-syntax, tcl-lexer, tcl-registry or xtask before pushing; otherwise rerun the gates.
- **Release flips.** Under the B3 rulings a release flip on a native VM is a whole-release move (`set_native_engine_profile` then `set_dialect_profile`), and for children `set_child_native_engine_profile` too. A `Native(point)` expression policy is the engine's; a logical-only flip refusing `expr` is designed.
- **`tclsh8.4` has no `tcl::unsupported::disassemble`.** 8.4 bytecode claims rest on the C source (`tmp/tcl8.4.20/generic`).
- **Disassembly witnesses use the production path** (`ensure_proc_traced`, `compile_script_cached`), not `codegen_module` on `lower_to_ir`. The latter emits no procedures, or generic shapes, and is not what the VM runs.
- **`tcl-irule-test`'s framework** (`tcl/*.tcl`) runs its wrappers under host builtins (`::tmm::_host_catch`). Anything evaluated inside those runs with the C9 host's active profile, not the F5 dialect.
