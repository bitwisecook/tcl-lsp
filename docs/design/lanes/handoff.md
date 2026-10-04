# Hand-off: resuming the value-transfer build

This page is the orchestrator's resume point for the work on branch
`claude/spectcl-optimization-discussion-5qhf42` (issue #1943: value
transfers, the diagnostic policy, the registry consumer contracts). It is
rewritten at every push. The two lane documents carry the detail; this page
says where each lane stands, what is queued, and how the work is run.

## Where the lanes stand

| Lane | Tracking document | Landed | In flight | Remaining |
|---|---|---|---|---|
| Value transfers | [value-transfers.md](value-transfers.md) | slices 1, 2, 3, 4, 5, 8, 6 (reviewed, reworked in three rounds, re-checked "land as is") and 9 (`f06639de`); slice 10 item 3 complete: items 1, 2, 3a to 3e and 3d's remainder (`ddb53a10`, `b9ab4993`, `7ee77e66`, `6f86956d`, `07633485`, `e0675133`, `22be2fd0`, `aa0cc76e`) with the SpecTcl catalogue fix `1d3a937f`; item 4 complete (`513196e5` the reconciliation note, `596a5d86` the registry's handler chain and `try`'s protocol, `2ebf9756` the CFG half carrying `rust`'s #2230 change and reading the chain, with two `try` lowering defects fixed on the way); item 5 complete (`2e4cf3b9`: the prefix rule in the faithful-exceptions build, a `try` body split per statement with every split point wired to its handlers and `finally`, closing D225's open case; 1033 programs differential-clean on tclsh 8.6, 9.0 and 9.1 apart from the pre-existing O125 and O126 defects); the item 3 fix `823e1287` (a `catch` body's store counts as run only where its target is proved writable, closing the lane's own miscompile where a store to a variable that may be an array was taken as succeeding; D244); item 6 (`2d40a40f`: a loop absorbs `break` and `continue`, the registry's loop rule stating each loop's result, `foreach` empty and `lmap` the collected list; D245); item 7 part 1 (`2b67421e`: O109 and O126 read a chain's uses through the running-use rule, a store a later definition preserved counting as read, closing the lane's own item 3d defect; D246); item 7 part 2 (`754baa7b`: `scan` goes on past a store it cannot make as tclsh does, a write in a substitution-run `catch` or `try` body is a may-definition, a `try` body that never rests gets its region entry; D247 to D249, correcting D232 and D224); item 8 (`613c2147`: the slice's witnesses in all four build shapes, the registry's routes against tclsh 8.4 to 9.1, three CLI witnesses); item 9 (`379982e3`: the docs sweep, with two precision gaps recorded in `precision-limitations.md`); the landing (`4e7b15f2`: "slice 10" in the `registry-axes` gate's LANDED list, the ledgers regenerated unchanged, the migration page, the lanes README bullet and the lane doc's status section with what slice 11 starts from). Slice 10 is landed: slices 1 to 6 and 8 to 10. The upstream merge then reshaped three of the lane's rules on the merged tree (`a1e32956` one marker for calls to code the module cannot see, correcting D195: an unnamed head widens the caller's locals held at the call once, a named head's frame effect comes from the registry, D250; W210's silence in a procedure is per name, for a name the callee is handed as a literal word and after `source`, D251; `bfca7a26` the raise proof reads the existence rung, which takes the registry's write class where a declared existence step is a may-bind or the generic widening, D252) and fixed one of the lane's own defects (`5b247801`: the deferred-writes scan reads an alias's command prefix as one command); the slices 9 and 10 review's first fix (`7f1d9c8b`: a raising store is never dead, the raise proof refusing a definition the solver answers Raised for and a preserved definition, the coupled constant dead-store removal refusing both, a command substitution in a value word qualifying only where the solver folds the definition; sixteen witnesses kept under tclsh 8.4 to 9.1; four tests that pinned the deletion of an unproven call's store restated with their oracle; D253); the second (`3d36d2bf`: one decoding of `return`'s options in the registry, read release by release as tclsh reads them and shared by every reader, a level-0 `return` lowered as the completion it names so the CFG reads an error as a throw point and `break` or `continue` as the jump, the rest staying the return barrier; closes #2357; D254); the third (`f1708203`: one helper finds each release's reference tclsh at its pinned patchlevel, a witness that cannot find one says so on stderr or fails where `TCL_LSP_REQUIRE_TCLSH` names the release, and CI requires 9.0, so the witnesses compare there); the fourth (`0e43c05e`: a stub states its frame effect, `-frame own|none|caller`, through the registry's own frame-effect field, so a plain-call stub so declared is bound beside the catalogue's names and widens nothing, a `caller` stub blinds the computed-name walk and the deferred-writes scan as argparse does, and W123 says in a procedure's own frame what the call widens and what a stub keeps; D255); the fifth (`1a4f44dc`: one vocabulary for a handler's completion code, a `try` `on` clause holding the registry's completion code that one decoder reads, the separate selector type gone); the sixth (`ae3e6af0`: W123 carries its widening sentence only where an unseen-call marker covers the head, the one fact the widening reads, so the top level and a `namespace eval` body get it and an `uplevel #0` body inside a procedure does not, and the stub pages say `tcl opt` does not read stub declarations yet, #2366; D256); the seventh and last (`2a1c7745`: a callback's lambda writes what its body writes, the deferred-writes scan reading an `apply` lambda as `apply` does and asking the procedure summary what the body writes in the global frame, the summary now counting a barrier's bindings and a structured statement's own bindings; the 384-run sweep of writer lambdas in callback forms against tclsh 8.5 to 9.1 goes from 72 mismatches to 4, all one program on the pre-existing #2367; D257). The slices 9 and 10 review is closed. Slice 11 checkpoint 1 (`0c02a60e`: the condition transfer's refinements recorded on executable edges, the registry's `string is` member type proving a type only for the integer and double classes under `-strict` and for `dict` as `tcl::unsupported::representation` reads under 8.6 to 9.1, the Explorer's SCCP view listing refinements on executable edges only; `tcl opt` and `tcl diag` byte-identical over 1231 files) | slice 11's landing: the nested equality deciding I230, `string is` typing its arm, the ranges read from the condition transfer's Range rows with the ordered half-lines (closing #2369), the guard helpers deleted; a definition in a refined arm takes the arm's value | slices 12, 7a, 13, 7, in that order |
| Consumer contracts | [consumer-contracts.md](consumer-contracts.md) | steps 1 to 9, each reviewed with fixes landed (the step 8 rework and step 9 review's fixes are `16e3a5a0`, `1b870f36`, `cdc7307a`); step 10 items 1 to 5 and item 6's first three parts (`69f5211b` the extension default, `ca5cc729` an extension described from three sources, `75b750df` the host load bridge, `141d30f0`, `2d570cf7` and `ecc1e4e2` one header for two hosts with its C-extension WASM gate, `599e7ac3`, `fb1c1dfe`, `0a562616` and `bfbac02f` the engine's completion, variable and package doors with the shim's side, its second test extension and the return options crossing the door; `56515003` the WASM runtime grown to a real extension's C API surface; `3d3956fd` `tcl spec import` describing one entry point at a time; `a3ce8d4c` the runtime as an engine with its own limits and confinement, every family run on both engines; `4f7d977a` the runtime compiled to `wasm32-wasip1` as an engine under in-process wasmtime in the new crate `rust/tcl-engine-wasm`, with the registry's extension-host seam; `1b4a5c78` on the main branch, the LSP e2e archive's committed closure taking `runtime/rust`, which the hook host's dev-dependency on the runtime pulled in and the merge gate caught; `c6e23a53` item 6 landed, its part 3 mutation run killing all twenty-one mutants after two tests were strengthened; `55feb617` step 10 landed: "step 10" in the `registry-axes` gate's LANDED list, the design page's status box, both design indexes, the lane doc's landing sections and the lanes README; `d8549edd` the step 10 review fixes: the C API's integer reads take C Tcl's ranges on every host, one `content_hash` helper for the artefact and implementation identities, a reused WASM engine granting its first-use fuel once, the extension host's evaluate path noted and the runtime's export list pinned, and two defects found on the way, an evaluation's epoch deadline outliving it and the side-module loader resolving any runtime export rather than the C API alone) | nothing: the lane is complete and reviewed | nothing |
| Diagnostic policy | (landed; see the design page `docs/design/compiler/diagnostic-policy.md`) | complete | | |

Branch heads at the last rewrite of this page: the main branch at the
commit that rewrites this page, above `0c02a60e`, slice 11's first
checkpoint, itself above `2a1c7745`, the last of the seven slices 9 and
10 review fixes, and `67d9ba5f`, the tip of the upstream merge (`086a3bc3` merges `origin/rust` at `aac0e0d5` above
`4e7b15f2`, then ten fix commits and the lane-doc record); `cc-step5` at
`d8549edd`,
merged into the main branch as `1cda89b6`, and receiving no further
commits now that its lane is complete. `cc-step5` is
merged into the main branch, gated and pushed after every one of its
commits, so nothing of either lane stays off GitHub for longer than one
gate cycle.

## Queued on the running implementers

- Value transfers: slice 11 (predicate refinement) on the merged tree;
  the slices 9 and 10 review's seven fixes are landed and pushed. A stub
  states its frame effect since `0e43c05e`; a stub with a
  body, expression, command-prefix or variable role stays unseen whatever
  its `-frame`, because the flow graph reads roles off the catalogue and
  not the document's declarations, and naming it would lose its body's
  writes; reading the document surface there is a follow-up if wanted.
  Item 4
  (`try`) was shaped to reconcile with `rust`'s PR #2230
  (the try/finally reachability fix for #2142, absent from this branch's
  base) at the upstream merge: one implementation per fact, with a
  "reconciling with #2230" paragraph in the lane doc naming the files and
  facts the merge has to settle. The slices 9 and 10 review is done: its
  findings are the fixes above, and the pre-existing defects it found are
  issues.
- Consumer contracts: complete, every step reviewed and its fixes
  landed. The extension evaluation route's vocabulary (`HostKind::WasmExtension`, the `extension FILE PREFIX` row,
  the artefact hash in the memo key) is value-transfer slice 7's to add
  (its item VT7.11); item 6 built the host side behind the registry's
  `ExtensionHost` seam, which slice 7 binds.

