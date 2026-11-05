# naming.namespace.origin-generated-opaque-failure-units

Kind: `native-observation`

## Problem statement

A binary-format-produced missing command name contains zero and ff units. A namespace-origin consumer could truncate or sanitize those units when reporting an invalid command and its errorCode, even though no successful origin exists.

## Question

Which invalid-command result and errorCode units does namespace origin preserve for the generated miss00ff operand?

## Conclusion

All five C captures preserve miss00ff in the invalid-command result. C8.4 reports errorCode NONE; C8.5–9.1 report TCL LOOKUP COMMAND with the same generated miss00ff operand. The capture observes final result/errorCode byte projections after the catch, not the operand primary, command token, native rawString00 producer or successful origin allocation.

## Scope

One ASCII source produces the operand through binary format H*, catches namespace origin and returns a list of code/result/errorCode. The retained observer wraps that exact script in set observed, binary scan and puts, then feeds stdin to a selected shell. Source and wrapper complete hashes, executable digests, process0 and exact double-encoded output are retained. C release labels are recorded, launched patchlevel/library/compiler/configuration unqueried; no Jim/BIG-IP attempt. Generated zero units do not imply raw NUL source or an original native String object.

## Provider answers

### tcl8.4

Status: `observed`. Channel: ASCII observer bytes supplied through shell stdin; operand produced at runtime by binary format. Version: tcl8.4 8.4.20; full launched patchlevel not independently queried by this capture.

Exact final list value bytes are hex 31207b696e76616c696420636f6d6d616e64206e616d6520226d69737300ff227d204e4f4e45. The original shell writes that hex line with process0 and empty stderr. The list has guest code1, invalid-command result with miss00ff, and NONE errorCode.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII observer bytes supplied through shell stdin; operand produced at runtime by binary format. Version: tcl8.5 8.5.19; full launched patchlevel not independently queried by this capture; tcl8.6 8.6.18; full launched patchlevel not independently queried by this capture; tcl9.0 9.0.4; full launched patchlevel not independently queried by this capture; tcl9.1 9.1.0; full launched patchlevel not independently queried by this capture.

Exact final list value bytes are hex 31207b696e76616c696420636f6d6d616e64206e616d6520226d69737300ff227d207b54434c204c4f4f4b555020434f4d4d414e44206d69737300ff7d. The original shell writes that hex line with process0 and empty stderr. The list has guest code1, invalid-command result with miss00ff, and TCL LOOKUP COMMAND miss00ff errorCode.

### jim, bigip

Status: `not-tested`.

No captured result for this exact purpose is attached for this provider.

## Consumer bindings

- [runtime/rust/src/interp/native_command_names.rs](../../../../runtime/rust/src/interp/native_command_names.rs), `native_namespace_origin_failure`: Retains the actual original failure operand units and selected error publication purpose independently of successful origin lookup/result issuance.
- [rust/tcl-vm/src/interp/native_command_names.rs](../../../../rust/tcl-vm/src/interp/native_command_names.rs), `native_namespace_origin_failure`: Retains the actual original failure operand units and selected error publication purpose independently of successful origin lookup/result issuance.
- [runtime/rust/src/interp/native_command_names.rs](../../../../runtime/rust/src/interp/native_command_names.rs), `interp::native_command_names::tests::native_namespace_origin_retains_string_result_birth_and_opaque_diagnostics` (linked): Evaluates the retained source and compares all five final result/errorCode projections. The separate source-only String origin-result assertion in this test is not a captured input/object-header observation.
- [rust/tcl-vm/src/interp/native_command_names.rs](../../../../rust/tcl-vm/src/interp/native_command_names.rs), `interp::native_command_names::tests::native_namespace_origin_retains_string_result_birth_and_opaque_diagnostics` (linked): Evaluates the retained source and compares all five final result/errorCode projections. The separate source-only String origin-result assertion in this test is not a captured input/object-header observation.

## Evidence

- [runtime/rust/tests/data/native_namespace_origin_failures/source.tcl](../../../../runtime/rust/tests/data/native_namespace_origin_failures/source.tcl)
- [runtime/rust/tests/data/native_namespace_origin_failures/manifest.json](../../../../runtime/rust/tests/data/native_namespace_origin_failures/manifest.json)
- [runtime/rust/tests/data/native_namespace_origin_failures/controls.tsv](../../../../runtime/rust/tests/data/native_namespace_origin_failures/controls.tsv)
