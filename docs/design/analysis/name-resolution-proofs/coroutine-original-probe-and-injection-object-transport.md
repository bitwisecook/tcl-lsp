# naming.coroutine.original-probe-and-injection-object-transport

Kind: `source-anchor`

## Problem statement

String or byte-vector copies in coroutine probe/injection adapters can change the original command-name object, argument identity and returned result despite retaining the same printable bytes.

## Question

Does the pinned C9.1 probe/injection source select the original coroutine-name object and retain the actual command-prefix and delivered argument objects for execution?

## Conclusion

GetCoroutineFromObj resolves the original Tcl_Obj through Tcl_GetCommandFromObj. Each callback holds a List constructed directly from original argv objects; InjectHandler appends the actual current result object and evaluates the retained element vector. This supports the object-transport boundary only, without a physical header, custom-release, frame, ordering, completion or Normal grant.

## Scope

Inspected Tcl9.1.0 generic/tclBasic.c GetCoroutineFromObj, TclNRCoroInjectObjCmd, TclNRCoroProbeObjCmd and InjectHandler. The implementation selectors cover one suspended-yield probe/injection with opaque byte names and unchanged original argument/result objects. Earlier providers, yieldto kind, multiple callback chronology, commandless lambda/cache and wasm suspension remain separate.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Pinned source inspection only; no executed build association.. Channel: Inspected original object callback source windows; no native input supplied.. Dialect: Tcl.

The original name Tcl_Obj is resolved, argv pointers are retained by Tcl_NewListObj, and the current result object is appended to the injection before direct object-vector evaluation.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `tcl9.1-coroutine-object-transport-0` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_object_transport_source/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_object_transport_source/source-windows.json). SHA-256 `b03940c855f6acd11bad740e9dd3ca4692f77090fc1878b1e11730f4390972d7`. JSON pointer `/0/snippet`. GetCoroutineFromObj pinned original-object source window; no native execution.
- `tcl9.1-coroutine-object-transport-1` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_object_transport_source/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_object_transport_source/source-windows.json). SHA-256 `b03940c855f6acd11bad740e9dd3ca4692f77090fc1878b1e11730f4390972d7`. JSON pointer `/1/snippet`. TclNRCoroInjectObjCmd and TclNRCoroProbeObjCmd pinned original-object source window; no native execution.
- `tcl9.1-coroutine-object-transport-2` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_object_transport_source/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_object_transport_source/source-windows.json). SHA-256 `b03940c855f6acd11bad740e9dd3ca4692f77090fc1878b1e11730f4390972d7`. JSON pointer `/2/snippet`. InjectHandler pinned original-object source window; no native execution.

## Source inspection

tcl9.1 9.1.0, revision `Pinned Tcl9.1.0 source tree; source inspection only`, `generic/tclBasic.c`, function `GetCoroutineFromObj`, lines 9738–9756. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `d1e3373fbbc2fc0644719ffe331fab3bb477b540d930eda7ac47ace162e6ff8e`; retained evidence `tcl9.1-coroutine-object-transport-0`.

```text
    Tcl_Interp *interp,
    Tcl_Obj *objPtr,
    const char *errMsg)
{
    /*
     * How to get a coroutine from its handle.
     */

    Command *cmdPtr = (Command *) Tcl_GetCommandFromObj(interp, objPtr);

    if ((!cmdPtr) || (cmdPtr->nreProc2 != TclNRInterpCoroutine)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(errMsg, TCL_INDEX_NONE));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "COROUTINE",
		TclGetString(objPtr), (char *)NULL);
	return NULL;
    }
    return (CoroutineData *)cmdPtr->objClientData2;
}


```

tcl9.1 9.1.0, revision `Pinned Tcl9.1.0 source tree; source inspection only`, `generic/tclBasic.c`, function `TclNRCoroInjectObjCmd and TclNRCoroProbeObjCmd`, lines 9758–9880. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `2271149923c5df711c613bc240789c3740925ca7798d7cdfbab1ac47ea94acce`; retained evidence `tcl9.1-coroutine-object-transport-1`.

