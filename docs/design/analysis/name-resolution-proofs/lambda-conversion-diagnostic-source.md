# naming.lambda.conversion-diagnostic-source

Kind: `source-anchor`

## Problem statement

Conversion result formatting, formal error frames and list/body/cache operations must not borrow one another's extents or authority.

## Question

Which selected C/Jim implementation formats a lambda conversion failure and C formal-parse frame?

## Conclusion

Pinned C8.5 SetLambdaFromAny appends the original object into the diagnostic. C8.6/C9 use Tcl_ObjPrintf with a CString operand and set TCL VALUE LAMBDA. All inspected C releases append the formal-parse lambda frame through CString printf. Jim_ApplyCoreCommand selects its own list-length and %#s diagnostic path. Later C length/element scheduling, cached lambda Proc, body preparation and namespace selection remain independent.

## Scope

Exact retained full-function LF windows for C8.5.19/8.6.18/9.0.4/9.1.0 and Jim pinned5bac7c9. C8.4 source function unavailable/not inspected for this recipe. Source inspection grants no executed object representation/callback/cache or successful body/Normal claim.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Pinned implementation function window; no native invocation. Version: tcl8.4 8.4.20.

No C8.4 apply implementation window inspected.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `inspected`. Channel: Pinned implementation function window; no native invocation. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

Pinned C8.5 SetLambdaFromAny appends the original object into the diagnostic. C8.6/C9 use Tcl_ObjPrintf with a CString operand and set TCL VALUE LAMBDA. All inspected C releases append the formal-parse lambda frame through CString printf. Jim_ApplyCoreCommand selects its own list-length and %#s diagnostic path. Later C length/element scheduling, cached lambda Proc, body preparation and namespace selection remain independent.

### bigip

Status: `not-tested`.

No appliance observation or source implementation claim.

## Source inspection

tcl8.5 8.5.19, revision `recorded configured source association; Jim 5bac7c9`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2444–2618. Retained reference `source-tcl8.5`.

