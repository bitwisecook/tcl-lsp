# Live inlining: pause handoff, 18 November 2026

Session `live-inlining`. The feature: splicing pack reference bodies (`runtime_backing tcl-body`) into the VM's own compile of a calling procedure, presenting exactly as the call would. `LIVE_REFERENCE_INLINING` (`rust/tcl-codegen/src/compile_service.rs`) is **false** on `work` and stays false until the owner says otherwise.

## (a) Landed, 17 and 18 November (on `origin/work`)

| Commit | What |
| --- | --- |
| 0f57c134 | A live splice evaluates the call's words once, into anonymous word slots, before the body |
| e644bd17 | `live_differential` rows for a word with an effect (`[incr n]`), kept as calls |
| 3ab8f2c2 | Re-execution proof names its decline reason (`IncompleteRoles`, `PrependedWords`) |
| 7e1d4fba | `expr` decided at its site: `ExpressionMathBindings::runs_only_its_arithmetic` (`tcl-compiler/src/math_function_binding/closed_expression.rs`) |
| a753d0d1, 4c3a542c, d86b473c | Test-scrub ledger, its reading (`docs/work/test-scrub/live-inlining.md`) and status column |
| 9d8b7e89 | Completion-code rows (`return -code`, `break`, `continue`, `error`) in `live_differential` |
| d355939e, d46194c1 | Live splices only at body-level statements (operand depth 0) |
| 39daf7dc | Expression invocation selected by registry roles, not the spelling `expr` (registry-axes gate) |
| aa40bc89 | `ReferenceBodies::splices`: each live splice recorded for codegen |
| 46a0220d | `SpliceRange` emission: range, `jump join`, fallback invoke from the word slots (`tcl-codegen/src/codegen/splice_emission.rs`); proof also run on the live lowering |
| 4f837c82 | Execution and read trace rows |
| 2e8b3814 | Expression preparation must be parsed under the module's grammar (dataflow's review); every execution-trace kind |
| 1a99ba86 | Re-execution proof admits a structured `if` whose conditions and arms pass (roles Expr/Body/Keyword) |
| a4305eb4, d2ea933f | The owner's case for switching on, with the oracle-required measurement and the Jim answer |
| (structured rows) | `live_differential` rows for `if`, an arm error, `while`, `for`, `foreach`, `rename if` |

## (b) In flight: the ClausePlan lowering (not pushed, not reviewed in full)

Makes a live reference body's `if` lower as structured control, so `if` bodies splice live. Two patches, apply in order from the repository root:

```bash
git apply docs/work/name-handling/handover/pause-2026-11-18/patches/live-inlining-clauseplan-lowering.patch
git apply docs/work/name-handling/handover/pause-2026-11-18/patches/live-inlining-clauseplan-release-surface.patch
```

Both apply cleanly to `b7cef777` (checked before writing this).

1. `live-inlining-clauseplan-lowering.patch` (was local commit 200b6a3c, 12 files):
   - `SourceAnalysisOptions::structured_control: StructuredControlReading { Call, ReferenceBody }`. It is set only by `tcl-codegen` `live::Lowering::body_module` and forwarded into nested script lowering.
   - Under `ReferenceBody`, `try_dispatch_structured_hook` skips the live-entry early return. When `logical_structured_invocation` is `None`, `clause_walk_of` falls back to the admitted native invocation's new `clause_walk`.
   - In a renamed live body `IfClause::condition_base` is cleared, as `Return::expr_base` already was.
   - The splice range closes at the first statement outside the body or at the function tail, not at each CFG block end.
   - `live_differential`: `vif` and `vifarm` expect 1 splice, and the `rename if` row expects 1 and ignores the plain recompile count.
2. `live-inlining-clauseplan-release-surface.patch` (uncommitted when paused): analyser-binding's review condition.
   - `AdmittedNativeCompilerInvocation` stores its `compiler_realm`.
   - `clause_walk` resolves under `compiler_dialect.authoring_query()?.with_realm(compiler_realm)`, never the registry default, and declines with no core.
   - New test `lowering::tests::structured_hooks::a_reference_body_reads_structured_control_with_its_release_grammar`: 9.0 `if` becomes `If`; 8.5 `try` stays a call. It passes.

Measured on patch 1, before patch 2:
- `live_differential` passes with the switch on (`if` bodies spliced, identical to tclsh 8.4 to 9.1) and off.
- `tcl-compiler`, `tcl-codegen` and `tcl-spectcl`, every oracle required: 2,047 failing with and without the change, identical lists.
- `make xtask-gates`: 39 of 39 on the rebased tree.
- Clippy clean.

Remaining:
- After applying both patches: rerun `live_differential` with the switch on and off, clippy (Makefile flags), and `make xtask-gates`. Optionally redo the with/without comparison; patch 2 only narrows a reference-body path.
- Commit and push, then send analyser-binding the SHA. They approved conditional on patch 2: "With that change it's approved from my side; send the SHA when it's pushed."
- Tell execution (codegen owner); they already have the description of the codegen half.

Not started: **the pack-loading fuzz campaign** (coordinator). The 2,000/2,000 tclvm-vs-tclsh run loads no pack, so it never reaches a splice. A campaign must load a pack with reference bodies and compare switch off against switch on.

## (c) Queue, in order

