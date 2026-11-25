Tcl_GetLongFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* The object from which to get a long. */
    long *longPtr)	/* Place to store resulting long. */
{
    do {
	if (objPtr->typePtr == &tclIntType) {
	    *longPtr = objPtr->internalRep.longValue;
	    return TCL_OK;
	}
#ifndef TCL_WIDE_INT_IS_LONG
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
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"expected integer but got \"%s\"",
			TclGetString(objPtr)));
		Tcl_SetErrorCode(interp, "TCL", "VALUE", "INTEGER", (char *)NULL);
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
	    if ((size_t) big.used <= (CHAR_BIT * sizeof(long) + MP_DIGIT_BIT - 1)
		    / MP_DIGIT_BIT) {
		unsigned long value = 0;
		size_t numBytes;
		long scratch;
		unsigned char *bytes = (unsigned char *) &scratch;

		if (mp_to_ubin(&big, bytes, sizeof(long), &numBytes) == MP_OKAY) {
		    while (numBytes-- > 0) {
			value = (value << CHAR_BIT) | *bytes++;
		    }
		    if (mp_isneg(&big)) {
			*longPtr = (long)(-value);
		    } else {
			*longPtr = (long)value;
		    }
		    return TCL_OK;
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
