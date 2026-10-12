# Does namespace upvar bind the selected namespace variable in this program?

Proof ID: `naming.corner.namespace-upvar-qualification`

## Problem statement

namespace upvar selects one variable in namespace n. Reusing ordinary upvar's grammar would miss release-specific command availability and the namespace operand's qualification purpose. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does namespace upvar bind the selected namespace variable in this program?

## Exact control

```tcl
namespace eval ::n {variable x VALUE}; proc p {} {namespace upvar ::n x alias; set alias}; p
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | unsupported | Process exit 0; guest completion 1; captured result hex 626164206f7074696f6e20227570766172223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368; result rendering "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C8.5–9.1 and Jim return VALUE; C8.4 retains its unavailable upvar-option error.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=namespace-upvar-qualification.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for namespace-upvar-qualification; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/namespace-upvar-qualification.tcl`; SHA256 `edb775ce9a825700a676a363296f3dd25b03961987c0bd4609f0d42238cf0775`. Exact 8.4.20 input for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/namespace-upvar-qualification.stdout`; SHA256 `1bc38b0a6059ea82ea5bbc6c405d76a72e00ce0f4af12f3de4d329ef9c320816`. Exact 8.4.20 stdout for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/namespace-upvar-qualification.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 namespace-upvar-qualification. JSON pointer `/rows/22`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/namespace-upvar-qualification.tcl`; SHA256 `edb775ce9a825700a676a363296f3dd25b03961987c0bd4609f0d42238cf0775`. Exact 8.5.19 input for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/namespace-upvar-qualification.stdout`; SHA256 `ae643b7cffa9e96b0964efad5d1c359bb85641003796e3cfb9a80fea5fefa4ce`. Exact 8.5.19 stdout for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/namespace-upvar-qualification.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 namespace-upvar-qualification. JSON pointer `/rows/62`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/namespace-upvar-qualification.tcl`; SHA256 `edb775ce9a825700a676a363296f3dd25b03961987c0bd4609f0d42238cf0775`. Exact 8.6.18 input for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/namespace-upvar-qualification.stdout`; SHA256 `ae643b7cffa9e96b0964efad5d1c359bb85641003796e3cfb9a80fea5fefa4ce`. Exact 8.6.18 stdout for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/namespace-upvar-qualification.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 namespace-upvar-qualification. JSON pointer `/rows/102`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/namespace-upvar-qualification.tcl`; SHA256 `edb775ce9a825700a676a363296f3dd25b03961987c0bd4609f0d42238cf0775`. Exact 9.0.4 input for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/namespace-upvar-qualification.stdout`; SHA256 `ae643b7cffa9e96b0964efad5d1c359bb85641003796e3cfb9a80fea5fefa4ce`. Exact 9.0.4 stdout for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/namespace-upvar-qualification.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 namespace-upvar-qualification. JSON pointer `/rows/142`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/namespace-upvar-qualification.tcl`; SHA256 `edb775ce9a825700a676a363296f3dd25b03961987c0bd4609f0d42238cf0775`. Exact 9.1.0 input for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/namespace-upvar-qualification.stdout`; SHA256 `ae643b7cffa9e96b0964efad5d1c359bb85641003796e3cfb9a80fea5fefa4ce`. Exact 9.1.0 stdout for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/namespace-upvar-qualification.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for namespace-upvar-qualification; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 namespace-upvar-qualification. JSON pointer `/rows/182`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/namespace-upvar-qualification.tcl`; SHA256 `edb775ce9a825700a676a363296f3dd25b03961987c0bd4609f0d42238cf0775`. Exact Jim input for namespace-upvar-qualification; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/namespace-upvar-qualification.stdout`; SHA256 `ae643b7cffa9e96b0964efad5d1c359bb85641003796e3cfb9a80fea5fefa4ce`. Exact Jim stdout for namespace-upvar-qualification; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/namespace-upvar-qualification.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for namespace-upvar-qualification; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim namespace-upvar-qualification. JSON pointer `/rows/222`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case namespace-upvar-qualification --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
