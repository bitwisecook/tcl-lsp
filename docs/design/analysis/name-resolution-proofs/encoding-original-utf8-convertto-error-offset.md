# naming.encoding.original-utf8-convertto-error-offset

Kind: `native-observation`

## Problem statement

A UTF-8 conversion error has two coordinates. After the fixed multibyte U+00E9 prefix, copying the byte offset into the message character index or the character index into errorCode gives different data.

## Question

For the original counted String C3A9EDA0BD, what completion, result storage, diagnostic character index and errorCode byte offset does encoding convertto utf-8 produce in each pinned C release, and what independent current-image availability answer does Jim supply?

## Conclusion

All six fresh host processes exit0 with empty stderr. C8.4, C8.5 and C8.6 return0 with bytearray primary and no resident string before the getter, then expose the five output bytes C3A9EDA0BD. C9.0 and C9.1 return1 with string primary and resident bytes; their message says unexpected character at index1 U+00D83D while the complete errorCode is TCL ENCODING ILLEGALSEQUENCE2. The retained String input has none primary and resident bytes before and after each C call. The exact initialized Jim image supplies command count0 plus caught code1 invalid command name encoding, with outer completion0; it supplies no C object-storage or error-coordinate observation. This one fixed prefix distinguishes the two C9 coordinates without generalizing to arbitrary inputs or encodings.

## Scope

One original counted Tcl_NewStringObj operand C3A9EDA0BD is invoked through the unchanged four-word object vector and Tcl_EvalObjv/TCL_EVAL_GLOBAL in each fresh C process. INPUT and RETURN/INPUT_AFTER storage observations precede independent RAW or MESSAGE/ERROR_CODE getters. Jim executes its own unchanged current command-count and caught invocation control, with no C input constructor substitute. Exact request/probe/capture launcher, six actual compile receipts/ELFs, six whole process receipts/stdout/stderr, actual versions and required before/after pins are retained. Five whole tclCmdAH/tclEncoding/tclUtf source sets and six independently identified Native292 compile receipts remain source/build evidence. Native306 ten-input windows are a separate question. No arbitrary offset, other encoding/profile, private object identity, Jim worker or software assertion result follows. Shared software conversion requires actual Native C authority, its original string getter and an independently authenticated backend binary result constructor. The pure byte codec and structured error presenter do not issue physical storage. Runtime and VM comparator definitions retain all fifty original Native306 C windows and the five separate Native307 C windows, using exact counted constructor inputs and every non-version storage/output/error row. These definitions establish no executed software pass, private pointer/refcount identity or Jim encoding capability; Jim current-image availability remains its own observed channel. Independent exact C9 Tcl_UtfToExternalDStringEx source forms the message character position with Tcl_NumUtfChars over the processed prefix, while the errorCode decimal field comes from nBytesProcessed. The reached formatter source explains why the fixed observed U+00E9 prefix produces character index1 and byte offset2. Source inspection itself is not another native execution, a generic encoding-profile result or a backend pass.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: One fresh original C object-vector window before result getters; Jim independently measures current command availability/invocation only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

The one original counted String C3A9EDA0BD returns0 with bytearray primary and no resident string before the binary getter. RAW retains length5 and bytes C3A9EDA0BD. Original input none/resident storage remains unchanged. No error coordinate is produced by this successful call. The host process exits0 with empty stderr. No private pointer/refcount, arbitrary encoding or executed software result follows.

### tcl9.0, tcl9.1

Status: `observed`. Channel: One fresh original C object-vector window before result getters; Jim independently measures current command availability/invocation only. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

The one original counted String C3A9EDA0BD returns1 with string primary and resident bytes before getters. The complete message identifies character index1 U+00D83D; the complete errorCode is TCL ENCODING ILLEGALSEQUENCE2. The original input remains none/resident. This fixed multibyte prefix distinguishes the character index from the byte offset. The host process exits0 with empty stderr. No private pointer/refcount, arbitrary encoding or executed software result follows.

