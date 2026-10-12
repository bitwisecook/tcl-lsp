# Does the namespace path provider precede the global command in this program?

Proof ID: `naming.corner.command-path-precedence`

## Problem statement

A namespace path contains a provider that competes with the global command. Local-before-root reasoning is insufficient unless the path creation door exists and its ordered lookup has been measured. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does the namespace path provider precede the global command in this program?

## Exact control

```tcl
namespace eval ::n {}; namespace eval ::path {proc leaf {} {return PATH}}; proc ::leaf {} {return ROOT}; namespace eval ::n {namespace path ::path; leaf}
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | unsupported | Process exit 0; guest completion 1; captured result hex 626164206f7074696f6e202270617468223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368; result rendering "bad option \"path\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 50415448; result rendering "PATH". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 50415448; result rendering "PATH". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 50415448; result rendering "PATH". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 50415448; result rendering "PATH". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 6e616d6573706163652c20756e6b6e6f776e20636f6d6d616e64202270617468223a2073686f756c642062652063616e6f6e6963616c2c20636f64652c2063757272656e742c2064656c6574652c20656e73656d626c652c206576616c2c206578706f72742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c2075707661722c207768696368; result rendering "namespace, unknown command \"path\": should be canonical, code, current, delete, ensemble, eval, export, import, inscope, origin, parent, qualifiers, tail, upvar, which". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C8.5–9.1 return PATH; C8.4 and Jim preserve the actual unavailable path-option errors.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=command-path-precedence.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for command-path-precedence; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-path-precedence.tcl`; SHA256 `8f640467a8e626e0e7320e80c6d28c96fe7bd62f5a208136b2c3b253c6649f61`. Exact 8.4.20 input for command-path-precedence; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-path-precedence.stdout`; SHA256 `dc2ae0615a2b2cbb6791b8c4487759cf59e935bb3442129355a8a57c5091e643`. Exact 8.4.20 stdout for command-path-precedence; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-path-precedence.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for command-path-precedence; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 command-path-precedence. JSON pointer `/rows/3`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-path-precedence.tcl`; SHA256 `8f640467a8e626e0e7320e80c6d28c96fe7bd62f5a208136b2c3b253c6649f61`. Exact 8.5.19 input for command-path-precedence; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-path-precedence.stdout`; SHA256 `64813d90a3d2f9fe9dd34a904180fc43e656568281dc41fc28020311470d22ee`. Exact 8.5.19 stdout for command-path-precedence; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-path-precedence.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for command-path-precedence; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 command-path-precedence. JSON pointer `/rows/43`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-path-precedence.tcl`; SHA256 `8f640467a8e626e0e7320e80c6d28c96fe7bd62f5a208136b2c3b253c6649f61`. Exact 8.6.18 input for command-path-precedence; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-path-precedence.stdout`; SHA256 `64813d90a3d2f9fe9dd34a904180fc43e656568281dc41fc28020311470d22ee`. Exact 8.6.18 stdout for command-path-precedence; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-path-precedence.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for command-path-precedence; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 command-path-precedence. JSON pointer `/rows/83`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-path-precedence.tcl`; SHA256 `8f640467a8e626e0e7320e80c6d28c96fe7bd62f5a208136b2c3b253c6649f61`. Exact 9.0.4 input for command-path-precedence; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-path-precedence.stdout`; SHA256 `64813d90a3d2f9fe9dd34a904180fc43e656568281dc41fc28020311470d22ee`. Exact 9.0.4 stdout for command-path-precedence; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-path-precedence.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for command-path-precedence; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 command-path-precedence. JSON pointer `/rows/123`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-path-precedence.tcl`; SHA256 `8f640467a8e626e0e7320e80c6d28c96fe7bd62f5a208136b2c3b253c6649f61`. Exact 9.1.0 input for command-path-precedence; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-path-precedence.stdout`; SHA256 `64813d90a3d2f9fe9dd34a904180fc43e656568281dc41fc28020311470d22ee`. Exact 9.1.0 stdout for command-path-precedence; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-path-precedence.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for command-path-precedence; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 command-path-precedence. JSON pointer `/rows/163`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-path-precedence.tcl`; SHA256 `8f640467a8e626e0e7320e80c6d28c96fe7bd62f5a208136b2c3b253c6649f61`. Exact Jim input for command-path-precedence; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-path-precedence.stdout`; SHA256 `746496b4f11b5e23ad843b2e9859e04e51b8217cac6b4d04e18c8d63be6ad6e4`. Exact Jim stdout for command-path-precedence; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-path-precedence.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for command-path-precedence; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim command-path-precedence. JSON pointer `/rows/203`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case command-path-precedence --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
