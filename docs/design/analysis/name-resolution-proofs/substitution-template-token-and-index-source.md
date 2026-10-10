# naming.substitution.template-token-and-index-source

Kind: `source-anchor`

## Problem statement

A root substitution mask can disable commands or variables while an admitted variable index still evaluates its own substitutions. Separately, merging literal template tokens can preserve text but change C9 result primary storage. Ordinary written-word merging and root template compilation therefore require different selected parser purposes.

## Question

Which pinned C template token boundaries and variable-index substitution flags are selected independently of ordinary written-word merging, and what flag does pinned Jim use for dictionary-sugar index expansion?

## Conclusion

The inspected C8.6/C9 parser emits separate TEXT/BS template tokens and uses TCL_SUBST_ALL for variable indices. TclSubstCompile pushes those tokens separately, while TclCompileTokens accumulates adjacent TEXT/BS. The independently inspected Jim index expansion uses JIM_NONE. These source excerpts explain purpose selection; they do not add native execution or arbitrary-object ownership claims.

## Scope

Pinned Tcl8.6.18/9.0.4/9.1.0 function bodies and Jim revision5bac7c99ad65864c87da513e22e2f01703fa4e03. C8.4/8.5 and BIG-IP are not inspected in this record. Template root token scheduling and index scheduling are separate; no command existence, CPP admission, native object header or frame authority follows.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not inspected. Build: not recorded. Channel: not inspected. Dialect: Tcl.

This source-inspection question has no retained excerpt for this provider.

### tcl8.5

Status: `not-tested`. Version: not inspected. Build: not recorded. Channel: not inspected. Dialect: Tcl.

This source-inspection question has no retained excerpt for this provider.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Source inspection only; no additional executable or build was invoked.. Channel: Inspected ParseTokens/Tcl_ParseVarName and native compiler function bodies; no native input invocation.. Dialect: Tcl.

ParseTokens keeps separate substitution tokens; disabled sigils are one-byte TEXT, array-index ParseTokens uses TCL_SUBST_ALL, TclSubstCompile registers TEXT/BS separately while TclCompileTokens accumulates adjacent TEXT/BS.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Source inspection only; no additional executable or build was invoked.. Channel: Inspected ParseTokens/Tcl_ParseVarName and native compiler function bodies; no native input invocation.. Dialect: Tcl.

ParseTokens keeps separate substitution tokens; disabled sigils are one-byte TEXT, array-index ParseTokens uses TCL_SUBST_ALL, TclSubstCompile registers TEXT/BS separately while TclCompileTokens accumulates adjacent TEXT/BS.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Source inspection only; no additional executable or build was invoked.. Channel: Inspected ParseTokens/Tcl_ParseVarName and native compiler function bodies; no native input invocation.. Dialect: Tcl.

ParseTokens keeps separate substitution tokens; disabled sigils are one-byte TEXT, array-index ParseTokens uses TCL_SUBST_ALL, TclSubstCompile registers TEXT/BS separately while TclCompileTokens accumulates adjacent TEXT/BS.

### jim

Status: `inspected`. Version: 0.84-9-g5bac7c9 source revision. Build: Source inspection only; no additional executable or build was invoked.. Channel: Inspected JimExpandDictSugar; no native input invocation.. Dialect: Jim Tcl.

Dictionary-sugar index expansion invokes Jim_SubstObj on the index object with JIM_NONE, independently of the outer template mask; this excerpt does not establish C template token splitting for Jim.

### bigip

Status: `not-tested`. Version: not inspected. Build: not recorded. Channel: not inspected. Dialect: F5 iRules.

This source-inspection question has no retained excerpt for this provider.

## Exact evidence

- `source-tcl8.6` (source-anchor): [rust/tcl-registry/tests/data/native_substitution_token_source/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_substitution_token_source/8.6.18.txt). SHA-256 `88cc94b4dc5ca78dbeb1cd978afbdab82f0a141ea35946aac6ef1423bf468eef`. Exact pinned C parser, variable-index parser, substitution compiler and ordinary word compiler function windows; complete-source hashes and one-based line locations retained.
- `source-tcl9.0` (source-anchor): [rust/tcl-registry/tests/data/native_substitution_token_source/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_substitution_token_source/9.0.4.txt). SHA-256 `c851c13aa7ac6d665130089a73637ea1732bda9d2f289cee8d40c08a1cb6e0f4`. Exact pinned C parser, variable-index parser, substitution compiler and ordinary word compiler function windows; complete-source hashes and one-based line locations retained.
- `source-tcl9.1` (source-anchor): [rust/tcl-registry/tests/data/native_substitution_token_source/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_substitution_token_source/9.1.0.txt). SHA-256 `b6e090f0417269ae766b4fe9c2f2ec8e1aa92098f81f90175d23b7bc9d59a87d`. Exact pinned C parser, variable-index parser, substitution compiler and ordinary word compiler function windows; complete-source hashes and one-based line locations retained.
- `source-jim` (source-anchor): [rust/tcl-registry/tests/data/native_substitution_token_source/jim-5bac7c9.txt](../../../../rust/tcl-registry/tests/data/native_substitution_token_source/jim-5bac7c9.txt). SHA-256 `cd609742df46cff9d3b06a38b2613c781846dd811dc7a5086b9960cbacd08071`. Exact JimExpandDictSugar function from pinned revision5bac7c99ad65864c87da513e22e2f01703fa4e03.

## Source inspection

tcl8.6 8.6.18, revision `core8-6-18`, `generic/tclParse.c`, function `ParseTokens`, lines 1123–1339. Full-source SHA-256 `bbab97189d8d3e1ccca11a1bb02cc2c8c6099c7ba33d3f35de4e614a7ba8bfb6`; snippet SHA-256 `e36e7892b797757a733676537f2de82b3be4a6c5957df60947b1f9c70d638988`; retained evidence `source-tcl8.6`.

