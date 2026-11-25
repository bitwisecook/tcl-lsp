Tcl_GetLongFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* The object from which to get a long. */
    long *longPtr)	/* Place to store resulting long. */
{
    do {
#ifdef TCL_WIDE_INT_IS_LONG
	if (TclHasInternalRep(objPtr, &tclIntType)) {
	    *longPtr = objPtr->internalRep.wideValue;
	    return TCL_OK;
	}
#else
	if (TclHasInternalRep(objPtr, &tclIntType)) {
	    /*
	     * We return any integer in the range LONG_MIN to ULONG_MAX
	     * converted to a long, ignoring overflow. The rule preserves
	     * existing semantics for conversion of integers on input, but
	     * avoids inadvertent demotion of wide integers to 32-bit ones in
	     * the internal rep.
	     */

	    Tcl_WideInt w = objPtr->internalRep.wideValue;

	    if (w >= (Tcl_WideInt)(LONG_MIN)
		    && w <= (Tcl_WideInt)(ULONG_MAX)) {
		*longPtr = (long)w;
		return TCL_OK;
	    }
	    goto tooLarge;
	}
#endif
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
	     * Must check for those bignum values that can fit in a long, even
	     * when auto-narrowing is enabled. Only those values in the signed
	     * long range get auto-narrowed to tclIntType, while all the
	     * values in the unsigned long range will fit in a long.
	     */

		{
	    mp_int big;
	    unsigned long scratch, value = 0;
	    unsigned char *bytes = (unsigned char *) &scratch;
	    size_t numBytes;

	    TclUnpackBignum(objPtr, big);
	    if (mp_to_ubin(&big, bytes, sizeof(long), &numBytes) == MP_OKAY) {
		while (numBytes-- > 0) {
			value = (value << CHAR_BIT) | *bytes++;
		}
		if (big.sign) {
		    if (value <= 1 + (unsigned long)LONG_MAX) {
			*longPtr = (long)(-value);
			return TCL_OK;
		    }
		} else {
		    if (value <= (unsigned long)ULONG_MAX) {
			*longPtr = (long)value;
			return TCL_OK;
		    }
		}
	    }
	    }
#ifndef TCL_WIDE_INT_IS_LONG
	tooLarge:
#endif
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
