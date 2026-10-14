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
| Compiler/CFG/deferred | Issued ProcedureId/body storage, original declaration summaries, separate Logical root/current/unknown-caller owners, retained values, original word vectors and source-only empty-name projections. | Sealed Compiler92 executes147 controls:123 pass/24 nonpassing, source subset drift0. Its original controls remain required after current producer changes. Finish protected-body and source summary/SSA consumers, preserving original assertions. |
| Lowering/diagnostics/codegen | Actual descriptor and registration requirements, retained slot readers, W116/W143 shared advice, original-cell W128 and original formal spans. | Current affected controls and remaining target audits; original dynamic-name and captured-value regressions remain obligations. |
| LSP/source/graphs | Actual body IDs, original member/source providers, linked delegate guards, sequential frame effects and genuine Logical namespace seeds. Sealed Core95 executes92 controls:66 pass/26 fail, source subset drift0. All71 original Core84 controls are retained with two gains and zero regressions; current CodeLens/completion fixes await replay. | Finish source definition/type producers, linked hover/rename joins and genuine declaration insertion readers; replay affected original and new controls. The retained deep graph control passes. |
| DB/CommonAOT | Actual Module/body/callsite owners and source dependency capsules; display-body lookup retained only in fixtures. DB61:10 original controls,0 pass/10 fail, source subset drift0. | Fix genuine cold/edit reuse and missing source metadata producers; retain all original memo-count and uncached-equivalence assertions. |
| Runtime/VM/Engine | Shared counted/C-string protocols, retained slots/generations, authored support registration, measured rename/visibility diagnostics and actual publication slots. Sealed Runtime99:99 controls,95 pass/4 fail, full captured source/provider/compiled-input/artifact drift0. All four new Jim key/error-frame controls pass; retained bootstrap/key/error/import controls remain red. | Integrate reviewed genuine Jim registration publication and VM original object/key consumers; replay retained controls and original #2286 source-scope closure. |
| Middleware/tooling/AI/editors | Shared symbol providers, URI/currentness guards and owned debugger cells. Selected-dialect MCP variable readers and AI regex retirement are integrated from actual source owners and C5/Jim observations. Actual VSCode/Xvfb provider tests execute. | Replay integrated MCP and AI consumers; Actual native server and fresh extension execute11 exact real VSCode controls:11 pass, source/out/binary drift0. Complete remaining surface audits. |
| Issues/F5 | Refreshed open/closed issue map and retained original naming observations. | Execute all wholly naming-related regressions. Obtain real BIG-IP answers for unanswered questions; C/Jim observations do not establish F5 behaviour. |

Compiler/DB/Core scoped captures do not establish complete provider closure or Native acceptance. Original failures and unchanged controls remain in retained experiments.

## Active parallel work

Root owns integration, Git and the single heavy Cargo lease. Compiler finishes original body/frame producers. DB repairs reuse and LSP joins. Values finishes source variable/escape consumers. Audit implements VM original key/publication consumers and reconciles outstanding boundaries. The documentation agent implements Runtime Jim registration producers; Root maintains tracking. Frames migrates LSP declaration/insertion consumers and independently reviews owner boundaries. Rust/runtime/data remain owned by Root during current capture scheduling; source patches stay in scratch until the capture is sealed.

## Cadence

Tracking checkpoint15:43 BST on14 October2026; next15:58 BST. Hourly report16:13 BST. Development commits/pushes are authorised while red. All required gates must pass before PR. No PR is ready.

## Oracles and F5

Original source-built C Tcl8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 with matching libraries are provisioned. Latest checked Jim commit is5bac7c99ad65864c87da513e22e2f01703fa4e03, version0.84-9-g5bac7c9; rediscover latest before new observations. If Tcltest is used, use that interpreter version's original init.tcl/tcltest.tcl. Preserve original source, licensing, raw output and reproducible provider provenance.

Use [existing F5 answers](../../../experiments/name-handling/f5/answers/README.md). The400-row snapshot has93 observed/244 unreached/63 unavailable; the separate pattern matrix has108 observed/25 unsupported/267 unreached across50 questions/eight contexts. Unanswered questions require actual BIG-IP observations returned as documents on `work`. Preserve the [security scrub](../../../experiments/name-handling/results/security-material-scrub/README.md).

## Merge readiness

Finish every consumer and its meaningful oracle-based controls. The current strict proof audit reports1959 structural errors; semantically correct records and narrowly reseal only genuinely executed closures. Review all authored/generated documentation and comments for current how/why, minimal comments and clear code. Remove process/history narratives, stale claims and completed TODOs; keep experiments and legal attribution. Fix link/index drift. Remove this temporary directory/references and the work-specific AGENTS push exception. Regenerate/format and pass required rust-check/prep-pr gates before creating a merge-ready PR. No merge is authorised.

Current checkpoint15:43 BST: sealed Compiler92 remains123/147 and Core95 remains66/92 with two gains and zero regressions among71 original Core controls. Runtime99 remains95/99 with zero captured source/provider/compiled-input/artifact drift; its four retained failures have active producer fixes. Reviewed VM original key/publication migration is ready for integration, with explicit Published(empty)/Unavailable handling and actual local key lifetime transfer. Runtime Jim repin captures original registration bytes and current slots; bounded receipt retirement is under review. LSP Logical procedure insertion uses genuine original header cards and shared quoting; the per-analysis inventory optimisation is pending. Full-effect source write and taint lexical consumers remain in progress.

Compiler101 linked no tests because the linker failed. After obsolete registry-cache variants were hashed and retired, Compiler102 built successfully: two TclOO controls passed, then the new isolated global read equality assertion failed and its full AnalysisResult output exhausted /tmp. The exact3.4GB output is compressed and hashed; the incomplete capture is not acceptance. A same-binary expected-source replay now targets10 original SSA/certificate/Authored controls with temporary immutable test-only stage output. Tracing files remain excluded from commits and must be removed before ordinary replay.

The historical92 #2286 phase probe completed two analysis/CFG cycles, then timed out30 seconds during another deferred source preparation. Exact executable/source/probe receipts are retained; this diagnosis does not close the original600-second timeout or establish an infinite loop. Namespace import and the original fixed-depth failures remain required. Next tracking15:58 BST; hourly16:13 BST. No PR is ready.
