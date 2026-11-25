Tcl_GetIntFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* The object from which to get a int. */
    int *intPtr)	/* Place to store resulting int. */
{
#if (LONG_MAX == INT_MAX)
    return TclGetLongFromObj(interp, objPtr, (long *) intPtr);
#else
    long l;

    if (TclGetLongFromObj(interp, objPtr, &l) != TCL_OK) {
	return TCL_ERROR;
    }
    if ((ULONG_MAX > UINT_MAX) && ((l > UINT_MAX) || (l < INT_MIN))) {
	if (interp != NULL) {
	    const char *s =
		    "integer value too large to represent";
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(s, -1));
	    Tcl_SetErrorCode(interp, "ARITH", "IOVERFLOW", s, (char *)NULL);
	}
	return TCL_ERROR;
    }
    *intPtr = (int) l;
    return TCL_OK;
#endif
}
