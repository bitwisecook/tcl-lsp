# naming.formal.raw-zero-list-and-binding-renaming

Kind: `native-observation`

## Problem statement

A raw counted zero formal can be clipped by one formal-list or storage purpose while its compiled body read retains a different name. Replacing that formal and read with a can turn a failing original procedure into a successful one. Encoded C080 and raw zero must be separate controls; declaration acceptance alone is insufficient to authorize a rename.

## Question

Do matching raw-zero formal/body operands behave like their a rename, and how do the separate encoded-C080 controls compare?

## Conclusion

C8.4/C8.5 accept the raw-zero declaration but each original body call fails with can't read "long": no such variable, while a succeeds and returns the held argument object. C8.6/C9.0/C9.1/Jim return that original object in both forms. The separate C080 formal succeeds in all six. This bounds rename suitability by the actual formal-list, storage and read purposes; no raw-zero name equivalence is inferred from acceptance or display.

## Scope

Exact original formal bytes long<raw00>tail and body return ${long<raw00>tail}, contrasted with a/return $a and long<C080>tail. Six ordinary counted native String argv inputs; original calls and diagnostics are observed rather than inferred from any internal table.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Raw-zero declaration succeeds; original calls fail with code1 and exact missing-long diagnostic, while renamed calls return code0/same argv/bytes. C080 original and renamed calls succeed.

### tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

Raw-zero and separate C080 original/renamed calls return code0/the same held argv object/exact bytes across all six argument inputs.

### bigip

Status: `not-tested`.

No appliance capture for this exact counted formal/body/argv question.

## Consumer bindings

- [rust/tcl-vm/src/command/native_formal_alpha_tests.rs](../../../../rust/tcl-vm/src/command/native_formal_alpha_tests.rs), `command::native_formal_alpha_tests::counted_formal_renaming_keeps_whole_object_return_and_binding_boundaries` (linked): Compares original counted proc/formal/body/value operands, guest codes, original result identity before getter and bytes for the six independently selected native VM contexts.
- [runtime/rust/src/cmd_proc/native_formal_alpha_tests.rs](../../../../runtime/rust/src/cmd_proc/native_formal_alpha_tests.rs), `cmd_proc::native_formal_alpha_tests::counted_formal_renaming_keeps_whole_object_return_and_binding_boundaries` (linked): Compares the exact original proc/formal/body/value operands and retained result pointer/bytes in the separately selected Runtime interpreter contexts. No execution result is inferred from this binding.
- [rust/tcl-compiler/src/command_binding/scalar_body_alpha.rs](../../../../rust/tcl-compiler/src/command_binding/scalar_body_alpha.rs), `command_binding::scalar_body_alpha::tests::original_scalar_alpha_raw_zero_requires_selected_formal_and_body_correspondence` (linked): Authentic NativeValue formal topology and body-root correspondence distinguish the observed C84/C85 raw-zero installation mismatch from C86/C9/Jim correspondence. The independently tested Document C080 projection is Rust model correspondence, outside the empirical counted-object observation.

## Evidence

- [rust/tcl-vm/tests/data/native_formal_alpha](../../../../rust/tcl-vm/tests/data/native_formal_alpha)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/jim](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/jim)
- [rust/tcl-vm/tests/data/native_formal_alpha/v1](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v1)
