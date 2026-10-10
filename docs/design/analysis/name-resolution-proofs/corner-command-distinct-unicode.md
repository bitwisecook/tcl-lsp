# Do precomposed and combining-character command names select distinct declarations?

Proof ID: `naming.corner.command-distinct-unicode`

## Problem statement

Two command names use precomposed e-acute and e plus a combining acute. A Unicode-normalised display key could merge the declarations even when this provider selects them separately. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Do precomposed and combining-character command names select distinct declarations?

## Exact control

```tcl
set one [format %c 233]; set two "e[format %c 769]"; proc $one {} {return ONE}; proc $two {} {return TWO}; list [eval [list $one]] [eval [list $two]]
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| tcl8.5 8.5.19 | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| tcl8.6 8.6.18 | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| tcl9.0 9.0.4 | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| tcl9.1 9.1.0 | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 4f4e452054574f; result rendering "ONE TWO". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

All six captured programs return ONE TWO for the two separately generated names; no Unicode equivalence is inferred.

## Evidence

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original six-provider case rows; filter case=command-distinct-unicode.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`; SHA256 `f43510314da33f66bf8bdd8ee9028f241946e5610e355793015d9fdad8ac713c`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`; SHA256 `6ad68938c04dc4c42f5112084942d867ac254624ea725c513e15683329c08c9f`. Authored control body for command-distinct-unicode; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-distinct-unicode.tcl`; SHA256 `66bcf10ebbb6c73401d9be67c98e42f19f0051113ba725064a8afdf7f432600f`. Exact 8.4.20 input for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-distinct-unicode.stdout`; SHA256 `c4174cf7440b179d82c84e63b53ea46aa7b42ed41ac058b166748701330a4f69`. Exact 8.4.20 stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-distinct-unicode.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.4.20 stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.4.20 command-distinct-unicode. JSON pointer `/rows/2`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-distinct-unicode.tcl`; SHA256 `66bcf10ebbb6c73401d9be67c98e42f19f0051113ba725064a8afdf7f432600f`. Exact 8.5.19 input for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-distinct-unicode.stdout`; SHA256 `c4174cf7440b179d82c84e63b53ea46aa7b42ed41ac058b166748701330a4f69`. Exact 8.5.19 stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-distinct-unicode.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.5.19 stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.5.19 command-distinct-unicode. JSON pointer `/rows/42`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-distinct-unicode.tcl`; SHA256 `66bcf10ebbb6c73401d9be67c98e42f19f0051113ba725064a8afdf7f432600f`. Exact 8.6.18 input for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-distinct-unicode.stdout`; SHA256 `c4174cf7440b179d82c84e63b53ea46aa7b42ed41ac058b166748701330a4f69`. Exact 8.6.18 stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-distinct-unicode.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 8.6.18 stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 8.6.18 command-distinct-unicode. JSON pointer `/rows/82`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-distinct-unicode.tcl`; SHA256 `66bcf10ebbb6c73401d9be67c98e42f19f0051113ba725064a8afdf7f432600f`. Exact 9.0.4 input for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-distinct-unicode.stdout`; SHA256 `c4174cf7440b179d82c84e63b53ea46aa7b42ed41ac058b166748701330a4f69`. Exact 9.0.4 stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-distinct-unicode.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.0.4 stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.0.4 command-distinct-unicode. JSON pointer `/rows/122`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-distinct-unicode.tcl`; SHA256 `66bcf10ebbb6c73401d9be67c98e42f19f0051113ba725064a8afdf7f432600f`. Exact 9.1.0 input for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-distinct-unicode.stdout`; SHA256 `c4174cf7440b179d82c84e63b53ea46aa7b42ed41ac058b166748701330a4f69`. Exact 9.1.0 stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-distinct-unicode.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact 9.1.0 stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for 9.1.0 command-distinct-unicode. JSON pointer `/rows/162`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-distinct-unicode.tcl`; SHA256 `66bcf10ebbb6c73401d9be67c98e42f19f0051113ba725064a8afdf7f432600f`. Exact Jim input for command-distinct-unicode; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-distinct-unicode.stdout`; SHA256 `c4174cf7440b179d82c84e63b53ea46aa7b42ed41ac058b166748701330a4f69`. Exact Jim stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-distinct-unicode.stderr`; SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact Jim stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`; SHA256 `aeddfe8965468da893d3483c1e44069d7955531084076b27b0e9e7a9c5f22da6`. Original process/guest status and hashes for Jim command-distinct-unicode. JSON pointer `/rows/202`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case command-distinct-unicode --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
