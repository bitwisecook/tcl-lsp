Tcl_GetWideIntFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* Object from which to get a wide int. */
    Tcl_WideInt *wideIntPtr)
				/* Place to store resulting long. */
{
    do {
#ifndef TCL_WIDE_INT_IS_LONG
	if (objPtr->typePtr == &tclWideIntType) {
	    *wideIntPtr = objPtr->internalRep.wideValue;
	    return TCL_OK;
	}
#endif
	if (objPtr->typePtr == &tclIntType) {
	    *wideIntPtr = (Tcl_WideInt)objPtr->internalRep.longValue;
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclDoubleType) {
	    if (interp != NULL) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"expected integer but got \"%s\"",
			TclGetString(objPtr)));
		Tcl_SetErrorCode(interp, "TCL", "VALUE", "INTEGER", (char *)NULL);
	    }
	    return TCL_ERROR;
	}
	if (objPtr->typePtr == &tclBignumType) {
	    /*
	     * Must check for those bignum values that can fit in a
	     * Tcl_WideInt, even when auto-narrowing is enabled.
	     */

	    mp_int big;

	    UNPACK_BIGNUM(objPtr, big);
	    if ((size_t) big.used <= (CHAR_BIT * sizeof(Tcl_WideInt)
		     + MP_DIGIT_BIT - 1) / MP_DIGIT_BIT) {
		Tcl_WideUInt value = 0;
		size_t numBytes;
		Tcl_WideInt scratch;
		unsigned char *bytes = (unsigned char *) &scratch;

		if (mp_to_ubin(&big, bytes, sizeof(Tcl_WideInt), &numBytes) == MP_OKAY) {
		    while (numBytes-- > 0) {
			value = (value << CHAR_BIT) | *bytes++;
		    }
		    if (mp_isneg(&big)) {
			*wideIntPtr = (Tcl_WideInt)(-value);
		    } else {
			*wideIntPtr = (Tcl_WideInt)value;
		    }
		    return TCL_OK;
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
