# naming.tcloo.method-info-option-selection

Kind: `native-observation`

## Problem statement

Method introspection must select original option objects under the provider-specific counted or prefix recipe and preserve the handler error order. Display-string decoding or comparison over an unrelated full extent can conflate raw zero, encoded zero and different option-table matches. Missing targets and malformed options require separate stage evidence.

## Question

How do original counted C TclOO method options select prefixes, raw zero and encoded C080, and does target lookup precede option parsing?

## Conclusion

For C8.6.18/9.0.4/9.1.0, the measured direct and counted-source options use CString unique-prefix Index selection: -private with raw-zero suffix succeeds, encoded C080 suffix fails, and the one-character - prefix is ambiguous. Missing target lookup fails before bad-option or missing-scope validation. C8.6 rejects -scope. C8.4/C8.5/Jim fail OO setup and do not reach the measured method operation.

## Scope

25 fixed cases x object/class x direct counted original string-vector or counted source; 100 controls per provider, 300 reached controls on three C providers and 300 explicitly not-attempted setup controls. No Document/read-channel, custom object hook, cache/header/Index identity, method allocation, worker CPP, callback, normal-completion or optimizer assertion.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `unsupported`. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

OO setup returned guest error on every case; method operation explicitly NOT_ATTEMPTED. This measures setup unavailability, no method selector/roster answer.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

For C8.6.18/9.0.4/9.1.0, the measured direct and counted-source options use CString unique-prefix Index selection: -private with raw-zero suffix succeeds, encoded C080 suffix fails, and the one-character - prefix is ambiguous. Missing target lookup fails before bad-option or missing-scope validation. C8.6 rejects -scope. C8.4/C8.5/Jim fail OO setup and do not reach the measured method operation.

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
