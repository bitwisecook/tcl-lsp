# naming.lambda.original-argument-object-return

Kind: `native-observation`

## Problem statement

VM apply serialized caller arguments into source words, replacing original object identity and requiring Unicode of opaque original values.

## Question

Does apply of a whole-object return lambda retain the original argument object and bytes?

## Conclusion

C8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9 return the exact original argument pointer for all seven measured counted string values. The pointer comparison precedes the only result getter. Raw zero, encoded C080, FF, D800 and supplementary bytes remain their separate original values. C8.4 reports apply unavailable.

## Scope

Seven ordinary original counted string-object arguments with lambda {x} {return $x}:35 successful direct controls plus7 unavailable C84 controls. No custom object hooks, type/header/refcount/cache, commandless Proc storage, compiler admission, general completion equivalence, namespace or Document/source ingress claim.

## Provider answers

### tcl8.4

Status: `unsupported`. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried. Version: tcl8.4 8.4.20.

Actual command unavailable; no entered body/resume claim.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

C8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9 return the exact original argument pointer for all seven measured counted string values. The pointer comparison precedes the only result getter. Raw zero, encoded C080, FF, D800 and supplementary bytes remain their separate original values. C8.4 reports apply unavailable.

### bigip

Status: `not-tested`.

No appliance observation.

## Consumer bindings

- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `Vm::defer_original_lambda_call`: Retains the original invoked operand and argv objects for the existing independently admitted procedure activation; no source reserialization.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `apply_cmd`: Selects the actual lambda object and original argument vector at the runtime handler boundary.
- [rust/tcl-vm/src/command/native_apply_original_tests.rs](../../../../rust/tcl-vm/src/command/native_apply_original_tests.rs), `command::native_apply_original_tests::apply_returns_original_counted_argument_objects` (linked): 35 direct original-object code, pre-getter identity and counted result-byte comparisons for C8.5/C8.6/C9.0/C9.1/Jim; C8.4 unsupported remains outside this selector. No refcount, custom callback, cache, header or completion-options assertion.
- [runtime/rust/src/cmd_proc/native_apply_original_tests.rs](../../../../runtime/rust/src/cmd_proc/native_apply_original_tests.rs), `cmd_proc::native_apply_original_tests::apply_returns_original_counted_argument_objects` (linked): 35 direct original-object code, pre-getter identity and counted result-byte comparisons for C8.5/C8.6/C9.0/C9.1/Jim; C8.4 unsupported remains outside this selector. No refcount, custom callback, cache, header or completion-options assertion.

## Evidence

- [rust/tcl-registry/tests/data/native_apply_original_argv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv)
- [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20)
- [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19)
- [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18)
- [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4)
- [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0)
- [rust/tcl-registry/tests/data/native_apply_original_argv/jim](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim)
