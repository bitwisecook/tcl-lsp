# Outstanding work: coordinator

Area: routing, snapshots, status documents, release gate.

Last updated: 2026-10-22T17:25:44.599561+00:00

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
- The old BIG-IP handoff timestamp discrepancy is resolved: the external file now reports an actual 14:58Z update. Future freshness checks use current reported times.

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

### Current cycle and routing (14:55 UTC)

- Published snapshot: `41e547bcb`. The recorded trial merge for that snapshot has merge base `fcb7927f02` and produces its exact tree without conflicts. This proves Git integration only. All section F boxes remain open; no audit admissions.
- The first g4 build completed the normal library, then reported nine missing include errors in the test build. It was interrupted before any tests ran. The root audited 2,152 literal tracked include paths and restored 166 absent inputs byte-for-byte (176,787 bytes). `coordinator-20261022/compile-time-inputs.json` distinguishes compiler-observed omissions from scan candidates. Seven unresolved literal examples remain explicit; generated and nonliteral includes are outside this scan. No assertions changed.
- Compiler and Core production source changes are held through the early g4 and initial N17 probes. Other roles prepare patches under `/tmp`. Restart g4 main after the root announces Git consistency. Build order: compiler-names, compiler-codegen, compiler, vm-runtime, coordination D3, lsp. Active logs stay under `/tmp`; copy completed receipts only.
- New D3 module: `tests/e2e/naming_lifecycle.rs`, selector `warm_lifecycle_answers_match_fresh_after_open_and_closed_provider_changes`. Formatted and checked for whitespace errors; uncompiled and unexecuted. It compares open and closed source lifecycle changes with a fresh server, retaining the retirement offset and asserting the first answers. The URI update log has no revision field; the causal boundary is documented. LSP retains production fixes.
- N13: compiler-names owns the authentic source-point issuer and child entry; LSP owns distinct current/child-interpreter transport and convergence. Never fabricate a `NativeCompilationEntry`. N11 membership must constrain identity, inventory and index lookup rather than only final definition locations.
- External strict Clippy at `aeecf397` reports 789 compiler and 379 VM errors. Inventories: `/tmp/naming-clippy-<role>.json`. Initial split: compiler 565, compiler-names 170, compiler-codegen 54, vm-runtime 379. Compiler-names transfers eight body-effects and four C6 root diagnostics to compiler, and two literal-argv diagnostics to LSP. Confirm shared hunks before edits; no new lint allowances. Current plans use shadow copies; the full gate still fails.
- External files checked at 14:53: bigip-lsp 14:43, perf 14:38, review-triage 14:42:43. Perf equality is pending; do not apply its realm/interprocedural patch. Its analyser image and segment drafts remain unproven.
- Review-triage follow-up after R1: inspect N01 provider behavior and typed allocations, alongside N04-N07 edits across six profiles. A display-map count alone cannot establish a slot collision. Run6 already prints definitions; avoid a duplicate full replay.
- Wasmtime 49.0.3, Binaryen 133, WASI SDK 34.0 and the Rust WASM targets are ready. Tool availability receipts do not establish host acceptance. Next cycle: 15:25:37 UTC. About 14 GB is free.

### Current cycle and routing (15:25 UTC)

- Active build files remain untouched. The separate publication checkout at `/workspace/naming-migration-publish/tcl-lsp` borrows active Git objects, fetches only `work` at depth one, and rebases/pushes snapshots there. Active WIP commits and published commits are mapped in `coordinator-20261022/publisher-workflow.json`; product tree differences must be recorded before integrating updates between build windows. No active rebase during compilation.
- The g4 retry reached compiler test-binary compilation. No fresh product test result exists. Keep the early g4/N17 source hold; no target cleanup during builds.
- Section D routing: coordination checks owns new native D1 and D3 test modules; LSP owns D2 inherited/deep state and D5 protocol edits; VM owns D4 unchanged environment transitions; root coordinates D6 browser/WASI and editor gates. Prepared tests and manual oracle controls do not close these gates. D3 readiness was corrected to consume the completed exact-URI update rather than provisional diagnostics.
- External files checked this cycle: BIG-IP 14:58, perf 15:16, review-triage 14:42:43. All are under 60 minutes old. Perf equality is still pending and its realm/interprocedural patch remains withheld. Strict lint blockers and ILX are already routed to compiler/VM owners; R1 remains with compiler-names.
- Shadow lint patches are prepared by compiler, compiler-names, compiler-codegen and VM; canonical production sources remain held. After the N17 functional chain, coordinate their shared API batch before broad compiler acceptance to avoid repeated expensive rebuilds. No new lint allowances.
- No validated product row, audit admission or release checkbox changed. Next cycle is due at 15:55:37 UTC.

