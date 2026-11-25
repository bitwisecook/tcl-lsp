InfoClassDefnNsCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    static const char *const kindList[] = {
	"-class",
	"-instance",
	NULL
    };
    int kind = 0;
    Tcl_Obj *nsNamePtr;
    Class *clsPtr;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "className ?kind?");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc == 3 && Tcl_GetIndexFromObj(interp, objv[2], kindList, "kind", 0,
	    &kind) != TCL_OK) {
	return TCL_ERROR;
    }

    if (kind) {			// -instance
	nsNamePtr = clsPtr->objDefinitionNs;
    } else {			// -class
	nsNamePtr = clsPtr->clsDefinitionNs;
    }
    if (nsNamePtr) {
	Tcl_SetObjResult(interp, nsNamePtr);
    }
    return TCL_OK;
