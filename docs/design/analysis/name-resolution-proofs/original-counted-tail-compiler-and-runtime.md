# naming.namespace.original-counted-tail-compiler-and-runtime

Kind: `native-observation`

## Problem statement

A namespace string compiler can operate on counted Unicode where the direct worker uses CString input. Reusing display strings or direct-worker byte slicing loses raw00 and opaque-unit distinctions in the actual compiled program.

## Question

For the ten retained counted inputs, does a genuine namespace Tail procedure compiler preserve its measured last-find/range program and produce a result distinct from direct worker CString behavior?

## Conclusion

C Tcl 8.6/9.0/9.1 compiles the exact one-operand Tail stack program. Its original counted Unicode search sees text beyond raw00 and reaches a fresh Unicode range result. The direct worker stops at raw00 and preserves raw FF bytes. The two surrogate-byte inputs stay distinct. C8.4/8.5 no-hook controls and Jim flat byte-oriented controls remain separate.

## Scope

Exactly ten native counted String inputs, two native object-vector call forms, six retained providers and three C8.6+ opcode windows. The fixed ASCII procedure body and counted runtime argv are distinct channels. Result/input primary descriptors are observed before counted byte getters; no general Unicode decoder theorem, source-channel re-encoding, object-name lookup, command table/slot existence, alias/compiler attachment, frame entry, method/property validity, Normal completion or native capability follows. The Registry original-source projection supplies instruction geometry only; concrete backends independently authenticate compiler admission and original object getters/result constructors. Qualifiers and non-Unicode/ByteArray range branches are outside this Tail implementation. Rust replay selectors are linked obligations without an attached passing execution receipt. Direct result construction is retained separately: C84 reaches counted append only for a nonempty original CString, including a zero-length selected suffix, on the fresh interpreter result, C85+ creates a fresh byte object, and Jim unpaired Tail retains the original object. This result path supplies no command or interpreter authority.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Executable SHA-256 a2c92d91027ad401a9493172ae166ba845e837ec17ce78bd8757a65f2934ea17; header SHA-256 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; linked library SHA-256 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47. Channel: Counted native NewString object-vector argv; separate fixed ASCII proc source; exact stdout hex of counted result/input bytes. Dialect: Tcl.

Direct and procedure calls have the same measured CString results: raw00 a00::tail returns a; raw FF stays FF; D800/D801 stay distinct. No C8.6 Tail opcode window is claimed. Reached Tcl_AppendToObj results are String-primary, including empty suffixes from a::; an empty original CString does not reach append and leaves the NULL interpreter result.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Executable SHA-256 8b0e8ca8cc5c62462fdc5204968f062e17bd2c9e4e1979551e423adc120027cd; header SHA-256 c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; linked library SHA-256 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc. Channel: Counted native NewString object-vector argv; separate fixed ASCII proc source; exact stdout hex of counted result/input bytes. Dialect: Tcl.

Direct and procedure calls have the same measured CString results: raw00 a00::tail returns a; raw FF stays FF; D800/D801 stay distinct. No C8.6 Tail opcode window is claimed. Fresh direct byte results remain NULL-primary through Tcl_NewStringObj; the counted original input remains NULL until an independently reached getter.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Executable SHA-256 1f0ea49ba28cae40769a3b851b95871fd4f156265a57c3d761bd015c490c19b3; header SHA-256 aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; linked library SHA-256 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb. Channel: Counted native NewString object-vector argv; separate fixed ASCII proc source; exact stdout hex of counted result/input bytes. Dialect: Tcl.

Direct raw00 a00::tail returns a and raw FF stays FF; the compiled original procedure returns tail for raw00 and converts raw FF to C3BF through counted Unicode. D800/D801 remain distinct. Compiled subject primary becomes String; nonempty result primary is String, empty result is NULL. The captured disassembly retains the exact last-find, guarded +2 and range program.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executable SHA-256 0bd86e91cc628c352a509b4aaff2887eaaefa62c5c3c13ea61945328f9e32ea8; header SHA-256 eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; linked library SHA-256 dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4. Channel: Counted native NewString object-vector argv; separate fixed ASCII proc source; exact stdout hex of counted result/input bytes. Dialect: Tcl.

Direct raw00 a00::tail returns a and raw FF stays FF; the compiled original procedure returns tail for raw00 and converts raw FF to C3BF through counted Unicode. D800/D801 remain distinct. Compiled subject primary becomes String; nonempty result primary is String, empty result is NULL. The captured disassembly retains the exact last-find, guarded +2 and range program.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executable SHA-256 6e53f83a6e671dc943b306bc6682bfd103b787f13b154bf9d23449049a1accba; header SHA-256 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; linked library SHA-256 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db. Channel: Counted native NewString object-vector argv; separate fixed ASCII proc source; exact stdout hex of counted result/input bytes. Dialect: Tcl.

