# naming.numeric.original-logical-operand-truth-sites

Kind: `native-observation`

## Problem statement

A logical operand, primitive Boolean getter and command condition can reach different conversion paths even for one original value. Operand position and generic versus held object-source entry are independent premises. This fixed experiment measures them with genuinely bound x/y originals and separates reached cache/result/error effects from source/compiler or application authority.

## Question

For each actual held original x constructor, how do primitive/expression controls and left/right &&/|| truth sites differ between generic Tcl_EvalEx source evaluation and independently held original Tcl_EvalObjEx compiled-script entry, with actual original y Int1 for && or Int0 for ||, across all six providers and their actual ABI? Jim independently repeats its genuine Jim_EvalObj route without assuming a second engine.

## Conclusion

Each of six original providers completes 648 independent constructor/site rows with 648 matching pre-call observations: 36 held x constructors across 18 separate primitive, expression API, generic-source and held object-source routes. All 3888 pre/post x variable identities remain the original; each pre-call x has refcount 2. Logical rows independently install actual y Int1 for AND or Int0 for OR, retaining its own binding/cache/residency/refcount fields. On C8.4 fresh String 4294967296, x in the left AND/OR position returns false with Boolean primary, while x in the reached right position returns true with integer primary; both generic and held object-source pairs reproduce this distinction. C8.4 cached Double NaN succeeds at the measured logical sites, but if fails in both generic and object-source routes. On C8.6, generic if on original String on leaves the x primary NULL, while object-source if installs booleanString; C9 object-source if installs boolean. C8.6/C9 cached NaN if reports a domain error through generic evaluation but floating point value is Not a Number through object-source evaluation, with their separately recorded error codes. C8.5 retains its own distinct cache/diagnostic behavior. Jim cached 4294967296 remains raw primitive Boolean 0 while all measured logical/expression sites return true; Jim cached NaN primitive refusal remains separate from successful logical/script routes and their String effects. All 1440 C object-source rows record an initially NULL script primary becoming bytecode; all 288 Jim object-source rows record script instead. These are actual held source-object primary observations, not arbitrary instruction or application admission.

The Runtime C84 operand-stage control preserves the separate WordBoolean and full integer primaries reached by the original left versus final logical instruction purposes for identical fresh source bytes.

The current C84 VM control preserves the bounded distinction between fresh logical-left Boolean conversion and the reached final logical integer conversion without equating their original primaries.

Successful rows within each exact route are counted below; an exit0 host process includes all measured route errors. Object-source entries name actual API calls, with their script primary recorded independently.

| Worker and actual entry | C8.4 | C8.5 | C8.6 | C9.0 | C9.1 | Jim |
|---|---:|---:|---:|---:|---:|---:|
| 0 — Raw primitive API | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 19 / 36 |
| 1 — Expression Boolean API | 30 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 2 — Two NOT, generic | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 3 — Ternary, generic | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 4 — if, generic | 30 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 5 — while, generic | 30 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 6 — AND x left, generic | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 7 — AND x right, generic | 31 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 8 — OR x left, generic | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 9 — OR x right, generic | 31 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 10 — Two NOT, object source | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 11 — Ternary, object source | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 12 — if, object source | 30 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 13 — while, object source | 30 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 14 — AND x left, object source | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 15 — AND x right, object source | 31 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 16 — OR x left, object source | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 17 — OR x right, object source | 31 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |

## Scope

Exactly 36 constructors and 18 workers, each in a fresh fully initialized original interpreter, on the measured native LP64 ABI: int=4, long=8, wide=8, double=8, pointer=8, CHAR_BIT=8; C8 Tcl_UniChar=2 and C9=4, current Jim UTF8/ABI 102. The original x value is installed through the public variable API without source serialization. Workers 0/1 are the raw primitive/public expression API controls; worker 2 is TWO NOT operations and a final expression result, not an isolated inner NOT. Workers 2–9 use genuine generic Tcl_EvalEx source; workers 10–17 separately retain the same corresponding original script object and call Tcl_EvalObjEx(flags0). Jim uses Jim_EvalObj independently for both groups without an assumed second engine. Logical y=1/y=0 makes x reached in either operand position; this gives no statement about unexecuted short-circuit operands. Source objects remain host-held across every post-call observer; their primary and native refcount fields are measured before result/error/input String observers. Input and partner cache/residency/equality fields are also recorded at that bounded pre-observer window; input_after_observers is later rendered data, not initial storage. Direct API output starts sentinel 777; failed API output retains it, and script-route out 777 is unused because truth is in result bytes. errno is an ambient return observation, not a pure numeric-stage or freshness witness. C8.5+ errorCode uses public return-options/DictObjGet, while C8.4/Jim use their separately seeded public variable; no private error object identity follows. All 18 provider compile/version/main operations exit 0 with empty stderr, and the separate compiler version operation exits0; successful process closure includes every expected route error. All 504 input associations and whole original streams/receipts are independently verified. The Root partial unsealed 30-row and complete focused 180-row arrays are preserved as derived review only, never closure or eligibility evidence. No wasm32, LLP64, BIG-IP, null-interpreter route, default/application CABI issuer, runtime representation authority, physical frame/SSA/edit permission or Rust assertion is inferred. Bytecode/script primary observations do not certify exact opcodes, a generic-source compilation path, arbitrary source entry, worker or compiled artifact. Pinned source windows explain distinct entry/conversion branches without claiming unrecorded binary build history.

This current software control is unexecuted and uses literal expectations corresponding only to original465 C84 case4 worker6/7 bounded input/cache/truth windows. It does not replay the whole native expression, assert instruction bytes, result-header equality, cache birth on external objects, all logical sites or all providers. Generic versus held-object native route evidence remains independently scoped and unchanged.

This current source control is unexecuted and asserts only selected operand-stage expectations corresponding to original465 case4 workers6/7. No whole expression/result comparison, opcode inventory, all-provider rule or physical object/header identity is added. Optimiser source interpretation remains separate: tclOptimize.c is not retroactively added as a launch-pinned input of original465.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Version: tcl8.4 8.4.20.

648 paired rows. Fresh String 4294967296 is false/Boolean in left logical position and true/int on the reached right, through both generic and object-source entry. Cached NaN logical sites succeed while generic/object if fails. All 288 held script objects acquire bytecode; original x/y identities and separate result/error/cache windows remain exact.

### tcl8.5

Status: `observed`. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Version: tcl8.5 8.5.19.

648 paired rows. All 18 routes have 31 successful constructors each. Generic/object if on on both install booleanString, and cached NaN if retains its domain-error presentation in both entries. All 288 held script objects acquire bytecode; full raw rows retain other purpose-specific diagnostics and cache effects.

### tcl8.6

Status: `observed`. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Version: tcl8.6 8.6.18.

648 paired rows. All 18 routes have 31 successful constructors each. Generic if on on leaves x primary NULL; held object-source if installs booleanString. Cached NaN if has domain error in generic entry versus floating point value is Not a Number and its typed code in object-source entry. All 288 script objects acquire bytecode.

### tcl9.0

Status: `observed`. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Version: tcl9.0 9.0.4.

648 paired rows. All 18 routes have 32 successful constructors each. Generic if on on leaves x primary NULL; object-source if installs boolean. Cached NaN generic/object if diagnostics and error codes differ. All 288 script objects acquire bytecode; all other raw result/cache/String-effect fields stay independently observed.

### tcl9.1

Status: `observed`. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Version: tcl9.1 9.1.0.

648 paired rows. All 18 routes have 32 successful constructors each. Generic if on on leaves x primary NULL; object-source if installs boolean. Cached NaN generic/object if diagnostics and error codes differ. All 288 script objects acquire bytecode, without arbitrary instruction or source-entry authority.

### jim

Status: `observed`. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Version: jim 0.84-9-g5bac7c9.

648 paired rows. Primitive succeeds 19 of 36, each other route 34 of 36. Cached integer 4294967296 raw primitive 0 differs from true at every logical/expression site. Cached NaN primitive fails while logical/script routes succeed with distinct String/result effects. All 288 held source objects acquire script; both groups use actual Jim_EvalObj, not two assumed engines.

### bigip

Status: `not-tested`.

No original appliance operand, object-source entry or ABI process was requested or executed.