```text
SetLambdaFromAny(
    Tcl_Interp *interp,		/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr)	/* The object to convert. */
{
    Interp *iPtr = (Interp *) interp;
    char *name;
    Tcl_Obj *argsPtr, *bodyPtr, *nsObjPtr, **objv, *errPtr;
    int isNew, objc, result;
    CmdFrame *cfPtr = NULL;
    Proc *procPtr;

    if (interp == NULL) {
	return TCL_ERROR;
    }

    /*
     * Convert objPtr to list type first; if it cannot be converted, or if its
     * length is not 2, then it cannot be converted to lambdaType.
     */

    result = TclListObjGetElements(NULL, objPtr, &objc, &objv);
    if ((result != TCL_OK) || ((objc != 2) && (objc != 3))) {
	TclNewLiteralStringObj(errPtr, "can't interpret \"");
	Tcl_AppendObjToObj(errPtr, objPtr);
	Tcl_AppendToObj(errPtr, "\" as a lambda expression", -1);
	Tcl_SetObjResult(interp, errPtr);
	return TCL_ERROR;
    }

    argsPtr = objv[0];
    bodyPtr = objv[1];

    /*
     * Create and initialize the Proc struct. The cmdPtr field is set to NULL
     * to signal that this is an anonymous function.
     */

    name = TclGetString(objPtr);

    if (TclCreateProc(interp, /*ignored nsPtr*/ NULL, name, argsPtr, bodyPtr,
	    &procPtr) != TCL_OK) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (parsing lambda expression \"%s\")", name));
	return TCL_ERROR;
    }

    /*
     * CAREFUL: TclCreateProc returns refCount==1! [Bug 1578454]
     * procPtr->refCount = 1;
     */

    procPtr->cmdPtr = NULL;

    /*
     * TIP #280: Remember the line the apply body is starting on. In a Byte
     * code context we ask the engine to provide us with the necessary
     * information. This is for the initialization of the byte code compiler
     * when the body is used for the first time.
     *
     * NOTE: The body is the second word in the 'objPtr'. Its location,
     * accessible through 'context.line[1]' (see below) is therefore only the
     * first approximation of the actual line the body is on. We have to use
     * the string rep of the 'objPtr' to determine the exact line. This is
     * available already through 'name'. Use 'TclListLines', see 'switch'
     * (tclCmdMZ.c).
     *
     * This code is nearly identical to the #280 code in Tcl_ProcObjCmd, see
     * this file. The differences are the different index of the body in the
     * line array of the context, and the special processing mentioned in the
     * previous paragraph to track into the list. Find a way to factor the
     * common elements into a single function.
     */

    if (iPtr->cmdFramePtr) {
	CmdFrame *contextPtr;

	contextPtr = (CmdFrame *) TclStackAlloc(interp, sizeof(CmdFrame));
	*contextPtr = *iPtr->cmdFramePtr;

	if (contextPtr->type == TCL_LOCATION_BC) {
	    /*
	     * Retrieve the source context from the bytecode. This call
	     * accounts for the reference to the source file, if any, held in
	     * 'context.data.eval.path'.
	     */

	    TclGetSrcInfoForPc(contextPtr);
	} else if (contextPtr->type == TCL_LOCATION_SOURCE) {
	    /*
	     * We created a new reference to the source file path name when we
	     * created 'context' above. Account for the reference.
	     */

	    Tcl_IncrRefCount(contextPtr->data.eval.path);

	}

	if (contextPtr->type == TCL_LOCATION_SOURCE) {
	    /*
	     * We can record source location within a lambda only if the body
	     * was not created by substitution.
	     */

	    if (contextPtr->line
		    && (contextPtr->nline >= 2) && (contextPtr->line[1] >= 0)) {
		int buf[2];

		/*
		 * Move from approximation (line of list cmd word) to actual
		 * location (line of 2nd list element).
		 */

		cfPtr = (CmdFrame *) ckalloc(sizeof(CmdFrame));
		TclListLines(objPtr, contextPtr->line[1], 2, buf, NULL);

		cfPtr->level = -1;
		cfPtr->type = contextPtr->type;
		cfPtr->line = (int *) ckalloc(sizeof(int));
		cfPtr->line[0] = buf[1];
		cfPtr->nline = 1;
		cfPtr->framePtr = NULL;
		cfPtr->nextPtr = NULL;

		cfPtr->data.eval.path = contextPtr->data.eval.path;
		Tcl_IncrRefCount(cfPtr->data.eval.path);

		cfPtr->cmd.str.cmd = NULL;
		cfPtr->cmd.str.len = 0;
	    }

	    /*
	     * 'contextPtr' is going out of scope. Release the reference that
	     * it's holding to the source file path
	     */

	    Tcl_DecrRefCount(contextPtr->data.eval.path);
	}
	TclStackFree(interp, contextPtr);
    }
    Tcl_SetHashValue(Tcl_CreateHashEntry(iPtr->linePBodyPtr, (char *) procPtr,
	    &isNew), cfPtr);

    /*
     * Set the namespace for this lambda: given by objv[2] understood as a
     * global reference, or else global per default.
     */

    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	char *nsName = TclGetString(objv[2]);

	if ((*nsName != ':') || (*(nsName+1) != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }

    Tcl_IncrRefCount(nsObjPtr);

    /*
     * Free the list internalrep of objPtr - this will free argsPtr, but
     * bodyPtr retains a reference from the Proc structure. Then finish the
     * conversion to lambdaType.
     */

    objPtr->typePtr->freeIntRepProc(objPtr);

    objPtr->internalRep.twoPtrValue.ptr1 = procPtr;
    objPtr->internalRep.twoPtrValue.ptr2 = nsObjPtr;
    objPtr->typePtr = &lambdaType;
    return TCL_OK;
}
```

tcl8.6 8.6.18, revision `recorded configured source association; Jim 5bac7c9`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2424–2596. Retained reference `source-tcl8.6`.

