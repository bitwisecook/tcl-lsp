# Hand-over: bytecode (#2490, #2542, #2544, #2545)

Written 18 November 2026 by the `bytecode` issue session (`session_017u5g3KBvRVWYHVoF5FSZmT`), paused on the owner's instruction. A fresh session in a clean container continues from this file alone.

## Issues

- **#2490** (braced `[expr {…}]` compiled inline, `startCommand` placement): closed, completed. Commits: 61edc359 (no `startCommand` under 8.4), 08fccbc8 (joined compiler selections: nested commands starting on one marker share it, a stale one replays the outermost), d0575d5a and 0ba17bb2 (an enclosing command where C has no marker rides on the nested start as `Instruction.start_hosts`). Closing comment 6953432126.
- **#2542** (Tcl 9.1 operand widths): closed, completed. Commits: 2f0c68b2 (`Op::wide_form`, four new 9.1 `incr` ops, widening after jump shortening under `TclVersion::emits_wide_instructions_only`), e979b915 (capture script and skill cover 9.1), 850ac5ca (9.1 names `lreplace` flags).
- **#2544** (9.1 `try` and `subst` through `jumpTableNum`): closed, completed. Commits: 72a785805 (a `try` with `finally` emits the `-` bridge, last jump and pop only where C does; changes 8.6/9.0 towards tclsh), 1f3364f18 (a marker hands over to its own command's start: no pc-0 marker on a declined `try`), ef1f95bb2 (`Op::JUMP_TABLE_NUM`; 9.1 `try` and `subst` dispatch through it; no `returnCodeBranch` under 9.1). Closing comment 6976930986.
- **#2545** (codegen corpus against tclsh): open. Title carries the scoreboard. Triage table: comment 6976452208; follow-up with the invalidated counts and the items deferred from #2544: comment 6976935184.

### #2545 scoreboard

`tcl dis --dialect tcl<release>` against the captured references, out of 72:

| | 8.5 | 8.6 | 9.0 | 9.1 |
|---|---:|---:|---:|---:|
| on `work` (82cece48e) | 62 | 63 | 64 | 63 |
| with the WIP on `issues/bytecode-3` | 62 | 65 | 66 | 65 |

### #2545 remaining groups

Each with tclsh's listing (9.0 unless named) and ours, and the fix.