## Source inspection

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclObj.c`, function `Tcl_GetBooleanFromObj`, lines 1422–1439. Retained reference `native_ensemble_map_prefix312-map-source-8.4.20-tclObj.c`.

```text
Tcl_GetBooleanFromObj(interp, objPtr, boolPtr)
    Tcl_Interp *interp; 	/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr;	/* The object from which to get boolean. */
    register int *boolPtr;	/* Place to store resulting boolean. */
{
    register int result;

    if (objPtr->typePtr == &tclBooleanType) {
	result = TCL_OK;
    } else {
	result = SetBooleanFromAny(interp, objPtr);
    }

    if (result == TCL_OK) {
	*boolPtr = (int) objPtr->internalRep.longValue;
    }
    return result;
}

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_ExprBooleanObj`, lines 5621–5642. Retained reference `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c`.

```text
Tcl_ExprBooleanObj(interp, objPtr, ptr)
    Tcl_Interp *interp;			/* Context in which to evaluate the
					 * expression. */
    register Tcl_Obj *objPtr;		/* Expression to evaluate. */
    int *ptr;				/* Where to store 0/1 result. */
{
    Tcl_Obj *resultPtr;
    int result;

    result = Tcl_ExprObj(interp, objPtr, &resultPtr);
    if (result == TCL_OK) {
	if (resultPtr->typePtr == &tclIntType) {
	    *ptr = (resultPtr->internalRep.longValue != 0);
	} else if (resultPtr->typePtr == &tclDoubleType) {
	    *ptr = (resultPtr->internalRep.doubleValue != 0.0);
	} else {
	    result = Tcl_GetBooleanFromObj(interp, resultPtr, ptr);
	}
	Tcl_DecrRefCount(resultPtr);  /* discard the result object */
    }
    return result;
}

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_IfObjCmd`, lines 188–303. Retained reference `native_info_inventory_original-sources-8.4.20-tclCmdIL.c`.

```text
Tcl_IfObjCmd(dummy, interp, objc, objv)
    ClientData dummy;			/* Not used. */
    Tcl_Interp *interp;			/* Current interpreter. */
    int objc;				/* Number of arguments. */
    Tcl_Obj *CONST objv[];		/* Argument objects. */
{
    int thenScriptIndex = 0;	/* then script to be evaled after syntax check */
#ifdef TCL_TIP280
    Interp* iPtr = (Interp*) interp;
#endif
    int i, result, value;
    char *clause;
    i = 1;
    while (1) {
	/*
	 * At this point in the loop, objv and objc refer to an expression
	 * to test, either for the main expression or an expression
	 * following an "elseif".  The arguments after the expression must
	 * be "then" (optional) and a script to execute if the expression is
	 * true.
	 */

	if (i >= objc) {
	    clause = Tcl_GetString(objv[i-1]);
	    Tcl_AppendResult(interp, "wrong # args: no expression after \"",
		    clause, "\" argument", (char *) NULL);
	    return TCL_ERROR;
	}
	if (!thenScriptIndex) {
	    result = Tcl_ExprBooleanObj(interp, objv[i], &value);
	    if (result != TCL_OK) {
		return result;
	    }
	}
	i++;
	if (i >= objc) {
	    missingScript:
	    clause = Tcl_GetString(objv[i-1]);
	    Tcl_AppendResult(interp, "wrong # args: no script following \"",
		    clause, "\" argument", (char *) NULL);
	    return TCL_ERROR;
	}
	clause = Tcl_GetString(objv[i]);
	if ((i < objc) && (strcmp(clause, "then") == 0)) {
	    i++;
	}
	if (i >= objc) {
	    goto missingScript;
	}
	if (value) {
	    thenScriptIndex = i;
	    value = 0;
	}
	
	/*
	 * The expression evaluated to false.  Skip the command, then
	 * see if there is an "else" or "elseif" clause.
	 */

	i++;
	if (i >= objc) {
	    if (thenScriptIndex) {
#ifndef TCL_TIP280
		return Tcl_EvalObjEx(interp, objv[thenScriptIndex], 0);
#else
		/* TIP #280. Make invoking context available to branch */
		return TclEvalObjEx(interp, objv[thenScriptIndex], 0,
				    iPtr->cmdFramePtr,thenScriptIndex);
#endif
	    }
	    return TCL_OK;
	}
	clause = Tcl_GetString(objv[i]);
	if ((clause[0] == 'e') && (strcmp(clause, "elseif") == 0)) {
	    i++;
	    continue;
	}
	break;
    }

    /*
     * Couldn't find a "then" or "elseif" clause to execute.  Check now
     * for an "else" clause.  We know that there's at least one more
     * argument when we get here.
     */

    if (strcmp(clause, "else") == 0) {
	i++;
	if (i >= objc) {
	    Tcl_AppendResult(interp,
		    "wrong # args: no script following \"else\" argument",
		    (char *) NULL);
	    return TCL_ERROR;
	}
    }
    if (i < objc - 1) {
	Tcl_AppendResult(interp,
		"wrong # args: extra words after \"else\" clause in \"if\" command",
		(char *) NULL);
	return TCL_ERROR;
    }
    if (thenScriptIndex) {
#ifndef TCL_TIP280
	return Tcl_EvalObjEx(interp, objv[thenScriptIndex], 0);
#else
	/* TIP #280. Make invoking context available to branch/else */
	return TclEvalObjEx(interp, objv[thenScriptIndex], 0,
			    iPtr->cmdFramePtr,thenScriptIndex);
#endif
    }
#ifndef TCL_TIP280
    return Tcl_EvalObjEx(interp, objv[i], 0);
#else
    return TclEvalObjEx(interp, objv[i], 0, iPtr->cmdFramePtr,i);
#endif
}

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_EvalEx`, lines 3925–3939. Retained reference `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c`.

```text
Tcl_EvalEx(interp, script, numBytes, flags)
    Tcl_Interp *interp;		/* Interpreter in which to evaluate the
				 * script.  Also used for error reporting. */
    CONST char *script;		/* First character of script to evaluate. */
    int numBytes;		/* Number of bytes in script.  If < 0, the
				 * script consists of all bytes up to the
				 * first null character. */
    int flags;			/* Collection of OR-ed bits that control
				 * the evaluation of the script.  Only
				 * TCL_EVAL_GLOBAL is currently
				 * supported. */
{
#ifdef TCL_TIP280
  return EvalEx (interp, script, numBytes, flags, 1, NULL, script);
}

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_EvalObjEx`, lines 4988–5002. Retained reference `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c`.

```text
Tcl_EvalObjEx(interp, objPtr, flags)
    Tcl_Interp *interp;			/* Token for command interpreter
					 * (returned by a previous call to
					 * Tcl_CreateInterp). */
    register Tcl_Obj *objPtr;		/* Pointer to object containing
					 * commands to execute. */
    int flags;				/* Collection of OR-ed bits that
					 * control the evaluation of the
					 * script.  Supported values are
					 * TCL_EVAL_GLOBAL and
					 * TCL_EVAL_DIRECT. */
{
#ifdef TCL_TIP280
  return TclEvalObjEx (interp, objPtr, flags, NULL, 0);
}

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclExecute.c`, function `conditional-jump-source-window`, lines 2270–2369. Retained reference `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclExecute.c`.

```text
    case INST_JUMP_FALSE4:
	opnd = 5;                             /* TRUE */
	pcAdjustment = TclGetInt4AtPtr(pc+1); /* FALSE */
	goto doJumpTrue;

    case INST_JUMP_TRUE4:
	opnd = TclGetInt4AtPtr(pc+1);         /* TRUE */
	pcAdjustment = 5;                     /* FALSE */
	goto doJumpTrue;

    case INST_JUMP_FALSE1:
	opnd = 2;                             /* TRUE */
	pcAdjustment = TclGetInt1AtPtr(pc+1); /* FALSE */
	goto doJumpTrue;

    case INST_JUMP_TRUE1:
	opnd = TclGetInt1AtPtr(pc+1);          /* TRUE */
	pcAdjustment = 2;                      /* FALSE */
	    
    doJumpTrue:
	{
	    int b;
		
	    valuePtr = stackPtr[stackTop];
	    if (valuePtr->typePtr == &tclIntType) {
		b = (valuePtr->internalRep.longValue != 0);
	    } else if (valuePtr->typePtr == &tclDoubleType) {
		b = (valuePtr->internalRep.doubleValue != 0.0);
	    } else if (valuePtr->typePtr == &tclWideIntType) {
		TclGetWide(w,valuePtr);
		b = (w != W0);
	    } else {
		result = Tcl_GetBooleanFromObj(interp, valuePtr, &b);
		if (result != TCL_OK) {
		    TRACE_WITH_OBJ(("%d => ERROR: ", opnd), Tcl_GetObjResult(interp));
		    goto checkForCatch;
		}
	    }
#ifndef TCL_COMPILE_DEBUG
	    NEXT_INST_F((b? opnd : pcAdjustment), 1, 0);
#else
	    if (b) {
		if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE1)) {
		    TRACE(("%d => %.20s true, new pc %u\n", opnd, O2S(valuePtr),
		            (unsigned int)(pc+opnd - codePtr->codeStart)));
		} else {
		    TRACE(("%d => %.20s true\n", pcAdjustment, O2S(valuePtr)));
		}
		NEXT_INST_F(opnd, 1, 0);
	    } else {
		if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE1)) {
		    TRACE(("%d => %.20s false\n", opnd, O2S(valuePtr)));
		} else {
		    opnd = pcAdjustment;
		    TRACE(("%d => %.20s false, new pc %u\n", opnd, O2S(valuePtr),
		            (unsigned int)(pc + opnd - codePtr->codeStart)));
		}
		NEXT_INST_F(pcAdjustment, 1, 0);
	    }
