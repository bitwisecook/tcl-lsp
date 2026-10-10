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

### tcl8.4

Status: `not-tested`. Version: 8.4.20. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Tcl.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### tcl8.5

Status: `not-tested`. Version: 8.5.19. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Tcl.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### tcl8.6

Status: `not-tested`. Version: 8.6.18. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Tcl.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### tcl9.0

Status: `not-tested`. Version: 9.0.4. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Tcl.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Tcl.

For all four yield/yieldto success/error controls, queueing A then B calls B then A. A runs even when B returns Error, sees the B error result object, and returns Ok with that object; the final completion is Ok. Success controls retain the delivered result through both callbacks. Kind arguments remain yield or yieldto respectively. This closes only the finite callback chronology/completion rule, without callback purity, frame equivalence or a general Normal grant.

### jim

Status: `not-tested`. Version: 0.84-9-g5bac7c9. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: C9.1 direct original counted string-object vectors. Raw input hex and pointer equality are sampled before the result getter; return options are later independent observations. Earlier providers are skipped.. Dialect: Jim.

Harness explicitly skips C9.1 probe/injection controls on this provider; no API rejection, cache or semantic equivalence inferred.

### bigip

Status: `not-tested`. Version: not measured. Build: not recorded. Channel: No input supplied. Dialect: BIG-IP.

No observation for this question.

## Exact evidence

- `probe-v1` (input): [rust/tcl-registry/tests/data/native_coroutine_publication/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/probe.c). SHA-256 `e47227595500d3399848c7a42fe0867de33df4f25c87a8f7b765692f7ee397bb`. Exact immutable original-object v1 probe source; count constructors, pointer timing, per-stage getters and untested API branches are inspectable.
- `probe-v2` (input): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c). SHA-256 `ab41e399af2ba7e074d779a1ee7e76fd9fe16e127c9d07f060010e2c1aae6a44`. Exact v2 probe, adding separate info-coroutine fullname controls without changing the v1 outcomes.
- `tcl9.1-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/receipt.json). SHA-256 `13209de752ec751ea45bb1c8136d13959a3d54f4001b79fefce9f2f06405734f`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.1-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv). SHA-256 `94805bbf1a96f65176a2aff451195912444e292b6ceec360898ebb1204830d04`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.1-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `tcl9.1-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/receipt.json). SHA-256 `9bd5decf3fa0e8b465b817c8eef53551aa674896bfe582592011660c006300a6`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.1-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv). SHA-256 `acaa106607bb5e8cbc663efaf2d39bc08cbcf5fe754fb8bb9524e743ae3429d3`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.1-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `source-3` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json). SHA-256 `ef5ccbf8675a6c164c672a128a6eab1cf44a59017558788e48d8c09dbc54c306`. JSON pointer `/3/snippet`. Exact pinned TclNRCoroInjectObjCmd source window, independent of result observations.
- `source-4` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json). SHA-256 `ef5ccbf8675a6c164c672a128a6eab1cf44a59017558788e48d8c09dbc54c306`. JSON pointer `/4/snippet`. Exact pinned InjectHandler source window, independent of result observations.

## Source inspection

tcl9.1 9.1.0, revision `Pinned full source, independently associated by captured receipt required_sha256`, `generic/tclBasic.c`, function `TclNRCoroInjectObjCmd`, lines 9757–9801. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `8106daf2ef964305777d79d1eb43d803882bf9f498a07a638ceafa1dd2b90fd5`; retained evidence `source-3`.

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

tcl9.1 9.1.0, revision `Pinned full source, independently associated by captured receipt required_sha256`, `generic/tclBasic.c`, function `InjectHandler`, lines 9897–9941. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `460c39ff6a22464d466d32fa04d2cae9d1c2018a2236f2b2c1c6cc401dc4634b`; retained evidence `source-4`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_coroutine_publication/verify.py"
]
```

Offline exact receipt/input/stream association verification only; zero native/compiler/Rust launches. Original capture.py and queue retain absolute provisioned input pins and immutable output paths; fresh recapture must use new output directories and recheck every required hash. Source windows reproduce separately from exact pinned full source LF ranges. No missing ELF is fabricated or claimed portable.
