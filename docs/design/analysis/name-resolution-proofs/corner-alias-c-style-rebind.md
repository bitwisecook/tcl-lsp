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

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=alias-c-style-rebind.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for alias-c-style-rebind; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/alias-c-style-rebind.tcl`. Exact 8.4.20 input for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/alias-c-style-rebind.stdout`. Exact 8.4.20 stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/alias-c-style-rebind.stderr`. Exact 8.4.20 stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 alias-c-style-rebind. JSON pointer `/rows/12`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/alias-c-style-rebind.tcl`. Exact 8.5.19 input for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/alias-c-style-rebind.stdout`. Exact 8.5.19 stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/alias-c-style-rebind.stderr`. Exact 8.5.19 stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 alias-c-style-rebind. JSON pointer `/rows/52`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/alias-c-style-rebind.tcl`. Exact 8.6.18 input for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/alias-c-style-rebind.stdout`. Exact 8.6.18 stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/alias-c-style-rebind.stderr`. Exact 8.6.18 stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 alias-c-style-rebind. JSON pointer `/rows/92`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/alias-c-style-rebind.tcl`. Exact 9.0.4 input for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/alias-c-style-rebind.stdout`. Exact 9.0.4 stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/alias-c-style-rebind.stderr`. Exact 9.0.4 stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 alias-c-style-rebind. JSON pointer `/rows/132`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/alias-c-style-rebind.tcl`. Exact 9.1.0 input for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/alias-c-style-rebind.stdout`. Exact 9.1.0 stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/alias-c-style-rebind.stderr`. Exact 9.1.0 stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 alias-c-style-rebind. JSON pointer `/rows/172`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/alias-c-style-rebind.tcl`. Exact Jim input for alias-c-style-rebind; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/alias-c-style-rebind.stdout`. Exact Jim stdout for alias-c-style-rebind; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/alias-c-style-rebind.stderr`. Exact Jim stderr for alias-c-style-rebind; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim alias-c-style-rebind. JSON pointer `/rows/212`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case alias-c-style-rebind --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The original executed script and its captured results are in [per-script evidence](../../../../evidence/tcl/native-naming-corner-alias-c-style-rebind.md). A replay is a separate execution and must record its own results.