Direct raw00 a00::tail returns a and raw FF stays FF; the compiled original procedure returns tail for raw00 and converts raw FF to C3BF through counted Unicode. D800/D801 remain distinct. Compiled subject primary becomes String; nonempty result primary is String, empty result is NULL. The captured disassembly retains the exact last-find, guarded +2 and range program.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Executable SHA-256 f4ce1ec812a7a4f8b2e7b39482addb2599555bfb6b2647557f6f91cfa62cd149; header SHA-256 d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d; linked library SHA-256 a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da. Channel: Counted native NewString object-vector argv; separate fixed ASCII proc source; exact stdout hex of counted result/input bytes. Dialect: Jim Tcl.

Direct and procedure calls retain raw FF and the distinct D800/D801 bytes. The raw00 input a00::tail is returned whole with the same original identity in both forms. No-separator, empty and single-colon inputs also retain the original identity. The D row records no C Tail compiler.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `counted-input` (input): [rust/tcl-registry/tests/data/native_namespace_string_compilation/probe.c](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/probe.c). Exact counted Tcl/Jim NewString input and native object-vector calls; fixed ASCII procedure source and no provider-dependent input branches.
- `actual-captures` (provider): [rust/tcl-registry/tests/data/native_namespace_string_compilation/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/receipt.json). All six exact header/library/executable/compiler/stream digests, exit codes and actual native version rows.
- `capture-protocol` (input): [rust/tcl-registry/tests/data/native_namespace_string_compilation/capture.py](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/capture.py). Exact native compilation/capture runner; replay requires its retained source/build paths.
- `source-anchors` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors`. Retained pinned source compiler and executor windows, full source file and exact snippet digests.
- `tcl8.4-output` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.4.20/stdout.tsv). Exact result code/primary/identity/count/hex, original primary/count/hex and actual version/disassembly rows.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_string_compilation/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.4.20/receipt.json). Exact provider compile/process and linked header/library/executable/stream hash joins.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.4.20/stderr). Actual empty stderr.
- `tcl8.5-output` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.5.19/stdout.tsv). Exact result code/primary/identity/count/hex, original primary/count/hex and actual version/disassembly rows.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_string_compilation/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.5.19/receipt.json). Exact provider compile/process and linked header/library/executable/stream hash joins.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.5.19/stderr). Actual empty stderr.
- `tcl8.6-output` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.6.18/stdout.tsv). Exact result code/primary/identity/count/hex, original primary/count/hex and actual version/disassembly rows.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_string_compilation/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.6.18/receipt.json). Exact provider compile/process and linked header/library/executable/stream hash joins.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.6.18/stderr). Actual empty stderr.
- `tcl9.0-output` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/9.0.4/stdout.tsv). Exact result code/primary/identity/count/hex, original primary/count/hex and actual version/disassembly rows.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_string_compilation/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/9.0.4/receipt.json). Exact provider compile/process and linked header/library/executable/stream hash joins.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/9.0.4/stderr). Actual empty stderr.
- `tcl9.1-output` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/9.1.0/stdout.tsv). Exact result code/primary/identity/count/hex, original primary/count/hex and actual version/disassembly rows.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_string_compilation/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/9.1.0/receipt.json). Exact provider compile/process and linked header/library/executable/stream hash joins.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/9.1.0/stderr). Actual empty stderr.
- `jim-output` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/jim/stdout.tsv). Exact result code/primary/identity/count/hex, original primary/count/hex and actual version/disassembly rows.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_string_compilation/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/jim/receipt.json). Exact provider compile/process and linked header/library/executable/stream hash joins.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_string_compilation/jim/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/jim/stderr). Actual empty stderr.
- `tail-source-window-0` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/0/snippet`. Original one-operand compiler stack and last-find/range instruction sequence; exact retained LF source window with independently pinned full-source and snippet digests.
- `tail-source-window-1` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/1/snippet`. Original haystack-before-needle Unicode getters and counted last-match coordinate; exact retained LF source window with independently pinned full-source and snippet digests.
- `tail-source-window-2` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/2/snippet`. Original character-length/index extraction and Unicode-backed range construction; exact retained LF source window with independently pinned full-source and snippet digests.
- `tail-source-window-3` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/3/snippet`. Native Unicode branch result construction; C bytearray and non-Unicode branches remain separate; exact retained LF source window with independently pinned full-source and snippet digests.
- `tail-source-window-4` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/4/snippet`. Original one-operand compiler stack and last-find/range instruction sequence; exact retained LF source window with independently pinned full-source and snippet digests.
- `tail-source-window-5` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/5/snippet`. Original haystack-before-needle Unicode getters and counted last-match coordinate; exact retained LF source window with independently pinned full-source and snippet digests.
- `tail-source-window-6` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/6/snippet`. Original character-length/index extraction and Unicode-backed range construction; exact retained LF source window with independently pinned full-source and snippet digests.
- `tail-source-window-7` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/7/snippet`. Native Unicode branch result construction; C bytearray and non-Unicode branches remain separate; exact retained LF source window with independently pinned full-source and snippet digests.
- `tail-source-window-8` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/8/snippet`. Original one-operand compiler stack and last-find/range instruction sequence; exact retained LF source window with independently pinned full-source and snippet digests.
- `tail-source-window-9` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/9/snippet`. Original haystack-before-needle Unicode getters and counted last-match coordinate; exact retained LF source window with independently pinned full-source and snippet digests.
- `tail-source-window-10` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/10/snippet`. Original character-length/index extraction and Unicode-backed range construction; exact retained LF source window with independently pinned full-source and snippet digests.
- `tail-source-window-11` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors-tail123.json). JSON pointer `/source_anchors/11/snippet`. Native Unicode branch result construction; C bytearray and non-Unicode branches remain separate; exact retained LF source window with independently pinned full-source and snippet digests.
- `direct-result-source-12` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors.json). JSON pointer `/source_anchors/12/snippet`. tcl8.4 direct Tail CString scan and actual append/new-string producer.
- `direct-result-source-13` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors.json). JSON pointer `/source_anchors/13/snippet`. tcl8.5 direct Tail CString scan and actual append/new-string producer.
- `direct-result-source-14` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors.json). JSON pointer `/source_anchors/14/snippet`. tcl8.6 direct Tail CString scan and actual append/new-string producer.
- `direct-result-source-15` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors.json). JSON pointer `/source_anchors/15/snippet`. tcl9.0 direct Tail CString scan and actual append/new-string producer.
- `direct-result-source-16` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/source-anchors.json). JSON pointer `/source_anchors/16/snippet`. tcl9.1 direct Tail CString scan and actual append/new-string producer.

