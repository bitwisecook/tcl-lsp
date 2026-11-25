InfoObjectCallCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    CallContext *contextPtr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName methodName");
	return TCL_ERROR;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Get the call context and render its call chain.
     */

    contextPtr = TclOOGetCallContext(oPtr, objv[2], PUBLIC_METHOD, NULL, NULL,
	    NULL);
    if (contextPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"cannot construct any call chain", TCL_AUTO_LENGTH));
	OO_ERROR(interp, BAD_CALL_CHAIN);
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp,
	    TclOORenderCallChain(interp, contextPtr->callPtr));
    TclOODeleteContext(contextPtr);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * InfoClassCallCmd --
 *
 *	Implements [info class call $clsName $methodName]
 *
 * ----------------------------------------------------------------------
 */

static int
InfoClassCallCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr;
    CallChain *callPtr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "className methodName");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Get an render the stereotypical call chain.
     */

    callPtr = TclOOGetStereotypeCallChain(clsPtr, objv[2], PUBLIC_METHOD);
    if (callPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"cannot construct any call chain", TCL_AUTO_LENGTH));
	OO_ERROR(interp, BAD_CALL_CHAIN);
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, TclOORenderCallChain(interp, callPtr));
    TclOODeleteChain(callPtr);
    return TCL_OK;
}

/*