```text
SetLambdaFromAny(
    Tcl_Interp *interp,		/* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr)	/* The object to convert. */
{
    Interp *iPtr = (Interp *) interp;
    const char *name;
    Tcl_Obj *argsPtr, *bodyPtr, *nsObjPtr, **objv;
    int isNew, objc, result;
    CmdFrame *cfPtr = NULL;
    Proc *procPtr;

    if (interp == NULL) {
	return TCL_ERROR;
    }

    /*
     * Convert objPtr to list type first; if it cannot be converted, or if its
     * length is not 2, then it cannot be converted to tclLambdaType.
     */

    result = TclListObjGetElements(NULL, objPtr, &objc, &objv);
    if ((result != TCL_OK) || ((objc != 2) && (objc != 3))) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't interpret \"%s\" as a lambda expression",
		TclGetString(objPtr)));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "LAMBDA", (char *)NULL);
	return TCL_ERROR;
    }

    argsPtr = objv[0];
    bodyPtr = objv[1];

    /*
     * Create and initialize the Proc struct. The cmdPtr field is set to NULL
     * to signal that this is an anonymous function.
     */

    name = TclGetString(objPtr);

    if (TclCreateProc(interp, /*ignored nsPtr*/ NULL, name, argsPtr, bodyPtr,
	    &procPtr) != TCL_OK) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (parsing lambda expression \"%s\")", name));
	return TCL_ERROR;
    }

    /*
     * CAREFUL: TclCreateProc returns refCount==1! [Bug 1578454]
     * procPtr->refCount = 1;
     */

    procPtr->cmdPtr = NULL;

    /*
     * TIP #280: Remember the line the apply body is starting on. In a Byte
     * code context we ask the engine to provide us with the necessary
     * information. This is for the initialization of the byte code compiler
     * when the body is used for the first time.
     *
     * NOTE: The body is the second word in the 'objPtr'. Its location,
     * accessible through 'context.line[1]' (see below) is therefore only the
     * first approximation of the actual line the body is on. We have to use
     * the string rep of the 'objPtr' to determine the exact line. This is
     * available already through 'name'. Use 'TclListLines', see 'switch'
     * (tclCmdMZ.c).
     *
     * This code is nearly identical to the #280 code in Tcl_ProcObjCmd, see
     * this file. The differences are the different index of the body in the
     * line array of the context, and the special processing mentioned in the
     * previous paragraph to track into the list. Find a way to factor the
     * common elements into a single function.
     */

    if (iPtr->cmdFramePtr) {
	CmdFrame *contextPtr = (CmdFrame *)TclStackAlloc(interp, sizeof(CmdFrame));

	*contextPtr = *iPtr->cmdFramePtr;
	if (contextPtr->type == TCL_LOCATION_BC) {
	    /*
	     * Retrieve the source context from the bytecode. This call
	     * accounts for the reference to the source file, if any, held in
	     * 'context.data.eval.path'.
	     */

	    TclGetSrcInfoForPc(contextPtr);
	} else if (contextPtr->type == TCL_LOCATION_SOURCE) {
	    /*
	     * We created a new reference to the source file path name when we
	     * created 'context' above. Account for the reference.
	     */

	    Tcl_IncrRefCount(contextPtr->data.eval.path);

	}

	if (contextPtr->type == TCL_LOCATION_SOURCE) {
	    /*
	     * We can record source location within a lambda only if the body
	     * was not created by substitution.
	     */

	    if (contextPtr->line
		    && (contextPtr->nline >= 2) && (contextPtr->line[1] >= 0)) {
		int buf[2];

		/*
		 * Move from approximation (line of list cmd word) to actual
		 * location (line of 2nd list element).
		 */

		cfPtr = (CmdFrame *)ckalloc(sizeof(CmdFrame));
		TclListLines(objPtr, contextPtr->line[1], 2, buf, NULL);

		cfPtr->level = -1;
		cfPtr->type = contextPtr->type;
		cfPtr->line = (int *)ckalloc(sizeof(int));
		cfPtr->line[0] = buf[1];
		cfPtr->nline = 1;
		cfPtr->framePtr = NULL;
		cfPtr->nextPtr = NULL;

		cfPtr->data.eval.path = contextPtr->data.eval.path;
		Tcl_IncrRefCount(cfPtr->data.eval.path);

		cfPtr->cmd = NULL;
		cfPtr->len = 0;
	    }

	    /*
	     * 'contextPtr' is going out of scope. Release the reference that
	     * it's holding to the source file path
	     */

	    Tcl_DecrRefCount(contextPtr->data.eval.path);
	}
	TclStackFree(interp, contextPtr);
    }
    Tcl_SetHashValue(Tcl_CreateHashEntry(iPtr->linePBodyPtr, procPtr,
	    &isNew), cfPtr);

    /*
     * Set the namespace for this lambda: given by objv[2] understood as a
     * global reference, or else global per default.
     */

    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	const char *nsName = TclGetString(objv[2]);

	if ((*nsName != ':') || (*(nsName+1) != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }

    Tcl_IncrRefCount(nsObjPtr);

    /*
     * Free the list internalrep of objPtr - this will free argsPtr, but
     * bodyPtr retains a reference from the Proc structure. Then finish the
     * conversion to tclLambdaType.
     */

    TclFreeIntRep(objPtr);

    objPtr->internalRep.twoPtrValue.ptr1 = procPtr;
    objPtr->internalRep.twoPtrValue.ptr2 = nsObjPtr;
    objPtr->typePtr = &tclLambdaType;
    return TCL_OK;
}
```

