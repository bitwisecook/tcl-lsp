# Coordinator handover for the 19 November 2026 pause

The owner paused the Tcl naming and shared-parser burn-down on `work` during the 14:00 UTC hour. All seven decisions from the 18 November pause were answered on restart. The thirteen areas now belong to six Codex replacement workers. The original Claude workers, their retired predecessor group and the separately paused issue sessions have no assignments from this coordinator.

## Start here

Read [current rulings and scoreboard](../../current-state.md), [scrub method and wrong answers](../../../test-scrub/README.md), [parser plan and sign-off](../../parser-crate-plan.md) and [successor rules](../successors.md). The current area handovers below supersede the [18 November pause](../pause-2026-11-18/coordinator.md). The earlier handovers retain original reproducers and patch provenance.

| Replacement worker | Area handovers |
| --- | --- |
| analyser_binding | [analyser-binding](analyser-binding.md), [class-facts](class-facts.md) |
| dataflow_optimiser | [dataflow-optimiser](dataflow-optimiser.md), [values](values.md), [taint](taint.md) |
| diagnostics | [diagnostics](diagnostics.md), [tk-oo](tk-oo.md) |
| execution | [execution](execution.md), [live-inlining](live-inlining.md) |
| lsp_features | [LSP features](lsp-features.md), [server, E2E and DB](server-e2e-db.md), [analysis-performance](analysis-performance.md) |
| irules_registry_tooling | [iRules, registry and tooling](irules-registry-tooling.md) |

Use a fresh `origin/work` on branch `work`. Reuse an available replacement worker or create a new subagent for its area. Area workers write the implementations. The coordinator rules on cross-area questions, records decisions immediately, points affected workers to the file, keeps the scoreboard and integrates work. Keep the original issue sessions paused. Open a PR only upon an explicit owner request.

## Measurements and next work

| Area | Evidence on a named head | Next action |
| --- | --- | --- |
| Compiler and LSP baseline | On clean `a6c66517c7` the compiler listed 13,166 tests. 11,289 passed, 1,871 failed, six skipped. Core LSP integration failed 26/294, 25/289, 47/322 and 81/349 for `it_2` through `it_5`, zero skipped. Registry passed 2,467/2,467. Codegen `it_3` passed 254 and failed 25. | Establish a new clean baseline for any claimed change in counts. Keep target, revision, failures and skipped/filtered cases separate. |
| O129, I230 and O101 | On `8d6ae9adde` the named O129 split regression passed. The synchronous I230 assertion passed before a fixture lookup failed. Fixing the lookup exposed a real callback defect: 12 wrong advisory claims out of 24 cases. All six interpreters report the caller-visible write. Strict SSA retained both branches. O101 still rewrites `no/yes` to `no/no`. | Run the named tests on resumed head. Fix callback frame effects and check strict SSA separately. Verify the declared-frame experiment against 65 clean passing compiler controls, six clean failing compiler controls and the separate LSP indentation case. |
| O103, strict rename and source edits | On `c989476b24`, latest Jim changes 99 to 42 if an external entry rebinds `answer` before `f` runs. All five C releases preserve 99. The six no-op controls preserve 42. The selected-entry repair is `9a0dc3fbd3`. W100/W105 and member rename have independent public counterexamples in the scrub. | Re-run both O103 rewrite paths including the legitimate reached-entry positive. Check computed-call rename through LSP and server, TclOO saved callbacks, and each diagnostic edit by executing the edited script. |
| Values, VM and tooling | 222 public list/dictionary optimiser executions on `c989476b24` match native output and completion. Six Studio importer modes match option arity and escape controls. The final bytecode candidate matched offsets and mnemonics in 52 release/entry cases. | Check normal VM/runtime and standalone entries for filesystem glob, Thread and structured errors. Verify remaining optimisations, shared-parser consumers and F5 tools. Device-dependent F5 facts belong in the [device ledger](../../bigip-name-checks.md). |
| Missing measurements | The clean `tcl-lsp-db::it_5` build was interrupted during compiler compilation; no DB case had run. The earlier 507 seconds and 2.7 MB per pack-churn round refer to earlier heads. Two standalone WASM crate baselines remain open. | Time DB cases individually, isolating timeouts. Follow with `interned_gc`, the WASM targets, and a fresh #2543 trace that verifies allocator trim occurred. |
| Parser coverage | Three common crates exist: `tcl-numeric`, `tcl-value-syntax` and `tcl-expr`. All 235 direct-call occurrences in the first register were traced. The [7,176-lead review](../../parser-review) is a first pass. The gate on `c989476b24` saw 9,273 Rust structural/host leads, 168 direct/default leads and 6,241 Rust syntax trees. | Re-run the expanded all-Rust scan on the resumed revision. Reconcile changed sites, generated and foreign-language sources, shipping paths named like tests and all feature/host entries. All 22 [grammar families](../../parser-coverage.tsv) still require family-level sign-off. |

