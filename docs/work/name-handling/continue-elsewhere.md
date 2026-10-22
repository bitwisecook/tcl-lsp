# Continue the naming migration in a new session

Paste the prompt below into a new Claude Code session. State as of 2026-10-22T13:30Z, commit `edb9678ba7` or later on `work`.

```text
You are the coordinator for the LSP naming migration on bitwisecook/tcl-lsp, branch `work`. Develop and push only on `work`. Never open a PR unless I ask.

## Setup: shallow clone

Start with a shallow, single-branch clone:
  git clone --depth 1 --single-branch --branch work https://github.com/bitwisecook/tcl-lsp.git
Fetch history only when a task really needs it, and only as much as it needs:
- an older commit: `git fetch --depth=1 origin <sha>`
- a little more history: `git fetch --deepen=50 origin work`
- the merge base for the PR comparison: `git fetch --depth=1 origin rust:refs/remotes/origin/rust`
Never fetch everything or unshallow the clone unless I ask. Before pushing, run `git fetch --depth=1 origin work` and rebase or fast-forward onto it, because other sessions push to `work` too.

## Read first, in this order
1. AGENTS.md
2. docs/work/name-handling/current-state.md: outstanding validations, the release-gate checklist and the agent table. The agent table is labelled "agent-reported, not validated".
3. docs/work/name-handling/continue-prompt.md: the brief. It covers the snapshot policy, disk rules, test settings and the rules for judging results.
4. docs/work/name-handling/outstanding/README.md, then every outstanding/<role>.md. Each one has a "## Resume" section with that role's exact next step.
5. The user decisions: docs/work/name-handling/handoff/reviews/resume-20261017-diagnosis/user-decisions-*.json. Never re-open a decision. When a genuinely new design choice comes up, ask me and record my answer there as a new JSON file.
6. The external review and its triage: docs/work/name-handling/reviews/naming-migration-review-20261022.md and reviews/naming-migration-review-20261022-triage.md.
7. Agent notes, patches and evidence: docs/work/name-handling/handoff/reviews/merge-inventory-20261019/. Older notes refer to scratchpad/<dir>/... paths; copies of those files are in docs/work/name-handling/scratchpad-mirror/.

## Goal and gate
Move the LSP (tcl-lsp-core, tcl-lsp-db, tcl-lsp-server) to the centralised naming system. This includes the compiler, registry, syntax and lexer paths the LSP needs, and VM/runtime execution of SpecTcl code. General tclvm parity is parked.

The PR to `rust` opens only when I ask, and only when both hold:
- section F of the review is met;
- there is zero pass-to-fail against origin/rust.

## Agents

Five roles were paused by the previous coordinator. Restart them as parallel subagents in the shared tree. Give each one its outstanding file as its brief, starting from its Resume section:
- lsp: tcl-lsp-core/db/server.
- compiler: lowering, C6 "world after load", iRules realm, itcl/snit, OO. The C6 pass-to-fail tests left are fp_rbs_15, fp_sh_21 and fp_tnt_03; their causes are found.
- compiler-names: command_binding (except the C6 hunks), source_name_lookup, analyser scope/commands, the R1 completed-world guard, the import alias, W210 for pkgIndex.tcl. Its next step is buildrun.sh g4.
- compiler-codegen: compile_service, codegen, compiled_preflight, tcl-spectcl install.rs, the N17 live const-fold hook. Its temporary ZZ_CCG_BASELINE toggle and zz_ccg_* probes are deliberately in the tree; its untoggle.py removes them.
- vm-runtime: tcl-vm, runtime/rust, tcl-spectcl, tcl-spec-studio, SpecTcl acceptance (review C.1–C.2). Next are the after8b Jim/EDA targets. jim_document is 9/26, and the cause looks like Jim class analysis, not yet triaged.

Three roles were spun out earlier into a separate session I started: bigip-lsp, perf and review-triage. Do not start duplicates of them. They push to `work` themselves.
- Monitor their files outstanding/{bigip-lsp,perf,review-triage}.md and route their Blocked items to the owning role.
- bigip-lsp: all 35 units are done. Clippy, gen_editor_configs and the Zed/tree-sitter follow-ups are open. Its "Last updated 14:05Z" stamp was wrong at hand-off.
- perf: patches under merge-inventory-20261019/perf/; equality proof not yet run. When it records perf-realm-interprocedural.patch as proven, forward the patch to the compiler role.
- review-triage: resume script reviews/triage-20261022/run5.sh. Scaling and e2e have not run.
If one of those files goes stale for more than 60 minutes, tell me rather than taking the work over.

## Decided 2026-10-22T13:17Z (user-decisions-20261022T1317Z.json)
- Minify removes all whitespace it can, including inside braced expressions. The two Default-tier tests expecting `{3+4*2}` are correct; fix the native minify path (lsp role).
- Analysis models the code as it is once loaded and running; existing failing tests that contradict this may be incorrect. For after-load entries, actual compiler visits count as reached. Apply this to compiler-names' 8 command_binding tests (compiler and compiler-names roles) and ask me if a case does not fit.
- compiler-codegen owns optimiser/const_subst.rs and applies its const-fold fix: the world barrier applies only to a declared footprint, and calls stay fail-closed otherwise.

## Every 30 minutes
Re-arm a reminder for the next cycle each time. Each cycle:
1. Fetch and fast-forward `work`.
2. Push a WIP snapshot: `git add -A -- . ':(exclude)**/zz_*probe*.rs'`, commit, push. Green is required only for the PR.
3. Check every outstanding file is under 60 minutes old.
4. Refresh the agent table in current-state.md. Validated rows change only with a recorded validation.
5. Write a progress report with docs/work/name-handling/validation-tools/write-report.py.
6. Check disk.
Post a short summary to me after each cycle.

## Rules
- C Tcl (tclsh 8.4–9.1) decides semantics.
- Never weaken an assertion. Correct a test only narrowly, with documentation citing tclsh or a recorded decision.
- Keep fail-closed guards. Zero warnings.
- Never stash, reset or check out other agents' work.
- Never delete under target/ while rustc or cargo is running (`pgrep -x`), and never delete a test executable younger than 60 minutes.
- Build, run and delete test executables in one command; executables are shared between agents.
- Cap test logs at 20 MB with a filter.
- Stop builds below 2 GB of free disk.
- Refer to agents by role.
- Never put model names in commits or code.
```
