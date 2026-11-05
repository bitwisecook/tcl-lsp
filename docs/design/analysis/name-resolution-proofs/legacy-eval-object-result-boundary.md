# naming.interpreter.legacy-eval-object-result-boundary

Kind: `native-observation`

## Problem statement

After a raw counted-zero variable name is reported through info object vars, a shortened list element can mean either that storage clipped the key or that the evaluator changed the result representation. Comparing Tcl_Eval results with object-vector dispatch would confuse those interpretations. This check compares the same reporter through three public evaluation APIs; it applies to the captured TclOO cases and does not attest unknown object getters or resident key identity.

## Question

For the captured counted TclOO variable names, does Tcl_Eval followed by Tcl_GetObjResult preserve the same reported list-element bytes as Tcl_EvalEx and Tcl_EvalObjv?

## Conclusion

C8.6 reports raw-zero k through the legacy Tcl_Eval result boundary but retains full k<00>tail through Tcl_EvalEx and Tcl_EvalObjv. C9.0/C9.1 retain the full counted bytes through all three APIs. Encoded C080 is distinct from raw zero. Source inspection separately selects the C8 legacy CString mirror and C9 direct EvalEx macro. Old-C/Jim TclOO-unavailable controls do not measure this API comparison there; no production namespace-key clipping or general object-getter permission follows.

## Scope

Six fixed declaration/primary arrangements (plain, raw zero, qualifier-after-zero, D800, FF, prefix negative), exact original v1..v5 inputs/observer chronology and API windows. C86/C90/C91 supported OO, C84/C85/Jim explicit unavailable OO controls. v3 C86 failed compile is retained separately.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `not-tested`. Channel: Original counted declaration/method objects and Tcl_EvalObjv calls; ASCII Tcl_Eval/Tcl_EvalEx reporter source; actual ListObjIndex child bytes. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

Captured OO_UNAVAILABLE control; no TclOO resident-key/API comparison executed for this provider.

### tcl8.6

Status: `observed`. Channel: Original counted declaration/method objects and Tcl_EvalObjv calls; ASCII Tcl_Eval/Tcl_EvalEx reporter source; actual ListObjIndex child bytes. Version: tcl8.6 8.6.18.

Legacy Tcl_Eval raw-zero report leaf is 6b; EvalEx and object-vector leaf are 6b007461696c.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted declaration/method objects and Tcl_EvalObjv calls; ASCII Tcl_Eval/Tcl_EvalEx reporter source; actual ListObjIndex child bytes. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

All three raw-zero report leaves are 6b007461696c.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `Pinned Tcl 8.4.20`, `tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_Eval`, lines 4907–4923. Retained reference `source-tcl8.4`.

```text
Tcl_Eval(interp, string)
    Tcl_Interp *interp;		/* Token for command interpreter (returned
				 * by previous call to Tcl_CreateInterp). */
    CONST char *string;		/* Pointer to TCL command to execute. */
{
    int code = Tcl_EvalEx(interp, string, -1, 0);

    /*
     * For backwards compatibility with old C code that predates the
     * object system in Tcl 8.0, we have to mirror the object result
     * back into the string result (some callers may expect it there).
     */

    Tcl_SetResult(interp, TclGetString(Tcl_GetObjResult(interp)),
	    TCL_VOLATILE);
    return code;
}
```

tcl8.5 8.5.19, revision `Pinned Tcl 8.5.19`, `tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_Eval`, lines 5000–5015. Retained reference `source-tcl8.5`.

```text
Tcl_Eval(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * previous call to Tcl_CreateInterp). */
    const char *script)		/* Pointer to TCL command to execute. */
{
    int code = Tcl_EvalEx(interp, script, -1, 0);

    /*
     * For backwards compatibility with old C code that predates the object
     * system in Tcl 8.0, we have to mirror the object result back into the
     * string result (some callers may expect it there).
     */

    (void) Tcl_GetStringResult(interp);
    return code;
}
```

tcl8.6 8.6.18, revision `Pinned Tcl 8.6.18`, `tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_Eval`, lines 5997–6012. Retained reference `source-tcl8.6`.

```text
Tcl_Eval(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * previous call to Tcl_CreateInterp). */
    const char *script)		/* Pointer to TCL command to execute. */
{
    int code = Tcl_EvalEx(interp, script, -1, 0);

    /*
     * For backwards compatibility with old C code that predates the object
     * system in Tcl 8.0, we have to mirror the object result back into the
     * string result (some callers may expect it there).
     */

    (void)Tcl_GetStringResult(interp);
    return code;
}
```

tcl9.0 9.0.4, revision `Pinned Tcl 9.0.4`, `tmp/tcl9.0.4/generic/tclDecls.h`, function `Tcl_Eval macro`, lines 3966–3967. Retained reference `source-tcl9.0`.

```text
#define Tcl_Eval(interp, objPtr) \
	Tcl_EvalEx(interp, objPtr, -1, 0)

```

tcl9.1 9.1.0, revision `Pinned Tcl 9.1.0`, `tmp/tcl9.1.0/generic/tclDecls.h`, function `Tcl_Eval macro`, lines 4085–4086. Retained reference `source-tcl9.1`.

```text
#define Tcl_Eval(interp, objPtr) \
	Tcl_EvalEx(interp, objPtr, TCL_INDEX_NONE, 0)

```

## Consumer bindings

- [rust/tcl-syntax/src/native_string.rs](../../../../rust/tcl-syntax/src/native_string.rs), `NativeStringProtocol::legacy_eval_result_input`: Pure selected legacy C result extent; does not mutate counted results or stored variable keys.
- [rust/tcl-vm/src/cmd_oo/native_variables.rs](../../../../rust/tcl-vm/src/cmd_oo/native_variables.rs), `cmd_oo::native_variables::tests::compiled_and_dynamic_declared_variables_match_original_native_controls` (linked): Compares original counted dispatch results with the legacy result projection only for rows actually captured through Tcl_Eval, and independently asserts v5 object-vector bytes without projection. No execution is inferred.
- [runtime/rust/src/cmd_oo/native_variables.rs](../../../../runtime/rust/src/cmd_oo/native_variables.rs), `cmd_oo::native_variables::tests::compiled_and_dynamic_declared_variables_match_original_native_controls` (linked): Compares original counted dispatch results with the legacy result projection only for rows actually captured through Tcl_Eval, and independently asserts v5 object-vector bytes without projection. No execution is inferred.
- [rust/tcl-syntax/src/native_string.rs](../../../../rust/tcl-syntax/src/native_string.rs), `native_string::tests::legacy_eval_result_extent_keeps_encoded_zero_separate_from_counted_zero` (linked): Pure C API result extent selection distinguishes raw zero from encoded C080; Jim C API purpose is unavailable. Source-only old C recipe checks are not empirical old C TclOO comparisons.

## Evidence

- [rust/tcl-vm/tests/data/native_oo_resident_keys/v5](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.4.20)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.5.19)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/8.6.18)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.0.4)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/9.1.0)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5/jim)
- [rust/tcl-vm/tests/data/native_oo_resident_keys](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys)
