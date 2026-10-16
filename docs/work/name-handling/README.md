# Centralised name handling on work

Active priority is **all LSP consumers and their required shared Compiler/DB/Registry owners**. Runtime/VM and BIG-IP work are deferred at their recorded checkpoints. For uncovered BIG-IP behaviour, Tcl8.4 is a working assumption; measured evidence keeps its original scope and provenance. The LSP baseline contains **325 obligations** (201 semantic,122 validation,2 documentation): **83 have integrated changes** (14 semantic,67 validation,2 documentation),242 have none and zero are verified complete. Shared dependencies keep their original counts and are not recounted here.

Implementation has resumed from `554085e`. **211 baseline units** have reviewed
implementation pending validation: 157 code units and all 54 authored-documentation
units. These include one explicitly partial caller (DB class cards).
**Zero units are verified complete**. Baseline counts remain **2,158**;
1,947 baseline units have no integrated implementation. The bounded
[owner/caller annex](implementation-owner-annex.json) adds **35 demonstrated
requirements**, 34 integrated pending validation and one blocked before integration.
Effective scope is **2,193 obligations**, with **245 integrated pending validation**
and **1,948 not integrated**. These counts establish neither semantic acceptance
nor completed gates.

Development checkpoint `5b4d97abe` is pushed to `work`. The
[current implementation and validation snapshot](current-state.md) records exact
ordinary builds, selected failures, source-only limitations, restored evidence
and pending gates. The [implementation ledger](implementation-ledger.md) retains
remaining boundaries and the [owner annex](implementation-owner-annex.json)
separates additional requirements from baseline units. No required repository
gate or whole-issue acceptance is green. Progress reports use 30-minute intervals.

The source/role audit is complete for **110,709 tracked baseline paths**, with explicit current-source annexes and zero unexamined paths. The reconciled catalogue contains **1,321 caller migrations**, **377 shared-owner repairs/extensions** and **101 metadata/target-schema changes**: **1,799 semantic code changes**. There are separately **305 validation-code repairs** and **54 authored-documentation corrections**, making **2,158 confirmed required changes**. These remain to be implemented. Examined requirements and original/provider provenance questions remain separate.

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
