# Does the qualified namespace array binding supply the later element read?

Proof ID: `naming.corner.variable-array-qualified`

## Problem statement

variable binds the qualified namespace array before an element read. Root variable binding and array index selection must remain separate, rather than treating the combined displayed word as a scalar local. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does the qualified namespace array binding supply the later element read?

## Exact control

```tcl
namespace eval ::n {variable a; set a(k) VALUE}; proc p {} {variable ::n::a; set a(k)}; p
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 56414c5545; result rendering "VALUE". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six captured programs return VALUE for the selected array element.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=variable-array-qualified.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for variable-array-qualified; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/variable-array-qualified.tcl`; SHA256 `47635cae306bd62f70f990253f3decf8fd356d280578cea9e30b09655a51e159`. Exact 8.4.20 input for variable-array-qualified; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/variable-array-qualified.stdout`; SHA256 `ae643b7cffa9e96b0964efad5d1c359bb85641003796e3cfb9a80fea5fefa4ce`. Exact 8.4.20 stdout for variable-array-qualified; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/variable-array-qualified.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for variable-array-qualified; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 variable-array-qualified. JSON pointer `/rows/26`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/variable-array-qualified.tcl`; SHA256 `47635cae306bd62f70f990253f3decf8fd356d280578cea9e30b09655a51e159`. Exact 8.5.19 input for variable-array-qualified; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/variable-array-qualified.stdout`; SHA256 `ae643b7cffa9e96b0964efad5d1c359bb85641003796e3cfb9a80fea5fefa4ce`. Exact 8.5.19 stdout for variable-array-qualified; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/variable-array-qualified.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for variable-array-qualified; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 variable-array-qualified. JSON pointer `/rows/66`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/variable-array-qualified.tcl`; SHA256 `47635cae306bd62f70f990253f3decf8fd356d280578cea9e30b09655a51e159`. Exact 8.6.18 input for variable-array-qualified; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/variable-array-qualified.stdout`; SHA256 `ae643b7cffa9e96b0964efad5d1c359bb85641003796e3cfb9a80fea5fefa4ce`. Exact 8.6.18 stdout for variable-array-qualified; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/variable-array-qualified.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for variable-array-qualified; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 variable-array-qualified. JSON pointer `/rows/106`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/variable-array-qualified.tcl`; SHA256 `47635cae306bd62f70f990253f3decf8fd356d280578cea9e30b09655a51e159`. Exact 9.0.4 input for variable-array-qualified; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/variable-array-qualified.stdout`; SHA256 `ae643b7cffa9e96b0964efad5d1c359bb85641003796e3cfb9a80fea5fefa4ce`. Exact 9.0.4 stdout for variable-array-qualified; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/variable-array-qualified.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for variable-array-qualified; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 variable-array-qualified. JSON pointer `/rows/146`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/variable-array-qualified.tcl`; SHA256 `47635cae306bd62f70f990253f3decf8fd356d280578cea9e30b09655a51e159`. Exact 9.1.0 input for variable-array-qualified; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/variable-array-qualified.stdout`; SHA256 `ae643b7cffa9e96b0964efad5d1c359bb85641003796e3cfb9a80fea5fefa4ce`. Exact 9.1.0 stdout for variable-array-qualified; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/variable-array-qualified.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for variable-array-qualified; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 variable-array-qualified. JSON pointer `/rows/186`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/variable-array-qualified.tcl`; SHA256 `47635cae306bd62f70f990253f3decf8fd356d280578cea9e30b09655a51e159`. Exact Jim input for variable-array-qualified; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/variable-array-qualified.stdout`; SHA256 `ae643b7cffa9e96b0964efad5d1c359bb85641003796e3cfb9a80fea5fefa4ce`. Exact Jim stdout for variable-array-qualified; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/variable-array-qualified.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for variable-array-qualified; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim variable-array-qualified. JSON pointer `/rows/226`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case variable-array-qualified --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
