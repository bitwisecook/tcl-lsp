Tcl_GetIntFromObj(interp, objPtr, intPtr)
    Tcl_Interp *interp; 	/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr;	/* The object from which to get a int. */
    register int *intPtr;	/* Place to store resulting int. */
{
    int result;
    Tcl_WideInt w = 0;

    /*
     * If the object isn't already an integer of any width, try to
     * convert it to one.
     */

    if (objPtr->typePtr != &tclIntType && objPtr->typePtr != &tclWideIntType) {
	result = SetIntOrWideFromAny(interp, objPtr);
	if (result != TCL_OK) {
	    return result;
	}
    }

    /*
     * Object should now be either int or wide. Get its value.
     */

#ifndef TCL_WIDE_INT_IS_LONG
    if (objPtr->typePtr == &tclWideIntType) {
	w = objPtr->internalRep.wideValue;
    } else
#endif
    {
	w = Tcl_LongAsWide(objPtr->internalRep.longValue);
    }

    if ((LLONG_MAX > UINT_MAX)
	    && ((w > UINT_MAX) || (w < -(Tcl_WideInt)UINT_MAX))) {
	if (interp != NULL) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"integer value too large to represent as non-long integer",
		-1));
	}
	return TCL_ERROR;
    }
    *intPtr = (int)w;
    return TCL_OK;
}