```text
TclNRCoroInjectObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    CoroutineData *corPtr;

    /*
     * Usage more or less like tailcall:
     *   coroinject coroName cmd ?arg1 arg2 ...?
     */

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "coroName cmd ?arg1 arg2 ...?");
	return TCL_ERROR;
    }

    corPtr = GetCoroutineFromObj(interp, objv[1],
	    "can only inject a command into a coroutine");
    if (!corPtr) {
	return TCL_ERROR;
    }
    if (!COR_IS_SUSPENDED(corPtr)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"can only inject a command into a suspended coroutine", TCL_INDEX_NONE));
	Tcl_SetErrorCode(interp, "TCL", "COROUTINE", "ACTIVE", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Add the callback to the coro's execEnv, so that it is the first thing
     * to happen when the coro is resumed.
     */

    ExecEnv *savedEEPtr = iPtr->execEnvPtr;
    iPtr->execEnvPtr = corPtr->eePtr;
    TclNRAddCallback(interp, InjectHandler, corPtr,
	    Tcl_NewListObj(objc - 2, objv + 2), INT2PTR(corPtr->nargs), NULL);
    iPtr->execEnvPtr = savedEEPtr;

    return TCL_OK;
}

static int
TclNRCoroProbeObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    CoroutineData *corPtr;

    /*
     * Usage more or less like tailcall:
     *   coroprobe coroName cmd ?arg1 arg2 ...?
     */

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "coroName cmd ?arg1 arg2 ...?");
	return TCL_ERROR;
    }

    corPtr = GetCoroutineFromObj(interp, objv[1],
	    "can only inject a probe command into a coroutine");
    if (!corPtr) {
	return TCL_ERROR;
    }
    if (!COR_IS_SUSPENDED(corPtr)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"can only inject a probe command into a suspended coroutine",
		TCL_INDEX_NONE));
	Tcl_SetErrorCode(interp, "TCL", "COROUTINE", "ACTIVE", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Add the callback to the coro's execEnv, so that it is the first thing
     * to happen when the coro is resumed.
     */

    ExecEnv *savedEEPtr = iPtr->execEnvPtr;
    iPtr->execEnvPtr = corPtr->eePtr;
    TclNRAddCallback(interp, InjectHandler, corPtr,
	    Tcl_NewListObj(objc - 2, objv + 2), INT2PTR(corPtr->nargs), corPtr);
    iPtr->execEnvPtr = savedEEPtr;

    /*
     * Now we immediately transfer control to the coroutine to run our probe.
     * TRICKY STUFF copied from the [yield] implementation.
     *
     * Push the callback to restore the caller's context on yield back.
     */

    TclNRAddCallback(interp, NRCoroutineCallerCallback, corPtr,
	    NULL, NULL, NULL);

    /*
     * Record the stackLevel at which the resume is happening, then swap
     * the interp's environment to make it suitable to run this coroutine.
     */

    corPtr->stackLevel = &corPtr;
    Tcl_Size numLevels = corPtr->auxNumLevels;
    corPtr->auxNumLevels = iPtr->numLevels;

    /*
     * Do the actual stack swap.
     */

    SAVE_CONTEXT(corPtr->caller);
    corPtr->callerEEPtr = iPtr->execEnvPtr;
    RESTORE_CONTEXT(corPtr->running);
    iPtr->execEnvPtr = corPtr->eePtr;
    iPtr->numLevels += numLevels;
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * InjectHandler, InjectHandlerPostProc --
 *

```

tcl9.1 9.1.0, revision `Pinned Tcl9.1.0 source tree; source inspection only`, `generic/tclBasic.c`, function `InjectHandler`, lines 9898–9941. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `443d02746fe61d51c12fd3b0f98e920d3d0096c048b93d9fc7d8be7bdb53ec35`; retained evidence `tcl9.1-coroutine-object-transport-2`.

```text
InjectHandler(
    void *data[],
    Tcl_Interp *interp,
    TCL_UNUSED(int) /*result*/)
{
    CoroutineData *corPtr = (CoroutineData *)data[0];
    Tcl_Obj *listPtr = (Tcl_Obj *)data[1];
    Tcl_Size nargs = PTR2INT(data[2]);
    void *isProbe = data[3];
    Tcl_Size objc;
    Tcl_Obj **objv;

    if (!isProbe) {
	/*
	 * If this is [coroinject], add the extra arguments now.
	 */

	if (nargs == COROUTINE_ARGUMENTS_SINGLE_OPTIONAL) {
	    Tcl_ListObjAppendElement(NULL, listPtr,
		    Tcl_NewStringObj("yield", TCL_INDEX_NONE));
	} else if (nargs == COROUTINE_ARGUMENTS_ARBITRARY) {
	    Tcl_ListObjAppendElement(NULL, listPtr,
		    Tcl_NewStringObj("yieldto", TCL_INDEX_NONE));
	} else {
	    /*
	     * I don't think this is reachable...
	     */
	    Tcl_Obj *nargsObj;
	    TclNewIndexObj(nargsObj, nargs);
	    Tcl_ListObjAppendElement(NULL, listPtr, nargsObj);
	}
	Tcl_ListObjAppendElement(NULL, listPtr, Tcl_GetObjResult(interp));
    }

    /*
     * Call the user's script; we're in the right place.
     */

    Tcl_IncrRefCount(listPtr);
    TclMarkTailcall(interp);
    TclNRAddCallback(interp, InjectHandlerPostCall, corPtr, listPtr,
	    INT2PTR(nargs), isProbe);
    TclListObjGetElements(NULL, listPtr, &objc, &objv);
    return TclNREvalObjv(interp, objc, objv, 0, NULL);

```


## Consumer bindings

- [runtime/rust/src/interp/native_command_names.rs](../../../../runtime/rust/src/interp/native_command_names.rs), `Interp::resolve_original_command_generation`: Resolve the original native operand to its actual selected command generation; missing selection and unavailable generation remain distinct typed outcomes.
- [runtime/rust/src/cmd_coro.rs](../../../../runtime/rust/src/cmd_coro.rs), `original_prefix`: Constructs the native owning List directly from unchanged original prefix objects for the serialised worker handoff.
- [runtime/rust/src/cmd_coro.rs](../../../../runtime/rust/src/cmd_coro.rs), `imp::eval_words`: Dispatches the retained original element vector and pins its result object before releasing the prefix owner.
- [runtime/rust/src/cmd_coro.rs](../../../../runtime/rust/src/cmd_coro.rs), `cmd_coro::original_operand_tests::coroutine_probe_and_injection_keep_opaque_names_and_original_objects` (linked): Implementation object transport: opaque original coroutine name/helper head and raw-zero/FF original argument preserve identity across probe, single injection and resumed result; no native execution or header/frame/order equivalence claim.

A named test is a coverage binding, not a claim that it executed.

## Replay

Reproduce exact full-source/snippet hashes and LF coordinates. No executable/native/Rust launch is attached; callback order/kind/error propagation and foreign object callbacks have independent obligations.
