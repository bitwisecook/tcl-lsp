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

These authored source receipts are separate from the original public Unicode command-identity experiment. They establish no native allocation, lookup/call completion, Unicode normalisation, object cache or frame, and attach no assertion or provider result.

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

A linked source constructor control preserves precomposed and decomposed class name bytes as distinct source slots and retains literal multiword Jim class names. Source reporting uses the genuine selected class name input rather than an ASCII label gate.

## Historical references

- `manifest`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original six-provider case rows; filter case=command-distinct-unicode.
- `versions`: `rust/tcl-syntax/tests/data/native_naming_corners/version-inventory.json`. Original reported interpreter identity.
- `controls`: `rust/tcl-syntax/tests/data/native_naming_corners/controls.json`. Authored control body for command-distinct-unicode; exact observer wrappers are separate provider inputs.
- `tcl8.4-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-distinct-unicode.tcl`. Exact 8.4.20 input for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.4-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-distinct-unicode.stdout`. Exact 8.4.20 stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.4-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.4.20/command-distinct-unicode.stderr`. Exact 8.4.20 stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.4-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.4.20 command-distinct-unicode. JSON pointer `/rows/2`.
- `tcl8.5-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-distinct-unicode.tcl`. Exact 8.5.19 input for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.5-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-distinct-unicode.stdout`. Exact 8.5.19 stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.5-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.5.19/command-distinct-unicode.stderr`. Exact 8.5.19 stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.5-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.5.19 command-distinct-unicode. JSON pointer `/rows/42`.
- `tcl8.6-input`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-distinct-unicode.tcl`. Exact 8.6.18 input for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.6-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-distinct-unicode.stdout`. Exact 8.6.18 stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.6-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/8.6.18/command-distinct-unicode.stderr`. Exact 8.6.18 stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl8.6-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 8.6.18 command-distinct-unicode. JSON pointer `/rows/82`.
- `tcl9.0-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-distinct-unicode.tcl`. Exact 9.0.4 input for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.0-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-distinct-unicode.stdout`. Exact 9.0.4 stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.0-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.0.4/command-distinct-unicode.stderr`. Exact 9.0.4 stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.0-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.0.4 command-distinct-unicode. JSON pointer `/rows/122`.
- `tcl9.1-input`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-distinct-unicode.tcl`. Exact 9.1.0 input for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.1-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-distinct-unicode.stdout`. Exact 9.1.0 stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.1-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/9.1.0/command-distinct-unicode.stderr`. Exact 9.1.0 stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `tcl9.1-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for 9.1.0 command-distinct-unicode. JSON pointer `/rows/162`.
- `jim-input`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-distinct-unicode.tcl`. Exact Jim input for command-distinct-unicode; source channel is the retained CLI script file.
- `jim-stdout`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-distinct-unicode.stdout`. Exact Jim stdout for command-distinct-unicode; source channel is the retained CLI script file.
- `jim-stderr`: `rust/tcl-syntax/tests/data/native_naming_corners/Jim/command-distinct-unicode.stderr`. Exact Jim stderr for command-distinct-unicode; source channel is the retained CLI script file.
- `jim-row`: `rust/tcl-syntax/tests/data/native_naming_corners/manifest.json`. Original process/guest status and hashes for Jim command-distinct-unicode. JSON pointer `/rows/202`.

## Source anchors and implementation tests

No interpreter-source excerpt or executed Rust test is attached to this question. The selected name-policy owner must retain each consumer purpose separately; this native script result cannot substitute for a Rust assertion.

- [rust/tcl-compiler/src/registry_invocation/source_structure.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_structure.rs), `source_constructor_call_at`: Join genuine source class declaration/factory call geometry while retaining exact producer name units independently of original runtime identity.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs), `command_binding::source_transition_advice::source_class::tests::original_native_class_names_keep_distinct_source_units_without_ascii_report_gate` (linked): Selected class source receipts preserve distinct UTF8 precomposed/decomposed name bytes and source slots without an ASCII report filter; a literal multiword Jim class name remains complete. Source constructor geometry does not execute or normalize either name.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

```text
python3 scripts/dev/replay-native-naming-corners.py --case command-distinct-unicode --provider <provider-id>=<exact-native-CLI-path> --library <provider-id>=<matching-native-library-directory> --output <new-independent-receipt.json>
```

The original executed script and its captured results are in [per-script evidence](../../../../evidence/tcl/native-naming-corner-command-distinct-unicode.md). A replay is a separate execution and must record its own results.
