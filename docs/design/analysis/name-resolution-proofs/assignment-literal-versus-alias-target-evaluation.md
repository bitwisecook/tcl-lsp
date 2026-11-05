# naming.list.assignment-literal-versus-alias-target-evaluation

Kind: `native-observation`

## Problem statement

A later lassign target word can read a variable assigned by an earlier target. Ordinary argv evaluation suggests that read must happen before all stores, but a selected native compiler may prepare each target immediately before its store. Applying either interpretation without the actual compilation purpose changes both completion and destination cells. These controls compare a literal command with an alias forcing generic dispatch; they do not prove original name-object conversion, trace continuity or normal-write release closure.

## Question

For lassign {one two} first [set first], does the later target read before the first store or after it, under literal compiled versus alias-forced generic execution, with first initially absent or old?

## Conclusion

C8.5/C8.6/C9.0/C9.1 literal commands perform the first store before evaluating the later target: both initial states succeed with first=one, one=two and old absent. Their interp-alias generic controls evaluate the read before any store: absent first errors with all three targets absent; initial old succeeds with first=one, old=two and one absent. Current Jim literal and Jim native-alias controls both follow the latter behaviour. C8.4 has no lassign. These results concern actual source evaluation and do not grant compiler preparation, normal writes, object conversion or Rust execution authority.

Original list-assignment recipe selection preserves opaque literal target bytes across document and native source channels while keeping bytecode, direct and unknown compilation modes independent.

A marked readonly compiler-preparation control preserves the original list/dynamic-target ordinals, release-specific recipe selection and Direct/missing-protocol boundaries.

## Scope

Two immutable exact ASCII source-file programs, fresh native shells, procedure activation per case, literal and actual alias routes, absent/old first states and existence/value of first/old/one. v1 Jim aborts at unsupported Tcl-style interp alias syntax before any lassign case; v2 independently selects its actual alias command and completes all cases. That v1 process failure is retained as a setup limitation, not a lassign answer. BIGIP untested. Source inspection and output observations are distinct.

Selection cannot establish alias-target evaluation, result/cell identity, a reached assignment, physical frame or rewrite equivalence. The original literal-versus-alias target experiment remains independent.

The helper receives a manually supplied recipe/context and actual source words; it does not compile an original interpreter, execute either target, compare literal/alias side effects or grant actual native instruction/frame/worker admission. Recipe presence and original operand ordinal are source preparation obligations only. The unchanged original list-assignment/alias-target observations remain independent; no passing software outcome is attached.

## Provider answers

### tcl8.4

Status: `unsupported`. Channel: Exact ASCII source file argument to the actual native shell; file character ingress. Separate ASCII stdin startup version query. Version: tcl8.4 8.4.20.

Actual available query is zero, all four controls explicitly unavailable; no lassign invocation.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact ASCII source file argument to the actual native shell; file character ingress. Separate ASCII stdin startup version query. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Literal both initial states succeed: first=one, one=two, old absent. Generic alias absent state errors without stores; initialized state succeeds with first=one, old=two, one absent.

### jim

Status: `observed`. Channel: Exact ASCII source file argument to the actual native shell; file character ingress. Separate ASCII stdin startup version query. Version: jim 0.84-9-g5bac7c9.

Literal and native alias both evaluate the later target before stores: absent first errors/no targets; old first succeeds with first=one, old=two, one absent. The independent v1 Tcl-style interp-alias setup exits1 before any case.

### bigip

Status: `not-tested`.

No appliance comparison for these exact source contexts.

## Source inspection

tcl8.5 8.5.19, revision `Pinned release source`, `generic/tclCompCmds.c`, function `TclCompileLassignCmd`, lines 2477–2571. Retained reference `tcl8.5-compiler-source`.

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

tcl8.6 8.6.18, revision `Pinned release source`, `generic/tclCompCmdsGR.c`, function `TclCompileLassignCmd`, lines 968–1051. Retained reference `tcl8.6-compiler-source`.

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

tcl9.0 9.0.4, revision `Pinned release source`, `generic/tclCompCmdsGR.c`, function `TclCompileLassignCmd`, lines 953–1035. Retained reference `tcl9.0-compiler-source`.

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

tcl9.1 9.1.0, revision `Pinned release source`, `generic/tclCompCmdsGR.c`, function `TclCompileLassignCmd`, lines 996–1079. Retained reference `tcl9.1-compiler-source`.

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

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c`, function `Jim_LassignCoreCommand`, lines 13245–13270. Retained reference `jim-compiler-source`.

```text
static int Jim_LassignCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int result = JIM_ERR;
    int i;
    Jim_ListIter iter;
    Jim_Obj *resultObj;

    JimListIterInit(&iter, argv[1]);

    for (i = 2; i < argc; i++) {
        Jim_Obj *valObj = JimListIterNext(interp, &iter);
        result = Jim_SetVariable(interp, argv[i], valObj ? valObj : interp->emptyObj);
        if (result != JIM_OK) {
            return result;
        }
    }

    resultObj = Jim_NewListObj(interp, NULL, 0);
    while (!JimListIterDone(interp, &iter)) {
        Jim_ListAppendElement(interp, resultObj, JimListIterNext(interp, &iter));
    }

    Jim_SetResult(interp, resultObj);

    return JIM_OK;
}

```

## Consumer bindings

No implementation binding is claimed by this observation record.

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::select_native_words`: Select the recipe from authentic captured source words plus explicit dialect/mode/context; missing premises retain Generic or Unknown rather than a donated compiler entry.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::original_assignment_selection_keeps_opaque_target_bytes_and_modes_separate` (linked): Original assignment selection retains literal document/native opaque target bytes and exact compiler mode: supported bytecode selection, direct generic execution and missing/unknown context remain separate without assuming alias target evaluation equivalence. This is the current software/API definition; no assertion outcome or new original-provider observation is attached to this binding.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

- [rust/tcl-compiler/src/registry_invocation/native_compilation_source.rs](../../../../rust/tcl-compiler/src/registry_invocation/native_compilation_source.rs), `original_native_compilation`: Capture the complete original word vector and selected source String/compiler dialect before readonly recipe selection/preparation; missing source/protocol is Unknown and actual compilation/frame admission stays separate.
- [rust/tcl-compiler/src/registry_invocation/native_compilation_source.rs](../../../../rust/tcl-compiler/src/registry_invocation/native_compilation_source.rs), `registry_invocation::native_compilation_source::tests::original_list_preparations_keep_selected_target_evaluation_order` (linked): Five selected C source/protocol recipes preserve lassign original list operand1 and original dynamic target operand3, inline only from C8.5, and lrange inline only from C8.6. Direct mode selects generic/no preparation; missing original source protocol selects Unknown. This is preparation geometry, not target evaluation execution.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v1)
- [rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation/v2)
- [rust/tcl-vm/tests/data/native_lassign_argv_evaluation](../../../../rust/tcl-vm/tests/data/native_lassign_argv_evaluation)
