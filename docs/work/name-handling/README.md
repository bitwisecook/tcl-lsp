# LSP naming migration on work

Updated 04 November 2026, 08:39 UTC. [current-state.md](current-state.md) is the authority for source-bound results, committed work, private drafts and blockers. [outstanding/README.md](outstanding/README.md) lists current responsibilities.

**The LSP is not release-ready.** The user prioritises the LSP and its transitive libraries. Separate VM, BIG-IP and ancillary feature development is deferred; linked VM/SpecTcl execution and required repository gates remain in scope.

Reviewed work and truthful status are pushed to `work` every five minutes with remote ancestry checked first. [repository-sync.json](repository-sync.json) records actual checkpoints, not test acceptance. No PR, merge, tag or release exists.

Current selected Compiler controls pass6/6. Sealed linked RMWv6 builds/run custody pass, but tests fail: Engine1/6, hooks10/13, VM Jim1/1. The Engine ScriptBytes test correction is adopted but unrerun; budget/F5 failures are unresolved. Earlier Server7/7 and native15+20+5 passes predate current style/index changes and require final repetition. Closed-cut Kernel V5 passes11/11 selected software controls with unchanged source/generated/dependency/binary/runner seals. Host-onlyv5 and negative rename wire tests are adopted; initial type checking ran, focused execution remains pending; Core/autoload vectors and cold-host plans remain unexecuted.

Ordinary [CI41781045833](https://github.com/bitwisecook/tcl-lsp/actions/runs/41781045833) is bound to caf4be84a; its06:56 snapshot has9success/15failure/5active/2skip. Separate [LSP bootstrap41788257234](https://github.com/bitwisecook/tcl-lsp/actions/runs/41788257234), sourcef5a527c, is terminal1success/3failure; Native tests were blocked by same-step nextest custody verification after successful installation. Browser Node and WASI rename assertions fail; separate Chromium extension smoke succeeds narrowly. Previous [LSP run41794544977](https://github.com/bitwisecook/tcl-lsp/actions/runs/41794544977), source95acc3bc, is terminal1success/3failure solely on the old Box/Arc compile error before its fix. Newest [run41798727716](https://github.com/bitwisecook/tcl-lsp/actions/runs/41798727716), exact `a857c60dd25aa1a58a924aecd6a0870516f2cbf7`, started08:20 and remains active at08:27; it predates new host-only adoption. No result replaces required PR-event, final-current host or packaging acceptance.

Canonical Runtime651179-row and seven-source997-row derivation corrections are committed and scoped checks pass. The prior global2181-error diagnostic has not been recounted; no aggregate proof pass or Native-authority transfer is claimed. Historical audits and the [external issue sweep](issues.md#external-reporter-issue-sweep--3-november-2026) remain limited to their original sources.

Use the [release plan](lsp-release-pr-plan.md) and current snapshot's acceptance checklist. Historical pause/continue prompts do not override the user's instruction to complete the LSP.