```text
ParseTokens(
    const char *src,	/* First character to parse. */
    int numBytes,	/* Max number of bytes to scan. */
    int mask,			/* Specifies when to stop parsing. The parse
				 * stops at the first unquoted character whose
				 * CHAR_TYPE contains any of the bits in
				 * mask. */
    int flags,			/* OR-ed bits indicating what substitutions to
				 * perform: TCL_SUBST_COMMANDS,
				 * TCL_SUBST_VARIABLES, and
				 * TCL_SUBST_BACKSLASHES */
    Tcl_Parse *parsePtr)	/* Information about parse in progress.
				 * Updated with additional tokens and
				 * termination information. */
{
    char type;
    int originalTokens;
    int noSubstCmds = !(flags & TCL_SUBST_COMMANDS);
    int noSubstVars = !(flags & TCL_SUBST_VARIABLES);
    int noSubstBS = !(flags & TCL_SUBST_BACKSLASHES);
    Tcl_Token *tokenPtr;

    /*
     * Each iteration through the following loop adds one token of type
     * TCL_TOKEN_TEXT, TCL_TOKEN_BS, TCL_TOKEN_COMMAND, or TCL_TOKEN_VARIABLE
     * to parsePtr. For TCL_TOKEN_VARIABLE tokens, additional tokens are added
     * for the parsed variable name.
     */

    originalTokens = parsePtr->numTokens;
    while (numBytes && !((type = CHAR_TYPE(*src)) & mask)) {
	TclGrowParseTokenArray(parsePtr, 1);
	tokenPtr = &parsePtr->tokenPtr[parsePtr->numTokens];
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

	if ((type & TYPE_SUBS) == 0) {
	    /*
	     * This is a simple range of characters. Scan to find the end of
	     * the range.
	     */

	    while ((++src, --numBytes)
		    && !(CHAR_TYPE(*src) & (mask | TYPE_SUBS))) {
		/* empty loop */
	    }
	    tokenPtr->type = TCL_TOKEN_TEXT;
	    tokenPtr->size = src - tokenPtr->start;
	    parsePtr->numTokens++;
	} else if (*src == '$') {
	    int varToken;

	    if (noSubstVars) {
		tokenPtr->type = TCL_TOKEN_TEXT;
		tokenPtr->size = 1;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    /*
	     * This is a variable reference. Call Tcl_ParseVarName to do all
	     * the dirty work of parsing the name.
	     */

	    varToken = parsePtr->numTokens;
	    if (Tcl_ParseVarName(parsePtr->interp, src, numBytes, parsePtr,
		    1) != TCL_OK) {
		return TCL_ERROR;
	    }
	    src += parsePtr->tokenPtr[varToken].size;
	    numBytes -= parsePtr->tokenPtr[varToken].size;
	} else if (*src == '[') {
	    Tcl_Parse *nestedPtr;

	    if (noSubstCmds) {
		tokenPtr->type = TCL_TOKEN_TEXT;
		tokenPtr->size = 1;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    /*
	     * Command substitution. Call Tcl_ParseCommand recursively (and
	     * repeatedly) to parse the nested command(s), then throw away the
	     * parse information.
	     */

	    src++;
	    numBytes--;
	    nestedPtr = TclStackAlloc(parsePtr->interp, sizeof(Tcl_Parse));
	    while (1) {
		const char *curEnd;

		if (Tcl_ParseCommand(parsePtr->interp, src, numBytes, 1,
			nestedPtr) != TCL_OK) {
		    parsePtr->errorType = nestedPtr->errorType;
		    parsePtr->term = nestedPtr->term;
		    parsePtr->incomplete = nestedPtr->incomplete;
		    TclStackFree(parsePtr->interp, nestedPtr);
		    return TCL_ERROR;
		}
		curEnd = src + numBytes;
		src = nestedPtr->commandStart + nestedPtr->commandSize;
		numBytes = curEnd - src;
		Tcl_FreeParse(nestedPtr);

		/*
		 * Check for the closing ']' that ends the command
		 * substitution. It must have been the last character of the
		 * parsed command.
		 */

		if ((nestedPtr->term < parsePtr->end)
			&& (*(nestedPtr->term) == ']')
			&& !(nestedPtr->incomplete)) {
		    break;
		}
		if (numBytes == 0) {
		    if (parsePtr->interp != NULL) {
			Tcl_SetObjResult(parsePtr->interp, Tcl_NewStringObj(
				"missing close-bracket", -1));
		    }
		    parsePtr->errorType = TCL_PARSE_MISSING_BRACKET;
		    parsePtr->term = tokenPtr->start;
		    parsePtr->incomplete = 1;
		    TclStackFree(parsePtr->interp, nestedPtr);
		    return TCL_ERROR;
		}
	    }
	    TclStackFree(parsePtr->interp, nestedPtr);
	    tokenPtr->type = TCL_TOKEN_COMMAND;
	    tokenPtr->size = src - tokenPtr->start;
	    parsePtr->numTokens++;
	} else if (*src == '\\') {
	    if (noSubstBS) {
		tokenPtr->type = TCL_TOKEN_TEXT;
		tokenPtr->size = 1;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    /*
	     * Backslash substitution.
	     */

	    TclParseBackslash(src, numBytes, &tokenPtr->size, NULL);

	    if (tokenPtr->size == 1) {
		/*
		 * Just a backslash, due to end of string.
		 */

		tokenPtr->type = TCL_TOKEN_TEXT;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    if (src[1] == '\n') {
		if (numBytes == 2) {
		    parsePtr->incomplete = 1;
		}

		/*
		 * Note: backslash-newline is special in that it is treated
		 * the same as a space character would be. This means that it
		 * could terminate the token.
		 */

		if (mask & TYPE_SPACE) {
		    if (parsePtr->numTokens == originalTokens) {
			goto finishToken;
		    }
		    break;
		}
	    }

	    tokenPtr->type = TCL_TOKEN_BS;
	    parsePtr->numTokens++;
	    src += tokenPtr->size;
	    numBytes -= tokenPtr->size;
	} else if (*src == 0) {
	    tokenPtr->type = TCL_TOKEN_TEXT;
	    tokenPtr->size = 1;
	    parsePtr->numTokens++;
	    src++;
	    numBytes--;
	} else {
	    Tcl_Panic("ParseTokens encountered unknown character");
	}
    }
    if (parsePtr->numTokens == originalTokens) {
	/*
	 * There was nothing in this range of text. Add an empty token for the
	 * empty range, so that there is always at least one token added.
	 */

	TclGrowParseTokenArray(parsePtr, 1);
	tokenPtr = &parsePtr->tokenPtr[parsePtr->numTokens];
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

    finishToken:
	tokenPtr->type = TCL_TOKEN_TEXT;
	tokenPtr->size = 0;
	parsePtr->numTokens++;
    }
    parsePtr->term = src;
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `core8-6-18`, `generic/tclParse.c`, function `Tcl_ParseVarName`, lines 1398–1569. Full-source SHA-256 `bbab97189d8d3e1ccca11a1bb02cc2c8c6099c7ba33d3f35de4e614a7ba8bfb6`; snippet SHA-256 `649e596ff7c56b98d7c3c1ecec4928ae626e041c3204ffb89080c70f1cece245`; retained evidence `source-tcl8.6`.

```text
Tcl_ParseVarName(
    Tcl_Interp *interp,		/* Interpreter to use for error reporting; if
				 * NULL, then no error message is provided. */
    const char *start,		/* Start of variable substitution string.
				 * First character must be "$". */
    int numBytes,	/* Total number of bytes in string. If < 0,
				 * the string consists of all bytes up to the
				 * first null character. */
    Tcl_Parse *parsePtr,	/* Structure to fill in with information about
				 * the variable name. */
    int append)			/* Non-zero means append tokens to existing
				 * information in parsePtr; zero means ignore
				 * existing tokens in parsePtr and
				 * reinitialize it. */
{
    Tcl_Token *tokenPtr;
    const char *src;
    int varIndex;
    unsigned array;

    if (numBytes < 0 && start) {
	numBytes = strlen(start);
    }
    if (!append) {
	TclParseInit(interp, start, numBytes, parsePtr);
    }
    if ((numBytes == 0) || (start == NULL)) {
	return TCL_ERROR;
    }

    /*
     * Generate one token for the variable, an additional token for the name,
     * plus any number of additional tokens for the index, if there is one.
     */

    src = start;
    TclGrowParseTokenArray(parsePtr, 2);
    tokenPtr = &parsePtr->tokenPtr[parsePtr->numTokens];
    tokenPtr->type = TCL_TOKEN_VARIABLE;
    tokenPtr->start = src;
    varIndex = parsePtr->numTokens;
    parsePtr->numTokens++;
    tokenPtr++;
    src++;
    numBytes--;
    if (numBytes == 0) {
	goto justADollarSign;
    }
    tokenPtr->type = TCL_TOKEN_TEXT;
    tokenPtr->start = src;
    tokenPtr->numComponents = 0;

    /*
     * The name of the variable can have three forms:
     * 1. The $ sign is followed by an open curly brace. Then the variable
     *	  name is everything up to the next close curly brace, and the
     *	  variable is a scalar variable.
     * 2. The $ sign is not followed by an open curly brace. Then the variable
     *	  name is everything up to the next character that isn't a letter,
     *	  digit, or underscore. :: sequences are also considered part of the
     *	  variable name, in order to support namespaces. If the following
     *	  character is an open parenthesis, then the information between
     *	  parentheses is the array element name.
     * 3. The $ sign is followed by something that isn't a letter, digit, or
     *	  underscore: in this case, there is no variable name and the token is
     *	  just "$".
     */

    if (*src == '{') {
	src++;
	numBytes--;
	tokenPtr->type = TCL_TOKEN_TEXT;
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

	while (numBytes && (*src != '}')) {
	    numBytes--;
	    src++;
	}
	if (numBytes == 0) {
	    if (parsePtr->interp != NULL) {
		Tcl_SetObjResult(parsePtr->interp, Tcl_NewStringObj(
			"missing close-brace for variable name", -1));
	    }
	    parsePtr->errorType = TCL_PARSE_MISSING_VAR_BRACE;
	    parsePtr->term = tokenPtr->start-1;
	    parsePtr->incomplete = 1;
	    goto error;
	}
	tokenPtr->size = src - tokenPtr->start;
	tokenPtr[-1].size = src - tokenPtr[-1].start;
	parsePtr->numTokens++;
	src++;
    } else {
	tokenPtr->type = TCL_TOKEN_TEXT;
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

	while (numBytes) {
	    if (TclIsBareword(*src)) {
		src += 1;
		numBytes -= 1;
		continue;
	    }
	    if ((src[0] == ':') && (numBytes != 1) && (src[1] == ':')) {
		src += 2;
		numBytes -= 2;
		while (numBytes && (*src == ':')) {
		    src++;
		    numBytes--;
		}
		continue;
	    }
	    break;
	}

	/*
	 * Support for empty array names here.
	 */

	array = (numBytes && (*src == '('));
	tokenPtr->size = src - tokenPtr->start;
	if ((tokenPtr->size == 0) && !array) {
	    goto justADollarSign;
	}
	parsePtr->numTokens++;
	if (array) {
	    /*
	     * This is a reference to an array element. Call ParseTokens
	     * recursively to parse the element name, since it could contain
	     * any number of substitutions.
	     */

	    if (TCL_OK != ParseTokens(src+1, numBytes-1, TYPE_CLOSE_PAREN,
		    TCL_SUBST_ALL, parsePtr)) {
		goto error;
	    }
	    if ((parsePtr->term == src+numBytes) || (*parsePtr->term != ')')){
		if (parsePtr->interp != NULL) {
		    Tcl_SetObjResult(parsePtr->interp, Tcl_NewStringObj(
			    "missing )", -1));
		}
		parsePtr->errorType = TCL_PARSE_MISSING_PAREN;
		parsePtr->term = src;
		parsePtr->incomplete = 1;
		goto error;
	    }
	    src = parsePtr->term + 1;
	}
    }
    tokenPtr = &parsePtr->tokenPtr[varIndex];
    tokenPtr->size = src - tokenPtr->start;
    tokenPtr->numComponents = parsePtr->numTokens - (varIndex + 1);
    return TCL_OK;

    /*
     * The dollar sign isn't followed by a variable name. Replace the
     * TCL_TOKEN_VARIABLE token with a TCL_TOKEN_TEXT token for the dollar
     * sign.
     */

  justADollarSign:
    tokenPtr = &parsePtr->tokenPtr[varIndex];
    tokenPtr->type = TCL_TOKEN_TEXT;
    tokenPtr->size = 1;
    tokenPtr->numComponents = 0;
    return TCL_OK;

  error:
    Tcl_FreeParse(parsePtr);
    return TCL_ERROR;
}

```

tcl8.6 8.6.18, revision `core8-6-18`, `generic/tclCompCmdsSZ.c`, function `TclSubstCompile`, lines 1460–1703. Full-source SHA-256 `b41bd49a6d4e4d550dda90b4788d4d3928b30479f1d2a84a02f09be7ce4dd942`; snippet SHA-256 `1e3bfb579469fa0d6adcccd4863e593ca1ba50070c966d68432766289002481e`; retained evidence `source-tcl8.6`.

```text
TclSubstCompile(
    Tcl_Interp *interp,
    const char *bytes,
    int numBytes,
    int flags,
    int line,
    CompileEnv *envPtr)
{
    Tcl_Token *endTokenPtr, *tokenPtr;
    int breakOffset = 0, count = 0, bline = line;
    Tcl_Parse parse;
    Tcl_InterpState state = NULL;

    TclSubstParse(interp, bytes, numBytes, flags, &parse, &state);
    if (state != NULL) {
	Tcl_ResetResult(interp);
    }

    /*
     * Tricky point! If the first token does not result in a *guaranteed* push
     * of a Tcl_Obj on the stack, we must push an empty object. Otherwise it
     * is possible to get to an INST_STR_CONCAT1 or INST_DONE without enough
     * values on the stack, resulting in a crash. Thanks to Joe Mistachkin for
     * identifying a script that could trigger this case.
     */

    tokenPtr = parse.tokenPtr;
    if (tokenPtr->type != TCL_TOKEN_TEXT && tokenPtr->type != TCL_TOKEN_BS) {
	PUSH("");
	count++;
    }

    for (endTokenPtr = tokenPtr + parse.numTokens;
	    tokenPtr < endTokenPtr; tokenPtr = TokenAfter(tokenPtr)) {
	int length, literal, catchRange, breakJump;
	char buf[TCL_UTF_MAX] = "";
	JumpFixup startFixup, okFixup, returnFixup, breakFixup;
	JumpFixup continueFixup, otherFixup, endFixup;

	switch (tokenPtr->type) {
	case TCL_TOKEN_TEXT:
	    literal = TclRegisterNewLiteral(envPtr,
		    tokenPtr->start, tokenPtr->size);
	    TclEmitPush(literal, envPtr);
	    TclAdvanceLines(&bline, tokenPtr->start,
		    tokenPtr->start + tokenPtr->size);
	    count++;
	    continue;
	case TCL_TOKEN_BS:
	    length = TclParseBackslash(tokenPtr->start, tokenPtr->size,
		    NULL, buf);
	    literal = TclRegisterNewLiteral(envPtr, buf, length);
	    TclEmitPush(literal, envPtr);
	    count++;
	    continue;
	case TCL_TOKEN_VARIABLE:
	    /*
	     * Check for simple variable access; see if we can only generate
	     * TCL_OK or TCL_ERROR from the substituted variable read; if so,
	     * there is no need to generate elaborate exception-management
	     * code. Note that the first component of TCL_TOKEN_VARIABLE is
	     * always TCL_TOKEN_TEXT...
	     */

	    if (tokenPtr->numComponents > 1) {
		int i, foundCommand = 0;

		for (i=2 ; i<=tokenPtr->numComponents ; i++) {
		    if (tokenPtr[i].type == TCL_TOKEN_COMMAND) {
			foundCommand = 1;
			break;
		    }
		}
		if (foundCommand) {
		    break;
		}
	    }

	    envPtr->line = bline;
	    TclCompileVarSubst(interp, tokenPtr, envPtr);
	    bline = envPtr->line;
	    count++;
	    continue;
	}

	while (count > 255) {
	    OP1(		STR_CONCAT1, 255);
	    count -= 254;
	}
	if (count > 1) {
	    OP1(		STR_CONCAT1, count);
	    count = 1;
	}

	if (breakOffset == 0) {
	    /* Jump to the start (jump over the jump to end) */
	    TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &startFixup);

	    /* Jump to the end (all BREAKs land here) */
	    breakOffset = CurrentOffset(envPtr);
	    TclEmitInstInt4(INST_JUMP4, 0, envPtr);

	    /* Start */
	    if (TclFixupForwardJumpToHere(envPtr, &startFixup, 127)) {
		Tcl_Panic("TclCompileSubstCmd: bad start jump distance %d",
			(int) (CurrentOffset(envPtr) - startFixup.codeOffset));
	    }
	}

	envPtr->line = bline;
	catchRange = TclCreateExceptRange(CATCH_EXCEPTION_RANGE, envPtr);
	OP4(	BEGIN_CATCH4, catchRange);
	ExceptionRangeStarts(envPtr, catchRange);

	switch (tokenPtr->type) {
	case TCL_TOKEN_COMMAND:
	    TclCompileScript(interp, tokenPtr->start+1, tokenPtr->size-2,
		    envPtr);
	    count++;
	    break;
	case TCL_TOKEN_VARIABLE:
	    TclCompileVarSubst(interp, tokenPtr, envPtr);
	    count++;
	    break;
	default:
	    Tcl_Panic("unexpected token type in TclCompileSubstCmd: %d",
		    tokenPtr->type);
	}

	ExceptionRangeEnds(envPtr, catchRange);

	/* Substitution produced TCL_OK */
	OP(	END_CATCH);
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &okFixup);
	TclAdjustStackDepth(-1, envPtr);

	/* Exceptional return codes processed here */
	ExceptionRangeTarget(envPtr, catchRange, catchOffset);
	OP(	PUSH_RETURN_OPTIONS);
	OP(	PUSH_RESULT);
	OP(	PUSH_RETURN_CODE);
	OP(	END_CATCH);
	OP(	RETURN_CODE_BRANCH);

	/* ERROR -> reraise it; NB: can't require BREAK/CONTINUE handling */
	OP(	RETURN_STK);
	OP(	NOP);

	/* RETURN */
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &returnFixup);

	/* BREAK */
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &breakFixup);

	/* CONTINUE */
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &continueFixup);

	/* OTHER */
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &otherFixup);

	TclAdjustStackDepth(1, envPtr);
	/* BREAK destination */
	if (TclFixupForwardJumpToHere(envPtr, &breakFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad break jump distance %d",
		    (int) (CurrentOffset(envPtr) - breakFixup.codeOffset));
	}
	OP(	POP);
	OP(	POP);

	breakJump = CurrentOffset(envPtr) - breakOffset;
	if (breakJump > 127) {
	    OP4(JUMP4, -breakJump);
	} else {
	    OP1(JUMP1, -breakJump);
	}

	TclAdjustStackDepth(2, envPtr);
	/* CONTINUE destination */
	if (TclFixupForwardJumpToHere(envPtr, &continueFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad continue jump distance %d",
		    (int) (CurrentOffset(envPtr) - continueFixup.codeOffset));
	}
	OP(	POP);
	OP(	POP);
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &endFixup);

	TclAdjustStackDepth(2, envPtr);
	/* RETURN + other destination */
	if (TclFixupForwardJumpToHere(envPtr, &returnFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad return jump distance %d",
		    (int) (CurrentOffset(envPtr) - returnFixup.codeOffset));
	}
	if (TclFixupForwardJumpToHere(envPtr, &otherFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad other jump distance %d",
		    (int) (CurrentOffset(envPtr) - otherFixup.codeOffset));
	}

	/*
	 * Pull the result to top of stack, discard options dict.
	 */

	OP4(	REVERSE, 2);
	OP(	POP);

	/* OK destination */
	if (TclFixupForwardJumpToHere(envPtr, &okFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad ok jump distance %d",
		    (int) (CurrentOffset(envPtr) - okFixup.codeOffset));
	}
	if (count > 1) {
	    OP1(STR_CONCAT1, count);
	    count = 1;
	}

	/* CONTINUE jump to here */
	if (TclFixupForwardJumpToHere(envPtr, &endFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad end jump distance %d",
		    (int) (CurrentOffset(envPtr) - endFixup.codeOffset));
	}
	bline = envPtr->line;
    }

    while (count > 255) {
	OP1(	STR_CONCAT1, 255);
	count -= 254;
    }
    if (count > 1) {
	OP1(	STR_CONCAT1, count);
    }

    Tcl_FreeParse(&parse);

    if (state != NULL) {
	Tcl_RestoreInterpState(interp, state);
	TclCompileSyntaxError(interp, envPtr);
	TclAdjustStackDepth(-1, envPtr);
    }

    /* Final target of the multi-jump from all BREAKs */
    if (breakOffset > 0) {
	TclUpdateInstInt4AtPc(INST_JUMP4, CurrentOffset(envPtr) - breakOffset,
		envPtr->codeStart + breakOffset);
    }
}

