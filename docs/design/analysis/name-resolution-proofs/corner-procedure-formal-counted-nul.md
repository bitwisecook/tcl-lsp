# Does this generated NUL-bearing formal create the plain x local?

Proof ID: `naming.corner.procedure-formal-counted-nul`

## Problem statement

The formal list contains a generated x-NUL-tail name. Treating a formal as a CString prefix could create x; formal-list storage must be measured independently of a variable operand or raw source channel. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this generated NUL-bearing formal create the plain x local?

## Exact control

```tcl
set formal "x[format %c 0]tail"; proc p [list $formal] {list [info locals] [info exists x]}; p VALUE
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 78007461696c2030; result rendering "x\u0000tail 0". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six captured programs enumerate the full formal and report plain x absent. This is a generated formal-name control, not a raw00 compiler preparation proof.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=procedure-formal-counted-nul.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for procedure-formal-counted-nul; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/procedure-formal-counted-nul.tcl`. Exact 8.4.20 input for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/procedure-formal-counted-nul.stdout`. Exact 8.4.20 stdout for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/procedure-formal-counted-nul.stderr`. Exact 8.4.20 stderr for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 procedure-formal-counted-nul. JSON pointer `/rows/9`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/procedure-formal-counted-nul.tcl`. Exact 8.5.19 input for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/procedure-formal-counted-nul.stdout`. Exact 8.5.19 stdout for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/procedure-formal-counted-nul.stderr`. Exact 8.5.19 stderr for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 procedure-formal-counted-nul. JSON pointer `/rows/49`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/procedure-formal-counted-nul.tcl`. Exact 8.6.18 input for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/procedure-formal-counted-nul.stdout`. Exact 8.6.18 stdout for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/procedure-formal-counted-nul.stderr`. Exact 8.6.18 stderr for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 procedure-formal-counted-nul. JSON pointer `/rows/89`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/procedure-formal-counted-nul.tcl`. Exact 9.0.4 input for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/procedure-formal-counted-nul.stdout`. Exact 9.0.4 stdout for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/procedure-formal-counted-nul.stderr`. Exact 9.0.4 stderr for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 procedure-formal-counted-nul. JSON pointer `/rows/129`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/procedure-formal-counted-nul.tcl`. Exact 9.1.0 input for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/procedure-formal-counted-nul.stdout`. Exact 9.1.0 stdout for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/procedure-formal-counted-nul.stderr`. Exact 9.1.0 stderr for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 procedure-formal-counted-nul. JSON pointer `/rows/169`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/procedure-formal-counted-nul.tcl`. Exact Jim input for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/procedure-formal-counted-nul.stdout`. Exact Jim stdout for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/procedure-formal-counted-nul.stderr`. Exact Jim stderr for procedure-formal-counted-nul; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim procedure-formal-counted-nul. JSON pointer `/rows/209`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case procedure-formal-counted-nul --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The original executed script and its captured results are in [per-script evidence](../../../../evidence/tcl/native-naming-corner-procedure-formal-counted-nul.md). A replay is a separate execution and must record its own results.
