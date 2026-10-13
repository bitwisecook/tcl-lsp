# Current name-handling state

Temporary coordination snapshot on `work`; remove this directory and its references before the PR. Git records implementation history; experiment receipts retain observations, failures and exact source/artifact provenance.

## Goal

Centralise name lookup in reusable owners and move every consumer to them. Fix #2286 and every issue entirely caused by naming: original representation, context, lookup order, binding role, namespace identity, lifetime or consumer disagreement. Include Syntax, Registry, compiler analyses, LSP/database, middleware, runtimes, every codegen target, tooling, AI and editor integrations.

Use the [owner contract](../../design/contracts/name-handling.md), [consumer integrations](../../design/analysis/name-resolution-proofs/consumer-integrations.md), [issue map](issues.md), [consumer inventory](consumer-inventory.md), [finishing plan](plan.md) and [procedure identity plan](structured-procedure-plan.md). Source advice, metadata, live binding and edit authority remain distinct. Preserve original input, selected protocol, actual context and generation, source image/configuration and current store; refuse absent, foreign, stale or dynamic evidence.

## Consumer status

Migration remains incomplete. These counts describe captured controls, never a percentage of consumers completed. Integrated source, scoped validation and whole-workspace acceptance are distinct.

| Consumer group | Integrated or validated state | Remaining work |
| --- | --- | --- |
| Shared naming | Main Syntax139 naming controls pass against stable harness-selected inputs. Original source mapping25 Syntax/6 Registry controls and C5/Jim LF/CRLF observations pass. Shared geometry serves Syntax/source/Compiler/API/Runtime/VM. | Close every actual adapter; owner tests do not certify consumer use or the complete compiled closure. |
| Registry/VM | Typed descriptor, role/layout and provider query owners are integrated.49 intended controls pass across two sealed captures. VM13 namespace/query controls and150 original C5/Jim Which observations pass. Distinct Jim origin/root adapters are integrated with42 original rows per provider retained. | Corrected alias/origin17 executes10pass/6fail/1test-build block. Prepared21-selector producer correction awaits its build lease. Preserve actual generation and pending-rename transaction owners; diagnose modern C preflight refusal. |
| Source reads/#2286 | Shared SourceReadScope/CompletePrefix producer is integrated. Frozen06 executes38:36pass,1missing `Unit::f` assertion failure,1original-program timeout. An unchanged same-binary replay passes the original #2286 C5 assertions in70.54s. Full6728-source/binary/dependency/oracle-library closure remains unchanged. All3 new source-purpose controls pass. A small typed declaration-schema enum follow-up is integrated after this seal and remains uncompiled; it preserves source-only reachability decisions. | Diagnose the separate earliest missing-body publication producer; rerun combined Main consumers. Retain the earlier timeout and measured timing uncertainty; source-only advice must not issue runtime applicability. |
| LSP/scope | Shared LogicalVariableScopeAdvice, genuine write cards, source-image guards and source publication selection are integrated. Callback Compiler4/Core12/DB8/server8 pass. Fresh isolated Scope15 executes5pass/10fail, including actual Native class publication positives. | Apply genuine Logical rename fixture ingress and validate nine producer-dependent public controls against the new SourceReadScope dependency. Finish audited constant-value/apply/dict/report consumers through purpose-specific scope/provenance owners. |
| Runtime | Ordinary variable16 controls pass. Main trace/rename use actual command generations and physical slots. Genuine replay10 controls pass. Corrected query31 executes29pass/2fail, both exposing C84 alias replacement after rename. | Runtime68 plus Engine7 are queued under the active Runtime build lease. Validate C84 actual registration-token retirement and retained imports, replay receivers, host/guest transport and remaining introspection consumers. |
| Compiler/body identity | Primary58-owner migration is integrated. Opaque ProcedureId retains repeated-display bodies across IR, CFG, compilation, original allocation joins, bytecode, WASM/native table plans and CommonAOT. Scoped Compiler lib strict06 check passes0errors/12warnings with sealed inputs/dependencies. | Finish typed interprocedural/call-site/frame-summary maps and exact DB/report/graph joins; unique-display interim adapters are not final semantic completion. Compiler tests, combined Main and genuine compiled-entry/table admission remain pending. |
| BPF | Shared exact descriptors and typed BPF operations integrated; frozen IR60/codegen28 controls pass. | Combined gates and applicable body-identity consumers. Other target capability declarations without emitters are separately attributed. |
| Extension inventory | Runtime/API/Engine use structured actual slot/generation inventory; checked root operands and whole-catalogue refusal replace report reconstruction. API cargo check passes. Genuine Runtime3/Engine3/real-C-side-module1 controls are integrated but unexecuted. | Run the7 controls against actual host/guest artifacts, preserve old-artifact unsupported inventory and ordinary evaluation behavior. C5 callback observations distinguish entered-context C registration from script publication. |
| CLI/Studio/tooling | SpecProbe/Studio use shared provider-family and original command inventory.24 original scoped groups pass, including132 presence rows and12 provider-specific package outcomes. C literals remain unresolved registration operands. SimulatorMockName, native SpecTcl hooks and variable snapshot support are integrated; snapshot16 source-form controls pass. | Run actual CLI/Studio selectors, simulator generation/drift and SpecTcl controls. Complete remaining AI/middleware/tool/editor handoff audit. |
| Issues | Open/closed scoped mapping in [issues](issues.md); #2286 remains open pending complete implementation/gates. | Resolve all wholly naming-related issues; distinguish non-naming validation dependencies. |
| PR gates | No full-workspace acceptance or PR. Previous strict checker has1739 errors, including evidence bindings and scrub-census currency. | Review semantics and narrowly renew affected proofs; finish owner-resolution/drift, formatting/codegen, rust-check/prep-pr and required editor SDK checks. Never bulk-refresh digests. |

