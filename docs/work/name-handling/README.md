# Centralised name handling on work

**Active since the origin/rust merge (19 October 2026).** Scope is the LSP, the compiler stack it requires, and VM/runtime execution of SpecTcl; progress reports run every 30 minutes. [current-state.md](current-state.md) is the single list of outstanding validations. The paragraphs below describe the 17 October checkpoint.

Resume with [continue-prompt.md](continue-prompt.md). [current-state.md](current-state.md) records current validation and blockers; [implementation-ledger.md](implementation-ledger.md) identifies the latest admitted changes. The authoritative per-unit audit remains [audit-status.json](audit-status.json) and [repository-consumer-audit-final.json](repository-consumer-audit-final.json).

Source-adoption counts are **570/2158 baseline**, **640/2229 effective**, and **290/325 in the original LSP area**. **Zero units are verified complete.** The original LSP-area count does not include every direct Studio/DB consumer:25 existing Studio NONE rows still have pending work. Helpers and unadmitted packets receive no credit.

Finish and verify LSP itself, including its direct Studio/DB consumers and genuinely necessary shared prerequisites; then migrate and verify the remaining compiler dependencies. VM/runtime migrations remain paused. Dedicated BIG-IP/iRules/ILX/XC/APL/tmsh semantics remain deferred; unmeasured BIG-IP is an assumption of Tcl8.4, never measured evidence. Reports belong only in this chat and repository.

The latest [compact checkpoint09](../../../experiments/name-handling/results/owner-adoption-20261017/lsp-generic-consumers-09/capture.json) preserves admitted source34/35, build results, the interrupted capture, and separately marked unadmitted proposals. The archive is1,060,094 bytes; no binaries or complete source/provider trees are stored. [Checkpoint details](../../../experiments/name-handling/results/owner-adoption-20261017/lsp-generic-consumers-09/checkpoint.json) include pending chains and exact blockers.

Supported build35r2 passed for six selected library test targets. The interrupted run completed790/1085 planned selectors:636 passed,154 failed including3 timeouts. Source/provider/test-ELF post-stop guards passed. This is a partial failed capture, with no full-run or semantic acceptance. Required integration, actual server process, editor-host, proof, generated, `make rust-check` and `make prep-pr` gates remain unaccepted. Development commits before green are authorised; no green PR, merge or issue closure is authorised by this checkpoint.

The [17:39 report](progress-report-20261017T1739Z.json) and earlier reports are immutable historical snapshots. Resume the30-minute chat/repository reporting cadence when active work resumes.
