# LSP naming migration on work

Updated 04 November 2026, 05:54 UTC. [current-state.md](current-state.md) is the authority for current scope, committed work, validation, drafts and blockers. [outstanding/README.md](outstanding/README.md) lists active responsibilities.

The user prioritises the LSP and its transitive libraries. Separate VM, BIG-IP and ancillary feature development is deferred; required linked dependencies and repository gates remain in scope. **The LSP is not release-ready.**

Reviewed code and truthful status are committed and pushed to `work` every five minutes while this task is active, with remote changes fetched first. The publication interruption and restart are recorded in the current snapshot. [repository-sync.json](repository-sync.json) records actual successful sync checkpoints; it certifies no tests. The manual CI runs and their exact source hashes are linked in the current snapshot. Neither establishes a green release candidate or required PR-event acceptance. No PR or release exists.

The current Main/scratch-equal Server cohort passes 7/7 selected controls. The default-feature native binary `7287fc0deff6` builds with matching source/generated seals; native protocol15/15 and mandatory targeted20/20 pass. The five-arm diagnostic also succeeds. Strict60s startup fails at autoload/package definition; packaging acceptance remains pending. Old [CI41749944578](https://github.com/bitwisecook/tcl-lsp/actions/runs/41749944578) is terminal with 8 successes, 23 failures and 16 skipped jobs. New hosted [CI41781045833](https://github.com/bitwisecook/tcl-lsp/actions/runs/41781045833) runs on `caf4be84a`; no passing aggregate is recorded. These observations do not replace the current snapshot's required gates.

The canonical proof corpus remains the authority for semantic/provider claims. Older audit reports, ledgers and role notes retain their original snapshot claims; their counts and timestamps are not a current verified-completion recount. Scoped source/helper passes do not transfer Native provider authority. No new aggregate completion count is asserted.

The 3 November external-reporter issue sweep is itemised in [issues.md](issues.md#external-reporter-issue-sweep--3-november-2026); its results remain bound to the recorded original/rebased sources.

Use the [release plan](lsp-release-pr-plan.md) and current snapshot’s acceptance checklist. Historical pause/continue prompts do not override the user’s instruction to complete the LSP.
