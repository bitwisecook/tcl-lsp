# naming.event.original-background-prefix

Kind: `native-observation`

## Problem statement

Rendering a configured background-error prefix drops original arguments and omits the selected options operand.

## Question

With the correctly shaped handler {prefix message options}, what do these configured background-error callbacks return, and where does setup fail on unsupported providers?

## Conclusion

C85/C86/C90/C91 return PRE BOOM 1 for the separately pinned v107 handler, which receives the original fixed prefix, message and options. C84 rejects interp bgerror during setup; Jim rejects the interp invocation during setup, before reaching the callback. The wrong-arity v106 handler does not establish a callback failure law: modern C processes do not complete before the external deadline.

## Scope

Two bounded source setups per provider, modern C v107 callback observation and older/Jim setup rejection. Wrong-arity external timeouts are scoped separately. No default Tcl library bgerror equivalence, missing-handler fallback, C84 configured-prefix availability or Jim prefix-options claim. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4

Status: `unsupported`. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest. Version: tcl8.4 8.4.20.

v106/PREFIX_BGERROR: PREFIX_BGERROR: code 1, bytes b'bad option "bgerror": must be alias, aliases, create, delete, eval, exists, expose, hide, hidden, issafe, invokehidden, marktrusted, recursionlimit, slaves, share, target, or transfer' v107/GLOBAL_PREFIX_BGERROR: GLOBAL_PREFIX_BGERROR: code 1, bytes b'bad option "bgerror": must be alias, aliases, create, delete, eval, exists, expose, hide, hidden, issafe, invokehidden, marktrusted, recursionlimit, slaves, share, target, or transfer'

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

v106/PREFIX_BGERROR: external deadline; no guest completion row v107/GLOBAL_PREFIX_BGERROR: GLOBAL_PREFIX_BGERROR: code 0, bytes b'PRE BOOM 1'

### jim

Status: `unsupported`. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest. Version: jim 0.84-9-g5bac7c9.

v106/PREFIX_BGERROR: PREFIX_BGERROR: code 1, bytes b'wrong # args: should be "interp"' v107/GLOBAL_PREFIX_BGERROR: GLOBAL_PREFIX_BGERROR: code 1, bytes b'wrong # args: should be "interp"'

### bigip

Status: `not-tested`. Channel: not exercised.

No BIG-IP provider was executed or inspected for this Event question.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `AfterProc`, lines 1005–1052. Retained reference `e7ca34f2d6fd9344d4610`.