## Source inspection

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; complete file digest identifies exact bytes`, `tmp/tcl8.6.18/generic/tclCompCmdsGR.c`, function `TclCompileNamespaceTailCmd`, lines 1918–1952. Retained reference `tail-source-window-0`.

```text
TclCompileNamespaceTailCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);
    JumpFixup jumpFixup;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    /*
     * Take care; only add 2 to found index if the string was actually found.
     */

    CompileWord(envPtr, tokenPtr, interp, 1);
    PushStringLiteral(envPtr, "::");
    TclEmitInstInt4(	INST_OVER, 1,			envPtr);
    TclEmitOpcode(	INST_STR_FIND_LAST,		envPtr);
    TclEmitOpcode(	INST_DUP,			envPtr);
    PushStringLiteral(envPtr, "0");
    TclEmitOpcode(	INST_GE,			envPtr);
    TclEmitForwardJump(envPtr, TCL_FALSE_JUMP, &jumpFixup);
    PushStringLiteral(envPtr, "2");
    TclEmitOpcode(	INST_ADD,			envPtr);
    TclFixupForwardJumpToHere(envPtr, &jumpFixup, 127);
    PushStringLiteral(envPtr, "end");
    TclEmitOpcode(	INST_STR_RANGE,			envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; complete file digest identifies exact bytes`, `tmp/tcl8.6.18/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_FIND_LAST case)`, lines 5902–5922. Retained reference `tail-source-window-1`.

```text
    case INST_STR_FIND_LAST:
	ustring1 = Tcl_GetUnicodeFromObj(OBJ_AT_TOS, &length);	/* Haystack */
	ustring2 = Tcl_GetUnicodeFromObj(OBJ_UNDER_TOS, &length2);/* Needle */

	match = -1;
	if (length2 > 0 && length2 <= length) {
	    for (p=ustring1+length-length2 ; p>=ustring1 ; p--) {
		if ((*p == *ustring2) &&
			memcmp(ustring2,p,sizeof(Tcl_UniChar)*length2) == 0) {
		    match = p - ustring1;
		    break;
		}
	    }
	}

	TRACE(("%.20s %.20s => %d\n",
		O2S(OBJ_UNDER_TOS), O2S(OBJ_AT_TOS), match));

	TclNewIntObj(objResultPtr, match);
	NEXT_INST_F(1, 2, 1);