#endif
	}
	    	    
    case INST_LOR:
    case INST_LAND:
    {
	/*
	 * Operands must be boolean or numeric. No int->double
	 * conversions are performed.
	 */
		
	int i1, i2;
	int iResult;
	char *s;
	Tcl_ObjType *t1Ptr, *t2Ptr;

	value2Ptr = stackPtr[stackTop];
	valuePtr  = stackPtr[stackTop - 1];;
	t1Ptr = valuePtr->typePtr;
	t2Ptr = value2Ptr->typePtr;

	if ((t1Ptr == &tclIntType) || (t1Ptr == &tclBooleanType)) {
	    i1 = (valuePtr->internalRep.longValue != 0);
	} else if (t1Ptr == &tclWideIntType) {
	    TclGetWide(w,valuePtr);
	    i1 = (w != W0);
	} else if (t1Ptr == &tclDoubleType) {
	    i1 = (valuePtr->internalRep.doubleValue != 0.0);
	} else {
	    s = Tcl_GetStringFromObj(valuePtr, &length);
	    if (TclLooksLikeInt(s, length)) {
		GET_WIDE_OR_INT(result, valuePtr, i, w);
		if (valuePtr->typePtr == &tclIntType) {
		    i1 = (i != 0);
		} else {
		    i1 = (w != W0);
		}
	    } else {
		result = Tcl_GetBooleanFromObj((Tcl_Interp *) NULL,
					       valuePtr, &i1);
		i1 = (i1 != 0);

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclExecute.c`, function `logical-not-source-window`, lines 3765–3864. Retained reference `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclExecute.c`.

```text
    case INST_LNOT:
    {
	/*
	 * The operand must be numeric or a boolean string as
	 * accepted by Tcl_GetBooleanFromObj(). If the operand
	 * object is unshared modify it directly, otherwise
	 * create a copy to modify: this is "copy on write".
	 * Free any old string representation since it is now
	 * invalid.
	 */

	double d;
	int boolvar;
	Tcl_ObjType *tPtr;

	valuePtr = stackPtr[stackTop];
	tPtr = valuePtr->typePtr;
	if (!IS_INTEGER_TYPE(tPtr) && ((tPtr != &tclDoubleType)
	        || (valuePtr->bytes != NULL))) {
	    if ((tPtr == &tclBooleanType) && (valuePtr->bytes == NULL)) {
		valuePtr->typePtr = &tclIntType;
	    } else {
		char *s = Tcl_GetStringFromObj(valuePtr, &length);
		if (TclLooksLikeInt(s, length)) {
		    GET_WIDE_OR_INT(result, valuePtr, i, w);
		} else {
		    result = Tcl_GetDoubleFromObj((Tcl_Interp *) NULL,
		            valuePtr, &d);
		}
		if (result == TCL_ERROR && *pc == INST_LNOT) {
		    result = Tcl_GetBooleanFromObj((Tcl_Interp *)NULL,
		            valuePtr, &boolvar);
		    i = (long)boolvar; /* i is long, not int! */
		}
		if (result != TCL_OK) {
		    TRACE(("\"%.20s\" => ILLEGAL TYPE %s\n",
		            s, (tPtr? tPtr->name : "null")));
		    DECACHE_STACK_INFO();
		    IllegalExprOperandType(interp, pc, valuePtr);
		    CACHE_STACK_INFO();
		    goto checkForCatch;
		}
	    }
	    tPtr = valuePtr->typePtr;
	}

	if (Tcl_IsShared(valuePtr)) {
	    /*
	     * Create a new object.
	     */
	    if ((tPtr == &tclIntType) || (tPtr == &tclBooleanType)) {
		i = valuePtr->internalRep.longValue;
		objResultPtr = Tcl_NewLongObj(
		    (*pc == INST_UMINUS)? -i : !i);
		TRACE_WITH_OBJ(("%ld => ", i), objResultPtr);
	    } else if (tPtr == &tclWideIntType) {
		TclGetWide(w,valuePtr);
		if (*pc == INST_UMINUS) {
		    objResultPtr = Tcl_NewWideIntObj(-w);
		} else {
		    objResultPtr = Tcl_NewLongObj(w == W0);
		}
		TRACE_WITH_OBJ((LLD" => ", w), objResultPtr);
	    } else {
		d = valuePtr->internalRep.doubleValue;
		if (*pc == INST_UMINUS) {
		    objResultPtr = Tcl_NewDoubleObj(-d);
		} else {
		    /*
		     * Should be able to use "!d", but apparently
		     * some compilers can't handle it.
		     */
		    objResultPtr = Tcl_NewLongObj((d==0.0)? 1 : 0);
		}
		TRACE_WITH_OBJ(("%.6g => ", d), objResultPtr);
	    }
	    NEXT_INST_F(1, 1, 1);
	} else {
	    /*
	     * valuePtr is unshared. Modify it directly.
	     */
	    if ((tPtr == &tclIntType) || (tPtr == &tclBooleanType)) {
		i = valuePtr->internalRep.longValue;
		Tcl_SetLongObj(valuePtr,
	                (*pc == INST_UMINUS)? -i : !i);
		TRACE_WITH_OBJ(("%ld => ", i), valuePtr);
	    } else if (tPtr == &tclWideIntType) {
		TclGetWide(w,valuePtr);
		if (*pc == INST_UMINUS) {
		    Tcl_SetWideIntObj(valuePtr, -w);
		} else {
		    Tcl_SetLongObj(valuePtr, w == W0);
		}
		TRACE_WITH_OBJ((LLD" => ", w), valuePtr);
	    } else {
		d = valuePtr->internalRep.doubleValue;
		if (*pc == INST_UMINUS) {
		    Tcl_SetDoubleObj(valuePtr, -d);
		} else {
		    /*

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclExecute.c`, function `logical-AND-OR-source-window`, lines 2332–2481. Retained reference `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclExecute.c`.

```text
    case INST_LOR:
    case INST_LAND:
    {
	/*
	 * Operands must be boolean or numeric. No int->double
	 * conversions are performed.
	 */
		
	int i1, i2;
	int iResult;
	char *s;
	Tcl_ObjType *t1Ptr, *t2Ptr;

	value2Ptr = stackPtr[stackTop];
	valuePtr  = stackPtr[stackTop - 1];;
	t1Ptr = valuePtr->typePtr;
	t2Ptr = value2Ptr->typePtr;

	if ((t1Ptr == &tclIntType) || (t1Ptr == &tclBooleanType)) {
	    i1 = (valuePtr->internalRep.longValue != 0);
	} else if (t1Ptr == &tclWideIntType) {
	    TclGetWide(w,valuePtr);
	    i1 = (w != W0);
	} else if (t1Ptr == &tclDoubleType) {
	    i1 = (valuePtr->internalRep.doubleValue != 0.0);
	} else {
	    s = Tcl_GetStringFromObj(valuePtr, &length);
	    if (TclLooksLikeInt(s, length)) {
		GET_WIDE_OR_INT(result, valuePtr, i, w);
		if (valuePtr->typePtr == &tclIntType) {
		    i1 = (i != 0);
		} else {
		    i1 = (w != W0);
		}
	    } else {
		result = Tcl_GetBooleanFromObj((Tcl_Interp *) NULL,
					       valuePtr, &i1);
		i1 = (i1 != 0);
	    }
	    if (result != TCL_OK) {
		TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		        (t1Ptr? t1Ptr->name : "null")));
		DECACHE_STACK_INFO();
		IllegalExprOperandType(interp, pc, valuePtr);
		CACHE_STACK_INFO();
		goto checkForCatch;
	    }
	}
		
	if ((t2Ptr == &tclIntType) || (t2Ptr == &tclBooleanType)) {
	    i2 = (value2Ptr->internalRep.longValue != 0);
	} else if (t2Ptr == &tclWideIntType) {
	    TclGetWide(w,value2Ptr);
	    i2 = (w != W0);
	} else if (t2Ptr == &tclDoubleType) {
	    i2 = (value2Ptr->internalRep.doubleValue != 0.0);
	} else {
	    s = Tcl_GetStringFromObj(value2Ptr, &length);
	    if (TclLooksLikeInt(s, length)) {
		GET_WIDE_OR_INT(result, value2Ptr, i, w);
		if (value2Ptr->typePtr == &tclIntType) {
		    i2 = (i != 0);
		} else {
		    i2 = (w != W0);
		}
	    } else {
		result = Tcl_GetBooleanFromObj((Tcl_Interp *) NULL, value2Ptr, &i2);
	    }
	    if (result != TCL_OK) {
		TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(value2Ptr),
		        (t2Ptr? t2Ptr->name : "null")));
		DECACHE_STACK_INFO();
		IllegalExprOperandType(interp, pc, value2Ptr);
		CACHE_STACK_INFO();
		goto checkForCatch;
	    }
	}

	/*
	 * Reuse the valuePtr object already on stack if possible.
	 */
	
	if (*pc == INST_LOR) {
	    iResult = (i1 || i2);
	} else {
	    iResult = (i1 && i2);
	}
	if (Tcl_IsShared(valuePtr)) {
	    objResultPtr = Tcl_NewLongObj(iResult);
	    TRACE(("%.20s %.20s => %d\n", O2S(valuePtr), O2S(value2Ptr), iResult));
	    NEXT_INST_F(1, 2, 1);
	} else {	/* reuse the valuePtr object */
	    TRACE(("%.20s %.20s => %d\n", O2S(valuePtr), O2S(value2Ptr), iResult));
	    Tcl_SetLongObj(valuePtr, iResult);
	    NEXT_INST_F(1, 1, 0);
	}
    }

    /*
     * ---------------------------------------------------------
     *     Start of INST_LIST and related instructions.
     */

    case INST_LIST:
	/*
	 * Pop the opnd (objc) top stack elements into a new list obj
	 * and then decrement their ref counts. 
	 */

	opnd = TclGetUInt4AtPtr(pc+1);
	objResultPtr = Tcl_NewListObj(opnd, &(stackPtr[stackTop - (opnd-1)]));
	TRACE_WITH_OBJ(("%u => ", opnd), objResultPtr);
	NEXT_INST_V(5, opnd, 1);

    case INST_LIST_LENGTH:
	valuePtr = stackPtr[stackTop];

	result = Tcl_ListObjLength(interp, valuePtr, &length);
	if (result != TCL_OK) {
	    TRACE_WITH_OBJ(("%.30s => ERROR: ", O2S(valuePtr)),
	            Tcl_GetObjResult(interp));
	    goto checkForCatch;
	}
	objResultPtr = Tcl_NewIntObj(length);
	TRACE(("%.20s => %d\n", O2S(valuePtr), length));
	NEXT_INST_F(1, 1, 1);
	    
    case INST_LIST_INDEX:
	/*** lindex with objc == 3 ***/
		
	/*
	 * Pop the two operands
	 */
	value2Ptr = stackPtr[stackTop];
	valuePtr  = stackPtr[stackTop- 1];

	/*
	 * Extract the desired list element
	 */
	objResultPtr = TclLindexList(interp, valuePtr, value2Ptr);
	if (objResultPtr == NULL) {
	    TRACE_WITH_OBJ(("%.30s %.30s => ERROR: ", O2S(valuePtr), O2S(value2Ptr)),
	            Tcl_GetObjResult(interp));
	    result = TCL_ERROR;
	    goto checkForCatch;
	}

	/*
	 * Stash the list element on the stack
	 */

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclObj.c`, function `Tcl_GetBooleanFromObj`, lines 1871–1915. Retained reference `native_dict_write_error_lifetime226-failed-request225-original-source-tclObj.c`.

```text
Tcl_GetBooleanFromObj(
    Tcl_Interp *interp, 	/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr,	/* The object from which to get boolean. */
    register int *boolPtr)	/* Place to store resulting boolean. */
{
    do {
	if (objPtr->typePtr == &tclIntType) {
	    *boolPtr = (objPtr->internalRep.longValue != 0);
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclBooleanType) {
	    *boolPtr = (int) objPtr->internalRep.longValue;
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclDoubleType) {
	    /*
	     * Caution: Don't be tempted to check directly for the "double"
	     * Tcl_ObjType and then compare the intrep to 0.0. This isn't
	     * reliable because a "double" Tcl_ObjType can hold the NaN value.
	     * Use the API Tcl_GetDoubleFromObj, which does the checking and
	     * sets the proper error message for us.
	     */

            double d;

	    if (Tcl_GetDoubleFromObj(interp, objPtr, &d) != TCL_OK) {
		return TCL_ERROR;
	    }
	    *boolPtr = (d != 0.0);
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclBignumType) {
	    *boolPtr = 1;
	    return TCL_OK;
	}
#ifndef NO_WIDE_TYPE
	if (objPtr->typePtr == &tclWideIntType) {
	    *boolPtr = (objPtr->internalRep.wideValue != 0);
	    return TCL_OK;
	}
#endif
    } while ((ParseBoolean(objPtr) == TCL_OK) || (TCL_OK ==
	    TclParseNumber(interp, objPtr, "boolean value", NULL,-1,NULL,0)));
    return TCL_ERROR;
}

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_ExprBooleanObj`, lines 5585–5601. Retained reference `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c`.

```text
Tcl_ExprBooleanObj(
    Tcl_Interp *interp,		/* Context in which to evaluate the
				 * expression. */
    register Tcl_Obj *objPtr,	/* Expression to evaluate. */
    int *ptr)			/* Where to store 0/1 result. */
{
    Tcl_Obj *resultPtr;
    int result;

    result = Tcl_ExprObj(interp, objPtr, &resultPtr);
    if (result == TCL_OK) {
	result = Tcl_GetBooleanFromObj(interp, resultPtr, ptr);
	Tcl_DecrRefCount(resultPtr);
				/* Discard the result object. */
    }
    return result;
}

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `Tcl_IfObjCmd`, lines 203–309. Retained reference `native_info_inventory_original-sources-8.5.19-tclCmdIL.c`.

```text
Tcl_IfObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int thenScriptIndex = 0;	/* "then" script to be evaled after syntax
				 * check. */
    Interp *iPtr = (Interp *) interp;
    int i, result, value;
    char *clause;

    i = 1;
    while (1) {
	/*
	 * At this point in the loop, objv and objc refer to an expression to
	 * test, either for the main expression or an expression following an
	 * "elseif". The arguments after the expression must be "then"
	 * (optional) and a script to execute if the expression is true.
	 */

	if (i >= objc) {
	    clause = TclGetString(objv[i-1]);
	    Tcl_AppendResult(interp, "wrong # args: ",
		    "no expression after \"", clause, "\" argument", NULL);
	    return TCL_ERROR;
	}
	if (!thenScriptIndex) {
	    result = Tcl_ExprBooleanObj(interp, objv[i], &value);
	    if (result != TCL_OK) {
		return result;
	    }
	}
	i++;
	if (i >= objc) {
	missingScript:
	    clause = TclGetString(objv[i-1]);
	    Tcl_AppendResult(interp, "wrong # args: ",
		    "no script following \"", clause, "\" argument", NULL);
	    return TCL_ERROR;
	}
	clause = TclGetString(objv[i]);
	if ((i < objc) && (strcmp(clause, "then") == 0)) {
	    i++;
	}
	if (i >= objc) {
	    goto missingScript;
	}
	if (value) {
	    thenScriptIndex = i;
	    value = 0;
	}

	/*
	 * The expression evaluated to false. Skip the command, then see if
	 * there is an "else" or "elseif" clause.
	 */

	i++;
	if (i >= objc) {
	    if (thenScriptIndex) {
		/*
		 * TIP #280. Make invoking context available to branch.
		 */

		return TclEvalObjEx(interp, objv[thenScriptIndex], 0,
			iPtr->cmdFramePtr, thenScriptIndex);
	    }
	    return TCL_OK;
	}
	clause = TclGetString(objv[i]);
	if ((clause[0] == 'e') && (strcmp(clause, "elseif") == 0)) {
	    i++;
	    continue;
	}
	break;
    }

    /*
     * Couldn't find a "then" or "elseif" clause to execute. Check now for an
     * "else" clause. We know that there's at least one more argument when we
     * get here.
     */

    if (strcmp(clause, "else") == 0) {
	i++;
	if (i >= objc) {
	    Tcl_AppendResult(interp, "wrong # args: ",
		    "no script following \"else\" argument", NULL);
	    return TCL_ERROR;
	}
    }
    if (i < objc - 1) {
	Tcl_AppendResult(interp, "wrong # args: ",
		"extra words after \"else\" clause in \"if\" command", NULL);
	return TCL_ERROR;
    }
    if (thenScriptIndex) {
	/*
	 * TIP #280. Make invoking context available to branch/else.
	 */

	return TclEvalObjEx(interp, objv[thenScriptIndex], 0,
		iPtr->cmdFramePtr, thenScriptIndex);
    }
    return TclEvalObjEx(interp, objv[i], 0, iPtr->cmdFramePtr, i);
}

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_EvalEx`, lines 4079–4091. Retained reference `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c`.

```text
Tcl_EvalEx(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * script. Also used for error reporting. */
    const char *script,		/* First character of script to evaluate. */
    int numBytes,		/* Number of bytes in script. If < 0, the
				 * script consists of all bytes up to the
				 * first null character. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL is currently supported. */
{
  return TclEvalEx(interp, script, numBytes, flags, 1, NULL, script);
}

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_EvalObjEx`, lines 5076–5086. Retained reference `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c`.

```text
Tcl_EvalObjEx(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * a previous call to Tcl_CreateInterp). */
    register Tcl_Obj *objPtr,	/* Pointer to object containing commands to
				 * execute. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Supported values
				 * are TCL_EVAL_GLOBAL and TCL_EVAL_DIRECT. */
{
    return TclEvalObjEx(interp, objPtr, flags, NULL, 0);
}

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclExecute.c`, function `conditional-jump-source-window`, lines 3595–3694. Retained reference `native_dict_write_error_lifetime226-failed-request225-original-source-tclExecute.c`.

```text
    case INST_JUMP_FALSE4:
	jmpOffset[0] = TclGetInt4AtPtr(pc+1);	/* FALSE offset */
	jmpOffset[1] = 5;			/* TRUE offset*/
	goto doCondJump;

    case INST_JUMP_TRUE4:
	jmpOffset[0] = 5;
	jmpOffset[1] = TclGetInt4AtPtr(pc+1);
	goto doCondJump;

    case INST_JUMP_FALSE1:
	jmpOffset[0] = TclGetInt1AtPtr(pc+1);
	jmpOffset[1] = 2;
	goto doCondJump;

    case INST_JUMP_TRUE1:
	jmpOffset[0] = 2;
	jmpOffset[1] = TclGetInt1AtPtr(pc+1);

    doCondJump:
	valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	result = TclGetBooleanFromObj(interp, valuePtr, &b);
	if (result != TCL_OK) {
	    TRACE_WITH_OBJ(("%d => ERROR: ", jmpOffset[
		    ((*pc == INST_JUMP_FALSE1) || (*pc == INST_JUMP_FALSE4))
		    ? 0 : 1]), Tcl_GetObjResult(interp));
	    goto checkForCatch;
	}

#ifdef TCL_COMPILE_DEBUG
	if (b) {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE(("%d => %.20s true, new pc %u\n", jmpOffset[1],
			O2S(valuePtr),
			(unsigned)(pc + jmpOffset[1] - codePtr->codeStart)));
	    } else {
		TRACE(("%d => %.20s true\n", jmpOffset[0], O2S(valuePtr)));
	    }
	} else {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE(("%d => %.20s false\n", jmpOffset[0], O2S(valuePtr)));
	    } else {
		TRACE(("%d => %.20s false, new pc %u\n", jmpOffset[0],
			O2S(valuePtr),
			(unsigned)(pc + jmpOffset[1] - codePtr->codeStart)));
	    }
	}
#endif
	NEXT_INST_F(jmpOffset[b], 1, 0);
    }

    case INST_JUMP_TABLE: {
	Tcl_HashEntry *hPtr;
	JumptableInfo *jtPtr;
	int opnd;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails.
	 */

	opnd = TclGetInt4AtPtr(pc+1);
	jtPtr = (JumptableInfo *) codePtr->auxDataArrayPtr[opnd].clientData;
	TRACE(("%d => %.20s ", opnd, O2S(OBJ_AT_TOS)));
	hPtr = Tcl_FindHashEntry(&jtPtr->hashTable, TclGetString(OBJ_AT_TOS));
	if (hPtr != NULL) {
	    int jumpOffset = PTR2INT(Tcl_GetHashValue(hPtr));

	    TRACE_APPEND(("found in table, new pc %u\n",
		    (unsigned)(pc - codePtr->codeStart + jumpOffset)));
	    NEXT_INST_F(jumpOffset, 1, 0);
	} else {
	    TRACE_APPEND(("not found in table\n"));
	    NEXT_INST_F(5, 1, 0);
	}
    }

    /*
     * These two instructions are now redundant: the complete logic of the LOR
     * and LAND is now handled by the expression compiler.
     */

    case INST_LOR:
    case INST_LAND: {
	/*
	 * Operands must be boolean or numeric. No int->double conversions are
	 * performed.
	 */

	int i1, i2, iResult;
	Tcl_Obj *value2Ptr = OBJ_AT_TOS;
	Tcl_Obj *valuePtr = OBJ_UNDER_TOS;

	result = TclGetBooleanFromObj(NULL, valuePtr, &i1);
	if (result != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclExecute.c`, function `logical-not-source-window`, lines 6275–6374. Retained reference `native_dict_write_error_lifetime226-failed-request225-original-source-tclExecute.c`.

```text
    case INST_LNOT: {
	int b;
	Tcl_Obj *valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	result = TclGetBooleanFromObj(NULL, valuePtr, &b);
	if (result != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s\n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}
	/* TODO: Consider peephole opt. */
	objResultPtr = constants[!b];
	NEXT_INST_F(1, 1, 1);
    }

    case INST_BITNOT: {
	mp_int big;
	ClientData ptr;
	int type;
	Tcl_Obj *valuePtr = OBJ_AT_TOS;

	result = GetNumberFromObj(NULL, valuePtr, &ptr, &type);
	if ((result != TCL_OK)
		|| (type == TCL_NUMBER_NAN) || (type == TCL_NUMBER_DOUBLE)) {
	    /*
	     * ... ~$NonInteger => raise an error.
	     */

	    result = TCL_ERROR;
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}
	if (type == TCL_NUMBER_LONG) {
	    long l = *((const long *)ptr);

	    if (Tcl_IsShared(valuePtr)) {
		TclNewLongObj(objResultPtr, ~l);
		NEXT_INST_F(1, 1, 1);
	    }
	    TclSetLongObj(valuePtr, ~l);
	    NEXT_INST_F(1, 0, 0);
	}
#ifndef NO_WIDE_TYPE
	if (type == TCL_NUMBER_WIDE) {
	    Tcl_WideInt w = *((const Tcl_WideInt *)ptr);

	    if (Tcl_IsShared(valuePtr)) {
		objResultPtr = Tcl_NewWideIntObj(~w);
		NEXT_INST_F(1, 1, 1);
	    }
	    Tcl_SetWideIntObj(valuePtr, ~w);
	    NEXT_INST_F(1, 0, 0);
	}
#endif
	Tcl_TakeBignumFromObj(NULL, valuePtr, &big);
	/* ~a = - a - 1 */
	mp_neg(&big, &big);
	mp_sub_d(&big, 1, &big);
	if (Tcl_IsShared(valuePtr)) {
	    objResultPtr = Tcl_NewBignumObj(&big);
	    NEXT_INST_F(1, 1, 1);
	}
	Tcl_SetBignumObj(valuePtr, &big);
	NEXT_INST_F(1, 0, 0);
    }

    case INST_UMINUS: {
	ClientData ptr;
	int type;
	Tcl_Obj *valuePtr = OBJ_AT_TOS;

	result = GetNumberFromObj(NULL, valuePtr, &ptr, &type);
	if ((result != TCL_OK)
#ifndef ACCEPT_NAN
		|| (type == TCL_NUMBER_NAN)
#endif
		) {
	    result = TCL_ERROR;
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}
	switch (type) {
	case TCL_NUMBER_DOUBLE: {
	    double d;

	    if (Tcl_IsShared(valuePtr)) {
		TclNewDoubleObj(objResultPtr, -(*((const double *)ptr)));

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclExecute.c`, function `logical-AND-OR-source-window`, lines 3680–3829. Retained reference `native_dict_write_error_lifetime226-failed-request225-original-source-tclExecute.c`.

```text
    case INST_LOR:
    case INST_LAND: {
	/*
	 * Operands must be boolean or numeric. No int->double conversions are
	 * performed.
	 */

	int i1, i2, iResult;
	Tcl_Obj *value2Ptr = OBJ_AT_TOS;
	Tcl_Obj *valuePtr = OBJ_UNDER_TOS;

	result = TclGetBooleanFromObj(NULL, valuePtr, &i1);
	if (result != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}

	result = TclGetBooleanFromObj(NULL, value2Ptr, &i2);
	if (result != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(value2Ptr),
		    (value2Ptr->typePtr? value2Ptr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, value2Ptr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}

	if (*pc == INST_LOR) {
	    iResult = (i1 || i2);
	} else {
	    iResult = (i1 && i2);
	}
	objResultPtr = constants[iResult];
	TRACE(("%.20s %.20s => %d\n", O2S(valuePtr),O2S(value2Ptr),iResult));
	NEXT_INST_F(1, 2, 1);
    }

    /*
     * ---------------------------------------------------------
     *	   Start of INST_LIST and related instructions.
     */

    case INST_LIST: {
	/*
	 * Pop the opnd (objc) top stack elements into a new list obj and then
	 * decrement their ref counts.
	 */

	int opnd;

	opnd = TclGetUInt4AtPtr(pc+1);
	objResultPtr = Tcl_NewListObj(opnd, &OBJ_AT_DEPTH(opnd-1));
	TRACE_WITH_OBJ(("%u => ", opnd), objResultPtr);
	NEXT_INST_V(5, opnd, 1);
    }

    case INST_LIST_LENGTH: {
	Tcl_Obj *valuePtr;
	int length;

	valuePtr = OBJ_AT_TOS;

	result = TclListObjLength(interp, valuePtr, &length);
	if (result == TCL_OK) {
	    TclNewIntObj(objResultPtr, length);
	    TRACE(("%.20s => %d\n", O2S(valuePtr), length));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_WITH_OBJ(("%.30s => ERROR: ", O2S(valuePtr)),
		    Tcl_GetObjResult(interp));
	    goto checkForCatch;
	}
    }

    case INST_LIST_INDEX: {
	/*** lindex with objc == 3 ***/

	/* Variables also for INST_LIST_INDEX_IMM */

	int listc, idx, opnd, pcAdjustment;
	Tcl_Obj **listv;
	Tcl_Obj *valuePtr, *value2Ptr;

	/*
	 * Pop the two operands.
	 */

	value2Ptr = OBJ_AT_TOS;
	valuePtr = OBJ_UNDER_TOS;

	/*
	 * Extract the desired list element.
	 */

	result = TclListObjGetElements(interp, valuePtr, &listc, &listv);
	if ((result == TCL_OK) && (value2Ptr->typePtr != &tclListType)
		&& (TclGetIntForIndexM(NULL , value2Ptr, listc-1,
			&idx) == TCL_OK)) {
	    TclDecrRefCount(value2Ptr);
	    tosPtr--;
	    pcAdjustment = 1;
	    goto lindexFastPath;
	}

	objResultPtr = TclLindexList(interp, valuePtr, value2Ptr);
	if (objResultPtr) {
	    /*
	     * Stash the list element on the stack.
	     */

	    TRACE(("%.20s %.20s => %s\n",
		    O2S(valuePtr), O2S(value2Ptr), O2S(objResultPtr)));
	    NEXT_INST_F(1, 2, -1);	/* Already has the correct refCount */
	} else {
	    TRACE_WITH_OBJ(("%.30s %.30s => ERROR: ", O2S(valuePtr),
		    O2S(value2Ptr)), Tcl_GetObjResult(interp));
	    result = TCL_ERROR;
	    goto checkForCatch;
	}

    case INST_LIST_INDEX_IMM:
	/*** lindex with objc==3 and index in bytecode stream ***/

	pcAdjustment = 5;

	/*
	 * Pop the list and get the index.
	 */

	valuePtr = OBJ_AT_TOS;
	opnd = TclGetInt4AtPtr(pc+1);

	/*
	 * Get the contents of the list, making sure that it really is a list
	 * in the process.
	 */

	result = TclListObjGetElements(interp, valuePtr, &listc, &listv);

	if (result == TCL_OK) {
	    /*
	     * Select the list item based on the index. Negative operand means
	     * end-based indexing.
	     */

	    if (opnd < -1) {

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclObj.c`, function `Tcl_GetBooleanFromObj`, lines 1885–1929. Retained reference `native_ensemble_map_prefix312-map-source-8.6.18-tclObj.c`.

```text
Tcl_GetBooleanFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* The object from which to get boolean. */
    int *intPtr)	/* Place to store resulting boolean. */
{
    do {
	if (objPtr->typePtr == &tclIntType) {
	    *intPtr = (objPtr->internalRep.longValue != 0);
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclBooleanType) {
	    *intPtr = (int) objPtr->internalRep.longValue;
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclDoubleType) {
	    /*
	     * Caution: Don't be tempted to check directly for the "double"
	     * Tcl_ObjType and then compare the internalrep to 0.0. This isn't
	     * reliable because a "double" Tcl_ObjType can hold the NaN value.
	     * Use the API Tcl_GetDoubleFromObj, which does the checking and
	     * sets the proper error message for us.
	     */

	    double d;

	    if (Tcl_GetDoubleFromObj(interp, objPtr, &d) != TCL_OK) {
		return TCL_ERROR;
	    }
	    *intPtr = (d != 0.0);
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclBignumType) {
	    *intPtr = 1;
	    return TCL_OK;
	}
#ifndef TCL_WIDE_INT_IS_LONG
	if (objPtr->typePtr == &tclWideIntType) {
	    *intPtr = (objPtr->internalRep.wideValue != 0);
	    return TCL_OK;
	}
#endif
    } while ((ParseBoolean(objPtr) == TCL_OK) || (TCL_OK ==
	    TclParseNumber(interp, objPtr, "boolean value", NULL,-1,NULL,0)));
    return TCL_ERROR;
}

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_ExprBooleanObj`, lines 6608–6624. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
Tcl_ExprBooleanObj(
    Tcl_Interp *interp,		/* Context in which to evaluate the
				 * expression. */
    Tcl_Obj *objPtr,	/* Expression to evaluate. */
    int *ptr)			/* Where to store 0/1 result. */
{
    Tcl_Obj *resultPtr;
    int result;

    result = Tcl_ExprObj(interp, objPtr, &resultPtr);
    if (result == TCL_OK) {
	result = Tcl_GetBooleanFromObj(interp, resultPtr, ptr);
	Tcl_DecrRefCount(resultPtr);
				/* Discard the result object. */
    }
    return result;
}

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `Tcl_IfObjCmd`, lines 205–212. Retained reference `native_info_inventory_original-sources-8.6.18-tclCmdIL.c`.

```text
Tcl_IfObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    return Tcl_NRCallObjProc(interp, TclNRIfObjCmd, dummy, objc, objv);
}

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_EvalEx`, lines 5065–5077. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
Tcl_EvalEx(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * script. Also used for error reporting. */
    const char *script,		/* First character of script to evaluate. */
    int numBytes,		/* Number of bytes in script. If < 0, the
				 * script consists of all bytes up to the
				 * first null character. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL is currently supported. */
{
    return TclEvalEx(interp, script, numBytes, flags, 1, NULL, script);
}

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_EvalObjEx`, lines 6077–6087. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
Tcl_EvalObjEx(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * a previous call to Tcl_CreateInterp). */
    Tcl_Obj *objPtr,		/* Pointer to object containing commands to
				 * execute. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Supported values
				 * are TCL_EVAL_GLOBAL and TCL_EVAL_DIRECT. */
{
    return TclEvalObjEx(interp, objPtr, flags, NULL, 0);
}

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclExecute.c`, function `conditional-jump-source-window`, lines 4574–4673. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclExecute.c`.

```text
    case INST_JUMP_FALSE4:
	jmpOffset[0] = TclGetInt4AtPtr(pc+1);	/* FALSE offset */
	jmpOffset[1] = 5;			/* TRUE offset */
	goto doCondJump;

    case INST_JUMP_TRUE4:
	jmpOffset[0] = 5;
	jmpOffset[1] = TclGetInt4AtPtr(pc+1);
	goto doCondJump;

    case INST_JUMP_FALSE1:
	jmpOffset[0] = TclGetInt1AtPtr(pc+1);
	jmpOffset[1] = 2;
	goto doCondJump;

    case INST_JUMP_TRUE1:
	jmpOffset[0] = 2;
	jmpOffset[1] = TclGetInt1AtPtr(pc+1);

    doCondJump:
	valuePtr = OBJ_AT_TOS;
	TRACE(("%d => ", jmpOffset[
		(*pc==INST_JUMP_FALSE1 || *pc==INST_JUMP_FALSE4) ? 0 : 1]));

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(interp, valuePtr, &b) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}

#ifdef TCL_COMPILE_DEBUG
	if (b) {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE_APPEND(("%.20s true, new pc %u\n", O2S(valuePtr),
			(unsigned)(pc + jmpOffset[1] - codePtr->codeStart)));
	    } else {
		TRACE_APPEND(("%.20s true\n", O2S(valuePtr)));
	    }
	} else {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE_APPEND(("%.20s false\n", O2S(valuePtr)));
	    } else {
		TRACE_APPEND(("%.20s false, new pc %u\n", O2S(valuePtr),
			(unsigned)(pc + jmpOffset[0] - codePtr->codeStart)));
	    }
	}
