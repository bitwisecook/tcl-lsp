Tcl_GetWideIntFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* Object from which to get a wide int. */
    Tcl_WideInt *wideIntPtr)
				/* Place to store resulting long. */
{
    do {
	if (TclHasInternalRep(objPtr, &tclIntType)) {
	    *wideIntPtr = objPtr->internalRep.wideValue;
	    return TCL_OK;
	}
	if (TclHasInternalRep(objPtr, &tclDoubleType)) {
	    if (interp != NULL) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"expected integer but got \"%s\"",
			TclGetString(objPtr)));
		Tcl_SetErrorCode(interp, "TCL", "VALUE", "INTEGER", (char *)NULL);
	    }
	    return TCL_ERROR;
	}
	if (TclHasInternalRep(objPtr, &tclBignumType)) {
	    /*
	     * Must check for those bignum values that can fit in a
	     * Tcl_WideInt, even when auto-narrowing is enabled.
	     */

	    mp_int big;
	    Tcl_WideUInt value = 0;
	    size_t numBytes;
	    Tcl_WideInt scratch;
	    unsigned char *bytes = (unsigned char *) &scratch;

	    TclUnpackBignum(objPtr, big);
	    if (mp_to_ubin(&big, bytes, sizeof(Tcl_WideInt), &numBytes) == MP_OKAY) {
		while (numBytes-- > 0) {
		    value = (value << CHAR_BIT) | *bytes++;
		}
		if (big.sign) {
		    if (value <= 1 + ~(Tcl_WideUInt)WIDE_MIN) {
			*wideIntPtr = (Tcl_WideInt)(-value);
			return TCL_OK;
		    }
		} else {
		    if (value <= (Tcl_WideUInt)WIDE_MAX) {
			*wideIntPtr = (Tcl_WideInt)value;
			return TCL_OK;
		    }
		}
	    }
	    if (interp != NULL) {
		const char *s = "integer value too large to represent";
		Tcl_Obj *msg = Tcl_NewStringObj(s, -1);

		Tcl_SetObjResult(interp, msg);
		Tcl_SetErrorCode(interp, "ARITH", "IOVERFLOW", s, (char *)NULL);
	    }
	    return TCL_ERROR;
	}
    } while (TclParseNumber(interp, objPtr, "integer", NULL, -1, NULL,
	    TCL_PARSE_INTEGER_ONLY)==TCL_OK);
    return TCL_ERROR;
}
