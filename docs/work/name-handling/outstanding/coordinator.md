# Outstanding work: coordinator

Area: routing, snapshots, status documents, release gate.

Last updated: 2026-10-22T13:12Z

## In progress

- Routing the external review's findings (N01–N17, section 3.2/3.3 seams) once the review-triage agent reports; transformation-safety findings first (N17, N08, N05, N07, N15, N04, N06).
- 30-minute cycle: WIP snapshot, status documents (this directory, `current-state.md`), progress report, disk/RAM watch.

## Outstanding (in priority order)

1. Route every non-fixed review finding to its owner with the review's repro and fix direction; record dispositions against section B in `current-state.md`.
2. Land the perf agent's batch (lexer/source-image and Analyser memo fixes) after its equality proof; warn agents first (rebuild).
3. Collect hand-backs and commit agents' final notes; keep roles' file ownership unambiguous.
4. Plan the six section D server scenarios and the section E shipping-build latency/memory measurements; assign owners.
5. Fixture reduction phase 2 (dedupe/merge, table conversion, the 208 unasserted expectations) after implementation agents land.
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
