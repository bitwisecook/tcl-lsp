# tcl-lsp release plan — remaining work only

No history. Delete items as they complete. Issues carry per-task detail.

## Merge queue

1. **PR #1667** — wedge census (`claude/f6c-1657-census`, `Refs #1657`).
   Branch updated to 1.98-fixed base; CI re-running. One `test-ext` re-run
   authorised. Merge on green; do **not** close #1657.
2. **PR #1668** — `$=` decoder removal + dict canonicalisation owner
   (`claude/f9-fixes-1608-1617`). Waiting on lane fix: literal-`$` must join
   the literal run (Tcl_ParseVarName), mixed-shape oracle vectors. Then green
   CI, merge, manually close #1608 #1617.

## Pre-release issues (assign as lanes free)

- **#1657** — wedge endgame. Suspect: `deliver_if_current` holds `documents`
  lock across unbounded client send; sibling `deliver_fast_tier_if_current`
  uses a timeout. Lock-across-send is documented load-bearing for a
  `did_close` race in `main.rs` → needs design, not a patch. First step: a
  holder tag on the `documents` lock to convict, not just localise. Spawn after
  #1667 merges (census lane is natural owner). Close only when fix survives
  loaded-loop repro.
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