## After the lanes

1. Done at `086a3bc3` to `67d9ba5f`, with the reconciliation recorded in the
   lane doc's slice 10 Record: merge `origin/rust` (at `aac0e0d5`, 254 commits above this branch's
   base `08bceb36`: the dialects/tclspec editor-config generation, the
   jimtcl updates, the Rust 1.99.0 baseline this branch already carries,
   PR #2230, and PR #2196's handler-barrier series, which lands in the
   same compiler code as slice 10) into the branch right after the slice
   10 landing commit, before the slices 9 and 10 review and before slice
   11, so that the review and the later slices build on the merged tree.
   A dry run at `2d40a40f` showed 76 conflicting files: fifteen in
   `rust/tcl-compiler/src` (`cfg_builder/mod.rs` and `cfg_lower.rs`,
   `sccp.rs`, `ssa.rs`, `optimiser/elimination.rs` and `propagation.rs`,
   `ir.rs`, `ir_helpers.rs`, `unit_scope.rs`, `lowering/mod.rs`,
   `tcl_expr_eval.rs` among them), ten in `rust/tcl-registry/src`,
   twenty-four in `rust/tcl-spectcl/tests`, and single files in
   `tcl-dialect`, `tcl-cmd-core`, `tcl-vm`, `tcl-syntax`, the studio, the
   MCP server, the language server, the CLI tests, the shard table, the
   Makefile, the glossary and three design pages. The lane doc's
   "reconciling with #2230" paragraph names the files and facts the merge
   has to settle, #2196's included. Reconcile through the registry
   interface (one implementation per fact), regenerate every generated
   artefact with the repository's tooling, rerun both lanes' suites and
   the gate, push. Repeat before the end if `rust` moves.
