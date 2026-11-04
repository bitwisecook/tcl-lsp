# Does the dynamically generated NUL-bearing command name call the full command in this program?

Proof ID: `naming.corner.command-counted-nul`

## Problem statement

A command operand is generated as leaf followed by NUL and tail, then invoked through a list. Prefix clipping would select a different name; the format-produced object must be distinguished from raw NUL source ingress. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does the dynamically generated NUL-bearing command name call the full command in this program?

## Exact control

```tcl
set name "leaf[format %c 0]tail"; proc $name {} {return FULL}; list [eval [list $name]] [info commands leaf*]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 46554c4c206c656166007461696c; result rendering "FULL leaf\u0000tail". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 46554c4c206c656166007461696c; result rendering "FULL leaf\u0000tail". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 46554c4c206c656166007461696c; result rendering "FULL leaf\u0000tail". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 46554c4c206c656166007461696c; result rendering "FULL leaf\u0000tail". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 46554c4c206c656166007461696c; result rendering "FULL leaf\u0000tail". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 46554c4c206c656166007461696c; result rendering "FULL leaf\u0000tail". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six programs call FULL and enumerate the retained NUL-bearing name. The NUL is produced by format; this does not test raw00 source bytes or native CString clipping.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=command-counted-nul.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for command-counted-nul; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-counted-nul.tcl`. Exact 8.4.20 input for command-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-counted-nul.stdout`. Exact 8.4.20 stdout for command-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-counted-nul.stderr`. Exact 8.4.20 stderr for command-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 command-counted-nul. JSON pointer `/rows/1`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-counted-nul.tcl`. Exact 8.5.19 input for command-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-counted-nul.stdout`. Exact 8.5.19 stdout for command-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-counted-nul.stderr`. Exact 8.5.19 stderr for command-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 command-counted-nul. JSON pointer `/rows/41`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-counted-nul.tcl`. Exact 8.6.18 input for command-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-counted-nul.stdout`. Exact 8.6.18 stdout for command-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-counted-nul.stderr`. Exact 8.6.18 stderr for command-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 command-counted-nul. JSON pointer `/rows/81`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-counted-nul.tcl`. Exact 9.0.4 input for command-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-counted-nul.stdout`. Exact 9.0.4 stdout for command-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-counted-nul.stderr`. Exact 9.0.4 stderr for command-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 command-counted-nul. JSON pointer `/rows/121`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-counted-nul.tcl`. Exact 9.1.0 input for command-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-counted-nul.stdout`. Exact 9.1.0 stdout for command-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-counted-nul.stderr`. Exact 9.1.0 stderr for command-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 command-counted-nul. JSON pointer `/rows/161`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-counted-nul.tcl`. Exact Jim input for command-counted-nul; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-counted-nul.stdout`. Exact Jim stdout for command-counted-nul; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-counted-nul.stderr`. Exact Jim stderr for command-counted-nul; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim command-counted-nul. JSON pointer `/rows/201`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case command-counted-nul --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The original executed script and its captured results are in [per-script evidence](../../../../evidence/tcl/native-naming-corner-command-counted-nul.md). A replay is a separate execution and must record its own results.
