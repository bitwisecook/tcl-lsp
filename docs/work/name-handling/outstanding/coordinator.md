# Outstanding work: coordinator

Area: routing, snapshots, status documents, release gate.

Last updated: 2026-10-22T15:02Z

## In progress

- Routing the external review's findings (N01–N17, section 3.2/3.3 seams) once the review-triage agent reports; transformation-safety findings first (N17, N08, N05, N07, N15, N04, N06).
- 30-minute cycle: WIP snapshot, status documents (this directory, `current-state.md`), progress report, disk/RAM watch.

## Outstanding (in priority order)

1. Route every non-fixed review finding to its owner with the review's repro and fix direction; record dispositions against section B in `current-state.md`.
2. Land the perf agent's batch (lexer/source-image and Analyser memo fixes) after its equality proof; warn agents first (rebuild).
3. Collect hand-backs and commit agents' final notes; keep roles' file ownership unambiguous.
4. Plan the six section D server scenarios and the section E shipping-build latency/memory measurements; assign owners.
5. Fixture reduction phase 2 after implementation agents land. Reconcile review/inventory counts first: keep list has 2,514 paths, while JSON identifies 195 substantive missing expectations and prose says 208. Preserve each real oracle row and empty expected output; no invented count correction.
6. Re-run full CI (`ci.yml`) on a snapshot once the shared tree compiles cleanly, and resolution-oracles; compare with run 5201 and origin/rust.
7. Open the PR to `rust` only when section F and zero pass-to-fail are met, and the user asks.

## Blocked (on whom, and what is needed)

- Validated rows in `current-state.md`: the user's validation agent last committed 2026-10-21 17:51 UTC; fresh validation runs are needed before agent-reported fixes count toward the gate.

## Done since the last update

- 12:50–13:12Z: re-armed the 30-minute cycle. Pushed the full tree (`1c719796fe`). Mirrored agent-referenced scratchpad files to `scratchpad-mirror/` (`6bc8ceaed2`) so agents in the user's separate session can read them. Truncated compiler-names' 2.1 GB runaway log (disk 1.7 → 2.7 GB). Archived the coordinator's three duplicate cloud sessions (no commits). Refreshed the agent table in `current-state.md` from all outstanding files.
- Ruled for compiler-names: `compiled_invocation.rs` import-alias gate and the W210 pkgIndex `dir` entry option are its own; it coordinates the `compilation_unit.rs` construction sites with the compiler agent; the `other.tcl` W210 row narrows to `dir` only (tclsh: the read of `dir` raises before `$missing` is substituted).
- Note: `outstanding/bigip-lsp.md` says "Last updated: 2026-10-22T14:05Z", which is later than the time it was pushed; the separate session should correct it on its next update.

- 11:40–12:10Z: the session usage limit stopped every agent; all eight resumed at 12:12Z from their transcripts. 3.65 GB of superseded libraries and 0.32 GB of idle test executables pruned while no build ran (disk 966 MB → 4.4 GB).
- Full working tree pushed on the user's request (`2fe1d48d97`, includes two temporary probes).

- All eight active agents' outstanding files are filled in and pushed; agent table in `current-state.md` refreshed from them.
- Routed: thread-exit TLS panic (vm-runtime), relative object names (compiler-names), schema withdrawal on undefined reads (compiler), N17 live hook depends on const-fold (compiler-codegen), N05 regression after R1 (compiler-names, lsp), N15 regressions and `compact_names` path (lsp); perf batch hunks for `realm.rs`/`interprocedural.rs` go through the compiler agent.

- Outstanding-work files created for every agent (this directory) and linked from `current-state.md`.
- `current-state.md` and `continue-prompt.md` brought up to date (`2d94063136`).
- User decisions recorded: force-import abstain (`d2f1bd9a22`), review section F as the rust PR gate (`b3ea121954`).

## Resume

