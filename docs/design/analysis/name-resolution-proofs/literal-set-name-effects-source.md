# naming.compiler.literal-set-name-effects-source

Kind: `source-anchor`

## Problem statement

A genuine stock set handler may be selected while its caller compilation mode is unknown. Treating uncertain opcode admission as an arbitrary compiler mutation erases previously installed commands before this invocation evaluates any arguments. The opposing risk is assuming compiler purity from the Set handler footprint: source substitutions can cause nested compilation, and an unrelated or replaced compiler has its own effects. The question is limited to the selected stock Set compiler and a complete substitution-free original vector.

## Question

Do the pinned C8.4 through C9.1 Set compiler paths for substitution-free name and value words perform command or namespace binding operations before runtime argv evaluation?

## Conclusion

The inspected stock Set compiler pushes or registers literal source components, allocates only compiled local-variable metadata when eligible, and emits load/store instructions. Its static variable-name and Text/backslash token paths do not invoke command scripts or mutate command or namespace bindings. Nested command token compilation is a distinct branch and is excluded. This supports a separate tracked-name effect closure for genuine literal Set vectors while opcode admission, variable resolution, literal-object identity and handler completion remain independent.

## Scope

Source inspection of pinned Tcl8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. Complete Set, variable-word, token compiler and local-allocation excerpts retain original full-file hashes and exact line ranges. No new native execution is claimed. Replacement compilers, nested substitutions, expansions, implementation-path compiler delegation, Jim and BIG-IP are excluded.

## Provider answers

### tcl8.4

Status: `inspected`. Version: 8.4.20. Build: Pinned release source inspection only; no guest build or execution.. Channel: Exact C source, not guest input.. Dialect: Tcl.

Literal Set compiles literal pushes and load/store opcodes; compiled-local allocation is metadata only. Command-token nested compilation is separately reached and excluded.

### tcl8.5

Status: `inspected`. Version: 8.5.19. Build: Pinned release source inspection only; no guest build or execution.. Channel: Exact C source, not guest input.. Dialect: Tcl.

Literal Set compiles literal pushes and load/store opcodes; compiled-local allocation is metadata only. Command-token nested compilation is separately reached and excluded.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Pinned release source inspection only; no guest build or execution.. Channel: Exact C source, not guest input.. Dialect: Tcl.

Literal Set compiles literal pushes and load/store opcodes; compiled-local allocation is metadata only. Command-token nested compilation is separately reached and excluded.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Pinned release source inspection only; no guest build or execution.. Channel: Exact C source, not guest input.. Dialect: Tcl.

Literal Set compiles literal pushes and load/store opcodes; compiled-local allocation is metadata only. Command-token nested compilation is separately reached and excluded.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Pinned release source inspection only; no guest build or execution.. Channel: Exact C source, not guest input.. Dialect: Tcl.

Literal Set compiles literal pushes and load/store opcodes; compiled-local allocation is metadata only. Command-token nested compilation is separately reached and excluded.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.4.20-TclCompileSetCmd.txt`. SHA-256 `a67197c79da42a0467050b5fdb76930a8d8b502ef2e8ca68d787b4ad45a3a76f`. Exact full TclCompileSetCmd definition; source full-file hash and line range retained independently.

- `e1`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.4.20-TclPushVarName.txt`. SHA-256 `b9fd87d654b758367c00fbd16fe8394e91c5f95dedc6b681d9ba699468b45107`. Exact full TclPushVarName definition; source full-file hash and line range retained independently.

- `e2`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.4.20-TclCompileTokens.txt`. SHA-256 `28cc01627a3f5190822bbbf9a864d45696cb8727a2cfbf3def2c7f09da492afa`. Exact full TclCompileTokens definition; source full-file hash and line range retained independently.

- `e3`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.4.20-TclFindCompiledLocal.txt`. SHA-256 `c719316f477c7007c60b313b7ca0b00b12bf22372981a3819bdbf232eb3dde79`. Exact full TclFindCompiledLocal definition; source full-file hash and line range retained independently.

- `e4`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.5.19-TclCompileSetCmd.txt`. SHA-256 `ef1e582aeeed894fe50713e0f8ce8a99a06c94f2466e1f94cd9f4e1c5d849dcf`. Exact full TclCompileSetCmd definition; source full-file hash and line range retained independently.

- `e5`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.5.19-PushVarName.txt`. SHA-256 `93765f759ed489263582d7d4f2b86f58fab7e0a7cf1232323fb9dcef835c0210`. Exact full PushVarName definition; source full-file hash and line range retained independently.

- `e6`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.5.19-TclCompileTokens.txt`. SHA-256 `f6f5b99e924c9008a95360d1e7a247721f85fbd997611560b5fe350837a4a3a5`. Exact full TclCompileTokens definition; source full-file hash and line range retained independently.

- `e7`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.5.19-TclFindCompiledLocal.txt`. SHA-256 `917731c247b97d3ce1ec4f6331879334c90f0eb0d1cc37c9ec2194b5954c00ff`. Exact full TclFindCompiledLocal definition; source full-file hash and line range retained independently.

- `e8`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.6.18-TclCompileSetCmd.txt`. SHA-256 `ec6366d9632f55a840c255b1cb09270653daeba0155e462257ef175f3b0e84e9`. Exact full TclCompileSetCmd definition; source full-file hash and line range retained independently.

- `e9`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.6.18-TclPushVarName.txt`. SHA-256 `414e0c602ecd47ea0909bc2af3016540dd61de38a199e3512377e4b06a328fba`. Exact full TclPushVarName definition; source full-file hash and line range retained independently.

- `e10`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.6.18-TclCompileTokens.txt`. SHA-256 `0ac0749c57f793286af6437c49c109c4146287f3bb34658926e5802980819dd7`. Exact full TclCompileTokens definition; source full-file hash and line range retained independently.

- `e11`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/8.6.18-TclFindCompiledLocal.txt`. SHA-256 `b4c836c0f9b9e6e7cf8515b2d2b32b4ab04e6442a41ef69a02ef5ded83dc134f`. Exact full TclFindCompiledLocal definition; source full-file hash and line range retained independently.

- `e12`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/9.0.4-TclCompileSetCmd.txt`. SHA-256 `34baed7dde71e469973938c03e11e8b92217cd3cf56e7430109715edb6e61098`. Exact full TclCompileSetCmd definition; source full-file hash and line range retained independently.

- `e13`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/9.0.4-TclPushVarName.txt`. SHA-256 `26b20fe759673164bffb8c6201656258f3eba27f3200c3d7fadf21af5e6b5d30`. Exact full TclPushVarName definition; source full-file hash and line range retained independently.

- `e14`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/9.0.4-TclCompileTokens.txt`. SHA-256 `2f8084251e4df3b7d823a494d9d8b1e7b38ec050a58ca905efab16dd0b52ed33`. Exact full TclCompileTokens definition; source full-file hash and line range retained independently.

- `e15`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/9.0.4-TclFindCompiledLocal.txt`. SHA-256 `a87ffe2b6f62282e8c4c51da0e7e6b045d3470c4c40c9e44ceef8a3519f4f347`. Exact full TclFindCompiledLocal definition; source full-file hash and line range retained independently.

- `e16`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/9.1.0-TclCompileSetCmd.txt`. SHA-256 `9ab577969f25ab94cd5720caff9bc0e85081472c78b0ac071eb24d2475cf41bf`. Exact full TclCompileSetCmd definition; source full-file hash and line range retained independently.

- `e17`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/9.1.0-TclPushVarName.txt`. SHA-256 `8c671c78715232e3ba32aa25ded0482b4c4b7644f96bb23d5eceab79c589ea45`. Exact full TclPushVarName definition; source full-file hash and line range retained independently.

- `e18`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/9.1.0-TclCompileTokens.txt`. SHA-256 `0192854ccad7c779a5db78f6ab91a9c47d6edcf50c2ffd889e0384ffab8ee5ba`. Exact full TclCompileTokens definition; source full-file hash and line range retained independently.

- `e19`: `rust/tcl-registry/tests/data/native_literal_set_compiler_source/9.1.0-TclFindCompiledLocal.txt`. SHA-256 `49bb8900d55b9cd0c4bec628d60e387147c71948cac15462778a850635653ab2`. Exact full TclFindCompiledLocal definition; source full-file hash and line range retained independently.

## Source inspection

tcl8.4 8.4.20: `TclCompileSetCmd` in `generic/tclCompCmds.c`, lines 2585–2684; full-file SHA-256 `38626005d02a7445a951f22b55dac182a3ccba76c01834f210fbca94c2dc6214`; excerpt SHA-256 `a67197c79da42a0467050b5fdb76930a8d8b502ef2e8ca68d787b4ad45a3a76f`; evidence `e0`.

```c
TclCompileSetCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int isAssignment, isScalar, simpleVarName, localIndex, numWords;
    int code = TCL_OK;

    DefineLineInformation;

    numWords = parsePtr->numWords;
    if ((numWords != 2) && (numWords != 3)) {
	Tcl_ResetResult(interp);
	Tcl_AppendToObj(Tcl_GetObjResult(interp),
	        "wrong # args: should be \"set varName ?newValue?\"", -1);
        return TCL_ERROR;
    }
    isAssignment = (numWords == 3);

    /*
     * Decide if we can use a frame slot for the var/array name or if we
     * need to emit code to compute and push the name at runtime. We use a
     * frame slot (entry in the array of local vars) if we are compiling a
     * procedure body and if the name is simple text that does not include
     * namespace qualifiers. 
     */

    varTokenPtr = parsePtr->tokenPtr
	    + (parsePtr->tokenPtr->numComponents + 1);

    code = TclPushVarNameWord(interp, varTokenPtr, envPtr, TCL_CREATE_VAR,
	    &localIndex, &simpleVarName, &isScalar, 1);
    if (code != TCL_OK) {
	goto done;
    }

    /*
     * If we are doing an assignment, push the new value.
     */

    if (isAssignment) {
	valueTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
	if (valueTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    TclEmitPush(TclRegisterNewLiteral(envPtr, valueTokenPtr[1].start,
		    valueTokenPtr[1].size), envPtr);
	} else {
	    SetLineInformation (2);
	    code = TclCompileTokens(interp, valueTokenPtr+1,
	            valueTokenPtr->numComponents, envPtr);
	    if (code != TCL_OK) {
		goto done;
	    }
	}
    }

    /*
     * Emit instructions to set/get the variable.
     */

    if (simpleVarName) {
	if (isScalar) {
	    if (localIndex >= 0) {
		if (localIndex <= 255) {
		    TclEmitInstInt1((isAssignment?
		            INST_STORE_SCALAR1 : INST_LOAD_SCALAR1),
			    localIndex, envPtr);
		} else {
		    TclEmitInstInt4((isAssignment?
			    INST_STORE_SCALAR4 : INST_LOAD_SCALAR4),
			    localIndex, envPtr);
		}
	    } else {
		TclEmitOpcode((isAssignment?
		        INST_STORE_SCALAR_STK : INST_LOAD_SCALAR_STK), envPtr);
	    }
	} else {
	    if (localIndex >= 0) {
		if (localIndex <= 255) {
		    TclEmitInstInt1((isAssignment?
		            INST_STORE_ARRAY1 : INST_LOAD_ARRAY1),
			    localIndex, envPtr);
		} else {
		    TclEmitInstInt4((isAssignment?
			    INST_STORE_ARRAY4 : INST_LOAD_ARRAY4),
			    localIndex, envPtr);
		}
	    } else {
		TclEmitOpcode((isAssignment?
		        INST_STORE_ARRAY_STK : INST_LOAD_ARRAY_STK), envPtr);
	    }
	}
    } else {
	TclEmitOpcode((isAssignment? INST_STORE_STK : INST_LOAD_STK), envPtr);
    }
	
    done:
    return code;
}
```

tcl8.4 8.4.20: `TclPushVarName` in `generic/tclCompCmds.c`, lines 3225–3464; full-file SHA-256 `38626005d02a7445a951f22b55dac182a3ccba76c01834f210fbca94c2dc6214`; excerpt SHA-256 `b9fd87d654b758367c00fbd16fe8394e91c5f95dedc6b681d9ba699468b45107`; evidence `e1`.

```c
TclPushVarName(interp, varTokenPtr, envPtr, flags, localIndexPtr,
#ifndef TCL_TIP280
	simpleVarNamePtr, isScalarPtr)
