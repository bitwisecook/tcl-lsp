Tcl_GetDoubleFromObj(
    Tcl_Interp *interp, 	/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr,	/* The object from which to get a double. */
    register double *dblPtr)	/* Place to store resulting double. */
{
    do {
	if (objPtr->typePtr == &tclDoubleType) {
	    if (TclIsNaN(objPtr->internalRep.doubleValue)) {
		if (interp != NULL) {
		    Tcl_SetObjResult(interp, Tcl_NewStringObj(
			    "floating point value is Not a Number", -1));
		}
		return TCL_ERROR;
	    }
	    *dblPtr = (double) objPtr->internalRep.doubleValue;
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclIntType) {
	    *dblPtr = objPtr->internalRep.longValue;
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclBignumType) {
	    mp_int big;
	    UNPACK_BIGNUM( objPtr, big );
	    *dblPtr = TclBignumToDouble( &big );
	    return TCL_OK;
	}
#ifndef NO_WIDE_TYPE
	if (objPtr->typePtr == &tclWideIntType) {
	    *dblPtr = (double) objPtr->internalRep.wideValue;
	    return TCL_OK;
	}
#endif
    } while (SetDoubleFromAny(interp, objPtr) == TCL_OK);
    return TCL_ERROR;
}
