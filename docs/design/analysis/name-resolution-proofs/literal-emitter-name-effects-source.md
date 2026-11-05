# naming.compiler.literal-emitter-name-effects-source

Kind: `source-anchor`

## Problem statement

A source command can have a genuine stock compiler registration and complete original operands while its actual bytecode context remains unknown. The existing source walker treated Unknown opcode admission as an arbitrary command or namespace mutation, even when the selected compiler only prepares literal data and emits an instruction. Equally, literal outer words do not close a compiler that recursively compiles a body or expression, or prepares a fresh command-name object through a resolver-mediated getter. Those competing paths must be distinguished before the actual handler and argv are evaluated.

## Question

Which selected stock C compiler paths for complete substitution-free original vectors preserve tracked command and namespace bindings before argv evaluation, and which still require nested-compilation or resolver proof?

## Conclusion

The retained direct-emitter paths register or push literal data, allocate compiled-local metadata where eligible, and emit instructions without executing command scripts or changing tracked command or namespace bindings. Selected namespace/string cases are restricted to their direct-emitter branches; dictionary body compilers remain excluded. Named invocation helpers independently prepare a command-name object through Tcl_GetCommandFromObj, and body/expression/template compilers recursively compile source, so literal outer words alone do not close those obligations. This source explanation supports tracked-name effect separation only, not opcode admission, cached object identity, variable receiver validity or normal handler completion.

## Scope

Pinned C8.4.20, C8.5.19, C8.6.18, C9.0.4 and C9.1.0 function definitions inspected for actual selected direct-emitter families. Available definitions are retained per release; an unavailable release registration cannot be inferred from a later function. Registry registration and same-owner dependency checks remain independent. No native guest execution is claimed. Jim, BIG-IP, arbitrary compiler hooks, recursive scripts/expressions/templates, named-invocation resolver paths and unrepresented Array compiler branches are excluded.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `inspected`. Channel: Exact C source definitions, not a guest input channel. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Retained definitions distinguish direct literal emission/local metadata from excluded recursive-source or command-name resolver preparation.

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileAppendCmd`, lines 102–203. Retained reference `e0`.

```text
TclCompileAppendCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int simpleVarName, isScalar, localIndex, numWords;
    int code = TCL_OK;

    DefineLineInformation;

    numWords = parsePtr->numWords;
    if (numWords == 1) {
	Tcl_ResetResult(interp);
	Tcl_AppendToObj(Tcl_GetObjResult(interp),
		"wrong # args: should be \"append varName ?value value ...?\"",
		-1);
	return TCL_ERROR;
    } else if (numWords == 2) {
	/*
	 * append varName === set varName
	 */
        return TclCompileSetCmd(interp, parsePtr, envPtr);
    } else if (numWords > 3) {
	/*
	 * APPEND instructions currently only handle one value
	 */
        return TCL_OUT_LINE_COMPILE;
    }

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
     * We are doing an assignment, otherwise TclCompileSetCmd was called,
     * so push the new value.  This will need to be extended to push a
     * value for each argument.
     */

    if (numWords > 2) {
	valueTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
	if (valueTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    TclEmitPush(TclRegisterNewLiteral(envPtr, 
		    valueTokenPtr[1].start, valueTokenPtr[1].size), envPtr);
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
		    TclEmitInstInt1(INST_APPEND_SCALAR1, localIndex, envPtr);
		} else {
		    TclEmitInstInt4(INST_APPEND_SCALAR4, localIndex, envPtr);
		}
	    } else {
		TclEmitOpcode(INST_APPEND_STK, envPtr);
	    }
	} else {
	    if (localIndex >= 0) {
		if (localIndex <= 255) {
		    TclEmitInstInt1(INST_APPEND_ARRAY1, localIndex, envPtr);
		} else {
		    TclEmitInstInt4(INST_APPEND_ARRAY4, localIndex, envPtr);
		}
	    } else {
		TclEmitOpcode(INST_APPEND_ARRAY_STK, envPtr);
	    }
	}
    } else {
	TclEmitOpcode(INST_APPEND_STK, envPtr);
    }

    done:
    return code;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileLappendCmd`, lines 1715–1821. Retained reference `e1`.

```text
TclCompileLappendCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int simpleVarName, isScalar, localIndex, numWords;
    int code = TCL_OK;

    DefineLineInformation;

    /*
     * If we're not in a procedure, don't compile.
     */
    if (envPtr->procPtr == NULL) {
	return TCL_OUT_LINE_COMPILE;
    }

    numWords = parsePtr->numWords;
    if (numWords == 1) {
	Tcl_ResetResult(interp);
	Tcl_AppendToObj(Tcl_GetObjResult(interp),
		"wrong # args: should be \"lappend varName ?value value ...?\"", -1);
	return TCL_ERROR;
    }
    if (numWords != 3) {
	/*
	 * LAPPEND instructions currently only handle one value appends
	 */
        return TCL_OUT_LINE_COMPILE;
    }

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
     * In the no values case, create an empty object.
     */

    if (numWords > 2) {
	valueTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
	if (valueTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    TclEmitPush(TclRegisterNewLiteral(envPtr, 
		    valueTokenPtr[1].start, valueTokenPtr[1].size), envPtr);
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

    /*
     * The *_STK opcodes should be refactored to make better use of existing
     * LOAD/STORE instructions.
     */
    if (simpleVarName) {
	if (isScalar) {
	    if (localIndex >= 0) {
		if (localIndex <= 255) {
		    TclEmitInstInt1(INST_LAPPEND_SCALAR1, localIndex, envPtr);
		} else {
		    TclEmitInstInt4(INST_LAPPEND_SCALAR4, localIndex, envPtr);
		}
	    } else {
		TclEmitOpcode(INST_LAPPEND_STK, envPtr);
	    }
	} else {
	    if (localIndex >= 0) {
		if (localIndex <= 255) {
		    TclEmitInstInt1(INST_LAPPEND_ARRAY1, localIndex, envPtr);
		} else {
		    TclEmitInstInt4(INST_LAPPEND_ARRAY4, localIndex, envPtr);
		}
	    } else {
		TclEmitOpcode(INST_LAPPEND_ARRAY_STK, envPtr);
	    }
	}
    } else {
	TclEmitOpcode(INST_LAPPEND_STK, envPtr);
    }

    done:
    return code;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileSetCmd`, lines 2585–2684. Retained reference `e2`.

```text
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

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileIncrCmd`, lines 1568–1689. Retained reference `e3`.

```text
TclCompileIncrCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr, *incrTokenPtr;
    int simpleVarName, isScalar, localIndex, haveImmValue, immValue;
    int code = TCL_OK;

    DefineLineInformation;

    if ((parsePtr->numWords != 2) && (parsePtr->numWords != 3)) {
	Tcl_ResetResult(interp);
	Tcl_AppendToObj(Tcl_GetObjResult(interp),
	        "wrong # args: should be \"incr varName ?increment?\"", -1);
	return TCL_ERROR;
    }

    varTokenPtr = parsePtr->tokenPtr
	    + (parsePtr->tokenPtr->numComponents + 1);

    code = TclPushVarNameWord(interp, varTokenPtr, envPtr, 
	    (TCL_NO_LARGE_INDEX | TCL_CREATE_VAR),
	    &localIndex, &simpleVarName, &isScalar, 1);
    if (code != TCL_OK) {
	goto done;
    }

    /*
     * If an increment is given, push it, but see first if it's a small
     * integer.
     */

    haveImmValue = 0;
    immValue = 1;
    if (parsePtr->numWords == 3) {
	incrTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
	if (incrTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    CONST char *word = incrTokenPtr[1].start;
	    int numBytes = incrTokenPtr[1].size;

	    /*
	     * Note there is a danger that modifying the string could have
	     * undesirable side effects.  In this case, TclLooksLikeInt has
	     * no dependencies on shared strings so we should be safe.
	     */

	    if (TclLooksLikeInt(word, numBytes)) {
		int code;
		Tcl_Obj *intObj = Tcl_NewStringObj(word, numBytes);
		Tcl_IncrRefCount(intObj);
		code = Tcl_GetIntFromObj(NULL, intObj, &immValue);
		Tcl_DecrRefCount(intObj);
		if ((code == TCL_OK)
			&& (-127 <= immValue) && (immValue <= 127)) {
		    haveImmValue = 1;
		}
	    }
	    if (!haveImmValue) {
		TclEmitPush(
			TclRegisterNewLiteral(envPtr, word, numBytes), envPtr);
	    }
	} else {
	    SetLineInformation (2);
	    code = TclCompileTokens(interp, incrTokenPtr+1, 
	            incrTokenPtr->numComponents, envPtr);
	    if (code != TCL_OK) {
		goto done;
	    }
	}
    } else {			/* no incr amount given so use 1 */
	haveImmValue = 1;
    }
    
    /*
     * Emit the instruction to increment the variable.
     */

    if (simpleVarName) {
	if (isScalar) {
	    if (localIndex >= 0) {
		if (haveImmValue) {
		    TclEmitInstInt1(INST_INCR_SCALAR1_IMM, localIndex, envPtr);
		    TclEmitInt1(immValue, envPtr);
		} else {
		    TclEmitInstInt1(INST_INCR_SCALAR1, localIndex, envPtr);
		}
	    } else {
		if (haveImmValue) {
		    TclEmitInstInt1(INST_INCR_SCALAR_STK_IMM, immValue, envPtr);
		} else {
		    TclEmitOpcode(INST_INCR_SCALAR_STK, envPtr);
		}
	    }
	} else {
	    if (localIndex >= 0) {
		if (haveImmValue) {
		    TclEmitInstInt1(INST_INCR_ARRAY1_IMM, localIndex, envPtr);
		    TclEmitInt1(immValue, envPtr);
		} else {
		    TclEmitInstInt1(INST_INCR_ARRAY1, localIndex, envPtr);
		}
	    } else {
		if (haveImmValue) {
		    TclEmitInstInt1(INST_INCR_ARRAY_STK_IMM, immValue, envPtr);
		} else {
		    TclEmitOpcode(INST_INCR_ARRAY_STK, envPtr);
		}
	    }
	}
    } else {			/* non-simple variable name */
	if (haveImmValue) {
	    TclEmitInstInt1(INST_INCR_STK_IMM, immValue, envPtr);
	} else {
	    TclEmitOpcode(INST_INCR_STK, envPtr);
	}
    }
	
    done:
    return code;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileVariableCmd`, lines 2958–2993. Retained reference `e4`.

```text
TclCompileVariableCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr;
    int i, numWords;
    CONST char *varName, *tail;
    
    if (envPtr->procPtr == NULL) {
	return TCL_OUT_LINE_COMPILE;
    }

    numWords = parsePtr->numWords;
    
    varTokenPtr = parsePtr->tokenPtr
	+ (parsePtr->tokenPtr->numComponents + 1);
    for (i = 1; i < numWords; i += 2) {
	if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    varName = varTokenPtr[1].start;
	    tail = varName + varTokenPtr[1].size - 1;
	    if ((*tail == ')') || (tail < varName)) continue;
	    while ((tail > varName) && ((*tail != ':') || (*(tail-1) != ':'))) {
		tail--;
	    }
	    if ((*tail == ':') && (tail > varName)) {
		tail++;
	    }
	    (void) TclFindCompiledLocal(tail, (tail-varName+1),
		    /*create*/ 1, /*flags*/ 0, envPtr->procPtr);
	    varTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
	}
    }
    return TCL_OUT_LINE_COMPILE;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileListCmd`, lines 1928–1978. Retained reference `e5`.

```text
TclCompileListCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    DefineLineInformation;

    /*
     * If we're not in a procedure, don't compile.
     */
    if (envPtr->procPtr == NULL) {
	return TCL_OUT_LINE_COMPILE;
    }

    if (parsePtr->numWords == 1) {
	/*
	 * Empty args case
	 */

	TclEmitPush(TclRegisterNewLiteral(envPtr, "", 0), envPtr);
    } else {
	/*
	 * Push the all values onto the stack.
	 */
	Tcl_Token *valueTokenPtr;
	int i, code, numWords;

	numWords = parsePtr->numWords;

	valueTokenPtr = parsePtr->tokenPtr
	    + (parsePtr->tokenPtr->numComponents + 1);
	for (i = 1; i < numWords; i++) {
	    if (valueTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
		TclEmitPush(TclRegisterNewLiteral(envPtr,
			valueTokenPtr[1].start, valueTokenPtr[1].size), envPtr);
	    } else {
		SetLineInformation (i);
		code = TclCompileTokens(interp, valueTokenPtr+1,
			valueTokenPtr->numComponents, envPtr);
		if (code != TCL_OK) {
		    return code;
		}
	    }
	    valueTokenPtr = valueTokenPtr + (valueTokenPtr->numComponents + 1);
	}
	TclEmitInstInt4(INST_LIST, numWords - 1, envPtr);
    }

    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileLindexCmd`, lines 1845–1902. Retained reference `e6`.

```text
TclCompileLindexCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr;
    int code, i;
    int numWords;

    DefineLineInformation;

    numWords = parsePtr->numWords;

    /*
     * Quit if too few args
     */

    if ( numWords <= 1 ) {
	return TCL_OUT_LINE_COMPILE;
    }

    varTokenPtr = parsePtr->tokenPtr
	+ (parsePtr->tokenPtr->numComponents + 1);
    
    /*
     * Push the operands onto the stack.
     */
	
    for ( i = 1 ; i < numWords ; i++ ) {
	if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    TclEmitPush(
		    TclRegisterNewLiteral( envPtr, varTokenPtr[1].start,
		    varTokenPtr[1].size), envPtr);
	} else {
	    SetLineInformation (i);
	    code = TclCompileTokens(interp, varTokenPtr+1,
				    varTokenPtr->numComponents, envPtr);
	    if (code != TCL_OK) {
		return code;
	    }
	}
	varTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
    }
	
    /*
     * Emit INST_LIST_INDEX if objc==3, or INST_LIST_INDEX_MULTI
     * if there are multiple index args.
     */

    if ( numWords == 3 ) {
	TclEmitOpcode( INST_LIST_INDEX, envPtr );
    } else {
 	TclEmitInstInt4( INST_LIST_INDEX_MULTI, numWords-1, envPtr );
    }

    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileLlengthCmd`, lines 2002–2038. Retained reference `e7`.

```text
TclCompileLlengthCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr;
    int code;

    DefineLineInformation;

    if (parsePtr->numWords != 2) {
	Tcl_SetResult(interp, "wrong # args: should be \"llength list\"",
		TCL_STATIC);
	return TCL_ERROR;
    }
    varTokenPtr = parsePtr->tokenPtr
	+ (parsePtr->tokenPtr->numComponents + 1);

    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	/*
	 * We could simply count the number of elements here and push
	 * that value, but that is too rare a case to waste the code space.
	 */
	TclEmitPush(TclRegisterNewLiteral(envPtr, varTokenPtr[1].start,
		varTokenPtr[1].size), envPtr);
    } else {
	SetLineInformation (1);
	code = TclCompileTokens(interp, varTokenPtr+1,
		varTokenPtr->numComponents, envPtr);
	if (code != TCL_OK) {
	    return code;
	}
    }
    TclEmitOpcode(INST_LIST_LENGTH, envPtr);
    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileLsetCmd`, lines 2085–2239. Retained reference `e8`.

```text
TclCompileLsetCmd( interp, parsePtr, envPtr )
    Tcl_Interp* interp;		/* Tcl interpreter for error reporting */
    Tcl_Parse* parsePtr;	/* Points to a parse structure for
				 * the command */
    CompileEnv* envPtr;		/* Holds the resulting instructions */
{

    int tempDepth;		/* Depth used for emitting one part
				 * of the code burst. */
    Tcl_Token* varTokenPtr;	/* Pointer to the Tcl_Token representing
				 * the parse of the variable name */

    int result;			/* Status return from library calls */

    int localIndex;		/* Index of var in local var table */
    int simpleVarName;		/* Flag == 1 if var name is simple */
    int isScalar;		/* Flag == 1 if scalar, 0 if array */

    int i;

    DefineLineInformation;

    /* Check argument count */

    if ( parsePtr->numWords < 3 ) {
	/* Fail at run time, not in compilation */
	return TCL_OUT_LINE_COMPILE;
    }

    /*
     * Decide if we can use a frame slot for the var/array name or if we
     * need to emit code to compute and push the name at runtime. We use a
     * frame slot (entry in the array of local vars) if we are compiling a
     * procedure body and if the name is simple text that does not include
     * namespace qualifiers. 
     */

    varTokenPtr = parsePtr->tokenPtr
	    + (parsePtr->tokenPtr->numComponents + 1);
    result = TclPushVarNameWord( interp, varTokenPtr, envPtr, 
            TCL_CREATE_VAR, &localIndex, &simpleVarName, &isScalar, 1);
    if (result != TCL_OK) {
	return result;
    }

    /* Push the "index" args and the new element value. */

    for ( i = 2; i < parsePtr->numWords; ++i ) {

	/* Advance to next arg */

	varTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);

	/* Push an arg */

	if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    TclEmitPush(TclRegisterNewLiteral( envPtr, varTokenPtr[1].start,
		    varTokenPtr[1].size), envPtr);
	} else {
	    SetLineInformation (i);
	    result = TclCompileTokens(interp, varTokenPtr+1,
				      varTokenPtr->numComponents, envPtr);
	    if ( result != TCL_OK ) {
		return result;
	    }
	}
    }

    /*
     * Duplicate the variable name if it's been pushed.  
     */

    if ( !simpleVarName || localIndex < 0 ) {
	if ( !simpleVarName || isScalar ) {
	    tempDepth = parsePtr->numWords - 2;
	} else {
	    tempDepth = parsePtr->numWords - 1;
	}
	TclEmitInstInt4( INST_OVER, tempDepth, envPtr );
    }

    /*
     * Duplicate an array index if one's been pushed
     */

    if ( simpleVarName && !isScalar ) {
	if ( localIndex < 0 ) {
	    tempDepth = parsePtr->numWords - 1;
	} else {
	    tempDepth = parsePtr->numWords - 2;
	}
	TclEmitInstInt4( INST_OVER, tempDepth, envPtr );
    }

    /*
     * Emit code to load the variable's value.
     */

    if ( !simpleVarName ) {
	TclEmitOpcode( INST_LOAD_STK, envPtr );
    } else if ( isScalar ) {
	if ( localIndex < 0 ) {
	    TclEmitOpcode( INST_LOAD_SCALAR_STK, envPtr );
	} else if ( localIndex < 0x100 ) {
	    TclEmitInstInt1( INST_LOAD_SCALAR1, localIndex, envPtr );
	} else {
	    TclEmitInstInt4( INST_LOAD_SCALAR4, localIndex, envPtr );
	}
    } else {
	if ( localIndex < 0 ) {
	    TclEmitOpcode( INST_LOAD_ARRAY_STK, envPtr );
	} else if ( localIndex < 0x100 ) {
	    TclEmitInstInt1( INST_LOAD_ARRAY1, localIndex, envPtr );
	} else {
	    TclEmitInstInt4( INST_LOAD_ARRAY4, localIndex, envPtr );
	}
    }

    /*
     * Emit the correct variety of 'lset' instruction
     */

    if ( parsePtr->numWords == 4 ) {
	TclEmitOpcode( INST_LSET_LIST, envPtr );
    } else {
	TclEmitInstInt4( INST_LSET_FLAT, (parsePtr->numWords - 1), envPtr );
    }

    /*
     * Emit code to put the value back in the variable
     */

    if ( !simpleVarName ) {
	TclEmitOpcode( INST_STORE_STK, envPtr );
    } else if ( isScalar ) {
	if ( localIndex < 0 ) {
	    TclEmitOpcode( INST_STORE_SCALAR_STK, envPtr );
	} else if ( localIndex < 0x100 ) {
	    TclEmitInstInt1( INST_STORE_SCALAR1, localIndex, envPtr );
	} else {
	    TclEmitInstInt4( INST_STORE_SCALAR4, localIndex, envPtr );
	}
    } else {
	if ( localIndex < 0 ) {
	    TclEmitOpcode( INST_STORE_ARRAY_STK, envPtr );
	} else if ( localIndex < 0x100 ) {
	    TclEmitInstInt1( INST_STORE_ARRAY1, localIndex, envPtr );
	} else {
	    TclEmitInstInt4( INST_STORE_ARRAY4, localIndex, envPtr );
	}
    }
    
    return TCL_OK;

}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileReturnCmd`, lines 2465–2559. Retained reference `e9`.

```text
TclCompileReturnCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr;
    int code;
    int index = envPtr->exceptArrayNext - 1;

    DefineLineInformation;

    /*
     * If we're not in a procedure, don't compile.
     */

    if (envPtr->procPtr == NULL) {
	return TCL_OUT_LINE_COMPILE;
    }

    /*
     * Look back through the ExceptionRanges of the current CompileEnv,
     * from exceptArrayPtr[(exceptArrayNext - 1)] down to 
     * exceptArrayPtr[0] to see if any of them is an enclosing [catch].
     * If there's an enclosing [catch], don't compile.
     */

    while (index >= 0) {
	ExceptionRange range = envPtr->exceptArrayPtr[index];
	if ((range.type == CATCH_EXCEPTION_RANGE) 
		&& (range.catchOffset == -1)) {
	    return TCL_OUT_LINE_COMPILE;
	}
	index--;
    }

    switch (parsePtr->numWords) {
	case 1: {
	    /*
	     * Simple case:  [return]
	     * Just push the literal string "".
	     */
	    TclEmitPush(TclRegisterNewLiteral(envPtr, "", 0), envPtr);
	    break;
	}
	case 2: {
	    /*
	     * More complex cases:
	     * [return "foo"]
	     * [return $value]
	     * [return [otherCmd]]
	     */
	    varTokenPtr = parsePtr->tokenPtr
		+ (parsePtr->tokenPtr->numComponents + 1);
	    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
		/*
		 * [return "foo"] case:  the parse token is a simple word,
		 * so just push it.
		 */
		TclEmitPush(TclRegisterNewLiteral(envPtr, varTokenPtr[1].start,
			varTokenPtr[1].size), envPtr);
	    } else {
		/*
		 * Parse token is more complex, so compile it; this handles the
		 * variable reference and nested command cases.  If the
		 * parse token can be byte-compiled, then this instance of
		 * "return" will be byte-compiled; otherwise it will be
		 * out line compiled.
		 */
		SetLineInformation (1);
		code = TclCompileTokens(interp, varTokenPtr+1,
			varTokenPtr->numComponents, envPtr);
		if (code != TCL_OK) {
		    return code;
		}
	    }
	    break;
	}
	default: {
	    /*
	     * Most complex return cases: everything else, including
	     * [return -code error], etc.
	     */
	    return TCL_OUT_LINE_COMPILE;
	}
    }

    /*
     * The INST_DONE opcode actually causes the branching out of the
     * subroutine, and takes the top stack item as the return result
     * (which is why we pushed the value above).
     */
    TclEmitOpcode(INST_DONE, envPtr);
    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileBreakCmd`, lines 225–244. Retained reference `e10`.

```text
TclCompileBreakCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    if (parsePtr->numWords != 1) {
	Tcl_ResetResult(interp);
	Tcl_AppendToObj(Tcl_GetObjResult(interp),
	        "wrong # args: should be \"break\"", -1);
	return TCL_ERROR;
    }

    /*
     * Emit a break instruction.
     */

    TclEmitOpcode(INST_BREAK, envPtr);
    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileContinueCmd`, lines 442–465. Retained reference `e11`.

```text
TclCompileContinueCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    /*
     * There should be no argument after the "continue".
     */

    if (parsePtr->numWords != 1) {
	Tcl_ResetResult(interp);
	Tcl_AppendToObj(Tcl_GetObjResult(interp),
	        "wrong # args: should be \"continue\"", -1);
	return TCL_ERROR;
    }

    /*
     * Emit a continue instruction.
     */

    TclEmitOpcode(INST_CONTINUE, envPtr);
    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileRegexpCmd`, lines 2264–2439. Retained reference `e12`.

```text
TclCompileRegexpCmd(interp, parsePtr, envPtr)
    Tcl_Interp* interp;		/* Tcl interpreter for error reporting */
    Tcl_Parse* parsePtr;	/* Points to a parse structure for
				 * the command */
    CompileEnv* envPtr;		/* Holds the resulting instructions */
{
    Tcl_Token *varTokenPtr;	/* Pointer to the Tcl_Token representing
				 * the parse of the RE or string */
    int i, len, code, nocase, anchorLeft, anchorRight, start;
    char *str;

    DefineLineInformation;

    /*
     * We are only interested in compiling simple regexp cases.
     * Currently supported compile cases are:
     *   regexp ?-nocase? ?--? staticString $var
     *   regexp ?-nocase? ?--? {^staticString$} $var
     */
    if (parsePtr->numWords < 3) {
	return TCL_OUT_LINE_COMPILE;
    }

    nocase = 0;
    varTokenPtr = parsePtr->tokenPtr;

    /*
     * We only look for -nocase and -- as options.  Everything else
     * gets pushed to runtime execution.  This is different than regexp's
     * runtime option handling, but satisfies our stricter needs.
     */
    for (i = 1; i < parsePtr->numWords - 2; i++) {
	varTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
	if (varTokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    /* Not a simple string - punt to runtime. */
	    return TCL_OUT_LINE_COMPILE;
	}
	str = (char *) varTokenPtr[1].start;
	len = varTokenPtr[1].size;
	if ((len == 2) && (str[0] == '-') && (str[1] == '-')) {
	    i++;
	    break;
	} else if ((len > 1)
		&& (strncmp(str, "-nocase", (unsigned) len) == 0)) {
	    nocase = 1;
	} else {
	    /* Not an option we recognize. */
	    return TCL_OUT_LINE_COMPILE;
	}
    }

    if ((parsePtr->numWords - i) != 2) {
	/* We don't support capturing to variables */
	return TCL_OUT_LINE_COMPILE;
    }

    /*
     * Get the regexp string.  If it is not a simple string, punt to runtime.
     * If it has a '-', it could be an incorrectly formed regexp command.
     */
    varTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
    str = (char *) varTokenPtr[1].start;
    len = varTokenPtr[1].size;
    if ((varTokenPtr->type != TCL_TOKEN_SIMPLE_WORD) || (*str == '-')) {
	return TCL_OUT_LINE_COMPILE;
    }

    if (len == 0) {
	/*
	 * The semantics of regexp are always match on re == "".
	 */
	TclEmitPush(TclRegisterNewLiteral(envPtr, "1", 1), envPtr);
	return TCL_OK;
    }

    /*
     * Make a copy of the string that is null-terminated for checks which
     * require such.
     */
    str = (char *) ckalloc((unsigned) len + 1);
    strncpy(str, varTokenPtr[1].start, (size_t) len);
    str[len] = '\0';
    start = 0;

    /*
     * Check for anchored REs (ie ^foo$), so we can use string equal if
     * possible. Do not alter the start of str so we can free it correctly.
     */
    if (str[0] == '^') {
	start++;
	anchorLeft = 1;
    } else {
	anchorLeft = 0;
    }
    if ((str[len-1] == '$') && ((len == 1) || (str[len-2] != '\\'))) {
	anchorRight = 1;
	str[--len] = '\0';
    } else {
	anchorRight = 0;
    }

    /*
     * On the first (pattern) arg, check to see if any RE special characters
     * are in the word.  If not, this is the same as 'string equal'.
     */
    if ((len > (1+start)) && (str[start] == '.') && (str[start+1] == '*')) {
	start += 2;
	anchorLeft = 0;
    }
    if ((len > (2+start)) && (str[len-3] != '\\')
	    && (str[len-2] == '.') && (str[len-1] == '*')) {
	len -= 2;
	str[len] = '\0';
	anchorRight = 0;
    }

    /*
     * Don't do anything with REs with other special chars.  Also check if
     * this is a bad RE (do this at the end because it can be expensive).
     * If so, let it complain at runtime.
     */
    if ((strpbrk(str + start, "*+?{}()[].\\|^$") != NULL)
	    || (Tcl_RegExpCompile(NULL, str) == NULL)) {
	ckfree((char *) str);
	return TCL_OUT_LINE_COMPILE;
    }

    if (anchorLeft && anchorRight) {
	TclEmitPush(TclRegisterNewLiteral(envPtr, str+start, len-start),
		envPtr);
    } else {
	/*
	 * This needs to find the substring anywhere in the string, so
	 * use string match and *foo*, with appropriate anchoring.
	 */
	char *newStr  = ckalloc((unsigned) len + 3);
	len -= start;
	if (anchorLeft) {
	    strncpy(newStr, str + start, (size_t) len);
	} else {
	    newStr[0] = '*';
	    strncpy(newStr + 1, str + start, (size_t) len++);
	}
	if (!anchorRight) {
	    newStr[len++] = '*';
	}
	newStr[len] = '\0';
	TclEmitPush(TclRegisterNewLiteral(envPtr, newStr, len), envPtr);
	ckfree((char *) newStr);
    }
    ckfree((char *) str);

    /*
     * Push the string arg
     */
    varTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	TclEmitPush(TclRegisterNewLiteral(envPtr,
		varTokenPtr[1].start, varTokenPtr[1].size), envPtr);
    } else {
	SetLineInformation (parsePtr->numWords-1);
	code = TclCompileTokens(interp, varTokenPtr+1,
		varTokenPtr->numComponents, envPtr);
	if (code != TCL_OK) {
	    return code;
	}
    }

    if (anchorLeft && anchorRight && !nocase) {
	TclEmitOpcode(INST_STR_EQ, envPtr);
    } else {
	TclEmitInstInt1(INST_STR_MATCH, nocase, envPtr);
    }

    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclCompileStringCmd`, lines 2708–2939. Retained reference `e13`.

```text
TclCompileStringCmd(interp, parsePtr, envPtr)
    Tcl_Interp *interp;		/* Used for error reporting. */
    Tcl_Parse *parsePtr;	/* Points to a parse structure for the
				 * command created by Tcl_ParseCommand. */
    CompileEnv *envPtr;		/* Holds resulting instructions. */
{
    Tcl_Token *opTokenPtr, *varTokenPtr;
    Tcl_Obj *opObj;
    int index;
    int code;
    
    static CONST char *options[] = {
	"bytelength",	"compare",	"equal",	"first",
	"index",	"is",		"last",		"length",
	"map",		"match",	"range",	"repeat",
	"replace",	"tolower",	"toupper",	"totitle",
	"trim",		"trimleft",	"trimright",
	"wordend",	"wordstart",	(char *) NULL
    };
    enum options {
	STR_BYTELENGTH,	STR_COMPARE,	STR_EQUAL,	STR_FIRST,
	STR_INDEX,	STR_IS,		STR_LAST,	STR_LENGTH,
	STR_MAP,	STR_MATCH,	STR_RANGE,	STR_REPEAT,
	STR_REPLACE,	STR_TOLOWER,	STR_TOUPPER,	STR_TOTITLE,
	STR_TRIM,	STR_TRIMLEFT,	STR_TRIMRIGHT,
	STR_WORDEND,	STR_WORDSTART
    };	  

    DefineLineInformation;

    if (parsePtr->numWords < 2) {
	/* Fail at run time, not in compilation */
	return TCL_OUT_LINE_COMPILE;
    }
    opTokenPtr = parsePtr->tokenPtr
	+ (parsePtr->tokenPtr->numComponents + 1);

    opObj = Tcl_NewStringObj(opTokenPtr->start, opTokenPtr->size);
    if (Tcl_GetIndexFromObj(interp, opObj, options, "option", 0,
	    &index) != TCL_OK) {
	Tcl_DecrRefCount(opObj);
	Tcl_ResetResult(interp);
	return TCL_OUT_LINE_COMPILE;
    }
    Tcl_DecrRefCount(opObj);

    varTokenPtr = opTokenPtr + (opTokenPtr->numComponents + 1);

    switch ((enum options) index) {
	case STR_BYTELENGTH:
	case STR_FIRST:
	case STR_IS:
	case STR_LAST:
	case STR_MAP:
	case STR_RANGE:
	case STR_REPEAT:
	case STR_REPLACE:
	case STR_TOLOWER:
	case STR_TOUPPER:
	case STR_TOTITLE:
	case STR_TRIM:
	case STR_TRIMLEFT:
	case STR_TRIMRIGHT:
	case STR_WORDEND:
	case STR_WORDSTART:
	    /*
	     * All other cases: compile out of line.
	     */
	    return TCL_OUT_LINE_COMPILE;

	case STR_COMPARE: 
	case STR_EQUAL: {
	    int i;
	    /*
	     * If there are any flags to the command, we can't byte compile it
	     * because the INST_STR_EQ bytecode doesn't support flags.
	     */

	    if (parsePtr->numWords != 4) {
		return TCL_OUT_LINE_COMPILE;
	    }

	    /*
	     * Push the two operands onto the stack.
	     */

	    for (i = 0; i < 2; i++) {
		if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
		    TclEmitPush(TclRegisterNewLiteral(envPtr,
			    varTokenPtr[1].start, varTokenPtr[1].size), envPtr);
		} else {
		    SetLineInformation (i);
		    code = TclCompileTokens(interp, varTokenPtr+1,
			    varTokenPtr->numComponents, envPtr);
		    if (code != TCL_OK) {
			return code;
		    }
		}
		varTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
	    }

	    TclEmitOpcode(((((enum options) index) == STR_COMPARE) ?
		    INST_STR_CMP : INST_STR_EQ), envPtr);
	    return TCL_OK;
	}
	case STR_INDEX: {
	    int i;

	    if (parsePtr->numWords != 4) {
		/* Fail at run time, not in compilation */
		return TCL_OUT_LINE_COMPILE;
	    }

	    /*
	     * Push the two operands onto the stack.
	     */

	    for (i = 0; i < 2; i++) {
		if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
		    TclEmitPush(TclRegisterNewLiteral(envPtr,
			    varTokenPtr[1].start, varTokenPtr[1].size), envPtr);
		} else {
		    SetLineInformation (i);
		    code = TclCompileTokens(interp, varTokenPtr+1,
			    varTokenPtr->numComponents, envPtr);
		    if (code != TCL_OK) {
			return code;
		    }
		}
		varTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
	    }

	    TclEmitOpcode(INST_STR_INDEX, envPtr);
	    return TCL_OK;
	}
	case STR_LENGTH: {
	    if (parsePtr->numWords != 3) {
		/* Fail at run time, not in compilation */
		return TCL_OUT_LINE_COMPILE;
	    }

	    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
		/*
		 * Here someone is asking for the length of a static string.
		 * Just push the actual character (not byte) length.
		 */
		char buf[TCL_INTEGER_SPACE];
		int len = Tcl_NumUtfChars(varTokenPtr[1].start,
			varTokenPtr[1].size);
		len = sprintf(buf, "%d", len);
		TclEmitPush(TclRegisterNewLiteral(envPtr, buf, len), envPtr);
		return TCL_OK;
	    } else {
		SetLineInformation (2);
		code = TclCompileTokens(interp, varTokenPtr+1,
			varTokenPtr->numComponents, envPtr);
		if (code != TCL_OK) {
		    return code;
		}
	    }
	    TclEmitOpcode(INST_STR_LEN, envPtr);
	    return TCL_OK;
	}
	case STR_MATCH: {
	    int i, length, exactMatch = 0, nocase = 0;
	    CONST char *str;

	    if (parsePtr->numWords < 4 || parsePtr->numWords > 5) {
		/* Fail at run time, not in compilation */
		return TCL_OUT_LINE_COMPILE;
	    }

	    if (parsePtr->numWords == 5) {
		if (varTokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
		    return TCL_OUT_LINE_COMPILE;
		}
		str    = varTokenPtr[1].start;
		length = varTokenPtr[1].size;
		if ((length > 1) &&
			strncmp(str, "-nocase", (size_t) length) == 0) {
		    nocase = 1;
		} else {
		    /* Fail at run time, not in compilation */
		    return TCL_OUT_LINE_COMPILE;
		}
		varTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
	    }

	    for (i = 0; i < 2; i++) {
		if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
		    str = varTokenPtr[1].start;
		    length = varTokenPtr[1].size;
		    if (!nocase && (i == 0)) {
			/*
			 * On the first (pattern) arg, check to see if any
			 * glob special characters are in the word '*[]?\\'.
			 * If not, this is the same as 'string equal'.  We
			 * can use strpbrk here because the glob chars are all
			 * in the ascii-7 range.  If -nocase was specified,
			 * we can't do this because INST_STR_EQ has no support
			 * for nocase.
			 */
			Tcl_Obj *copy = Tcl_NewStringObj(str, length);
			Tcl_IncrRefCount(copy);
			exactMatch = (strpbrk(Tcl_GetString(copy),
				"*[]?\\") == NULL);
			Tcl_DecrRefCount(copy);
		    }
		    TclEmitPush(
			    TclRegisterNewLiteral(envPtr, str, length), envPtr);
		} else {
		    SetLineInformation (i);
		    code = TclCompileTokens(interp, varTokenPtr+1,
			    varTokenPtr->numComponents, envPtr);
		    if (code != TCL_OK) {
			return code;
		    }
		}
		varTokenPtr = varTokenPtr + (varTokenPtr->numComponents + 1);
	    }

	    if (exactMatch) {
		TclEmitOpcode(INST_STR_EQ, envPtr);
	    } else {
		TclEmitInstInt1(INST_STR_MATCH, nocase, envPtr);
	    }
	    return TCL_OK;
	}
    }

    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompCmds.c`, function `TclPushVarName`, lines 3225–3464. Retained reference `e14`.

```text
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

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompile.c`, function `TclCompileTokens`, lines 1519–1809. Retained reference `e15`.

```text
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

tcl8.4 8.4.20, revision `pinned Tcl release 8.4.20`, `generic/tclCompile.c`, function `TclFindCompiledLocal`, lines 2237–2307. Retained reference `e16`.

```text
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

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileAppendCmd`, lines 236–316. Retained reference `e17`.

```text
TclCompileAppendCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int simpleVarName, isScalar, localIndex, numWords;
    DefineLineInformation;	/* TIP #280 */

    numWords = parsePtr->numWords;
    if (numWords == 1) {
	return TCL_ERROR;
    } else if (numWords == 2) {
	/*
	 * append varName == set varName
	 */

	return TclCompileSetCmd(interp, parsePtr, cmdPtr, envPtr);
    } else if (numWords > 3) {
	/*
	 * APPEND instructions currently only handle one value.
	 */

	return TCL_ERROR;
    }

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
     * We are doing an assignment, otherwise TclCompileSetCmd was called, so
     * push the new value. This will need to be extended to push a value for
     * each argument.
     */

    if (numWords > 2) {
	valueTokenPtr = TokenAfter(varTokenPtr);
	CompileWord(envPtr, valueTokenPtr, interp, 2);
    }

    /*
     * Emit instructions to set/get the variable.
     */

    if (simpleVarName) {
	if (isScalar) {
	    if (localIndex < 0) {
		TclEmitOpcode(INST_APPEND_STK, envPtr);
	    } else if (localIndex <= 255) {
		TclEmitInstInt1(INST_APPEND_SCALAR1, localIndex, envPtr);
	    } else {
		TclEmitInstInt4(INST_APPEND_SCALAR4, localIndex, envPtr);
	    }
	} else {
	    if (localIndex < 0) {
		TclEmitOpcode(INST_APPEND_ARRAY_STK, envPtr);
	    } else if (localIndex <= 255) {
		TclEmitInstInt1(INST_APPEND_ARRAY1, localIndex, envPtr);
	    } else {
		TclEmitInstInt4(INST_APPEND_ARRAY4, localIndex, envPtr);
	    }
	}
    } else {
	TclEmitOpcode(INST_APPEND_STK, envPtr);
    }

    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileLappendCmd`, lines 2369–2456. Retained reference `e18`.

```text
TclCompileLappendCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr;
    int simpleVarName, isScalar, localIndex, numWords;
    DefineLineInformation;	/* TIP #280 */

    /*
     * If we're not in a procedure, don't compile.
     */

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    numWords = parsePtr->numWords;
    if (numWords == 1) {
	return TCL_ERROR;
    }
    if (numWords != 3) {
	/*
	 * LAPPEND instructions currently only handle one value appends.
	 */

	return TCL_ERROR;
    }

    /*
     * Decide if we can use a frame slot for the var/array name or if we
     * need to emit code to compute and push the name at runtime. We use a
     * frame slot (entry in the array of local vars) if we are compiling a
     * procedure body and if the name is simple text that does not include
     * namespace qualifiers.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    PushVarNameWord(interp, varTokenPtr, envPtr, TCL_CREATE_VAR,
		    &localIndex, &simpleVarName, &isScalar, 1);

    /*
     * If we are doing an assignment, push the new value. In the no values
     * case, create an empty object.
     */

    if (numWords > 2) {
	Tcl_Token *valueTokenPtr = TokenAfter(varTokenPtr);
	CompileWord(envPtr, valueTokenPtr, interp, 2);
    }

    /*
     * Emit instructions to set/get the variable.
     */

    /*
     * The *_STK opcodes should be refactored to make better use of existing
     * LOAD/STORE instructions.
     */

    if (simpleVarName) {
	if (isScalar) {
	    if (localIndex < 0) {
		TclEmitOpcode(INST_LAPPEND_STK, envPtr);
	    } else if (localIndex <= 255) {
		TclEmitInstInt1(INST_LAPPEND_SCALAR1, localIndex, envPtr);
	    } else {
		TclEmitInstInt4(INST_LAPPEND_SCALAR4, localIndex, envPtr);
	    }
	} else {
	    if (localIndex < 0) {
		TclEmitOpcode(INST_LAPPEND_ARRAY_STK, envPtr);
	    } else if (localIndex <= 255) {
		TclEmitInstInt1(INST_LAPPEND_ARRAY1, localIndex, envPtr);
	    } else {
		TclEmitInstInt4(INST_LAPPEND_ARRAY4, localIndex, envPtr);
	    }
	}
    } else {
	TclEmitOpcode(INST_LAPPEND_STK, envPtr);
    }

    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileSetCmd`, lines 3328–3404. Retained reference `e19`.

```text
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

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileIncrCmd`, lines 2251–2348. Retained reference `e20`.

```text
TclCompileIncrCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr, *incrTokenPtr;
    int simpleVarName, isScalar, localIndex, haveImmValue, immValue;
    DefineLineInformation;	/* TIP #280 */

    if ((parsePtr->numWords != 2) && (parsePtr->numWords != 3)) {
	return TCL_ERROR;
    }

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    PushVarNameWord(interp, varTokenPtr, envPtr, TCL_NO_LARGE_INDEX|TCL_CREATE_VAR,
		    &localIndex, &simpleVarName, &isScalar, 1);

    /*
     * If an increment is given, push it, but see first if it's a small
     * integer.
     */

    haveImmValue = 0;
    immValue = 1;
    if (parsePtr->numWords == 3) {
	incrTokenPtr = TokenAfter(varTokenPtr);
	if (incrTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    const char *word = incrTokenPtr[1].start;
	    int numBytes = incrTokenPtr[1].size;
	    int code;
	    Tcl_Obj *intObj = Tcl_NewStringObj(word, numBytes);
	    Tcl_IncrRefCount(intObj);
	    code = TclGetIntFromObj(NULL, intObj, &immValue);
	    TclDecrRefCount(intObj);
	    if ((code == TCL_OK) && (-127 <= immValue) && (immValue <= 127)) {
		haveImmValue = 1;
	    }
	    if (!haveImmValue) {
		PushLiteral(envPtr, word, numBytes);
	    }
	} else {
	    SetLineInformation (2);
	    CompileTokens(envPtr, incrTokenPtr, interp);
	}
    } else {			/* No incr amount given so use 1. */
	haveImmValue = 1;
    }

    /*
     * Emit the instruction to increment the variable.
     */

    if (simpleVarName) {
	if (isScalar) {
	    if (localIndex >= 0) {
		if (haveImmValue) {
		    TclEmitInstInt1(INST_INCR_SCALAR1_IMM, localIndex, envPtr);
		    TclEmitInt1(immValue, envPtr);
		} else {
		    TclEmitInstInt1(INST_INCR_SCALAR1, localIndex, envPtr);
		}
	    } else {
		if (haveImmValue) {
		    TclEmitInstInt1(INST_INCR_SCALAR_STK_IMM, immValue, envPtr);
		} else {
		    TclEmitOpcode(INST_INCR_SCALAR_STK, envPtr);
		}
	    }
	} else {
	    if (localIndex >= 0) {
		if (haveImmValue) {
		    TclEmitInstInt1(INST_INCR_ARRAY1_IMM, localIndex, envPtr);
		    TclEmitInt1(immValue, envPtr);
		} else {
		    TclEmitInstInt1(INST_INCR_ARRAY1, localIndex, envPtr);
		}
	    } else {
		if (haveImmValue) {
		    TclEmitInstInt1(INST_INCR_ARRAY_STK_IMM, immValue, envPtr);
		} else {
		    TclEmitOpcode(INST_INCR_ARRAY_STK, envPtr);
		}
	    }
	}
    } else {			/* Non-simple variable name. */
	if (haveImmValue) {
	    TclEmitInstInt1(INST_INCR_STK_IMM, immValue, envPtr);
	} else {
	    TclEmitOpcode(INST_INCR_STK, envPtr);
	}
    }

    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileUpvarCmd`, lines 5676–5762. Retained reference `e21`.

```text
TclCompileUpvarCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *tokenPtr, *otherTokenPtr, *localTokenPtr;
    int localIndex, numWords, i;
    DefineLineInformation;	/* TIP #280 */
    Tcl_Obj *objPtr;

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    numWords = parsePtr->numWords;
    if (numWords < 3) {
	return TCL_ERROR;
    }

    /*
     * Push the frame index if it is known at compile time
     */

    objPtr = Tcl_NewObj();
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    if(TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	CallFrame *framePtr;
	Tcl_ObjType *newTypePtr, *typePtr = objPtr->typePtr;

	/*
	 * Attempt to convert to a level reference. Note that TclObjGetFrame
	 * only changes the obj type when a conversion was successful.
	 */

	TclObjGetFrame(interp, objPtr, &framePtr);
	newTypePtr = objPtr->typePtr;
	Tcl_DecrRefCount(objPtr);

	if (newTypePtr != typePtr) {
	    if(numWords%2) {
		return TCL_ERROR;
	    }
	    /* TODO: Push the known value instead? */
	    CompileWord(envPtr, tokenPtr, interp, 1);
	    otherTokenPtr = TokenAfter(tokenPtr);
	    i = 2;
	} else {
	    if(!(numWords%2)) {
		return TCL_ERROR;
	    }
	    PushLiteral(envPtr, "1", 1);
	    otherTokenPtr = tokenPtr;
	    i = 1;
	}
    } else {
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    /*
     * Loop over the (otherVar, thisVar) pairs. If any of the thisVar is not a
     * local variable, return an error so that the non-compiled command will
     * be called at runtime.
     */

    for(; i<numWords; i+=2, otherTokenPtr = TokenAfter(localTokenPtr)) {
	localTokenPtr = TokenAfter(otherTokenPtr);

	CompileWord(envPtr, otherTokenPtr, interp, i);
	localIndex = LocalScalarFromToken(localTokenPtr, envPtr);
	if (localIndex < 0) {
	    return TCL_ERROR;
	}
	TclEmitInstInt4(INST_UPVAR, localIndex, envPtr);
    }

    /*
     * Pop the frame index, and set the result to empty
     */

    TclEmitOpcode(INST_POP, envPtr);
    PushLiteral(envPtr, "", 0);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileGlobalCmd`, lines 5873–5930. Retained reference `e22`.

```text
TclCompileGlobalCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr;
    int localIndex, numWords, i;
    DefineLineInformation;	/* TIP #280 */

    numWords = parsePtr->numWords;
    if (numWords < 2) {
	return TCL_ERROR;
    }

    /*
     * 'global' has no effect outside of proc bodies; handle that at runtime
     */

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Push the namespace
     */

    PushLiteral(envPtr, "::", 2);

    /*
     * Loop over the variables.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    for(i=1; i<numWords; varTokenPtr = TokenAfter(varTokenPtr),i++) {
	localIndex = IndexTailVarIfKnown(interp, varTokenPtr, envPtr);

	if(localIndex < 0) {
	    return TCL_ERROR;
	}

	/* TODO: Consider what values can pass through the
	 * IndexTailVarIfKnown() screen.  Full CompileWord()
	 * likely does not apply here.  Push known value instead. */
	CompileWord(envPtr, varTokenPtr, interp, i);
	TclEmitInstInt4(INST_NSUPVAR, localIndex, envPtr);
    }

    /*
     * Pop the namespace, and set the result to empty
     */

    TclEmitOpcode(INST_POP, envPtr);
    PushLiteral(envPtr, "", 0);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileVariableCmd`, lines 5951–6014. Retained reference `e23`.

```text
TclCompileVariableCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int localIndex, numWords, i;
    DefineLineInformation;	/* TIP #280 */

    numWords = parsePtr->numWords;
    if (numWords < 2) {
	return TCL_ERROR;
    }

    /*
     * Bail out if not compiling a proc body
     */

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Loop over the (var, value) pairs.
     */

    valueTokenPtr = parsePtr->tokenPtr;
    for(i=1; i<numWords; i+=2) {
	varTokenPtr = TokenAfter(valueTokenPtr);
	valueTokenPtr = TokenAfter(varTokenPtr);

	localIndex = IndexTailVarIfKnown(interp, varTokenPtr, envPtr);

	if(localIndex < 0) {
	    return TCL_ERROR;
	}

	/* TODO: Consider what values can pass through the
	 * IndexTailVarIfKnown() screen.  Full CompileWord()
	 * likely does not apply here.  Push known value instead. */
	CompileWord(envPtr, varTokenPtr, interp, i);
	TclEmitInstInt4(INST_VARIABLE, localIndex, envPtr);

	if (i != numWords-1) {
	    /*
	     * A value has been given: set the variable, pop the value
	     */

	    CompileWord(envPtr, valueTokenPtr, interp, i+1);
	    TclEmitInstInt4(INST_STORE_SCALAR4, localIndex, envPtr);
	    TclEmitOpcode(INST_POP, envPtr);
	}
    }

    /*
     * Set the result to empty
     */

    PushLiteral(envPtr, "", 0);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileLassignCmd`, lines 2477–2571. Retained reference `e24`.

```text
TclCompileLassignCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *tokenPtr;
    int simpleVarName, isScalar, localIndex, numWords, idx;
    DefineLineInformation;	/* TIP #280 */

    numWords = parsePtr->numWords;

    /*
     * Check for command syntax error, but we'll punt that to runtime.
     */

    if (numWords < 3) {
	return TCL_ERROR;
    }

    /*
     * Generate code to push list being taken apart by [lassign].
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);

    /*
     * Generate code to assign values from the list to variables.
     */

    for (idx=0 ; idx<numWords-2 ; idx++) {
	tokenPtr = TokenAfter(tokenPtr);

	/*
	 * Generate the next variable name.
	 */

	PushVarNameWord(interp, tokenPtr, envPtr, TCL_CREATE_VAR, &localIndex,
			&simpleVarName, &isScalar, idx+2);

	/*
	 * Emit instructions to get the idx'th item out of the list value on
	 * the stack and assign it to the variable.
	 */

	if (simpleVarName) {
	    if (isScalar) {
		if (localIndex >= 0) {
		    TclEmitOpcode(INST_DUP, envPtr);
		    TclEmitInstInt4(INST_LIST_INDEX_IMM, idx, envPtr);
		    if (localIndex <= 255) {
			TclEmitInstInt1(INST_STORE_SCALAR1,localIndex,envPtr);
		    } else {
			TclEmitInstInt4(INST_STORE_SCALAR4,localIndex,envPtr);
		    }
		} else {
		    TclEmitInstInt4(INST_OVER, 1, envPtr);
		    TclEmitInstInt4(INST_LIST_INDEX_IMM, idx, envPtr);
		    TclEmitOpcode(INST_STORE_SCALAR_STK, envPtr);
		}
	    } else {
		if (localIndex >= 0) {
		    TclEmitInstInt4(INST_OVER, 1, envPtr);
		    TclEmitInstInt4(INST_LIST_INDEX_IMM, idx, envPtr);
		    if (localIndex <= 255) {
			TclEmitInstInt1(INST_STORE_ARRAY1, localIndex, envPtr);
		    } else {
			TclEmitInstInt4(INST_STORE_ARRAY4, localIndex, envPtr);
		    }
		} else {
		    TclEmitInstInt4(INST_OVER, 2, envPtr);
		    TclEmitInstInt4(INST_LIST_INDEX_IMM, idx, envPtr);
		    TclEmitOpcode(INST_STORE_ARRAY_STK, envPtr);
		}
	    }
	} else {
	    TclEmitInstInt4(INST_OVER, 1, envPtr);
	    TclEmitInstInt4(INST_LIST_INDEX_IMM, idx, envPtr);
	    TclEmitOpcode(INST_STORE_STK, envPtr);
	}
	TclEmitOpcode(INST_POP, envPtr);
    }

    /*
     * Generate code to leave the rest of the list on the stack.
     */

    TclEmitInstInt4(INST_LIST_RANGE_IMM, idx, envPtr);
    TclEmitInt4(-2, envPtr);	/* -2 == "end" */

    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileListCmd`, lines 2689–2732. Retained reference `e25`.

```text
TclCompileListCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */

    /*
     * If we're not in a procedure, don't compile.
     */

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    if (parsePtr->numWords == 1) {
	/*
	 * [list] without arguments just pushes an empty object.
	 */

	PushLiteral(envPtr, "", 0);
    } else {
	/*
	 * Push the all values onto the stack.
	 */

	Tcl_Token *valueTokenPtr;
	int i, numWords;

	numWords = parsePtr->numWords;

	valueTokenPtr = TokenAfter(parsePtr->tokenPtr);
	for (i = 1; i < numWords; i++) {
	    CompileWord(envPtr, valueTokenPtr, interp, i);
	    valueTokenPtr = TokenAfter(valueTokenPtr);
	}
	TclEmitInstInt4(INST_LIST, numWords - 1, envPtr);
    }

    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileLindexCmd`, lines 2592–2668. Retained reference `e26`.

```text
TclCompileLindexCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *idxTokenPtr, *valTokenPtr;
    int i, numWords = parsePtr->numWords;
    DefineLineInformation;	/* TIP #280 */

    /*
     * Quit if too few args.
     */

    if (numWords <= 1) {
	return TCL_ERROR;
    }

    valTokenPtr = TokenAfter(parsePtr->tokenPtr);
    if (numWords != 3) {
	goto emitComplexLindex;
    }

    idxTokenPtr = TokenAfter(valTokenPtr);
    if (idxTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	Tcl_Obj *tmpObj;
	int idx, result;

	tmpObj = Tcl_NewStringObj(idxTokenPtr[1].start, idxTokenPtr[1].size);
	result = TclGetIntFromObj(NULL, tmpObj, &idx);
	TclDecrRefCount(tmpObj);

	if (result == TCL_OK && idx >= 0) {
	    /*
	     * All checks have been completed, and we have exactly this
	     * construct:
	     *	 lindex <arbitraryValue> <posInt>
	     * This is best compiled as a push of the arbitrary value followed
	     * by an "immediate lindex" which is the most efficient variety.
	     */

	    CompileWord(envPtr, valTokenPtr, interp, 1);
	    TclEmitInstInt4(INST_LIST_INDEX_IMM, idx, envPtr);
	    return TCL_OK;
	}

	/*
	 * If the conversion failed or the value was negative, we just keep on
	 * going with the more complex compilation.
	 */
    }

    /*
     * Push the operands onto the stack.
     */

  emitComplexLindex:
    for (i=1 ; i<numWords ; i++) {
	CompileWord(envPtr, valTokenPtr, interp, i);
	valTokenPtr = TokenAfter(valTokenPtr);
    }

    /*
     * Emit INST_LIST_INDEX if objc==3, or INST_LIST_INDEX_MULTI if there are
     * multiple index args.
     */

    if (numWords == 3) {
	TclEmitOpcode(INST_LIST_INDEX, envPtr);
    } else {
 	TclEmitInstInt4(INST_LIST_INDEX_MULTI, numWords-1, envPtr);
    }

    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileLlengthCmd`, lines 2753–2772. Retained reference `e27`.

```text
TclCompileLlengthCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr;
    DefineLineInformation;	/* TIP #280 */

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    CompileWord(envPtr, varTokenPtr, interp, 1);
    TclEmitOpcode(INST_LIST_LENGTH, envPtr);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileLsetCmd`, lines 2815–2951. Retained reference `e28`.

```text
TclCompileLsetCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    int tempDepth;		/* Depth used for emitting one part of the
				 * code burst. */
    Tcl_Token *varTokenPtr;	/* Pointer to the Tcl_Token representing the
				 * parse of the variable name. */
    int localIndex;		/* Index of var in local var table. */
    int simpleVarName;		/* Flag == 1 if var name is simple. */
    int isScalar;		/* Flag == 1 if scalar, 0 if array. */
    int i;
    DefineLineInformation;	/* TIP #280 */

    /*
     * Check argument count.
     */

    if (parsePtr->numWords < 3) {
	/*
	 * Fail at run time, not in compilation.
	 */

	return TCL_ERROR;
    }

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
     * Push the "index" args and the new element value.
     */

    for (i=2 ; i<parsePtr->numWords ; ++i) {
	varTokenPtr = TokenAfter(varTokenPtr);
	CompileWord(envPtr, varTokenPtr, interp, i);
    }

    /*
     * Duplicate the variable name if it's been pushed.
     */

    if (!simpleVarName || localIndex < 0) {
	if (!simpleVarName || isScalar) {
	    tempDepth = parsePtr->numWords - 2;
	} else {
	    tempDepth = parsePtr->numWords - 1;
	}
	TclEmitInstInt4(INST_OVER, tempDepth, envPtr);
    }

    /*
     * Duplicate an array index if one's been pushed.
     */

    if (simpleVarName && !isScalar) {
	if (localIndex < 0) {
	    tempDepth = parsePtr->numWords - 1;
	} else {
	    tempDepth = parsePtr->numWords - 2;
	}
	TclEmitInstInt4(INST_OVER, tempDepth, envPtr);
    }

    /*
     * Emit code to load the variable's value.
     */

    if (!simpleVarName) {
	TclEmitOpcode(INST_LOAD_STK, envPtr);
    } else if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(INST_LOAD_SCALAR_STK, envPtr);
	} else if (localIndex < 0x100) {
	    TclEmitInstInt1(INST_LOAD_SCALAR1, localIndex, envPtr);
	} else {
	    TclEmitInstInt4(INST_LOAD_SCALAR4, localIndex, envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(INST_LOAD_ARRAY_STK, envPtr);
	} else if (localIndex < 0x100) {
	    TclEmitInstInt1(INST_LOAD_ARRAY1, localIndex, envPtr);
	} else {
	    TclEmitInstInt4(INST_LOAD_ARRAY4, localIndex, envPtr);
	}
    }

    /*
     * Emit the correct variety of 'lset' instruction.
     */

    if (parsePtr->numWords == 4) {
	TclEmitOpcode(INST_LSET_LIST, envPtr);
    } else {
	TclEmitInstInt4(INST_LSET_FLAT, parsePtr->numWords-1, envPtr);
    }

    /*
     * Emit code to put the value back in the variable.
     */

    if (!simpleVarName) {
	TclEmitOpcode(INST_STORE_STK, envPtr);
    } else if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(INST_STORE_SCALAR_STK, envPtr);
	} else if (localIndex < 0x100) {
	    TclEmitInstInt1(INST_STORE_SCALAR1, localIndex, envPtr);
	} else {
	    TclEmitInstInt4(INST_STORE_SCALAR4, localIndex, envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(INST_STORE_ARRAY_STK, envPtr);
	} else if (localIndex < 0x100) {
	    TclEmitInstInt1(INST_STORE_ARRAY1, localIndex, envPtr);
	} else {
	    TclEmitInstInt4(INST_STORE_ARRAY4, localIndex, envPtr);
	}
    }

    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileReturnCmd`, lines 3134–3280. Retained reference `e29`.

```text
TclCompileReturnCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * General syntax: [return ?-option value ...? ?result?]
     * An even number of words means an explicit result argument is present.
     */
    int level, code, objc, size, status = TCL_OK;
    int numWords = parsePtr->numWords;
    int explicitResult = (0 == (numWords % 2));
    int numOptionWords = numWords - 1 - explicitResult;
    Tcl_Obj *returnOpts, **objv;
    Tcl_Token *wordTokenPtr = TokenAfter(parsePtr->tokenPtr);
    DefineLineInformation;	/* TIP #280 */

    /*
     * Check for special case which can always be compiled:
     *	    return -options <opts> <msg>
     * Unlike the normal [return] compilation, this version does everything at
     * runtime so it can handle arbitrary words and not just literals. Note
     * that if INST_RETURN_STK wasn't already needed for something else
     * ('finally' clause processing) this piece of code would not be present.
     */

    if ((numWords == 4) && (wordTokenPtr->type == TCL_TOKEN_SIMPLE_WORD)
	    && (wordTokenPtr[1].size == 8)
	    && (strncmp(wordTokenPtr[1].start, "-options", 8) == 0)) {
	Tcl_Token *optsTokenPtr = TokenAfter(wordTokenPtr);
	Tcl_Token *msgTokenPtr = TokenAfter(optsTokenPtr);

	CompileWord(envPtr, optsTokenPtr, interp, 2);
	CompileWord(envPtr, msgTokenPtr,  interp, 3);
	TclEmitOpcode(INST_RETURN_STK, envPtr);
	return TCL_OK;
    }

    /*
     * Allocate some working space.
     */

    objv = (Tcl_Obj **) TclStackAlloc(interp,
	    numOptionWords * sizeof(Tcl_Obj *));

    /*
     * Scan through the return options. If any are unknown at compile time,
     * there is no value in bytecompiling. Save the option values known in an
     * objv array for merging into a return options dictionary.
     */

    for (objc = 0; objc < numOptionWords; objc++) {
	objv[objc] = Tcl_NewObj();
	Tcl_IncrRefCount(objv[objc]);
	if (!TclWordKnownAtCompileTime(wordTokenPtr, objv[objc])) {
	    objc++;
	    status = TCL_ERROR;
	    goto cleanup;
	}
	wordTokenPtr = TokenAfter(wordTokenPtr);
    }
    status = TclMergeReturnOptions(interp, objc, objv,
	    &returnOpts, &code, &level);
  cleanup:
    while (--objc >= 0) {
	TclDecrRefCount(objv[objc]);
    }
    TclStackFree(interp, objv);
    if (TCL_ERROR == status) {
	/*
	 * Something was bogus in the return options. Clear the error message,
	 * and report back to the compiler that this must be interpreted at
	 * runtime.
	 */

	Tcl_ResetResult(interp);
	return TCL_ERROR;
    }

    /*
     * All options are known at compile time, so we're going to bytecompile.
     * Emit instructions to push the result on the stack.
     */

    if (explicitResult) {
	 CompileWord(envPtr, wordTokenPtr, interp, numWords-1);
    } else {
	/*
	 * No explict result argument, so default result is empty string.
	 */

	PushLiteral(envPtr, "", 0);
    }

    /*
     * Check for optimization: When [return] is in a proc, and there's no
     * enclosing [catch], and there are no return options, then the INST_DONE
     * instruction is equivalent, and may be more efficient.
     */

    if (numOptionWords == 0 && envPtr->procPtr != NULL) {
	/*
	 * We have default return options and we're in a proc ...
	 */

	int index = envPtr->exceptArrayNext - 1;
	int enclosingCatch = 0;

	while (index >= 0) {
	    ExceptionRange range = envPtr->exceptArrayPtr[index];
	    if ((range.type == CATCH_EXCEPTION_RANGE)
		    && (range.catchOffset == -1)) {
		enclosingCatch = 1;
		break;
	    }
	    index--;
	}
	if (!enclosingCatch) {
	    /*
	     * ... and there is no enclosing catch. Issue the maximally
	     * efficient exit instruction.
	     */

	    Tcl_DecrRefCount(returnOpts);
	    TclEmitOpcode(INST_DONE, envPtr);
	    return TCL_OK;
	}
    }

    /* Optimize [return -level 0 $x]. */
    Tcl_DictObjSize(NULL, returnOpts, &size);
    if (size == 0 && level == 0 && code == TCL_OK) {
	Tcl_DecrRefCount(returnOpts);
	return TCL_OK;
    }

    /*
     * Could not use the optimization, so we push the return options dict, and
     * emit the INST_RETURN_IMM instruction with code and level as operands.
     */

    CompileReturnInternal(envPtr, INST_RETURN_IMM, code, level, returnOpts);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileBreakCmd`, lines 337–355. Retained reference `e30`.

```text
TclCompileBreakCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * Emit a break instruction.
     */

    TclEmitOpcode(INST_BREAK, envPtr);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileContinueCmd`, lines 588–610. Retained reference `e31`.

```text
TclCompileContinueCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * There should be no argument after the "continue".
     */

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * Emit a continue instruction.
     */

    TclEmitOpcode(INST_CONTINUE, envPtr);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileInfoExistsCmd`, lines 6327–6378. Retained reference `e32`.

```text
TclCompileInfoExistsCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *tokenPtr;
    int isScalar, simpleVarName, localIndex;
    DefineLineInformation;	/* TIP #280 */

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PushVarNameWord(interp, tokenPtr, envPtr, TCL_CREATE_VAR, &localIndex,
		    &simpleVarName, &isScalar, 1);

    /*
     * Emit instruction to check the variable for existence.
     */

    if (simpleVarName) {
	if (isScalar) {
	    if (localIndex < 0) {
		TclEmitOpcode(INST_EXIST_STK, envPtr);
	    } else {
		TclEmitInstInt4(INST_EXIST_SCALAR, localIndex, envPtr);
	    }
	} else {
	    if (localIndex < 0) {
		TclEmitOpcode(INST_EXIST_ARRAY_STK, envPtr);
	    } else {
		TclEmitInstInt4(INST_EXIST_ARRAY, localIndex, envPtr);
	    }
	}
    } else {
	TclEmitOpcode(INST_EXIST_STK, envPtr);
    }

    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileNamespaceCmd`, lines 5784–5852. Retained reference `e33`.

```text
TclCompileNamespaceCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *tokenPtr, *otherTokenPtr, *localTokenPtr;
    int localIndex, numWords, i;
    DefineLineInformation;	/* TIP #280 */

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Only compile [namespace upvar ...]: needs an odd number of args, >=5
     */

    numWords = parsePtr->numWords;
    if (!(numWords%2) || (numWords < 5)) {
	return TCL_ERROR;
    }

    /*
     * Check if the second argument is "upvar"
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    if ((tokenPtr->size != 5)  /* 5 == strlen("upvar") */
	    || strncmp(tokenPtr->start, "upvar", 5)) {
	return TCL_ERROR;
    }

    /*
     * Push the namespace
     */

    tokenPtr = TokenAfter(tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 2);

    /*
     * Loop over the (otherVar, thisVar) pairs. If any of the thisVar is not a
     * local variable, return an error so that the non-compiled command will
     * be called at runtime.
     */

    localTokenPtr = tokenPtr;
    for(i=3; i<numWords; i+=2) {
	otherTokenPtr = TokenAfter(localTokenPtr);
	localTokenPtr = TokenAfter(otherTokenPtr);

	CompileWord(envPtr, otherTokenPtr, interp, i);
	localIndex = LocalScalarFromToken(localTokenPtr, envPtr);
	if (localIndex < 0) {
	    return TCL_ERROR;
	}
	TclEmitInstInt4(INST_NSUPVAR, localIndex, envPtr);
    }

    /*
     * Pop the namespace, and set the result to empty
     */

    TclEmitOpcode(INST_POP, envPtr);
    PushLiteral(envPtr, "", 0);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileRegexpCmd`, lines 2972–3113. Retained reference `e34`.

```text
TclCompileRegexpCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    Tcl_Token *varTokenPtr;	/* Pointer to the Tcl_Token representing the
				 * parse of the RE or string. */
    int i, len, nocase, exact, sawLast, simple;
    char *str;
    DefineLineInformation;	/* TIP #280 */

    /*
     * We are only interested in compiling simple regexp cases. Currently
     * supported compile cases are:
     *   regexp ?-nocase? ?--? staticString $var
     *   regexp ?-nocase? ?--? {^staticString$} $var
     */

    if (parsePtr->numWords < 3) {
	return TCL_ERROR;
    }

    simple = 0;
    nocase = 0;
    sawLast = 0;
    varTokenPtr = parsePtr->tokenPtr;

    /*
     * We only look for -nocase and -- as options. Everything else gets pushed
     * to runtime execution. This is different than regexp's runtime option
     * handling, but satisfies our stricter needs.
     */

    for (i = 1; i < parsePtr->numWords - 2; i++) {
	varTokenPtr = TokenAfter(varTokenPtr);
	if (varTokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    /*
	     * Not a simple string, so punt to runtime.
	     */

	    return TCL_ERROR;
	}
	str = (char *) varTokenPtr[1].start;
	len = varTokenPtr[1].size;
	if ((len == 2) && (str[0] == '-') && (str[1] == '-')) {
	    sawLast++;
	    i++;
	    break;
	} else if ((len > 1) && (strncmp(str,"-nocase",(unsigned)len) == 0)) {
	    nocase = 1;
	} else {
	    /*
	     * Not an option we recognize.
	     */

	    return TCL_ERROR;
	}
    }

    if ((parsePtr->numWords - i) != 2) {
	/*
	 * We don't support capturing to variables.
	 */

	return TCL_ERROR;
    }

    /*
     * Get the regexp string. If it is not a simple string or can't be
     * converted to a glob pattern, push the word for the INST_REGEXP.
     * Keep changes here in sync with TclCompileSwitchCmd Switch_Regexp.
     */

    varTokenPtr = TokenAfter(varTokenPtr);

    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	Tcl_DString ds;

	str = (char *) varTokenPtr[1].start;
	len = varTokenPtr[1].size;
	/*
	 * If it has a '-', it could be an incorrectly formed regexp command.
	 */

	if ((*str == '-') && !sawLast) {
	    return TCL_ERROR;
	}

	if (len == 0) {
	    /*
	     * The semantics of regexp are always match on re == "".
	     */

	    PushLiteral(envPtr, "1", 1);
	    return TCL_OK;
	}

	/*
	 * Attempt to convert pattern to glob.  If successful, push the
	 * converted pattern as a literal.
	 */

	if (TclReToGlob(NULL, varTokenPtr[1].start, len, &ds, &exact)
		== TCL_OK) {
	    simple = 1;
	    PushLiteral(envPtr, Tcl_DStringValue(&ds),Tcl_DStringLength(&ds));
	    Tcl_DStringFree(&ds);
	}
    }

    if (!simple) {
	CompileWord(envPtr, varTokenPtr, interp, parsePtr->numWords-2);
    }

    /*
     * Push the string arg.
     */

    varTokenPtr = TokenAfter(varTokenPtr);
    CompileWord(envPtr, varTokenPtr, interp, parsePtr->numWords-1);

    if (simple) {
	if (exact && !nocase) {
	    TclEmitOpcode(INST_STR_EQ, envPtr);
	} else {
	    TclEmitInstInt1(INST_STR_MATCH, nocase, envPtr);
	}
    } else {
	/*
	 * Pass correct RE compile flags.  We use only Int1 (8-bit), but
	 * that handles all the flags we want to pass.
	 * Don't use TCL_REG_NOSUB as we may have backrefs.
	 */
	int cflags = TCL_REG_ADVANCED | (nocase ? TCL_REG_NOCASE : 0);
	TclEmitInstInt1(INST_REGEXP, cflags, envPtr);
    }

    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileStringEqualCmd`, lines 3477–3506. Retained reference `e35`.

```text
TclCompileStringEqualCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    /*
     * We don't support any flags; the bytecode isn't that sophisticated.
     */

    if (parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    /*
     * Push the two operands onto the stack and then the test.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);
    tokenPtr = TokenAfter(tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 2);
    TclEmitOpcode(INST_STR_EQ, envPtr);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileStringLenCmd`, lines 3675–3708. Retained reference `e36`.

```text
TclCompileStringLenCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    if (tokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	/*
	 * Here someone is asking for the length of a static string. Just push
	 * the actual character (not byte) length.
	 */

	char buf[TCL_INTEGER_SPACE];
	int len = Tcl_NumUtfChars(tokenPtr[1].start, tokenPtr[1].size);

	len = sprintf(buf, "%d", len);
	PushLiteral(envPtr, buf, len);
    } else {
	SetLineInformation (1);
	CompileTokens(envPtr, tokenPtr, interp);
	TclEmitOpcode(INST_STR_LEN, envPtr);
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileStringMatchCmd`, lines 3575–3653. Retained reference `e37`.

```text
TclCompileStringMatchCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i, length, exactMatch = 0, nocase = 0;
    const char *str;

    if (parsePtr->numWords < 3 || parsePtr->numWords > 4) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Check if we have a -nocase flag.
     */

    if (parsePtr->numWords == 4) {
	if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    return TCL_ERROR;
	}
	str = tokenPtr[1].start;
	length = tokenPtr[1].size;
	if ((length <= 1) || strncmp(str, "-nocase", (size_t) length)) {
	    /*
	     * Fail at run time, not in compilation.
	     */

	    return TCL_ERROR;
	}
	nocase = 1;
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Push the strings to match against each other.
     */

    for (i = 0; i < 2; i++) {
	if (tokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    str = tokenPtr[1].start;
	    length = tokenPtr[1].size;
	    if (!nocase && (i == 0)) {
		/*
		 * Trivial matches can be done by 'string equal'. If -nocase
		 * was specified, we can't do this because INST_STR_EQ has no
		 * support for nocase.
		 */

		Tcl_Obj *copy = Tcl_NewStringObj(str, length);

		Tcl_IncrRefCount(copy);
		exactMatch = TclMatchIsTrivial(TclGetString(copy));
		TclDecrRefCount(copy);
	    }
	    PushLiteral(envPtr, str, length);
	} else {
	    SetLineInformation (i+1+nocase);
	    CompileTokens(envPtr, tokenPtr, interp);
	}
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Push the matcher.
     */

    if (exactMatch) {
	TclEmitOpcode(INST_STR_EQ, envPtr);
    } else {
	TclEmitInstInt1(INST_STR_MATCH, nocase, envPtr);
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileDictAppendCmd`, lines 1146–1197. Retained reference `e38`.

```text
TclCompileDictAppendCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *tokenPtr;
    int i, dictVarIndex;
    DefineLineInformation;	/* TIP #280 */

    /*
     * There must be at least two argument after the command. Since we
     * implement using INST_CONCAT1, make sure the number of arguments
     * stays within its range.
     */

    if (parsePtr->numWords<4 || parsePtr->numWords>258) {
	return TCL_ERROR;
    }

    /*
     * Get the index of the local variable that we will be working with.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = LocalScalarFromToken(tokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TCL_ERROR;
    }

    /*
     * Produce the string to concatenate onto the dictionary entry.
     */

    tokenPtr = TokenAfter(tokenPtr);
    for (i=2 ; i<parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    if (parsePtr->numWords > 4) {
	TclEmitInstInt1(INST_CONCAT1, parsePtr->numWords-3, envPtr);
    }

    /*
     * Do the concatenation.
     */

    TclEmitInstInt4(INST_DICT_APPEND, dictVarIndex, envPtr);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileDictLappendCmd`, lines 1200–1231. Retained reference `e39`.

```text
TclCompileDictLappendCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr, *keyTokenPtr, *valueTokenPtr;
    int dictVarIndex;
    DefineLineInformation;	/* TIP #280 */

    /*
     * There must be three arguments after the command.
     */

    if (parsePtr->numWords != 4) {
	return TCL_ERROR;
    }

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    keyTokenPtr = TokenAfter(varTokenPtr);
    valueTokenPtr = TokenAfter(keyTokenPtr);
    dictVarIndex = LocalScalarFromToken(varTokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TCL_ERROR;
    }
    CompileWord(envPtr, keyTokenPtr, interp, 2);
    CompileWord(envPtr, valueTokenPtr, interp, 3);
    TclEmitInstInt4( INST_DICT_LAPPEND, dictVarIndex, envPtr);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileDictGetCmd`, lines 764–796. Retained reference `e40`.

```text
TclCompileDictGetCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *tokenPtr;
    int i;
    DefineLineInformation;	/* TIP #280 */

    /*
     * There must be at least two arguments after the command (the single-arg
     * case is legal, but too special and magic for us to deal with here).
     */

    if (parsePtr->numWords < 3) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Only compile this because we need INST_DICT_GET anyway.
     */

    for (i=1 ; i<parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    TclEmitInstInt4(INST_DICT_GET, parsePtr->numWords-2, envPtr);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileDictIncrCmd`, lines 702–761. Retained reference `e41`.

```text
TclCompileDictIncrCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *varTokenPtr, *keyTokenPtr;
    int dictVarIndex, incrAmount;
    DefineLineInformation;	/* TIP #280 */

    /*
     * There must be at least two arguments after the command.
     */

    if (parsePtr->numWords < 3 || parsePtr->numWords > 4) {
	return TCL_ERROR;
    }

    /*
     * The dictionary variable must be a local scalar that is knowable at
     * compile time; anything else exceeds the complexity of the opcode. So
     * discover what the index is.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = LocalScalarFromToken(varTokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TCL_ERROR;
    }

    keyTokenPtr = TokenAfter(varTokenPtr);

    /*
     * Parse the increment amount, if present.
     */

    if (parsePtr->numWords == 4) {
	Tcl_Token *incrTokenPtr = TokenAfter(keyTokenPtr);
	Tcl_Obj *intObj = Tcl_NewObj();
	int fail = (!TclWordKnownAtCompileTime(incrTokenPtr, intObj)
		|| TCL_ERROR == TclGetIntFromObj(NULL, intObj, &incrAmount));
	Tcl_DecrRefCount(intObj);
	if (fail) {
	    return TCL_ERROR;
	}
    } else {
	incrAmount = 1;
    }

    /*
     * Emit the key and the code to actually do the increment.
     */

    CompileWord(envPtr, keyTokenPtr, interp, 2);
    TclEmitInstInt4( INST_DICT_INCR_IMM, incrAmount,	envPtr);
    TclEmitInt4(     dictVarIndex,			envPtr);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `TclCompileDictSetCmd`, lines 650–699. Retained reference `e42`.

```text
TclCompileDictSetCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *tokenPtr, *varTokenPtr;
    int i, dictVarIndex;
    DefineLineInformation;	/* TIP #280 */

    /*
     * There must be at least three arguments after the (sub-)command.
     */

    if (parsePtr->numWords < 4) {
	return TCL_ERROR;
    }

    /*
     * The dictionary variable must be a local scalar that is knowable at
     * compile time; anything else exceeds the complexity of the opcode. So
     * discover what the index is.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = LocalScalarFromToken(varTokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TCL_ERROR;
    }

    /*
     * Remaining words (key path and value to set) can be handled normally.
     */

    tokenPtr = TokenAfter(varTokenPtr);
    for (i=2 ; i< parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Now emit the instruction to do the dict manipulation.
     */

    TclEmitInstInt4( INST_DICT_SET, parsePtr->numWords-3,	envPtr);
    TclEmitInt4(     dictVarIndex,			envPtr);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `CompileUnaryOpCmd`, lines 5005–5021. Retained reference `e43`.

```text
CompileUnaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    Tcl_Token *tokenPtr;
    DefineLineInformation;	/* TIP #280 */

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);
    TclEmitOpcode(instruction, envPtr);
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `CompileAssociativeBinaryOpCmd`, lines 5046–5076. Retained reference `e44`.

```text
CompileAssociativeBinaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    const char *identity,
    int instruction,
    CompileEnv *envPtr)
{
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    DefineLineInformation;	/* TIP #280 */
    int words;

    for (words=1 ; words<parsePtr->numWords ; words++) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, words);
    }
    if (parsePtr->numWords <= 2) {
	PushLiteral(envPtr, identity, -1);
	words++;
    }
    if (words > 3) {
	/*
	 * Reverse order of arguments to get precise agreement with
	 * [expr] in calcuations, including roundoff errors.
	 */
	TclEmitInstInt4(INST_REVERSE, words-1, envPtr);
    }
    while (--words > 1) {
	TclEmitOpcode(instruction, envPtr);
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `CompileStrictlyBinaryOpCmd`, lines 5098–5109. Retained reference `e45`.

```text
CompileStrictlyBinaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    if (parsePtr->numWords != 3) {
	return TCL_ERROR;
    }
    return CompileAssociativeBinaryOpCmd(interp, parsePtr,
	    NULL, instruction, envPtr);
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `CompileComparisonOpCmd`, lines 5130–5202. Retained reference `e46`.

```text
CompileComparisonOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    Tcl_Token *tokenPtr;
    DefineLineInformation;	/* TIP #280 */

    if (parsePtr->numWords < 3) {
	PushLiteral(envPtr, "1", 1);
    } else if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(parsePtr->tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 1);
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 2);
	TclEmitOpcode(instruction, envPtr);
    } else if (envPtr->procPtr == NULL) {
	/*
	 * No local variable space!
	 */

	return TCL_ERROR;
    } else {
	int tmpIndex = TclFindCompiledLocal(NULL, 0, 1, envPtr->procPtr);
	int words;

	tokenPtr = TokenAfter(parsePtr->tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 1);
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 2);
	if (tmpIndex <= 255) {
	    TclEmitInstInt1(INST_STORE_SCALAR1, tmpIndex, envPtr);
	} else {
	    TclEmitInstInt4(INST_STORE_SCALAR4, tmpIndex, envPtr);
	}
	TclEmitOpcode(instruction, envPtr);
	for (words=3 ; words<parsePtr->numWords ;) {
	    if (tmpIndex <= 255) {
		TclEmitInstInt1(INST_LOAD_SCALAR1, tmpIndex, envPtr);
	    } else {
		TclEmitInstInt4(INST_LOAD_SCALAR4, tmpIndex, envPtr);
	    }
	    tokenPtr = TokenAfter(tokenPtr);
	    CompileWord(envPtr, tokenPtr, interp, words);
	    if (++words < parsePtr->numWords) {
		if (tmpIndex <= 255) {
		    TclEmitInstInt1(INST_STORE_SCALAR1, tmpIndex, envPtr);
		} else {
		    TclEmitInstInt4(INST_STORE_SCALAR4, tmpIndex, envPtr);
		}
	    }
	    TclEmitOpcode(instruction, envPtr);
	}
	for (; words>3 ; words--) {
	    TclEmitOpcode(INST_BITAND, envPtr);
	}

	/*
	 * Drop the value from the temp variable; retaining that reference
	 * might be expensive elsewhere.
	 */

	PushLiteral(envPtr, "", 0);
	if (tmpIndex <= 255) {
	    TclEmitInstInt1(INST_STORE_SCALAR1, tmpIndex, envPtr);
	} else {
	    TclEmitInstInt4(INST_STORE_SCALAR4, tmpIndex, envPtr);
	}
	TclEmitOpcode(INST_POP, envPtr);
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompCmds.c`, function `PushVarName`, lines 4774–4984. Retained reference `e47`.

```text
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

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompile.c`, function `TclCompileTokens`, lines 1660–1921. Retained reference `e48`.

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

tcl8.5 8.5.19, revision `pinned Tcl release 8.5.19`, `generic/tclCompile.c`, function `TclFindCompiledLocal`, lines 2286–2354. Retained reference `e49`.

```text
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

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileAppendCmd`, lines 121–226. Retained reference `e50`.

```text
TclCompileAppendCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int isScalar, localIndex, numWords, i;

    /* TODO: Consider support for compiling expanded args. */
    numWords = parsePtr->numWords;
    if (numWords == 1) {
	return TCL_ERROR;
    } else if (numWords == 2) {
	/*
	 * append varName == set varName
	 */

	return TclCompileSetCmd(interp, parsePtr, cmdPtr, envPtr);
    } else if (numWords > 3) {
	/*
	 * APPEND instructions currently only handle one value, but we can
	 * handle some multi-value cases by stringing them together.
	 */

	goto appendMultiple;
    }

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
     * We are doing an assignment, otherwise TclCompileSetCmd was called, so
     * push the new value. This will need to be extended to push a value for
     * each argument.
     */

	valueTokenPtr = TokenAfter(varTokenPtr);
	CompileWord(envPtr, valueTokenPtr, interp, 2);

    /*
     * Emit instructions to set/get the variable.
     */

	if (isScalar) {
	    if (localIndex < 0) {
		TclEmitOpcode(INST_APPEND_STK, envPtr);
	    } else {
		Emit14Inst(INST_APPEND_SCALAR, localIndex, envPtr);
	    }
	} else {
	    if (localIndex < 0) {
		TclEmitOpcode(INST_APPEND_ARRAY_STK, envPtr);
	    } else {
		Emit14Inst(INST_APPEND_ARRAY, localIndex, envPtr);
	    }
	}

    return TCL_OK;

  appendMultiple:
    /*
     * Can only handle the case where we are appending to a local scalar when
     * there are multiple values to append.  Fortunately, this is common.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    localIndex = LocalScalarFromToken(varTokenPtr, envPtr);
    if (localIndex < 0) {
	return TCL_ERROR;
    }

    /*
     * Definitely appending to a local scalar; generate the words and append
     * them.
     */

    valueTokenPtr = TokenAfter(varTokenPtr);
    for (i = 2 ; i < numWords ; i++) {
	CompileWord(envPtr, valueTokenPtr, interp, i);
	valueTokenPtr = TokenAfter(valueTokenPtr);
    }
    TclEmitInstInt4(	  INST_REVERSE, numWords-2,		envPtr);
    for (i = 2 ; i < numWords ;) {
	Emit14Inst(	  INST_APPEND_SCALAR, localIndex,	envPtr);
	if (++i < numWords) {
	    TclEmitOpcode(INST_POP,				envPtr);
	}
    }

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileConcatCmd`, lines 855–921. Retained reference `e51`.

```text
TclCompileConcatCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Obj *objPtr, *listObj;
    Tcl_Token *tokenPtr;
    int i;

    /* TODO: Consider compiling expansion case. */
    if (parsePtr->numWords == 1) {
	/*
	 * [concat] without arguments just pushes an empty object.
	 */

	PushStringLiteral(envPtr, "");
	return TCL_OK;
    }

    /*
     * Test if all arguments are compile-time known. If they are, we can
     * implement with a simple push.
     */

    TclNewObj(listObj);
    for (i = 1, tokenPtr = parsePtr->tokenPtr; i < parsePtr->numWords; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	TclNewObj(objPtr);
	if (!TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	    Tcl_DecrRefCount(objPtr);
	    Tcl_DecrRefCount(listObj);
	    listObj = NULL;
	    break;
	}
	(void) Tcl_ListObjAppendElement(NULL, listObj, objPtr);
    }
    if (listObj != NULL) {
	Tcl_Obj **objs;
	const char *bytes;
	int len;

	TclListObjGetElements(NULL, listObj, &len, &objs);
	objPtr = Tcl_ConcatObj(len, objs);
	Tcl_DecrRefCount(listObj);
	bytes = Tcl_GetStringFromObj(objPtr, &len);
	PushLiteral(envPtr, bytes, len);
	Tcl_DecrRefCount(objPtr);
	return TCL_OK;
    }

    /*
     * General case: runtime concat.
     */

    for (i = 1, tokenPtr = parsePtr->tokenPtr; i < parsePtr->numWords; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i);
    }

    TclEmitInstInt4(	INST_CONCAT_STK, i-1,		envPtr);

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileErrorCmd`, lines 2320–2373. Retained reference `e52`.

```text
TclCompileErrorCmd(
    Tcl_Interp *interp,		/* Used for context. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    /*
     * General syntax: [error message ?errorInfo? ?errorCode?]
     */

    if (parsePtr->numWords < 2 || parsePtr->numWords > 4) {
	return TCL_ERROR;
    }

    /*
     * Handle the message.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);

    /*
     * Construct the options. Note that -code and -level are not here.
     */

    if (parsePtr->numWords == 2) {
	PushStringLiteral(envPtr, "");
    } else {
	PushStringLiteral(envPtr, "-errorinfo");
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 2);
	if (parsePtr->numWords == 3) {
	    TclEmitInstInt4(	INST_LIST, 2,			envPtr);
	} else {
	    PushStringLiteral(envPtr, "-errorcode");
	    tokenPtr = TokenAfter(tokenPtr);
	    CompileWord(envPtr, tokenPtr, interp, 3);
	    TclEmitInstInt4(	INST_LIST, 4,			envPtr);
	}
    }

    /*
     * Issue the error via 'returnImm error 0'.
     */

    TclEmitInstInt4(		INST_RETURN_IMM, TCL_ERROR,	envPtr);
    TclEmitInt4(			0,			envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileBreakCmd`, lines 512–549. Retained reference `e53`.

```text
TclCompileBreakCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    ExceptionRange *rangePtr;
    ExceptionAux *auxPtr;

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * Find the innermost exception range that contains this command.
     */

    rangePtr = TclGetInnermostExceptionRange(envPtr, TCL_BREAK, &auxPtr);
    if (rangePtr && rangePtr->type == LOOP_EXCEPTION_RANGE) {
	/*
	 * Found the target! No need for a nasty INST_BREAK here.
	 */

	TclCleanupStackForBreakContinue(envPtr, auxPtr);
	TclAddLoopBreakFixup(envPtr, auxPtr);
    } else {
	/*
	 * Emit a real break.
	 */

	TclEmitOpcode(INST_BREAK, envPtr);
    }
    TclAdjustStackDepth(1, envPtr);

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileContinueCmd`, lines 942–984. Retained reference `e54`.

```text
TclCompileContinueCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    ExceptionRange *rangePtr;
    ExceptionAux *auxPtr;

    /*
     * There should be no argument after the "continue".
     */

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * See if we can find a valid continueOffset (i.e., not -1) in the
     * innermost containing exception range.
     */

    rangePtr = TclGetInnermostExceptionRange(envPtr, TCL_CONTINUE, &auxPtr);
    if (rangePtr && rangePtr->type == LOOP_EXCEPTION_RANGE) {
	/*
	 * Found the target! No need for a nasty INST_CONTINUE here.
	 */

	TclCleanupStackForBreakContinue(envPtr, auxPtr);
	TclAddLoopContinueFixup(envPtr, auxPtr);
    } else {
	/*
	 * Emit a real continue.
	 */

	TclEmitOpcode(INST_CONTINUE, envPtr);
    }
    TclAdjustStackDepth(1, envPtr);

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileDictAppendCmd`, lines 1870–1922. Retained reference `e55`.

```text
TclCompileDictAppendCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i, dictVarIndex;

    /*
     * There must be at least two argument after the command. And we impose an
     * (arbitrary) safe limit; anyone exceeding it should stop worrying about
     * speed quite so much. ;-)
     */

    /* TODO: Consider support for compiling expanded args. */
    if (parsePtr->numWords<4 || parsePtr->numWords>100) {
	return TCL_ERROR;
    }

    /*
     * Get the index of the local variable that we will be working with.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = LocalScalarFromToken(tokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TclCompileBasicMin2ArgCmd(interp, parsePtr,cmdPtr, envPtr);
    }

    /*
     * Produce the string to concatenate onto the dictionary entry.
     */

    tokenPtr = TokenAfter(tokenPtr);
    for (i=2 ; i<parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    if (parsePtr->numWords > 4) {
	TclEmitInstInt1(INST_STR_CONCAT1, parsePtr->numWords-3, envPtr);
    }

    /*
     * Do the concatenation.
     */

    TclEmitInstInt4(INST_DICT_APPEND, dictVarIndex, envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileDictLappendCmd`, lines 1925–1967. Retained reference `e56`.

```text
TclCompileDictLappendCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *keyTokenPtr, *valueTokenPtr;
    int dictVarIndex;

    /*
     * There must be three arguments after the command.
     */

    /* TODO: Consider support for compiling expanded args. */
    /* Probably not.  Why is INST_DICT_LAPPEND limited to one value? */
    if (parsePtr->numWords != 4) {
	return TCL_ERROR;
    }

    /*
     * Parse the arguments.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    keyTokenPtr = TokenAfter(varTokenPtr);
    valueTokenPtr = TokenAfter(keyTokenPtr);
    dictVarIndex = LocalScalarFromToken(varTokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TclCompileBasic3ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }

    /*
     * Issue the implementation.
     */

    CompileWord(envPtr, keyTokenPtr, interp, 2);
    CompileWord(envPtr, valueTokenPtr, interp, 3);
    TclEmitInstInt4(	INST_DICT_LAPPEND, dictVarIndex,	envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileDictCreateCmd`, lines 1257–1347. Retained reference `e57`.

```text
TclCompileDictCreateCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    int worker;			/* Temp var for building the value in. */
    Tcl_Token *tokenPtr;
    Tcl_Obj *keyObj, *valueObj, *dictObj;
    const char *bytes;
    int i, len;

    if ((parsePtr->numWords & 1) == 0) {
	return TCL_ERROR;
    }

    /*
     * See if we can build the value at compile time...
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(dictObj);
    Tcl_IncrRefCount(dictObj);
    for (i=1 ; i<parsePtr->numWords ; i+=2) {
	TclNewObj(keyObj);
	Tcl_IncrRefCount(keyObj);
	if (!TclWordKnownAtCompileTime(tokenPtr, keyObj)) {
	    Tcl_DecrRefCount(keyObj);
	    Tcl_DecrRefCount(dictObj);
	    goto nonConstant;
	}
	tokenPtr = TokenAfter(tokenPtr);
	TclNewObj(valueObj);
	Tcl_IncrRefCount(valueObj);
	if (!TclWordKnownAtCompileTime(tokenPtr, valueObj)) {
	    Tcl_DecrRefCount(keyObj);
	    Tcl_DecrRefCount(valueObj);
	    Tcl_DecrRefCount(dictObj);
	    goto nonConstant;
	}
	tokenPtr = TokenAfter(tokenPtr);
	Tcl_DictObjPut(NULL, dictObj, keyObj, valueObj);
	Tcl_DecrRefCount(keyObj);
	Tcl_DecrRefCount(valueObj);
    }

    /*
     * We did! Excellent. The "verifyDict" is to do type forcing.
     */

    bytes = Tcl_GetStringFromObj(dictObj, &len);
    PushLiteral(envPtr, bytes, len);
    TclEmitOpcode(		INST_DUP,			envPtr);
    TclEmitOpcode(		INST_DICT_VERIFY,		envPtr);
    Tcl_DecrRefCount(dictObj);
    return TCL_OK;

    /*
     * Otherwise, we've got to issue runtime code to do the building, which we
     * do by [dict set]ting into an unnamed local variable. This requires that
     * we are in a context with an LVT.
     */

  nonConstant:
    worker = AnonymousLocal(envPtr);
    if (worker < 0) {
	return TclCompileBasicMin0ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }

    PushStringLiteral(envPtr,		"");
    Emit14Inst(			INST_STORE_SCALAR, worker,	envPtr);
    TclEmitOpcode(		INST_POP,			envPtr);
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (i=1 ; i<parsePtr->numWords ; i+=2) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i+1);
	tokenPtr = TokenAfter(tokenPtr);
	TclEmitInstInt4(	INST_DICT_SET, 1,		envPtr);
	TclEmitInt4(			worker,			envPtr);
	TclAdjustStackDepth(-1, envPtr);
	TclEmitOpcode(		INST_POP,			envPtr);
    }
    Emit14Inst(			INST_LOAD_SCALAR, worker,	envPtr);
    TclEmitInstInt1(		INST_UNSET_SCALAR, 0,		envPtr);
    TclEmitInt4(			worker,			envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileDictExistsCmd`, lines 1167–1201. Retained reference `e58`.

```text
TclCompileDictExistsCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i;

    /*
     * There must be at least two arguments after the command (the single-arg
     * case is legal, but too special and magic for us to deal with here).
     */

    /* TODO: Consider support for compiling expanded args. */
    if (parsePtr->numWords < 3) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Now we do the code generation.
     */

    for (i=1 ; i<parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    TclEmitInstInt4(INST_DICT_EXISTS, parsePtr->numWords-2, envPtr);
    TclAdjustStackDepth(-1, envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileDictGetCmd`, lines 1130–1164. Retained reference `e59`.

```text
TclCompileDictGetCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i;

    /*
     * There must be at least two arguments after the command (the single-arg
     * case is legal, but too special and magic for us to deal with here).
     */

    /* TODO: Consider support for compiling expanded args. */
    if (parsePtr->numWords < 3) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Only compile this because we need INST_DICT_GET anyway.
     */

    for (i=1 ; i<parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    TclEmitInstInt4(INST_DICT_GET, parsePtr->numWords-2, envPtr);
    TclAdjustStackDepth(-1, envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileDictIncrCmd`, lines 1058–1127. Retained reference `e60`.

```text
TclCompileDictIncrCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *keyTokenPtr;
    int dictVarIndex, incrAmount;

    /*
     * There must be at least two arguments after the command.
     */

    if (parsePtr->numWords < 3 || parsePtr->numWords > 4) {
	return TCL_ERROR;
    }
    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    keyTokenPtr = TokenAfter(varTokenPtr);

    /*
     * Parse the increment amount, if present.
     */

    if (parsePtr->numWords == 4) {
	const char *word;
	int numBytes, code;
	Tcl_Token *incrTokenPtr;
	Tcl_Obj *intObj;

	incrTokenPtr = TokenAfter(keyTokenPtr);
	if (incrTokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    return TclCompileBasic2Or3ArgCmd(interp, parsePtr,cmdPtr, envPtr);
	}
	word = incrTokenPtr[1].start;
	numBytes = incrTokenPtr[1].size;

	intObj = Tcl_NewStringObj(word, numBytes);
	Tcl_IncrRefCount(intObj);
	code = TclGetIntFromObj(NULL, intObj, &incrAmount);
	TclDecrRefCount(intObj);
	if (code != TCL_OK) {
	    return TclCompileBasic2Or3ArgCmd(interp, parsePtr,cmdPtr, envPtr);
	}
    } else {
	incrAmount = 1;
    }

    /*
     * The dictionary variable must be a local scalar that is knowable at
     * compile time; anything else exceeds the complexity of the opcode. So
     * discover what the index is.
     */

    dictVarIndex = LocalScalarFromToken(varTokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TclCompileBasic2Or3ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }

    /*
     * Emit the key and the code to actually do the increment.
     */

    CompileWord(envPtr, keyTokenPtr, interp, 2);
    TclEmitInstInt4( INST_DICT_INCR_IMM, incrAmount,	envPtr);
    TclEmitInt4(     dictVarIndex,			envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileDictMergeCmd`, lines 1350–1461. Retained reference `e61`.

```text
TclCompileDictMergeCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i, workerIndex, infoIndex, outLoop;

    /*
     * Deal with some special edge cases. Note that in the case with one
     * argument, the only thing to do is to verify the dict-ness.
     */

    /* TODO: Consider support for compiling expanded args. (less likely) */
    if (parsePtr->numWords < 2) {
	PushStringLiteral(envPtr, "");
	return TCL_OK;
    } else if (parsePtr->numWords == 2) {
	tokenPtr = TokenAfter(parsePtr->tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 1);
	TclEmitOpcode(		INST_DUP,			envPtr);
	TclEmitOpcode(		INST_DICT_VERIFY,		envPtr);
	return TCL_OK;
    }

    /*
     * There's real merging work to do.
     *
     * Allocate some working space. This means we'll only ever compile this
     * command when there's an LVT present.
     */

    workerIndex = AnonymousLocal(envPtr);
    if (workerIndex < 0) {
	return TclCompileBasicMin2ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }
    infoIndex = AnonymousLocal(envPtr);

    /*
     * Get the first dictionary and verify that it is so.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);
    TclEmitOpcode(		INST_DUP,			envPtr);
    TclEmitOpcode(		INST_DICT_VERIFY,		envPtr);
    Emit14Inst(			INST_STORE_SCALAR, workerIndex,	envPtr);
    TclEmitOpcode(		INST_POP,			envPtr);

    /*
     * For each of the remaining dictionaries...
     */

    outLoop = TclCreateExceptRange(CATCH_EXCEPTION_RANGE, envPtr);
    TclEmitInstInt4(		INST_BEGIN_CATCH4, outLoop,	envPtr);
    ExceptionRangeStarts(envPtr, outLoop);
    for (i=2 ; i<parsePtr->numWords ; i++) {
	/*
	 * Get the dictionary, and merge its pairs into the first dict (using
	 * a small loop).
	 */

	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i);
	TclEmitInstInt4(	INST_DICT_FIRST, infoIndex,	envPtr);
	TclEmitInstInt1(	INST_JUMP_TRUE1, 24,		envPtr);
	TclEmitInstInt4(	INST_REVERSE, 2,		envPtr);
	TclEmitInstInt4(	INST_DICT_SET, 1,		envPtr);
	TclEmitInt4(			workerIndex,		envPtr);
	TclAdjustStackDepth(-1, envPtr);
	TclEmitOpcode(		INST_POP,			envPtr);
	TclEmitInstInt4(	INST_DICT_NEXT, infoIndex,	envPtr);
	TclEmitInstInt1(	INST_JUMP_FALSE1, -20,		envPtr);
	TclEmitOpcode(		INST_POP,			envPtr);
	TclEmitOpcode(		INST_POP,			envPtr);
	TclEmitInstInt1(	INST_UNSET_SCALAR, 0,		envPtr);
	TclEmitInt4(			infoIndex,		envPtr);
    }
    ExceptionRangeEnds(envPtr, outLoop);
    TclEmitOpcode(		INST_END_CATCH,			envPtr);

    /*
     * Clean up any state left over.
     */

    Emit14Inst(			INST_LOAD_SCALAR, workerIndex,	envPtr);
    TclEmitInstInt1(		INST_UNSET_SCALAR, 0,		envPtr);
    TclEmitInt4(			workerIndex,		envPtr);
    TclEmitInstInt1(		INST_JUMP1, 18,			envPtr);

    /*
     * If an exception happens when starting to iterate over the second (and
     * subsequent) dicts. This is strictly not necessary, but it is nice.
     */

    TclAdjustStackDepth(-1, envPtr);
    ExceptionRangeTarget(envPtr, outLoop, catchOffset);
    TclEmitOpcode(		INST_PUSH_RETURN_OPTIONS,	envPtr);
    TclEmitOpcode(		INST_PUSH_RESULT,		envPtr);
    TclEmitOpcode(		INST_END_CATCH,			envPtr);
    TclEmitInstInt1(		INST_UNSET_SCALAR, 0,		envPtr);
    TclEmitInt4(			workerIndex,		envPtr);
    TclEmitInstInt1(		INST_UNSET_SCALAR, 0,		envPtr);
    TclEmitInt4(			infoIndex,		envPtr);
    TclEmitOpcode(		INST_RETURN_STK,		envPtr);

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileDictSetCmd`, lines 1005–1055. Retained reference `e62`.

```text
TclCompileDictSetCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr, *varTokenPtr;
    int i, dictVarIndex;

    /*
     * There must be at least one argument after the command.
     */

    if (parsePtr->numWords < 4) {
	return TCL_ERROR;
    }

    /*
     * The dictionary variable must be a local scalar that is knowable at
     * compile time; anything else exceeds the complexity of the opcode. So
     * discover what the index is.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = LocalScalarFromToken(varTokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TCL_ERROR;
    }

    /*
     * Remaining words (key path and value to set) can be handled normally.
     */

    tokenPtr = TokenAfter(varTokenPtr);
    for (i=2 ; i< parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Now emit the instruction to do the dict manipulation.
     */

    TclEmitInstInt4( INST_DICT_SET, parsePtr->numWords-3,	envPtr);
    TclEmitInt4(     dictVarIndex,			envPtr);
    TclAdjustStackDepth(-1, envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclCompileDictUnsetCmd`, lines 1204–1254. Retained reference `e63`.

```text
TclCompileDictUnsetCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i, dictVarIndex;

    /*
     * There must be at least one argument after the variable name for us to
     * compile to bytecode.
     */

    /* TODO: Consider support for compiling expanded args. */
    if (parsePtr->numWords < 3) {
	return TCL_ERROR;
    }

    /*
     * The dictionary variable must be a local scalar that is knowable at
     * compile time; anything else exceeds the complexity of the opcode. So
     * discover what the index is.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = LocalScalarFromToken(tokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TclCompileBasicMin2ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }

    /*
     * Remaining words (the key path) can be handled normally.
     */

    for (i=2 ; i<parsePtr->numWords ; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i);
    }

    /*
     * Now emit the instruction to do the dict manipulation.
     */

    TclEmitInstInt4( INST_DICT_UNSET, parsePtr->numWords-2,	envPtr);
    TclEmitInt4(	dictVarIndex,				envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmds.c`, function `TclPushVarName`, lines 3387–3591. Retained reference `e64`.

```text
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

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileLappendCmd`, lines 852–947. Retained reference `e65`.

```text
TclCompileLappendCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int isScalar, localIndex, numWords, i;

    /* TODO: Consider support for compiling expanded args. */
    numWords = parsePtr->numWords;
    if (numWords < 3) {
	return TCL_ERROR;
    }

    if (numWords != 3 || envPtr->procPtr == NULL) {
	goto lappendMultiple;
    }

    /*
     * Decide if we can use a frame slot for the var/array name or if we
     * need to emit code to compute and push the name at runtime. We use a
     * frame slot (entry in the array of local vars) if we are compiling a
     * procedure body and if the name is simple text that does not include
     * namespace qualifiers.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    PushVarNameWord(interp, varTokenPtr, envPtr, 0,
	    &localIndex, &isScalar, 1);

    /*
     * If we are doing an assignment, push the new value. In the no values
     * case, create an empty object.
     */

    if (numWords > 2) {
	valueTokenPtr = TokenAfter(varTokenPtr);

	CompileWord(envPtr, valueTokenPtr, interp, 2);
    }

    /*
     * Emit instructions to set/get the variable.
     */

    /*
     * The *_STK opcodes should be refactored to make better use of existing
     * LOAD/STORE instructions.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_LAPPEND_STK,		envPtr);
	} else {
	    Emit14Inst(		INST_LAPPEND_SCALAR, localIndex, envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_LAPPEND_ARRAY_STK,		envPtr);
	} else {
	    Emit14Inst(		INST_LAPPEND_ARRAY, localIndex,	envPtr);
	}
    }

    return TCL_OK;

  lappendMultiple:
    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    PushVarNameWord(interp, varTokenPtr, envPtr, 0,
	    &localIndex, &isScalar, 1);
    valueTokenPtr = TokenAfter(varTokenPtr);
    for (i = 2 ; i < numWords ; i++) {
	CompileWord(envPtr, valueTokenPtr, interp, i);
	valueTokenPtr = TokenAfter(valueTokenPtr);
    }
    TclEmitInstInt4(	    INST_LIST, numWords - 2,		envPtr);
    if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(  INST_LAPPEND_LIST_STK,		envPtr);
	} else {
	    TclEmitInstInt4(INST_LAPPEND_LIST, localIndex,	envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(  INST_LAPPEND_LIST_ARRAY_STK,	envPtr);
	} else {
	    TclEmitInstInt4(INST_LAPPEND_LIST_ARRAY, localIndex,envPtr);
	}
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileIncrCmd`, lines 472–567. Retained reference `e66`.

```text
TclCompileIncrCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *incrTokenPtr;
    int isScalar, localIndex, haveImmValue;
    Tcl_WideInt immValue;

    if ((parsePtr->numWords != 2) && (parsePtr->numWords != 3)) {
	return TCL_ERROR;
    }

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    PushVarNameWord(interp, varTokenPtr, envPtr, TCL_NO_LARGE_INDEX,
	    &localIndex, &isScalar, 1);

    /*
     * If an increment is given, push it, but see first if it's a small
     * integer.
     */

    haveImmValue = 0;
    immValue = 1;
    if (parsePtr->numWords == 3) {
	incrTokenPtr = TokenAfter(varTokenPtr);
	if (incrTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    const char *word = incrTokenPtr[1].start;
	    int numBytes = incrTokenPtr[1].size;
	    int code;
	    Tcl_Obj *intObj = Tcl_NewStringObj(word, numBytes);

	    Tcl_IncrRefCount(intObj);
	    code = Tcl_GetWideIntFromObj(NULL, intObj, &immValue);
	    if ((code == TCL_OK) && (
#ifndef TCL_WIDE_INT_IS_LONG
		    intObj->typePtr == &tclWideIntType ||
#endif
		    intObj->typePtr == &tclIntType) && (-127 <= immValue) && (immValue <= 127)) {
		haveImmValue = 1;
	    }
	    TclDecrRefCount(intObj);
	    if (!haveImmValue) {
		PushLiteral(envPtr, word, numBytes);
	    }
	} else {
	    SetLineInformation(2);
	    CompileTokens(envPtr, incrTokenPtr, interp);
	}
    } else {			/* No incr amount given so use 1. */
	haveImmValue = 1;
    }

    /*
     * Emit the instruction to increment the variable.
     */

    if (isScalar) {	/* Simple scalar variable. */
	if (localIndex >= 0) {
	    if (haveImmValue) {
		TclEmitInstInt1(INST_INCR_SCALAR1_IMM, localIndex, envPtr);
		TclEmitInt1(immValue, envPtr);
	    } else {
		TclEmitInstInt1(INST_INCR_SCALAR1, localIndex,	envPtr);
	    }
	} else {
	    if (haveImmValue) {
		TclEmitInstInt1(INST_INCR_STK_IMM, immValue, envPtr);
	    } else {
		TclEmitOpcode(	INST_INCR_STK,		envPtr);
	    }
	}
    } else {			/* Simple array variable. */
	if (localIndex >= 0) {
	    if (haveImmValue) {
		TclEmitInstInt1(INST_INCR_ARRAY1_IMM, localIndex, envPtr);
		TclEmitInt1(immValue, envPtr);
	    } else {
		TclEmitInstInt1(INST_INCR_ARRAY1, localIndex,	envPtr);
	    }
	} else {
	    if (haveImmValue) {
		TclEmitInstInt1(INST_INCR_ARRAY_STK_IMM, immValue, envPtr);
	    } else {
		TclEmitOpcode(	INST_INCR_ARRAY_STK,		envPtr);
	    }
	}
    }

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileUpvarCmd`, lines 2672–2758. Retained reference `e67`.

```text
TclCompileUpvarCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr, *otherTokenPtr, *localTokenPtr;
    int localIndex, numWords, i;
    Tcl_Obj *objPtr;

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    numWords = parsePtr->numWords;
    if (numWords < 3) {
	return TCL_ERROR;
    }

    /*
     * Push the frame index if it is known at compile time
     */

    TclNewObj(objPtr);
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    if (TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	CallFrame *framePtr;
	const Tcl_ObjType *newTypePtr, *typePtr = objPtr->typePtr;

	/*
	 * Attempt to convert to a level reference. Note that TclObjGetFrame
	 * only changes the obj type when a conversion was successful.
	 */

	TclObjGetFrame(interp, objPtr, &framePtr);
	newTypePtr = objPtr->typePtr;
	Tcl_DecrRefCount(objPtr);

	if (newTypePtr != typePtr) {
	    if (numWords%2) {
		return TCL_ERROR;
	    }
	    /* TODO: Push the known value instead? */
	    CompileWord(envPtr, tokenPtr, interp, 1);
	    otherTokenPtr = TokenAfter(tokenPtr);
	    i = 2;
	} else {
	    if (!(numWords%2)) {
		return TCL_ERROR;
	    }
	    PushStringLiteral(envPtr, "1");
	    otherTokenPtr = tokenPtr;
	    i = 1;
	}
    } else {
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    /*
     * Loop over the (otherVar, thisVar) pairs. If any of the thisVar is not a
     * local variable, return an error so that the non-compiled command will
     * be called at runtime.
     */

    for (; i<numWords; i+=2, otherTokenPtr = TokenAfter(localTokenPtr)) {
	localTokenPtr = TokenAfter(otherTokenPtr);

	CompileWord(envPtr, otherTokenPtr, interp, i);
	localIndex = LocalScalarFromToken(localTokenPtr, envPtr);
	if (localIndex < 0) {
	    return TCL_ERROR;
	}
	TclEmitInstInt4(	INST_UPVAR, localIndex,		envPtr);
    }

    /*
     * Pop the frame index, and set the result to empty
     */

    TclEmitOpcode(		INST_POP,			envPtr);
    PushStringLiteral(envPtr, "");
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileGlobalCmd`, lines 86–147. Retained reference `e68`.

```text
TclCompileGlobalCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;
    int localIndex, numWords, i;

    /* TODO: Consider support for compiling expanded args. */
    numWords = parsePtr->numWords;
    if (numWords < 2) {
	return TCL_ERROR;
    }

    /*
     * 'global' has no effect outside of proc bodies; handle that at runtime
     */

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Push the namespace
     */

    PushStringLiteral(envPtr, "::");

    /*
     * Loop over the variables.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (i=1; i<numWords; varTokenPtr = TokenAfter(varTokenPtr),i++) {
	localIndex = IndexTailVarIfKnown(interp, varTokenPtr, envPtr);

	if (localIndex < 0) {
	    return TCL_ERROR;
	}

	/*
	 * TODO: Consider what value can pass through the
	 * IndexTailVarIfKnown() screen. Full CompileWord() likely does not
	 * apply here. Push known value instead.
	 */

	CompileWord(envPtr, varTokenPtr, interp, i);
	TclEmitInstInt4(	INST_NSUPVAR, localIndex,	envPtr);
    }

    /*
     * Pop the namespace, and set the result to empty
     */

    TclEmitOpcode(		INST_POP,			envPtr);
    PushStringLiteral(envPtr, "");
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileVariableCmd`, lines 2779–2842. Retained reference `e69`.

```text
TclCompileVariableCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int localIndex, numWords, i;

    numWords = parsePtr->numWords;
    if (numWords < 2) {
	return TCL_ERROR;
    }

    /*
     * Bail out if not compiling a proc body
     */

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Loop over the (var, value) pairs.
     */

    valueTokenPtr = parsePtr->tokenPtr;
    for (i=1; i<numWords; i+=2) {
	varTokenPtr = TokenAfter(valueTokenPtr);
	valueTokenPtr = TokenAfter(varTokenPtr);

	localIndex = IndexTailVarIfKnown(interp, varTokenPtr, envPtr);

	if (localIndex < 0) {
	    return TCL_ERROR;
	}

	/* TODO: Consider what value can pass through the
	 * IndexTailVarIfKnown() screen.  Full CompileWord()
	 * likely does not apply here.  Push known value instead. */
	CompileWord(envPtr, varTokenPtr, interp, i);
	TclEmitInstInt4(	INST_VARIABLE, localIndex,	envPtr);

	if (i + 1 < numWords) {
	    /*
	     * A value has been given: set the variable, pop the value
	     */

	    CompileWord(envPtr, valueTokenPtr, interp, i + 1);
	    Emit14Inst(		INST_STORE_SCALAR, localIndex,	envPtr);
	    TclEmitOpcode(	INST_POP,			envPtr);
	}
    }

    /*
     * Set the result to empty
     */

    PushStringLiteral(envPtr, "");
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileLassignCmd`, lines 968–1051. Retained reference `e70`.

```text
TclCompileLassignCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int isScalar, localIndex, numWords, idx;

    numWords = parsePtr->numWords;

    /*
     * Check for command syntax error, but we'll punt that to runtime.
     */

    if (numWords < 3) {
	return TCL_ERROR;
    }

    /*
     * Generate code to push list being taken apart by [lassign].
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);

    /*
     * Generate code to assign values from the list to variables.
     */

    for (idx=0 ; idx<numWords-2 ; idx++) {
	tokenPtr = TokenAfter(tokenPtr);

	/*
	 * Generate the next variable name.
	 */

	PushVarNameWord(interp, tokenPtr, envPtr, 0, &localIndex,
		&isScalar, idx + 2);

	/*
	 * Emit instructions to get the idx'th item out of the list value on
	 * the stack and assign it to the variable.
	 */

	if (isScalar) {
	    if (localIndex >= 0) {
		TclEmitOpcode(	INST_DUP,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		Emit14Inst(	INST_STORE_SCALAR, localIndex,	envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    } else {
		TclEmitInstInt4(INST_OVER, 1,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		TclEmitOpcode(	INST_STORE_STK,			envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    }
	} else {
	    if (localIndex >= 0) {
		TclEmitInstInt4(INST_OVER, 1,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		Emit14Inst(	INST_STORE_ARRAY, localIndex,	envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    } else {
		TclEmitInstInt4(INST_OVER, 2,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		TclEmitOpcode(	INST_STORE_ARRAY_STK,		envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    }
	}
    }

    /*
     * Generate code to leave the rest of the list on the stack.
     */

    TclEmitInstInt4(		INST_LIST_RANGE_IMM, idx,	envPtr);
    TclEmitInt4(			TCL_INDEX_END,		envPtr);

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileListCmd`, lines 1163–1256. Retained reference `e71`.

```text
TclCompileListCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *valueTokenPtr;
    int i, numWords, concat, build;
    Tcl_Obj *listObj, *objPtr;

    if (parsePtr->numWords == 1) {
	/*
	 * [list] without arguments just pushes an empty object.
	 */

	PushStringLiteral(envPtr, "");
	return TCL_OK;
    }

    /*
     * Test if all arguments are compile-time known. If they are, we can
     * implement with a simple push.
     */

    numWords = parsePtr->numWords;
    valueTokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(listObj);
    for (i = 1; i < numWords && listObj != NULL; i++) {
	TclNewObj(objPtr);
	if (TclWordKnownAtCompileTime(valueTokenPtr, objPtr)) {
	    (void) Tcl_ListObjAppendElement(NULL, listObj, objPtr);
	} else {
	    Tcl_DecrRefCount(objPtr);
	    Tcl_DecrRefCount(listObj);
	    listObj = NULL;
	}
	valueTokenPtr = TokenAfter(valueTokenPtr);
    }
    if (listObj != NULL) {
	TclEmitPush(TclAddLiteralObj(envPtr, listObj, NULL), envPtr);
	return TCL_OK;
    }

    /*
     * Push the all values onto the stack.
     */

    numWords = parsePtr->numWords;
    valueTokenPtr = TokenAfter(parsePtr->tokenPtr);
    concat = build = 0;
    for (i = 1; i < numWords; i++) {
	if (valueTokenPtr->type == TCL_TOKEN_EXPAND_WORD && build > 0) {
	    TclEmitInstInt4(	INST_LIST, build,	envPtr);
	    if (concat) {
		TclEmitOpcode(	INST_LIST_CONCAT,	envPtr);
	    }
	    build = 0;
	    concat = 1;
	}
	CompileWord(envPtr, valueTokenPtr, interp, i);
	if (valueTokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	    if (concat) {
		TclEmitOpcode(	INST_LIST_CONCAT,	envPtr);
	    } else {
		concat = 1;
	    }
	} else {
	    build++;
	}
	valueTokenPtr = TokenAfter(valueTokenPtr);
    }
    if (build > 0) {
	TclEmitInstInt4(	INST_LIST, build,	envPtr);
	if (concat) {
	    TclEmitOpcode(	INST_LIST_CONCAT,	envPtr);
	}
    }

    /*
     * If there was just one expanded word, we must ensure that it is a list
     * at this point. We use an [lrange ... 0 end] for this (instead of
     * [llength], as with literals) as we must drop any string representation
     * that might be hanging around.
     */

    if (concat && numWords == 2) {
	TclEmitInstInt4(	INST_LIST_RANGE_IMM, 0,	envPtr);
	TclEmitInt4(			TCL_INDEX_END,	envPtr);
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileLindexCmd`, lines 1072–1142. Retained reference `e72`.

```text
TclCompileLindexCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *idxTokenPtr, *valTokenPtr;
    int i, idx, numWords = parsePtr->numWords;

    /*
     * Quit if not enough args.
     */

    /* TODO: Consider support for compiling expanded args. */
    if (numWords <= 1) {
	return TCL_ERROR;
    }

    valTokenPtr = TokenAfter(parsePtr->tokenPtr);
    if (numWords != 3) {
	goto emitComplexLindex;
    }

    idxTokenPtr = TokenAfter(valTokenPtr);
    if (TclGetIndexFromToken(idxTokenPtr, TCL_INDEX_BEFORE, TCL_INDEX_BEFORE,
	    &idx) == TCL_OK) {
	/*
	 * The idxTokenPtr parsed as a valid index value and was
	 * encoded as expected by INST_LIST_INDEX_IMM.
	 *
	 * NOTE: that we rely on indexing before a list producing the
	 * same result as indexing after a list.
	 */

	CompileWord(envPtr, valTokenPtr, interp, 1);
	TclEmitInstInt4(	INST_LIST_INDEX_IMM, idx,	envPtr);
	return TCL_OK;
    }

    /*
     * If the value was not known at compile time, the conversion failed or
     * the value was negative, we just keep on going with the more complex
     * compilation.
     */

    /*
     * Push the operands onto the stack.
     */

  emitComplexLindex:
    for (i=1 ; i<numWords ; i++) {
	CompileWord(envPtr, valTokenPtr, interp, i);
	valTokenPtr = TokenAfter(valTokenPtr);
    }

    /*
     * Emit INST_LIST_INDEX if objc==3, or INST_LIST_INDEX_MULTI if there are
     * multiple index args.
     */

    if (numWords == 3) {
	TclEmitOpcode(		INST_LIST_INDEX,		envPtr);
    } else {
	TclEmitInstInt4(	INST_LIST_INDEX_MULTI, numWords-1, envPtr);
    }

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileLlengthCmd`, lines 1277–1296. Retained reference `e73`.

```text
TclCompileLlengthCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    CompileWord(envPtr, varTokenPtr, interp, 1);
    TclEmitOpcode(		INST_LIST_LENGTH,		envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileLrangeCmd`, lines 1310–1357. Retained reference `e74`.

```text
TclCompileLrangeCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for context. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr, *listTokenPtr;
    int idx1, idx2;

    if (parsePtr->numWords != 4) {
	return TCL_ERROR;
    }
    listTokenPtr = TokenAfter(parsePtr->tokenPtr);

    tokenPtr = TokenAfter(listTokenPtr);
    if (TclGetIndexFromToken(tokenPtr, TCL_INDEX_START, TCL_INDEX_AFTER,
	    &idx1) != TCL_OK) {
	return TCL_ERROR;
    }
    /*
     * Token was an index value, and we treat all "first" indices
     * before the list same as the start of the list.
     */

    tokenPtr = TokenAfter(tokenPtr);
    if (TclGetIndexFromToken(tokenPtr, TCL_INDEX_BEFORE, TCL_INDEX_END,
	    &idx2) != TCL_OK) {
	return TCL_ERROR;
    }
    /*
     * Token was an index value, and we treat all "last" indices
     * after the list same as the end of the list.
     */

    /*
     * Issue instructions. It's not safe to skip doing the LIST_RANGE, as
     * we've not proved that the 'list' argument is really a list. Not that it
     * is worth trying to do that given current knowledge.
     */

    CompileWord(envPtr, listTokenPtr, interp, 1);
    TclEmitInstInt4(		INST_LIST_RANGE_IMM, idx1,	envPtr);
    TclEmitInt4(		idx2,				envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileLinsertCmd`, lines 1371–1460. Retained reference `e75`.

```text
TclCompileLinsertCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for context. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr, *listTokenPtr;
    int idx, i;

    if (parsePtr->numWords < 3) {
	return TCL_ERROR;
    }
    listTokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Parse the index. Will only compile if it is constant and not an
     * _integer_ less than zero (since we reserve negative indices here for
     * end-relative indexing) or an end-based index greater than 'end' itself.
     */

    tokenPtr = TokenAfter(listTokenPtr);

    /*
     * NOTE: This command treats all inserts at indices before the list
     * the same as inserts at the start of the list, and all inserts
     * after the list the same as inserts at the end of the list. We
     * make that transformation here so we can use the optimized bytecode
     * as much as possible.
     */
    if (TclGetIndexFromToken(tokenPtr, TCL_INDEX_START, TCL_INDEX_END,
	    &idx) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * There are four main cases. If there are no values to insert, this is
     * just a confirm-listiness check. If the index is '0', this is a prepend.
     * If the index is 'end' (== TCL_INDEX_END), this is an append. Otherwise,
     * this is a splice (== split, insert values as list, concat-3).
     */

    CompileWord(envPtr, listTokenPtr, interp, 1);
    if (parsePtr->numWords == 3) {
	TclEmitInstInt4(	INST_LIST_RANGE_IMM, 0,		envPtr);
	TclEmitInt4(			TCL_INDEX_END,		envPtr);
	return TCL_OK;
    }

    for (i=3 ; i<parsePtr->numWords ; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i);
    }
    TclEmitInstInt4(		INST_LIST, i-3,			envPtr);

    if (idx == TCL_INDEX_START) {
	TclEmitInstInt4(	INST_REVERSE, 2,		envPtr);
	TclEmitOpcode(		INST_LIST_CONCAT,		envPtr);
    } else if (idx == TCL_INDEX_END) {
	TclEmitOpcode(		INST_LIST_CONCAT,		envPtr);
    } else {
	/*
	 * Here we handle two ranges for idx. First when idx > 0, we
	 * want the first half of the split to end at index idx-1 and
	 * the second half to start at index idx.
	 * Second when idx < TCL_INDEX_END, indicating "end-N" indexing,
	 * we want the first half of the split to end at index end-N and
	 * the second half to start at index end-N+1. We accomplish this
	 * with a preadjustment of the end-N value.
	 * The root of this is that the commands [lrange] and [linsert]
	 * differ in their interpretation of the "end" index.
	 */

	if (idx < TCL_INDEX_END) {
	    idx++;
	}
	TclEmitInstInt4(	INST_OVER, 1,			envPtr);
	TclEmitInstInt4(	INST_LIST_RANGE_IMM, 0,		envPtr);
	TclEmitInt4(			idx-1,			envPtr);
	TclEmitInstInt4(	INST_REVERSE, 3,		envPtr);
	TclEmitInstInt4(	INST_LIST_RANGE_IMM, idx,	envPtr);
	TclEmitInt4(			TCL_INDEX_END,		envPtr);
	TclEmitOpcode(		INST_LIST_CONCAT,		envPtr);
	TclEmitOpcode(		INST_LIST_CONCAT,		envPtr);
    }

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileLsetCmd`, lines 1638–1762. Retained reference `e76`.

```text
TclCompileLsetCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    int tempDepth;		/* Depth used for emitting one part of the
				 * code burst. */
    Tcl_Token *varTokenPtr;	/* Pointer to the Tcl_Token representing the
				 * parse of the variable name. */
    int localIndex;		/* Index of var in local var table. */
    int isScalar;		/* Flag == 1 if scalar, 0 if array. */
    int i;

    /*
     * Check argument count.
     */

    /* TODO: Consider support for compiling expanded args. */
    if (parsePtr->numWords < 3) {
	/*
	 * Fail at run time, not in compilation.
	 */

	return TCL_ERROR;
    }

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
     * Push the "index" args and the new element value.
     */

    for (i=2 ; i<parsePtr->numWords ; ++i) {
	varTokenPtr = TokenAfter(varTokenPtr);
	CompileWord(envPtr, varTokenPtr, interp, i);
    }

    /*
     * Duplicate the variable name if it's been pushed.
     */

    if (localIndex < 0) {
	if (isScalar) {
	    tempDepth = parsePtr->numWords - 2;
	} else {
	    tempDepth = parsePtr->numWords - 1;
	}
	TclEmitInstInt4(	INST_OVER, tempDepth,		envPtr);
    }

    /*
     * Duplicate an array index if one's been pushed.
     */

    if (!isScalar) {
	if (localIndex < 0) {
	    tempDepth = parsePtr->numWords - 1;
	} else {
	    tempDepth = parsePtr->numWords - 2;
	}
	TclEmitInstInt4(	INST_OVER, tempDepth,		envPtr);
    }

    /*
     * Emit code to load the variable's value.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_LOAD_STK,			envPtr);
	} else {
	    Emit14Inst(		INST_LOAD_SCALAR, localIndex,	envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_LOAD_ARRAY_STK,		envPtr);
	} else {
	    Emit14Inst(		INST_LOAD_ARRAY, localIndex,	envPtr);
	}
    }

    /*
     * Emit the correct variety of 'lset' instruction.
     */

    if (parsePtr->numWords == 4) {
	TclEmitOpcode(		INST_LSET_LIST,			envPtr);
    } else {
	TclEmitInstInt4(	INST_LSET_FLAT, parsePtr->numWords-1, envPtr);
    }

    /*
     * Emit code to put the value back in the variable.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_STORE_STK,			envPtr);
	} else {
	    Emit14Inst(		INST_STORE_SCALAR, localIndex,	envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_STORE_ARRAY_STK,		envPtr);
	} else {
	    Emit14Inst(		INST_STORE_ARRAY, localIndex,	envPtr);
	}
    }

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileReturnCmd`, lines 2419–2605. Retained reference `e77`.

```text
TclCompileReturnCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    /*
     * General syntax: [return ?-option value ...? ?result?]
     * An even number of words means an explicit result argument is present.
     */
    int level, code, status = TCL_OK;
    int size;
    int numWords = parsePtr->numWords;
    int explicitResult = (0 == (numWords % 2));
    int objc, numOptionWords = numWords - 1 - explicitResult;
    Tcl_Obj *returnOpts, **objv;
    Tcl_Token *wordTokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Check for special case which can always be compiled:
     *	    return -options <opts> <msg>
     * Unlike the normal [return] compilation, this version does everything at
     * runtime so it can handle arbitrary words and not just literals. Note
     * that if INST_RETURN_STK wasn't already needed for something else
     * ('finally' clause processing) this piece of code would not be present.
     */

    if ((numWords == 4) && (wordTokenPtr->type == TCL_TOKEN_SIMPLE_WORD)
	    && (wordTokenPtr[1].size == 8)
	    && (strncmp(wordTokenPtr[1].start, "-options", 8) == 0)) {
	Tcl_Token *optsTokenPtr = TokenAfter(wordTokenPtr);
	Tcl_Token *msgTokenPtr = TokenAfter(optsTokenPtr);

	CompileWord(envPtr, optsTokenPtr, interp, 2);
	CompileWord(envPtr, msgTokenPtr,  interp, 3);
	TclEmitInvoke(envPtr, INST_RETURN_STK);
	return TCL_OK;
    }

    /*
     * Allocate some working space.
     */

    objv = (Tcl_Obj **)TclStackAlloc(interp, numOptionWords * sizeof(Tcl_Obj *));

    /*
     * Scan through the return options. If any are unknown at compile time,
     * there is no value in bytecompiling. Save the option values known in an
     * objv array for merging into a return options dictionary.
     *
     * TODO: There is potential for improvement if all option keys are known
     * at compile time and all option values relating to '-code' and '-level'
     * are known at compile time.
     */

    for (objc = 0; objc < numOptionWords; objc++) {
	TclNewObj(objv[objc]);
	Tcl_IncrRefCount(objv[objc]);
	if (!TclWordKnownAtCompileTime(wordTokenPtr, objv[objc])) {
	    /*
	     * Non-literal, so punt to run-time assembly of the dictionary.
	     */

	    for (; objc>=0 ; objc--) {
		TclDecrRefCount(objv[objc]);
	    }
	    TclStackFree(interp, objv);
	    goto issueRuntimeReturn;
	}
	wordTokenPtr = TokenAfter(wordTokenPtr);
    }
    status = TclMergeReturnOptions(interp, objc, objv,
	    &returnOpts, &code, &level);
    while (--objc >= 0) {
	TclDecrRefCount(objv[objc]);
    }
    TclStackFree(interp, objv);
    if (TCL_ERROR == status) {
	/*
	 * Something was bogus in the return options. Clear the error message,
	 * and report back to the compiler that this must be interpreted at
	 * runtime.
	 */

	Tcl_ResetResult(interp);
	return TCL_ERROR;
    }

    /*
     * All options are known at compile time, so we're going to bytecompile.
     * Emit instructions to push the result on the stack.
     */

    if (explicitResult) {
	 CompileWord(envPtr, wordTokenPtr, interp, numWords - 1);
    } else {
	/*
	 * No explict result argument, so default result is empty string.
	 */

	PushStringLiteral(envPtr, "");
    }

    /*
     * Check for optimization: When [return] is in a proc, and there's no
     * enclosing [catch], and there are no return options, then the INST_DONE
     * instruction is equivalent, and may be more efficient.
     */

    if (numOptionWords == 0 && envPtr->procPtr != NULL) {
	/*
	 * We have default return options and we're in a proc ...
	 */

	int index = envPtr->exceptArrayNext - 1;
	int enclosingCatch = 0;

	while (index >= 0) {
	    ExceptionRange range = envPtr->exceptArrayPtr[index];

	    if ((range.type == CATCH_EXCEPTION_RANGE)
		    && (range.catchOffset == -1)) {
		enclosingCatch = 1;
		break;
	    }
	    index--;
	}
	if (!enclosingCatch) {
	    /*
	     * ... and there is no enclosing catch. Issue the maximally
	     * efficient exit instruction.
	     */

	    Tcl_DecrRefCount(returnOpts);
	    TclEmitOpcode(INST_DONE, envPtr);
	    TclAdjustStackDepth(1, envPtr);
	    return TCL_OK;
	}
    }

    /* Optimize [return -level 0 $x]. */
    Tcl_DictObjSize(NULL, returnOpts, &size);
    if (size == 0 && level == 0 && code == TCL_OK) {
	Tcl_DecrRefCount(returnOpts);
	return TCL_OK;
    }

    /*
     * Could not use the optimization, so we push the return options dict, and
     * emit the INST_RETURN_IMM instruction with code and level as operands.
     */

    CompileReturnInternal(envPtr, INST_RETURN_IMM, code, level, returnOpts);
    return TCL_OK;

  issueRuntimeReturn:
    /*
     * Assemble the option dictionary (as a list as that's good enough).
     */

    wordTokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (objc=1 ; objc<=numOptionWords ; objc++) {
	CompileWord(envPtr, wordTokenPtr, interp, objc);
	wordTokenPtr = TokenAfter(wordTokenPtr);
    }
    TclEmitInstInt4(INST_LIST, numOptionWords, envPtr);

    /*
     * Push the result.
     */

    if (explicitResult) {
	CompileWord(envPtr, wordTokenPtr, interp, numWords - 1);
    } else {
	PushStringLiteral(envPtr, "");
    }

    /*
     * Issue the RETURN itself.
     */

    TclEmitInvoke(envPtr, INST_RETURN_STK);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoExistsCmd`, lines 674–720. Retained reference `e78`.

```text
TclCompileInfoExistsCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int isScalar, localIndex;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PushVarNameWord(interp, tokenPtr, envPtr, 0, &localIndex, &isScalar, 1);

    /*
     * Emit instruction to check the variable for existence.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_EXIST_STK,			envPtr);
	} else {
	    TclEmitInstInt4(	INST_EXIST_SCALAR, localIndex,	envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_EXIST_ARRAY_STK,		envPtr);
	} else {
	    TclEmitInstInt4(	INST_EXIST_ARRAY, localIndex,	envPtr);
	}
    }

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoLevelCmd`, lines 723–755. Retained reference `e79`.

```text
TclCompileInfoLevelCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * Only compile [info level] without arguments or with a single argument.
     */

    if (parsePtr->numWords == 1) {
	/*
	 * Not much to do; we compile to a single instruction...
	 */

	TclEmitOpcode(		INST_INFO_LEVEL_NUM,		envPtr);
    } else if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    } else {
	DefineLineInformation;	/* TIP #280 */

	/*
	 * Compile the argument, then add the instruction to convert it into a
	 * list of arguments.
	 */

	CompileWord(envPtr, TokenAfter(parsePtr->tokenPtr), interp, 1);
	TclEmitOpcode(		INST_INFO_LEVEL_ARGS,		envPtr);
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileNamespaceCurrentCmd`, lines 1785–1807. Retained reference `e80`.

```text
TclCompileNamespaceCurrentCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * Only compile [namespace current] without arguments.
     */

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * Not much to do; we compile to a single instruction...
     */

    TclEmitOpcode(		INST_NS_CURRENT,		envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileNamespaceCodeCmd`, lines 1810–1857. Retained reference `e81`.

```text
TclCompileNamespaceCodeCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * The specification of [namespace code] is rather shocking, in that it is
     * supposed to check if the argument is itself the result of [namespace
     * code] and not apply itself in that case. Which is excessively cautious,
     * but what the test suite checks for.
     */

    if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD || (tokenPtr[1].size > 20
	    && strncmp(tokenPtr[1].start, "::namespace inscope ", 20) == 0)) {
	/*
	 * Technically, we could just pass a literal '::namespace inscope '
	 * term through, but that's something which really shouldn't be
	 * occurring as something that the user writes so we'll just punt it.
	 */

	return TCL_ERROR;
    }

    /*
     * Now we can compile using the same strategy as [namespace code]'s normal
     * implementation does internally. Note that we can't bind the namespace
     * name directly here, because TclOO plays complex games with namespaces;
     * the value needs to be determined at runtime for safety.
     */

    PushStringLiteral(envPtr,		"::namespace");
    PushStringLiteral(envPtr,		"inscope");
    TclEmitOpcode(		INST_NS_CURRENT,	envPtr);
    CompileWord(envPtr,		tokenPtr,		interp, 1);
    TclEmitInstInt4(		INST_LIST, 4,		envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileNamespaceOriginCmd`, lines 1860–1879. Retained reference `e82`.

```text
TclCompileNamespaceOriginCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    CompileWord(envPtr,	tokenPtr,			interp, 1);
    TclEmitOpcode(	INST_ORIGIN_COMMAND,		envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoCommandsCmd`, lines 588–646. Retained reference `e83`.

```text
TclCompileInfoCommandsCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Obj *objPtr;
    const char *bytes;

    /*
     * We require one compile-time known argument for the case we can compile.
     */

    if (parsePtr->numWords == 1) {
	return TclCompileBasic0ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    } else if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);
    if (!TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	goto notCompilable;
    }
    bytes = Tcl_GetString(objPtr);

    /*
     * We require that the argument start with "::" and not have any of "*\[?"
     * in it. (Theoretically, we should look in only the final component, but
     * the difference is so slight given current naming practices.)
     */

    if (bytes[0] != ':' || bytes[1] != ':' || !TclMatchIsTrivial(bytes)) {
	goto notCompilable;
    }
    Tcl_DecrRefCount(objPtr);

    /*
     * Confirmed as a literal that will not frighten the horses. Compile.
     * The result must be made into a list.
     */

    /* TODO: Just push the known value */
    CompileWord(envPtr, tokenPtr,		interp, 1);
    TclEmitOpcode(	INST_RESOLVE_COMMAND,	envPtr);
    TclEmitOpcode(	INST_DUP,		envPtr);
    TclEmitOpcode(	INST_STR_LEN,		envPtr);
    TclEmitInstInt1(	INST_JUMP_FALSE1, 7,	envPtr);
    TclEmitInstInt4(	INST_LIST, 1,		envPtr);
    return TCL_OK;

  notCompilable:
    Tcl_DecrRefCount(objPtr);
    return TclCompileBasic1ArgCmd(interp, parsePtr, cmdPtr, envPtr);
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileRegexpCmd`, lines 2080–2222. Retained reference `e84`.

```text
TclCompileRegexpCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;	/* Pointer to the Tcl_Token representing the
				 * parse of the RE or string. */
    int len;
    int i, nocase, exact, sawLast, simple;
    const char *str;

    /*
     * We are only interested in compiling simple regexp cases. Currently
     * supported compile cases are:
     *   regexp ?-nocase? ?--? staticString $var
     *   regexp ?-nocase? ?--? {^staticString$} $var
     */

    if (parsePtr->numWords < 3) {
	return TCL_ERROR;
    }

    simple = 0;
    nocase = 0;
    sawLast = 0;
    varTokenPtr = parsePtr->tokenPtr;

    /*
     * We only look for -nocase and -- as options. Everything else gets pushed
     * to runtime execution. This is different than regexp's runtime option
     * handling, but satisfies our stricter needs.
     */

    for (i = 1; i < parsePtr->numWords - 2; i++) {
	varTokenPtr = TokenAfter(varTokenPtr);
	if (varTokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    /*
	     * Not a simple string, so punt to runtime.
	     */

	    return TCL_ERROR;
	}
	str = varTokenPtr[1].start;
	len = varTokenPtr[1].size;
	if ((len == 2) && (str[0] == '-') && (str[1] == '-')) {
	    sawLast++;
	    i++;
	    break;
	} else if ((len > 1) && (strncmp(str, "-nocase", len) == 0)) {
	    nocase = 1;
	} else {
	    /*
	     * Not an option we recognize.
	     */

	    return TCL_ERROR;
	}
    }

    if ((parsePtr->numWords - i) != 2) {
	/*
	 * We don't support capturing to variables.
	 */

	return TCL_ERROR;
    }

    /*
     * Get the regexp string. If it is not a simple string or can't be
     * converted to a glob pattern, push the word for the INST_REGEXP.
     * Keep changes here in sync with TclCompileSwitchCmd Switch_Regexp.
     */

    varTokenPtr = TokenAfter(varTokenPtr);

    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	Tcl_DString ds;

	str = varTokenPtr[1].start;
	len = varTokenPtr[1].size;

	/*
	 * If it has a '-', it could be an incorrectly formed regexp command.
	 */

	if ((*str == '-') && !sawLast) {
	    return TCL_ERROR;
	}

	/*
	 * Note: do not optimize for len == 0, as error should be generated
	 * at runtime if operand is not a resolvable variable.
	 * Bug https://core.tcl-lang.org/tcl/info/cb03e57a7b24d22c
	 */

	/*
	 * Attempt to convert pattern to glob.  If successful, push the
	 * converted pattern as a literal.
	 */

	if (TclReToGlob(NULL, varTokenPtr[1].start, len, &ds, &exact, NULL)
		== TCL_OK) {
	    simple = 1;
	    PushLiteral(envPtr, Tcl_DStringValue(&ds),Tcl_DStringLength(&ds));
	    Tcl_DStringFree(&ds);
	}
    }

    if (!simple) {
	CompileWord(envPtr, varTokenPtr, interp, parsePtr->numWords - 2);
    }

    /*
     * Push the string arg.
     */

    varTokenPtr = TokenAfter(varTokenPtr);
    CompileWord(envPtr, varTokenPtr, interp, parsePtr->numWords - 1);

    if (simple) {
	if (exact && !nocase) {
	    TclEmitOpcode(	INST_STR_EQ,			envPtr);
	} else {
	    TclEmitInstInt1(	INST_STR_MATCH, nocase,		envPtr);
	}
    } else {
	/*
	 * Pass correct RE compile flags.  We use only Int1 (8-bit), but
	 * that handles all the flags we want to pass.
	 * Don't use TCL_REG_NOSUB as we may have backrefs.
	 */

	int cflags = TCL_REG_ADVANCED | (nocase ? TCL_REG_NOCASE : 0);

	TclEmitInstInt1(	INST_REGEXP, cflags,		envPtr);
    }

    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileObjectNextCmd`, lines 2954–2976. Retained reference `e85`.

```text
TclCompileObjectNextCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    int i;

    if (parsePtr->numWords > 255) {
	return TCL_ERROR;
    }

    for (i=0 ; i<parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    TclEmitInstInt1(	INST_TCLOO_NEXT, i,		envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileObjectNextToCmd`, lines 2979–3001. Retained reference `e86`.

```text
TclCompileObjectNextToCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    int i;

    if (parsePtr->numWords < 2 || parsePtr->numWords > 255) {
	return TCL_ERROR;
    }

    for (i=0 ; i<parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    TclEmitInstInt1(	INST_TCLOO_NEXT_CLASS, i,	envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileObjectSelfCmd`, lines 3004–3064. Retained reference `e87`.

```text
TclCompileObjectSelfCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * We only handle [self] and [self object] (which is the same operation).
     * These are the only very common operations on [self] for which
     * bytecoding is at all reasonable.
     */

    if (parsePtr->numWords == 1) {
	goto compileSelfObject;
    } else if (parsePtr->numWords == 2) {
	Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr), *subcmd;

	if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD || tokenPtr[1].size==0) {
	    return TCL_ERROR;
	}

	subcmd = tokenPtr + 1;
	if (strncmp(subcmd->start, "object", subcmd->size) == 0) {
	    goto compileSelfObject;
	} else if (strncmp(subcmd->start, "namespace", subcmd->size) == 0) {
	    goto compileSelfNamespace;
	}
    }

    /*
     * Can't compile; handle with runtime call.
     */

    return TCL_ERROR;

  compileSelfObject:

    /*
     * This delegates the entire problem to a single opcode.
     */

    TclEmitOpcode(		INST_TCLOO_SELF,		envPtr);
    return TCL_OK;

  compileSelfNamespace:

    /*
     * This is formally only correct with TclOO methods as they are currently
     * implemented; it assumes that the current namespace is invariably when a
     * TclOO context is present is the object's namespace, and that's
     * technically only something that's a matter of current policy. But it
     * avoids creating another opcode, so that's all good!
     */

    TclEmitOpcode(		INST_TCLOO_SELF,		envPtr);
    TclEmitOpcode(		INST_POP,			envPtr);
    TclEmitOpcode(		INST_NS_CURRENT,		envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoObjectClassCmd`, lines 758–775. Retained reference `e88`.

```text
TclCompileInfoObjectClassCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    CompileWord(envPtr,		tokenPtr,		interp, 1);
    TclEmitOpcode(		INST_TCLOO_CLASS,	envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoObjectNamespaceCmd`, lines 814–831. Retained reference `e89`.

```text
TclCompileInfoObjectNamespaceCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    CompileWord(envPtr,		tokenPtr,		interp, 1);
    TclEmitOpcode(		INST_TCLOO_NS,		envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoObjectIsACmd`, lines 778–811. Retained reference `e90`.

```text
TclCompileInfoObjectIsACmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * We only handle [info object isa object <somevalue>]. The first three
     * words are compressed to a single token by the ensemble compilation
     * engine.
     */

    if (parsePtr->numWords != 3) {
	return TCL_ERROR;
    }
    if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD || tokenPtr[1].size < 1
	    || strncmp(tokenPtr[1].start, "object", tokenPtr[1].size)) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(tokenPtr);

    /*
     * Issue the code.
     */

    CompileWord(envPtr,		tokenPtr,		interp, 2);
    TclEmitOpcode(		INST_TCLOO_IS_OBJECT,	envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `TclCompileSetCmd`, lines 125–197. Retained reference `e91`.

```text
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

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `TclCompileUnsetCmd`, lines 3565–3683. Retained reference `e92`.

```text
TclCompileUnsetCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;
    int isScalar, localIndex, flags = 1, i, varCount = 0, haveFlags = 0;

    /* TODO: Consider support for compiling expanded args. */

    /*
     * Verify that all words - except the first non-option one - are known at
     * compile time so that we can handle them without needing to do a nasty
     * push/rotate. [Bug 3970f54c4e]
     */

    for (i=1,varTokenPtr=parsePtr->tokenPtr ; i<parsePtr->numWords ; i++) {
	Tcl_Obj *leadingWord;

	TclNewObj(leadingWord);
	varTokenPtr = TokenAfter(varTokenPtr);
	if (!TclWordKnownAtCompileTime(varTokenPtr, leadingWord)) {
	    TclDecrRefCount(leadingWord);

	    /*
	     * We can tolerate non-trivial substitutions in the first variable
	     * to be unset. If a '--' or '-nocomplain' was present, anything
	     * goes in that one place! (All subsequent variable names must be
	     * constants since we don't want to have to push them all first.)
	     */

	    if (varCount == 0) {
		if (haveFlags) {
		    continue;
		}

		/*
		 * In fact, we're OK as long as we're the first argument *and*
		 * we provably don't start with a '-'. If that is true, then
		 * even if everything else is varying, we still can't be a
		 * flag. Otherwise we'll spill to runtime to place a limit on
		 * the trickiness.
		 */

		if (varTokenPtr->type == TCL_TOKEN_WORD
			&& varTokenPtr[1].type == TCL_TOKEN_TEXT
			&& varTokenPtr[1].size > 0
			&& varTokenPtr[1].start[0] != '-') {
		    continue;
		}
	    }
	    return TCL_ERROR;
	}
	if (varCount == 0) {
	    const char *bytes;
	    int len;

	    bytes = Tcl_GetStringFromObj(leadingWord, &len);
	    if (i == 1 && len == 11 && !strncmp("-nocomplain", bytes, 11)) {
		flags = 0;
		haveFlags++;
	    } else if (i == (2 - flags) && len == 2 && !strncmp("--", bytes, 2)) {
		haveFlags++;
	    } else {
		varCount++;
	    }
	} else {
	    varCount++;
	}
	TclDecrRefCount(leadingWord);
    }

    /*
     * Issue instructions to unset each of the named variables.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (i=0; i<haveFlags;i++) {
	varTokenPtr = TokenAfter(varTokenPtr);
    }
    for (i=1+haveFlags ; i<parsePtr->numWords ; i++) {
	/*
	 * Decide if we can use a frame slot for the var/array name or if we
	 * need to emit code to compute and push the name at runtime. We use a
	 * frame slot (entry in the array of local vars) if we are compiling a
	 * procedure body and if the name is simple text that does not include
	 * namespace qualifiers.
	 */

	PushVarNameWord(interp, varTokenPtr, envPtr, 0,
		&localIndex, &isScalar, i);

	/*
	 * Emit instructions to unset the variable.
	 */

	if (isScalar) {
	    if (localIndex < 0) {
		OP1(	UNSET_STK, flags);
	    } else {
		OP14(	UNSET_SCALAR, flags, localIndex);
	    }
	} else {
	    if (localIndex < 0) {
		OP1(	UNSET_ARRAY_STK, flags);
	    } else {
		OP14(	UNSET_ARRAY, flags, localIndex);
	    }
	}

	varTokenPtr = TokenAfter(varTokenPtr);
    }
    PUSH("");
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `TclCompileTailcallCmd`, lines 2601–2627. Retained reference `e93`.

```text
TclCompileTailcallCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    int i;

    if (parsePtr->numWords < 2 || parsePtr->numWords >= 256
	    || envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    /* make room for the nsObjPtr */
    /* TODO: Doesn't this have to be a known value? */
    CompileWord(envPtr, tokenPtr, interp, 0);
    for (i=1 ; i<parsePtr->numWords ; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i);
    }
    TclEmitInstInt1(	INST_TAILCALL, parsePtr->numWords,	envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `TclCompileYieldCmd`, lines 3882–3904. Retained reference `e94`.

```text
TclCompileYieldCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    if (parsePtr->numWords < 1 || parsePtr->numWords > 2) {
	return TCL_ERROR;
    }

    if (parsePtr->numWords == 1) {
	PUSH("");
    } else {
	DefineLineInformation;	/* TIP #280 */
	Tcl_Token *valueTokenPtr = TokenAfter(parsePtr->tokenPtr);

	CompileWord(envPtr, valueTokenPtr, interp, 1);
    }
    OP(		YIELD);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `TclCompileYieldToCmd`, lines 3925–3949. Retained reference `e95`.

```text
TclCompileYieldToCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);
    int i;

    if (parsePtr->numWords < 2) {
	return TCL_ERROR;
    }

    OP(		NS_CURRENT);
    for (i = 1 ; i < parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    OP4(	LIST, i);
    OP(		YIELD_TO_INVOKE);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringEqualCmd`, lines 323–352. Retained reference `e96`.

```text
TclCompileStringEqualCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    /*
     * We don't support any flags; the bytecode isn't that sophisticated.
     */

    if (parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    /*
     * Push the two operands onto the stack and then the test.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);
    tokenPtr = TokenAfter(tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 2);
    TclEmitOpcode(INST_STR_EQ, envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringLenCmd`, lines 807–844. Retained reference `e97`.

```text
TclCompileStringLenCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Obj *objPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(objPtr);
    if (TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	/*
	 * Here someone is asking for the length of a static string (or
	 * something with backslashes). Just push the actual character (not
	 * byte) length.
	 */

	char buf[TCL_INTEGER_SPACE];
	int len = Tcl_GetCharLength(objPtr);

	len = snprintf(buf, sizeof(buf), "%d", len);
	PushLiteral(envPtr, buf, len);
    } else {
	SetLineInformation(1);
	CompileTokens(envPtr, tokenPtr, interp);
	TclEmitOpcode(INST_STR_LEN, envPtr);
    }
    TclDecrRefCount(objPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringMatchCmd`, lines 726–804. Retained reference `e98`.

```text
TclCompileStringMatchCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i, length, exactMatch = 0, nocase = 0;
    const char *str;

    if (parsePtr->numWords < 3 || parsePtr->numWords > 4) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Check if we have a -nocase flag.
     */

    if (parsePtr->numWords == 4) {
	if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    return TclCompileBasic3ArgCmd(interp, parsePtr, cmdPtr, envPtr);
	}
	str = tokenPtr[1].start;
	length = tokenPtr[1].size;
	if ((length <= 1) || strncmp(str, "-nocase", length)) {
	    /*
	     * Fail at run time, not in compilation.
	     */

	    return TclCompileBasic3ArgCmd(interp, parsePtr, cmdPtr, envPtr);
	}
	nocase = 1;
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Push the strings to match against each other.
     */

    for (i = 0; i < 2; i++) {
	if (tokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    str = tokenPtr[1].start;
	    length = tokenPtr[1].size;
	    if (!nocase && (i == 0)) {
		/*
		 * Trivial matches can be done by 'string equal'. If -nocase
		 * was specified, we can't do this because INST_STR_EQ has no
		 * support for nocase.
		 */

		Tcl_Obj *copy = Tcl_NewStringObj(str, length);

		Tcl_IncrRefCount(copy);
		exactMatch = TclMatchIsTrivial(TclGetString(copy));
		TclDecrRefCount(copy);
	    }
	    PushLiteral(envPtr, str, length);
	} else {
	    SetLineInformation(i+1+nocase);
	    CompileTokens(envPtr, tokenPtr, interp);
	}
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Push the matcher.
     */

    if (exactMatch) {
	TclEmitOpcode(INST_STR_EQ, envPtr);
    } else {
	TclEmitInstInt1(INST_STR_MATCH, nocase, envPtr);
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringTrimCmd`, lines 1244–1269. Retained reference `e99`.

```text
TclCompileStringTrimCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2 && parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr,			interp, 1);
    if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr,			interp, 2);
    } else {
	PushLiteral(envPtr, tclDefaultTrimSet, strlen(tclDefaultTrimSet));
    }
    OP(			STR_TRIM);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringTrimLCmd`, lines 1188–1213. Retained reference `e100`.

```text
TclCompileStringTrimLCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2 && parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr,			interp, 1);
    if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr,			interp, 2);
    } else {
	PushLiteral(envPtr, tclDefaultTrimSet, strlen(tclDefaultTrimSet));
    }
    OP(			STR_TRIM_LEFT);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringTrimRCmd`, lines 1216–1241. Retained reference `e101`.

```text
TclCompileStringTrimRCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2 && parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr,			interp, 1);
    if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr,			interp, 2);
    } else {
	PushLiteral(envPtr, tclDefaultTrimSet, strlen(tclDefaultTrimSet));
    }
    OP(			STR_TRIM_RIGHT);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `CompileUnaryOpCmd`, lines 3970–3986. Retained reference `e102`.

```text
CompileUnaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);
    TclEmitOpcode(instruction, envPtr);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `CompileAssociativeBinaryOpCmd`, lines 4011–4043. Retained reference `e103`.

```text
CompileAssociativeBinaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    const char *identity,
    int instruction,
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    int words;

    /* TODO: Consider support for compiling expanded args. */
    for (words=1 ; words<parsePtr->numWords ; words++) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, words);
    }
    if (parsePtr->numWords <= 2) {
	PushLiteral(envPtr, identity, -1);
	words++;
    }
    if (words > 3) {
	/*
	 * Reverse order of arguments to get precise agreement with [expr] in
	 * calculations, including roundoff errors.
	 */

	OP4(	REVERSE, words-1);
    }
    while (--words > 1) {
	TclEmitOpcode(instruction, envPtr);
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `CompileStrictlyBinaryOpCmd`, lines 4065–4076. Retained reference `e104`.

```text
CompileStrictlyBinaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    if (parsePtr->numWords != 3) {
	return TCL_ERROR;
    }
    return CompileAssociativeBinaryOpCmd(interp, parsePtr,
	    NULL, instruction, envPtr);
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompCmdsSZ.c`, function `CompileComparisonOpCmd`, lines 4097–4152. Retained reference `e105`.

```text
CompileComparisonOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    /* TODO: Consider support for compiling expanded args. */
    if (parsePtr->numWords < 3) {
	PUSH("1");
    } else if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(parsePtr->tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 1);
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 2);
	TclEmitOpcode(instruction, envPtr);
    } else if (envPtr->procPtr == NULL) {
	/*
	 * No local variable space!
	 */

	return TCL_ERROR;
    } else {
	int tmpIndex = AnonymousLocal(envPtr);
	int words;

	tokenPtr = TokenAfter(parsePtr->tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 1);
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 2);
	STORE(tmpIndex);
	TclEmitOpcode(instruction, envPtr);
	for (words=3 ; words<parsePtr->numWords ;) {
	    LOAD(tmpIndex);
	    tokenPtr = TokenAfter(tokenPtr);
	    CompileWord(envPtr, tokenPtr, interp, words);
	    if (++words < parsePtr->numWords) {
		STORE(tmpIndex);
	    }
	    TclEmitOpcode(instruction, envPtr);
	}
	for (; words>3 ; words--) {
	    OP(	BITAND);
	}

	/*
	 * Drop the value from the temp variable; retaining that reference
	 * might be expensive elsewhere.
	 */

	OP14(	UNSET_SCALAR, 0, tmpIndex);
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompile.c`, function `TclCompileTokens`, lines 2355–2550. Retained reference `e106`.

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

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompile.c`, function `TclFindCompiledLocal`, lines 2933–3028. Retained reference `e107`.

```text
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

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompile.c`, function `CompileCmdLiteral`, lines 1773–1795. Retained reference `e108`.

```text
CompileCmdLiteral(
    Tcl_Interp *interp,
    Tcl_Obj *cmdObj,
    CompileEnv *envPtr)
{
    int numBytes;
    const char *bytes;
    Command *cmdPtr;
    int cmdLitIdx, extraLiteralFlags = LITERAL_CMD_NAME;

    cmdPtr = (Command *) Tcl_GetCommandFromObj(interp, cmdObj);
    if ((cmdPtr != NULL) && (cmdPtr->flags & CMD_VIA_RESOLVER)) {
	extraLiteralFlags |= LITERAL_UNSHARED;
    }

    bytes = Tcl_GetStringFromObj(cmdObj, &numBytes);
    cmdLitIdx = TclRegisterLiteral(envPtr, (char *)bytes, numBytes, extraLiteralFlags);

    if (cmdPtr) {
	TclSetCmdNameObj(interp, TclFetchLiteral(envPtr, cmdLitIdx), cmdPtr);
    }
    TclEmitPush(cmdLitIdx, envPtr);
}

```

tcl8.6 8.6.18, revision `pinned Tcl release 8.6.18`, `generic/tclCompile.c`, function `TclCompileInvocation`, lines 1798–1839. Retained reference `e109`.

```text
TclCompileInvocation(
    Tcl_Interp *interp,
    Tcl_Token *tokenPtr,
    Tcl_Obj *cmdObj,
    int numWords,
    CompileEnv *envPtr)
{
    DefineLineInformation;
    int wordIdx = 0, depth = TclGetStackDepth(envPtr);

    if (cmdObj) {
	CompileCmdLiteral(interp, cmdObj, envPtr);
	wordIdx = 1;
	tokenPtr = TokenAfter(tokenPtr);
    }

    for (; wordIdx < numWords; wordIdx++, tokenPtr = TokenAfter(tokenPtr)) {
	int objIdx;

	SetLineInformation(wordIdx);

	if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    CompileTokens(envPtr, tokenPtr, interp);
	    continue;
	}

	objIdx = TclRegisterNewLiteral(envPtr,
		tokenPtr[1].start, tokenPtr[1].size);
	if (envPtr->clNext) {
	    TclContinuationsEnterDerived(TclFetchLiteral(envPtr, objIdx),
		    tokenPtr[1].start - envPtr->source, envPtr->clNext);
	}
	TclEmitPush(objIdx, envPtr);
    }

    if (wordIdx <= 255) {
	TclEmitInvoke(envPtr, INST_INVOKE_STK1, wordIdx);
    } else {
	TclEmitInvoke(envPtr, INST_INVOKE_STK4, wordIdx);
    }
    TclCheckStackDepth(depth+1, envPtr);
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileAppendCmd`, lines 120–225. Retained reference `e110`.

```text
TclCompileAppendCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int isScalar, localIndex, numWords, i;

    /* TODO: Consider support for compiling expanded args. */
    numWords = parsePtr->numWords;
    if (numWords == 1) {
	return TCL_ERROR;
    } else if (numWords == 2) {
	/*
	 * append varName == set varName
	 */

	return TclCompileSetCmd(interp, parsePtr, cmdPtr, envPtr);
    } else if (numWords > 3) {
	/*
	 * APPEND instructions currently only handle one value, but we can
	 * handle some multi-value cases by stringing them together.
	 */

	goto appendMultiple;
    }

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
     * We are doing an assignment, otherwise TclCompileSetCmd was called, so
     * push the new value. This will need to be extended to push a value for
     * each argument.
     */

	valueTokenPtr = TokenAfter(varTokenPtr);
	CompileWord(envPtr, valueTokenPtr, interp, 2);

    /*
     * Emit instructions to set/get the variable.
     */

	if (isScalar) {
	    if (localIndex < 0) {
		TclEmitOpcode(INST_APPEND_STK, envPtr);
	    } else {
		Emit14Inst(INST_APPEND_SCALAR, localIndex, envPtr);
	    }
	} else {
	    if (localIndex < 0) {
		TclEmitOpcode(INST_APPEND_ARRAY_STK, envPtr);
	    } else {
		Emit14Inst(INST_APPEND_ARRAY, localIndex, envPtr);
	    }
	}

    return TCL_OK;

  appendMultiple:
    /*
     * Can only handle the case where we are appending to a local scalar when
     * there are multiple values to append.  Fortunately, this is common.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    localIndex = LocalScalarFromToken(varTokenPtr, envPtr);
    if (localIndex < 0) {
	return TCL_ERROR;
    }

    /*
     * Definitely appending to a local scalar; generate the words and append
     * them.
     */

    valueTokenPtr = TokenAfter(varTokenPtr);
    for (i = 2 ; i < numWords ; i++) {
	CompileWord(envPtr, valueTokenPtr, interp, i);
	valueTokenPtr = TokenAfter(valueTokenPtr);
    }
    TclEmitInstInt4(	  INST_REVERSE, numWords-2,		envPtr);
    for (i = 2 ; i < numWords ;) {
	Emit14Inst(	  INST_APPEND_SCALAR, localIndex,	envPtr);
	if (++i < numWords) {
	    TclEmitOpcode(INST_POP,				envPtr);
	}
    }

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileConcatCmd`, lines 848–913. Retained reference `e111`.

```text
TclCompileConcatCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Obj *objPtr, *listObj;
    Tcl_Token *tokenPtr;
    int i;

    /* TODO: Consider compiling expansion case. */
    if (parsePtr->numWords == 1) {
	/*
	 * [concat] without arguments just pushes an empty object.
	 */

	PushStringLiteral(envPtr, "");
	return TCL_OK;
    }

    /*
     * Test if all arguments are compile-time known. If they are, we can
     * implement with a simple push.
     */

    TclNewObj(listObj);
    for (i = 1, tokenPtr = parsePtr->tokenPtr; i < (int)parsePtr->numWords; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	TclNewObj(objPtr);
	if (!TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	    Tcl_DecrRefCount(objPtr);
	    Tcl_DecrRefCount(listObj);
	    listObj = NULL;
	    break;
	}
	(void) Tcl_ListObjAppendElement(NULL, listObj, objPtr);
    }
    if (listObj != NULL) {
	Tcl_Obj **objs;
	const char *bytes;
	Tcl_Size len, slen;

	TclListObjGetElements(NULL, listObj, &len, &objs);
	objPtr = Tcl_ConcatObj(len, objs);
	Tcl_DecrRefCount(listObj);
	bytes = TclGetStringFromObj(objPtr, &slen);
	PushLiteral(envPtr, bytes, slen);
	Tcl_DecrRefCount(objPtr);
	return TCL_OK;
    }

    /*
     * General case: runtime concat.
     */

    for (i = 1, tokenPtr = parsePtr->tokenPtr; i < (int)parsePtr->numWords; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i);
    }

    TclEmitInstInt4(	INST_CONCAT_STK, i-1,		envPtr);

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileErrorCmd`, lines 2421–2473. Retained reference `e112`.

```text
TclCompileErrorCmd(
    Tcl_Interp *interp,		/* Used for context. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    /*
     * General syntax: [error message ?errorInfo? ?errorCode?]
     */

    if ((int)parsePtr->numWords < 2 || (int)parsePtr->numWords > 4) {
	return TCL_ERROR;
    }

    /*
     * Handle the message.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);

    /*
     * Construct the options. Note that -code and -level are not here.
     */

    if (parsePtr->numWords == 2) {
	PushStringLiteral(envPtr, "");
    } else {
	PushStringLiteral(envPtr, "-errorinfo");
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 2);
	if (parsePtr->numWords == 3) {
	    TclEmitInstInt4(	INST_LIST, 2,			envPtr);
	} else {
	    PushStringLiteral(envPtr, "-errorcode");
	    tokenPtr = TokenAfter(tokenPtr);
	    CompileWord(envPtr, tokenPtr, interp, 3);
	    TclEmitInstInt4(	INST_LIST, 4,			envPtr);
	}
    }

    /*
     * Issue the error via 'returnImm error 0'.
     */

    TclEmitInstInt4(		INST_RETURN_IMM, TCL_ERROR,	envPtr);
    TclEmitInt4(			0,			envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileBreakCmd`, lines 511–547. Retained reference `e113`.

```text
TclCompileBreakCmd(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    ExceptionRange *rangePtr;
    ExceptionAux *auxPtr;

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * Find the innermost exception range that contains this command.
     */

    rangePtr = TclGetInnermostExceptionRange(envPtr, TCL_BREAK, &auxPtr);
    if (rangePtr && rangePtr->type == LOOP_EXCEPTION_RANGE) {
	/*
	 * Found the target! No need for a nasty INST_BREAK here.
	 */

	TclCleanupStackForBreakContinue(envPtr, auxPtr);
	TclAddLoopBreakFixup(envPtr, auxPtr);
    } else {
	/*
	 * Emit a real break.
	 */

	TclEmitOpcode(INST_BREAK, envPtr);
    }
    TclAdjustStackDepth(1, envPtr);

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileContinueCmd`, lines 1012–1053. Retained reference `e114`.

```text
TclCompileContinueCmd(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    ExceptionRange *rangePtr;
    ExceptionAux *auxPtr;

    /*
     * There should be no argument after the "continue".
     */

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * See if we can find a valid continueOffset (i.e., not -1) in the
     * innermost containing exception range.
     */

    rangePtr = TclGetInnermostExceptionRange(envPtr, TCL_CONTINUE, &auxPtr);
    if (rangePtr && rangePtr->type == LOOP_EXCEPTION_RANGE) {
	/*
	 * Found the target! No need for a nasty INST_CONTINUE here.
	 */

	TclCleanupStackForBreakContinue(envPtr, auxPtr);
	TclAddLoopContinueFixup(envPtr, auxPtr);
    } else {
	/*
	 * Emit a real continue.
	 */

	TclEmitOpcode(INST_CONTINUE, envPtr);
    }
    TclAdjustStackDepth(1, envPtr);

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileDictAppendCmd`, lines 1971–2023. Retained reference `e115`.

```text
TclCompileDictAppendCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i, dictVarIndex;

    /*
     * There must be at least two argument after the command. And we impose an
     * (arbitrary) safe limit; anyone exceeding it should stop worrying about
     * speed quite so much. ;-)
     */

    /* TODO: Consider support for compiling expanded args. */
    if ((int)parsePtr->numWords<4 || (int)parsePtr->numWords>100) {
	return TCL_ERROR;
    }

    /*
     * Get the index of the local variable that we will be working with.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = LocalScalarFromToken(tokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TclCompileBasicMin2ArgCmd(interp, parsePtr,cmdPtr, envPtr);
    }

    /*
     * Produce the string to concatenate onto the dictionary entry.
     */

    tokenPtr = TokenAfter(tokenPtr);
    for (i=2 ; i<(int)parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    if ((int)parsePtr->numWords > 4) {
	TclEmitInstInt1(INST_STR_CONCAT1, (int)parsePtr->numWords-3, envPtr);
    }

    /*
     * Do the concatenation.
     */

    TclEmitInstInt4(INST_DICT_APPEND, dictVarIndex, envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileDictLappendCmd`, lines 2026–2068. Retained reference `e116`.

```text
TclCompileDictLappendCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *keyTokenPtr, *valueTokenPtr;
    int dictVarIndex;

    /*
     * There must be three arguments after the command.
     */

    /* TODO: Consider support for compiling expanded args. */
    /* Probably not.  Why is INST_DICT_LAPPEND limited to one value? */
    if (parsePtr->numWords != 4) {
	return TCL_ERROR;
    }

    /*
     * Parse the arguments.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    keyTokenPtr = TokenAfter(varTokenPtr);
    valueTokenPtr = TokenAfter(keyTokenPtr);
    dictVarIndex = LocalScalarFromToken(varTokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TclCompileBasic3ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }

    /*
     * Issue the implementation.
     */

    CompileWord(envPtr, keyTokenPtr, interp, 2);
    CompileWord(envPtr, valueTokenPtr, interp, 3);
    TclEmitInstInt4(	INST_DICT_LAPPEND, dictVarIndex,	envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileDictCreateCmd`, lines 1356–1447. Retained reference `e117`.

```text
TclCompileDictCreateCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    int worker;			/* Temp var for building the value in. */
    Tcl_Token *tokenPtr;
    Tcl_Obj *keyObj, *valueObj, *dictObj;
    const char *bytes;
    int i;
    Tcl_Size len;

    if ((parsePtr->numWords & 1) == 0) {
	return TCL_ERROR;
    }

    /*
     * See if we can build the value at compile time...
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(dictObj);
    Tcl_IncrRefCount(dictObj);
    for (i=1 ; i<(int)parsePtr->numWords ; i+=2) {
	TclNewObj(keyObj);
	Tcl_IncrRefCount(keyObj);
	if (!TclWordKnownAtCompileTime(tokenPtr, keyObj)) {
	    Tcl_DecrRefCount(keyObj);
	    Tcl_DecrRefCount(dictObj);
	    goto nonConstant;
	}
	tokenPtr = TokenAfter(tokenPtr);
	TclNewObj(valueObj);
	Tcl_IncrRefCount(valueObj);
	if (!TclWordKnownAtCompileTime(tokenPtr, valueObj)) {
	    Tcl_DecrRefCount(keyObj);
	    Tcl_DecrRefCount(valueObj);
	    Tcl_DecrRefCount(dictObj);
	    goto nonConstant;
	}
	tokenPtr = TokenAfter(tokenPtr);
	Tcl_DictObjPut(NULL, dictObj, keyObj, valueObj);
	Tcl_DecrRefCount(keyObj);
	Tcl_DecrRefCount(valueObj);
    }

    /*
     * We did! Excellent. The "verifyDict" is to do type forcing.
     */

    bytes = TclGetStringFromObj(dictObj, &len);
    PushLiteral(envPtr, bytes, len);
    TclEmitOpcode(		INST_DUP,			envPtr);
    TclEmitOpcode(		INST_DICT_VERIFY,		envPtr);
    Tcl_DecrRefCount(dictObj);
    return TCL_OK;

    /*
     * Otherwise, we've got to issue runtime code to do the building, which we
     * do by [dict set]ting into an unnamed local variable. This requires that
     * we are in a context with an LVT.
     */

  nonConstant:
    worker = AnonymousLocal(envPtr);
    if (worker < 0) {
	return TclCompileBasicMin0ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }

    PushStringLiteral(envPtr,		"");
    Emit14Inst(			INST_STORE_SCALAR, worker,	envPtr);
    TclEmitOpcode(		INST_POP,			envPtr);
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (i=1 ; i<(int)parsePtr->numWords ; i+=2) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i+1);
	tokenPtr = TokenAfter(tokenPtr);
	TclEmitInstInt4(	INST_DICT_SET, 1,		envPtr);
	TclEmitInt4(			worker,			envPtr);
	TclAdjustStackDepth(-1, envPtr);
	TclEmitOpcode(		INST_POP,			envPtr);
    }
    Emit14Inst(			INST_LOAD_SCALAR, worker,	envPtr);
    TclEmitInstInt1(		INST_UNSET_SCALAR, 0,		envPtr);
    TclEmitInt4(			worker,			envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileDictExistsCmd`, lines 1267–1300. Retained reference `e118`.

```text
TclCompileDictExistsCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i;

    /*
     * There must be at least two arguments after the command (the single-arg
     * case is legal, but too special and magic for us to deal with here).
     */

    /* TODO: Consider support for compiling expanded args. */
    if ((int)parsePtr->numWords < 3) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Now we do the code generation.
     */

    for (i=1 ; i<(int)parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    TclEmitInstInt4(INST_DICT_EXISTS, (int)parsePtr->numWords-2, envPtr);
    TclAdjustStackDepth(-1, envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileDictGetCmd`, lines 1200–1233. Retained reference `e119`.

```text
TclCompileDictGetCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i;

    /*
     * There must be at least two arguments after the command (the single-arg
     * case is legal, but too special and magic for us to deal with here).
     */

    /* TODO: Consider support for compiling expanded args. */
    if ((int)parsePtr->numWords < 3) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Only compile this because we need INST_DICT_GET anyway.
     */

    for (i=1 ; i<(int)parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    TclEmitInstInt4(INST_DICT_GET, (int)parsePtr->numWords-2, envPtr);
    TclAdjustStackDepth(-1, envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileDictGetWithDefaultCmd`, lines 1236–1264. Retained reference `e120`.

```text
TclCompileDictGetWithDefaultCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i;

    /*
     * There must be at least three arguments after the command.
     */

    /* TODO: Consider support for compiling expanded args. */
    if ((int)parsePtr->numWords < 4) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    for (i=1 ; i<(int)parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    TclEmitInstInt4(INST_DICT_GET_DEF, (int)parsePtr->numWords-3, envPtr);
    TclAdjustStackDepth(-2, envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileDictIncrCmd`, lines 1127–1197. Retained reference `e121`.

```text
TclCompileDictIncrCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *keyTokenPtr;
    int dictVarIndex, incrAmount;

    /*
     * There must be at least two arguments after the command.
     */

    if ((int)parsePtr->numWords < 3 || (int)parsePtr->numWords > 4) {
	return TCL_ERROR;
    }
    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    keyTokenPtr = TokenAfter(varTokenPtr);

    /*
     * Parse the increment amount, if present.
     */

    if (parsePtr->numWords == 4) {
	const char *word;
	Tcl_Size numBytes;
	int code;
	Tcl_Token *incrTokenPtr;
	Tcl_Obj *intObj;

	incrTokenPtr = TokenAfter(keyTokenPtr);
	if (incrTokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    return TclCompileBasic2Or3ArgCmd(interp, parsePtr,cmdPtr, envPtr);
	}
	word = incrTokenPtr[1].start;
	numBytes = incrTokenPtr[1].size;

	intObj = Tcl_NewStringObj(word, numBytes);
	Tcl_IncrRefCount(intObj);
	code = TclGetIntFromObj(NULL, intObj, &incrAmount);
	TclDecrRefCount(intObj);
	if (code != TCL_OK) {
	    return TclCompileBasic2Or3ArgCmd(interp, parsePtr,cmdPtr, envPtr);
	}
    } else {
	incrAmount = 1;
    }

    /*
     * The dictionary variable must be a local scalar that is knowable at
     * compile time; anything else exceeds the complexity of the opcode. So
     * discover what the index is.
     */

    dictVarIndex = LocalScalarFromToken(varTokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TclCompileBasic2Or3ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }

    /*
     * Emit the key and the code to actually do the increment.
     */

    CompileWord(envPtr, keyTokenPtr, interp, 2);
    TclEmitInstInt4( INST_DICT_INCR_IMM, incrAmount,	envPtr);
    TclEmitInt4(     dictVarIndex,			envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileDictMergeCmd`, lines 1450–1561. Retained reference `e122`.

```text
TclCompileDictMergeCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i, workerIndex, infoIndex, outLoop;

    /*
     * Deal with some special edge cases. Note that in the case with one
     * argument, the only thing to do is to verify the dict-ness.
     */

    /* TODO: Consider support for compiling expanded args. (less likely) */
    if ((int)parsePtr->numWords < 2) {
	PushStringLiteral(envPtr, "");
	return TCL_OK;
    } else if (parsePtr->numWords == 2) {
	tokenPtr = TokenAfter(parsePtr->tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 1);
	TclEmitOpcode(		INST_DUP,			envPtr);
	TclEmitOpcode(		INST_DICT_VERIFY,		envPtr);
	return TCL_OK;
    }

    /*
     * There's real merging work to do.
     *
     * Allocate some working space. This means we'll only ever compile this
     * command when there's an LVT present.
     */

    workerIndex = AnonymousLocal(envPtr);
    if (workerIndex < 0) {
	return TclCompileBasicMin2ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }
    infoIndex = AnonymousLocal(envPtr);

    /*
     * Get the first dictionary and verify that it is so.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);
    TclEmitOpcode(		INST_DUP,			envPtr);
    TclEmitOpcode(		INST_DICT_VERIFY,		envPtr);
    Emit14Inst(			INST_STORE_SCALAR, workerIndex,	envPtr);
    TclEmitOpcode(		INST_POP,			envPtr);

    /*
     * For each of the remaining dictionaries...
     */

    outLoop = TclCreateExceptRange(CATCH_EXCEPTION_RANGE, envPtr);
    TclEmitInstInt4(		INST_BEGIN_CATCH4, outLoop,	envPtr);
    ExceptionRangeStarts(envPtr, outLoop);
    for (i=2 ; i<(int)parsePtr->numWords ; i++) {
	/*
	 * Get the dictionary, and merge its pairs into the first dict (using
	 * a small loop).
	 */

	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i);
	TclEmitInstInt4(	INST_DICT_FIRST, infoIndex,	envPtr);
	TclEmitInstInt1(	INST_JUMP_TRUE1, 24,		envPtr);
	TclEmitInstInt4(	INST_REVERSE, 2,		envPtr);
	TclEmitInstInt4(	INST_DICT_SET, 1,		envPtr);
	TclEmitInt4(			workerIndex,		envPtr);
	TclAdjustStackDepth(-1, envPtr);
	TclEmitOpcode(		INST_POP,			envPtr);
	TclEmitInstInt4(	INST_DICT_NEXT, infoIndex,	envPtr);
	TclEmitInstInt1(	INST_JUMP_FALSE1, -20,		envPtr);
	TclEmitOpcode(		INST_POP,			envPtr);
	TclEmitOpcode(		INST_POP,			envPtr);
	TclEmitInstInt1(	INST_UNSET_SCALAR, 0,		envPtr);
	TclEmitInt4(			infoIndex,		envPtr);
    }
    ExceptionRangeEnds(envPtr, outLoop);
    TclEmitOpcode(		INST_END_CATCH,			envPtr);

    /*
     * Clean up any state left over.
     */

    Emit14Inst(			INST_LOAD_SCALAR, workerIndex,	envPtr);
    TclEmitInstInt1(		INST_UNSET_SCALAR, 0,		envPtr);
    TclEmitInt4(			workerIndex,		envPtr);
    TclEmitInstInt1(		INST_JUMP1, 18,			envPtr);

    /*
     * If an exception happens when starting to iterate over the second (and
     * subsequent) dicts. This is strictly not necessary, but it is nice.
     */

    TclAdjustStackDepth(-1, envPtr);
    ExceptionRangeTarget(envPtr, outLoop, catchOffset);
    TclEmitOpcode(		INST_PUSH_RETURN_OPTIONS,	envPtr);
    TclEmitOpcode(		INST_PUSH_RESULT,		envPtr);
    TclEmitOpcode(		INST_END_CATCH,			envPtr);
    TclEmitInstInt1(		INST_UNSET_SCALAR, 0,		envPtr);
    TclEmitInt4(			workerIndex,		envPtr);
    TclEmitInstInt1(		INST_UNSET_SCALAR, 0,		envPtr);
    TclEmitInt4(			infoIndex,		envPtr);
    TclEmitOpcode(		INST_RETURN_STK,		envPtr);

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileDictSetCmd`, lines 1074–1124. Retained reference `e123`.

```text
TclCompileDictSetCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i, dictVarIndex;
    Tcl_Token *varTokenPtr;

    /*
     * There must be at least one argument after the command.
     */

    if ((int)parsePtr->numWords < 4) {
	return TCL_ERROR;
    }

    /*
     * The dictionary variable must be a local scalar that is knowable at
     * compile time; anything else exceeds the complexity of the opcode. So
     * discover what the index is.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = LocalScalarFromToken(varTokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TCL_ERROR;
    }

    /*
     * Remaining words (key path and value to set) can be handled normally.
     */

    tokenPtr = TokenAfter(varTokenPtr);
    for (i=2 ; i< (int)parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Now emit the instruction to do the dict manipulation.
     */

    TclEmitInstInt4( INST_DICT_SET, (int)parsePtr->numWords-3,	envPtr);
    TclEmitInt4(     dictVarIndex,			envPtr);
    TclAdjustStackDepth(-1, envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclCompileDictUnsetCmd`, lines 1303–1353. Retained reference `e124`.

```text
TclCompileDictUnsetCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i, dictVarIndex;

    /*
     * There must be at least one argument after the variable name for us to
     * compile to bytecode.
     */

    /* TODO: Consider support for compiling expanded args. */
    if ((int)parsePtr->numWords < 3) {
	return TCL_ERROR;
    }

    /*
     * The dictionary variable must be a local scalar that is knowable at
     * compile time; anything else exceeds the complexity of the opcode. So
     * discover what the index is.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = LocalScalarFromToken(tokenPtr, envPtr);
    if (dictVarIndex < 0) {
	return TclCompileBasicMin2ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }

    /*
     * Remaining words (the key path) can be handled normally.
     */

    for (i=2 ; i<(int)parsePtr->numWords ; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i);
    }

    /*
     * Now emit the instruction to do the dict manipulation.
     */

    TclEmitInstInt4( INST_DICT_UNSET, (int)parsePtr->numWords-2,	envPtr);
    TclEmitInt4(	dictVarIndex,				envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmds.c`, function `TclPushVarName`, lines 3488–3694. Retained reference `e125`.

```text
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

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileLappendCmd`, lines 838–932. Retained reference `e126`.

```text
TclCompileLappendCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int isScalar, localIndex, numWords, i;

    /* TODO: Consider support for compiling expanded args. */
    numWords = parsePtr->numWords;
    if (numWords < 3) {
	return TCL_ERROR;
    }

    if (numWords != 3 || envPtr->procPtr == NULL) {
	goto lappendMultiple;
    }

    /*
     * Decide if we can use a frame slot for the var/array name or if we
     * need to emit code to compute and push the name at runtime. We use a
     * frame slot (entry in the array of local vars) if we are compiling a
     * procedure body and if the name is simple text that does not include
     * namespace qualifiers.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    PushVarNameWord(interp, varTokenPtr, envPtr, 0,
	    &localIndex, &isScalar, 1);

    /*
     * If we are doing an assignment, push the new value. In the no values
     * case, create an empty object.
     */

    if (numWords > 2) {
	valueTokenPtr = TokenAfter(varTokenPtr);

	CompileWord(envPtr, valueTokenPtr, interp, 2);
    }

    /*
     * Emit instructions to set/get the variable.
     */

    /*
     * The *_STK opcodes should be refactored to make better use of existing
     * LOAD/STORE instructions.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_LAPPEND_STK,		envPtr);
	} else {
	    Emit14Inst(		INST_LAPPEND_SCALAR, localIndex, envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_LAPPEND_ARRAY_STK,		envPtr);
	} else {
	    Emit14Inst(		INST_LAPPEND_ARRAY, localIndex,	envPtr);
	}
    }

    return TCL_OK;

  lappendMultiple:
    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    PushVarNameWord(interp, varTokenPtr, envPtr, 0,
	    &localIndex, &isScalar, 1);
    valueTokenPtr = TokenAfter(varTokenPtr);
    for (i = 2 ; i < numWords ; i++) {
	CompileWord(envPtr, valueTokenPtr, interp, i);
	valueTokenPtr = TokenAfter(valueTokenPtr);
    }
    TclEmitInstInt4(	    INST_LIST, numWords - 2,		envPtr);
    if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(  INST_LAPPEND_LIST_STK,		envPtr);
	} else {
	    TclEmitInstInt4(INST_LAPPEND_LIST, localIndex,	envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(  INST_LAPPEND_LIST_ARRAY_STK,	envPtr);
	} else {
	    TclEmitInstInt4(INST_LAPPEND_LIST_ARRAY, localIndex,envPtr);
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileIncrCmd`, lines 469–559. Retained reference `e127`.

```text
TclCompileIncrCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *incrTokenPtr;
    int isScalar, localIndex, haveImmValue;
    Tcl_WideInt immValue;

    if ((parsePtr->numWords != 2) && (parsePtr->numWords != 3)) {
	return TCL_ERROR;
    }

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    PushVarNameWord(interp, varTokenPtr, envPtr, TCL_NO_LARGE_INDEX,
	    &localIndex, &isScalar, 1);

    /*
     * If an increment is given, push it, but see first if it's a small
     * integer.
     */

    haveImmValue = 0;
    immValue = 1;
    if (parsePtr->numWords == 3) {
	incrTokenPtr = TokenAfter(varTokenPtr);
	if (incrTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    const char *word = incrTokenPtr[1].start;
	    size_t numBytes = incrTokenPtr[1].size;
	    int code;
	    Tcl_Obj *intObj = Tcl_NewStringObj(word, numBytes);

	    Tcl_IncrRefCount(intObj);
	    code = TclGetWideIntFromObj(NULL, intObj, &immValue);
	    if ((code == TCL_OK) && (-127 <= immValue) && (immValue <= 127)) {
		haveImmValue = 1;
	    }
	    TclDecrRefCount(intObj);
	    if (!haveImmValue) {
		PushLiteral(envPtr, word, numBytes);
	    }
	} else {
	    SetLineInformation(2);
	    CompileTokens(envPtr, incrTokenPtr, interp);
	}
    } else {			/* No incr amount given so use 1. */
	haveImmValue = 1;
    }

    /*
     * Emit the instruction to increment the variable.
     */

    if (isScalar) {	/* Simple scalar variable. */
	if (localIndex >= 0) {
	    if (haveImmValue) {
		TclEmitInstInt1(INST_INCR_SCALAR1_IMM, localIndex, envPtr);
		TclEmitInt1(immValue, envPtr);
	    } else {
		TclEmitInstInt1(INST_INCR_SCALAR1, localIndex,	envPtr);
	    }
	} else {
	    if (haveImmValue) {
		TclEmitInstInt1(INST_INCR_STK_IMM, immValue, envPtr);
	    } else {
		TclEmitOpcode(	INST_INCR_STK,		envPtr);
	    }
	}
    } else {			/* Simple array variable. */
	if (localIndex >= 0) {
	    if (haveImmValue) {
		TclEmitInstInt1(INST_INCR_ARRAY1_IMM, localIndex, envPtr);
		TclEmitInt1(immValue, envPtr);
	    } else {
		TclEmitInstInt1(INST_INCR_ARRAY1, localIndex,	envPtr);
	    }
	} else {
	    if (haveImmValue) {
		TclEmitInstInt1(INST_INCR_ARRAY_STK_IMM, immValue, envPtr);
	    } else {
		TclEmitOpcode(	INST_INCR_ARRAY_STK,		envPtr);
	    }
	}
    }

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileUpvarCmd`, lines 2508–2593. Retained reference `e128`.

```text
TclCompileUpvarCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr, *otherTokenPtr, *localTokenPtr;
    int localIndex, numWords, i;
    Tcl_Obj *objPtr;

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    numWords = parsePtr->numWords;
    if (numWords < 3) {
	return TCL_ERROR;
    }

    /*
     * Push the frame index if it is known at compile time
     */

    TclNewObj(objPtr);
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    if (TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	CallFrame *framePtr;
	const Tcl_ObjType *newTypePtr, *typePtr = objPtr->typePtr;

	/*
	 * Attempt to convert to a level reference. Note that TclObjGetFrame
	 * only changes the obj type when a conversion was successful.
	 */

	TclObjGetFrame(interp, objPtr, &framePtr);
	newTypePtr = objPtr->typePtr;
	Tcl_DecrRefCount(objPtr);

	if (newTypePtr != typePtr) {
	    if (numWords%2) {
		return TCL_ERROR;
	    }
	    /* TODO: Push the known value instead? */
	    CompileWord(envPtr, tokenPtr, interp, 1);
	    otherTokenPtr = TokenAfter(tokenPtr);
	    i = 2;
	} else {
	    if (!(numWords%2)) {
		return TCL_ERROR;
	    }
	    PushStringLiteral(envPtr, "1");
	    otherTokenPtr = tokenPtr;
	    i = 1;
	}
    } else {
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    /*
     * Loop over the (otherVar, thisVar) pairs. If any of the thisVar is not a
     * local variable, return an error so that the non-compiled command will
     * be called at runtime.
     */

    for (; i<numWords; i+=2, otherTokenPtr = TokenAfter(localTokenPtr)) {
	localTokenPtr = TokenAfter(otherTokenPtr);

	CompileWord(envPtr, otherTokenPtr, interp, i);
	localIndex = LocalScalarFromToken(localTokenPtr, envPtr);
	if (localIndex < 0) {
	    return TCL_ERROR;
	}
	TclEmitInstInt4(	INST_UPVAR, localIndex,		envPtr);
    }

    /*
     * Pop the frame index, and set the result to empty
     */

    TclEmitOpcode(		INST_POP,			envPtr);
    PushStringLiteral(envPtr, "");
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileGlobalCmd`, lines 85–145. Retained reference `e129`.

```text
TclCompileGlobalCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;
    int localIndex, numWords, i;

    /* TODO: Consider support for compiling expanded args. */
    numWords = parsePtr->numWords;
    if (numWords < 2) {
	return TCL_ERROR;
    }

    /*
     * 'global' has no effect outside of proc bodies; handle that at runtime
     */

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Push the namespace
     */

    PushStringLiteral(envPtr, "::");

    /*
     * Loop over the variables.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (i=1; i<numWords; varTokenPtr = TokenAfter(varTokenPtr),i++) {
	localIndex = IndexTailVarIfKnown(interp, varTokenPtr, envPtr);

	if (localIndex < 0) {
	    return TCL_ERROR;
	}

	/*
	 * TODO: Consider what value can pass through the
	 * IndexTailVarIfKnown() screen. Full CompileWord() likely does not
	 * apply here. Push known value instead.
	 */

	CompileWord(envPtr, varTokenPtr, interp, i);
	TclEmitInstInt4(	INST_NSUPVAR, localIndex,	envPtr);
    }

    /*
     * Pop the namespace, and set the result to empty
     */

    TclEmitOpcode(		INST_POP,			envPtr);
    PushStringLiteral(envPtr, "");
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileVariableCmd`, lines 2614–2676. Retained reference `e130`.

```text
TclCompileVariableCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int localIndex, numWords, i;

    numWords = parsePtr->numWords;
    if (numWords < 2) {
	return TCL_ERROR;
    }

    /*
     * Bail out if not compiling a proc body
     */

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Loop over the (var, value) pairs.
     */

    valueTokenPtr = parsePtr->tokenPtr;
    for (i=1; i<numWords; i+=2) {
	varTokenPtr = TokenAfter(valueTokenPtr);
	valueTokenPtr = TokenAfter(varTokenPtr);

	localIndex = IndexTailVarIfKnown(interp, varTokenPtr, envPtr);

	if (localIndex < 0) {
	    return TCL_ERROR;
	}

	/* TODO: Consider what value can pass through the
	 * IndexTailVarIfKnown() screen.  Full CompileWord()
	 * likely does not apply here.  Push known value instead. */
	CompileWord(envPtr, varTokenPtr, interp, i);
	TclEmitInstInt4(	INST_VARIABLE, localIndex,	envPtr);

	if (i + 1 < numWords) {
	    /*
	     * A value has been given: set the variable, pop the value
	     */

	    CompileWord(envPtr, valueTokenPtr, interp, i + 1);
	    Emit14Inst(		INST_STORE_SCALAR, localIndex,	envPtr);
	    TclEmitOpcode(	INST_POP,			envPtr);
	}
    }

    /*
     * Set the result to empty
     */

    PushStringLiteral(envPtr, "");
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileLassignCmd`, lines 953–1035. Retained reference `e131`.

```text
TclCompileLassignCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int isScalar, localIndex, numWords, idx;

    numWords = parsePtr->numWords;

    /*
     * Check for command syntax error, but we'll punt that to runtime.
     */

    if (numWords < 3) {
	return TCL_ERROR;
    }

    /*
     * Generate code to push list being taken apart by [lassign].
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);

    /*
     * Generate code to assign values from the list to variables.
     */

    for (idx=0 ; idx<numWords-2 ; idx++) {
	tokenPtr = TokenAfter(tokenPtr);

	/*
	 * Generate the next variable name.
	 */

	PushVarNameWord(interp, tokenPtr, envPtr, 0, &localIndex,
		&isScalar, idx + 2);

	/*
	 * Emit instructions to get the idx'th item out of the list value on
	 * the stack and assign it to the variable.
	 */

	if (isScalar) {
	    if (localIndex >= 0) {
		TclEmitOpcode(	INST_DUP,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		Emit14Inst(	INST_STORE_SCALAR, localIndex,	envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    } else {
		TclEmitInstInt4(INST_OVER, 1,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		TclEmitOpcode(	INST_STORE_STK,			envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    }
	} else {
	    if (localIndex >= 0) {
		TclEmitInstInt4(INST_OVER, 1,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		Emit14Inst(	INST_STORE_ARRAY, localIndex,	envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    } else {
		TclEmitInstInt4(INST_OVER, 2,			envPtr);
		TclEmitInstInt4(INST_LIST_INDEX_IMM, idx,	envPtr);
		TclEmitOpcode(	INST_STORE_ARRAY_STK,		envPtr);
		TclEmitOpcode(	INST_POP,			envPtr);
	    }
	}
    }

    /*
     * Generate code to leave the rest of the list on the stack.
     */

    TclEmitInstInt4(		INST_LIST_RANGE_IMM, idx,	envPtr);
    TclEmitInt4(			(int)TCL_INDEX_END,		envPtr);

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileListCmd`, lines 1146–1238. Retained reference `e132`.

```text
TclCompileListCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *valueTokenPtr;
    int i, numWords, concat, build;
    Tcl_Obj *listObj, *objPtr;

    if (parsePtr->numWords == 1) {
	/*
	 * [list] without arguments just pushes an empty object.
	 */

	PushStringLiteral(envPtr, "");
	return TCL_OK;
    }

    /*
     * Test if all arguments are compile-time known. If they are, we can
     * implement with a simple push.
     */

    numWords = parsePtr->numWords;
    valueTokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(listObj);
    for (i = 1; i < numWords && listObj != NULL; i++) {
	TclNewObj(objPtr);
	if (TclWordKnownAtCompileTime(valueTokenPtr, objPtr)) {
	    (void) Tcl_ListObjAppendElement(NULL, listObj, objPtr);
	} else {
	    Tcl_DecrRefCount(objPtr);
	    Tcl_DecrRefCount(listObj);
	    listObj = NULL;
	}
	valueTokenPtr = TokenAfter(valueTokenPtr);
    }
    if (listObj != NULL) {
	TclEmitPush(TclAddLiteralObj(envPtr, listObj, NULL), envPtr);
	return TCL_OK;
    }

    /*
     * Push the all values onto the stack.
     */

    numWords = parsePtr->numWords;
    valueTokenPtr = TokenAfter(parsePtr->tokenPtr);
    concat = build = 0;
    for (i = 1; i < numWords; i++) {
	if (valueTokenPtr->type == TCL_TOKEN_EXPAND_WORD && build > 0) {
	    TclEmitInstInt4(	INST_LIST, build,	envPtr);
	    if (concat) {
		TclEmitOpcode(	INST_LIST_CONCAT,	envPtr);
	    }
	    build = 0;
	    concat = 1;
	}
	CompileWord(envPtr, valueTokenPtr, interp, i);
	if (valueTokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	    if (concat) {
		TclEmitOpcode(	INST_LIST_CONCAT,	envPtr);
	    } else {
		concat = 1;
	    }
	} else {
	    build++;
	}
	valueTokenPtr = TokenAfter(valueTokenPtr);
    }
    if (build > 0) {
	TclEmitInstInt4(	INST_LIST, build,	envPtr);
	if (concat) {
	    TclEmitOpcode(	INST_LIST_CONCAT,	envPtr);
	}
    }

    /*
     * If there was just one expanded word, we must ensure that it is a list
     * at this point. We use an [lrange ... 0 end] for this (instead of
     * [llength], as with literals) as we must drop any string representation
     * that might be hanging around.
     */

    if (concat && numWords == 2) {
	TclEmitInstInt4(	INST_LIST_RANGE_IMM, 0,	envPtr);
	TclEmitInt4(			(int)TCL_INDEX_END,	envPtr);
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileLindexCmd`, lines 1056–1125. Retained reference `e133`.

```text
TclCompileLindexCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *idxTokenPtr, *valTokenPtr;
    int i, idx, numWords = parsePtr->numWords;

    /*
     * Quit if not enough args.
     */

    /* TODO: Consider support for compiling expanded args. */
    if (numWords <= 1) {
	return TCL_ERROR;
    }

    valTokenPtr = TokenAfter(parsePtr->tokenPtr);
    if (numWords != 3) {
	goto emitComplexLindex;
    }

    idxTokenPtr = TokenAfter(valTokenPtr);
    if (TclGetIndexFromToken(idxTokenPtr, TCL_INDEX_NONE,
	    TCL_INDEX_NONE, &idx) == TCL_OK) {
	/*
	 * The idxTokenPtr parsed as a valid index value and was
	 * encoded as expected by INST_LIST_INDEX_IMM.
	 *
	 * NOTE: that we rely on indexing before a list producing the
	 * same result as indexing after a list.
	 */

	CompileWord(envPtr, valTokenPtr, interp, 1);
	TclEmitInstInt4(	INST_LIST_INDEX_IMM, idx,	envPtr);
	return TCL_OK;
    }

    /*
     * If the value was not known at compile time, the conversion failed or
     * the value was negative, we just keep on going with the more complex
     * compilation.
     */

    /*
     * Push the operands onto the stack.
     */

  emitComplexLindex:
    for (i=1 ; i<numWords ; i++) {
	CompileWord(envPtr, valTokenPtr, interp, i);
	valTokenPtr = TokenAfter(valTokenPtr);
    }

    /*
     * Emit INST_LIST_INDEX if objc==3, or INST_LIST_INDEX_MULTI if there are
     * multiple index args.
     */

    if (numWords == 3) {
	TclEmitOpcode(		INST_LIST_INDEX,		envPtr);
    } else {
	TclEmitInstInt4(	INST_LIST_INDEX_MULTI, numWords-1, envPtr);
    }

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileLlengthCmd`, lines 1259–1277. Retained reference `e134`.

```text
TclCompileLlengthCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    CompileWord(envPtr, varTokenPtr, interp, 1);
    TclEmitOpcode(		INST_LIST_LENGTH,		envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileLrangeCmd`, lines 1291–1337. Retained reference `e135`.

```text
TclCompileLrangeCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for context. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr, *listTokenPtr;
    int idx1, idx2;

    if (parsePtr->numWords != 4) {
	return TCL_ERROR;
    }
    listTokenPtr = TokenAfter(parsePtr->tokenPtr);

    tokenPtr = TokenAfter(listTokenPtr);
    if ((TclGetIndexFromToken(tokenPtr, TCL_INDEX_START, TCL_INDEX_NONE,
	    &idx1) != TCL_OK) || (idx1 == (int)TCL_INDEX_NONE)) {
	return TCL_ERROR;
    }
    /*
     * Token was an index value, and we treat all "first" indices
     * before the list same as the start of the list.
     */

    tokenPtr = TokenAfter(tokenPtr);
    if (TclGetIndexFromToken(tokenPtr, TCL_INDEX_NONE, TCL_INDEX_END,
	    &idx2) != TCL_OK) {
	return TCL_ERROR;
    }
    /*
     * Token was an index value, and we treat all "last" indices
     * after the list same as the end of the list.
     */

    /*
     * Issue instructions. It's not safe to skip doing the LIST_RANGE, as
     * we've not proved that the 'list' argument is really a list. Not that it
     * is worth trying to do that given current knowledge.
     */

    CompileWord(envPtr, listTokenPtr, interp, 1);
    TclEmitInstInt4(		INST_LIST_RANGE_IMM, idx1,	envPtr);
    TclEmitInt4(		idx2,				envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileLinsertCmd`, lines 1351–1389. Retained reference `e136`.

```text
TclCompileLinsertCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for context. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int i;

    if ((int)parsePtr->numWords < 3) {
	return TCL_ERROR;
    }

    /* Push list, insertion index onto the stack */
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);
    tokenPtr = TokenAfter(tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 2);

    /* Push new elements to be inserted */
    for (i=3 ; i<(int)parsePtr->numWords ; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i);
    }

    /* First operand is count of arguments */
    TclEmitInstInt4(INST_LREPLACE4, parsePtr->numWords - 1, envPtr);
    /*
     * Second operand is bitmask
     *  TCL_LREPLACE4_END_IS_LAST - end refers to last element
     *  TCL_LREPLACE4_SINGLE_INDEX - second index is not present
     *     indicating this is a pure insert
     */
    TclEmitInt1(TCL_LREPLACE4_SINGLE_INDEX, envPtr);

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileLsetCmd`, lines 1484–1607. Retained reference `e137`.

```text
TclCompileLsetCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    int tempDepth;		/* Depth used for emitting one part of the
				 * code burst. */
    Tcl_Token *varTokenPtr;	/* Pointer to the Tcl_Token representing the
				 * parse of the variable name. */
    int localIndex;		/* Index of var in local var table. */
    int isScalar;		/* Flag == 1 if scalar, 0 if array. */
    int i;

    /*
     * Check argument count.
     */

    /* TODO: Consider support for compiling expanded args. */
    if ((int)parsePtr->numWords < 3) {
	/*
	 * Fail at run time, not in compilation.
	 */

	return TCL_ERROR;
    }

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
     * Push the "index" args and the new element value.
     */

    for (i=2 ; i<(int)parsePtr->numWords ; ++i) {
	varTokenPtr = TokenAfter(varTokenPtr);
	CompileWord(envPtr, varTokenPtr, interp, i);
    }

    /*
     * Duplicate the variable name if it's been pushed.
     */

    if (localIndex < 0) {
	if (isScalar) {
	    tempDepth = parsePtr->numWords - 2;
	} else {
	    tempDepth = parsePtr->numWords - 1;
	}
	TclEmitInstInt4(	INST_OVER, tempDepth,		envPtr);
    }

    /*
     * Duplicate an array index if one's been pushed.
     */

    if (!isScalar) {
	if (localIndex < 0) {
	    tempDepth = parsePtr->numWords - 1;
	} else {
	    tempDepth = parsePtr->numWords - 2;
	}
	TclEmitInstInt4(	INST_OVER, tempDepth,		envPtr);
    }

    /*
     * Emit code to load the variable's value.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_LOAD_STK,			envPtr);
	} else {
	    Emit14Inst(		INST_LOAD_SCALAR, localIndex,	envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_LOAD_ARRAY_STK,		envPtr);
	} else {
	    Emit14Inst(		INST_LOAD_ARRAY, localIndex,	envPtr);
	}
    }

    /*
     * Emit the correct variety of 'lset' instruction.
     */

    if (parsePtr->numWords == 4) {
	TclEmitOpcode(		INST_LSET_LIST,			envPtr);
    } else {
	TclEmitInstInt4(	INST_LSET_FLAT, parsePtr->numWords-1, envPtr);
    }

    /*
     * Emit code to put the value back in the variable.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_STORE_STK,			envPtr);
	} else {
	    Emit14Inst(		INST_STORE_SCALAR, localIndex,	envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_STORE_ARRAY_STK,		envPtr);
	} else {
	    Emit14Inst(		INST_STORE_ARRAY, localIndex,	envPtr);
	}
    }

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileReturnCmd`, lines 2256–2441. Retained reference `e138`.

```text
TclCompileReturnCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    /*
     * General syntax: [return ?-option value ...? ?result?]
     * An even number of words means an explicit result argument is present.
     */
    int level, code, objc, status = TCL_OK;
    Tcl_Size size;
    int numWords = parsePtr->numWords;
    int explicitResult = (0 == (numWords % 2));
    int numOptionWords = numWords - 1 - explicitResult;
    Tcl_Obj *returnOpts, **objv;
    Tcl_Token *wordTokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Check for special case which can always be compiled:
     *	    return -options <opts> <msg>
     * Unlike the normal [return] compilation, this version does everything at
     * runtime so it can handle arbitrary words and not just literals. Note
     * that if INST_RETURN_STK wasn't already needed for something else
     * ('finally' clause processing) this piece of code would not be present.
     */

    if ((numWords == 4) && (wordTokenPtr->type == TCL_TOKEN_SIMPLE_WORD)
	    && (wordTokenPtr[1].size == 8)
	    && (strncmp(wordTokenPtr[1].start, "-options", 8) == 0)) {
	Tcl_Token *optsTokenPtr = TokenAfter(wordTokenPtr);
	Tcl_Token *msgTokenPtr = TokenAfter(optsTokenPtr);

	CompileWord(envPtr, optsTokenPtr, interp, 2);
	CompileWord(envPtr, msgTokenPtr,  interp, 3);
	TclEmitInvoke(envPtr, INST_RETURN_STK);
	return TCL_OK;
    }

    /*
     * Allocate some working space.
     */

    objv = (Tcl_Obj **)TclStackAlloc(interp, numOptionWords * sizeof(Tcl_Obj *));

    /*
     * Scan through the return options. If any are unknown at compile time,
     * there is no value in bytecompiling. Save the option values known in an
     * objv array for merging into a return options dictionary.
     *
     * TODO: There is potential for improvement if all option keys are known
     * at compile time and all option values relating to '-code' and '-level'
     * are known at compile time.
     */

    for (objc = 0; objc < numOptionWords; objc++) {
	TclNewObj(objv[objc]);
	Tcl_IncrRefCount(objv[objc]);
	if (!TclWordKnownAtCompileTime(wordTokenPtr, objv[objc])) {
	    /*
	     * Non-literal, so punt to run-time assembly of the dictionary.
	     */

	    for (; objc>=0 ; objc--) {
		TclDecrRefCount(objv[objc]);
	    }
	    TclStackFree(interp, objv);
	    goto issueRuntimeReturn;
	}
	wordTokenPtr = TokenAfter(wordTokenPtr);
    }
    status = TclMergeReturnOptions(interp, objc, objv,
	    &returnOpts, &code, &level);
    while (--objc >= 0) {
	TclDecrRefCount(objv[objc]);
    }
    TclStackFree(interp, objv);
    if (TCL_ERROR == status) {
	/*
	 * Something was bogus in the return options. Clear the error message,
	 * and report back to the compiler that this must be interpreted at
	 * runtime.
	 */

	Tcl_ResetResult(interp);
	return TCL_ERROR;
    }

    /*
     * All options are known at compile time, so we're going to bytecompile.
     * Emit instructions to push the result on the stack.
     */

    if (explicitResult) {
	 CompileWord(envPtr, wordTokenPtr, interp, numWords - 1);
    } else {
	/*
	 * No explict result argument, so default result is empty string.
	 */

	PushStringLiteral(envPtr, "");
    }

    /*
     * Check for optimization: When [return] is in a proc, and there's no
     * enclosing [catch], and there are no return options, then the INST_DONE
     * instruction is equivalent, and may be more efficient.
     */

    if (numOptionWords == 0 && envPtr->procPtr != NULL) {
	/*
	 * We have default return options and we're in a proc ...
	 */

	int index = envPtr->exceptArrayNext - 1;
	int enclosingCatch = 0;

	while (index >= 0) {
	    ExceptionRange range = envPtr->exceptArrayPtr[index];

	    if ((range.type == CATCH_EXCEPTION_RANGE)
		    && (range.catchOffset == TCL_INDEX_NONE)) {
		enclosingCatch = 1;
		break;
	    }
	    index--;
	}
	if (!enclosingCatch) {
	    /*
	     * ... and there is no enclosing catch. Issue the maximally
	     * efficient exit instruction.
	     */

	    Tcl_DecrRefCount(returnOpts);
	    TclEmitOpcode(INST_DONE, envPtr);
	    TclAdjustStackDepth(1, envPtr);
	    return TCL_OK;
	}
    }

    /* Optimize [return -level 0 $x]. */
    Tcl_DictObjSize(NULL, returnOpts, &size);
    if (size == 0 && level == 0 && code == TCL_OK) {
	Tcl_DecrRefCount(returnOpts);
	return TCL_OK;
    }

    /*
     * Could not use the optimization, so we push the return options dict, and
     * emit the INST_RETURN_IMM instruction with code and level as operands.
     */

    CompileReturnInternal(envPtr, INST_RETURN_IMM, code, level, returnOpts);
    return TCL_OK;

  issueRuntimeReturn:
    /*
     * Assemble the option dictionary (as a list as that's good enough).
     */

    wordTokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (objc=1 ; objc<=numOptionWords ; objc++) {
	CompileWord(envPtr, wordTokenPtr, interp, objc);
	wordTokenPtr = TokenAfter(wordTokenPtr);
    }
    TclEmitInstInt4(INST_LIST, numOptionWords, envPtr);

    /*
     * Push the result.
     */

    if (explicitResult) {
	CompileWord(envPtr, wordTokenPtr, interp, numWords - 1);
    } else {
	PushStringLiteral(envPtr, "");
    }

    /*
     * Issue the RETURN itself.
     */

    TclEmitInvoke(envPtr, INST_RETURN_STK);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoExistsCmd`, lines 665–710. Retained reference `e139`.

```text
TclCompileInfoExistsCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int isScalar, localIndex;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PushVarNameWord(interp, tokenPtr, envPtr, 0, &localIndex, &isScalar, 1);

    /*
     * Emit instruction to check the variable for existence.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_EXIST_STK,			envPtr);
	} else {
	    TclEmitInstInt4(	INST_EXIST_SCALAR, localIndex,	envPtr);
	}
    } else {
	if (localIndex < 0) {
	    TclEmitOpcode(	INST_EXIST_ARRAY_STK,		envPtr);
	} else {
	    TclEmitInstInt4(	INST_EXIST_ARRAY, localIndex,	envPtr);
	}
    }

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoLevelCmd`, lines 713–744. Retained reference `e140`.

```text
TclCompileInfoLevelCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * Only compile [info level] without arguments or with a single argument.
     */

    if (parsePtr->numWords == 1) {
	/*
	 * Not much to do; we compile to a single instruction...
	 */

	TclEmitOpcode(		INST_INFO_LEVEL_NUM,		envPtr);
    } else if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    } else {
	DefineLineInformation;	/* TIP #280 */

	/*
	 * Compile the argument, then add the instruction to convert it into a
	 * list of arguments.
	 */

	CompileWord(envPtr, TokenAfter(parsePtr->tokenPtr), interp, 1);
	TclEmitOpcode(		INST_INFO_LEVEL_ARGS,		envPtr);
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileNamespaceCurrentCmd`, lines 1630–1651. Retained reference `e141`.

```text
TclCompileNamespaceCurrentCmd(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * Only compile [namespace current] without arguments.
     */

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * Not much to do; we compile to a single instruction...
     */

    TclEmitOpcode(		INST_NS_CURRENT,		envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileNamespaceCodeCmd`, lines 1654–1700. Retained reference `e142`.

```text
TclCompileNamespaceCodeCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * The specification of [namespace code] is rather shocking, in that it is
     * supposed to check if the argument is itself the result of [namespace
     * code] and not apply itself in that case. Which is excessively cautious,
     * but what the test suite checks for.
     */

    if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD || (tokenPtr[1].size > 20
	    && strncmp(tokenPtr[1].start, "::namespace inscope ", 20) == 0)) {
	/*
	 * Technically, we could just pass a literal '::namespace inscope '
	 * term through, but that's something which really shouldn't be
	 * occurring as something that the user writes so we'll just punt it.
	 */

	return TCL_ERROR;
    }

    /*
     * Now we can compile using the same strategy as [namespace code]'s normal
     * implementation does internally. Note that we can't bind the namespace
     * name directly here, because TclOO plays complex games with namespaces;
     * the value needs to be determined at runtime for safety.
     */

    PushStringLiteral(envPtr,		"::namespace");
    PushStringLiteral(envPtr,		"inscope");
    TclEmitOpcode(		INST_NS_CURRENT,	envPtr);
    CompileWord(envPtr,		tokenPtr,		interp, 1);
    TclEmitInstInt4(		INST_LIST, 4,		envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileNamespaceOriginCmd`, lines 1703–1721. Retained reference `e143`.

```text
TclCompileNamespaceOriginCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    CompileWord(envPtr,	tokenPtr,			interp, 1);
    TclEmitOpcode(	INST_ORIGIN_COMMAND,		envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoCommandsCmd`, lines 580–638. Retained reference `e144`.

```text
TclCompileInfoCommandsCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Obj *objPtr;
    const char *bytes;

    /*
     * We require one compile-time known argument for the case we can compile.
     */

    if (parsePtr->numWords == 1) {
	return TclCompileBasic0ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    } else if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);
    if (!TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	goto notCompilable;
    }
    bytes = TclGetString(objPtr);

    /*
     * We require that the argument start with "::" and not have any of "*\[?"
     * in it. (Theoretically, we should look in only the final component, but
     * the difference is so slight given current naming practices.)
     */

    if (bytes[0] != ':' || bytes[1] != ':' || !TclMatchIsTrivial(bytes)) {
	goto notCompilable;
    }
    Tcl_DecrRefCount(objPtr);

    /*
     * Confirmed as a literal that will not frighten the horses. Compile.
     * The result must be made into a list.
     */

    /* TODO: Just push the known value */
    CompileWord(envPtr, tokenPtr,		interp, 1);
    TclEmitOpcode(	INST_RESOLVE_COMMAND,	envPtr);
    TclEmitOpcode(	INST_DUP,		envPtr);
    TclEmitOpcode(	INST_STR_LEN,		envPtr);
    TclEmitInstInt1(	INST_JUMP_FALSE1, 7,	envPtr);
    TclEmitInstInt4(	INST_LIST, 1,		envPtr);
    return TCL_OK;

  notCompilable:
    Tcl_DecrRefCount(objPtr);
    return TclCompileBasic1ArgCmd(interp, parsePtr, cmdPtr, envPtr);
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileRegexpCmd`, lines 1918–2059. Retained reference `e145`.

```text
TclCompileRegexpCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;	/* Pointer to the Tcl_Token representing the
				 * parse of the RE or string. */
    size_t len;
    int i, nocase, exact, sawLast, simple;
    const char *str;

    /*
     * We are only interested in compiling simple regexp cases. Currently
     * supported compile cases are:
     *   regexp ?-nocase? ?--? staticString $var
     *   regexp ?-nocase? ?--? {^staticString$} $var
     */

    if ((int)parsePtr->numWords < 3) {
	return TCL_ERROR;
    }

    simple = 0;
    nocase = 0;
    sawLast = 0;
    varTokenPtr = parsePtr->tokenPtr;

    /*
     * We only look for -nocase and -- as options. Everything else gets pushed
     * to runtime execution. This is different than regexp's runtime option
     * handling, but satisfies our stricter needs.
     */

    for (i = 1; i < (int)parsePtr->numWords - 2; i++) {
	varTokenPtr = TokenAfter(varTokenPtr);
	if (varTokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    /*
	     * Not a simple string, so punt to runtime.
	     */

	    return TCL_ERROR;
	}
	str = varTokenPtr[1].start;
	len = varTokenPtr[1].size;
	if ((len == 2) && (str[0] == '-') && (str[1] == '-')) {
	    sawLast++;
	    i++;
	    break;
	} else if ((len > 1) && (strncmp(str, "-nocase", len) == 0)) {
	    nocase = 1;
	} else {
	    /*
	     * Not an option we recognize.
	     */

	    return TCL_ERROR;
	}
    }

    if (((int)parsePtr->numWords - i) != 2) {
	/*
	 * We don't support capturing to variables.
	 */

	return TCL_ERROR;
    }

    /*
     * Get the regexp string. If it is not a simple string or can't be
     * converted to a glob pattern, push the word for the INST_REGEXP.
     * Keep changes here in sync with TclCompileSwitchCmd Switch_Regexp.
     */

    varTokenPtr = TokenAfter(varTokenPtr);

    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	Tcl_DString ds;

	str = varTokenPtr[1].start;
	len = varTokenPtr[1].size;

	/*
	 * If it has a '-', it could be an incorrectly formed regexp command.
	 */

	if ((*str == '-') && !sawLast) {
	    return TCL_ERROR;
	}

	/*
	 * Note: do not optimize for len == 0, as error should be generated
	 * at runtime if operand is not a resolvable variable.
	 * Bug https://core.tcl-lang.org/tcl/info/cb03e57a7b24d22c
	 */

	/*
	 * Attempt to convert pattern to glob.  If successful, push the
	 * converted pattern as a literal.
	 */

	if (TclReToGlob(NULL, varTokenPtr[1].start, len, &ds, &exact, NULL)
		== TCL_OK) {
	    simple = 1;
	    PushLiteral(envPtr, Tcl_DStringValue(&ds),Tcl_DStringLength(&ds));
	    Tcl_DStringFree(&ds);
	}
    }

    if (!simple) {
	CompileWord(envPtr, varTokenPtr, interp, (int)parsePtr->numWords - 2);
    }

    /*
     * Push the string arg.
     */

    varTokenPtr = TokenAfter(varTokenPtr);
    CompileWord(envPtr, varTokenPtr, interp, (int)parsePtr->numWords - 1);

    if (simple) {
	if (exact && !nocase) {
	    TclEmitOpcode(	INST_STR_EQ,			envPtr);
	} else {
	    TclEmitInstInt1(	INST_STR_MATCH, nocase,		envPtr);
	}
    } else {
	/*
	 * Pass correct RE compile flags.  We use only Int1 (8-bit), but
	 * that handles all the flags we want to pass.
	 * Don't use TCL_REG_NOSUB as we may have backrefs.
	 */

	int cflags = TCL_REG_ADVANCED | (nocase ? TCL_REG_NOCASE : 0);

	TclEmitInstInt1(	INST_REGEXP, cflags,		envPtr);
    }

    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileObjectNextCmd`, lines 2788–2809. Retained reference `e146`.

```text
TclCompileObjectNextCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    int i;

    if ((int)parsePtr->numWords > 255) {
	return TCL_ERROR;
    }

    for (i=0 ; i<(int)parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    TclEmitInstInt1(	INST_TCLOO_NEXT, i,		envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileObjectNextToCmd`, lines 2812–2833. Retained reference `e147`.

```text
TclCompileObjectNextToCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    int i;

    if ((int)parsePtr->numWords < 2 || (int)parsePtr->numWords > 255) {
	return TCL_ERROR;
    }

    for (i=0 ; i<(int)parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    TclEmitInstInt1(	INST_TCLOO_NEXT_CLASS, i,	envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileObjectSelfCmd`, lines 2836–2895. Retained reference `e148`.

```text
TclCompileObjectSelfCmd(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * We only handle [self] and [self object] (which is the same operation).
     * These are the only very common operations on [self] for which
     * bytecoding is at all reasonable.
     */

    if (parsePtr->numWords == 1) {
	goto compileSelfObject;
    } else if (parsePtr->numWords == 2) {
	Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr), *subcmd;

	if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD || tokenPtr[1].size==0) {
	    return TCL_ERROR;
	}

	subcmd = tokenPtr + 1;
	if (strncmp(subcmd->start, "object", subcmd->size) == 0) {
	    goto compileSelfObject;
	} else if (strncmp(subcmd->start, "namespace", subcmd->size) == 0) {
	    goto compileSelfNamespace;
	}
    }

    /*
     * Can't compile; handle with runtime call.
     */

    return TCL_ERROR;

  compileSelfObject:

    /*
     * This delegates the entire problem to a single opcode.
     */

    TclEmitOpcode(		INST_TCLOO_SELF,		envPtr);
    return TCL_OK;

  compileSelfNamespace:

    /*
     * This is formally only correct with TclOO methods as they are currently
     * implemented; it assumes that the current namespace is invariably when a
     * TclOO context is present is the object's namespace, and that's
     * technically only something that's a matter of current policy. But it
     * avoids creating another opcode, so that's all good!
     */

    TclEmitOpcode(		INST_TCLOO_SELF,		envPtr);
    TclEmitOpcode(		INST_POP,			envPtr);
    TclEmitOpcode(		INST_NS_CURRENT,		envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoObjectClassCmd`, lines 747–763. Retained reference `e149`.

```text
TclCompileInfoObjectClassCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    CompileWord(envPtr,		tokenPtr,		interp, 1);
    TclEmitOpcode(		INST_TCLOO_CLASS,	envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoObjectNamespaceCmd`, lines 801–817. Retained reference `e150`.

```text
TclCompileInfoObjectNamespaceCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    CompileWord(envPtr,		tokenPtr,		interp, 1);
    TclEmitOpcode(		INST_TCLOO_NS,		envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoObjectIsACmd`, lines 766–798. Retained reference `e151`.

```text
TclCompileInfoObjectIsACmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * We only handle [info object isa object <somevalue>]. The first three
     * words are compressed to a single token by the ensemble compilation
     * engine.
     */

    if (parsePtr->numWords != 3) {
	return TCL_ERROR;
    }
    if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD || tokenPtr[1].size < 1
	    || strncmp(tokenPtr[1].start, "object", tokenPtr[1].size)) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(tokenPtr);

    /*
     * Issue the code.
     */

    CompileWord(envPtr,		tokenPtr,		interp, 2);
    TclEmitOpcode(		INST_TCLOO_IS_OBJECT,	envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `TclCompileSetCmd`, lines 124–195. Retained reference `e152`.

```text
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

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `TclCompileUnsetCmd`, lines 3618–3735. Retained reference `e153`.

```text
TclCompileUnsetCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;
    int isScalar, localIndex, flags = 1, i, varCount = 0, haveFlags = 0;

    /* TODO: Consider support for compiling expanded args. */

    /*
     * Verify that all words - except the first non-option one - are known at
     * compile time so that we can handle them without needing to do a nasty
     * push/rotate. [Bug 3970f54c4e]
     */

    for (i=1,varTokenPtr=parsePtr->tokenPtr ; i<(int)parsePtr->numWords ; i++) {
	Tcl_Obj *leadingWord;

	TclNewObj(leadingWord);
	varTokenPtr = TokenAfter(varTokenPtr);
	if (!TclWordKnownAtCompileTime(varTokenPtr, leadingWord)) {
	    TclDecrRefCount(leadingWord);

	    /*
	     * We can tolerate non-trivial substitutions in the first variable
	     * to be unset. If a '--' or '-nocomplain' was present, anything
	     * goes in that one place! (All subsequent variable names must be
	     * constants since we don't want to have to push them all first.)
	     */

	    if (varCount == 0) {
		if (haveFlags) {
		    continue;
		}

		/*
		 * In fact, we're OK as long as we're the first argument *and*
		 * we provably don't start with a '-'. If that is true, then
		 * even if everything else is varying, we still can't be a
		 * flag. Otherwise we'll spill to runtime to place a limit on
		 * the trickiness.
		 */

		if (varTokenPtr->type == TCL_TOKEN_WORD
			&& varTokenPtr[1].type == TCL_TOKEN_TEXT
			&& varTokenPtr[1].size > 0
			&& varTokenPtr[1].start[0] != '-') {
		    continue;
		}
	    }
	    return TCL_ERROR;
	}
	if (varCount == 0) {
	    const char *bytes;
	    Tcl_Size len;

	    bytes = TclGetStringFromObj(leadingWord, &len);
	    if (i == 1 && len == 11 && !strncmp("-nocomplain", bytes, 11)) {
		flags = 0;
		haveFlags++;
	    } else if (i == (2 - flags) && len == 2 && !strncmp("--", bytes, 2)) {
		haveFlags++;
	    } else {
		varCount++;
	    }
	} else {
	    varCount++;
	}
	TclDecrRefCount(leadingWord);
    }

    /*
     * Issue instructions to unset each of the named variables.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (i=0; i<haveFlags;i++) {
	varTokenPtr = TokenAfter(varTokenPtr);
    }
    for (i=1+haveFlags ; i<(int)parsePtr->numWords ; i++) {
	/*
	 * Decide if we can use a frame slot for the var/array name or if we
	 * need to emit code to compute and push the name at runtime. We use a
	 * frame slot (entry in the array of local vars) if we are compiling a
	 * procedure body and if the name is simple text that does not include
	 * namespace qualifiers.
	 */

	PushVarNameWord(interp, varTokenPtr, envPtr, 0,
		&localIndex, &isScalar, i);

	/*
	 * Emit instructions to unset the variable.
	 */

	if (isScalar) {
	    if (localIndex < 0) {
		OP1(	UNSET_STK, flags);
	    } else {
		OP14(	UNSET_SCALAR, flags, localIndex);
	    }
	} else {
	    if (localIndex < 0) {
		OP1(	UNSET_ARRAY_STK, flags);
	    } else {
		OP14(	UNSET_ARRAY, flags, localIndex);
	    }
	}

	varTokenPtr = TokenAfter(varTokenPtr);
    }
    PUSH("");
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `TclCompileTailcallCmd`, lines 2655–2680. Retained reference `e154`.

```text
TclCompileTailcallCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    int i;

    if (parsePtr->numWords < 2 || parsePtr->numWords >= 256
	    || envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    /* make room for the nsObjPtr */
    /* TODO: Doesn't this have to be a known value? */
    CompileWord(envPtr, tokenPtr, interp, 0);
    for (i=1 ; i<(int)parsePtr->numWords ; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, i);
    }
    TclEmitInstInt1(	INST_TAILCALL, (int)parsePtr->numWords,	envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `TclCompileYieldCmd`, lines 3933–3954. Retained reference `e155`.

```text
TclCompileYieldCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    if (parsePtr->numWords < 1 || parsePtr->numWords > 2) {
	return TCL_ERROR;
    }

    if (parsePtr->numWords == 1) {
	PUSH("");
    } else {
	DefineLineInformation;	/* TIP #280 */
	Tcl_Token *valueTokenPtr = TokenAfter(parsePtr->tokenPtr);

	CompileWord(envPtr, valueTokenPtr, interp, 1);
    }
    OP(		YIELD);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `TclCompileYieldToCmd`, lines 3975–3998. Retained reference `e156`.

```text
TclCompileYieldToCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);
    int i;

    if ((int)parsePtr->numWords < 2) {
	return TCL_ERROR;
    }

    OP(		NS_CURRENT);
    for (i = 1 ; i < (int)parsePtr->numWords ; i++) {
	CompileWord(envPtr, tokenPtr, interp, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    OP4(	LIST, i);
    OP(		YIELD_TO_INVOKE);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringEqualCmd`, lines 319–347. Retained reference `e157`.

```text
TclCompileStringEqualCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    /*
     * We don't support any flags; the bytecode isn't that sophisticated.
     */

    if (parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    /*
     * Push the two operands onto the stack and then the test.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);
    tokenPtr = TokenAfter(tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 2);
    TclEmitOpcode(INST_STR_EQ, envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringLenCmd`, lines 869–905. Retained reference `e158`.

```text
TclCompileStringLenCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Obj *objPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(objPtr);
    if (TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	/*
	 * Here someone is asking for the length of a static string (or
	 * something with backslashes). Just push the actual character (not
	 * byte) length.
	 */

	char buf[TCL_INTEGER_SPACE];
	size_t len = Tcl_GetCharLength(objPtr);

	len = snprintf(buf, sizeof(buf), "%" TCL_Z_MODIFIER "u", len);
	PushLiteral(envPtr, buf, len);
    } else {
	SetLineInformation(1);
	CompileTokens(envPtr, tokenPtr, interp);
	TclEmitOpcode(INST_STR_LEN, envPtr);
    }
    TclDecrRefCount(objPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringMatchCmd`, lines 787–866. Retained reference `e159`.

```text
TclCompileStringMatchCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
	size_t length;
    int i, exactMatch = 0, nocase = 0;
    const char *str;

    if (parsePtr->numWords < 3 || parsePtr->numWords > 4) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Check if we have a -nocase flag.
     */

    if (parsePtr->numWords == 4) {
	if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    return TclCompileBasic3ArgCmd(interp, parsePtr, cmdPtr, envPtr);
	}
	str = tokenPtr[1].start;
	length = tokenPtr[1].size;
	if ((length <= 1) || strncmp(str, "-nocase", length)) {
	    /*
	     * Fail at run time, not in compilation.
	     */

	    return TclCompileBasic3ArgCmd(interp, parsePtr, cmdPtr, envPtr);
	}
	nocase = 1;
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Push the strings to match against each other.
     */

    for (i = 0; i < 2; i++) {
	if (tokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    str = tokenPtr[1].start;
	    length = tokenPtr[1].size;
	    if (!nocase && (i == 0)) {
		/*
		 * Trivial matches can be done by 'string equal'. If -nocase
		 * was specified, we can't do this because INST_STR_EQ has no
		 * support for nocase.
		 */

		Tcl_Obj *copy = Tcl_NewStringObj(str, length);

		Tcl_IncrRefCount(copy);
		exactMatch = TclMatchIsTrivial(TclGetString(copy));
		TclDecrRefCount(copy);
	    }
	    PushLiteral(envPtr, str, length);
	} else {
	    SetLineInformation(i+1+nocase);
	    CompileTokens(envPtr, tokenPtr, interp);
	}
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Push the matcher.
     */

    if (exactMatch) {
	TclEmitOpcode(INST_STR_EQ, envPtr);
    } else {
	TclEmitInstInt1(INST_STR_MATCH, nocase, envPtr);
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringTrimCmd`, lines 1298–1322. Retained reference `e160`.

```text
TclCompileStringTrimCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2 && parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr,			interp, 1);
    if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr,			interp, 2);
    } else {
	PushLiteral(envPtr, tclDefaultTrimSet, strlen(tclDefaultTrimSet));
    }
    OP(			STR_TRIM);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringTrimLCmd`, lines 1244–1268. Retained reference `e161`.

```text
TclCompileStringTrimLCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2 && parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr,			interp, 1);
    if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr,			interp, 2);
    } else {
	PushLiteral(envPtr, tclDefaultTrimSet, strlen(tclDefaultTrimSet));
    }
    OP(			STR_TRIM_LEFT);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringTrimRCmd`, lines 1271–1295. Retained reference `e162`.

```text
TclCompileStringTrimRCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2 && parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr,			interp, 1);
    if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr,			interp, 2);
    } else {
	PushLiteral(envPtr, tclDefaultTrimSet, strlen(tclDefaultTrimSet));
    }
    OP(			STR_TRIM_RIGHT);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `CompileUnaryOpCmd`, lines 4019–4035. Retained reference `e163`.

```text
CompileUnaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    CompileWord(envPtr, tokenPtr, interp, 1);
    TclEmitOpcode(instruction, envPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `CompileAssociativeBinaryOpCmd`, lines 4060–4092. Retained reference `e164`.

```text
CompileAssociativeBinaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    const char *identity,
    int instruction,
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    Tcl_Size words;

    /* TODO: Consider support for compiling expanded args. */
    for (words=1 ; words<parsePtr->numWords ; words++) {
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, words);
    }
    if (parsePtr->numWords <= 2) {
	PushLiteral(envPtr, identity, -1);
	words++;
    }
    if (words > 3) {
	/*
	 * Reverse order of arguments to get precise agreement with [expr] in
	 * calculations, including roundoff errors.
	 */

	OP4(	REVERSE, words-1);
    }
    while (--words > 1) {
	TclEmitOpcode(instruction, envPtr);
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `CompileStrictlyBinaryOpCmd`, lines 4114–4125. Retained reference `e165`.

```text
CompileStrictlyBinaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    if (parsePtr->numWords != 3) {
	return TCL_ERROR;
    }
    return CompileAssociativeBinaryOpCmd(interp, parsePtr,
	    NULL, instruction, envPtr);
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompCmdsSZ.c`, function `CompileComparisonOpCmd`, lines 4146–4201. Retained reference `e166`.

```text
CompileComparisonOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    /* TODO: Consider support for compiling expanded args. */
    if ((int)parsePtr->numWords < 3) {
	PUSH("1");
    } else if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(parsePtr->tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 1);
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 2);
	TclEmitOpcode(instruction, envPtr);
    } else if (envPtr->procPtr == NULL) {
	/*
	 * No local variable space!
	 */

	return TCL_ERROR;
    } else {
	int tmpIndex = AnonymousLocal(envPtr);
	Tcl_Size words;

	tokenPtr = TokenAfter(parsePtr->tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 1);
	tokenPtr = TokenAfter(tokenPtr);
	CompileWord(envPtr, tokenPtr, interp, 2);
	STORE(tmpIndex);
	TclEmitOpcode(instruction, envPtr);
	for (words=3 ; words<parsePtr->numWords ;) {
	    LOAD(tmpIndex);
	    tokenPtr = TokenAfter(tokenPtr);
	    CompileWord(envPtr, tokenPtr, interp, words);
	    if (++words < parsePtr->numWords) {
		STORE(tmpIndex);
	    }
	    TclEmitOpcode(instruction, envPtr);
	}
	for (; words>3 ; words--) {
	    OP(	BITAND);
	}

	/*
	 * Drop the value from the temp variable; retaining that reference
	 * might be expensive elsewhere.
	 */

	OP14(	UNSET_SCALAR, 0, tmpIndex);
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompile.c`, function `TclCompileTokens`, lines 2416–2610. Retained reference `e167`.

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

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompile.c`, function `TclFindCompiledLocal`, lines 3020–3115. Retained reference `e168`.

```text
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

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompile.c`, function `CompileCmdLiteral`, lines 1814–1836. Retained reference `e169`.

```text
CompileCmdLiteral(
    Tcl_Interp *interp,
    Tcl_Obj *cmdObj,
    CompileEnv *envPtr)
{
    const char *bytes;
    Command *cmdPtr;
    int cmdLitIdx, extraLiteralFlags = LITERAL_CMD_NAME;
    Tcl_Size length;

    cmdPtr = (Command *) Tcl_GetCommandFromObj(interp, cmdObj);
    if ((cmdPtr != NULL) && (cmdPtr->flags & CMD_VIA_RESOLVER)) {
	extraLiteralFlags |= LITERAL_UNSHARED;
    }

    bytes = TclGetStringFromObj(cmdObj, &length);
    cmdLitIdx = TclRegisterLiteral(envPtr, bytes, length, extraLiteralFlags);

    if (cmdPtr && TclRoutineHasName(cmdPtr)) {
	TclSetCmdNameObj(interp, TclFetchLiteral(envPtr, cmdLitIdx), cmdPtr);
    }
    TclEmitPush(cmdLitIdx, envPtr);
}

```

tcl9.0 9.0.4, revision `pinned Tcl release 9.0.4`, `generic/tclCompile.c`, function `TclCompileInvocation`, lines 1839–1881. Retained reference `e170`.

```text
TclCompileInvocation(
    Tcl_Interp *interp,
    Tcl_Token *tokenPtr,
    Tcl_Obj *cmdObj,
    size_t numWords,
    CompileEnv *envPtr)
{
    DefineLineInformation;
    size_t wordIdx = 0;
    int depth = TclGetStackDepth(envPtr);

    if (cmdObj) {
	CompileCmdLiteral(interp, cmdObj, envPtr);
	wordIdx = 1;
	tokenPtr = TokenAfter(tokenPtr);
    }

    for (; wordIdx < numWords; wordIdx++, tokenPtr = TokenAfter(tokenPtr)) {
	int objIdx;

	SetLineInformation(wordIdx);

	if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    CompileTokens(envPtr, tokenPtr, interp);
	    continue;
	}

	objIdx = TclRegisterLiteral(envPtr,
		tokenPtr[1].start, tokenPtr[1].size, 0);
	if (envPtr->clNext) {
	    TclContinuationsEnterDerived(TclFetchLiteral(envPtr, objIdx),
		    tokenPtr[1].start - envPtr->source, envPtr->clNext);
	}
	TclEmitPush(objIdx, envPtr);
    }

    if (wordIdx <= 255) {
	TclEmitInvoke(envPtr, INST_INVOKE_STK1, wordIdx);
    } else {
	TclEmitInvoke(envPtr, INST_INVOKE_STK4, wordIdx);
    }
    TclCheckStackDepth(depth+1, envPtr);
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileAppendCmd`, lines 128–235. Retained reference `e171`.

```text
TclCompileAppendCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    int isScalar;
    Tcl_LVTIndex localIndex;
    Tcl_Size i, numWords = parsePtr->numWords;

    /* TODO: Consider support for compiling expanded args. */
    if (numWords == 1 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    } else if (numWords == 2) {
	/*
	 * append varName == set varName
	 */

	return TclCompileSetCmd(interp, parsePtr, cmdPtr, envPtr);
    } else if (numWords > 3) {
	/*
	 * APPEND instructions currently only handle one value, but we can
	 * handle some multi-value cases by stringing them together.
	 */

	goto appendMultiple;
    }

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
     * We are doing an assignment, otherwise TclCompileSetCmd was called, so
     * push the new value. This will need to be extended to push a value for
     * each argument.
     */

    valueTokenPtr = TokenAfter(varTokenPtr);
    PUSH_TOKEN(			valueTokenPtr, 2);

    /*
     * Emit instructions to set/get the variable.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    OP(			APPEND_STK);
	} else {
	    OP4(		APPEND_SCALAR, localIndex);
	}
    } else {
	if (localIndex < 0) {
	    OP(			APPEND_ARRAY_STK);
	} else {
	    OP4(		APPEND_ARRAY, localIndex);
	}
    }

    return TCL_OK;

  appendMultiple:
    /*
     * Can only handle the case where we are appending to a local scalar when
     * there are multiple values to append.  Fortunately, this is common.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    localIndex = TclLocalScalarFromToken(varTokenPtr, envPtr);
    if (OutOfUintRange(localIndex)) {
	return TCL_ERROR;
    }

    /*
     * Definitely appending to a local scalar; generate the words and append
     * them.
     */

    valueTokenPtr = TokenAfter(varTokenPtr);
    for (i = 2 ; i < numWords ; i++) {
	PUSH_TOKEN(		valueTokenPtr, i);
	valueTokenPtr = TokenAfter(valueTokenPtr);
    }
    OP4(			REVERSE, numWords - 2);
    for (i = 2 ; i < numWords ;) {
	OP4(			APPEND_SCALAR, localIndex);
	if (++i < numWords) {
	    OP(			POP);
	}
    }

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileConcatCmd`, lines 854–911. Retained reference `e172`.

```text
TclCompileConcatCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Obj *objPtr, *listObj, **objs;
    Tcl_Size len, i, numWords = parsePtr->numWords;
    Tcl_Token *tokenPtr;

    /* TODO: Consider compiling expansion case. */
    if (numWords == 1) {
	/*
	 * [concat] without arguments just pushes an empty object.
	 */

	PUSH(			"");
	return TCL_OK;
    } else if (OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /*
     * Test if all arguments are compile-time known. If they are, we can
     * implement with a simple push of a literal.
     */

    TclNewObj(listObj);
    for (i = 1, tokenPtr = parsePtr->tokenPtr; i < numWords; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	TclNewObj(objPtr);
	if (!TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	    Tcl_BounceRefCount(objPtr);
	    Tcl_BounceRefCount(listObj);
	    goto runtimeConcat;
	}
	(void) Tcl_ListObjAppendElement(NULL, listObj, objPtr);
    }

    TclListObjGetElements(NULL, listObj, &len, &objs);
    PUSH_OBJ(			Tcl_ConcatObj(len, objs));
    Tcl_BounceRefCount(listObj);
    return TCL_OK;

    /*
     * General case: do the concatenation at runtime.
     */

  runtimeConcat:
    for (i = 1, tokenPtr = parsePtr->tokenPtr; i < numWords; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	PUSH_TOKEN(		tokenPtr, i);
    }
    OP4(			CONCAT_STK, i - 1);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileErrorCmd`, lines 2558–2608. Retained reference `e173`.

```text
TclCompileErrorCmd(
    Tcl_Interp *interp,		/* Used for context. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Size numWords = parsePtr->numWords;

    /*
     * General syntax: [error message ?errorInfo? ?errorCode?]
     */

    if (numWords < 2 || numWords > 4) {
	return TCL_ERROR;
    }

    /*
     * Handle the message.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PUSH_TOKEN(			tokenPtr, 1);

    /*
     * Construct the options. Note that -code and -level are not here.
     */

    PUSH(			"");
    if (numWords > 2) {
	tokenPtr = TokenAfter(tokenPtr);
	PUSH(			"-errorinfo");
	PUSH_TOKEN(		tokenPtr, 2);
	OP(			DICT_PUT);
	if (numWords > 3) {
	    tokenPtr = TokenAfter(tokenPtr);
	    PUSH(		"-errorcode");
	    PUSH_TOKEN(		tokenPtr, 3);
	    OP(			DICT_PUT);
	}
    }

    /*
     * Issue the error via 'returnImm error 0'.
     */

    OP44(			RETURN_IMM, TCL_ERROR, 0);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileBreakCmd`, lines 533–569. Retained reference `e174`.

```text
TclCompileBreakCmd(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    ExceptionRange *rangePtr;
    ExceptionAux *auxPtr;

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * Find the innermost exception range that contains this command.
     */

    rangePtr = TclGetInnermostExceptionRange(envPtr, TCL_BREAK, &auxPtr);
    if (rangePtr && rangePtr->type == LOOP_EXCEPTION_RANGE) {
	/*
	 * Found the target! No need for a nasty INST_BREAK here.
	 */

	TclCleanupStackForBreakContinue(envPtr, auxPtr);
	TclAddLoopBreakFixup(envPtr, auxPtr);
    } else {
	/*
	 * Emit a real break.
	 */

	OP(			BREAK);
    }
    STKDELTA(+1);

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileContinueCmd`, lines 1010–1051. Retained reference `e175`.

```text
TclCompileContinueCmd(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    ExceptionRange *rangePtr;
    ExceptionAux *auxPtr;

    /*
     * There should be no argument after the "continue".
     */

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * See if we can find a valid continueOffset (i.e., not -1) in the
     * innermost containing exception range.
     */

    rangePtr = TclGetInnermostExceptionRange(envPtr, TCL_CONTINUE, &auxPtr);
    if (rangePtr && rangePtr->type == LOOP_EXCEPTION_RANGE) {
	/*
	 * Found the target! No need for a nasty INST_CONTINUE here.
	 */

	TclCleanupStackForBreakContinue(envPtr, auxPtr);
	TclAddLoopContinueFixup(envPtr, auxPtr);
    } else {
	/*
	 * Emit a real continue.
	 */

	OP(			CONTINUE);
    }
    STKDELTA(+1);

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictAppendCmd`, lines 1597–1651. Retained reference `e176`.

```text
TclCompileDictAppendCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;
    Tcl_LVTIndex dictVarIndex;

    /*
     * There must be at least two argument after the command. And we impose an
     * (arbitrary) safe limit; anyone exceeding it should stop worrying about
     * speed quite so much. ;-)
     * TODO: Raise the limit...
     */

    /* TODO: Consider support for compiling expanded args. */
    if (numWords < 4 || numWords > 100) {
	return TCL_ERROR;
    }

    /*
     * Get the index of the local variable that we will be working with.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = TclLocalScalarFromToken(tokenPtr, envPtr);
    if (OutOfUintRange(dictVarIndex)) {
	return TclCompileBasicMin2ArgCmd(interp, parsePtr,cmdPtr, envPtr);
    }

    /*
     * Produce the string to concatenate onto the dictionary entry.
     */

    tokenPtr = TokenAfter(tokenPtr);
    for (i=2 ; i<numWords ; i++) {
	PUSH_TOKEN(		tokenPtr, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    if (numWords > 4) {
	OP1(			STR_CONCAT1, numWords - 3);
    }

    /*
     * Do the concatenation.
     */

    OP4(			DICT_APPEND, dictVarIndex);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictLappendCmd`, lines 1654–1696. Retained reference `e177`.

```text
TclCompileDictLappendCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *keyTokenPtr, *valueTokenPtr;
    Tcl_LVTIndex dictVarIndex;

    /*
     * There must be three arguments after the command.
     */

    /* TODO: Consider support for compiling expanded args. */
    /* Probably not.  Why is INST_DICT_LAPPEND limited to one value? */
    if (parsePtr->numWords != 4) {
	return TCL_ERROR;
    }

    /*
     * Parse the arguments.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    keyTokenPtr = TokenAfter(varTokenPtr);
    valueTokenPtr = TokenAfter(keyTokenPtr);
    dictVarIndex = TclLocalScalarFromToken(varTokenPtr, envPtr);
    if (OutOfUintRange(dictVarIndex)) {
	return TclCompileBasic3ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }

    /*
     * Issue the implementation.
     */

    PUSH_TOKEN(			keyTokenPtr, 2);
    PUSH_TOKEN(			valueTokenPtr, 3);
    OP4(			DICT_LAPPEND, dictVarIndex);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictCreateCmd`, lines 1418–1487. Retained reference `e178`.

```text
TclCompileDictCreateCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *keyToken, *valueToken;
    Tcl_Obj *keyObj, *valueObj, *dictObj;
    Tcl_Size i, numWords = parsePtr->numWords;
    /* TODO: Consider support for compiling expanded args. */

    if ((numWords & 1) == 0 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /*
     * See if we can build the value at compile time...
     */

    keyToken = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(dictObj);
    for (i=1 ; i<numWords ; i+=2) {
	TclNewObj(keyObj);
	if (!TclWordKnownAtCompileTime(keyToken, keyObj)) {
	    Tcl_BounceRefCount(keyObj);
	    Tcl_BounceRefCount(dictObj);
	    goto nonConstant;
	}
	valueToken = TokenAfter(keyToken);
	TclNewObj(valueObj);
	if (!TclWordKnownAtCompileTime(valueToken, valueObj)) {
	    Tcl_BounceRefCount(keyObj);
	    Tcl_BounceRefCount(valueObj);
	    Tcl_BounceRefCount(dictObj);
	    goto nonConstant;
	}
	keyToken = TokenAfter(valueToken);
	Tcl_DictObjPut(NULL, dictObj, keyObj, valueObj);
	Tcl_BounceRefCount(keyObj);
    }

    /*
     * We did! Excellent. The "verifyDict" is to do type forcing.
     */

    PUSH_OBJ(			dictObj);
    OP(				DUP);
    OP(				DICT_VERIFY);
    return TCL_OK;

    /*
     * Otherwise, we've got to issue runtime code to do the building, which we
     * do by [dict set]ting into an unnamed local variable. This requires that
     * we are in a context with an LVT.
     */

  nonConstant:
    PUSH(			"");
    keyToken = TokenAfter(parsePtr->tokenPtr);
    for (i=1 ; i<numWords ; i+=2) {
	valueToken = TokenAfter(keyToken);
	PUSH_TOKEN(		keyToken, i);
	PUSH_TOKEN(		valueToken, i + 1);
	OP(			DICT_PUT);
	keyToken = TokenAfter(valueToken);
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictExistsCmd`, lines 1257–1289. Retained reference `e179`.

```text
TclCompileDictExistsCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;

    /*
     * There must be at least two arguments after the command (the single-arg
     * case is legal, but too special and magic for us to deal with here).
     */

    /* TODO: Consider support for compiling expanded args. */
    if (numWords < 3 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Now we do the code generation.
     */

    for (i=1 ; i<numWords ; i++) {
	PUSH_TOKEN(		tokenPtr, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    OP4(			DICT_EXISTS, numWords - 2);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictGetCmd`, lines 1192–1224. Retained reference `e180`.

```text
TclCompileDictGetCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;

    /*
     * There must be at least two arguments after the command (the single-arg
     * case is legal, but too special and magic for us to deal with here).
     */

    /* TODO: Consider support for compiling expanded args. */
    if (numWords < 3 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Only compile this because we need INST_DICT_GET anyway.
     */

    for (i=1 ; i<numWords ; i++) {
	PUSH_TOKEN(		tokenPtr, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    OP4(			DICT_GET, numWords - 2);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictGetWithDefaultCmd`, lines 1227–1254. Retained reference `e181`.

```text
TclCompileDictGetWithDefaultCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;

    /*
     * There must be at least three arguments after the command.
     */

    /* TODO: Consider support for compiling expanded args. */
    if (numWords < 4 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    for (i=1 ; i<numWords ; i++) {
	PUSH_TOKEN(		tokenPtr, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    OP4(			DICT_GET_DEF, numWords - 3);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictIncrCmd`, lines 1125–1189. Retained reference `e182`.

```text
TclCompileDictIncrCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *keyTokenPtr;
    Tcl_LVTIndex dictVarIndex;
    int incrAmount;

    /*
     * There must be at least two arguments after the command.
     */

    if (parsePtr->numWords < 3 || parsePtr->numWords > 4) {
	return TCL_ERROR;
    }
    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    keyTokenPtr = TokenAfter(varTokenPtr);

    /*
     * Parse the increment amount, if present.
     */

    if (parsePtr->numWords == 4) {
	Tcl_Token *incrTokenPtr = TokenAfter(keyTokenPtr);
	Tcl_Obj *intObj;
	int code;

	TclNewObj(intObj);
	if (!TclWordKnownAtCompileTime(incrTokenPtr, intObj)) {
	    Tcl_BounceRefCount(intObj);
	    return TclCompileBasic2Or3ArgCmd(interp, parsePtr,cmdPtr, envPtr);
	}
	code = TclGetIntFromObj(NULL, intObj, &incrAmount);
	Tcl_BounceRefCount(intObj);
	if (code != TCL_OK) {
	    return TclCompileBasic2Or3ArgCmd(interp, parsePtr,cmdPtr, envPtr);
	}
    } else {
	incrAmount = 1;
    }

    /*
     * The dictionary variable must be a local scalar that is knowable at
     * compile time; anything else exceeds the complexity of the opcode. So
     * discover what the index is.
     */

    dictVarIndex = TclLocalScalarFromToken(varTokenPtr, envPtr);
    if (OutOfUintRange(dictVarIndex)) {
	return TclCompileBasic2Or3ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }

    /*
     * Emit the key and the code to actually do the increment.
     */

    PUSH_TOKEN(			keyTokenPtr, 2);
    OP44(			DICT_INCR_IMM, incrAmount, dictVarIndex);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictMergeCmd`, lines 1490–1594. Retained reference `e183`.

```text
TclCompileDictMergeCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;
    Tcl_LVTIndex infoIndex;
    Tcl_ExceptionRange outLoop;
    Tcl_BytecodeLabel end;

    /*
     * Deal with some special edge cases. Note that in the case with one
     * argument, the only thing to do is to verify the dict-ness.
     */

    /* TODO: Consider support for compiling expanded args. (less likely) */
    if (numWords < 2) {
	PUSH(			"");
	return TCL_OK;
    } else if (numWords == 2) {
	tokenPtr = TokenAfter(parsePtr->tokenPtr);
	PUSH_TOKEN(		tokenPtr, 1);
	OP(			DUP);
	OP(			DICT_VERIFY);
	return TCL_OK;
    }

    /*
     * There's real merging work to do.
     *
     * Allocate some working space. This means we'll only ever compile this
     * command when there's an LVT present.
     */

    if (!EnvIsProc(envPtr)) {
	return TclCompileBasicMin2ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }
    infoIndex = AnonymousLocal(envPtr);

    /*
     * Get the first dictionary and verify that it is so.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PUSH_TOKEN(			tokenPtr, 1);
    OP(				DUP);
    OP(				DICT_VERIFY);

    /*
     * For each of the remaining dictionaries...
     */

    outLoop = MAKE_CATCH_RANGE();
    OP4(			BEGIN_CATCH, outLoop);
    CATCH_RANGE(outLoop) {
	for (i=2 ; i<numWords ; i++) {
	    Tcl_BytecodeLabel haveNext, noNext;
	    /*
	     * Get the dictionary, and merge its pairs into the first dict (using
	     * a small loop).
	     */

	    tokenPtr = TokenAfter(tokenPtr);
	    PUSH_TOKEN(		tokenPtr, i);
	    OP4(		DICT_FIRST, infoIndex);
	    FWDJUMP(		JUMP_TRUE, noNext);
	    BACKLABEL(	haveNext);
	    OP(			SWAP);
	    OP(			DICT_PUT);
	    OP4(		DICT_NEXT, infoIndex);
	    BACKJUMP(		JUMP_FALSE, haveNext);
	    FWDLABEL(	noNext);
	    OP(			POP);
	    OP(			POP);
	    OP14(		UNSET_SCALAR, 0, infoIndex);
	}
    }
    OP(				END_CATCH);

    /*
     * Clean up any state left over.
     */

    FWDJUMP(			JUMP, end);
    STKDELTA(-1);

    /*
     * If an exception happens when starting to iterate over the second (and
     * subsequent) dicts. This is strictly not necessary, but it is nice.
     */

    CATCH_TARGET(outLoop);
    OP(				PUSH_RETURN_OPTIONS);
    OP(				PUSH_RESULT);
    OP(				END_CATCH);
    OP14(			UNSET_SCALAR, 0, infoIndex);
    INVOKE(			RETURN_STK);
    FWDLABEL(		end);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictRemoveCmd`, lines 1330–1362. Retained reference `e184`.

```text
TclCompileDictRemoveCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Size i, numWords = parsePtr->numWords;
    Tcl_Token *tokenPtr;
    /* TODO: Consider support for compiling expanded args. */

    /*
     * Don't compile [dict remove $dict]; it's an edge case.
     */
    if (numWords <= 2 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    // Push starting dictionary
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PUSH_TOKEN(			tokenPtr, 1);

    // Push the keys, and remove them from the dictionary
    for (i=2; i<numWords; i++) {
	// Push key
	tokenPtr = TokenAfter(tokenPtr);
	PUSH_TOKEN(		tokenPtr, i);
	OP(			DICT_REMOVE);
    }

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictReplaceCmd`, lines 1292–1327. Retained reference `e185`.

```text
TclCompileDictReplaceCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Size i, numWords = parsePtr->numWords;
    Tcl_Token *tokenPtr;
    /* TODO: Consider support for compiling expanded args. */

    /*
     * Don't compile [dict replace $dict]; it's an edge case.
     */
    if (numWords <= 3 || OutOfUintRange(numWords) || (numWords % 1)) {
	return TCL_ERROR;
    }

    // Push starting dictionary
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PUSH_TOKEN(			tokenPtr, 1);

    // Push the keys and values, and add them to the dictionary
    for (i=2; i<numWords; i+=2) {
	// Push key
	tokenPtr = TokenAfter(tokenPtr);
	PUSH_TOKEN(		tokenPtr, i);
	// Push value
	tokenPtr = TokenAfter(tokenPtr);
	PUSH_TOKEN(		tokenPtr, i + 1);
	OP(			DICT_PUT);
    }

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictSetCmd`, lines 1072–1122. Retained reference `e186`.

```text
TclCompileDictSetCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;
    Tcl_LVTIndex dictVarIndex;
    Tcl_Token *varTokenPtr;
    /* TODO: Consider support for compiling expanded args. */

    /*
     * There must be at least one argument after the command.
     */

    if (numWords < 4 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /*
     * The dictionary variable must be a local scalar that is knowable at
     * compile time; anything else exceeds the complexity of the opcode. So
     * discover what the index is.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = TclLocalScalarFromToken(varTokenPtr, envPtr);
    if (OutOfUintRange(dictVarIndex)) {
	return TCL_ERROR;
    }

    /*
     * Remaining words (key path and value to set) can be handled normally.
     */

    tokenPtr = TokenAfter(varTokenPtr);
    for (i=2 ; i<numWords ; i++) {
	PUSH_TOKEN(		tokenPtr, i);
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Now emit the instruction to do the dict manipulation.
     */

    OP44(			DICT_SET, numWords - 3, dictVarIndex);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmds.c`, function `TclCompileDictUnsetCmd`, lines 1365–1415. Retained reference `e187`.

```text
TclCompileDictUnsetCmd(
    Tcl_Interp *interp,		/* Used for looking up stuff. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;
    Tcl_LVTIndex dictVarIndex;

    /*
     * There must be at least one argument after the variable name for us to
     * compile to bytecode.
     */

    /* TODO: Consider support for compiling expanded args. */
    if (numWords < 3 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /*
     * The dictionary variable must be a local scalar that is knowable at
     * compile time; anything else exceeds the complexity of the opcode. So
     * discover what the index is.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    dictVarIndex = TclLocalScalarFromToken(tokenPtr, envPtr);
    if (OutOfUintRange(dictVarIndex)) {
	return TclCompileBasicMin2ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    }

    /*
     * Remaining words (the key path) can be handled normally.
     */

    for (i=2 ; i<numWords ; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	PUSH_TOKEN(		tokenPtr, i);
    }

    /*
     * Now emit the instruction to do the dict manipulation.
     */

    OP44(			DICT_UNSET, numWords - 2, dictVarIndex);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileLappendCmd`, lines 812–975. Retained reference `e188`.

```text
TclCompileLappendCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    Tcl_Size numWords = parsePtr->numWords, i;
    int isScalar;
    Tcl_LVTIndex localIndex;

    if (numWords < 2 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /*
     * Decide if we can use a frame slot for the var/array name or if we
     * need to emit code to compute and push the name at runtime. We use a
     * frame slot (entry in the array of local vars) if we are compiling a
     * procedure body and if the name is simple text that does not include
     * namespace qualifiers.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    if (varTokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	/* Cannot compile if we don't know the variable properly! */
	return TCL_ERROR;
    }
    PushVarNameWord(varTokenPtr, 0, &localIndex, &isScalar, 1);
    if (OutOfUintRangeUpper(localIndex)) {
	return TCL_ERROR;
    }

    if (numWords != 3) {
	goto lappendMultiple;
    }

    /*
     * We are doing an assignment, so push the new value.
     */

    valueTokenPtr = TokenAfter(varTokenPtr);
    PUSH_TOKEN(			valueTokenPtr, 2);
    if (valueTokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	/*
	 * Special case: appending a single expanded list. MUST force a drop of
	 * the string representation at this point because INST_LAPPEND_LIST*
	 * might use it directly.
	 */
	OP44(			LIST_RANGE_IMM, 0, TCL_INDEX_END);
	goto lappendList;
    } else if (!EnvHasLVT(envPtr)) {
	/*
	 * The weird cluster of bugs around INST_LAPPEND_STK without a LVT
	 * ought to be sorted out. INST_LAPPEND_LIST_STK does the right thing.
	 */
	OP4(			LIST, 1);
	goto lappendList;
    }

    /*
     * Emit instructions to append the item to the variable.
     *
     * The *_STK opcodes should be refactored to make better use of existing
     * LOAD/STORE instructions.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    OP(			LAPPEND_STK);
	} else {
	    OP4(		LAPPEND_SCALAR, localIndex);
	}
    } else {
	if (localIndex < 0) {
	    OP(			LAPPEND_ARRAY_STK);
	} else {
	    OP4(		LAPPEND_ARRAY, localIndex);
	}
    }
    return TCL_OK;

    /*
     * In the cases where there's not a single value to append to the list in
     * the variable, we use a different strategy. This is to turn the arguments
     * into a list and then append that list's elements. The downside is that
     * this allocates a temporary working list, but at least it simplifies the
     * code issuing a lot.
     */

  lappendMultiple:

    /*
     * Concatenate all our remaining arguments into a list. This is slightly
     * complicated because we also handle expansion.
     */

    if (numWords == 2) {
	PUSH(			"");
    } else {
	Tcl_Size build = 0;
	int concat = 0;

	valueTokenPtr = TokenAfter(varTokenPtr);
	for (i = 2; i < numWords; i++) {
	    if (valueTokenPtr->type == TCL_TOKEN_EXPAND_WORD && build > 0) {
		OP4(		LIST, build);
		if (concat) {
		    OP(		LIST_CONCAT);
		}
		build = 0;
		concat = 1;
	    }
	    PUSH_TOKEN(		valueTokenPtr, i);
	    if (valueTokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
		if (concat) {
		    OP(		LIST_CONCAT);
		} else {
		    concat = 1;
		}
	    } else {
		build++;
	    }
	    if (build > LIST_CONCAT_THRESHOLD) {
		OP4(		LIST, build);
		if (concat) {
		    OP(		LIST_CONCAT);
		}
		build = 0;
		concat = 1;
	    }
	    valueTokenPtr = TokenAfter(valueTokenPtr);
	}
	if (build > 0) {
	    OP4(		LIST, build);
	    if (concat) {
		OP(		LIST_CONCAT);
	    }
	}
    }

    /*
     * Append the items of the list to the variable. The implementation of
     * these opcodes handles all the special cases that [lappend] knows about.
     */

  lappendList:
    if (isScalar) {
	if (localIndex < 0) {
	    OP(			LAPPEND_LIST_STK);
	} else {
	    OP4(		LAPPEND_LIST, localIndex);
	}
    } else {
	if (localIndex < 0) {
	    OP(			LAPPEND_LIST_ARRAY_STK);
	} else {
	    OP4(		LAPPEND_LIST_ARRAY, localIndex);
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileIncrCmd`, lines 425–509. Retained reference `e189`.

```text
TclCompileIncrCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *incrTokenPtr;
    int isScalar, haveImmValue;
    Tcl_LVTIndex localIndex;
    Tcl_WideInt immValue;

    if ((parsePtr->numWords != 2) && (parsePtr->numWords != 3)) {
	return TCL_ERROR;
    }

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    PushVarNameWord(varTokenPtr, 0, &localIndex, &isScalar, 1);
    if (OutOfUintRangeUpper(localIndex)) {
	return TCL_ERROR;
    }

    /*
     * If an increment is given, push it, but see first if it's a small
     * integer.
     */

    haveImmValue = 0;
    immValue = 1;
    if (parsePtr->numWords == 3) {
	Tcl_Obj *intObj;
	incrTokenPtr = TokenAfter(varTokenPtr);
	TclNewObj(intObj);
	if (TclWordKnownAtCompileTime(incrTokenPtr, intObj)) {
	    int code = TclGetWideIntFromObj(NULL, intObj, &immValue);
	    if ((code == TCL_OK) && (-127 <= immValue) && (immValue <= 127)) {
		haveImmValue = 1;
	    }
	}
	Tcl_BounceRefCount(intObj);
	if (!haveImmValue) {
	    SetLineInformation(2);
	    CompileTokens(envPtr, incrTokenPtr, interp);
	}
    } else {			/* No incr amount given so use 1. */
	haveImmValue = 1;
    }

    /*
     * Emit the instruction to increment the variable.
     */

    if (isScalar) {		/* Simple scalar variable. */
	if (localIndex >= 0) {
	    if (haveImmValue) {
		OP41(		INCR_SCALAR_IMM, localIndex, immValue);
	    } else {
		OP4(		INCR_SCALAR, localIndex);
	    }
	} else {
	    if (haveImmValue) {
		OP1(		INCR_STK_IMM, immValue);
	    } else {
		OP(		INCR_STK);
	    }
	}
    } else {			/* Simple array variable. */
	if (localIndex >= 0) {
	    if (haveImmValue) {
		OP41(		INCR_ARRAY_IMM, localIndex, immValue);
	    } else {
		OP4(		INCR_ARRAY, localIndex);
	    }
	} else {
	    if (haveImmValue) {
		OP1(		INCR_ARRAY_STK_IMM, immValue);
	    } else {
		OP(		INCR_ARRAY_STK);
	    }
	}
    }

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileUpvarCmd`, lines 2963–3043. Retained reference `e190`.

```text
TclCompileUpvarCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr, *otherTokenPtr, *localTokenPtr;
    Tcl_LVTIndex localIndex;
    Tcl_Size numWords = parsePtr->numWords, i;
    Tcl_Obj *objPtr;

    if (!EnvIsProc(envPtr)) {
	return TCL_ERROR;
    }

    if (numWords < 3 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /*
     * Push the frame index if it is known at compile time
     */

    TclNewObj(objPtr);
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    if (TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	/*
	 * Attempt to convert to a level reference.
	 */

	int numFrameWords = TclObjGetFrame(interp, objPtr, NULL);
	Tcl_DecrRefCount(objPtr);

	if (numFrameWords) {
	    if (numWords % 2) {
		return TCL_ERROR;
	    }
	    /* TODO: Push the known value instead? */
	    PUSH_TOKEN(		tokenPtr, 1);
	    otherTokenPtr = TokenAfter(tokenPtr);
	    i = 2;
	} else {
	    if (!(numWords % 2)) {
		return TCL_ERROR;
	    }
	    PUSH(		"1");
	    otherTokenPtr = tokenPtr;
	    i = 1;
	}
    } else {
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    /*
     * Loop over the (otherVar, thisVar) pairs. If any of the thisVar is not a
     * local variable, return an error so that the non-compiled command will
     * be called at runtime.
     */

    for (; i<numWords; i+=2, otherTokenPtr = TokenAfter(localTokenPtr)) {
	localTokenPtr = TokenAfter(otherTokenPtr);

	PUSH_TOKEN(		otherTokenPtr, i);
	localIndex = TclLocalScalarFromToken(localTokenPtr, envPtr);
	if (OutOfUintRange(localIndex)) {
	    return TCL_ERROR;
	}
	OP4(			UPVAR, localIndex);
    }

    /*
     * Pop the frame index, and set the result to empty
     */

    OP(				POP);
    PUSH(			"");
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileGlobalCmd`, lines 86–145. Retained reference `e191`.

```text
TclCompileGlobalCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;
    Tcl_LVTIndex localIndex;
    Tcl_Size i, numWords = parsePtr->numWords;

    if (numWords < 2 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /*
     * 'global' has no effect outside of proc bodies; handle that at runtime
     */

    if (!EnvIsProc(envPtr)) {
	return TCL_ERROR;
    }

    /*
     * Push the namespace
     */

    PUSH(			"::");

    /*
     * Loop over the variables.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (i=1; i<numWords; varTokenPtr = TokenAfter(varTokenPtr),i++) {
	localIndex = IndexTailVarIfKnown(interp, varTokenPtr, envPtr);

	if (OutOfUintRange(localIndex)) {
	    return TCL_ERROR;
	}

	/*
	 * TODO: Consider what value can pass through the
	 * IndexTailVarIfKnown() screen. Full CompileWord() likely does not
	 * apply here. Push known value instead.
	 */

	PUSH_TOKEN(		varTokenPtr, i);
	OP4(			NSUPVAR, localIndex);
    }

    /*
     * Pop the namespace, and set the result to empty
     */

    OP(				POP);
    PUSH(			"");
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileVariableCmd`, lines 3064–3126. Retained reference `e192`.

```text
TclCompileVariableCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr, *valueTokenPtr;
    Tcl_LVTIndex localIndex;
    Tcl_Size numWords = parsePtr->numWords, i;

    if (numWords < 2 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /*
     * Bail out if not compiling a proc body
     */

    if (!EnvIsProc(envPtr)) {
	return TCL_ERROR;
    }

    /*
     * Loop over the (var, value) pairs.
     */

    valueTokenPtr = parsePtr->tokenPtr;
    for (i=1; i<numWords; i+=2) {
	varTokenPtr = TokenAfter(valueTokenPtr);
	valueTokenPtr = TokenAfter(varTokenPtr);

	localIndex = IndexTailVarIfKnown(interp, varTokenPtr, envPtr);

	if (localIndex < 0) {
	    return TCL_ERROR;
	}

	/* TODO: Consider what value can pass through the
	 * IndexTailVarIfKnown() screen.  Full CompileWord()
	 * likely does not apply here.  Push known value instead. */
	PUSH_TOKEN(		varTokenPtr, i);
	OP4(			VARIABLE, localIndex);

	if (i + 1 < numWords) {
	    /*
	     * A value has been given: set the variable, pop the value
	     */

	    PUSH_TOKEN(		valueTokenPtr, i + 1);
	    OP4(		STORE_SCALAR, localIndex);
	    OP(			POP);
	}
    }

    /*
     * Set the result to empty
     */

    PUSH(			"");
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileLassignCmd`, lines 996–1079. Retained reference `e193`.

```text
TclCompileLassignCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int isScalar;
    Tcl_Size numWords = parsePtr->numWords, idx;
    Tcl_LVTIndex localIndex;
    /* TODO: Consider support for compiling expanded args. */

    /*
     * Check for command syntax error, but we'll punt that to runtime.
     */

    if (numWords < 3 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /*
     * Generate code to push list being taken apart by [lassign].
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PUSH_TOKEN(			tokenPtr, 1);

    /*
     * Generate code to assign values from the list to variables.
     */

    for (idx=0 ; idx<numWords-2 ; idx++) {
	/*
	 * Generate the next variable name.
	 */

	tokenPtr = TokenAfter(tokenPtr);
	PushVarNameWord(tokenPtr, 0, &localIndex, &isScalar, idx + 2);
	if (OutOfUintRangeUpper(localIndex)) {
	    return TCL_ERROR;
	}

	/*
	 * Emit instructions to get the idx'th item out of the list value on
	 * the stack and assign it to the variable.
	 */

	if (isScalar) {
	    if (localIndex >= 0) {
		OP(		DUP);
		OP4(		LIST_INDEX_IMM, idx);
		OP4(		STORE_SCALAR, localIndex);
		OP(		POP);
	    } else {
		OP4(		OVER, 1);
		OP4(		LIST_INDEX_IMM, idx);
		OP(		STORE_STK);
		OP(		POP);
	    }
	} else {
	    if (localIndex >= 0) {
		OP4(		OVER, 1);
		OP4(		LIST_INDEX_IMM, idx);
		OP4(		STORE_ARRAY, localIndex);
		OP(		POP);
	    } else {
		OP4(		OVER, 2);
		OP4(		LIST_INDEX_IMM, idx);
		OP(		STORE_ARRAY_STK);
		OP(		POP);
	    }
	}
    }

    /*
     * Generate code to leave the rest of the list on the stack.
     */

    OP44(		LIST_RANGE_IMM, idx, TCL_INDEX_END);

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileListCmd`, lines 1192–1292. Retained reference `e194`.

```text
TclCompileListCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *valueTokenPtr;
    Tcl_Size i, build, numWords = parsePtr->numWords;
    int concat;
    Tcl_Obj *listObj, *objPtr;

    if (OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }
    if (numWords == 1) {
	/*
	 * [list] without arguments just pushes an empty object.
	 */

	PUSH(			"");
	return TCL_OK;
    }

    /*
     * Test if all arguments are compile-time known. If they are, we can
     * implement with a simple push.
     */

    valueTokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(listObj);
    for (i = 1; i < numWords && listObj != NULL; i++) {
	TclNewObj(objPtr);
	if (TclWordKnownAtCompileTime(valueTokenPtr, objPtr)) {
	    (void) Tcl_ListObjAppendElement(NULL, listObj, objPtr);
	} else {
	    Tcl_DecrRefCount(objPtr);
	    Tcl_DecrRefCount(listObj);
	    listObj = NULL;
	}
	valueTokenPtr = TokenAfter(valueTokenPtr);
    }
    if (listObj != NULL) {
	PUSH_OBJ(		listObj);
	return TCL_OK;
    }

    /*
     * Push the all values onto the stack.
     */

    valueTokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (concat = 0, build = 0, i = 1; i < numWords; i++) {
	if (valueTokenPtr->type == TCL_TOKEN_EXPAND_WORD && build > 0) {
	    OP4(		LIST, build);
	    if (concat) {
		OP(		LIST_CONCAT);
	    }
	    build = 0;
	    concat = 1;
	}
	PUSH_TOKEN(		valueTokenPtr, i);
	if (valueTokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	    if (concat) {
		OP(		LIST_CONCAT);
	    } else {
		concat = 1;
	    }
	} else {
	    build++;
	}
	if (build > LIST_CONCAT_THRESHOLD) {
	    OP4(		LIST, build);
	    if (concat) {
		OP(		LIST_CONCAT);
	    }
	    build = 0;
	    concat = 1;
	}
	valueTokenPtr = TokenAfter(valueTokenPtr);
    }
    if (build > 0) {
	OP4(			LIST, build);
	if (concat) {
	    OP(			LIST_CONCAT);
	}
    }

    /*
     * If there was just one expanded word, we must ensure that it is a list
     * at this point. We use an [lrange ... 0 end] for this (instead of
     * [llength], as with literals) as we must drop any string representation
     * that might be hanging around.
     */

    if (concat && numWords == 2) {
	OP44(			LIST_RANGE_IMM, 0, TCL_INDEX_END);
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileLindexCmd`, lines 1100–1170. Retained reference `e195`.

```text
TclCompileLindexCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *idxTokenPtr, *valTokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;
    int idx;

    /*
     * Quit if not enough args.
     */

    /* TODO: Consider support for compiling expanded args. */
    if (numWords <= 1 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    valTokenPtr = TokenAfter(parsePtr->tokenPtr);
    if (numWords != 3) {
	goto emitComplexLindex;
    }

    idxTokenPtr = TokenAfter(valTokenPtr);
    if (TclGetIndexFromToken(idxTokenPtr, TCL_INDEX_NONE,
	    TCL_INDEX_NONE, &idx) == TCL_OK) {
	/*
	 * The idxTokenPtr parsed as a valid index value and was
	 * encoded as expected by INST_LIST_INDEX_IMM.
	 *
	 * NOTE: that we rely on indexing before a list producing the
	 * same result as indexing after a list.
	 */

	PUSH_TOKEN(		valTokenPtr, 1);
	OP4(			LIST_INDEX_IMM, idx);
	return TCL_OK;
    }

    /*
     * If the value was not known at compile time, the conversion failed or
     * the value was negative, we just keep on going with the more complex
     * compilation.
     */

    /*
     * Push the operands onto the stack.
     */

  emitComplexLindex:
    for (i=1 ; i<numWords ; i++) {
	PUSH_TOKEN(		valTokenPtr, i);
	valTokenPtr = TokenAfter(valTokenPtr);
    }

    /*
     * Emit INST_LIST_INDEX if objc==3, or INST_LIST_INDEX_MULTI if there are
     * multiple index args.
     */

    if (numWords == 3) {
	OP(			LIST_INDEX);
    } else {
	OP4(			LIST_INDEX_MULTI, numWords - 1);
    }

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileLlengthCmd`, lines 1313–1331. Retained reference `e196`.

```text
TclCompileLlengthCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    varTokenPtr = TokenAfter(parsePtr->tokenPtr);

    PUSH_TOKEN(			varTokenPtr, 1);
    OP(				LIST_LENGTH);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileLrangeCmd`, lines 1345–1390. Retained reference `e197`.

```text
TclCompileLrangeCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for context. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr, *listTokenPtr;
    int idx1, idx2;

    if (parsePtr->numWords != 4) {
	return TCL_ERROR;
    }
    listTokenPtr = TokenAfter(parsePtr->tokenPtr);

    tokenPtr = TokenAfter(listTokenPtr);
    if ((TclGetIndexFromToken(tokenPtr, TCL_INDEX_START, TCL_INDEX_NONE,
	    &idx1) != TCL_OK) || (idx1 == (int)TCL_INDEX_NONE)) {
	return TCL_ERROR;
    }
    /*
     * Token was an index value, and we treat all "first" indices
     * before the list same as the start of the list.
     */

    tokenPtr = TokenAfter(tokenPtr);
    if (TclGetIndexFromToken(tokenPtr, TCL_INDEX_NONE, TCL_INDEX_END,
	    &idx2) != TCL_OK) {
	return TCL_ERROR;
    }
    /*
     * Token was an index value, and we treat all "last" indices
     * after the list same as the end of the list.
     */

    /*
     * Issue instructions. It's not safe to skip doing the LIST_RANGE, as
     * we've not proved that the 'list' argument is really a list. Not that it
     * is worth trying to do that given current knowledge.
     */

    PUSH_TOKEN(			listTokenPtr, 1);
    OP44(			LIST_RANGE_IMM, idx1, idx2);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileLinsertCmd`, lines 1404–1443. Retained reference `e198`.

```text
TclCompileLinsertCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for context. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *listToken, *indexToken, *tokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;
    /* TODO: Consider support for compiling expanded args. */

    if (numWords < 3 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /* Push list, insertion index onto the stack */
    listToken = TokenAfter(parsePtr->tokenPtr);
    indexToken = TokenAfter(listToken);

    PUSH_TOKEN(			listToken, 1);
    PUSH_TOKEN(			indexToken, 2);

    /* Push new elements to be inserted */
    tokenPtr = TokenAfter(indexToken);
    for (i=3 ; i<numWords ; i++,tokenPtr=TokenAfter(tokenPtr)) {
	PUSH_TOKEN(		tokenPtr, i);
    }

    /*
     * First operand is count of arguments.
     * Second operand is bitmask
     *  TCL_LREPLACE_END_IS_LAST - end refers to last element
     *  TCL_LREPLACE_SINGLE_INDEX - second index is not present
     *     indicating this is a pure insert
     */
    OP41(			LREPLACE, numWords - 1,
					TCL_LREPLACE_SINGLE_INDEX);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileLsetCmd`, lines 1964–2081. Retained reference `e199`.

```text
TclCompileLsetCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Size tempDepth;		/* Depth used for emitting one part of the
				 * code burst. */
    Tcl_Token *varTokenPtr;	/* Pointer to the Tcl_Token representing the
				 * parse of the variable name. */
    Tcl_LVTIndex localIndex;	/* Index of var in local var table. */
    int isScalar;		/* Flag == 1 if scalar, 0 if array. */
    Tcl_Size i, numWords = parsePtr->numWords;

    /*
     * Check argument count.
     */

    /* TODO: Consider support for compiling expanded args. */
    if (numWords < 3 || OutOfUintRange(numWords)) {
	/*
	 * Fail at run time, not in compilation.
	 */

	return TCL_ERROR;
    }

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
     * Push the "index" args and the new element value.
     */

    for (i=2 ; i<numWords ; ++i) {
	varTokenPtr = TokenAfter(varTokenPtr);
	PUSH_TOKEN(		varTokenPtr, i);
    }

    /*
     * Duplicate the variable name if it's been pushed.
     */

    if (localIndex < 0) {
	tempDepth = numWords - (isScalar ? 2 : 1);
	OP4(			OVER, tempDepth);
    }

    /*
     * Duplicate an array index if one's been pushed.
     */

    if (!isScalar) {
	tempDepth = numWords - (localIndex >= 0 ? 2 : 1);
	OP4(			OVER, tempDepth);
    }

    /*
     * Emit code to load the variable's value.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    OP(			LOAD_STK);
	} else {
	    OP4(		LOAD_SCALAR, localIndex);
	}
    } else {
	if (localIndex < 0) {
	    OP(			LOAD_ARRAY_STK);
	} else {
	    OP4(		LOAD_ARRAY, localIndex);
	}
    }

    /*
     * Emit the correct variety of 'lset' instruction.
     */

    if (numWords == 4) {
	OP(			LSET_LIST);
    } else {
	OP4(			LSET_FLAT, numWords - 1);
    }

    /*
     * Emit code to put the value back in the variable.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    OP(			STORE_STK);
	} else {
	    OP4(		STORE_SCALAR, localIndex);
	}
    } else {
	if (localIndex < 0) {
	    OP(			STORE_ARRAY_STK);
	} else {
	    OP4(		STORE_ARRAY, localIndex);
	}
    }

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileReturnCmd`, lines 2709–2897. Retained reference `e200`.

```text
TclCompileReturnCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    /*
     * General syntax: [return ?-option value ...? ?result?]
     * An even number of words means an explicit result argument is present.
     */
    int level, code, objc, status = TCL_OK;
    Tcl_Size size;
    Tcl_Size numWords = parsePtr->numWords;
    int explicitResult = (0 == (numWords % 2));
    Tcl_Size numOptionWords = numWords - 1 - explicitResult;
    Tcl_Obj *returnOpts, **objv;
    Tcl_Token *wordTokenPtr = TokenAfter(parsePtr->tokenPtr);

    if (OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    /*
     * Check for special case which can always be compiled:
     *	    return -options <opts> <msg>
     * Unlike the normal [return] compilation, this version does everything at
     * runtime so it can handle arbitrary words and not just literals. Note
     * that if INST_RETURN_STK wasn't already needed for something else
     * ('finally' clause processing) this piece of code would not be present.
     */

    if ((numWords == 4) && IS_TOKEN_LITERALLY(wordTokenPtr, "-options")) {
	Tcl_Token *optsTokenPtr = TokenAfter(wordTokenPtr);
	Tcl_Token *msgTokenPtr = TokenAfter(optsTokenPtr);

	PUSH_TOKEN(		optsTokenPtr, 2);
	PUSH_TOKEN(		msgTokenPtr, 3);
	INVOKE(			RETURN_STK);
	return TCL_OK;
    }

    /*
     * Allocate some working space.
     */

    objv = (Tcl_Obj **)TclStackAlloc(interp,
	    numOptionWords * sizeof(Tcl_Obj *));

    /*
     * Scan through the return options. If any are unknown at compile time,
     * there is no value in bytecompiling. Save the option values known in an
     * objv array for merging into a return options dictionary.
     *
     * TODO: There is potential for improvement if all option keys are known
     * at compile time and all option values relating to '-code' and '-level'
     * are known at compile time.
     */

    for (objc = 0; objc < numOptionWords; objc++) {
	TclNewObj(objv[objc]);
	Tcl_IncrRefCount(objv[objc]);
	if (!TclWordKnownAtCompileTime(wordTokenPtr, objv[objc])) {
	    /*
	     * Non-literal, so punt to run-time assembly of the dictionary.
	     */

	    for (; objc>=0 ; objc--) {
		TclDecrRefCount(objv[objc]);
	    }
	    TclStackFree(interp, objv);
	    goto issueRuntimeReturn;
	}
	wordTokenPtr = TokenAfter(wordTokenPtr);
    }
    status = TclMergeReturnOptions(interp, objc, objv,
	    &returnOpts, &code, &level);
    while (--objc >= 0) {
	TclDecrRefCount(objv[objc]);
    }
    TclStackFree(interp, objv);
    if (TCL_ERROR == status) {
	/*
	 * Something was bogus in the return options. Clear the error message,
	 * and report back to the compiler that this must be interpreted at
	 * runtime.
	 */

	Tcl_ResetResult(interp);
	return TCL_ERROR;
    }

    /*
     * All options are known at compile time, so we're going to bytecompile.
     * Emit instructions to push the result on the stack.
     */

    if (explicitResult) {
	PUSH_TOKEN(		wordTokenPtr, numWords - 1);
    } else {
	/*
	 * No explict result argument, so default result is empty string.
	 */

	PUSH(			"");
    }

    /*
     * Check for optimization: When [return] is in a proc, and there's no
     * enclosing [catch], and there are no return options, then the INST_DONE
     * instruction is equivalent, and may be more efficient.
     */

    if (numOptionWords == 0 && EnvIsProc(envPtr)) {
	/*
	 * We have default return options and we're in a proc ...
	 */

	Tcl_ExceptionRange index = envPtr->exceptArrayNext - 1;
	int enclosingCatch = 0;

	while (index >= 0) {
	    const ExceptionRange *rangePtr = &envPtr->exceptArrayPtr[index];

	    if ((rangePtr->type == CATCH_EXCEPTION_RANGE)
		    && (rangePtr->catchOffset == TCL_INDEX_NONE)) {
		enclosingCatch = 1;
		break;
	    }
	    index--;
	}
	if (!enclosingCatch) {
	    /*
	     * ... and there is no enclosing catch. Issue the maximally
	     * efficient exit instruction.
	     */

	    Tcl_DecrRefCount(returnOpts);
	    OP(			DONE);
	    STKDELTA(+1);
	    return TCL_OK;
	}
    }

    /* Optimize [return -level 0 $x]. */
    Tcl_DictObjSize(NULL, returnOpts, &size);
    if (size == 0 && level == 0 && code == TCL_OK) {
	Tcl_DecrRefCount(returnOpts);
	return TCL_OK;
    }

    /*
     * Could not use the optimization, so we push the return options dict, and
     * emit the INST_RETURN_IMM instruction with code and level as operands.
     */

    CompileReturnInternal(envPtr, INST_RETURN_IMM, code, level, returnOpts);
    return TCL_OK;

  issueRuntimeReturn:
    /*
     * Assemble the option dictionary (as a list as that's good enough).
     */

    wordTokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (objc=1 ; objc<=numOptionWords ; objc++) {
	PUSH_TOKEN(		wordTokenPtr, objc);
	wordTokenPtr = TokenAfter(wordTokenPtr);
    }
    OP4(			LIST, numOptionWords);

    /*
     * Push the result.
     */

    if (explicitResult) {
	PUSH_TOKEN(		wordTokenPtr, numWords - 1);
    } else {
	PUSH(			"");
    }

    /*
     * Issue the RETURN itself.
     */

    INVOKE(			RETURN_STK);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoExistsCmd`, lines 617–666. Retained reference `e201`.

```text
TclCompileInfoExistsCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int isScalar;
    Tcl_LVTIndex localIndex;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    /*
     * Decide if we can use a frame slot for the var/array name or if we need
     * to emit code to compute and push the name at runtime. We use a frame
     * slot (entry in the array of local vars) if we are compiling a procedure
     * body and if the name is simple text that does not include namespace
     * qualifiers.
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PushVarNameWord(tokenPtr, 0, &localIndex, &isScalar, 1);
    if (OutOfUintRangeUpper(localIndex)) {
	return TCL_ERROR;
    }

    /*
     * Emit instruction to check the variable for existence.
     */

    if (isScalar) {
	if (localIndex < 0) {
	    OP(			EXIST_STK);
	} else {
	    OP4(		EXIST_SCALAR, localIndex);
	}
    } else {
	if (localIndex < 0) {
	    OP(			EXIST_ARRAY_STK);
	} else {
	    OP4(		EXIST_ARRAY, localIndex);
	}
    }

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoLevelCmd`, lines 669–700. Retained reference `e202`.

```text
TclCompileInfoLevelCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * Only compile [info level] without arguments or with a single argument.
     */

    if (parsePtr->numWords == 1) {
	/*
	 * Not much to do; we compile to a single instruction...
	 */

	OP(			INFO_LEVEL_NUM);
    } else if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    } else {
	DefineLineInformation;	/* TIP #280 */

	/*
	 * Compile the argument, then add the instruction to convert it into a
	 * list of arguments.
	 */

	PUSH_TOKEN(		TokenAfter(parsePtr->tokenPtr), 1);
	OP(			INFO_LEVEL_ARGS);
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileNamespaceCurrentCmd`, lines 2104–2125. Retained reference `e203`.

```text
TclCompileNamespaceCurrentCmd(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * Only compile [namespace current] without arguments.
     */

    if (parsePtr->numWords != 1) {
	return TCL_ERROR;
    }

    /*
     * Not much to do; we compile to a single instruction...
     */

    OP(				NS_CURRENT);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileNamespaceCodeCmd`, lines 2128–2174. Retained reference `e204`.

```text
TclCompileNamespaceCodeCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * The specification of [namespace code] is rather shocking, in that it is
     * supposed to check if the argument is itself the result of [namespace
     * code] and not apply itself in that case. Which is excessively cautious,
     * but what the test suite checks for.
     */

    if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD ||
	    IS_TOKEN_PREFIXED_BY(tokenPtr, "::namespace inscope ")) {
	/*
	 * Technically, we could just pass a literal '::namespace inscope '
	 * term through, but that's something which really shouldn't be
	 * occurring as something that the user writes so we'll just punt it.
	 */

	return TCL_ERROR;
    }

    /*
     * Now we can compile using the same strategy as [namespace code]'s normal
     * implementation does internally. Note that we can't bind the namespace
     * name directly here, because TclOO plays complex games with namespaces;
     * the value needs to be determined at runtime for safety.
     */

    PUSH(			"::namespace");
    PUSH(			"inscope");
    OP(				NS_CURRENT);
    PUSH_TOKEN(			tokenPtr, 1);
    OP4(			LIST, 4);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileNamespaceOriginCmd`, lines 2177–2195. Retained reference `e205`.

```text
TclCompileNamespaceOriginCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    PUSH_TOKEN(			tokenPtr, 1);
    OP(				ORIGIN_COMMAND);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoCommandsCmd`, lines 530–590. Retained reference `e206`.

```text
TclCompileInfoCommandsCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Obj *objPtr;
    const char *bytes;
    Tcl_BytecodeLabel isList;

    /*
     * We require one compile-time known argument for the case we can compile.
     */

    if (parsePtr->numWords == 1) {
	return TclCompileBasic0ArgCmd(interp, parsePtr, cmdPtr, envPtr);
    } else if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);
    if (!TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	goto notCompilable;
    }
    bytes = TclGetString(objPtr);

    /*
     * We require that the argument start with "::" and not have any of "*\[?"
     * in it. (Theoretically, we should look in only the final component, but
     * the difference is so slight given current naming practices.)
     */

    if (bytes[0] != ':' || bytes[1] != ':' || !TclMatchIsTrivial(bytes)) {
	goto notCompilable;
    }
    Tcl_DecrRefCount(objPtr);

    /*
     * Confirmed as a literal that will not frighten the horses. Compile.
     * The result must be made into a list.
     */

    /* TODO: Just push the known value */
    PUSH_TOKEN(			tokenPtr, 1);
    OP(				RESOLVE_COMMAND);
    OP(				DUP);
    OP(				STR_LEN);
    FWDJUMP(			JUMP_FALSE, isList);
    OP4(			LIST, 1);
    FWDLABEL(		isList);
    return TCL_OK;

  notCompilable:
    Tcl_DecrRefCount(objPtr);
    return TclCompileBasic1ArgCmd(interp, parsePtr, cmdPtr, envPtr);
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileRegexpCmd`, lines 2386–2518. Retained reference `e207`.

```text
TclCompileRegexpCmd(
    Tcl_Interp *interp,		/* Tcl interpreter for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the
				 * command. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds the resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;	/* Pointer to the Tcl_Token representing the
				 * parse of the RE or string. */
    size_t len;
    Tcl_Size i, numWords = parsePtr->numWords;
    int nocase, exact, sawLast, simple;
    const char *str;

    /*
     * We are only interested in compiling simple regexp cases. Currently
     * supported compile cases are:
     *   regexp ?-nocase? ?--? staticString $var
     *   regexp ?-nocase? ?--? {^staticString$} $var
     */

    if (numWords < 3 || OutOfUintRange(numWords)) {
	return TCL_ERROR;
    }

    simple = 0;
    nocase = 0;
    sawLast = 0;
    varTokenPtr = parsePtr->tokenPtr;

    /*
     * We only look for -nocase and -- as options. Everything else gets pushed
     * to runtime execution. This is different than regexp's runtime option
     * handling, but satisfies our stricter needs.
     */

    for (i = 1; i < numWords - 2; i++) {
	varTokenPtr = TokenAfter(varTokenPtr);
	if (IS_TOKEN_LITERALLY(varTokenPtr, "--")) {
	    sawLast++;
	    i++;
	    break;
	} else if (IS_TOKEN_PREFIX(varTokenPtr, 2, "-nocase")) {
	    nocase = 1;
	} else {
	    /*
	     * Not an option we recognize or something the compiler can't see.
	     */

	    return TCL_ERROR;
	}
    }

    if (numWords - i != 2) {
	/*
	 * We don't support capturing to variables.
	 */

	return TCL_ERROR;
    }

    /*
     * Get the regexp string. If it is not a simple string or can't be
     * converted to a glob pattern, push the word for the INST_REGEXP.
     * Keep changes here in sync with TclCompileSwitchCmd Switch_Regexp.
     */

    varTokenPtr = TokenAfter(varTokenPtr);

    if (varTokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	Tcl_DString ds;

	str = varTokenPtr[1].start;
	len = varTokenPtr[1].size;

	/*
	 * If it has a '-', it could be an incorrectly formed regexp command.
	 */

	if ((*str == '-') && !sawLast) {
	    return TCL_ERROR;
	}

	/*
	 * Note: do not optimize for len == 0, as error should be generated
	 * at runtime if operand is not a resolvable variable.
	 * Bug https://core.tcl-lang.org/tcl/info/cb03e57a7b24d22c
	 */

	/*
	 * Attempt to convert pattern to glob.  If successful, push the
	 * converted pattern as a literal.
	 */

	if (TclReToGlob(NULL, str, len, &ds, &exact, NULL) == TCL_OK) {
	    simple = 1;
	    TclPushDString(envPtr, &ds);
	    Tcl_DStringFree(&ds);
	}
    }

    if (!simple) {
	PUSH_TOKEN(		varTokenPtr, numWords - 2);
    }

    /*
     * Push the string arg.
     */

    varTokenPtr = TokenAfter(varTokenPtr);
    PUSH_TOKEN(			varTokenPtr, numWords - 1);

    if (simple) {
	if (exact && !nocase) {
	    OP(			STR_EQ);
	} else {
	    OP1(		STR_MATCH, nocase);
	}
    } else {
	/*
	 * Pass correct RE compile flags.  We use only Int1 (8-bit), but
	 * that handles all the flags we want to pass.
	 * Don't use TCL_REG_NOSUB as we may have backrefs.
	 */

	int cflags = TCL_REG_ADVANCED | (nocase ? TCL_REG_NOCASE : 0);

	OP1(			REGEXP, cflags);
    }

    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileObjectNextCmd`, lines 3238–3314. Retained reference `e208`.

```text
TclCompileObjectNextCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;

    if (OutOfUintRange(parsePtr->numWords)) {
	goto issueExpanded;
    }

    // Check for expansion
    for (i=0 ; i<numWords ; i++) {
	if (tokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	    goto issueExpanded;
	}
	tokenPtr = TokenAfter(tokenPtr);
    }

    // Simple instruction issue
    tokenPtr = parsePtr->tokenPtr;
    for (i=0 ; i<numWords ; i++) {
	PUSH_TOKEN(		tokenPtr, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    INVOKE4(			TCLOO_NEXT, i);
    return TCL_OK;

  issueExpanded:
    // Concatenate all arguments into a list; handles expansion
    tokenPtr = parsePtr->tokenPtr;
    Tcl_Size build;
    int concat;
    for (concat = 0, build = 0, i = 0; i < numWords; i++) {
	if (tokenPtr->type == TCL_TOKEN_EXPAND_WORD && build > 0) {
	    OP4(		LIST, build);
	    if (concat) {
		OP(		LIST_CONCAT);
	    }
	    build = 0;
	    concat = 1;
	}
	PUSH_TOKEN(		tokenPtr, i);
	if (tokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	    if (concat) {
		OP(		LIST_CONCAT);
	    } else {
		concat = 1;
	    }
	} else {
	    build++;
	}
	if (build > LIST_CONCAT_THRESHOLD) {
	    OP4(		LIST, build);
	    if (concat) {
		OP(		LIST_CONCAT);
	    }
	    build = 0;
	    concat = 1;
	}
	tokenPtr = TokenAfter(tokenPtr);
    }
    if (build > 0) {
	OP4(			LIST, build);
	if (concat) {
	    OP(			LIST_CONCAT);
	}
    }

    // Invoke the underlying [next] implementation
    INVOKE(			TCLOO_NEXT_LIST);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileObjectNextToCmd`, lines 3317–3396. Retained reference `e209`.

```text
TclCompileObjectNextToCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;

    if (numWords < 2) {
	return TCL_ERROR;
    } else if (OutOfUintRange(numWords)) {
	// Very large number of words anyway
	goto issueExpanded;
    }

    // Check for expansion
    for (i=0 ; i<numWords ; i++) {
	if (tokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	    goto issueExpanded;
	}
	tokenPtr = TokenAfter(tokenPtr);
    }

    // Simple instruction issue
    tokenPtr = parsePtr->tokenPtr;
    for (i=0 ; i<numWords ; i++) {
	PUSH_TOKEN(		tokenPtr, i);
	tokenPtr = TokenAfter(tokenPtr);
    }
    INVOKE4(			TCLOO_NEXT_CLASS, i);
    return TCL_OK;

  issueExpanded:
    // Concatenate all arguments into a list; handles expansion
    tokenPtr = parsePtr->tokenPtr;
    Tcl_Size build;
    int concat;
    for (concat = 0, build = 0, i = 0; i < numWords; i++) {
	if (tokenPtr->type == TCL_TOKEN_EXPAND_WORD && build > 0) {
	    OP4(		LIST, build);
	    if (concat) {
		OP(		LIST_CONCAT);
	    }
	    build = 0;
	    concat = 1;
	}
	PUSH_TOKEN(		tokenPtr, i);
	if (tokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	    if (concat) {
		OP(		LIST_CONCAT);
	    } else {
		concat = 1;
	    }
	} else {
	    build++;
	}
	if (build > LIST_CONCAT_THRESHOLD) {
	    OP4(		LIST, build);
	    if (concat) {
		OP(		LIST_CONCAT);
	    }
	    build = 0;
	    concat = 1;
	}
	tokenPtr = TokenAfter(tokenPtr);
    }
    if (build > 0) {
	OP4(			LIST, build);
	if (concat) {
	    OP(			LIST_CONCAT);
	}
    }

    // Invoke the underlying [nextto] implementation
    INVOKE(			TCLOO_NEXT_CLASS_LIST);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileObjectSelfCmd`, lines 3399–3454. Retained reference `e210`.

```text
TclCompileObjectSelfCmd(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    /*
     * We only handle [self], [self object] (which is the same operation) and
     * [self namespace]. These are the only very common operations on [self]
     * for which bytecoding is at all reasonable, with [self namespace] being
     * just because it is convenient with ops we already have.
     */

    if (parsePtr->numWords == 1) {
	goto compileSelfObject;
    } else if (parsePtr->numWords == 2) {
	const Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);

	if (IS_TOKEN_PREFIX(tokenPtr, 1, "object")) {
	    goto compileSelfObject;
	} else if (IS_TOKEN_PREFIX(tokenPtr, 1, "namespace")) {
	    goto compileSelfNamespace;
	}
    }

    /*
     * Can't compile; handle with runtime call.
     */

    return TCL_ERROR;

  compileSelfObject:

    /*
     * This delegates the entire problem to a single opcode.
     */

    OP(				TCLOO_SELF);
    return TCL_OK;

  compileSelfNamespace:

    /*
     * This is formally only correct with TclOO methods as they are currently
     * implemented; it assumes that the current namespace is invariably when a
     * TclOO context is present is the object's namespace, and that's
     * technically only something that's a matter of current policy. But it
     * avoids creating another opcode, so that's all good!
     */

    OP(				TCLOO_SELF);
    OP(				POP);
    OP(				NS_CURRENT);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoObjectClassCmd`, lines 703–719. Retained reference `e211`.

```text
TclCompileInfoObjectClassCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    PUSH_TOKEN(			tokenPtr, 1);
    OP(				TCLOO_CLASS);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoObjectNamespaceCmd`, lines 775–791. Retained reference `e212`.

```text
TclCompileInfoObjectNamespaceCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    PUSH_TOKEN(			tokenPtr, 1);
    OP(				TCLOO_NS);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoObjectIsACmd`, lines 741–772. Retained reference `e213`.

```text
TclCompileInfoObjectIsACmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * We only handle [info object isa object <somevalue>]. The first three
     * words are compressed to a single token by the ensemble compilation
     * engine.
     */

    if (parsePtr->numWords != 3) {
	return TCL_ERROR;
    }
    if (!IS_TOKEN_PREFIX(tokenPtr, 2, "object")) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(tokenPtr);

    /*
     * Issue the code.
     */

    PUSH_TOKEN(			tokenPtr, 2);
    OP(				TCLOO_IS_OBJECT);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsGR.c`, function `TclCompileInfoObjectCreationIdCmd`, lines 722–738. Retained reference `e214`.

```text
TclCompileInfoObjectCreationIdCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    PUSH_TOKEN(			tokenPtr, 1);
    OP(				TCLOO_ID);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileSetCmd`, lines 136–212. Retained reference `e215`.

```text
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

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileUnsetCmd`, lines 4450–4575. Retained reference `e216`.

```text
TclCompileUnsetCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *varTokenPtr;
    int isScalar, flags = 1;
    Tcl_Size haveFlags = 0, i, varCount = 0;
    Tcl_LVTIndex localIndex;

    /* TODO: Consider support for compiling expanded args. */

    if (OutOfUintRange(parsePtr->numWords)) {
	return TCL_ERROR;
    }

    /*
     * Verify that all words - except the first non-option one - are known at
     * compile time so that we can handle them without needing to do a nasty
     * push/rotate. [Bug 3970f54c4e]
     */

    for (i=1,varTokenPtr=parsePtr->tokenPtr ; i<parsePtr->numWords ; i++) {
	Tcl_Obj *leadingWord;

	TclNewObj(leadingWord);
	varTokenPtr = TokenAfter(varTokenPtr);
	if (!TclWordKnownAtCompileTime(varTokenPtr, leadingWord)) {
	    TclDecrRefCount(leadingWord);

	    /*
	     * We can tolerate non-trivial substitutions in the first variable
	     * to be unset. If a '--' or '-nocomplain' was present, anything
	     * goes in that one place! (All subsequent variable names must be
	     * constants since we don't want to have to push them all first.)
	     */

	    if (varCount == 0) {
		if (haveFlags) {
		    continue;
		}

		/*
		 * In fact, we're OK as long as we're the first argument *and*
		 * we provably don't start with a '-'. If that is true, then
		 * even if everything else is varying, we still can't be a
		 * flag. Otherwise we'll spill to runtime to place a limit on
		 * the trickiness.
		 */

		if (varTokenPtr->type == TCL_TOKEN_WORD
			&& varTokenPtr[1].type == TCL_TOKEN_TEXT
			&& varTokenPtr[1].size > 0
			&& varTokenPtr[1].start[0] != '-') {
		    continue;
		}
	    }
	    return TCL_ERROR;
	}
	if (varCount == 0) {
	    const char *bytes;
	    Tcl_Size len;

	    bytes = TclGetStringFromObj(leadingWord, &len);
	    if (i == 1 && len == 11 && !strncmp("-nocomplain", bytes, 11)) {
		flags = 0;
		haveFlags++;
	    } else if (i == (2 - flags) && len == 2 && !strncmp("--", bytes, 2)) {
		haveFlags++;
	    } else {
		varCount++;
	    }
	} else {
	    varCount++;
	}
	TclDecrRefCount(leadingWord);
    }

    /*
     * Issue instructions to unset each of the named variables.
     */

    varTokenPtr = TokenAfter(parsePtr->tokenPtr);
    for (i=0; i<haveFlags;i++) {
	varTokenPtr = TokenAfter(varTokenPtr);
    }
    for (i=1+haveFlags ; i<parsePtr->numWords ; i++) {
	/*
	 * Decide if we can use a frame slot for the var/array name or if we
	 * need to emit code to compute and push the name at runtime. We use a
	 * frame slot (entry in the array of local vars) if we are compiling a
	 * procedure body and if the name is simple text that does not include
	 * namespace qualifiers.
	 */

	PushVarNameWord(varTokenPtr, 0, &localIndex, &isScalar, i);
	if (OutOfUintRangeUpper(localIndex)) {
	    return TCL_ERROR;
	}

	/*
	 * Emit instructions to unset the variable.
	 */

	if (isScalar) {
	    if (localIndex < 0) {
		OP1(		UNSET_STK, flags);
	    } else {
		OP14(		UNSET_SCALAR, flags, localIndex);
	    }
	} else {
	    if (localIndex < 0) {
		OP1(		UNSET_ARRAY_STK, flags);
	    } else {
		OP14(		UNSET_ARRAY, flags, localIndex);
	    }
	}

	varTokenPtr = TokenAfter(varTokenPtr);
    }
    PUSH(			"");
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileTailcallCmd`, lines 2924–3025. Retained reference `e217`.

```text
TclCompileTailcallCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    Tcl_Size i, numWords = parsePtr->numWords;
    Tcl_Size build = 1;
    int concat = 0;

    if (!EnvIsProc(envPtr)) {
	return TCL_ERROR;
    }

    // All paths want the current namespace at this point.
    OP(				NS_CURRENT);

    // If the number of words is too large, use the concat sequence.
    // Also send the no-command route there.
    if (numWords < 2 || numWords > LIST_CONCAT_THRESHOLD) {
	goto tailcallExpanded;
    }

    // Check if we're doing expansion.
    for (i=1 ; i<numWords ; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	if (tokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	    goto tailcallExpanded;
	}
    }
    tokenPtr = parsePtr->tokenPtr;

    // Push the words. The first one is marked for limited sharing.
    for (i=1 ; i<numWords ; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	if (i == 1 && tokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    PUSH_COMMAND_TOKEN(	tokenPtr);
	} else {
	    PUSH_TOKEN(		tokenPtr, i);
	}
    }

    // The tailcall operation itself.
    OP4(			TAILCALL, numWords);
    return TCL_OK;

  tailcallExpanded:
    // Build all the words into a list. Handles expansion.
    tokenPtr = parsePtr->tokenPtr;
    for (i = 1; i < numWords; i++) {
	tokenPtr = TokenAfter(tokenPtr);
	// If we're about to expand, make sure we have a single list before.
	if (tokenPtr->type == TCL_TOKEN_EXPAND_WORD && build > 0) {
	    OP4(		LIST, build);
	    if (concat) {
		OP(		LIST_CONCAT);
	    }
	    build = 0;
	    concat = 1;
	}
	// Push the word. The first one is marked for limited sharing.
	if (i == 1 && tokenPtr->type == TCL_TOKEN_SIMPLE_WORD) {
	    PUSH_COMMAND_TOKEN(	tokenPtr);
	} else {
	    PUSH_TOKEN(		tokenPtr, i);
	}
	// If it was expansion, join to list.
	if (tokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	    if (concat) {
		OP(		LIST_CONCAT);
	    } else {
		concat = 1;
	    }
	} else {
	    // Otherwise, count how many words are pending to make into a list.
	    build++;
	}
	// Too many words pending? Convert to a list now.
	if (build > LIST_CONCAT_THRESHOLD) {
	    OP4(		LIST, build);
	    if (concat) {
		OP(		LIST_CONCAT);
	    }
	    build = 0;
	    concat = 1;
	}
    }
    // Anything left over? Handle now.
    if (build > 0) {
	OP4(			LIST, build);
	if (concat) {
	    OP(			LIST_CONCAT);
	}
    }

    // The tailcall operation itself.
    OP(				TAILCALL_LIST);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileYieldCmd`, lines 4843–4864. Retained reference `e218`.

```text
TclCompileYieldCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    if (parsePtr->numWords < 1 || parsePtr->numWords > 2) {
	return TCL_ERROR;
    }

    if (parsePtr->numWords == 1) {
	PUSH(			"");
    } else {
	DefineLineInformation;	/* TIP #280 */
	Tcl_Token *valueTokenPtr = TokenAfter(parsePtr->tokenPtr);

	PUSH_TOKEN(		valueTokenPtr, 1);
    }
    INVOKE(			YIELD);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileYieldToCmd`, lines 4885–4935. Retained reference `e219`.

```text
TclCompileYieldToCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = TokenAfter(parsePtr->tokenPtr);
    Tcl_Size i, numWords = parsePtr->numWords, build;
    int concat = 0;

    OP(				NS_CURRENT);
    for (build = i = 1; i < numWords; i++) {
	if (tokenPtr->type == TCL_TOKEN_EXPAND_WORD && build > 0) {
	    OP4(		LIST, build);
	    if (concat) {
		OP(		LIST_CONCAT);
	    }
	    build = 0;
	    concat = 1;
	}
	PUSH_TOKEN(		tokenPtr, i);
	if (tokenPtr->type == TCL_TOKEN_EXPAND_WORD) {
	    if (concat) {
		OP(		LIST_CONCAT);
	    } else {
		concat = 1;
	    }
	} else {
	    build++;
	}
	if (build > LIST_CONCAT_THRESHOLD) {
	    OP4(		LIST, build);
	    if (concat) {
		OP(		LIST_CONCAT);
	    }
	    build = 0;
	    concat = 1;
	}
	tokenPtr = TokenAfter(tokenPtr);
    }
    if (build > 0) {
	OP4(			LIST, build);
	if (concat) {
	    OP(			LIST_CONCAT);
	}
    }
    INVOKE(			YIELD_TO_INVOKE);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileUplevelCmd`, lines 4596–4664. Retained reference `e220`.

```text
TclCompileUplevelCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Size numWords = parsePtr->numWords;
    Tcl_Token *tokenPtr;
    Tcl_Obj *objPtr;
    Tcl_Size i, first;

    /* TODO: Consider support for compiling expanded args. */
    if (numWords < 2 || numWords > 1<<8 || !EnvIsProc(envPtr)) {
	/*
	 * The limit on the max number of words is arbitrary; could be higher,
	 * but I doubt we'll ever hit it anyway.
	 */
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(objPtr);
    if (!TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	/*
	 * If the first argument isn't known at compile time, we can't know if
	 * it is a script fragment or a level descriptor. Punt.
	 */
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    /*
     * Attempt to convert to a level reference. Note that TclObjGetFrame
     * only changes the obj type when a conversion was successful.
     */

    int numFrameWords = TclObjGetFrame(interp, objPtr, NULL);
    Tcl_DecrRefCount(objPtr);
    if (numFrameWords < 0) {
	return TCL_ERROR;
    }

    if (numFrameWords) {
	PUSH_TOKEN(		tokenPtr, 1);
	tokenPtr = TokenAfter(tokenPtr);
	first = 2;
    } else {
	PUSH(			"1");
	first = 1;
    }
    if (first >= numWords) {
	// In this case, there's ambiguity about sole argument meaning.
	return TCL_ERROR;
    }

    // Push all remaining words and concatenate them to make a single script word
    for (i=first; i<numWords; i++, tokenPtr=TokenAfter(tokenPtr)) {
	PUSH_TOKEN(		tokenPtr, i);
    }
    if (numWords - first > 1) {
	OP4(			CONCAT_STK, numWords - first);
    }

    // Do the actual uplevel operation.
    INVOKE(			UPLEVEL);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringEqualCmd`, lines 329–357. Retained reference `e221`.

```text
TclCompileStringEqualCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *aTokenPtr, *bTokenPtr;

    /*
     * We don't support any flags; the bytecode isn't that sophisticated.
     */

    if (parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    /*
     * Push the two operands onto the stack and then the test.
     */

    aTokenPtr = TokenAfter(parsePtr->tokenPtr);
    bTokenPtr = TokenAfter(aTokenPtr);
    PUSH_TOKEN(			aTokenPtr, 1);
    PUSH_TOKEN(			bTokenPtr, 2);
    OP(				STR_EQ);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringLenCmd`, lines 857–890. Retained reference `e222`.

```text
TclCompileStringLenCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    Tcl_Obj *objPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    TclNewObj(objPtr);
    if (TclWordKnownAtCompileTime(tokenPtr, objPtr)) {
	/*
	 * Here someone is asking for the length of a static string (or
	 * something with backslashes). Just push the actual character (not
	 * byte) length.
	 */

	Tcl_Obj *objLen = Tcl_NewWideUIntObj(Tcl_GetCharLength(objPtr));
	PUSH_OBJ(		objLen);
    } else {
	SetLineInformation(1);
	CompileTokens(envPtr, tokenPtr, interp);
	OP(			STR_LEN);
    }
    TclDecrRefCount(objPtr);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringMatchCmd`, lines 790–854. Retained reference `e223`.

```text
TclCompileStringMatchCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to definition of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;
    int exactMatch = 0, nocase = 0;
    Tcl_Size i, numWords = parsePtr->numWords;

    if (numWords < 3 || numWords > 4) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);

    /*
     * Check if we have a -nocase flag.
     */

    if (numWords == 4) {
	if (!IS_TOKEN_PREFIX(tokenPtr, 2, "-nocase")) {
	    /*
	     * Fail at run time, not in compilation.
	     */

	    return TclCompileBasic3ArgCmd(interp, parsePtr, cmdPtr, envPtr);
	}
	nocase = 1;
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Push the strings to match against each other.
     */

    for (i = 0; i < 2; i++) {
	if (tokenPtr->type == TCL_TOKEN_SIMPLE_WORD && !nocase && (i == 0)) {
	    /*
	     * Trivial matches can be done by 'string equal'. If -nocase was
	     * specified, we can't do this because INST_STR_EQ has no support
	     * for nocase.
	     */

	    Tcl_Obj *copy = TokenToObj(tokenPtr);
	    exactMatch = TclMatchIsTrivial(TclGetString(copy));
	    Tcl_BounceRefCount(copy);
	}
	PUSH_TOKEN(		tokenPtr, i + 1 + nocase);
	tokenPtr = TokenAfter(tokenPtr);
    }

    /*
     * Push the matcher.
     */

    if (exactMatch) {
	OP(			STR_EQ);
    } else {
	OP1(			STR_MATCH, nocase);
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringTrimCmd`, lines 1279–1303. Retained reference `e224`.

```text
TclCompileStringTrimCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2 && parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PUSH_TOKEN(			tokenPtr, 1);
    if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(tokenPtr);
	PUSH_TOKEN(		tokenPtr, 2);
    } else {
	PUSH_STRING(		tclDefaultTrimSet);
    }
    OP(				STR_TRIM);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringTrimLCmd`, lines 1225–1249. Retained reference `e225`.

```text
TclCompileStringTrimLCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2 && parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PUSH_TOKEN(			tokenPtr, 1);
    if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(tokenPtr);
	PUSH_TOKEN(		tokenPtr, 2);
    } else {
	PUSH_STRING(		tclDefaultTrimSet);
    }
    OP(				STR_TRIM_LEFT);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `TclCompileStringTrimRCmd`, lines 1252–1276. Retained reference `e226`.

```text
TclCompileStringTrimRCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    TCL_UNUSED(Command *),
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2 && parsePtr->numWords != 3) {
	return TCL_ERROR;
    }

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PUSH_TOKEN(			tokenPtr, 1);
    if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(tokenPtr);
	PUSH_TOKEN(		tokenPtr, 2);
    } else {
	PUSH_STRING(		tclDefaultTrimSet);
    }
    OP(				STR_TRIM_RIGHT);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `CompileUnaryOpCmd`, lines 4956–4972. Retained reference `e227`.

```text
CompileUnaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    if (parsePtr->numWords != 2) {
	return TCL_ERROR;
    }
    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    PUSH_TOKEN(			tokenPtr, 1);
    TclEmitOpcode(instruction, envPtr);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `CompileAssociativeBinaryOpCmd`, lines 4997–5032. Retained reference `e228`.

```text
CompileAssociativeBinaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    const char *identity,
    int instruction,
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr = parsePtr->tokenPtr;
    Tcl_Size words;

    /* TODO: Consider support for compiling expanded args. */
    if (OutOfUintRange(parsePtr->numWords)) {
	return TCL_ERROR;
    }
    for (words=1 ; words<parsePtr->numWords ; words++) {
	tokenPtr = TokenAfter(tokenPtr);
	PUSH_TOKEN(		tokenPtr, words);
    }
    if (parsePtr->numWords <= 2) {
	PUSH_STRING(		identity);
	words++;
    }
    if (words > 3) {
	/*
	 * Reverse order of arguments to get precise agreement with [expr] in
	 * calculations, including roundoff errors.
	 */

	OP4(			REVERSE, words - 1);
    }
    while (--words > 1) {
	TclEmitOpcode(instruction, envPtr);
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `CompileStrictlyBinaryOpCmd`, lines 5054–5065. Retained reference `e229`.

```text
CompileStrictlyBinaryOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    if (parsePtr->numWords != 3) {
	return TCL_ERROR;
    }
    return CompileAssociativeBinaryOpCmd(interp, parsePtr,
	    NULL, instruction, envPtr);
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompCmdsSZ.c`, function `CompileComparisonOpCmd`, lines 5086–5147. Retained reference `e230`.

```text
CompileComparisonOpCmd(
    Tcl_Interp *interp,
    Tcl_Parse *parsePtr,
    int instruction,
    CompileEnv *envPtr)
{
    DefineLineInformation;	/* TIP #280 */
    Tcl_Token *tokenPtr;

    /* TODO: Consider support for compiling expanded args. */
    if (OutOfUintRange(parsePtr->numWords)) {
	return TCL_ERROR;
    }
    if (parsePtr->numWords < 3) {
	PUSH(			"1");
    } else if (parsePtr->numWords == 3) {
	tokenPtr = TokenAfter(parsePtr->tokenPtr);
	PUSH_TOKEN(		tokenPtr, 1);
	tokenPtr = TokenAfter(tokenPtr);
	PUSH_TOKEN(		tokenPtr, 2);
	TclEmitOpcode(instruction, envPtr);
    } else if (!EnvIsProc(envPtr)) {
	/*
	 * No local variable space!
	 */

	return TCL_ERROR;
    } else {
	Tcl_LVTIndex tmpIndex = AnonymousLocal(envPtr);
	Tcl_Size words;

	if (OutOfUintRangeUpper(tmpIndex)) {
	    return TCL_ERROR;
	}
	tokenPtr = TokenAfter(parsePtr->tokenPtr);
	PUSH_TOKEN(		tokenPtr, 1);
	tokenPtr = TokenAfter(tokenPtr);
	PUSH_TOKEN(		tokenPtr, 2);
	OP4(			STORE_SCALAR, tmpIndex);
	TclEmitOpcode(instruction, envPtr);
	for (words=3 ; words<parsePtr->numWords ;) {
	    OP4(		LOAD_SCALAR, tmpIndex);
	    tokenPtr = TokenAfter(tokenPtr);
	    PUSH_TOKEN(		tokenPtr, words);
	    if (++words < parsePtr->numWords) {
		OP4(		STORE_SCALAR, tmpIndex);
	    }
	    TclEmitOpcode(instruction, envPtr);
	}
	for (; words>3 ; words--) {
	    OP(			BITAND);
	}

	/*
	 * Drop the value from the temp variable; retaining that reference
	 * might be expensive elsewhere.
	 */

	OP14(			UNSET_SCALAR, 0, tmpIndex);
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompile.c`, function `TclCompileTokens`, lines 2739–2927. Retained reference `e231`.

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

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompile.c`, function `TclFindCompiledLocal`, lines 3337–3433. Retained reference `e232`.

```text
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

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompile.c`, function `TclPushVarName`, lines 4919–5117. Retained reference `e233`.

```text
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

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompile.c`, function `CompileCmdLiteral`, lines 2133–2150. Retained reference `e234`.

```text
CompileCmdLiteral(
    Tcl_Interp *interp,
    Tcl_Obj *cmdObj,
    CompileEnv *envPtr)
{
    Command *cmdPtr;
    int cmdLitIdx, extraLiteralFlags = LITERAL_CMD_NAME;

    cmdPtr = (Command *) Tcl_GetCommandFromObj(interp, cmdObj);
    if ((cmdPtr != NULL) && (cmdPtr->flags & CMD_VIA_RESOLVER)) {
	extraLiteralFlags |= LITERAL_UNSHARED;
    }

    cmdLitIdx = PUSH_OBJ_FLAGS(cmdObj, extraLiteralFlags);
    if (cmdPtr && TclRoutineHasName(cmdPtr)) {
	TclSetCmdNameObj(interp, TclFetchLiteral(envPtr, cmdLitIdx), cmdPtr);
    }
}

```

tcl9.1 9.1.0, revision `pinned Tcl release 9.1.0`, `generic/tclCompile.c`, function `TclCompileInvocation`, lines 2153–2189. Retained reference `e235`.

```text
TclCompileInvocation(
    Tcl_Interp *interp,
    Tcl_Token *tokenPtr,
    Tcl_Obj *cmdObj,
    size_t numWords,
    CompileEnv *envPtr)
{
    DefineLineInformation;
    size_t wordIdx = 0;
    Tcl_Size depth = TclGetStackDepth(envPtr);

    if (cmdObj) {
	CompileCmdLiteral(interp, cmdObj, envPtr);
	wordIdx = 1;
	tokenPtr = TokenAfter(tokenPtr);
    }

    for (; wordIdx < numWords; wordIdx++, tokenPtr = TokenAfter(tokenPtr)) {
	int objIdx;

	SetLineInformation(wordIdx);

	if (tokenPtr->type != TCL_TOKEN_SIMPLE_WORD) {
	    CompileTokens(envPtr, tokenPtr, interp);
	    continue;
	}

	objIdx = PUSH_SIMPLE_TOKEN(tokenPtr);
	if (envPtr->clNext) {
	    TclContinuationsEnterDerived(TclFetchLiteral(envPtr, objIdx),
		    tokenPtr[1].start - envPtr->source, envPtr->clNext);
	}
    }

    INVOKE4(			INVOKE_STK, wordIdx);
    TclCheckStackDepth(depth+1, envPtr);
}

```

## Consumer bindings

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::preserves_names_before_original_arguments`: Select the independently supported tracked-name effect branch from genuine compiler metadata and complete original literal words.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::original_literal_emitter_families_preserve_names_without_native_admission` (linked): Selected literal emitter metadata is tested separately from Unknown opcode admission and original word ownership; this is no native execution claim.

## Evidence

- [rust/tcl-registry/tests/data/native_literal_emitter_compiler_source](../../../../rust/tcl-registry/tests/data/native_literal_emitter_compiler_source)
