# naming.event.global-wait-trigger

Kind: `native-observation`

## Problem statement

Polling rendered caller variables cannot implement the selected global wait trigger.

## Question

Do these global scalar/array waits stop on an unchanged write, an unset, or only a changed value after the current timer turn?

## Conclusion

C wakes on a global write/unset trace, whereas Jim compares retained global values after processing events. Deterministic v110 nested registration yields SAME/OLD on all five C providers and SAME NEXT/NEW on Jim. Same-turn v106 writes both run before the wait returns; unsetting yields existence 0 on all six. The v107 staggered rows are timing-conditioned observations, not deadline-independent laws.

## Scope

Seven finite source controls across v106/v107/v110, fresh interpreter per case. Global subject lookup/trigger and actual returned values only. No unlimited notifier availability, signal/body options, trace-free ordinary read, or physical slot/lifetime claim. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

v106/SAME_WRITE: SAME_WRITE: code 0, bytes b'SAME NEXT' v106/ARRAY_SAME_WRITE: ARRAY_SAME_WRITE: code 0, bytes b'NEW' v106/UNSET_WRITE: UNSET_WRITE: code 0, bytes b'0' v107/SAME_STAGGER: SAME_STAGGER: code 0, bytes b'SAME' v107/ARRAY_STAGGER: ARRAY_STAGGER: code 0, bytes b'OLD' v110/SAME_NESTED: SAME_NESTED: code 0, bytes b'SAME' v110/ARRAY_NESTED: ARRAY_NESTED: code 0, bytes b'OLD'

### jim

Status: `observed`. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest. Version: jim 0.84-9-g5bac7c9.

v106/SAME_WRITE: SAME_WRITE: code 0, bytes b'SAME NEXT' v106/ARRAY_SAME_WRITE: ARRAY_SAME_WRITE: code 0, bytes b'NEW' v106/UNSET_WRITE: UNSET_WRITE: code 0, bytes b'0' v107/SAME_STAGGER: SAME_STAGGER: code 0, bytes b'SAME NEXT' v107/ARRAY_STAGGER: ARRAY_STAGGER: code 0, bytes b'NEW' v110/SAME_NESTED: SAME_NESTED: code 0, bytes b'SAME NEXT' v110/ARRAY_NESTED: ARRAY_NESTED: code 0, bytes b'NEW'

### bigip

Status: `not-tested`. Channel: not exercised.

No BIG-IP provider was executed or inspected for this Event question.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1176–1216. Retained reference `e98d4bc726037374525cd`.

