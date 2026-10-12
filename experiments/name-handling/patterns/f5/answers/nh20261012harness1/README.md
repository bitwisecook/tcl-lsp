# Explicit real-package source: incomplete capture

This TMM child-interpreter diagnostic assembled the unchanged 98,763-byte real Tcltest 2.2.8 package through nine configured module procedures. All three job configurations loaded successfully, and independent reassembly of every configured module group matches retained `real-tcltest-2.2.8.tcl`, SHA-256 `2ef6cc2a65738bf7cf8b9c247db7a3f1548a498adfc669f05e71c12cbad921a0`.

The setup ended with `append`, so its result was the entire package. Emitting that result caused missing chunks and lost later records. The report has 22 identifiable records: 17 labelled observed because a complete lookup result exists, and five with no captured lookup. Twenty-one captured fields are incomplete. This is an incomplete diagnostic capture, with no proof candidates in [reviewed-proofs.json](reviewed-proofs.json).

| Context | Identifiable records | Complete lookup results | Measured outcome |
| --- | --- | --- | --- |
| RULE_INIT | Four units `0:0`–`0:3` | Unit `0:2` only | Outer list completion 0; nested module evaluation 1 (`pwd` unavailable), nested package require 1 (package not found) |
| Native procedure called from RULE_INIT | Units `0:1`, `0:2` only | None | No lookup outcome captured; absence of other units is a capture gap |
| HTTP_REQUEST | 16 invocations, actual unit `0:0` | All 16 | Same outer completion 0 and nested failures as above |

Outer completion 0 means the diagnostic constructed its result list after catching errors. It does not mean the real harness loaded. Where captured, the child reports Tcl 8.4.6 in namespace `::`. No selected Wiki test body is observed, and the fresh-child result is not a native-parent result. The compact follow-up is retained separately as `nh20261012harness2`.

[review.json](review.json) records raw identity/hash checks, missing chunks and exact configured-source reassembly. [records.json](records.json) and [ANSWERS.md](ANSWERS.md) preserve the partial measurements without filling gaps. The table uses ASCII byte projection; original TMM result encoding remains `binary-scan-tcl-value`.

Child deletion returned code 0 for the one complete RULE_INIT lookup and all 16 traffic lookups. Cleanup was not captured for the other five identities; no child deletion is inferred there. Post-delete configuration checks show the configured task objects absent. Remote staging remains pending a separate cleanup receipt.

Retained sanitised transport/deployment projection: documentation address, NHO-only logs, and omitted hardware/unrelated inventory. Tcl operation and result bytes are preserved. Historical executed-input hashes describe original execution, not these projected deployment fixtures.
