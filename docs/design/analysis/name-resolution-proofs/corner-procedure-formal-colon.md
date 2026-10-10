# Are :odd and n::q accepted as formal parameters by this declaration?

Proof ID: `naming.corner.procedure-formal-colon`

## Problem statement

A procedure formal list contains :odd and n::q. Qualified variable-name acceptance does not establish that the procedure's formal grammar accepts these names at declaration. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Are :odd and n::q accepted as formal parameters by this declaration?

## Exact control

```tcl
proc p {:odd n::q} {list ${:odd} ${n::q} [info locals]}; p A B
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 1; captured result hex 70726f636564757265202270222068617320666f726d616c20706172616d6574657220226e3a3a71222074686174206973206e6f7420612073696d706c65206e616d65; result rendering "procedure \"p\" has formal parameter \"n::q\" that is not a simple name". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 1; captured result hex 666f726d616c20706172616d6574657220226e3a3a7122206973206e6f7420612073696d706c65206e616d65; result rendering "formal parameter \"n::q\" is not a simple name". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 1; captured result hex 666f726d616c20706172616d6574657220226e3a3a7122206973206e6f7420612073696d706c65206e616d65; result rendering "formal parameter \"n::q\" is not a simple name". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 1; captured result hex 666f726d616c20706172616d6574657220226e3a3a7122206973206e6f7420612073696d706c65206e616d65; result rendering "formal parameter \"n::q\" is not a simple name". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 1; captured result hex 666f726d616c20706172616d6574657220226e3a3a7122206973206e6f7420612073696d706c65206e616d65; result rendering "formal parameter \"n::q\" is not a simple name". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 412042207b3a6f6464206e3a3a717d; result rendering "A B {:odd n::q}". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The C programs reject n::q as a nonsimple formal; Jim accepts the two names and returns A B with its captured local roster.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=procedure-formal-colon.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for procedure-formal-colon; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/procedure-formal-colon.tcl`; SHA256 `b647712825e92003e04bb8e3aadf619ccfc4f5b51119f9eddad818a82c4144f2`. Exact 8.4.20 input for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/procedure-formal-colon.stdout`; SHA256 `2af34c3095dd038beea433d1ad5f967fccdadad1c57ccdeefd7e34830dc7faf9`. Exact 8.4.20 stdout for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/procedure-formal-colon.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 procedure-formal-colon. JSON pointer `/rows/10`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/procedure-formal-colon.tcl`; SHA256 `b647712825e92003e04bb8e3aadf619ccfc4f5b51119f9eddad818a82c4144f2`. Exact 8.5.19 input for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/procedure-formal-colon.stdout`; SHA256 `27c9afadffc22743316f47db93746769ed33d68ad5c50f3f369e239a3f18d43e`. Exact 8.5.19 stdout for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/procedure-formal-colon.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 procedure-formal-colon. JSON pointer `/rows/50`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/procedure-formal-colon.tcl`; SHA256 `b647712825e92003e04bb8e3aadf619ccfc4f5b51119f9eddad818a82c4144f2`. Exact 8.6.18 input for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/procedure-formal-colon.stdout`; SHA256 `27c9afadffc22743316f47db93746769ed33d68ad5c50f3f369e239a3f18d43e`. Exact 8.6.18 stdout for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/procedure-formal-colon.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 procedure-formal-colon. JSON pointer `/rows/90`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/procedure-formal-colon.tcl`; SHA256 `b647712825e92003e04bb8e3aadf619ccfc4f5b51119f9eddad818a82c4144f2`. Exact 9.0.4 input for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/procedure-formal-colon.stdout`; SHA256 `27c9afadffc22743316f47db93746769ed33d68ad5c50f3f369e239a3f18d43e`. Exact 9.0.4 stdout for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/procedure-formal-colon.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 procedure-formal-colon. JSON pointer `/rows/130`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/procedure-formal-colon.tcl`; SHA256 `b647712825e92003e04bb8e3aadf619ccfc4f5b51119f9eddad818a82c4144f2`. Exact 9.1.0 input for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/procedure-formal-colon.stdout`; SHA256 `27c9afadffc22743316f47db93746769ed33d68ad5c50f3f369e239a3f18d43e`. Exact 9.1.0 stdout for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/procedure-formal-colon.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for procedure-formal-colon; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 procedure-formal-colon. JSON pointer `/rows/170`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/procedure-formal-colon.tcl`; SHA256 `b647712825e92003e04bb8e3aadf619ccfc4f5b51119f9eddad818a82c4144f2`. Exact Jim input for procedure-formal-colon; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/procedure-formal-colon.stdout`; SHA256 `2be2c4352a3c1f4d23c81feeb1e8b973a98ef557e281b5719b945e13df173e8a`. Exact Jim stdout for procedure-formal-colon; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/procedure-formal-colon.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for procedure-formal-colon; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim procedure-formal-colon. JSON pointer `/rows/210`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case procedure-formal-colon --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
