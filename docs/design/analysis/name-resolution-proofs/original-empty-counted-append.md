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

Status: `observed`. Version: 8.4.20. Build: Executable SHA-256 b1a4510fd6c50bf4e49095039c78f43bab9caef584361b5edf2a250d35daa41e; header SHA-256 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; library SHA-256 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47. Channel: Direct unshared original native objects; zero-length counted append; exact header observations and later counted getter hex. Dialect: Tcl.

NULL and int primaries become String before the empty return; String/Unicode primary and original counted bytes remain intact.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Executable SHA-256 95bb26cf55fc650825751b942ff05fc8aaf4fbb3e8dd724ec2df6f7621c07cf3; header SHA-256 c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; library SHA-256 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc. Channel: Direct unshared original native objects; zero-length counted append; exact header observations and later counted getter hex. Dialect: Tcl.

NULL and int primaries become String before the empty return; String/Unicode primary and original counted bytes remain intact.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Executable SHA-256 29e4494f674eb8eeef3ae533c05a8d6cad1ac404a5bd0908af77823adcdef385; header SHA-256 aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; library SHA-256 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb. Channel: Direct unshared original native objects; zero-length counted append; exact header observations and later counted getter hex. Dialect: Tcl.

NULL and int primaries remain unchanged across the empty return; String/Unicode primary and original counted bytes remain intact.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executable SHA-256 9fcbe5d4b83b4d6cc76332435bd868315930530d47efb72fa2f680015b827c22; header SHA-256 eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; library SHA-256 dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4. Channel: Direct unshared original native objects; zero-length counted append; exact header observations and later counted getter hex. Dialect: Tcl.

NULL and int primaries remain unchanged across the empty return; String/Unicode primary and original counted bytes remain intact.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executable SHA-256 dc0a372b4f7c461f743aa9e7e86ac5252e84f97e8041f58449b2fe279e19df1c; header SHA-256 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; library SHA-256 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db. Channel: Direct unshared original native objects; zero-length counted append; exact header observations and later counted getter hex. Dialect: Tcl.

NULL and int primaries remain unchanged across the empty return; String/Unicode primary and original counted bytes remain intact.

### jim

Status: `not-tested`. Version: not-tested. Build: No execution receipt for this question. Channel: not-tested. Dialect: jim.

No C append API observation is attached for this provider.

### bigip

Status: `not-tested`. Version: not-tested. Build: No execution receipt for this question. Channel: not-tested. Dialect: bigip.

No C append API observation is attached for this provider.

## Exact evidence

