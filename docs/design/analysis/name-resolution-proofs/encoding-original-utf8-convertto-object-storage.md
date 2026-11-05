# naming.encoding.original-utf8-convertto-object-storage

Kind: `native-observation`

## Problem statement

Counted String input bytes, ByteArray octets and external UTF-8 output are different operands. Treating an unchanged Unicode input object as a converted external binary object can expose E9 instead of C3A9 to a later binary getter. Release-specific surrogate/astral interpretation and result storage require their own original object-vector controls.

## Question

For ten fixed counted String or ByteArray operands, what original completion, result primary/string residency before getters, input materialisation and raw UTF-8 octets does encoding convertto produce on each pinned C release, and what separate availability/invocation answer does the exact initialized Jim image supply?

## Conclusion

All sixty fresh host processes exit0 with empty stderr. The fifty original C object-vector calls retain46 code0 results whose primary is bytearray with no resident string before the getter, and4 C9 code1 results whose primary is string with resident bytes. C9 rejects the fixed surrogate-pair and isolated-high-surrogate String inputs with the retained illegal-sequence diagnostic/errorCode. The successful eacute operand returns C3A9; both fixed zero forms return00. A ByteArray containing C3A9 octets returns C383C2A9 rather than the unchanged octets. C8.4/8.5 retain the fixed surrogate pair as six output bytes and interpret the fixed astral input as eight output bytes; C8.6 combines the pair and preserves the astral four bytes, while C9 accepts the astral input and rejects the two surrogate cases. ByteArray input retains its primary while acquiring resident string bytes; String input keeps its none primary and resident bytes in these windows. Each Jim availability control returns a zero command count and caught code1 invalid-command diagnostic for encoding in this exact fullInit image; it supplies no C input/object-storage observation. These fixed fields establish neither arbitrary encoding behavior nor private object identity or software execution success.

## Scope

The unchanged ASCII C probe constructs each exact counted Tcl_NewStringObj or Tcl_NewByteArrayObj operand and invokes the original four-word object vector with Tcl_EvalObjv/TCL_EVAL_GLOBAL in a fresh initialized process. INPUT and RETURN/INPUT_AFTER primary and string-residency fields precede the result binary/string getter; RAW, MESSAGE and ERROR_CODE are independent subsequent channels. No CLI/source parser, input replacement or Unicode rendering substitutes the fixed constructor bytes. Jim has its own original info commands count plus caught invocation and no C object constructor/type substitute. Complete unchanged request/probe/capture launcher, six actual compile receipts/ELFs, sixty process receipts/stdout/stderr, actual versions and required before/after pins are retained. The five complete tclCmdAH/tclEncoding/tclUtf source sets and six separately identified Native292 compile receipts are independent source/build evidence, not additional executions. No pointer/refcount/table/name/parent lifetime, arbitrary codepage/profile/failindex/streaming behavior, BIG-IP or software assertion result is established. Shared software conversion requires actual Native C authority, its original string getter and an independently authenticated backend binary result constructor. The pure byte codec and structured error presenter do not issue physical storage. Runtime and VM comparator definitions retain all fifty original Native306 C windows and the five separate Native307 C windows, using exact counted constructor inputs and every non-version storage/output/error row. These definitions establish no executed software pass, private pointer/refcount identity or Jim encoding capability; Jim current-image availability remains its own observed channel. Independent exact command source explains the observed result boundary: C8.4 reads the original counted String and writes binary bytes with Tcl_SetByteArrayObj on the interpreter result; C8.5/C8.6 read the original String and publish Tcl_NewByteArrayObj. C9 reads the original String through Tcl_UtfToExternalDStringEx, publishes ByteArray after successful conversion, and takes its separate error return before that constructor when no fail-index receiver is requested. Exact UtfToUtfProc bodies retain their release-selected counted UTF conversion, modified-zero and surrogate handling. Those source explanations do not extend the finite observed cases to arbitrary profiles/streaming/codepages or prove compiled/loaded private identity.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Ten fresh original C object-vector windows before result getters; Jim independently measures original current command availability/invocation only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Ten original object vectors retain10 code0 bytearray/no-resident-string results and0 code1 string/resident results before getters. Input and INPUT_AFTER fields retain the original constructor/type/materialisation boundary. Exact zero/eacute/FF/ByteArray-octet and surrogate/astral RAW bytes, or complete MESSAGE/ERROR_CODE, remain independent. Fixed surrogate-pair output is eda0bdedb880; fixed astral input outputs c3b0c29fc298c280. Every host process exits0 with empty stderr. No private pointer/refcount, arbitrary codepage or executed software result follows.