#endif
	NEXT_INST_F(jmpOffset[b], 1, 0);
    }
    break;

    case INST_JUMP_TABLE: {
	Tcl_HashEntry *hPtr;
	JumptableInfo *jtPtr;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails.
	 */

	opnd = TclGetInt4AtPtr(pc+1);
	jtPtr = (JumptableInfo *) codePtr->auxDataArrayPtr[opnd].clientData;
	TRACE(("%d \"%.20s\" => ", opnd, O2S(OBJ_AT_TOS)));
	hPtr = Tcl_FindHashEntry(&jtPtr->hashTable, TclGetString(OBJ_AT_TOS));
	if (hPtr != NULL) {
	    int jumpOffset = PTR2INT(Tcl_GetHashValue(hPtr));

	    TRACE_APPEND(("found in table, new pc %u\n",
		    (unsigned)(pc - codePtr->codeStart + jumpOffset)));
	    NEXT_INST_F(jumpOffset, 1, 0);
	} else {
	    TRACE_APPEND(("not found in table\n"));
	    NEXT_INST_F(5, 1, 0);
	}
    }
    break;

    /*
     * These two instructions are now redundant: the complete logic of the LOR
     * and LAND is now handled by the expression compiler.
     */

    case INST_LOR:
    case INST_LAND: {
	/*
	 * Operands must be boolean or numeric. No int->double conversions are
	 * performed.
	 */

	int i1, i2, iResult;

	value2Ptr = OBJ_AT_TOS;
	valuePtr = OBJ_UNDER_TOS;
	if (TclGetBooleanFromObj(NULL, valuePtr, &i1) != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclExecute.c`, function `logical-not-source-window`, lines 6612–6711. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclExecute.c`.

```text
    case INST_LNOT: {
	int b;

	valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(NULL, valuePtr, &b) != TCL_OK) {
	    TRACE(("\"%.20s\" => ERROR: illegal type %s\n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	/* TODO: Consider peephole opt. */
	objResultPtr = TCONST(!b);
	TRACE_WITH_OBJ(("%s => ", O2S(valuePtr)), objResultPtr);
	NEXT_INST_F(1, 1, 1);
    }

    case INST_BITNOT:
	valuePtr = OBJ_AT_TOS;
	TRACE(("\"%.20s\" => ", O2S(valuePtr)));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| (type1==TCL_NUMBER_NAN) || (type1==TCL_NUMBER_DOUBLE)) {
	    /*
	     * ... ~$NonInteger => raise an error.
	     */

	    TRACE_APPEND(("ERROR: illegal type %s\n",
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	if (type1 == TCL_NUMBER_LONG) {
	    l1 = *((const long *) ptr1);
	    if (Tcl_IsShared(valuePtr)) {
		TclNewLongObj(objResultPtr, ~l1);
		TRACE_APPEND(("%s\n", O2S(objResultPtr)));
		NEXT_INST_F(1, 1, 1);
	    }
	    TclSetLongObj(valuePtr, ~l1);
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}

    case INST_UMINUS:
	valuePtr = OBJ_AT_TOS;
	TRACE(("\"%.20s\" => ", O2S(valuePtr)));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| IsErroringNaNType(type1)) {
	    TRACE_APPEND(("ERROR: illegal type %s \n",
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	switch (type1) {
	case TCL_NUMBER_NAN:
	    /* -NaN => NaN */
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	break;
	case TCL_NUMBER_LONG:
	    l1 = *((const long *) ptr1);
	    if (l1 != LONG_MIN) {
		if (Tcl_IsShared(valuePtr)) {
		    TclNewLongObj(objResultPtr, -l1);
		    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
		    NEXT_INST_F(1, 1, 1);
		}
		TclSetLongObj(valuePtr, -l1);
		TRACE_APPEND(("%s\n", O2S(valuePtr)));
		NEXT_INST_F(1, 0, 0);
	    }
	    /* FALLTHROUGH */
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}

    case INST_UPLUS:
    case INST_TRY_CVT_TO_NUMERIC:

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclExecute.c`, function `logical-AND-OR-source-window`, lines 4657–4806. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclExecute.c`.

```text
    case INST_LOR:
    case INST_LAND: {
	/*
	 * Operands must be boolean or numeric. No int->double conversions are
	 * performed.
	 */

	int i1, i2, iResult;

	value2Ptr = OBJ_AT_TOS;
	valuePtr = OBJ_UNDER_TOS;
	if (TclGetBooleanFromObj(NULL, valuePtr, &i1) != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}

	if (TclGetBooleanFromObj(NULL, value2Ptr, &i2) != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(value2Ptr),
		    (value2Ptr->typePtr? value2Ptr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, value2Ptr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}

	if (*pc == INST_LOR) {
	    iResult = (i1 || i2);
	} else {
	    iResult = (i1 && i2);
	}
	objResultPtr = TCONST(iResult);
	TRACE(("%.20s %.20s => %d\n", O2S(valuePtr),O2S(value2Ptr),iResult));
	NEXT_INST_F(1, 2, 1);
    }
    break;

    /*
     * -----------------------------------------------------------------
     *	   Start of general introspector instructions.
     */

    case INST_NS_CURRENT: {
	Namespace *currNsPtr = (Namespace *) TclGetCurrentNamespace(interp);

	if (currNsPtr == (Namespace *) TclGetGlobalNamespace(interp)) {
	    TclNewLiteralStringObj(objResultPtr, "::");
	} else {
	    TclNewStringObj(objResultPtr, currNsPtr->fullName,
		    strlen(currNsPtr->fullName));
	}
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    }
    break;
    case INST_COROUTINE_NAME: {
	CoroutineData *corPtr = iPtr->execEnvPtr->corPtr;

	TclNewObj(objResultPtr);
	if (corPtr && !(corPtr->cmdPtr->flags & CMD_IS_DELETED)) {
	    Tcl_GetCommandFullName(interp, (Tcl_Command) corPtr->cmdPtr,
		    objResultPtr);
	}
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    }
    break;
    case INST_INFO_LEVEL_NUM:
	TclNewIntObj(objResultPtr, iPtr->varFramePtr->level);
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    break;
    case INST_INFO_LEVEL_ARGS: {
	int level;
	CallFrame *framePtr = iPtr->varFramePtr;
	CallFrame *rootFramePtr = iPtr->rootFramePtr;

	TRACE(("\"%.30s\" => ", O2S(OBJ_AT_TOS)));
	if (TclGetIntFromObj(interp, OBJ_AT_TOS, &level) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}
	if (level <= 0) {
	    level += framePtr->level;
	}
	for (; (framePtr->level!=level) && (framePtr!=rootFramePtr) ;
		framePtr = framePtr->callerVarPtr) {
	    /* Empty loop body */
	}
	if (framePtr == rootFramePtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad level \"%s\"", TclGetString(OBJ_AT_TOS)));
	    TRACE_ERROR(interp);
	    DECACHE_STACK_INFO();
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "STACK_LEVEL",
		    TclGetString(OBJ_AT_TOS), (char *)NULL);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	objResultPtr = Tcl_NewListObj(framePtr->objc, framePtr->objv);
	TRACE_APPEND(("%.30s\n", O2S(objResultPtr)));
	NEXT_INST_F(1, 1, 1);
    }
    {
	Tcl_Command cmd, origCmd;

    case INST_RESOLVE_COMMAND:
	cmd = Tcl_GetCommandFromObj(interp, OBJ_AT_TOS);
	TclNewObj(objResultPtr);
	if (cmd != NULL) {
	    Tcl_GetCommandFullName(interp, cmd, objResultPtr);
	}
	TRACE_WITH_OBJ(("\"%.20s\" => ", O2S(OBJ_AT_TOS)), objResultPtr);
	NEXT_INST_F(1, 1, 1);

    case INST_ORIGIN_COMMAND:
	TRACE(("\"%.30s\" => ", O2S(OBJ_AT_TOS)));
	cmd = Tcl_GetCommandFromObj(interp, OBJ_AT_TOS);
	if (cmd == NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid command name \"%s\"", TclGetString(OBJ_AT_TOS)));
	    DECACHE_STACK_INFO();
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "COMMAND",
		    TclGetString(OBJ_AT_TOS), (char *)NULL);
	    CACHE_STACK_INFO();
	    TRACE_APPEND(("ERROR: not command\n"));
	    goto gotError;
	}
	origCmd = TclGetOriginalCommand(cmd);
	if (origCmd == NULL) {
	    origCmd = cmd;
	}
	TclNewObj(objResultPtr);
	Tcl_GetCommandFullName(interp, origCmd, objResultPtr);
	TRACE_APPEND(("\"%.30s\"", O2S(OBJ_AT_TOS)));
	NEXT_INST_F(1, 1, 1);
    }

    /*
     * -----------------------------------------------------------------
     *	   Start of TclOO support instructions.
     */

    {
	Object *oPtr;
	CallFrame *framePtr;
	CallContext *contextPtr;

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclObj.c`, function `Tcl_GetBooleanFromObj`, lines 2071–2077. Retained reference `native_ensemble_map_prefix312-map-source-9.0.4-tclObj.c`.

```text
Tcl_GetBooleanFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* The object from which to get boolean. */
    int *intPtr)	/* Place to store resulting boolean. */
{
    return Tcl_GetBoolFromObj(interp, objPtr, (TCL_NULL_OK-2)&(int)sizeof(int), (char *)(void *)intPtr);
}

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_ExprBooleanObj`, lines 6696–6712. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
Tcl_ExprBooleanObj(
    Tcl_Interp *interp,		/* Context in which to evaluate the
				 * expression. */
    Tcl_Obj *objPtr,		/* Expression to evaluate. */
    int *ptr)			/* Where to store 0/1 result. */
{
    Tcl_Obj *resultPtr;
    int result;

    result = Tcl_ExprObj(interp, objPtr, &resultPtr);
    if (result == TCL_OK) {
	result = Tcl_GetBooleanFromObj(interp, resultPtr, ptr);
	Tcl_DecrRefCount(resultPtr);
				/* Discard the result object. */
    }
    return result;
}

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_IfObjCmd`, lines 204–211. Retained reference `native_info_inventory_original-sources-9.0.4-tclCmdIL.c`.

