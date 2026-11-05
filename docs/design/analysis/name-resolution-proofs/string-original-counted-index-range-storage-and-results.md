# naming.string.original-counted-index-range-storage-and-results

Kind: `native-observation`

## Problem statement

Which original counted storage, native unit, constructor, lazy cache and alias branches do actual string index/range reach, and how do those differ from replacement and a Rust Unicode projection?

## Question

For the exact 20 counted original object inputs, what command completion, public original/result primary and resident-byte fields, alias identity and result bytes do string index and string range produce on each measured C Tcl 8.4–9.1 and current Jim library? Which original constructor cases are explicitly unavailable in Jim?

## Conclusion

Index and range are distinct native purposes: index may encode a selected storage unit while range preserves original counted bytes. Raw supplementary bytes differ across C8.4/8.5, C8.6 and C9/Jim; explicit surrogate units remain a separate constructor/input axis. Native binary and resident-String branches differ by release. Jim returns the exact subject for two whole ranges and accepts the aliased index expression that all five C builds reject. Counted index acceptance differs by release and syntax. These are 116 observed command outcomes and four explicit Jim constructor skips, not 120 commands.

## Scope

120 fresh case processes (20 per selected library), all process exit 0: 116 actual command outcomes (108 code 0, eight code 1) and four explicit Jim constructor skips. Before-command, reached-before-result-getter, materialised-result and materialised-subject observations remain separate. Probe byte inputs are counted C arrays, not Tcl source decoding. Original reference fields and alias identities are snapshots under this exact EvalObjv schedule. The proof does not expose C private Unicode/ByteArray caches, private cache epochs, callback or teardown chronology, general lifetime ownership, arbitrary platforms/builds, direct-helper sharing, replacement semantics, Rust assertions or BIG-IP. Earlier CLI and six independent same-library version processes are separate evidence. Linked Rust definitions are software coverage obligations. Their source assertions require independent execution receipts; they add no provider observation, feature availability, native object/frame or edit authority.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands. Version: tcl8.4 8.4.20.

Twenty commands: 18 code 0 and two code 1 (cases 14 and 18). Raw four-byte index 1 returns efbfb0 and index 2 efbe9f, while range 1..2 preserves f09f. Raw invalid index returns efbe80 but range preserves 80ff. Counted NUL index emits c080 while whole range preserves 410042. Pure ByteArray index/range results remain ByteArray before their getters; the already materialised shared ByteArray in case 16 also remains ByteArray. Original Unicode-surrogate range has no resident String bytes before the result getter. All observed results are distinct from the original operands. Counted ordinary index with NUL tail fails, but counted end-1 with NUL tail succeeds.

### tcl8.5

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands. Version: tcl8.5 8.5.19.

Twenty commands: 19 code 0 and one code 1 (case 14). Raw four-byte/invalid/NUL index and range bytes match the measured 8.4 cases, but ordinary counted index with NUL tail succeeds. Pure ByteArray cases stay binary before getters. The already materialised shared ByteArray in case 16 reaches String primary and a fresh lazy String result. All results are distinct from original operands. Counted end-1 with NUL tail succeeds. Surrogate range, same-first/last and original Unicode ranges have absent resident result bytes before materialisation.

### tcl8.6

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands. Version: tcl8.6 8.6.18.

Twenty commands: 19 code 0 and one code 1 (case 14). Raw four-byte index 1 and 2 emit eda0bd and edb880; their range emits eda0bdedb880. Raw invalid index emits c280 but range preserves 80ff. Counted NUL index emits c080 while whole range preserves 410042. Pure ByteArray index/range stay binary; the already materialised shared ByteArray in case 16 reaches String primary and a fresh lazy String result. All results are distinct from the original operands. Both counted ordinary and end-1 indices with NUL tails succeed.

### tcl9.0

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands. Version: tcl9.0 9.0.4.

Twenty commands: 18 code 0 and two code 1 (cases 14 and 19). Raw four-byte index 1 emits f09f9880, index 2 emits 5a, and range 1..2 emits f09f98805a. Explicit CESU surrogate input and original Unicode surrogate units remain separately observed eda0bdedb880. Invalid index emits c280 while range preserves 80ff; NUL index emits c080 while range preserves 410042. Pure ByteArray and already materialised shared ByteArray case 16 stay binary before result getters. All results are distinct from original operands. Counted ordinary index succeeds; counted end-1 with NUL tail fails. The failed aliased subject/index case 14 keeps the subject primary none at its reached snapshot.

### tcl9.1

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands. Version: tcl9.1 9.1.0.

Twenty commands: 18 code 0 and two code 1 (cases 14 and 19). These 20 byte outputs, result alias identities and public primary/resident-byte fields match the separately measured 9.0.4 cases. Raw supplementary bytes are one selected index character, whereas explicit CESU units remain separate. Pure and materialised shared ByteArrays remain binary on the observed branches. Counted ordinary index succeeds and counted end-1 with NUL tail fails. No result reuses an original operand header.

### jim

Status: `observed`. Channel: Fresh counted C-created original EvalObjv objects. Each distinct object has one caller reference; case 16 adds one genuine subject reference, and alias argv roles reuse the original header. C uses Tcl_CreateInterp without Tcl_Init; Jim uses CreateInterp/RegisterCoreCommands. Version: jim 0.84-9-g5bac7c9.

Sixteen commands, all code 0, and four explicit constructor-unavailable skips (12, 13, 16, 17). Raw supplementary index/range bytes match C9 for these inputs, but NUL index preserves 00 and raw invalid index preserves 80. Whole ASCII range and whole counted-NUL range return the exact original subject header (cases 1 and 9); partial ranges are distinct. The aliased subject-as-index expression 2*1 succeeds and returns 31, with the same original subject reaching String primary. Both counted ordinary and end-1 index objects with NUL tails succeed. Jim charLength is observed only on actual String primary; no native binary/Unicode constructor result is substituted for the four skips.

### bigip

Status: `not-tested`. Channel: No BIG-IP original object driver capture.

Measured C Tcl and Jim objects do not establish a BIG-IP/TMM storage or execution protocol.

## Source inspection

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_StringObjCmd STR_INDEX case`, lines 1456–1500. Line unit: physical LF only. Original byte range [40373, 41584). Retained reference `source-window-0`.

```text
	case STR_INDEX: {
	    if (objc != 4) {
	        Tcl_WrongNumArgs(interp, 2, objv, "string charIndex");
		return TCL_ERROR;
	    }

	    /*
	     * If we have a ByteArray object, avoid indexing in the
	     * Utf string since the byte array contains one byte per
	     * character.  Otherwise, use the Unicode string rep to
	     * get the index'th char.
	     */

	    if (objv[2]->typePtr == &tclByteArrayType) {
		string1 = (char *) Tcl_GetByteArrayFromObj(objv[2], &length1);

		if (TclGetIntForIndex(interp, objv[3], length1 - 1,
			&index) != TCL_OK) {
		    return TCL_ERROR;
		}
		if ((index >= 0) && (index < length1)) {
		    Tcl_SetByteArrayObj(resultPtr,
			    (unsigned char *)(&string1[index]), 1);
		}
	    } else {
		/*
		 * Get Unicode char length to calulate what 'end' means.
		 */
		length1 = Tcl_GetCharLength(objv[2]);

		if (TclGetIntForIndex(interp, objv[3], length1 - 1,
			&index) != TCL_OK) {
		    return TCL_ERROR;
		}
		if ((index >= 0) && (index < length1)) {
		    char buf[TCL_UTF_MAX];
		    Tcl_UniChar ch;

		    ch      = Tcl_GetUniChar(objv[2], index);
		    length1 = Tcl_UniCharToUtf(ch, buf);
		    Tcl_SetStringObj(resultPtr, buf, length1);
		}
	    }
	    break;
	}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdMZ.c`, function `Tcl_StringObjCmd STR_RANGE case`, lines 2084–2134. Line unit: physical LF only. Original byte range [57197, 58454). Retained reference `source-window-1`.

```text
	case STR_RANGE: {
	    int first, last;

	    if (objc != 5) {
	        Tcl_WrongNumArgs(interp, 2, objv, "string first last");
		return TCL_ERROR;
	    }

	    /*
	     * If we have a ByteArray object, avoid indexing in the
	     * Utf string since the byte array contains one byte per
	     * character.  Otherwise, use the Unicode string rep to
	     * get the range.
	     */

	    if (objv[2]->typePtr == &tclByteArrayType) {
		string1 = (char *)Tcl_GetByteArrayFromObj(objv[2], &length1);
		length1--;
	    } else {
		/*
		 * Get the length in actual characters.
		 */
		string1 = NULL;
		length1 = Tcl_GetCharLength(objv[2]) - 1;
	    }

	    if ((TclGetIntForIndex(interp, objv[3], length1, &first) != TCL_OK)
		    || (TclGetIntForIndex(interp, objv[4], length1,
			    &last) != TCL_OK)) {
		return TCL_ERROR;
	    }

	    if (first < 0) {
		first = 0;
	    }
	    if (last >= length1) {
		last = length1;
	    }
	    if (last >= first) {
		if (string1 != NULL) {
		    int numBytes = last - first + 1;
		    resultPtr = Tcl_NewByteArrayObj(
			(unsigned char *) &string1[first], numBytes);
		    Tcl_SetObjResult(interp, resultPtr);
		} else {
		    Tcl_SetObjResult(interp,
			    Tcl_GetRange(objv[2], first, last));
		}
	    }
	    break;
	}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclStringObj.c`, function `Tcl_GetUniChar`, lines 482–522. Line unit: physical LF only. Original byte range [16457, 17473). Retained reference `source-window-2`.

```text
Tcl_GetUniChar(objPtr, index)
    Tcl_Obj *objPtr;	/* The object to get the Unicode charater from. */
    int index;		/* Get the index'th Unicode character. */
{
    Tcl_UniChar unichar;
    String *stringPtr;
    
    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->numChars == -1) {

	/*
	 * We haven't yet calculated the length, so we don't have the
	 * Unicode str.  We need to know the number of chars before we
	 * can do indexing.
	 */

	Tcl_GetCharLength(objPtr);

	/*
	 * We need to fetch the pointer again because we may have just
	 * reallocated the structure.
	 */
	
	stringPtr = GET_STRING(objPtr);
    }
    if (stringPtr->hasUnicode == 0) {

	/*
	 * All of the characters in the Utf string are 1 byte chars,
	 * so we don't store the unicode char.  We get the Utf string
	 * and convert the index'th byte to a Unicode character.
	 */

	unichar = (Tcl_UniChar) objPtr->bytes[index];
    } else {
	unichar = stringPtr->unicode[index];
    }
    return unichar;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 651–704. Line unit: physical LF only. Original byte range [21100, 22520). Retained reference `source-window-3`.