```text
AfterProc(clientData)
    ClientData clientData;	/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *) clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;
    char *script;
    int numBytes;

    /*
     * First remove the callback from our list of callbacks;  otherwise
     * someone could delete the callback while it's being executed, which
     * could cause a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve((ClientData) interp);
    script = Tcl_GetStringFromObj(afterPtr->commandPtr, &numBytes);
    result = Tcl_EvalEx(interp, script, numBytes, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundError(interp);
    }
    Tcl_Release((ClientData) interp);
    
    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    ckfree((char *) afterPtr);
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `AfterProc`, lines 1114–1158. Retained reference `eb83569215d92fa37da10`.

```text
AfterProc(
    ClientData clientData)	/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *) clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve((ClientData) interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	TclBackgroundException(interp, result);
    }
    Tcl_Release((ClientData) interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    ckfree((char *) afterPtr);
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `SlaveBgerror`, lines 2097–2116. Retained reference `ef8bb2ebdccb8192ef78d`.

```text
SlaveBgerror(
    Tcl_Interp *interp,		/* Interp for error return. */
    Tcl_Interp *slaveInterp,	/* Interp in which limit is set/queried. */
    int objc,			/* Set or Query. */
    Tcl_Obj *const objv[])	/* Argument strings. */
{
    if (objc) {
	int length;

	if (TCL_ERROR == TclListObjLength(NULL, objv[0], &length)
		|| (length < 1)) {
	    Tcl_AppendResult(interp, "cmdPrefix must be list of length >= 1",
		    NULL);
	    return TCL_ERROR;
	}
	TclSetBgErrorHandler(slaveInterp, objv[0]);
    }
    Tcl_SetObjResult(interp, TclGetBgErrorHandler(slaveInterp));
    return TCL_OK;
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `AfterProc`, lines 1168–1212. Retained reference `e5cad7b4101332ff96f7c`.

```text
AfterProc(
    ClientData clientData)	/* Describes command to execute. */
{
    AfterInfo *afterPtr = clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve(interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundException(interp, result);
    }
    Tcl_Release(interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    ckfree(afterPtr);
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `ChildBgerror`, lines 2268–2289. Retained reference `ea9bf8d7d5c098c478df8`.

```text
ChildBgerror(
    Tcl_Interp *interp,		/* Interp for error return. */
    Tcl_Interp *childInterp,	/* Interp in which limit is set/queried. */
    int objc,			/* Set or Query. */
    Tcl_Obj *const objv[])	/* Argument strings. */
{
    if (objc) {
	int length;

	if (TCL_ERROR == TclListObjLength(NULL, objv[0], &length)
		|| (length < 1)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "cmdPrefix must be list of length >= 1", -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
		    "BGERRORFORMAT", (char *)NULL);
	    return TCL_ERROR;
	}
	TclSetBgErrorHandler(childInterp, objv[0]);
    }
    Tcl_SetObjResult(interp, TclGetBgErrorHandler(childInterp));
    return TCL_OK;
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `AfterProc`, lines 1147–1191. Retained reference `e4d5b46a62af04d177edb`.

```text
AfterProc(
    void *clientData)		/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *)clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve(interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundException(interp, result);
    }
    Tcl_Release(interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    Tcl_Free(afterPtr);
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `ChildBgerror`, lines 2342–2363. Retained reference `ea3de3c863d05700ccfc3`.

```text
ChildBgerror(
    Tcl_Interp *interp,		/* Interp for error return. */
    Tcl_Interp *childInterp,	/* Interp in which limit is set/queried. */
    Tcl_Size objc,		/* Set or Query. */
    Tcl_Obj *const objv[])	/* Argument strings. */
{
    if (objc) {
	Tcl_Size length;

	if (TCL_ERROR == TclListObjLength(NULL, objv[0], &length)
		|| (length < 1)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "cmdPrefix must be list of length >= 1", -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
		    "BGERRORFORMAT", (char *)NULL);
	    return TCL_ERROR;
	}
	TclSetBgErrorHandler(childInterp, objv[0]);
    }
    Tcl_SetObjResult(interp, TclGetBgErrorHandler(childInterp));
    return TCL_OK;
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `AfterProc`, lines 1533–1577. Retained reference `eaf331dbfda81fff6c4db`.

```text
AfterProc(
    void *clientData)		/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *)clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve(interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundException(interp, result);
    }
    Tcl_Release(interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    Tcl_Free(afterPtr);
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `ChildBgerror`, lines 2727–2748. Retained reference `e791ac2cd6ff96da72529`.

```text
ChildBgerror(
    Tcl_Interp *interp,		/* Interp for error return. */
    Tcl_Interp *childInterp,	/* Interp in which limit is set/queried. */
    Tcl_Size objc,		/* Set or Query. */
    Tcl_Obj *const *objv)	/* Argument strings. */
{
    if (objc) {
	Tcl_Size length;

	if (TCL_ERROR == TclListObjLength(NULL, objv[0], &length)
		|| (length < 1)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "cmdPrefix must be list of length >= 1", -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
		    "BGERRORFORMAT", (char *)NULL);
	    return TCL_ERROR;
	}
	TclSetBgErrorHandler(childInterp, objv[0]);
    }
    Tcl_SetObjResult(interp, TclGetBgErrorHandler(childInterp));
    return TCL_OK;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `Jim_EvalObjBackground`, lines 112–148. Retained reference `e4521e47b782c5165c471`.

```text
int Jim_EvalObjBackground(Jim_Interp *interp, Jim_Obj *scriptObjPtr)
{
    Jim_EventLoop *eventLoop = Jim_GetAssocData(interp, "eventloop");
    Jim_CallFrame *savedFramePtr;
    int retval;

    savedFramePtr = interp->framePtr;
    interp->framePtr = interp->topFramePtr;
    retval = Jim_EvalObj(interp, scriptObjPtr);
    interp->framePtr = savedFramePtr;
    /* Try to report the error (if any) via the bgerror proc */
    if (retval != JIM_OK && retval != JIM_RETURN && !eventLoop->suppress_bgerror) {
        Jim_Obj *objv[2];
        int rc = JIM_ERR;

        objv[0] = Jim_NewStringObj(interp, "bgerror", -1);
        objv[1] = Jim_GetResult(interp);
        Jim_IncrRefCount(objv[0]);
        Jim_IncrRefCount(objv[1]);
        if (Jim_GetCommand(interp, objv[0], JIM_NONE) == NULL || (rc = Jim_EvalObjVector(interp, 2, objv)) != JIM_OK) {
            if (rc == JIM_BREAK) {
                /* No more bgerror calls */
                eventLoop->suppress_bgerror++;
            }
            else {
                /* Report the error to stderr. */
                Jim_MakeErrorMessage(interp);
                fprintf(stderr, "%s\n", Jim_String(Jim_GetResult(interp)));
                /* And reset the result */
                Jim_SetResultString(interp, "", -1);
            }
        }
        Jim_DecrRefCount(interp, objv[0]);
        Jim_DecrRefCount(interp, objv[1]);
    }
    return retval;
}
```

## Consumer bindings

- [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs), `NativeEventProtocol`: Pure independently selected event purpose; no original-object or callback-free grant.
- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `EventQueue`: Live original script-handle queue and current service-turn topology.
- [rust/tcl-vm/src/cmd_event.rs](../../../../rust/tcl-vm/src/cmd_event.rs), `report_bg_error`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `process_bg_errors`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.

## Evidence

- [rust/tcl-registry/tests/data/native_event_original/v106/capture](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_event_original/v107/capture](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture)
