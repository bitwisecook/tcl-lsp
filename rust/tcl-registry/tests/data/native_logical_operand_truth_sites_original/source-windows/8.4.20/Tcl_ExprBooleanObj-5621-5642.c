Tcl_ExprBooleanObj(interp, objPtr, ptr)
    Tcl_Interp *interp;			/* Context in which to evaluate the
					 * expression. */
    register Tcl_Obj *objPtr;		/* Expression to evaluate. */
    int *ptr;				/* Where to store 0/1 result. */
{
    Tcl_Obj *resultPtr;
    int result;

    result = Tcl_ExprObj(interp, objPtr, &resultPtr);
    if (result == TCL_OK) {
	if (resultPtr->typePtr == &tclIntType) {
	    *ptr = (resultPtr->internalRep.longValue != 0);
	} else if (resultPtr->typePtr == &tclDoubleType) {
	    *ptr = (resultPtr->internalRep.doubleValue != 0.0);
	} else {
	    result = Tcl_GetBooleanFromObj(interp, resultPtr, ptr);
	}
	Tcl_DecrRefCount(resultPtr);  /* discard the result object */
    }
    return result;
}
