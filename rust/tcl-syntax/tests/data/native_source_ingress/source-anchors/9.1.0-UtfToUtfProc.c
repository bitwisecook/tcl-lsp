UtfToUtfProc(
    void *clientData,
    const char *src,
    int srcLen,
    int flags,
    Tcl_EncodingState *statePtr,
    char *dst,
    int dstLen,
    int *srcReadPtr,
    int *dstWrotePtr,
    int *dstCharsPtr)
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars = 0, charLimit = INT_MAX;
    int ch;
    int profile;

    *statePtr = 0;
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    flags |= PTR2INT(clientData);

    dstEnd = dst + dstLen - TCL_UTF_MAX;

    /* Checks if a source byte can be copied directly to destination */
#define BYTE_COPYABLE(byte_) \
    (UCHAR(byte_) < 0x80 && ((UCHAR(byte_) != 0)))

    profile = ENCODING_PROFILE_GET(flags);

    for (numChars = 0; src < srcEnd && numChars < charLimit; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}

	if (BYTE_COPYABLE(*src)) {
	    /*
	     * Common case fast path for non-nul ASCII bytes.
	     *
	     * Because we resolved any pending surrogate above (before the
	     * loop), *statePtr is guaranteed to be 0 here, so no
	     * CHECK_ISOLATEDSURROGATE() call is needed.
	     */
	    const char *p = src;
	    ptrdiff_t copyCount = srcEnd - src;
	    if (charLimit != INT_MAX) {
		if (copyCount > (charLimit - numChars)) {
		    copyCount = charLimit - numChars;
		}
	    }
	    if (copyCount > (dstEnd - dst + 1)) {
		copyCount = dstEnd - dst + 1;
	    }
	    const char *srcStop = src + copyCount;
	    while (p < srcStop && BYTE_COPYABLE(*p)) {
		*dst++ = *p++;
	    }
	    numChars += (int)(p - src) - 1;
	    src = p;
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd) &&
		(UCHAR(src[1]) == 0x80) &&
		(!(flags & ENCODING_INPUT) || !PROFILE_TCL8(profile))) {
	    /* Special sequence \xC0\x80 */
	    if (!PROFILE_TCL8(profile) && (flags & ENCODING_INPUT)) {
		if (PROFILE_REPLACE(profile)) {
		    dst += Tcl_UniCharToUtf(UNICODE_REPLACE_CHAR, dst);
		    src += 2;
		} else {
		    /* PROFILE_STRICT */
		    result = TCL_CONVERT_SYNTAX;
		    break;
		}
	    } else {
		*dst++ = 0;
		src += 2;
	    }
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Incomplete byte sequence (truncated UTF-8, not just end of
	     * source buffer — that is caught by the srcClose check above).
	     */
	    if (flags & ENCODING_INPUT) {
		if (PROFILE_STRICT(profile)) {
		    result = (flags & TCL_ENCODING_CHAR_LIMIT)
			    ? TCL_CONVERT_MULTIBYTE
			    : TCL_CONVERT_SYNTAX;
		    break;
		}
	    }
	    if (PROFILE_REPLACE(profile)) {
		ch = UNICODE_REPLACE_CHAR;
		++src;
	    } else {
		/* TCL_ENCODING_PROFILE_TCL8 */
		char chbuf[2];
		chbuf[0] = UCHAR(*src++);
		chbuf[1] = 0;
		TclUtfToUniChar(chbuf, &ch);
	    }
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    /* Have a complete character */
	    size_t len = TclUtfToUniChar(src, &ch);

	    /*
	     * For invalid inputs, Tcl_UtfToUniChar will return the byte
	     * itself and len == 1. We know this is invalid because valid
	     * single byte ASCII is already handled above except for 0.
	     * For the output case, Tcl should not have invalid byte sequences
	     * internally but if it does, garbage in, garbage out. This is
	     * historical behavior that should be changed imho. See
	     * ticket [b69e00ecf6]. TODO.
	     */
	    if (flags & ENCODING_INPUT) {
		if ((len < 2) && (ch != 0)) {
		    if (PROFILE_STRICT(profile)) {
			result = TCL_CONVERT_SYNTAX;
			break;
		    } else if (PROFILE_REPLACE(profile)) {
			ch = UNICODE_REPLACE_CHAR;
		    }
		}
	    }

	    const char *saveSrc = src;
	    src += len;

	    if (SURROGATE(ch)) {
		if (PROFILE_STRICT(profile)) {
		    result = (flags & ENCODING_INPUT)
			    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
		    src = saveSrc;
		    break;
		} else if (PROFILE_REPLACE(profile)) {
		    ch = UNICODE_REPLACE_CHAR;
		}
		/* PROFILE_TCL8: fall through and output as-is */
	    }
	    /* Normal character (or surrogate resolved to replacement/as-is) */
	    assert(ch >= 0 && ch <= 0x10FFFF);
	    if (ch == 0) {
		if (flags & ENCODING_INPUT) {
		    *dst++ = (char)0xC0;
		    *dst++ = (char)0x80;
		} else {
		    *dst++ = 0;
		}
	    } else if ((unsigned)ch < 0x800) {
		assert(ch >= 0x80);
		*dst++ = (char)(0xC0 | (ch >> 6));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    } else if ((unsigned)ch < 0x10000) {
		*dst++ = (char)(0xE0 | (ch >> 12));
		*dst++ = (char)(0x80 | ((ch >> 6) & 0x3F));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    } else {
		*dst++ = (char)(0xF0 | (ch >> 18));
		*dst++ = (char)(0x80 | ((ch >> 12) & 0x3F));
		*dst++ = (char)(0x80 | ((ch >> 6)  & 0x3F));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    }
	}
    }

    *srcReadPtr  = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;

#undef BYTE_COPYABLE
}