tcl9.0 9.0.4, revision `recorded configured source association; Jim 5bac7c9`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2449–2624. Retained reference `source-tcl9.0`.

```text
SetLambdaFromAny(
    Tcl_Interp *interp,		/* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr)		/* The object to convert. */
{
    Interp *iPtr = (Interp *) interp;
    const char *name;
    Tcl_Obj *argsPtr, *bodyPtr, *nsObjPtr, **objv;
    int isNew, result;
    Tcl_Size objc;
    CmdFrame *cfPtr = NULL;
    Proc *procPtr;

    if (interp == NULL) {
	return TCL_ERROR;
    }

    /*
     * Convert objPtr to list type first; if it cannot be converted, or if its
     * length is not 2, then it cannot be converted to lambdaType.
     */

    result = TclListObjLength(NULL, objPtr, &objc);
    if ((result != TCL_OK) || ((objc != 2) && (objc != 3))) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't interpret \"%s\" as a lambda expression",
		Tcl_GetString(objPtr)));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "LAMBDA", (char *)NULL);
	return TCL_ERROR;
    }
    result = TclListObjGetElements(NULL, objPtr, &objc, &objv);
    if ((result != TCL_OK) || ((objc != 2) && (objc != 3))) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't interpret \"%s\" as a lambda expression",
		TclGetString(objPtr)));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "LAMBDA", (char *)NULL);
	return TCL_ERROR;
    }

    argsPtr = objv[0];
    bodyPtr = objv[1];

    /*
     * Create and initialize the Proc struct. The cmdPtr field is set to NULL
     * to signal that this is an anonymous function.
     */

    name = TclGetString(objPtr);

    if (TclCreateProc(interp, /*ignored nsPtr*/ NULL, name, argsPtr, bodyPtr,
	    &procPtr) != TCL_OK) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (parsing lambda expression \"%s\")", name));
	return TCL_ERROR;
    }

    /*
     * CAREFUL: TclCreateProc returns refCount==1! [Bug 1578454]
     * procPtr->refCount = 1;
     */

    procPtr->cmdPtr = NULL;

    /*
     * TIP #280: Remember the line the apply body is starting on. In a Byte
     * code context we ask the engine to provide us with the necessary
     * information. This is for the initialization of the byte code compiler
     * when the body is used for the first time.
     *
     * NOTE: The body is the second word in the 'objPtr'. Its location,
     * accessible through 'context.line[1]' (see below) is therefore only the
     * first approximation of the actual line the body is on. We have to use
     * the string rep of the 'objPtr' to determine the exact line. This is
     * available already through 'name'. Use 'TclListLines', see 'switch'
     * (tclCmdMZ.c).
     *
     * This code is nearly identical to the #280 code in Tcl_ProcObjCmd, see
     * this file. The differences are the different index of the body in the
     * line array of the context, and the special processing mentioned in the
     * previous paragraph to track into the list. Find a way to factor the
     * common elements into a single function.
     */

    if (iPtr->cmdFramePtr) {
	CmdFrame *contextPtr = (CmdFrame *)TclStackAlloc(interp, sizeof(CmdFrame));

	*contextPtr = *iPtr->cmdFramePtr;
	if (contextPtr->type == TCL_LOCATION_BC) {
	    /*
	     * Retrieve the source context from the bytecode. This call
	     * accounts for the reference to the source file, if any, held in
	     * 'context.data.eval.path'.
	     */

	    TclGetSrcInfoForPc(contextPtr);
	} else if (contextPtr->type == TCL_LOCATION_SOURCE) {
	    /*
	     * We created a new reference to the source file path name when we
	     * created 'context' above. Account for the reference.
	     */

	    Tcl_IncrRefCount(contextPtr->data.eval.path);

	}

	if (contextPtr->type == TCL_LOCATION_SOURCE) {
	    /*
	     * We can record source location within a lambda only if the body
	     * was not created by substitution.
	     */

	    if (contextPtr->line
		    && (contextPtr->nline >= 2) && (contextPtr->line[1] >= 0)) {
		Tcl_Size buf[2];

		/*
		 * Move from approximation (line of list cmd word) to actual
		 * location (line of 2nd list element).
		 */

		cfPtr = (CmdFrame *)Tcl_Alloc(sizeof(CmdFrame));
		TclListLines(objPtr, contextPtr->line[1], 2, buf, NULL);

		cfPtr->level = -1;
		cfPtr->type = contextPtr->type;
		cfPtr->line = (Tcl_Size *)Tcl_Alloc(sizeof(Tcl_Size));
		cfPtr->line[0] = buf[1];
		cfPtr->nline = 1;
		cfPtr->framePtr = NULL;
		cfPtr->nextPtr = NULL;

		cfPtr->data.eval.path = contextPtr->data.eval.path;
		Tcl_IncrRefCount(cfPtr->data.eval.path);

		cfPtr->cmd = NULL;
		cfPtr->len = 0;
	    }

	    /*
	     * 'contextPtr' is going out of scope. Release the reference that
	     * it's holding to the source file path
	     */

	    Tcl_DecrRefCount(contextPtr->data.eval.path);
	}
	TclStackFree(interp, contextPtr);
    }
    Tcl_SetHashValue(Tcl_CreateHashEntry(iPtr->linePBodyPtr, procPtr,
	    &isNew), cfPtr);

    /*
     * Set the namespace for this lambda: given by objv[2] understood as a
     * global reference, or else global per default.
     */

    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	const char *nsName = TclGetString(objv[2]);

	if ((nsName[0] != ':') || (nsName[1] != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }

    /*
     * Free the list internalrep of objPtr - this will free argsPtr, but
     * bodyPtr retains a reference from the Proc structure. Then finish the
     * conversion to lambdaType.
     */

    LambdaSetInternalRep(objPtr, procPtr, nsObjPtr);
    return TCL_OK;
}
```

