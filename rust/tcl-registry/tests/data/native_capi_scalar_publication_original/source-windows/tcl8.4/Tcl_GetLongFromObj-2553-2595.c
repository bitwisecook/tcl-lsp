Tcl_GetLongFromObj(interp, objPtr, longPtr)
    Tcl_Interp *interp; 	/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr;	/* The object from which to get a long. */
    register long *longPtr;	/* Place to store resulting long. */
{
    register int result;
    
    if (objPtr->typePtr != &tclIntType && objPtr->typePtr != &tclWideIntType) {
	result = SetIntOrWideFromAny(interp, objPtr);
	if (result != TCL_OK) {
	    return result;
	}
    }

#ifndef TCL_WIDE_INT_IS_LONG
    if (objPtr->typePtr == &tclWideIntType) {
	/*
	 * If the object is already a wide integer, don't convert it.
	 * This code allows for any integer in the range -ULONG_MAX to
	 * ULONG_MAX to be converted to a long, ignoring overflow.
	 * The rule preserves existing semantics for conversion of
	 * integers on input, but avoids inadvertent demotion of
	 * wide integers to 32-bit ones in the internal rep.
	 */

	Tcl_WideInt w = objPtr->internalRep.wideValue;
	if (w >= -(Tcl_WideInt)(ULONG_MAX) && w <= (Tcl_WideInt)(ULONG_MAX)) {
	    *longPtr = Tcl_WideAsLong(w);
	    return TCL_OK;
	} else {
	    if (interp != NULL) {
		Tcl_ResetResult(interp);
		Tcl_AppendToObj(Tcl_GetObjResult(interp),
			"integer value too large to represent", -1);
	    }
	    return TCL_ERROR;
	}
    }
#endif

    *longPtr = objPtr->internalRep.longValue;
    return TCL_OK;
}
