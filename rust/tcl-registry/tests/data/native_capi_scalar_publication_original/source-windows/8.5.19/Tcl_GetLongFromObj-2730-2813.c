Tcl_GetLongFromObj(
    Tcl_Interp *interp, 	/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr,	/* The object from which to get a long. */
    register long *longPtr)	/* Place to store resulting long. */
{
    do {
	if (objPtr->typePtr == &tclIntType) {
	    *longPtr = objPtr->internalRep.longValue;
	    return TCL_OK;
	}
#ifndef NO_WIDE_TYPE
	if (objPtr->typePtr == &tclWideIntType) {
	    /*
	     * We return any integer in the range -ULONG_MAX to ULONG_MAX
	     * converted to a long, ignoring overflow. The rule preserves
	     * existing semantics for conversion of integers on input, but
	     * avoids inadvertent demotion of wide integers to 32-bit ones in
	     * the internal rep.
	     */

	    Tcl_WideInt w = objPtr->internalRep.wideValue;
	    if (w >= -(Tcl_WideInt)(ULONG_MAX)
		    && w <= (Tcl_WideInt)(ULONG_MAX)) {
		*longPtr = Tcl_WideAsLong(w);
		return TCL_OK;
	    }
	    goto tooLarge;
	}
#endif
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
	     * Must check for those bignum values that can fit in a long, even
	     * when auto-narrowing is enabled. Only those values in the signed
	     * long range get auto-narrowed to tclIntType, while all the
	     * values in the unsigned long range will fit in a long.
	     */

	    mp_int big;

	    UNPACK_BIGNUM(objPtr, big);
	    if ((size_t)(big.used) <= (CHAR_BIT * sizeof(long) + DIGIT_BIT - 1)
		    / DIGIT_BIT) {
		unsigned long value = 0, numBytes = sizeof(long);
		long scratch;
		unsigned char *bytes = (unsigned char *)&scratch;
		if (mp_to_unsigned_bin_n(&big, bytes, &numBytes) == MP_OKAY) {
		    while (numBytes-- > 0) {
			value = (value << CHAR_BIT) | *bytes++;
		    }
		    if (big.sign) {
			*longPtr = - (long) value;
		    } else {
			*longPtr = (long) value;
		    }
		    return TCL_OK;
		}
	    }
#ifndef NO_WIDE_TYPE
	tooLarge:
#endif
	    if (interp != NULL) {
		const char *s = "integer value too large to represent";
		Tcl_Obj *msg = Tcl_NewStringObj(s, -1);

		Tcl_SetObjResult(interp, msg);
		Tcl_SetErrorCode(interp, "ARITH", "IOVERFLOW", s, NULL);
	    }
	    return TCL_ERROR;
	}
    } while (TclParseNumber(interp, objPtr, "integer", NULL, -1, NULL,
	    TCL_PARSE_INTEGER_ONLY)==TCL_OK);
    return TCL_ERROR;
}