tcl9.1 9.1.0, revision `recorded configured source association; Jim 5bac7c9`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclProc.c`, function `SetLambdaFromAny`, lines 2458–2651. Retained reference `source-tcl9.1`.

```text
SetLambdaFromAny(
    Tcl_Interp *interp,		/* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr)		/* The object to convert. */
{
    Interp *iPtr = (Interp *) interp;
    const char *name;
    Tcl_Obj *argsPtr, *bodyPtr, *nsObjPtr, **objv;
    int result;
    Tcl_Size objc;
    CmdFrame *cfPtr = NULL;
    Proc *procPtr;
    Tcl_Namespace *nsPtr;
    Command *cmdPtr;

    if (interp == NULL) {
	return TCL_ERROR;
    }

    /*
     * Convert objPtr to list type first; if it cannot be converted, or if its
     * length is not 2, then it cannot be converted to lambdaType.
     */

    result = TclListObjLength(NULL, objPtr, &objc);
    if ((result != TCL_OK) || ((objc != 2) && (objc != 3))) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't interpret \"%s\" as a lambda expression",
		Tcl_GetString(objPtr)));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "LAMBDA", (char *)NULL);
	return TCL_ERROR;
    }
    result = TclListObjGetElements(NULL, objPtr, &objc, &objv);
    if ((result != TCL_OK) || ((objc != 2) && (objc != 3))) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't interpret \"%s\" as a lambda expression",
		TclGetString(objPtr)));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "LAMBDA", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Set the namespace for this lambda: given by objv[2] understood as a
     * global reference, or else global per default.
     */

    if (objc == 2) {
	TclNewLiteralStringObj(nsObjPtr, "::");
    } else {
	const char *nsName = TclGetString(objv[2]);

	if ((*nsName != ':') || (*(nsName+1) != ':')) {
	    TclNewLiteralStringObj(nsObjPtr, "::");
	    Tcl_AppendObjToObj(nsObjPtr, objv[2]);
	} else {
	    nsObjPtr = objv[2];
	}
    }
    Tcl_IncrRefCount(nsObjPtr);
    /* Find the namespace where this lambda should run. */
    result = TclGetNamespaceFromObj(interp, nsObjPtr, &nsPtr);
    if (result != TCL_OK) {
	Tcl_DecrRefCount(nsObjPtr);
	return TCL_ERROR;
    }

    argsPtr = objv[0];
    bodyPtr = objv[1];

    /*
     * Create and initialize the Proc struct. The cmdPtr field is set to
     * artificial command owned by the procPtr with few info, e. g. used
     * in TclInfoFrame.
     */

    name = TclGetString(objPtr);

    if (TclCreateProc(interp, /*ignored nsPtr*/ NULL, name, argsPtr, bodyPtr,
	    &procPtr) != TCL_OK) {
	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (parsing lambda expression \"%s\")", name));
	Tcl_DecrRefCount(nsObjPtr);
	return TCL_ERROR;
    }

    /*
     * CAREFUL: TclCreateProc returns refCount==1! [Bug 1578454]
     * procPtr->refCount = 1;
     */

    cmdPtr = (Command *)Tcl_Alloc(sizeof(Command));
    memset(cmdPtr, 0, sizeof(*cmdPtr));
    cmdPtr->nsPtr = (Namespace *) nsPtr;
    ((Namespace *)nsPtr)->refCount++;
    cmdPtr->objClientData2 = objPtr;
    cmdPtr->refCount++;
    procPtr->cmdPtr = cmdPtr;
    procPtr->flags = PROC_CMD_OWNED;

    /*
     * TIP #280: Remember the line the apply body is starting on. In a Byte
     * code context we ask the engine to provide us with the necessary
     * information. This is for the initialization of the byte code compiler
     * when the body is used for the first time.
     *
     * NOTE: The body is the second word in the 'objPtr'. Its location,
     * accessible through 'context.line[1]' (see below) is therefore only the
     * first approximation of the actual line the body is on. We have to use
     * the string rep of the 'objPtr' to determine the exact line. This is
     * available already through 'name'. Use 'TclListLines', see 'switch'
     * (tclCmdMZ.c).
     *
     * This code is nearly identical to the #280 code in Tcl_ProcObjCmd, see
     * this file. The differences are the different index of the body in the
     * line array of the context, and the special processing mentioned in the
     * previous paragraph to track into the list. Find a way to factor the
     * common elements into a single function.
     */

    if (iPtr->cmdFramePtr) {
	CmdFrame *contextPtr = (CmdFrame *)TclStackAlloc(interp, sizeof(CmdFrame));

	*contextPtr = *iPtr->cmdFramePtr;
	if (contextPtr->type == TCL_LOCATION_BC) {
	    /*
	     * Retrieve the source context from the bytecode. This call
	     * accounts for the reference to the source file, if any, held in
	     * 'context.data.eval.path'.
	     */

	    TclGetSrcInfoForPc(contextPtr);
	} else if (contextPtr->type == TCL_LOCATION_SOURCE) {
	    /*
	     * We created a new reference to the source file path name when we
	     * created 'context' above. Account for the reference.
	     */

	    Tcl_IncrRefCount(contextPtr->data.eval.path);
	}

	if (contextPtr->type == TCL_LOCATION_SOURCE) {
	    /*
	     * We can record source location within a lambda only if the body
	     * was not created by substitution.
	     */

	    if (contextPtr->line
		    && (contextPtr->nline >= 2) && (contextPtr->line[1] >= 0)) {
		int buf[2];

		/*
		 * Move from approximation (line of list cmd word) to actual
		 * location (line of 2nd list element).
		 */

		cfPtr = (CmdFrame *)Tcl_Alloc(sizeof(CmdFrame));
		TclListLines(objPtr, contextPtr->line[1], 2, buf, NULL);

		cfPtr->level = -1;
		cfPtr->type = contextPtr->type;
		cfPtr->line = (int *)Tcl_Alloc(sizeof(int));
		cfPtr->line[0] = buf[1];
		cfPtr->nline = 1;
		cfPtr->framePtr = NULL;
		cfPtr->nextPtr = NULL;

		cfPtr->data.eval.path = contextPtr->data.eval.path;
		Tcl_IncrRefCount(cfPtr->data.eval.path);

		cfPtr->cmd = NULL;
		cfPtr->len = 0;
	    }

	    /*
	     * 'contextPtr' is going out of scope. Release the reference that
	     * it's holding to the source file path
	     */

	    Tcl_DecrRefCount(contextPtr->data.eval.path);
	}
	TclStackFree(interp, contextPtr);
    }
    Tcl_SetHashValue(Tcl_CreateHashEntry(iPtr->linePBodyPtr, procPtr,
	    NULL), cfPtr);

    /*
     * Free the list internalrep of objPtr - this will free argsPtr, but
     * bodyPtr retains a reference from the Proc structure. Then finish the
     * conversion to lambdaType.
     */

    LambdaSetInternalRep(objPtr, procPtr, nsObjPtr);
    Tcl_DecrRefCount(nsObjPtr);
    return TCL_OK;
}
```

jim 0.84-9-g5bac7c9, revision `recorded configured source association; Jim 5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_ApplyCoreCommand`, lines 14546–14589. Retained reference `source-jim`.

```text
static int Jim_ApplyCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int ret;
    Jim_Cmd *cmd;
    Jim_Obj *argListObjPtr;
    Jim_Obj *bodyObjPtr;
    Jim_Obj *nsObj = NULL;
    Jim_Obj **nargv;

    int len = Jim_ListLength(interp, argv[1]);
    if (len != 2 && len != 3) {
        Jim_SetResultFormatted(interp, "can't interpret \"%#s\" as a lambda expression", argv[1]);
        return JIM_ERR;
    }

    if (len == 3) {
#ifdef jim_ext_namespace
        /* Note that the namespace is always treated as global */
        nsObj = Jim_ListGetIndex(interp, argv[1], 2);
#else
        Jim_SetResultString(interp, "namespaces not enabled", -1);
        return JIM_ERR;
#endif
    }
    argListObjPtr = Jim_ListGetIndex(interp, argv[1], 0);
    bodyObjPtr = Jim_ListGetIndex(interp, argv[1], 1);

    cmd = JimCreateProcedureCmd(interp, argListObjPtr, NULL, bodyObjPtr, nsObj);

    if (cmd) {
        /* Create a new argv array with a dummy argv[0], for error messages */
        nargv = Jim_Alloc((argc - 2 + 1) * sizeof(*nargv));
        nargv[0] = Jim_NewStringObj(interp, "apply lambdaExpr", -1);
        Jim_IncrRefCount(nargv[0]);
        memcpy(&nargv[1], argv + 2, (argc - 2) * sizeof(*nargv));
        ret = JimCallProcedure(interp, cmd, argc - 2 + 1, nargv);
        Jim_DecrRefCount(interp, nargv[0]);
        Jim_Free(nargv);

        JimDecrCmdRefCount(interp, cmd);
        return ret;
    }
    return JIM_ERR;
}
```

## Consumer bindings

- [rust/tcl-registry/src/native_lambda.rs](../../../../rust/tcl-registry/src/native_lambda.rs), `NativeLambdaDiagnosticProtocol::select`: Independently selected exact engine diagnostic recipe.
- [rust/tcl-registry/src/native_lambda.rs](../../../../rust/tcl-registry/src/native_lambda.rs), `NativeLambdaDiagnosticProtocol::conversion_error`: Counted C85 versus later C/Jim CString guest wording and error-code action.
- [rust/tcl-registry/src/native_lambda.rs](../../../../rust/tcl-registry/src/native_lambda.rs), `NativeLambdaDiagnosticProtocol::parameter_error_frame`: Separate C-only CString formal parse frame.

## Evidence

- [rust/tcl-registry/tests/data/native_lambda_diagnostics/8.5.19-source.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/8.5.19-source.json)
- [rust/tcl-registry/tests/data/native_lambda_diagnostics/8.6.18-source.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/8.6.18-source.json)
- [rust/tcl-registry/tests/data/native_lambda_diagnostics/9.0.4-source.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/9.0.4-source.json)
- [rust/tcl-registry/tests/data/native_lambda_diagnostics/9.1.0-source.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/9.1.0-source.json)
- [rust/tcl-registry/tests/data/native_lambda_diagnostics/jim-source.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/jim-source.json)
