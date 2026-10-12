# Does this package loader install the variable and command visible to the requiring namespace?

Proof ID: `naming.corner.package-loader-namespace`

## Problem statement

A package loader modifies namespace state and publishes a procedure while a namespace body requires it. Assuming that a require operand is merely metadata would miss the entered loader effects and its lookup context. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this package loader install the variable and command visible to the requiring namespace?

## Exact control

```tcl
namespace eval ::n {variable stage BEFORE}; package ifneeded Demo 1.0 {namespace eval ::n {set stage LOADED; proc leaf {} {return INSTALLED}}; package provide Demo 1.0}; namespace eval ::n {list [package require Demo] $stage [leaf]}
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 312e30204c4f4144454420494e5354414c4c4544; result rendering "1.0 LOADED INSTALLED". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 312e30204c4f4144454420494e5354414c4c4544; result rendering "1.0 LOADED INSTALLED". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 312e30204c4f4144454420494e5354414c4c4544; result rendering "1.0 LOADED INSTALLED". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 312e30204c4f4144454420494e5354414c4c4544; result rendering "1.0 LOADED INSTALLED". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 312e30204c4f4144454420494e5354414c4c4544; result rendering "1.0 LOADED INSTALLED". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 7061636b6167652c20756e6b6e6f776e20636f6d6d616e64202269666e6565646564223a2073686f756c6420626520666f726765742c206e616d65732c2070726f766964652c2072657175697265; result rendering "package, unknown command \"ifneeded\": should be forget, names, provide, require". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

The five C programs return 1.0 LOADED INSTALLED; Jim retains its unavailable ifneeded subcommand result.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=package-loader-namespace.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for package-loader-namespace; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/package-loader-namespace.tcl`; SHA256 `753e3bf4540c538243cbfe61a8a1c2b8110e84af38aac7cbf93998da51e12fd7`. Exact 8.4.20 input for package-loader-namespace; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/package-loader-namespace.stdout`; SHA256 `1ba049413af8a9353b7d88391f3893ebea9ded379fd33eec961b338df402035d`. Exact 8.4.20 stdout for package-loader-namespace; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/package-loader-namespace.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for package-loader-namespace; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 package-loader-namespace. JSON pointer `/rows/35`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/package-loader-namespace.tcl`; SHA256 `753e3bf4540c538243cbfe61a8a1c2b8110e84af38aac7cbf93998da51e12fd7`. Exact 8.5.19 input for package-loader-namespace; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/package-loader-namespace.stdout`; SHA256 `1ba049413af8a9353b7d88391f3893ebea9ded379fd33eec961b338df402035d`. Exact 8.5.19 stdout for package-loader-namespace; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/package-loader-namespace.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for package-loader-namespace; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 package-loader-namespace. JSON pointer `/rows/75`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/package-loader-namespace.tcl`; SHA256 `753e3bf4540c538243cbfe61a8a1c2b8110e84af38aac7cbf93998da51e12fd7`. Exact 8.6.18 input for package-loader-namespace; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/package-loader-namespace.stdout`; SHA256 `1ba049413af8a9353b7d88391f3893ebea9ded379fd33eec961b338df402035d`. Exact 8.6.18 stdout for package-loader-namespace; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/package-loader-namespace.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for package-loader-namespace; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 package-loader-namespace. JSON pointer `/rows/115`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/package-loader-namespace.tcl`; SHA256 `753e3bf4540c538243cbfe61a8a1c2b8110e84af38aac7cbf93998da51e12fd7`. Exact 9.0.4 input for package-loader-namespace; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/package-loader-namespace.stdout`; SHA256 `1ba049413af8a9353b7d88391f3893ebea9ded379fd33eec961b338df402035d`. Exact 9.0.4 stdout for package-loader-namespace; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/package-loader-namespace.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for package-loader-namespace; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 package-loader-namespace. JSON pointer `/rows/155`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/package-loader-namespace.tcl`; SHA256 `753e3bf4540c538243cbfe61a8a1c2b8110e84af38aac7cbf93998da51e12fd7`. Exact 9.1.0 input for package-loader-namespace; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/package-loader-namespace.stdout`; SHA256 `1ba049413af8a9353b7d88391f3893ebea9ded379fd33eec961b338df402035d`. Exact 9.1.0 stdout for package-loader-namespace; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/package-loader-namespace.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for package-loader-namespace; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 package-loader-namespace. JSON pointer `/rows/195`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/package-loader-namespace.tcl`; SHA256 `753e3bf4540c538243cbfe61a8a1c2b8110e84af38aac7cbf93998da51e12fd7`. Exact Jim input for package-loader-namespace; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/package-loader-namespace.stdout`; SHA256 `bb5536a5ba9a6563be35b7a747c3d2f1e5e12c3cd04848b3d3d5b3e9ee49e29c`. Exact Jim stdout for package-loader-namespace; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/package-loader-namespace.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for package-loader-namespace; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim package-loader-namespace. JSON pointer `/rows/235`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case package-loader-namespace --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
