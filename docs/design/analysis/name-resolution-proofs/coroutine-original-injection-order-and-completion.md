# naming.coroutine.original-injection-order-and-completion

Kind: `native-observation`

## Problem statement

Dropping queued injections, evaluating them in arrival order, aborting after the newest error, or labelling yieldto as yield changes the observed callback sequence and final completion.

## Question

For two queued C9.1 injections, which callback runs first, does an older callback run after the newer callback returns Error, and which suspend-kind argument is delivered?

## Conclusion

For all four yield/yieldto success/error controls, queueing A then B calls B then A. A runs even when B returns Error, sees the B error result object, and returns Ok with that object; the final completion is Ok. Success controls retain the delivered result through both callbacks. Kind arguments remain yield or yieldto respectively. This closes only the finite callback chronology/completion rule, without callback purity, frame equivalence or a general Normal grant. The shared implementation scheduler can also select C9.0 through the separately observed original public-completion question. That public selection does not widen this C9.1 custom callback object-vector observation or its independent object/frame protocols.

## Scope

Four C9.1 controls: yield and yieldto, each with successful B and error B. Both native custom callbacks retain/report actual input/result pointers, kind bytes and callback sequence; final options are read after result getter. Older providers are explicitly not tested for this C9.1 API. Error-code/error-info continuity after a superseded callback error, arbitrary callback suspension, teardown and custom releases remain separate.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, jim

Status: `not-tested`. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; jim 0.84-9-g5bac7c9.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### tcl9.1

Status: `observed`. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped. Version: tcl9.1 9.1.0.

For all four yield/yieldto success/error controls, queueing A then B calls B then A. A runs even when B returns Error, sees the B error result object, and returns Ok with that object; the final completion is Ok. Success controls retain the delivered result through both callbacks. Kind arguments remain yield or yieldto respectively. This closes only the finite callback chronology/completion rule, without callback purity, frame equivalence or a general Normal grant.

### bigip

Status: `not-tested`. Channel: No input supplied.

No observation for this question.

## Source inspection

tcl9.1 9.1.0, revision `Pinned full source, independently associated by captured receipt required_sha256`, `generic/tclBasic.c`, function `TclNRCoroInjectObjCmd`, lines 9757–9801. Retained reference `source-3`.

```text
static int
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

```

tcl9.1 9.1.0, revision `Pinned full source, independently associated by captured receipt required_sha256`, `generic/tclBasic.c`, function `InjectHandler`, lines 9897–9941. Retained reference `source-4`.

```text
static int
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

- [rust/tcl-vm/src/cmd_coro/native_injection_tests.rs](../../../../rust/tcl-vm/src/cmd_coro/native_injection_tests.rs), `original_injection_order_and_error_continuation_match_native_callbacks`: Finite original native object-vector comparisons; no Rust execution claimed.
- [runtime/rust/src/cmd_coro/native_injection_tests.rs](../../../../runtime/rust/src/cmd_coro/native_injection_tests.rs), `original_injection_order_and_error_continuation_match_native_callbacks`: Finite original native object-vector comparisons; no Rust execution claimed.
- [rust/tcl-registry/src/native_coroutine_compilation.rs](../../../../rust/tcl-registry/src/native_coroutine_compilation.rs), `NativeCoroutineInjectionProtocol::run_pending`: Shared selected callback chronology/completion fold, independent of object transport/frame/Normal.
- [rust/tcl-vm/src/cmd_coro.rs](../../../../rust/tcl-vm/src/cmd_coro.rs), `run_injections`: Promotes the unchanged previous completion object to an actual callback argv reference before native result reset; shared selected scheduler retains newest-first/error replacement. No body, frame or Normal permission follows.
- [rust/tcl-vm/src/cmd_coro/native_injection_tests.rs](../../../../rust/tcl-vm/src/cmd_coro/native_injection_tests.rs), `cmd_coro::native_injection_tests::original_injection_order_and_error_continuation_match_native_callbacks` (linked): Four actual callback-order/completion controls with kind, code, result bytes and callback object identity.
- [runtime/rust/src/cmd_coro/native_injection_tests.rs](../../../../runtime/rust/src/cmd_coro/native_injection_tests.rs), `cmd_coro::native_injection_tests::original_injection_order_and_error_continuation_match_native_callbacks` (linked): Four actual callback-order/completion controls with kind, code, result bytes and callback object identity.
- [rust/tcl-registry/src/native_coroutine_compilation.rs](../../../../rust/tcl-registry/src/native_coroutine_compilation.rs), `native_coroutine_compilation::injection_tests::original_injection_chronology_replaces_errors_and_retains_suspend_kind` (linked): Pure selected C9.0/C9.1 scheduler retains newest-first error replacement and suspend kind; C8.x/Logical/Jim selection refuses. The current implementation selector does not extend this C9.1 native custom callback object-vector observation to other providers.

## Evidence

- [rust/tcl-registry/tests/data/native_coroutine_publication/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/probe.c)
- [rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c)
- [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv)
- [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr)
- [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv)
- [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr)