### jim

Status: `observed`. Channel: One fresh original C object-vector window before result getters; Jim independently measures current command availability/invocation only. Version: jim 0.84-9-g5bac7c9.

The fresh original availability/invocation control records actual version0.84-9-g5bac7c9, command count0 and caught code1 invalid command name encoding in this exact fullInit image. Its outer completion is0; it creates no C operand and measures no result primary, UTF-8 worker or error coordinates. This bounded image-specific answer does not establish absence in other Jim images or user command stores. The host process exits0 with empty stderr. No private pointer/refcount, arbitrary encoding or executed software result follows.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Source inspection

tcl9.0 9.0.4; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `Tcl_UtfToExternalDStringEx`, lines 1495–1613. Retained reference `native_encoding_utf8_offset307-9.0.4-tclEncoding.c`.

```text
Tcl_UtfToExternalDStringEx(
    Tcl_Interp *interp,		/* For error messages. May be NULL. */
    Tcl_Encoding encoding,	/* The encoding for the converted string, or
				 * NULL for the default system encoding. */
    const char *src,		/* Source string in UTF-8. */
    Tcl_Size srcLen,		/* Source string length in bytes, or < 0 for
				 * strlen(). */
    int flags,			/* Conversion control flags. */
    Tcl_DString *dstPtr,	/* Uninitialized or free DString in which the
				 * converted string is stored. */
    Tcl_Size *errorLocPtr)	/* Where to store the error location
				 * (or TCL_INDEX_NONE if no error). May
				 * be NULL. */
{
    char *dst;
    Tcl_EncodingState state;
    const Encoding *encodingPtr;
    int result;
    const char *srcStart = src;
    Tcl_Size dstLen, soFar;

    /* DO FIRST - must always be initialized on return */
    Tcl_DStringInit(dstPtr);

    dst = Tcl_DStringValue(dstPtr);
    dstLen = dstPtr->spaceAvl - 1;

    if (encoding == NULL) {
	encoding = systemEncoding;
    }
    encodingPtr = (Encoding *) encoding;

    if (src == NULL) {
	srcLen = 0;
	src = "";
    } else if (srcLen < 0) {
	srcLen = strlen(src);
    }

    flags &= ~TCL_ENCODING_END;
    flags |= TCL_ENCODING_START;
    while (1) {
	int srcChunkLen, srcChunkRead;
	int dstChunkLen, dstChunkWrote, dstChunkChars;

	if (srcLen > INT_MAX) {
	    srcChunkLen = INT_MAX;
	} else {
	    srcChunkLen = srcLen;
	    flags |= TCL_ENCODING_END; /* Last chunk */
	}
	dstChunkLen = dstLen > INT_MAX ? INT_MAX : dstLen;

	result = encodingPtr->fromUtfProc(encodingPtr->clientData, src,
		srcChunkLen, flags, &state, dst, dstChunkLen,
		&srcChunkRead, &dstChunkWrote, &dstChunkChars);
	soFar = dst + dstChunkWrote - Tcl_DStringValue(dstPtr);

	/* Move past the part processed in this go around */
	src += srcChunkRead;

	/*
	 * Keep looping in two case -
	 *   - our destination buffer did not have enough room
	 *   - we had not passed in all the data and error indicated fragment
	 *     of a multibyte character
	 * In both cases we have to grow buffer, move the input source pointer
	 * and loop. Otherwise, return the result we got.
	 */
	if ((result != TCL_CONVERT_NOSPACE) &&
		(result != TCL_CONVERT_MULTIBYTE || (flags & TCL_ENCODING_END))) {
	    Tcl_Size nBytesProcessed = (src - srcStart);
	    Tcl_Size i = soFar + encodingPtr->nullSize - 1;
	    /* Loop as DStringSetLength only stores one nul byte at a time */
	    while (i >= soFar) {
		Tcl_DStringSetLength(dstPtr, i--);
	    }
	    if (errorLocPtr) {
		/*
		 * Do not write error message into interpreter if caller
		 * wants to know error location.
		 */
		*errorLocPtr = result == TCL_OK
			? TCL_INDEX_NONE : nBytesProcessed;
	    } else {
		/* Caller wants error message on failure */
		if (result != TCL_OK && interp != NULL) {
		    Tcl_Size pos = Tcl_NumUtfChars(srcStart, nBytesProcessed);
		    int ucs4;
		    char buf[TCL_INTEGER_SPACE];

		    TclUtfToUniChar(&srcStart[nBytesProcessed], &ucs4);
		    snprintf(buf, sizeof(buf), "%" TCL_SIZE_MODIFIER "d",
			    nBytesProcessed);
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "unexpected character at index %" TCL_SIZE_MODIFIER
			    "u: 'U+%06X'",
			    pos, ucs4));
		    Tcl_SetErrorCode(interp, "TCL", "ENCODING", "ILLEGALSEQUENCE",
			    buf, (char *)NULL);
		}
	    }
	    if (result != TCL_OK) {
		errno = (result == TCL_CONVERT_NOSPACE) ? ENOMEM : EILSEQ;
	    }
	    return result;
	}

	flags &= ~TCL_ENCODING_START;
	srcLen -= srcChunkRead;

	if (Tcl_DStringLength(dstPtr) == 0) {
	    Tcl_DStringSetLength(dstPtr, dstLen);
	}
	Tcl_DStringSetLength(dstPtr, 2 * Tcl_DStringLength(dstPtr) + 1);
	dst = Tcl_DStringValue(dstPtr) + soFar;
	dstLen = Tcl_DStringLength(dstPtr) - soFar - 1;
    }
}

```

