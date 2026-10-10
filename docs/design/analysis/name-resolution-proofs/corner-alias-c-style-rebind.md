# Does this C-style interpreter alias call the replacement target after rename and redefinition?

Proof ID: `naming.corner.alias-c-style-rebind`

## Problem statement

A C-style interpreter alias targets a procedure that is renamed and replaced under the old target name. An implementation-bound alias and a name-bound alias would enter different bodies. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this C-style interpreter alias call the replacement target after rename and redefinition?

## Exact control

```tcl
proc target args {return OLD}; interp alias {} a {} target; rename target original; proc target args {return NEW}; a
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | unsupported | Process exit 0; guest completion 1; captured result hex 77726f6e67202320617267733a2073686f756c642062652022696e7465727022; result rendering "wrong # args: should be \"interp\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

C8.4–9.1 return NEW; Jim retains the actual interp grammar error for this creation door.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=alias-c-style-rebind.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for alias-c-style-rebind; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/alias-c-style-rebind.tcl`; SHA256 `adf063a862f1e5c75a9dd18a0b0b3ed511efab9272f3381345423fcd762003a1`. Exact 8.4.20 input for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/alias-c-style-rebind.stdout`; SHA256 `120666364bcd8d0719e03c6efd81156bfad2a1d747fc50bb0291e20ac07479e0`. Exact 8.4.20 stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/alias-c-style-rebind.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 alias-c-style-rebind. JSON pointer `/rows/12`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/alias-c-style-rebind.tcl`; SHA256 `adf063a862f1e5c75a9dd18a0b0b3ed511efab9272f3381345423fcd762003a1`. Exact 8.5.19 input for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/alias-c-style-rebind.stdout`; SHA256 `120666364bcd8d0719e03c6efd81156bfad2a1d747fc50bb0291e20ac07479e0`. Exact 8.5.19 stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/alias-c-style-rebind.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 alias-c-style-rebind. JSON pointer `/rows/52`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/alias-c-style-rebind.tcl`; SHA256 `adf063a862f1e5c75a9dd18a0b0b3ed511efab9272f3381345423fcd762003a1`. Exact 8.6.18 input for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/alias-c-style-rebind.stdout`; SHA256 `120666364bcd8d0719e03c6efd81156bfad2a1d747fc50bb0291e20ac07479e0`. Exact 8.6.18 stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/alias-c-style-rebind.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 alias-c-style-rebind. JSON pointer `/rows/92`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/alias-c-style-rebind.tcl`; SHA256 `adf063a862f1e5c75a9dd18a0b0b3ed511efab9272f3381345423fcd762003a1`. Exact 9.0.4 input for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/alias-c-style-rebind.stdout`; SHA256 `120666364bcd8d0719e03c6efd81156bfad2a1d747fc50bb0291e20ac07479e0`. Exact 9.0.4 stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/alias-c-style-rebind.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 alias-c-style-rebind. JSON pointer `/rows/132`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/alias-c-style-rebind.tcl`; SHA256 `adf063a862f1e5c75a9dd18a0b0b3ed511efab9272f3381345423fcd762003a1`. Exact 9.1.0 input for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/alias-c-style-rebind.stdout`; SHA256 `120666364bcd8d0719e03c6efd81156bfad2a1d747fc50bb0291e20ac07479e0`. Exact 9.1.0 stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/alias-c-style-rebind.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 alias-c-style-rebind. JSON pointer `/rows/172`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/alias-c-style-rebind.tcl`; SHA256 `adf063a862f1e5c75a9dd18a0b0b3ed511efab9272f3381345423fcd762003a1`. Exact Jim input for alias-c-style-rebind; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/alias-c-style-rebind.stdout`; SHA256 `efa186a533139a7d0c56df52a74d44d9ff13043109652ddd8e8a7360bed55121`. Exact Jim stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/alias-c-style-rebind.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim alias-c-style-rebind. JSON pointer `/rows/212`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case alias-c-style-rebind --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