- `empty-append-tcl8.4-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/receipt.json). SHA-256 `93ce63312985fe010e26dfd6ccdcb7d03521740bc637d4629fe266b6bda19a7e`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.4-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/stdout.tsv). SHA-256 `d94c097977f4af6872cd3b79299d69f9e4dc77e490e8283d9f14643b51ca369d`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.5-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/receipt.json). SHA-256 `311501369d19b02389913aaca2d4dee226d0e6f987dd42156195dfea62161430`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.5-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/stdout.tsv). SHA-256 `3a9877dd8d3464dc67e1941da2f77d092c19262e4fb15fcfb9a5685a3bde78c1`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.6-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/receipt.json). SHA-256 `27c528da221c8955ebcbde90546b12352d5b6b59debe502cf096df83b9851189`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.6-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/stdout.tsv). SHA-256 `e0ad9992ae9a94efcad016409276098c8b95bb475609ecea0101008efa0da354`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.0-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/receipt.json). SHA-256 `67765b48593b3de51ebef2117f9ea82144c9398b4fc17ec3f74db0488045b4ea`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.0-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/stdout.tsv). SHA-256 `31bb23b3ac92176f57893359743e041f6f94291c0b6228fc59e598155873ead9`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.1-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/receipt.json). SHA-256 `17b2c87f4d26f7d4db918d2f35556fc2d1fee5cb3c629c206d3500dae5ebe3f1`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.1-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/stdout.tsv). SHA-256 `915317b3b7156ae09a3d04accc819e8295c9f085d7e430663f0c35f6a8754374`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact provider receipt/stream, independent primary/resident windows before counted getter.
- `empty-append-probe-c` (input): [rust/tcl-registry/tests/data/native_empty_counted_append/probe.c](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/probe.c). SHA-256 `fb6e8bd4944088d6911f0c53a3104712197c3f1ca9b62001f1784397fe125ad4`. Exact native input/capture protocol or aggregate provider receipts; queue criteria remain provider scaffolding.
- `empty-append-queue-json` (input): [rust/tcl-registry/tests/data/native_empty_counted_append/queue.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/queue.json). SHA-256 `f1dbedec4fd846a0c0a89791ae2b96f2bbb0f418aef037f30bc24f5267563879`. Exact native input/capture protocol or aggregate provider receipts; queue criteria remain provider scaffolding.
- `empty-append-capture-py` (input): [rust/tcl-registry/tests/data/native_empty_counted_append/capture.py](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/capture.py). SHA-256 `ca3c6fade401b4f2942fcf4fb1b270b19682bd715f1099ba3488a32e85721106`. Exact native input/capture protocol or aggregate provider receipts; queue criteria remain provider scaffolding.
- `empty-append-receipt-json` (provider): [rust/tcl-registry/tests/data/native_empty_counted_append/receipt.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/receipt.json). SHA-256 `484dbca512b7ac65158d0bdcecad22a5979c9f6b81032c0feba5153a919d3075`. Exact native input/capture protocol or aggregate provider receipts; queue criteria remain provider scaffolding.
- `empty-append-source-0` (source-anchor): [rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json). SHA-256 `90f76694d0b6173472d0cfa68d5078aeb8108f587b9adc7052dd76a745b79808`. JSON pointer `/source_anchors/0/snippet`. tcl8.4 reached conversion and zero-length return ordering in exact C source.
- `empty-append-source-1` (source-anchor): [rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json). SHA-256 `90f76694d0b6173472d0cfa68d5078aeb8108f587b9adc7052dd76a745b79808`. JSON pointer `/source_anchors/1/snippet`. tcl8.5 reached conversion and zero-length return ordering in exact C source.
- `empty-append-source-2` (source-anchor): [rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json). SHA-256 `90f76694d0b6173472d0cfa68d5078aeb8108f587b9adc7052dd76a745b79808`. JSON pointer `/source_anchors/2/snippet`. tcl8.6 reached conversion and zero-length return ordering in exact C source.
- `empty-append-source-3` (source-anchor): [rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json). SHA-256 `90f76694d0b6173472d0cfa68d5078aeb8108f587b9adc7052dd76a745b79808`. JSON pointer `/source_anchors/3/snippet`. tcl9.0 reached conversion and zero-length return ordering in exact C source.
- `empty-append-source-4` (source-anchor): [rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_empty_counted_append/source-anchors.json). SHA-256 `90f76694d0b6173472d0cfa68d5078aeb8108f587b9adc7052dd76a745b79808`. JSON pointer `/source_anchors/4/snippet`. tcl9.1 reached conversion and zero-length return ordering in exact C source.

## Source inspection

tcl8.4 8.4.20, revision `Retained release source identified by complete file digest`, `tmp/tcl8.4.20/generic/tclStringObj.c`, function `Tcl_AppendToObj`, lines 1086–1123. Full-source SHA-256 `a86bf9f649ce6de16363984f1d8af398c464984e7d5677543f279ed87e35e62c`; snippet SHA-256 `060d4535189ef28a05c01ded0e00df45efe12d9514eebd9abbb01fd386456388`; retained evidence `empty-append-source-0`.

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

tcl8.5 8.5.19, revision `Retained release source identified by complete file digest`, `tmp/tcl8.5.19/generic/tclStringObj.c`, function `Tcl_AppendLimitedToObj`, lines 1107–1169. Full-source SHA-256 `d014bc85fc8f0d12abbd60116f403f9131dd4e933580833d2b02610e211ab449`; snippet SHA-256 `2a05dcf8cfce9a91340c2a7f37c4863ff9931ce1e4a15227da67a798a2545a18`; retained evidence `empty-append-source-1`.

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

tcl8.6 8.6.18, revision `Retained release source identified by complete file digest`, `tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_AppendLimitedToObj`, lines 1193–1263. Full-source SHA-256 `d5cae88e9008d6b9a6101f4c00d7e489fc66fb3d51c86b3b0eb13f7fd8e469b9`; snippet SHA-256 `8f08b2def6d344b2ccc74647d718f97eff9ef182f3aed9d19b6741c7b7f58aba`; retained evidence `empty-append-source-2`.

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

tcl9.0 9.0.4, revision `Retained release source identified by complete file digest`, `tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_AppendLimitedToObj`, lines 1203–1279. Full-source SHA-256 `d44c80d7de637da41ac841c6c55c25c4ee302efc774c2c4cfe5d8ff73193b7b4`; snippet SHA-256 `f471adc4d10d3ef0668796f7a4ef0bd47f0b14edbb40bb2f7ed97661daa2ba3f`; retained evidence `empty-append-source-3`.

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

tcl9.1 9.1.0, revision `Retained release source identified by complete file digest`, `tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_AppendLimitedToObj`, lines 1206–1282. Full-source SHA-256 `d1ecd65e375cee3b1ab86111cf5b90fb6bf4d7b5e71448cd361e4026d32a4bf8`; snippet SHA-256 `f471adc4d10d3ef0668796f7a4ef0bd47f0b14edbb40bb2f7ed97661daa2ba3f`; retained evidence `empty-append-source-4`.

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