```text
Tcl_IfObjCmd(
    void *clientData,
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    return Tcl_NRCallObjProc(interp, TclNRIfObjCmd, clientData, objc, objv);
}

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_EvalEx`, lines 5228–5240. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
Tcl_EvalEx(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * script. Also used for error reporting. */
    const char *script,		/* First character of script to evaluate. */
    Tcl_Size numBytes,		/* Number of bytes in script. If -1, the
				 * script consists of all bytes up to the
				 * first null character. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL is currently supported. */
{
    return TclEvalEx(interp, script, numBytes, flags, 1, NULL, script);
}

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_EvalObjEx`, lines 6180–6190. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
Tcl_EvalObjEx(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * a previous call to Tcl_CreateInterp). */
    Tcl_Obj *objPtr,		/* Pointer to object containing commands to
				 * execute. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Supported values
				 * are TCL_EVAL_GLOBAL and TCL_EVAL_DIRECT. */
{
    return TclEvalObjEx(interp, objPtr, flags, NULL, 0);
}

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclExecute.c`, function `conditional-jump-source-window`, lines 4217–4316. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclExecute.c`.

```text
    case INST_JUMP_FALSE4:
	jmpOffset[0] = TclGetInt4AtPtr(pc + 1);	/* FALSE offset */
	jmpOffset[1] = 5;			/* TRUE offset */
	goto doCondJump;

    case INST_JUMP_TRUE4:
	jmpOffset[0] = 5;
	jmpOffset[1] = TclGetInt4AtPtr(pc + 1);
	goto doCondJump;

    case INST_JUMP_FALSE1:
	jmpOffset[0] = TclGetInt1AtPtr(pc + 1);
	jmpOffset[1] = 2;
	goto doCondJump;

    case INST_JUMP_TRUE1:
	jmpOffset[0] = 2;
	jmpOffset[1] = TclGetInt1AtPtr(pc + 1);

    doCondJump:
	valuePtr = OBJ_AT_TOS;
	TRACE(("%d => ", jmpOffset[
		(*pc==INST_JUMP_FALSE1 || *pc==INST_JUMP_FALSE4) ? 0 : 1]));

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(interp, valuePtr, &b) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}

