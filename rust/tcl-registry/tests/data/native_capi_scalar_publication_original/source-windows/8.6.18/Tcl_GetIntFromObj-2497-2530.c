Tcl_GetIntFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* The object from which to get a int. */
    int *intPtr)	/* Place to store resulting int. */
{
#if (LONG_MAX == INT_MAX)
    return TclGetLongFromObj(interp, objPtr, (long *) intPtr);
#else
    void *p;
    int type;

    if ((TclGetNumberFromObj(NULL, objPtr, &p, &type) != TCL_OK)
	    || (type == TCL_NUMBER_DOUBLE)) {
	if (interp != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "expected integer but got \"%s\"", Tcl_GetString(objPtr)));
	    Tcl_SetErrorCode(interp, "TCL", "VALUE", "INTEGER", (char *)NULL);
	}
	return TCL_ERROR;
    }
    if ((type != TCL_NUMBER_LONG) || ((ULONG_MAX > UINT_MAX)
	    && ((*(long *)p > UINT_MAX) || (*(long *)p < -(long)UINT_MAX)))) {
	if (interp != NULL) {
	    const char *s =
		    "integer value too large to represent";
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(s, -1));
	    Tcl_SetErrorCode(interp, "ARITH", "IOVERFLOW", s, (char *)NULL);
	}
	return TCL_ERROR;
    }
    *intPtr = (int)*(long *)p;
    return TCL_OK;
#endif
}