```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; complete file digest identifies exact bytes`, `tmp/tcl8.6.18/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_RANGE case)`, lines 5595–5620. Retained reference `tail-source-window-2`.

```text
    case INST_STR_RANGE:
	TRACE(("\"%.20s\" %.20s %.20s =>",
		O2S(OBJ_AT_DEPTH(2)), O2S(OBJ_UNDER_TOS), O2S(OBJ_AT_TOS)));
	length = Tcl_GetCharLength(OBJ_AT_DEPTH(2)) - 1;
	if (TclGetIntForIndexM(interp, OBJ_UNDER_TOS, length,
		    &fromIdx) != TCL_OK
	    || TclGetIntForIndexM(interp, OBJ_AT_TOS, length,
		    &toIdx) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}

	if (fromIdx < 0) {
	    fromIdx = 0;
	}
	if (toIdx >= length) {
	    toIdx = length;
	}
	if (toIdx >= fromIdx) {
	    objResultPtr = Tcl_GetRange(OBJ_AT_DEPTH(2), fromIdx, toIdx);
	} else {
	    TclNewObj(objResultPtr);
	}
	TRACE_APPEND(("\"%.30s\"\n", O2S(objResultPtr)));
	NEXT_INST_V(1, 3, 1);


```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; complete file digest identifies exact bytes`, `tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 741–829. Retained reference `tail-source-window-3`.

```text
Tcl_GetRange(
    Tcl_Obj *objPtr,		/* The Tcl object to find the range of. */
    int first,			/* First index of the range. */
    int last)			/* Last index of the range. */
{
    Tcl_Obj *newObjPtr;		/* The Tcl object to find the range of. */
    String *stringPtr;
    int length;

    if (first < 0) {
	first = 0;
    }

    /*
     * Optimize the case where we're really dealing with a bytearray object
     * we don't need to convert to a string to perform the substring operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	unsigned char *bytes = Tcl_GetByteArrayFromObj(objPtr, &length);

	if (last < 0 || last >= length) {
	    last = length - 1;
	}
	if (last < first) {
	    TclNewObj(newObjPtr);
	    return newObjPtr;
	}
	return Tcl_NewByteArrayObj(bytes + first, last - first + 1);
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->hasUnicode == 0) {
	/*
	 * If numChars is unknown, compute it.
	 */

	if (stringPtr->numChars == -1) {
	    TclNumUtfChars(stringPtr->numChars, objPtr->bytes, objPtr->length);
	}
	if (stringPtr->numChars == objPtr->length) {
	    if (last < 0 || last >= stringPtr->numChars) {
		last = stringPtr->numChars - 1;
	    }
	    if (last < first) {
		TclNewObj(newObjPtr);
		return newObjPtr;
	    }
	    newObjPtr = Tcl_NewStringObj(objPtr->bytes + first, last - first + 1);

	    /*
	     * Since we know the char length of the result, store it.
	     */

	    SetStringFromAny(NULL, newObjPtr);
	    stringPtr = GET_STRING(newObjPtr);
	    stringPtr->numChars = newObjPtr->length;
	    return newObjPtr;
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }
    if (last < 0 || last >= stringPtr->numChars) {
	last = stringPtr->numChars - 1;
    }
    if (last < first) {
	TclNewObj(newObjPtr);
	return newObjPtr;
    }
#if TCL_UTF_MAX == 4
    /* See: bug [11ae2be95dac9417] */
    if ((first > 0) && ((stringPtr->unicode[first] & 0xFC00) == 0xDC00)
	    && ((stringPtr->unicode[first-1] & 0xFC00) == 0xD800)) {
	++first;
    }
    if ((last + 1 < stringPtr->numChars)
	    && ((stringPtr->unicode[last+1] & 0xFC00) == 0xDC00)
	    && ((stringPtr->unicode[last] & 0xFC00) == 0xD800)) {
	++last;
    }
#endif
    return Tcl_NewUnicodeObj(stringPtr->unicode + first, last - first + 1);
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclCompCmdsGR.c`, function `TclCompileNamespaceTailCmd`, lines 1759–1792. Retained reference `tail-source-window-4`.