#else
	simpleVarNamePtr, isScalarPtr, line, clNext)
#endif
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Token *varTokenPtr;	/* Points to a variable token. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
    int flags;			/* takes TCL_CREATE_VAR or
				 * TCL_NO_LARGE_INDEX */
    int *localIndexPtr;		/* must not be NULL */
    int *simpleVarNamePtr;	/* must not be NULL */
    int *isScalarPtr;		/* must not be NULL */
#ifdef TCL_TIP280
    int line;                   /* line the token starts on */
    int* clNext;
#endif
{
    register CONST char *p;
    CONST char *name, *elName;
    register int i, n;
    int nameChars, elNameChars, simpleVarName, localIndex;
    int code = TCL_OK;

    Tcl_Token *elemTokenPtr = NULL;
    int elemTokenCount = 0;
    int allocedTokens = 0;
    int removedParen = 0;

    /*
     * Decide if we can use a frame slot for the var/array name or if we
     * need to emit code to compute and push the name at runtime. We use a
     * frame slot (entry in the array of local vars) if we are compiling a
     * procedure body and if the name is simple text that does not include
     * namespace qualifiers. 
     */

    simpleVarName = 0;
    name = elName = NULL;
    nameChars = elNameChars = 0;
    localIndex = -1;

    /*
     * Check not only that the type is TCL_TOKEN_SIMPLE_WORD, but whether
     * curly braces surround the variable name.
     * This really matters for array elements to handle things like
     *    set {x($foo)} 5
     * which raises an undefined var error if we are not careful here.
     */

    if ((varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) &&
	    (varTokenPtr->start[0] != '{')) {
	/*
	 * A simple variable name. Divide it up into "name" and "elName"
	 * strings. If it is not a local variable, look it up at runtime.
	 */
	simpleVarName = 1;

	name = varTokenPtr[1].start;
	nameChars = varTokenPtr[1].size;
	if ( *(name + nameChars - 1) == ')') {
	    /* 
	     * last char is ')' => potential array reference.
	     */

	    for (i = 0, p = name;  i < nameChars;  i++, p++) {
		if (*p == '(') {
		    elName = p + 1;
		    elNameChars = nameChars - i - 2;
		    nameChars = i ;
		    break;
		}
	    }

	    if ((elName != NULL) && elNameChars) {
		/*
		 * An array element, the element name is a simple
		 * string: assemble the corresponding token.
		 */

		elemTokenPtr = (Tcl_Token *) ckalloc(sizeof(Tcl_Token));
		allocedTokens = 1;
		elemTokenPtr->type = TCL_TOKEN_TEXT;
		elemTokenPtr->start = elName;
		elemTokenPtr->size = elNameChars;
		elemTokenPtr->numComponents = 0;
		elemTokenCount = 1;
	    }
	}
    } else if (((n = varTokenPtr->numComponents) > 1)
	    && (varTokenPtr[1].type == TCL_TOKEN_TEXT)
            && (varTokenPtr[n].type == TCL_TOKEN_TEXT)
            && (varTokenPtr[n].start[varTokenPtr[n].size - 1] == ')')) {

        /*
	 * Check for parentheses inside first token
	 */

        simpleVarName = 0;
        for (i = 0, p = varTokenPtr[1].start; 
	     i < varTokenPtr[1].size; i++, p++) {
            if (*p == '(') {
                simpleVarName = 1;
                break;
            }
        }
        if (simpleVarName) {
	    int remainingChars;

	    /*
	     * Check the last token: if it is just ')', do not count
	     * it. Otherwise, remove the ')' and flag so that it is
	     * restored at the end.
	     */

	    if (varTokenPtr[n].size == 1) {
		--n;
	    } else {
		--varTokenPtr[n].size;
		removedParen = n;
	    }

            name = varTokenPtr[1].start;
            nameChars = p - varTokenPtr[1].start;
            elName = p + 1;
            remainingChars = (varTokenPtr[2].start - p) - 1;
            elNameChars = (varTokenPtr[n].start - p) + varTokenPtr[n].size - 2;

	    if (remainingChars) {
		/*
		 * Make a first token with the extra characters in the first 
		 * token.
		 */

		elemTokenPtr = (Tcl_Token *) ckalloc(n * sizeof(Tcl_Token));
		allocedTokens = 1;
		elemTokenPtr->type = TCL_TOKEN_TEXT;
		elemTokenPtr->start = elName;
		elemTokenPtr->size = remainingChars;
		elemTokenPtr->numComponents = 0;
		elemTokenCount = n;
		
		/*
		 * Copy the remaining tokens.
		 */
		
		memcpy((void *) (elemTokenPtr+1), (void *) (&varTokenPtr[2]),
		       ((n-1) * sizeof(Tcl_Token)));
	    } else {
		/*
		 * Use the already available tokens.
		 */
		
		elemTokenPtr = &varTokenPtr[2];
		elemTokenCount = n - 1;	    
	    }
	}
    }

    if (simpleVarName) {
	/*
	 * See whether name has any namespace separators (::'s).
	 */

	int hasNsQualifiers = 0;
	for (i = 0, p = name;  i < nameChars;  i++, p++) {
	    if ((*p == ':') && ((i+1) < nameChars) && (*(p+1) == ':')) {
		hasNsQualifiers = 1;
		break;
	    }
	}

	/*
	 * Look up the var name's index in the array of local vars in the
	 * proc frame. If retrieving the var's value and it doesn't already
	 * exist, push its name and look it up at runtime.
	 */

	if ((envPtr->procPtr != NULL) && !hasNsQualifiers) {
	    localIndex = TclFindCompiledLocal(name, nameChars,
		    /*create*/ (flags & TCL_CREATE_VAR),
                    /*flags*/ ((elName==NULL)? VAR_SCALAR : VAR_ARRAY),
		    envPtr->procPtr);
	    if ((flags & TCL_NO_LARGE_INDEX) && (localIndex > 255)) {
		/* we'll push the name */
		localIndex = -1;
	    }
	}
	if (localIndex < 0) {
	    TclEmitPush(TclRegisterNewLiteral(envPtr, name, nameChars), envPtr);
	}

	/*
	 * Compile the element script, if any.
	 */

	if (elName != NULL) {
	    if (elNameChars) {
#ifdef TCL_TIP280
	        envPtr->line   = line;
	        envPtr->clNext = clNext;
#endif
		code = TclCompileTokens(interp, elemTokenPtr,
                        elemTokenCount, envPtr);
		if (code != TCL_OK) {
		    goto done;
		}
	    } else {
		TclEmitPush(TclRegisterNewLiteral(envPtr, "", 0), envPtr);
	    }
	}
    } else {
	/*
	 * The var name isn't simple: compile and push it.
	 */

#ifdef TCL_TIP280
        envPtr->line   = line;
        envPtr->clNext = clNext;
#endif
	code = TclCompileTokens(interp, varTokenPtr+1,
		varTokenPtr->numComponents, envPtr);
	if (code != TCL_OK) {
	    goto done;
	}
    }

    done:
    if (removedParen) {
	++varTokenPtr[removedParen].size;
    }
    if (allocedTokens) {
        ckfree((char *) elemTokenPtr);
    }
    *localIndexPtr	= localIndex;
    *simpleVarNamePtr	= simpleVarName;
    *isScalarPtr	= (elName == NULL);
    return code;
}
```

tcl8.4 8.4.20: `TclCompileTokens` in `generic/tclCompile.c`, lines 1519–1809; full-file SHA-256 `0bc55b283d6cb62a4298d4dc30e050b587d5d1b4e7f145e4f644e237dd7639f8`; excerpt SHA-256 `28cc01627a3f5190822bbbf9a864d45696cb8727a2cfbf3def2c7f09da492afa`; evidence `e2`.

```c
TclCompileTokens(interp, tokenPtr, count, envPtr)
    Tcl_Interp *interp;		/* Used for error and status reporting. */
    Tcl_Token *tokenPtr;	/* Pointer to first in an array of tokens
				 * to compile. */
    int count;			/* Number of tokens to consider at tokenPtr.
				 * Must be at least 1. */
    CompileEnv *envPtr;		/* Holds the resulting instructions. */
{
    Tcl_DString textBuffer;	/* Holds concatenated chars from adjacent
				 * TCL_TOKEN_TEXT, TCL_TOKEN_BS tokens. */
    char buffer[TCL_UTF_MAX];
    CONST char *name, *p;
    int numObjsToConcat, nameBytes, localVarName, localVar;
    int length, i, code;
    unsigned char *entryCodeNext = envPtr->codeNext;
#ifdef TCL_TIP280
#define NUM_STATIC_POS 20
    int isLiteral, maxNumCL, numCL;
    int* clPosition = NULL;

    /*
     * For the handling of continuation lines in literals we first check if
     * this is actually a literal. For if not we can forego the additional
     * processing. Otherwise we pre-allocate a small table to store the
     * locations of all continuation lines we find in this literal, if
     * any. The table is extended if needed.
     *
     * Note: Different to the equivalent code in function
     * 'EvalTokensStandard()' (see file "tclBasic.c") we do not seem to need
     * the 'adjust' variable. We also do not seem to need code which merges
     * continuation line information of multiple words which concat'd at
     * runtime. Either that or I have not managed to find a test case for
     * these two possibilities yet. It might be a difference between compile-
     * versus runtime processing.
     */

    numCL     = 0;
    maxNumCL  = 0;
    isLiteral = 1;
    for (i=0 ; i < count; i++) {
	if ((tokenPtr[i].type != TCL_TOKEN_TEXT) &&
	    (tokenPtr[i].type != TCL_TOKEN_BS)) {
	    isLiteral = 0;
	    break;
	}
    }

    if (isLiteral) {
	maxNumCL   = NUM_STATIC_POS;
	clPosition = (int*) ckalloc (maxNumCL*sizeof(int));
    }
#endif

    Tcl_DStringInit(&textBuffer);
    numObjsToConcat = 0;
    for ( ;  count > 0;  count--, tokenPtr++) {
	switch (tokenPtr->type) {
	    case TCL_TOKEN_TEXT:
		Tcl_DStringAppend(&textBuffer, tokenPtr->start,
			tokenPtr->size);
		break;

	    case TCL_TOKEN_BS:
		length = TclParseBackslash(tokenPtr->start, tokenPtr->size,
			(int *) NULL, buffer);
		Tcl_DStringAppend(&textBuffer, buffer, length);

#ifdef TCL_TIP280
		/*
		 * If the backslash sequence we found is in a literal, and
		 * represented a continuation line, we compute and store its
		 * location (as char offset to the beginning of the _result_
		 * script). We may have to extend the table of locations.
		 *
		 * Note that the continuation line information is relevant
		 * even if the word we are processing is not a literal, as it
		 * can affect nested commands. See the branch for
		 * TCL_TOKEN_COMMAND below, where the adjustment we are
		 * tracking here is taken into account. The good thing is that
		 * we do not need a table of everything, just the number of
		 * lines we have to add as correction.
		 */

		if ((length == 1) && (buffer[0] == ' ') &&
		    (tokenPtr->start[1] == '\n')) {
		    if (isLiteral) {
			int clPos = Tcl_DStringLength (&textBuffer);

			if (numCL >= maxNumCL) {
			    maxNumCL *= 2;
			    clPosition = (int*) ckrealloc ((char*)clPosition,
							   maxNumCL*sizeof(int));
			}
			clPosition[numCL] = clPos;
			numCL ++;
		    }
		}
#endif
		break;

	    case TCL_TOKEN_COMMAND:
		/*
		 * Push any accumulated chars appearing before the command.
		 */
		
		if (Tcl_DStringLength(&textBuffer) > 0) {
		    int literal;
		    
		    literal = TclRegisterLiteral(envPtr,
			    Tcl_DStringValue(&textBuffer),
			    Tcl_DStringLength(&textBuffer), /*onHeap*/ 0);
		    TclEmitPush(literal, envPtr);
		    numObjsToConcat++;
		    Tcl_DStringFree(&textBuffer);
#ifdef TCL_TIP280
		    if (numCL) {
			TclContinuationsEnter(envPtr->literalArrayPtr[literal].objPtr,
					      numCL, clPosition);
		    }
		    numCL = 0;
#endif
		}
		
		code = TclCompileScript(interp, tokenPtr->start+1,
			tokenPtr->size-2, /*nested*/ 0,	envPtr);
		if (code != TCL_OK) {
		    goto error;
		}
		numObjsToConcat++;
		break;

	    case TCL_TOKEN_VARIABLE:
		/*
		 * Push any accumulated chars appearing before the $<var>.
		 */
		
		if (Tcl_DStringLength(&textBuffer) > 0) {
		    int literal;
		    
		    literal = TclRegisterLiteral(envPtr,
			    Tcl_DStringValue(&textBuffer),
			    Tcl_DStringLength(&textBuffer), /*onHeap*/ 0);
		    TclEmitPush(literal, envPtr);
		    numObjsToConcat++;
		    Tcl_DStringFree(&textBuffer);
		}
		
		/*
		 * Determine how the variable name should be handled: if it contains 
		 * any namespace qualifiers it is not a local variable (localVarName=-1);
		 * if it looks like an array element and the token has a single component, 
		 * it should not be created here [Bug 569438] (localVarName=0); otherwise, 
		 * the local variable can safely be created (localVarName=1).
		 */
		
		name = tokenPtr[1].start;
		nameBytes = tokenPtr[1].size;
		localVarName = -1;
		if (envPtr->procPtr != NULL) {
		    localVarName = 1;
		    for (i = 0, p = name;  i < nameBytes;  i++, p++) {
			if ((*p == ':') && (i < (nameBytes-1))
			        && (*(p+1) == ':')) {
			    localVarName = -1;
			    break;
			} else if ((*p == '(')
			        && (tokenPtr->numComponents == 1) 
				&& (*(name + nameBytes - 1) == ')')) {
			    localVarName = 0;
			    break;
			}
		    }
		}

		/*
		 * Either push the variable's name, or find its index in
		 * the array of local variables in a procedure frame. 
		 */

		localVar = -1;
		if (localVarName != -1) {
		    localVar = TclFindCompiledLocal(name, nameBytes, 
			        localVarName, /*flags*/ 0, envPtr->procPtr);
		}
		if (localVar < 0) {
		    TclEmitPush(TclRegisterNewLiteral(envPtr, name, nameBytes),
			    envPtr); 
		}

		/*
		 * Emit instructions to load the variable.
		 */
		
		if (tokenPtr->numComponents == 1) {
		    if (localVar < 0) {
			TclEmitOpcode(INST_LOAD_SCALAR_STK, envPtr);
		    } else if (localVar <= 255) {
			TclEmitInstInt1(INST_LOAD_SCALAR1, localVar,
			        envPtr);
		    } else {
			TclEmitInstInt4(INST_LOAD_SCALAR4, localVar,
				envPtr);
		    }
		} else {
		    code = TclCompileTokens(interp, tokenPtr+2,
			    tokenPtr->numComponents-1, envPtr);
		    if (code != TCL_OK) {
			char errorBuffer[150];
			sprintf(errorBuffer,
			        "\n    (parsing index for array \"%.*s\")",
				((nameBytes > 100)? 100 : nameBytes), name);
			Tcl_AddObjErrorInfo(interp, errorBuffer, -1);
			goto error;
		    }
		    if (localVar < 0) {
			TclEmitOpcode(INST_LOAD_ARRAY_STK, envPtr);
		    } else if (localVar <= 255) {
			TclEmitInstInt1(INST_LOAD_ARRAY1, localVar,
			        envPtr);
		    } else {
			TclEmitInstInt4(INST_LOAD_ARRAY4, localVar,
			        envPtr);
		    }
		}
		numObjsToConcat++;
		count -= tokenPtr->numComponents;
		tokenPtr += tokenPtr->numComponents;
		break;

	    default:
		panic("Unexpected token type in TclCompileTokens");
	}
    }

    /*
     * Push any accumulated characters appearing at the end.
     */

    if (Tcl_DStringLength(&textBuffer) > 0) {
	int literal;

	literal = TclRegisterLiteral(envPtr, Tcl_DStringValue(&textBuffer),
	        Tcl_DStringLength(&textBuffer), /*onHeap*/ 0);
	TclEmitPush(literal, envPtr);
	numObjsToConcat++;

#ifdef TCL_TIP280
	if (numCL) {
	    TclContinuationsEnter(envPtr->literalArrayPtr[literal].objPtr,
				  numCL, clPosition);
	}
	numCL = 0;
#endif
    }

    /*
     * If necessary, concatenate the parts of the word.
     */

    while (numObjsToConcat > 255) {
	TclEmitInstInt1(INST_CONCAT1, 255, envPtr);
	numObjsToConcat -= 254;	/* concat pushes 1 obj, the result */
    }
    if (numObjsToConcat > 1) {
	TclEmitInstInt1(INST_CONCAT1, numObjsToConcat, envPtr);
    }

    /*
     * If the tokens yielded no instructions, push an empty string.
     */
    
    if (envPtr->codeNext == entryCodeNext) {
	TclEmitPush(TclRegisterLiteral(envPtr, "", 0, /*onHeap*/ 0),
	        envPtr);
    }
    code = TCL_OK;

    error:
    Tcl_DStringFree(&textBuffer);
#ifdef TCL_TIP280
    /*
     * Release the temp table we used to collect the locations of
     * continuation lines, if any.
     */

    if (maxNumCL) {
	ckfree ((char*) clPosition);
    }
#endif
    return code;
}
```

tcl8.4 8.4.20: `TclFindCompiledLocal` in `generic/tclCompile.c`, lines 2237–2307; full-file SHA-256 `0bc55b283d6cb62a4298d4dc30e050b587d5d1b4e7f145e4f644e237dd7639f8`; excerpt SHA-256 `c719316f477c7007c60b313b7ca0b00b12bf22372981a3819bdbf232eb3dde79`; evidence `e3`.

```c
TclFindCompiledLocal(name, nameBytes, create, flags, procPtr)
    register CONST char *name;	/* Points to first character of the name of
				 * a scalar or array variable. If NULL, a
				 * temporary var should be created. */
    int nameBytes;		/* Number of bytes in the name. */
    int create;			/* If 1, allocate a local frame entry for
				 * the variable if it is new. */
    int flags;			/* Flag bits for the compiled local if
				 * created. Only VAR_SCALAR, VAR_ARRAY, and
				 * VAR_LINK make sense. */
    register Proc *procPtr;	/* Points to structure describing procedure
				 * containing the variable reference. */
{
    register CompiledLocal *localPtr;
    int localVar = -1;
    register int i;

    /*
     * If not creating a temporary, does a local variable of the specified
     * name already exist?
     */

    if (name != NULL) {	
	int localCt = procPtr->numCompiledLocals;
	localPtr = procPtr->firstLocalPtr;
	for (i = 0;  i < localCt;  i++) {
	    if (!TclIsVarTemporary(localPtr)) {
		char *localName = localPtr->name;
		if ((nameBytes == localPtr->nameLength)
	                && (strncmp(name, localName, (unsigned) nameBytes) == 0)) {
		    return i;
		}
	    }
	    localPtr = localPtr->nextPtr;
	}
    }

    /*
     * Create a new variable if appropriate.
     */
    
    if (create || (name == NULL)) {
	localVar = procPtr->numCompiledLocals;
	localPtr = (CompiledLocal *) ckalloc((unsigned) 
	        (sizeof(CompiledLocal) - sizeof(localPtr->name)
		+ nameBytes+1));
	if (procPtr->firstLocalPtr == NULL) {
	    procPtr->firstLocalPtr = procPtr->lastLocalPtr = localPtr;
	} else {
	    procPtr->lastLocalPtr->nextPtr = localPtr;
	    procPtr->lastLocalPtr = localPtr;
	}
	localPtr->nextPtr = NULL;
	localPtr->nameLength = nameBytes;
	localPtr->frameIndex = localVar;
	localPtr->flags = flags | VAR_UNDEFINED;
	if (name == NULL) {
	    localPtr->flags |= VAR_TEMPORARY;
	}
	localPtr->defValuePtr = NULL;
	localPtr->resolveInfo = NULL;

	if (name != NULL) {
	    memcpy((VOID *) localPtr->name, (VOID *) name,
	            (size_t) nameBytes);
	}
	localPtr->name[nameBytes] = '\0';
	procPtr->numCompiledLocals++;
    }
    return localVar;
}
```

tcl8.5 8.5.19: `TclCompileSetCmd` in `generic/tclCompCmds.c`, lines 3328–3404; full-file SHA-256 `117274255882a35828318ccbfab476c180622310187c5e3a4796867b48ba7414`; excerpt SHA-256 `ef1e582aeeed894fe50713e0f8ce8a99a06c94f2466e1f94cd9f4e1c5d849dcf`; evidence `e4`.

```c
TclCompileSetCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int isAssignment, isScalar, simpleVarName, localIndex, numWords;
    DefineLineInformation;	/* TIP #280 */

    numWords = parsePtr->numWords;
    if ((numWords != 2) && (numWords != 3)) {
	return TCL_ERROR;
    }
    isAssignment = (numWords == 3);

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    PushVarNameWord(interp, varTokenPtr, envPtr, TCL_CREATE_VAR,
		    &localIndex, &simpleVarName, &isScalar, 1);

    /*
     * If we are doing an assignment, push the new value.
     */

    if (isAssignment) {
	valueTokenPtr = TokenAfter(varTokenPtr);
	CompileWord(envPtr, valueTokenPtr, interp, 2);
    }

    /*
     * Emit instructions to set/get the variable.
     */

    if (simpleVarName) {
	if (isScalar) {
	    if (localIndex < 0) {
		TclEmitOpcode((isAssignment?
			INST_STORE_SCALAR_STK : INST_LOAD_SCALAR_STK), envPtr);
	    } else if (localIndex <= 255) {
		TclEmitInstInt1((isAssignment?
			INST_STORE_SCALAR1 : INST_LOAD_SCALAR1),
			localIndex, envPtr);
	    } else {
		TclEmitInstInt4((isAssignment?
			INST_STORE_SCALAR4 : INST_LOAD_SCALAR4),
			localIndex, envPtr);
	    }
	} else {
	    if (localIndex < 0) {
		TclEmitOpcode((isAssignment?
			INST_STORE_ARRAY_STK : INST_LOAD_ARRAY_STK), envPtr);
	    } else if (localIndex <= 255) {
		TclEmitInstInt1((isAssignment?
			INST_STORE_ARRAY1 : INST_LOAD_ARRAY1),
			localIndex, envPtr);
	    } else {
		TclEmitInstInt4((isAssignment?
			INST_STORE_ARRAY4 : INST_LOAD_ARRAY4),
			localIndex, envPtr);
	    }
	}
    } else {
	TclEmitOpcode((isAssignment? INST_STORE_STK : INST_LOAD_STK), envPtr);
    }

    return TCL_OK;
}
```

tcl8.5 8.5.19: `PushVarName` in `generic/tclCompCmds.c`, lines 4774–4984; full-file SHA-256 `117274255882a35828318ccbfab476c180622310187c5e3a4796867b48ba7414`; excerpt SHA-256 `93765f759ed489263582d7d4f2b86f58fab7e0a7cf1232323fb9dcef835c0210`; evidence `e5`.

```c
PushVarName(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Token *varTokenPtr,	/* Points to a variable token. */
    CompileEnv *envPtr,		/* Holds resulting instructions. */
    int flags,			/* TCL_CREATE_VAR or TCL_NO_LARGE_INDEX. */
    int *localIndexPtr,		/* Must not be NULL. */
    int *simpleVarNamePtr,	/* Must not be NULL. */
    int *isScalarPtr,		/* Must not be NULL. */
    int line,                   /* Line the token starts on. */
    int* clNext)                /* Reference to offset of next hidden cont. line */
{
    register const char *p;
    const char *name, *elName;
    register int i, n;
    Tcl_Token *elemTokenPtr = NULL;
    int nameChars, elNameChars, simpleVarName, localIndex;
    int elemTokenCount = 0, allocedTokens = 0, removedParen = 0;

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    simpleVarName = 0;
    name = elName = NULL;
    nameChars = elNameChars = 0;
    localIndex = -1;

    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	/*
	 * A simple variable name. Divide it up into "name" and "elName"
	 * strings. If it is not a local variable, look it up at runtime.
	 */

	simpleVarName = 1;

	name = varTokenPtr[1].start;
	nameChars = varTokenPtr[1].size;
	if (name[nameChars-1] == ')') {
	    /*
	     * last char is ')' => potential array reference.
	     */

	    for (i=0,p=name ; i<nameChars ; i++,p++) {
		if (*p == '(') {
		    elName = p + 1;
		    elNameChars = nameChars - i - 2;
		    nameChars = i;
		    break;
		}
	    }

	    if (interp && (elName != NULL) && elNameChars) {
		/*
		 * An array element, the element name is a simple string:
		 * assemble the corresponding token.
		 */

		elemTokenPtr = (Tcl_Token *) TclStackAlloc(interp,
			sizeof(Tcl_Token));
		allocedTokens = 1;
		elemTokenPtr->type = TCL_TOKEN_TEXT;
		elemTokenPtr->start = elName;
		elemTokenPtr->size = elNameChars;
		elemTokenPtr->numComponents = 0;
		elemTokenCount = 1;
	    }
	}
    } else if (interp && ((n = varTokenPtr->numComponents) > 1)
	    && (varTokenPtr[1].type == TCL_TOKEN_TEXT)
	    && (varTokenPtr[n].type == TCL_TOKEN_TEXT)
	    && (varTokenPtr[n].start[varTokenPtr[n].size - 1] == ')')) {

	/*
	 * Check for parentheses inside first token.
	 */

	simpleVarName = 0;
	for (i = 0, p = varTokenPtr[1].start;
		i < varTokenPtr[1].size; i++, p++) {
	    if (*p == '(') {
		simpleVarName = 1;
		break;
	    }
	}
	if (simpleVarName) {
	    int remainingChars;

	    /*
	     * Check the last token: if it is just ')', do not count it.
	     * Otherwise, remove the ')' and flag so that it is restored at
	     * the end.
	     */

	    if (varTokenPtr[n].size == 1) {
		--n;
	    } else {
		--varTokenPtr[n].size;
		removedParen = n;
	    }

	    name = varTokenPtr[1].start;
	    nameChars = p - varTokenPtr[1].start;
	    elName = p + 1;
	    remainingChars = (varTokenPtr[2].start - p) - 1;
	    elNameChars = (varTokenPtr[n].start-p) + varTokenPtr[n].size - 1;

	    if (remainingChars) {
		/*
		 * Make a first token with the extra characters in the first
		 * token.
		 */

		elemTokenPtr = (Tcl_Token *) TclStackAlloc(interp,
			n * sizeof(Tcl_Token));
		allocedTokens = 1;
		elemTokenPtr->type = TCL_TOKEN_TEXT;
		elemTokenPtr->start = elName;
		elemTokenPtr->size = remainingChars;
		elemTokenPtr->numComponents = 0;
		elemTokenCount = n;

		/*
		 * Copy the remaining tokens.
		 */

		memcpy(elemTokenPtr+1, varTokenPtr+2,
			(n-1) * sizeof(Tcl_Token));
	    } else {
		/*
		 * Use the already available tokens.
		 */

		elemTokenPtr = &varTokenPtr[2];
		elemTokenCount = n - 1;
	    }
	}
    }

    if (simpleVarName) {
	/*
	 * See whether name has any namespace separators (::'s).
	 */

	int hasNsQualifiers = 0;
	for (i = 0, p = name;  i < nameChars;  i++, p++) {
	    if ((*p == ':') && ((i+1) < nameChars) && (*(p+1) == ':')) {
		hasNsQualifiers = 1;
		break;
	    }
	}

	/*
	 * Look up the var name's index in the array of local vars in the proc
	 * frame. If retrieving the var's value and it doesn't already exist,
	 * push its name and look it up at runtime.
	 */

	if ((envPtr->procPtr != NULL) && !hasNsQualifiers) {
	    localIndex = TclFindCompiledLocal(name, nameChars,
		    /*create*/ flags & TCL_CREATE_VAR,
		    envPtr->procPtr);
	    if ((flags & TCL_NO_LARGE_INDEX) && (localIndex > 255)) {
		/*
		 * We'll push the name.
		 */

		localIndex = -1;
	    }
	}
	if (interp && localIndex < 0) {
	    PushLiteral(envPtr, name, nameChars);
	}

	/*
	 * Compile the element script, if any.
	 */

	if (interp && elName != NULL) {
	    if (elNameChars) {
		envPtr->line = line;
		envPtr->clNext = clNext;
		TclCompileTokens(interp, elemTokenPtr, elemTokenCount, envPtr);
	    } else {
		PushLiteral(envPtr, "", 0);
	    }
	}
    } else if (interp) {
	/*
	 * The var name isn't simple: compile and push it.
	 */

	envPtr->line = line;
	envPtr->clNext = clNext;
	CompileTokens(envPtr, varTokenPtr, interp);
    }

    if (removedParen) {
	++varTokenPtr[removedParen].size;
    }
    if (allocedTokens) {
	TclStackFree(interp, elemTokenPtr);
    }
    *localIndexPtr = localIndex;
    *simpleVarNamePtr = simpleVarName;
    *isScalarPtr = (elName == NULL);
    return TCL_OK;
}
```

tcl8.5 8.5.19: `TclCompileTokens` in `generic/tclCompile.c`, lines 1660–1921; full-file SHA-256 `8c2ec76dbbe697201bcad79f9e32c0c1db23faf7ca01eb19e08f3ea9e68dd988`; excerpt SHA-256 `f6f5b99e924c9008a95360d1e7a247721f85fbd997611560b5fe350837a4a3a5`; evidence `e6`.

```c
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
    char buffer[TCL_UTF_MAX];
    const char *name, *p;
    int numObjsToConcat, nameBytes, localVarName, localVar;
    int length, i;
    unsigned char *entryCodeNext = envPtr->codeNext;