### tcl8.6

Status: `observed`. Channel: Ten fresh original C object-vector windows before result getters; Jim independently measures original current command availability/invocation only. Version: tcl8.6 8.6.18.

Ten original object vectors retain10 code0 bytearray/no-resident-string results and0 code1 string/resident results before getters. Input and INPUT_AFTER fields retain the original constructor/type/materialisation boundary. Exact zero/eacute/FF/ByteArray-octet and surrogate/astral RAW bytes, or complete MESSAGE/ERROR_CODE, remain independent. Fixed pair and astral inputs both output f09f9880; isolated high-surrogate output remains eda0bd. Every host process exits0 with empty stderr. No private pointer/refcount, arbitrary codepage or executed software result follows.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Ten fresh original C object-vector windows before result getters; Jim independently measures original current command availability/invocation only. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Ten original object vectors retain8 code0 bytearray/no-resident-string results and2 code1 string/resident results before getters. Input and INPUT_AFTER fields retain the original constructor/type/materialisation boundary. Exact zero/eacute/FF/ByteArray-octet and surrogate/astral RAW bytes, or complete MESSAGE/ERROR_CODE, remain independent. The two fixed surrogate String cases return1 with String/resident result and TCL ENCODING ILLEGALSEQUENCE0; fixed astral input outputs f09f9880. Every host process exits0 with empty stderr. No private pointer/refcount, arbitrary codepage or executed software result follows.

### jim

Status: `observed`. Channel: Ten fresh original C object-vector windows before result getters; Jim independently measures original current command availability/invocation only. Version: jim 0.84-9-g5bac7c9.

Ten fresh availability/invocation controls each observe actual version0.84-9-g5bac7c9, command count0 and caught code1 invalid command name encoding in this exact fullInit image. The outer AVAILABILITY completion is0; no String/ByteArray constructor, result primary/residency or UTF-8 worker is substituted. This is a bounded current-image answer, not a statement about other Jim builds, user commands or source configurations. Every host process exits0 with empty stderr. No private pointer/refcount, arbitrary codepage or executed software result follows.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

tcl8.4 8.4.20; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdAH.c`, function `Tcl_EncodingObjCmd`, lines 423–527. Retained reference `native_encoding_utf8_object306-8.4.20-tclCmdAH.c`.

```text
Tcl_EncodingObjCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int index, length;
    Tcl_Encoding encoding;
    char *string;
    Tcl_DString ds;
    Tcl_Obj *resultPtr;

    static CONST char *optionStrings[] = {
	"convertfrom", "convertto", "names", "system",
	NULL
    };
    enum options {
	ENC_CONVERTFROM, ENC_CONVERTTO, ENC_NAMES, ENC_SYSTEM
    };

    if (objc < 2) {
    	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
        return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], optionStrings, "option", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum options) index) {
	case ENC_CONVERTTO:
	case ENC_CONVERTFROM: {
	    Tcl_Obj *data;
	    if (objc == 3) {
		encoding = Tcl_GetEncoding(interp, NULL);
		data = objv[2];
	    } else if (objc == 4) {
		if (TclGetEncodingFromObj(interp, objv[2], &encoding)
			!= TCL_OK) {
		    return TCL_ERROR;
		}
		data = objv[3];
	    } else {
		Tcl_WrongNumArgs(interp, 2, objv, "?encoding? data");
		return TCL_ERROR;
	    }
	    
	    if ((enum options) index == ENC_CONVERTFROM) {
		/*
		 * Treat the string as binary data.
		 */

		string = (char *) Tcl_GetByteArrayFromObj(data, &length);
		Tcl_ExternalToUtfDString(encoding, string, length, &ds);

		/*
		 * Note that we cannot use Tcl_DStringResult here because
		 * it will truncate the string at the first null byte.
		 */

		Tcl_SetStringObj(Tcl_GetObjResult(interp),
			Tcl_DStringValue(&ds), Tcl_DStringLength(&ds));
		Tcl_DStringFree(&ds);
	    } else {
		/*
		 * Store the result as binary data.
		 */

		string = Tcl_GetStringFromObj(data, &length);
		Tcl_UtfToExternalDString(encoding, string, length, &ds);
		resultPtr = Tcl_GetObjResult(interp);
		Tcl_SetByteArrayObj(resultPtr, 
			(unsigned char *) Tcl_DStringValue(&ds),
			Tcl_DStringLength(&ds));
		Tcl_DStringFree(&ds);
	    }

	    Tcl_FreeEncoding(encoding);
	    break;
	}
	case ENC_NAMES: {
	    if (objc > 2) {
		Tcl_WrongNumArgs(interp, 2, objv, NULL);
		return TCL_ERROR;
	    }
	    Tcl_GetEncodingNames(interp);
	    break;
	}
	case ENC_SYSTEM: {
	    if (objc > 3) {
		Tcl_WrongNumArgs(interp, 2, objv, "?encoding?");
		return TCL_ERROR;
	    }
	    if (objc == 2) {
		Tcl_SetStringObj(Tcl_GetObjResult(interp),
			Tcl_GetEncodingName(NULL), -1);
	    } else {
	        return Tcl_SetSystemEncoding(interp,
			Tcl_GetStringFromObj(objv[2], NULL));
	    }
	    break;
	}
    }
    return TCL_OK;
}

