# Current name-handling state

Temporary coordination snapshot on `work`; remove this directory and its references before the PR. Git records implementation history. Keep original experiments, failures and provenance.

## Goal

Centralise name lookup in reusable owners and move every consumer to them across Syntax, Registry, compiler analyses and all codegen targets, LSP/database, middleware, runtimes, tooling, AI and editors. Fix #2286 and other issues entirely caused by naming. Consumers require the original representation, actual context and current owner. Source advice, descriptor metadata, live bindings and editing authority have separate purposes.

See the [owner contract](../../design/contracts/name-handling.md), [consumer integrations](../../design/analysis/name-resolution-proofs/consumer-integrations.md), [issue map](issues.md), [consumer inventory](consumer-inventory.md) and [finishing plan](plan.md).

## Current consumer status

Migration is incomplete. Integrated source, scoped execution and final acceptance are separate; there is no verified completion percentage.

| Consumer group | Integrated architecture | Remaining implementation and verification |
| --- | --- | --- |
| Syntax/Registry | Shared geometry, original contexts, purpose-specific projections, descriptor identities and explicit Logical frame-alias metadata. | Audit remaining adapters; execute original and new refusal controls against current source. Register private owners through bounded source gates. |
| Compiler/CFG/deferred | Issued ProcedureId/body storage, original declaration summaries, separate Logical root/current/unknown-caller owners, retained values, original word vectors and source-only empty-name projections. | Compiler88 executes113 controls:96 pass/17 nonpassing, source subset drift0; broader67 retains599 controls,442 pass/157 fail. Finish protected-body and source summary/SSA consumers, preserving original assertions. |
| Lowering/diagnostics/codegen | Actual descriptor and registration requirements, retained slot readers, W116/W143 shared advice, original-cell W128 and original formal spans. | Current affected controls and remaining target audits; original dynamic-name and captured-value regressions remain obligations. |
| LSP/source/graphs | Actual body IDs, original member/source providers, linked delegate guards, sequential frame effects and genuine Logical namespace seeds. Latest semantic Core84:71 controls,48 pass/23 fail, source subset drift0; all original68 plus three new editor controls retained. Current post84 fixes need affected replay. | Repair graph analysis stack overflow, missing source definition/type producers, linked hover/rename joins and workspace coordinates; replay new seed/provider controls. |
| DB/CommonAOT | Actual Module/body/callsite owners and source dependency capsules; display-body lookup retained only in fixtures. DB61:10 original controls,0 pass/10 fail, source subset drift0. | Fix genuine cold/edit reuse and missing source metadata producers; retain all original memo-count and uncached-equivalence assertions. |
| Runtime/VM/Engine | Shared counted/C-string protocols, retained slots/generations, authored support registration, measured rename/visibility diagnostics and actual publication slots. Runtime09:79 controls,78 pass/1 fail; all original77 pass, full captured closure drift0. | Replay integrated C8.5 error-code correction and visibility controls; execute current VM, Engine and original #2286 source-scope closure. |
| Middleware/tooling/AI/editors | Shared symbol providers, URI/currentness guards and owned debugger cells. Selected-dialect MCP variable readers and AI regex retirement are integrated from actual source owners and C5/Jim observations. Actual VSCode/Xvfb provider tests execute. | Replay integrated MCP and AI consumers; Actual native server and fresh extension execute11 exact real VSCode controls:11 pass, source/out/binary drift0. Complete remaining surface audits. |
| Issues/F5 | Refreshed open/closed issue map and retained original naming observations. | Execute all wholly naming-related regressions. Obtain real BIG-IP answers for unanswered questions; C/Jim observations do not establish F5 behaviour. |

Compiler/DB/Core scoped captures do not establish complete provider closure or Native acceptance. Original failures and unchanged controls remain in retained experiments.

## Active parallel work

Root owns integration, Git and the single heavy Cargo lease. Compiler finishes original body/frame producers. DB repairs reuse and LSP joins. Values finishes source variable/escape consumers. Audit reconciles remaining consumers and taint report boundaries. Docs maintains tracking and reviews current contracts. Frames independently reviews consumer and stack transport patches. Rust/runtime/data remain owned by Root during current capture scheduling; source patches stay in scratch until the capture is sealed.

## Cadence

Tracking checkpoint14:28 BST on14 October2026; next14:43 BST. Hourly report15:13 BST. Development commits/pushes are authorised while red. All required gates must pass before PR. No PR is ready.

## Oracles and F5

Original source-built C Tcl8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 with matching libraries are provisioned. Latest checked Jim commit is5bac7c99ad65864c87da513e22e2f01703fa4e03, version0.84-9-g5bac7c9; rediscover latest before new observations. If Tcltest is used, use that interpreter version's original init.tcl/tcltest.tcl. Preserve original source, licensing, raw output and reproducible provider provenance.

Use [existing F5 answers](../../../experiments/name-handling/f5/answers/README.md). The400-row snapshot has93 observed/244 unreached/63 unavailable; the separate pattern matrix has108 observed/25 unsupported/267 unreached across50 questions/eight contexts. Unanswered questions require actual BIG-IP observations returned as documents on `work`. Preserve the [security scrub](../../../experiments/name-handling/results/security-material-scrub/README.md).

## Merge readiness

Finish every consumer and its meaningful oracle-based controls. The current strict proof audit reports1959 structural errors; semantically correct records and narrowly reseal only genuinely executed closures. Review all authored/generated documentation and comments for current how/why, minimal comments and clear code. Remove process/history narratives, stale claims and completed TODOs; keep experiments and legal attribution. Fix link/index drift. Remove this temporary directory/references and the work-specific AGENTS push exception. Regenerate/format and pass required rust-check/prep-pr gates before creating a merge-ready PR. No merge is authorised.

Current checkpoint14:28 BST: fresh clean Compiler92 build passes and the actual consumer replay is active. First28 controls complete:20 pass/8 fail; capture not sealed and no full source closure acceptance yet. All four source syntax-cache controls pass, including cold3/edit1/shift0 reuse, current receipts, exact same-owner cached/fresh body equality and expansion refusal. Shared SCCP inventory and both instance-variable candidate controls pass; Native declaration-child admission now passes genuine foreign-attachment refusal. Original40 alias setup/body/cleanup controls pass; namespace-scoped Tcltest import still fails. The exact #2286 program is running with a600-second harness timeout, unchanged assertions and internal budgets. SSA definitions/reads remain unavailable, the public lexical-vector foreign-owner negative fails, and Authored inner Catch/CFG scalar/dictionary geometry controls need producer or original-input repairs. Parallel scratch work covers actual attachment ownership, Logical LSP member joins, current-source outline coordinates, dynamic alias/shared parsing, remaining Call/unset readers, inputless Authored definitions and VM Jim original-key publication. Existing integrated consumer contracts, original observations, failed build receipts and clean recovery provenance remain retained. Runtime99 and separate API4 plans are ready; no execution claim. Next tracking14:43 BST; hourly15:13 BST. No PR is ready.
