# Collision-avoiding spellings in TMM

This separately adapted run changes only enumerated procedure and frame-variable spellings. It preserves the earlier original-program collision failures. Twelve configuration jobs loaded, producing 72 complete records: eight programs on all four RULE_INIT units, two on all four procedure units (called from RULE_INIT), and two across 16 HTTP_REQUEST invocations on actual unit `0:0`.

| Adapted program | Contexts | Whole-program completion and result | Final operation |
| --- | --- | --- | --- |
| Frame wrapper | RULE_INIT, native procedure, traffic | 0: `changed` | Reached |
| Frame wrapper near miss | RULE_INIT, native procedure, traffic | 0: `original` | Reached |
| Alias prefix lifetime | RULE_INIT | 0: `{::nh::emit {a b}} {{a b} {c d}}` | Reached |
| Computed binding mutation | RULE_INIT | 0: `replacement` | Reached |
| Computed qualified name | RULE_INIT | 0: `first` | Reached |
| List prefix | RULE_INIT | 0: `{a b} {c d}` | Reached |
| String prefix near miss | RULE_INIT | 1: invalid command `::nh::two` | Adapted final `eval` reached; splitting the command name causes the error |
| OO deleted-object near miss | RULE_INIT | 1: invalid command `oo::class` | Unreached: class creation fails at the first operation |

The first two rows each represent three case/context combinations, giving 11 combinations whose adapted final operation runs. The OO row is an earlier capability failure. Generated `lookup` fields describe whole-program evaluation; they must not be read as proof that the OO final deleted-object call ran. No original matrix answer is promoted. No per-job Tcl patchlevel control was captured in this run; it remains unrecorded rather than borrowed from another run.

[review.json](review.json) anchors each original source hash, mapping and adapted operation hash. Independent reconstruction confirms the actual operation differs from the original solely by its listed procedure/variable substitutions, and the exact adapted bytes are materialised in each configured input. The original portable source remains unchanged. Metadata's ownership lists describe the original names; actual guards and cleanup use the renamed spellings through the retained generated scripts.

Every guard and authored cleanup returned code 0. Guards first reject existing owned namespace, procedure, variable or absolute-command slots; they also check exact fallback global procedure names where a namespace definition could place them there. Cleanup removes those guarded names, reports `ownership-cleanup execution-namespace :: remaining {}`, and successfully executes the guarded fallback deletions. The native `::forward` command and existing original frame variables were neither renamed nor removed. These scoped checks do not establish absence of all interpreter state.

All task configuration objects are absent after deletion. [records.json](records.json) and [ANSWERS.md](ANSWERS.md) retain all records, including error stacks. [reviewed-proofs.json](reviewed-proofs.json) contains 11 independent adapted-program representatives and excludes the OO earlier failure. All readable TMM values use reversible ASCII byte projection, retaining original `binary-scan-tcl-value` encoding. Remote staging awaits a separate cleanup receipt.

Retained sanitised transport/deployment projection: documentation address, NHO-only logs, and omitted hardware/unrelated inventory. Tcl operation and result bytes are preserved. Historical executed-input hashes describe original execution, not these projected deployment fixtures.
