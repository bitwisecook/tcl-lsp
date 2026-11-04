# Which interpreter target and baked argv does the C child-interpreter alias enter?

Proof ID: `naming.corner.child-c-style-alias`

## Problem statement

A child interpreter alias targets a parent command with PREFIX baked before ARG. Equal command labels do not identify the target interpreter or the captured versus callsite argv roles. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Which interpreter target and baked argv does the C child-interpreter alias enter?

## Exact control

```tcl
set child [interp create]; proc target args {return [list PARENT $args]}; interp alias $child a {} target PREFIX; set result [interp eval $child {a ARG}]; interp delete $child; set result
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 504152454e54207b505245464958204152477d; result rendering "PARENT {PREFIX ARG}". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 504152454e54207b505245464958204152477d; result rendering "PARENT {PREFIX ARG}". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 504152454e54207b505245464958204152477d; result rendering "PARENT {PREFIX ARG}". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 504152454e54207b505245464958204152477d; result rendering "PARENT {PREFIX ARG}". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 504152454e54207b505245464958204152477d; result rendering "PARENT {PREFIX ARG}". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727022; result rendering "wrong # args: should be \"interp\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The five C captures return PARENT with PREFIX ARG; Jim retains the creation-door interp grammar error.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=child-c-style-alias.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for child-c-style-alias; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/child-c-style-alias.tcl`. Exact 8.4.20 input for child-c-style-alias; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/child-c-style-alias.stdout`. Exact 8.4.20 stdout for child-c-style-alias; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/child-c-style-alias.stderr`. Exact 8.4.20 stderr for child-c-style-alias; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 child-c-style-alias. JSON pointer `/rows/16`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/child-c-style-alias.tcl`. Exact 8.5.19 input for child-c-style-alias; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/child-c-style-alias.stdout`. Exact 8.5.19 stdout for child-c-style-alias; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/child-c-style-alias.stderr`. Exact 8.5.19 stderr for child-c-style-alias; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 child-c-style-alias. JSON pointer `/rows/56`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/child-c-style-alias.tcl`. Exact 8.6.18 input for child-c-style-alias; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/child-c-style-alias.stdout`. Exact 8.6.18 stdout for child-c-style-alias; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/child-c-style-alias.stderr`. Exact 8.6.18 stderr for child-c-style-alias; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 child-c-style-alias. JSON pointer `/rows/96`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/child-c-style-alias.tcl`. Exact 9.0.4 input for child-c-style-alias; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/child-c-style-alias.stdout`. Exact 9.0.4 stdout for child-c-style-alias; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/child-c-style-alias.stderr`. Exact 9.0.4 stderr for child-c-style-alias; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 child-c-style-alias. JSON pointer `/rows/136`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/child-c-style-alias.tcl`. Exact 9.1.0 input for child-c-style-alias; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/child-c-style-alias.stdout`. Exact 9.1.0 stdout for child-c-style-alias; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/child-c-style-alias.stderr`. Exact 9.1.0 stderr for child-c-style-alias; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 child-c-style-alias. JSON pointer `/rows/176`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/child-c-style-alias.tcl`. Exact Jim input for child-c-style-alias; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/child-c-style-alias.stdout`. Exact Jim stdout for child-c-style-alias; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/child-c-style-alias.stderr`. Exact Jim stderr for child-c-style-alias; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim child-c-style-alias. JSON pointer `/rows/216`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case child-c-style-alias --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The original executed script and its captured results are in [per-script evidence](../../../../evidence/tcl/native-naming-corner-child-c-style-alias.md). A replay is a separate execution and must record its own results.
