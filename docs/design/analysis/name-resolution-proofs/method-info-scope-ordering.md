# naming.tcloo.method-info-scope-ordering

Kind: `native-observation`

## Problem statement

Runtime inspected only the first -scope and could retain recursive -all enumeration; VM ignored -scope and -localprivate. Option order changes which installed script-method names are listed.

## Question

Which measured local script-method membership results from repeated visibility options, repeated C9 scope, and -all before or after an explicit scope?

## Conclusion

In the fixed C8.6/C9 ASCII script-method setup, -localprivate reports no methods because no direct-instance-private C API methods were installed; a following -private restores ordinary local methods, and reversing the order reports none. C9 public/unexported/private scope selects Public/hidden/Secret for the class and corresponding object methods; repeated scope takes the last value. Explicit scope forces local enumeration even with later -all. Default -all adds inherited/destroy names. These outcomes do not establish arbitrary inheritance/export layering or physical result-name object identity.

## Scope

Same finite original counted/source vectors, with separate source setup of inherited/public/unexported script methods and C9 private method declarations. Local hash order is not an equivalence premise; tests compare sorted native list-member bytes for successful rows. No arbitrary C API private flags, original method headers/caches, opaque method name serialization, CPP or Normal grant.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `unsupported`. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

OO setup returned guest error on every case; method operation explicitly NOT_ATTEMPTED. This measures setup unavailability, no method selector/roster answer.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

In the fixed C8.6/C9 ASCII script-method setup, -localprivate reports no methods because no direct-instance-private C API methods were installed; a following -private restores ordinary local methods, and reversing the order reports none. C9 public/unexported/private scope selects Public/hidden/Secret for the class and corresponding object methods; repeated scope takes the last value. Explicit scope forces local enumeration even with later -all. Default -all adds inherited/destroy names. These outcomes do not establish arbitrary inheritance/export layering or physical result-name object identity.

### bigip

Status: `not-tested`.

No appliance observation.

## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_info/methods.rs](../../../../rust/tcl-registry/src/native_tcloo_info/methods.rs), `NativeTclooMethodInfoProtocol::parse_original`: Actual original-option sequential table selection after target lookup.
- [rust/tcl-vm/src/cmd_oo/native_method_info_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_info_tests.rs), `cmd_oo::native_method_info_tests::method_info_matches_original_counted_option_and_scope_controls` (linked): 150 direct supported-C code/result controls; successful list members compared as sorted exact native bytes, errors as exact bytes. Does not assert physical options/header/CPP/Normal.
- [runtime/rust/src/cmd_oo/native_method_info_tests.rs](../../../../runtime/rust/src/cmd_oo/native_method_info_tests.rs), `cmd_oo::native_method_info_tests::method_info_matches_original_counted_option_and_scope_controls` (linked): 150 direct supported-C code/result controls; successful list members compared as sorted exact native bytes, errors as exact bytes. Does not assert physical options/header/CPP/Normal.

## Evidence

- [rust/tcl-registry/tests/data/native_tcloo_method_info_original](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original)
- [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20)
- [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19)
- [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18)
- [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4)
- [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0)
- [rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim)