```

tcl8.4 8.4.20; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2008–2105. Retained reference `native_encoding_utf8_object306-8.4.20-tclEncoding.c`.

```text
UtfToUtfProc(clientData, src, srcLen, flags, statePtr, dst, dstLen,
	     srcReadPtr, dstWrotePtr, dstCharsPtr, pureNullMode)
    ClientData clientData;	/* Not used. */
    CONST char *src;		/* Source string in UTF-8. */
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
    int *srcReadPtr;		/* Filled with the number of bytes from the
				 * source string that were converted.  This
				 * may be less than the original source length
				 * if there was a problem converting some
				 * source characters. */
    int *dstWrotePtr;		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr;		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode;		/* Convert embedded nulls from
				 * internal representation to real
				 * null-bytes or vice versa */

{
    CONST char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd;
    int result, numChars;
    Tcl_UniChar ch;

    result = TCL_OK;
    
    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - TCL_UTF_MAX;

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

```

tcl8.5 8.5.19; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdAH.c`, function `Tcl_EncodingObjCmd`, lines 426–528. Retained reference `native_encoding_utf8_object306-8.5.19-tclCmdAH.c`.

```text
Tcl_EncodingObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int index;

    static const char *optionStrings[] = {
	"convertfrom", "convertto", "dirs", "names", "system",
	NULL
    };
    enum options {
	ENC_CONVERTFROM, ENC_CONVERTTO, ENC_DIRS, ENC_NAMES, ENC_SYSTEM
    };

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], optionStrings, "option", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum options) index) {
    case ENC_CONVERTTO:
    case ENC_CONVERTFROM: {
	Tcl_Obj *data;
	Tcl_DString ds;
	Tcl_Encoding encoding;
	int length;
	char *stringPtr;

	if (objc == 3) {
	    encoding = Tcl_GetEncoding(interp, NULL);
	    data = objv[2];
	} else if (objc == 4) {
	    if (Tcl_GetEncodingFromObj(interp, objv[2], &encoding) != TCL_OK) {
		return TCL_ERROR;
	    }
	    data = objv[3];
	} else {
	    Tcl_WrongNumArgs(interp, 2, objv, "?encoding? data");
	    return TCL_ERROR;
	}

	if ((enum options) index == ENC_CONVERTFROM) {
	    /*
	     * Treat the string as binary data.
	     */

	    stringPtr = (char *) Tcl_GetByteArrayFromObj(data, &length);
	    Tcl_ExternalToUtfDString(encoding, stringPtr, length, &ds);

	    /*
	     * Note that we cannot use Tcl_DStringResult here because it will
	     * truncate the string at the first null byte.
	     */

	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    Tcl_DStringValue(&ds), Tcl_DStringLength(&ds)));
	    Tcl_DStringFree(&ds);
	} else {
	    /*
	     * Store the result as binary data.
	     */

	    stringPtr = TclGetStringFromObj(data, &length);
	    Tcl_UtfToExternalDString(encoding, stringPtr, length, &ds);
	    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(
		    (unsigned char *) Tcl_DStringValue(&ds),
		    Tcl_DStringLength(&ds)));
	    Tcl_DStringFree(&ds);
	}

	Tcl_FreeEncoding(encoding);
	break;
    }
    case ENC_DIRS:
	return EncodingDirsObjCmd(dummy, interp, objc, objv);
    case ENC_NAMES:
	if (objc > 2) {
	    Tcl_WrongNumArgs(interp, 2, objv, NULL);
	    return TCL_ERROR;
	}
	Tcl_GetEncodingNames(interp);
	break;
    case ENC_SYSTEM:
	if (objc > 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?encoding?");
	    return TCL_ERROR;
	}
	if (objc == 2) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    Tcl_GetEncodingName(NULL), -1));
	} else {
	    return Tcl_SetSystemEncoding(interp, TclGetString(objv[2]));
	}
	break;
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2218–2313. Retained reference `native_encoding_utf8_object306-8.5.19-tclEncoding.c`.

```text
UtfToUtfProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr,		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode)		/* Convert embedded nulls from internal
				 * representation to real null-bytes or vice
				 * versa. */
{
    const char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd;
    int result, numChars;
    Tcl_UniChar ch;

    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - TCL_UTF_MAX;

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
	if (UCHAR(*src) < 0x80 && !(UCHAR(*src) == 0 && pureNullMode == 0)) {
	    /*
	     * Copy 7bit chatacters, but skip null-bytes when we are in input
	     * mode, so that they get converted to 0xc080.
	     */

	    *dst++ = *src++;
	} else if (pureNullMode == 1 && UCHAR(*src) == 0xc0 &&
		(src + 1 < srcEnd) && UCHAR(*(src+1)) == 0x80) {
	    /*
	     * Convert 0xc080 to real nulls when we are in output mode.
	     */

	    *dst++ = 0;
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Always check before using Tcl_UtfToUniChar. Not doing can so
	     * cause it run beyond the endof the buffer! If we happen such an
	     * incomplete char its byts are made to represent themselves.
	     */

	    ch = (unsigned char) *src;
	    src += 1;
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    src += Tcl_UtfToUniChar(src, &ch);
	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.6 8.6.18; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdAH.c`, function `EncodingConverttoObjCmd`, lines 712–755. Retained reference `native_encoding_utf8_object306-8.6.18-tclCmdAH.c`.

```text
EncodingConverttoObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *data;		/* String to convert */
    Tcl_DString ds;		/* Buffer to hold the byte array */
    Tcl_Encoding encoding;	/* Encoding to use */
    int length;			/* Length of the string being converted */
    const char *stringPtr;	/* Pointer to the first byte of the string */

    if (objc == 2) {
	encoding = Tcl_GetEncoding(interp, NULL);
	data = objv[1];
    } else if (objc == 3) {
	if (Tcl_GetEncodingFromObj(interp, objv[1], &encoding) != TCL_OK) {
	    return TCL_ERROR;
	}
	data = objv[2];
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?encoding? data");
	return TCL_ERROR;
    }

    /*
     * Convert the string to a byte array in 'ds'
     */

    stringPtr = TclGetStringFromObj(data, &length);
    Tcl_UtfToExternalDString(encoding, stringPtr, length, &ds);
    Tcl_SetObjResult(interp,
		     Tcl_NewByteArrayObj((unsigned char*) Tcl_DStringValue(&ds),
					 Tcl_DStringLength(&ds)));
    Tcl_DStringFree(&ds);

    /*
     * We're done with the encoding
     */

    Tcl_FreeEncoding(encoding);
    return TCL_OK;

}

```

tcl8.6 8.6.18; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2276–2420. Retained reference `native_encoding_utf8_object306-8.6.18-tclEncoding.c`.

```text
UtfToUtfProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr,		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode)		/* Convert embedded nulls from internal
				 * representation to real null-bytes or vice
				 * versa. Also combine or separate surrogate pairs */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars, charLimit = INT_MAX;
    Tcl_UniChar *chPtr = (Tcl_UniChar *) statePtr;

    if (flags & TCL_ENCODING_START) {
    	*statePtr = 0;
    }
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= 6;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - ((pureNullMode == 1) ? 4 : TCL_UTF_MAX);

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

