# Outstanding work: coordinator

Area: routing, snapshots, status documents, release gate.

Last updated: 2026-10-22 11:10 UTC

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

- Outstanding-work files created for every agent (this directory) and linked from `current-state.md`.
- `current-state.md` and `continue-prompt.md` brought up to date (`2d94063136`).
- User decisions recorded: force-import abstain (`d2f1bd9a22`), review section F as the rust PR gate (`b3ea121954`).