### Current cycle and routing (15:55 UTC)

- Previous publication: `9b3f4f273`; active source snapshot: `db2623ed2`. Product trees were identical at those snapshots. The isolated publisher handles remote fetch/rebase/push; active build source remains in place. The next full WIP includes prepared native D1–D5 scenarios and the shared browser/WASI rename application proof, all unexecuted.
- Early g4 main: 0 passed / 1 failed, 168.78s, ten positive-world mismatches across C8.6/C9.0. Negative/body checks reported no mismatches, not separate passed tests. The body guard uses `.all()` and its vacuity limit is explicit. R1 attribution points to confusing callback capability (UNKNOWN/TRACE) with installed hooks; compiler-names prepares an authenticated registration correction, retaining generic unknown-call refusal.
- Coordination support temporarily changed and restored canonical parse_cut.rs around 15:34, violating the source hold. Current bytes match HEAD. The completed main used the earlier cached lexer and has no direct compiler include of this file; retain it only as agent-reported evidence. The following ZZ run legitimately rebuilt dependencies, then was interrupted before tests. No timestamp/fingerprint override or cleanup. Exact command history was unavailable and remains explicit in the incident receipt. Support now does read-only review.
- compiler-codegen received the slot at 15:51 for the unchanged N17 probe, then the narrowly authorised barrier fix and functional controls. Remaining g4/ZZ/eight-case attribution is deferred to the coordinated compiler batch; no extra compiler-test rebuild solely for the interrupted probe. Compiler C6, VM acceptance, native protocol checks and LSP minify follow.
- Root owns three lexer blocker candidates after the source hold: reject empty token geometry without weakening the existing assertion; correct Jim BOM and original quoted-source expectations narrowly using pinned Jim and C Tcl evidence. Candidates are off-tree and have not run. Shared browser/WASI proof applies every returned UTF-16 edit, validates captured URI/version/ranges and checks first new-revision targets. Its twelve manual original/control executions across six engines are fixture controls only.
- Browser dependencies, Chromium, exact wasm-bindgen and fixed Tcllib are provisioned. Their receipts state setup warnings and initial failed attempts; no transport acceptance is implied. A source-hold operational inventory covers 4,539 materialised Rust src/SpecTcl files from 15:52 onward; it is not a whole-source validation guard.
- External status: BIG-IP 15:46, perf 15:45, review-triage 15:46:08; all fresh. Perf equality remains pending. Its realm/interprocedural patch is still withheld. Core60 Clippy primary inventory is requested in the routing record but not yet published; generic Core belongs to LSP. ILX source relation stays with compiler; its server latency remains with compiler/LSP and measured perf work.
- Next cycle: 16:25:37 UTC. Re-arm the one-shot local reminder each cycle; hosted automations do not support this cadence. No release gate or audit admission changed.

### Current cycle and routing (16:25 UTC)

- Frozen N17 baseline completed with identical whole-source hashes; its observational pass reproduces missing folding and is not functional acceptance. The narrow declared-footprint barrier correction is the only production source change in the functional chain. Full variable-reference, pack-source and hook-fold targets are next/current. All other Rust and test source remains frozen.
- Original g4 still fails ten positive subcases. R1 registry/consumer candidates and a public-API diagnosis retain old expectations and add retained-script negatives; original expensive library filters remain mandatory after stabilization. C6 assertions remain unchanged.
- External files are fresh: BIG-IP16:06, perf16:18, triage16:12:43. Old-base complete PackSet equality does not establish latest/full corpus equality; compiler patch remains withheld. BIG-IP loaned current_diagnostics.rs is handed back for production while its isolated comparison restores exact bytes.
- Publication remains isolated from active compilation. Twenty-five external BIG-IP/Core lint source/test files are retained in published work but not yet integrated into active builds; combined-tree acceptance is outstanding. No active checkout/rebase, target cleanup, audit admissions or validated-row changes.
- Native D1-D5 and browser/WASI D6 fixtures remain unexecuted. Read-only support maps exact release gate commands/prerequisites. Next scheduled cycle16:55:37UTC.