```

tcl9.0 9.0.4; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdAH.c`, function `EncodingConverttoObjCmd`, lines 620–698. Retained reference `native_encoding_utf8_object306-9.0.4-tclCmdAH.c`.

```text
EncodingConverttoObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Obj *data;		/* String to convert */
    Tcl_DString ds;		/* Buffer to hold the byte array */
    Tcl_Encoding encoding;	/* Encoding to use */
    Tcl_Size length;		/* Length of the string being converted */
    const char *stringPtr;	/* Pointer to the first byte of the string */
    int result;
    int flags;
    Tcl_Obj *failVarObj;
    Tcl_Size errorLocation;

    if (EncodingConvertParseOptions(interp, objc, objv, &encoding, &data,
	    &flags, &failVarObj) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * Convert the string to a byte array in 'ds'
     */

    stringPtr = TclGetStringFromObj(data, &length);
    result = Tcl_UtfToExternalDStringEx(interp, encoding, stringPtr, length, flags,
	    &ds, failVarObj ? &errorLocation : NULL);
    /* NOTE: ds must be freed beyond this point even on error */

    switch (result) {
    case TCL_OK:
	errorLocation = TCL_INDEX_NONE;
	break;
    case TCL_ERROR:
	/* Error in parameters. Should not happen. interp will have error */
	goto done;
    default:
	/*
	 * One of the TCL_CONVERT_* errors. If we were not interested in the
	 * error location, interp result would already have been filled in
	 * and we can just return the error. Otherwise, we have to return
	 * what could be decoded and the returned error location.
	 */
	if (failVarObj == NULL) {
	    result = TCL_ERROR;
	    goto done;
	}
	break;
    }
    /*
     * TCL_OK or a TCL_CONVERT_* error where the caller wants back as much
     * data as was converted.
     */
    if (failVarObj) {
	Tcl_Obj *failIndex;

	TclNewIndexObj(failIndex, errorLocation);
	if (Tcl_ObjSetVar2(interp, failVarObj, NULL, failIndex,
		TCL_LEAVE_ERR_MSG) == NULL) {
	    result = TCL_ERROR;
	    goto done;
	}
    }

    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(
	    (unsigned char*) Tcl_DStringValue(&ds),
	    Tcl_DStringLength(&ds)));

    result = TCL_OK;

done:
    Tcl_DStringFree(&ds);
    if (encoding) {
	Tcl_FreeEncoding(encoding);
    }
    return result;

}

```