tcl9.1 9.1.0; independently pinned source inspection, revision `Exact retained source SHA; no new execution or source-control revision claimed.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `Tcl_UtfToExternalDStringEx`, lines 1745–1863. Retained reference `native_encoding_utf8_offset307-9.1.0-tclEncoding.c`.

```text
Tcl_UtfToExternalDStringEx(
    Tcl_Interp *interp,		/* For error messages. May be NULL. */
    Tcl_Encoding encoding,	/* The encoding for the converted string, or
				 * NULL for the default system encoding. */
    const char *src,		/* Source string in UTF-8. */
    Tcl_Size srcLen,		/* Source string length in bytes, or < 0 for
				 * strlen(). */
    int flags,			/* Conversion control flags. */
    Tcl_DString *dstPtr,	/* Uninitialized or free DString in which the
				 * converted string is stored. */
    Tcl_Size *errorLocPtr)	/* Where to store the error location
				 * (or TCL_INDEX_NONE if no error). May
				 * be NULL. */
{
    char *dst;
    Tcl_EncodingState state;
    const Encoding *encodingPtr;
    int result;
    const char *srcStart = src;
    Tcl_Size dstLen, soFar;

    /* DO FIRST - must always be initialized on return */
    Tcl_DStringInit(dstPtr);

    dst = Tcl_DStringValue(dstPtr);
    dstLen = dstPtr->spaceAvl - 1;

    if (encoding == NULL) {
	encoding = systemEncoding;
    }
    encodingPtr = (Encoding *) encoding;

    if (src == NULL) {
	srcLen = 0;
	src = "";	/* Avoid ABSAN warnings about ops on NULL pointers */
    } else if (srcLen < 0) {
	srcLen = strlen(src);
    }

    flags &= ~TCL_ENCODING_END;
    flags |= TCL_ENCODING_START;
    while (1) {
	int srcChunkLen, srcChunkRead;
	int dstChunkLen, dstChunkWrote, dstChunkChars;

	if (srcLen > INT_MAX) {
	    srcChunkLen = INT_MAX;
	} else {
	    srcChunkLen = srcLen;
	    flags |= TCL_ENCODING_END; /* Last chunk */
	}
	dstChunkLen = dstLen > INT_MAX ? INT_MAX : dstLen;

	result = encodingPtr->fromUtfProc(encodingPtr->clientData, src,
		srcChunkLen, flags, &state, dst, dstChunkLen,
		&srcChunkRead, &dstChunkWrote, &dstChunkChars);
	soFar = dst + dstChunkWrote - Tcl_DStringValue(dstPtr);

	/* Move past the part processed in this go around */
	src += srcChunkRead;

	/*
	 * Keep looping in two case -
	 *   - our destination buffer did not have enough room
	 *   - we had not passed in all the data and error indicated fragment
	 *     of a multibyte character
	 * In both cases we have to grow buffer, move the input source pointer
	 * and loop. Otherwise, return the result we got.
	 */
	if ((result != TCL_CONVERT_NOSPACE) &&
		(result != TCL_CONVERT_MULTIBYTE || (flags & TCL_ENCODING_END))) {
	    Tcl_Size nBytesProcessed = (src - srcStart);
	    Tcl_Size i = soFar + encodingPtr->nullSize - 1;
	    /* Loop as DStringSetLength only stores one nul byte at a time */
	    while (i >= soFar) {
		Tcl_DStringSetLength(dstPtr, i--);
	    }
	    if (errorLocPtr) {
		/*
		 * Do not write error message into interpreter if caller
		 * wants to know error location.
		 */
		*errorLocPtr = result == TCL_OK
			? TCL_INDEX_NONE : nBytesProcessed;
	    } else {
		/* Caller wants error message on failure */
		if (result != TCL_OK && interp != NULL) {
		    Tcl_Size pos = Tcl_NumUtfChars(srcStart, nBytesProcessed);
		    int ucs4;
		    char buf[TCL_INTEGER_SPACE];

		    TclUtfToUniChar(&srcStart[nBytesProcessed], &ucs4);
		    snprintf(buf, sizeof(buf), "%" TCL_SIZE_MODIFIER "d",
			    nBytesProcessed);
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "unexpected character at index %" TCL_SIZE_MODIFIER
			    "u: 'U+%06X'",
			    pos, ucs4));
		    Tcl_SetErrorCode(interp, "TCL", "ENCODING", "ILLEGALSEQUENCE",
			    buf, (char *)NULL);
		}
	    }
	    if (result != TCL_OK) {
		errno = (result == TCL_CONVERT_NOSPACE) ? ENOMEM : EILSEQ;
	    }
	    return result;
	}

	flags &= ~TCL_ENCODING_START;
	srcLen -= srcChunkRead;

	if (Tcl_DStringLength(dstPtr) == 0) {
	    Tcl_DStringSetLength(dstPtr, dstLen);
	}
	Tcl_DStringSetLength(dstPtr, 2 * Tcl_DStringLength(dstPtr) + 1);
	dst = Tcl_DStringValue(dstPtr) + soFar;
	dstLen = Tcl_DStringLength(dstPtr) - soFar - 1;
    }
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

- [rust/tcl-registry/tests/data/native_encoding_utf8_offset307](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307)
- [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/string-eacute-single-high-surrogate](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/string-eacute-single-high-surrogate)
- [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/string-eacute-single-high-surrogate](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/string-eacute-single-high-surrogate)
- [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/string-eacute-single-high-surrogate](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/string-eacute-single-high-surrogate)
- [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/string-eacute-single-high-surrogate](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/string-eacute-single-high-surrogate)
- [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/string-eacute-single-high-surrogate](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/string-eacute-single-high-surrogate)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/string-eacute-single-high-surrogate](../../../../rust/tcl-registry/tests/data/native_encoding_utf8_offset307/jim/string-eacute-single-high-surrogate)