```

tcl8.6 8.6.18, revision `core8-6-18`, `generic/tclCompile.c`, function `TclCompileTokens`, lines 2355–2550. Full-source SHA-256 `d1a494975cb56b8850804669e17fb7c5aacc1d70de505ea661f59ca365ab45c8`; snippet SHA-256 `0ac0749c57f793286af6437c49c109c4146287f3bb34658926e5802980819dd7`; retained evidence `source-tcl8.6`.

```text
TclCompileTokens(
    Tcl_Interp *interp,		/* Used for error and status reporting. */
    Tcl_Token *tokenPtr,	/* Pointer to first in an array of tokens to
				 * compile. */
    int count,			/* Number of tokens to consider at tokenPtr.
				 * Must be at least 1. */
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    Tcl_DString textBuffer;	/* Holds concatenated chars from adjacent
				 * TCL_TOKEN_TEXT, TCL_TOKEN_BS tokens. */
    char buffer[TCL_UTF_MAX] = "";
    int i, numObjsToConcat, length, adjust;
    unsigned char *entryCodeNext = envPtr->codeNext;
#define NUM_STATIC_POS 20
    int isLiteral, maxNumCL, numCL;
    int *clPosition = NULL;
    int depth = TclGetStackDepth(envPtr);

    /*
     * For the handling of continuation lines in literals, first check if
     * this is actually a literal. For if not we can forego the additional
     * processing. Otherwise preallocate a small table to store the
     * locations of all continuation lines found in this literal, if any.
     * The table is extended if needed.
     *
     * Note: Different to the equivalent code in function 'TclSubstTokens()'
     * (see file "tclParse.c") there seem to be no need the 'adjust' variable.
     * There also seems to be no need for code which merges continuation line
     * information of multiple words which concat'd at runtime. Either that or
     * I have not managed to find a test case for these two possibilities yet.
     * It might be a difference between compile- versus run-time processing.
     */

    numCL = 0;
    maxNumCL = 0;
    isLiteral = 1;
    for (i=0 ; i < count; i++) {
	if ((tokenPtr[i].type != TCL_TOKEN_TEXT)
		&& (tokenPtr[i].type != TCL_TOKEN_BS)) {
	    isLiteral = 0;
	    break;
	}
    }

    if (isLiteral) {
	maxNumCL = NUM_STATIC_POS;
	clPosition = (int *)ckalloc(maxNumCL * sizeof(int));
    }

    adjust = 0;
    Tcl_DStringInit(&textBuffer);
    numObjsToConcat = 0;
    for ( ;  count > 0;  count--, tokenPtr++) {
	switch (tokenPtr->type) {
	case TCL_TOKEN_TEXT:
	    TclDStringAppendToken(&textBuffer, tokenPtr);
	    TclAdvanceLines(&envPtr->line, tokenPtr->start,
		    tokenPtr->start + tokenPtr->size);
	    break;

	case TCL_TOKEN_BS:
	    length = TclParseBackslash(tokenPtr->start, tokenPtr->size,
		    NULL, buffer);
	    Tcl_DStringAppend(&textBuffer, buffer, length);

	    /*
	     * If the backslash sequence we found is in a literal, and
	     * represented a continuation line, we compute and store its
	     * location (as char offset to the beginning of the _result_
	     * script). We may have to extend the table of locations.
	     *
	     * Note that the continuation line information is relevant even if
	     * the word we are processing is not a literal, as it can affect
	     * nested commands. See the branch for TCL_TOKEN_COMMAND below,
	     * where the adjustment we are tracking here is taken into
	     * account. The good thing is that we do not need a table of
	     * everything, just the number of lines we have to add as
	     * correction.
	     */

	    if ((length == 1) && (buffer[0] == ' ') &&
		(tokenPtr->start[1] == '\n')) {
		if (isLiteral) {
		    int clPos = Tcl_DStringLength(&textBuffer);

		    if (numCL >= maxNumCL) {
			maxNumCL *= 2;
			clPosition = (int *)ckrealloc(clPosition,
                                maxNumCL * sizeof(int));
		    }
		    clPosition[numCL] = clPos;
		    numCL ++;
		}
		adjust++;
	    }
	    break;

	case TCL_TOKEN_COMMAND:
	    /*
	     * Push any accumulated chars appearing before the command.
	     */

	    if (Tcl_DStringLength(&textBuffer) > 0) {
		int literal = TclRegisterDStringLiteral(envPtr, &textBuffer);

		TclEmitPush(literal, envPtr);
		numObjsToConcat++;
		Tcl_DStringFree(&textBuffer);

		if (numCL) {
		    TclContinuationsEnter(TclFetchLiteral(envPtr, literal),
			    numCL, clPosition);
		}
		numCL = 0;
	    }

	    envPtr->line += adjust;
	    TclCompileScript(interp, tokenPtr->start+1,
		    tokenPtr->size-2, envPtr);
	    envPtr->line -= adjust;
	    numObjsToConcat++;
	    break;

	case TCL_TOKEN_VARIABLE:
	    /*
	     * Push any accumulated chars appearing before the $<var>.
	     */

	    if (Tcl_DStringLength(&textBuffer) > 0) {
		int literal;

		literal = TclRegisterDStringLiteral(envPtr, &textBuffer);
		TclEmitPush(literal, envPtr);
		numObjsToConcat++;
		Tcl_DStringFree(&textBuffer);
	    }

	    TclCompileVarSubst(interp, tokenPtr, envPtr);
	    numObjsToConcat++;
	    count -= tokenPtr->numComponents;
	    tokenPtr += tokenPtr->numComponents;
	    break;

	default:
	    Tcl_Panic("Unexpected token type in TclCompileTokens: %d; %.*s",
		    tokenPtr->type, tokenPtr->size, tokenPtr->start);
	}
    }

    /*
     * Push any accumulated characters appearing at the end.
     */

    if (Tcl_DStringLength(&textBuffer) > 0) {
	int literal = TclRegisterDStringLiteral(envPtr, &textBuffer);

	TclEmitPush(literal, envPtr);
	numObjsToConcat++;
	if (numCL) {
	    TclContinuationsEnter(TclFetchLiteral(envPtr, literal),
		    numCL, clPosition);
	}
	numCL = 0;
    }

    /*
     * If necessary, concatenate the parts of the word.
     */

    while (numObjsToConcat > 255) {
	TclEmitInstInt1(INST_STR_CONCAT1, 255, envPtr);
	numObjsToConcat -= 254;	/* concat pushes 1 obj, the result */
    }
    if (numObjsToConcat > 1) {
	TclEmitInstInt1(INST_STR_CONCAT1, numObjsToConcat, envPtr);
    }

    /*
     * If the tokens yielded no instructions, push an empty string.
     */

    if (envPtr->codeNext == entryCodeNext) {
	PushStringLiteral(envPtr, "");
    }
    Tcl_DStringFree(&textBuffer);

    /*
     * Release the temp table we used to collect the locations of continuation
     * lines, if any.
     */

    if (maxNumCL) {
	ckfree(clPosition);
    }
    TclCheckStackDepth(depth+1, envPtr);
}

