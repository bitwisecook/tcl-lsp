# Does this Jim alias prefix call the replacement target after rename and redefinition?

Proof ID: `naming.corner.alias-jim-prefix-rebind`

## Problem statement

A Jim alias prefix targets a procedure that is renamed and replaced. Its creation grammar and target retention cannot be inferred from C interp alias, even when both aliases have similar reporting names. Applicability is limited to the retained script, selected native build, startup environment and reported results; other input channels and BIG-IP require independent evidence.

## Question

Does this Jim alias prefix call the replacement target after rename and redefinition?

## Exact control

```tcl
proc target args {return OLD}; alias a target PREFIX; rename target original; proc target args {return NEW}; a ARG
```

## Scope

The six original provider scripts, process statuses, caught guest completion codes and binary-scan result hex are retained independently. Binary-scan output is a script result observation, not a physical native UTF storage or object-header window. No raw00 source ingress, return options, refcounts, compiler preparation, physical namespace/frame/object identity, executed Rust correspondence or BIG-IP claim is made.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022616c69617322; result rendering "invalid command name \"alias\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.5 8.5.19 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022616c69617322; result rendering "invalid command name \"alias\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl8.6 8.6.18 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022616c69617322; result rendering "invalid command name \"alias\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl9.0 9.0.4 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022616c69617322; result rendering "invalid command name \"alias\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| tcl9.1 9.1.0 | unsupported | Process exit 0; guest completion 1; captured result hex 696e76616c696420636f6d6d616e64206e616d652022616c69617322; result rendering "invalid command name \"alias\"". The precise command/option creation door is unavailable in this capture; its error remains data. |
| jim 0.84-9-g5bac7c9 (commit 5bac7c99ad65864c87da513e22e2f01703fa4e03) | observed | Process exit 0; guest completion 0; captured result hex 4e4557; result rendering "NEW". This answer applies only to this exact script and captured startup environment. |
| bigip not recorded | not-tested | No appliance execution of this precise question is attached. Stock Tcl and Jim results provide no BIG-IP applicability. |

## Conclusion

Jim returns NEW; C8.4–9.1 retain the missing alias command errors. This creation door is distinct from the C interp alias control.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=alias-jim-prefix-rebind.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity and retained selected binary SHA256.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for alias-jim-prefix-rebind; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/alias-jim-prefix-rebind.tcl`. Exact 8.4.20 input for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/alias-jim-prefix-rebind.stdout`. Exact 8.4.20 stdout for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/alias-jim-prefix-rebind.stderr`. Exact 8.4.20 stderr for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 alias-jim-prefix-rebind. JSON pointer `/rows/13`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/alias-jim-prefix-rebind.tcl`. Exact 8.5.19 input for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/alias-jim-prefix-rebind.stdout`. Exact 8.5.19 stdout for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/alias-jim-prefix-rebind.stderr`. Exact 8.5.19 stderr for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 alias-jim-prefix-rebind. JSON pointer `/rows/53`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/alias-jim-prefix-rebind.tcl`. Exact 8.6.18 input for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/alias-jim-prefix-rebind.stdout`. Exact 8.6.18 stdout for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/alias-jim-prefix-rebind.stderr`. Exact 8.6.18 stderr for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 alias-jim-prefix-rebind. JSON pointer `/rows/93`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/alias-jim-prefix-rebind.tcl`. Exact 9.0.4 input for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/alias-jim-prefix-rebind.stdout`. Exact 9.0.4 stdout for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/alias-jim-prefix-rebind.stderr`. Exact 9.0.4 stderr for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 alias-jim-prefix-rebind. JSON pointer `/rows/133`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/alias-jim-prefix-rebind.tcl`. Exact 9.1.0 input for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/alias-jim-prefix-rebind.stdout`. Exact 9.1.0 stdout for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/alias-jim-prefix-rebind.stderr`. Exact 9.1.0 stderr for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 alias-jim-prefix-rebind. JSON pointer `/rows/173`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/alias-jim-prefix-rebind.tcl`. Exact Jim input for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/alias-jim-prefix-rebind.stdout`. Exact Jim stdout for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/alias-jim-prefix-rebind.stderr`. Exact Jim stderr for alias-jim-prefix-rebind; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim alias-jim-prefix-rebind. JSON pointer `/rows/213`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case alias-jim-prefix-rebind --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The replayer validates retained source and stream hashes, requires the exact recorded executable SHA256, and separately checks its identity script. Supply each tested provider explicitly and matching native startup libraries. The original library tree hashes were not captured; fresh startup configuration is recorded and exact result comparison remains required. Replay results are a new receipt; none is claimed executed by this proof page.