tcl9.0 9.0.4; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2441–2743. Retained reference `native_encoding_utf8_object306-9.0.4-tclEncoding.c`.

```text
UtfToUtfProc(
    void *clientData,		/* additional flags */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* TCL_ENCODING_* conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars, charLimit = INT_MAX;
    int ch;
    int profile;

    if (flags & TCL_ENCODING_START) {
	/* *statePtr will hold high surrogate in a split surrogate pair */
	*statePtr = 0;
    }
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= 6;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    flags |= PTR2INT(clientData);

    /*
     * If output is UTF-8 or encoding for Tcl's internal encoding,
     * max space needed is TCL_UTF_MAX. Otherwise, need 6 bytes (CESU-8)
     */
    dstEnd = dst + dstLen - ((flags & (ENCODING_INPUT|ENCODING_UTF)) ? TCL_UTF_MAX : 6);

    /*
     * Macro to output an isolated high surrogate when it is not followed
     * by a low surrogate. NOT to be called for strict profile since
     * that should raise an error.
     */
#define OUTPUT_ISOLATEDSURROGATE                                \
    do {                                                        \
	Tcl_UniChar high;                                       \
	if (PROFILE_REPLACE(profile)) {                         \
	    high = UNICODE_REPLACE_CHAR;                        \
	} else {                                                \
	    high = (Tcl_UniChar)(ptrdiff_t) *statePtr;          \
	}                                                       \
	assert(!(flags & ENCODING_UTF)); /* Must be CESU-8 */   \
	assert(HIGH_SURROGATE(high));                           \
	assert(!PROFILE_STRICT(profile));                       \
	dst += Tcl_UniCharToUtf(high, dst);                     \
	*statePtr = 0; /* Reset state */                        \
    } while (0)

    /*
     * Macro to check for isolated surrogate and either break with
     * an error if profile is strict, or output an appropriate
     * character for replace and tcl8 profiles and continue.
     */
#define CHECK_ISOLATEDSURROGATE                                         \
    if (*statePtr) {                                                    \
	if (PROFILE_STRICT(profile)) {                                  \
	    result = TCL_CONVERT_SYNTAX;                                \
	    break;                                                      \
	}                                                               \
	OUTPUT_ISOLATEDSURROGATE;                                       \
	continue; /* Rerun loop so length checks etc. repeated */       \
    } else                                                              \
	(void) 0

    profile = ENCODING_PROFILE_GET(flags);
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
	if (UCHAR(*src) < 0x80 && !((UCHAR(*src) == 0) && (flags & ENCODING_INPUT))) {

	    CHECK_ISOLATEDSURROGATE;
	    /*
	     * Copy 7bit characters, but skip null-bytes when we are in input
	     * mode, so that they get converted to \xC0\x80.
	     */
	    *dst++ = *src++;
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd) &&
		 (UCHAR(src[1]) == 0x80) &&
		 (!(flags & ENCODING_INPUT) || !PROFILE_TCL8(profile))) {
	    /* Special sequence \xC0\x80 */

	    CHECK_ISOLATEDSURROGATE;
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
		/*
		 * Convert 0xC080 to real nulls when we are in output mode,
		 * irrespective of the profile.
		 */
		*dst++ = 0;
		src += 2;
	    }

	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Incomplete byte sequence not because there are insufficient
	     * bytes in source buffer (have already checked that above) but
	     * because the UTF-8 sequence is truncated.
	     */

	    CHECK_ISOLATEDSURROGATE;

	    if (flags & ENCODING_INPUT) {
		/* Incomplete bytes for modified UTF-8 target */
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

	    Tcl_UniChar savedSurrogate = (Tcl_UniChar) (ptrdiff_t)*statePtr;
	    *statePtr = 0; /* Reset surrogate */

	    if (flags & ENCODING_INPUT) {
		if (((len < 2) && (ch != 0)) || ((ch > 0xFFFF) && !(flags & ENCODING_UTF))) {
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
	    if (!(flags & ENCODING_UTF) && !(flags & ENCODING_INPUT)
		    && (ch > 0x7FF)) {
		assert(savedSurrogate == 0); /* Since this flag combo
						will never set *statePtr */
		if (ch > 0xFFFF) {
		    /* CESU-8 6-byte sequence for chars > U+FFFF */
		    ch -= 0x10000;
		    *dst++ = 0xED;
		    *dst++ = (char) (((ch >> 16) & 0x0F) | 0xA0);
		    *dst++ = (char) (((ch >> 10) & 0x3F) | 0x80);
		    ch = (ch & 0x03FF) | 0xDC00;
		}
		*dst++ = (char)(((ch >> 12) | 0xE0) & 0xEF);
		*dst++ = (char)(((ch >> 6) | 0x80) & 0xBF);
		*dst++ = (char)((ch | 0x80) & 0xBF);
		continue;
	    } else if (SURROGATE(ch)) {
		if ((flags & ENCODING_UTF)) {
		    /* UTF-8, not CESU-8, so surrogates should not appear */
		    if (PROFILE_STRICT(profile)) {
			result = (flags & ENCODING_INPUT)
			    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
			src = saveSrc;
			break;
		    } else if (PROFILE_REPLACE(profile)) {
			ch = UNICODE_REPLACE_CHAR;
		    } else {
			/* PROFILE_TCL8 - output as is */
		    }
		} else {
		    /* CESU-8 */
		    if (LOW_SURROGATE(ch)) {
			if (savedSurrogate) {
			    assert(HIGH_SURROGATE(savedSurrogate));
			    ch = 0x10000 + ((savedSurrogate - 0xd800) << 10) + (ch - 0xdc00);
			} else {
			    /* Isolated low surrogate */
			    if (PROFILE_STRICT(profile)) {
				result = (flags & ENCODING_INPUT)
				    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
				src = saveSrc;
				break;
			    } else if (PROFILE_REPLACE(profile)) {
				ch = UNICODE_REPLACE_CHAR;
			    } else {
				/* Tcl8 profile. Output low surrogate as is */
			    }
			}
		    } else {
			assert(HIGH_SURROGATE(ch));
			/* Save the high surrogate */
			*statePtr = (Tcl_EncodingState) (ptrdiff_t) ch;
			if (savedSurrogate) {
			    assert(HIGH_SURROGATE(savedSurrogate));
			    if (PROFILE_STRICT(profile)) {
				result = (flags & ENCODING_INPUT)
				    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
				src = saveSrc;
				break;
			    } else if (PROFILE_REPLACE(profile)) {
				ch = UNICODE_REPLACE_CHAR;
			    } else {
				/* Output the isolated high surrogate */
				ch = savedSurrogate;
			    }
			} else {
			    /* High surrogate saved in *statePtr. Do not output anything just yet. */
			    --numChars; /* Cancel the increment at end of loop */
			    continue;
			}
		    }
		}
	    } else {
		/* Normal character */
		CHECK_ISOLATEDSURROGATE;
	    }

	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    /* Check if an high surrogate left over */
    if (*statePtr) {
	assert(!(flags & ENCODING_UTF)); /* CESU-8, Not UTF-8 */
	if (!(flags & TCL_ENCODING_END)) {
	    /* More data coming */
	} else {
	    /* No more data coming */
	    if (PROFILE_STRICT(profile)) {
		result = (flags & ENCODING_INPUT)
		    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
	    } else {
		if (PROFILE_REPLACE(profile)) {
		    ch = UNICODE_REPLACE_CHAR;
		} else {
		    ch = (Tcl_UniChar) (ptrdiff_t) *statePtr;
		}
		if (dst < dstEnd) {
		    dst += Tcl_UniCharToUtf(ch, dst);
		    ++numChars;
		} else {
		    /* No room in destination */
		    result = TCL_CONVERT_NOSPACE;
		}
	    }
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl9.1 9.1.0; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdAH.c`, function `EncodingConverttoObjCmd`, lines 642–719. Retained reference `native_encoding_utf8_object306-9.1.0-tclCmdAH.c`.