#define NUM_STATIC_POS 20
    int isLiteral, maxNumCL, numCL;
    int* clPosition = NULL;

    /*
     * For the handling of continuation lines in literals we first check if
     * this is actually a literal. For if not we can forego the additional
     * processing. Otherwise we pre-allocate a small table to store the
     * locations of all continuation lines we find in this literal, if
     * any. The table is extended if needed.
     *
     * Note: Different to the equivalent code in function
     * 'TclSubstTokens()' (see file "tclParse.c") we do not seem to need
     * the 'adjust' variable. We also do not seem to need code which merges
     * continuation line information of multiple words which concat'd at
     * runtime. Either that or I have not managed to find a test case for
     * these two possibilities yet. It might be a difference between compile-
     * versus runtime processing.
     */

    numCL     = 0;
    maxNumCL  = 0;
    isLiteral = 1;
    for (i=0 ; i < count; i++) {
	if ((tokenPtr[i].type != TCL_TOKEN_TEXT) &&
	    (tokenPtr[i].type != TCL_TOKEN_BS)) {
	    isLiteral = 0;
	    break;
	}
    }

    if (isLiteral) {
	maxNumCL   = NUM_STATIC_POS;
	clPosition = (int*) ckalloc (maxNumCL*sizeof(int));
    }

    Tcl_DStringInit(&textBuffer);
    numObjsToConcat = 0;
    for ( ;  count > 0;  count--, tokenPtr++) {
	switch (tokenPtr->type) {
	case TCL_TOKEN_TEXT:
	    Tcl_DStringAppend(&textBuffer, tokenPtr->start, tokenPtr->size);
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
		    int clPos = Tcl_DStringLength (&textBuffer);

		    if (numCL >= maxNumCL) {
			maxNumCL *= 2;
			clPosition = (int*) ckrealloc ((char*)clPosition,
						       maxNumCL*sizeof(int));
		    }
		    clPosition[numCL] = clPos;
		    numCL ++;
		}
	    }
	    break;

	case TCL_TOKEN_COMMAND:
	    /*
	     * Push any accumulated chars appearing before the command.
	     */

	    if (Tcl_DStringLength(&textBuffer) > 0) {
		int literal = TclRegisterNewLiteral(envPtr,
			Tcl_DStringValue(&textBuffer),
			Tcl_DStringLength(&textBuffer));

		TclEmitPush(literal, envPtr);
		numObjsToConcat++;
		Tcl_DStringFree(&textBuffer);

		if (numCL) {
		    TclContinuationsEnter(envPtr->literalArrayPtr[literal].objPtr,
					  numCL, clPosition);
		}
		numCL = 0;
	    }

	    TclCompileScript(interp, tokenPtr->start+1,
		    tokenPtr->size-2, envPtr);
	    numObjsToConcat++;
	    break;

	case TCL_TOKEN_VARIABLE:
	    /*
	     * Push any accumulated chars appearing before the $<var>.
	     */

	    if (Tcl_DStringLength(&textBuffer) > 0) {
		int literal;

		literal = TclRegisterNewLiteral(envPtr,
			Tcl_DStringValue(&textBuffer),
			Tcl_DStringLength(&textBuffer));
		TclEmitPush(literal, envPtr);
		numObjsToConcat++;
		Tcl_DStringFree(&textBuffer);
	    }

	    /*
	     * Determine how the variable name should be handled: if it
	     * contains any namespace qualifiers it is not a local variable
	     * (localVarName=-1); if it looks like an array element and the
	     * token has a single component, it should not be created here
	     * [Bug 569438] (localVarName=0); otherwise, the local variable
	     * can safely be created (localVarName=1).
	     */

	    name = tokenPtr[1].start;
	    nameBytes = tokenPtr[1].size;
	    localVarName = -1;
	    if (envPtr->procPtr != NULL) {
		localVarName = 1;
		for (i = 0, p = name;  i < nameBytes;  i++, p++) {
		    if ((*p == ':') && (i < nameBytes-1) && (*(p+1) == ':')) {
			localVarName = -1;
			break;
		    } else if ((*p == '(')
			    && (tokenPtr->numComponents == 1)
			    && (*(name + nameBytes - 1) == ')')) {
			localVarName = 0;
			break;
		    }
		}
	    }

	    /*
	     * Either push the variable's name, or find its index in the array
	     * of local variables in a procedure frame.
	     */

	    localVar = -1;
	    if (localVarName != -1) {
		localVar = TclFindCompiledLocal(name, nameBytes, localVarName,
			envPtr->procPtr);
	    }
	    if (localVar < 0) {
		TclEmitPush(TclRegisterNewLiteral(envPtr, name, nameBytes),
			envPtr);
	    }

	    /*
	     * Emit instructions to load the variable.
	     */

	    if (tokenPtr->numComponents == 1) {
		if (localVar < 0) {
		    TclEmitOpcode(INST_LOAD_SCALAR_STK, envPtr);
		} else if (localVar <= 255) {
		    TclEmitInstInt1(INST_LOAD_SCALAR1, localVar, envPtr);
		} else {
		    TclEmitInstInt4(INST_LOAD_SCALAR4, localVar, envPtr);
		}
	    } else {
		TclCompileTokens(interp, tokenPtr+2,
			tokenPtr->numComponents-1, envPtr);
		if (localVar < 0) {
		    TclEmitOpcode(INST_LOAD_ARRAY_STK, envPtr);
		} else if (localVar <= 255) {
		    TclEmitInstInt1(INST_LOAD_ARRAY1, localVar, envPtr);
		} else {
		    TclEmitInstInt4(INST_LOAD_ARRAY4, localVar, envPtr);
		}
	    }
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
	int literal;

	literal = TclRegisterNewLiteral(envPtr, Tcl_DStringValue(&textBuffer),
		Tcl_DStringLength(&textBuffer));
	TclEmitPush(literal, envPtr);
	numObjsToConcat++;

	if (numCL) {
	    TclContinuationsEnter(envPtr->literalArrayPtr[literal].objPtr,
				  numCL, clPosition);
	}
	numCL = 0;
    }

    /*
     * If necessary, concatenate the parts of the word.
     */

    while (numObjsToConcat > 255) {
	TclEmitInstInt1(INST_CONCAT1, 255, envPtr);
	numObjsToConcat -= 254;	/* concat pushes 1 obj, the result */
    }
    if (numObjsToConcat > 1) {
	TclEmitInstInt1(INST_CONCAT1, numObjsToConcat, envPtr);
    }

    /*
     * If the tokens yielded no instructions, push an empty string.
     */

    if (envPtr->codeNext == entryCodeNext) {
	TclEmitPush(TclRegisterNewLiteral(envPtr, "", 0), envPtr);
    }
    Tcl_DStringFree(&textBuffer);

    /*
     * Release the temp table we used to collect the locations of
     * continuation lines, if any.
     */

    if (maxNumCL) {
	ckfree ((char*) clPosition);
    }
}
```

tcl8.5 8.5.19: `TclFindCompiledLocal` in `generic/tclCompile.c`, lines 2286–2354; full-file SHA-256 `8c2ec76dbbe697201bcad79f9e32c0c1db23faf7ca01eb19e08f3ea9e68dd988`; excerpt SHA-256 `917731c247b97d3ce1ec4f6331879334c90f0eb0d1cc37c9ec2194b5954c00ff`; evidence `e7`.

```c
TclFindCompiledLocal(
    register const char *name,	/* Points to first character of the name of a
				 * scalar or array variable. If NULL, a
				 * temporary var should be created. */
    int nameBytes,		/* Number of bytes in the name. */
    int create,			/* If non-zero, allocate a local frame entry
				 * for the variable if it is new. */
    register Proc *procPtr)	/* Points to structure describing procedure
				 * containing the variable reference. */
{
    register CompiledLocal *localPtr;
    int localVar = -1;
    register int i;

    /*
     * If not creating a temporary, does a local variable of the specified
     * name already exist?
     */

    if (name != NULL) {
	int localCt = procPtr->numCompiledLocals;

	localPtr = procPtr->firstLocalPtr;
	for (i = 0;  i < localCt;  i++) {
	    if (!TclIsVarTemporary(localPtr)) {
		char *localName = localPtr->name;

		if ((nameBytes == localPtr->nameLength) &&
			(strncmp(name,localName,(unsigned)nameBytes) == 0)) {
		    return i;
		}
	    }
	    localPtr = localPtr->nextPtr;
	}
    }

    /*
     * Create a new variable if appropriate.
     */

    if (create || (name == NULL)) {
	localVar = procPtr->numCompiledLocals;
	localPtr = (CompiledLocal *) ckalloc((unsigned)
		(sizeof(CompiledLocal) - sizeof(localPtr->name)
		+ nameBytes + 1));
	if (procPtr->firstLocalPtr == NULL) {
	    procPtr->firstLocalPtr = procPtr->lastLocalPtr = localPtr;
	} else {
	    procPtr->lastLocalPtr->nextPtr = localPtr;
	    procPtr->lastLocalPtr = localPtr;
	}
	localPtr->nextPtr = NULL;
	localPtr->nameLength = nameBytes;
	localPtr->frameIndex = localVar;
	localPtr->flags = 0;
	if (name == NULL) {
	    localPtr->flags |= VAR_TEMPORARY;
	}
	localPtr->defValuePtr = NULL;
	localPtr->resolveInfo = NULL;

	if (name != NULL) {
	    memcpy(localPtr->name, name, (size_t) nameBytes);
	}
	localPtr->name[nameBytes] = '\0';
	procPtr->numCompiledLocals++;
    }
    return localVar;
}
```

tcl8.6 8.6.18: `TclCompileSetCmd` in `generic/tclCompCmdsSZ.c`, lines 125–197; full-file SHA-256 `b41bd49a6d4e4d550dda90b4788d4d3928b30479f1d2a84a02f09be7ce4dd942`; excerpt SHA-256 `ec6366d9632f55a840c255b1cb09270653daeba0155e462257ef175f3b0e84e9`; evidence `e8`.

```c
TclCompileSetCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int isAssignment, isScalar, localIndex, numWords;

    numWords = parsePtr->numWords;
    if ((numWords != 2) && (numWords != 3)) {
	return TCL_ERROR;
    }
    isAssignment = (numWords == 3);

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    PushVarNameWord(interp, varTokenPtr, envPtr, 0,
	    &localIndex, &isScalar, 1);

    /*
     * If we are doing an assignment, push the new value.
     */

    if (isAssignment) {
	valueTokenPtr = TokenAfter(varTokenPtr);
	CompileWord(envPtr, valueTokenPtr, interp, 2);
    }

    /*
     * Emit instructions to set/get the variable.
     */

	if (isScalar) {
	    if (localIndex < 0) {
		TclEmitOpcode((isAssignment?
			INST_STORE_STK : INST_LOAD_STK), envPtr);
	    } else if (localIndex <= 255) {
		TclEmitInstInt1((isAssignment?
			INST_STORE_SCALAR1 : INST_LOAD_SCALAR1),
			localIndex, envPtr);
	    } else {
		TclEmitInstInt4((isAssignment?
			INST_STORE_SCALAR4 : INST_LOAD_SCALAR4),
			localIndex, envPtr);
	    }
	} else {
	    if (localIndex < 0) {
		TclEmitOpcode((isAssignment?
			INST_STORE_ARRAY_STK : INST_LOAD_ARRAY_STK), envPtr);
	    } else if (localIndex <= 255) {
		TclEmitInstInt1((isAssignment?
			INST_STORE_ARRAY1 : INST_LOAD_ARRAY1),
			localIndex, envPtr);
	    } else {
		TclEmitInstInt4((isAssignment?
			INST_STORE_ARRAY4 : INST_LOAD_ARRAY4),
			localIndex, envPtr);
	    }
	}

    return TCL_OK;
}
```

tcl8.6 8.6.18: `TclPushVarName` in `generic/tclCompCmds.c`, lines 3387–3591; full-file SHA-256 `c42255a9d4ebb83dfb6f7d956ca411e746a434297b703194e2832eefe705a13e`; excerpt SHA-256 `414e0c602ecd47ea0909bc2af3016540dd61de38a199e3512377e4b06a328fba`; evidence `e9`.

```c
TclPushVarName(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Token *varTokenPtr,	/* Points to a variable token. */
    CompileEnv *envPtr,		/* Holds resulting instructions. */
    int flags,			/* TCL_NO_LARGE_INDEX | TCL_NO_ELEMENT. */
    int *localIndexPtr,		/* Must not be NULL. */
    int *isScalarPtr)		/* Must not be NULL. */
{
    const char *p;
    const char *last, *name, *elName;
    int n;
    Tcl_Token *elemTokenPtr = NULL;
    int nameLen, elNameLen, simpleVarName, localIndex;
    int elemTokenCount = 0, allocedTokens = 0, removedParen = 0;

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    simpleVarName = 0;
    name = elName = NULL;
    nameLen = elNameLen = 0;
    localIndex = -1;

    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	/*
	 * A simple variable name. Divide it up into "name" and "elName"
	 * strings. If it is not a local variable, look it up at runtime.
	 */

	simpleVarName = 1;

	name = varTokenPtr[1].start;
	nameLen = varTokenPtr[1].size;
	if (nameLen > 0 && name[nameLen-1] == ')') {
	    /*
	     * last char is ')' => potential array reference.
	     */
	    last = &name[nameLen-1];

	    if (*last == ')') {
		for (p = name;  p < last;  p++) {
		    if (*p == '(') {
			elName = p + 1;
			elNameLen = last - elName;
			nameLen = p - name;
			break;
		    }
		}
	    }

	    if (!(flags & TCL_NO_ELEMENT) && elNameLen) {
		/*
		 * An array element, the element name is a simple string:
		 * assemble the corresponding token.
		 */

		elemTokenPtr = (Tcl_Token *)TclStackAlloc(interp, sizeof(Tcl_Token));
		allocedTokens = 1;
		elemTokenPtr->type = TCL_TOKEN_TEXT;
		elemTokenPtr->start = elName;
		elemTokenPtr->size = elNameLen;
		elemTokenPtr->numComponents = 0;
		elemTokenCount = 1;
	    }
	}
    } else if (interp && ((n = varTokenPtr->numComponents) > 1)
	    && (varTokenPtr[1].type == TCL_TOKEN_TEXT)
	    && (varTokenPtr[n].type == TCL_TOKEN_TEXT)
	    && (*(varTokenPtr[n].start + varTokenPtr[n].size - 1) == ')')) {
	/*
	 * Check for parentheses inside first token.
	 */

	simpleVarName = 0;
	for (p = varTokenPtr[1].start,
	     last = p + varTokenPtr[1].size;  p < last;  p++) {
	    if (*p == '(') {
		simpleVarName = 1;
		break;
	    }
	}
	if (simpleVarName) {
	    int remainingLen;

	    /*
	     * Check the last token: if it is just ')', do not count it.
	     * Otherwise, remove the ')' and flag so that it is restored at
	     * the end.
	     */

	    if (varTokenPtr[n].size == 1) {
		n--;
	    } else {
		varTokenPtr[n].size--;
		removedParen = n;
	    }

	    name = varTokenPtr[1].start;
	    nameLen = p - varTokenPtr[1].start;
	    elName = p + 1;
	    remainingLen = (varTokenPtr[2].start - p) - 1;
	    elNameLen = (varTokenPtr[n].start-p) + varTokenPtr[n].size - 1;

	    if (!(flags & TCL_NO_ELEMENT)) {
	      if (remainingLen) {
		/*
		 * Make a first token with the extra characters in the first
		 * token.
		 */

		elemTokenPtr = (Tcl_Token *)TclStackAlloc(interp, n * sizeof(Tcl_Token));
		allocedTokens = 1;
		elemTokenPtr->type = TCL_TOKEN_TEXT;
		elemTokenPtr->start = elName;
		elemTokenPtr->size = remainingLen;
		elemTokenPtr->numComponents = 0;
		elemTokenCount = n;

		/*
		 * Copy the remaining tokens.
		 */

		memcpy(elemTokenPtr+1, varTokenPtr+2,
			(n-1) * sizeof(Tcl_Token));
	      } else {
		/*
		 * Use the already available tokens.
		 */

		elemTokenPtr = &varTokenPtr[2];
		elemTokenCount = n - 1;
	      }
	    }
	}
    }

    if (simpleVarName) {
	/*
	 * See whether name has any namespace separators (::'s).
	 */

	int hasNsQualifiers = 0;

	for (p = name, last = p + nameLen-1;  p < last;  p++) {
	    if ((*p == ':') && (*(p+1) == ':')) {
		hasNsQualifiers = 1;
		break;
	    }
	}

	/*
	 * Look up the var name's index in the array of local vars in the proc
	 * frame. If retrieving the var's value and it doesn't already exist,
	 * push its name and look it up at runtime.
	 */

	if (!hasNsQualifiers) {
	    localIndex = TclFindCompiledLocal(name, nameLen, 1, envPtr);
	    if ((flags & TCL_NO_LARGE_INDEX) && (localIndex > 255)) {
		/*
		 * We'll push the name.
		 */

		localIndex = -1;
	    }
	}
	if (interp && localIndex < 0) {
	    PushLiteral(envPtr, name, nameLen);
	}

	/*
	 * Compile the element script, if any, and only if not inhibited. [Bug
	 * 3600328]
	 */

	if (elName != NULL && !(flags & TCL_NO_ELEMENT)) {
	    if (elNameLen) {
		TclCompileTokens(interp, elemTokenPtr, elemTokenCount,
			envPtr);
	    } else {
		PushStringLiteral(envPtr, "");
	    }
	}
    } else if (interp) {
	/*
	 * The var name isn't simple: compile and push it.
	 */

	CompileTokens(envPtr, varTokenPtr, interp);
    }

    if (removedParen) {
	varTokenPtr[removedParen].size++;
    }
    if (allocedTokens) {
	TclStackFree(interp, elemTokenPtr);
    }
    *localIndexPtr = localIndex;
    *isScalarPtr = (elName == NULL);
}
```

tcl8.6 8.6.18: `TclCompileTokens` in `generic/tclCompile.c`, lines 2355–2550; full-file SHA-256 `d1a494975cb56b8850804669e17fb7c5aacc1d70de505ea661f59ca365ab45c8`; excerpt SHA-256 `0ac0749c57f793286af6437c49c109c4146287f3bb34658926e5802980819dd7`; evidence `e10`.

```c
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

