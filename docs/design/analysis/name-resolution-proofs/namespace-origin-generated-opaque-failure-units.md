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

Status: `observed`. Version: 8.4.20; full launched patchlevel not independently queried by this capture. Build: Recorded executable SHA 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4; library/header/compiler/configuration closure unrecorded.. Channel: ASCII observer bytes supplied through shell stdin; operand produced at runtime by binary format. Dialect: C Tcl.

Exact final list value bytes are hex 31207b696e76616c696420636f6d6d616e64206e616d6520226d69737300ff227d204e4f4e45. The original shell writes that hex line with process0 and empty stderr. The list has guest code1, invalid-command result with miss00ff, and NONE errorCode.

### tcl8.5

Status: `observed`. Version: 8.5.19; full launched patchlevel not independently queried by this capture. Build: Recorded executable SHA 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; library/header/compiler/configuration closure unrecorded.. Channel: ASCII observer bytes supplied through shell stdin; operand produced at runtime by binary format. Dialect: C Tcl.

Exact final list value bytes are hex 31207b696e76616c696420636f6d6d616e64206e616d6520226d69737300ff227d207b54434c204c4f4f4b555020434f4d4d414e44206d69737300ff7d. The original shell writes that hex line with process0 and empty stderr. The list has guest code1, invalid-command result with miss00ff, and TCL LOOKUP COMMAND miss00ff errorCode.

### tcl8.6

Status: `observed`. Version: 8.6.18; full launched patchlevel not independently queried by this capture. Build: Recorded executable SHA 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0; library/header/compiler/configuration closure unrecorded.. Channel: ASCII observer bytes supplied through shell stdin; operand produced at runtime by binary format. Dialect: C Tcl.

Exact final list value bytes are hex 31207b696e76616c696420636f6d6d616e64206e616d6520226d69737300ff227d207b54434c204c4f4f4b555020434f4d4d414e44206d69737300ff7d. The original shell writes that hex line with process0 and empty stderr. The list has guest code1, invalid-command result with miss00ff, and TCL LOOKUP COMMAND miss00ff errorCode.

### tcl9.0

Status: `observed`. Version: 9.0.4; full launched patchlevel not independently queried by this capture. Build: Recorded executable SHA cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; library/header/compiler/configuration closure unrecorded.. Channel: ASCII observer bytes supplied through shell stdin; operand produced at runtime by binary format. Dialect: C Tcl.

Exact final list value bytes are hex 31207b696e76616c696420636f6d6d616e64206e616d6520226d69737300ff227d207b54434c204c4f4f4b555020434f4d4d414e44206d69737300ff7d. The original shell writes that hex line with process0 and empty stderr. The list has guest code1, invalid-command result with miss00ff, and TCL LOOKUP COMMAND miss00ff errorCode.

### tcl9.1

Status: `observed`. Version: 9.1.0; full launched patchlevel not independently queried by this capture. Build: Recorded executable SHA d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; library/header/compiler/configuration closure unrecorded.. Channel: ASCII observer bytes supplied through shell stdin; operand produced at runtime by binary format. Dialect: C Tcl.

Exact final list value bytes are hex 31207b696e76616c696420636f6d6d616e64206e616d6520226d69737300ff227d207b54434c204c4f4f4b555020434f4d4d414e44206d69737300ff7d. The original shell writes that hex line with process0 and empty stderr. The list has guest code1, invalid-command result with miss00ff, and TCL LOOKUP COMMAND miss00ff errorCode.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No captured result for this exact purpose is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No captured result for this exact purpose is attached for this provider.

## Exact evidence