- Active development tree: `/workspace/naming-migration-active/tcl-lsp`, shallow `work`, with all fixture families materialised (the 2,514 keep-list paths remain present). Large experiments and unrelated evidence stay omitted. The exact requested full shallow clone at `/workspace/naming-migration/tcl-lsp` is now an object cache; do not edit or build there. Omitted evidence remains in its objects. Required full gates must use their complete inputs.
- Rust 1.99.1, Node 26.12.1, Corepack 0.36.0 and all five exact Tcl releases plus pinned Jim are provisioned. Shared build environment: `/tmp/tcl-lsp-naming-env.sh`; oracle receipt in this cycle's evidence directory. compiler-names reached dependency compilation; no fresh test verdict. Own verified process tree briefly SIGSTOP-paused for safe Git rebase; resume after tree consistency.
- The bounded shallow sync fast-forwarded to d7224c706; later external updates required WIP rebase onto8c902c555. All committed source/evidence survived. Reduced sparse patterns2,991->11 to avoid repeated Git overhead; full fixture materialisation cost about1.5GB. The five source roles received GO at14:20Z; compiler-names claimed the first build window. Keep both checkouts shallow; never unshallow.
- Serialize complete build/run/cleanup commands using `flock /tmp/tcl-lsp-naming-build.lock`; compiler-names g4 gets first lock. Jobs=2, incremental=0, test/dev debug=0, draining logs capped20MB. Stop below2GB. Cleanup must retain executables younger60min and run only with no cargo/rustc process; old runners need adaptation.
- Five implementation roles resumed; external bigip-lsp/perf/review-triage are monitored without duplicates. All external files are fresh at14:26 (bigip14:22,perf14:00,triage14:17:22). ILX source contract routed to compiler; perf realm/interprocedural proof still pending.
- Historical CI5202/run39616304947 has finished:24 failed/4 passed/16 skipped/3 cancelled jobs. Cancelled partitions expose273pass/71fail observations only; compare identities with5201 and run a fresh complete CI snapshot when compilation is ready. Receipts: `../handoff/reviews/merge-inventory-20261019/coordinator-20261022/`. No admission or release-gate checkbox changes.
- The overdue13:55 and14:25 cycles are being published after checkout/sync. WIP commits rebased onto8c902c555 as d1a5bf950/e4bee2529. The first push was normally rejected after external advancement; no force push. Next scheduled cycle15:25:37Z; rearm30min each cycle. Fetchdepth1work before each push and rebase/FF without stashing or resetting agent work.
- Target remains clean merge/release readiness into rust under reviewF and zero pass-to-fail against origin/rust; general runtime parity follows release. Never open a PR unless the user asks.

### Current cycle and routing (14:55Z)

- WIP41e547bcb is pushed. Baseline rust remainsfcb7927f02; recorded merge-base/trial-merge proof for41e547 shows a clean fast-forward geometry, not product acceptance. All sectionF checkboxes open, no admissions.
- First g4 compile: normal library built, then9missinginclude errors in test phase; interrupted before0test execution. Audited2,152tracked literal include paths and restored166absent tracked inputs byte-for-byte (176,787bytes). Receipt `coordinator-20261022/compile-time-inputs.json`; compiler-observed missing paths are distinguished from scan candidates, seven untracked literal examples remain explicit. No assertions changed.
- Source holds: compiler and Core production sources frozen through earlyg4/N17probe; other roles prepare /tmp patches. Names orchestration paused for current Git window; restartmain after rootCLEAR. Build order: namesearlyg4,codegen,compiler,VM,coordinationD3,LSP. Active logs /tmp only.
- New D3 test-only module `tests/e2e/naming_lifecycle.rs`, selector `warm_lifecycle_answers_match_fresh_after_open_and_closed_provider_changes`, formatted/diffchecked but uncompiled/unexecuted. Closed-source edge, stable retirementoffset, firstpostchange diagnostics and provider answers vsfresh; URI update log lacksrevisionfield, documentedcausalityboundary. LSP retains production fixes.
- N13 names owns authentic source-point seed/child entry; LSP owns distinct current/child-interpreter transport and convergence. Never fabricate NativeCompilationEntry. N11 membership filtering must be applied across inventory/identity/index, not only final definition locations.
- External strictClippy789compiler/379VM ataeecf397; inventory split /tmp/naming-clippy-<role>.json. Initialsplitcompiler565,names170,codegen54,VM379; names transfers8body_effects+4C6root tocompiler and2literal_argv toLSP. Confirm sharedhun hands before changes; no newallows. Shadowcopies only duringhold; full gate stillred.
- Externalfilesfresh14:53: bigip14:43,perf14:38,triage14:42:43. Perf equalitystillpending; do notapplyrealm/interprocedural patch. Newunproven analysersegments draft awareness to names, noadmission.
- Review-triage followup afterR1: N01 actual provider/typed allocation behavior, not just all_procs display-mapcount, alongsideN04-N07 six-profile edits. Existing run6 prints definitions; avoid duplicatefullreplay.
- Pinned Wasmtime49.0.3/Binaryen133/WASI34.0 and RustWASMtargets ready; toolavailability receipts only, hostgates unexecuted. Nextcycle15:25:37Z, diskabout14GB.
