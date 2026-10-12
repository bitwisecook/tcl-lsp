TclOODefineUnexportObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstanceUnexport = (clientData != NULL), changed = false;
    Object *oPtr;
    Class *clsPtr;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;
    if (!isInstanceUnexport && !clsPtr) {
	return ReportAbuse(interp);
    }

    for (Tcl_Size i = 1; i < objc; i++) {
	if (isInstanceUnexport) {
	    changed |= UnexportInstanceMethod(oPtr, objv[i]);
	} else {
	    changed |= UnexportMethod(clsPtr, objv[i]);
	}
    }

    /*
     * Bump the right epoch if we actually changed anything.
     */

    if (changed) {
	if (isInstanceUnexport) {
	    BumpInstanceEpoch(oPtr);
	} else {
	    BumpGlobalEpoch(interp, clsPtr);
	}
    }
    return TCL_OK;
}

/*
