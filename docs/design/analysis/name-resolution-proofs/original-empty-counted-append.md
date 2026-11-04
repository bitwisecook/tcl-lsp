# naming.object.original-empty-counted-append

Kind: `native-observation`

## Problem statement

Returning early from a zero-length append before the actual selected release converts its receiver changes the original primary, even though counted bytes are equal.

## Question

For six original unshared C object constructors and reached String/Unicode states, which release converts the primary during a zero-length counted append, before any result string getter?

## Conclusion

C8.4/8.5 reach SetStringFromAny before the empty return: NULL and int become String. C8.6/9.0/9.1 return first and preserve those primaries. Existing String/Unicode states and original counted bytes are preserved by all five measured providers.

## Scope

Exactly six constructor/representation states, one unshared Tcl_AppendToObj(obj,"",0) call and separate before/after-primary/resident windows per C provider. The counted byte getter follows those observations and does not donate a prior resident representation. No shared-object behavior, observer/variable publication, command dispatch, Normal completion, general append theorem, Jim C API, CPP/source receipt or Native admission follows.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Executable; header; library. Channel: Direct unshared original native objects; zero-length counted append; exact header observations and later counted getter hex. Dialect: Tcl.

NULL and int primaries become String before the empty return; String/Unicode primary and original counted bytes remain intact.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Executable; header; library. Channel: Direct unshared original native objects; zero-length counted append; exact header observations and later counted getter hex. Dialect: Tcl.

NULL and int primaries become String before the empty return; String/Unicode primary and original counted bytes remain intact.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Executable; header; library. Channel: Direct unshared original native objects; zero-length counted append; exact header observations and later counted getter hex. Dialect: Tcl.

NULL and int primaries remain unchanged across the empty return; String/Unicode primary and original counted bytes remain intact.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executable; header; library. Channel: Direct unshared original native objects; zero-length counted append; exact header observations and later counted getter hex. Dialect: Tcl.

NULL and int primaries remain unchanged across the empty return; String/Unicode primary and original counted bytes remain intact.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executable; header; library. Channel: Direct unshared original native objects; zero-length counted append; exact header observations and later counted getter hex. Dialect: Tcl.

NULL and int primaries remain unchanged across the empty return; String/Unicode primary and original counted bytes remain intact.

### jim

Status: `not-tested`. Version: not-tested. Build: No execution receipt for this question. Channel: not-tested. Dialect: jim.

No C append API observation is attached for this provider.

### bigip

Status: `not-tested`. Version: not-tested. Build: No execution receipt for this question. Channel: not-tested. Dialect: bigip.

No C append API observation is attached for this provider.

## Historical references

