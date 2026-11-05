# naming.string.original-binary-replacement-sharing-and-result-roles

Kind: `native-observation`

## Problem statement

String replacement consumers must preserve the physical ByteArray producer, original argument/result roles and actual sharing schedule. Treating binary inputs as fresh Unicode Strings loses native result identity; treating allocator bytes as deterministic data overclaims the native contract.

## Question

For 11 native ByteArray replacement inputs on C Tcl 8.4–9.1, what public original/result primaries, resident-byte fields, alias identities, completions and result String bytes are measured under actual indexed EvalObjv caller references?

## Conclusion

C8 converts these original ByteArrays through its Unicode path. C9 preserves ByteArray primaries on binary branches and returns the original replacement for whole replacement. Both equal-splice caller-reference cases return fresh objects in indexed EvalObjv; this does not prove the separate direct helper cannot splice an unshared subject in place. C9 partial deletion appends to an uninitialised nonzero-length constructor; its recorded leading zeros are not a deterministic result-byte contract.

## Scope

55 fresh case processes and five independent same-static-library version diagnostics, with complete original streams/receipts and whole selected input bytes. Before-command, reached-before-result-getter and materialised snapshots remain distinct. Public object headers do not expose private ByteArray internals or entered EvalObjv temporary references. Direct parsed-index helper behavior, arbitrary build/allocator configurations, adverse callback chronology, Rust assertions, Jim binary equivalence and BIG-IP are not established. Linked Rust definitions are software coverage obligations. Their source assertions require independent execution receipts; they add no provider observation, feature availability, native object/frame or edit authority.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: Fresh Tcl_EvalObjv with native ByteArray inputs. Unique argv objects have one caller reference; case4 adds one genuine subject reference. Tcl_CreateInterp is used without Tcl_Init. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

The subject reaches String primary through Unicode conversion. Partial deletion produces 415a; replacement bytes fe become c3be and NUL becomes c080. Whole replacement is a fresh String result, including cases with an actual extra subject reference. Inverted and empty cases return the original subject. Exact public header fields are case-specific.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh Tcl_EvalObjv with native ByteArray inputs. Unique argv objects have one caller reference; case4 adds one genuine subject reference. Tcl_CreateInterp is used without Tcl_Init. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

The subject remains ByteArray primary. Whole replacement returns the exact original replacement; inverted and empty cases return the original subject. Equal splices return fresh ByteArray results under this actual EvalObjv caller schedule, both with one and with two caller references. Resident original Strings do not turn these observed splices into a Unicode path. Ordinary String replacement does reach Unicode. Partial deletion records c080c080415a, but the leading native storage is not initialised by its source producer, so these zeros are not a reusable expected-value guarantee.

### jim

Status: `not-tested`. Channel: No substitution of an ordinary Jim String for a C ByteArray input.

The C ByteArray question does not establish a Jim binary-object protocol. Independent ordinary String replacement evidence has its own record.

### bigip

Status: `not-tested`. Channel: No BIG-IP ByteArray input process.

Measured C object results do not establish an appliance/TMM protocol.

