Tcl_GetWideIntFromObj(
    Tcl_Interp *interp, 	/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr,	/* Object from which to get a wide int. */
    register Tcl_WideInt *wideIntPtr)
				/* Place to store resulting long. */
{
    do {
#ifndef NO_WIDE_TYPE
	if (objPtr->typePtr == &tclWideIntType) {
	    *wideIntPtr = objPtr->internalRep.wideValue;
	    return TCL_OK;
	}
#endif
	if (objPtr->typePtr == &tclIntType) {
	    *wideIntPtr = (Tcl_WideInt) objPtr->internalRep.longValue;
	    return TCL_OK;
	}
        if (objPtr->typePtr == &tclDoubleType) {
            if (interp != NULL) {
		Tcl_Obj *msg;

		TclNewLiteralStringObj(msg, "expected integer but got \"");
		Tcl_AppendObjToObj(msg, objPtr);
		Tcl_AppendToObj(msg, "\"", -1);
		Tcl_SetObjResult(interp, msg);
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
	    if ((size_t)(big.used) <= (CHAR_BIT * sizeof(Tcl_WideInt)
		     + DIGIT_BIT - 1) / DIGIT_BIT) {
		Tcl_WideUInt value = 0;
		unsigned long numBytes = sizeof(Tcl_WideInt);
		Tcl_WideInt scratch;
		unsigned char *bytes = (unsigned char *) &scratch;

		if (mp_to_unsigned_bin_n(&big, bytes, &numBytes) == MP_OKAY) {
		    while (numBytes-- > 0) {
			value = (value << CHAR_BIT) | *bytes++;
		    }
		    if (big.sign) {
			*wideIntPtr = - (Tcl_WideInt) value;
		    } else {
			*wideIntPtr = (Tcl_WideInt) value;
		    }
		    return TCL_OK;
		}
	    }
	    if (interp != NULL) {
		const char *s = "integer value too large to represent";
		Tcl_Obj* msg = Tcl_NewStringObj(s, -1);

		Tcl_SetObjResult(interp, msg);
		Tcl_SetErrorCode(interp, "ARITH", "IOVERFLOW", s, NULL);
	    }
	    return TCL_ERROR;
	}
    } while (TclParseNumber(interp, objPtr, "integer", NULL, -1, NULL,
	    TCL_PARSE_INTEGER_ONLY)==TCL_OK);
    return TCL_ERROR;
}
