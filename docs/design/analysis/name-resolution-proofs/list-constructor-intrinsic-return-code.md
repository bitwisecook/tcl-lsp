# naming.list.constructor-intrinsic-return-code

Kind: `source-anchor`

## Problem statement

A generic error alternative for object-vector list construction loses retained cells even though the selected intrinsic handler has a fixed return expression; callback effects remain independent.

## Question

Does the selected original List handler derive a guest error code from its children or result publication?

## Conclusion

Pinned C5 Tcl_ListObjCmd and Jim_ListCoreCommand finish with unconditional TCL_OK/JIM_OK. The original argv pointers are passed to native list construction without per-child string getters. This closes only the intrinsic return-code axis for a selected valid handler invocation. It does not close argv evaluation, command lookup, execution traces, previous result release/free callbacks, later rendering, allocation failure, or general Normal completion.

## Scope

Inspected exact C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim5bac7 handler functions. No guest/native/Rust launch; no Native object/frame/cache or Normal capability.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `inspected`. Channel: Inspected original handler function body; no guest input. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim pinned5bac7c99ad65864c87da513e22e2f01703fa4e03.

The handler constructs a list from the original object vector, publishes it and returns the fixed OK constant; child getters are absent from this function. Prior-result release may affect world state but is not used to choose the intrinsic return code.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_ListObjCmd`, lines 2889–2904. Retained reference `tcl8.4-source`.

```text
Tcl_ListObjCmd(dummy, interp, objc, objv)
    ClientData dummy;			/* Not used. */
    Tcl_Interp *interp;			/* Current interpreter. */
    register int objc;			/* Number of arguments. */
    register Tcl_Obj *CONST objv[];	/* The argument objects. */
{
    /*
     * If there are no list elements, the result is an empty object.
     * Otherwise modify the interpreter's result object to be a list object.
     */
    
    if (objc > 1) {
	Tcl_SetListObj(Tcl_GetObjResult(interp), (objc-1), &(objv[1]));
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `Tcl_ListObjCmd`, lines 2268–2284. Retained reference `tcl8.5-source`.

```text
Tcl_ListObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    register int objc,		/* Number of arguments. */
    register Tcl_Obj *const objv[])
				/* The argument objects. */
{
    /*
     * If there are no list elements, the result is an empty object.
     * Otherwise set the interpreter's result object to be a list object.
     */

    if (objc > 1) {
	Tcl_SetObjResult(interp, Tcl_NewListObj((objc-1), &(objv[1])));
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `Tcl_ListObjCmd`, lines 2431–2447. Retained reference `tcl8.6-source`.

```text
Tcl_ListObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,		/* Number of arguments. */
    Tcl_Obj *const objv[])
				/* The argument objects. */
{
    /*
     * If there are no list elements, the result is an empty object.
     * Otherwise set the interpreter's result object to be a list object.
     */

    if (objc > 1) {
	Tcl_SetObjResult(interp, Tcl_NewListObj(objc-1, &objv[1]));
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_ListObjCmd`, lines 2519–2535. Retained reference `tcl9.0-source`.

```text
Tcl_ListObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,		/* Number of arguments. */
    Tcl_Obj *const objv[])
				/* The argument objects. */
{
    /*
     * If there are no list elements, the result is an empty object.
     * Otherwise set the interpreter's result object to be a list object.
     */

    if (objc > 1) {
	Tcl_SetObjResult(interp, Tcl_NewListObj(objc-1, &objv[1]));
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_ListObjCmd`, lines 2517–2532. Retained reference `tcl9.1-source`.

```text
Tcl_ListObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The argument objects. */
{
    /*
     * If there are no list elements, the result is an empty object.
     * Otherwise set the interpreter's result object to be a list object.
     */

    if (objc > 1) {
	Tcl_SetObjResult(interp, Tcl_NewListObj(objc-1, &objv[1]));
    }
    return TCL_OK;
}

```

jim pinned5bac7c99ad65864c87da513e22e2f01703fa4e03, revision `pinned5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_ListCoreCommand`, lines 13466–13473. Retained reference `jim-source`.

```text
static int Jim_ListCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_Obj *listObjPtr;

    listObjPtr = Jim_NewListObj(interp, argv + 1, argc - 1);
    Jim_SetResult(interp, listObjPtr);
    return JIM_OK;
}

```

## Consumer bindings

- [rust/tcl-registry/src/native_result.rs](../../../../rust/tcl-registry/src/native_result.rs), `NativeResultContract::list_constructor_completion_code`: Select only the original C/Jim list constructor intrinsic return code after exact argv cardinality; result release, effects, observers and Normal completion remain separate.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `PreparedSourceNativeInvocation::completion_route`: Use the independently selected Leaf+ListArguments intrinsic code under genuine handler dispatch without issuing native Normal or world-effect certificates.
- [rust/tcl-registry/src/native_result.rs](../../../../rust/tcl-registry/src/native_result.rs), `native_result::tests::list_intrinsic_code_retains_unknown_effects_and_requires_native_cardinality` (linked): C5 and Jim empty/dynamic/native-byte operand vectors select only OK; missing dialect, hosted unsupported protocol, unknown argv shape, expansion, offset and different result contract decline. This test binding is not an execution claim.

## Evidence

- [rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.4-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.4-source.json)
- [rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.5-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.5-source.json)
- [rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.6-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/tcl8.6-source.json)
- [rust/tcl-registry/tests/data/native_list_constructor_return/tcl9.0-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/tcl9.0-source.json)
- [rust/tcl-registry/tests/data/native_list_constructor_return/tcl9.1-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/tcl9.1-source.json)
- [rust/tcl-registry/tests/data/native_list_constructor_return/jim-source.json](../../../../rust/tcl-registry/tests/data/native_list_constructor_return/jim-source.json)
