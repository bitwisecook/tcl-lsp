# tcl-lsp release plan — remaining work only

No history. Delete items as they complete. Issues carry per-task detail.

## Merge queue

(empty — #1657 is the only open work item; F9 lane idle, held for the
#1662 gate once #1657 closes)

## Pre-release issues

- **#1657** — wedge endgame, ACTIVE (census lane,
  `claude/f6g-1657-lostwake` @ 6452fb127). VERDICT: contention EXCLUDED —
  waiter parked 18s on a FREE map, acquisition counter flat across the
  250ms window. Remaining split: waiter never woken vs task never polled
  again (a parent-loop poll starvation looks identical; mutex defect is
  the extraordinary claim). STRUCTURAL CONFIRMATION: all handler futures
  are branches of ONE `FuturesUnordered` driven by the single
  `Server::serve` task (tower-lsp-server 0.23, unbounded concurrency) —
  parent-poll starvation is the leading mechanism; permit starvation
  excluded (notifications take no permit). PICTURE INVERTED by pollshim
  capture: waiters were victims — the HOLDER froze inside a 2s timeout for
  149.3s while the time driver provably worked. Singular question: timer
  wake never delivered vs fired-but-never-polled. Instrument answering it
  is on `claude/f6h-1657-pollshim` (PR #1679, `Refs #1657` — merge on
  green): hand-rolled timeout with recording wakers, verdict stated in the
  stall line. SUSPECT: tokio 1.53.0 time-driver rework (lockfile adopted
  1.53.1 Aug 15; no sighting predates it); 1.52.0 downgrade experiment
  pre-staged with criteria agreed (30 clean ≈ corroboration at ~1-in-8;
  any capture on 1.52 kills it) — runs after current loop. 70s pre-send
  hold split off as #1678 (post-release). Checkpoints through 22.
- **#1662** — DECIDED (user): per-PR path-filtered `make lsp-server-wasm-test`
  job. Do LAST — when the rest of this pool is empty, immediately before the
  release handoff. No time on it before then; Pages deploy stays the gate.
- **E1 (low priority)** — `claude/e1-expr-numbers` @ a8058849d. Remaining:
  fmt, 8.4/8.5/9.1 matrix + guard flips, gates, adversary, PR
  (`Fixes #1382 #1425 #1428 #1432`).

## Release

Order: empty the pool above → land #1662's CI gate (last) → `rust` green
end-to-end (CI + pages deploy) → hand to user. 2.x cut from `rust` via
`scripts/release/rust_release.sh` (`release` skill); marketplace approvals
from release laptop. Never run unilaterally.

## Standing rules

- Merge when CI green + feedback resolved + no conflicts. Closes-keywords
  don't fire on `rust` — close issues manually with merge SHA.
- Wedge (~1-in-5 `test-ext`): read failure block first, post census evidence
  to #1657, then one re-run. Wedges twice → hold PR.
- wasm: `crate::rt::Instant` only on wasm-reachable server paths;
  `make lsp-server-wasm-test` before push. Pages deploy is the only
  post-merge gate until #1662.
- Low disk: commit+push all worktrees first (targeted adds; never
  `git add -A`; `tmp` symlink stays untracked), persist context to issues,
  then clean.
- Every lane: push after each green unit; laconic issue comment with branch
  name + discoveries at every checkpoint. GitHub is the only durable store.
- Keep this file current at every transition; no history, laconic; every
  update is committed AND pushed immediately — an unpushed plan is no plan.
- Labels: vm/codegen/runtime issues → `post-release` unless they impact
  `.tclspec` load/exec; `pre-release`/`post-release` mutually exclusive.
- `stable` toolchain floats: new Rust can break `pr-gate` overnight → hotfix
  lane with new toolchain via rustup.

## Parked (post-release — do not start)

`post-release` pool (~45), Track B/SslicTcl/BIG-IP, E5, #1631, #1633,
#1643, #1646, #1648, #1655, E3 PR-2 (#1569 #1574 #1575, corpus on
`claude/e3-oracle-artifacts`), `rust/`→`crates/` layout PR last.
