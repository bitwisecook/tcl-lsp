# naming.namespace.original-counted-qualifiers-compiler-and-runtime

Kind: `native-observation`

## Problem statement

Direct namespace Qualifiers scans CString bytes while the C8.6+ compiler searches counted Unicode and loops over the preceding colon run. Display strings or direct-worker slicing lose raw00/FF and surrogate distinctions.

## Question

For twenty retained native counted inputs, how do direct and genuine procedure Qualifiers calls differ across C5 and Jim, including colon runs and opaque/raw00 boundaries?

## Conclusion

C8.6/9.0/9.1 compiles the exact last-find/decrement/index/equality-loop/range program over counted Unicode; raw00 and FF prefix results differ from direct CString. The two surrogate prefixes stay distinct. C strips a preceding colon run while Jim retains preceding colons. C84/85 and Jim are independent no-C86-compiler controls.

## Scope

Exactly twenty fixed counted String inputs, two actual object-vector call forms, six providers and three C8.6+ compiler programs. Fixed ASCII procedure source is separate from counted argv. Primary/identity observations precede counted byte getters. No general Unicode theorem, source-channel byte re-encoding, ByteArray/non-Unicode range branch, mutable command/namespace existence, alias/compiler attachment, frame/Normal/property/member validity or native capability is inferred. Pure Registry original-word recipe is separate from current selected compiler/implementation token and actual backend object authority. Rust comparisons are linked obligations without an attached passing execution receipt.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Counted native NewString object-vector argv, separate fixed ASCII proc source; result/input primary before counted result getters and exact hex streams. Version: tcl8.4 8.4.20.

Direct and procedure calls have the same CString results and strip preceding colon runs. Nonempty results are String-primary through Tcl_AppendToObj; empty results are NULL. Raw00 stops the direct scan; raw FF and D800/D801 prefix bytes stay distinct. No C8.6 Qualifiers compiler is claimed.

### tcl8.5

Status: `observed`. Channel: Counted native NewString object-vector argv, separate fixed ASCII proc source; result/input primary before counted result getters and exact hex streams. Version: tcl8.5 8.5.19.

Direct and procedure calls have the same CString results and strip preceding colon runs. Fresh byte results remain NULL-primary. Raw00 stops the scan; raw FF and D800/D801 prefix bytes stay distinct. No C8.6 Qualifiers compiler is claimed.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Counted native NewString object-vector argv, separate fixed ASCII proc source; result/input primary before counted result getters and exact hex streams. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The genuine procedure compiles last-find, decrement/index/colon-equality backward loop, then range. Compiled raw00 prefix becomes aC080 and FF prefix becomes C3BF through counted Unicode; direct raw00 returns empty and FF stays FF. D800/D801 prefixes remain distinct. C strips preceding colon runs. Nonempty compiled result and subject are String-primary; empty result is NULL. Direct fresh result stays NULL-primary.

### jim

Status: `observed`. Channel: Counted native NewString object-vector argv, separate fixed ASCII proc source; result/input primary before counted result getters and exact hex streams. Version: jim 0.84-9-g5bac7c9.

Direct and procedure calls have equal fresh NULL-primary results. Jim retains preceding colons: :::a returns one colon, :::: returns two and a::::b returns a::. Raw00 stops the qualifier scan. Raw FF and D800/D801 prefixes remain exact. The D row records no C Qualifiers compiler.

### bigip

Status: `not-tested`.

No appliance observation for this finite question.

## Source inspection

tcl8.4 8.4.20, revision `Retained Tcl 8.4.20 release source; complete file digest identifies exact bytes`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `NamespaceQualifiersCmd`, lines 3589–3627. Retained reference `source-window-0`.