tcl8.6 8.6.18: `TclFindCompiledLocal` in `generic/tclCompile.c`, lines 2933–3028; full-file SHA-256 `d1a494975cb56b8850804669e17fb7c5aacc1d70de505ea661f59ca365ab45c8`; excerpt SHA-256 `b4c836c0f9b9e6e7cf8515b2d2b32b4ab04e6442a41ef69a02ef5ded83dc134f`; evidence `e11`.

```c
TclFindCompiledLocal(
    const char *name,	/* Points to first character of the name of a
				 * scalar or array variable. If NULL, a
				 * temporary var should be created. */
    int nameBytes,		/* Number of bytes in the name. */
    int create,			/* If 1, allocate a local frame entry for the
				 * variable if it is new. */
    CompileEnv *envPtr)		/* Points to the current compile environment*/
{
    CompiledLocal *localPtr;
    int localVar = -1;
    int i;
    Proc *procPtr;

    /*
     * If not creating a temporary, does a local variable of the specified
     * name already exist?
     */

    procPtr = envPtr->procPtr;

    if (procPtr == NULL) {
	/*
	 * Compiling a non-body script: give it read access to the LVT in the
	 * current localCache
	 */

	LocalCache *cachePtr = envPtr->iPtr->varFramePtr->localCachePtr;
	const char *localName;
	Tcl_Obj **varNamePtr;
	int len;

	if (!cachePtr || !name) {
	    return -1;
	}

	varNamePtr = &cachePtr->varName0;
	for (i=0; i < cachePtr->numVars; varNamePtr++, i++) {
	    if (*varNamePtr) {
		localName = Tcl_GetStringFromObj(*varNamePtr, &len);
		if ((len == nameBytes) && !strncmp(name, localName, len)) {
		    return i;
		}
	    }
	}
	return -1;
    }

    if (name != NULL) {
	int localCt = procPtr->numCompiledLocals;

	localPtr = procPtr->firstLocalPtr;
	for (i = 0;  i < localCt;  i++) {
	    if (!TclIsVarTemporary(localPtr)) {
		char *localName = localPtr->name;

		if ((nameBytes == localPtr->nameLength) &&
			(strncmp(name, localName, nameBytes) == 0)) {
		    return i;
		}
	    }
	    localPtr = localPtr->nextPtr;
	}
    }

    /*
     * Create a new variable if appropriate.
     */

    if (create || (name == NULL)) {
	localVar = procPtr->numCompiledLocals;
	localPtr = (CompiledLocal *)ckalloc(TclOffset(CompiledLocal, name) + 1U + nameBytes);
	if (procPtr->firstLocalPtr == NULL) {
	    procPtr->firstLocalPtr = procPtr->lastLocalPtr = localPtr;
	} else {
	    procPtr->lastLocalPtr->nextPtr = localPtr;
	    procPtr->lastLocalPtr = localPtr;
	}
	localPtr->nextPtr = NULL;
	localPtr->nameLength = nameBytes;
	localPtr->frameIndex = localVar;
	localPtr->flags = 0;
	if (name == NULL) {
	    localPtr->flags |= VAR_TEMPORARY;
	}
	localPtr->defValuePtr = NULL;
	localPtr->resolveInfo = NULL;

	if (name != NULL) {
	    memcpy(localPtr->name, name, nameBytes);
	}
	localPtr->name[nameBytes] = '\0';
	procPtr->numCompiledLocals++;
    }
    return localVar;
}
```

