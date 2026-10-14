TclOOLinkObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    // Set up common bits.
    CallContext *context = TclOOGetContextFromCurrentStackFrame(interp,
	    TclGetString(objv[0]));
    if (!context) {
	return TCL_ERROR;
    }
    Object *oPtr = context->oPtr;
    if (!oPtr->myCommand) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"cannot link to non-existent callback handle"));
	OO_ERROR(interp, MY_GONE);
	return TCL_ERROR;
    }
    Tcl_Obj *myCmd = Tcl_NewObj();
    Tcl_GetCommandFullName(interp, oPtr->myCommand, myCmd);
    if (!oPtr->linkedCmdsList) {
	oPtr->linkedCmdsList = Tcl_NewListObj(0, NULL);
	Tcl_IncrRefCount(oPtr->linkedCmdsList);
    }

    // For each argument
    for (Tcl_Size i=1; i<objc; i++) {
	Tcl_Size linkc;
	Tcl_Obj **linkv, *src, *dst;

	// Parse as list of (one or) two items: source and destination names
	if (TclListObjGetElements(interp, objv[i], &linkc, &linkv) != TCL_OK) {
	    Tcl_BounceRefCount(myCmd);
	    return TCL_ERROR;
	}
	switch (linkc) {
	case 1:
	    // Degenerate case
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

	// Qualify the source if necessary
	Tcl_Size srcLen;
	const char *srcStr = TclGetStringFromObj(src, &srcLen);
	if (srcStr[0] != ':' || srcStr[1] != ':') {
	    Tcl_DString ds;
	    Tcl_DStringInit(&ds);
	    Tcl_DStringAppend(&ds, context->oPtr->namespacePtr->fullName, -1);
	    TclDStringAppendLiteral(&ds, "::");
	    Tcl_DStringAppend(&ds, srcStr, srcLen);
	    src = Tcl_DStringToObj(&ds);
	}

	// Make the alias command
	if (TclAliasCreate(interp, interp, interp, src, myCmd, 1, &dst) != TCL_OK) {
	    Tcl_BounceRefCount(myCmd);
	    Tcl_BounceRefCount(src);
	    return TCL_ERROR;
	}

	// Remember the alias for cleanup if necessary
	Tcl_ListObjAppendElement(NULL, oPtr->linkedCmdsList, src);
    }
    Tcl_BounceRefCount(myCmd);
    return TCL_OK;
}