```text
NamespaceQualifiersCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    register char *name, *p;
    int length;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 2, objv, "string");
        return TCL_ERROR;
    }

    /*
     * Find the end of the string, then work backward and find
     * the start of the last "::" qualifier.
     */

    name = Tcl_GetString(objv[2]);
    for (p = name;  *p != '\0';  p++) {
	/* empty body */
    }
    while (--p >= name) {
        if ((*p == ':') && (p > name) && (*(p-1) == ':')) {
	    p -= 2;		/* back up over the :: */
	    while ((p >= name) && (*p == ':')) {
		p--;		/* back up over the preceeding : */
	    }
	    break;
        }
    }

    if (p >= name) {
        length = p-name+1;
        Tcl_AppendToObj(Tcl_GetObjResult(interp), name, length);
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `Retained Tcl 8.5.19 release source; complete file digest identifies exact bytes`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceQualifiersCmd`, lines 4137–4175. Retained reference `source-window-1`.

```text
NamespaceQualifiersCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    register char *name, *p;
    int length;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 2, objv, "string");
	return TCL_ERROR;
    }

    /*
     * Find the end of the string, then work backward and find the start of
     * the last "::" qualifier.
     */

    name = TclGetString(objv[2]);
    for (p = name;  *p != '\0';  p++) {
	/* empty body */
    }
    while (--p >= name) {
	if ((*p == ':') && (p > name) && (*(p-1) == ':')) {
	    p -= 2;			/* Back up over the :: */
	    while ((p >= name) && (*p == ':')) {
		p--;			/* Back up over the preceeding : */
	    }
	    break;
	}
    }

    if (p >= name) {
	length = p-name+1;
	Tcl_SetObjResult(interp, Tcl_NewStringObj(name, length));
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; complete file digest identifies exact bytes`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `NamespaceQualifiersCmd`, lines 4165–4203. Retained reference `source-window-2`.

```text
NamespaceQualifiersCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *name, *p;
    int length;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "string");
	return TCL_ERROR;
    }

    /*
     * Find the end of the string, then work backward and find the start of
     * the last "::" qualifier.
     */

    name = TclGetString(objv[1]);
    for (p = name;  p[0] != '\0';  p++) {
	/* empty body */
    }
    while (--p >= name) {
	if ((p[0] == ':') && (p > name) && (p[-1] == ':')) {
	    p -= 2;			/* Back up over the :: */
	    while ((p >= name) && (p[0] == ':')) {
		p--;			/* Back up over the preceding : */
	    }
	    break;
	}
    }

    if (p >= name) {
	length = p-name+1;
	Tcl_SetObjResult(interp, Tcl_NewStringObj(name, length));
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; complete file digest identifies exact bytes`, `tmp/tcl8.6.18/generic/tclCompCmdsGR.c`, function `TclCompileNamespaceQualifiersCmd`, lines 1882–1915. Retained reference `source-window-3`.

```text
TclCompileNamespaceQualifiersCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);
    int off;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    CompileWord(envPtr, tokenPtr, interp, 1);
    PushStringLiteral(envPtr, "0");
    PushStringLiteral(envPtr, "::");
    TclEmitInstInt4(	INST_OVER, 2,			envPtr);
    TclEmitOpcode(	INST_STR_FIND_LAST,		envPtr);
    off = CurrentOffset(envPtr);
    PushStringLiteral(envPtr, "1");
    TclEmitOpcode(	INST_SUB,			envPtr);
    TclEmitInstInt4(	INST_OVER, 2,			envPtr);
    TclEmitInstInt4(	INST_OVER, 1,			envPtr);
    TclEmitOpcode(	INST_STR_INDEX,			envPtr);
    PushStringLiteral(envPtr, ":");
    TclEmitOpcode(	INST_STR_EQ,			envPtr);
    off = off - CurrentOffset(envPtr);
    TclEmitInstInt1(	INST_JUMP_TRUE1, off,		envPtr);
    TclEmitOpcode(	INST_STR_RANGE,			envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; complete file digest identifies exact bytes`, `tmp/tcl8.6.18/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_INDEX case)`, lines 5561–5594. Retained reference `source-window-4`.