2. The final scrub before the pull request to `rust`: delete this
   directory and every link to it, remove slice, step, decision and lane
   references from every page and comment outside it, retire the
   `registry-axis-ok … until` clause and the `LANDED` list in
   `rust/xtask/src/registry_axes.rs`, keep the `value-transfers` ledger
   only in the form its gate needs, and review the result before pushing.
3. The pull request's first CI run is the first time the output witnesses
   compare under the pinned Tcl 9.0 there (`f1708203` made CI require it
   through `TCL_LSP_REQUIRE_TCLSH=9.0`; before that the witnesses looked
   only on `PATH`, where CI's 9.0 is not), so read that run for a 9.0
   witness that never ran in CI before.
4. A last fable review of the whole branch, and the artifact mirror
   (`claude.ai/code/artifact/f1aa5423-0def-49ac-bdb9-30dc19f65830`, at
   Version 8 as of 1a67ca4f) brought up to date.

## How the work is run

- Four worktrees: the main worktree on the branch (value transfers, and
  the upstream merge), a second worktree on `cc-step5` (consumer contracts,
  complete; its `target/` was removed to free disk and is rebuilt only if a
  review needs it), and two detached worktrees with their own `target/` for
  gates and reviews (one can hold a running review while the other gates a
  push; the first one's `target/` is also removed).
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
  mirror the registry's id tables, and both lanes' suites include
  `tcl-cli`. `cargo test -p <crate>` stops at the first failing test
  binary, so a failure hides the binaries after it until it is fixed.
- Each landed slice or step gets a fable review in a gate worktree; its
  findings land as "review fixes" commits (blocking ones before the next
  item), and pre-existing defects found in passing become GitHub issues
  (#2253 to #2272, #2291 to #2297, #2299 to #2303, #2305 to #2316, #2323
  to #2328, #2330 to #2335, #2337 to #2358, #2365 to #2369 so far), never fixes on this
  branch.
- Implementers ran on sonnet until the account's weekly sonnet limit was
  reached on 2 October (it resets on 6 October, 16:00 UTC) and run on opus
  until then; reviewers are fable. Commit messages are
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
  whenever it is idle; the extracted registry sources under
  `~/.cargo/registry/src` can be removed whenever no dependency is
  compiling, since cargo re-extracts them from its crate cache. The
  consumer-contract tree holds several rlib variants of the large crates
  (one per feature set it builds), which is inherent to its feature matrix.
- Each agent keeps its scripts in a private directory under the scratchpad
  (two agents once collided on a shared `gates.sh`), and nothing is run with
  a gate worktree as its working directory except the gate itself. A review
  of a lane whose implementer is idle runs in that lane's worktree, whose
  `target/` is warm, read-only and with every experiment reverted; the
  implementer is resumed afterwards to land the fixes.
- The session has restarted several times; drafts survive on disk, so a
  restart is recovered by reading `git status` and `git log` in each
  worktree and relaunching the implementer on the draft.
