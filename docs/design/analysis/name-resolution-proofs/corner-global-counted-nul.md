# What value and local roster does this generated NUL-bearing global binding expose?

Proof ID: `naming.corner.global-counted-nul`

## Problem statement

global receives a generated NUL-bearing variable name while a plain prefix variable also exists. Clipping a reporting name could select the wrong global; the formal name and generated operand retain separate roles. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

What value and local roster does this generated NUL-bearing global binding expose?

## Exact control

```tcl
set name "v[format %c 0]tail"; set $name FULL; set v SHORT; proc p {name} {global $name; list [info locals] [set $name]}; p $name
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 6e616d652046554c4c; result rendering "name FULL". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six captures retain the observed name roster and FULL result; the binding purpose cannot be inferred from a rendered prefix.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=global-counted-nul.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for global-counted-nul; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/global-counted-nul.tcl`; SHA256 `9e45364947e50b790de25c813569dce03e6f2e0a28266acfec81fd46d4ddb9e0`. Exact 8.4.20 input for global-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/global-counted-nul.stdout`; SHA256 `4beba2ecd7bb18e6ffa4e08d102782db6f311776b169fb73cf6fc3609d5a6ca0`. Exact 8.4.20 stdout for global-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/global-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for global-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 global-counted-nul. JSON pointer `/rows/24`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/global-counted-nul.tcl`; SHA256 `9e45364947e50b790de25c813569dce03e6f2e0a28266acfec81fd46d4ddb9e0`. Exact 8.5.19 input for global-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/global-counted-nul.stdout`; SHA256 `4beba2ecd7bb18e6ffa4e08d102782db6f311776b169fb73cf6fc3609d5a6ca0`. Exact 8.5.19 stdout for global-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/global-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for global-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 global-counted-nul. JSON pointer `/rows/64`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/global-counted-nul.tcl`; SHA256 `9e45364947e50b790de25c813569dce03e6f2e0a28266acfec81fd46d4ddb9e0`. Exact 8.6.18 input for global-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/global-counted-nul.stdout`; SHA256 `4beba2ecd7bb18e6ffa4e08d102782db6f311776b169fb73cf6fc3609d5a6ca0`. Exact 8.6.18 stdout for global-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/global-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for global-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 global-counted-nul. JSON pointer `/rows/104`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/global-counted-nul.tcl`; SHA256 `9e45364947e50b790de25c813569dce03e6f2e0a28266acfec81fd46d4ddb9e0`. Exact 9.0.4 input for global-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/global-counted-nul.stdout`; SHA256 `4beba2ecd7bb18e6ffa4e08d102782db6f311776b169fb73cf6fc3609d5a6ca0`. Exact 9.0.4 stdout for global-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/global-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for global-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 global-counted-nul. JSON pointer `/rows/144`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/global-counted-nul.tcl`; SHA256 `9e45364947e50b790de25c813569dce03e6f2e0a28266acfec81fd46d4ddb9e0`. Exact 9.1.0 input for global-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/global-counted-nul.stdout`; SHA256 `4beba2ecd7bb18e6ffa4e08d102782db6f311776b169fb73cf6fc3609d5a6ca0`. Exact 9.1.0 stdout for global-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/global-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for global-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 global-counted-nul. JSON pointer `/rows/184`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/global-counted-nul.tcl`; SHA256 `9e45364947e50b790de25c813569dce03e6f2e0a28266acfec81fd46d4ddb9e0`. Exact Jim input for global-counted-nul; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/global-counted-nul.stdout`; SHA256 `4beba2ecd7bb18e6ffa4e08d102782db6f311776b169fb73cf6fc3609d5a6ca0`. Exact Jim stdout for global-counted-nul; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/global-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for global-counted-nul; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim global-counted-nul. JSON pointer `/rows/224`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case global-counted-nul --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
