# Remaining naming implementation obligations

Implementation has resumed from `554085e`. Twenty-four catalogue units have
reviewed changes pending complete validation: 13 code units and 11 authored-doc
units; **zero units are verified complete**. All **2,158** units remain
unverified or unimplemented (2,134 have no integrated implementation yet).
Development checkpoint `1b9bc6ea1` is pushed to `work`. Its fresh broad build
failed one SourceLoad field access; the repair now checks the actual scope
ancestry before issuing current-interpreter source advice. A new build is
pending. Two array-snapshot regressions pass across all five pinned C releases,
and the original interpreter/library/Tcltest pairing control passes. Logs are
`/tmp/naming-syntax-snapshot-1b9bc6ea1.log`,
`/tmp/naming-provider-pairing.log` and `/tmp/naming-build-1b9bc6ea1.log`.
The historical M12 results below are unchanged. Chat reports use 30-minute
intervals, with implementation and acceptance counted separately.

The source/role audit is complete for **110,709 tracked baseline paths**, with explicit current-source annexes and zero unexamined paths. The reconciled catalogue contains **1,321 caller migrations**, **377 shared-owner repairs/extensions** and **101 metadata/target-schema changes**: **1,799 semantic code changes**. There are separately **305 validation-code repairs** and **54 authored-documentation corrections**, making **2,158 confirmed required changes**. These remain to be implemented. Examined requirements and original/provider provenance questions remain separate.

[Searchable report](naming-audit.html) · [Full JSON catalogue](repository-consumer-audit-final.json) · [Current validation](current-state.md) · [Implementation obligations](implementation-ledger.md)

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

1. Compiler/shared producers: complete genuine Logical source scope/occurrence/declaration/frame transport; readonly Authored coverage must use genuine Installed BodyId/FunctionUnit/prepared-CFG/source owners without applying declaration-only APIs to Installed-only inventories. Finish live alias-cell source-definition inventory and recursive parameter naming geometry.
2. Database: complete retained Logical occurrence catalogue and actual graph-selected occupancy. Direct diagnostics must consume genuine occupancy, preserve stronger builtin/local/alias occupants and unknown-effect barriers, and never derive absence from unresolved/W123 ledgers or missing representation.
3. LSP/Core/iRules/MCP: use the shared configured scalar constructor and full following-fragment proof for minifier literals. Finish typed navigation/references/aliases/indexing/rename, reachable-region/tombstone/literal candidates and empty MCP pool candidates through genuine shared bindings.
4. Runtime: original-selector namespace helper dispatch, Import consumers and live lifecycle. Preserve argv identity, getter/publication order, finite alias/cycle/cache/lifetime guards. Parent/Text are integrated but Runtime controls still need execution.
5. VM/codegen/hosts: namespace retirement and no-argument Parent current-holder transport. Preserve actual namespace incarnation, deletion order and strict currentness across every registered backend/host.
6. Tooling/generated/guidance: implement the exact remaining boundary units, genuine shared target/source facets and actual adapters. Regenerate projections from their owner inputs, preserving source/provider purpose and accepted output domains.

Keep written source, evaluated names, namespace identity, command slots, live command identity, variable places/cells and display text distinct. Shared owners retain original bytes, spans, complete configuration, source ownership, namespace/frame/interpreter context and lifetimes/generations. A display string cannot reconstruct identity. Source advice grants no Native execution, physical cell, entered frame, absence, edit/optimisation authority or normal completion. Repair missing owner transport instead of weakening guards or adding String inverses.

Use the full JSON for exact IDs, source locations, APIs, required changes, dependencies and S/M/L/XL scope. Existing validation obligations and examined provider/domain requirements are retained separately; none is silently accepted or counted as another implementation change.

Latest executed ordinary M12 is frozen to `c9e826a23972590699a87d248c43a957858831ee23`: **1,546 controls; 1,133 PASS, 410 FAIL, one TIMEOUT, two NOT_ACCEPTED_RUN_COUNT**. All 1,491 predecessors remain, with four gains, zero prior-PASS regressions and zero drops. The 55 additions are 53 PASS and two FAIL. Seven original EPERM aborts and their exact successful permission repairs remain separate. Two nested Host raw-exit-zero runs remain rejected by the unchanged single-run guard. Its fresh single-job build produced 23 test binaries with zero compiler errors in 678.804699595 seconds. No newer execution follows from this audit.

Runtime174, VM133, Formatter88 and integration55/15 remain **UNEXECUTED**. The full proof checker on frozen 6294 failed with 2,128 structural errors; no current green proof gate exists. Required current owner/proof/generated checks, `make rust-check` and `make prep-pr` remain red or unrun. No PR, issue closure or merge acceptance is supplied.
