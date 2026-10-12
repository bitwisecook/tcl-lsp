# naming.substitution.compiler-source-purpose

Kind: `source-anchor`

## Problem statement

An empirical substcode descriptor alone does not explain which compiler inputs govern its currency or whether it owns a procedure header. The native implementation must be inspected independently, including direct older implementations and the modern original-template compiler, so a runtime API does not borrow script or procedure semantics.

## Question

Which actual source routine selects direct versus compiled C substitution, and which independent inputs does the compiled original template retain and revalidate?

## Conclusion

Inspected C8.4/C8.5 Tcl_SubstObj appends into a result object and does not use CompileSubstObj. C8.6/C9.0/C9.1 CompileSubstObj independently validates flags, interpreter, compileEpoch, actual varFrame namespace pointer/resolver epoch and localCachePtr; it initialises a compile environment with procPtr NULL, emits substitution code plus DONE, installs substcode and increments the borrowed localCache reference. These source selections explain a template-only cache purpose and do not authenticate caller frames, arbitrary objects or native source-header grants.

## Scope

Pinned retained function windows and complete source hashes for five exact C releases. Source inspection is distinct from native output and implementation tests; Jim and BIG-IP source recipes are not covered.

## Provider answers

### tcl8.4

Status: `inspected`. Version: 8.4.20. Build: Pinned retained source-file and exact function-window digests; this answer comes from source inspection, independently of the executed probes.. Channel: Inspection of the retained pinned native C function window; no guest input is executed for this source-only answer.. Dialect: Tcl.

Inspected Tcl_SubstObj in generic/tclCmdMZ.c. Direct incremental result-object append.

### tcl8.5

Status: `inspected`. Version: 8.5.19. Build: Pinned retained source-file and exact function-window digests; this answer comes from source inspection, independently of the executed probes.. Channel: Inspection of the retained pinned native C function window; no guest input is executed for this source-only answer.. Dialect: Tcl.

Inspected Tcl_SubstObj in generic/tclParse.c. Direct incremental result-object append.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Pinned retained source-file and exact function-window digests; this answer comes from source inspection, independently of the executed probes.. Channel: Inspection of the retained pinned native C function window; no guest input is executed for this source-only answer.. Dialect: Tcl.

Inspected CompileSubstObj in generic/tclCompile.c. Independent substcode currency and retained localCache; procPtr NULL.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Pinned retained source-file and exact function-window digests; this answer comes from source inspection, independently of the executed probes.. Channel: Inspection of the retained pinned native C function window; no guest input is executed for this source-only answer.. Dialect: Tcl.

Inspected CompileSubstObj in generic/tclCompile.c. Independent substcode currency and retained localCache; procPtr NULL.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Pinned retained source-file and exact function-window digests; this answer comes from source inspection, independently of the executed probes.. Channel: Inspection of the retained pinned native C function window; no guest input is executed for this source-only answer.. Dialect: Tcl.

Inspected CompileSubstObj in generic/tclCompile.c. Independent substcode currency and retained localCache; procPtr NULL.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This exact C substitution/private-header question was not tested on this provider.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This exact C substitution/private-header question was not tested on this provider.

## Exact evidence