### Baseline identity correction and new run (16:44 UTC)

- Direct API metadata proves CI5201 is manual `work` at `efc58507c1`, not origin/rust. Green origin/rust5199 at `fcb7927f02` skipped native LSP/e2e and four Rust shards and has no retained inventories. Neither supports zero pass-to-fail. Existing historical work-to-work comparisons remain attributed as such.
- Started exact-current-rust full baseline CI run39816429638 at16:44:07Z, hosted, native_release_build_proof=false. Actual head verifiedfcb; workflow dispatch selects full changed closures and branch release/publish jobs are disabled by tag conditions. No source push/tag/PR. Monitor at16:55; capture complete package/binary/selector outcomes and actual toolchain/features.
- Active prefetch completed at16:42 without changing HEAD/index/source. All20 old shallow boundaries retained; exact missing parents fetched depth1 only. Batched cached-header guard prepared to avoid repeated per-commit process spawning; real296-commit walk and missing-tip fail-closed control preserve shallow bytes.

### Current cycle and routing (16:55 UTC)

- N17 suites have matching35,072-file source guards: boundary4/4, pack-source3/6, hook-fold3/6. Combined explicit diagnostic identifies native compiler preflight provider required; caller formals/declaredness are not that failure. Derived rows and exact cleanup follow.
- Exact temporary-toggle cleanup passes dry-copy assertions and seven syntax parses after one exact comment-pattern maintenance fix; canonical cleanup is pending final measurements.
- Generic Core54 lint inventory is published and routed; all roles have off-tree lint/API candidates, none is a current zero-warning verdict.
- CI5201 is historical work at efc58507, not origin/rust. Green origin/rust5199 skipped native LSP and four shards and has no retained selector inventory.
- Started full nonpublishing baseline CI39816429638 on exact origin/rustfcb at16:44:07Z; complete test outcomes are pending, cargo-deny already failed separately. No source push/tag/PR.
- Bounded active work prefetch completed without changing HEAD/index/source; old20 shallow boundaries retained. Combined-tree source integration follows CCG stable handback.
- External role files remain under60minutes. Perf equality is incomplete and routed compiler patch remains withheld. No audit admissions, validated product row or sectionF checkbox changed.

### Current cycle and routing (17:25 UTC)

- Mapped source integration atde445 includes25external product/testpaths and exact CCG toggle/probe cleanup; publisher normal-pushed41e25f0047 after depth1 fetch/rebase. Committed product trees match; no active-source rebase during R1.
- R1 fact/consumer build completed without warnings; public54-subcase diagnosis is running with original expectations and stronger nonempty body checks. Early8.6positive rows match; no final verdict yet. C6 original3-case public diagnosis owns next slot.
- N17 missing StringIs compiler descriptor isolated without bypassing require_provider;11file typedASCII draft stays off-tree. Modern3C oracles each20semantics pass;8.4 extraNUL failure remains recorded outside admitted8.6+ model.
- Full exactorigin/rust baselineCI5203 completed47jobs:30success,16condition-skipped,1cargo-denyfailure. Five Rust shards22,017passes and3nativeLSPpartitions2,295passes reconciled by named reporter identities; unitbinaryIDs and unnamedSKIPs limits explicit. No currentwork zero-P2F claim.
- All4baseline advisories are already remedied by current lock versions (static inspection only); fresh shipping cargo-deny required. Historical work5202 failures yield49unique rendered package/selector baselinePASS matches and22unknowns; roster is retest prioritisation, not latestwork verdict.
- New53xtask and40registry lint findings routed; xtask exactassertion-spelling draft prepared on copies and registrydraft underway. All prior shared compiler/Core/VM lint/API candidates remain off-tree and uncompiled.
- Externalfiles are fresh:BIGIP17:14,perf16:52,triage17:15. Optional exhaustive89-field perf diagnostic is preserved, not applied; full equality and routedcompiler patch remain unproven. No audit admissions, validatedproduct-row changes or sectionF closures.
