# Which prefix argv does the Jim child-interpreter handle alias enter?

Proof ID: `naming.corner.child-jim-handle-alias`

## Problem statement

A Jim child handle installs an alias with a baked PREFIX. C interp grammar does not establish that this handle syntax exists or that it preserves the same argv layout. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Which prefix argv does the Jim child-interpreter handle alias enter?

## Exact control

```tcl
set child [interp]; $child alias a list PREFIX; set result [$child eval {a ARG}]; $child delete; set result
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727020636d64203f617267202e2e2e3f22; result rendering "wrong # args: should be \"interp cmd ?arg ...?\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.5 8.5.19 | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727020636d64203f617267202e2e2e3f22; result rendering "wrong # args: should be \"interp cmd ?arg ...?\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.6 8.6.18 | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727020636d64203f617267202e2e2e3f22; result rendering "wrong # args: should be \"interp cmd ?arg ...?\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl9.0 9.0.4 | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727020636d64203f617267202e2e2e3f22; result rendering "wrong # args: should be \"interp cmd ?arg ...?\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl9.1 9.1.0 | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727020636d64203f617267202e2e2e3f22; result rendering "wrong # args: should be \"interp cmd ?arg ...?\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 50524546495820415247; result rendering "PREFIX ARG". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

Jim returns PREFIX ARG; the five C captures preserve the distinct interp grammar errors for this handle syntax.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=child-jim-handle-alias.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for child-jim-handle-alias; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/child-jim-handle-alias.tcl`; SHA256 `ead88549e26d23144743da82d589cf54670cfa2d6794d610b7771b57c7ebe6f9`. Exact 8.4.20 input for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/child-jim-handle-alias.stdout`; SHA256 `84ad3edc3d26102f5472e192926a67e2d63d5fcf6717d14b584432343910f3cd`. Exact 8.4.20 stdout for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/child-jim-handle-alias.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 child-jim-handle-alias. JSON pointer `/rows/17`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/child-jim-handle-alias.tcl`; SHA256 `ead88549e26d23144743da82d589cf54670cfa2d6794d610b7771b57c7ebe6f9`. Exact 8.5.19 input for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/child-jim-handle-alias.stdout`; SHA256 `84ad3edc3d26102f5472e192926a67e2d63d5fcf6717d14b584432343910f3cd`. Exact 8.5.19 stdout for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/child-jim-handle-alias.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 child-jim-handle-alias. JSON pointer `/rows/57`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/child-jim-handle-alias.tcl`; SHA256 `ead88549e26d23144743da82d589cf54670cfa2d6794d610b7771b57c7ebe6f9`. Exact 8.6.18 input for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/child-jim-handle-alias.stdout`; SHA256 `84ad3edc3d26102f5472e192926a67e2d63d5fcf6717d14b584432343910f3cd`. Exact 8.6.18 stdout for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/child-jim-handle-alias.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 child-jim-handle-alias. JSON pointer `/rows/97`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/child-jim-handle-alias.tcl`; SHA256 `ead88549e26d23144743da82d589cf54670cfa2d6794d610b7771b57c7ebe6f9`. Exact 9.0.4 input for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/child-jim-handle-alias.stdout`; SHA256 `84ad3edc3d26102f5472e192926a67e2d63d5fcf6717d14b584432343910f3cd`. Exact 9.0.4 stdout for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/child-jim-handle-alias.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 child-jim-handle-alias. JSON pointer `/rows/137`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/child-jim-handle-alias.tcl`; SHA256 `ead88549e26d23144743da82d589cf54670cfa2d6794d610b7771b57c7ebe6f9`. Exact 9.1.0 input for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/child-jim-handle-alias.stdout`; SHA256 `84ad3edc3d26102f5472e192926a67e2d63d5fcf6717d14b584432343910f3cd`. Exact 9.1.0 stdout for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/child-jim-handle-alias.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for child-jim-handle-alias; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 child-jim-handle-alias. JSON pointer `/rows/177`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/child-jim-handle-alias.tcl`; SHA256 `ead88549e26d23144743da82d589cf54670cfa2d6794d610b7771b57c7ebe6f9`. Exact Jim input for child-jim-handle-alias; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/child-jim-handle-alias.stdout`; SHA256 `872bd3f0a948a90fdbc3f2e80e845004aba59e3377c7e6b6d6267d450bc0f8d9`. Exact Jim stdout for child-jim-handle-alias; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/child-jim-handle-alias.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for child-jim-handle-alias; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim child-jim-handle-alias. JSON pointer `/rows/217`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case child-jim-handle-alias --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
