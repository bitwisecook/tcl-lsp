# Hand-over: bytecode (#2490)

Written 17 November 2026 by the outgoing session, paused since 16 November, as a message to the orchestrator and filed here unedited. The successor is the `bytecode` issue session.

The paths and commits below predate the file split of 16 November: see *Starting* in `../README.md` before applying anything.

```text
HANDOFF — session_017u5g3LQRXsvGjGUg7g3tgp, group "bytecode issues" (#2207, #2490)

1. ISSUES
- #2207 (catch / info exists inlined only in assignment position): CLOSED as completed. Evidence from tclsh 8.5.19–9.1.0 is in comment 6901093861.
- #2490: OPEN. The `tcl dis` half was done earlier (static fold shows `syntax`; inline expr/if). Placement progress is in comment 6904298430. Left before it can close:
  (a) Two commands starting on one instruction. Ours `I S1 N S1 S1`, C `I S3 S1`, for `puts $x; set y [expr {[string length $x]+1}]; return $y`. Also the 8.5 compactible pc-0 nested case `set a [expr {$x+1}]; set b [string length $a]` (C `S2`; compact `""` from 8.6). Plan in §6.
  (b) Under 8.4 (no INST_START_CMD): fixed in c90b3510, which is on the branch only and not yet verified for `work`.
  (c) 9.1: tclsh 9.1.0 blanks `push`+`pop` to SIX nops because 9.1 deprecates every one-byte-operand opcode (tclCompile.h:666 DEPRECATED_OPCODE(INST_PUSH1); INST_PUSH = 2). Our 9.1 profile still emits 8.6-width ops (PUSH1, JUMP1…), and no width mechanism exists. That's a separate encoding design change; post it on #2490 as a blocker or split it into its own issue. The optimiser test deliberately stops at 9.0.

2. BRANCHES / COMMITS
- On `work`: 96799b39, daf8da6e, 2bae1b38, 13b851d8, f908ba05 (#2207 and earlier #2490 work), plus 16c18f36 (place live startCommand like C's atCmdStart; layout test) and 478ef476 (8.6+ optimiser blanks a discarded push; placement after the fold).
- Only on `issues/bytecode`: c90b3510 "Emit no startCommand under Tcl 8.4" (parent 478ef476 = work as of 0e42e0ab+2). The tree is clean and pushed.

3. VERIFICATION
- Env: `export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`, dev profile only. On ENOSPC, `rm -rf target/debug`.
- Suites, each with `cargo test -p <crate> --lib --tests --no-fail-fast` for tcl-registry, tcl-codegen, tcl-vm, tcl-explorer, tcl-cli:
  - also `cargo test -p tcl-explorer --features live-entry --lib --tests --no-fail-fast`;
  - and `cargo test -p tcl-compiler --lib --tests --no-fail-fast -- --skip depth_guard::tests::the_source_walk_cap_fits_its_stack_budget`.
- Baseline: `grep -E '^test .* FAILED$' log | sed 's/^test //;s/ \.\.\. FAILED//' | sort -u`, then `comm -23`.
  - The baseline files were in /tmp and are lost. Regenerate by running the same commands at the parent SHA (git checkout <sha>; run; checkout back). To check one new failure, run that single test at the parent commit.
- Pre-push checks:
  - `rustfmt --edition 2024 <files>`;
  - `cargo clippy -p tcl-registry -p tcl-vm --lib --tests -- -D warnings -A clippy::assert_is_empty`, and `-p tcl-codegen --lib --tests` the same way. Codegen lib tests build now that 18055039 is in;
  - `cargo xtask registry-axes`, `cargo xtask value-transfers --check`, `cargo xtask owner-resolution`, `cargo xtask callback-inventory --check`.
- 478ef476 (as 6aa1625d before the rebase): full suites, no new failures. Clippy and the 4 xtasks passed. Codegen LIB tests could NOT build then (FrameReach), so 16c18f36 and 478ef476 have never been checked against codegen lib tests.
- c90b3510 full-suite run, compared with the old baselines and NOT yet triaged. New failure names:
  - registry: taint::tests::a_getter_called_with_only_its_own_flags_keeps_its_colour. Probably from upstream fcd7b971/taint.rs; unverified.
  - vm: cmd_thread_e2e::{mutex_serialises_a_non_atomic_read_modify_write, rwmutex_write_lock_guards_a_non_atomic_increment, thread_id_and_platform_flag, tsv_counter_is_atomic_across_threads, tsv_get_missing_key_errors_but_with_var_reports_presence}. Thread e2e tests that may depend on the environment; unverified.
  - compiler: taint_depth::interproc_depth::sanitising_helper_result_via_var_clears_t101. Likely upstream taint work; unverified.
  - codegen: 189 failing in total, now including lib tests. There is NO lib baseline: run codegen lib at 0e42e0ab and compare.
  - Known pre-existing: cli diag_reads_the_jim_program_under_jim_and_refuses_it_under_tcl_8_6, compiler it_4 caller_frame_effects::global_aliases_write_the_global_variable, codegen it_3 release_optimiser_safety::issue_2218_preserves_known_command_heads (fails on origin/work 0e42e0ab), tcl-runtime --all-targets tcl_test_support unresolved.
- Key focused tests:
  - `cargo test -p tcl-vm --lib native_command_start_layout` (3 tests: the C layout skeletons over 8.5–9.1; tcl_8_4_bodies_have_no_start_command; the_optimiser_blanks_an_empty_for_start_from_8_6);
  - `cargo test -p tcl-codegen --test it_2 empty_` (empty_init_clause / empty_step_clause).

4. DESIGN DECISIONS
- `place_command_starts` (tcl-codegen/src/codegen/native_compiler_pass.rs) runs in generate.rs `finalize_function`, only when native_entry is set, AFTER fold_const_push_pop_nops.
  - Rule: a marker before each compiled command; none for a generic invoke; none at pc 0; a marker straight after another adds its count to the earlier one; none in the compact second pass.
  - An omitted marker gives its metadata (selections, entered, cos, boundary) to the next instruction of the same command (`receives_marker`); otherwise it stays.
  - `collapse_repeated_starts` merges duplicate same-text starts.
  - Selection NOPs at index>0 are materialised to START_CMD [Label(end), Imm(1)].
  - The generic tag counts only when the marker carries no selections.
- 8.6+ optimiser: registry fact `tcl_registry::native_compiler_pass::native_optimizer_blanks_discarded_pushes(version) = version >= 8.6` (TclCreateInterp installs TclOptimizeBytecode from 8.6). The fold now also blanks `push ""; pop` under it unless the pop is a label target. The first nop keeps the push's metadata so the pc-0 marker can hand over.
- 8.4: reuses `TclVersion::revalidates_running_bytecode()`, whose doc ties it to INST_START_CMD. `omit_release_markers` removes every START_CMD: it hands over when possible, else converts it to a NOP that keeps the metadata. Our VM already gives 8.4's answer to the redefinition probe (2, vs hijack on 8.5+), and the test pins that.
- Release keying: profile != tcl8.5 → compact. tclsh skeletons are identical for 8.5–9.1.

5. DEAD ENDS
- Running placement BEFORE the fold: it sets no_fold through the handed marker and breaks the empty for-clause nops.
- Incrementing the count when merging a selection of the same command: wrong. Only add the count when source_cmd_text differs.
- Tagging catch/list starts as generic dropped needed markers.
- Rewriting the fold with Instruction::new loses metadata and leaves a pc-0 START.
- rustfmt on runtime/ files recurses into submodules: format individual files only.

6. NEXT STEPS
1) Triage the c90b3510 run. Run codegen lib at 0e42e0ab to get a baseline, and check the new names at 478ef476. If clean, rebase onto origin/work, rerun the focused tests, clippy and xtasks, then `git push origin HEAD:work`. work moves often: fetch, rebase, push in quick succession.
2) For (a), "joined compiler selections", following C's semantics: one combined marker = one epoch check, and on stale C re-evaluates the OUTERMOST command from that pc.
   - Add `Instruction.joined_compiler_selections: Vec<NativeCompilerSelectionSite>` (tcl-bytecode lib.rs ~1512).
   - In place_command_starts, when merged && the marker has a selection && no entered/cos, drop the marker and push its site into the host START. Optionally guard with outer text containing inner text.
   - VM exec.rs `enter_native_compiler_selection` (~5674) must require the primary AND all joined prerequisites. On failure it replays the host's source_cmd_text and continues at the host's end (the primary site.end, else the START_CMD label0).
   - Also include joined sites in the chunk-entry prerequisite collection (~2283) and in closed_native_execution/owned_application.rs:346.
   - Then add `puts $x; set y [expr {[string length $x]+1}]; return $y` → "I S3 S1" to CASES.
3) (c) needs a design decision; comment on #2490.
- Pitfalls: the full suites take about 2h, so don't edit the tree while they run. Disk is about 12G free with target at 14G. tclsh8.4–9.1 are in ~/.local/bin; 8.4 has no disassemble. Comments must end with the Generated-by footer; commits use Refs #2490 and the Co-Authored-By / Claude-Session trailers.
```