- `empty-append-tcl8.4-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/receipt.json). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.4-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/stdout.tsv). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/stderr). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.5-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/receipt.json). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.5-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/stdout.tsv). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/stderr). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.6-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/receipt.json). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.6-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/stdout.tsv). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/stderr). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.0-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/receipt.json). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.0-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/stdout.tsv). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/stderr). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.1-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/receipt.json). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.1-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/stdout.tsv). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/stderr). Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-probe-c` (input): [rust/tcl-registry/tests/data/native_empty_counted_append/probe.c](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/probe.c). Exact native input/capture protocol or aggregate provider receipts; queue criteria remain provider scaffolding.
- `empty-append-queue-json` (input): [rust/tcl-registry/tests/data/native_empty_counted_append/queue.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/queue.json). Exact native input/capture protocol or aggregate provider receipts; queue criteria remain provider scaffolding.
- `empty-append-capture-py` (input): [rust/tcl-registry/tests/data/native_empty_counted_append/capture.py](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/capture.py). Exact native input/capture protocol or aggregate provider receipts; queue criteria remain provider scaffolding.
- `empty-append-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/receipt.json). Exact native input/capture protocol or aggregate provider receipts; queue criteria remain provider scaffolding.
- `empty-append-source-0` (source-anchor): [rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json). JSON pointer `/source_anchors/0/snippet`. tcl8.4 reached conversion and zero-length return ordering in exact C source.
- `empty-append-source-1` (source-anchor): [rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json). JSON pointer `/source_anchors/1/snippet`. tcl8.5 reached conversion and zero-length return ordering in exact C source.
- `empty-append-source-2` (source-anchor): [rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json). JSON pointer `/source_anchors/2/snippet`. tcl8.6 reached conversion and zero-length return ordering in exact C source.
- `empty-append-source-3` (source-anchor): [rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json). JSON pointer `/source_anchors/3/snippet`. tcl9.0 reached conversion and zero-length return ordering in exact C source.
- `empty-append-source-4` (source-anchor): [rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json). JSON pointer `/source_anchors/4/snippet`. tcl9.1 reached conversion and zero-length return ordering in exact C source.

## Source inspection

tcl8.4 8.4.20, revision `Retained release source identified by complete file digest`, `tmp/tcl8.4.20/generic/tclStringObj.c`, function `Tcl_AppendToObj`, lines 1086–1123. Retained reference `empty-append-source-0`.

```text
Tcl_AppendToObj(objPtr, bytes, length)
    register Tcl_Obj *objPtr;	/* Points to the object to append to. */
    CONST char *bytes;		/* Points to the bytes to append to the
				 * object. */
    register int length;	/* The number of bytes to append from
				 * "bytes". If < 0, then append all bytes
				 * up to NULL byte. */
{
    String *stringPtr;

    if (Tcl_IsShared(objPtr)) {
	panic("Tcl_AppendToObj called with shared object");
    }
    
    SetStringFromAny(NULL, objPtr);

    if (length < 0) {
	length = (bytes ? strlen(bytes) : 0);
    }
    if (length == 0) {
	return;
    }

    /*
     * If objPtr has a valid Unicode rep, then append the Unicode
     * conversion of "bytes" to the objPtr's Unicode rep, otherwise
     * append "bytes" to objPtr's string rep.
     */

    stringPtr = GET_STRING(objPtr);
    if (stringPtr->hasUnicode != 0) {
	AppendUtfToUnicodeRep(objPtr, bytes, length);

	stringPtr = GET_STRING(objPtr);
    } else {
	AppendUtfToUtfRep(objPtr, bytes, length);
    }
}

```

tcl8.5 8.5.19, revision `Retained release source identified by complete file digest`, `tmp/tcl8.5.19/generic/tclStringObj.c`, function `Tcl_AppendLimitedToObj`, lines 1107–1169. Retained reference `empty-append-source-1`.

```text
Tcl_AppendLimitedToObj(
    register Tcl_Obj *objPtr,	/* Points to the object to append to. */
    const char *bytes,		/* Points to the bytes to append to the
				 * object. */
    register int length,	/* The number of bytes available to be
				 * appended from "bytes". If < 0, then all
				 * bytes up to a NUL byte are available. */
    register int limit,		/* The maximum number of bytes to append to
				 * the object. */
    const char *ellipsis)	/* Ellipsis marker string, appended to the
				 * object to indicate not all available bytes
				 * at "bytes" were appended. */
{
    String *stringPtr;
    int toCopy = 0;

    if (Tcl_IsShared(objPtr)) {
	Tcl_Panic("%s called with shared object", "Tcl_AppendLimitedToObj");
    }

    SetStringFromAny(NULL, objPtr);

    if (length < 0) {
	length = (bytes ? strlen(bytes) : 0);
    }
    if (length == 0) {
	return;
    }

    if (length <= limit) {
	toCopy = length;
    } else {
	if (ellipsis == NULL) {
	    ellipsis = "...";
	}
	toCopy = (bytes == NULL) ? limit
		: Tcl_UtfPrev(bytes+limit+1-strlen(ellipsis), bytes) - bytes;
    }

    /*
     * If objPtr has a valid Unicode rep, then append the Unicode conversion
     * of "bytes" to the objPtr's Unicode rep, otherwise append "bytes" to
     * objPtr's string rep.
     */

    stringPtr = GET_STRING(objPtr);
    if (stringPtr->hasUnicode != 0) {
	AppendUtfToUnicodeRep(objPtr, bytes, toCopy);
    } else {
	AppendUtfToUtfRep(objPtr, bytes, toCopy);
    }

    if (length <= limit) {
	return;
    }

    stringPtr = GET_STRING(objPtr);
    if (stringPtr->hasUnicode != 0) {
	AppendUtfToUnicodeRep(objPtr, ellipsis, -1);
    } else {
	AppendUtfToUtfRep(objPtr, ellipsis, -1);
    }
}

