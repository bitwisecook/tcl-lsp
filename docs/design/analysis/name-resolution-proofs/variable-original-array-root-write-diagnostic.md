# naming.variable.original-array-root-write-diagnostic

Kind: `native-observation`

## Problem statement

An existing array root is a different variable receiver from an array element. Literal, dynamic and procedure source entries need their actual release-specific write result and errorCode; a read-failure tuple or presentation label cannot establish that boundary.

## Question

What catch result, errorCode and remaining array state follow the three fixed source entries that write an existing array root?

## Conclusion

C8.4.20 and C8.5.19 refuse all three writes with variable is array and errorCode NONE. C8.6.18, C9.0.4 and C9.1.0 refuse with the same result and TCL WRITE VARNAME; each array remains present. Current Jim succeeds with NEW, errorCode NONE and no remaining array. These eighteen source-file CLI results remain separate from original receiver identity, physical storage and compiler admission.

## Scope

One actual CLI interpreter per selected provider executes the same fixed ASCII LF source in literal, dynamic-command and procedure order; each form starts with a recreated array a containing k=OLD. Six separately printed patchlevels and eighteen complete result/error/state rows are retained. No arbitrary array names, opaque input, observers, physical cell/object/header/cache, original counted-argv ingress, compiler entry, Native Normal, pointer layout or BIG-IP answer is inferred.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: ASCII LF source-file CLI; three existing-array root variable write cases. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

All three writes return code1 with can't set "a": variable is array; array exists1. The recorded errorCode is NONE.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII LF source-file CLI; three existing-array root variable write cases. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All three writes return code1 with can't set "a": variable is array; array exists1. The recorded errorCode is TCL WRITE VARNAME.

### jim

Status: `observed`. Channel: ASCII LF source-file CLI; three existing-array root variable write cases. Version: jim 0.84-9-g5bac7c9.

All three writes return code0 and NEW, errorCode NONE and array exists0: the existing array becomes a scalar in these fixed source entries.

### bigip

Status: `not-tested`.

No BIG-IP attempt or corresponding source-file CLI answer is attached.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `native_variable_failure_code`: Select only the measured C ValueWrite/IsArray report tuple. NameLookup, Read, current receiver identity and physical storage remain independent.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::tests::c_array_root_write_failure_uses_the_measured_value_store_tuple` (linked): Rust-only site-table and mismatch coverage: the selected C ValueWrite/IsArray tuple retains actual NONE versus TCL WRITE VARNAME; Read and inappropriate NameLookup sites do not acquire this privilege. Native CLI outputs establish no private receiver layout.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::array_root_write_diagnostics_match_five_original_source_controls` (linked): Replay the identical complete ASCII source through an independently authentic C interpreter and compare all three original result/error/state rows. Only the separately identified process-version line is excluded; compiler/receiver/storage prerequisites are independent.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::array_root_write_preserves_current_jim_dictionary_replacement` (linked): Replay the same complete original Jim source and compare all three successful NEW/NONE/no-array rows; dictionary replacement is a finite source result, not a private physical-layout or cache observation.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::array_root_write_diagnostics_match_five_original_source_controls` (linked): Replay the identical complete ASCII source through an independently authentic C interpreter and compare all three original result/error/state rows. Only the separately identified process-version line is excluded; compiler/receiver/storage prerequisites are independent.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::array_root_write_preserves_current_jim_dictionary_replacement` (linked): Replay the same complete original Jim source and compare all three successful NEW/NONE/no-array rows; dictionary replacement is a finite source result, not a private physical-layout or cache observation.

## Evidence

- [rust/tcl-registry/tests/data/native_variable_array_write](../../../../rust/tcl-registry/tests/data/native_variable_array_write)
- [rust/tcl-registry/tests/data/native_variable_array_write/8.4.20](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.4.20)
- [rust/tcl-registry/tests/data/native_variable_array_write/8.5.19](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.5.19)
- [rust/tcl-registry/tests/data/native_variable_array_write/8.6.18](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.6.18)
- [rust/tcl-registry/tests/data/native_variable_array_write/9.0.4](../../../../rust/tcl-registry/tests/data/native_variable_array_write/9.0.4)
- [rust/tcl-registry/tests/data/native_variable_array_write/9.1.0](../../../../rust/tcl-registry/tests/data/native_variable_array_write/9.1.0)
- [rust/tcl-registry/tests/data/native_variable_array_write/jim](../../../../rust/tcl-registry/tests/data/native_variable_array_write/jim)