```text
TclCompileNamespaceTailCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);
    JumpFixup jumpFixup;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    /*
     * Take care; only add 2 to found index if the string was actually found.
     */

    CompileWord(envPtr, tokenPtr, interp, 1);
    PushStringLiteral(envPtr, "::");
    TclEmitInstInt4(	INST_OVER, 1,			envPtr);
    TclEmitOpcode(	INST_STR_FIND_LAST,		envPtr);
    TclEmitOpcode(	INST_DUP,			envPtr);
    PushStringLiteral(envPtr, "0");
    TclEmitOpcode(	INST_GE,			envPtr);
    TclEmitForwardJump(envPtr, TCL_FALSE_JUMP, &jumpFixup);
    PushStringLiteral(envPtr, "2");
    TclEmitOpcode(	INST_ADD,			envPtr);
    TclFixupForwardJumpToHere(envPtr, &jumpFixup, 127);
    PushStringLiteral(envPtr, "end");
    TclEmitOpcode(	INST_STR_RANGE,			envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_FIND_LAST case)`, lines 5562–5568. Retained reference `tail-source-window-5`.

```text
    case INST_STR_FIND_LAST:
	objResultPtr = TclStringLast(OBJ_UNDER_TOS, OBJ_AT_TOS, TCL_SIZE_MAX - 1);

	TRACE(("%.20s %.20s => %s\n",
		O2S(OBJ_UNDER_TOS), O2S(OBJ_AT_TOS), O2S(objResultPtr)));
	NEXT_INST_F(1, 2, 1);


```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_RANGE case)`, lines 5382–5407. Retained reference `tail-source-window-6`.

```text
    case INST_STR_RANGE:
	TRACE(("\"%.20s\" %.20s %.20s =>",
		O2S(OBJ_AT_DEPTH(2)), O2S(OBJ_UNDER_TOS), O2S(OBJ_AT_TOS)));
	slength = Tcl_GetCharLength(OBJ_AT_DEPTH(2)) - 1;

	DECACHE_STACK_INFO();
	if (TclGetIntForIndexM(interp, OBJ_UNDER_TOS, slength, &fromIdx) != TCL_OK) {
	    CACHE_STACK_INFO();
	    TRACE_ERROR(interp);
	    goto gotError;
	}
	if (TclGetIntForIndexM(interp, OBJ_AT_TOS, slength, &toIdx) != TCL_OK) {
	    CACHE_STACK_INFO();
	    TRACE_ERROR(interp);
	    goto gotError;
	}
	CACHE_STACK_INFO();

	if (toIdx == TCL_INDEX_NONE) {
	    TclNewObj(objResultPtr);
	} else {
	    objResultPtr = Tcl_GetRange(OBJ_AT_DEPTH(2), fromIdx, toIdx);
	}
	TRACE_APPEND(("\"%.30s\"\n", O2S(objResultPtr)));
	NEXT_INST_V(1, 3, 1);


```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 725–802. Retained reference `tail-source-window-7`.

```text
Tcl_GetRange(
    Tcl_Obj *objPtr,		/* The Tcl object to find the range of. */
    Tcl_Size first,		/* First index of the range. */
    Tcl_Size last)		/* Last index of the range. */
{
    Tcl_Obj *newObjPtr;		/* The Tcl object to return that is the new
				 * range. */
    String *stringPtr;
    Tcl_Size length = 0;

    if (first < 0) {
	first = 0;
    }

    /*
     * Optimize the case where we're really dealing with a byte-array object
     * we don't need to convert to a string to perform the substring operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &length);

	if (last < 0 || last >= length) {
	    last = length - 1;
	}
	if (last < first) {
	    TclNewObj(newObjPtr);
	    return newObjPtr;
	}
	return Tcl_NewByteArrayObj(bytes + first, last - first + 1);
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->hasUnicode == 0) {
	/*
	 * If numChars is unknown, compute it.
	 */

	if (stringPtr->numChars == TCL_INDEX_NONE) {
	    TclNumUtfCharsM(stringPtr->numChars, objPtr->bytes, objPtr->length);
	}
	if (stringPtr->numChars == objPtr->length) {
	    if (last < 0 || last >= stringPtr->numChars) {
		last = stringPtr->numChars - 1;
	    }
	    if (last < first) {
		TclNewObj(newObjPtr);
		return newObjPtr;
	    }
	    newObjPtr = Tcl_NewStringObj(objPtr->bytes + first, last - first + 1);

	    /*
	     * Since we know the char length of the result, store it.
	     */

	    SetStringFromAny(NULL, newObjPtr);
	    stringPtr = GET_STRING(newObjPtr);
	    stringPtr->numChars = newObjPtr->length;
	    return newObjPtr;
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }
    if (last < 0 || last >= stringPtr->numChars) {
	last = stringPtr->numChars - 1;
    }
    if (last < first) {
	TclNewObj(newObjPtr);
	return newObjPtr;
    }
    return Tcl_NewUnicodeObj(stringPtr->unicode + first, last - first + 1);
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclCompCmdsGR.c`, function `TclCompileNamespaceTailCmd`, lines 2232–2265. Retained reference `tail-source-window-8`.

```text
TclCompileNamespaceTailCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);
    Tcl_BytecodeLabel dontSkipSeparator;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    /*
     * Take care; only add 2 to found index if the string was actually found.
     */

    PUSH_TOKEN(			tokenPtr, 1);
    PUSH(			"::");
    OP4(			OVER, 1);
    OP(				STR_FIND_LAST);
    OP(				DUP);
    PUSH(			"0");
    OP(				GE);
    FWDJUMP(			JUMP_FALSE, dontSkipSeparator);
    PUSH(			"2");
    OP(				ADD);
    FWDLABEL(		dontSkipSeparator);
    PUSH(			"end");
    OP(				STR_RANGE);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_FIND_LAST case)`, lines 5896–5901. Retained reference `tail-source-window-9`.

```text
    case INST_STR_FIND_LAST:
	TRACE("%.20s %.20s => ", O2S(OBJ_UNDER_TOS), O2S(OBJ_AT_TOS));
	objResultPtr = TclStringLast(OBJ_UNDER_TOS, OBJ_AT_TOS, TCL_SIZE_MAX - 1);
	TRACE_APPEND_NUM_OBJ(objResultPtr);
	NEXT_INST_F(1, 2, 1);


