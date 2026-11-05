# naming.time.original-script-object-evaluation

Kind: `source-anchor`

## Problem statement

Decoding a time script as Rust String or rebuilding bytes discards the original object and can reject opaque native script bytes before evaluation, including when count zero should skip the loop.

## Question

Does the selected native time handler evaluate its original script object on each positive iteration, or first rebuild a Unicode source string?

## Conclusion

All inspected C5 handlers retain objv[1] and repeatedly evaluate that same object with flags 0. The pinned Jim handler passes argv[1] directly to Jim_EvalObj on each iteration. Non-OK body completion is returned immediately. The loop does not evaluate the object at count zero. Cache/list dispatch, string materialization, caller context, source compilation and its admission remain independently selected; this is not an observed timing/result-format/count-conversion or negative-count parity claim.

## Scope

Source inspection of Tcl8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim commit5bac7c99ad65864c87da513e22e2f01703fa4e03. Only original script object reuse and body completion propagation, independently of timing/format/count widths, custom release/observers or arbitrary compilation grants.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `inspected`. Channel: Inspected original time handler body; no native input supplied. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

Original script object is reused inside the positive-count loop; a non-OK body completion propagates immediately.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_TimeObjCmd`, lines 3002–3051. Retained reference `tcl8.4-time-object`.

```text
Tcl_TimeObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    register Tcl_Obj *objPtr;
    Tcl_Obj *objs[4];
    register int i, result;
    int count;
    double totalMicroSec;
    Tcl_Time start, stop;

    if (objc == 2) {
	count = 1;
    } else if (objc == 3) {
	result = Tcl_GetIntFromObj(interp, objv[2], &count);
	if (result != TCL_OK) {
	    return result;
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "command ?count?");
	return TCL_ERROR;
    }
    
    objPtr = objv[1];
    i = count;
    Tcl_GetTime(&start);
    while (i-- > 0) {
	result = Tcl_EvalObjEx(interp, objPtr, 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
    Tcl_GetTime(&stop);
    
    totalMicroSec = ( ( (double) ( stop.sec - start.sec ) ) * 1.0e6
		      + ( stop.usec - start.usec ) );
    if (count <= 1) {
	/* Use int obj since we know time is not fractional [Bug 1202178] */
	objs[0] = Tcl_NewIntObj((count <= 0) ? 0 : (int) totalMicroSec);
    } else {
	objs[0] = Tcl_NewDoubleObj(totalMicroSec/count);
    }
    objs[1] = Tcl_NewStringObj("microseconds", -1);
    objs[2] = Tcl_NewStringObj("per", -1);
    objs[3] = Tcl_NewStringObj("iteration", -1);
    Tcl_SetObjResult(interp, Tcl_NewListObj(4, objs));
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdMZ.c`, function `Tcl_TimeObjCmd`, lines 3867–3939. Retained reference `tcl8.5-time-object`.

```text
Tcl_TimeObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    register Tcl_Obj *objPtr;
    Tcl_Obj *objs[4];
    register int i, result;
    int count;
    double totalMicroSec;
#ifndef TCL_WIDE_CLICKS
    Tcl_Time start, stop;
#else
    Tcl_WideInt start, stop;
#endif

    if (objc == 2) {
	count = 1;
    } else if (objc == 3) {
	result = TclGetIntFromObj(interp, objv[2], &count);
	if (result != TCL_OK) {
	    return result;
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "command ?count?");
	return TCL_ERROR;
    }

    objPtr = objv[1];
    i = count;
#ifndef TCL_WIDE_CLICKS
    Tcl_GetTime(&start);
#else
    start = TclpGetWideClicks();
#endif
    while (i-- > 0) {
	result = Tcl_EvalObjEx(interp, objPtr, 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
#ifndef TCL_WIDE_CLICKS
    Tcl_GetTime(&stop);
    totalMicroSec = ((double) (stop.sec - start.sec)) * 1.0e6
	    + (stop.usec - start.usec);
#else
    stop = TclpGetWideClicks();
    totalMicroSec = ((double) TclpWideClicksToNanoseconds(stop - start))/1.0e3;
#endif

    if (count <= 1) {
	/*
	 * Use int obj since we know time is not fractional. [Bug 1202178]
	 */

	objs[0] = Tcl_NewIntObj((count <= 0) ? 0 : (int) totalMicroSec);
    } else {
	objs[0] = Tcl_NewDoubleObj(totalMicroSec/count);
    }

    /*
     * Construct the result as a list because many programs have always parsed
     * as such (extracting the first element, typically).
     */

    TclNewLiteralStringObj(objs[1], "microseconds");
    TclNewLiteralStringObj(objs[2], "per");
    TclNewLiteralStringObj(objs[3], "iteration");
    Tcl_SetObjResult(interp, Tcl_NewListObj(4, objs));

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdMZ.c`, function `Tcl_TimeObjCmd`, lines 4184–4256. Retained reference `tcl8.6-time-object`.

```text
Tcl_TimeObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *objPtr;
    Tcl_Obj *objs[4];
    int i, result;
    int count;
    double totalMicroSec;
#ifndef TCL_WIDE_CLICKS
    Tcl_Time start, stop;
#else
    Tcl_WideInt start, stop;
#endif

    if (objc == 2) {
	count = 1;
    } else if (objc == 3) {
	result = TclGetIntFromObj(interp, objv[2], &count);
	if (result != TCL_OK) {
	    return result;
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "command ?count?");
	return TCL_ERROR;
    }

    objPtr = objv[1];
    i = count;
#ifndef TCL_WIDE_CLICKS
    Tcl_GetTime(&start);
#else
    start = TclpGetWideClicks();
#endif
    while (i-- > 0) {
	result = TclEvalObjEx(interp, objPtr, 0, NULL, 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
#ifndef TCL_WIDE_CLICKS
    Tcl_GetTime(&stop);
    totalMicroSec = ((double) (stop.sec - start.sec)) * 1.0e6
	    + (stop.usec - start.usec);
#else
    stop = TclpGetWideClicks();
    totalMicroSec = ((double) TclpWideClicksToNanoseconds(stop - start))/1.0e3;
#endif

    if (count <= 1) {
	/*
	 * Use int obj since we know time is not fractional. [Bug 1202178]
	 */

	objs[0] = Tcl_NewWideIntObj((count <= 0) ? 0 : (Tcl_WideInt)totalMicroSec);
    } else {
	objs[0] = Tcl_NewDoubleObj(totalMicroSec/count);
    }

    /*
     * Construct the result as a list because many programs have always parsed
     * as such (extracting the first element, typically).
     */

    TclNewLiteralStringObj(objs[1], "microseconds");
    TclNewLiteralStringObj(objs[2], "per");
    TclNewLiteralStringObj(objs[3], "iteration");
    Tcl_SetObjResult(interp, Tcl_NewListObj(4, objs));

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdMZ.c`, function `Tcl_TimeObjCmd`, lines 4022–4094. Retained reference `tcl9.0-time-object`.

```text
Tcl_TimeObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *objPtr;
    Tcl_Obj *objs[4];
    int i, result;
    int count;
    double totalMicroSec;
#ifndef TCL_WIDE_CLICKS
    Tcl_Time start, stop;
#else
    Tcl_WideInt start, stop;
#endif

    if (objc == 2) {
	count = 1;
    } else if (objc == 3) {
	result = TclGetIntFromObj(interp, objv[2], &count);
	if (result != TCL_OK) {
	    return result;
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "command ?count?");
	return TCL_ERROR;
    }

    objPtr = objv[1];
    i = count;
#ifndef TCL_WIDE_CLICKS
    Tcl_GetTime(&start);
#else
    start = TclpGetWideClicks();
#endif
    while (i-- > 0) {
	result = TclEvalObjEx(interp, objPtr, 0, NULL, 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
#ifndef TCL_WIDE_CLICKS
    Tcl_GetTime(&stop);
    totalMicroSec = ((double) (stop.sec - start.sec)) * 1.0e6
	    + (stop.usec - start.usec);
#else
    stop = TclpGetWideClicks();
    totalMicroSec = ((double) TclpWideClicksToNanoseconds(stop - start))/1.0e3;
#endif

    if (count <= 1) {
	/*
	 * Use int obj since we know time is not fractional. [Bug 1202178]
	 */

	TclNewIntObj(objs[0], (count <= 0) ? 0 : (Tcl_WideInt)totalMicroSec);
    } else {
	TclNewDoubleObj(objs[0], totalMicroSec/count);
    }

    /*
     * Construct the result as a list because many programs have always parsed
     * as such (extracting the first element, typically).
     */

    TclNewLiteralStringObj(objs[1], "microseconds");
    TclNewLiteralStringObj(objs[2], "per");
    TclNewLiteralStringObj(objs[3], "iteration");
    Tcl_SetObjResult(interp, Tcl_NewListObj(4, objs));

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdMZ.c`, function `Tcl_TimeObjCmd`, lines 4068–4135. Retained reference `tcl9.1-time-object`.

```text
Tcl_TimeObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Obj *objPtr;
    Tcl_Obj *objs[4];
    int result;
    long long i, count;
    double totalMicroSec;
    long long start, stop;

    if (objc == 2) {
	count = 1;
    } else if (objc == 3) {
	result = TclGetWideIntFromObj(interp, objv[2], &count);
	if (result != TCL_OK) {
	    return result;
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "command ?count?");
	return TCL_ERROR;
    }

    objPtr = objv[1];
    i = count;
#ifndef TCL_WIDE_CLICKS
    start = Tcl_GetMonotonicTime();
#else
    start = TclpGetWideClicks();
#endif
    while (i-- > 0) {
	result = TclEvalObjEx(interp, objPtr, 0, NULL, 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
#ifndef TCL_WIDE_CLICKS
    stop = Tcl_GetMonotonicTime();
    totalMicroSec = (double) (stop - start);
#else
    stop = TclpGetWideClicks();
    totalMicroSec = ((double) TclpWideClicksToNanoseconds(stop - start))/1.0e3;
#endif

    if (count <= 1) {
	/*
	 * Use int obj since we know time is not fractional. [Bug 1202178]
	 */

	TclNewIntObj(objs[0], (count <= 0) ? 0 : (Tcl_WideInt)totalMicroSec);
    } else {
	TclNewDoubleObj(objs[0], totalMicroSec/count);
    }

    /*
     * Construct the result as a list because many programs have always parsed
     * as such (extracting the first element, typically).
     */

    TclNewLiteralStringObj(objs[1], "microseconds");
    TclNewLiteralStringObj(objs[2], "per");
    TclNewLiteralStringObj(objs[3], "iteration");
    Tcl_SetObjResult(interp, Tcl_NewListObj(4, objs));

    return TCL_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_TimeCoreCommand`, lines 15120–15150. Retained reference `jim-time-object`.

```text
static int Jim_TimeCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    long i, count = 1;
    jim_wide start, elapsed;

    if (argc == 3) {
        if (Jim_GetLong(interp, argv[2], &count) != JIM_OK)
            return JIM_ERR;
    }
    if (count < 0)
        return JIM_OK;
    i = count;
    start = Jim_GetTimeUsec(CLOCK_MONOTONIC_RAW);
    while (i-- > 0) {
        int retval;

        retval = Jim_EvalObj(interp, argv[1]);
        if (retval != JIM_OK) {
            return retval;
        }
    }
    elapsed = Jim_GetTimeUsec(CLOCK_MONOTONIC_RAW) - start;
    if (elapsed < count * 10) {
        Jim_SetResult(interp, Jim_NewDoubleObj(interp, elapsed * 1.0 / count));
    }
    else {
        Jim_SetResultInt(interp, count == 0 ? 0 : elapsed / count);
    }
    Jim_AppendString(interp, Jim_GetResult(interp)," microseconds per iteration", -1);
    return JIM_OK;
}

```

## Consumer bindings

- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `cmd_time`: Evaluate original script handle through existing selected ControlBody object door.
- [runtime/rust/src/cmd_control.rs](../../../../runtime/rust/src/cmd_control.rs), `time_cmd`: Retain original script object through generic control-body evaluation rather than byte rebuilding.
- [rust/tcl-registry/src/native_eval_object.rs](../../../../rust/tcl-registry/src/native_eval_object.rs), `NativeEvalObjectProtocol::dispatches_list`: Keep independent original object/purpose/release list dispatch selection.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `command::tests::time_keeps_original_counted_script_and_skips_zero_count_materialization` (linked): Implementation regression retains opaque counted script names in genuine selected core and separately skips invalid script decoding at zero count; no actual native or Rust pass claimed.
- [runtime/rust/src/cmd_control.rs](../../../../runtime/rust/src/cmd_control.rs), `cmd_control::tests::time_retains_original_list_body_and_result_objects` (linked): Implementation regression preserves selected modern/Jim original List dispatch, same returned element identity and missing script string representation; native cache/dispatch remains independently selected and no launch/pass is claimed.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_time_object_source/tcl8.4-source.json](../../../../rust/tcl-cmd-core/tests/data/native_time_object_source/tcl8.4-source.json)
- [rust/tcl-cmd-core/tests/data/native_time_object_source/tcl8.5-source.json](../../../../rust/tcl-cmd-core/tests/data/native_time_object_source/tcl8.5-source.json)
- [rust/tcl-cmd-core/tests/data/native_time_object_source/tcl8.6-source.json](../../../../rust/tcl-cmd-core/tests/data/native_time_object_source/tcl8.6-source.json)
- [rust/tcl-cmd-core/tests/data/native_time_object_source/tcl9.0-source.json](../../../../rust/tcl-cmd-core/tests/data/native_time_object_source/tcl9.0-source.json)
- [rust/tcl-cmd-core/tests/data/native_time_object_source/tcl9.1-source.json](../../../../rust/tcl-cmd-core/tests/data/native_time_object_source/tcl9.1-source.json)
- [rust/tcl-cmd-core/tests/data/native_time_object_source/jim-source.json](../../../../rust/tcl-cmd-core/tests/data/native_time_object_source/jim-source.json)