```

tcl8.6 8.6.18, revision `Retained release source identified by complete file digest`, `tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_AppendLimitedToObj`, lines 1193–1263. Retained reference `empty-append-source-2`.

```text
Tcl_AppendLimitedToObj(
    Tcl_Obj *objPtr,		/* Points to the object to append to. */
    const char *bytes,		/* Points to the bytes to append to the
				 * object. */
    int length,		/* The number of bytes available to be
				 * appended from "bytes". If -1, then
				 * all bytes up to a NUL byte are available. */
    int limit,		/* The maximum number of bytes to append to
				 * the object. */
    const char *ellipsis)	/* Ellipsis marker string, appended to the
				 * object to indicate not all available bytes
				 * at "bytes" were appended. */
{
    String *stringPtr;
    int toCopy = 0;
    int eLen = 0;

    if (length < 0) {
	length = (bytes ? strlen(bytes) : 0);
    }
    if (length == 0) {
	return;
    }
    if (limit <= 0) {
	return;
    }

    if (length <= limit) {
	toCopy = length;
    } else {
	if (ellipsis == NULL) {
	    ellipsis = "...";
	}
	eLen = strlen(ellipsis);
	while (eLen > limit) {
	    eLen = TclUtfPrev(ellipsis+eLen, ellipsis) - ellipsis;
	}

	toCopy = TclUtfPrev(bytes+limit+1-eLen, bytes) - bytes;
    }

    /*
     * If objPtr has a valid Unicode rep, then append the Unicode conversion
     * of "bytes" to the objPtr's Unicode rep, otherwise append "bytes" to
     * objPtr's string rep.
     */

    if (Tcl_IsShared(objPtr)) {
	Tcl_Panic("%s called with shared object", "Tcl_AppendLimitedToObj");
    }

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->hasUnicode && stringPtr->numChars > 0) {
	AppendUtfToUnicodeRep(objPtr, bytes, toCopy);
    } else {
	AppendUtfToUtfRep(objPtr, bytes, toCopy);
    }

    if (length <= limit) {
	return;
    }

    stringPtr = GET_STRING(objPtr);
    if (stringPtr->hasUnicode && (stringPtr->numChars > 0)) {
	AppendUtfToUnicodeRep(objPtr, ellipsis, eLen);
    } else {
	AppendUtfToUtfRep(objPtr, ellipsis, eLen);
    }
}

```

tcl9.0 9.0.4, revision `Retained release source identified by complete file digest`, `tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_AppendLimitedToObj`, lines 1203–1279. Retained reference `empty-append-source-3`.

```text
Tcl_AppendLimitedToObj(
    Tcl_Obj *objPtr,		/* Points to the object to append to. */
    const char *bytes,		/* Points to the bytes to append to the
				 * object. */
    Tcl_Size length,		/* The number of bytes available to be
				 * appended from "bytes". If < 0, then
				 * all bytes up to a NUL byte are available. */
    Tcl_Size limit,		/* The maximum number of bytes to append to
				 * the object. */
    const char *ellipsis)	/* Ellipsis marker string, appended to the
				 * object to indicate not all available bytes
				 * at "bytes" were appended. */
{
    String *stringPtr;
    Tcl_Size toCopy = 0;
    Tcl_Size eLen = 0;

    if (length < 0) {
	length = (bytes ? strlen(bytes) : 0);
    }
    if (length == 0) {
	return;
    }
    if (limit <= 0) {
	return;
    }

    if (length <= limit) {
	toCopy = length;
    } else {
	if (ellipsis == NULL) {
	    ellipsis = "...";
	}
	eLen = strlen(ellipsis);
	while (eLen > limit) {
	    eLen = Tcl_UtfPrev(ellipsis+eLen, ellipsis) - ellipsis;
	}

	toCopy = Tcl_UtfPrev(bytes+limit+1-eLen, bytes) - bytes;
    }

    /*
     * If objPtr has a valid Unicode rep, then append the Unicode conversion
     * of "bytes" to the objPtr's Unicode rep, otherwise append "bytes" to
     * objPtr's string rep.
     */

    if (Tcl_IsShared(objPtr)) {
	Tcl_Panic("%s called with shared object", "Tcl_AppendLimitedToObj");
    }

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    /* If appended string starts with a continuation byte or a lower surrogate,
     * force objPtr to unicode representation. See [7f1162a867] */
    if (bytes && ISCONTINUATION(bytes)) {
	Tcl_GetUnicode(objPtr);
	stringPtr = GET_STRING(objPtr);
    }
    if (stringPtr->hasUnicode && (stringPtr->numChars > 0)) {
	AppendUtfToUnicodeRep(objPtr, bytes, toCopy);
    } else {
	AppendUtfToUtfRep(objPtr, bytes, toCopy);
    }

    if (length <= limit) {
	return;
    }

    stringPtr = GET_STRING(objPtr);
    if (stringPtr->hasUnicode && (stringPtr->numChars > 0)) {
	AppendUtfToUnicodeRep(objPtr, ellipsis, eLen);
    } else {
	AppendUtfToUtfRep(objPtr, ellipsis, eLen);
    }
}

