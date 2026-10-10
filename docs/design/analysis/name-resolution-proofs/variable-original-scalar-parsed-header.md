# naming.variable.original-scalar-parsed-header

Kind: `native-observation`

## Problem statement

Original String, integer, List and Bytearray names can retain a numeric/container primary on a missing read or acquire a parsed name on store. The name object reference count and its duplicate need not describe the same ownership.

## Question

Which scalar-name header states are reached across missing read, store, read, duplication and unset-followed-by-read?

## Conclusion

All stored/read scalar names have parsedVarName; C8.4 keeps original refs1, while later C keeps refs2 until the unset. Duplicates have refs1. C8.4 preserves an integer primary on the first missing read but retires List/Bytearray; C8.5/8.6 retire all three to an untyped primary. C9 preserves their initial primaries on the missing read. After unset, C8 retires the scalar parsed primary; C9 retains parsedVarName. These are header snapshots, not measured read/store normality or complete table ownership.

## Scope

150 original C observations. Fields are input kind, phase, primary, refcount, free-hook and resident bytes/hex. The observer never calls a string getter: only resident bytes are printed. Operation return codes/values are not emitted, so these header windows grant no successful read/write result. The exact eight-byte combined name has raw zero; no character-channel ingress. No Jim or BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (captured release association; launched patchlevel unqueried). Build: header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=4a36721ec9cb58c5d6a5ba5490104d1fe6504947019393d61ff245e5b922cf33; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_ObjGetVar2/Tcl_ObjSetVar2 on original counted String, Int, List and Bytearray names; Tcl_DuplicateObj and ASCII Tcl_EvalEx unset.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
string|before|none|1|0|1|6d697373696e67
string|missing|none|1|0|1|6d697373696e67
string|stored|parsedVarName|1|1|1|6d697373696e67
string|read|parsedVarName|1|1|1|6d697373696e67
string|duplicate|parsedVarName|1|1|1|6d697373696e67
string|after-unset-missing|none|1|0|1|6d697373696e67
int|before|int|1|0|0|
int|missing|int|1|0|1|3432
int|stored|parsedVarName|1|1|1|3432
int|read|parsedVarName|1|1|1|3432
int|duplicate|parsedVarName|1|1|1|3432
int|after-unset-missing|none|1|0|1|3432
list|before|list|1|1|0|
list|missing|none|1|0|1|6d697373696e67
list|stored|parsedVarName|1|1|1|6d697373696e67
list|read|parsedVarName|1|1|1|6d697373696e67
list|duplicate|parsedVarName|1|1|1|6d697373696e67
list|after-unset-missing|none|1|0|1|6d697373696e67
bytearray|before|bytearray|1|1|0|
bytearray|missing|none|1|0|1|6d697373696e67
bytearray|stored|parsedVarName|1|1|1|6d697373696e67
bytearray|read|parsedVarName|1|1|1|6d697373696e67
bytearray|duplicate|parsedVarName|1|1|1|6d697373696e67
bytearray|after-unset-missing|none|1|0|1|6d697373696e67
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19 (captured release association; launched patchlevel unqueried). Build: header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=39e29882399f9b86abb9d72cd90623d6af44630a04afa9333977984aaab082be; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_ObjGetVar2/Tcl_ObjSetVar2 on original counted String, Int, List and Bytearray names; Tcl_DuplicateObj and ASCII Tcl_EvalEx unset.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
string|before|none|1|0|1|6d697373696e67
string|missing|none|1|0|1|6d697373696e67
string|stored|parsedVarName|2|1|1|6d697373696e67
string|read|parsedVarName|2|1|1|6d697373696e67
string|duplicate|parsedVarName|1|1|1|6d697373696e67
string|after-unset-missing|none|1|0|1|6d697373696e67
int|before|int|1|0|0|
int|missing|none|1|0|1|3432
int|stored|parsedVarName|2|1|1|3432
int|read|parsedVarName|2|1|1|3432
int|duplicate|parsedVarName|1|1|1|3432
int|after-unset-missing|none|1|0|1|3432
list|before|list|1|1|0|
list|missing|none|1|0|1|6d697373696e67
list|stored|parsedVarName|2|1|1|6d697373696e67
list|read|parsedVarName|2|1|1|6d697373696e67
list|duplicate|parsedVarName|1|1|1|6d697373696e67
list|after-unset-missing|none|1|0|1|6d697373696e67
bytearray|before|bytearray|1|1|0|
bytearray|missing|none|1|0|1|6d697373696e67
bytearray|stored|parsedVarName|2|1|1|6d697373696e67
bytearray|read|parsedVarName|2|1|1|6d697373696e67
bytearray|duplicate|parsedVarName|1|1|1|6d697373696e67
bytearray|after-unset-missing|none|1|0|1|6d697373696e67
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18 (captured release association; launched patchlevel unqueried). Build: header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=3687fb9bf24fb73bb96470a6b8f54bf742944a316014ef19955762b56059dfee; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_ObjGetVar2/Tcl_ObjSetVar2 on original counted String, Int, List and Bytearray names; Tcl_DuplicateObj and ASCII Tcl_EvalEx unset.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
string|before|none|1|0|1|6d697373696e67
string|missing|none|1|0|1|6d697373696e67
string|stored|parsedVarName|2|1|1|6d697373696e67
string|read|parsedVarName|2|1|1|6d697373696e67
string|duplicate|parsedVarName|1|1|1|6d697373696e67
string|after-unset-missing|none|1|0|1|6d697373696e67
int|before|int|1|0|0|
int|missing|none|1|0|1|3432
int|stored|parsedVarName|2|1|1|3432
int|read|parsedVarName|2|1|1|3432
int|duplicate|parsedVarName|1|1|1|3432
int|after-unset-missing|none|1|0|1|3432
list|before|list|1|1|0|
list|missing|none|1|0|1|6d697373696e67
list|stored|parsedVarName|2|1|1|6d697373696e67
list|read|parsedVarName|2|1|1|6d697373696e67
list|duplicate|parsedVarName|1|1|1|6d697373696e67
list|after-unset-missing|none|1|0|1|6d697373696e67
bytearray|before|bytearray|1|1|0|
bytearray|missing|none|1|0|1|6d697373696e67
bytearray|stored|parsedVarName|2|1|1|6d697373696e67
bytearray|read|parsedVarName|2|1|1|6d697373696e67
bytearray|duplicate|parsedVarName|1|1|1|6d697373696e67
bytearray|after-unset-missing|none|1|0|1|6d697373696e67
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4 (captured release association; launched patchlevel unqueried). Build: header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=1ede2b6aa265354a8ad549de9c2e7b1b067172820f1d5482e72d7bfe17250793; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_ObjGetVar2/Tcl_ObjSetVar2 on original counted String, Int, List and Bytearray names; Tcl_DuplicateObj and ASCII Tcl_EvalEx unset.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
string|before|none|1|0|1|6d697373696e67
string|missing|none|1|0|1|6d697373696e67
string|stored|parsedVarName|2|1|1|6d697373696e67
string|read|parsedVarName|2|1|1|6d697373696e67
string|duplicate|parsedVarName|1|1|1|6d697373696e67
string|after-unset-missing|parsedVarName|1|1|1|6d697373696e67
int|before|int|1|0|0|
int|missing|int|1|0|1|3432
int|stored|parsedVarName|2|1|1|3432
int|read|parsedVarName|2|1|1|3432
int|duplicate|parsedVarName|1|1|1|3432
int|after-unset-missing|parsedVarName|1|1|1|3432
list|before|list|1|1|0|
list|missing|list|1|1|1|6d697373696e67
list|stored|parsedVarName|2|1|1|6d697373696e67
list|read|parsedVarName|2|1|1|6d697373696e67
list|duplicate|parsedVarName|1|1|1|6d697373696e67
list|after-unset-missing|parsedVarName|1|1|1|6d697373696e67
bytearray|before|bytearray|1|1|0|
bytearray|missing|bytearray|1|1|1|6d697373696e67
bytearray|stored|parsedVarName|2|1|1|6d697373696e67
bytearray|read|parsedVarName|2|1|1|6d697373696e67
bytearray|duplicate|parsedVarName|1|1|1|6d697373696e67
bytearray|after-unset-missing|parsedVarName|1|1|1|6d697373696e67
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0 (captured release association; launched patchlevel unqueried). Build: header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=927411405c508cdc276aef5bf83f742cf6b4e8ff820c090b0827345ad6d990ce; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_ObjGetVar2/Tcl_ObjSetVar2 on original counted String, Int, List and Bytearray names; Tcl_DuplicateObj and ASCII Tcl_EvalEx unset.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
string|before|none|1|0|1|6d697373696e67
string|missing|none|1|0|1|6d697373696e67
string|stored|parsedVarName|2|1|1|6d697373696e67
string|read|parsedVarName|2|1|1|6d697373696e67
string|duplicate|parsedVarName|1|1|1|6d697373696e67
string|after-unset-missing|parsedVarName|1|1|1|6d697373696e67
int|before|int|1|0|0|
int|missing|int|1|0|1|3432
int|stored|parsedVarName|2|1|1|3432
int|read|parsedVarName|2|1|1|3432
int|duplicate|parsedVarName|1|1|1|3432
int|after-unset-missing|parsedVarName|1|1|1|3432
list|before|list|1|1|0|
list|missing|list|1|1|1|6d697373696e67
list|stored|parsedVarName|2|1|1|6d697373696e67
list|read|parsedVarName|2|1|1|6d697373696e67
list|duplicate|parsedVarName|1|1|1|6d697373696e67
list|after-unset-missing|parsedVarName|1|1|1|6d697373696e67
bytearray|before|bytearray|1|1|0|
bytearray|missing|bytearray|1|1|1|6d697373696e67
bytearray|stored|parsedVarName|2|1|1|6d697373696e67
bytearray|read|parsedVarName|2|1|1|6d697373696e67
bytearray|duplicate|parsedVarName|1|1|1|6d697373696e67
bytearray|after-unset-missing|parsedVarName|1|1|1|6d697373696e67
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No observation of this exact question is retained for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No observation of this exact question is retained for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_variable_name/parsed_headers.c](../../../../rust/tcl-syntax/tests/data/native_variable_name/parsed_headers.c). SHA-256 `96c7bbd1714d9c8bc7318a7a87ffc7d394fefca1892162b73ace95bfdbdcd7c3`. Exact original public/private observer and declared reference ownership.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_variable_name/parsed_headers_provenance.json](../../../../rust/tcl-syntax/tests/data/native_variable_name/parsed_headers_provenance.json). SHA-256 `aa61e180b37b57b4fda6794ea6d6c866a50d3954e52cc56eb3f72edbac1e86f5`. Original independently attributed compilation and process captures; inline observations preserve full raw outcomes.
- `rows` (observation): [rust/tcl-syntax/tests/data/native_variable_name/parsed_headers.txt](../../../../rust/tcl-syntax/tests/data/native_variable_name/parsed_headers.txt). SHA-256 `829a739abfcad3bd6df72b1e06d36d2a12e41ab128969a5aedf424b8974f0aef`. Exact checked per-provider projection. Selection is specified in the provider answer; omitted call lines remain in the receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeParsedVariableName`: Retains independently owned parser parts and original name/header metadata apart from variable table membership.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::original_parsed_headers_match_all_150_actual_c_windows` (linked): Compares exact primary/refcount/free-hook/resident bytes across the 150 snapshots; array-header observations do not establish table-key or child-object identity.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::original_parsed_headers_match_all_150_actual_c_windows` (linked): Compares exact primary/refcount/free-hook/resident bytes across the 150 snapshots; array-header observations do not establish table-key or child-object identity.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test result is asserted. A new comparison must compile the exact retained probe against independently identified release headers and archive, preserve its original API/flags and declared observer references, then capture separate process status, stdout and stderr. Original absolute compiler paths are attribution metadata, not a portable replay command. The probe is input source, not an excerpt of the Tcl/Jim implementation.