```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_RANGE case)`, lines 5720–5741. Retained reference `tail-source-window-10`.

```text
    case INST_STR_RANGE:
	TRACE("\"%.20s\" %.20s %.20s =>",
		O2S(OBJ_AT_DEPTH(2)), O2S(OBJ_UNDER_TOS), O2S(OBJ_AT_TOS));
	slength = Tcl_GetCharLength(OBJ_AT_DEPTH(2)) - 1;

	DECACHE_STACK_INFO();
	if (TclGetIntForIndexM(interp, OBJ_UNDER_TOS, slength, &fromIdx) != TCL_OK ||
		TclGetIntForIndexM(interp, OBJ_AT_TOS, slength, &toIdx) != TCL_OK) {
	    CACHE_STACK_INFO();
	    TRACE_ERROR(interp);
	    goto gotError;
	}
	CACHE_STACK_INFO();

	if (toIdx == TCL_INDEX_NONE) {
	    TclNewObj(objResultPtr);
	} else {
	    objResultPtr = Tcl_GetRange(OBJ_AT_DEPTH(2), fromIdx, toIdx);
	}
	TRACE_APPEND_OBJ(objResultPtr);
	NEXT_INST_V(1, 3, 1);


```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 728–805. Retained reference `tail-source-window-11`.

```text
Tcl_GetRange(
    Tcl_Obj *objPtr,		/* The Tcl object to find the range of. */
    Tcl_Size first,		/* First index of the range. */
    Tcl_Size last)		/* Last index of the range. */
{
    Tcl_Obj *newObjPtr;		/* The Tcl object to return that is the new
				 * range. */
    String *stringPtr;
    Tcl_Size length = 0;

    if (first < 0) {
	first = 0;
    }

    /*
     * Optimize the case where we're really dealing with a byte-array object
     * we don't need to convert to a string to perform the substring operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &length);

	if (last < 0 || last >= length) {
	    last = length - 1;
	}
	if (last < first) {
	    TclNewObj(newObjPtr);
	    return newObjPtr;
	}
	return Tcl_NewByteArrayObj(bytes + first, last - first + 1);
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->hasUnicode == 0) {
	/*
	 * If numChars is unknown, compute it.
	 */

	if (stringPtr->numChars == TCL_INDEX_NONE) {
	    TclNumUtfCharsM(stringPtr->numChars, objPtr->bytes, objPtr->length);
	}
	if (stringPtr->numChars == objPtr->length) {
	    if (last < 0 || last >= stringPtr->numChars) {
		last = stringPtr->numChars - 1;
	    }
	    if (last < first) {
		TclNewObj(newObjPtr);
		return newObjPtr;
	    }
	    newObjPtr = Tcl_NewStringObj(objPtr->bytes + first, last - first + 1);

	    /*
	     * Since we know the char length of the result, store it.
	     */

	    SetStringFromAny(NULL, newObjPtr);
	    stringPtr = GET_STRING(newObjPtr);
	    stringPtr->numChars = newObjPtr->length;
	    return newObjPtr;
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }
    if (last < 0 || last >= stringPtr->numChars) {
	last = stringPtr->numChars - 1;
    }
    if (last < first) {
	TclNewObj(newObjPtr);
	return newObjPtr;
    }
    return Tcl_NewUnicodeObj(stringPtr->unicode + first, last - first + 1);
}

```

