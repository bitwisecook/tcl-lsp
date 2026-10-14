# Current consumer implementation status

This source audit reconciles all 36 inherited actionable groups and the newly traced production families. Implementation state is independent of execution acceptance; no test or oracle acceptance is issued by this report. The JSON report records exact sampled source files and unchanged required controls. Whole-codebase closure still requires the listed producer/caller audits.

| Consumer family | Current source state | Remaining completion |
| --- | --- | --- |
| Compiler and codegen procedure bodies | ProcedureId carries actual bodies through retained Module/CU, interprocedural, WASM index/arity/direct and Native entry maps | Actual current controls and failure repairs |
| DB analyses and summary consumers | Body issuer/capsule dependencies and the fifth CFG owner are retained | Actual memo reuse/collision/currentness tests |
| Explorer, Core graphs and Diagram | IDs retain distinct same-label bodies; Diagram source selection refinement is active | Finish source_graph owner refinement and execute graph/tooling controls |
| Core variable surfaces | Shared LogicalVariableScopeAdvice and shared partial array grammar are integrated | Current failing scope controls must pass |
| F5 configuration consumers | Shared explicit object-path policies are integrated | Local policy controls plus unanswered BIG-IP handoffs; no appliance acceptance |
| Trace/deferred/optimiser variable protection | Shared purpose-specific textual MAY owner is integrated | Actual tests and Native observer controls |
| Declared frame-effect metadata | Typed selected descriptor factory/query and collision withdrawal are in the reviewed scratch patch | Integrate and execute fixed metadata controls |
| Runtime/VM/Engine | Shared physical namespace/slot/input owners and counted inventories are present | Owner-specific current publication/import/rename/replay tests; prior scoped runs do not certify changed closures |
| Debugger, CLI, MCP, Studio | Exact captured labels, shared source/registry inventory and contextual metadata adapters | Run current controls; retain separate reporting/source/native boundaries |
| Compiler metadata classifiers | Producer-traced frameless substitutions, slot eligibility, authored path constants and BoundsMetadataContext migrations are reviewed scratch patches; self-dispatch withdrawal still has mixed source-table naming policy | Integrate and test exact/root/colon/unknown controls; finish self-dispatch table producer/caller audit |
| AI and editors | Inspected scaffold/JSON/DOM seams do not independently select Tcl targets | Complete whole generated prompt/template/current-comment audit and editor validation |

No percentage is reported: migrated production families, remaining audited consumers, unknown callers, failing checks and unmeasured providers are separate obligations.
