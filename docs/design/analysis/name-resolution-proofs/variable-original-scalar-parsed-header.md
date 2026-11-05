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

Status: `observed`. Channel: Public Tcl_ObjGetVar2/Tcl_ObjSetVar2 on original counted String, Int, List and Bytearray names; Tcl_DuplicateObj and ASCII Tcl_EvalEx unset. Version: tcl8.4 8.4.20 (captured release association; launched patchlevel unqueried).

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

### tcl8.5, tcl8.6

Status: `observed`. Channel: Public Tcl_ObjGetVar2/Tcl_ObjSetVar2 on original counted String, Int, List and Bytearray names; Tcl_DuplicateObj and ASCII Tcl_EvalEx unset. Version: tcl8.5 8.5.19 (captured release association; launched patchlevel unqueried); tcl8.6 8.6.18 (captured release association; launched patchlevel unqueried).

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

### tcl9.0, tcl9.1

Status: `observed`. Channel: Public Tcl_ObjGetVar2/Tcl_ObjSetVar2 on original counted String, Int, List and Bytearray names; Tcl_DuplicateObj and ASCII Tcl_EvalEx unset. Version: tcl9.0 9.0.4 (captured release association; launched patchlevel unqueried); tcl9.1 9.1.0 (captured release association; launched patchlevel unqueried).

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

### jim, bigip

Status: `not-tested`.

No observation of this exact question is retained for this provider.

## Consumer bindings

- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeParsedVariableName`: Retains independently owned parser parts and original name/header metadata apart from variable table membership.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::original_parsed_headers_match_all_150_actual_c_windows` (linked): Compares exact primary/refcount/free-hook/resident bytes across the 150 snapshots; array-header observations do not establish table-key or child-object identity.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::original_parsed_headers_match_all_150_actual_c_windows` (linked): Compares exact primary/refcount/free-hook/resident bytes across the 150 snapshots; array-header observations do not establish table-key or child-object identity.

## Evidence

- [rust/tcl-syntax/tests/data/native_variable_name/parsed_headers.c](../../../../rust/tcl-syntax/tests/data/native_variable_name/parsed_headers.c)
- [rust/tcl-syntax/tests/data/native_variable_name/parsed_headers_provenance.json](../../../../rust/tcl-syntax/tests/data/native_variable_name/parsed_headers_provenance.json)
- [rust/tcl-syntax/tests/data/native_variable_name/parsed_headers.txt](../../../../rust/tcl-syntax/tests/data/native_variable_name/parsed_headers.txt)