#ifdef TCL_COMPILE_DEBUG
	if (b) {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE_APPEND(("%.20s true, new pc %" TCL_Z_MODIFIER "u\n", O2S(valuePtr),
			(size_t)(pc + jmpOffset[1] - codePtr->codeStart)));
	    } else {
		TRACE_APPEND(("%.20s true\n", O2S(valuePtr)));
	    }
	} else {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE_APPEND(("%.20s false\n", O2S(valuePtr)));
	    } else {
		TRACE_APPEND(("%.20s false, new pc %" TCL_Z_MODIFIER "u\n", O2S(valuePtr),
			(size_t)(pc + jmpOffset[0] - codePtr->codeStart)));
	    }
	}
#endif
	NEXT_INST_F(jmpOffset[b], 1, 0);
    }
    break;

    case INST_JUMP_TABLE: {
	Tcl_HashEntry *hPtr;
	JumptableInfo *jtPtr;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails.
	 */

	opnd = TclGetInt4AtPtr(pc + 1);
	jtPtr = (JumptableInfo *) codePtr->auxDataArrayPtr[opnd].clientData;
	TRACE(("%d \"%.20s\" => ", opnd, O2S(OBJ_AT_TOS)));
	hPtr = Tcl_FindHashEntry(&jtPtr->hashTable, TclGetString(OBJ_AT_TOS));
	if (hPtr != NULL) {
	    Tcl_Size jumpOffset = PTR2INT(Tcl_GetHashValue(hPtr));

	    TRACE_APPEND(("found in table, new pc %" TCL_Z_MODIFIER "u\n",
		    (size_t)(pc - codePtr->codeStart + jumpOffset)));
	    NEXT_INST_F(jumpOffset, 1, 0);
	} else {
	    TRACE_APPEND(("not found in table\n"));
	    NEXT_INST_F(5, 1, 0);
	}
    }
    break;

    /*
     * -----------------------------------------------------------------
     *	   Start of general introspector instructions.
     */

    case INST_NS_CURRENT:
	objResultPtr = TclNewNamespaceObj(TclGetCurrentNamespace(interp));
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    break;
    case INST_COROUTINE_NAME: {
	CoroutineData *corPtr = iPtr->execEnvPtr->corPtr;

	TclNewObj(objResultPtr);
	if (corPtr && !(corPtr->cmdPtr->flags & CMD_DYING)) {
	    Tcl_GetCommandFullName(interp, (Tcl_Command) corPtr->cmdPtr,
		    objResultPtr);
	}
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    }
    break;

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclExecute.c`, function `logical-not-source-window`, lines 6214–6313. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclExecute.c`.

```text
    case INST_LNOT: {
	int b;

	valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(NULL, valuePtr, &b) != TCL_OK) {
	    TRACE(("\"%.20s\" => ERROR: illegal type %s\n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	/* TODO: Consider peephole opt. */
	objResultPtr = TCONST(!b);
	TRACE_WITH_OBJ(("%s => ", O2S(valuePtr)), objResultPtr);
	NEXT_INST_F(1, 1, 1);
    }

    case INST_BITNOT:
	valuePtr = OBJ_AT_TOS;
	TRACE(("\"%.20s\" => ", O2S(valuePtr)));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| (type1==TCL_NUMBER_NAN) || (type1==TCL_NUMBER_DOUBLE)) {
	    /*
	     * ... ~$NonInteger => raise an error.
	     */

	    TRACE_APPEND(("ERROR: illegal type %s\n",
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	if (type1 == TCL_NUMBER_INT) {
	    w1 = *((const Tcl_WideInt *) ptr1);
	    if (Tcl_IsShared(valuePtr)) {
		TclNewIntObj(objResultPtr, ~w1);
		TRACE_APPEND(("%s\n", O2S(objResultPtr)));
		NEXT_INST_F(1, 1, 1);
	    }
	    TclSetIntObj(valuePtr, ~w1);
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}

    case INST_UMINUS:
	valuePtr = OBJ_AT_TOS;
	TRACE(("\"%.20s\" => ", O2S(valuePtr)));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| IsErroringNaNType(type1)) {
	    TRACE_APPEND(("ERROR: illegal type %s \n",
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	switch (type1) {
	case TCL_NUMBER_NAN:
	    /* -NaN => NaN */
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	break;
	case TCL_NUMBER_INT:
	    w1 = *((const Tcl_WideInt *) ptr1);
	    if (w1 != WIDE_MIN) {
		if (Tcl_IsShared(valuePtr)) {
		    TclNewIntObj(objResultPtr, -w1);
		    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
		    NEXT_INST_F(1, 1, 1);
		}
		TclSetIntObj(valuePtr, -w1);
		TRACE_APPEND(("%s\n", O2S(valuePtr)));
		NEXT_INST_F(1, 0, 0);
	    }
	    /* FALLTHROUGH */
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}

    case INST_UPLUS:
    case INST_TRY_CVT_TO_NUMERIC:

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclObj.c`, function `Tcl_GetBooleanFromObj`, lines 2021–2028. Retained reference `native_ensemble_map_prefix312-map-source-9.1.0-tclObj.c`.

```text
Tcl_GetBooleanFromObj(
    Tcl_Interp *interp,		/* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,		/* The object from which to get boolean. */
    int *intPtr)		/* Place to store resulting boolean. */
{
    return Tcl_GetBoolFromObj(interp, objPtr, (TCL_NULL_OK-2)&(int)sizeof(int),
	    (char *)(void *)intPtr);
}

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_ExprBooleanObj`, lines 6653–6669. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
Tcl_ExprBooleanObj(
    Tcl_Interp *interp,		/* Context in which to evaluate the
				 * expression. */
    Tcl_Obj *objPtr,		/* Expression to evaluate. */
    int *ptr)			/* Where to store 0/1 result. */
{
    Tcl_Obj *resultPtr;
    int result;

    result = Tcl_ExprObj(interp, objPtr, &resultPtr);
    if (result == TCL_OK) {
	result = Tcl_GetBooleanFromObj(interp, resultPtr, ptr);
	Tcl_DecrRefCount(resultPtr);
				/* Discard the result object. */
    }
    return result;
}

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_IfObjCmd`, lines 205–212. Retained reference `native_info_inventory_original-sources-9.1.0-tclCmdIL.c`.

```text
Tcl_IfObjCmd(
    void *clientData,
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    return Tcl_NRCallObjProc2(interp, TclNRIfObjCmd, clientData, objc, objv);
}

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_EvalEx`, lines 5196–5208. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
Tcl_EvalEx(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * script. Also used for error reporting. */
    const char *script,		/* First character of script to evaluate. */
    Tcl_Size numBytes,		/* Number of bytes in script. If -1, the
				 * script consists of all bytes up to the
				 * first null character. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL is currently supported. */
{
    return TclEvalEx(interp, script, numBytes, flags, 1, NULL, script);
}

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_EvalObjEx`, lines 6137–6147. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
Tcl_EvalObjEx(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * a previous call to Tcl_CreateInterp). */
    Tcl_Obj *objPtr,		/* Pointer to object containing commands to
				 * execute. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Supported values
				 * are TCL_EVAL_GLOBAL and TCL_EVAL_DIRECT. */
{
    return TclEvalObjEx(interp, objPtr, flags, NULL, 0);
}

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclExecute.c`, function `conditional-jump-source-window`, lines 4507–4606. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclExecute.c`.