```text
EncodingConverttoObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Obj *data;		/* String to convert */
    Tcl_DString ds;		/* Buffer to hold the byte array */
    Tcl_Encoding encoding;	/* Encoding to use */
    Tcl_Size length;		/* Length of the string being converted */
    const char *stringPtr;	/* Pointer to the first byte of the string */
    int result;
    int flags;
    Tcl_Obj *failVarObj;
    Tcl_Size errorLocation;

    if (EncodingConvertParseOptions(interp, objc, objv, &encoding, &data,
	    &flags, &failVarObj) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * Convert the string to a byte array in 'ds'
     */

    stringPtr = TclGetStringFromObj(data, &length);
    result = Tcl_UtfToExternalDStringEx(interp, encoding, stringPtr, length, flags,
	    &ds, failVarObj ? &errorLocation : NULL);
    /* NOTE: ds must be freed beyond this point even on error */

    switch (result) {
    case TCL_OK:
	errorLocation = TCL_INDEX_NONE;
	break;
    case TCL_ERROR:
	/* Error in parameters. Should not happen. interp will have error */
	goto done;
    default:
	/*
	 * One of the TCL_CONVERT_* errors. If we were not interested in the
	 * error location, interp result would already have been filled in
	 * and we can just return the error. Otherwise, we have to return
	 * what could be decoded and the returned error location.
	 */
	if (failVarObj == NULL) {
	    result = TCL_ERROR;
	    goto done;
	}
	break;
    }
    /*
     * TCL_OK or a TCL_CONVERT_* error where the caller wants back as much
     * data as was converted.
     */
    if (failVarObj) {
	Tcl_Obj *failIndex;

	TclNewIndexObj(failIndex, errorLocation);
	if (Tcl_ObjSetVar2(interp, failVarObj, NULL, failIndex,
		TCL_LEAVE_ERR_MSG) == NULL) {
	    result = TCL_ERROR;
	    goto done;
	}
    }

    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(
	    (unsigned char*) Tcl_DStringValue(&ds),
	    Tcl_DStringLength(&ds)));

    result = TCL_OK;

  done:
    Tcl_DStringFree(&ds);
    if (encoding) {
	Tcl_FreeEncoding(encoding);
    }
    return result;
}

```

