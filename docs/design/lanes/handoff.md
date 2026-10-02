# Hand-off: resuming the value-transfer build

This page is the orchestrator's resume point for the work on branch
`claude/spectcl-optimization-discussion-5qhf42` (issue #1943: value
transfers, the diagnostic policy, the registry consumer contracts). It is
rewritten at every push. The two lane documents carry the detail; this page
says where each lane stands, what is queued, and how the work is run.

## Where the lanes stand

| Lane | Tracking document | Landed | In flight | Remaining |
|---|---|---|---|---|
| Value transfers | [value-transfers.md](value-transfers.md) | slices 1, 2, 3, 4, 5, 8, 6 (reviewed, reworked in three rounds, re-checked "land as is") and 9 (`f06639de`); slice 10 items 1, 2, 3a, 3b, 3c and 3d (`ddb53a10`, `b9ab4993`, `7ee77e66`, `6f86956d`, `07633485`, `e0675133`) with the SpecTcl catalogue fix `1d3a937f` | slice 10 item 3d's remainder (a quoted `catch "…"` script descended, a computed `catch $s` raising the barrier as the statement form does) on the running implementer, then 3e (the flattened `catch` end-block marker) and items 4 onward | the rest of slice 10, then the fable review of slices 9 and 10 together with its fixes; then slices 11, 12, 7a, 13, 7, in that order |
| Consumer contracts | [consumer-contracts.md](consumer-contracts.md) | steps 1 to 9, each reviewed with fixes landed (the step 8 rework and step 9 review's fixes are `16e3a5a0`, `1b870f36`, `cdc7307a`); step 10 items 1 to 3 and item 4's first part (`69f5211b` the extension default, `ca5cc729` an extension described from three sources, `75b750df` the host load bridge, `141d30f0` the WASM runtime's registration seam; `69ed4b81` puts the package-provide requirement into item 5's plan) | step 10 item 4's parts 2 and 3 (the authored `tcl.h` for both hosts, the C-extension WASM gate) on the running implementer, then items 5 and 6 | the step 10 review with its fixes |
| Diagnostic policy | (landed; see the design page `docs/design/compiler/diagnostic-policy.md`) | complete | | |

Branch heads at the last rewrite of this page: the main branch at the
commit that rewrites this page, above `1802ac33` (the merge of `cc-step5`
at `141d30f0`) and `e0675133` (slice 10 item 3d); `cc-step5` at
`141d30f0`, containing the main branch up to `6f86956d`. `cc-step5` is
merged into the main branch, gated and pushed after every one of its
commits, so nothing of either lane stays off GitHub for longer than one
gate cycle.

## Queued on the running implementers

- Value transfers: slice 10 item 3d's remainder and 3e, each its own
  green commit; then item 4 (`try`), which is shaped to reconcile with `rust`'s PR #2230
  (the try/finally reachability fix for #2142, absent from this branch's
  base) at the upstream merge: one implementation per fact, with a
  "reconciling with #2230" paragraph in the lane doc naming the files and
  facts the merge has to settle. The slice 9 review runs together with
  slice 10's once slice 10 lands.
- Consumer contracts: step 10 item 4's parts 2 and 3, then items 5 and 6,
  each its own green commit, then the fable review of step 10 and its
  fixes.

## After the lanes

1. Merge `origin/rust` (185 commits ahead: the dialects/tclspec
   editor-config generation, the jimtcl updates, the Rust 1.99.0 baseline
   this branch already carries, PR #2230) into the branch at the next point
   where both lanes are at a clean checkpoint; reconcile through the
   registry interface, regenerate every generated artefact with the
   repository's tooling, rerun the suites and the gate, push. Repeat before
   the end if `rust` moves.
2. The final scrub before the pull request to `rust`: delete this
   directory and every link to it, remove slice, step, decision and lane
   references from every page and comment outside it, retire the
   `registry-axis-ok … until` clause and the `LANDED` list in
   `rust/xtask/src/registry_axes.rs`, keep the `value-transfers` ledger
   only in the form its gate needs, and review the result before pushing.
3. A last fable review of the whole branch, and the artifact mirror
   (`claude.ai/code/artifact/f1aa5423-0def-49ac-bdb9-30dc19f65830`, at
   Version 8 as of 1a67ca4f) brought up to date.

## How the work is run

- Four worktrees: the main worktree on the branch (value transfers), a
  second worktree on `cc-step5` (consumer contracts), and two detached
  worktrees with their own `target/` for gates and reviews (one can hold a
  running review while the other gates a push).
- Every commit on the main branch is gated with `make rust-check` plus
  `cargo xtask dialect-drift` (eight pre-existing sites; a lane adds none)
  in a gate worktree, then pushed with
  `git push -u origin <hash>:refs/heads/claude/spectcl-optimization-discussion-5qhf42`.
  Nothing is pushed to any other branch. A docs-only commit above a gated
  head gets `make xtask-check` before its push.
- Implementers commit every green sub-item at once, and a sub-item that
  runs past about two hours commits a green intermediate state rather than
  keeping a long-lived draft. `make rust-check` compiles but does not run
  tests, so each lane runs its crates' suites before every commit; the
  value-transfer suites include `tcl-spectcl`, whose catalogue tables
  mirror the registry's id tables.
- Each landed slice or step gets a fable review in a gate worktree; its
  findings land as "review fixes" commits (blocking ones before the next
  item), and pre-existing defects found in passing become GitHub issues
  (#2253 to #2272, #2291 to #2297, #2299 to #2303, #2305 to #2316, #2323
  to #2328, #2330 to #2335, #2337, #2338 so far), never fixes on this
  branch.
- Implementers are sonnet, reviewers fable. Commit messages are
  `wip(<lane>): <slice or step> — <phrase>` with the session's two trailer
  lines; files outside this directory describe current state only.
- The branch builds under Rust 1.99.0, the same way `rust` does: `rust`'s
  baseline commit (`ee4aeb7a` there) is cherry-picked as `81ffe5f3`, and
  `e96cb2b5` clears the one branch-local site it does not cover (the
  renamed `AtomicU64::try_update`). The Makefile's `CLIPPY_LINT_FLAGS`
  carries `-A clippy::assert_is_empty`, so a direct clippy run outside
  `make` needs the same flag:
  `cargo clippy --workspace --all-targets -- -D warnings -A clippy::assert_is_empty`.
  The repository pins `stable`; every worktree carries a rustup directory
  override to `1.99.0`.
- The machine has four CPUs and fifteen gigabytes: `~/.cargo/config.toml`
  pins `jobs = 2`, gates run with `CARGO_BUILD_JOBS=1`. Disk is tight: test
  executables over 20 to 30 MB and duplicate rlibs under
  `target/debug/deps` are deleted after each run, never while that tree's
  implementer is running; a gate worktree's `target/` can be removed
  whenever it is idle.
- Each agent keeps its scripts in a private directory under the scratchpad
  (two agents once collided on a shared `gates.sh`), and nothing is run with
  a gate worktree as its working directory except the gate itself.
- The session has restarted several times; drafts survive on disk, so a
  restart is recovered by reading `git status` and `git log` in each
  worktree and relaunching the implementer on the draft.