- `e0` (input): [rust/tcl-registry/tests/data/native_substitution_owner/inputs.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/inputs.json). SHA-256 `364bf581c1b5d6b4e6b2b321ae4005d553f7c84fe120ca17c0c61e443022c307`. Complete ten native template byte arrays as hexadecimal. v1 shortened counts are separately encoded in its original probe.
- `e1` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v1/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1/probe.c). SHA-256 `b633702ddc869247bc9b00acb1f6068a72bf2b36066c21595d5e879a72f6bfc4`. Exact immutable v1 translation unit and original counted template/setup operations.
- `e2` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v2/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2/probe.c). SHA-256 `eb0eb2cc580340b982745d616d9d9db9c6fc36bb4c0171c2d275f0fe9c1bdd9c`. Exact immutable v2 translation unit and original counted template/setup operations.
- `e3` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v3/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3/probe.c). SHA-256 `7d7cedcd46b03c3634d8c974e9557a21333a2424c75a25aaec3603b5feac3538`. Exact immutable v3 translation unit and original counted template/setup operations.
- `e4` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v4/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4/probe.c). SHA-256 `9a0dd266956f6bbd7a283d95b8942681642135dd5e8b1edbdc5a3828ac562b8b`. Exact immutable v4 translation unit and original counted template/setup operations.
- `e5` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v5/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5/probe.c). SHA-256 `f42aa9712f61038336f07e84b6b4182a8e0e4bde03621457f0c1e742d605139a`. Exact immutable v5 translation unit and original counted template/setup operations.
- `e6` (input): [rust/tcl-registry/tests/data/native_substitution_owner/v6/probe.c](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6/probe.c). SHA-256 `1d19fec43fcd1f3de7809dcce3f9fead2dbac32de27c3711a01dcd9aefb08f56`. Exact immutable v6 translation unit and original counted template/setup operations.
- `e7` (provider): [rust/tcl-registry/tests/data/native_substitution_owner/manifest.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/manifest.json). SHA-256 `762b4287fd6746d901efbfb000d1bc4e50012af00d0260ca9364e395f88377f4`. Finite corpus provider mapping, variant input/setup/process limits and source excerpts.
- `e8` (source-anchor): [rust/tcl-registry/tests/data/native_substitution_owner/source-anchors/8.4.20.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/source-anchors/8.4.20.json). SHA-256 `9aac3b4133f72a9c85c20ab494a86c35a85e56631126d0c9d95464a8080961cf`. JSON pointer `/snippet`. Actual retained function text from exact configured native release.
- `e9` (source-anchor): [rust/tcl-registry/tests/data/native_substitution_owner/source-anchors/8.5.19.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/source-anchors/8.5.19.json). SHA-256 `ba1772af2cf3f9ad9416853099aceee4669f53023c5f040a8b898ac089255266`. JSON pointer `/snippet`. Actual retained function text from exact configured native release.
- `e10` (source-anchor): [rust/tcl-registry/tests/data/native_substitution_owner/source-anchors/8.6.18.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/source-anchors/8.6.18.json). SHA-256 `1a31bc570e4965fb2cc150a6337ab2c88e04fb02f8e20663a3aded0d9c88e009`. JSON pointer `/snippet`. Actual retained function text from exact configured native release.
- `e11` (source-anchor): [rust/tcl-registry/tests/data/native_substitution_owner/source-anchors/9.0.4.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/source-anchors/9.0.4.json). SHA-256 `f5ab0243e1ecfd6e11fe6434d5f6f5792f574c7394643955d8fb5caf38f69b0b`. JSON pointer `/snippet`. Actual retained function text from exact configured native release.
- `e12` (source-anchor): [rust/tcl-registry/tests/data/native_substitution_owner/source-anchors/9.1.0.json](../../../../rust/tcl-registry/tests/data/native_substitution_owner/source-anchors/9.1.0.json). SHA-256 `5ae5ddbaf48a6ef7bea8d63ad9b90937ff27c362e681ad72f95739d44ce9fbe0`. JSON pointer `/snippet`. Actual retained function text from exact configured native release.

## Source inspection

tcl8.4 8.4.20, revision `Pinned Tcl release 8.4.20; https://github.com/tcltk/tcl/blob/core-8-4-20/generic/tclCmdMZ.c`, `generic/tclCmdMZ.c`, function `Tcl_SubstObj`, lines 2577–2704. Full-source SHA-256 `b374cf4353b86235b189d432239a1eedf8e854eab266b701e9d9625b969d27dc`; snippet SHA-256 `fab16539af8a6f8f908eb092de556c3351bca4562740b134ac59122d642ed122`; retained evidence `e8`.

