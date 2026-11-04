# Does this entered execution trace replace the implementation selected for the command?

Proof ID: `naming.corner.trace-execution-target-rebind`

## Problem statement

An enter trace renames the command and installs a new implementation under its old name. Pre-trace lookup and entered implementation selection can disagree; callback-free reasoning cannot decide this call. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this entered execution trace replace the implementation selected for the command?

## Exact control

```tcl
proc before {command operation} {rename leaf original; proc leaf {} {return NEW}}; proc leaf {} {return OLD}; trace add execution leaf enter before; leaf
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022747261636522; result rendering "invalid command name \"trace\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The five C programs return NEW after the trace replacement; Jim retains the missing trace command error. This supplies no arbitrary callback-free lookup licence.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=trace-execution-target-rebind.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for trace-execution-target-rebind; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/trace-execution-target-rebind.tcl`. Exact 8.4.20 input for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/trace-execution-target-rebind.stdout`. Exact 8.4.20 stdout for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/trace-execution-target-rebind.stderr`. Exact 8.4.20 stderr for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 trace-execution-target-rebind. JSON pointer `/rows/32`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/trace-execution-target-rebind.tcl`. Exact 8.5.19 input for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/trace-execution-target-rebind.stdout`. Exact 8.5.19 stdout for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/trace-execution-target-rebind.stderr`. Exact 8.5.19 stderr for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 trace-execution-target-rebind. JSON pointer `/rows/72`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/trace-execution-target-rebind.tcl`. Exact 8.6.18 input for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/trace-execution-target-rebind.stdout`. Exact 8.6.18 stdout for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/trace-execution-target-rebind.stderr`. Exact 8.6.18 stderr for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 trace-execution-target-rebind. JSON pointer `/rows/112`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/trace-execution-target-rebind.tcl`. Exact 9.0.4 input for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/trace-execution-target-rebind.stdout`. Exact 9.0.4 stdout for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/trace-execution-target-rebind.stderr`. Exact 9.0.4 stderr for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 trace-execution-target-rebind. JSON pointer `/rows/152`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/trace-execution-target-rebind.tcl`. Exact 9.1.0 input for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/trace-execution-target-rebind.stdout`. Exact 9.1.0 stdout for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/trace-execution-target-rebind.stderr`. Exact 9.1.0 stderr for trace-execution-target-rebind; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 trace-execution-target-rebind. JSON pointer `/rows/192`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/trace-execution-target-rebind.tcl`. Exact Jim input for trace-execution-target-rebind; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/trace-execution-target-rebind.stdout`. Exact Jim stdout for trace-execution-target-rebind; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/trace-execution-target-rebind.stderr`. Exact Jim stderr for trace-execution-target-rebind; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim trace-execution-target-rebind. JSON pointer `/rows/232`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case trace-execution-target-rebind --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The original executed script and its captured results are in [per-script evidence](../../../../evidence/tcl/native-naming-corner-trace-execution-target-rebind.md). A replay is a separate execution and must record its own results.