## Source inspection

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_StringObjCmd STR_REPLACE case`, lines 2189–2225. Retained reference `exact-source-snippet-0`.

```text
	case STR_REPLACE: {
	    Tcl_UniChar *ustring1;
	    int first, last;

	    if (objc < 5 || objc > 6) {
	        Tcl_WrongNumArgs(interp, 2, objv,
				 "string first last ?string?");
		return TCL_ERROR;
	    }

	    ustring1 = Tcl_GetUnicodeFromObj(objv[2], &length1);
	    length1--;

	    if ((TclGetIntForIndex(interp, objv[3], length1, &first) != TCL_OK)
		    || (TclGetIntForIndex(interp, objv[4], length1,
			    &last) != TCL_OK)) {
		return TCL_ERROR;
	    }

	    if ((last < first) || (last < 0) || (first > length1)) {
		Tcl_SetObjResult(interp, objv[2]);
	    } else {
		if (first < 0) {
		    first = 0;
		}

		Tcl_SetUnicodeObj(resultPtr, ustring1, first);
		if (objc == 6) {
		    Tcl_AppendObjToObj(resultPtr, objv[5]);
		}
		if (last < length1) {
		    Tcl_AppendUnicodeToObj(resultPtr, ustring1 + last + 1,
			    length1 - last);
		}
	    }
	    break;
	}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdMZ.c`, function `StringRplcCmd`, lines 2258–2303. Retained reference `exact-source-snippet-1`.

```text
StringRplcCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_UniChar *ustring;
    int first, last, length;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last ?string?");
	return TCL_ERROR;
    }

    ustring = Tcl_GetUnicodeFromObj(objv[1], &length);
    length--;

    if (TclGetIntForIndexM(interp, objv[2], length, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], length, &last) != TCL_OK){
	return TCL_ERROR;
    }

    if ((last < first) || (last < 0) || (first > length)) {
	Tcl_SetObjResult(interp, objv[1]);
    } else {
	Tcl_Obj *resultPtr;

	ustring = Tcl_GetUnicodeFromObj(objv[1], &length);
	length--;

	if (first < 0) {
	    first = 0;
	}

	resultPtr = Tcl_NewUnicodeObj(ustring, first);
	if (objc == 5) {
	    Tcl_AppendObjToObj(resultPtr, objv[4]);
	}
	if (last < length) {
	    Tcl_AppendUnicodeToObj(resultPtr, ustring + last + 1,
		    length - last);
	}
	Tcl_SetObjResult(interp, resultPtr);
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdMZ.c`, function `StringRplcCmd`, lines 2329–2392. Retained reference `exact-source-snippet-2`.

```text
StringRplcCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_UniChar *ustring;
    int first, last, length, end;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last ?string?");
	return TCL_ERROR;
    }

    ustring = Tcl_GetUnicodeFromObj(objv[1], &length);
    end = length - 1;

    if (TclGetIntForIndexM(interp, objv[2], end, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], end, &last) != TCL_OK){
	return TCL_ERROR;
    }

    /*
     * The following test screens out most empty substrings as
     * candidates for replacement. When they are detected, no
     * replacement is done, and the result is the original string,
     */
    if ((last < 0) ||		/* Range ends before start of string */
	    (first > end) ||	/* Range begins after end of string */
	    (last < first)) {	/* Range begins after it starts */

	/*
	 * BUT!!! when (end < 0) -- an empty original string -- we can
	 * have (first <= end < 0 <= last) and an empty string is permitted
	 * to be replaced.
	 */
	Tcl_SetObjResult(interp, objv[1]);
    } else {
	Tcl_Obj *resultPtr;

	/*
	 * We are re-fetching in case the string argument is same value as
	 * an index argument, and shimmering cost us our ustring.
	 */

	ustring = Tcl_GetUnicodeFromObj(objv[1], &length);
	end = length-1;

	if (first < 0) {
	    first = 0;
	}

	resultPtr = Tcl_NewUnicodeObj(ustring, first);
	if (objc == 5) {
	    Tcl_AppendObjToObj(resultPtr, objv[4]);
	}
	if (last < end) {
	    Tcl_AppendUnicodeToObj(resultPtr, ustring + last + 1,
		    end - last);
	}
	Tcl_SetObjResult(interp, resultPtr);
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdMZ.c`, function `StringRplcCmd`, lines 2392–2448. Retained reference `exact-source-snippet-3`.

```text
StringRplcCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Size first, last, end;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last ?string?");
	return TCL_ERROR;
    }

    end = Tcl_GetCharLength(objv[1]) - 1;

    if (TclGetIntForIndexM(interp, objv[2], end, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], end, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * The following test screens out most empty substrings as candidates for
     * replacement. When they are detected, no replacement is done, and the
     * result is the original string.
     */

    if ((last < 0) ||	/* Range ends before start of string */
	    (first > end) ||	/* Range begins after end of string */
	    (last < first)) {	/* Range begins after it starts */
	/*
	 * BUT!!! when (end < 0) -- an empty original string -- we can
	 * have (first <= end < 0 <= last) and an empty string is permitted
	 * to be replaced.
	 */

	Tcl_SetObjResult(interp, objv[1]);
    } else {
	Tcl_Obj *resultPtr;

	if (first < 0) {
	    first = TCL_INDEX_START;
	}
	if (last > end) {
	    last = end;
	}

	resultPtr = TclStringReplace(interp, objv[1], first,
		last + 1 - first, (objc == 5) ? objv[4] : NULL,
		TCL_STRING_IN_PLACE);

	if (resultPtr == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, resultPtr);
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `TclStringReplace`, lines 4189–4304. Retained reference `exact-source-snippet-4`.

```text
TclStringReplace(
    Tcl_Interp *interp,		/* For error reporting, may be NULL */
    Tcl_Obj *objPtr,		/* String to act upon */
    Tcl_Size first,		/* First index to replace */
    Tcl_Size count,		/* How many chars to replace */
    Tcl_Obj *insertPtr,		/* Replacement string, may be NULL */
    int flags)			/* TCL_STRING_IN_PLACE => attempt in-place */
{
    int inPlace = flags & TCL_STRING_IN_PLACE;
    Tcl_Obj *result;

    /* Replace nothing with nothing */
    if ((insertPtr == NULL) && (count <= 0)) {
	if (inPlace) {
	    return objPtr;
	} else {
	    return Tcl_DuplicateObj(objPtr);
	}
    }
    if (first < 0) {
	first = 0;
    }

    /*
     * The caller very likely had to call Tcl_GetCharLength() or similar
     * to be able to process index values.  This means it is likely that
     * objPtr is either a proper "byte-array" or a "string" or else it has
     * a known and short string rep.
     */

    if (TclIsPureByteArray(objPtr)) {
	Tcl_Size numBytes = 0;
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &numBytes);

	if (insertPtr == NULL) {
	    /* Replace something with nothing. */

	    assert ( first <= numBytes ) ;
	    assert ( count <= numBytes ) ;
	    assert ( first + count <= numBytes ) ;

	    result = Tcl_NewByteArrayObj(NULL, numBytes - count);/* PANIC? */
	    TclAppendBytesToByteArray(result, bytes, first);
	    TclAppendBytesToByteArray(result, bytes + first + count,
		    numBytes - count - first);
	    return result;
	}

	/* Replace everything */
	if ((first == 0) && (count == numBytes)) {
	    return insertPtr;
	}

	if (TclIsPureByteArray(insertPtr)) {
	    Tcl_Size newBytes = 0;
	    unsigned char *iBytes
		    = Tcl_GetBytesFromObj(NULL, insertPtr, &newBytes);

	    if (count == newBytes && inPlace && !Tcl_IsShared(objPtr)) {
		/*
		 * Removal count and replacement count are equal.
		 * Other conditions permit. Do in-place splice.
		 */

		memcpy(bytes + first, iBytes, count);
		Tcl_InvalidateStringRep(objPtr);
		return objPtr;
	    }

	    if (newBytes > (TCL_SIZE_MAX - (numBytes - count))) {
		if (interp) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "max size for a Tcl value (%" TCL_SIZE_MODIFIER "d bytes) exceeded",
			    TCL_SIZE_MAX));
		    Tcl_SetErrorCode(interp, "TCL", "MEMORY", (char *)NULL);
		}
		return NULL;
	    }
	    result = Tcl_NewByteArrayObj(NULL, numBytes - count + newBytes);
								/* PANIC? */
	    Tcl_SetByteArrayLength(result, 0);
	    TclAppendBytesToByteArray(result, bytes, first);
	    TclAppendBytesToByteArray(result, iBytes, newBytes);
	    TclAppendBytesToByteArray(result, bytes + first + count,
		    numBytes - count - first);
	    return result;
	}

	/* Flow through to try other approaches below */
    }

    /*
     * TODO: Figure out how not to generate a Tcl_UniChar array rep
     * when it can be determined objPtr->bytes points to a string of
     * all single-byte characters so we can index it directly.
     */

    /* The traditional implementation... */
    {
	Tcl_Size numChars;
	Tcl_UniChar *ustring = Tcl_GetUnicodeFromObj(objPtr, &numChars);

	/* TODO: Is there an in-place option worth pursuing here? */

	result = Tcl_NewUnicodeObj(ustring, first);
	if (insertPtr) {
	    Tcl_AppendObjToObj(result, insertPtr);
	}
	if ((first + count) < numChars) {
	    Tcl_AppendUnicodeToObj(result, ustring + first + count,
		    numChars - first - count);
	}

	return result;
    }
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBinary.c`, function `Tcl_SetByteArrayObj`, lines 315–341. Retained reference `exact-source-snippet-5`.

```text
Tcl_SetByteArrayObj(
    Tcl_Obj *objPtr,		/* Object to initialize as a ByteArray. */
    const unsigned char *bytes,	/* The array of bytes to use as the new value.
				 * May be NULL even if numBytes > 0. */
    Tcl_Size numBytes)		/* Number of bytes in the array,
				 * must be >= 0 */
{
    ByteArray *byteArrayPtr;
    Tcl_ObjInternalRep ir;

    if (Tcl_IsShared(objPtr)) {
	Tcl_Panic("%s called with shared object", "Tcl_SetByteArrayObj");
    }
    TclInvalidateStringRep(objPtr);

    assert(numBytes >= 0);
    byteArrayPtr = (ByteArray *)Tcl_Alloc(BYTEARRAY_SIZE(numBytes));
    byteArrayPtr->used = numBytes;
    byteArrayPtr->allocated = numBytes;

    if ((bytes != NULL) && (numBytes > 0)) {
	memcpy(byteArrayPtr->bytes, bytes, numBytes);
    }
    SET_BYTEARRAY(&ir, byteArrayPtr);

    Tcl_StoreInternalRep(objPtr, &properByteArrayType, &ir);
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdMZ.c`, function `StringRplcCmd`, lines 2457–2513. Retained reference `exact-source-snippet-6`.

```text
StringRplcCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size first, last, end;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last ?string?");
	return TCL_ERROR;
    }

    end = Tcl_GetCharLength(objv[1]) - 1;

    if (TclGetIntForIndexM(interp, objv[2], end, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], end, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * The following test screens out most empty substrings as candidates for
     * replacement. When they are detected, no replacement is done, and the
     * result is the original string.
     */

    if ((last < 0) ||		/* Range ends before start of string */
	    (first > end) ||	/* Range begins after end of string */
	    (last < first)) {	/* Range begins after it starts */
	/*
	 * BUT!!! when (end < 0) -- an empty original string -- we can
	 * have (first <= end < 0 <= last) and an empty string is permitted
	 * to be replaced.
	 */

	Tcl_SetObjResult(interp, objv[1]);
    } else {
	Tcl_Obj *resultPtr;

	if (first < 0) {
	    first = TCL_INDEX_START;
	}
	if (last > end) {
	    last = end;
	}

	resultPtr = TclStringReplace(interp, objv[1], first,
		last + 1 - first, (objc == 5) ? objv[4] : NULL,
		TCL_STRING_IN_PLACE);

	if (resultPtr == NULL) {
	    return TCL_ERROR;
	}
	Tcl_SetObjResult(interp, resultPtr);
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `TclStringReplace`, lines 4197–4312. Retained reference `exact-source-snippet-7`.

```text
TclStringReplace(
    Tcl_Interp *interp,		/* For error reporting, may be NULL */
    Tcl_Obj *objPtr,		/* String to act upon */
    Tcl_Size first,		/* First index to replace */
    Tcl_Size count,		/* How many chars to replace */
    Tcl_Obj *insertPtr,		/* Replacement string, may be NULL */
    int flags)			/* TCL_STRING_IN_PLACE => attempt in-place */
{
    int inPlace = flags & TCL_STRING_IN_PLACE;
    Tcl_Obj *result;

    /* Replace nothing with nothing */
    if ((insertPtr == NULL) && (count <= 0)) {
	if (inPlace) {
	    return objPtr;
	} else {
	    return Tcl_DuplicateObj(objPtr);
	}
    }
    if (first < 0) {
	first = 0;
    }

    /*
     * The caller very likely had to call Tcl_GetCharLength() or similar
     * to be able to process index values.  This means it is likely that
     * objPtr is either a proper "byte-array" or a "string" or else it has
     * a known and short string rep.
     */

    if (TclIsPureByteArray(objPtr)) {
	Tcl_Size numBytes = 0;
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &numBytes);

	if (insertPtr == NULL) {
	    /* Replace something with nothing. */

	    assert ( first <= numBytes ) ;
	    assert ( count <= numBytes ) ;
	    assert ( first + count <= numBytes ) ;

	    result = Tcl_NewByteArrayObj(NULL, numBytes - count);/* PANIC? */
	    TclAppendBytesToByteArray(result, bytes, first);
	    TclAppendBytesToByteArray(result, bytes + first + count,
		    numBytes - count - first);
	    return result;
	}

	/* Replace everything */
	if ((first == 0) && (count == numBytes)) {
	    return insertPtr;
	}

	if (TclIsPureByteArray(insertPtr)) {
	    Tcl_Size newBytes = 0;
	    unsigned char *iBytes
		    = Tcl_GetBytesFromObj(NULL, insertPtr, &newBytes);

	    if (count == newBytes && inPlace && !Tcl_IsShared(objPtr)) {
		/*
		 * Removal count and replacement count are equal.
		 * Other conditions permit. Do in-place splice.
		 */

		memcpy(bytes + first, iBytes, count);
		Tcl_InvalidateStringRep(objPtr);
		return objPtr;
	    }

	    if (newBytes > (TCL_SIZE_MAX - (numBytes - count))) {
		if (interp) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "max size for a Tcl value (%" TCL_SIZE_MODIFIER "d bytes) exceeded",
			    TCL_SIZE_MAX));
		    Tcl_SetErrorCode(interp, "TCL", "MEMORY", (char *)NULL);
		}
		return NULL;
	    }
	    result = Tcl_NewByteArrayObj(NULL, numBytes - count + newBytes);
								/* PANIC? */
	    Tcl_SetByteArrayLength(result, 0);
	    TclAppendBytesToByteArray(result, bytes, first);
	    TclAppendBytesToByteArray(result, iBytes, newBytes);
	    TclAppendBytesToByteArray(result, bytes + first + count,
		    numBytes - count - first);
	    return result;
	}

	/* Flow through to try other approaches below */
    }

    /*
     * TODO: Figure out how not to generate a Tcl_UniChar array rep
     * when it can be determined objPtr->bytes points to a string of
     * all single-byte characters so we can index it directly.
     */

    /* The traditional implementation... */
    {
	Tcl_Size numChars;
	Tcl_UniChar *ustring = Tcl_GetUnicodeFromObj(objPtr, &numChars);

	/* TODO: Is there an in-place option worth pursuing here? */

	result = Tcl_NewUnicodeObj(ustring, first);
	if (insertPtr) {
	    Tcl_AppendObjToObj(result, insertPtr);
	}
	if ((first + count) < numChars) {
	    Tcl_AppendUnicodeToObj(result, ustring + first + count,
		    numChars - first - count);
	}

	return result;
    }
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBinary.c`, function `Tcl_SetByteArrayObj`, lines 317–343. Retained reference `exact-source-snippet-8`.

```text
Tcl_SetByteArrayObj(
    Tcl_Obj *objPtr,		/* Object to initialize as a ByteArray. */
    const unsigned char *bytes,	/* The array of bytes to use as the new value.
				 * May be NULL even if numBytes > 0. */
    Tcl_Size numBytes)		/* Number of bytes in the array,
				 * must be >= 0 */
{
    ByteArray *byteArrayPtr;
    Tcl_ObjInternalRep ir;

    if (Tcl_IsShared(objPtr)) {
	Tcl_Panic("%s called with shared object", "Tcl_SetByteArrayObj");
    }
    TclInvalidateStringRep(objPtr);

    assert(numBytes >= 0);
    byteArrayPtr = (ByteArray *)Tcl_Alloc(BYTEARRAY_SIZE(numBytes));
    byteArrayPtr->used = numBytes;
    byteArrayPtr->allocated = numBytes;

    if ((bytes != NULL) && (numBytes > 0)) {
	memcpy(byteArrayPtr->bytes, bytes, numBytes);
    }
    SET_BYTEARRAY(&ir, byteArrayPtr);

    Tcl_StoreInternalRep(objPtr, &properByteArrayType, &ir);
}

