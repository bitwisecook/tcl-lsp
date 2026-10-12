# Does package present distinguish this generated NUL-bearing package name from Pkg?

Proof ID: `naming.corner.package-counted-nul-key`

## Problem statement

A package is provided under a generated NUL-bearing name and queried both by that name and its prefix. Package key selection is independent of command publication and the present subcommand may be unavailable. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does package present distinguish this generated NUL-bearing package name from Pkg?

## Exact control

```tcl
set name "Pkg[format %c 0]Tail"; package provide $name 1.0; list [package present $name] [catch {package present Pkg}]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 312e302031; result rendering "1.0 1". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 312e302031; result rendering "1.0 1". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 312e302031; result rendering "1.0 1". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 312e302031; result rendering "1.0 1". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 312e302031; result rendering "1.0 1". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 7061636b6167652c20756e6b6e6f776e20636f6d6d616e64202270726573656e74223a2073686f756c6420626520666f726765742c206e616d65732c2070726f766964652c2072657175697265; result rendering "package, unknown command \"present\": should be forget, names, provide, require". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C programs report full-name version1.0 and plain-name absence; Jim preserves its unavailable present subcommand result, so no Jim key-equivalence conclusion follows.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=package-counted-nul-key.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for package-counted-nul-key; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/package-counted-nul-key.tcl`; SHA256 `64a5b8a3e1092a41a08fa17d573335d3a84dfcc0b5447553e90bad807ca72b09`. Exact 8.4.20 input for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/package-counted-nul-key.stdout`; SHA256 `e83d0b14f75932729ff0633fd046bd3001d856fec2500133f9cbc54670379d42`. Exact 8.4.20 stdout for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/package-counted-nul-key.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 package-counted-nul-key. JSON pointer `/rows/33`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/package-counted-nul-key.tcl`; SHA256 `64a5b8a3e1092a41a08fa17d573335d3a84dfcc0b5447553e90bad807ca72b09`. Exact 8.5.19 input for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/package-counted-nul-key.stdout`; SHA256 `e83d0b14f75932729ff0633fd046bd3001d856fec2500133f9cbc54670379d42`. Exact 8.5.19 stdout for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/package-counted-nul-key.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 package-counted-nul-key. JSON pointer `/rows/73`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/package-counted-nul-key.tcl`; SHA256 `64a5b8a3e1092a41a08fa17d573335d3a84dfcc0b5447553e90bad807ca72b09`. Exact 8.6.18 input for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/package-counted-nul-key.stdout`; SHA256 `e83d0b14f75932729ff0633fd046bd3001d856fec2500133f9cbc54670379d42`. Exact 8.6.18 stdout for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/package-counted-nul-key.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 package-counted-nul-key. JSON pointer `/rows/113`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/package-counted-nul-key.tcl`; SHA256 `64a5b8a3e1092a41a08fa17d573335d3a84dfcc0b5447553e90bad807ca72b09`. Exact 9.0.4 input for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/package-counted-nul-key.stdout`; SHA256 `e83d0b14f75932729ff0633fd046bd3001d856fec2500133f9cbc54670379d42`. Exact 9.0.4 stdout for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/package-counted-nul-key.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 package-counted-nul-key. JSON pointer `/rows/153`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/package-counted-nul-key.tcl`; SHA256 `64a5b8a3e1092a41a08fa17d573335d3a84dfcc0b5447553e90bad807ca72b09`. Exact 9.1.0 input for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/package-counted-nul-key.stdout`; SHA256 `e83d0b14f75932729ff0633fd046bd3001d856fec2500133f9cbc54670379d42`. Exact 9.1.0 stdout for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/package-counted-nul-key.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for package-counted-nul-key; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 package-counted-nul-key. JSON pointer `/rows/193`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/package-counted-nul-key.tcl`; SHA256 `64a5b8a3e1092a41a08fa17d573335d3a84dfcc0b5447553e90bad807ca72b09`. Exact Jim input for package-counted-nul-key; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/package-counted-nul-key.stdout`; SHA256 `c05a1a92d6c473bd9d87ed8024eab2c4cff69359836ebea376bbc225f65c8db8`. Exact Jim stdout for package-counted-nul-key; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/package-counted-nul-key.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for package-counted-nul-key; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim package-counted-nul-key. JSON pointer `/rows/233`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case package-counted-nul-key --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
