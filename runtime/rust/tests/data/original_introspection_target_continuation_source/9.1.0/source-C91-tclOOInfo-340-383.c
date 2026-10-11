static int
InfoObjectClassCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName ?className?");
	return TCL_ERROR;
    }

    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    if (objc == 2) {
	Tcl_SetObjResult(interp,
		TclOOObjectName(interp, oPtr->selfCls->thisPtr));
	return TCL_OK;
    } else {
	Class *mixinPtr, *o2clsPtr;
	Tcl_Size i;

	o2clsPtr = TclOOGetClassFromObj(interp, objv[2]);
	if (o2clsPtr == NULL) {
	    return TCL_ERROR;
	}

	FOREACH(mixinPtr, oPtr->mixins) {
	    if (!mixinPtr) {
		continue;
	    }
	    if (TclOOIsReachable(o2clsPtr, mixinPtr)) {
		Tcl_SetObjResult(interp, Tcl_NewBooleanObj(1));
		return TCL_OK;
	    }
	}
	Tcl_SetObjResult(interp, Tcl_NewBooleanObj(
		TclOOIsReachable(o2clsPtr, oPtr->selfCls)));
	return TCL_OK;
