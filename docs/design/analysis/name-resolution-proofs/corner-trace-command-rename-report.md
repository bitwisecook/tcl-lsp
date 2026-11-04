# Which old name, new name and operation does this command rename trace report?

Proof ID: `naming.corner.trace-command-rename-report`

## Problem statement

A rename trace receives old name, new name and operation. These callback values may be rooted reporting names rather than the original source operands, so their producer roles must be measured separately. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Which old name, new name and operation does this command rename trace report?

## Exact control

```tcl
set log {}; proc observe {old new op} {lappend ::log [list $old $new $op]}; proc leaf {} {return VALUE}; trace add command leaf rename observe; rename leaf moved; list [moved] $log
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b3a3a6c656166203a3a6d6f7665642072656e616d657d7d; result rendering "VALUE {{::leaf ::moved rename}}". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b3a3a6c656166203a3a6d6f7665642072656e616d657d7d; result rendering "VALUE {{::leaf ::moved rename}}". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b3a3a6c656166203a3a6d6f7665642072656e616d657d7d; result rendering "VALUE {{::leaf ::moved rename}}". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b3a3a6c656166203a3a6d6f7665642072656e616d657d7d; result rendering "VALUE {{::leaf ::moved rename}}". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b3a3a6c656166203a3a6d6f7665642072656e616d657d7d; result rendering "VALUE {{::leaf ::moved rename}}". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022747261636522; result rendering "invalid command name \"trace\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The five C programs return VALUE and ::leaf/::moved/rename; Jim preserves the missing trace command error.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=trace-command-rename-report.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for trace-command-rename-report; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/trace-command-rename-report.tcl`. Exact 8.4.20 input for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/trace-command-rename-report.stdout`. Exact 8.4.20 stdout for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/trace-command-rename-report.stderr`. Exact 8.4.20 stderr for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 trace-command-rename-report. JSON pointer `/rows/31`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/trace-command-rename-report.tcl`. Exact 8.5.19 input for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/trace-command-rename-report.stdout`. Exact 8.5.19 stdout for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/trace-command-rename-report.stderr`. Exact 8.5.19 stderr for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 trace-command-rename-report. JSON pointer `/rows/71`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/trace-command-rename-report.tcl`. Exact 8.6.18 input for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/trace-command-rename-report.stdout`. Exact 8.6.18 stdout for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/trace-command-rename-report.stderr`. Exact 8.6.18 stderr for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 trace-command-rename-report. JSON pointer `/rows/111`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/trace-command-rename-report.tcl`. Exact 9.0.4 input for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/trace-command-rename-report.stdout`. Exact 9.0.4 stdout for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/trace-command-rename-report.stderr`. Exact 9.0.4 stderr for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 trace-command-rename-report. JSON pointer `/rows/151`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/trace-command-rename-report.tcl`. Exact 9.1.0 input for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/trace-command-rename-report.stdout`. Exact 9.1.0 stdout for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/trace-command-rename-report.stderr`. Exact 9.1.0 stderr for trace-command-rename-report; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 trace-command-rename-report. JSON pointer `/rows/191`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/trace-command-rename-report.tcl`. Exact Jim input for trace-command-rename-report; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/trace-command-rename-report.stdout`. Exact Jim stdout for trace-command-rename-report; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/trace-command-rename-report.stderr`. Exact Jim stderr for trace-command-rename-report; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim trace-command-rename-report. JSON pointer `/rows/231`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case trace-command-rename-report --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The original executed script and its captured results are in [per-script evidence](../../../../evidence/tcl/native-naming-corner-trace-command-rename-report.md). A replay is a separate execution and must record its own results.
