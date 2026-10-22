# Continue the naming migration in a new session

Paste the prompt below into a new Claude Code session on `bitwisecook/tcl-lsp`. All agents were paused on 2026-10-22 at about 13:20 UTC. Their state is on `work`.

```text
You are the coordinator for the LSP naming migration on bitwisecook/tcl-lsp, branch `work`. Develop and push only on `work`. Never open a PR unless I ask.

Read first, in this order:
1. AGENTS.md
2. docs/work/name-handling/current-state.md: outstanding validations, the release-gate checklist and the agent table.
3. docs/work/name-handling/continue-prompt.md: the brief. It covers the snapshot policy, disk rules, test-run settings and the rules for judging results.
4. docs/work/name-handling/outstanding/README.md, then every outstanding/<role>.md. Each one has a "## Resume" section with that role's exact next step.
5. The user decisions: handoff/reviews/resume-20261017-diagnosis/user-decisions-*.json under docs/work/name-handling/. Never re-open a decision. When a genuinely new design choice comes up, ask me and record the answer as a new JSON file there.

Goal: move the LSP (tcl-lsp-core, tcl-lsp-db, tcl-lsp-server) to the centralised naming system. This includes the compiler, registry, syntax and lexer paths the LSP needs, and VM/runtime execution of SpecTcl code. General tclvm parity is parked.

Merge gate: the PR to `rust` opens only when I ask, and only when both hold:
- section F of docs/work/name-handling/reviews/naming-migration-review-20261022.md is met;
- there is zero pass-to-fail against origin/rust.
Per-finding status: reviews/naming-migration-review-20261022-triage.md.

Restart these paused roles as parallel subagents. Give each one its outstanding file as its brief, starting from its Resume section:
- lsp: tcl-lsp-core/db/server.
- compiler: lowering, C6 "world after load", iRules realm, itcl/snit, OO.
- compiler-names: command_binding (except the C6 hunks), source_name_lookup, analyser scope/commands, the R1 completed-world guard, import alias, W210 for pkgIndex.tcl.
- compiler-codegen: compile_service, codegen, compiled_preflight, tcl-spectcl install.rs, the N17 live const-fold hook.
- vm-runtime: tcl-vm, runtime/rust, tcl-spectcl, tcl-spec-studio, SpecTcl acceptance (review C.1–C.2).

The bigip-lsp, perf and review-triage roles are already running in my other session. They push to `work` themselves. Monitor their outstanding files and route their Blocked items. When perf records perf-realm-interprocedural.patch as proven, forward it to the compiler role.

Older notes refer to scratchpad/<dir>/... paths. Copies of those files are in docs/work/name-handling/scratchpad-mirror/.

Every 30 minutes, run this cycle and re-arm a reminder for the next one:
1. Fetch `work` and fast-forward.
2. Push a WIP snapshot: `git add -A -- . ':(exclude)**/zz_*probe*.rs'`, commit, push. Green is required only for the PR.
3. Check each agent's outstanding file is under 60 minutes old.
4. Refresh the agent table in current-state.md. It is labelled "agent-reported, not validated"; validated rows change only with a recorded validation.
5. Write a progress report with validation-tools/write-report.py.
6. Manage disk, following the rules in the brief.

Rules:
- C Tcl (tclsh 8.4–9.1) decides semantics.
- Never weaken an assertion. Correct a test only narrowly, with documentation citing tclsh or a recorded decision.
- Keep fail-closed guards. Zero warnings.
- Never stash, reset or check out other agents' work.
- Never delete under target/ while rustc or cargo is running, and never delete a test executable younger than 60 minutes.
- Build, run and delete test executables in one command.
- Cap test logs at 20 MB with a filter.
- Refer to agents by role.
- Commit messages carry the session's attribution lines. Never put model names in commits or code.
```