tcl9.0 9.0.4: `TclCompileSetCmd` in `generic/tclCompCmdsSZ.c`, lines 124–195; full-file SHA-256 `55f4cd62b44a84d795804041a5a19ebe9e8ab49095bdb063f6e139df1cc6b4ac`; excerpt SHA-256 `34baed7dde71e469973938c03e11e8b92217cd3cf56e7430109715edb6e61098`; evidence `e12`.

```c
TclCompileSetCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int isAssignment, isScalar, localIndex, numWords;

    numWords = parsePtr->numWords;
    if ((numWords != 2) && (numWords != 3)) {
	return TCL_ERROR;
    }
    isAssignment = (numWords == 3);

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    PushVarNameWord(interp, varTokenPtr, envPtr, 0,
	    &localIndex, &isScalar, 1);

    /*
     * If we are doing an assignment, push the new value.
     */

    if (isAssignment) {
	valueTokenPtr = TokenAfter(varTokenPtr);
	CompileWord(envPtr, valueTokenPtr, interp, 2);
    }

    /*
     * Emit instructions to set/get the variable.
     */

	if (isScalar) {
	    if (localIndex < 0) {
		TclEmitOpcode((isAssignment?
			INST_STORE_STK : INST_LOAD_STK), envPtr);
	    } else if (localIndex <= 255) {
		TclEmitInstInt1((isAssignment?
			INST_STORE_SCALAR1 : INST_LOAD_SCALAR1),
			localIndex, envPtr);
	    } else {
		TclEmitInstInt4((isAssignment?
			INST_STORE_SCALAR4 : INST_LOAD_SCALAR4),
			localIndex, envPtr);
	    }
	} else {
	    if (localIndex < 0) {
		TclEmitOpcode((isAssignment?
			INST_STORE_ARRAY_STK : INST_LOAD_ARRAY_STK), envPtr);
	    } else if (localIndex <= 255) {
		TclEmitInstInt1((isAssignment?
			INST_STORE_ARRAY1 : INST_LOAD_ARRAY1),
			localIndex, envPtr);
	    } else {
		TclEmitInstInt4((isAssignment?
			INST_STORE_ARRAY4 : INST_LOAD_ARRAY4),
			localIndex, envPtr);
	    }
	}

    return TCL_OK;
}
```

