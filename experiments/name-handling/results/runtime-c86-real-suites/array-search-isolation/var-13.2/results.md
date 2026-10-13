# Original var-13.2 results

| Provider | Process result | Actual Tcltest result | Wrapper returned |
| --- | --- | --- | --- |
| Production Runtime, Tcl 8.6 mode | SIGABRT (-6), 10.609 seconds | No final counts; only `var-13.2` started | No |
| C Tcl 8.6.18 | Exit 0, 0.034 seconds | Total 110, passed 1, skipped 109, failed 0 | Yes |

The original test fails to complete in a fresh Runtime process even when it is the only selected test. This establishes a reproducible compatibility gap without needing preceding test bodies from the broader run. The Runtime start marker and absence of a Tcltest failure banner are not pass evidence. The process outcome alone does not establish a source-level cause.

The existing runner hash matched its original build receipt. Its 2,787 tracked dep-info inputs were unchanged from that build commit, and all current compiled file hashes stayed unchanged during the runs. The original six harness input hashes stayed exact. The sole conservative-envelope mismatch is the sanitised JSON fixture recorded in `record.json`; it is absent from runner dep-info and production Rust include references. Nothing was restored from historical fixture snapshots.

`wrapper.tcl` contains the exact selection. `capture.py` records argv, bounds, original inputs, executable hashes, actual markers and final count lines. The two provider stdout/stderr files retain actual observations; `compiled-inputs-before.json.gz` records current compiled file hashes. `sealed-files.json` hashes each retained file. No source modification, build, VM execution, alternate Tcl release, JimTcl or F5 execution is claimed by this experiment.
