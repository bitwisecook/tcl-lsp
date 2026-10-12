
int
TclOODefineDefnNsObjCmd(
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
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);
    Tcl_Namespace *nsPtr;
    Tcl_Obj *nsNamePtr, **storagePtr;

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (clsPtr->thisPtr->flags & (ROOT_OBJECT | ROOT_CLASS)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"may not modify the definition namespace of the root classes",
		TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    /*
     * Parse the arguments and work out what the user wants to do.
     */

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "?kind? namespace");
	return TCL_ERROR;
    }
    if (objc == 3 && Tcl_GetIndexFromObj(interp, objv[1], kindList, "kind", 0,
	    &kind) != TCL_OK) {
	return TCL_ERROR;
    }
    if (!TclGetString(objv[objc - 1])[0]) {
	nsNamePtr = NULL;
    } else {
	nsPtr = GetNamespaceInOuterContext(interp, objv[objc - 1]);
	if (nsPtr == NULL) {
	    return TCL_ERROR;
	}
	nsNamePtr = TclNewNamespaceObj(nsPtr);
	Tcl_IncrRefCount(nsNamePtr);
    }

    /*
     * Update the correct field of the class definition.
     */

    if (kind) {			// -instance
	storagePtr = &clsPtr->objDefinitionNs;
    } else {			// -class
	storagePtr = &clsPtr->clsDefinitionNs;
    }
    if (*storagePtr != NULL) {
	Tcl_DecrRefCount(*storagePtr);
    }
    *storagePtr = nsNamePtr;
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