```

tcl9.0 9.0.4, revision `core9-0-4`, `generic/tclParse.c`, function `ParseTokens`, lines 1040–1256. Full-source SHA-256 `5937f0c6d669c1e1848a739f40e5d22b21a443a2da83e8b292c3e023bb8e59e0`; snippet SHA-256 `479a7d74bf25d9a3c057d3f427d36b38b62875c71c6f9d88a850ba868a94e4d4`; retained evidence `source-tcl9.0`.

```text
ParseTokens(
    const char *src,		/* First character to parse. */
    Tcl_Size numBytes,		/* Max number of bytes to scan. */
    int mask,			/* Specifies when to stop parsing. The parse
				 * stops at the first unquoted character whose
				 * CHAR_TYPE contains any of the bits in
				 * mask. */
    int flags,			/* OR-ed bits indicating what substitutions to
				 * perform: TCL_SUBST_COMMANDS,
				 * TCL_SUBST_VARIABLES, and
				 * TCL_SUBST_BACKSLASHES */
    Tcl_Parse *parsePtr)	/* Information about parse in progress.
				 * Updated with additional tokens and
				 * termination information. */
{
    char type;
    Tcl_Size originalTokens;
    int noSubstCmds = !(flags & TCL_SUBST_COMMANDS);
    int noSubstVars = !(flags & TCL_SUBST_VARIABLES);
    int noSubstBS = !(flags & TCL_SUBST_BACKSLASHES);
    Tcl_Token *tokenPtr;

    /*
     * Each iteration through the following loop adds one token of type
     * TCL_TOKEN_TEXT, TCL_TOKEN_BS, TCL_TOKEN_COMMAND, or TCL_TOKEN_VARIABLE
     * to parsePtr. For TCL_TOKEN_VARIABLE tokens, additional tokens are added
     * for the parsed variable name.
     */

    originalTokens = parsePtr->numTokens;
    while (numBytes && !((type = CHAR_TYPE(*src)) & mask)) {
	TclGrowParseTokenArray(parsePtr, 1);
	tokenPtr = &parsePtr->tokenPtr[parsePtr->numTokens];
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

	if ((type & TYPE_SUBS) == 0) {
	    /*
	     * This is a simple range of characters. Scan to find the end of
	     * the range.
	     */

	    while ((++src, --numBytes)
		    && !(CHAR_TYPE(*src) & (mask | TYPE_SUBS))) {
		/* empty loop */
	    }
	    tokenPtr->type = TCL_TOKEN_TEXT;
	    tokenPtr->size = src - tokenPtr->start;
	    parsePtr->numTokens++;
	} else if (*src == '$') {
	    Tcl_Size varToken;

	    if (noSubstVars) {
		tokenPtr->type = TCL_TOKEN_TEXT;
		tokenPtr->size = 1;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    /*
	     * This is a variable reference. Call Tcl_ParseVarName to do all
	     * the dirty work of parsing the name.
	     */

	    varToken = parsePtr->numTokens;
	    if (Tcl_ParseVarName(parsePtr->interp, src, numBytes, parsePtr,
		    1) != TCL_OK) {
		return TCL_ERROR;
	    }
	    src += parsePtr->tokenPtr[varToken].size;
	    numBytes -= parsePtr->tokenPtr[varToken].size;
	} else if (*src == '[') {
	    Tcl_Parse *nestedPtr;

	    if (noSubstCmds) {
		tokenPtr->type = TCL_TOKEN_TEXT;
		tokenPtr->size = 1;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    /*
	     * Command substitution. Call Tcl_ParseCommand recursively (and
	     * repeatedly) to parse the nested command(s), then throw away the
	     * parse information.
	     */

	    src++;
	    numBytes--;
	    nestedPtr = (Tcl_Parse *)TclStackAlloc(parsePtr->interp, sizeof(Tcl_Parse));
	    while (1) {
		const char *curEnd;

		if (Tcl_ParseCommand(parsePtr->interp, src, numBytes, 1,
			nestedPtr) != TCL_OK) {
		    parsePtr->errorType = nestedPtr->errorType;
		    parsePtr->term = nestedPtr->term;
		    parsePtr->incomplete = nestedPtr->incomplete;
		    TclStackFree(parsePtr->interp, nestedPtr);
		    return TCL_ERROR;
		}
		curEnd = src + numBytes;
		src = nestedPtr->commandStart + nestedPtr->commandSize;
		numBytes = curEnd - src;
		Tcl_FreeParse(nestedPtr);

		/*
		 * Check for the closing ']' that ends the command
		 * substitution. It must have been the last character of the
		 * parsed command.
		 */

		if ((nestedPtr->term < parsePtr->end)
			&& (*(nestedPtr->term) == ']')
			&& !(nestedPtr->incomplete)) {
		    break;
		}
		if (numBytes == 0) {
		    if (parsePtr->interp != NULL) {
			Tcl_SetObjResult(parsePtr->interp, Tcl_NewStringObj(
				"missing close-bracket", -1));
		    }
		    parsePtr->errorType = TCL_PARSE_MISSING_BRACKET;
		    parsePtr->term = tokenPtr->start;
		    parsePtr->incomplete = 1;
		    TclStackFree(parsePtr->interp, nestedPtr);
		    return TCL_ERROR;
		}
	    }
	    TclStackFree(parsePtr->interp, nestedPtr);
	    tokenPtr->type = TCL_TOKEN_COMMAND;
	    tokenPtr->size = src - tokenPtr->start;
	    parsePtr->numTokens++;
	} else if (*src == '\\') {
	    if (noSubstBS) {
		tokenPtr->type = TCL_TOKEN_TEXT;
		tokenPtr->size = 1;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    /*
	     * Backslash substitution.
	     */

	    TclParseBackslash(src, numBytes, &tokenPtr->size, NULL);

	    if (tokenPtr->size == 1) {
		/*
		 * Just a backslash, due to end of string.
		 */

		tokenPtr->type = TCL_TOKEN_TEXT;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    if (src[1] == '\n') {
		if (numBytes == 2) {
		    parsePtr->incomplete = 1;
		}

		/*
		 * Note: backslash-newline is special in that it is treated
		 * the same as a space character would be. This means that it
		 * could terminate the token.
		 */

		if (mask & TYPE_SPACE) {
		    if (parsePtr->numTokens == originalTokens) {
			goto finishToken;
		    }
		    break;
		}
	    }

	    tokenPtr->type = TCL_TOKEN_BS;
	    parsePtr->numTokens++;
	    src += tokenPtr->size;
	    numBytes -= tokenPtr->size;
	} else if (*src == 0) {
	    tokenPtr->type = TCL_TOKEN_TEXT;
	    tokenPtr->size = 1;
	    parsePtr->numTokens++;
	    src++;
	    numBytes--;
	} else {
	    Tcl_Panic("ParseTokens encountered unknown character");
	}
    }
    if (parsePtr->numTokens == originalTokens) {
	/*
	 * There was nothing in this range of text. Add an empty token for the
	 * empty range, so that there is always at least one token added.
	 */

	TclGrowParseTokenArray(parsePtr, 1);
	tokenPtr = &parsePtr->tokenPtr[parsePtr->numTokens];
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

    finishToken:
	tokenPtr->type = TCL_TOKEN_TEXT;
	tokenPtr->size = 0;
	parsePtr->numTokens++;
    }
    parsePtr->term = src;
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `core9-0-4`, `generic/tclParse.c`, function `Tcl_ParseVarName`, lines 1315–1512. Full-source SHA-256 `5937f0c6d669c1e1848a739f40e5d22b21a443a2da83e8b292c3e023bb8e59e0`; snippet SHA-256 `48adc96829325ff76f8dc1d02f8e48abf32a4f3c4e0929ac603b3335269d751c`; retained evidence `source-tcl9.0`.

```text
Tcl_ParseVarName(
    Tcl_Interp *interp,		/* Interpreter to use for error reporting; if
				 * NULL, then no error message is provided. */
    const char *start,		/* Start of variable substitution string.
				 * First character must be "$". */
    Tcl_Size numBytes,		/* Total number of bytes in string. If -1,
				 * the string consists of all bytes up to the
				 * first null character. */
    Tcl_Parse *parsePtr,	/* Structure to fill in with information about
				 * the variable name. */
    int append)			/* Non-zero means append tokens to existing
				 * information in parsePtr; zero means ignore
				 * existing tokens in parsePtr and
				 * reinitialize it. */
{
    Tcl_Token *tokenPtr;
    const char *src;
    int varIndex;
    unsigned array;

    if (numBytes < 0 && start) {
	numBytes = strlen(start);
    }
    if (!append) {
	TclParseInit(interp, start, numBytes, parsePtr);
    }
    if ((numBytes == 0) || (start == NULL)) {
	return TCL_ERROR;
    }

    /*
     * Generate one token for the variable, an additional token for the name,
     * plus any number of additional tokens for the index, if there is one.
     */

    src = start;
    TclGrowParseTokenArray(parsePtr, 2);
    tokenPtr = &parsePtr->tokenPtr[parsePtr->numTokens];
    tokenPtr->type = TCL_TOKEN_VARIABLE;
    tokenPtr->start = src;
    varIndex = parsePtr->numTokens;
    parsePtr->numTokens++;
    tokenPtr++;
    src++;
    numBytes--;
    if (numBytes == 0) {
	goto justADollarSign;
    }
    tokenPtr->type = TCL_TOKEN_TEXT;
    tokenPtr->start = src;
    tokenPtr->numComponents = 0;

    /*
     * The name of the variable can have three forms:
     * 1. The $ sign is followed by an open curly brace. Then the variable
     *	  name is everything up to the next close curly brace, and the
     *	  variable is a scalar variable.
     * 2. The $ sign is not followed by an open curly brace. Then the variable
     *	  name is everything up to the next character that isn't a letter,
     *	  digit, or underscore. :: sequences are also considered part of the
     *	  variable name, in order to support namespaces. If the following
     *	  character is an open parenthesis, then the information between
     *	  parentheses is the array element name.
     * 3. The $ sign is followed by something that isn't a letter, digit, or
     *	  underscore: in this case, there is no variable name and the token is
     *	  just "$".
     */

    if (*src == '{') {
	char ch;
	int braceCount = 0;
	src++;
	numBytes--;
	tokenPtr->type = TCL_TOKEN_TEXT;
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

	ch = *src;
	while (numBytes && (braceCount>0 || ch != '}')) {
	    switch (ch) {
	    case '{':
		braceCount++;
		break;
	    case '}':
		braceCount--;
		break;
	    case '\\':
		/* if 2 or more left, consume 2, else consume
		 * just the \ and let it run into the end */
		if (numBytes > 1) {
		   src++; numBytes--;
		}
	    }
	    numBytes--;
	    src++;
	    ch= *src;
	}
	if (numBytes == 0) {
	    if (parsePtr->interp != NULL) {
		Tcl_SetObjResult(parsePtr->interp, Tcl_NewStringObj(
			"missing close-brace for variable name", -1));
	    }
	    parsePtr->errorType = TCL_PARSE_MISSING_VAR_BRACE;
	    parsePtr->term = tokenPtr->start-1;
	    parsePtr->incomplete = 1;
	    goto error;
	}
	tokenPtr->size = src - tokenPtr->start;
	tokenPtr[-1].size = src - tokenPtr[-1].start;
	parsePtr->numTokens++;
	src++;
    } else {
	tokenPtr->type = TCL_TOKEN_TEXT;
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

	while (numBytes) {
	    if (TclIsBareword(*src)) {
		src += 1;
		numBytes -= 1;
		continue;
	    }
	    if ((src[0] == ':') && (numBytes != 1) && (src[1] == ':')) {
		src += 2;
		numBytes -= 2;
		while (numBytes && (*src == ':')) {
		    src++;
		    numBytes--;
		}
		continue;
	    }
	    break;
	}

	/*
	 * Support for empty array names here.
	 */

	array = (numBytes && (*src == '('));
	tokenPtr->size = src - tokenPtr->start;
	if ((tokenPtr->size == 0) && !array) {
	    goto justADollarSign;
	}
	parsePtr->numTokens++;
	if (array) {
	    /*
	     * This is a reference to an array element. Call ParseTokens
	     * recursively to parse the element name, since it could contain
	     * any number of substitutions.
	     */

	    if (TCL_OK != ParseTokens(src+1, numBytes-1, TYPE_BAD_ARRAY_INDEX,
		    TCL_SUBST_ALL, parsePtr)) {
		goto error;
	    }
	    if (parsePtr->term == src+numBytes){
		if (parsePtr->interp != NULL) {
		    Tcl_SetObjResult(parsePtr->interp, Tcl_NewStringObj(
			    "missing )", -1));
		}
		parsePtr->errorType = TCL_PARSE_MISSING_PAREN;
		parsePtr->term = src;
		parsePtr->incomplete = 1;
		goto error;
	    } else if ((*parsePtr->term != ')')){
		if (parsePtr->interp != NULL) {
		    Tcl_SetObjResult(parsePtr->interp, Tcl_NewStringObj(
			    "invalid character in array index", -1));
		}
		parsePtr->errorType = TCL_PARSE_SYNTAX;
		parsePtr->term = src;
		goto error;
	    }
	    src = parsePtr->term + 1;
	}
    }
    tokenPtr = &parsePtr->tokenPtr[varIndex];
    tokenPtr->size = src - tokenPtr->start;
    tokenPtr->numComponents = parsePtr->numTokens - (varIndex + 1);
    return TCL_OK;

    /*
     * The dollar sign isn't followed by a variable name. Replace the
     * TCL_TOKEN_VARIABLE token with a TCL_TOKEN_TEXT token for the dollar
     * sign.
     */

  justADollarSign:
    tokenPtr = &parsePtr->tokenPtr[varIndex];
    tokenPtr->type = TCL_TOKEN_TEXT;
    tokenPtr->size = 1;
    tokenPtr->numComponents = 0;
    return TCL_OK;

  error:
    Tcl_FreeParse(parsePtr);
    return TCL_ERROR;
}

```

tcl9.0 9.0.4, revision `core9-0-4`, `generic/tclCompCmdsSZ.c`, function `TclSubstCompile`, lines 1512–1758. Full-source SHA-256 `55f4cd62b44a84d795804041a5a19ebe9e8ab49095bdb063f6e139df1cc6b4ac`; snippet SHA-256 `3f5c721972dc511da5828d61cd4a15a4e552a03d4a3e55105929bb37be8aeb1f`; retained evidence `source-tcl9.0`.

```text
TclSubstCompile(
    Tcl_Interp *interp,
    const char *bytes,
    Tcl_Size numBytes,
    int flags,
    Tcl_Size line,
    CompileEnv *envPtr)
{
    Tcl_Token *endTokenPtr, *tokenPtr;
    int breakOffset = 0, count = 0;
    Tcl_Size bline = line;
    Tcl_Parse parse;
    Tcl_InterpState state = NULL;

    TclSubstParse(interp, bytes, numBytes, flags, &parse, &state);
    if (state != NULL) {
	Tcl_ResetResult(interp);
    }

    /*
     * Tricky point! If the first token does not result in a *guaranteed* push
     * of a Tcl_Obj on the stack, we must push an empty object. Otherwise it
     * is possible to get to an INST_STR_CONCAT1 or INST_DONE without enough
     * values on the stack, resulting in a crash. Thanks to Joe Mistachkin for
     * identifying a script that could trigger this case.
     */

    tokenPtr = parse.tokenPtr;
    if (tokenPtr->type != TCL_TOKEN_TEXT && tokenPtr->type != TCL_TOKEN_BS) {
	PUSH("");
	count++;
    }

    for (endTokenPtr = tokenPtr + parse.numTokens;
	    tokenPtr < endTokenPtr; tokenPtr = TokenAfter(tokenPtr)) {
	Tcl_Size length;
	int literal, catchRange, breakJump;
	char buf[4] = "";
	JumpFixup startFixup, okFixup, returnFixup, breakFixup;
	JumpFixup continueFixup, otherFixup, endFixup;

	switch (tokenPtr->type) {
	case TCL_TOKEN_TEXT:
	    literal = TclRegisterLiteral(envPtr,
		    tokenPtr->start, tokenPtr->size, 0);
	    TclEmitPush(literal, envPtr);
	    TclAdvanceLines(&bline, tokenPtr->start,
		    tokenPtr->start + tokenPtr->size);
	    count++;
	    continue;
	case TCL_TOKEN_BS:
	    length = TclParseBackslash(tokenPtr->start, tokenPtr->size,
		    NULL, buf);
	    literal = TclRegisterLiteral(envPtr, buf, length, 0);
	    TclEmitPush(literal, envPtr);
	    count++;
	    continue;
	case TCL_TOKEN_VARIABLE:
	    /*
	     * Check for simple variable access; see if we can only generate
	     * TCL_OK or TCL_ERROR from the substituted variable read; if so,
	     * there is no need to generate elaborate exception-management
	     * code. Note that the first component of TCL_TOKEN_VARIABLE is
	     * always TCL_TOKEN_TEXT...
	     */

	    if (tokenPtr->numComponents > 1) {
		Tcl_Size i;
		int foundCommand = 0;

		for (i=2 ; i<=tokenPtr->numComponents ; i++) {
		    if (tokenPtr[i].type == TCL_TOKEN_COMMAND) {
			foundCommand = 1;
			break;
		    }
		}
		if (foundCommand) {
		    break;
		}
	    }

	    envPtr->line = bline;
	    TclCompileVarSubst(interp, tokenPtr, envPtr);
	    bline = envPtr->line;
	    count++;
	    continue;
	}

	while (count > 255) {
	    OP1(		STR_CONCAT1, 255);
	    count -= 254;
	}
	if (count > 1) {
	    OP1(		STR_CONCAT1, count);
	    count = 1;
	}

	if (breakOffset == 0) {
	    /* Jump to the start (jump over the jump to end) */
	    TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &startFixup);

	    /* Jump to the end (all BREAKs land here) */
	    breakOffset = CurrentOffset(envPtr);
	    TclEmitInstInt4(INST_JUMP4, 0, envPtr);

	    /* Start */
	    if (TclFixupForwardJumpToHere(envPtr, &startFixup, 127)) {
		Tcl_Panic("TclCompileSubstCmd: bad start jump distance %" TCL_Z_MODIFIER "d",
			CurrentOffset(envPtr) - startFixup.codeOffset);
	    }
	}

	envPtr->line = bline;
	catchRange = TclCreateExceptRange(CATCH_EXCEPTION_RANGE, envPtr);
	OP4(	BEGIN_CATCH4, catchRange);
	ExceptionRangeStarts(envPtr, catchRange);

	switch (tokenPtr->type) {
	case TCL_TOKEN_COMMAND:
	    TclCompileScript(interp, tokenPtr->start+1, tokenPtr->size-2,
		    envPtr);
	    count++;
	    break;
	case TCL_TOKEN_VARIABLE:
	    TclCompileVarSubst(interp, tokenPtr, envPtr);
	    count++;
	    break;
	default:
	    Tcl_Panic("unexpected token type in TclCompileSubstCmd: %d",
		    tokenPtr->type);
	}

	ExceptionRangeEnds(envPtr, catchRange);

	/* Substitution produced TCL_OK */
	OP(	END_CATCH);
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &okFixup);
	TclAdjustStackDepth(-1, envPtr);

	/* Exceptional return codes processed here */
	ExceptionRangeTarget(envPtr, catchRange, catchOffset);
	OP(	PUSH_RETURN_OPTIONS);
	OP(	PUSH_RESULT);
	OP(	PUSH_RETURN_CODE);
	OP(	END_CATCH);
	OP(	RETURN_CODE_BRANCH);

	/* ERROR -> reraise it; NB: can't require BREAK/CONTINUE handling */
	OP(	RETURN_STK);
	OP(	NOP);

	/* RETURN */
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &returnFixup);

	/* BREAK */
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &breakFixup);

	/* CONTINUE */
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &continueFixup);

	/* OTHER */
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &otherFixup);

	TclAdjustStackDepth(1, envPtr);
	/* BREAK destination */
	if (TclFixupForwardJumpToHere(envPtr, &breakFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad break jump distance %" TCL_Z_MODIFIER "d",
		    CurrentOffset(envPtr) - breakFixup.codeOffset);
	}
	OP(	POP);
	OP(	POP);

	breakJump = CurrentOffset(envPtr) - breakOffset;
	if (breakJump > 127) {
	    OP4(JUMP4, -breakJump);
	} else {
	    OP1(JUMP1, -breakJump);
	}

	TclAdjustStackDepth(2, envPtr);
	/* CONTINUE destination */
	if (TclFixupForwardJumpToHere(envPtr, &continueFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad continue jump distance %" TCL_Z_MODIFIER "d",
		    CurrentOffset(envPtr) - continueFixup.codeOffset);
	}
	OP(	POP);
	OP(	POP);
	TclEmitForwardJump(envPtr, TCL_UNCONDITIONAL_JUMP, &endFixup);

	TclAdjustStackDepth(2, envPtr);
	/* RETURN + other destination */
	if (TclFixupForwardJumpToHere(envPtr, &returnFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad return jump distance %" TCL_Z_MODIFIER "d",
		    CurrentOffset(envPtr) - returnFixup.codeOffset);
	}
	if (TclFixupForwardJumpToHere(envPtr, &otherFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad other jump distance %" TCL_Z_MODIFIER "d",
		    CurrentOffset(envPtr) - otherFixup.codeOffset);
	}

	/*
	 * Pull the result to top of stack, discard options dict.
	 */

	OP4(	REVERSE, 2);
	OP(	POP);

	/* OK destination */
	if (TclFixupForwardJumpToHere(envPtr, &okFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad ok jump distance %" TCL_Z_MODIFIER "d",
		    CurrentOffset(envPtr) - okFixup.codeOffset);
	}
	if (count > 1) {
	    OP1(STR_CONCAT1, count);
	    count = 1;
	}

	/* CONTINUE jump to here */
	if (TclFixupForwardJumpToHere(envPtr, &endFixup, 127)) {
	    Tcl_Panic("TclCompileSubstCmd: bad end jump distance %" TCL_Z_MODIFIER "d",
		    CurrentOffset(envPtr) - endFixup.codeOffset);
	}
	bline = envPtr->line;
    }

    while (count > 255) {
	OP1(	STR_CONCAT1, 255);
	count -= 254;
    }
    if (count > 1) {
	OP1(	STR_CONCAT1, count);
    }

    Tcl_FreeParse(&parse);

    if (state != NULL) {
	Tcl_RestoreInterpState(interp, state);
	TclCompileSyntaxError(interp, envPtr);
	TclAdjustStackDepth(-1, envPtr);
    }

    /* Final target of the multi-jump from all BREAKs */
    if (breakOffset > 0) {
	TclUpdateInstInt4AtPc(INST_JUMP4, CurrentOffset(envPtr) - breakOffset,
		envPtr->codeStart + breakOffset);
    }
}

```

tcl9.0 9.0.4, revision `core9-0-4`, `generic/tclCompile.c`, function `TclCompileTokens`, lines 2416–2610. Full-source SHA-256 `51b709e7bf2c1f25bb7b3ab371c7dcbd6cf6d2bce509931397303cb770ee21f3`; snippet SHA-256 `2f8084251e4df3b7d823a494d9d8b1e7b38ec050a58ca905efab16dd0b52ed33`; retained evidence `source-tcl9.0`.

```text
TclCompileTokens(
    Tcl_Interp *interp,		/* Used for error and status reporting. */
    Tcl_Token *tokenPtr,	/* Pointer to first in an array of tokens to
				 * compile. */
    Tcl_Size count,		/* Number of tokens to consider at tokenPtr.
				 * Must be at least 1. */
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    Tcl_DString textBuffer;	/* Holds concatenated chars from adjacent
				 * TCL_TOKEN_TEXT, TCL_TOKEN_BS tokens. */
    char buffer[4] = "";
    Tcl_Size i, numObjsToConcat, adjust;
    int length;
    unsigned char *entryCodeNext = envPtr->codeNext;
#define NUM_STATIC_POS 20
    int isLiteral;
    Tcl_Size maxNumCL, numCL;
    Tcl_Size *clPosition = NULL;
    int depth = TclGetStackDepth(envPtr);

    /*
     * If this is actually a literal, handle continuation lines by
     * preallocating a small table to store the locations of any continuation
     * lines found in this literal.  The table is extended if needed.
     *
     * Note: In contrast with the analagous code in 'TclSubstTokens()' the
     * 'adjust' variable seems unneeded here.  The code which merges
     * continuation line information of multiple words which concat'd at
     * runtime also seems unneeded. Either that or I have not managed to find a
     * test case for these two possibilities yet.  It might be a difference
     * between compile- versus run-time processing.
     */

    numCL = 0;
    maxNumCL = 0;
    isLiteral = 1;
    for (i=0 ; i < count; i++) {
	if ((tokenPtr[i].type != TCL_TOKEN_TEXT)
		&& (tokenPtr[i].type != TCL_TOKEN_BS)) {
	    isLiteral = 0;
	    break;
	}
    }

    if (isLiteral) {
	maxNumCL = NUM_STATIC_POS;
	clPosition = (Tcl_Size *)Tcl_Alloc(maxNumCL * sizeof(Tcl_Size));
    }

    adjust = 0;
    Tcl_DStringInit(&textBuffer);
    numObjsToConcat = 0;
    for ( ;  count > 0;  count--, tokenPtr++) {
	switch (tokenPtr->type) {
	case TCL_TOKEN_TEXT:
	    TclDStringAppendToken(&textBuffer, tokenPtr);
	    TclAdvanceLines(&envPtr->line, tokenPtr->start,
		    tokenPtr->start + tokenPtr->size);
	    break;

	case TCL_TOKEN_BS:
	    length = TclParseBackslash(tokenPtr->start, tokenPtr->size,
		    NULL, buffer);
	    Tcl_DStringAppend(&textBuffer, buffer, length);

	    /*
	     * If the identified backslash sequence is in a literal and
	     * represented a continuation line, compute and store its
	     * location (as char offset to the beginning of the _result_
	     * script). We may have to extend the table of locations.
	     *
	     * The continuation line information is relevant even if the word
	     * being processed is not a literal, as it can affect nested
	     * commands. See the branch below for TCL_TOKEN_COMMAND, where the
	     * adjustment being tracked here is taken into account. The good
	     * thing is a table of everything is not needed, just the number of
	     * lines to add as correction.
	     */

	    if ((length == 1) && (buffer[0] == ' ') &&
		    (tokenPtr->start[1] == '\n')) {
		if (isLiteral) {
		    int clPos = Tcl_DStringLength(&textBuffer);

		    if (numCL >= maxNumCL) {
			maxNumCL *= 2;
			clPosition = (Tcl_Size *)Tcl_Realloc(clPosition,
				maxNumCL * sizeof(Tcl_Size));
		    }
		    clPosition[numCL] = clPos;
		    numCL ++;
		}
		adjust++;
	    }
	    break;

	case TCL_TOKEN_COMMAND:
	    /*
	     * Push any accumulated chars appearing before the command.
	     */

	    if (Tcl_DStringLength(&textBuffer) > 0) {
		int literal = TclRegisterDStringLiteral(envPtr, &textBuffer);

		TclEmitPush(literal, envPtr);
		numObjsToConcat++;
		Tcl_DStringFree(&textBuffer);

		if (numCL) {
		    TclContinuationsEnter(TclFetchLiteral(envPtr, literal),
			    numCL, clPosition);
		}
		numCL = 0;
	    }

	    envPtr->line += adjust;
	    TclCompileScript(interp, tokenPtr->start+1,
		    tokenPtr->size-2, envPtr);
	    envPtr->line -= adjust;
	    numObjsToConcat++;
	    break;

	case TCL_TOKEN_VARIABLE:
	    /*
	     * Push any accumulated chars appearing before the $<var>.
	     */

	    if (Tcl_DStringLength(&textBuffer) > 0) {
		int literal;

		literal = TclRegisterDStringLiteral(envPtr, &textBuffer);
		TclEmitPush(literal, envPtr);
		numObjsToConcat++;
		Tcl_DStringFree(&textBuffer);
	    }

	    TclCompileVarSubst(interp, tokenPtr, envPtr);
	    numObjsToConcat++;
	    count -= tokenPtr->numComponents;
	    tokenPtr += tokenPtr->numComponents;
	    break;

	default:
	    Tcl_Panic("Unexpected token type in TclCompileTokens: %d; %.*s",
		    tokenPtr->type, (int)tokenPtr->size, tokenPtr->start);
	}
    }

    /*
     * Push any accumulated characters appearing at the end.
     */

    if (Tcl_DStringLength(&textBuffer) > 0) {
	int literal = TclRegisterDStringLiteral(envPtr, &textBuffer);

	TclEmitPush(literal, envPtr);
	numObjsToConcat++;
	if (numCL) {
	    TclContinuationsEnter(TclFetchLiteral(envPtr, literal),
		    numCL, clPosition);
	}
	numCL = 0;
    }

    /*
     * If necessary, concatenate the parts of the word.
     */

    while (numObjsToConcat > 255) {
	TclEmitInstInt1(INST_STR_CONCAT1, 255, envPtr);
	numObjsToConcat -= 254;	/* concat pushes 1 obj, the result */
    }
    if (numObjsToConcat > 1) {
	TclEmitInstInt1(INST_STR_CONCAT1, numObjsToConcat, envPtr);
    }

    /*
     * If the tokens yielded no instructions, push an empty string.
     */

    if (envPtr->codeNext == entryCodeNext) {
	PushStringLiteral(envPtr, "");
    }
    Tcl_DStringFree(&textBuffer);

    /*
     * Release the temp table we used to collect the locations of continuation
     * lines, if any.
     */

    if (maxNumCL) {
	Tcl_Free(clPosition);
    }
    TclCheckStackDepth(depth+1, envPtr);
}

```

tcl9.1 9.1.0, revision `core9-1-0`, `generic/tclParse.c`, function `ParseTokens`, lines 1039–1252. Full-source SHA-256 `914176e1d5a1fb79630a45f5f45df438da9943281751855057caa17e921b969f`; snippet SHA-256 `1729687b120ce3cb8c2dbdfcb41dae9f3118b5446204d55ee27e5efe64268ca5`; retained evidence `source-tcl9.1`.

```text
ParseTokens(
    const char *src,		/* First character to parse. */
    Tcl_Size numBytes,		/* Max number of bytes to scan. */
    int mask,			/* Specifies when to stop parsing. The parse
				 * stops at the first unquoted character whose
				 * CHAR_TYPE contains any of the bits in
				 * mask. */
    int flags,			/* OR-ed bits indicating what substitutions to
				 * perform: TCL_SUBST_COMMANDS,
				 * TCL_SUBST_VARIABLES, and
				 * TCL_SUBST_BACKSLASHES */
    Tcl_Parse *parsePtr)	/* Information about parse in progress.
				 * Updated with additional tokens and
				 * termination information. */
{
    unsigned char type;
    Tcl_Size originalTokens;
    Tcl_Token *tokenPtr;

    /*
     * Each iteration through the following loop adds one token of type
     * TCL_TOKEN_TEXT, TCL_TOKEN_BS, TCL_TOKEN_COMMAND, or TCL_TOKEN_VARIABLE
     * to parsePtr. For TCL_TOKEN_VARIABLE tokens, additional tokens are added
     * for the parsed variable name.
     */

    originalTokens = parsePtr->numTokens;
    while (numBytes && !((type = CHAR_TYPE(*src)) & mask)) {
	TclGrowParseTokenArray(parsePtr, 1);
	tokenPtr = &parsePtr->tokenPtr[parsePtr->numTokens];
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

	if ((type & TYPE_SUBS) == 0) {
	    /*
	     * This is a simple range of characters. Scan to find the end of
	     * the range.
	     */

	    while ((++src, --numBytes)
		    && !(CHAR_TYPE(*src) & (mask | TYPE_SUBS))) {
		/* empty loop */
	    }
	    tokenPtr->type = TCL_TOKEN_TEXT;
	    tokenPtr->size = src - tokenPtr->start;
	    parsePtr->numTokens++;
	} else if (*src == '$') {
	    Tcl_Size varToken;

	    if (!(flags & TCL_SUBST_VARIABLES)) {
		tokenPtr->type = TCL_TOKEN_TEXT;
		tokenPtr->size = 1;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    /*
	     * This is a variable reference. Call Tcl_ParseVarName to do all
	     * the dirty work of parsing the name.
	     */

	    varToken = parsePtr->numTokens;
	    if (Tcl_ParseVarName(parsePtr->interp, src, numBytes, parsePtr,
		    1) != TCL_OK) {
		return TCL_ERROR;
	    }
	    src += parsePtr->tokenPtr[varToken].size;
	    numBytes -= parsePtr->tokenPtr[varToken].size;
	} else if (*src == '[') {
	    Tcl_Parse *nestedPtr;

	    if (!(flags & TCL_SUBST_COMMANDS)) {
		tokenPtr->type = TCL_TOKEN_TEXT;
		tokenPtr->size = 1;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    /*
	     * Command substitution. Call Tcl_ParseCommand recursively (and
	     * repeatedly) to parse the nested command(s), then throw away the
	     * parse information.
	     */

	    src++;
	    numBytes--;
	    nestedPtr = (Tcl_Parse *)TclStackAlloc(parsePtr->interp, sizeof(Tcl_Parse));
	    while (true) {
		const char *curEnd;

		if (Tcl_ParseCommand(parsePtr->interp, src, numBytes, 1,
			nestedPtr) != TCL_OK) {
		    parsePtr->errorType = nestedPtr->errorType;
		    parsePtr->term = nestedPtr->term;
		    parsePtr->incomplete = nestedPtr->incomplete;
		    TclStackFree(parsePtr->interp, nestedPtr);
		    return TCL_ERROR;
		}
		curEnd = src + numBytes;
		src = nestedPtr->commandStart + nestedPtr->commandSize;
		numBytes = curEnd - src;
		Tcl_FreeParse(nestedPtr);

		/*
		 * Check for the closing ']' that ends the command
		 * substitution. It must have been the last character of the
		 * parsed command.
		 */

		if ((nestedPtr->term < parsePtr->end)
			&& (*(nestedPtr->term) == ']')
			&& !(nestedPtr->incomplete)) {
		    break;
		}
		if (numBytes == 0) {
		    if (parsePtr->interp != NULL) {
			Tcl_SetObjResult(parsePtr->interp, Tcl_NewStringObj(
				"missing close-bracket", -1));
		    }
		    parsePtr->errorType = TCL_PARSE_MISSING_BRACKET;
		    parsePtr->term = tokenPtr->start;
		    parsePtr->incomplete = 1;
		    TclStackFree(parsePtr->interp, nestedPtr);
		    return TCL_ERROR;
		}
	    }
	    TclStackFree(parsePtr->interp, nestedPtr);
	    tokenPtr->type = TCL_TOKEN_COMMAND;
	    tokenPtr->size = src - tokenPtr->start;
	    parsePtr->numTokens++;
	} else if (*src == '\\') {
	    if (!(flags & TCL_SUBST_BACKSLASHES)) {
		tokenPtr->type = TCL_TOKEN_TEXT;
		tokenPtr->size = 1;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    /*
	     * Backslash substitution.
	     */

	    TclParseBackslash(src, numBytes, &tokenPtr->size, NULL);

	    if (tokenPtr->size == 1) {
		/*
		 * Just a backslash, due to end of string.
		 */

		tokenPtr->type = TCL_TOKEN_TEXT;
		parsePtr->numTokens++;
		src++;
		numBytes--;
		continue;
	    }

	    if (src[1] == '\n') {
		if (numBytes == 2) {
		    parsePtr->incomplete = 1;
		}

		/*
		 * Note: backslash-newline is special in that it is treated
		 * the same as a space character would be. This means that it
		 * could terminate the token.
		 */

		if (mask & TYPE_SPACE) {
		    if (parsePtr->numTokens == originalTokens) {
			goto finishToken;
		    }
		    break;
		}
	    }

	    tokenPtr->type = TCL_TOKEN_BS;
	    parsePtr->numTokens++;
	    src += tokenPtr->size;
	    numBytes -= tokenPtr->size;
	} else if (*src == 0) {
	    tokenPtr->type = TCL_TOKEN_TEXT;
	    tokenPtr->size = 1;
	    parsePtr->numTokens++;
	    src++;
	    numBytes--;
	} else {
	    Tcl_Panic("ParseTokens encountered unknown character");
	}
    }
    if (parsePtr->numTokens == originalTokens) {
	/*
	 * There was nothing in this range of text. Add an empty token for the
	 * empty range, so that there is always at least one token added.
	 */

	TclGrowParseTokenArray(parsePtr, 1);
	tokenPtr = &parsePtr->tokenPtr[parsePtr->numTokens];
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

    finishToken:
	tokenPtr->type = TCL_TOKEN_TEXT;
	tokenPtr->size = 0;
	parsePtr->numTokens++;
    }
    parsePtr->term = src;
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `core9-1-0`, `generic/tclParse.c`, function `Tcl_ParseVarName`, lines 1311–1509. Full-source SHA-256 `914176e1d5a1fb79630a45f5f45df438da9943281751855057caa17e921b969f`; snippet SHA-256 `7b015100e701db035720761d63ded16ef5f84ffc922d6941022e99d3e6e023cf`; retained evidence `source-tcl9.1`.

```text
Tcl_ParseVarName(
    Tcl_Interp *interp,		/* Interpreter to use for error reporting; if
				 * NULL, then no error message is provided. */
    const char *start,		/* Start of variable substitution string.
				 * First character must be "$". */
    Tcl_Size numBytes,		/* Total number of bytes in string. If -1,
				 * the string consists of all bytes up to the
				 * first null character. */
    Tcl_Parse *parsePtr,	/* Structure to fill in with information about
				 * the variable name. */
    int append)			/* Non-zero means append tokens to existing
				 * information in parsePtr; zero means ignore
				 * existing tokens in parsePtr and
				 * reinitialize it. */
{
    Tcl_Token *tokenPtr;
    const char *src;
    Tcl_Size varIndex;
    unsigned array;

    if (numBytes < 0 && start) {
	numBytes = strlen(start);
    }
    if (!append) {
	TclParseInit(interp, start, numBytes, parsePtr);
    }
    if ((numBytes == 0) || (start == NULL)) {
	return TCL_ERROR;
    }

    /*
     * Generate one token for the variable, an additional token for the name,
     * plus any number of additional tokens for the index, if there is one.
     */

    src = start;
    TclGrowParseTokenArray(parsePtr, 2);
    tokenPtr = &parsePtr->tokenPtr[parsePtr->numTokens];
    tokenPtr->type = TCL_TOKEN_VARIABLE;
    tokenPtr->start = src;
    varIndex = parsePtr->numTokens;
    parsePtr->numTokens++;
    tokenPtr++;
    src++;
    numBytes--;
    if (numBytes == 0) {
	goto justADollarSign;
    }
    tokenPtr->type = TCL_TOKEN_TEXT;
    tokenPtr->start = src;
    tokenPtr->numComponents = 0;

    /*
     * The name of the variable can have three forms:
     * 1. The $ sign is followed by an open curly brace. Then the variable
     *	  name is everything up to the next close curly brace, and the
     *	  variable is a scalar variable.
     * 2. The $ sign is not followed by an open curly brace. Then the variable
     *	  name is everything up to the next character that isn't a letter,
     *	  digit, or underscore. :: sequences are also considered part of the
     *	  variable name, in order to support namespaces. If the following
     *	  character is an open parenthesis, then the information between
     *	  parentheses is the array element name.
     * 3. The $ sign is followed by something that isn't a letter, digit, or
     *	  underscore: in this case, there is no variable name and the token is
     *	  just "$".
     */

    if (*src == '{') {
	char ch;
	int braceCount = 0;
	src++;
	numBytes--;
	tokenPtr->type = TCL_TOKEN_TEXT;
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

	ch = *src;
	while (numBytes && (braceCount>0 || ch != '}')) {
	    switch (ch) {
	    case '{':
		braceCount++;
		break;
	    case '}':
		braceCount--;
		break;
	    case '\\':
		/* if 2 or more left, consume 2, else consume
		 * just the \ and let it run into the end */
		if (numBytes > 1) {
		    src++;
		    numBytes--;
		}
	    }
	    numBytes--;
	    src++;
	    ch= *src;
	}
	if (numBytes == 0) {
	    if (parsePtr->interp != NULL) {
		Tcl_SetObjResult(parsePtr->interp, Tcl_NewStringObj(
			"missing close-brace for variable name", -1));
	    }
	    parsePtr->errorType = TCL_PARSE_MISSING_VAR_BRACE;
	    parsePtr->term = tokenPtr->start-1;
	    parsePtr->incomplete = 1;
	    goto error;
	}
	tokenPtr->size = src - tokenPtr->start;
	tokenPtr[-1].size = src - tokenPtr[-1].start;
	parsePtr->numTokens++;
	src++;
    } else {
	tokenPtr->type = TCL_TOKEN_TEXT;
	tokenPtr->start = src;
	tokenPtr->numComponents = 0;

	while (numBytes) {
	    if (TclIsBareword(*src)) {
		src += 1;
		numBytes -= 1;
		continue;
	    }
	    if ((src[0] == ':') && (numBytes != 1) && (src[1] == ':')) {
		src += 2;
		numBytes -= 2;
		while (numBytes && (*src == ':')) {
		    src++;
		    numBytes--;
		}
		continue;
	    }
	    break;
	}

	/*
	 * Support for empty array names here.
	 */

	array = (numBytes && (*src == '('));
	tokenPtr->size = src - tokenPtr->start;
	if ((tokenPtr->size == 0) && !array) {
	    goto justADollarSign;
	}
	parsePtr->numTokens++;
	if (array) {
	    /*
	     * This is a reference to an array element. Call ParseTokens
	     * recursively to parse the element name, since it could contain
	     * any number of substitutions.
	     */

	    if (TCL_OK != ParseTokens(src+1, numBytes-1, TYPE_BAD_ARRAY_INDEX,
		    TCL_SUBST_ALL, parsePtr)) {
		goto error;
	    }
	    if (parsePtr->term == src+numBytes){
		if (parsePtr->interp != NULL) {
		    Tcl_SetObjResult(parsePtr->interp, Tcl_NewStringObj(
			    "missing )", -1));
		}
		parsePtr->errorType = TCL_PARSE_MISSING_PAREN;
		parsePtr->term = src;
		parsePtr->incomplete = 1;
		goto error;
	    } else if ((*parsePtr->term != ')')){
		if (parsePtr->interp != NULL) {
		    Tcl_SetObjResult(parsePtr->interp, Tcl_NewStringObj(
			    "invalid character in array index", -1));
		}
		parsePtr->errorType = TCL_PARSE_SYNTAX;
		parsePtr->term = src;
		goto error;
	    }
	    src = parsePtr->term + 1;
	}
    }
    tokenPtr = &parsePtr->tokenPtr[varIndex];
    tokenPtr->size = src - tokenPtr->start;
    tokenPtr->numComponents = parsePtr->numTokens - (varIndex + 1);
    return TCL_OK;

    /*
     * The dollar sign isn't followed by a variable name. Replace the
     * TCL_TOKEN_VARIABLE token with a TCL_TOKEN_TEXT token for the dollar
     * sign.
     */

  justADollarSign:
    tokenPtr = &parsePtr->tokenPtr[varIndex];
    tokenPtr->type = TCL_TOKEN_TEXT;
    tokenPtr->size = 1;
    tokenPtr->numComponents = 0;
    return TCL_OK;

  error:
    Tcl_FreeParse(parsePtr);
    return TCL_ERROR;
}

```

tcl9.1 9.1.0, revision `core9-1-0`, `generic/tclCompCmdsSZ.c`, function `TclSubstCompile`, lines 1493–1695. Full-source SHA-256 `931ca711522e331cc9f34aff756e770824a86ec97567197f5aa894ab90fc585a`; snippet SHA-256 `64595211dc576ab048fd702a7da9fc4dc01622185305bff48c1b5a6155a10861`; retained evidence `source-tcl9.1`.

```text
TclSubstCompile(
    Tcl_Interp *interp,
    const char *bytes,
    Tcl_Size numBytes,
    int flags,
    int line,
    CompileEnv *envPtr)
{
    Tcl_Token *endTokenPtr, *tokenPtr;
    Tcl_BytecodeLabel breakOffset = 0;
    Tcl_Size count = 0;
    int bline = line;
    Tcl_Parse parse;
    Tcl_InterpState state = NULL;

    TclSubstParse(interp, bytes, numBytes, flags, &parse, &state);
    if (state != NULL) {
	Tcl_ResetResult(interp);
    }

    /*
     * Tricky point! If the first token does not result in a *guaranteed* push
     * of a Tcl_Obj on the stack, we must push an empty object. Otherwise it
     * is possible to get to an INST_STR_CONCAT1 or INST_DONE without enough
     * values on the stack, resulting in a crash. Thanks to Joe Mistachkin for
     * identifying a script that could trigger this case.
     */

    tokenPtr = parse.tokenPtr;
    if (tokenPtr->type != TCL_TOKEN_TEXT && tokenPtr->type != TCL_TOKEN_BS) {
	PUSH(			"");
	count++;
    }

    for (endTokenPtr = tokenPtr + parse.numTokens;
	    tokenPtr < endTokenPtr; tokenPtr = TokenAfter(tokenPtr)) {
	Tcl_Size length;
	Tcl_ExceptionRange catchRange;
	Tcl_BytecodeLabel end, haveOk, haveOther, tableBase;
	JumptableNumInfo *retCodeTable;
	Tcl_AuxDataRef tableIdx;
	char buf[4] = "";

	switch (tokenPtr->type) {
	case TCL_TOKEN_TEXT:
	    PushLiteral(envPtr, tokenPtr->start, tokenPtr->size);
	    TclAdvanceLines(&bline, tokenPtr->start,
		    tokenPtr->start + tokenPtr->size);
	    count++;
	    continue;
	case TCL_TOKEN_BS:
	    length = TclParseBackslash(tokenPtr->start, tokenPtr->size,
		    NULL, buf);
	    PushLiteral(envPtr, buf, length);
	    count++;
	    continue;
	case TCL_TOKEN_VARIABLE:
	    /*
	     * Check for simple variable access; see if we can only generate
	     * TCL_OK or TCL_ERROR from the substituted variable read; if so,
	     * there is no need to generate elaborate exception-management
	     * code. Note that the first component of TCL_TOKEN_VARIABLE is
	     * always TCL_TOKEN_TEXT...
	     */

	    if (tokenPtr->numComponents > 1) {
		Tcl_Size i;
		int foundCommand = 0;

		for (i=2 ; i<=tokenPtr->numComponents ; i++) {
		    if (tokenPtr[i].type == TCL_TOKEN_COMMAND) {
			foundCommand = 1;
			break;
		    }
		}
		if (foundCommand) {
		    break;
		}
	    }

	    envPtr->line = bline;
	    TclCompileVarSubst(interp, tokenPtr, envPtr);
	    bline = envPtr->line;
	    count++;
	    continue;
	}

	while (count > 255) {
	    OP1(		STR_CONCAT1, 255);
	    count -= 254;
	}
	if (count > 1) {
	    OP1(		STR_CONCAT1, count);
	    count = 1;
	}

	if (breakOffset == 0) {
	    Tcl_BytecodeLabel start;
	    /* Jump to the start (jump over the jump to end) */
	    FWDJUMP(		JUMP, start);

	    /* Jump to the end (all BREAKs land here) */
	    FWDJUMP(		JUMP, breakOffset);

	    /* Start */
	    FWDLABEL(	start);
	}

	envPtr->line = bline;
	catchRange = MAKE_CATCH_RANGE();
	OP4(			BEGIN_CATCH, catchRange);
	CATCH_RANGE(catchRange) {
	    switch (tokenPtr->type) {
	    case TCL_TOKEN_COMMAND:
		TclCompileScript(interp, tokenPtr->start+1, tokenPtr->size-2,
			envPtr);
		count++;
		break;
	    case TCL_TOKEN_VARIABLE:
		TclCompileVarSubst(interp, tokenPtr, envPtr);
		count++;
		break;
	    default:
		Tcl_Panic("unexpected token type in TclCompileSubstCmd: %d",
			tokenPtr->type);
	    }
	}

	/* Substitution produced TCL_OK */
	OP(			END_CATCH);
	FWDJUMP(		JUMP, haveOk);
	STKDELTA(-1);

	/* Exceptional return codes processed here */
	CATCH_TARGET(	catchRange);
	OP(			PUSH_RETURN_CODE);

	retCodeTable = AllocJumptableNum();
	tableIdx = RegisterJumptableNum(retCodeTable, envPtr);
	tableBase = CurrentOffset(envPtr);
	OP4(			JUMP_TABLE_NUM, tableIdx);
	FWDJUMP(		JUMP, haveOther);

	/* ERROR -> reraise it; NB: can't require BREAK/CONTINUE handling */
	CreateJumptableNumEntryToHere(retCodeTable, TCL_ERROR, tableBase);
	OP(			PUSH_RETURN_OPTIONS);
	OP(			PUSH_RESULT);
	OP(			END_CATCH); // catchRange
	OP(			RETURN_STK);
	STKDELTA(-1);

	/* BREAK destination */
	CreateJumptableNumEntryToHere(retCodeTable, TCL_BREAK, tableBase);
	OP(			END_CATCH); // catchRange
	BACKJUMP(		JUMP, breakOffset);

	/* CONTINUE destination */
	CreateJumptableNumEntryToHere(retCodeTable, TCL_CONTINUE, tableBase);
	OP(			END_CATCH); // catchRange
	FWDJUMP(		JUMP, end);

	/* RETURN + other destination */
	FWDLABEL(	haveOther);
	OP(			PUSH_RESULT);
	OP(			END_CATCH); // catchRange

	/*
	 * Pull the result to top of stack, discard options dict.
	 */

	/* OK destination */
	FWDLABEL(	haveOk);
	if (count > 1) {
	    OP1(		STR_CONCAT1, count);
	    count = 1;
	}

	/* CONTINUE jump to here */
	FWDLABEL(		end);
	bline = envPtr->line;
    }

    while (count > 255) {
	OP1(			STR_CONCAT1, 255);
	count -= 254;
    }
    if (count > 1) {
	OP1(			STR_CONCAT1, count);
    }

    Tcl_FreeParse(&parse);

    if (state != NULL) {
	Tcl_RestoreInterpState(interp, state);
	TclCompileSyntaxError(interp, envPtr);
	STKDELTA(-1);
    }

    /* Final target of the multi-jump from all BREAKs */
    if (breakOffset > 0) {
	FWDLABEL(		breakOffset);
    }
}

```

tcl9.1 9.1.0, revision `core9-1-0`, `generic/tclCompile.c`, function `TclCompileTokens`, lines 2739–2927. Full-source SHA-256 `941eaa2c57e4d7d5266134c5b713dca5218d29eea951c89a0676c14b8260eb4f`; snippet SHA-256 `0192854ccad7c779a5db78f6ab91a9c47d6edcf50c2ffd889e0384ffab8ee5ba`; retained evidence `source-tcl9.1`.

```text
TclCompileTokens(
    Tcl_Interp *interp,		/* Used for error and status reporting. */
    Tcl_Token *tokenPtr,	/* Pointer to first in an array of tokens to
				 * compile. */
    Tcl_Size count,		/* Number of tokens to consider at tokenPtr.
				 * Must be at least 1. */
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    Tcl_DString textBuffer;	/* Holds concatenated chars from adjacent
				 * TCL_TOKEN_TEXT, TCL_TOKEN_BS tokens. */
    char buffer[4] = "";
    Tcl_Size i, numObjsToConcat, adjust;
    Tcl_Size length;
    unsigned char *entryCodeNext = envPtr->codeNext;
#define NUM_STATIC_POS 20
    int isLiteral;
    Tcl_Size maxNumCL, numCL;
    Tcl_Size *clPosition = NULL;
    Tcl_Size depth = TclGetStackDepth(envPtr);

    /*
     * If this is actually a literal, handle continuation lines by
     * preallocating a small table to store the locations of any continuation
     * lines found in this literal.  The table is extended if needed.
     *
     * Note: In contrast with the analagous code in 'TclSubstTokens()' the
     * 'adjust' variable seems unneeded here.  The code which merges
     * continuation line information of multiple words which concat'd at
     * runtime also seems unneeded. Either that or I have not managed to find a
     * test case for these two possibilities yet.  It might be a difference
     * between compile- versus run-time processing.
     */

    numCL = 0;
    maxNumCL = 0;
    isLiteral = 1;
    for (i=0 ; i < count; i++) {
	if ((tokenPtr[i].type != TCL_TOKEN_TEXT)
		&& (tokenPtr[i].type != TCL_TOKEN_BS)) {
	    isLiteral = 0;
	    break;
	}
    }

    if (isLiteral) {
	maxNumCL = NUM_STATIC_POS;
	clPosition = (Tcl_Size *)Tcl_Alloc(maxNumCL * sizeof(Tcl_Size));
    }

    adjust = 0;
    Tcl_DStringInit(&textBuffer);
    numObjsToConcat = 0;
    for ( ;  count > 0;  count--, tokenPtr++) {
	switch (tokenPtr->type) {
	case TCL_TOKEN_TEXT:
	    TclDStringAppendToken(&textBuffer, tokenPtr);
	    TclAdvanceLines(&envPtr->line, tokenPtr->start,
		    tokenPtr->start + tokenPtr->size);
	    break;

	case TCL_TOKEN_BS:
	    length = TclParseBackslash(tokenPtr->start, tokenPtr->size,
		    NULL, buffer);
	    Tcl_DStringAppend(&textBuffer, buffer, length);

	    /*
	     * If the identified backslash sequence is in a literal and
	     * represented a continuation line, compute and store its
	     * location (as char offset to the beginning of the _result_
	     * script). We may have to extend the table of locations.
	     *
	     * The continuation line information is relevant even if the word
	     * being processed is not a literal, as it can affect nested
	     * commands. See the branch below for TCL_TOKEN_COMMAND, where the
	     * adjustment being tracked here is taken into account. The good
	     * thing is a table of everything is not needed, just the number of
	     * lines to add as correction.
	     */

	    if ((length == 1) && (buffer[0] == ' ') &&
		    (tokenPtr->start[1] == '\n')) {
		if (isLiteral) {
		    Tcl_Size clPos = Tcl_DStringLength(&textBuffer);

		    if (numCL >= maxNumCL) {
			maxNumCL *= 2;
			clPosition = (Tcl_Size *)Tcl_Realloc(clPosition,
				maxNumCL * sizeof(Tcl_Size));
		    }
		    clPosition[numCL] = clPos;
		    numCL ++;
		}
		adjust++;
	    }
	    break;

	case TCL_TOKEN_COMMAND:
	    /*
	     * Push any accumulated chars appearing before the command.
	     */

	    if (Tcl_DStringLength(&textBuffer) > 0) {
		int literal = TclPushDString(envPtr, &textBuffer);

		numObjsToConcat++;
		Tcl_DStringFree(&textBuffer);
		if (numCL) {
		    TclContinuationsEnter(TclFetchLiteral(envPtr, literal),
			    numCL, clPosition);
		}
		numCL = 0;
	    }

	    envPtr->line += adjust;
	    TclCompileScript(interp, tokenPtr->start+1,
		    tokenPtr->size-2, envPtr);
	    envPtr->line -= adjust;
	    numObjsToConcat++;
	    break;

	case TCL_TOKEN_VARIABLE:
	    /*
	     * Push any accumulated chars appearing before the $<var>.
	     */

	    if (Tcl_DStringLength(&textBuffer) > 0) {
		TclPushDString(envPtr, &textBuffer);
		numObjsToConcat++;
		Tcl_DStringFree(&textBuffer);
	    }

	    TclCompileVarSubst(interp, tokenPtr, envPtr);
	    numObjsToConcat++;
	    count -= tokenPtr->numComponents;
	    tokenPtr += tokenPtr->numComponents;
	    break;

	default:
	    Tcl_Panic("Unexpected token type in TclCompileTokens: %d; %.*s",
		    tokenPtr->type, (int)tokenPtr->size, tokenPtr->start);
	}
    }

    /*
     * Push any accumulated characters appearing at the end.
     */

    if (Tcl_DStringLength(&textBuffer) > 0) {
	int literal = TclPushDString(envPtr, &textBuffer);

	numObjsToConcat++;
	if (numCL) {
	    TclContinuationsEnter(TclFetchLiteral(envPtr, literal),
		    numCL, clPosition);
	}
	numCL = 0;
    }

    /*
     * If necessary, concatenate the parts of the word.
     */

    while (numObjsToConcat > 255) {
	OP1(			STR_CONCAT1, 255);
	numObjsToConcat -= 254;	/* concat pushes 1 obj, the result */
    }
    if (numObjsToConcat > 1) {
	OP1(			STR_CONCAT1, numObjsToConcat);
    }

    /*
     * If the tokens yielded no instructions, push an empty string.
     */

    if (envPtr->codeNext == entryCodeNext) {
	PUSH(			"");
    }
    Tcl_DStringFree(&textBuffer);

    /*
     * Release the temp table we used to collect the locations of continuation
     * lines, if any.
     */

    if (maxNumCL) {
	Tcl_Free(clPosition);
    }
    TclCheckStackDepth(depth+1, envPtr);
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `JimExpandDictSugar`, lines 5231–5254. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `66140f88847571c4275cc1b6114b5a40e1244f3eca9d6dc8dd59f0cbf2196278`; retained evidence `source-jim`.

```text
static Jim_Obj *JimExpandDictSugar(Jim_Interp *interp, Jim_Obj *objPtr)
{
    Jim_Obj *resObjPtr = NULL;
    Jim_Obj *substKeyObjPtr = NULL;

    if (interp->safeexpr) {
        return objPtr;
    }

    SetDictSubstFromAny(interp, objPtr);

    if (Jim_SubstObj(interp, objPtr->internalRep.dictSubstValue.indexObjPtr,
            &substKeyObjPtr, JIM_NONE)
        != JIM_OK) {
        return NULL;
    }
    Jim_IncrRefCount(substKeyObjPtr);
    resObjPtr =
        JimDictExpandArrayVariable(interp, objPtr->internalRep.dictSubstValue.varNameObjPtr,
        substKeyObjPtr, 0);
    Jim_DecrRefCount(interp, substKeyObjPtr);

    return resObjPtr;
}

```


## Consumer bindings

- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `scan_parts`: Shared selected template and recursive index component scanner.
- [rust/tcl-lexer/src/executable_parts.rs](../../../../rust/tcl-lexer/src/executable_parts.rs), `ExecutablePartArena::from_inputs_with_variables`: Flat index child scheduling retains independent ALL mask and ordinary C word token compiler purpose.
- [rust/tcl-registry/src/substitution.rs](../../../../rust/tcl-registry/src/substitution.rs), `TemplateParseErrors::variable_syntax`: Select the actual C template grammar independently of written source words.
- [rust/tcl-lexer/src/executable_parts.rs](../../../../rust/tcl-lexer/src/executable_parts.rs), `executable_parts::tests::template_masks_leave_array_index_substitutions_enabled` (linked): Independent ALL array-index children retain variable, decoded escape and command under disabled outer command/backslash flags.
- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `word_parts::template_policy_tests::c_template_retains_disabled_sigil_and_escape_token_boundaries` (linked): Selected CTcl root tokens preserve disabled-sigil/raw-zero/escape splitting; ordinary WrittenWord retains its merged text purpose.
- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `word_parts::tests::outer_substitution_mask_does_not_disable_array_index_commands` (linked): Recursive scanner index children use independently enabled substitutions, while disabled outer variable substitution keeps the entire reference literal.

A named test is a coverage binding, not a claim that it executed.

## Replay

Source inspection only. Reconfirm each exact retained function window against the pinned full source file SHA and one-based lines; no compiler or native executable is launched. Empirical matrix comparisons remain in naming.substitution.counted-template-completions.