tcl9.0 9.0.4: `TclPushVarName` in `generic/tclCompCmds.c`, lines 3488–3694; full-file SHA-256 `9170e5ff4f14aceb78d1627d4e8ab0b101500dc522e545b60dab855144c0a77a`; excerpt SHA-256 `26b20fe759673164bffb8c6201656258f3eba27f3200c3d7fadf21af5e6b5d30`; evidence `e13`.

```c
TclPushVarName(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Token *varTokenPtr,	/* Points to a variable token. */
    CompileEnv *envPtr,		/* Holds resulting instructions. */
    int flags,			/* TCL_NO_LARGE_INDEX | TCL_NO_ELEMENT. */
    int *localIndexPtr,		/* Must not be NULL. */
    int *isScalarPtr)		/* Must not be NULL. */
{
    const char *p;
    const char *last, *name, *elName;
    Tcl_Size n;
    Tcl_Token *elemTokenPtr = NULL;
	size_t nameLen, elNameLen;
    int simpleVarName, localIndex;
    Tcl_Size elemTokenCount = 0, removedParen = 0;
    int allocedTokens = 0;

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    simpleVarName = 0;
    name = elName = NULL;
    nameLen = elNameLen = 0;
    localIndex = -1;

    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	/*
	 * A simple variable name. Divide it up into "name" and "elName"
	 * strings. If it is not a local variable, look it up at runtime.
	 */

	simpleVarName = 1;

	name = varTokenPtr[1].start;
	nameLen = varTokenPtr[1].size;
	if (nameLen > 0 && name[nameLen-1] == ')') {
	    /*
	     * last char is ')' => potential array reference.
	     */
	    last = &name[nameLen-1];

	    if (*last == ')') {
		for (p = name;  p < last;  p++) {
		    if (*p == '(') {
			elName = p + 1;
			elNameLen = last - elName;
			nameLen = p - name;
			break;
		    }
		}
	    }

	    if (!(flags & TCL_NO_ELEMENT) && elNameLen) {
		/*
		 * An array element, the element name is a simple string:
		 * assemble the corresponding token.
		 */

		elemTokenPtr = (Tcl_Token *)TclStackAlloc(interp, sizeof(Tcl_Token));
		allocedTokens = 1;
		elemTokenPtr->type = TCL_TOKEN_TEXT;
		elemTokenPtr->start = elName;
		elemTokenPtr->size = elNameLen;
		elemTokenPtr->numComponents = 0;
		elemTokenCount = 1;
	    }
	}
    } else if (interp && ((n = varTokenPtr->numComponents) > 1)
	    && (varTokenPtr[1].type == TCL_TOKEN_TEXT)
	    && (varTokenPtr[n].type == TCL_TOKEN_TEXT)
	    && (*(varTokenPtr[n].start + varTokenPtr[n].size - 1) == ')')) {
	/*
	 * Check for parentheses inside first token.
	 */

	simpleVarName = 0;
	for (p = varTokenPtr[1].start, last = p + varTokenPtr[1].size;
		p < last;  p++) {
	    if (*p == '(') {
		simpleVarName = 1;
		break;
	    }
	}
	if (simpleVarName) {
	    size_t remainingLen;

	    /*
	     * Check the last token: if it is just ')', do not count it.
	     * Otherwise, remove the ')' and flag so that it is restored at
	     * the end.
	     */

	    if (varTokenPtr[n].size == 1) {
		n--;
	    } else {
		varTokenPtr[n].size--;
		removedParen = n;
	    }

	    name = varTokenPtr[1].start;
	    nameLen = p - varTokenPtr[1].start;
	    elName = p + 1;
	    remainingLen = (varTokenPtr[2].start - p) - 1;
	    elNameLen = (varTokenPtr[n].start-p) + varTokenPtr[n].size - 1;

	    if (!(flags & TCL_NO_ELEMENT)) {
	      if (remainingLen) {
		/*
		 * Make a first token with the extra characters in the first
		 * token.
		 */

		elemTokenPtr = (Tcl_Token *)TclStackAlloc(interp, n * sizeof(Tcl_Token));
		allocedTokens = 1;
		elemTokenPtr->type = TCL_TOKEN_TEXT;
		elemTokenPtr->start = elName;
		elemTokenPtr->size = remainingLen;
		elemTokenPtr->numComponents = 0;
		elemTokenCount = n;

		/*
		 * Copy the remaining tokens.
		 */

		memcpy(elemTokenPtr+1, varTokenPtr+2,
			(n-1) * sizeof(Tcl_Token));
	      } else {
		/*
		 * Use the already available tokens.
		 */

		elemTokenPtr = &varTokenPtr[2];
		elemTokenCount = n - 1;
	      }
	    }
	}
    }

    if (simpleVarName) {
	/*
	 * See whether name has any namespace separators (::'s).
	 */

	int hasNsQualifiers = 0;

	for (p = name, last = p + nameLen-1;  p < last;  p++) {
	    if ((p[0] == ':') && (p[1] == ':')) {
		hasNsQualifiers = 1;
		break;
	    }
	}

	/*
	 * Look up the var name's index in the array of local vars in the proc
	 * frame. If retrieving the var's value and it doesn't already exist,
	 * push its name and look it up at runtime.
	 */

	if (!hasNsQualifiers) {
	    localIndex = TclFindCompiledLocal(name, nameLen, 1, envPtr);
	    if ((flags & TCL_NO_LARGE_INDEX) && (localIndex > 255)) {
		/*
		 * We'll push the name.
		 */

		localIndex = -1;
	    }
	}
	if (interp && localIndex < 0) {
	    PushLiteral(envPtr, name, nameLen);
	}

	/*
	 * Compile the element script, if any, and only if not inhibited. [Bug
	 * 3600328]
	 */

	if (elName != NULL && !(flags & TCL_NO_ELEMENT)) {
	    if (elNameLen) {
		TclCompileTokens(interp, elemTokenPtr, elemTokenCount,
			envPtr);
	    } else {
		PushStringLiteral(envPtr, "");
	    }
	}
    } else if (interp) {
	/*
	 * The var name isn't simple: compile and push it.
	 */

	CompileTokens(envPtr, varTokenPtr, interp);
    }

    if (removedParen) {
	varTokenPtr[removedParen].size++;
    }
    if (allocedTokens) {
	TclStackFree(interp, elemTokenPtr);
    }
    *localIndexPtr = localIndex;
    *isScalarPtr = (elName == NULL);
}
```

