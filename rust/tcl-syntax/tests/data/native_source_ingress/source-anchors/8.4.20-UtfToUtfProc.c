
    for (numChars = 0; src < srcEnd; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 &&
	    !(UCHAR(*src) == 0 && pureNullMode == 0)) {
	    /*
	     * Copy 7bit chatacters, but skip null-bytes when we are
	     * in input mode, so that they get converted to 0xc080.
	     */
	    *dst++ = *src++;
	} else if (pureNullMode == 1 &&
		   UCHAR(*src) == 0xc0 &&
		   UCHAR(*(src+1)) == 0x80) {
	    /* 
	     * Convert 0xc080 to real nulls when we are in output mode.
	     */
	    *dst++ = 0;
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /* Always check before using Tcl_UtfToUniChar. Not doing
	     * can so cause it run beyond the endof the buffer!  If we
	     * happen such an incomplete char its bytes are made to
	     * represent themselves.
	     */

	    ch = (unsigned char) *src;
	    src += 1;
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    src += Tcl_UtfToUniChar(src, &ch);
	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    *srcReadPtr  = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

/*
 *-------------------------------------------------------------------------
 *
 * UnicodeToUtfProc --
 *
 *	Convert from Unicode to UTF-8.
 *
 * Results:
 *	Returns TCL_OK if conversion was successful.
 *
 * Side effects:
 *	None.
 *
 *-------------------------------------------------------------------------
 */

static int 
UnicodeToUtfProc(clientData, src, srcLen, flags, statePtr, dst, dstLen,
	srcReadPtr, dstWrotePtr, dstCharsPtr)
    ClientData clientData;	/* Not used. */
    CONST char *src;		/* Source string in Unicode. */
    int srcLen;			/* Source string length in bytes. */
    int flags;			/* Conversion control flags. */
    Tcl_EncodingState *statePtr;/* Place for conversion routine to store
				 * state information used during a piecewise
				 * conversion.  Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst;			/* Output buffer in which converted string
				 * is stored. */
    int dstLen;			/* The maximum length of output buffer in
				 * bytes. */