```

tcl9.1 9.1.0, revision `Retained release source identified by complete file digest`, `tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_AppendLimitedToObj`, lines 1206–1282. Retained reference `empty-append-source-4`.

```text
Tcl_AppendLimitedToObj(
    Tcl_Obj *objPtr,		/* Points to the object to append to. */
    const char *bytes,		/* Points to the bytes to append to the
				 * object. */
    Tcl_Size length,		/* The number of bytes available to be
				 * appended from "bytes". If < 0, then
				 * all bytes up to a NUL byte are available. */
    Tcl_Size limit,		/* The maximum number of bytes to append to
				 * the object. */
    const char *ellipsis)	/* Ellipsis marker string, appended to the
				 * object to indicate not all available bytes
				 * at "bytes" were appended. */
{
    String *stringPtr;
    Tcl_Size toCopy = 0;
    Tcl_Size eLen = 0;

    if (length < 0) {
	length = (bytes ? strlen(bytes) : 0);
    }
    if (length == 0) {
	return;
    }
    if (limit <= 0) {
	return;
    }

    if (length <= limit) {
	toCopy = length;
    } else {
	if (ellipsis == NULL) {
	    ellipsis = "...";
	}
	eLen = strlen(ellipsis);
	while (eLen > limit) {
	    eLen = Tcl_UtfPrev(ellipsis+eLen, ellipsis) - ellipsis;
	}

	toCopy = Tcl_UtfPrev(bytes+limit+1-eLen, bytes) - bytes;
    }

    /*
     * If objPtr has a valid Unicode rep, then append the Unicode conversion
     * of "bytes" to the objPtr's Unicode rep, otherwise append "bytes" to
     * objPtr's string rep.
     */

    if (Tcl_IsShared(objPtr)) {
	Tcl_Panic("%s called with shared object", "Tcl_AppendLimitedToObj");
    }

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    /* If appended string starts with a continuation byte or a lower surrogate,
     * force objPtr to unicode representation. See [7f1162a867] */
    if (bytes && ISCONTINUATION(bytes)) {
	Tcl_GetUnicode(objPtr);
	stringPtr = GET_STRING(objPtr);
    }
    if (stringPtr->hasUnicode && (stringPtr->numChars > 0)) {
	AppendUtfToUnicodeRep(objPtr, bytes, toCopy);
    } else {
	AppendUtfToUtfRep(objPtr, bytes, toCopy);
    }

    if (length <= limit) {
	return;
    }

    stringPtr = GET_STRING(objPtr);
    if (stringPtr->hasUnicode && (stringPtr->numChars > 0)) {
	AppendUtfToUnicodeRep(objPtr, ellipsis, eLen);
    } else {
	AppendUtfToUtfRep(objPtr, ellipsis, eLen);
    }
}

```


## Consumer bindings

- [rust/tcl-syntax/src/native_object_append.rs](../../../../rust/tcl-syntax/src/native_object_append.rs), `NativeObjectAppendProtocol::converts_empty_counted_receiver`: Pure selected C release zero-length conversion order; no backend authority.
- [rust/tcl-cmd-core/src/native_append.rs](../../../../rust/tcl-cmd-core/src/native_append.rs), `append_counted_bytes`: Existing concrete receiver COW/getter/String conversion path is reached only by releases that actually convert before empty return.
- [rust/tcl-vm/src/exec/native_namespace_string_tests.rs](../../../../rust/tcl-vm/src/exec/native_namespace_string_tests.rs), `exec::native_namespace_string_tests::original_empty_counted_append_matches_thirty_native_constructor_windows` (linked): Original constructor primary/resident before and after a zero-length append, reached-cache preservation and later exact counted bytes across thirty measured windows.

A named test is a coverage binding, not a claim that it executed.

## Replay

The native probe and retained capture runner use exact C5 headers/libraries; native statuses are observed from retained receipts, while the Rust selector is linked without an execution receipt. Jim and vendor are not tested.
