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

Status: `observed`. Version: 9.0.4. Build: Exact compiler invocation/output, selected actual static library/source/header/configuration/CLI bytes, and independent same-library version process. Complete system toolchain/load closure is not recorded.. Channel: Fresh direct TclStringReplace call with literal parsed first/count. Subject/replacement have one genuine reference; case1 adds one subject reference. Flags come from selected tclInt.h. No EvalObjv, index parse or interpreter-result publication is performed.. Dialect: tcl9.0.

With the actual in-place flag, the unshared equal-size splice returns the exact subject; an extra subject reference returns a fresh zero-reference result. Flags0 also returns a fresh result. Whole replacement returns the original replacement. Whole deletion returns a fresh empty ByteArray. Resident subject/replacement String bytes preserve ByteArray eligibility and the unshared in-place role. Partial deletion reaches a fresh ByteArray with unspecified leading payload: the recorded result String is c080c080415a, whose leading bytes are not a deterministic constructor guarantee.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact compiler invocation/output, selected actual static library/source/header/configuration/CLI bytes, and independent same-library version process. Complete system toolchain/load closure is not recorded.. Channel: Fresh direct TclStringReplace call with literal parsed first/count. Subject/replacement have one genuine reference; case1 adds one subject reference. Flags come from selected tclInt.h. No EvalObjv, index parse or interpreter-result publication is performed.. Dialect: tcl9.1.

With the actual in-place flag, the unshared equal-size splice returns the exact subject; an extra subject reference returns a fresh zero-reference result. Flags0 also returns a fresh result. Whole replacement returns the original replacement. Whole deletion returns a fresh empty ByteArray. Resident subject/replacement String bytes preserve ByteArray eligibility and the unshared in-place role. Partial deletion reaches a fresh ByteArray with unspecified leading payload: the recorded result String is 40c289415a, whose leading bytes are not a deterministic constructor guarantee.

### jim

Status: `not-tested`. Version: not tested. Build: No genuine native ByteArray producer is available in the selected Jim object model.. Channel: No substitution of an ordinary Jim String for a C ByteArray input.. Dialect: jim.

The C ByteArray question does not establish a Jim binary-object protocol. Independent ordinary String replacement evidence has its own record.

### bigip

Status: `not-tested`. Version: not tested. Build: No appliance original-header driver capture.. Channel: No BIG-IP ByteArray input process.. Dialect: F5 iRules.

Measured C object results do not establish an appliance/TMM protocol.

### tcl8.4

Status: `not-tested`. Version: not tested. Build: The direct parsed-range helper question is restricted to selected C9 libraries.. Channel: No direct helper process requested for 8.4.20.. Dialect: tcl8.4.

This direct C9 helper capture does not establish tcl8.4 behavior. Indexed original ByteArray command evidence has its own record.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: The direct parsed-range helper question is restricted to selected C9 libraries.. Channel: No direct helper process requested for 8.5.19.. Dialect: tcl8.5.

This direct C9 helper capture does not establish tcl8.5 behavior. Indexed original ByteArray command evidence has its own record.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: The direct parsed-range helper question is restricted to selected C9 libraries.. Channel: No direct helper process requested for 8.6.18.. Dialect: tcl8.6.

This direct C9 helper capture does not establish tcl8.6 behavior. Indexed original ByteArray command evidence has its own record.

## Exact evidence