```text
Tcl_GetRange(objPtr, first, last)
    Tcl_Obj *objPtr;		/* The Tcl object to find the range of. */
    int first;			/* First index of the range. */
    int last;			/* Last index of the range. */
{
    Tcl_Obj *newObjPtr;		/* The Tcl object to find the range of. */
    String *stringPtr;
    
    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->numChars == -1) {
    
	/*
	 * We haven't yet calculated the length, so we don't have the
	 * Unicode str.  We need to know the number of chars before we
	 * can do indexing.
	 */

	Tcl_GetCharLength(objPtr);

	/*
	 * We need to fetch the pointer again because we may have just
	 * reallocated the structure.
	 */
	
	stringPtr = GET_STRING(objPtr);
    }

    if (objPtr->bytes && stringPtr->numChars == objPtr->length) {
	char *str = Tcl_GetString(objPtr);

	/*
	 * All of the characters in the Utf string are 1 byte chars,
	 * so we don't store the unicode char.  Create a new string
	 * object containing the specified range of chars.
	 */
	
	newObjPtr = Tcl_NewStringObj(&str[first], last-first+1);

	/*
	 * Since we know the new string only has 1-byte chars, we
	 * can set it's numChars field.
	 */
	
	SetStringFromAny(NULL, newObjPtr);
	stringPtr = GET_STRING(newObjPtr);
	stringPtr->numChars = last-first+1;
    } else {
	newObjPtr = Tcl_NewUnicodeObj(stringPtr->unicode + first,
		last-first+1);
    }
    return newObjPtr;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclStringObj.c`, function `Tcl_GetCharLength`, lines 402–462. Line unit: physical LF only. Original byte range [14356, 15994). Retained reference `source-window-4`.

```text
Tcl_GetCharLength(objPtr)
    Tcl_Obj *objPtr;	/* The String object to get the num chars of. */
{
    String *stringPtr;
    
    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    /*
     * If numChars is unknown, then calculate the number of characaters
     * while populating the Unicode string.
     */
    
    if (stringPtr->numChars == -1) {
	register int i = objPtr->length;
	register unsigned char *str = (unsigned char *) objPtr->bytes;

	/*
	 * This is a speed sensitive function, so run specially over the
	 * string to count continuous ascii characters before resorting
	 * to the Tcl_NumUtfChars call.  This is a long form of:
	 stringPtr->numChars = Tcl_NumUtfChars(objPtr->bytes, objPtr->length);
	*/

	while (i && (*str < 0xC0)) { i--; str++; }
	stringPtr->numChars = objPtr->length - i;
	if (i) {
	    stringPtr->numChars += Tcl_NumUtfChars(objPtr->bytes
		    + (objPtr->length - i), i);
	}

 	if (stringPtr->numChars == objPtr->length) {

	    /*
	     * Since we've just calculated the number of chars, and all
	     * UTF chars are 1-byte long, we don't need to store the
	     * unicode string.
	     */

	    stringPtr->hasUnicode = 0;

	} else {
    
	    /*
	     * Since we've just calucalated the number of chars, and not
	     * all UTF chars are 1-byte long, go ahead and populate the
	     * unicode string.
	     */

	    FillUnicodeRep(objPtr);

	    /*
	     * We need to fetch the pointer again because we have just
	     * reallocated the structure to make room for the Unicode data.
	     */
	    
	    stringPtr = GET_STRING(objPtr);
	}
    }
    return stringPtr->numChars;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclStringObj.c`, function `Tcl_NewUnicodeObj`, lines 371–382. Line unit: physical LF only. Original byte range [13600, 13926). Retained reference `source-window-5`.

```text
Tcl_NewUnicodeObj(unicode, numChars)
    CONST Tcl_UniChar *unicode;	/* The unicode string used to initialize
				 * the new object. */
    int numChars;		/* Number of characters in the unicode
				 * string. */
{
    Tcl_Obj *objPtr;

    TclNewObj(objPtr);
    SetUnicodeObj(objPtr, unicode, numChars);
    return objPtr;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclUtil.c`, function `TclGetIntForIndex`, lines 2331–2381. Line unit: physical LF only. Original byte range [62656, 64092). Retained reference `source-window-6`.

```text
TclGetIntForIndex(interp, objPtr, endValue, indexPtr)
    Tcl_Interp *interp;		/* Interpreter to use for error reporting. 
				 * If NULL, then no error message is left
				 * after errors. */
    Tcl_Obj *objPtr;		/* Points to an object containing either
				 * "end" or an integer. */
    int endValue;		/* The value to be stored at "indexPtr" if
				 * "objPtr" holds "end". */
    int *indexPtr;		/* Location filled in with an integer
				 * representing an index. */
{
    if (Tcl_GetIntFromObj(NULL, objPtr, indexPtr) == TCL_OK) {
	return TCL_OK;
    }

    if (SetEndOffsetFromAny(NULL, objPtr) == TCL_OK) {
	/*
	 * If the object is already an offset from the end of the
	 * list, or can be converted to one, use it.
	 */

	*indexPtr = endValue + objPtr->internalRep.longValue;

    } else {
	/*
	 * Report a parse error.
	 */

	if (interp != NULL) {
	    char *bytes = Tcl_GetString(objPtr);
	    /*
	     * The result might not be empty; this resets it which
	     * should be both a cheap operation, and of little problem
	     * because this is an error-generation path anyway.
	     */
	    Tcl_ResetResult(interp);
	    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
				   "bad index \"", bytes,
				   "\": must be integer or end?-integer?",
				   (char *) NULL);
	    if (!strncmp(bytes, "end-", 3)) {
		bytes += 3;
	    }
	    TclCheckBadOctal(interp, bytes);
	}

	return TCL_ERROR;
    }
	    
    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInt.h`, function `TclUtfToUniChar macro`, lines 2604–2623. Line unit: physical LF only. Original byte range [106143, 106974). Retained reference `source-window-7`.

```text
#define TclUtfToUniChar(str, chPtr) \
	((((unsigned char) *(str)) < 0xC0) ? \
	    ((*(chPtr) = (Tcl_UniChar) *(str)), 1) \
	    : Tcl_UtfToUniChar(str, chPtr))

/*
 *----------------------------------------------------------------
 * Macro used by the Tcl core to compare Unicode strings.  On
 * big-endian systems we can use the more efficient memcmp, but
 * this would not be lexically correct on little-endian systems.
 * The ANSI C "prototype" for this macro is:
 *
 * EXTERN int TclUniCharNcmp _ANSI_ARGS_((CONST Tcl_UniChar *cs,
 *         CONST Tcl_UniChar *ct, unsigned long n));
 *----------------------------------------------------------------
 */
#ifdef WORDS_BIGENDIAN
#   define TclUniCharNcmp(cs,ct,n) memcmp((cs),(ct),(n)*sizeof(Tcl_UniChar))
#else /* !WORDS_BIGENDIAN */
#   define TclUniCharNcmp Tcl_UniCharNcmp

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdMZ.c`, function `StringIndexCmd`, lines 1331–1381. Line unit: physical LF only. Original byte range [32691, 34104). Retained reference `source-window-8`.

```text
StringIndexCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int length, index;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "string charIndex");
	return TCL_ERROR;
    }

    /*
     * If we have a ByteArray object, avoid indexing in the Utf string since
     * the byte array contains one byte per character. Otherwise, use the
     * Unicode string rep to get the index'th char.
     */

    if (TclIsPureByteArray(objv[1])) {
	const unsigned char *string =
		Tcl_GetByteArrayFromObj(objv[1], &length);

	if (TclGetIntForIndexM(interp, objv[2], length-1, &index) != TCL_OK){
	    return TCL_ERROR;
	}
	string = Tcl_GetByteArrayFromObj(objv[1], &length);
	if ((index >= 0) && (index < length)) {
	    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(string + index, 1));
	}
    } else {
	/*
	 * Get Unicode char length to calulate what 'end' means.
	 */

	length = Tcl_GetCharLength(objv[1]);

	if (TclGetIntForIndexM(interp, objv[2], length-1, &index) != TCL_OK){
	    return TCL_ERROR;
	}
	if ((index >= 0) && (index < length)) {
	    char buf[TCL_UTF_MAX];
	    Tcl_UniChar ch;

	    ch = Tcl_GetUniChar(objv[1], index);
	    length = Tcl_UniCharToUtf(ch, buf);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(buf, length));
	}
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdMZ.c`, function `StringRangeCmd`, lines 2071–2128. Line unit: physical LF only. Original byte range [51436, 52881). Retained reference `source-window-9`.

```text
StringRangeCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const unsigned char *string;
    int length, first, last;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last");
	return TCL_ERROR;
    }

    /*
     * If we have a ByteArray object, avoid indexing in the Utf string since
     * the byte array contains one byte per character. Otherwise, use the
     * Unicode string rep to get the range.
     */

    if (TclIsPureByteArray(objv[1])) {
	string = Tcl_GetByteArrayFromObj(objv[1], &length);
	length--;
    } else {
	/*
	 * Get the length in actual characters.
	 */

	string = NULL;
	length = Tcl_GetCharLength(objv[1]) - 1;
    }

    if (TclGetIntForIndexM(interp, objv[2], length, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], length, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    if (first < 0) {
	first = 0;
    }
    if (last >= length) {
	last = length;
    }
    if (last >= first) {
	if (string != NULL) {
	    /*
	     * Reread the string to prevent shimmering nasties.
	     */

	    string = Tcl_GetByteArrayFromObj(objv[1], &length);
	    Tcl_SetObjResult(interp,
		    Tcl_NewByteArrayObj(string+first, last - first + 1));
	} else {
	    Tcl_SetObjResult(interp, Tcl_GetRange(objv[1], first, last));
	}
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclStringObj.c`, function `Tcl_GetUniChar`, lines 478–516. Line unit: physical LF only. Original byte range [16509, 17507). Retained reference `source-window-10`.