```text
    case INST_JUMP_FALSE1:
	DEPRECATED_OPCODE_MARK(INST_JUMP_FALSE1);
	jmpOffset[0] = TclGetInt1AtPtr(pc + 1);
	jmpOffset[1] = 2;
	TRACE("%d => ", jmpOffset[0]);
	goto doCondJump;

    case INST_JUMP_TRUE1:
	DEPRECATED_OPCODE_MARK(INST_JUMP_TRUE1);
	jmpOffset[0] = 2;
	jmpOffset[1] = TclGetInt1AtPtr(pc + 1);
	TRACE("%d => ", jmpOffset[1]);
	goto doCondJump;
#endif

    case INST_JUMP_FALSE:
	jmpOffset[0] = TclGetInt4AtPtr(pc + 1);	/* FALSE offset */
	jmpOffset[1] = 5;			/* TRUE offset */
	TRACE("%d => ", jmpOffset[0]);
	goto doCondJump;

    case INST_JUMP_TRUE:
	jmpOffset[0] = 5;
	jmpOffset[1] = TclGetInt4AtPtr(pc + 1);
	TRACE("%d => ", jmpOffset[1]);

    doCondJump:
	valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(interp, valuePtr, &b) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}

#ifdef TCL_COMPILE_DEBUG
	if (b) {
	    if ((*pc == INST_JUMP_TRUE)
#ifndef REMOVE_DEPRECATED_OPCODES
		    ||  (*pc == INST_JUMP_TRUE1)
#endif
		    ) {
		TRACE_APPEND("%.20s true, new pc %" SIZEd "\n", O2S(valuePtr),
			PC_REL + jmpOffset[1]);
	    } else {
		TRACE_APPEND("%.20s true\n", O2S(valuePtr));
	    }
	} else {
	    if ((*pc == INST_JUMP_TRUE)
#ifndef REMOVE_DEPRECATED_OPCODES
		    || (*pc == INST_JUMP_TRUE1)
#endif
		    ) {
		TRACE_APPEND("%.20s false\n", O2S(valuePtr));
	    } else {
		TRACE_APPEND("%.20s false, new pc %" SIZEd "\n", O2S(valuePtr),
			PC_REL + jmpOffset[0]);
	    }
	}
#endif
	NEXT_INST_F0(jmpOffset[b], 1);
    }

    {
	Tcl_HashEntry *hPtr;
	JumptableInfo *jtPtr;
	JumptableNumInfo *jtnPtr;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails. Lookup by string.
	 */

    case INST_JUMP_TABLE:
	tblIdx = TclGetInt4AtPtr(pc + 1);
	jtPtr = (JumptableInfo *)
		codePtr->auxDataArrayPtr[tblIdx].clientData;
	TRACE("%u \"%.20s\" => ", tblIdx, O2S(OBJ_AT_TOS));
	hPtr = Tcl_FindHashEntry(&jtPtr->hashTable, TclGetString(OBJ_AT_TOS));
	goto processJumpTableEntry;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails or key is non-integer. Lookup by integer.
	 */

    case INST_JUMP_TABLE_NUM:
	tblIdx = TclGetInt4AtPtr(pc + 1);
	jtnPtr = (JumptableNumInfo *)
		codePtr->auxDataArrayPtr[tblIdx].clientData;
	TRACE("%u \"%.20s\" => ", tblIdx, O2S(OBJ_AT_TOS));
	DECACHE_STACK_INFO();
	Tcl_WideInt key;
	if (Tcl_GetWideIntFromObj(interp, OBJ_AT_TOS, &key) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}
	CACHE_STACK_INFO();
	hPtr = Tcl_FindHashEntry(&jtnPtr->hashTable, INT2PTR(key));

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclExecute.c`, function `logical-not-source-window`, lines 6532–6631. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclExecute.c`.

```text
    case INST_LNOT: {
	valuePtr = OBJ_AT_TOS;
	TRACE("\"%.20s\" => ", O2S(valuePtr));

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	int b;
	if (TclGetBooleanFromObj(NULL, valuePtr, &b) != TCL_OK) {
	    TRACE_APPEND("ERROR: illegal type %s\n", TY2S(valuePtr->typePtr));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	/* TODO: Consider peephole opt. */
	objResultPtr = TCONST(!b);
	TRACE_APPEND_OBJ(objResultPtr);
	NEXT_INST_F(1, 1, 1);
    }

    case INST_BITNOT:
	valuePtr = OBJ_AT_TOS;
	TRACE("\"%.20s\" => ", O2S(valuePtr));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| (type1==TCL_NUMBER_NAN) || (type1==TCL_NUMBER_DOUBLE)) {
	    /*
	     * ... ~$NonInteger => raise an error.
	     */

	    TRACE_APPEND("ERROR: illegal type %s\n", TY2S(valuePtr->typePtr));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	if (type1 == TCL_NUMBER_INT) {
	    w1 = *((const Tcl_WideInt *) ptr1);
	    if (Tcl_IsShared(valuePtr)) {
		TclNewIntObj(objResultPtr, ~w1);
		TRACE_APPEND_NUM_OBJ(objResultPtr);
		NEXT_INST_F(1, 1, 1);
	    }
	    TclSetIntObj(valuePtr, ~w1);
	    TRACE_APPEND("%s\n", O2S(valuePtr));
	    NEXT_INST_F0(1, 0);
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND_NUM_OBJ(objResultPtr);
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND_NUM_OBJ(valuePtr);
	    NEXT_INST_F0(1, 0);
	}

    case INST_UMINUS:
	valuePtr = OBJ_AT_TOS;
	TRACE("\"%.20s\" => ", O2S(valuePtr));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| IsErroringNaNType(type1)) {
	    TRACE_APPEND("ERROR: illegal type %s\n", TY2S(valuePtr->typePtr));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	switch (type1) {
	case TCL_NUMBER_NAN:
	    /* -NaN => NaN */
	    TRACE_APPEND_NUM_OBJ(valuePtr);
	    NEXT_INST_F0(1, 0);
	case TCL_NUMBER_INT:
	    w1 = *((const Tcl_WideInt *) ptr1);
	    if (w1 != WIDE_MIN) {
		if (Tcl_IsShared(valuePtr)) {
		    TclNewIntObj(objResultPtr, -w1);
		    TRACE_APPEND_NUM_OBJ(objResultPtr);
		    NEXT_INST_F(1, 1, 1);
		}
		TclSetIntObj(valuePtr, -w1);
		TRACE_APPEND_NUM_OBJ(valuePtr);
		NEXT_INST_F0(1, 0);
	    }
	    TCL_FALLTHROUGH();
	default:
	    break;
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND_NUM_OBJ(objResultPtr);
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND_NUM_OBJ(valuePtr);
	    NEXT_INST_F0(1, 0);
	}

    case INST_UPLUS:
    case INST_TRY_CVT_TO_NUMERIC:
	/*
	 * Try to convert the topmost stack object to numeric object. This is

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetBoolean`, lines 6576–6582. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
int Jim_GetBoolean(Jim_Interp *interp, Jim_Obj *objPtr, int * booleanPtr)
{
    if (objPtr->typePtr != &intObjType && SetBooleanFromAny(interp, objPtr, JIM_ERRMSG) == JIM_ERR)
        return JIM_ERR;
    *booleanPtr = (int) JimWideValue(objPtr);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `ExprBool`, lines 9147–9169. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
static int ExprBool(Jim_Interp *interp, Jim_Obj *obj)
{
    long l;
    double d;
    int b;
    int ret = -1;

    /* In case the object is interp->result with refcount 1*/
    Jim_IncrRefCount(obj);

    if (Jim_GetLong(interp, obj, &l) == JIM_OK) {
        ret = (l != 0);
    }
    else if (Jim_GetDouble(interp, obj, &d) == JIM_OK) {
        ret = (d != 0);
    }
    else if (Jim_GetBoolean(interp, obj, &b) == JIM_OK) {
        ret = (b != 0);
    }

    Jim_DecrRefCount(interp, obj);
    return ret;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetBoolFromExpr`, lines 10314–10334. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
int Jim_GetBoolFromExpr(Jim_Interp *interp, Jim_Obj *exprObjPtr, int *boolPtr)
{
    int retcode = Jim_EvalExpression(interp, exprObjPtr);

    if (retcode == JIM_OK) {
        switch (ExprBool(interp, Jim_GetResult(interp))) {
            case 0:
                *boolPtr = 0;
                break;

            case 1:
                *boolPtr = 1;
                break;

            case -1:
                retcode = JIM_ERR;
                break;
        }
    }
    return retcode;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_ExprCoreCommand`, lines 14255–14271. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
static int Jim_ExprCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
#ifdef JIM_COMPAT
    if (argc > 2) {
        int retcode;
        Jim_Obj *objPtr;

        objPtr = Jim_ConcatObj(interp, argc - 1, argv + 1);
        Jim_IncrRefCount(objPtr);
        retcode = Jim_EvalExpression(interp, objPtr);
        Jim_DecrRefCount(interp, objPtr);

        return retcode;
    }
#endif
    return Jim_EvalExpression(interp, argv[1]);
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_IfCoreCommand`, lines 13273–13321. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
static int Jim_IfCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int boolean, retval, current = 1, falsebody = 0;

    while (1) {
        /* not enough arguments given! */
        if (current >= argc) {
            return JIM_USAGE;
        }
        if ((retval = Jim_GetBoolFromExpr(interp, argv[current++], &boolean))
            != JIM_OK)
            return retval;
        /* There lacks something, isn't it? */
        if (current >= argc) {
            return JIM_USAGE;
        }
        if (Jim_CompareStringImmediate(interp, argv[current], "then"))
            current++;
        /* Tsk tsk, no then-clause? */
        if (current >= argc) {
            return JIM_USAGE;
        }
        if (boolean)
            return Jim_EvalObj(interp, argv[current]);
        /* Ok: no else-clause follows */
        if (++current >= argc) {
            Jim_SetResult(interp, Jim_NewEmptyStringObj(interp));
            return JIM_OK;
        }
        falsebody = current++;
        if (Jim_CompareStringImmediate(interp, argv[falsebody], "else")) {
            /* IIICKS - else-clause isn't last cmd? */
            if (current != argc - 1) {
                return JIM_USAGE;
            }
            return Jim_EvalObj(interp, argv[current]);
        }
        else if (Jim_CompareStringImmediate(interp, argv[falsebody], "elseif"))
            /* Ok: elseif follows meaning all the stuff
             * again (how boring...) */
            continue;
        /* OOPS - else-clause is not last cmd? */
        else if (falsebody != argc - 1) {
            return JIM_USAGE;
        }
        return Jim_EvalObj(interp, argv[falsebody]);
    }
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_WhileCoreCommand`, lines 12801–12829. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
static int Jim_WhileCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    /* The general purpose implementation of while starts here */
    while (1) {
        int boolean = 0, retval;

        if ((retval = Jim_GetBoolFromExpr(interp, argv[1], &boolean)) != JIM_OK)
            return retval;
        if (!boolean)
            break;

        if ((retval = Jim_EvalObj(interp, argv[2])) != JIM_OK) {
            if (JimCheckLoopRetcode(interp, retval)) {
                return retval;
            }
            switch (retval) {
                case JIM_BREAK:
                    goto out;
                case JIM_CONTINUE:
                    continue;
                default:
                    return retval;
            }
        }
    }
  out:
    Jim_SetEmptyResult(interp);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimExprOpAnd`, lines 9172–9186. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
static int JimExprOpAnd(Jim_Interp *interp, struct JimExprNode *node)
{
    /* evaluate left */
    int result = JimExprGetTermBoolean(interp, node->left);

    if (result == 1) {
        /* true so evaluate right */
        result = JimExprGetTermBoolean(interp, node->right);
    }
    if (result == -1) {
        return JIM_ERR;
    }
    Jim_SetResultInt(interp, result);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimExprOpOr`, lines 9189–9203. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
static int JimExprOpOr(Jim_Interp *interp, struct JimExprNode *node)
{
    /* evaluate left */
    int result = JimExprGetTermBoolean(interp, node->left);

    if (result == 0) {
        /* false so evaluate right */
        result = JimExprGetTermBoolean(interp, node->right);
    }
    if (result == -1) {
        return JIM_ERR;
    }
    Jim_SetResultInt(interp, result);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalObj`, lines 11554–11796. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
int Jim_EvalObj(Jim_Interp *interp, Jim_Obj *scriptObjPtr)
{
    int i;
    ScriptObj *script;
    ScriptToken *token;
    int retcode = JIM_OK;
    Jim_Obj *sargv[JIM_EVAL_SARGV_LEN], **argv = NULL;
    Jim_EvalFrame frame;

    /* If the object is of type "list", with no string rep we can call
     * a specialized version of Jim_EvalObj() */
    if (Jim_IsList(scriptObjPtr) && scriptObjPtr->bytes == NULL) {
        return JimEvalObjList(interp, scriptObjPtr);
    }

    Jim_IncrRefCount(scriptObjPtr);     /* Make sure it's shared. */
    script = JimGetScript(interp, scriptObjPtr);
    if (JimParseCheckMissing(interp, script->missing) == JIM_ERR) {
        JimSetErrorStack(interp, script);
        Jim_DecrRefCount(interp, scriptObjPtr);
        return JIM_ERR;
    }

    /* Reset the interpreter result. This is useful to
     * return the empty result in the case of empty program. */
    Jim_SetEmptyResult(interp);

    token = script->token;

#ifdef JIM_OPTIMIZATION
    /* Check for one of the following common scripts used by for, while
     *
     *   {}
     *   incr a
     */
    if (script->len == 0) {
        Jim_DecrRefCount(interp, scriptObjPtr);
        /* Allow a signal to interrupt loops that dispatch no command. */
        if (Jim_CheckSignal(interp))
            return JIM_SIGNAL;
        return JIM_OK;
    }
    if (script->len == 3
        && token[1].objPtr->typePtr == &commandObjType
        && (token[1].objPtr->internalRep.cmdValue.cmdPtr->flags & JIM_CMD_ISPROC) == 0
        && token[1].objPtr->internalRep.cmdValue.cmdPtr->u.native.cmdProc == Jim_IncrCoreCommand
        && token[2].objPtr->typePtr == &variableObjType) {

        Jim_Obj *objPtr = Jim_GetVariable(interp, token[2].objPtr, JIM_NONE);

        if (objPtr && !Jim_IsShared(objPtr) && objPtr->typePtr == &intObjType) {
            JimWideValue(objPtr)++;
            Jim_InvalidateStringRep(objPtr);
            Jim_DecrRefCount(interp, scriptObjPtr);
            Jim_SetResult(interp, objPtr);
            /* Allow a signal to interrupt loops that dispatch no command. */
            if (Jim_CheckSignal(interp))
                return JIM_SIGNAL;
            return JIM_OK;
        }
    }
#endif

    /* Now we have to make sure the internal repr will not be
     * freed on shimmering.
     *
     * Think for example to this:
     *
     * set x {llength $x; ... some more code ...}; eval $x
     *
     * In order to preserve the internal rep, we increment the
     * inUse field of the script internal rep structure. */
    script->inUse++;

    JimPushEvalFrame(interp, &frame, scriptObjPtr);

    /* Collect a new error stack trace if an error occurs */
    interp->hasErrorStackTrace = 0;
    argv = sargv;

    /* Execute every command sequentially until the end of the script
     * or an error occurs.
     */
    for (i = 0; i < script->len && retcode == JIM_OK; ) {
        int argc;
        int j;

        /* First token of the line is always JIM_TT_LINE */
        argc = token[i].objPtr->internalRep.scriptLineValue.argc;
        script->linenr = token[i].objPtr->internalRep.scriptLineValue.line;

        /* Allocate the arguments vector if required */
        if (argc > JIM_EVAL_SARGV_LEN)
            argv = Jim_Alloc(sizeof(Jim_Obj *) * argc);

        /* Skip the JIM_TT_LINE token */
        i++;

        /* Populate the arguments objects.
         * If an error occurs, retcode will be set and
         * 'j' will be set to the number of args expanded
         */
        for (j = 0; j < argc; j++) {
            long wordtokens = 1;
            int expand = 0;
            Jim_Obj *wordObjPtr = NULL;

            if (token[i].type == JIM_TT_WORD) {
                wordtokens = JimWideValue(token[i++].objPtr);
                if (wordtokens < 0) {
                    expand = 1;
                    wordtokens = -wordtokens;
                }
            }

            if (wordtokens == 1) {
                /* Fast path if the token does not
                 * need interpolation */

                switch (token[i].type) {
                    case JIM_TT_ESC:
                    case JIM_TT_STR:
                        wordObjPtr = token[i].objPtr;
                        break;
                    case JIM_TT_VAR:
                        wordObjPtr = Jim_GetVariable(interp, token[i].objPtr, JIM_ERRMSG);
                        break;
                    case JIM_TT_EXPRSUGAR:
                        retcode = Jim_EvalExpression(interp, token[i].objPtr);
                        if (retcode == JIM_OK) {
                            wordObjPtr = Jim_GetResult(interp);
                        }
                        else {
                            wordObjPtr = NULL;
                        }
                        break;
                    case JIM_TT_DICTSUGAR:
                        wordObjPtr = JimExpandDictSugar(interp, token[i].objPtr);
                        break;
                    case JIM_TT_CMD:
                        retcode = Jim_EvalObj(interp, token[i].objPtr);
                        if (retcode == JIM_OK) {
                            wordObjPtr = Jim_GetResult(interp);
                        }
                        break;
                    default:
                        JimPanic((1, "default token type reached " "in Jim_EvalObj()."));
                }
            }
            else {
                /* For interpolation we call a helper
                 * function to do the work for us. */
                wordObjPtr = JimInterpolateTokens(interp, token + i, wordtokens, JIM_NONE);
            }

            if (!wordObjPtr) {
                if (retcode == JIM_OK) {
                    retcode = JIM_ERR;
                }
                break;
            }

            Jim_IncrRefCount(wordObjPtr);
            i += wordtokens;

            if (!expand) {
                argv[j] = wordObjPtr;
            }
            else {
                /* Need to expand wordObjPtr into multiple args from argv[j] ... */
                int len = Jim_ListLength(interp, wordObjPtr);
                int newargc = argc + len - 1;
                int k;

                if (len > 1) {
                    if (argv == sargv) {
                        if (newargc > JIM_EVAL_SARGV_LEN) {
                            argv = Jim_Alloc(sizeof(*argv) * newargc);
                            memcpy(argv, sargv, sizeof(*argv) * j);
                        }
                    }
                    else {
                        /* Need to realloc to make room for (len - 1) more entries */
                        argv = Jim_Realloc(argv, sizeof(*argv) * newargc);
                    }
                }

                /* Now copy in the expanded version */
                for (k = 0; k < len; k++) {
                    argv[j++] = wordObjPtr->internalRep.listValue.ele[k];
                    Jim_IncrRefCount(wordObjPtr->internalRep.listValue.ele[k]);
                }

                /* The original object reference is no longer needed,
                 * after the expansion it is no longer present on
                 * the argument vector, but the single elements are
                 * in its place. */
                Jim_DecrRefCount(interp, wordObjPtr);

                /* And update the indexes */
                j--;
                argc += len - 1;
            }
        }

        if (retcode == JIM_OK && argc) {
            /* Invoke the command */
            retcode = JimInvokeCommand(interp, argc, argv);
            interp->taint = 0;
            /* Check for a signal after each command */
            if (Jim_CheckSignal(interp)) {
                retcode = JIM_SIGNAL;
            }
        }

        /* Finished with the command, so decrement ref counts of each argument */
        while (j-- > 0) {
            Jim_DecrRefCount(interp, argv[j]);
        }

        if (argv != sargv) {
            Jim_Free(argv);
            argv = sargv;
        }
    }

    /* Possibly add to the error stack trace */
    if (retcode == JIM_ERR) {
        JimSetErrorStack(interp, NULL);
    }

    JimPopEvalFrame(interp);

    /* Note that we don't have to decrement inUse, because the
     * following code transfers our use of the reference again to
     * the script object. */
    Jim_FreeIntRep(interp, scriptObjPtr);
    scriptObjPtr->typePtr = &scriptObjType;
    Jim_SetIntRepPtr(scriptObjPtr, script);
    Jim_DecrRefCount(interp, scriptObjPtr);

    return retcode;
}

```

## Consumer bindings

No implementation binding is claimed by this observation record.

- [runtime/rust/src/typed_value/native_boolean_truth.rs](../../../../runtime/rust/src/typed_value/native_boolean_truth.rs), `native_boolean_for_interp`: Select a direct original operand instruction from actual entered interpreter protocol and explicit source purpose. Refuse result-purpose tags before probing; original object/getter and Host currency stay independent of public expression result production.
- [runtime/rust/src/typed_value/native_boolean_truth.rs](../../../../runtime/rust/src/typed_value/native_boolean_truth.rs), `typed_value::native_boolean_truth::tests::original_c84_logical_sides_keep_distinct_boolean_and_integer_primary` (linked): Under genuine entered C84 software ingress, two fresh identical4294967296 String operands reach distinct LogicalAnd operand versus final LogicalAndInstruction purposes: false WordBoolean primary versus true Tcl84Long2^32, with exact original bytes retained. Literal expectations correspond the original465 case4 worker6/7 input cache and result-truth windows; this is a bounded operand-stage software control, not a whole expression/native opcode comparator.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

- [rust/tcl-vm/src/expr/native_boolean_truth.rs](../../../../rust/tcl-vm/src/expr/native_boolean_truth.rs), `boolean_for_vm`: Apply the separately selected original operand truth purpose through the actual entered VM operation. An expression-result-required purpose is unavailable here; positively selected authored simulation uses its own provider. This direct operand worker issues no completed script, source, instruction or public-API result receipt.
- [rust/tcl-vm/src/expr/native_boolean_truth.rs](../../../../rust/tcl-vm/src/expr/native_boolean_truth.rs), `expr::native_boolean_truth::tests::original_c84_logical_sides_preserve_distinct_selected_primary` (linked): In an actually selected C84 core/compiler fixture, fresh4294967296 original operands use distinct reached LogicalAnd and LogicalAndInstruction purposes: false WordBoolean versus true retained Tcl84Long, with unchanged resident spelling. Expectations correspond to original465 case4 workers6/7 operand stages only; the test neither reads the whole native stream nor asserts whole expression result, all providers or opcodes.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-test-support/tests/data/native_resolution_reference_builds](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds)