- `original-counted-probe` (input): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/probe.c](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/probe.c). SHA-256 `12e3a810df236b87cf35405cfd3cb707d5beb6a7167ed298b133cd7ddbab6e2f`. Actual direct parsed-index helper, original ByteArray constructors, one or two explicit subject references, header-issued flags and snapshots before result materialisation.
- `original-request` (input): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/request.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/request.json). SHA-256 `ad10337f0a1690214d987c8678f1c5f181acda84e71ec59789fcd9a4e81590e8`. Question and input requirements; no expected output is supplied.
- `actual-case-projection` (observation): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/interpretation.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/interpretation.json). SHA-256 `7f5cf0bf75e5fa0c48452d677ed582ef14660e4e57f0dfaf3362a03ca1f647ce`. All16 raw stream projections; process completion and command completion remain separate.
- `exact-input-map` (provider): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/pinned-input-map.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/pinned-input-map.json). SHA-256 `1ab7435b9a7b1e83b8f89b49967dfc0b4974627c3ff5f5147f89f83c1a6d198e`. All original selected compiler/library/source/header/CLI/input pins and content-addressed archive join.
- `exact-input-bytes` (provider): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/pinned-input-bytes.tar.gz](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/pinned-input-bytes.tar.gz). SHA-256 `8935acfc1bbed72bcec39de143da671b98098855ec1132ea5fd0fbfe49784215`. Whole retained original inputs, independently hash-verified after decompression; system toolchain/load closure remains outside the recorded scope.
- `exact-source-windows` (source-anchor): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/source-windows.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/source-windows.json). SHA-256 `898a5ed06f8bdd65790b2750c5ce73d336d9c38402bdda7ad043b50848c3d039`. Actual selected command/helper/setter source explains C8 Unicode conversion, C9 result identity and unspecified partial-deletion prefix.
- `all-original-retention` (provider): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/retention.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/retention.json). SHA-256 `d39775e4fb0abed991e825d1a155b541cd16b0488f43f6501e6779193d8e7155`. Original receipts, raw streams and executable images retained without truncation.
- `tcl9.0-receipt-json` (provider): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.0.4/receipt.json). SHA-256 `028d63ef3c7e27198d2c673a9381b05d48cc75ed6abdc59739db7c4d2b8cfe9d`. Actual original selected compile/link recipe, CLI version, input pins and fresh case processes.
- `tcl9.0-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.0.4/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.0.4/same-library-version/receipt.json). SHA-256 `beb578e8296562e62f4264e2bc943a77e264c19de4f9740897fd7a69553f5223`. Independent later diagnostic using the identical actual static library and headers; original case streams are unchanged.
- `tcl9.0-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.0.4/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.0.4/same-library-version/execute.stdout). SHA-256 `d016aed0da5636f83bfe6d76f7c26b06e30ce7951203b5d3543001780eb51a2c`. Direct selected-library version query and patchlevel output.
- `tcl9.1-receipt-json` (provider): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.1.0/receipt.json). SHA-256 `3d9c89fedd7bc17eb487229d929db6de91038883bc07dfed0a9def10530ee551`. Actual original selected compile/link recipe, CLI version, input pins and fresh case processes.
- `tcl9.1-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.1.0/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.1.0/same-library-version/receipt.json). SHA-256 `862765a346b1abfb223b3d07d114639aaac4398d0998c4b7cf3389190469da93`. Independent later diagnostic using the identical actual static library and headers; original case streams are unchanged.
- `tcl9.1-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.1.0/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/9.1.0/same-library-version/execute.stdout). SHA-256 `b225f15b75641002abfa4f29d4d1be66051123c85bc897b9a7234b22317ea071`. Direct selected-library version query and patchlevel output.
- `exact-source-snippet-0` (source-anchor): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/source-windows.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/source-windows.json). SHA-256 `898a5ed06f8bdd65790b2750c5ce73d336d9c38402bdda7ad043b50848c3d039`. JSON pointer `/anchors/0/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.
- `exact-source-snippet-1` (source-anchor): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/source-windows.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/source-windows.json). SHA-256 `898a5ed06f8bdd65790b2750c5ce73d336d9c38402bdda7ad043b50848c3d039`. JSON pointer `/anchors/1/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.
- `exact-source-snippet-2` (source-anchor): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/source-windows.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/source-windows.json). SHA-256 `898a5ed06f8bdd65790b2750c5ce73d336d9c38402bdda7ad043b50848c3d039`. JSON pointer `/anchors/2/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.
- `exact-source-snippet-3` (source-anchor): [runtime/rust/tests/data/native_original_direct_binary_replacement634a/source-windows.json](../../../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/source-windows.json). SHA-256 `898a5ed06f8bdd65790b2750c5ce73d336d9c38402bdda7ad043b50848c3d039`. JSON pointer `/anchors/3/snippet`. Original physical-LF snippet selected as decoded JSON text and joined to the retained whole source hash.

## Source inspection

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `TclStringReplace`, lines 4189–4304. Full-source SHA-256 `d44c80d7de637da41ac841c6c55c25c4ee302efc774c2c4cfe5d8ff73193b7b4`; snippet SHA-256 `042f4ccded53127fb70a28760d34fba835483c1d57d52f78c17fcf718bce92ea`; retained evidence `exact-source-snippet-0`.

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

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBinary.c`, function `Tcl_SetByteArrayObj`, lines 315–341. Full-source SHA-256 `5660d9f82d02f1ca213a0e9e0b96ff959a81d6f6a474229fb7eada8edfde40bf`; snippet SHA-256 `27908deda03f1103aa76bf019a780f614b3c6b03b2cc654846a68346702a8991`; retained evidence `exact-source-snippet-1`.

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

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `TclStringReplace`, lines 4197–4312. Full-source SHA-256 `d1ecd65e375cee3b1ab86111cf5b90fb6bf4d7b5e71448cd361e4026d32a4bf8`; snippet SHA-256 `042f4ccded53127fb70a28760d34fba835483c1d57d52f78c17fcf718bce92ea`; retained evidence `exact-source-snippet-2`.

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

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBinary.c`, function `Tcl_SetByteArrayObj`, lines 317–343. Full-source SHA-256 `2c4d8facc256ec7e135164f92119d4dcf5f98fc3369c0b35a1e89ba950d61b7a`; snippet SHA-256 `27908deda03f1103aa76bf019a780f614b3c6b03b2cc654846a68346702a8991`; retained evidence `exact-source-snippet-3`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Retain the exact direct helper source and eight cases. Headers supply the actual prototype and in-place flag. Each provider receipt records compile/link/launch/environment and selected input hashes; verify them or treat rebuilt artifacts as a new experiment. Keep partial-deletion leading bytes observational and distinguish these parsed-range/helper roles from indexed EvalObjv.