```text

Tcl_SubstObj(interp, objPtr, flags)
    Tcl_Interp *interp;
    Tcl_Obj *objPtr;
    int flags;
{
    Tcl_Obj *resultObj;
    char *p, *old;
    int length;

    old = p = Tcl_GetStringFromObj(objPtr, &length);
    resultObj = Tcl_NewStringObj("", 0);
    while (length) {
	switch (*p) {
	case '\\':
	    if (flags & TCL_SUBST_BACKSLASHES) {
		char buf[TCL_UTF_MAX];
		int count;

		if (p != old) {
		    Tcl_AppendToObj(resultObj, old, p-old);
		}
		Tcl_AppendToObj(resultObj, buf,
				TclParseBackslash(p, length, &count, buf));
		p += count; length -= count;
		old = p;
	    } else {
		p++; length--;
	    }
	    break;

	case '$':
	    if (flags & TCL_SUBST_VARIABLES) {
		Tcl_Parse parse;
		int code;

		/*
		 * Code is simpler overall if we (effectively) inline
		 * Tcl_ParseVar, particularly as that allows us to use
		 * a non-string interface when we come to appending
		 * the variable contents to the result object.  There
		 * are a few other optimisations that doing this
		 * enables (like being able to continue the run of
		 * unsubstituted characters straight through if a '$'
		 * does not precede a variable name.)
		 */
		if (Tcl_ParseVarName(interp, p, length, &parse, 0) != TCL_OK) {
		    goto errorResult;
		}
		if (parse.numTokens == 1) {
		    /*
		     * There isn't a variable name after all: the $ is
		     * just a $.
		     */
		    p++; length--;
		    break;
		}
		if (p != old) {
		    Tcl_AppendToObj(resultObj, old, p-old);
		}
		p += parse.tokenPtr->size;
		length -= parse.tokenPtr->size;
		code = Tcl_EvalTokensStandard(interp, parse.tokenPtr,
		        parse.numTokens);
		if (code == TCL_ERROR) {
		    goto errorResult;
		}
		if (code == TCL_BREAK) {
		    Tcl_ResetResult(interp);
		    return resultObj;
		}
		if (code != TCL_CONTINUE) {
		    Tcl_AppendObjToObj(resultObj, Tcl_GetObjResult(interp));
		}
		Tcl_ResetResult(interp);
		old = p;
	    } else {
		p++; length--;
	    }
	    break;

	case '[':
	    if (flags & TCL_SUBST_COMMANDS) {
		Interp *iPtr = (Interp *) interp;
		int code;

		if (p != old) {
		    Tcl_AppendToObj(resultObj, old, p-old);
		}
		iPtr->evalFlags = TCL_BRACKET_TERM;
		iPtr->numLevels++;
		code = TclInterpReady(interp);
		if (code == TCL_OK) {
		    code = Tcl_EvalEx(interp, p+1, length-1, 0);
		}
		iPtr->numLevels--;
		switch (code) {
		case TCL_ERROR:
		    goto errorResult;
		case TCL_BREAK:
		    Tcl_ResetResult(interp);
		    return resultObj;
		default:
		    Tcl_AppendObjToObj(resultObj, Tcl_GetObjResult(interp));
		case TCL_CONTINUE:
		    Tcl_ResetResult(interp);
		    old = p = (p+1 + iPtr->termOffset + 1);
		    length -= (iPtr->termOffset + 2);
		}
	    } else {
		p++; length--;
	    }
	    break;
	default:
	    p++; length--;
	    break;
	}
    }
    if (p != old) {
	Tcl_AppendToObj(resultObj, old, p-old);
    }
    return resultObj;

 errorResult:
    Tcl_DecrRefCount(resultObj);
    return NULL;
}

```

tcl8.5 8.5.19, revision `Pinned Tcl release 8.5.19; https://github.com/tcltk/tcl/blob/core-8-5-19/generic/tclParse.c`, `generic/tclParse.c`, function `Tcl_SubstObj`, lines 1925–2155. Full-source SHA-256 `4855b382f7acc6e02e53383f667169a6a9d348a30b44c586c8622e97f34a3a8c`; snippet SHA-256 `48efdcd6f516f2f9489d9569601bb55abea122a26605877f83fbfb720a275aa7`; retained evidence `e9`.

