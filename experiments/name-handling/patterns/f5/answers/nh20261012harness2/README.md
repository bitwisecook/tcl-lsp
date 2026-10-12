# Explicit real-package source: complete failure boundary

The unchanged real Tcltest 2.2.8 package fails in a fresh TMM child at `pwd`, before any selected Wiki test body runs. This diagnostic uses explicit `interp eval` of the real package bytes. It does not replace the native parent harness or establish successful test execution.

| Context | Actual identities | Setup | Real module evaluation | Child deletion |
| --- | --- | --- | --- | --- |
| RULE_INIT | All four units, `0:0`–`0:3` | Code 0 | Code 1: `invalid command name "pwd"` | Code 0 |
| Native procedure called from RULE_INIT | All four units, `0:0`–`0:3` | Code 0 | Code 1: `invalid command name "pwd"` | Code 0 |
| HTTP_REQUEST | 16 invocations, actual unit `0:0` | Code 0 | Code 1: `invalid command name "pwd"` | Code 0 |

All 24 records have complete setup, result and error capture fields. Setup's compact result is exactly `child-provider {8.4.6 8.4 ::} module-bytes 98763`. The error stack identifies `pwd` in `namespace eval ::tcltest` (script line 245), called from the actual `interp eval $nh_child $nh_module`. The retained package's initialisation invokes `[pwd]` for its working directory. This establishes this exact package's failure at that command in these children; it does not establish every possible harness failure or that an unexecuted original name lookup is unsupported.

All three configurations loaded. Each contains nine module procedures whose decoded bytes, concatenated in actual call order, exactly equal `real-tcltest-2.2.8.tcl`: 98,763 bytes, SHA-256 `2ef6cc2a65738bf7cf8b9c247db7a3f1548a498adfc669f05e71c12cbad921a0`. The package was not rewritten. The controller changed the preceding attempt's setup result to a compact summary and exposed the module evaluation's actual completion instead of a list containing nested caught errors.

[review.json](review.json) records independent validation of 192 raw rows, all 24 identities, descriptor hashes, configured source reassembly, operation materialisation and cleanup. [records.json](records.json) and [ANSWERS.md](ANSWERS.md) retain all measurements. [reviewed-proofs.json](reviewed-proofs.json) contains three narrowly scoped failure-control representatives; none answers an original Wiki body. Readable results and stacks use reversible ASCII byte projection only; their original encoding remains `binary-scan-tcl-value`.

Every captured child deletion returned code 0. Post-delete configuration checks show all task configuration objects absent. No separate child inventory proves total interpreter-state absence, and the remaining remote staging is handled by a separate cleanup receipt.

Retained sanitised transport/deployment projection: documentation address, NHO-only logs, and omitted hardware/unrelated inventory. Tcl operation and result bytes are preserved. Historical executed-input hashes describe original execution, not these projected deployment fixtures.
