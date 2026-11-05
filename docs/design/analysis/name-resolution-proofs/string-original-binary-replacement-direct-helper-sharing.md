# naming.string.original-binary-replacement-direct-helper-sharing

Kind: `native-observation`

## Problem statement

The parsed-range replacement helper and indexed EvalObjv caller have different reference schedules. A shared replacement algorithm must preserve the genuine helper sharing and flag producers without inferring them from command snapshots or manufacturing native holds.

## Question

For eight direct C Tcl9 replacement helper cases with parsed first/count, header-issued flags and original ByteArray references, which subject/replacement/fresh header is returned, and what public fields and result String bytes are measured?

## Conclusion

Both selected C9 helpers return the original subject for an unshared equal-size in-place splice, including resident String cases. A shared subject or flags0 produces a fresh zero-reference result. Whole replacement returns the original replacement; whole deletion produces a fresh empty ByteArray. Partial-deletion leading bytes differ between the two measured processes and are not initialised by the source constructor. The direct helper does not establish an indexed command ownership schedule.

## Scope

16 fresh direct-helper processes and two independent identical-static-library version diagnostics. Original public snapshots precede result String materialisation; actual fresh results are retired afterward with BounceRefCount. Private cache internals, parsed index conversion, command dispatch, interpreter result publication, arbitrary allocator configurations, Rust assertions, C8, Jim and BIG-IP are not measured. Linked Rust definitions are software coverage obligations. Their source assertions require independent execution receipts; they add no provider observation, feature availability, native object/frame or edit authority.

## Provider answers

### tcl9.0

Status: `observed`. Channel: Fresh direct TclStringReplace call with literal parsed first/count. Subject/replacement have one genuine reference; case1 adds one subject reference. Flags come from selected tclInt.h. No EvalObjv, index parse or interpreter-result publication is performed. Version: tcl9.0 9.0.4.

With the actual in-place flag, the unshared equal-size splice returns the exact subject; an extra subject reference returns a fresh zero-reference result. Flags0 also returns a fresh result. Whole replacement returns the original replacement. Whole deletion returns a fresh empty ByteArray. Resident subject/replacement String bytes preserve ByteArray eligibility and the unshared in-place role. Partial deletion reaches a fresh ByteArray with unspecified leading payload: the recorded result String is c080c080415a, whose leading bytes are not a deterministic constructor guarantee.

### tcl9.1

Status: `observed`. Channel: Fresh direct TclStringReplace call with literal parsed first/count. Subject/replacement have one genuine reference; case1 adds one subject reference. Flags come from selected tclInt.h. No EvalObjv, index parse or interpreter-result publication is performed. Version: tcl9.1 9.1.0.

With the actual in-place flag, the unshared equal-size splice returns the exact subject; an extra subject reference returns a fresh zero-reference result. Flags0 also returns a fresh result. Whole replacement returns the original replacement. Whole deletion returns a fresh empty ByteArray. Resident subject/replacement String bytes preserve ByteArray eligibility and the unshared in-place role. Partial deletion reaches a fresh ByteArray with unspecified leading payload: the recorded result String is 40c289415a, whose leading bytes are not a deterministic constructor guarantee.

### jim

Status: `not-tested`. Channel: No substitution of an ordinary Jim String for a C ByteArray input.

The C ByteArray question does not establish a Jim binary-object protocol. Independent ordinary String replacement evidence has its own record.

### bigip

Status: `not-tested`. Channel: No BIG-IP ByteArray input process.

Measured C object results do not establish an appliance/TMM protocol.

### tcl8.4

Status: `not-tested`. Channel: No direct helper process requested for 8.4.20.

This direct C9 helper capture does not establish tcl8.4 behavior. Indexed original ByteArray command evidence has its own record.

### tcl8.5

Status: `not-tested`. Channel: No direct helper process requested for 8.5.19.

This direct C9 helper capture does not establish tcl8.5 behavior. Indexed original ByteArray command evidence has its own record.

### tcl8.6

Status: `not-tested`. Channel: No direct helper process requested for 8.6.18.

This direct C9 helper capture does not establish tcl8.6 behavior. Indexed original ByteArray command evidence has its own record.

## Source inspection

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `TclStringReplace`, lines 4189–4304. Retained reference `exact-source-snippet-0`.

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

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBinary.c`, function `Tcl_SetByteArrayObj`, lines 315–341. Retained reference `exact-source-snippet-1`.

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

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `TclStringReplace`, lines 4197–4312. Retained reference `exact-source-snippet-2`.

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

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBinary.c`, function `Tcl_SetByteArrayObj`, lines 317–343. Retained reference `exact-source-snippet-3`.

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

- [rust/tcl-vm/src/value_ops/native_jim_index.rs](../../../../rust/tcl-vm/src/value_ops/native_jim_index.rs), `original_direct_c9_binary_replacement_matches_deterministic_native_roles` (linked): Software source definition checks direct c9 binary replacement matches deterministic native roles.
- [rust/tcl-vm/src/value_ops/native_jim_index.rs](../../../../rust/tcl-vm/src/value_ops/native_jim_index.rs), `omitted_zero_count_without_in_place_uses_original_duplicate_constructor` (linked): Software source definition checks omitted zero count without in place uses original duplicate constructor.
- [runtime/rust/src/value_ops/native_jim_index.rs](../../../../runtime/rust/src/value_ops/native_jim_index.rs), `original_direct_c9_binary_replacement_matches_deterministic_native_roles` (linked): Software source definition checks direct c9 binary replacement matches deterministic native roles.
- [runtime/rust/src/value_ops/native_jim_index.rs](../../../../runtime/rust/src/value_ops/native_jim_index.rs), `omitted_zero_count_without_in_place_uses_original_duplicate_constructor` (linked): Software source definition checks omitted zero count without in place uses original duplicate constructor.

## Evidence

- [runtime/rust/tests/data/native_original_direct_binary_replacement634a/probe.c](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/probe.c)
- [runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.0.4/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.0.4/same-library-version/execute.stdout)
- [runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.1.0/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.1.0/same-library-version/execute.stdout)