```text

Tcl_SubstObj(
    Tcl_Interp *interp,		/* Interpreter in which substitution occurs */
    Tcl_Obj *objPtr,		/* The value to be substituted. */
    int flags)			/* What substitutions to do. */
{
    int length, tokensLeft, code;
    Tcl_Token *endTokenPtr;
    Tcl_Obj *result, *errMsg = NULL;
    const char *p = TclGetStringFromObj(objPtr, &length);
    Tcl_Parse *parsePtr = (Tcl_Parse *)
	    TclStackAlloc(interp, sizeof(Tcl_Parse));

    TclParseInit(interp, p, length, parsePtr);

    /*
     * First parse the string rep of objPtr, as if it were enclosed as a
     * "-quoted word in a normal Tcl command. Honor flags that selectively
     * inhibit types of substitution.
     */

    if (TCL_OK != ParseTokens(p, length, /* mask */ 0, flags, parsePtr)) {
	/*
	 * There was a parse error. Save the error message for possible
	 * reporting later.
	 */

	errMsg = Tcl_GetObjResult(interp);
	Tcl_IncrRefCount(errMsg);

	/*
	 * We need to re-parse to get the portion of the string we can [subst]
	 * before the parse error. Sadly, all the Tcl_Token's created by the
	 * first parse attempt are gone, freed according to the public spec
	 * for the Tcl_Parse* routines. The only clue we have is parse.term,
	 * which points to either the unmatched opener, or to characters that
	 * follow a close brace or close quote.
	 *
	 * Call ParseTokens again, working on the string up to parse.term.
	 * Keep repeating until we get a good parse on a prefix.
	 */

	do {
	    parsePtr->numTokens = 0;
	    parsePtr->tokensAvailable = NUM_STATIC_TOKENS;
	    parsePtr->end = parsePtr->term;
	    parsePtr->incomplete = 0;
	    parsePtr->errorType = TCL_PARSE_SUCCESS;
	} while (TCL_OK !=
		ParseTokens(p, parsePtr->end - p, 0, flags, parsePtr));

	/*
	 * The good parse will have to be followed by {, (, or [.
	 */

	switch (*(parsePtr->term)) {
	case '{':
	    /*
	     * Parse error was a missing } in a ${varname} variable
	     * substitution at the toplevel. We will subst everything up to
	     * that broken variable substitution before reporting the parse
	     * error. Substituting the leftover '$' will have no side-effects,
	     * so the current token stream is fine.
	     */
	    break;

	case '(':
	    /*
	     * Parse error was during the parsing of the index part of an
	     * array variable substitution at the toplevel.
	     */

	    if (*(parsePtr->term - 1) == '$') {
		/*
		 * Special case where removing the array index left us with
		 * just a dollar sign (array variable with name the empty
		 * string as its name), instead of with a scalar variable
		 * reference.
		 *
		 * As in the previous case, existing token stream is OK.
		 */
	    } else {
		/*
		 * The current parse includes a successful parse of a scalar
		 * variable substitution where there should have been an array
		 * variable substitution. We remove that mistaken part of the
		 * parse before moving on. A scalar variable substitution is
		 * two tokens.
		 */

		Tcl_Token *varTokenPtr =
			parsePtr->tokenPtr + parsePtr->numTokens - 2;

		if (varTokenPtr->type != TCL_TOKEN_VARIABLE) {
		    Tcl_Panic("Tcl_SubstObj: programming error");
		}
		if (varTokenPtr[1].type != TCL_TOKEN_TEXT) {
		    Tcl_Panic("Tcl_SubstObj: programming error");
		}
		parsePtr->numTokens -= 2;
	    }
	    break;
	case '[':
	    /*
	     * Parse error occurred during parsing of a toplevel command
	     * substitution.
	     */

	    parsePtr->end = p + length;
	    p = parsePtr->term + 1;
	    length = parsePtr->end - p;
	    if (length == 0) {
		/*
		 * No commands, just an unmatched [. As in previous cases,
		 * existing token stream is OK.
		 */
	    } else {
		/*
		 * We want to add the parsing of as many commands as we can
		 * within that substitution until we reach the actual parse
		 * error. We'll do additional parsing to determine what length
		 * to claim for the final TCL_TOKEN_COMMAND token.
		 */

		Tcl_Token *tokenPtr;
		const char *lastTerm = parsePtr->term;
		Tcl_Parse *nestedPtr = (Tcl_Parse *)
			TclStackAlloc(interp, sizeof(Tcl_Parse));

		while (TCL_OK ==
			Tcl_ParseCommand(NULL, p, length, 0, nestedPtr)) {
		    Tcl_FreeParse(nestedPtr);
		    p = nestedPtr->term + (nestedPtr->term < nestedPtr->end);
		    length = nestedPtr->end - p;
		    if ((length == 0) && (nestedPtr->term == nestedPtr->end)) {
			/*
			 * If we run out of string, blame the missing close
			 * bracket on the last command, and do not evaluate it
			 * during substitution.
			 */

			break;
		    }
		    lastTerm = nestedPtr->term;
		}
		TclStackFree(interp, nestedPtr);

		if (lastTerm == parsePtr->term) {
		    /*
		     * Parse error in first command. No commands to subst, add
		     * no more tokens.
		     */
		    break;
		}

		/*
		 * Create a command substitution token for whatever commands
		 * got parsed.
		 */

		TclGrowParseTokenArray(parsePtr, 1);
		tokenPtr = &(parsePtr->tokenPtr[parsePtr->numTokens]);
		tokenPtr->start = parsePtr->term;
		tokenPtr->numComponents = 0;
		tokenPtr->type = TCL_TOKEN_COMMAND;
		tokenPtr->size = lastTerm - tokenPtr->start + 1;
		parsePtr->numTokens++;
	    }
	    break;

	default:
	    Tcl_Panic("bad parse in Tcl_SubstObj: %c", p[length]);
	}
    }

    /*
     * Next, substitute the parsed tokens just as in normal Tcl evaluation.
     */

    endTokenPtr = parsePtr->tokenPtr + parsePtr->numTokens;
    tokensLeft = parsePtr->numTokens;
    code = TclSubstTokens(interp, endTokenPtr - tokensLeft, tokensLeft,
	    &tokensLeft, 1, NULL, NULL);
    if (code == TCL_OK) {
	Tcl_FreeParse(parsePtr);
	TclStackFree(interp, parsePtr);
	if (errMsg != NULL) {
	    Tcl_SetObjResult(interp, errMsg);
	    Tcl_DecrRefCount(errMsg);
	    return NULL;
	}
	return Tcl_GetObjResult(interp);
    }

    result = Tcl_NewObj();
    while (1) {
	switch (code) {
	case TCL_ERROR:
	    Tcl_FreeParse(parsePtr);
	    TclStackFree(interp, parsePtr);
	    Tcl_DecrRefCount(result);
	    if (errMsg != NULL) {
		Tcl_DecrRefCount(errMsg);
	    }
	    return NULL;
	case TCL_BREAK:
	    tokensLeft = 0;		/* Halt substitution */
	default:
	    Tcl_AppendObjToObj(result, Tcl_GetObjResult(interp));
	}

	if (tokensLeft == 0) {
	    Tcl_FreeParse(parsePtr);
	    TclStackFree(interp, parsePtr);
	    if (errMsg != NULL) {
		if (code != TCL_BREAK) {
		    Tcl_DecrRefCount(result);
		    Tcl_SetObjResult(interp, errMsg);
		    Tcl_DecrRefCount(errMsg);
		    return NULL;
		}
		Tcl_DecrRefCount(errMsg);
	    }
	    return result;
	}

	code = TclSubstTokens(interp, endTokenPtr - tokensLeft, tokensLeft,
		&tokensLeft, 1, NULL, NULL);
    }
}

```