```text
    case INST_STR_INDEX:
	value2Ptr = OBJ_AT_TOS;
	valuePtr = OBJ_UNDER_TOS;
	TRACE(("\"%.20s\" %.20s => ", O2S(valuePtr), O2S(value2Ptr)));

	/*
	 * Get char length to calculate what 'end' means.
	 */

	length = Tcl_GetCharLength(valuePtr);
	if (TclGetIntForIndexM(interp, value2Ptr, length-1, &index)!=TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}

	if ((index < 0) || (index >= length)) {
	    TclNewObj(objResultPtr);
	} else if (TclIsPureByteArray(valuePtr)) {
	    objResultPtr = Tcl_NewByteArrayObj(
		    Tcl_GetByteArrayFromObj(valuePtr, NULL)+index, 1);
	} else if (valuePtr->bytes && length == valuePtr->length) {
	    objResultPtr = Tcl_NewStringObj((const char *)
		    valuePtr->bytes+index, 1);
	} else {
	    char buf[8] = "";
	    int ch = TclGetUCS4(valuePtr, index);

	    length = TclUCS4ToUtf(ch, buf);
	    objResultPtr = Tcl_NewStringObj(buf, length);
	}

	TRACE_APPEND(("\"%s\"\n", O2S(objResultPtr)));
	NEXT_INST_F(1, 2, 1);

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `NamespaceQualifiersCmd`, lines 4360–4398. Retained reference `source-window-5`.

```text
NamespaceQualifiersCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    const char *name, *p;
    size_t length;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "string");
	return TCL_ERROR;
    }

    /*
     * Find the end of the string, then work backward and find the start of
     * the last "::" qualifier.
     */

    name = TclGetString(objv[1]);
    for (p = name;  p[0] != '\0';  p++) {
	/* empty body */
    }
    while (--p >= name) {
	if ((p[0] == ':') && (p > name) && (p[-1] == ':')) {
	    p -= 2;			/* Back up over the :: */
	    while ((p >= name) && (p[0] == ':')) {
		p--;			/* Back up over the preceding : */
	    }
	    break;
	}
    }

    if (p >= name) {
	length = p-name+1;
	Tcl_SetObjResult(interp, Tcl_NewStringObj(name, length));
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclCompCmdsGR.c`, function `TclCompileNamespaceQualifiersCmd`, lines 1724–1756. Retained reference `source-window-6`.

```text
TclCompileNamespaceQualifiersCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);
    int off;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    CompileWord(envPtr, tokenPtr, interp, 1);
    PushStringLiteral(envPtr, "0");
    PushStringLiteral(envPtr, "::");
    TclEmitInstInt4(	INST_OVER, 2,			envPtr);
    TclEmitOpcode(	INST_STR_FIND_LAST,		envPtr);
    off = CurrentOffset(envPtr);
    PushStringLiteral(envPtr, "1");
    TclEmitOpcode(	INST_SUB,			envPtr);
    TclEmitInstInt4(	INST_OVER, 2,			envPtr);
    TclEmitInstInt4(	INST_OVER, 1,			envPtr);
    TclEmitOpcode(	INST_STR_INDEX,			envPtr);
    PushStringLiteral(envPtr, ":");
    TclEmitOpcode(	INST_STR_EQ,			envPtr);
    off = off - CurrentOffset(envPtr);
    TclEmitInstInt1(	INST_JUMP_TRUE1, off,		envPtr);
    TclEmitOpcode(	INST_STR_RANGE,			envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_INDEX case)`, lines 5336–5381. Retained reference `source-window-7`.

```text
    case INST_STR_INDEX:
	value2Ptr = OBJ_AT_TOS;
	valuePtr = OBJ_UNDER_TOS;
	TRACE(("\"%.20s\" %.20s => ", O2S(valuePtr), O2S(value2Ptr)));

	/*
	 * Get char length to calculate what 'end' means.
	 */

	slength = Tcl_GetCharLength(valuePtr);
	DECACHE_STACK_INFO();
	if (TclGetIntForIndexM(interp, value2Ptr, slength-1, &index)!=TCL_OK) {
	    CACHE_STACK_INFO();
	    TRACE_ERROR(interp);
	    goto gotError;
	}
	CACHE_STACK_INFO();

	if (index < 0 || index >= slength) {
	    TclNewObj(objResultPtr);
	} else if (TclIsPureByteArray(valuePtr)) {
	    objResultPtr = Tcl_NewByteArrayObj(
		    Tcl_GetBytesFromObj(NULL, valuePtr, (Tcl_Size *)NULL)+index, 1);
	} else if (valuePtr->bytes && slength == valuePtr->length) {
	    objResultPtr = Tcl_NewStringObj((const char *)
		    valuePtr->bytes+index, 1);
	} else {
	    char buf[4] = "";
	    int ch = Tcl_GetUniChar(valuePtr, index);

	    /*
	     * This could be: Tcl_NewUnicodeObj((const Tcl_UniChar *)&ch, 1)
	     * but creating the object as a string seems to be faster in
	     * practical use.
	     */
	    if (ch == -1) {
		TclNewObj(objResultPtr);
	    } else {
		slength = Tcl_UniCharToUtf(ch, buf);
		objResultPtr = Tcl_NewStringObj(buf, slength);
	    }
	}

	TRACE_APPEND(("\"%s\"\n", O2S(objResultPtr)));
	NEXT_INST_F(1, 2, 1);

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclStringObj.c`, function `TclStringLast`, lines 3948–4014. Retained reference `source-window-8`.

```text
TclStringLast(
    Tcl_Obj *needle,
    Tcl_Obj *haystack,
    Tcl_Size last)
{
    Tcl_Size lh = 0, ln = Tcl_GetCharLength(needle);
    Tcl_Size value = -1;
    Tcl_UniChar *checkStr, *uh, *un;
    Tcl_Obj *obj;

    if (ln == 0) {
	/*
	 *	We don't find empty substrings.  Bizarre!
	 *
	 *	TODO: When we one day make this a true substring
	 *	finder, change this to "return last", after limitation.
	 */
	goto lastEnd;
    }

    if (TclIsPureByteArray(needle) && TclIsPureByteArray(haystack)) {
	unsigned char *check, *bh = Tcl_GetBytesFromObj(NULL, haystack, &lh);
	unsigned char *bn = Tcl_GetBytesFromObj(NULL, needle, &ln);

	if (last >= lh) {
	    last = lh - 1;
	}
	if (last + 1 < ln) {
	    /* Don't start the loop if there cannot be a valid answer */
	    goto lastEnd;
	}
	check = bh + last + 1 - ln;

	while (check >= bh) {
	    if ((*check == bn[0])
		    && (0 == memcmp(check+1, bn+1, ln-1))) {
		value = (check - bh);
		goto lastEnd;
	    }
	    check--;
	}
	goto lastEnd;
    }

    uh = Tcl_GetUnicodeFromObj(haystack, &lh);
    un = Tcl_GetUnicodeFromObj(needle, &ln);

    if (last >= lh) {
	last = lh - 1;
    }
    if (last + 1 < ln) {
	/* Don't start the loop if there cannot be a valid answer */
	goto lastEnd;
    }
    checkStr = uh + last + 1 - ln;
    while (checkStr >= uh) {
	if ((*checkStr == un[0])
		&& (0 == memcmp(checkStr+1, un+1, (ln-1)*sizeof(Tcl_UniChar)))) {
	    value = (checkStr - uh);
	    goto lastEnd;
	}
	checkStr--;
    }
  lastEnd:
    TclNewIndexObj(obj, value);
    return obj;
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `NamespaceQualifiersCmd`, lines 4333–4371. Retained reference `source-window-9`.

```text
NamespaceQualifiersCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    const char *name, *p;
    size_t length;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "string");
	return TCL_ERROR;
    }

    /*
     * Find the end of the string, then work backward and find the start of
     * the last "::" qualifier.
     */

    name = TclGetString(objv[1]);
    for (p = name;  p[0] != '\0';  p++) {
	/* empty body */
    }
    while (--p >= name) {
	if ((p[0] == ':') && (p > name) && (p[-1] == ':')) {
	    p -= 2;			/* Back up over the :: */
	    while ((p >= name) && (p[0] == ':')) {
		p--;			/* Back up over the preceding : */
	    }
	    break;
	}
    }

    if (p >= name) {
	length = p-name+1;
	Tcl_SetObjResult(interp, Tcl_NewStringObj(name, length));
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclCompCmdsGR.c`, function `TclCompileNamespaceQualifiersCmd`, lines 2198–2229. Retained reference `source-window-10`.

```text
TclCompileNamespaceQualifiersCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);
    Tcl_BytecodeLabel off;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    PUSH_TOKEN(			tokenPtr, 1);
    PUSH(			"0");
    PUSH(			"::");
    OP4(			OVER, 2);
    OP(				STR_FIND_LAST);
    BACKLABEL(		off);
    PUSH(			"1");
    OP(				SUB);
    OP4(			OVER, 2);
    OP4(			OVER, 1);
    OP(				STR_INDEX);
    PUSH(			":");
    OP(				STR_EQ);
    BACKJUMP(			JUMP_TRUE, off);
    OP(				STR_RANGE);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_INDEX case)`, lines 5672–5719. Retained reference `source-window-11`.

```text
    case INST_STR_INDEX:
	value2Ptr = OBJ_AT_TOS;
	valuePtr = OBJ_UNDER_TOS;
	TRACE("\"%.30s\" %.20s => ", O2S(valuePtr), O2S(value2Ptr));

	/*
	 * Get char length to calculate what 'end' means.
	 */

	slength = Tcl_GetCharLength(valuePtr);
	{
	    DECACHE_STACK_INFO();
	    int code = TclGetIntForIndexM(interp, value2Ptr, slength - 1, &index);
	    CACHE_STACK_INFO();
	    if (code != TCL_OK) {
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}

	if (index < 0 || index >= slength) {
	    TclNewObj(objResultPtr);
	} else if (TclIsPureByteArray(valuePtr)) {
	    objResultPtr = Tcl_NewByteArrayObj(
		    Tcl_GetBytesFromObj(NULL, valuePtr, (Tcl_Size *)NULL)+index, 1);
	} else if (valuePtr->bytes && slength == valuePtr->length) {
	    objResultPtr = Tcl_NewStringObj((const char *)
		    valuePtr->bytes + index, 1);
	} else {
	    char buf[4] = "";
	    int ch = Tcl_GetUniChar(valuePtr, index);

	    /*
	     * This could be: Tcl_NewUnicodeObj((const Tcl_UniChar *)&ch, 1)
	     * but creating the object as a string seems to be faster in
	     * practical use.
	     */
	    if (ch == -1) {
		TclNewObj(objResultPtr);
	    } else {
		slength = Tcl_UniCharToUtf(ch, buf);
		objResultPtr = Tcl_NewStringObj(buf, slength);
	    }
	}

	TRACE_APPEND_OBJ(objResultPtr);
	NEXT_INST_F(1, 2, 1);

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclStringObj.c`, function `TclStringLast`, lines 3956–4022. Retained reference `source-window-12`.

```text
TclStringLast(
    Tcl_Obj *needle,
    Tcl_Obj *haystack,
    Tcl_Size last)
{
    Tcl_Size lh = 0, ln = Tcl_GetCharLength(needle);
    Tcl_Size value = -1;
    Tcl_UniChar *checkStr, *uh, *un;
    Tcl_Obj *obj;

    if (ln == 0) {
	/*
	 *	We don't find empty substrings.  Bizarre!
	 *
	 *	TODO: When we one day make this a true substring
	 *	finder, change this to "return last", after limitation.
	 */
	goto lastEnd;
    }

    if (TclIsPureByteArray(needle) && TclIsPureByteArray(haystack)) {
	unsigned char *check, *bh = Tcl_GetBytesFromObj(NULL, haystack, &lh);
	unsigned char *bn = Tcl_GetBytesFromObj(NULL, needle, &ln);

	if (last >= lh) {
	    last = lh - 1;
	}
	if (last + 1 < ln) {
	    /* Don't start the loop if there cannot be a valid answer */
	    goto lastEnd;
	}
	check = bh + last + 1 - ln;

	while (check >= bh) {
	    if ((*check == bn[0])
		    && (0 == memcmp(check+1, bn+1, ln-1))) {
		value = (check - bh);
		goto lastEnd;
	    }
	    check--;
	}
	goto lastEnd;
    }

    uh = Tcl_GetUnicodeFromObj(haystack, &lh);
    un = Tcl_GetUnicodeFromObj(needle, &ln);

    if (last >= lh) {
	last = lh - 1;
    }
    if (last + 1 < ln) {
	/* Don't start the loop if there cannot be a valid answer */
	goto lastEnd;
    }
    checkStr = uh + last + 1 - ln;
    while (checkStr >= uh) {
	if ((*checkStr == un[0])
		&& (0 == memcmp(checkStr+1, un+1, (ln-1)*sizeof(Tcl_UniChar)))) {
	    value = (checkStr - uh);
	    goto lastEnd;
	}
	checkStr--;
    }
  lastEnd:
    TclNewIndexObj(obj, value);
    return obj;
}

```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; complete file digest identifies exact bytes`, `tmp/tcl8.6.18/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_FIND_LAST case)`, lines 5902–5922. Retained reference `source-window-13`.

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

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; complete file digest identifies exact bytes`, `tmp/tcl8.6.18/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_RANGE case)`, lines 5595–5620. Retained reference `source-window-14`.

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

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; complete file digest identifies exact bytes`, `tmp/tcl8.6.18/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 741–829. Retained reference `source-window-15`.

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

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_FIND_LAST case)`, lines 5562–5568. Retained reference `source-window-16`.

```text
    case INST_STR_FIND_LAST:
	objResultPtr = TclStringLast(OBJ_UNDER_TOS, OBJ_AT_TOS, TCL_SIZE_MAX - 1);

	TRACE(("%.20s %.20s => %s\n",
		O2S(OBJ_UNDER_TOS), O2S(OBJ_AT_TOS), O2S(objResultPtr)));
	NEXT_INST_F(1, 2, 1);

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_RANGE case)`, lines 5382–5407. Retained reference `source-window-17`.

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

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; complete file digest identifies exact bytes`, `tmp/tcl9.0.4/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 725–802. Retained reference `source-window-18`.

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

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_FIND_LAST case)`, lines 5896–5901. Retained reference `source-window-19`.

