# tcl-lsp release plan — remaining work only

Forward-looking plan for finishing the 2.x pre-release. No history; every
completed item is deleted from this file. Kept current on
`claude/tcl-lsp-full-remainder-i1o1bx` so any session can pick it up cold.
Issues carry the per-task detail: every lane posts its branch name and
discoveries as comments on the issues it works.

## Merge queue (strict order)

1. **PR #1667** — wedge census instrumentation (`claude/f6c-1657-census`,
   `Refs #1657`). Branch updated to the 1.98-fixed base; CI re-running
   (`test-ext` has one authorised re-run; its wedge evidence is already
   banked on #1657). Merge on green. Do **not** close #1657.
2. **PR #1668** — `$=` decoder removal + dict canonicalisation owner
   (`claude/f9-fixes-1608-1617`, `Fixes #1608 #1617`). Codex round in
   progress (literal-`$` must join the literal run instead of aborting the
   template parse — mixed-shape oracle vectors required). Then branch update,
   green CI, merge, manually close #1608 #1617 with evidence.

## Remaining pre-release issues (assign as lanes free)

- **#1657 — the server wedge endgame.** Census landed in #1667 and named the
  suspect: `deliver_if_current` holds the `documents` lock across an
  unbounded client send (its sibling `deliver_fast_tier_if_current` wraps the
  identical hold in a timeout). The lock-across-send is documented as
  load-bearing for a `did_close` race in `main.rs`, so the fix needs a small
  design, not a patch. Spawn a design+fix lane after #1667 merges; the lane
  that built the census (its findings are all on #1657) is the natural owner.
  Close #1657 only when a fix survives the loaded-loop repro.
- **#1644 — per-argument lifecycle wiring** (authorable but unconsumed).
  A layering decision recorded in the issue: floor-parameterised registry
  accessors vs a per-document projected-spec cache, with a cheap fast-reject
  (`spec.arg_rows.is_empty()`) worth trying first. Needs a design-capable
  lane; acceptance criteria are in the issue.
- **#1656 — `apply $lambda` invisible to the fall-through walk.** The
  LambdaLiteral arg role never reaches the materiality check; the
  DEFERS_BODY vocabulary from #1652 is the fix's language. Repro shape in
  the issue.
- **#1660 — post-pass-proved metaclass cannot classify a same-file creation
  call.** Deferred-verdict shape suggested in the issue (the #1642 floor
  pattern).
- **#1662 — CI gate for the wasm LSP server.** USER DECISION on CI budget:
  options ranked in the issue (per-PR `make lsp-server-wasm-test` ~7 min /
  merge_group-only / cargo-check-only / nightly). The gap let two deploy
  breaks reach `rust` in 24h; the pages deploy is currently green.
- **E1 finish (low priority, cleared to land):** remote branch
  `claude/e1-expr-numbers` @ a8058849d. Remaining: fmt, the 8.4/8.5/9.1
  matrix + guard flips, full gates, adversary pass, PR
  (`Fixes #1382 #1425 #1428 #1432` — all post-release-labelled but cleared
  to land if convenient).

## Then: release

When the pre-release pool above is empty and `rust` is green end-to-end
(CI + pages deploy), hand to the user for the release workflow — 2.x
pre-releases are cut from `rust` via `scripts/release/rust_release.sh`
(see the `release` skill); marketplace Environment approvals happen from
the release laptop. Not something a remote session runs unilaterally.

## Standing rules for whoever orchestrates

- Merge authority: merge when CI is green, review feedback dealt with, and
  conflicts resolved. Closes-keywords do NOT fire on the non-default `rust`
  branch — close issues manually with an evidence comment naming the merge
  SHA.
- Wedge policy (#1657, ~1-in-5 `test-ext` runs): read the failure block
  FIRST (it self-diagnoses; it is census evidence — post it to #1657), then
  one re-run. If a PR wedges twice, hold it and weigh the wedge fix instead.
- wasm safety: any change the wasm LSP server ships must use
  `crate::rt::Instant` (never `std::time` on wasm-reachable paths) and run
  `make lsp-server-wasm-test` before push. The pages deploy is the only
  post-merge gate until #1662 is decided.
- Low-disk directive (user): on ANY low-disk signal, FIRST commit+push all
  worktree work (targeted adds — never `git add -A`; `tmp` symlink stays
  untracked) and persist continuation context to the issue, THEN clean.
- Every lane: push after each locally-green unit; post branch name and
  discoveries as comments on its issues at every checkpoint. Scratchpads and
  transcripts do not survive container loss; GitHub is the only durable
  store.
- Labelling: new vm/codegen/runtime issues → `post-release` unless they
  impact `.tclspec` loading/execution; `pre-release`/`post-release` are
  mutually exclusive.
- Toolchain floats (`stable` pin): a new Rust release can break `pr-gate`
  repo-wide overnight; fix mechanically on a hotfix lane with the new
  toolchain installed via rustup (the #1669 pattern).

## Parked for post-release (do not start)

The `post-release`-labelled pool (~45 issues), Track B / SslicTcl / BIG-IP
report work, lane E5, #1631 (dialect-vs-package architecture), #1633,
#1643, #1646, #1648, #1655, E3's PR-2 (#1569 #1574 #1575 — oracle corpus
preserved on `claude/e3-oracle-artifacts`), and the `rust/`→`crates/`
layout PR (absolutely last).
