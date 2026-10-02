# Hand-off: resuming the value-transfer build

This page is the orchestrator's resume point for the work on branch
`claude/spectcl-optimization-discussion-5qhf42` (issue #1943: value
transfers, the diagnostic policy, the registry consumer contracts). It is
rewritten at every push. The two lane documents carry the detail; this page
says where each lane stands, what is queued, and how the work is run.

## Where the lanes stand

| Lane | Tracking document | Landed | In flight | Remaining |
|---|---|---|---|---|
| Value transfers | [value-transfers.md](value-transfers.md) | slices 1, 2, 3, 4, 5, 8, 6 (reviewed, reworked in three rounds, re-checked "land as is") and 9 (landed at `f06639de`) | slice 10 (completion paths): its first item is committed at `ddb53a10`, its second (per-path publication) is in draft on the running implementer | the rest of slice 10, then the fable review of slices 9 and 10 together with its fixes; then slices 11, 12, 7a, 13, 7, in that order |
| Consumer contracts | [consumer-contracts.md](consumer-contracts.md) | steps 1 to 7, each reviewed with fixes landed; the step 8 rework (three groups) and step 9 (landed at `ef740eb8` on `cc-step5`) | step 10 (the extension legs) on the running implementer, and the fable review of the step 8 rework and step 9 in the gate worktree | the step 10 review with its fixes, then the merge of `cc-step5` into the main branch at a value-transfer checkpoint |
| Diagnostic policy | (landed; see the design page `docs/design/compiler/diagnostic-policy.md`) | complete | | |

Branch heads at the last rewrite of this page: the main branch at the
commit that rewrites this page, above `e96cb2b5` (the move to Rust 1.99.0)
and `ddb53a10` (slice 10's first item); `cc-step5` at `ef740eb8` (step 9
landed) with step 10 in draft, containing the main branch up to `203fb917`.

## Queued on the running implementers

- Value transfers: slice 10's second item is in draft; the implementer
  commits it on top of the 1.99.0 move and stops for the gate. The slice 9
  review runs together with slice 10's once slice 10 lands.
- Consumer contracts: step 10 runs under Rust 1.98.1 until the main branch
  is next merged into `cc-step5`, which brings the 1.99.0 move; the
  worktree's toolchain override moves to 1.99.0 at that merge. The review
  of the step 8 rework and step 9 reports its findings as fixes to land on
  that implementer before step 10's own review.

## After the lanes

1. Merge `origin/rust` (the dialects/tclspec editor-config generation and
   the jimtcl updates) into the branch at the next point where both lanes
   are at a clean checkpoint; reconcile through the registry interface,
   regenerate every generated artefact with the repository's tooling, rerun
   the suites and the gate, push. Repeat before the end if `rust` moves.
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
  second worktree on `cc-step5` (consumer contracts; merged into the main
  branch at a value-transfer checkpoint after each step's review), and two
  detached worktrees with their own `target/` for gates and reviews (one
  can hold a running review while the other gates a push).
- Every committed checkpoint is gated with `make rust-check` plus
  `cargo xtask dialect-drift` (eight pre-existing sites; a lane adds none)
  in a gate worktree, then pushed with
  `git push -u origin <hash>:refs/heads/claude/spectcl-optimization-discussion-5qhf42`.
  Nothing is pushed to any other branch.
- Each landed slice or step gets a fable review in a gate worktree; its
  findings land as "review fixes" commits (blocking ones before the next
  item), and pre-existing defects found in passing become GitHub issues
  (#2253 to #2272, #2291 to #2297, #2299 to #2303, #2305 to #2316, #2323
  to #2328, #2330 to #2335 so far), never fixes on this branch.
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
  The repository pins `stable`; the worktrees carry a rustup directory
  override to `1.99.0` (`cc-step5`'s moves at its next merge from the main
  branch), and the `1.98.1` toolchain goes once nothing builds on it.
- The machine has four CPUs and fifteen gigabytes: `~/.cargo/config.toml`
  pins `jobs = 2`, gates run with `CARGO_BUILD_JOBS=1`. Disk is tight: test
  executables over 20 to 30 MB and duplicate rlibs under
  `target/debug/deps` are deleted after each run, never while that tree's
  implementer is running; a gate worktree's `target/` can be removed
  whenever it is idle.
- The session has restarted several times; drafts survive on disk, so a
  restart is recovered by reading `git status` and `git log` in each
  worktree and relaunching the implementer on the draft.