tcl9.1 9.1.0; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2965–3152. Retained reference `native_encoding_utf8_object306-9.1.0-tclEncoding.c`.

```text
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

```

## Consumer bindings

- [rust/tcl-cmd-core/src/encoding.rs](../../../../rust/tcl-cmd-core/src/encoding.rs), `external_utf8_octets`: Own release-selected external UTF-8 byte transformation and structured error coordinates independently of any physical result storage.
- [rust/tcl-cmd-core/src/encoding.rs](../../../../rust/tcl-cmd-core/src/encoding.rs), `convert_to_utf8`: Require actual Native C name authority and original string getter before requesting independently authenticated external binary result storage.
- [rust/tcl-cmd-core/src/encoding.rs](../../../../rust/tcl-cmd-core/src/encoding.rs), `Utf8EncodingError`: Keep the diagnostic character index, original source-byte offset and rejected decoded unit as distinct fields.
- [rust/tcl-syntax/src/value.rs](../../../../rust/tcl-syntax/src/value.rs), `ValueOps::native_external_utf8_result`: Declare fallible external UTF-8 binary result construction; unavailable or foreign issuers supply no result storage.
- [runtime/rust/src/value_ops.rs](../../../../runtime/rust/src/value_ops.rs), `native_external_utf8_result`: Authenticate actual Runtime C release/result owner before constructing external binary bytes; chosen version or name recipe alone supplies no object authority.
- [rust/tcl-vm/src/value_ops.rs](../../../../rust/tcl-vm/src/value_ops.rs), `native_external_utf8_result`: Authenticate the VM actual C issuer independently before constructing the binary result; Jim and foreign owner cannot borrow it.
- [rust/tcl-test-support/src/encoding_utf8.rs](../../../../rust/tcl-test-support/src/encoding_utf8.rs), `CONTROLS`: Retain complete immutable original constructor/output columns shared by codec and both backend comparators, keeping constructor kind independent from observed result primary.
- [rust/tcl-cmd-core/src/encoding/native_controls.rs](../../../../rust/tcl-cmd-core/src/encoding/native_controls.rs), `encoding::native_controls::selected_utf8_units_match_all_55_original_c_conversion_windows` (linked): The pure selected C codec compares converted bytes or complete structured message/errorCode against fifty Native306 and five independent Native307 C columns. This tests byte/error presentation only; it owns no physical result header.
- [runtime/rust/src/cmd_misc/native_encoding_tests.rs](../../../../runtime/rust/src/cmd_misc/native_encoding_tests.rs), `cmd_misc::native_encoding_tests::original_utf8_conversion_matches_all_55_c_object_storage_windows` (linked): Authentic actual C original counted String/ByteArray object vectors compare all non-version INPUT, RETURN, INPUT_AFTER and RAW or MESSAGE/ERROR_CODE rows against the complete original fifty storage and five offset columns. Primary/residency is observed before getters; host refusal is excluded. No native pointer/refcount identity or Jim worker is inferred.
- [rust/tcl-vm/src/command/native_encoding_tests.rs](../../../../rust/tcl-vm/src/command/native_encoding_tests.rs), `command::native_encoding_tests::original_utf8_conversion_matches_all_55_c_object_storage_windows` (linked): The actual VM selected C environment executes original counted constructors and object vectors, then compares every non-version input/result storage and output/error field against the complete fifty storage and five offset columns. Result getters follow the independent primary/residency observation; no Native header identity or Jim worker is granted.

## Evidence

- [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.4.20)
- [rust/tcl-registry/tests/data/native_encoding_utf8_object306](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306)
- [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.5.19)
- [rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/8.6.18)
- [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.0.4)
- [rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_object306/jim)
