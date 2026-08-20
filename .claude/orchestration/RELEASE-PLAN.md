# tcl-lsp release plan — remaining work only

No history. Delete items as they complete. Issues carry per-task detail.

## Merge queue

1. **PR #1668** — `$=` decoder removal + dict canonicalisation owner
   (`claude/f9-fixes-1608-1617`). Conflict with post-#1669 base resolved,
   gates green under 1.98, Codex thread resolved; CI running on cbc127e8f.
   Merge on green, manually close #1608 #1617.

## Pre-release issues

- **#1657** — wedge endgame, ACTIVE (census lane, branch
  `claude/f6e-1657-endgame`). Step 1: holder tag on `documents` lock.
  Step 2: design around `deliver_if_current`'s unbounded send under the
  lock — the hold is documented load-bearing for a `did_close` race in
  `main.rs`; design goes on the issue before code. Close only when fix
  survives loaded-loop repro. Census is on `rust` (bad41e1df).
- **#1644** — per-arg lifecycle wiring. Layering decision + acceptance
  criteria in issue.
- **#1656** — `apply $lambda` invisible to fall-through walk. Use
  DEFERS_BODY vocabulary; repro in issue.
- **#1660** — metaclass can't classify same-file creation call.
  Deferred-verdict shape in issue.
- **#1662** — wasm LSP server CI gate. USER decision on CI budget; options
  in issue.
- **E1 (low priority)** — `claude/e1-expr-numbers` @ a8058849d. Remaining:
  fmt, 8.4/8.5/9.1 matrix + guard flips, gates, adversary, PR
  (`Fixes #1382 #1425 #1428 #1432`).

## Release

When the pool above is empty and `rust` green end-to-end (CI + pages
deploy): hand to user. 2.x cut from `rust` via
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
- Keep this file current at every transition; no history, laconic.
- Labels: vm/codegen/runtime issues → `post-release` unless they impact
  `.tclspec` load/exec; `pre-release`/`post-release` mutually exclusive.
- `stable` toolchain floats: new Rust can break `pr-gate` overnight → hotfix
  lane with new toolchain via rustup.

## Parked (post-release — do not start)

`post-release` pool (~45), Track B/SslicTcl/BIG-IP, E5, #1631, #1633,
#1643, #1646, #1648, #1655, E3 PR-2 (#1569 #1574 #1575, corpus on
`claude/e3-oracle-artifacts`), `rust/`→`crates/` layout PR last.
