# naming.lambda.original-coroutine-call-and-shadowing

Kind: `native-observation`

## Problem statement

Coroutine had a textual apply special case that bypassed the actual selected handler; serialized handoff could also replace the retained value across suspension.

## Question

Does native coroutine apply retain its original argument across yield and honor a shadowed apply handler?

## Conclusion

C8.6.18/9.0.4/9.1.0 return READY on suspension, then the same original argument pointer and bytes on resume for all seven values. Replacing apply with a source procedure makes coroutine call that handler and return SHADOW. C8.4/C8.5/current Jim report coroutine unavailable, and resume is explicitly not attempted.

## Scope

Seven original counted vectors per provider and one ASCII shadow source per provider.21 reached suspension/resume pairs and3 shadow controls. Unsupported providers are operation failures, not repaired setup. No arbitrary coroutine injection/frame introspection/trace/refcount/cache, complete options equivalence, custom callbacks or Normal certificate.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `unsupported`. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

Actual command unavailable; no entered body/resume claim.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

C8.6.18/9.0.4/9.1.0 return READY on suspension, then the same original argument pointer and bytes on resume for all seven values. Replacing apply with a source procedure makes coroutine call that handler and return SHADOW. C8.4/C8.5/current Jim report coroutine unavailable, and resume is explicitly not attempted.

### bigip

Status: `not-tested`.

No appliance observation.

## Consumer bindings

- [rust/tcl-vm/src/cmd_coro.rs](../../../../rust/tcl-vm/src/cmd_coro.rs), `cmd_coroutine`: Dispatches the original requested command operand and unchanged argv; textual apply recognition supplies no handler identity.
- [runtime/rust/src/cmd_coro.rs](../../../../runtime/rust/src/cmd_coro.rs), `coroutine_cmd`: Runs the selected original command and argv across the existing serialised coroutine handoff.
- [rust/tcl-vm/src/command/native_apply_original_tests.rs](../../../../rust/tcl-vm/src/command/native_apply_original_tests.rs), `command::native_apply_original_tests::apply_coroutine_keeps_arguments_and_selects_the_actual_shadowed_handler` (linked): 21 suspension starts,21 resumed original-object returns and three shadowed-apply handler controls for C8.6/C9.0/C9.1. Asserts code, pre-getter result identity and counted result bytes; no commandless cache, refcount or custom-release assertion. Runtime selector is non-WASM.
- [runtime/rust/src/cmd_proc/native_apply_original_tests.rs](../../../../runtime/rust/src/cmd_proc/native_apply_original_tests.rs), `cmd_proc::native_apply_original_tests::apply_coroutine_keeps_arguments_and_selects_the_actual_shadowed_handler` (linked): 21 suspension starts,21 resumed original-object returns and three shadowed-apply handler controls for C8.6/C9.0/C9.1. Asserts code, pre-getter result identity and counted result bytes; no commandless cache, refcount or custom-release assertion. Runtime selector is non-WASM.

## Evidence

- [rust/tcl-registry/tests/data/native_apply_original_argv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv)
- [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20)
- [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19)
- [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18)
- [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4)
- [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0)
- [rust/tcl-registry/tests/data/native_apply_original_argv/jim](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim)
