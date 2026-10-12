# Parent and fresh-child capability diagnostics

These are six independent controls in six execution contexts, measured on BIG-IP 21.1.0.1. They produced 162 complete setup/lookup records: six each in tmsh CLI, iApp implementation and iCall; 24 each in RULE_INIT and a native procedure called from RULE_INIT; and 96 in HTTP_REQUEST. RULE_INIT and procedure measurements reached all four units, `0:0` through `0:3`. Six cases ran for each of 16 traffic invocations, all on actual unit `0:0`.

All 12 configuration loads succeeded. Every diagnostic setup completed with code 0; every lookup has complete result chunks. “Observed” includes the measured errors below. The original portable programs and selected Wiki test bodies were not executed by these controls.

| Context | Parent `package require tcltest` | Fresh-child `package require tcltest` | Parent `::forward` | Frame variables `ns`, `target`, `description`, `prefix` |
| --- | --- | --- | --- | --- |
| tmsh CLI script | Code 1: missing `debug` variable | Code 0: `2.2.8 ::tcltest::test {}` | Command/procedure absent | All absent |
| iApp implementation | Code 1: missing `debug` variable | Code 0: `2.2.8 ::tcltest::test {}` | Command/procedure absent | All absent |
| iCall | Code 1: missing `debug` variable | Code 0: `2.2.8 ::tcltest::test {}` | Command/procedure absent | All absent |
| RULE_INIT, all four units | Code 1: package not found | Code 1: package not found | Command exists; procedure absent; `info body` code 1 | All present, no reported traces |
| Native procedure, RULE_INIT, all four units | Code 1: package not found | Code 1: package not found | Command exists; procedure absent; `info body` code 1 | All absent |
| HTTP_REQUEST, 16 invocations on `0:0` | Code 1: package not found | Code 1: package not found | Command exists; procedure absent; `info body` code 1 | All absent |

The hosted parent errors include the real package's `DebugPuts` / `ConstraintInitializer` stack. A new child provides a different interpreter environment: its successful package load does not repair the native parent and does not prove that a selected test body or its real harness will execute there. The child exposes the qualified `::tcltest::test`; the unqualified global `::test` query is empty. TMM's fresh child still cannot discover this package. Package discovery failure alone does not establish failure of every possible explicit source operation.

Each context's own version control reports Tcl 8.4.6. The CLI parent reports namespace `::script`; other parents and all fresh children report `::`. Fresh-child namespace/procedure controls place `tail` in the requested unique namespace and successfully call it there. The separate parent control explicitly defines a unique global procedure, renames it into the unique namespace, then calls it successfully; it does not test an unqualified parent definition. No child result is assigned to the native parent context.

The inventory distinguishes the native `::forward` command from a Tcl procedure. It does not call, rename or remove `::forward`. The four RULE_INIT frame variables are existing visible variables, with ownership unknown; this run does not remove or redefine them. Existence in that frame is narrower than an assertion of global storage or cross-event persistence.

The retained real package source is 98,763 bytes, SHA-256 `2ef6cc2a65738bf7cf8b9c247db7a3f1548a498adfc669f05e71c12cbad921a0`. The controls use `package require`; they do not substitute a rewritten test harness. The retained source provides package provenance, while each result is anchored to its actual configured operation and raw capture.

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain all identities and exact result encodings. Readable TMM values above use reversible ASCII byte projection only; their original encoding remains `binary-scan-tcl-value`. [review.json](review.json) records validation of 702 raw rows, 10,926 descriptor occurrences and 369 distinct evidence files. [reviewed-proofs.json](reviewed-proofs.json) contains 36 narrowly scoped control representatives, one per case/context; their authoring input is the retained case metadata, not a portable original.

All authored child/procedure cleanup operations returned code 0, and post-delete configuration checks show the task objects absent. There is no separate interpreter-child inventory or namespace-after inventory, so cleanup completion is not expanded into total interpreter-state absence. Remote staging remains until a separate cleanup receipt removes the sealed captures.

Retained sanitised transport/deployment projection: documentation address, NHO-only logs, and omitted hardware/unrelated inventory. Tcl operation and result bytes are preserved. Historical executed-input hashes describe original execution, not these projected deployment fixtures.
