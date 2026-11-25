Tcl_GetBooleanFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* The object from which to get boolean. */
    int *intPtr)	/* Place to store resulting boolean. */
{
    do {
	if (objPtr->typePtr == &tclIntType) {
	    *intPtr = (objPtr->internalRep.longValue != 0);
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclBooleanType) {
	    *intPtr = (int) objPtr->internalRep.longValue;
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclDoubleType) {
	    /*
	     * Caution: Don't be tempted to check directly for the "double"
	     * Tcl_ObjType and then compare the internalrep to 0.0. This isn't
	     * reliable because a "double" Tcl_ObjType can hold the NaN value.
	     * Use the API Tcl_GetDoubleFromObj, which does the checking and
	     * sets the proper error message for us.
	     */

	    double d;

	    if (Tcl_GetDoubleFromObj(interp, objPtr, &d) != TCL_OK) {
		return TCL_ERROR;
	    }
	    *intPtr = (d != 0.0);
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclBignumType) {
	    *intPtr = 1;
	    return TCL_OK;
	}
#ifndef TCL_WIDE_INT_IS_LONG
	if (objPtr->typePtr == &tclWideIntType) {
	    *intPtr = (objPtr->internalRep.wideValue != 0);
	    return TCL_OK;
	}
#endif
    } while ((ParseBoolean(objPtr) == TCL_OK) || (TCL_OK ==
	    TclParseNumber(interp, objPtr, "boolean value", NULL,-1,NULL,0)));
    return TCL_ERROR;
}