tcl8.6 8.6.18, revision `Pinned Tcl release 8.6.18; https://github.com/tcltk/tcl/blob/core-8-6-18/generic/tclCompile.c`, `generic/tclCompile.c`, function `CompileSubstObj`, lines 1275–1344. Full-source SHA-256 `d1a494975cb56b8850804669e17fb7c5aacc1d70de505ea661f59ca365ab45c8`; snippet SHA-256 `c7651a3355ff148e6450d252f681e57ea91882a4ecb6c7a999b4a4bb2ddfa875`; retained evidence `e10`.

```text

CompileSubstObj(
    Tcl_Interp *interp,
    Tcl_Obj *objPtr,
    int flags)
{
    Interp *iPtr = (Interp *) interp;
    ByteCode *codePtr = NULL;

    if (objPtr->typePtr == &substCodeType) {
	Namespace *nsPtr = iPtr->varFramePtr->nsPtr;

	codePtr = (ByteCode *)objPtr->internalRep.twoPtrValue.ptr1;
	if (flags != PTR2INT(objPtr->internalRep.twoPtrValue.ptr2)
		|| ((Interp *) *codePtr->interpHandle != iPtr)
		|| (codePtr->compileEpoch != iPtr->compileEpoch)
		|| (codePtr->nsPtr != nsPtr)
		|| (codePtr->nsEpoch != nsPtr->resolverEpoch)
		|| (codePtr->localCachePtr !=
		iPtr->varFramePtr->localCachePtr)) {
	    FreeSubstCodeInternalRep(objPtr);
	}
    }
    if (objPtr->typePtr != &substCodeType) {
	CompileEnv compEnv;
	int numBytes;
	const char *bytes = Tcl_GetStringFromObj(objPtr, &numBytes);

	/* TODO: Check for more TIP 280 */
	TclInitCompileEnv(interp, &compEnv, bytes, numBytes, NULL, 0);

	TclSubstCompile(interp, bytes, numBytes, flags, 1, &compEnv);

	TclEmitOpcode(INST_DONE, &compEnv);
	TclInitByteCodeObj(objPtr, &compEnv);
	objPtr->typePtr = &substCodeType;
	TclFreeCompileEnv(&compEnv);

	codePtr = (ByteCode *)objPtr->internalRep.twoPtrValue.ptr1;
	objPtr->internalRep.twoPtrValue.ptr1 = codePtr;
	objPtr->internalRep.twoPtrValue.ptr2 = INT2PTR(flags);
	if (iPtr->varFramePtr->localCachePtr) {
	    codePtr->localCachePtr = iPtr->varFramePtr->localCachePtr;
	    codePtr->localCachePtr->refCount++;
	}
	TclDebugPrintByteCodeObj(objPtr);
    }
    return codePtr;
}

/*
 *----------------------------------------------------------------------
 *
 * FreeSubstCodeInternalRep --
 *
 *	Part of the substcode Tcl object type implementation. Frees the
 *	storage associated with a substcode object's internal representation
 *	unless its code is actively being executed.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	The substcode object's internal rep is marked invalid and its code
 *	gets freed unless the code is actively being executed. In that case
 *	the cleanup is delayed until the last execution of the code completes.
 *
 *----------------------------------------------------------------------
 */

```