## Active parallel work

Runtime owns the serial heavy-build lease after Compiler strict06 released it. Lookup implements strict analysis IDs; SourceReadFlow diagnoses the remaining actual body producer; ConsumerAudit implements audited constant-value consumers; RegistryNaming finishes outstanding semantic handoff audits; Reporting migrates graph/Explorer/Diagram consumers. Root owns Main, Git, DB/compiler report producers, integration and leases. Next build lanes are VM21 and fresh source/scope controls, then combined Main consumers. Separate targets and frozen source closures are mandatory.

The bounded audit covers63 package boundaries,330 attributed LSP candidates and36 semantic handoffs. Coverage is not acceptance. Original argv/body retention and Native compiler permission remain independent; no metadata or report inversion can substitute for a genuine selected allocation.

## Cadence and references

Tracking checkpoint:20:46 UTC on13 October2026; next21:01 UTC. Next hourly progress report21:06 UTC. Update tracking and commit/push coherent reviewed batches every15minutes, including honest red states. Current migration batches are formatted, with combined compilation/testing still pending.

Original source-built C8.4.20,8.5.19,8.6.18,9.0.4,9.1.0 and matching original libraries are provisioned. Latest checked Jim is5bac7c99ad65864c87da513e22e2f01703fa4e03, reporting0.84-9-g5bac7c9. Rediscover latest before new observations. If using Tcltest, use the version's original init.tcl/tcltest.tcl, including authentic zipfs channels where applicable. Source `/tmp/name-env.sh`; lease one distinct Cargo target, jobs1/test-debug0. Sealed experiments and hashes remain; executable build products stay outside Git.

## F5

Use [existing answers](../../../experiments/name-handling/f5/answers/README.md) and the outstanding-operation audit. The numbered400-row snapshot has93observed/244unreached/63unavailable; the separate pattern matrix has108observed/25unsupported/267unreached across50questions/eightcontexts. Supplements do not promote original unreached rows. Prepare only unanswered questions for the BIG-IP chat; actual answer documents must return on `work`. Never infer F5 observations from C/Jim or run Tcltest in TMM/MCPD. Suspended frames, restart/reload and traffic-unit contexts remain unmeasured.

Preserve the [security scrub](../../../experiments/name-handling/results/security-material-scrub/README.md). Do not restore deleted lab/device/security/certificate material; redacted hashes are not original execution seals.

## Merge readiness

After every consumer is finished, audit all authored/generated documentation and comments, including untouched code/tests/scripts/templates. Keep minimal current how/why, clear self-describing code, legal attribution, experiments and oracle-based tests. Remove stale claims, completed TODOs, abandoned plans, process/history narratives, this temporary directory/references and the work-specific AGENTS push exception. Format/regenerate and make all required gates green before creating a merge-ready PR. No merge is authorised.
