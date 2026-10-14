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

    