# Centralised name handling on work

Implementation has resumed from `554085e`. Seventy-nine catalogue units have
reviewed changes pending complete validation: 30 code units and 49 authored-doc
units; **zero units are verified complete**. All **2,158** baseline units remain
unverified or unimplemented (2,079 have no integrated implementation yet).
Development checkpoint `2459c9924` is pushed to `work`. Its clean-source Runtime
build passes. Fresh selected continuations (7), original Import fixtures (7),
original/public Jim Import controls (13), and registered legacy Length control
(1) all pass; earlier failures remain retained. Three Info-helper controls passed
at the prior checkpoint. Compiler Logical variables were 4 PASS/2 FAIL, with
repaired new fixture premises awaiting retest. The W125 test qualification is
repaired; complete workspace test compilation remains pending. Newly integrated
Logical point/namespace source transport, procedure byte queries, report parts
and VM ensemble List/Dictionary caller checks await compilation and controls.
The separate Jim physical info-body result-header/getter boundary remains open.
Exact selected logs/source hashes are in the compact durable
[control capture](../../../experiments/name-handling/results/owner-adoption-20261016/records.json).
The [provider capture](../../../experiments/name-handling/oracles/provider-pairing-20261016/capture.json)
authenticates all six installed C/Jim setup pairings, without semantic acceptance.
Alias-action drafts remain withheld because the source-action owner is missing.
Additional owner-facet and baseline test-repair count reconciliation stays open;
no baseline completion is inferred from it. Historical M12 remains unchanged.
Chat reports use 30-minute intervals; implementation and acceptance stay separate.

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
