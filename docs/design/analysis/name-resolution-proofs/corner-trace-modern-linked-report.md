# What root and element do modern read traces report through this local alias?

Proof ID: `naming.corner.trace-modern-linked-report`

## Problem statement

A modern read trace is attached through a local alias to an array element. The same target can report different element fields across releases, so reporting geometry cannot serve as cell identity. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

What root and element do modern read traces report through this local alias?

## Exact control

```tcl
set a(k) VALUE; set log {}; proc observe {n e op} {lappend ::log [list $n $e $op]}; proc p {} {upvar #0 a(k) alias; trace add variable alias read observe; set alias}; list [p] $log
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b616c696173207b7d20726561647d7d; result rendering "VALUE {{alias {} read}}". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b616c696173207b7d20726561647d7d; result rendering "VALUE {{alias {} read}}". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b616c696173207b7d20726561647d7d; result rendering "VALUE {{alias {} read}}". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b616c696173206b20726561647d7d; result rendering "VALUE {{alias k read}}". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545207b7b616c696173206b20726561647d7d; result rendering "VALUE {{alias k read}}". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022747261636522; result rendering "invalid command name \"trace\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C8.4–8.6 report alias with an empty element; C9 reports alias with k. Jim retains the missing trace command result.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=trace-modern-linked-report.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for trace-modern-linked-report; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/trace-modern-linked-report.tcl`; SHA256 `887745edc5e3dea4838bb4ba6977dcf7117e080c61a84180cf1f810e904893e7`. Exact 8.4.20 input for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/trace-modern-linked-report.stdout`; SHA256 `0d1e79b9ffc3ee8493f609bdf3522a8027fed40edddc231c124f92444369630c`. Exact 8.4.20 stdout for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/trace-modern-linked-report.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 trace-modern-linked-report. JSON pointer `/rows/30`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/trace-modern-linked-report.tcl`; SHA256 `887745edc5e3dea4838bb4ba6977dcf7117e080c61a84180cf1f810e904893e7`. Exact 8.5.19 input for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/trace-modern-linked-report.stdout`; SHA256 `0d1e79b9ffc3ee8493f609bdf3522a8027fed40edddc231c124f92444369630c`. Exact 8.5.19 stdout for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/trace-modern-linked-report.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 trace-modern-linked-report. JSON pointer `/rows/70`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/trace-modern-linked-report.tcl`; SHA256 `887745edc5e3dea4838bb4ba6977dcf7117e080c61a84180cf1f810e904893e7`. Exact 8.6.18 input for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/trace-modern-linked-report.stdout`; SHA256 `0d1e79b9ffc3ee8493f609bdf3522a8027fed40edddc231c124f92444369630c`. Exact 8.6.18 stdout for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/trace-modern-linked-report.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 trace-modern-linked-report. JSON pointer `/rows/110`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/trace-modern-linked-report.tcl`; SHA256 `887745edc5e3dea4838bb4ba6977dcf7117e080c61a84180cf1f810e904893e7`. Exact 9.0.4 input for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/trace-modern-linked-report.stdout`; SHA256 `b8aadf743dcf33beceee029260f0f9bcd4dee1169e6a9db12e7842d6f8cfaee8`. Exact 9.0.4 stdout for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/trace-modern-linked-report.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 trace-modern-linked-report. JSON pointer `/rows/150`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/trace-modern-linked-report.tcl`; SHA256 `887745edc5e3dea4838bb4ba6977dcf7117e080c61a84180cf1f810e904893e7`. Exact 9.1.0 input for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/trace-modern-linked-report.stdout`; SHA256 `b8aadf743dcf33beceee029260f0f9bcd4dee1169e6a9db12e7842d6f8cfaee8`. Exact 9.1.0 stdout for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/trace-modern-linked-report.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for trace-modern-linked-report; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 trace-modern-linked-report. JSON pointer `/rows/190`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/trace-modern-linked-report.tcl`; SHA256 `887745edc5e3dea4838bb4ba6977dcf7117e080c61a84180cf1f810e904893e7`. Exact Jim input for trace-modern-linked-report; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/trace-modern-linked-report.stdout`; SHA256 `8f255880286ee15b140e7b6ec8856def5a8c8688d3d255e119eea129bf30d4ec`. Exact Jim stdout for trace-modern-linked-report; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/trace-modern-linked-report.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for trace-modern-linked-report; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim trace-modern-linked-report. JSON pointer `/rows/230`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case trace-modern-linked-report --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
