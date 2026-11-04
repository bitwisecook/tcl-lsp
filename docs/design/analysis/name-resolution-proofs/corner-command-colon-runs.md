# How does this command publication and lookup treat a repeated colon run?

Proof ID: `naming.corner.command-colon-runs`

## Problem statement

A command is declared and called through ::n:::leaf. Collapsing every colon run or retaining every byte gives different lookup keys; a UI QName cannot decide which behaviour the selected interpreter uses. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

How does this command publication and lookup treat a repeated colon run?

## Exact control

```tcl
namespace eval ::n {}; proc ::n:::leaf {} {return FOUND}; list [::n:::leaf] [namespace which -command ::n:::leaf]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 464f554e44203a3a6e3a3a6c656166; result rendering "FOUND ::n::leaf". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 464f554e44203a3a6e3a3a6c656166; result rendering "FOUND ::n::leaf". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 464f554e44203a3a6e3a3a6c656166; result rendering "FOUND ::n::leaf". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 464f554e44203a3a6e3a3a6c656166; result rendering "FOUND ::n::leaf". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 464f554e44203a3a6e3a3a6c656166; result rendering "FOUND ::n::leaf". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 464f554e44203a3a6e3a3a3a6c656166; result rendering "FOUND ::n:::leaf". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The captured C calls use ::n::leaf, while Jim retains the extra colon in the reported flat command key.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=command-colon-runs.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for command-colon-runs; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-colon-runs.tcl`. Exact 8.4.20 input for command-colon-runs; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-colon-runs.stdout`. Exact 8.4.20 stdout for command-colon-runs; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-colon-runs.stderr`. Exact 8.4.20 stderr for command-colon-runs; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 command-colon-runs. JSON pointer `/rows/0`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-colon-runs.tcl`. Exact 8.5.19 input for command-colon-runs; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-colon-runs.stdout`. Exact 8.5.19 stdout for command-colon-runs; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-colon-runs.stderr`. Exact 8.5.19 stderr for command-colon-runs; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 command-colon-runs. JSON pointer `/rows/40`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-colon-runs.tcl`. Exact 8.6.18 input for command-colon-runs; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-colon-runs.stdout`. Exact 8.6.18 stdout for command-colon-runs; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-colon-runs.stderr`. Exact 8.6.18 stderr for command-colon-runs; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 command-colon-runs. JSON pointer `/rows/80`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-colon-runs.tcl`. Exact 9.0.4 input for command-colon-runs; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-colon-runs.stdout`. Exact 9.0.4 stdout for command-colon-runs; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-colon-runs.stderr`. Exact 9.0.4 stderr for command-colon-runs; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 command-colon-runs. JSON pointer `/rows/120`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-colon-runs.tcl`. Exact 9.1.0 input for command-colon-runs; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-colon-runs.stdout`. Exact 9.1.0 stdout for command-colon-runs; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-colon-runs.stderr`. Exact 9.1.0 stderr for command-colon-runs; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 command-colon-runs. JSON pointer `/rows/160`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-colon-runs.tcl`. Exact Jim input for command-colon-runs; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-colon-runs.stdout`. Exact Jim stdout for command-colon-runs; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-colon-runs.stderr`. Exact Jim stderr for command-colon-runs; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim command-colon-runs. JSON pointer `/rows/200`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case command-colon-runs --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The original executed script and its captured results are in [per-script evidence](../../../../evidence/tcl/native-naming-corner-command-colon-runs.md). A replay is a separate execution and must record its own results.