```text
Tcl_VwaitObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int done, foundEvent;
    char *nameString;

    if (objc != 2) {
        Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
    }
    Tcl_UntraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done);

    /*
     * Clear out the interpreter's result, since it may have been set
     * by event handlers.
     */

    Tcl_ResetResult(interp);
    if (!foundEvent) {
	Tcl_AppendResult(interp, "can't wait for variable \"", nameString,
		"\":  would wait forever", (char *) NULL);
	return TCL_ERROR;
    }
    return TCL_OK;
}
```

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEvent.c`, function `VwaitVarProc`, lines 1220–1231. Retained reference `e98d4bc726037374525cd`.

```text
VwaitVarProc(clientData, interp, name1, name2, flags)
    ClientData clientData;	/* Pointer to integer to set to 1. */
    Tcl_Interp *interp;		/* Interpreter containing variable. */
    CONST char *name1;		/* Name of variable. */
    CONST char *name2;		/* Second part of variable name. */
    int flags;			/* Information about what happened. */
{
    int *donePtr = (int *) clientData;

    *donePtr = 1;
    return (char *) NULL;
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1321–1368. Retained reference `ebcd14ee056c41f642dc1`.

```text
Tcl_VwaitObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *CONST objv[])	/* Argument objects. */
{
    int done, foundEvent;
    char *nameString;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
	if (Tcl_LimitExceeded(interp)) {
	    break;
	}
    }
    Tcl_UntraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done);

    /*
     * Clear out the interpreter's result, since it may have been set by event
     * handlers.
     */

    Tcl_ResetResult(interp);
    if (!foundEvent) {
	Tcl_AppendResult(interp, "can't wait for variable \"", nameString,
		"\": would wait forever", NULL);
	return TCL_ERROR;
    }
    if (!done) {
	Tcl_AppendResult(interp, "limit exceeded", NULL);
	return TCL_ERROR;
    }
    return TCL_OK;
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEvent.c`, function `VwaitVarProc`, lines 1372–1383. Retained reference `ebcd14ee056c41f642dc1`.

```text
VwaitVarProc(
    ClientData clientData,	/* Pointer to integer to set to 1. */
    Tcl_Interp *interp,		/* Interpreter containing variable. */
    CONST char *name1,		/* Name of variable. */
    CONST char *name2,		/* Second part of variable name. */
    int flags)			/* Information about what happened. */
{
    int *donePtr = (int *) clientData;

    *donePtr = 1;
    return NULL;
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1387–1447. Retained reference `e1799f8f23cd143629768`.

```text
Tcl_VwaitObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int done, foundEvent;
    const char *nameString;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar2(interp, nameString, NULL,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    break;
	}
    }
    Tcl_UntraceVar2(interp, nameString, NULL,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, &done);

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't wait for variable \"%s\": would wait forever",
		nameString));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	return TCL_ERROR;
    }
    if (!done) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */

	return TCL_ERROR;
    }

    /*
     * Clear out the interpreter's result, since it may have been set by event
     * handlers.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEvent.c`, function `VwaitVarProc`, lines 1450–1463. Retained reference `e1799f8f23cd143629768`.

```text
VwaitVarProc(
    ClientData clientData,	/* Pointer to integer to set to 1. */
    Tcl_Interp *interp,		/* Interpreter containing variable. */
    const char *name1,		/* Name of variable. */
    const char *name2,		/* Second part of variable name. */
    int flags)			/* Information about what happened. */
{
    int *donePtr = (int *)clientData;

    *donePtr = 1;
    Tcl_UntraceVar2(interp, name1, name2, TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, clientData);
    return NULL;
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1498–1867. Retained reference `e30bb9be41c1824d28e5d`.

```text
Tcl_VwaitObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int i, done = 0, timedOut = 0, foundEvent, any = 1, timeout = 0;
    int numItems = 0, extended = 0, result, mode, mask = TCL_ALL_EVENTS;
    Tcl_InterpState saved = NULL;
    Tcl_TimerToken timer = NULL;
    Tcl_Time before, after;
    Tcl_Channel chan;
    Tcl_WideInt diff = -1;
    VwaitItem localItems[32], *vwaitItems = localItems;
    static const char *const vWaitOptionStrings[] = {
	"-all",	"-extended", "-nofileevents", "-noidleevents",
	"-notimerevents", "-nowindowevents", "-readable",
	"-timeout", "-variable", "-writable", "--", NULL
    };
    enum vWaitOptions {
	OPT_ALL, OPT_EXTD, OPT_NO_FEVTS, OPT_NO_IEVTS,
	OPT_NO_TEVTS, OPT_NO_WEVTS, OPT_READABLE,
	OPT_TIMEOUT, OPT_VARIABLE, OPT_WRITABLE, OPT_LAST
    } index;

    if ((objc == 2) && (strcmp(Tcl_GetString(objv[1]), "--") != 0)) {
	/*
	 * Legacy "vwait" syntax, skip option handling.
	 */
	i = 1;
	goto endOfOptionLoop;
    }

    if ((unsigned) objc - 1 > sizeof(localItems) / sizeof(localItems[0])) {
	vwaitItems = (VwaitItem *)Tcl_Alloc(sizeof(VwaitItem) * (objc - 1));
    }

    for (i = 1; i < objc; i++) {
	const char *name;

	name = TclGetString(objv[i]);
	if (name[0] != '-') {
	    break;
	}
	if (Tcl_GetIndexFromObj(interp, objv[i], vWaitOptionStrings, "option", 0,
		&index) != TCL_OK) {
	    result = TCL_ERROR;
	    goto done;
	}
	switch (index) {
	case OPT_ALL:
	    any = 0;
	    break;
	case OPT_EXTD:
	    extended = 1;
	    break;
	case OPT_NO_FEVTS:
	    mask &= ~TCL_FILE_EVENTS;
	    break;
	case OPT_NO_IEVTS:
	    mask &= ~TCL_IDLE_EVENTS;
	    break;
	case OPT_NO_TEVTS:
	    mask &= ~TCL_TIMER_EVENTS;
	    break;
	case OPT_NO_WEVTS:
	    mask &= ~TCL_WINDOW_EVENTS;
	    break;
	case OPT_TIMEOUT:
	    if (++i >= objc) {
	needArg:
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"argument required for \"%s\"", vWaitOptionStrings[index]));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "ARGUMENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (Tcl_GetIntFromObj(interp, objv[i], &timeout) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (timeout < 0) {
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"timeout must be positive", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NEGTIME", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    break;
	case OPT_LAST:
	    i++;
	    goto endOfOptionLoop;
	case OPT_VARIABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[numItems]);
	    if (result != TCL_OK) {
		goto done;
	    }
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = 0;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_READABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_READABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for reading",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_READABLE,
		    VwaitChannelReadProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = TCL_READABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_WRITABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_WRITABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for writing",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_WRITABLE,
		    VwaitChannelWriteProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = TCL_WRITABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

  endOfOptionLoop:
    if ((mask & (TCL_FILE_EVENTS | TCL_IDLE_EVENTS |
	    TCL_TIMER_EVENTS | TCL_WINDOW_EVENTS)) == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"can't wait: would block forever", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if ((timeout > 0) && ((mask & TCL_TIMER_EVENTS) == 0)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"timer events disabled with timeout specified", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_TIME", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    for (result = TCL_OK; i < objc; i++) {
	result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		VwaitVarProc, &vwaitItems[numItems]);
	if (result != TCL_OK) {
	    break;
	}
	vwaitItems[numItems].donePtr = &done;
	vwaitItems[numItems].sequence = -1;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = objv[i];
	numItems++;
    }
    if (result != TCL_OK) {
	result = TCL_ERROR;
	goto done;
    }

    if (!(mask & TCL_FILE_EVENTS)) {
	for (i = 0; i < numItems; i++) {
	    if (vwaitItems[i].mask) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"file events disabled with channel(s) specified", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_FILE_EVENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	}
    }

    if (timeout > 0) {
	vwaitItems[numItems].donePtr = &timedOut;
	vwaitItems[numItems].sequence = -1;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = NULL;
	timer = Tcl_CreateTimerHandler(timeout, VwaitTimeoutProc,
		&vwaitItems[numItems]);
	Tcl_GetTime(&before);
    } else {
	timeout = 0;
    }

    if ((numItems == 0) && (timeout == 0)) {
	/*
	 * "vwait" is equivalent to "update",
	 * "vwait -nofileevents -notimerevents -nowindowevents"
	 * is equivalent to "update idletasks"
	 */
	any = 1;
	mask |= TCL_DONT_WAIT;
    }

    foundEvent = 1;
    while (!timedOut && foundEvent &&
	   ((!any && (done < numItems)) || (any && !done))) {
	foundEvent = Tcl_DoOneEvent(mask);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    Tcl_SetErrorCode(interp, "TCL", "EVENT", "LIMIT", (char *)NULL);
	    break;
	}
	if ((numItems == 0) && (timeout == 0)) {
	    /*
	     * Behavior like "update": clear interpreter's result because
	     * event handlers could have executed commands.
	     */
	    Tcl_ResetResult(interp);
	    result = TCL_OK;
	    goto done;
	}
    }

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_NewStringObj((numItems == 0) ?
		"can't wait: would wait forever" :
		"can't wait for variable(s)/channel(s): would wait forever",
		-1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (!done && !timedOut) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */
	result = TCL_ERROR;
	goto done;
    }

    result = TCL_OK;
    if (timeout <= 0) {
	/*
	 * Clear out the interpreter's result, since it may have been set
	 * by event handlers.
	 */
	Tcl_ResetResult(interp);
	goto done;
    }

    /*
     * When timeout was specified, report milliseconds left or -1 on timeout.
     */
    if (timedOut) {
	diff = -1;
    } else {
	Tcl_GetTime(&after);
	diff = after.sec * 1000 + after.usec / 1000;
	diff -= before.sec * 1000 + before.usec / 1000;
	diff = timeout - diff;
	if (diff < 0) {
	    diff = 0;
	}
    }

  done:
    if ((timeout > 0) && (timer != NULL)) {
	Tcl_DeleteTimerHandler(timer);
    }
    if (result != TCL_OK) {
	saved = Tcl_SaveInterpState(interp, result);
    }
    for (i = 0; i < numItems; i++) {
	if (vwaitItems[i].mask & TCL_READABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelReadProc,
			&vwaitItems[i]);
	    }
	} else if (vwaitItems[i].mask & TCL_WRITABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelWriteProc,
			&vwaitItems[i]);
	    }
	} else {
	    Tcl_UntraceVar2(interp, TclGetString(vwaitItems[i].sourceObj),
		    NULL, TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[i]);
	}
    }

    if (result == TCL_OK) {
	if (extended) {
	    int k;
	    Tcl_Obj *listObj, *keyObj;

	    TclNewObj(listObj);
	    for (k = 0; k < done; k++) {
		for (i = 0; i < numItems; i++) {
		    if (vwaitItems[i].sequence != k) {
			continue;
		    }
		    if (vwaitItems[i].mask & TCL_READABLE) {
			TclNewLiteralStringObj(keyObj, "readable");
		    } else if (vwaitItems[i].mask & TCL_WRITABLE) {
			TclNewLiteralStringObj(keyObj, "writable");
		    } else {
			TclNewLiteralStringObj(keyObj, "variable");
		    }
		    Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		    Tcl_ListObjAppendElement(NULL, listObj,
			    vwaitItems[i].sourceObj);
		}
	    }
	    if (timeout > 0) {
		TclNewLiteralStringObj(keyObj, "timeleft");
		Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		Tcl_ListObjAppendElement(NULL, listObj,
			Tcl_NewWideIntObj(diff));
	    }
	    Tcl_SetObjResult(interp, listObj);
	} else if (timeout > 0) {
	    Tcl_SetObjResult(interp, Tcl_NewWideIntObj(diff));
	}
    } else {
	result = Tcl_RestoreInterpState(interp, saved);
    }
    if (vwaitItems != localItems) {
	Tcl_Free(vwaitItems);
    }
    return result;
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEvent.c`, function `VwaitVarProc`, lines 1916–1933. Retained reference `e30bb9be41c1824d28e5d`.

```text
VwaitVarProc(
    void *clientData,		/* Pointer to vwait info record. */
    Tcl_Interp *interp,		/* Interpreter containing variable. */
    const char *name1,		/* Name of variable. */
    const char *name2,		/* Second part of variable name. */
    TCL_UNUSED(int) /*flags*/)	/* Information about what happened. */
{
    VwaitItem *itemPtr = (VwaitItem *) clientData;

    if (itemPtr->donePtr != NULL) {
	itemPtr->sequence = itemPtr->donePtr[0];
	itemPtr->donePtr[0] += 1;
	itemPtr->donePtr = NULL;
    }
    Tcl_UntraceVar2(interp, name1, name2, TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, clientData);
    return NULL;
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1536–1906. Retained reference `eaa5f919dc246958bc5d1`.

```text
Tcl_VwaitObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size i, done = 0, numItems = 0, timedOut = 0;
    int foundEvent, any = 1, timeout = 0;
    int extended = 0, result, mode, mask = TCL_ALL_EVENTS;
    Tcl_InterpState saved = NULL;
    Tcl_TimerToken timer = NULL;
    long long before = -1, after;
    Tcl_Channel chan;
    Tcl_WideInt diff = -1;
    VwaitItem localItems[32], *vwaitItems = localItems;
    static const char *const vWaitOptionStrings[] = {
	"-all",	"-extended", "-nofileevents", "-noidleevents",
	"-notimerevents", "-nowindowevents", "-readable",
	"-timeout", "-variable", "-writable", "--", NULL
    };
    enum vWaitOptions {
	OPT_ALL, OPT_EXTD, OPT_NO_FEVTS, OPT_NO_IEVTS,
	OPT_NO_TEVTS, OPT_NO_WEVTS, OPT_READABLE,
	OPT_TIMEOUT, OPT_VARIABLE, OPT_WRITABLE, OPT_LAST
    } index;

    if ((objc == 2) && (strcmp(Tcl_GetString(objv[1]), "--") != 0)) {
	/*
	 * Legacy "vwait" syntax, skip option handling.
	 */
	i = 1;
	goto endOfOptionLoop;
    }

    if ((unsigned) objc - 1 > sizeof(localItems) / sizeof(localItems[0])) {
	vwaitItems = (VwaitItem *)Tcl_Alloc(sizeof(VwaitItem) * (objc - 1));
    }

    for (i = 1; i < objc; i++) {
	const char *name;

	name = TclGetString(objv[i]);
	if (name[0] != '-') {
	    break;
	}
	if (Tcl_GetIndexFromObj(interp, objv[i], vWaitOptionStrings, "option", 0,
		&index) != TCL_OK) {
	    result = TCL_ERROR;
	    goto done;
	}
	switch (index) {
	case OPT_ALL:
	    any = 0;
	    break;
	case OPT_EXTD:
	    extended = 1;
	    break;
	case OPT_NO_FEVTS:
	    mask &= ~TCL_FILE_EVENTS;
	    break;
	case OPT_NO_IEVTS:
	    mask &= ~TCL_IDLE_EVENTS;
	    break;
	case OPT_NO_TEVTS:
	    mask &= ~TCL_TIMER_EVENTS;
	    break;
	case OPT_NO_WEVTS:
	    mask &= ~TCL_WINDOW_EVENTS;
	    break;
	case OPT_TIMEOUT:
	    if (++i >= objc) {
	needArg:
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"argument required for \"%s\"", vWaitOptionStrings[index]));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "ARGUMENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (Tcl_GetIntFromObj(interp, objv[i], &timeout) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (timeout < 0) {
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"timeout must be positive", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NEGTIME", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    break;
	case OPT_LAST:
	    i++;
	    goto endOfOptionLoop;
	case OPT_VARIABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[numItems]);
	    if (result != TCL_OK) {
		goto done;
	    }
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = 0;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_READABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_READABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for reading",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_READABLE,
		    VwaitChannelReadProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = TCL_READABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_WRITABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_WRITABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for writing",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_WRITABLE,
		    VwaitChannelWriteProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = TCL_WRITABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

  endOfOptionLoop:
    if ((mask & (TCL_FILE_EVENTS | TCL_IDLE_EVENTS |
	    TCL_TIMER_EVENTS | TCL_WINDOW_EVENTS)) == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"can't wait: would block forever", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if ((timeout > 0) && ((mask & TCL_TIMER_EVENTS) == 0)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"timer events disabled with timeout specified", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_TIME", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    for (result = TCL_OK; i < objc; i++) {
	result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		VwaitVarProc, &vwaitItems[numItems]);
	if (result != TCL_OK) {
	    break;
	}
	vwaitItems[numItems].donePtr = &done;
	vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = objv[i];
	numItems++;
    }
    if (result != TCL_OK) {
	result = TCL_ERROR;
	goto done;
    }

    if (!(mask & TCL_FILE_EVENTS)) {
	for (i = 0; i < numItems; i++) {
	    if (vwaitItems[i].mask) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"file events disabled with channel(s) specified", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_FILE_EVENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	}
    }

    if (timeout > 0) {
	vwaitItems[numItems].donePtr = &timedOut;
	vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = NULL;
	timer = Tcl_CreateTimerHandler(timeout, VwaitTimeoutProc,
		&vwaitItems[numItems]);
	before = Tcl_GetDayTime();
    } else {
	timeout = 0;
    }

    if ((numItems == 0) && (timeout == 0)) {
	/*
	 * "vwait" is equivalent to "update",
	 * "vwait -nofileevents -notimerevents -nowindowevents"
	 * is equivalent to "update idletasks"
	 */
	any = 1;
	mask |= TCL_DONT_WAIT;
    }

    foundEvent = 1;
    while (!timedOut && foundEvent &&
	    ((!any && (done < numItems)) || (any && !done))) {
	foundEvent = Tcl_DoOneEvent(mask);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    Tcl_SetErrorCode(interp, "TCL", "EVENT", "LIMIT", (char *)NULL);
	    break;
	}
	if ((numItems == 0) && (timeout == 0)) {
	    /*
	     * Behavior like "update": clear interpreter's result because
	     * event handlers could have executed commands.
	     */
	    Tcl_ResetResult(interp);
	    result = TCL_OK;
	    goto done;
	}
    }

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_NewStringObj((numItems == 0) ?
		"can't wait: would wait forever" :
		"can't wait for variable(s)/channel(s): would wait forever",
		-1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (!done && !timedOut) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */
	result = TCL_ERROR;
	goto done;
    }

    result = TCL_OK;
    if (timeout <= 0) {
	/*
	 * Clear out the interpreter's result, since it may have been set
	 * by event handlers.
	 */
	Tcl_ResetResult(interp);
	goto done;
    }

    /*
     * When timeout was specified, report milliseconds left or -1 on timeout.
     */
    if (timedOut) {
	diff = -1;
    } else {
	after = Tcl_GetDayTime();
	diff = after / 1000;
	diff -= before / 1000;
	diff = timeout - diff;
	if (diff < 0) {
	    diff = 0;
	}
    }

  done:
    if ((timeout > 0) && (timer != NULL)) {
	Tcl_DeleteTimerHandler(timer);
    }
    if (result != TCL_OK) {
	saved = Tcl_SaveInterpState(interp, result);
    }
    for (i = 0; i < numItems; i++) {
	if (vwaitItems[i].mask & TCL_READABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelReadProc,
			&vwaitItems[i]);
	    }
	} else if (vwaitItems[i].mask & TCL_WRITABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelWriteProc,
			&vwaitItems[i]);
	    }
	} else {
	    Tcl_UntraceVar2(interp, TclGetString(vwaitItems[i].sourceObj),
		    NULL, TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[i]);
	}
    }

    if (result == TCL_OK) {
	if (extended) {
	    Tcl_Size k;
	    Tcl_Obj *listObj, *keyObj;

	    TclNewObj(listObj);
	    for (k = 0; k < done; k++) {
		for (i = 0; i < numItems; i++) {
		    if (vwaitItems[i].sequence != k) {
			continue;
		    }
		    if (vwaitItems[i].mask & TCL_READABLE) {
			TclNewLiteralStringObj(keyObj, "readable");
		    } else if (vwaitItems[i].mask & TCL_WRITABLE) {
			TclNewLiteralStringObj(keyObj, "writable");
		    } else {
			TclNewLiteralStringObj(keyObj, "variable");
		    }
		    Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		    Tcl_ListObjAppendElement(NULL, listObj,
			    vwaitItems[i].sourceObj);
		}
	    }
	    if (timeout > 0) {
		TclNewLiteralStringObj(keyObj, "timeleft");
		Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		Tcl_ListObjAppendElement(NULL, listObj,
			Tcl_NewWideIntObj(diff));
	    }
	    Tcl_SetObjResult(interp, listObj);
	} else if (timeout > 0) {
	    Tcl_SetObjResult(interp, Tcl_NewWideIntObj(diff));
	}
    } else {
	result = Tcl_RestoreInterpState(interp, saved);
    }
    if (vwaitItems != localItems) {
	Tcl_Free(vwaitItems);
    }
    return result;
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEvent.c`, function `VwaitVarProc`, lines 1955–1972. Retained reference `eaa5f919dc246958bc5d1`.

```text
VwaitVarProc(
    void *clientData,		/* Pointer to vwait info record. */
    Tcl_Interp *interp,		/* Interpreter containing variable. */
    const char *name1,		/* Name of variable. */
    const char *name2,		/* Second part of variable name. */
    TCL_UNUSED(int) /*flags*/)	/* Information about what happened. */
{
    VwaitItem *itemPtr = (VwaitItem *) clientData;

    if (itemPtr->donePtr != NULL) {
	itemPtr->sequence = itemPtr->donePtr[0];
	itemPtr->donePtr[0] += 1;
	itemPtr->donePtr = NULL;
    }
    Tcl_UntraceVar2(interp, name1, name2, TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, clientData);
    return NULL;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `Jim_ProcessEvents`, lines 377–534. Retained reference `e4521e47b782c5165c471`.

```text
int Jim_ProcessEvents(Jim_Interp *interp, int flags)
{
    jim_wide sleep_us = -1;
    int processed = 0;
    Jim_EventLoop *eventLoop = Jim_GetAssocData(interp, "eventloop");
    Jim_FileEvent *fe = eventLoop->fileEventHead;
    Jim_TimeEvent *te;
    jim_wide maxId;

    if ((flags & JIM_FILE_EVENTS) == 0 || fe == NULL) {
        /* No file events */
        if ((flags & JIM_TIME_EVENTS) == 0 || eventLoop->timeEventHead == NULL) {
            /* No time events */
            return -1;
        }
    }

    /* Note that we want call select() even if there are no
     * file events to process as long as we want to process time
     * events, in order to sleep until the next time event is ready
     * to fire. */

    if (flags & JIM_DONT_WAIT) {
        /* Wait no time */
        sleep_us = 0;
    }
    else if (flags & JIM_TIME_EVENTS) {
        /* The nearest timer is always at the head of the list */
        if (eventLoop->timeEventHead) {
            Jim_TimeEvent *shortest = eventLoop->timeEventHead;

            /* Calculate the time missing for the nearest
             * timer to fire. */
            sleep_us = shortest->when - Jim_GetTimeUsec(CLOCK_MONOTONIC_RAW);
            if (sleep_us < 0) {
                sleep_us = 0;
            }
        }
        else {
            /* Wait forever */
            sleep_us = -1;
        }
    }

#ifdef HAVE_SELECT
    if (flags & JIM_FILE_EVENTS) {
        int retval;
        struct timeval tv, *tvp = NULL;
        fd_set rfds, wfds, efds;
        int maxfd = -1;

        FD_ZERO(&rfds);
        FD_ZERO(&wfds);
        FD_ZERO(&efds);

        /* Check file events */
        while (fe != NULL) {
            if (fe->mask & JIM_EVENT_READABLE)
                FD_SET(fe->fd, &rfds);
            if (fe->mask & JIM_EVENT_WRITABLE)
                FD_SET(fe->fd, &wfds);
            if (fe->mask & JIM_EVENT_EXCEPTION)
                FD_SET(fe->fd, &efds);
            if (maxfd < fe->fd)
                maxfd = fe->fd;
            fe = fe->next;
        }

        if (sleep_us >= 0) {
            tvp = &tv;
            tvp->tv_sec = sleep_us / 1000000;
            tvp->tv_usec = sleep_us % 1000000;
        }

        retval = select(maxfd + 1, &rfds, &wfds, &efds, tvp);

        if (retval < 0) {
            if (errno == EINVAL) {
                /* This can happen on mingw32 if a non-socket filehandle is passed */
                Jim_SetResultString(interp, "non-waitable filehandle", -1);
                return -2;
            }
        }
        else if (retval > 0) {
            fe = eventLoop->fileEventHead;
            while (fe != NULL) {
                int mask = 0;
                int fd = fe->fd;

                if ((fe->mask & JIM_EVENT_READABLE) && FD_ISSET(fd, &rfds))
                    mask |= JIM_EVENT_READABLE;
                if (fe->mask & JIM_EVENT_WRITABLE && FD_ISSET(fd, &wfds))
                    mask |= JIM_EVENT_WRITABLE;
                if (fe->mask & JIM_EVENT_EXCEPTION && FD_ISSET(fd, &efds))
                    mask |= JIM_EVENT_EXCEPTION;

                if (mask) {
                    int ret = fe->fileProc(interp, fe->clientData, mask);
                    if (ret != JIM_OK && ret != JIM_RETURN) {
                        /* Remove the element on handler error */
                        Jim_DeleteFileHandler(interp, fd, mask);
                        /* At this point fe is no longer valid - it will be assigned below */
                    }
                    processed++;
                    /* After an event is processed our file event list
                     * may no longer be the same, so what we do
                     * is to clear the bit for this file descriptor and
                     * restart again from the head. */
                    FD_CLR(fd, &rfds);
                    FD_CLR(fd, &wfds);
                    FD_CLR(fd, &efds);
                    fe = eventLoop->fileEventHead;
                }
                else {
                    fe = fe->next;
                }
            }
        }
    }
#else
    if (sleep_us > 0) {
        usleep(sleep_us);
    }
#endif

    /* Check time events */
    te = eventLoop->timeEventHead;
    maxId = eventLoop->timeEventNextId;
    while (te) {
        jim_wide id;

        if (te->id > maxId) {
            te = te->next;
            continue;
        }
        if (Jim_GetTimeUsec(CLOCK_MONOTONIC_RAW) >= te->when) {
            id = te->id;
            /* Remove from the list before executing */
            Jim_RemoveTimeHandler(eventLoop, id);
            te->timeProc(interp, te->clientData);
            /* After an event is processed our time event list may
             * no longer be the same, so we restart from head.
             * Still we make sure to don't process events registered
             * by event handlers itself in order to don't loop forever
             * even in case an [after 0] that continuously register
             * itself. To do so we saved the max ID we want to handle. */
            Jim_FreeTimeHandler(interp, te);

            te = eventLoop->timeEventHead;
            processed++;
        }
        else {
            te = te->next;
        }
    }

    return processed;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimELVwaitCommand`, lines 565–645. Retained reference `e4521e47b782c5165c471`.

```text
static int JimELVwaitCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_EventLoop *eventLoop = Jim_CmdPrivData(interp);
    Jim_Obj *oldValue = NULL;
    Jim_Obj *scriptObjPtr = NULL;
    int rc;
    int signal = 0;

    if (argc > 2 && Jim_CompareStringImmediate(interp, argv[1], "-signal")) {
        signal++;
    }

    if (argc - signal == 3) {
        scriptObjPtr = argv[2 + signal];
    }
    else if (argc - signal != 2) {
        return JIM_USAGE;
    }

    oldValue = Jim_GetGlobalVariable(interp, argv[1 + signal], JIM_NONE);

    if (oldValue) {
        Jim_IncrRefCount(oldValue);
    }
    else {
        /* If a result was left, it is an error */
        if (Jim_Length(Jim_GetResult(interp))) {
            return JIM_ERR;
        }
    }

    eventLoop->suppress_bgerror = 0;

    while ((rc = Jim_ProcessEvents(interp, JIM_ALL_EVENTS)) >= 0) {
        Jim_Obj *currValue;

        if (signal && interp->sigmask) {
            /* vwait -signal and handled signals were received, so transfer them
             * to ignored signals so that 'signal check -clear' will return them.
             * It's possible that if signals aren't supported we shouldn't even
             * allow the -signal option.
             */
#ifdef jim_ext_signal
            Jim_SignalSetIgnored(interp->sigmask);
#endif
            interp->sigmask = 0;
            break;
        }

        currValue = Jim_GetGlobalVariable(interp, argv[1 + signal], JIM_NONE);
        /* Stop the loop if the vwait-ed variable changed value,
         * or if was unset and now is set (or the contrary)
         * or if a signal was caught
         */
        if ((oldValue && !currValue) ||
            (!oldValue && currValue) ||
            (oldValue && currValue && !Jim_StringEqObj(oldValue, currValue)) ||
            Jim_CheckSignal(interp)) {
            break;
        }
        if (scriptObjPtr) {
            /* Stop the loop if a provided script returns BREAK or ERR */
            int retval = Jim_EvalObj(interp, scriptObjPtr);
            if (retval == JIM_ERR || retval == JIM_BREAK) {
                if (retval == JIM_ERR) {
                    rc = -2;
                }
                break;
            }
        }
    }
    if (oldValue)
        Jim_DecrRefCount(interp, oldValue);

    if (rc == -2) {
        return JIM_ERR;
    }

    Jim_SetEmptyResult(interp);
    return JIM_OK;
}
```

## Consumer bindings

- [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs), `NativeEventProtocol`: Pure independently selected event purpose; no original-object or callback-free grant.
- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `EventQueue`: Live original script-handle queue and current service-turn topology.
- [rust/tcl-vm/src/cmd_event.rs](../../../../rust/tcl-vm/src/cmd_event.rs), `cmd_vwait`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [runtime/rust/src/cmd_event.rs](../../../../runtime/rust/src/cmd_event.rs), `vwait_cmd`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [rust/tcl-vm/src/cmd_event/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_event_scripts_global_waits_and_timer_turns_match_native_controls` (linked): Finite code/result comparison for 21 retained source controls across six independently selected profiles (126 comparisons per port). No execution result is asserted by this binding.
- [runtime/rust/src/cmd_event/native_original_tests.rs](../../../../runtime/rust/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_event_scripts_global_waits_and_timer_turns_match_native_controls` (linked): Finite code/result comparison for 21 retained source controls across six independently selected profiles (126 comparisons per port). No execution result is asserted by this binding.

## Evidence

- [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19)
- [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18)
- [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4)
- [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0)
- [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED)
- [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED)