```text
Tcl_GetUniChar(
    Tcl_Obj *objPtr,		/* The object to get the Unicode charater
				 * from. */
    int index)			/* Get the index'th Unicode character. */
{
    Tcl_UniChar unichar;
    String *stringPtr;

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->numChars == -1) {
	/*
	 * We haven't yet calculated the length, so we don't have the Unicode
	 * str. We need to know the number of chars before we can do indexing.
	 */

	Tcl_GetCharLength(objPtr);

	/*
	 * We need to fetch the pointer again because we may have just
	 * reallocated the structure.
	 */

	stringPtr = GET_STRING(objPtr);
    }
    if (stringPtr->hasUnicode == 0) {
	/*
	 * All of the characters in the Utf string are 1 byte chars, so we
	 * don't store the unicode char. We get the Utf string and convert the
	 * index'th byte to a Unicode character.
	 */

	unichar = (Tcl_UniChar) objPtr->bytes[index];
    } else {
	unichar = stringPtr->unicode[index];
    }
    return unichar;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 645–696. Line unit: physical LF only. Original byte range [21100, 22483). Retained reference `source-window-11`.

```text
Tcl_GetRange(
    Tcl_Obj *objPtr,		/* The Tcl object to find the range of. */
    int first,			/* First index of the range. */
    int last)			/* Last index of the range. */
{
    Tcl_Obj *newObjPtr;		/* The Tcl object to find the range of. */
    String *stringPtr;

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    if (stringPtr->numChars == -1) {
	/*
	 * We haven't yet calculated the length, so we don't have the Unicode
	 * str. We need to know the number of chars before we can do indexing.
	 */

	Tcl_GetCharLength(objPtr);

	/*
	 * We need to fetch the pointer again because we may have just
	 * reallocated the structure.
	 */

	stringPtr = GET_STRING(objPtr);
    }

    if (objPtr->bytes && (stringPtr->numChars == objPtr->length)) {
	char *str = TclGetString(objPtr);

	/*
	 * All of the characters in the Utf string are 1 byte chars, so we
	 * don't store the unicode char. Create a new string object containing
	 * the specified range of chars.
	 */

	newObjPtr = Tcl_NewStringObj(&str[first], last-first+1);

	/*
	 * Since we know the new string only has 1-byte chars, we can set it's
	 * numChars field.
	 */

	SetStringFromAny(NULL, newObjPtr);
	stringPtr = GET_STRING(newObjPtr);
	stringPtr->numChars = last-first+1;
    } else {
	newObjPtr = Tcl_NewUnicodeObj(stringPtr->unicode + first,
		last-first+1);
    }
    return newObjPtr;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclStringObj.c`, function `Tcl_GetCharLength`, lines 395–458. Line unit: physical LF only. Original byte range [14378, 16047). Retained reference `source-window-12`.

```text
Tcl_GetCharLength(
    Tcl_Obj *objPtr)		/* The String object to get the num chars
				 * of. */
{
    String *stringPtr;

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);

    /*
     * If numChars is unknown, then calculate the number of characaters while
     * populating the Unicode string.
     */

    if (stringPtr->numChars == -1) {
	register int i = objPtr->length;
	register unsigned char *str = (unsigned char *) objPtr->bytes;

	/*
	 * This is a speed sensitive function, so run specially over the
	 * string to count continuous ascii characters before resorting to the
	 * Tcl_NumUtfChars call. This is a long form of:
	 stringPtr->numChars = Tcl_NumUtfChars(objPtr->bytes,objPtr->length);
	 *
	 * TODO: Consider macro-izing this.
	 */

	while (i && (*str < 0xC0)) {
	    i--;
	    str++;
	}
	stringPtr->numChars = objPtr->length - i;
	if (i) {
	    stringPtr->numChars += Tcl_NumUtfChars(objPtr->bytes
		    + (objPtr->length - i), i);
	}

	if (stringPtr->numChars == objPtr->length) {
	    /*
	     * Since we've just calculated the number of chars, and all UTF
	     * chars are 1-byte long, we don't need to store the unicode
	     * string.
	     */

	    stringPtr->hasUnicode = 0;
	} else {
	    /*
	     * Since we've just calucalated the number of chars, and not all
	     * UTF chars are 1-byte long, go ahead and populate the unicode
	     * string.
	     */

	    FillUnicodeRep(objPtr);

	    /*
	     * We need to fetch the pointer again because we have just
	     * reallocated the structure to make room for the Unicode data.
	     */

	    stringPtr = GET_STRING(objPtr);
	}
    }
    return stringPtr->numChars;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclStringObj.c`, function `Tcl_NewUnicodeObj`, lines 364–375. Line unit: physical LF only. Original byte range [13641, 13949). Retained reference `source-window-13`.

```text
Tcl_NewUnicodeObj(
    const Tcl_UniChar *unicode,	/* The unicode string used to initialize the
				 * new object. */
    int numChars)		/* Number of characters in the unicode
				 * string. */
{
    Tcl_Obj *objPtr;

    TclNewObj(objPtr);
    SetUnicodeObj(objPtr, unicode, numChars);
    return objPtr;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclUtil.c`, function `TclGetIntForIndex`, lines 3234–3325. Line unit: physical LF only. Original byte range [92509, 94743). Retained reference `source-window-14`.

```text
TclGetIntForIndex(
    Tcl_Interp *interp,		/* Interpreter to use for error reporting. If
				 * NULL, then no error message is left after
				 * errors. */
    Tcl_Obj *objPtr,		/* Points to an object containing either "end"
				 * or an integer. */
    int endValue,		/* The value to be stored at "indexPtr" if
				 * "objPtr" holds "end". */
    int *indexPtr)		/* Location filled in with an integer
				 * representing an index. */
{
    int length;
    char *opPtr, *bytes;

    if (TclGetIntFromObj(NULL, objPtr, indexPtr) == TCL_OK) {
	return TCL_OK;
    }

    if (SetEndOffsetFromAny(NULL, objPtr) == TCL_OK) {
	/*
	 * If the object is already an offset from the end of the list, or can
	 * be converted to one, use it.
	 */

	*indexPtr = endValue + objPtr->internalRep.longValue;
	return TCL_OK;
    }

    bytes = TclGetStringFromObj(objPtr, &length);

    /*
     * Leading whitespace is acceptable in an index.
     */

    while (length && TclIsSpaceProc(*bytes)) {
	bytes++;
	length--;
    }

    if (TclParseNumber(NULL, NULL, NULL, bytes, length, (const char **)&opPtr,
	    TCL_PARSE_INTEGER_ONLY | TCL_PARSE_NO_WHITESPACE) == TCL_OK) {
	int code, first, second;
	char savedOp = *opPtr;

	if ((savedOp != '+') && (savedOp != '-')) {
	    goto parseError;
	}
	if (TclIsSpaceProc(opPtr[1])) {
	    goto parseError;
	}
	*opPtr = '\0';
	code = Tcl_GetInt(interp, bytes, &first);
	*opPtr = savedOp;
	if (code == TCL_ERROR) {
	    goto parseError;
	}
	if (TCL_ERROR == Tcl_GetInt(interp, opPtr+1, &second)) {
	    goto parseError;
	}
	if (savedOp == '+') {
	    *indexPtr = first + second;
	} else {
	    *indexPtr = first - second;
	}
	return TCL_OK;
    }

    /*
     * Report a parse error.
     */

  parseError:
    if (interp != NULL) {
	char *bytes = Tcl_GetString(objPtr);

	/*
	 * The result might not be empty; this resets it which should be both
	 * a cheap operation, and of little problem because this is an
	 * error-generation path anyway.
	 */

	Tcl_ResetResult(interp);
	Tcl_AppendResult(interp, "bad index \"", bytes,
		"\": must be integer?[+-]integer? or end?[+-]integer?", NULL);
	if (!strncmp(bytes, "end-", 4)) {
	    bytes += 4;
	}
	TclCheckBadOctal(interp, bytes);
    }

    return TCL_ERROR;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInt.h`, function `TclUtfToUniChar macro`, lines 3670–3708. Line unit: physical LF only. Original byte range [141500, 143213). Retained reference `source-window-15`.

```text
#define TclUtfToUniChar(str, chPtr) \
	((((unsigned char) *(str)) < 0xC0) ?		\
	    ((*(chPtr) = (Tcl_UniChar) *(str)), 1)	\
	    : Tcl_UtfToUniChar(str, chPtr))

/*
 *----------------------------------------------------------------
 * Macro that encapsulates the logic that determines when it is safe to
 * interpret a string as a byte array directly. In summary, the object must be
 * a byte array and must not have a string representation (as the operations
 * that it is used in are defined on strings, not byte arrays). Theoretically
 * it is possible to also be efficient in the case where the object's bytes
 * field is filled by generation from the byte array (c.f. list canonicality)
 * but we don't do that at the moment since this is purely about efficiency.
 * The ANSI C "prototype" for this macro is:
 *
 * MODULE_SCOPE int	TclIsPureByteArray(Tcl_Obj *objPtr);
 *----------------------------------------------------------------
 */

#define TclIsPureByteArray(objPtr) \
	(((objPtr)->typePtr==&tclByteArrayType) && ((objPtr)->bytes==NULL))

/*
 *----------------------------------------------------------------
 * Macro used by the Tcl core to compare Unicode strings. On big-endian
 * systems we can use the more efficient memcmp, but this would not be
 * lexically correct on little-endian systems. The ANSI C "prototype" for
 * this macro is:
 *
 * MODULE_SCOPE int	TclUniCharNcmp(const Tcl_UniChar *cs,
 *			    const Tcl_UniChar *ct, unsigned long n);
 *----------------------------------------------------------------
 */

#ifdef WORDS_BIGENDIAN
#   define TclUniCharNcmp(cs,ct,n) memcmp((cs),(ct),(n)*sizeof(Tcl_UniChar))
#else /* !WORDS_BIGENDIAN */
#   define TclUniCharNcmp Tcl_UniCharNcmp

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdMZ.c`, function `StringIndexCmd`, lines 1381–1423. Line unit: physical LF only. Original byte range [34868, 35899). Retained reference `source-window-16`.

```text
StringIndexCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int length, index;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "string charIndex");
	return TCL_ERROR;
    }

    /*
     * Get the char length to calculate what 'end' means.
     */

    length = Tcl_GetCharLength(objv[1]);
    if (TclGetIntForIndexM(interp, objv[2], length-1, &index) != TCL_OK) {
	return TCL_ERROR;
    }

    if ((index >= 0) && (index < length)) {
	int ch = TclGetUCS4(objv[1], index);

	/*
	 * If we have a ByteArray object, we're careful to generate a new
	 * bytearray for a result.
	 */

	if (TclIsPureByteArray(objv[1])) {
	    unsigned char uch = UCHAR(ch);

	    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(&uch, 1));
	} else {
	    char buf[8] = "";

	    length = TclUCS4ToUtf(ch, buf);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(buf, length));
	}
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdMZ.c`, function `StringRangeCmd`, lines 2161–2196. Line unit: physical LF only. Original byte range [54494, 55397). Retained reference `source-window-17`.

```text
StringRangeCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int length, first, last;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last");
	return TCL_ERROR;
    }

    /*
     * Get the length in actual characters; Then reduce it by one because
     * 'end' refers to the last character, not one past it.
     */

    length = Tcl_GetCharLength(objv[1]) - 1;

    if (TclGetIntForIndexM(interp, objv[2], length, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], length, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    if (first < 0) {
	first = 0;
    }
    if (last >= length) {
	last = length;
    }
    if (last >= first) {
	Tcl_SetObjResult(interp, Tcl_GetRange(objv[1], first, last));
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_GetUniChar`, lines 515–570. Line unit: physical LF only. Original byte range [16001, 17254). Retained reference `source-window-18`.

```text
Tcl_GetUniChar(
    Tcl_Obj *objPtr,		/* The object to get the Unicode charater
				 * from. */
    int index)			/* Get the index'th Unicode character. */
{
    String *stringPtr;

    if (index < 0) {
	return 0xFFFD;
    }

    /*
     * Optimize the case where we're really dealing with a ByteArray object
     * we don't need to convert to a string to perform the indexing operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	int length;
	unsigned char *bytes = Tcl_GetByteArrayFromObj(objPtr, &length);
	if (index >= length) {
		return 0xFFFD;
	}

	return bytes[index];
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
	if (index >= stringPtr->numChars) {
	    return 0xFFFD;
	}
	if (stringPtr->numChars == objPtr->length) {
	    return (unsigned char) objPtr->bytes[index];
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }

    if (index >= stringPtr->numChars) {
	return 0xFFFD;
    }
    return stringPtr->unicode[index];
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 741–829. Line unit: physical LF only. Original byte range [21606, 23971). Retained reference `source-window-19`.

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

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_GetCharLength`, lines 402–453. Line unit: physical LF only. Original byte range [13279, 14616). Retained reference `source-window-20`.

```text
Tcl_GetCharLength(
    Tcl_Obj *objPtr)		/* The String object to get the num chars
				 * of. */
{
    String *stringPtr;
    int numChars;

    /*
     * Quick, no-shimmer return for short string reps.
     */

    if ((objPtr->bytes) && (objPtr->length < 2)) {
	/* 0 bytes -> 0 chars; 1 byte -> 1 char */
	return objPtr->length;
    }

    /*
     * Optimize the case where we're really dealing with a bytearray object;
     * we don't need to convert to a string to perform the get-length operation.
     *
     * NOTE that we do not need the ByteArray to be "pure".  A ByteArray value
     * with a string rep cannot be trusted to represent the same value as the
     * string rep, but it *can* be trusted to have the same character length
     * as the string rep, which is all this routine cares about.
     */

    if (objPtr->typePtr == &tclByteArrayType) {
	int length;

	(void) Tcl_GetByteArrayFromObj(objPtr, &length);
	return length;
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);
    numChars = stringPtr->numChars;

    /*
     * If numChars is unknown, compute it.
     */

    if (numChars == -1) {
	TclNumUtfChars(numChars, objPtr->bytes, objPtr->length);
	stringPtr->numChars = numChars;

    }
    return numChars;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_NewUnicodeObj`, lines 371–382. Line unit: physical LF only. Original byte range [12542, 12850). Retained reference `source-window-21`.

```text
Tcl_NewUnicodeObj(
    const Tcl_UniChar *unicode,	/* The unicode string used to initialize the
				 * new object. */
    int numChars)		/* Number of characters in the unicode
				 * string. */
{
    Tcl_Obj *objPtr;

    TclNewObj(objPtr);
    SetUnicodeObj(objPtr, unicode, numChars);
    return objPtr;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclUtil.c`, function `TclGetIntForIndex`, lines 3634–3714. Line unit: physical LF only. Original byte range [101826, 103787). Retained reference `source-window-22`.

```text
TclGetIntForIndex(
    Tcl_Interp *interp,		/* Interpreter to use for error reporting. If
				 * NULL, then no error message is left after
				 * errors. */
    Tcl_Obj *objPtr,		/* Points to an object containing either "end"
				 * or an integer. */
    int endValue,		/* The value to be stored at "indexPtr" if
				 * "objPtr" holds "end". */
    int *indexPtr)		/* Location filled in with an integer
				 * representing an index. */
{
    int length;
    char *opPtr;
    const char *bytes;

    if (TclGetIntFromObj(NULL, objPtr, indexPtr) == TCL_OK) {
	return TCL_OK;
    }

    if (GetEndOffsetFromObj(objPtr, endValue, indexPtr) == TCL_OK) {
	return TCL_OK;
    }

    bytes = TclGetStringFromObj(objPtr, &length);

    /*
     * Leading whitespace is acceptable in an index.
     */

    while (length && TclIsSpaceProcM(*bytes)) {
	bytes++;
	length--;
    }

    if (TclParseNumber(NULL, NULL, NULL, bytes, length, (const char **)&opPtr,
	    TCL_PARSE_INTEGER_ONLY | TCL_PARSE_NO_WHITESPACE) == TCL_OK) {
	int code, first, second;
	char savedOp = *opPtr;

	if ((savedOp != '+') && (savedOp != '-')) {
	    goto parseError;
	}
	if (TclIsSpaceProcM(opPtr[1])) {
	    goto parseError;
	}
	*opPtr = '\0';
	code = Tcl_GetInt(interp, bytes, &first);
	*opPtr = savedOp;
	if (code == TCL_ERROR) {
	    goto parseError;
	}
	if (TCL_ERROR == Tcl_GetInt(interp, opPtr+1, &second)) {
	    goto parseError;
	}
	if (savedOp == '+') {
	    *indexPtr = first + second;
	} else {
	    *indexPtr = first - second;
	}
	return TCL_OK;
    }

    /*
     * Report a parse error.
     */

  parseError:
    if (interp != NULL) {
	bytes = TclGetString(objPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"bad index \"%s\": must be integer?[+-]integer? or"
		" end?[+-]integer?", bytes));
	if (!strncmp(bytes, "end-", 4)) {
	    bytes += 4;
	}
	TclCheckBadOctal(interp, bytes);
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX", (char *)NULL);
    }

    return TCL_ERROR;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclUtil.c`, function `GetEndOffsetFromObj`, lines 3772–3786. Line unit: physical LF only. Original byte range [105138, 105708). Retained reference `source-window-23`.

```text
GetEndOffsetFromObj(
    Tcl_Obj *objPtr,            /* Pointer to the object to parse */
    int endValue,               /* The value to be stored at "indexPtr" if
                                 * "objPtr" holds "end". */
    int *indexPtr)              /* Location filled in with an integer
                                 * representing an index. */
{
    if (SetEndOffsetFromAny(NULL, objPtr) != TCL_OK) {
	return TCL_ERROR;
    }

    /* TODO: Handle overflow cases sensibly */
    *indexPtr = endValue + (int)objPtr->internalRep.longValue;
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdMZ.c`, function `StringIndexCmd`, lines 1399–1445. Line unit: physical LF only. Original byte range [36397, 37452). Retained reference `source-window-24`.

```text
StringIndexCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Size index, end;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "string charIndex");
	return TCL_ERROR;
    }

    /*
     * Get the char length to calculate what 'end' means.
     */

    end = Tcl_GetCharLength(objv[1]) - 1;
    if (TclGetIntForIndexM(interp, objv[2], end, &index) != TCL_OK) {
	return TCL_ERROR;
    }

    if ((index >= 0) && (index <= end)) {
	int ch = Tcl_GetUniChar(objv[1], index);

	if (ch == -1) {
	    return TCL_OK;
	}

	/*
	 * If we have a ByteArray object, we're careful to generate a new
	 * bytearray for a result.
	 */

	if (TclIsPureByteArray(objv[1])) {
	    unsigned char uch = UCHAR(ch);

	    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(&uch, 1));
	} else {
	    char buf[4] = "";

	    end = Tcl_UniCharToUtf(ch, buf);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(buf, end));
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdMZ.c`, function `StringRangeCmd`, lines 2286–2315. Line unit: physical LF only. Original byte range [58732, 59522). Retained reference `source-window-25`.

```text
StringRangeCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Size first, last, end;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last");
	return TCL_ERROR;
    }

    /*
     * Get the length in actual characters; Then reduce it by one because
     * 'end' refers to the last character, not one past it.
     */

    end = Tcl_GetCharLength(objv[1]) - 1;

    if (TclGetIntForIndexM(interp, objv[2], end, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], end, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    if (last >= 0) {
	Tcl_SetObjResult(interp, Tcl_GetRange(objv[1], first, last));
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_GetUniChar`, lines 532–589. Line unit: physical LF only. Original byte range [16892, 18182). Retained reference `source-window-26`.

```text
Tcl_GetUniChar(
    Tcl_Obj *objPtr,		/* The object to get the Unicode charater
				 * from. */
    Tcl_Size index)		/* Get the index'th Unicode character. */
{
    String *stringPtr;
    int ch;

    if (index < 0) {
	return -1;
    }

    /*
     * Optimize the case where we're really dealing with a ByteArray object
     * we don't need to convert to a string to perform the indexing operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	Tcl_Size length = 0;
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &length);
	if (index >= length) {
		return -1;
	}

	return bytes[index];
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
	if (index >= stringPtr->numChars) {
	    return -1;
	}
	if (stringPtr->numChars == objPtr->length) {
	    return (unsigned char) objPtr->bytes[index];
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }

    if (index >= stringPtr->numChars) {
	return -1;
    }
    ch = stringPtr->unicode[index];
    return ch;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 725–802. Line unit: physical LF only. Original byte range [21719, 23746). Retained reference `source-window-27`.

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

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_GetCharLength`, lines 378–427. Line unit: physical LF only. Original byte range [12857, 14260). Retained reference `source-window-28`.

```text
Tcl_GetCharLength(
    Tcl_Obj *objPtr)		/* The String object to get the num chars
				 * of. */
{
    String *stringPtr;
    Tcl_Size numChars = 0;

    /*
     * Quick, no-shimmer return for short string reps.
     */

    if ((objPtr->bytes) && (objPtr->length < 2)) {
	/* 0 bytes -> 0 chars; 1 byte -> 1 char */
	return objPtr->length;
    }

    /*
     * Optimize the case where we're really dealing with a byte-array object;
     * we don't need to convert to a string to perform the get-length operation.
     *
     * Starting in Tcl 9.0, we check for a "pure" byte-array, because the
     * machinery behind that test is using a proper byte-array ObjType.  We
     * could also compute length of an improper byte-array without shimmering
     * but there's no value in that. We *want* to shimmer an improper byte-array
     * because improper byte-arrays have worthless internal reps.
     */

    if (TclIsPureByteArray(objPtr)) {
	(void) Tcl_GetBytesFromObj(NULL, objPtr, &numChars);
	return numChars;
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);
    numChars = stringPtr->numChars;

    /*
     * If numChars is unknown, compute it.
     */

    if (numChars < 0) {
	TclNumUtfCharsM(numChars, objPtr->bytes, objPtr->length);
	stringPtr->numChars = numChars;
    }
    return numChars;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_NewUnicodeObj`, lines 347–358. Line unit: physical LF only. Original byte range [12110, 12423). Retained reference `source-window-29`.

```text
Tcl_NewUnicodeObj(
    const Tcl_UniChar *unicode,	/* The unicode string used to initialize the
				 * new object. */
    Tcl_Size numChars)		/* Number of characters in the unicode
				 * string. */
{
    Tcl_Obj *objPtr;

    TclNewObj(objPtr);
    SetUnicodeObj(objPtr, unicode, numChars);
    return objPtr;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclUtil.c`, function `Tcl_GetIntForIndex`, lines 3470–3500. Line unit: physical LF only. Original byte range [97702, 98838). Retained reference `source-window-30`.

```text
Tcl_GetIntForIndex(
    Tcl_Interp *interp,		/* Interpreter to use for error reporting. If
				 * NULL, then no error message is left after
				 * errors. */
    Tcl_Obj *objPtr,		/* Points to an object containing either "end"
				 * or an integer. */
    Tcl_Size endValue,		/* The value corresponding to the "end" index */
    Tcl_Size *indexPtr)		/* Location filled in with an integer
				 * representing an index. May be NULL.*/
{
    Tcl_WideInt wide;

    if (GetWideForIndex(interp, objPtr, endValue, &wide) == TCL_ERROR) {
	return TCL_ERROR;
    }
    if (indexPtr != NULL) {
	/* Note: check against TCL_SIZE_MAX needed for 32-bit builds */
	if (wide >= 0 && wide <= TCL_SIZE_MAX) {
	    *indexPtr = (Tcl_Size)wide; /* A valid index */
	} else if (wide > TCL_SIZE_MAX) {
	    *indexPtr = TCL_SIZE_MAX;   /* Beyond max possible index */
	} else if (wide < -1-TCL_SIZE_MAX) {
	    *indexPtr = -1-TCL_SIZE_MAX; /* Below most negative index */
	} else if ((wide < 0) && (endValue >= 0)) {
	    *indexPtr = TCL_INDEX_NONE; /* No clue why this special case */
	} else {
	    *indexPtr = (Tcl_Size) wide;
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclUtil.c`, function `GetEndOffsetFromObj`, lines 3532–3770. Line unit: physical LF only. Original byte range [99708, 106500). Retained reference `source-window-31`.

```text
GetEndOffsetFromObj(
    Tcl_Interp *interp,
    Tcl_Obj *objPtr,		/* Pointer to the object to parse */
    Tcl_WideInt endValue,	/* The value to be stored at "widePtr" if
				 * "objPtr" holds "end". */
    Tcl_WideInt *widePtr)	/* Location filled in with an integer
				 * representing an index. */
{
    Tcl_ObjInternalRep *irPtr;
    Tcl_WideInt offset = -1;	/* Offset in the "end-offset" expression - 1 */
    void *cd;

    while ((irPtr = TclFetchInternalRep(objPtr, &endOffsetType)) == NULL) {
	Tcl_ObjInternalRep ir;
	Tcl_Size length;
	const char *bytes = TclGetStringFromObj(objPtr, &length);

	if (*bytes != 'e') {
	    int numType;
	    const char *opPtr;
	    int t1 = 0, t2 = 0;

	    /* Value doesn't start with "e" */

	    /* If we reach here, the string rep of objPtr exists. */

	    /*
	     * The valid index syntax does not include any value that is
	     * a list of more than one element. This is necessary so that
	     * lists of index values can be reliably distinguished from any
	     * single index value.
	     */

	    /*
	     * Quick scan to see if multi-value list is even possible.
	     * This relies on TclGetString() returning a NUL-terminated string.
	     */
	    if ((TclMaxListLength(bytes, TCL_INDEX_NONE, NULL) > 1)
		    /* If it's possible, do the full list parse. */
		    && (TCL_OK == TclListObjLength(NULL, objPtr, &length))
		    && (length > 1)) {
		goto parseError;
	    }

	    /* Passed the list screen, so parse for index arithmetic expression */
	    if (TCL_OK == TclParseNumber(NULL, objPtr, NULL, NULL, TCL_INDEX_NONE, &opPtr,
		    TCL_PARSE_INTEGER_ONLY)) {
		Tcl_WideInt w1=0, w2=0;

		/* value starts with valid integer... */

		if ((*opPtr == '-') || (*opPtr == '+')) {
		    /* ... value continues with [-+] ... */

		    /* Save first integer as wide if possible */
		    Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t1);
		    if (t1 == TCL_NUMBER_INT) {
			w1 = (*(Tcl_WideInt *)cd);
		    }

		    if (TCL_OK == TclParseNumber(NULL, objPtr, NULL, opPtr + 1,
			    TCL_INDEX_NONE, NULL, TCL_PARSE_INTEGER_ONLY)) {
			/* ... value concludes with second valid integer */

			/* Save second integer as wide if possible */
			Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t2);
			if (t2 == TCL_NUMBER_INT) {
			    w2 = (*(Tcl_WideInt *)cd);
			}
		    }
		}
		/* Clear invalid internalreps left by TclParseNumber */
		TclFreeInternalRep(objPtr);

		if (t1 && t2) {
		    /* We have both integer values */
		    if ((t1 == TCL_NUMBER_INT) && (t2 == TCL_NUMBER_INT)) {
			/* Both are wide, do wide-integer math */
			if (*opPtr == '-') {
			    if (w2 == WIDE_MIN) {
				goto extreme;
			    }
			    w2 = -w2;
			}

			if ((w1 ^ w2) < 0) {
			    /* Different signs, sum cannot overflow */
			    offset = w1 + w2;
			} else if (w1 >= 0) {
			    if (w1 < WIDE_MAX - w2) {
				offset = w1 + w2;
			    } else {
				offset = WIDE_MAX;
			    }
			} else {
			    if (w1 > WIDE_MIN - w2) {
				offset = w1 + w2;
			    } else {
				offset = WIDE_MIN;
			    }
			}
		    } else {
			/*
			 * At least one is big, do bignum math. Little reason to
			 * value performance here. Re-use code.  Parse has verified
			 * objPtr is an expression. Compute it.
			 */

			Tcl_Obj *sum;

		    extreme:
			if (interp) {
			    Tcl_ExprObj(interp, objPtr, &sum);
			} else {
			    Tcl_Interp *compute = Tcl_CreateInterp();
			    Tcl_ExprObj(compute, objPtr, &sum);
			    Tcl_DeleteInterp(compute);
			}
			Tcl_GetNumberFromObj(NULL, sum, &cd, &numType);

			if (numType == TCL_NUMBER_INT) {
			    /* sum holds an integer in the signed wide range */
			    offset = *(Tcl_WideInt *)cd;
			} else {
			    /* sum holds an integer outside the signed wide range */
			    /* Truncate to the signed wide range. */
			    if (mp_isneg((mp_int *)cd)) {
				offset = WIDE_MIN;
			    } else {
				offset = WIDE_MAX;
			    }
			}
			Tcl_DecrRefCount(sum);
		    }
		    if (offset < 0) {
			offset = (offset == -1) ? WIDE_MIN : WIDE_MIN+1;
		    }
		    goto parseOK;
		}
	    }
	    goto parseError;
	}

	if ((length < 3) || (length == 4) || (strncmp(bytes, "end", 3) != 0)) {
	    /* Doesn't start with "end" */
	    goto parseError;
	}
	if (length > 4) {
	    int t;

	    /* Parse for the "end-..." or "end+..." formats */

	    if ((bytes[3] != '-') && (bytes[3] != '+')) {
		/* No operator where we need one */
		goto parseError;
	    }
	    if (TclIsSpaceProc(bytes[4])) {
		/* Space after + or - not permitted. */
		goto parseError;
	    }

	    /* Parse the integer offset */
	    if (TCL_OK != TclParseNumber(NULL, objPtr, NULL,
		    bytes + 4, length - 4, NULL, TCL_PARSE_INTEGER_ONLY)) {
		/* Not a recognized integer format */
		goto parseError;
	    }

	    /* Got an integer offset; pull it from where parser left it. */
	    Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t);

	    if (t == TCL_NUMBER_BIG) {
		/* Truncate to the signed wide range. */
		if (mp_isneg((mp_int *)cd)) {
		    offset = (bytes[3] == '-') ? WIDE_MAX : WIDE_MIN;
		} else {
		    offset = (bytes[3] == '-') ? WIDE_MIN : WIDE_MAX;
		}
	    } else {
		/* assert (t == TCL_NUMBER_INT); */
		offset = (*(Tcl_WideInt *)cd);
		if (bytes[3] == '-') {
		    offset = (offset == WIDE_MIN) ? WIDE_MAX : -offset;
		}
		if (offset == 1) {
		    offset = WIDE_MAX; /* "end+1" */
		} else if (offset > 1) {
		    offset = WIDE_MAX - 1; /* "end+n", out of range */
		} else if (offset != WIDE_MIN) {
		    offset--;
		}
	    }
	}

    parseOK:
	/* Success. Store the new internal rep. */
	ir.wideValue = offset;
	Tcl_StoreInternalRep(objPtr, &endOffsetType, &ir);
    }

    offset = irPtr->wideValue;

    if (offset == WIDE_MAX) {
	/*
	 * Encodes end+1. This is distinguished from end+n as noted
	 * in function header.
	 * NOTE: this may wrap around if the caller passes (as lset does)
	 * listLen-1 as endValue and listLen is 0. The -1 will be
	 * interpreted as FF...FF and adding 1 will result in 0 which
	 * is what we want. Callers like lset which pass in listLen-1 == -1
	 * as endValue will have to adjust accordingly.
	 */
	*widePtr = (endValue == -1) ? WIDE_MAX : endValue + 1;
    } else if (offset == WIDE_MIN) {
	*widePtr = (endValue == -1) ? WIDE_MIN : -1;
    } else if (offset < 0) {
	/* end-(n-1) - Different signs, sum cannot overflow */
	*widePtr = endValue + offset + 1;
    } else {
	/* 0:WIDE_MAX - plain old index. */
	*widePtr = offset;
    }
    return TCL_OK;

    /* Report a parse error. */
  parseError:
    if (interp != NULL) {
	char * bytes = TclGetString(objPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"bad index \"%s\": must be integer?[+-]integer? or"
		" end?[+-]integer?", bytes));
	if (!strncmp(bytes, "end-", 4)) {
	    bytes += 4;
	}
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX", (char *)NULL);
    }

    return TCL_ERROR;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdMZ.c`, function `StringIndexCmd`, lines 1464–1510. Line unit: physical LF only. Original byte range [39626, 40684). Retained reference `source-window-32`.

```text
StringIndexCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size index, end;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "string charIndex");
	return TCL_ERROR;
    }

    /*
     * Get the char length to calculate what 'end' means.
     */

    end = Tcl_GetCharLength(objv[1]) - 1;
    if (TclGetIntForIndexM(interp, objv[2], end, &index) != TCL_OK) {
	return TCL_ERROR;
    }

    if ((index >= 0) && (index <= end)) {
	int ch = Tcl_GetUniChar(objv[1], index);

	if (ch == -1) {
	    return TCL_OK;
	}

	/*
	 * If we have a ByteArray object, we're careful to generate a new
	 * bytearray for a result.
	 */

	if (TclIsPureByteArray(objv[1])) {
	    unsigned char uch = UCHAR(ch);

	    Tcl_SetObjResult(interp, Tcl_NewByteArrayObj(&uch, 1));
	} else {
	    char buf[4] = "";

	    end = Tcl_UniCharToUtf(ch, buf);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(buf, end));
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdMZ.c`, function `StringRangeCmd`, lines 2351–2380. Line unit: physical LF only. Original byte range [61981, 62774). Retained reference `source-window-33`.

```text
StringRangeCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size first, last, end;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "string first last");
	return TCL_ERROR;
    }

    /*
     * Get the length in actual characters; Then reduce it by one because
     * 'end' refers to the last character, not one past it.
     */

    end = Tcl_GetCharLength(objv[1]) - 1;

    if (TclGetIntForIndexM(interp, objv[2], end, &first) != TCL_OK ||
	    TclGetIntForIndexM(interp, objv[3], end, &last) != TCL_OK) {
	return TCL_ERROR;
    }

    if (last >= 0) {
	Tcl_SetObjResult(interp, Tcl_GetRange(objv[1], first, last));
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_GetUniChar`, lines 535–592. Line unit: physical LF only. Original byte range [16905, 18198). Retained reference `source-window-34`.

```text
Tcl_GetUniChar(
    Tcl_Obj *objPtr,		/* The object to get the Unicode charater
				 * from. */
    Tcl_Size index)		/* Get the index'th Unicode character. */
{
    String *stringPtr;
    int ch;

    if (index < 0) {
	return -1;
    }

    /*
     * Optimize the case where we're really dealing with a ByteArray object
     * we don't need to convert to a string to perform the indexing operation.
     */

    if (TclIsPureByteArray(objPtr)) {
	Tcl_Size length = 0;
	unsigned char *bytes = Tcl_GetBytesFromObj(NULL, objPtr, &length);
	if (index >= length) {
	    return -1;
	}

	return bytes[index];
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
	if (index >= stringPtr->numChars) {
	    return -1;
	}
	if (stringPtr->numChars == objPtr->length) {
	    return (unsigned char) objPtr->bytes[index];
	}
	FillUnicodeRep(objPtr);
	stringPtr = GET_STRING(objPtr);
    }

    if (index >= stringPtr->numChars) {
	return -1;
    }
    ch = stringPtr->unicode[index];
    return ch;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 728–805. Line unit: physical LF only. Original byte range [21720, 23747). Retained reference `source-window-35`.

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

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_GetCharLength`, lines 381–430. Line unit: physical LF only. Original byte range [12869, 14272). Retained reference `source-window-36`.

```text
Tcl_GetCharLength(
    Tcl_Obj *objPtr)		/* The String object to get the num chars
				 * of. */
{
    String *stringPtr;
    Tcl_Size numChars = 0;

    /*
     * Quick, no-shimmer return for short string reps.
     */

    if ((objPtr->bytes) && (objPtr->length < 2)) {
	/* 0 bytes -> 0 chars; 1 byte -> 1 char */
	return objPtr->length;
    }

    /*
     * Optimize the case where we're really dealing with a byte-array object;
     * we don't need to convert to a string to perform the get-length operation.
     *
     * Starting in Tcl 9.0, we check for a "pure" byte-array, because the
     * machinery behind that test is using a proper byte-array ObjType.  We
     * could also compute length of an improper byte-array without shimmering
     * but there's no value in that. We *want* to shimmer an improper byte-array
     * because improper byte-arrays have worthless internal reps.
     */

    if (TclIsPureByteArray(objPtr)) {
	(void) Tcl_GetBytesFromObj(NULL, objPtr, &numChars);
	return numChars;
    }

    /*
     * OK, need to work with the object as a string.
     */

    SetStringFromAny(NULL, objPtr);
    stringPtr = GET_STRING(objPtr);
    numChars = stringPtr->numChars;

    /*
     * If numChars is unknown, compute it.
     */

    if (numChars < 0) {
	TclNumUtfCharsM(numChars, objPtr->bytes, objPtr->length);
	stringPtr->numChars = numChars;
    }
    return numChars;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_NewUnicodeObj`, lines 350–361. Line unit: physical LF only. Original byte range [12122, 12435). Retained reference `source-window-37`.

```text
Tcl_NewUnicodeObj(
    const Tcl_UniChar *unicode,	/* The unicode string used to initialize the
				 * new object. */
    Tcl_Size numChars)		/* Number of characters in the unicode
				 * string. */
{
    Tcl_Obj *objPtr;

    TclNewObj(objPtr);
    SetUnicodeObj(objPtr, unicode, numChars);
    return objPtr;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclUtil.c`, function `Tcl_GetIntForIndex`, lines 3535–3565. Line unit: physical LF only. Original byte range [100090, 101224). Retained reference `source-window-38`.

```text
Tcl_GetIntForIndex(
    Tcl_Interp *interp,		/* Interpreter to use for error reporting. If
				 * NULL, then no error message is left after
				 * errors. */
    Tcl_Obj *objPtr,		/* Points to an object containing either "end"
				 * or an integer. */
    Tcl_Size endValue,		/* The value corresponding to the "end" index */
    Tcl_Size *indexPtr)		/* Location filled in with an integer
				 * representing an index. May be NULL.*/
{
    Tcl_WideInt wide;

    if (GetWideForIndex(interp, objPtr, endValue, &wide) == TCL_ERROR) {
	return TCL_ERROR;
    }
    if (indexPtr != NULL) {
	/* Note: check against TCL_SIZE_MAX needed for 32-bit builds */
	if (wide >= 0 && wide <= TCL_SIZE_MAX) {
	    *indexPtr = (Tcl_Size)wide;	/* A valid index */
	} else if (wide > TCL_SIZE_MAX) {
	    *indexPtr = TCL_SIZE_MAX;	/* Beyond max possible index */
	} else if (wide < -1-TCL_SIZE_MAX) {
	    *indexPtr = -1-TCL_SIZE_MAX; /* Below most negative index */
	} else if ((wide < 0) && (endValue >= 0)) {
	    *indexPtr = TCL_INDEX_NONE;	/* No clue why this special case */
	} else {
	    *indexPtr = (Tcl_Size) wide;
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclUtil.c`, function `GetEndOffsetFromObj`, lines 3597–3835. Line unit: physical LF only. Original byte range [102089, 108881). Retained reference `source-window-39`.

```text
GetEndOffsetFromObj(
    Tcl_Interp *interp,
    Tcl_Obj *objPtr,		/* Pointer to the object to parse */
    Tcl_WideInt endValue,	/* The value to be stored at "widePtr" if
				 * "objPtr" holds "end". */
    Tcl_WideInt *widePtr)	/* Location filled in with an integer
				 * representing an index. */
{
    Tcl_ObjInternalRep *irPtr;
    Tcl_WideInt offset = -1;	/* Offset in the "end-offset" expression - 1 */
    void *cd;

    while ((irPtr = TclFetchInternalRep(objPtr, &endOffsetType)) == NULL) {
	Tcl_ObjInternalRep ir;
	Tcl_Size length;
	const char *bytes = TclGetStringFromObj(objPtr, &length);

	if (*bytes != 'e') {
	    int numType;
	    const char *opPtr;
	    int t1 = 0, t2 = 0;

	    /* Value doesn't start with "e" */

	    /* If we reach here, the string rep of objPtr exists. */

	    /*
	     * The valid index syntax does not include any value that is
	     * a list of more than one element. This is necessary so that
	     * lists of index values can be reliably distinguished from any
	     * single index value.
	     */

	    /*
	     * Quick scan to see if multi-value list is even possible.
	     * This relies on TclGetString() returning a NUL-terminated string.
	     */
	    if ((TclMaxListLength(bytes, TCL_INDEX_NONE, NULL) > 1)
		    /* If it's possible, do the full list parse. */
		    && (TCL_OK == TclListObjLength(NULL, objPtr, &length))
		    && (length > 1)) {
		goto parseError;
	    }

	    /* Passed the list screen, so parse for index arithmetic expression */
	    if (TCL_OK == TclParseNumber(NULL, objPtr, NULL, NULL, TCL_INDEX_NONE, &opPtr,
		    TCL_PARSE_INTEGER_ONLY)) {
		Tcl_WideInt w1=0, w2=0;

		/* value starts with valid integer... */

		if ((*opPtr == '-') || (*opPtr == '+')) {
		    /* ... value continues with [-+] ... */

		    /* Save first integer as wide if possible */
		    Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t1);
		    if (t1 == TCL_NUMBER_INT) {
			w1 = (*(Tcl_WideInt *)cd);
		    }

		    if (TCL_OK == TclParseNumber(NULL, objPtr, NULL, opPtr + 1,
			    TCL_INDEX_NONE, NULL, TCL_PARSE_INTEGER_ONLY)) {
			/* ... value concludes with second valid integer */

			/* Save second integer as wide if possible */
			Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t2);
			if (t2 == TCL_NUMBER_INT) {
			    w2 = (*(Tcl_WideInt *)cd);
			}
		    }
		}
		/* Clear invalid internalreps left by TclParseNumber */
		TclFreeInternalRep(objPtr);

		if (t1 && t2) {
		    /* We have both integer values */
		    if ((t1 == TCL_NUMBER_INT) && (t2 == TCL_NUMBER_INT)) {
			/* Both are wide, do wide-integer math */
			if (*opPtr == '-') {
			    if (w2 == WIDE_MIN) {
				goto extreme;
			    }
			    w2 = -w2;
			}

			if ((w1 ^ w2) < 0) {
			    /* Different signs, sum cannot overflow */
			    offset = w1 + w2;
			} else if (w1 >= 0) {
			    if (w1 < WIDE_MAX - w2) {
				offset = w1 + w2;
			    } else {
				offset = WIDE_MAX;
			    }
			} else {
			    if (w1 > WIDE_MIN - w2) {
				offset = w1 + w2;
			    } else {
				offset = WIDE_MIN;
			    }
			}
		    } else {
			/*
			 * At least one is big, do bignum math. Little reason to
			 * value performance here. Re-use code.  Parse has verified
			 * objPtr is an expression. Compute it.
			 */

			Tcl_Obj *sum;

		    extreme:
			if (interp) {
			    Tcl_ExprObj(interp, objPtr, &sum);
			} else {
			    Tcl_Interp *compute = Tcl_CreateInterp();
			    Tcl_ExprObj(compute, objPtr, &sum);
			    Tcl_DeleteInterp(compute);
			}
			Tcl_GetNumberFromObj(NULL, sum, &cd, &numType);

			if (numType == TCL_NUMBER_INT) {
			    /* sum holds an integer in the signed wide range */
			    offset = *(Tcl_WideInt *)cd;
			} else {
			    /* sum holds an integer outside the signed wide range */
			    /* Truncate to the signed wide range. */
			    if (mp_isneg((mp_int *)cd)) {
				offset = WIDE_MIN;
			    } else {
				offset = WIDE_MAX;
			    }
			}
			Tcl_DecrRefCount(sum);
		    }
		    if (offset < 0) {
			offset = (offset == -1) ? WIDE_MIN : WIDE_MIN+1;
		    }
		    goto parseOK;
		}
	    }
	    goto parseError;
	}

	if ((length < 3) || (length == 4) || (strncmp(bytes, "end", 3) != 0)) {
	    /* Doesn't start with "end" */
	    goto parseError;
	}
	if (length > 4) {
	    int t;

	    /* Parse for the "end-..." or "end+..." formats */

	    if ((bytes[3] != '-') && (bytes[3] != '+')) {
		/* No operator where we need one */
		goto parseError;
	    }
	    if (TclIsSpaceProc(bytes[4])) {
		/* Space after + or - not permitted. */
		goto parseError;
	    }

	    /* Parse the integer offset */
	    if (TCL_OK != TclParseNumber(NULL, objPtr, NULL,
		    bytes + 4, length - 4, NULL, TCL_PARSE_INTEGER_ONLY)) {
		/* Not a recognized integer format */
		goto parseError;
	    }

	    /* Got an integer offset; pull it from where parser left it. */
	    Tcl_GetNumberFromObj(NULL, objPtr, &cd, &t);

	    if (t == TCL_NUMBER_BIG) {
		/* Truncate to the signed wide range. */
		if (mp_isneg((mp_int *)cd)) {
		    offset = (bytes[3] == '-') ? WIDE_MAX : WIDE_MIN;
		} else {
		    offset = (bytes[3] == '-') ? WIDE_MIN : WIDE_MAX;
		}
	    } else {
		/* assert (t == TCL_NUMBER_INT); */
		offset = (*(Tcl_WideInt *)cd);
		if (bytes[3] == '-') {
		    offset = (offset == WIDE_MIN) ? WIDE_MAX : -offset;
		}
		if (offset == 1) {
		    offset = WIDE_MAX; /* "end+1" */
		} else if (offset > 1) {
		    offset = WIDE_MAX - 1; /* "end+n", out of range */
		} else if (offset != WIDE_MIN) {
		    offset--;
		}
	    }
	}

    parseOK:
	/* Success. Store the new internal rep. */
	ir.wideValue = offset;
	Tcl_StoreInternalRep(objPtr, &endOffsetType, &ir);
    }

    offset = irPtr->wideValue;

    if (offset == WIDE_MAX) {
	/*
	 * Encodes end+1. This is distinguished from end+n as noted
	 * in function header.
	 * NOTE: this may wrap around if the caller passes (as lset does)
	 * listLen-1 as endValue and listLen is 0. The -1 will be
	 * interpreted as FF...FF and adding 1 will result in 0 which
	 * is what we want. Callers like lset which pass in listLen-1 == -1
	 * as endValue will have to adjust accordingly.
	 */
	*widePtr = (endValue == -1) ? WIDE_MAX : endValue + 1;
    } else if (offset == WIDE_MIN) {
	*widePtr = (endValue == -1) ? WIDE_MIN : -1;
    } else if (offset < 0) {
	/* end-(n-1) - Different signs, sum cannot overflow */
	*widePtr = endValue + offset + 1;
    } else {
	/* 0:WIDE_MAX - plain old index. */
	*widePtr = offset;
    }
    return TCL_OK;

    /* Report a parse error. */
  parseError:
    if (interp != NULL) {
	char * bytes = TclGetString(objPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"bad index \"%s\": must be integer?[+-]integer? or"
		" end?[+-]integer?", bytes));
	if (!strncmp(bytes, "end-", 4)) {
	    bytes += 4;
	}
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "INDEX", (char *)NULL);
    }

    return TCL_ERROR;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimStringGetRange`, lines 2706–2719. Line unit: physical LF only. Original byte range [79361, 79839). Retained reference `source-window-40`.

```text
static int JimStringGetRange(Jim_Interp *interp, Jim_Obj *firstObjPtr, Jim_Obj *lastObjPtr,
    int len, int *first, int *last, int *range)
{
    if (Jim_GetIndex(interp, firstObjPtr, first) != JIM_OK) {
        return JIM_ERR;
    }
    if (Jim_GetIndex(interp, lastObjPtr, last) != JIM_OK) {
        return JIM_ERR;
    }
    *first = JimRelToAbsIndex(len, *first);
    *last = JimRelToAbsIndex(len, *last);
    JimRelToAbsRange(len, first, last, range);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_StringRangeObj`, lines 2741–2768. Line unit: physical LF only. Original byte range [80361, 81177). Retained reference `source-window-41`.

```text
Jim_Obj *Jim_StringRangeObj(Jim_Interp *interp,
    Jim_Obj *strObjPtr, Jim_Obj *firstObjPtr, Jim_Obj *lastObjPtr)
{
#ifdef JIM_UTF8
    int first, last;
    const char *str;
    int len, rangeLen;
    int bytelen;

    str = Jim_GetString(strObjPtr, &bytelen);
    len = Jim_Utf8Length(interp, strObjPtr);

    if (JimStringGetRange(interp, firstObjPtr, lastObjPtr, len, &first, &last, &rangeLen) != JIM_OK) {
        return NULL;
    }

    if (first == 0 && rangeLen == len) {
        return strObjPtr;
    }
    if (len == bytelen) {
        /* ASCII optimisation */
        return Jim_NewStringObj(interp, str + first, rangeLen);
    }
    return Jim_NewStringObjUtf8(interp, str + utf8_index(str, first), rangeLen);
#else
    return Jim_StringByteRangeObj(interp, strObjPtr, firstObjPtr, lastObjPtr);
#endif
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_StringByteRangeObj`, lines 2721–2739. Line unit: physical LF only. Original byte range [79840, 80360). Retained reference `source-window-42`.

```text
Jim_Obj *Jim_StringByteRangeObj(Jim_Interp *interp,
    Jim_Obj *strObjPtr, Jim_Obj *firstObjPtr, Jim_Obj *lastObjPtr)
{
    int first, last;
    const char *str;
    int rangeLen;
    int bytelen;

    str = Jim_GetString(strObjPtr, &bytelen);

    if (JimStringGetRange(interp, firstObjPtr, lastObjPtr, bytelen, &first, &last, &rangeLen) != JIM_OK) {
        return NULL;
    }

    if (first == 0 && rangeLen == bytelen) {
        return strObjPtr;
    }
    return Jim_NewStringObj(interp, str + first, rangeLen);
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetIndex`, lines 8326–8344. Line unit: physical LF only. Original byte range [259686, 260276). Retained reference `source-window-43`.

```text
int Jim_GetIndex(Jim_Interp *interp, Jim_Obj *objPtr, int *indexPtr)
{
    /* Avoid shimmering if the object is an integer. */
    if (objPtr->typePtr == &intObjType) {
        jim_wide val = JimWideValue(objPtr);

        if (val < 0)
            *indexPtr = -INT_MAX;
        else if (val > INT_MAX)
            *indexPtr = INT_MAX;
        else
            *indexPtr = (int)val;
        return JIM_OK;
    }
    if (objPtr->typePtr != &indexObjType && SetIndexFromAny(interp, objPtr) == JIM_ERR)
        return JIM_ERR;
    *indexPtr = objPtr->internalRep.intValue;
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `SetIndexFromAny`, lines 8254–8324. Line unit: physical LF only. Original byte range [257892, 259685). Retained reference `source-window-44`.

```text
static int SetIndexFromAny(Jim_Interp *interp, Jim_Obj *objPtr)
{
    jim_wide idx;
    int end = 0;
    const char *str;
    Jim_Obj *exprObj = objPtr;

    JimPanic((objPtr->refCount == 0, "SetIndexFromAny() called with zero refcount object"));

    /* Get the string representation */
    str = Jim_String(objPtr);

    /* Try to convert into an index */
    if (strncmp(str, "end", 3) == 0) {
        end = 1;
        str += 3;
        idx = 0;
        switch (*str) {
            case '\0':
                exprObj = NULL;
                break;

            case '-':
            case '+':
                /* Create a temp object here for evaluation, but this only happens
                 * once unless the index object shimmers since the result is kept
                 */
                exprObj = Jim_NewStringObj(interp, str, -1);
                break;

            default:
                goto badindex;
        }
    }
    if (exprObj) {
        int ret;
        Jim_IncrRefCount(exprObj);
        ret = Jim_GetWideExpr(interp, exprObj, &idx);
        Jim_DecrRefCount(interp, exprObj);
        if (ret != JIM_OK) {
            goto badindex;
        }
    }

    if (end) {
        if (idx > 0) {
            idx = INT_MAX;
        }
        else {
            /* end-1 is repesented as -2 */
            idx--;
        }
    }
    else if (idx < 0) {
        idx = -INT_MAX;
    }
    else if (idx > INT_MAX) {
        goto badindex;
    }

    /* Free the old internal repr and set the new one. */
    Jim_FreeIntRep(interp, objPtr);
    objPtr->typePtr = &indexObjType;
    objPtr->internalRep.intValue = idx;
    return JIM_OK;

  badindex:
    Jim_SetResultFormatted(interp,
        "bad index \"%#s\": must be intexpr or end?[+-]intexpr?", objPtr);
    return JIM_ERR;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_Utf8Length`, lines 2491–2503. Line unit: physical LF only. Original byte range [73003, 73369). Retained reference `source-window-45`.

```text
int Jim_Utf8Length(Jim_Interp *interp, Jim_Obj *objPtr)
{
#ifdef JIM_UTF8
    SetStringFromAny(interp, objPtr);

    if (objPtr->internalRep.strValue.charLength < 0) {
        objPtr->internalRep.strValue.charLength = utf8_strlen(objPtr->bytes, objPtr->length);
    }
    return objPtr->internalRep.strValue.charLength;
#else
    return Jim_Length(objPtr);
#endif
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_StringCoreCommand OPT_RANGE case`, lines 14962–14979. Line unit: physical LF only. Original byte range [473428, 474034). Retained reference `source-window-46`.

```text
        case OPT_RANGE:{
                Jim_Obj *objPtr = Jim_StringRangeObj(interp, argv[2], argv[3], argv[4]);
                if (objPtr == NULL) {
                    return JIM_ERR;
                }
                Jim_SetResult(interp, objPtr);
                return JIM_OK;
            }

        case OPT_BYTERANGE:{
                Jim_Obj *objPtr = Jim_StringByteRangeObj(interp, argv[2], argv[3], argv[4]);
                if (objPtr == NULL) {
                    return JIM_ERR;
                }
                Jim_SetResult(interp, objPtr);
                return JIM_OK;
            }

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_StringCoreCommand OPT_INDEX case`, lines 15029–15053. Line unit: physical LF only. Original byte range [475600, 476521). Retained reference `source-window-47`.

```text
        case OPT_INDEX:{
                int idx;
                const char *str;

                if (Jim_GetIndex(interp, argv[3], &idx) != JIM_OK) {
                    return JIM_ERR;
                }
                str = Jim_String(argv[2]);
                len = Jim_Utf8Length(interp, argv[2]);
                idx = JimRelToAbsIndex(len, idx);
                if (idx < 0 || idx >= len || str == NULL) {
                    Jim_SetResultString(interp, "", 0);
                }
                else if (len == Jim_Length(argv[2])) {
                    /* ASCII optimisation */
                    Jim_SetResultString(interp, str + idx, 1);
                }
                else {
                    int c;
                    int i = utf8_index(str, idx);
                    Jim_SetResultString(interp, str + i, utf8_tounicode(str + i, &c));
                }
                return JIM_OK;
            }

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/utf8.c`, function `utf8_strlen`, lines 63–77. Line unit: physical LF only. Original byte range [1330, 1628). Retained reference `source-window-48`.

```text
int utf8_strlen(const char *str, int bytelen)
{
    int charlen = 0;
    if (bytelen < 0) {
        bytelen = strlen(str);
    }
    while (bytelen > 0) {
        int c;
        int l = utf8_tounicode(str, &c);
        charlen++;
        str += l;
        bytelen -= l;
    }
    return charlen;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/utf8.c`, function `utf8_index`, lines 79–86. Line unit: physical LF only. Original byte range [1629, 1780). Retained reference `source-window-49`.

```text
int utf8_index(const char *str, int index)
{
    const char *s = str;
    while (index--) {
        s += utf8_charlen(*s);
    }
    return s - str;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/utf8.c`, function `utf8_tounicode`, lines 109–148. Line unit: physical LF only. Original byte range [2207, 3429). Retained reference `source-window-50`.

```text
int utf8_tounicode(const char *str, int *uc)
{
    unsigned const char *s = (unsigned const char *)str;

    if (s[0] < 0xc0) {
        *uc = s[0];
        return 1;
    }
    if (s[0] < 0xe0) {
        if ((s[1] & 0xc0) == 0x80) {
            *uc = ((s[0] & ~0xc0) << 6) | (s[1] & ~0x80);
            if (*uc >= 0x80) {
                return 2;
            }
            /* Otherwise this is an invalid sequence */
        }
    }
    else if (s[0] < 0xf0) {
        if (((str[1] & 0xc0) == 0x80) && ((str[2] & 0xc0) == 0x80)) {
            *uc = ((s[0] & ~0xe0) << 12) | ((s[1] & ~0x80) << 6) | (s[2] & ~0x80);
            if (*uc >= 0x800) {
                return 3;
            }
            /* Otherwise this is an invalid sequence */
        }
    }
    else if (s[0] < 0xf8) {
        if (((str[1] & 0xc0) == 0x80) && ((str[2] & 0xc0) == 0x80) && ((str[3] & 0xc0) == 0x80)) {
            *uc = ((s[0] & ~0xf0) << 18) | ((s[1] & ~0x80) << 12) | ((s[2] & ~0x80) << 6) | (s[3] & ~0x80);
            if (*uc >= 0x10000) {
                return 4;
            }
            /* Otherwise this is an invalid sequence */
        }
    }

    /* Invalid sequence, so just return the byte */
    *uc = *s;
    return 1;
}

```

## Consumer bindings

- [rust/tcl-registry/src/native_string_length.rs](../../../../rust/tcl-registry/src/native_string_length.rs), `actual_length_conversion_retains_release_specific_bytearray_rules` (linked): Software source definition checks actual length conversion retains release specific bytearray rules.
- [rust/tcl-vm/src/value_ops/native_jim_index.rs](../../../../rust/tcl-vm/src/value_ops/native_jim_index.rs), `original_index_and_range_project_exact_measured_public_predicates` (linked): Software source definition checks index and range project exact measured public predicates.
- [rust/tcl-vm/src/value_ops/native_jim_index.rs](../../../../rust/tcl-vm/src/value_ops/native_jim_index.rs), `original_c_count_retains_source_backed_eager_and_lazy_unicode_caches` (linked): Software source definition checks c count retains source backed eager and lazy unicode caches.
- [runtime/rust/src/value_ops/native_jim_index.rs](../../../../runtime/rust/src/value_ops/native_jim_index.rs), `original_index_and_range_project_exact_measured_public_predicates` (linked): Software source definition checks index and range project exact measured public predicates.
- [runtime/rust/src/value_ops/native_jim_index.rs](../../../../runtime/rust/src/value_ops/native_jim_index.rs), `original_c_binary_pointer_and_reread_stages_use_actual_header_and_no_protective_reference` (linked): Software source definition checks c binary pointer and reread stages use actual header and no protective reference.

## Evidence

- [runtime/rust/tests/data/native_original_index_range635](../../../../runtime/rust/tests/data/native_original_index_range635)
- [runtime/rust/tests/data/native_original_index_range635/8.4.20/same-library-version](../../../../runtime/rust/tests/data/native_original_index_range635/8.4.20/same-library-version)
- [runtime/rust/tests/data/native_original_index_range635/8.5.19/same-library-version](../../../../runtime/rust/tests/data/native_original_index_range635/8.5.19/same-library-version)
- [runtime/rust/tests/data/native_original_index_range635/8.6.18/same-library-version](../../../../runtime/rust/tests/data/native_original_index_range635/8.6.18/same-library-version)
- [runtime/rust/tests/data/native_original_index_range635/9.0.4/same-library-version](../../../../runtime/rust/tests/data/native_original_index_range635/9.0.4/same-library-version)
- [runtime/rust/tests/data/native_original_index_range635/9.1.0/same-library-version](../../../../runtime/rust/tests/data/native_original_index_range635/9.1.0/same-library-version)
- [runtime/rust/tests/data/native_original_index_range635/Jim/same-library-version](../../../../runtime/rust/tests/data/native_original_index_range635/Jim/same-library-version)
