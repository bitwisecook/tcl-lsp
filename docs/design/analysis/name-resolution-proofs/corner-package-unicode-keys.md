# Which versions are returned for these two generated Unicode package names?

Proof ID: `naming.corner.package-unicode-keys`

## Problem statement

Two Unicode package names are provided with different versions. A command-name distinctness result does not establish package-name key comparison or version selection for these evaluated names. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Which versions are returned for these two generated Unicode package names?

## Exact control

```tcl
set one [format %c 233]; set two "e[format %c 769]"; package provide $one 1.0; package provide $two 2.0; list [package require $one] [package require $two]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 312e3020322e30; result rendering "1.0 2.0". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 312e3020322e30; result rendering "1.0 2.0". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 312e3020322e30; result rendering "1.0 2.0". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 312e3020322e30; result rendering "1.0 2.0". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 312e3020322e30; result rendering "1.0 2.0". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 312e3020312e30; result rendering "1.0 1.0". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C programs return 1.0 2.0; Jim returns 1.0 1.0 in this exact control. These observations alone do not identify the internal key comparison mechanism.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=package-unicode-keys.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for package-unicode-keys; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/package-unicode-keys.tcl`. Exact 8.4.20 input for package-unicode-keys; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/package-unicode-keys.stdout`. Exact 8.4.20 stdout for package-unicode-keys; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/package-unicode-keys.stderr`. Exact 8.4.20 stderr for package-unicode-keys; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 package-unicode-keys. JSON pointer `/rows/34`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/package-unicode-keys.tcl`. Exact 8.5.19 input for package-unicode-keys; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/package-unicode-keys.stdout`. Exact 8.5.19 stdout for package-unicode-keys; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/package-unicode-keys.stderr`. Exact 8.5.19 stderr for package-unicode-keys; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 package-unicode-keys. JSON pointer `/rows/74`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/package-unicode-keys.tcl`. Exact 8.6.18 input for package-unicode-keys; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/package-unicode-keys.stdout`. Exact 8.6.18 stdout for package-unicode-keys; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/package-unicode-keys.stderr`. Exact 8.6.18 stderr for package-unicode-keys; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 package-unicode-keys. JSON pointer `/rows/114`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/package-unicode-keys.tcl`. Exact 9.0.4 input for package-unicode-keys; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/package-unicode-keys.stdout`. Exact 9.0.4 stdout for package-unicode-keys; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/package-unicode-keys.stderr`. Exact 9.0.4 stderr for package-unicode-keys; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 package-unicode-keys. JSON pointer `/rows/154`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/package-unicode-keys.tcl`. Exact 9.1.0 input for package-unicode-keys; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/package-unicode-keys.stdout`. Exact 9.1.0 stdout for package-unicode-keys; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/package-unicode-keys.stderr`. Exact 9.1.0 stderr for package-unicode-keys; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 package-unicode-keys. JSON pointer `/rows/194`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/package-unicode-keys.tcl`. Exact Jim input for package-unicode-keys; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/package-unicode-keys.stdout`. Exact Jim stdout for package-unicode-keys; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/package-unicode-keys.stderr`. Exact Jim stderr for package-unicode-keys; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim package-unicode-keys. JSON pointer `/rows/234`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case package-unicode-keys --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The original executed script and its captured results are in [per-script evidence](../../../../evidence/tcl/native-naming-corner-package-unicode-keys.md). A replay is a separate execution and must record its own results.
