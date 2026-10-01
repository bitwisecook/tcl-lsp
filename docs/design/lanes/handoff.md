# Hand-off: resuming the value-transfer build

This page is the orchestrator's resume point for the work on branch
`claude/spectcl-optimization-discussion-5qhf42` (issue #1943: value
transfers, the diagnostic policy, the registry consumer contracts). It is
rewritten at every push. The two lane documents carry the detail; this page
says where each lane stands, what is queued, and how the work is run.

## Where the lanes stand

| Lane | Tracking document | Landed | In flight | Remaining |
|---|---|---|---|---|
| Value transfers | [value-transfers.md](value-transfers.md) | slices 1, 2, 3, 4, 5, 8, 6 (reviewed, reworked in three rounds, re-checked "land as is"); slice 9 items 9.1, 9.3, 9.4, 9.2, 9.5 and the slice 6 round-three review fixes | slice 9's landing item VT9.6, then its fable review and fixes | slices 10, 11, 12, 7a, 13, 7, in that order |
| Consumer contracts | [consumer-contracts.md](consumer-contracts.md) | steps 1 to 7, each reviewed with fixes landed; step 8 built, landed and in rework after its review | the step 8 rework (group 1, the inliner, is committed; groups 2 and 3 follow), then step 9 | step 9 (after the rework), step 10, the step 8 re-review |
| Diagnostic policy | (landed; see the design page `docs/design/compiler/diagnostic-policy.md`) | complete | | |

Branch heads at the last rewrite of this page: the main branch at the
commit that adds this page (above `d256e8cc`, slice 9's witnesses);
`cc-step5` at `e9aaa1de` (the inliner rework), which already contains the
main branch up to the slice 6 round-two fixes (`c932864a`).

## Queued on the running implementers

- Value transfers: nothing beyond VT9.6; the slice 9 review comes after it.
- Consumer contracts: the step 8 rework groups 2 (derived implementations
  become an explicit author opt-in; prefix-matched ensemble words in the
  scan) and 3 (`tcl spec test`: a `SPEC-TEST done N` line required whatever
  the exit status; the policy taken from the operator's project, never the
  pack's own tree; namespace variables in the purity probe; the timeout's
  exit status and the `source -encoding` names documented), then step 9.

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

- Three worktrees: the main worktree on the branch (value transfers), a
  second worktree on `cc-step5` (consumer contracts; merged into the main
  branch at a value-transfer checkpoint after each step's review), and a
  third detached worktree with its own `target/` for gates and reviews.
- Every committed checkpoint is gated with `make rust-check` plus
  `cargo xtask dialect-drift` (eight pre-existing sites; a lane adds none)
  in the gate worktree, then pushed with
  `git push -u origin <hash>:refs/heads/claude/spectcl-optimization-discussion-5qhf42`.
  Nothing is pushed to any other branch.
- Each landed slice or step gets a fable review in the gate worktree; its
  findings land as "review fixes" commits (blocking ones before the next
  item), and pre-existing defects found in passing become GitHub issues
  (#2253 to #2272, #2291 to #2297, #2299 to #2303, #2305 to #2316, #2323
  to #2328, #2330 to #2333 so far), never fixes on this branch.
- Implementers are sonnet, reviewers fable. Commit messages are
  `wip(<lane>): <slice or step> — <phrase>` with the session's two trailer
  lines; files outside this directory describe current state only.
- The machine has four CPUs and fifteen gigabytes: `~/.cargo/config.toml`
  pins `jobs = 2`, gates run with `CARGO_BUILD_JOBS=1`, and the three
  worktrees carry a rustup directory override to `1.98.1` (the repository
  pins `stable`; the owner asked that no Rust update happen on this branch).
  Disk is tight: test executables over 20 to 30 MB and duplicate rlibs under
  `target/debug/deps` are deleted after each run; the gate worktree's
  `target/debug/deps` can be removed whenever it is idle.
- The session has restarted several times; drafts survive on disk, so a
  restart is recovered by reading `git status` and `git log` in each
  worktree and relaunching the implementer on the draft.
