# naming.variable.original-counted-array-parsed-header

Kind: `native-observation`

## Problem statement

A combined name containing an original raw zero index has counted storage independently from the table matcher. A parsed-name header may survive an absent root even when scalar parsed headers retire.

## Question

Does arr(k raw-zero z) keep its exact combined bytes and parsed primary across read/store/copy and root removal?

## Conclusion

Every tested C capture retains the exact eight combined bytes 61 72 72 28 6B 00 7A 29 and reports parsedVarName at the first missing read through the final missing read. Original and duplicate refs remain1 with a free hook and resident bytes. This does not measure the selected table key, successful element membership or independent root/index child identity.

## Scope

150 original C observations. Fields are input kind, phase, primary, refcount, free-hook and resident bytes/hex. The observer never calls a string getter: only resident bytes are printed. Operation return codes/values are not emitted, so these header windows grant no successful read/write result. The exact eight-byte combined name has raw zero; no character-channel ingress. No Jim or BIG-IP attempt.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public Tcl_ObjGetVar2/Tcl_ObjSetVar2 on original counted String, Int, List and Bytearray names; Tcl_DuplicateObj and ASCII Tcl_EvalEx unset. Version: tcl8.4 8.4.20 (captured release association; launched patchlevel unqueried); tcl8.5 8.5.19 (captured release association; launched patchlevel unqueried); tcl8.6 8.6.18 (captured release association; launched patchlevel unqueried); tcl9.0 9.0.4 (captured release association; launched patchlevel unqueried); tcl9.1 9.1.0 (captured release association; launched patchlevel unqueried).

Exact selected observer rows, in original column order:

```text
array|before|none|1|0|1|617272286b007a29
array|missing|parsedVarName|1|1|1|617272286b007a29
array|stored|parsedVarName|1|1|1|617272286b007a29
array|read|parsedVarName|1|1|1|617272286b007a29
array|duplicate|parsedVarName|1|1|1|617272286b007a29
array|after-unset-missing|parsedVarName|1|1|1|617272286b007a29
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
