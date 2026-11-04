# Does moving a command to a repeated-colon destination preserve its body and remove the old entry?

Proof ID: `naming.corner.rename-colon-destination`

## Problem statement

A command moves to a repeated-colon destination. Normalising the destination by another provider's rules or leaving the old entry installed changes the callable table. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does moving a command to a repeated-colon destination preserve its body and remove the old entry?

## Exact control

```tcl
proc leaf {} {return ORIGINAL}; namespace eval ::n {}; rename leaf ::n:::moved; list [::n:::moved] [info commands ::leaf]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c207b7d; result rendering "ORIGINAL {}". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c207b7d; result rendering "ORIGINAL {}". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c207b7d; result rendering "ORIGINAL {}". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c207b7d; result rendering "ORIGINAL {}". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c207b7d; result rendering "ORIGINAL {}". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 4f524947494e414c207b7d; result rendering "ORIGINAL {}". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six captured programs return ORIGINAL and enumerate an empty old command result. Publication spellings remain provider-specific.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=rename-colon-destination.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for rename-colon-destination; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/rename-colon-destination.tcl`. Exact 8.4.20 input for rename-colon-destination; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/rename-colon-destination.stdout`. Exact 8.4.20 stdout for rename-colon-destination; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/rename-colon-destination.stderr`. Exact 8.4.20 stderr for rename-colon-destination; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 rename-colon-destination. JSON pointer `/rows/14`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/rename-colon-destination.tcl`. Exact 8.5.19 input for rename-colon-destination; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/rename-colon-destination.stdout`. Exact 8.5.19 stdout for rename-colon-destination; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/rename-colon-destination.stderr`. Exact 8.5.19 stderr for rename-colon-destination; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 rename-colon-destination. JSON pointer `/rows/54`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/rename-colon-destination.tcl`. Exact 8.6.18 input for rename-colon-destination; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/rename-colon-destination.stdout`. Exact 8.6.18 stdout for rename-colon-destination; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/rename-colon-destination.stderr`. Exact 8.6.18 stderr for rename-colon-destination; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 rename-colon-destination. JSON pointer `/rows/94`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/rename-colon-destination.tcl`. Exact 9.0.4 input for rename-colon-destination; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/rename-colon-destination.stdout`. Exact 9.0.4 stdout for rename-colon-destination; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/rename-colon-destination.stderr`. Exact 9.0.4 stderr for rename-colon-destination; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 rename-colon-destination. JSON pointer `/rows/134`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/rename-colon-destination.tcl`. Exact 9.1.0 input for rename-colon-destination; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/rename-colon-destination.stdout`. Exact 9.1.0 stdout for rename-colon-destination; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/rename-colon-destination.stderr`. Exact 9.1.0 stderr for rename-colon-destination; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 rename-colon-destination. JSON pointer `/rows/174`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/rename-colon-destination.tcl`. Exact Jim input for rename-colon-destination; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/rename-colon-destination.stdout`. Exact Jim stdout for rename-colon-destination; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/rename-colon-destination.stderr`. Exact Jim stderr for rename-colon-destination; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim rename-colon-destination. JSON pointer `/rows/214`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case rename-colon-destination --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The original executed script and its captured results are in [per-script evidence](../../../../evidence/tcl/native-naming-corner-rename-colon-destination.md). A replay is a separate execution and must record its own results.
