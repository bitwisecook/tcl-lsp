# Centralised name handling on work

Implementation has resumed from `554085e`. **98 baseline units** have reviewed
implementation pending validation: 44 code units and all 54 authored-documentation
units. **Zero units are verified complete**. Baseline counts remain **2,158**;
2,060 baseline units have no integrated implementation. The bounded
[owner/caller annex](implementation-owner-annex.json) adds **25 demonstrated
requirements**, of which 24 have integrated implementation pending validation.
Effective scope is **2,183 obligations**, with **122 integrated pending validation**
and **2,061 not integrated**. These counts establish neither semantic acceptance
nor completed gates.

Development checkpoint `80541dfbc` is pushed to `work`. Its fresh ordinary
six-package workspace build passed in 9m18s. Selected controls are **164 PASS /
79 FAIL**, with one zero-test invocation explicitly rejected. Core minifier
controls are 92 PASS/50 FAIL and VM namespace controls 44 PASS/12 FAIL; all six
actual VM report-parts controls and the import cleanup control pass. Exact logs,
binary hashes and strict original-provider invocation scope are retained.
The ordinary Runtime build at `35c8ce634` passed; selected controls were
294 PASS/5 FAIL. In the two broad comparison groups all 197 prior passes were
retained, with eight gains and zero regressions. Reviewed repairs for the five
remaining failures are integrated. A fresh ordinary successor build passed in
57.46s with zero warnings; its selected controls finished at 298 PASS/1 FAIL. The remaining local-cache
fixture entered a fresh Native read after refusal; its reviewed correction
inspects the original physical compiled slot and awaits a fresh build. The workspace
successor encountered 19 missing API docs and one test borrow error, now repaired;
its seven-package rebuild compiled production libraries but found four
new test-only scope errors in the instance-report control; moving that control
to its existing helper module repairs the scope. Another build is pending. Genuine Logical receiver/navigation,
parameter-trait source transport, pure expression-gap action and mode-preserving
instance reports are integrated pending validation. Native variable expressions,
foreign constructor collections and VM physical body transport remain open.

The full proof checker at `2a86f7dfd` reported 3,574 errors. A reviewed repair
addresses its 17 source/binding/document consistency messages and passes bounded
108-owner/85-binding checks without changing evidence, result or status digests.
The current full rerun reports 3,403 errors. Thirteen small historical files (3,520,174 bytes)
were restored with Git blob and all 164 retained proof hashes verified. This is
availability restoration only; 638 native binary references across 470 paths
remain missing. Census/evidence drift and required acceptance remain open.

Exact selected ordinary results remain in the compact durable
[control capture](../../../experiments/name-handling/results/owner-adoption-20261016/records.json).
The [provider capture](../../../experiments/name-handling/oracles/provider-pairing-20261016/capture.json)
authenticates six installed original setups, without semantic acceptance.
Reviewed physical Jim body-result ownership, report serialization and compiled-stack
purpose repairs are integrated; Rust execution remains pending. Alias edit
authority, foreign class collections and separate VM physical-body adoption remain
open. Historical M12 is unchanged. Progress reports use 30-minute intervals.

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
