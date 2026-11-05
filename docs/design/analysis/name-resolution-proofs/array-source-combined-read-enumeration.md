# naming.array-source.combined-read-enumeration

Kind: `native-observation`

## Problem statement

A scalar element VALUE is installed before array get/names consumes a(b). Treating that indexed receiver as an ordinary whole-array name can suppress a selected dictionary conversion error.

## Question

What do array get and array names return for a(b) after the element holds the scalar VALUE?

## Conclusion

All five C captures return normal empty for array get a(b) and array names a(b). Jim returns missing value to go with key for both: its indexed receiver attempts the scalar VALUE as a dictionary/list representation. This finite odd-list conversion result supplies neither a general dictionary success nor a normal read certificate.

## Scope

Selected cases array-get-parens,array-names-parens from25 actual source programs per provider,150 total captured completions. Every case uses a new interpreter. Public Tcl_Eval or Jim_Eval consumes NUL-terminated ASCII source; binary-format/source-escape values are produced at runtime and are not raw zero source or independently held original native String objects. Result bytes are sampled after the guest completion through the actual public string getter. Source/header/library/executable and separate stdout/stderr hashes, compile/process0 and actual original command arrays are retained. C release associations and Jim git-describe tag are recorded, but this probe does not query a linked runtime patchlevel or object primary/refcount. BIG-IP is not tested; no Rust execution is inferred.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Version: tcl8.4 8.4.20 (recorded build association; tcl8.5 8.5.19 (recorded build association; tcl8.6 8.6.18 (recorded build association; tcl9.0 9.0.4 (recorded build association; tcl9.1 9.1.0 (recorded build association.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
array-get-parens	0	0	
array-names-parens	0	0	
```

Compile status0 and whole process0 are separate from the printed guest completions.

### jim

Status: `observed`. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Version: jim 0.84-9-g5bac7c9 (recorded build association.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
array-get-parens	1	28	6d697373696e672076616c756520746f20676f2077697468206b6579
array-names-parens	1	28	6d697373696e672076616c756520746f20676f2077697468206b6579
```

Compile status0 and whole process0 are separate from the printed guest completions.

### bigip

Status: `not-tested`.

No BIG-IP load/event or appliance-Tcl result for these exact programs is attached.

## Consumer bindings

- [rust/tcl-registry/src/native_variable_table.rs](../../../../rust/tcl-registry/src/native_variable_table.rs), `NativeVariableTableProtocol::dictionary_arrays`: Selects array versus dictionary container model; actual indexed receiver/read/write/observer currency remains an independent compiler/runtime obligation.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `resolved_invocation::selected_variable_receiver_form_keeps_array_argv_combined_and_aliases_separate` (linked): Selected C8.6 receiver metadata preserves the Combined original argv for scalar set and array set/get/names/unset a(b), while role-only and alias arguments cannot supply receiver form. It asserts no guest success, Normal transfer, physical root-only API or runtime cell identity.
- [rust/tcl-compiler/src/analyser/scope.rs](../../../../rust/tcl-compiler/src/analyser/scope.rs), `analyser::scope::tests::original_array_receiver_symbols_keep_combined_names_and_guest_error_independent` (linked): Five independently analysed absolute ::N::a(b) source programs preserve selected Combined receiver naming and source declaration roles without inferring guest success or later reachability from a prior failing array operation.

## Evidence

- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls)
- [scripts/dev](../../../../scripts/dev)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim)
