# What namespace current value results from the dynamically generated NUL-bearing namespace name?

Proof ID: `naming.corner.namespace-counted-nul`

## Problem statement

A namespace operand is generated with a NUL and suffix. A full counted operand and a clipped namespace operand would report different current names; command lookup results do not answer this namespace purpose. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

What namespace current value results from the dynamically generated NUL-bearing namespace name?

## Exact control

```tcl
set name "::n[format %c 0]tail"; namespace eval $name {namespace current}
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e007461696c; result rendering "::n\u0000tail". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e007461696c; result rendering "::n\u0000tail". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e007461696c; result rendering "::n\u0000tail". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e007461696c; result rendering "::n\u0000tail". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e007461696c; result rendering "::n\u0000tail". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 3a3a6e; result rendering "::n". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The C programs report the full ::n-NUL-tail current name; Jim reports ::n. This establishes only the format-produced namespace operand and its report, without a raw source00 or physical allocation claim.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=namespace-counted-nul.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for namespace-counted-nul; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/namespace-counted-nul.tcl`; SHA256 `6625c63f24c6466e9ce2bcfa4c70f70ea3ec121bdee2b51787f167239c50b625`. Exact 8.4.20 input for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/namespace-counted-nul.stdout`; SHA256 `cf3315b9d63a42d7d0249a9a919a16feef99582eeaba594669ac26723d88b7cf`. Exact 8.4.20 stdout for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/namespace-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 namespace-counted-nul. JSON pointer `/rows/5`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/namespace-counted-nul.tcl`; SHA256 `6625c63f24c6466e9ce2bcfa4c70f70ea3ec121bdee2b51787f167239c50b625`. Exact 8.5.19 input for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/namespace-counted-nul.stdout`; SHA256 `cf3315b9d63a42d7d0249a9a919a16feef99582eeaba594669ac26723d88b7cf`. Exact 8.5.19 stdout for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/namespace-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 namespace-counted-nul. JSON pointer `/rows/45`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/namespace-counted-nul.tcl`; SHA256 `6625c63f24c6466e9ce2bcfa4c70f70ea3ec121bdee2b51787f167239c50b625`. Exact 8.6.18 input for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/namespace-counted-nul.stdout`; SHA256 `cf3315b9d63a42d7d0249a9a919a16feef99582eeaba594669ac26723d88b7cf`. Exact 8.6.18 stdout for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/namespace-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 namespace-counted-nul. JSON pointer `/rows/85`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/namespace-counted-nul.tcl`; SHA256 `6625c63f24c6466e9ce2bcfa4c70f70ea3ec121bdee2b51787f167239c50b625`. Exact 9.0.4 input for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/namespace-counted-nul.stdout`; SHA256 `cf3315b9d63a42d7d0249a9a919a16feef99582eeaba594669ac26723d88b7cf`. Exact 9.0.4 stdout for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/namespace-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 namespace-counted-nul. JSON pointer `/rows/125`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/namespace-counted-nul.tcl`; SHA256 `6625c63f24c6466e9ce2bcfa4c70f70ea3ec121bdee2b51787f167239c50b625`. Exact 9.1.0 input for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/namespace-counted-nul.stdout`; SHA256 `cf3315b9d63a42d7d0249a9a919a16feef99582eeaba594669ac26723d88b7cf`. Exact 9.1.0 stdout for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/namespace-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for namespace-counted-nul; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 namespace-counted-nul. JSON pointer `/rows/165`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/namespace-counted-nul.tcl`; SHA256 `6625c63f24c6466e9ce2bcfa4c70f70ea3ec121bdee2b51787f167239c50b625`. Exact Jim input for namespace-counted-nul; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/namespace-counted-nul.stdout`; SHA256 `9076a874c91be22d94537588069b7a70ed7c8fb4c1148a1fd4433dd25441a0df`. Exact Jim stdout for namespace-counted-nul; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/namespace-counted-nul.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for namespace-counted-nul; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim namespace-counted-nul. JSON pointer `/rows/205`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case namespace-counted-nul --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