tcl8.4 8.4.20, revision `Retained Tcl 8.4.20 release source; complete file digest identifies exact bytes`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `NamespaceTailCmd`, lines 3656–3689. Retained reference `direct-result-source-12`.

```text
NamespaceTailCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    register char *name, *p;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 2, objv, "string");
        return TCL_ERROR;
    }

    /*
     * Find the end of the string, then work backward and find the
     * last "::" qualifier.
     */

    name = Tcl_GetString(objv[2]);
    for (p = name;  *p != '\0';  p++) {
	/* empty body */
    }
    while (--p > name) {
        if ((*p == ':') && (*(p-1) == ':')) {
            p++;		/* just after the last "::" */
            break;
        }
    }
    
    if (p >= name) {
        Tcl_AppendToObj(Tcl_GetObjResult(interp), p, -1);
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `Retained Tcl 8.5.19 release source; complete file digest identifies exact bytes`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceTailCmd`, lines 4392–4425. Retained reference `direct-result-source-13`.

```text
NamespaceTailCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    register char *name, *p;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 2, objv, "string");
	return TCL_ERROR;
    }

    /*
     * Find the end of the string, then work backward and find the last "::"
     * qualifier.
     */

    name = TclGetString(objv[2]);
    for (p = name;  *p != '\0';  p++) {
	/* empty body */
    }
    while (--p > name) {
	if ((*p == ':') && (*(p-1) == ':')) {
	    p++;			/* Just after the last "::" */
	    break;
	}
    }

    if (p >= name) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(p, -1));
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; complete file digest identifies exact bytes`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `NamespaceTailCmd`, lines 4420–4453. Retained reference `direct-result-source-14`.

```text
NamespaceTailCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *name, *p;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "string");
	return TCL_ERROR;
    }

    /*
     * Find the end of the string, then work backward and find the last "::"
     * qualifier.
     */

    name = TclGetString(objv[1]);
    for (p = name;  *p != '\0';  p++) {
	/* empty body */
    }
    while (--p > name) {
	if ((p[0] == ':') && (p[-1] == ':')) {
	    p++;			/* Just after the last "::" */
	    break;
	}
    }

    if (p >= name) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(p, -1));
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `NamespaceTailCmd`, lines 4615–4648. Retained reference `direct-result-source-15`.

```text
NamespaceTailCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *name, *p;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "string");
	return TCL_ERROR;
    }

    /*
     * Find the end of the string, then work backward and find the last "::"
     * qualifier.
     */

    name = TclGetString(objv[1]);
    for (p = name;  *p != '\0';  p++) {
	/* empty body */
    }
    while (--p > name) {
	if ((p[0] == ':') && (p[-1] == ':')) {
	    p++;			/* Just after the last "::" */
	    break;
	}
    }

    if (p >= name) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(p, -1));
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `NamespaceTailCmd`, lines 4588–4621. Retained reference `direct-result-source-16`.

```text
NamespaceTailCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    const char *name, *p;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "string");
	return TCL_ERROR;
    }

    /*
     * Find the end of the string, then work backward and find the last "::"
     * qualifier.
     */

    name = TclGetString(objv[1]);
    for (p = name;  *p != '\0';  p++) {
	/* empty body */
    }
    while (--p > name) {
	if ((p[0] == ':') && (p[-1] == ':')) {
	    p++;			/* Just after the last "::" */
	    break;
	}
    }

    if (p >= name) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(p, -1));
    }
    return TCL_OK;
}

```


## Consumer bindings