tcl9.0 9.0.4: `TclCompileTokens` in `generic/tclCompile.c`, lines 2416–2610; full-file SHA-256 `51b709e7bf2c1f25bb7b3ab371c7dcbd6cf6d2bce509931397303cb770ee21f3`; excerpt SHA-256 `2f8084251e4df3b7d823a494d9d8b1e7b38ec050a58ca905efab16dd0b52ed33`; evidence `e14`.

```c
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

tcl9.0 9.0.4: `TclFindCompiledLocal` in `generic/tclCompile.c`, lines 3020–3115; full-file SHA-256 `51b709e7bf2c1f25bb7b3ab371c7dcbd6cf6d2bce509931397303cb770ee21f3`; excerpt SHA-256 `a87ffe2b6f62282e8c4c51da0e7e6b045d3470c4c40c9e44ceef8a3519f4f347`; evidence `e15`.

```c
TclFindCompiledLocal(
    const char *name,	/* Points to first character of the name of a
				 * scalar or array variable. If NULL, a
				 * temporary var should be created. */
    Tcl_Size nameBytes,		/* Number of bytes in the name. */
    int create,			/* If 1, allocate a local frame entry for the
				 * variable if it is new. */
    CompileEnv *envPtr)		/* Points to the current compile environment*/
{
    CompiledLocal *localPtr;
    Tcl_Size localVar = TCL_INDEX_NONE;
    Tcl_Size i;
    Proc *procPtr;

    /*
     * If not creating a temporary, does a local variable of the specified
     * name already exist?
     */

    procPtr = envPtr->procPtr;

    if (procPtr == NULL) {
	/*
	 * Compiling a non-body script: give it read access to the LVT in the
	 * current localCache
	 */

	LocalCache *cachePtr = envPtr->iPtr->varFramePtr->localCachePtr;
	const char *localName;
	Tcl_Obj **varNamePtr;
	Tcl_Size len;

	if (!cachePtr || !name) {
	    return TCL_INDEX_NONE;
	}

	varNamePtr = &cachePtr->varName0;
	for (i=0; i < cachePtr->numVars; varNamePtr++, i++) {
	    if (*varNamePtr) {
		localName = TclGetStringFromObj(*varNamePtr, &len);
		if ((len == nameBytes) && !strncmp(name, localName, len)) {
		    return i;
		}
	    }
	}
	return TCL_INDEX_NONE;
    }

    if (name != NULL) {
	Tcl_Size localCt = procPtr->numCompiledLocals;

	localPtr = procPtr->firstLocalPtr;
	for (i = 0;  i < localCt;  i++) {
	    if (!TclIsVarTemporary(localPtr)) {
		char *localName = localPtr->name;

		if ((nameBytes == localPtr->nameLength) &&
			(strncmp(name,localName,nameBytes) == 0)) {
		    return i;
		}
	    }
	    localPtr = localPtr->nextPtr;
	}
    }

    /*
     * Create a new variable if appropriate.
     */

    if (create || (name == NULL)) {
	localVar = procPtr->numCompiledLocals;
	localPtr = (CompiledLocal *)Tcl_Alloc(offsetof(CompiledLocal, name) + 1U + nameBytes);
	if (procPtr->firstLocalPtr == NULL) {
	    procPtr->firstLocalPtr = procPtr->lastLocalPtr = localPtr;
	} else {
	    procPtr->lastLocalPtr->nextPtr = localPtr;
	    procPtr->lastLocalPtr = localPtr;
	}
	localPtr->nextPtr = NULL;
	localPtr->nameLength = nameBytes;
	localPtr->frameIndex = localVar;
	localPtr->flags = 0;
	if (name == NULL) {
	    localPtr->flags |= VAR_TEMPORARY;
	}
	localPtr->defValuePtr = NULL;
	localPtr->resolveInfo = NULL;

	if (name != NULL) {
	    memcpy(localPtr->name, name, nameBytes);
	}
	localPtr->name[nameBytes] = '\0';
	procPtr->numCompiledLocals++;
    }
    return localVar;
}
```

tcl9.1 9.1.0: `TclCompileSetCmd` in `generic/tclCompCmdsSZ.c`, lines 136–212; full-file SHA-256 `931ca711522e331cc9f34aff756e770824a86ec97567197f5aa894ab90fc585a`; excerpt SHA-256 `9ab577969f25ab94cd5720caff9bc0e85081472c78b0ac071eb24d2475cf41bf`; evidence `e16`.

```c
TclCompileSetCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int isAssignment, isScalar;
    Tcl_Size numWords = parsePtr->numWords;
    Tcl_LVTIndex localIndex;

    if ((numWords != 2) && (numWords != 3)) {
	return TCL_ERROR;
    }
    isAssignment = (numWords == 3);

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    PushVarNameWord(varTokenPtr, 0, &localIndex, &isScalar, 1);
    if (OutOfUintRangeUpper(localIndex)) {
	return TCL_ERROR;
    }

    /*
     * If we are doing an assignment, push the new value.
     */

    if (isAssignment) {
	valueTokenPtr = TokenAfter(varTokenPtr);
	PUSH_TOKEN(		valueTokenPtr, 2);
    }

    /*
     * Emit instructions to set/get the variable.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    if (isAssignment) {
		OP(		STORE_STK);
	    } else {
		OP(		LOAD_STK);
	    }
	} else {
	    if (isAssignment) {
		OP4(		STORE_SCALAR, localIndex);
	    } else {
		OP4(		LOAD_SCALAR, localIndex);
	    }
	}
    } else {
	if (localIndex < 0) {
	    if (isAssignment) {
		OP(		STORE_ARRAY_STK);
	    } else {
		OP(		LOAD_ARRAY_STK);
	    }
	} else {
	    if (isAssignment) {
		OP4(		STORE_ARRAY, localIndex);
	    } else {
		OP4(		LOAD_ARRAY, localIndex);
	    }
	}
    }

    return TCL_OK;
}
```

tcl9.1 9.1.0: `TclPushVarName` in `generic/tclCompile.c`, lines 4919–5117; full-file SHA-256 `941eaa2c57e4d7d5266134c5b713dca5218d29eea951c89a0676c14b8260eb4f`; excerpt SHA-256 `8c671c78715232e3ba32aa25ded0482b4c4b7644f96bb23d5eceab79c589ea45`; evidence `e17`.

```c
TclPushVarName(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Token *varTokenPtr,	/* Points to a variable token. */
    CompileEnv *envPtr,		/* Holds resulting instructions. */
    int flags,			/* TCL_NO_ELEMENT. */
    Tcl_Size *localIndexPtr,	/* Must not be NULL. */
    int *isScalarPtr)		/* Must not be NULL. */
{
    const char *p;
    const char *last, *name, *elName;
    Tcl_Token *elemTokenPtr = NULL;
    Tcl_Size nameLen, elNameLen, n;
    int simpleVarName = 0, allocedTokens = 0;
    Tcl_Size elemTokenCount = 0, removedParen = 0;
    Tcl_LVTIndex localIndex;

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    name = elName = NULL;
    nameLen = elNameLen = 0;
    localIndex = TCL_INDEX_NONE;

    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	/*
	 * A simple variable name. Divide it up into "name" and "elName"
	 * strings. If it is not a local variable, look it up at runtime.
	 */

	simpleVarName = 1;

	name = varTokenPtr[1].start;
	nameLen = varTokenPtr[1].size;
	if (nameLen > 0 && name[nameLen - 1] == ')') {
	    /*
	     * last char is ')' => potential array reference.
	     */
	    last = &name[nameLen - 1];

	    if (*last == ')') {
		for (p = name;  p < last;  p++) {
		    if (*p == '(') {
			elName = p + 1;
			elNameLen = last - elName;
			nameLen = p - name;
			break;
		    }
		}
	    }

	    if (!(flags & TCL_NO_ELEMENT) && elNameLen) {
		/*
		 * An array element, the element name is a simple string:
		 * assemble the corresponding token.
		 */

		elemTokenPtr = (Tcl_Token *)TclStackAlloc(interp, sizeof(Tcl_Token));
		allocedTokens = 1;
		elemTokenPtr->type = TCL_TOKEN_TEXT;
		elemTokenPtr->start = elName;
		elemTokenPtr->size = elNameLen;
		elemTokenPtr->numComponents = 0;
		elemTokenCount = 1;
	    }
	}
    } else if (interp && ((n = varTokenPtr->numComponents) > 1)
	    && (varTokenPtr[1].type == TCL_TOKEN_TEXT)
	    && (varTokenPtr[n].type == TCL_TOKEN_TEXT)
	    && (varTokenPtr[n].start[varTokenPtr[n].size - 1] == ')')) {
	/*
	 * Check for parentheses inside first token.
	 */

	simpleVarName = 0;
	for (p = varTokenPtr[1].start, last = p + varTokenPtr[1].size;
		p < last;  p++) {
	    if (*p == '(') {
		simpleVarName = 1;
		break;
	    }
	}
	if (simpleVarName) {
	    size_t remainingLen;

	    /*
	     * Check the last token: if it is just ')', do not count it.
	     * Otherwise, remove the ')' and flag so that it is restored at
	     * the end.
	     */

	    if (varTokenPtr[n].size == 1) {
		n--;
	    } else {
		varTokenPtr[n].size--;
		removedParen = n;
	    }

	    name = varTokenPtr[1].start;
	    nameLen = p - varTokenPtr[1].start;
	    elName = p + 1;
	    remainingLen = (varTokenPtr[2].start - p) - 1;
	    elNameLen = (varTokenPtr[n].start-p) + varTokenPtr[n].size - 1;

	    if (!(flags & TCL_NO_ELEMENT)) {
		if (remainingLen) {
		    /*
		     * Make a first token with the extra characters in the
		     * first token.
		     */

		    elemTokenPtr = (Tcl_Token *)TclStackAlloc(interp,
			    n * sizeof(Tcl_Token));
		    allocedTokens = 1;
		    elemTokenPtr->type = TCL_TOKEN_TEXT;
		    elemTokenPtr->start = elName;
		    elemTokenPtr->size = remainingLen;
		    elemTokenPtr->numComponents = 0;
		    elemTokenCount = n;

		    /*
		     * Copy the remaining tokens.
		     */

		    memcpy(elemTokenPtr + 1, varTokenPtr + 2,
			    (n - 1) * sizeof(Tcl_Token));
		} else {
		    /*
		     * Use the already available tokens.
		     */

		    elemTokenPtr = &varTokenPtr[2];
		    elemTokenCount = n - 1;
		}
	    }
	}
    }

    if (simpleVarName) {
	/*
	 * See whether name has any namespace separators (::'s).
	 */

	int hasNsQualifiers = 0;

	for (p = name, last = p + nameLen-1;  p < last;  p++) {
	    if ((p[0] == ':') && (p[1] == ':')) {
		hasNsQualifiers = 1;
		break;
	    }
	}

	/*
	 * Look up the var name's index in the array of local vars in the proc
	 * frame. If retrieving the var's value and it doesn't already exist,
	 * push its name and look it up at runtime.
	 */

	if (!hasNsQualifiers) {
	    localIndex = TclFindCompiledLocal(name, nameLen, true, envPtr);
	}
	if (interp && localIndex < 0) {
	    PushLiteral(envPtr, name, nameLen);
	}

	/*
	 * Compile the element script, if any, and only if not inhibited. [Bug
	 * 3600328]
	 */

	if (elName != NULL && !(flags & TCL_NO_ELEMENT)) {
	    if (elNameLen) {
		TclCompileTokens(interp, elemTokenPtr, elemTokenCount,
			envPtr);
	    } else {
		PUSH(		"");
	    }
	}
    } else if (interp) {
	/*
	 * The var name isn't simple: compile and push it.
	 */

	CompileTokens(envPtr, varTokenPtr, interp);
    }

    if (removedParen) {
	varTokenPtr[removedParen].size++;
    }
    if (allocedTokens) {
	TclStackFree(interp, elemTokenPtr);
    }
    *localIndexPtr = localIndex;
    *isScalarPtr = (elName == NULL);
}
```

tcl9.1 9.1.0: `TclCompileTokens` in `generic/tclCompile.c`, lines 2739–2927; full-file SHA-256 `941eaa2c57e4d7d5266134c5b713dca5218d29eea951c89a0676c14b8260eb4f`; excerpt SHA-256 `0192854ccad7c779a5db78f6ab91a9c47d6edcf50c2ffd889e0384ffab8ee5ba`; evidence `e18`.

```c
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

