TclOODefineExportObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstanceExport = (clientData != NULL), changed = false;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    Class *clsPtr = oPtr->classPtr;
    if (!isInstanceExport && !clsPtr) {
	return ReportAbuse(interp);
    }

    for (Tcl_Size i = 1; i < objc; i++) {
	/*
	 * Exporting is done by adding the PUBLIC_METHOD flag to the method
	 * record. If there is no such method in this object or class (i.e.
	 * the method comes from something inherited from or that we're an
	 * instance of) then we put in a blank record with that flag; such
	 * records are skipped over by the call chain engine *except* for
	 * their flags member.
	 */

	if (isInstanceExport) {
	    changed |= ExportInstanceMethod(oPtr, objv[i]);
	} else {
	    changed |= ExportMethod(clsPtr, objv[i]);
	}
    }

    /*
     * Bump the right epoch if we actually changed anything.
     */

    if (changed) {
	if (isInstanceExport) {
	    BumpInstanceEpoch(oPtr);
	} else {
	    BumpGlobalEpoch(interp, clsPtr);
	}
    }
    return TCL_OK;
}
