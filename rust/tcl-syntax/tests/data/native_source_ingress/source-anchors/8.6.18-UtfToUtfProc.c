    for (numChars = 0; src < srcEnd && numChars <= charLimit; numChars++) {
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
	if (UCHAR(*src) < 0x80 && !((UCHAR(*src) == 0) && (pureNullMode == 0))) {
	    /*
	     * Copy 7bit characters, but skip null-bytes when we are in input
	     * mode, so that they get converted to 0xC080.
	     */

	    *dst++ = *src++;
	    *chPtr = 0; /* reset surrogate handling */
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd)
		&& (UCHAR(src[1]) == 0x80) && (pureNullMode == 1)) {
	    /*
	     * Convert 0xC080 to real nulls when we are in output mode.
	     */

	    *dst++ = 0;
	    *chPtr = 0; /* reset surrogate handling */
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Always check before using TclUtfToUniChar. Not doing can so
	     * cause it run beyond the end of the buffer! If we happen such an
	     * incomplete char its bytes are made to represent themselves
	     * unless the user has explicitly asked to be told.
	     */

	    if ((flags & TCL_ENCODING_STOPONERROR) && (pureNullMode == 0)) {
		result = TCL_CONVERT_MULTIBYTE;
		break;
	    }
	    *chPtr = UCHAR(*src);
	    src += 1;
	    dst += Tcl_UniCharToUtf(*chPtr, dst);
	} else {
	    size_t len = TclUtfToUniChar(src, chPtr);
	    if ((len < 2) && (*chPtr != 0) && (flags & TCL_ENCODING_STOPONERROR)
		    && ((*chPtr & ~0x7FF) != 0xD800) && (pureNullMode == 0)) {
		result = TCL_CONVERT_SYNTAX;
		break;
	    }
	    src += len;
	    if ((*chPtr & ~0x7FF) == 0xD800) {
		Tcl_UniChar low;
		/* A surrogate character is detected, handle especially */
#if TCL_UTF_MAX <= 4
	    if ((len < 3) && ((src[3 - len] & 0xC0) != 0x80)) {
	    /* It's invalid. See [ed29806ba] */
		*chPtr = UCHAR(src[-1]);
		dst += Tcl_UniCharToUtf(*chPtr, dst);
		continue;
	    }
#endif
		low = *chPtr;
		len = (src <= srcEnd-3) ? Tcl_UtfToUniChar(src, &low) : 0;
		if (((low & ~0x3FF) != 0xDC00) || (*chPtr & 0x400)) {
		    *dst++ = (char) (((*chPtr >> 12) | 0xE0) & 0xEF);
		    *dst++ = (char) (((*chPtr >> 6) | 0x80) & 0xBF);
		    *dst++ = (char) ((*chPtr | 0x80) & 0xBF);
		    *chPtr = 0; /* reset surrogate handling */
		    continue;
		} else if ((TCL_UTF_MAX > 3) || (pureNullMode == 1)) {
		    int full = (((*chPtr & 0x3FF) << 10) | (low & 0x3FF)) + 0x10000;
		    *dst++ = (char) (((full >> 18) | 0xF0) & 0xF7);
		    *dst++ = (char) (((full >> 12) | 0x80) & 0xBF);
		    *dst++ = (char) (((full >> 6) | 0x80) & 0xBF);
		    *dst++ = (char) ((full | 0x80) & 0xBF);
			*chPtr = 0; /* reset surrogate handling */
		    src += len;
		    continue;
		}
	    }
	    dst += Tcl_UniCharToUtf(*chPtr, dst);
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

/*
 *-------------------------------------------------------------------------
 *
 * UnicodeToUtfProc --
 *
 *	Convert from UTF-16 to UTF-8.
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
UnicodeToUtfProc(
    ClientData clientData,	/* != NULL means LE, == NUL means BE */
    const char *src,		/* Source string in Unicode. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
