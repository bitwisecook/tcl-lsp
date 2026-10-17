# Centralised name handling on work

Active work is all LSP consumers and their required shared Compiler/DB/Registry owners. Runtime/VM and BIG-IP migration work remains deferred. Uncovered BIG-IP behaviour uses the Tcl 8.4 working assumption separately from measured evidence.

The immutable LSP baseline is **325 obligations: 201 semantic, 122 validation and two documentation**. **229 have integrated changes pending validation: 116 semantic, 111 validation and two documentation**. There are 96 without integration and zero verified complete. The 2,158 baseline has 363 integrated (327 full scope pending validation and 36 partial), 1,795 without integration. The annex has 64 required, 63 integrated, one blocked. Effective scope is **2,222 required / 426 integrated / 1,796 without integration / zero verified**. Both JSON catalogues have identical implementation_progress. Helpers, API methods and new tests add no counts.

Current34-path reviewed batch is integrated with151cumulativeadmittedpaths; fresh five-library buildPASS323.055s39,058inputsunchanged, all5ELFshashverified; editor443inputsunchanged allchecksPASS. Focused2PASS withallprepostchecks; full994running. Extra3librariesbuildPASS201.675s; xtaskbuildrunning beforeextra18. Previous standard962 completed **763 PASS /196 FAIL /3 TIMEOUT**, allretained930outcomesunchanged; extra18 **16 PASS /2 FAIL**. Final source/artifact/provider checks matched, but the temporary source-freeze violation is explicitly preserved and ordinary acceptance remainsfalse. Both SpecTcl failures persist. All original assertions/fixtures/selectors/budgets retained. Actual host and repository gates remain unaccepted. See [current state](current-state.md), [exact catalogue](repository-consumer-audit-final.json) and [implementation ledger](implementation-ledger.md).

The source/role audit is complete for **110,709 tracked baseline paths**, with explicit current-source annexes and zero unexamined paths. The reconciled catalogue contains **1,321 caller migrations**, **377 shared-owner repairs/extensions** and **101 metadata/target-schema changes**: **1,799 semantic code changes**. There are separately **305 validation-code repairs** and **54 authored-documentation corrections**, making **2,158 confirmed required changes**. Implementation progress is tracked separately from these immutable audit totals. Examined requirements and original/provider provenance questions remain separate.

[Searchable report](naming-audit.html) · [Full JSON catalogue](repository-consumer-audit-final.json) · [Current validation](current-state.md) · [Implementation obligations](implementation-ledger.md)

The [beginner API and integration guide](https://chatgpt.com/space/page_87188db946408191aa04822f5b653a90) explains dialects, SSA/SCCP, arrays and dictionaries with diagrams.

Keep written source, evaluated names, namespace identity, command slots, live command identity, variable places/cells and display text distinct. Shared owners retain original bytes, spans, complete configuration, source ownership, namespace/frame/interpreter context and lifetimes/generations. A display string cannot reconstruct identity. Source advice grants no Native execution, physical cell, entered frame, absence, edit/optimisation authority or normal completion. Repair missing owner transport instead of weakening guards or adding String inverses.

| Area | Semantic code | Validation repairs | Authored docs | Total |
|---|---:|---:|---:|---:|
| Compiler, analyser, inference and optimisation | 297 | 44 | 0 | 341 |
| Database, registry and SpecTcl | 177 | 7 | 0 | 184 |
| LSP, server, editors, iRules and MCP | 201 | 122 | 2 | 325 |
| Runtime, shared Core/API and hosts | 370 | 5 | 1 | 376 |
| VM, codegen and registered targets | 319 | 9 | 0 | 328 |
| Additional tooling, query and target consumers | 164 | 16 | 8 | 188 |
| Repository-wide tooling, generated and guidance consumers | 271 | 102 | 43 | 416 |

One change means one actual integration boundary with a distinct purpose. Count a genuinely missing shared-owner facet once, then each caller whose code must separately change. Unchanged callers, raw purpose rows, filenames, source anchors, repair families, open issues and existing tests are not migration counts. S/M/L/XL describes relative scope and coupling, not days.

Read [the continuation](continue-prompt.md), [plan](plan.md), [issue catalogue](issues.md) and [portable handoff](handoff/README.md).

- [Naming contract](../../design/contracts/name-handling.md)
- [Implementer guide](../../design/compiler/name-resolution-implementer-guide.md)
- [Shared-utility contracts](../../design/contracts/shared-utility-contracts-rust.md)
- [Consumer matrix](../../design/contracts/naming-consumer-matrix.md)
- [Semantic query contracts](../../design/contracts/resolved-semantic-queries.md)
- [Name-resolution proof index](../../design/analysis/name-resolution-proofs/README.md)

Before the final PR, move useful contracts and real experiments/tests into durable locations. Correct documentation/comments across the entire tracked repository, including generated guidance and files outside the implementation diff. Remove obsolete guidance, completed TODOs, development narratives, temporary ledgers/prompts/archives and their references, including this directory and the work-specific AGENTS exception. Preserve reproducible evidence and legal attribution. Complete all gates before creating a ready-to-merge PR; do not merge. This final cleanup is still required and is not completed by source-audit closure.

Root admitted the reviewed V6 successor across 78 changed paths (205 cumulative admitted paths). The initial actual five-library build failed64.43s on three missing enum documentation comments, with39,100 inputs unchanged. The exact comment-only repair is applied after independent source review and the retry build failed190.392s on three unqualified types in a new server test fixture;39,100 inputs unchanged. Narrow qualification repair is prepared outside main for independent review; no tests ran. Actual editor checks passed TypeScript/configured formatting/worker/Node+browserbundles/Studio web typecheck,443inputs unchanged; IDEhost execution remains pending. Prospective standard selection1024 preserves the exact994 prefix and adds30 controls; exact extra18 and seven external new Rust controls remain separate obligations. This supersedes prior queued/unadmitted next-batch notes; workspace/refactor/633 packets remain outside main and uncounted.

The exact three-type test qualification repair is now applied after independent review; actual retry2 build is running with39,100 source inputs frozen. Both prior build failures remain explicit.