tcl9.1 9.1.0: `TclFindCompiledLocal` in `generic/tclCompile.c`, lines 3337–3433; full-file SHA-256 `941eaa2c57e4d7d5266134c5b713dca5218d29eea951c89a0676c14b8260eb4f`; excerpt SHA-256 `49bb8900d55b9cd0c4bec628d60e387147c71948cac15462778a850635653ab2`; evidence `e19`.

```c
TclFindCompiledLocal(
    const char *name,		/* Points to first character of the name of a
				 * scalar or array variable. If NULL, a
				 * temporary var should be created. */
    Tcl_Size nameBytes,		/* Number of bytes in the name. */
    bool create,		/* If 1, allocate a local frame entry for the
				 * variable if it is new. */
    CompileEnv *envPtr)		/* Points to the current compile environment*/
{
    CompiledLocal *localPtr;
    Tcl_Size localVar = TCL_INDEX_NONE;
    Tcl_Size i;
    Proc *procPtr;

    /*
     * If not creating a temporary, does a local variable of the specified
     * name already exist?
     */

    procPtr = envPtr->procPtr;

    if (procPtr == NULL) {
	/*
	 * Compiling a non-body script: give it read access to the LVT in the
	 * current localCache
	 */

	LocalCache *cachePtr = envPtr->iPtr->varFramePtr->localCachePtr;
	const char *localName;
	Tcl_Obj **varNamePtr;
	Tcl_Size len;

	if (!cachePtr || !name) {
	    return TCL_INDEX_NONE;
	}

	varNamePtr = &cachePtr->varName0;
	for (i=0; i < cachePtr->numVars; varNamePtr++, i++) {
	    if (*varNamePtr) {
		localName = TclGetStringFromObj(*varNamePtr, &len);
		if ((len == nameBytes) && !strncmp(name, localName, len)) {
		    return i;
		}
	    }
	}
	return TCL_INDEX_NONE;
    }

    if (name != NULL) {
	Tcl_Size localCt = procPtr->numCompiledLocals;

	localPtr = procPtr->firstLocalPtr;
	for (i = 0;  i < localCt;  i++) {
	    if (!TclIsVarTemporary(localPtr)) {
		char *localName = localPtr->name;

		if ((nameBytes == localPtr->nameLength) &&
			(strncmp(name,localName,nameBytes) == 0)) {
		    return i;
		}
	    }
	    localPtr = localPtr->nextPtr;
	}
    }

    /*
     * Create a new variable if appropriate.
     */

    if (create || (name == NULL)) {
	localVar = procPtr->numCompiledLocals;
	localPtr = (CompiledLocal *)Tcl_Alloc(
		offsetof(CompiledLocal, name) + 1U + nameBytes);
	if (procPtr->firstLocalPtr == NULL) {
	    procPtr->firstLocalPtr = procPtr->lastLocalPtr = localPtr;
	} else {
	    procPtr->lastLocalPtr->nextPtr = localPtr;
	    procPtr->lastLocalPtr = localPtr;
	}
	localPtr->nextPtr = NULL;
	localPtr->nameLength = nameBytes;
	localPtr->frameIndex = localVar;
	localPtr->flags = 0;
	if (name == NULL) {
	    localPtr->flags |= VAR_TEMPORARY;
	}
	localPtr->defValuePtr = NULL;
	localPtr->resolveInfo = NULL;

	if (name != NULL) {
	    memcpy(localPtr->name, name, nameBytes);
	}
	localPtr->name[nameBytes] = '\0';
	procPtr->numCompiledLocals++;
    }
    return localVar;
}
```

## Consumer bindings

- `rust/tcl-registry/src/native_compilation.rs`: `native_compilation::tests::original_literal_set_compiler_effects_do_not_grant_opcode_admission` (linked). The Rust consumer excludes nested/dynamic vectors and retains Unknown native admission independently; this is not guest execution.

A linked selector records coverage, not executed results.

## Replay

Exact source inspection.

Compare pinned C release sources against exact full-file and definition hashes. No current native guest observation or Rust execution is implied by this source record.