```text
    case INST_STR_FIND_LAST:
	TRACE("%.20s %.20s => ", O2S(OBJ_UNDER_TOS), O2S(OBJ_AT_TOS));
	objResultPtr = TclStringLast(OBJ_UNDER_TOS, OBJ_AT_TOS, TCL_SIZE_MAX - 1);
	TRACE_APPEND_NUM_OBJ(objResultPtr);
	NEXT_INST_F(1, 2, 1);

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclExecute.c`, function `TclExecuteByteCode (INST_STR_RANGE case)`, lines 5720–5741. Retained reference `source-window-20`.

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

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; complete file digest identifies exact bytes`, `tmp/tcl9.1.0/generic/tclStringObj.c`, function `Tcl_GetRange`, lines 728–805. Retained reference `source-window-21`.

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

## Consumer bindings

- [rust/tcl-registry/src/native_namespace_string_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_string_compilation.rs), `compile_native_namespace_string`: Same complete original one-operand geometry for separately selected Tail and Qualifiers recipes; no compiler admission.
- [rust/tcl-cmd-core/src/string/native_compiled.rs](../../../../rust/tcl-cmd-core/src/string/native_compiled.rs), `compiled_find`: Shared actual counted Unicode search/getter order and last-match coordinate.
- [rust/tcl-cmd-core/src/string/native_compiled.rs](../../../../rust/tcl-cmd-core/src/string/native_compiled.rs), `compiled_index`: Prepared original String index byte shortcut and shared selected native unit UTF encoder; no alternate decoder.
- [rust/tcl-cmd-core/src/string/native_compiled.rs](../../../../rust/tcl-cmd-core/src/string/native_compiled.rs), `compiled_qualifiers`: Selected original counted previous-colon predicate and shared reached prefix-range construction.
- [rust/tcl-cmd-core/src/string/native_compiled.rs](../../../../rust/tcl-cmd-core/src/string/native_compiled.rs), `compiled_range`: Same reached Unicode range constructor, independent actual backend protocol.
- [rust/tcl-compiler/src/codegen/statements/native_namespace_string.rs](../../../../rust/tcl-compiler/src/codegen/statements/native_namespace_string.rs), `append_native_namespace_string_tasks`: Emit exact selected original Qualifiers stack/loop/version stamps.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::namespace_text_result`: Separate direct C84 append, later counted byte object, Jim unchanged Tail/result actions.
- [rust/tcl-registry/src/native_namespace_string_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_string_compilation.rs), `native_namespace_string_compilation::tests::original_namespace_qualifiers_recipe_retains_dynamic_and_counted_operands` (linked): Retain original dynamic/raw00/surrogate source operands and actual private registration; C84/85 NoHook stays Generic.
- [rust/tcl-registry/src/native_namespace_string_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_string_compilation.rs), `native_namespace_string_compilation::tests::original_namespace_qualifiers_declines_arity_and_unresolved_expansion` (linked): Incorrect complete argv arity and unresolved expansion decline original one-operand recipe.
- [rust/tcl-vm/src/exec/native_namespace_string_tests.rs](../../../../rust/tcl-vm/src/exec/native_namespace_string_tests.rs), `exec::native_namespace_string_tests::original_namespace_qualifiers_matches_two_hundred_forty_counted_native_windows` (linked): Compare all C5 two-hundred windows and separate Jim forty controls: result code/primary/identity/count/bytes and original descriptor/bytes.
- [runtime/rust/src/interp/native_body_artifact/native_namespace_string.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_namespace_string.rs), `interp::native_body_artifact::native_namespace_string::tests::original_namespace_qualifiers_matches_two_hundred_forty_counted_native_windows` (linked): Runtime compares exact captured C5 and Jim direct/procedure results and original headers/bytes; no C Jim compiler inferred.
- [rust/tcl-vm/src/exec/native_namespace_string_tests.rs](../../../../rust/tcl-vm/src/exec/native_namespace_string_tests.rs), `exec::native_namespace_string_tests::original_compiled_index_preserves_counted_single_bytes_and_native_units` (linked): Prepared original index returns fresh byte-primary FF/raw00/D800/D801; foreign protocol or missing reached Unicode backing refuses.
- [rust/tcl-compiler/src/codegen/statements/native_namespace_string_tests.rs](../../../../rust/tcl-compiler/src/codegen/statements/native_namespace_string_tests.rs), `codegen::statements::native_namespace_string::tests::original_namespace_qualifiers_emits_the_observed_counted_compiler_loop` (linked): The selected C8.6/9.0/9.1 emitter retains the captured last-find, decrement/index, colon predicate, backward loop and reached range sequence; C84/85 original NoHook selection emits its generic call instead. This is emitted-program model coverage, with no executed backend or native compiler grant.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.4.20](../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.4.20)
- [rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.5.19](../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.5.19)
- [rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.6.18](../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.6.18)
- [rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/9.0.4](../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/9.0.4)
- [rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/9.1.0](../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/9.1.0)
- [rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/jim](../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/jim)
