Tcl_GetDoubleFromObj(
    Tcl_Interp *interp,		/* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,		/* The object from which to get a double. */
    double *dblPtr)		/* Place to store resulting double. */
{
    do {
	if (TclHasInternalRep(objPtr, &tclDoubleType)) {
	    if (isnan(objPtr->internalRep.doubleValue)) {
		if (interp != NULL) {
		    Tcl_SetObjResult(interp, Tcl_NewStringObj(
			    "floating point value is Not a Number", -1));
		    Tcl_SetErrorCode(interp, "TCL", "VALUE", "DOUBLE", "NAN",
			    (char *)NULL);
		}
		return TCL_ERROR;
	    }
	    *dblPtr = (double) objPtr->internalRep.doubleValue;
	    return TCL_OK;
	}
	if (TclHasInternalRep(objPtr, &tclIntType)) {
	    *dblPtr = (double) objPtr->internalRep.wideValue;
	    return TCL_OK;
	}
	if (TclHasInternalRep(objPtr, &tclBignumType)) {
	    mp_int big;

	    TclUnpackBignum(objPtr, big);
	    *dblPtr = TclBignumToDouble(&big);
	    return TCL_OK;
	}
    } while (SetDoubleFromAny(interp, objPtr) == TCL_OK);
    return TCL_ERROR;
}