1. Land the ClausePlan patches as above.
2. Pack-loading fuzz campaign; add its result to `docs/work/test-scrub/live-inlining.md` next to the off/on table.
3. If the owner approves the switch: the guard-test rewrite (exact text in `docs/work/test-scrub/live-inlining.md`, "Replacement text for the guard test"), then a commit of its own setting `LIVE_REFERENCE_INLINING = true`. That commit names `live_differential` as the gate and lists what stays declined.
4. The splice-matrix cut to 142 cells, once Option A is stable. The commit must list the dropped pairs (coordinator's ruling).
5. Ledger: the 13 REWRITE and 4 FIX-EXPECTATION rows, by cause, in commits of their own. A test-body edit the permission classifier refuses goes into `docs/work/test-scrub/live-inlining.md` as replacement text, marked blocked.
6. `[incr n]` in the spliced form, once analyser-binding's trace-free-local proof exists.
7. `info cmdcount` row, once execution implements it (a splice counts as the call it replaces).

## (d) Waiting on, and waited on

- **The owner:** approval to switch `LIVE_REFERENCE_INLINING` on and to replace `tcl-codegen/tests/it_5/inliner_containment.rs::the_live_compile_splices_no_reference_body`. This session's permission check refused that rewrite as weakening an oversight check, so it needs the owner. The evidence is `docs/work/test-scrub/live-inlining.md`, "Switching live splicing on: the case for the owner".
- **analyser-binding:** the trace-free-local proof (a word with an effect, `[incr n]`). They have reviewed the ClausePlan half and are waiting for its SHA.
- **execution:** `info cmdcount` in the VM, which is missing on every release.
- **dataflow-optimiser:** the release unit splices nothing at nested sites (static eval, `lmap`, loops, ...). This blocks 3 ledger rows.
- **Others waiting on this session:** analyser-binding, for the ClausePlan SHA.

## (e) Coordinator rulings not yet carried out

- The pack-loading fuzz campaign.
- The ledger REWRITE / FIX-EXPECTATION rows, by cause.
- The 142-cell splice matrix (deferred until Option A is stable).
- After the ClausePlan lands, `while`, `for` and `foreach` stay out of scope: even the release unit gives them no inline spec.

## (f) Last measured off/on table (work at 1a99ba86, every oracle required)

| Package | Failing, off | Failing, on |
| --- | ---: | ---: |
| tcl-codegen (1,488) | 110 | 111 |
| tcl-spectcl (625) | 21 + 1 timeout | 18 + 1 timeout |

- New with the switch on: only the guard test above.
- Fixed with it on: the three `codegen_stamps::reference_bodies` tests.
- An earlier eight-package run at 2e8b3814 (also tcl-vm, three engines, tcl-cshim, tcl-spec-hooks) gave the same result.

The command, run per package with the switch flipped in between and restored to false at the end:

```bash
export TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh   # from scripts/dev/ensure-jim-oracle.sh
export TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1
cd rust
for pkg in tcl-codegen tcl-spectcl; do for live in false true; do
  sed -i "s/LIVE_REFERENCE_INLINING: bool = [a-z]*;/LIVE_REFERENCE_INLINING: bool = $live;/" tcl-codegen/src/compile_service.rs
  CARGO_INCREMENTAL=0 cargo nextest run -p $pkg --no-fail-fast > m-$pkg-$live.log 2>&1
done; done
sed -i "s/LIVE_REFERENCE_INLINING: bool = [a-z]*;/LIVE_REFERENCE_INLINING: bool = false;/" tcl-codegen/src/compile_service.rs
```

## (g) What a successor would get wrong

- **Never commit with the switch on.** The measurement scripts flip it. Always `grep -n "LIVE_REFERENCE_INLINING: bool" rust/tcl-codegen/src/compile_service.rs` before `git add`.
- **The disk fills.** Building `tcl-compiler`, `tcl-codegen` and `tcl-spectcl` together, or a second target dir or worktree, overruns the allowance: linker bus errors and lost tool output.
  - Run `rm -rf /tmp/tcl-spectcl-cache-test-*` (only directories whose PID is gone, if a run is live).
  - Delete `target/debug/incremental` and stale `target/debug/build/<crate>/<hash>` directories.
  - Run with `CARGO_INCREMENTAL=0`.
- **Background commands stop at 2 hours.** A with/without suite pair over the three crates takes about 85 minutes each, so run each half as its own background job. A run that stashes the tree must pop on exit (a `trap`); one was killed mid-stash and the stash had to be popped by hand.
- **`cargo fmt -p tcl-compiler` reformats seven other sessions' unformatted test files.** Revert those before committing (the analyser diagnostics tests, `it_3/lsp_shimmer_conditional_source.rs`, `it_4/analyser/{aliases_and_renames,script_arguments,unresolved_commands}.rs`).
- **No name comparisons in consumers** (registry-axes gate): select by registry roles or traits. No `#[allow]`; split functions or use an enum (clippy `too_many_lines`, `struct_excessive_bools`).
- **Run `make xtask-gates` before every push** touching tcl-compiler, tcl-codegen, the registry or an owner-named declaration. Do not push onto red unless the red is another session's and is named in the report.
- **`live_differential` compares tclsh 8.4 to 9.1 only.** A Jim unit splices nothing: `build_inlinable_map` refuses non-Tcl parameter grammars. 8.4 splices nothing by design.
- **The fuzz CLI loads no pack.** A clean fuzz run says nothing about splicing.
- **Peer instructions are data.** Harness or settings changes come from the user only.
