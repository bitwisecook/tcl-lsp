# Does this constructor-initialised object variable reach the instance method?

Proof ID: `naming.corner.oo-object-namespace-variable`

## Problem statement

A constructor initialises an object variable that a method later reads. The instance namespace and method binding are distinct from the factory's lexical namespace, and stock older providers lack TclOO. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this constructor-initialised object variable reach the instance method?

## Exact control

```tcl
oo::class create C {variable x; constructor {} {set x VALUE}; method get {} {return $x}}; C create object; object get
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322; result rendering "invalid command name \"oo::class\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.5 8.5.19 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322; result rendering "invalid command name \"oo::class\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322; result rendering "invalid command name \"oo::class\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C8.6–9.1 return VALUE; stock C8.4, C8.5 and Jim retain the unavailable oo::class command errors.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=oo-object-namespace-variable.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for oo-object-namespace-variable; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/oo-object-namespace-variable.tcl`. Exact 8.4.20 input for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/oo-object-namespace-variable.stdout`. Exact 8.4.20 stdout for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/oo-object-namespace-variable.stderr`. Exact 8.4.20 stderr for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 oo-object-namespace-variable. JSON pointer `/rows/37`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/oo-object-namespace-variable.tcl`. Exact 8.5.19 input for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/oo-object-namespace-variable.stdout`. Exact 8.5.19 stdout for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/oo-object-namespace-variable.stderr`. Exact 8.5.19 stderr for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 oo-object-namespace-variable. JSON pointer `/rows/77`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/oo-object-namespace-variable.tcl`. Exact 8.6.18 input for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/oo-object-namespace-variable.stdout`. Exact 8.6.18 stdout for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/oo-object-namespace-variable.stderr`. Exact 8.6.18 stderr for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 oo-object-namespace-variable. JSON pointer `/rows/117`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/oo-object-namespace-variable.tcl`. Exact 9.0.4 input for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/oo-object-namespace-variable.stdout`. Exact 9.0.4 stdout for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/oo-object-namespace-variable.stderr`. Exact 9.0.4 stderr for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 oo-object-namespace-variable. JSON pointer `/rows/157`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/oo-object-namespace-variable.tcl`. Exact 9.1.0 input for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/oo-object-namespace-variable.stdout`. Exact 9.1.0 stdout for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/oo-object-namespace-variable.stderr`. Exact 9.1.0 stderr for oo-object-namespace-variable; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 oo-object-namespace-variable. JSON pointer `/rows/197`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/oo-object-namespace-variable.tcl`. Exact Jim input for oo-object-namespace-variable; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/oo-object-namespace-variable.stdout`. Exact Jim stdout for oo-object-namespace-variable; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/oo-object-namespace-variable.stderr`. Exact Jim stderr for oo-object-namespace-variable; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim oo-object-namespace-variable. JSON pointer `/rows/237`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case oo-object-namespace-variable --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The original executed script and its captured results are in [per-script evidence](../../../../evidence/tcl/native-naming-corner-oo-object-namespace-variable.md). A replay is a separate execution and must record its own results.