tcl9.0 9.0.4, revision `Pinned Tcl release 9.0.4; https://github.com/tcltk/tcl/blob/core-9-0-4/generic/tclCompile.c`, `generic/tclCompile.c`, function `CompileSubstObj`, lines 1316–1384. Full-source SHA-256 `51b709e7bf2c1f25bb7b3ab371c7dcbd6cf6d2bce509931397303cb770ee21f3`; snippet SHA-256 `60b41530211e2c334c36aeadf0eb5d7593993fed3c549903a12e3c7c53e2776a`; retained evidence `e11`.

```text

CompileSubstObj(
    Tcl_Interp *interp,
    Tcl_Obj *objPtr,
    int flags)
{
    Interp *iPtr = (Interp *) interp;
    ByteCode *codePtr = NULL;

    ByteCodeGetInternalRep(objPtr, &substCodeType, codePtr);

    if (codePtr != NULL) {
	Namespace *nsPtr = iPtr->varFramePtr->nsPtr;

	if (flags != PTR2INT(SubstFlags(objPtr))
		|| ((Interp *) *codePtr->interpHandle != iPtr)
		|| (codePtr->compileEpoch != iPtr->compileEpoch)
		|| (codePtr->nsPtr != nsPtr)
		|| (codePtr->nsEpoch != nsPtr->resolverEpoch)
		|| (codePtr->localCachePtr !=
		iPtr->varFramePtr->localCachePtr)) {
	    Tcl_StoreInternalRep(objPtr, &substCodeType, NULL);
	    codePtr = NULL;
	}
    }
    if (codePtr == NULL) {
	CompileEnv compEnv;
	Tcl_Size numBytes;
	const char *bytes = TclGetStringFromObj(objPtr, &numBytes);

	/* TODO: Check for more TIP 280 */
	TclInitCompileEnv(interp, &compEnv, bytes, numBytes, NULL, 0);

	TclSubstCompile(interp, bytes, numBytes, flags, 1, &compEnv);

	TclEmitOpcode(INST_DONE, &compEnv);
	codePtr = TclInitByteCodeObj(objPtr, &substCodeType, &compEnv);
	TclFreeCompileEnv(&compEnv);

	SubstFlags(objPtr) = INT2PTR(flags);
	if (iPtr->varFramePtr->localCachePtr) {
	    codePtr->localCachePtr = iPtr->varFramePtr->localCachePtr;
	    codePtr->localCachePtr->refCount++;
	}
	TclDebugPrintByteCodeObj(objPtr);
    }
    return codePtr;
}

/*
 *----------------------------------------------------------------------
 *
 * FreeSubstCodeInternalRep --
 *
 *	Part of the "substcode" Tcl object type implementation. Frees the
 *	storage associated with the substcode internal representation of a
 *	Tcl_Obj unless its code is actively being executed.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	The substcode object's internal rep is marked invalid and its code
 *	gets freed unless the code is actively being executed. In that case
 *	the cleanup is delayed until the last execution of the code completes.
 *
 *----------------------------------------------------------------------
 */

```