- [rust/tcl-registry/src/native_namespace_string_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_string_compilation.rs), `compile_native_namespace_string`: Original pure complete compiler-word operand projection; no native admission.
- [rust/tcl-cmd-core/src/string/native_compiled.rs](../../../../rust/tcl-cmd-core/src/string/native_compiled.rs), `compiled_find`: Same original counted Unicode getters, haystack before needle, and exact unit matching.
- [rust/tcl-cmd-core/src/string/native_compiled.rs](../../../../rust/tcl-cmd-core/src/string/native_compiled.rs), `compiled_range`: Reached original Unicode backing and selected native range result, independent physical constructor.
- [rust/tcl-cmd-core/src/string/native_compiled.rs](../../../../rust/tcl-cmd-core/src/string/native_compiled.rs), `compiled_tail`: Portable original selected Tail stack result over actual prepared operand and literal.
- [rust/tcl-syntax/src/value.rs](../../../../rust/tcl-syntax/src/value.rs), `ValueOps::native_unicode_string_result`: Default refusing native Unicode constructor facade; concrete backend owns actual current protocol.
- [rust/tcl-compiler/src/codegen/statements/native_namespace_string.rs](../../../../rust/tcl-compiler/src/codegen/statements/native_namespace_string.rs), `append_native_namespace_string_tasks`: Exact selected original Tail opcode sequence and independently stamped native string instructions.
- [runtime/rust/src/interp/native_body_artifact/native_namespace_string.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_namespace_string.rs), `execute_body_namespace_string`: Original immutable operand preparation and shared selected counted Tail execution.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::namespace_text_result`: Pure selected direct worker result action: C84 counted append, later C fresh byte result and Jim original identity on unpaired Tail; backend invokes independently selected append/physical object owner.
- [rust/tcl-syntax/src/native_object_append.rs](../../../../rust/tcl-syntax/src/native_object_append.rs), `NativeObjectAppendProtocol::converts_empty_counted_receiver`: Pure selected C release zero-length conversion order; no backend authority.
- [rust/tcl-cmd-core/src/native_append.rs](../../../../rust/tcl-cmd-core/src/native_append.rs), `append_counted_bytes`: Existing concrete receiver COW/getter/String conversion path is reached only by releases that actually convert before empty return.
- [rust/tcl-registry/src/native_namespace_string_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_string_compilation.rs), `native_namespace_string_compilation::tests::original_namespace_tail_recipe_retains_dynamic_and_counted_operands` (linked): Original dynamic, raw00 and surrogate operands retain genuine source positions; older C no-hook selection stays Generic.
- [rust/tcl-vm/src/exec/native_namespace_string_tests.rs](../../../../rust/tcl-vm/src/exec/native_namespace_string_tests.rs), `exec::native_namespace_string_tests::original_namespace_tail_matches_one_hundred_counted_native_windows` (linked): Compare exact C5 direct/procedure result and original primary, identity and counted bytes against all 100 retained windows.
- [runtime/rust/src/interp/native_body_artifact/native_namespace_string.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_namespace_string.rs), `interp::native_body_artifact::native_namespace_string::tests::original_namespace_tail_matches_one_hundred_counted_native_windows` (linked): Compare Runtime exact C5 direct/procedure result and original primary, identity and counted bytes against all 100 retained windows.
- [rust/tcl-vm/src/exec/native_namespace_string_tests.rs](../../../../rust/tcl-vm/src/exec/native_namespace_string_tests.rs), `exec::native_namespace_string_tests::original_compiled_tail_declines_foreign_policy_and_unreached_unicode_range` (linked): Foreign version and range without independently reached Unicode backing refuse.
- [rust/tcl-compiler/src/codegen/statements/native_namespace_string_tests.rs](../../../../rust/tcl-compiler/src/codegen/statements/native_namespace_string_tests.rs), `codegen::statements::native_namespace_string::tests::original_namespace_tail_emits_the_observed_counted_compiler_program` (linked): Exact original emitted opcode/version window and independent unknown-worker withdrawal.
- [rust/tcl-compiler/src/codegen/statements/native_namespace_string_tests.rs](../../../../rust/tcl-compiler/src/codegen/statements/native_namespace_string_tests.rs), `codegen::statements::native_namespace_string::tests::original_namespace_tail_unknown_worker_withdraws_the_compiler_program` (linked): Exact original emitted opcode/version window and independent unknown-worker withdrawal.
- [rust/tcl-vm/src/exec/native_namespace_string_tests.rs](../../../../rust/tcl-vm/src/exec/native_namespace_string_tests.rs), `exec::native_namespace_string_tests::original_empty_counted_append_preserves_the_reached_release_conversion` (linked): Independent concrete C5 zero-length append primary order preserves reached C84/85 conversion and later release early return.

A named test is a coverage binding, not a claim that it executed.

## Replay

Replay requires the exact retained native fixture entry, source/config/compiler and original object headers. capture.py requires the retained source/build paths; the original provider receipts and raw streams preserve the actual captures. Named Registry, VM and Runtime Rust selectors are linked coverage only; no passing Rust execution receipt is attached. Counted runtime object-vector inputs and the fixed ASCII compiler program retain separate channels.


## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

[Exact original body hashes and process outcomes](rust-validation/frozen349/question-body-outcomes.json) join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.
