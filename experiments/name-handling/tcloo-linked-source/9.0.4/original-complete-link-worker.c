TclOOLinkObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    /* Set up common bits. */
    CallFrame *framePtr = ((Interp *) interp)->varFramePtr;
    CallContext *context;
    Object *oPtr;
    Tcl_Obj *myCmd, **linkv, *src, *dst;
    Tcl_Size linkc;
    const char *srcStr;
    int i;

    if (framePtr == NULL || !(framePtr->isProcCallFrame & FRAME_IS_METHOD)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"%s may only be called from inside a method",
		TclGetString(objv[0])));
	OO_ERROR(interp, CONTEXT_REQUIRED);
	return TCL_ERROR;
    }
    context = (CallContext *) framePtr->clientData;
    oPtr = context->oPtr;
    if (!oPtr->myCommand) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"cannot link to non-existent callback handle"));
	OO_ERROR(interp, MY_GONE);
	return TCL_ERROR;
    }
    myCmd = Tcl_NewObj();
    Tcl_GetCommandFullName(interp, oPtr->myCommand, myCmd);
    if (!oPtr->linkedCmdsList) {
	oPtr->linkedCmdsList = Tcl_NewListObj(0, NULL);
	Tcl_IncrRefCount(oPtr->linkedCmdsList);
    }

    /* For each argument */
    for (i=1; i<objc; i++) {
	/* Parse as list of (one or) two items: source and destination names */
	if (TclListObjGetElements(interp, objv[i], &linkc, &linkv) != TCL_OK) {
	    Tcl_BounceRefCount(myCmd);
	    return TCL_ERROR;
	}
	switch (linkc) {
	case 1:
	    /* Degenerate case */
	    src = dst = linkv[0];
	    break;
	case 2:
	    src = linkv[0];
	    dst = linkv[1];
	    break;
	default:
	    Tcl_BounceRefCount(myCmd);
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad link description; must only have one or two elements"));
	    OO_ERROR(interp, CMDLINK_FORMAT);
	    return TCL_ERROR;
	}

	/* Qualify the source if necessary */
	srcStr = TclGetString(src);
	if (srcStr[0] != ':' || srcStr[1] != ':') {
	    src = Tcl_ObjPrintf("%s::%s",
		    context->oPtr->namespacePtr->fullName, srcStr);
	}

	/* Make the alias command */
	if (TclAliasCreate(interp, interp, interp, src, myCmd, 1, &dst) != TCL_OK) {
	    Tcl_BounceRefCount(myCmd);
	    Tcl_BounceRefCount(src);
	    return TCL_ERROR;
	}

	/* Remember the alias for cleanup if necessary */
	Tcl_ListObjAppendElement(NULL, oPtr->linkedCmdsList, src);
    }
    Tcl_BounceRefCount(myCmd);
    return TCL_OK;
}