- `source` (input): [runtime/rust/tests/data/native_namespace_origin_failures/source.tcl](../../../../runtime/rust/tests/data/native_namespace_origin_failures/source.tcl). SHA-256 `b17c5c05efa5cd7e252d755486b2d26c0de2aa51bb9744b3ae3fc6294c997b6c`. Exact binary-format original source.
- `observer` (input): [runtime/rust/tests/data/native_namespace_origin_failures/run.py](../../../../runtime/rust/tests/data/native_namespace_origin_failures/run.py). SHA-256 `14fd14a468631155ad0748fb4b8fa442114994912190e8a19399e7c1504f2df8`. Retained original wrapper/capture algorithm; reconstructed observer matches its recorded full digest.
- `receipt` (provider): [runtime/rust/tests/data/native_namespace_origin_failures/manifest.json](../../../../runtime/rust/tests/data/native_namespace_origin_failures/manifest.json). SHA-256 `7917d75a2fa66fd434f8a6cb762731b8042b3d9c18ce2dec7d80084bc4e2dca8`. Five exact stdin shell/source/wrapper/status/stream associations.
- `projection` (observation): [runtime/rust/tests/data/native_namespace_origin_failures/controls.tsv](../../../../runtime/rust/tests/data/native_namespace_origin_failures/controls.tsv). SHA-256 `85f53ec956998e3b390de992750962d92485863b605e60ccafa9ae1454a3b278`. Five exact final value-byte projections; not original input-object headers.
- `row-tcl8.4` (observation): [runtime/rust/tests/data/native_namespace_origin_failures/manifest.json](../../../../runtime/rust/tests/data/native_namespace_origin_failures/manifest.json). SHA-256 `7917d75a2fa66fd434f8a6cb762731b8042b3d9c18ce2dec7d80084bc4e2dca8`. JSON pointer `/0`. Actual process0/stdin result/errorCode byte projection.
- `row-tcl8.5` (observation): [runtime/rust/tests/data/native_namespace_origin_failures/manifest.json](../../../../runtime/rust/tests/data/native_namespace_origin_failures/manifest.json). SHA-256 `7917d75a2fa66fd434f8a6cb762731b8042b3d9c18ce2dec7d80084bc4e2dca8`. JSON pointer `/1`. Actual process0/stdin result/errorCode byte projection.
- `row-tcl8.6` (observation): [runtime/rust/tests/data/native_namespace_origin_failures/manifest.json](../../../../runtime/rust/tests/data/native_namespace_origin_failures/manifest.json). SHA-256 `7917d75a2fa66fd434f8a6cb762731b8042b3d9c18ce2dec7d80084bc4e2dca8`. JSON pointer `/2`. Actual process0/stdin result/errorCode byte projection.
- `row-tcl9.0` (observation): [runtime/rust/tests/data/native_namespace_origin_failures/manifest.json](../../../../runtime/rust/tests/data/native_namespace_origin_failures/manifest.json). SHA-256 `7917d75a2fa66fd434f8a6cb762731b8042b3d9c18ce2dec7d80084bc4e2dca8`. JSON pointer `/3`. Actual process0/stdin result/errorCode byte projection.
- `row-tcl9.1` (observation): [runtime/rust/tests/data/native_namespace_origin_failures/manifest.json](../../../../runtime/rust/tests/data/native_namespace_origin_failures/manifest.json). SHA-256 `7917d75a2fa66fd434f8a6cb762731b8042b3d9c18ce2dec7d80084bc4e2dca8`. JSON pointer `/4`. Actual process0/stdin result/errorCode byte projection.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_command_names.rs](../../../../runtime/rust/src/interp/native_command_names.rs), `native_namespace_origin_failure`: Retains the actual original failure operand units and selected error publication purpose independently of successful origin lookup/result issuance.
- [rust/tcl-vm/src/interp/native_command_names.rs](../../../../rust/tcl-vm/src/interp/native_command_names.rs), `native_namespace_origin_failure`: Retains the actual original failure operand units and selected error publication purpose independently of successful origin lookup/result issuance.
- [runtime/rust/src/interp/native_command_names.rs](../../../../runtime/rust/src/interp/native_command_names.rs), `interp::native_command_names::tests::native_namespace_origin_retains_string_result_birth_and_opaque_diagnostics` (linked): Evaluates the retained source and compares all five final result/errorCode projections. The separate source-only String origin-result assertion in this test is not a captured input/object-header observation.
- [rust/tcl-vm/src/interp/native_command_names.rs](../../../../rust/tcl-vm/src/interp/native_command_names.rs), `interp::native_command_names::tests::native_namespace_origin_retains_string_result_birth_and_opaque_diagnostics` (linked): Evaluates the retained source and compares all five final result/errorCode projections. The separate source-only String origin-result assertion in this test is not a captured input/object-header observation.

A named test is a coverage binding, not a claim that it executed.

## Replay

The retained run.py specifies the exact stdin wrapper, and its reconstructed bytes match every original observer digest. That script uses retained absolute shell paths and overwrites retained artifacts, so it is not advertised as a maintained safe replay. A new capture must use independently selected provider builds and a distinct output destination. No new native or Rust execution is claimed.
