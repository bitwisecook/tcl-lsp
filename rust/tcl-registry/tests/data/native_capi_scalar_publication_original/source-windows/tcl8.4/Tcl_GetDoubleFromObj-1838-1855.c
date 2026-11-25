Tcl_GetDoubleFromObj(interp, objPtr, dblPtr)
    Tcl_Interp *interp; 	/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr;	/* The object from which to get a double. */
    register double *dblPtr;	/* Place to store resulting double. */
{
    register int result;
    
    if (objPtr->typePtr == &tclDoubleType) {
	*dblPtr = objPtr->internalRep.doubleValue;
	return TCL_OK;
    }

    result = SetDoubleFromAny(interp, objPtr);
    if (result == TCL_OK) {
	*dblPtr = objPtr->internalRep.doubleValue;
    }
    return result;
}
