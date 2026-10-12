# Does forgetting this import remove the imported callable after the first call?

Proof ID: `naming.corner.namespace-import-forget`

## Problem statement

An imported command is called and then forgotten. Retaining the original declaration forever would leave a stale callable; the forget grammar and resulting removal must be observed separately. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does forgetting this import remove the imported callable after the first call?

## Exact control

```tcl
namespace eval ::source {proc leaf {} {return SRC}; namespace export leaf}; namespace eval ::target {namespace import ::source::leaf; set before [leaf]; namespace forget ::source::leaf; list $before [info commands leaf]}
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 535243207b7d; result rendering "SRC {}". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 535243207b7d; result rendering "SRC {}". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 535243207b7d; result rendering "SRC {}". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 535243207b7d; result rendering "SRC {}". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 535243207b7d; result rendering "SRC {}". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 6e616d6573706163652c20756e6b6e6f776e20636f6d6d616e642022666f72676574223a2073686f756c642062652063616e6f6e6963616c2c20636f64652c2063757272656e742c2064656c6574652c20656e73656d626c652c206576616c2c206578706f72742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c2075707661722c207768696368; result rendering "namespace, unknown command \"forget\": should be canonical, code, current, delete, ensemble, eval, export, import, inscope, origin, parent, qualifiers, tail, upvar, which". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The captured C programs distinguish the successful imported call from the later empty command enumeration; Jim preserves the actual unsupported forget-option result.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=namespace-import-forget.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for namespace-import-forget; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/namespace-import-forget.tcl`; SHA256 `865242983c789b2ff47d81fd0abe2ffc244a2fa690509fb22c71dbc79ae27926`. Exact 8.4.20 input for namespace-import-forget; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/namespace-import-forget.stdout`; SHA256 `a999c88368d2ebee7938298c39b6aa3d0fc435d0a21ac65729cd89ebe9f87481`. Exact 8.4.20 stdout for namespace-import-forget; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/namespace-import-forget.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for namespace-import-forget; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 namespace-import-forget. JSON pointer `/rows/6`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/namespace-import-forget.tcl`; SHA256 `865242983c789b2ff47d81fd0abe2ffc244a2fa690509fb22c71dbc79ae27926`. Exact 8.5.19 input for namespace-import-forget; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/namespace-import-forget.stdout`; SHA256 `a999c88368d2ebee7938298c39b6aa3d0fc435d0a21ac65729cd89ebe9f87481`. Exact 8.5.19 stdout for namespace-import-forget; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/namespace-import-forget.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for namespace-import-forget; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 namespace-import-forget. JSON pointer `/rows/46`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/namespace-import-forget.tcl`; SHA256 `865242983c789b2ff47d81fd0abe2ffc244a2fa690509fb22c71dbc79ae27926`. Exact 8.6.18 input for namespace-import-forget; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/namespace-import-forget.stdout`; SHA256 `a999c88368d2ebee7938298c39b6aa3d0fc435d0a21ac65729cd89ebe9f87481`. Exact 8.6.18 stdout for namespace-import-forget; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/namespace-import-forget.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for namespace-import-forget; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 namespace-import-forget. JSON pointer `/rows/86`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/namespace-import-forget.tcl`; SHA256 `865242983c789b2ff47d81fd0abe2ffc244a2fa690509fb22c71dbc79ae27926`. Exact 9.0.4 input for namespace-import-forget; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/namespace-import-forget.stdout`; SHA256 `a999c88368d2ebee7938298c39b6aa3d0fc435d0a21ac65729cd89ebe9f87481`. Exact 9.0.4 stdout for namespace-import-forget; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/namespace-import-forget.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for namespace-import-forget; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 namespace-import-forget. JSON pointer `/rows/126`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/namespace-import-forget.tcl`; SHA256 `865242983c789b2ff47d81fd0abe2ffc244a2fa690509fb22c71dbc79ae27926`. Exact 9.1.0 input for namespace-import-forget; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/namespace-import-forget.stdout`; SHA256 `a999c88368d2ebee7938298c39b6aa3d0fc435d0a21ac65729cd89ebe9f87481`. Exact 9.1.0 stdout for namespace-import-forget; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/namespace-import-forget.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for namespace-import-forget; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 namespace-import-forget. JSON pointer `/rows/166`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/namespace-import-forget.tcl`; SHA256 `865242983c789b2ff47d81fd0abe2ffc244a2fa690509fb22c71dbc79ae27926`. Exact Jim input for namespace-import-forget; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/namespace-import-forget.stdout`; SHA256 `743997c3a322c3017acc7d46be3c069be350bbfe3beb1d65021cd32ffe7530af`. Exact Jim stdout for namespace-import-forget; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/namespace-import-forget.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for namespace-import-forget; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim namespace-import-forget. JSON pointer `/rows/206`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case namespace-import-forget --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
