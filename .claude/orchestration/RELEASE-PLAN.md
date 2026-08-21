# tcl-lsp release plan — remaining work only

No history. Delete items as they complete. Issues carry per-task detail.

## Merge queue

(empty — merge PRs as lanes open them, on green + resolved feedback)

## Pre-release issues

- **#1657** — wedge endgame, ACTIVE (census lane,
  `claude/f6e-1657-endgame`). #1670 merged (8902ca746): send caps,
  closed-file retries, holder tag + retags — hardening, not the cure.
  Hold narrowed to `cache_and_deliver`'s two uncapped awaits
  (`pull_diag_cache.lock` / `diag_slots.lock` via `redeliver_later`;
  the latter is new in #1670 and NOT excluded as cause). Lane looping for
  the naming capture (~2-in-20 loaded runs reproduce). A green loop alone
  does not close this — repro rate too low.
- **#1614** — ACTIVE (F11 lane, `claude/f11-enums-1614`). Closed string
  vocabularies → enums; 7 independent sites inventoried in issue (taint
  basis highest value); wire formats must not change.
- **#1672** — ACTIVE (F9 lane, `claude/f9-fix-1672`). Untyped material arm:
  prove fall-through or abstain unless DEFERS_BODY. Deliberate tcllib
  clay-corpus run required; corpus shifts are findings to report.
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