```

## Consumer bindings

- [rust/tcl-vm/src/value_ops/native_jim_index.rs](../../../../rust/tcl-vm/src/value_ops/native_jim_index.rs), `original_indexed_binary_inputs_match_supported_native_public_projections` (linked): Software source definition checks indexed binary inputs match supported native public projections.
- [runtime/rust/src/value_ops/native_jim_index.rs](../../../../runtime/rust/src/value_ops/native_jim_index.rs), `original_indexed_binary_inputs_match_supported_native_public_projections` (linked): Software source definition checks indexed binary inputs match supported native public projections.

## Evidence

- [runtime/rust/tests/data/native_original_binary_replacement633/probe.c](../../../../runtime/rust/tests/data/native_original_binary_replacement633/probe.c)
- [runtime/rust/tests/data/native_original_binary_replacement633/8.4.20/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.4.20/same-library-version/execute.stdout)
- [runtime/rust/tests/data/native_original_binary_replacement633/8.5.19/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.5.19/same-library-version/execute.stdout)
- [runtime/rust/tests/data/native_original_binary_replacement633/8.6.18/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_binary_replacement633/8.6.18/same-library-version/execute.stdout)
- [runtime/rust/tests/data/native_original_binary_replacement633/9.0.4/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_binary_replacement633/9.0.4/same-library-version/execute.stdout)
- [runtime/rust/tests/data/native_original_binary_replacement633/9.1.0/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_binary_replacement633/9.1.0/same-library-version/execute.stdout)
