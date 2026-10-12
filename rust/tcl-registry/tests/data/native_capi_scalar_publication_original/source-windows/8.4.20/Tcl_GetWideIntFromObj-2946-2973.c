Tcl_GetWideIntFromObj(interp, objPtr, wideIntPtr)
    Tcl_Interp *interp; 	/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr;	/* Object from which to get a wide int. */
    register Tcl_WideInt *wideIntPtr; /* Place to store resulting long. */
{
    register int result;

    if (objPtr->typePtr == &tclWideIntType) {
    gotWide:
	*wideIntPtr = objPtr->internalRep.wideValue;
	return TCL_OK;
    }
    if (objPtr->typePtr == &tclIntType) {
	/*
	 * This cast is safe; all valid ints/longs are wides.
	 */

	objPtr->internalRep.wideValue =
		Tcl_LongAsWide(objPtr->internalRep.longValue);
	objPtr->typePtr = &tclWideIntType;
	goto gotWide;
    }
    result = SetWideIntFromAny(interp, objPtr);
    if (result == TCL_OK) {
	*wideIntPtr = objPtr->internalRep.wideValue;
    }
    return result;
}