The hosted [reference-oracle run on b528174d13](https://github.com/bitwisecook/tcl-lsp/actions/runs/44541025331) completed six syntax targets and passed Jim arithmetic. The C arithmetic test stopped on a pre-existing declined C 8.4 fold for `18446744073709551615 + 0`; the oracle returns −1. Later targets in that workflow did not run. See the tooling handover for the driver.

## Verification and scheduling

Rebuild Jim with `bash scripts/dev/ensure-jim-oracle.sh`, export the reported executable, and require `TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1`. This checkpoint used C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84-15-g2f8e155. Verify the current upstream Jim pin on resumption. Choose the most correct answer the selected dialect and entry can prove. Keep the F5 simulator on C Tcl 9.1; physical device checks stay in the ledger.

Apply only held patches the relevant handover identifies as pending, against the stated base or the current equivalent. Some handovers also supply already-integrated patches as provenance. Preserve experimental collector, guarded eval, solver and live-inlining work separately until its matched A/B passes. Use one heavy Rust compile/link or bulk AST scan at a time on the shared four-core host. For a failing or interrupted job, preserve the last completed assertion and leave an unexecuted test uncounted.

Run workspace formatting, all-target clippy and `make xtask-gates` before every push. The published development checkpoint `b528174d13` passed formatting and 39 of 40 gates. Two GVN default-grammar calls caused the remaining gate failure and strict clippy reported five compiler findings. Their fixes entered the later pause work. See the final pause check below for its own result.

The hourly **Burn-down session check** is paused. Its ID is `Automation_75f6459fdc048191ae7f06e0e86e6b81` and its schedule is hourly at minute 52, Europe/London. Resume it only when the owner resumes development. If a new coordinator cannot inherit it, create one equivalent bound to that coordinator and leave the old one paused. The approved prompt follows.

> Coordinate the bitwisecook/tcl-lsp naming burn-down on work. Read docs/work/name-handling/current-state.md and docs/work/test-scrub/README.md first, then the successor roster and the area handoffs. Check replacement workers' progress, acknowledgements, commits and clean-head baselines with C Tcl 8.4–9.1 and latest Jim required. Follow up on missed messages. Prioritise open wrong answers, then measurement gaps, declared-frame collector verification, and the remaining #2543 memory growth. Keep F5 device questions in docs/work/name-handling/bigip-name-checks.md. Rule on questions, write each ruling and verified scoreboard change to the tracking documents on work, and direct waiting workers to read them. Report material changes and blockers. Coordinate only, leave area code to the area workers, and use work without opening a pull request. Leave the retired and separately paused Claude sessions alone.

## Pause inventory

All six replacement workers acknowledged the pause and stopped their jobs, including the interrupted DB build and the suspended diagnostics driver. The hosted oracle run had already finished. The thirteen area handovers are present. All 29 saved patch files were readable by Git’s patch parser. Some are copies of integrated changes, and each area identifies its pending patches and prerequisites. The latest public diagnostics replay on frozen code passed 29 cases across strict rename, W210, TK1003 and W110. It ran independently of the three-test compiler integration run.

The two distinct APL readers in Zed and the hosted parser disagree even on the bytes of a complete embedded expression. The saved C and Rust probes reproduce 28..36 versus 28..42 for `[list {]} yes]`. The [tooling handover](irules-registry-tooling.md) includes the pinned editor grammar and the remaining malformed and Unicode examples.

## Prompt for the next coordinator

> Resume the naming, shared-parser migration and test scrub in bitwisecook/tcl-lsp on branch `work` from the 19 November 2026 pause. Read `docs/work/name-handling/handover/pause-2026-11-19/coordinator.md` first, then the current scoreboard, scrub README, parser plan, roster and all thirteen linked area handovers. Carry forward the seven owner decisions. Re-enable the hourly Burn-down session check at :52 for the active coordinator. Resume or replace the six area workers with appropriate subagents. Keep the retired and separately paused sessions untouched.
>
> Coordinate, rule, record and verify. Delegate area code. Establish clean baselines, rebuild latest Jim, require every C Tcl 8.4–9.1 and Jim oracle, and apply only the pending patches identified by the handovers. Fix wrong answers first, then measurement holes, the declared-frame collector and #2543. Complete the shared parsing migration across the LSP, compiler, VM, runtime and tools. Require per-consumer, selected-dialect proof before marking a family complete. Keep device-only questions in the F5 ledger. Run formatting, all-target clippy and `make xtask-gates` before pushing. Work only on `work`. Open a PR only if I ask.