tcl9.1 9.1.0, revision `Pinned Tcl release 9.1.0; https://github.com/tcltk/tcl/blob/core-9-1-0/generic/tclCompile.c`, `generic/tclCompile.c`, function `CompileSubstObj`, lines 1634–1702. Full-source SHA-256 `941eaa2c57e4d7d5266134c5b713dca5218d29eea951c89a0676c14b8260eb4f`; snippet SHA-256 `29b23e12e807e9d4f31ed1a0e5afe7e8cf7dcfa461309795e7375cbdd4cb2333`; retained evidence `e12`.

```text

CompileSubstObj(
    Tcl_Interp *interp,
    Tcl_Obj *objPtr,
    int flags)
{
    Interp *iPtr = (Interp *) interp;
    ByteCode *codePtr = NULL;

    ByteCodeGetInternalRep(objPtr, &substCodeType, codePtr);

    if (codePtr != NULL) {
	Namespace *nsPtr = iPtr->varFramePtr->nsPtr;

	if (flags != PTR2INT(SubstFlags(objPtr))
		|| ((Interp *) *codePtr->interpHandle != iPtr)
		|| (codePtr->compileEpoch != iPtr->compileEpoch)
		|| (codePtr->nsPtr != nsPtr)
		|| (codePtr->nsEpoch != nsPtr->resolverEpoch)
		|| (codePtr->localCachePtr !=
		iPtr->varFramePtr->localCachePtr)) {
	    Tcl_StoreInternalRep(objPtr, &substCodeType, NULL);
	    codePtr = NULL;
	}
    }
    if (codePtr == NULL) {
	CompileEnv compEnv;
	Tcl_Size numBytes;
	const char *bytes = TclGetStringFromObj(objPtr, &numBytes);

	/* TODO: Check for more TIP 280 */
	TclInitCompileEnv(interp, &compEnv, bytes, numBytes, NULL, 0);

	TclSubstCompile(interp, bytes, numBytes, flags, 1, &compEnv);

	TclEmitOpcode(		INST_DONE,			&compEnv);
	codePtr = TclInitByteCodeObj(objPtr, &substCodeType, &compEnv);
	TclFreeCompileEnv(&compEnv);

	SubstFlags(objPtr) = INT2PTR(flags);
	if (iPtr->varFramePtr->localCachePtr) {
	    codePtr->localCachePtr = iPtr->varFramePtr->localCachePtr;
	    codePtr->localCachePtr->refCount++;
	}
	TclDebugPrintByteCodeObj(objPtr);
    }
    return codePtr;
}

/*
 *----------------------------------------------------------------------
 *
 * FreeSubstCodeInternalRep --
 *
 *	Part of the "substcode" Tcl object type implementation. Frees the
 *	storage associated with the substcode internal representation of a
 *	Tcl_Obj unless its code is actively being executed.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	The substcode object's internal rep is marked invalid and its code
 *	gets freed unless the code is actively being executed. In that case
 *	the cleanup is delayed until the last execution of the code completes.
 *
 *----------------------------------------------------------------------
 */

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Retrieve each original C file from its attached pinned release URL, verify the complete source SHA-256, then compare the recorded function line range and exact snippet SHA-256. The separate Subst replay.py validates observed program captures; it does not reproduce this implementation-source inspection.
