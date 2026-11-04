# Does this rooted auto_index key install and call its matching procedure?

Proof ID: `naming.corner.autoload-qualified-name`

## Problem statement

auto_index has a rooted key whose script installs a matching procedure. The captured startup unknown helper determines whether this registration is entered; a key's presence alone is not a provider-independent callable proof. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this rooted auto_index key install and call its matching procedure?

## Exact control

```tcl
namespace eval ::n {}; set auto_index(::n::leaf) {proc ::n::leaf {} {return LOADED}}; ::n::leaf
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 4c4f41444544; result rendering "LOADED". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 4c4f41444544; result rendering "LOADED". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 4c4f41444544; result rendering "LOADED". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 4c4f41444544; result rendering "LOADED". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 4c4f41444544; result rendering "LOADED". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d6520223a3a6e3a3a6c65616622; result rendering "invalid command name \"::n::leaf\"". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The five C programs return LOADED; Jim retains the unknown ::n::leaf command result under its captured startup environment.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=autoload-qualified-name.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for autoload-qualified-name; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/autoload-qualified-name.tcl`. Exact 8.4.20 input for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/autoload-qualified-name.stdout`. Exact 8.4.20 stdout for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/autoload-qualified-name.stderr`. Exact 8.4.20 stderr for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 autoload-qualified-name. JSON pointer `/rows/36`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/autoload-qualified-name.tcl`. Exact 8.5.19 input for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/autoload-qualified-name.stdout`. Exact 8.5.19 stdout for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/autoload-qualified-name.stderr`. Exact 8.5.19 stderr for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 autoload-qualified-name. JSON pointer `/rows/76`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/autoload-qualified-name.tcl`. Exact 8.6.18 input for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/autoload-qualified-name.stdout`. Exact 8.6.18 stdout for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/autoload-qualified-name.stderr`. Exact 8.6.18 stderr for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 autoload-qualified-name. JSON pointer `/rows/116`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/autoload-qualified-name.tcl`. Exact 9.0.4 input for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/autoload-qualified-name.stdout`. Exact 9.0.4 stdout for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/autoload-qualified-name.stderr`. Exact 9.0.4 stderr for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 autoload-qualified-name. JSON pointer `/rows/156`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/autoload-qualified-name.tcl`. Exact 9.1.0 input for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/autoload-qualified-name.stdout`. Exact 9.1.0 stdout for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/autoload-qualified-name.stderr`. Exact 9.1.0 stderr for autoload-qualified-name; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 autoload-qualified-name. JSON pointer `/rows/196`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/autoload-qualified-name.tcl`. Exact Jim input for autoload-qualified-name; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/autoload-qualified-name.stdout`. Exact Jim stdout for autoload-qualified-name; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/autoload-qualified-name.stderr`. Exact Jim stderr for autoload-qualified-name; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim autoload-qualified-name. JSON pointer `/rows/236`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case autoload-qualified-name --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
