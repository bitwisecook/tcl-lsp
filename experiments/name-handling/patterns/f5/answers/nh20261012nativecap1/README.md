# Native `call` boundary in non-TMM interpreters

The native `call` command is absent in each of the five tested interpreter contexts. Each job first records its own version, namespace and command surface, then executes all five native-question dispatch forms. All five forms fail with completion 1, `invalid command name "call"`; none selects or invokes the named rule provider.

| Context | Own Tcl patchlevel | Execution namespace | `info commands call` / `info procs call` / `namespace which -command call` | Five actual dispatch forms |
| --- | --- | --- | --- | --- |
| tmsh CLI script | 8.4.6 | `::script` | All empty | All code 1, missing `call` |
| iApp implementation | 8.4.6 | `::` | All empty | All code 1, missing `call` |
| iCall | 8.4.6 | `::` | All empty | All code 1, missing `call` |
| Appliance stock Tcl 8.4 | 8.4.13 | `::` | All empty | All code 1, missing `call` |
| Appliance stock Tcl 8.5 | 8.5.13 | `::` | All empty | All code 1, missing `call` |

The forms cover the rule path, a computed target, a list prefix with arguments, and the entry calls for lifetime and storage questions. Missing `call` prevents the native entry operation in these contexts; it does not supply hypothetical provider lifetime, replacement, detachment or storage results. These are independent capability controls, not original portable inputs and not measurements of TMM dispatch.

The derived [reviewed-capability-prerequisites.json](reviewed-capability-prerequisites.json) records each same-job surface control's result, raw evidence, input identity and context. [records.json](records.json) derives its patchlevel and namespace exclusively from that job's surface record. Its 25 native-form records are narrowly `unsupported` because the required command is actually absent and the attempted operation fails at it; the five successful surface controls remain `observed`. This classification is a reviewed derivation from the retained receipts, not the report generator's general error heuristic.

All 30 setups complete with code 0 and all capture fields are complete. [review.json](review.json) records raw row and hash checks plus exact setup/operation materialisation. [reviewed-proofs.json](reviewed-proofs.json) retains 30 scoped control proofs, with authoring input explicitly identified as case metadata. No matrix or central proof authority is changed by this file.

The three hosted configurations loaded and were removed, with post-delete absence checks. Stock invocations have no configuration load or object deletion requirement. These controls create only wrapper frame variables; no independent total interpreter-state cleanup inventory exists. Remote staging awaits a separate cleanup receipt.

Retained sanitised transport/deployment projection: documentation address, NHO-only logs, and omitted hardware/unrelated inventory. Tcl operation and result bytes are preserved. Historical executed-input hashes describe original execution, not these projected deployment fixtures.