1. **Top-level command markers** (`format-literal`, `dict-create-fold` on 8.6+; `expr-add`, `multi-stmt`, `if-else`, `while-simple`, `for-body-break` on 8.5). `set z [expr {$a + $b}]` on 8.5.19: `push1; startCommand +16 1; push1; loadScalarStk; …`; ours had no marker. `set s [format {%d+%d=%d} 1 2 3]` on 9.0.4: `push1; push1; storeStk; done`; ours had `startCommand` at pc 2. Cause: a static compile (no live entry) never ran the placement pass and never compacted. Fix in progress on `issues/bytecode-3` (`settle_command_starts`): a named release without a live entry compacts where `native_compiler_opcodes_compact` says (C's `IsCompactibleCompileEnv` for a root interpreter without limits, as `disassemble script` compiles) and otherwise places markers by C's `atCmdStart` rule. It fixes the 8.6+ rows; the five 8.5 rows still differ and need reading (8.5 has no compact pass and its `atCmdStart` handling is in `tclCompile.c` 8.5.19 around line 1425).
2. **Ensemble subcommand calls** (`array-names`, `array-for`, `dict-set-ensemble`, `namespace-eval`, 8.5–9.1). tclsh invokes the implementation: `array names $arr` is `push "::tcl::array::names"; push "arr"; loadStk; invokeStk1 2`; `dict set d k v` is `push dict; push set; push d; push k; push v; push "::tcl::dict::set"; invokeReplace 5 2`; ours invokes the ensemble generically (`push array; push names; …; invokeStk1 3`). Fix: the ensemble rewrite C does in `TclCompileEnsemble`/`CompileToInvokedCommand`; check which subcommands are compiled vs rewritten per release.
3. **`linsert` before 9.0** (`linsert`, 8.5 and 8.6). tclsh 8.6.18: `push1; loadStk; push1; list 1; over 1; listRangeImm 0 1; reverse 3; listRangeImm 2 end; listConcat; listConcat; done`; ours emits 9.0's `lreplace4 3 2`. Fix: a release fact for `TclCompileLinsertCmd`'s shape before 9.0.
4. **`list` of expansions** (`list-expand-concat`, all releases). tclsh: `push; push; loadStk; push; loadStk; listConcat; storeStk`; ours: `startCommand; expandStart; …; invokeExpanded`. Fix: `TclCompileListCmd`'s all-expanded case.
5. **`continue` in a `for` body** (`for-continue-runs-next`, all releases). tclsh: `jump1 +13; jump4 +6; pop; push1 0; incrStkImm…`; ours: `jump1 +12; jump4 +5; push1 0; …` (no `pop` at the continue target). Fix in the `for` emitter's continue target.
6. **8.4**: unmeasured (see the 8.4 plan below).

### Deferred to #2545 from #2544 (not in the corpus; add a snippet for each when fixing)

- Our 8.6/9.0 `try` uses shorter jumps than C in places (`jumpFalse1` where tclsh 8.6.18/9.0.4 print `jumpFalse4`).
- Our 8.6/9.0 compiled `subst` lacks C's leading `jump start; jump end` break trampoline (9.1 has it, from ef1f95bb2).

### 8.4 plan

`scripts/capture/bytecode_84.sh` reads `tcl_traceCompile 2`, which prints only in a Tcl built with `TCL_COMPILE_DEBUG`; neither `~/.local/bin/tclsh8.4` nor `tmp/tcl8.4.20/unix/tclsh` is. Build one in scratch: copy `tmp/tcl8.4.20`, `cd unix && ./configure && make COMPILE_DEBUG_FLAGS='-DTCL_COMPILE_DEBUG -DTCL_COMPILE_STATS'` (the `COMPILE_DEBUG_FLAGS` line in `unix/Makefile`, commented out by default), then `./scripts/capture/bytecode_84.sh <that tclsh>`. Record the build steps in the capture script's header so it can be repeated. Note that the script wraps each snippet as a procedure body `_snippet_`, so its references are procedure bodies, not top-level scripts.

## On `work`

This session's commits since 17 November, newest last:

- 61edc359 Emit no startCommand under Tcl 8.4
- 08fccbc8 Join a nested command's compiler selection to the startCommand it shares
- d0575d5a Omit the first startCommand where a nested command starts with it
- 0ba17bb2 Carry an enclosing command that starts where C has no marker as a host
- 2f0c68b2 Compile Tcl 9.1's four-byte operand forms under the tcl9.1 profile
- e979b915 Compare bytecode against tclsh 9.1 in the capture script and skill
- 850ac5ca Name lreplace's flags as tclsh 9.1 disassembles them
- e21d3aa1a An opaque foreach or lmap writes what its body writes (#2385)
- 861386277 Expand a foreach with a qualified binder in the analysis CFG (#2385)
- 72a785805 A try with finally emits a handler's entry bridge, jump and pop only where C does
- 1f3364f18 A command marker hands over to its own command's start
- ef1f95bb2 Tcl 9.1 dispatches try handlers and subst codes through jumpTableNum
- 945d9446c Capture tclsh's procedure bodies as the parent interpreter compiles them
- 6b84e9b57 Disassemble dict update's and dict with's local as a slot
- 738313739 Name unset's complain flag as tclsh 9.1 disassembles it
- 6caba173d Name a literal scalar on the stack before 8.6 for the target release
- 82cece48e Keep a declaration's result where the release has no optimiser

## Only on branches

- `issues/bytecode-3` at 7a7f1434a: "WIP: settle command markers for a static compile of a named release" (group 1 above). Unverified: no suite, clippy or gates run. It rebases onto `work` as one commit; touches `tcl-codegen` `emitter/generate.rs` and `native_compiler_pass.rs`, and `tcl-registry` `native_compiler_pass.rs`.
- `issues/bytecode-2`: superseded; everything on it is on `work`.

## How to measure

- References: `export PATH=$HOME/.local/bin:$PATH; ./scripts/capture/bytecode.sh` writes `tests/bytecode_reference/{8.5,8.6,9.0,9.1}/` (git-ignored; about a minute). 945d9446c fixed the capture: it used to disassemble procs in a safe child interpreter, which from 9.0 has no `tcl::unsupported::disassemble` (so 9.0/9.1 references had no procedure bodies: the "extra done" rows), and a child interpreter never takes C's compact pass (`Tcl_GetParent(interp) == NULL`, `tclCompile.c`). Each snippet now runs in its own process's interpreter; only the procs it defines are emitted, in name order. Every count given before 945d9446c (21/51, 23/49) is invalid.
- Ours per release: `tcl dis --dialect tcl<release> <snippet.tcl>` (`cargo build -p tcl-cli --bin tcl`, then `target/debug/tcl`). The committed goldens are dialect-blind (`tcl8.6` registry, no profile) and `codegen_golden::rust_codegen_matches_golden_corpus` fails in the baseline; do not compare against them.
- Scoreboard: a small Python loop over `bytecode_compare.py`'s helpers, run with `python3 -I` from the repository root: `sys.path.insert(0, '.claude/skills/bytecode-compare')`, then for each `bc.snippet_names()` and release compare `[bc.normalise_instruction(i) for i in bc.extract_instructions(text)]` of `tcl dis --dialect` output against `bc.load_disasm(bc.ref_dir(v) / f'{name}.disasm')`. The skill itself: `python3 .claude/skills/bytecode-compare/bytecode_compare.py -v <8.5|8.6|9.0|9.1> summary|diff <snippet>`; `-v 9.1` uses `tcl dis --dialect tcl9.1`, other releases use the goldens.
- tclsh skeletons for a body: `proc t {x} $body; foreach l [split [tcl::unsupported::disassemble proc t] \n] {if {[regexp {^\s+\((\d+)\) (\S+)} $l -> pc op]} {lappend out $pc:$op}}`.
- Suites: `source scripts/dev/agent-build-env.sh; export CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0; cargo nextest run -p tcl-bytecode -p tcl-codegen -p tcl-vm -p tcl-registry -p tcl-explorer -p tcl-cli --no-fail-fast --status-level fail --final-status-level fail --failure-output never`. Failing names: `grep -E '^\s+(FAIL|SIGKILL|SIGSEGV|SIGABRT|TIMEOUT) ' log | awk '{print $NF}' | sort -u`; compare with `comm -13 base new`. Cold build about 25 minutes, run about 6 minutes, rebuild after a codegen edit about 7 minutes. Baseline at a50d3b1e4: 230 failing of 7,643.
- For a `tcl-compiler` change: `-p tcl-compiler -p tcl-codegen` with `TCL_LSP_AGENT_INCREMENTAL=0 CARGO_BUILD_JOBS=3` (a second selection with incremental on fills the disk); about 10 minutes to run.
- Before a push: clippy on the touched crates with `-D warnings -A clippy::assert_is_empty` (check its exit code, not a grep: an `&&` after a grep let one failing clippy through), `make xtask-gates` (39 gates, about 4 minutes), `cargo check --workspace` after the final rebase, then `git merge-base --is-ancestor <sha> origin/work`.

## Rulings in force

- For #2545, changing existing 8.5–9.0 output towards what tclsh prints is wanted, construct by construct, each with its tclsh listing and a VM run. Release differences are registry facts read by codegen, citing `tmp/tcl*/generic/`.
- Announce every change in `tcl-codegen` or `tcl-vm` to `execution` (session_017u5g3HoXkKEw6EtdbxXPDn) and `live-inlining` (session_017u5g3KCTRHzSduK23yDtc2) before the push. 6caba173d and 82cece48e were pushed before `execution` was told; it was told afterwards.
- #2385 (a `foreach` with a `::`-qualified binder miscompiled by O100): fixed by e21d3aa1a (the opaque `foreach`/`lmap` call defines and reads what its body may write) and 861386277 (analysis builds expand a qualified binder; codegen keeps it opaque). #2385 stays closed; its tests are `release_optimiser_safety::issue_2385` and `issue_2385_loop_bodies_keep_their_writes`.
- A test that turns green is marked in its `docs/work/test-scrub/ledger/` row in the same push.

## Dead ends

- Comparing against the committed goldens: they are dialect-blind and stale.
- Measuring 8.4 with the toolchain's tclsh: `tcl_traceCompile` prints nothing.
- Changing a test's expected 9.0 skeletons to exact widths for #2544: our 8.6/9.0 jump widths differ from C's; that is #2545 work, and the #2544 test compares 8.6/9.0 without width suffixes by ruling.

## Next steps, in order

1. Verify and land `issues/bytecode-3` (group 1): run the suites, clippy and gates; then read the five 8.5 top-level rows against `tclCompile.c` 8.5.19 and fix them in a following commit. Add a `tcl dis` witness in `rust/tcl-cli/tests/compile_verbs.rs`, where the 8.5 witnesses are.
2. Ensemble subcommand calls (group 2).
3. `continue` in `for`, `list` of expansions, `linsert` before 9.0 (groups 3–5), and the two items deferred from #2544, each with a new corpus snippet.
4. 8.4: the debug build and its references.
5. The gate: committed references (no tclsh at test time) and a test over the corpus and releases, as the issue's second "Done when" item asks; it can land only once every row matches or has its reason and pinning test on the issue.
6. Keep the issue title's scoreboard current with each landing.

## Hazards

- Disk: about 5 GB free with a full `target/`. A second package selection with incremental builds fills it; remove `target/debug/incremental` when it does. `rm -rf target` before a fresh baseline frees about 20 GB.
- `/tmp/tcl-spectcl-cache-test-*` directories from `tcl-spectcl` tests leak and can fill `/tmp`; delete them between runs.
- `work` moves about 100 commits an hour: fetch, rebase, check and push in quick succession, and re-run the focused tests on what landed.
