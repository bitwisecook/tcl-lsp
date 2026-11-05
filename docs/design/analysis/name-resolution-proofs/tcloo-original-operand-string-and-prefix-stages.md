# naming.tcloo.original-operand-string-and-prefix-stages

Kind: `native-observation`

## Problem statement

Retaining an original TclOO command operand, a forward prefix List or a definition-frame report can have different String and reference effects from presenting or invoking it. An application cannot pre-read every operand or replace retained original members based on successful public bytes alone. This fixed original probe measures the actual storage/report/invocation and method-return boundaries while preserving the evaluator and explicit presentation effects separately.

## Question

When are Strings, native references and retained original member identities reached at TclOO forward definition/info/invocation, definition-frame reporting, and selected method option/parameter/body stages through the original public object-vector APIs?

## Conclusion

The initialized original C8.4/C8.5 and Jim probes report availability bytes 0 0 0 and execute no C TclOO operand cases. C8.6/C9.0/C9.1 each complete nine fresh-interpreter cases and retain all53 stage rows. Forward definition keeps the original prefix object stringless at refcount2; info class forward returns a two-member List containing that same original without a getter, while explicit List presentation reaches its updater. A separate no-report invocation preserves that member into the actual capture callback, which then reaches its String getter. Definition-frame entry keeps the original operand stringless; info level 0 returns a four-member List containing it, and raw member access adds no String. Explicit report/result presentation reaches the updater in its separate stage. Method return stages preserve release-specific arity/option outcomes and all reached update/free/refcount fields. C9 valid -export succeeds while C8.6 reports arity error; invalid -bad is an export-flag error on C9 and an arity error on C8.6. Failed method return stages can include public evaluator error-context String rendering, so they do not attest the stock worker alone reached every later operand. Exact whole streams retain interleaved callback events in RESULT/PRESENTATION output. All six provider dependency/compile/version/main commands and the shared compiler-version command exit0 with empty stderr. These original object-stage observations supply no application Native issuer or Rust parity.

## Scope

Exact unchanged C99 object-vector probe with fresh original initialized interpreters on measured C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9. The actual six-provider availability/version/API channels are independently retained; only C86/C90/C91 execute nine TclOO cases. Each custom original object starts at caller +1 with invalidated String and recorded public update/free/duplicate callbacks, and stays held through interpreter teardown. ROW inspects only its public fields/counters without reaching its String getter. Public Tcl_EvalObjv return stages include evaluator propagation/error context; RESULT and PRESENTATION explicitly request the original result String later and their output can interleave with updater events. Raw List member equality identifies only the actual retained original object at that stage. No private TclOO structure, procedure/frame/generation/source issuer or active retirement equivalence is inferred. The legacy public Tcl_CreateObjCommand observer callback uses its retained compatibility family and does not attest arbitrary callback/count widths. Probe VERSION/BUILD, independent CLI-version stdout, ABI int4/long8/ptr8/CHAR_BIT8, actual source/header/config/static-library/compiler/dependency/link/probe/script before/after hashes and all process streams are retained. Original SDK source/library inputs are retained; configure/build processes lack replay receipts, so source-to-library equivalence is unclaimed. Exact external system toolchain/link/runtime inputs without retained payloads remain hash-recorded and require independently supplied bytes. Selected initialized command absence does not imply package absence or unsupported TclOO after other package loading. No Rust executable/assertion, Native preparation/Normal/effect/SSA/editor permission, application Host/currency/frame authority, Jim C TclOO recipe or BIG-IP outcome follows.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Actual original initialized public object-vector operations or genuine initialized command/API unavailability; held +1 custom input objects, ROW public fields/callback counters, explicit result/report String presentation and complete raw streams. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Actual fresh initialized availability query returns status0 and bytes 0 0 0. No operand-stage CASE/ROW or custom getter callback is executed. The measured initialized command surface has no TclOO commands; extension/package-loaded availability is not measured.

### tcl8.6

Status: `observed`. Channel: Actual original initialized public object-vector operations or genuine initialized command/API unavailability; held +1 custom input objects, ROW public fields/callback counters, explicit result/report String presentation and complete raw streams. Version: tcl8.6 8.6.18.

Nine fresh-interpreter cases preserve53 original stage rows. Forward and reported-frame Lists retain the actual original member without its String getter before explicit presentation. Independent no-report invocation reaches the original capture member/getter. Method valid/malformed-formal and option cases retain exact public return-stage callback/header/refcount fields. Both optional five-operand method cases return1 arity errors. Public failure return stages may include evaluator error-context rendering; internal worker stop/whole-operation cache provenance is not inferred.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Actual original initialized public object-vector operations or genuine initialized command/API unavailability; held +1 custom input objects, ROW public fields/callback counters, explicit result/report String presentation and complete raw streams. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Nine fresh-interpreter cases preserve53 original stage rows. Forward and reported-frame Lists retain the actual original member without its String getter before explicit presentation. Independent no-report invocation reaches the original capture member/getter. Method valid/malformed-formal and option cases retain exact public return-stage callback/header/refcount fields. Valid -export returns0; invalid -bad returns1 export-flag error. Public failure return stages may include evaluator error-context rendering; internal worker stop/whole-operation cache provenance is not inferred.

### jim

Status: `observed`. Channel: Actual original initialized public object-vector operations or genuine initialized command/API unavailability; held +1 custom input objects, ROW public fields/callback counters, explicit result/report String presentation and complete raw streams. Version: jim 0.84-9-g5bac7c9.

Actual fresh initialized availability query returns status0 and bytes 0 0 0. No operand-stage CASE/ROW or custom getter callback is executed. The original public C TclOO object API is not a Jim API; no C object/frame/forward recipe or scripted class package behavior is inferred.

### bigip

Status: `not-tested`.

No original BIG-IP TclOO availability, object-vector operand, reported-frame or getter-stage process was requested or executed.

## Source inspection

tcl8.6 8.6.18, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `InitDefineContext`, lines 667–695. Retained reference `native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
InitDefineContext(
    Tcl_Interp *interp,
    Tcl_Namespace *namespacePtr,
    Object *oPtr,
    int objc,
    Tcl_Obj *const objv[])
{
    CallFrame *framePtr, **framePtrPtr = &framePtr;

    if (namespacePtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"cannot process definitions; support namespace deleted",
		-1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * framePtrPtr is needed to satisfy GCC 3.3's strict aliasing rules.
     */

    (void) TclPushStackFrame(interp, (Tcl_CallFrame **) framePtrPtr,
	    namespacePtr, FRAME_IS_OO_DEFINE);
    framePtr->clientData = oPtr;
    framePtr->objc = objc;
    framePtr->objv = objv;	/* Reference counts do not need to be
				 * incremented here. */
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOODefineForwardObjCmd`, lines 1488–1535. Retained reference `native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
TclOODefineForwardObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceForward = (clientData != NULL);
    Object *oPtr;
    Method *mPtr;
    int isPublic;
    Tcl_Obj *prefixObj;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name cmdName ?arg ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceForward && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }
    isPublic = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
	    ? PUBLIC_METHOD : 0;

    /*
     * Create the method structure.
     */

    prefixObj = Tcl_NewListObj(objc - 2, objv + 2);
    if (isInstanceForward) {
	mPtr = TclOONewForwardInstanceMethod(interp, oPtr, isPublic, objv[1],
		prefixObj);
    } else {
	mPtr = TclOONewForwardMethod(interp, oPtr->classPtr, isPublic,
		objv[1], prefixObj);
    }
    if (mPtr == NULL) {
	Tcl_DecrRefCount(prefixObj);
	return TCL_ERROR;
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 1548–1592. Retained reference `native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
TclOODefineMethodObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceMethod = (clientData != NULL);
    Object *oPtr;
    int isPublic;

    if (objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "name args body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }
    isPublic = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
	    ? PUBLIC_METHOD : 0;

    /*
     * Create the method by using the right back-end API.
     */

    if (isInstanceMethod) {
	if (TclOONewProcInstanceMethod(interp, oPtr, isPublic, objv[1],
		objv[2], objv[3], NULL) == NULL) {
	    return TCL_ERROR;
	}
    } else {
	if (TclOONewProcMethod(interp, oPtr->classPtr, isPublic, objv[1],
		objv[2], objv[3], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `InfoLevelCmd`, lines 1592–1642. Retained reference `native_info_inventory_original-sources-8.6.18-tclCmdIL.c`.

```text
InfoLevelCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;

    if (objc == 1) {		/* Just "info level" */
	Tcl_SetObjResult(interp, Tcl_NewIntObj(iPtr->varFramePtr->level));
	return TCL_OK;
    }

    if (objc == 2) {
	int level;
	CallFrame *framePtr, *rootFramePtr = iPtr->rootFramePtr;

	if (TclGetIntFromObj(interp, objv[1], &level) != TCL_OK) {
	    return TCL_ERROR;
	}
	if (level <= 0) {
	    if (iPtr->varFramePtr == rootFramePtr) {
		goto levelError;
	    }
	    level += iPtr->varFramePtr->level;
	}
	for (framePtr=iPtr->varFramePtr ; framePtr!=rootFramePtr;
		framePtr=framePtr->callerVarPtr) {
	    if (framePtr->level == level) {
		break;
	    }
	}
	if (framePtr == rootFramePtr) {
	    goto levelError;
	}

	Tcl_SetObjResult(interp,
		Tcl_NewListObj(framePtr->objc, framePtr->objv));
	return TCL_OK;
    }

    Tcl_WrongNumArgs(interp, 1, objv, "?number?");
    return TCL_ERROR;

  levelError:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "bad level \"%s\"", TclGetString(objv[1])));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "LEVEL",
	    TclGetString(objv[1]), (char *)NULL);
    return TCL_ERROR;
}

```

tcl8.6 8.6.18, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `TclOONewProcMethod`, lines 366–414. Retained reference `native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
TclOONewProcMethod(
    Tcl_Interp *interp,		/* The interpreter containing the class. */
    Class *clsPtr,		/* The class to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which may be NULL;
				 * if so, up to caller to manage storage
				 * (e.g., because it is a constructor or
				 * destructor). */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which may be NULL; if so, it is equivalent
				 * to an empty list. */
    Tcl_Obj *bodyObj,		/* The body of the method, which must not be
				 * NULL. */
    ProcedureMethod **pmPtrPtr)	/* Place to write pointer to procedure method
				 * structure to allow for deeper tuning of the
				 * structure's contents. NULL if caller is not
				 * interested. */
{
    int argsLen;		/* -1 => delete argsObj before exit */
    ProcedureMethod *pmPtr;
    const char *procName;
    Tcl_Method method;

    if (argsObj == NULL) {
	argsLen = -1;
	TclNewObj(argsObj);
	Tcl_IncrRefCount(argsObj);
	procName = "<destructor>";
    } else if (TclListObjLength(interp, argsObj, &argsLen) != TCL_OK) {
	return NULL;
    } else {
	procName = (nameObj==NULL ? "<constructor>" : TclGetString(nameObj));
    }

    pmPtr = AllocProcedureMethodRecord(flags);
    method = TclOOMakeProcMethod(interp, clsPtr, flags, nameObj, procName,
	    argsObj, bodyObj, &procMethodType, pmPtr, &pmPtr->procPtr);

    if (argsLen == -1) {
	Tcl_DecrRefCount(argsObj);
    }
    if (method == NULL) {
	ckfree(pmPtr);
    } else if (pmPtrPtr != NULL) {
	*pmPtrPtr = pmPtr;
    }

    return (Method *) method;
}

```

tcl9.0 9.0.4, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `InitDefineContext`, lines 970–997. Retained reference `native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
InitDefineContext(
    Tcl_Interp *interp,
    Tcl_Namespace *namespacePtr,
    Object *oPtr,
    int objc,
    Tcl_Obj *const objv[])
{
    CallFrame *framePtr, **framePtrPtr = &framePtr;

    if (namespacePtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"no definition namespace available", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    /*
     * framePtrPtr is needed to satisfy GCC 3.3's strict aliasing rules.
     */

    (void) TclPushStackFrame(interp, (Tcl_CallFrame **) framePtrPtr,
	    namespacePtr, FRAME_IS_OO_DEFINE);
    framePtr->clientData = oPtr;
    framePtr->objc = objc;
    framePtr->objv = objv;	/* Reference counts do not need to be
				 * incremented here. */
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOODefineForwardObjCmd`, lines 2166–2216. Retained reference `native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
TclOODefineForwardObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceForward = (clientData != NULL);
    Object *oPtr;
    Method *mPtr;
    int isPublic;
    Tcl_Obj *prefixObj;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name cmdName ?arg ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceForward && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }
    isPublic = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
	    ? PUBLIC_METHOD : 0;
    if (IsPrivateDefine(interp)) {
	isPublic = TRUE_PRIVATE_METHOD;
    }

    /*
     * Create the method structure.
     */

    prefixObj = Tcl_NewListObj(objc - 2, objv + 2);
    if (isInstanceForward) {
	mPtr = TclOONewForwardInstanceMethod(interp, oPtr, isPublic, objv[1],
		prefixObj);
    } else {
	mPtr = TclOONewForwardMethod(interp, oPtr->classPtr, isPublic,
		objv[1], prefixObj);
    }
    if (mPtr == NULL) {
	Tcl_DecrRefCount(prefixObj);
	return TCL_ERROR;
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 2277–2361. Retained reference `native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
TclOODefineMethodObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    /*
     * Table of export modes for methods and their corresponding enum.
     */

    static const char *const exportModes[] = {
	"-export",
	"-private",
	"-unexport",
	NULL
    };
    enum ExportMode {
	MODE_EXPORT,
	MODE_PRIVATE,
	MODE_UNEXPORT
    } exportMode;

    int isInstanceMethod = (clientData != NULL);
    Object *oPtr;
    int isPublic = 0;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?option? args body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }
    if (objc == 5) {
	if (Tcl_GetIndexFromObj(interp, objv[2], exportModes, "export flag",
		0, &exportMode) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (exportMode) {
	case MODE_EXPORT:
	    isPublic = PUBLIC_METHOD;
	    break;
	case MODE_PRIVATE:
	    isPublic = TRUE_PRIVATE_METHOD;
	    break;
	case MODE_UNEXPORT:
	    isPublic = 0;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    } else {
	if (IsPrivateDefine(interp)) {
	    isPublic = TRUE_PRIVATE_METHOD;
	} else {
	    isPublic = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
		    ? PUBLIC_METHOD : 0;
	}
    }

    /*
     * Create the method by using the right back-end API.
     */

    if (isInstanceMethod) {
	if (TclOONewProcInstanceMethod(interp, oPtr, isPublic, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    } else {
	if (TclOONewProcMethod(interp, oPtr->classPtr, isPublic, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `InfoLevelCmd`, lines 1581–1631. Retained reference `native_info_inventory_original-sources-9.0.4-tclCmdIL.c`.

```text
InfoLevelCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;

    if (objc == 1) {		/* Just "info level" */
	Tcl_SetObjResult(interp, Tcl_NewWideIntObj((int)iPtr->varFramePtr->level));
	return TCL_OK;
    }

    if (objc == 2) {
	int level;
	CallFrame *framePtr, *rootFramePtr = iPtr->rootFramePtr;

	if (TclGetIntFromObj(interp, objv[1], &level) != TCL_OK) {
	    return TCL_ERROR;
	}
	if (level <= 0) {
	    if (iPtr->varFramePtr == rootFramePtr) {
		goto levelError;
	    }
	    level += iPtr->varFramePtr->level;
	}
	for (framePtr=iPtr->varFramePtr ; framePtr!=rootFramePtr;
		framePtr=framePtr->callerVarPtr) {
	    if ((int)framePtr->level == level) {
		break;
	    }
	}
	if (framePtr == rootFramePtr) {
	    goto levelError;
	}

	Tcl_SetObjResult(interp,
		Tcl_NewListObj(framePtr->objc, framePtr->objv));
	return TCL_OK;
    }

    Tcl_WrongNumArgs(interp, 1, objv, "?number?");
    return TCL_ERROR;

  levelError:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "bad level \"%s\"", TclGetString(objv[1])));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "LEVEL",
	    TclGetString(objv[1]), (char *)NULL);
    return TCL_ERROR;
}

```

tcl9.0 9.0.4, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `TclOONewProcMethod`, lines 467–515. Retained reference `native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
TclOONewProcMethod(
    Tcl_Interp *interp,		/* The interpreter containing the class. */
    Class *clsPtr,		/* The class to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which may be NULL;
				 * if so, up to caller to manage storage
				 * (e.g., because it is a constructor or
				 * destructor). */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which may be NULL; if so, it is equivalent
				 * to an empty list. */
    Tcl_Obj *bodyObj,		/* The body of the method, which must not be
				 * NULL. */
    ProcedureMethod **pmPtrPtr)	/* Place to write pointer to procedure method
				 * structure to allow for deeper tuning of the
				 * structure's contents. NULL if caller is not
				 * interested. */
{
    Tcl_Size argsLen;		/* TCL_INDEX_NONE => delete argsObj before exit */
    ProcedureMethod *pmPtr;
    const char *procName;
    Tcl_Method method;

    if (argsObj == NULL) {
	argsLen = TCL_INDEX_NONE;
	TclNewObj(argsObj);
	Tcl_IncrRefCount(argsObj);
	procName = "<destructor>";
    } else if (TclListObjLength(interp, argsObj, &argsLen) != TCL_OK) {
	return NULL;
    } else {
	procName = (nameObj==NULL ? "<constructor>" : TclGetString(nameObj));
    }

    pmPtr = AllocProcedureMethodRecord(flags);
    method = TclOOMakeProcMethod(interp, clsPtr, flags, nameObj, procName,
	    argsObj, bodyObj, &procMethodType, pmPtr, &pmPtr->procPtr);

    if (argsLen == TCL_INDEX_NONE) {
	Tcl_DecrRefCount(argsObj);
    }
    if (method == NULL) {
	Tcl_Free(pmPtr);
    } else if (pmPtrPtr != NULL) {
	*pmPtrPtr = pmPtr;
    }

    return (Method *) method;
}

```

tcl9.1 9.1.0, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `InitDefineContext`, lines 953–980. Retained reference `native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
InitDefineContext(
    Tcl_Interp *interp,
    Tcl_Namespace *namespacePtr,
    Object *oPtr,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    CallFrame *framePtr, **framePtrPtr = &framePtr;

    if (namespacePtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"no definition namespace available", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    /*
     * framePtrPtr is needed to satisfy GCC 3.3's strict aliasing rules.
     */

    (void) TclPushStackFrame(interp, (Tcl_CallFrame **) framePtrPtr,
	    namespacePtr, FRAME_IS_OO_DEFINE);
    framePtr->clientData = oPtr;
    framePtr->objc = objc;
    framePtr->objv = objv;	/* Reference counts do not need to be
				 * incremented here. */
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOODefineForwardObjCmd`, lines 2129–2176. Retained reference `native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
TclOODefineForwardObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstanceForward = (clientData != NULL);
    Object *oPtr;
    Method *mPtr;
    int flags;
    Tcl_Obj *prefixObj;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name cmdName ?arg ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceForward && !oPtr->classPtr) {
	return ReportAbuse(interp);
    }
    flags = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
	    ? PUBLIC_METHOD : 0;
    if (IsPrivateDefine(interp)) {
	flags = TRUE_PRIVATE_METHOD;
    }

    /*
     * Create the method structure.
     */

    prefixObj = Tcl_NewListObj(objc - 2, objv + 2);
    if (isInstanceForward) {
	mPtr = TclOONewForwardInstanceMethod(interp, oPtr, flags, objv[1],
		prefixObj);
    } else {
	mPtr = TclOONewForwardMethod(interp, oPtr->classPtr, flags,
		objv[1], prefixObj);
    }
    if (mPtr == NULL) {
	Tcl_DecrRefCount(prefixObj);
	return TCL_ERROR;
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 2237–2318. Retained reference `native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
TclOODefineMethodObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    /*
     * Table of export modes for methods and their corresponding enum.
     */

    static const char *const exportModes[] = {
	"-export",
	"-private",
	"-unexport",
	NULL
    };
    enum ExportMode {
	MODE_EXPORT,
	MODE_PRIVATE,
	MODE_UNEXPORT
    } exportMode;

    bool isInstanceMethod = (clientData != NULL);
    Object *oPtr;
    int flags = 0;

    if (objc < 4 || objc > 5) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?option? args body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceMethod && !oPtr->classPtr) {
	return ReportAbuse(interp);
    }
    if (objc == 5) {
	if (Tcl_GetIndexFromObj(interp, objv[2], exportModes, "export flag",
		0, &exportMode) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (exportMode) {
	case MODE_EXPORT:
	    flags = PUBLIC_METHOD;
	    break;
	case MODE_PRIVATE:
	    flags = TRUE_PRIVATE_METHOD;
	    break;
	case MODE_UNEXPORT:
	    flags = 0;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    } else {
	if (IsPrivateDefine(interp)) {
	    flags = TRUE_PRIVATE_METHOD;
	} else {
	    flags = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
		    ? PUBLIC_METHOD : 0;
	}
    }

    /*
     * Create the method by using the right back-end API.
     */

    if (isInstanceMethod) {
	if (TclOONewProcInstanceMethod(interp, oPtr, flags, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    } else {
	if (TclOONewProcMethod(interp, oPtr->classPtr, flags, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `InfoLevelCmd`, lines 1565–1615. Retained reference `native_info_inventory_original-sources-9.1.0-tclCmdIL.c`.

```text
InfoLevelCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Interp *iPtr = (Interp *) interp;

    if (objc == 1) {		/* Just "info level" */
	Tcl_SetObjResult(interp, Tcl_NewWideIntObj((int)iPtr->varFramePtr->level));
	return TCL_OK;
    }

    if (objc == 2) {
	int level;
	CallFrame *framePtr, *rootFramePtr = iPtr->rootFramePtr;

	if (TclGetIntFromObj(interp, objv[1], &level) != TCL_OK) {
	    return TCL_ERROR;
	}
	if (level <= 0) {
	    if (iPtr->varFramePtr == rootFramePtr) {
		goto levelError;
	    }
	    level += (int)iPtr->varFramePtr->level;
	}
	for (framePtr=iPtr->varFramePtr ; framePtr!=rootFramePtr;
		framePtr=framePtr->callerVarPtr) {
	    if (framePtr->level == level) {
		break;
	    }
	}
	if (framePtr == rootFramePtr) {
	    goto levelError;
	}

	Tcl_SetObjResult(interp,
		Tcl_NewListObj(framePtr->objc, framePtr->objv));
	return TCL_OK;
    }

    Tcl_WrongNumArgs(interp, 1, objv, "?number?");
    return TCL_ERROR;

  levelError:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "bad level \"%s\"", TclGetString(objv[1])));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "LEVEL",
	    TclGetString(objv[1]), (char *)NULL);
    return TCL_ERROR;
}

```

tcl9.1 9.1.0, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `TclOONewProcMethod`, lines 473–521. Retained reference `native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
TclOONewProcMethod(
    Tcl_Interp *interp,		/* The interpreter containing the class. */
    Class *clsPtr,		/* The class to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which may be NULL;
				 * if so, up to caller to manage storage
				 * (e.g., because it is a constructor or
				 * destructor). */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which may be NULL; if so, it is equivalent
				 * to an empty list. */
    Tcl_Obj *bodyObj,		/* The body of the method, which must not be
				 * NULL. */
    ProcedureMethod **pmPtrPtr)	/* Place to write pointer to procedure method
				 * structure to allow for deeper tuning of the
				 * structure's contents. NULL if caller is not
				 * interested. */
{
    Tcl_Size argsLen;		/* TCL_INDEX_NONE => delete argsObj before exit */
    ProcedureMethod *pmPtr;
    const char *procName;
    Tcl_Method method;

    if (argsObj == NULL) {
	argsLen = TCL_INDEX_NONE;
	TclNewObj(argsObj);
	Tcl_IncrRefCount(argsObj);
	procName = "<destructor>";
    } else if (TclListObjLength(interp, argsObj, &argsLen) != TCL_OK) {
	return NULL;
    } else {
	procName = (nameObj==NULL ? "<constructor>" : TclGetString(nameObj));
    }

    pmPtr = AllocProcedureMethodRecord(flags);
    method = TclOOMakeProcMethod2(interp, clsPtr, flags, nameObj, procName,
	    argsObj, bodyObj, &procMethodType, pmPtr, &pmPtr->procPtr);

    if (argsLen == TCL_INDEX_NONE) {
	Tcl_DecrRefCount(argsObj);
    }
    if (method == NULL) {
	Tcl_Free(pmPtr);
    } else if (pmPtrPtr != NULL) {
	*pmPtrPtr = pmPtr;
    }

    return (Method *) method;
}

```

tcl8.4 8.4.20, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_CreateInterp`, lines 290–646. Retained reference `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c`.

```text
Tcl_CreateInterp()
{
    Interp *iPtr;
    Tcl_Interp *interp;
    Command *cmdPtr;
    BuiltinFunc *builtinFuncPtr;
    MathFunc *mathFuncPtr;
    Tcl_HashEntry *hPtr;
    CONST CmdInfo *cmdInfoPtr;
    int i;
    union {
	char c[sizeof(short)];
	short s;
    } order;
#ifdef TCL_COMPILE_STATS
    ByteCodeStats *statsPtr;
#endif /* TCL_COMPILE_STATS */

    TclInitSubsystems(NULL);

    /*
     * Panic if someone updated the CallFrame structure without
     * also updating the Tcl_CallFrame structure (or vice versa).
     */  

    if (sizeof(Tcl_CallFrame) < sizeof(CallFrame)) {
	/*NOTREACHED*/
	panic("Tcl_CallFrame must not be smaller than CallFrame");
    }

#if defined(_WIN32) && !defined(_WIN64)
    if (sizeof(time_t) != 4) {
	/*NOTREACHED*/
	Tcl_Panic("sys/time.h is not compatible with MSVC");
    }
    if (sizeof(Tcl_StatBuf) != 48) {
	/*NOTREACHED*/
	Tcl_Panic("sys/stat.h is not compatible with MSVC");
    }
#endif

    /*
     * Initialize support for namespaces and create the global namespace
     * (whose name is ""; an alias is "::"). This also initializes the
     * Tcl object type table and other object management code.
     */

    iPtr = (Interp *) ckalloc(sizeof(Interp));
    interp = (Tcl_Interp *) iPtr;

    iPtr->result		= iPtr->resultSpace;
    iPtr->freeProc		= NULL;
    iPtr->errorLine		= 0;
    iPtr->objResultPtr		= Tcl_NewObj();
    Tcl_IncrRefCount(iPtr->objResultPtr);
    iPtr->handle		= TclHandleCreate(iPtr);
    iPtr->globalNsPtr		= NULL;
    iPtr->hiddenCmdTablePtr	= NULL;
    iPtr->interpInfo		= NULL;
    Tcl_InitHashTable(&iPtr->mathFuncTable, TCL_STRING_KEYS);

    iPtr->numLevels = 0;
    iPtr->maxNestingDepth = MAX_NESTING_DEPTH;
    iPtr->framePtr = NULL;
    iPtr->varFramePtr = NULL;

#ifdef TCL_TIP280
    /*
     * TIP #280 - Initialize the arrays used to extend the ByteCode and
     * Proc structures.
     */
    iPtr->cmdFramePtr  = NULL;
    iPtr->linePBodyPtr = (Tcl_HashTable*) ckalloc (sizeof (Tcl_HashTable));
    iPtr->lineBCPtr    = (Tcl_HashTable*) ckalloc (sizeof (Tcl_HashTable));
    iPtr->lineLAPtr    = (Tcl_HashTable*) ckalloc (sizeof (Tcl_HashTable));
    iPtr->lineLABCPtr  = (Tcl_HashTable*) ckalloc (sizeof (Tcl_HashTable));
    Tcl_InitHashTable(iPtr->linePBodyPtr, TCL_ONE_WORD_KEYS);
    Tcl_InitHashTable(iPtr->lineBCPtr,    TCL_ONE_WORD_KEYS);
    Tcl_InitHashTable(iPtr->lineLAPtr,    TCL_ONE_WORD_KEYS);
    Tcl_InitHashTable(iPtr->lineLABCPtr,  TCL_ONE_WORD_KEYS);
    iPtr->scriptCLLocPtr = NULL;
#endif

    iPtr->activeVarTracePtr = NULL;
    iPtr->returnCode = TCL_OK;
    iPtr->errorInfo = NULL;
    iPtr->errorCode = NULL;

    iPtr->appendResult = NULL;
    iPtr->appendAvl = 0;
    iPtr->appendUsed = 0;

    Tcl_InitHashTable(&iPtr->packageTable, TCL_STRING_KEYS);
    iPtr->packageUnknown = NULL;
#ifdef TCL_TIP268
    /* TIP #268 */
    iPtr->packagePrefer = (getenv ("TCL_PKG_PREFER_LATEST") == NULL ? 
			   PKG_PREFER_STABLE   :
			   PKG_PREFER_LATEST);
#endif
    iPtr->cmdCount = 0;
    iPtr->termOffset = 0;
    TclInitLiteralTable(&(iPtr->literalTable));
    iPtr->compileEpoch = 0;
    iPtr->compiledProcPtr = NULL;
    iPtr->resolverPtr = NULL;
    iPtr->evalFlags = 0;
    iPtr->scriptFile = NULL;
    iPtr->flags = 0;
    iPtr->tracePtr = NULL;
    iPtr->tracesForbiddingInline = 0;
    iPtr->activeCmdTracePtr = NULL;
    iPtr->activeInterpTracePtr = NULL;
    iPtr->assocData = (Tcl_HashTable *) NULL;
    iPtr->execEnvPtr = NULL;	      /* set after namespaces initialized */
    iPtr->emptyObjPtr = Tcl_NewObj(); /* another empty object */
    Tcl_IncrRefCount(iPtr->emptyObjPtr);
    iPtr->resultSpace[0] = 0;
    iPtr->threadId = Tcl_GetCurrentThread();

    iPtr->globalNsPtr = NULL;	/* force creation of global ns below */
    iPtr->globalNsPtr = (Namespace *) Tcl_CreateNamespace(interp, "",
	    (ClientData) NULL, (Tcl_NamespaceDeleteProc *) NULL);
    if (iPtr->globalNsPtr == NULL) {
        panic("Tcl_CreateInterp: can't create global namespace");
    }

    /*
     * Initialize support for code compilation and execution. We call
     * TclCreateExecEnv after initializing namespaces since it tries to
     * reference a Tcl variable (it links to the Tcl "tcl_traceExec"
     * variable).
     */

    iPtr->execEnvPtr = TclCreateExecEnv(interp);

    /*
     * Initialize the compilation and execution statistics kept for this
     * interpreter.
     */

#ifdef TCL_COMPILE_STATS
    statsPtr = &(iPtr->stats);
    statsPtr->numExecutions = 0;
    statsPtr->numCompilations = 0;
    statsPtr->numByteCodesFreed = 0;
    (VOID *) memset(statsPtr->instructionCount, 0,
	    sizeof(statsPtr->instructionCount));

    statsPtr->totalSrcBytes = 0.0;
    statsPtr->totalByteCodeBytes = 0.0;
    statsPtr->currentSrcBytes = 0.0;
    statsPtr->currentByteCodeBytes = 0.0;
    (VOID *) memset(statsPtr->srcCount, 0, sizeof(statsPtr->srcCount));
    (VOID *) memset(statsPtr->byteCodeCount, 0,
	    sizeof(statsPtr->byteCodeCount));
    (VOID *) memset(statsPtr->lifetimeCount, 0,
	    sizeof(statsPtr->lifetimeCount));
    
    statsPtr->currentInstBytes   = 0.0;
    statsPtr->currentLitBytes    = 0.0;
    statsPtr->currentExceptBytes = 0.0;
    statsPtr->currentAuxBytes    = 0.0;
    statsPtr->currentCmdMapBytes = 0.0;
    
    statsPtr->numLiteralsCreated    = 0;
    statsPtr->totalLitStringBytes   = 0.0;
    statsPtr->currentLitStringBytes = 0.0;
    (VOID *) memset(statsPtr->literalCount, 0,
            sizeof(statsPtr->literalCount));
#endif /* TCL_COMPILE_STATS */    

    /*
     * Initialise the stub table pointer.
     */

    iPtr->stubTable = &tclStubs;

    
    /*
     * Create the core commands. Do it here, rather than calling
     * Tcl_CreateCommand, because it's faster (there's no need to check for
     * a pre-existing command by the same name). If a command has a
     * Tcl_CmdProc but no Tcl_ObjCmdProc, set the Tcl_ObjCmdProc to
     * TclInvokeStringCommand. This is an object-based wrapper procedure
     * that extracts strings, calls the string procedure, and creates an
     * object for the result. Similarly, if a command has a Tcl_ObjCmdProc
     * but no Tcl_CmdProc, set the Tcl_CmdProc to TclInvokeObjectCommand.
     */

    for (cmdInfoPtr = builtInCmds;  cmdInfoPtr->name != NULL;
	    cmdInfoPtr++) {
	int new;
	Tcl_HashEntry *hPtr;

	if ((cmdInfoPtr->proc == (Tcl_CmdProc *) NULL)
	        && (cmdInfoPtr->objProc == (Tcl_ObjCmdProc *) NULL)
	        && (cmdInfoPtr->compileProc == (CompileProc *) NULL)) {
	    panic("Tcl_CreateInterp: builtin command with NULL string and object command procs and a NULL compile proc\n");
	}
	
	hPtr = Tcl_CreateHashEntry(&iPtr->globalNsPtr->cmdTable,
	        cmdInfoPtr->name, &new);
	if (new) {
	    cmdPtr = (Command *) ckalloc(sizeof(Command));
	    cmdPtr->hPtr = hPtr;
	    cmdPtr->nsPtr = iPtr->globalNsPtr;
	    cmdPtr->refCount = 1;
	    cmdPtr->cmdEpoch = 0;
	    cmdPtr->compileProc = cmdInfoPtr->compileProc;
	    if (cmdInfoPtr->proc == (Tcl_CmdProc *) NULL) {
		cmdPtr->proc = TclInvokeObjectCommand;
		cmdPtr->clientData = (ClientData) cmdPtr;
	    } else {
		cmdPtr->proc = cmdInfoPtr->proc;
		cmdPtr->clientData = (ClientData) NULL;
	    }
	    if (cmdInfoPtr->objProc == (Tcl_ObjCmdProc *) NULL) {
		cmdPtr->objProc = TclInvokeStringCommand;
		cmdPtr->objClientData = (ClientData) cmdPtr;
	    } else {
		cmdPtr->objProc = cmdInfoPtr->objProc;
		cmdPtr->objClientData = (ClientData) NULL;
	    }
	    cmdPtr->deleteProc = NULL;
	    cmdPtr->deleteData = (ClientData) NULL;
	    cmdPtr->flags = 0;
	    cmdPtr->importRefPtr = NULL;
	    cmdPtr->tracePtr = NULL;
	    Tcl_SetHashValue(hPtr, cmdPtr);
	}
    }

#ifdef USE_DTRACE
    /*
     * Register the tcl::dtrace command.
     */

    Tcl_CreateObjCommand(interp, "::tcl::dtrace", DTraceObjCmd, NULL, NULL);
#endif /* USE_DTRACE */

    /*
     * Register the builtin math functions.
     */

    i = 0;
    for (builtinFuncPtr = tclBuiltinFuncTable;  builtinFuncPtr->name != NULL;
	    builtinFuncPtr++) {
	Tcl_CreateMathFunc((Tcl_Interp *) iPtr, builtinFuncPtr->name,
		builtinFuncPtr->numArgs, builtinFuncPtr->argTypes,
		(Tcl_MathProc *) NULL, (ClientData) 0);
	hPtr = Tcl_FindHashEntry(&iPtr->mathFuncTable,
		builtinFuncPtr->name);
	if (hPtr == NULL) {
	    panic("Tcl_CreateInterp: Tcl_CreateMathFunc incorrectly registered '%s'", builtinFuncPtr->name);
	    return NULL;
	}
	mathFuncPtr = (MathFunc *) Tcl_GetHashValue(hPtr);
	mathFuncPtr->builtinFuncIndex = i;
	i++;
    }
    iPtr->flags |= EXPR_INITIALIZED;

    /*
     * Do Multiple/Safe Interps Tcl init stuff
     */

    TclInterpInit(interp);

    /*
     * We used to create the "errorInfo" and "errorCode" global vars at this
     * point because so much of the Tcl implementation assumes they already
     * exist. This is not quite enough, however, since they can be unset
     * at any time.
     *
     * There are 2 choices:
     *    + Check every place where a GetVar of those is used 
     *      and the NULL result is not checked (like in tclLoad.c)
     *    + Make SetVar,... NULL friendly
     * We choose the second option because :
     *    + It is easy and low cost to check for NULL pointer before
     *      calling strlen()
     *    + It can be helpfull to other people using those API
     *    + Passing a NULL value to those closest 'meaning' is empty string
     *      (specially with the new objects where 0 bytes strings are ok)
     * So the following init is commented out:              -- dl
     *
     * (void) Tcl_SetVar2((Tcl_Interp *)iPtr, "errorInfo", (char *) NULL,
     *       "", TCL_GLOBAL_ONLY);
     * (void) Tcl_SetVar2((Tcl_Interp *)iPtr, "errorCode", (char *) NULL,
     *       "NONE", TCL_GLOBAL_ONLY);
     */

#ifndef TCL_GENERIC_ONLY
    TclSetupEnv(interp);
#endif

    /*
     * Compute the byte order of this machine.
     */

    order.s = 1;
    Tcl_SetVar2(interp, "tcl_platform", "byteOrder",
	    ((order.c[0] == 1) ? "littleEndian" : "bigEndian"),
	    TCL_GLOBAL_ONLY);

    Tcl_SetVar2Ex(interp, "tcl_platform", "wordSize",
	    Tcl_NewLongObj((long) sizeof(long)), TCL_GLOBAL_ONLY);

    /*
     * Set up other variables such as tcl_version and tcl_library
     */

    Tcl_SetVar(interp, "tcl_patchLevel", TCL_PATCH_LEVEL, TCL_GLOBAL_ONLY);
    Tcl_SetVar(interp, "tcl_version", TCL_VERSION, TCL_GLOBAL_ONLY);
    Tcl_TraceVar2(interp, "tcl_precision", (char *) NULL,
	    TCL_GLOBAL_ONLY|TCL_TRACE_READS|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    TclPrecTraceProc, (ClientData) NULL);
    TclpSetVariables(interp);

#ifdef TCL_THREADS
    /*
     * The existence of the "threaded" element of the tcl_platform array indicates
     * that this particular Tcl shell has been compiled with threads turned on.
     * Using "info exists tcl_platform(threaded)" a Tcl script can introspect on the 
     * interpreter level of thread safety.
     */

    Tcl_SetVar2(interp, "tcl_platform", "threaded", "1",
	    TCL_GLOBAL_ONLY);
#endif

    /*
     * Register Tcl's version number.
     * TIP#268: Expose information about its status,
     *          for runtime switches in the core library
     *          and tests.
     */

    Tcl_PkgProvideEx(interp, "Tcl", TCL_VERSION, (ClientData) &tclStubs);

#ifdef TCL_TIP268
    Tcl_SetVar2(interp, "tcl_platform", "tip,268", "1",
	    TCL_GLOBAL_ONLY);
#endif
#ifdef TCL_TIP280
    Tcl_SetVar2(interp, "tcl_platform", "tip,280", "1",
	    TCL_GLOBAL_ONLY);
#endif
#ifdef Tcl_InitStubs
#undef Tcl_InitStubs
#endif
    Tcl_InitStubs(interp, TCL_VERSION, 1);

    return interp;
}

```

tcl8.5 8.5.19, revision `Exact pinned release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_CreateInterp`, lines 381–854. Retained reference `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c`.

```text
Tcl_CreateInterp(void)
{
    Interp *iPtr;
    Tcl_Interp *interp;
    Command *cmdPtr;
    const BuiltinFuncDef *builtinFuncPtr;
    const OpCmdInfo *opcmdInfoPtr;
    const CmdInfo *cmdInfoPtr;
    Tcl_Namespace *mathfuncNSPtr, *mathopNSPtr;
    union {
	char c[sizeof(short)];
	short s;
    } order;
#ifdef TCL_COMPILE_STATS
    ByteCodeStats *statsPtr;
#endif /* TCL_COMPILE_STATS */
    char mathFuncName[32];
    CallFrame *framePtr;
    int result;

    TclInitSubsystems();

    /*
     * Panic if someone updated the CallFrame structure without also updating
     * the Tcl_CallFrame structure (or vice versa).
     */

    if (sizeof(Tcl_CallFrame) < sizeof(CallFrame)) {
	/*NOTREACHED*/
	Tcl_Panic("Tcl_CallFrame must not be smaller than CallFrame");
    }

#if defined(_WIN32) && !defined(_WIN64)
    if (sizeof(time_t) != 4) {
	/*NOTREACHED*/
	Tcl_Panic("<time.h> is not compatible with MSVC");
    }
    if ((TclOffset(Tcl_StatBuf,st_atime) != 32)
	    || (TclOffset(Tcl_StatBuf,st_ctime) != 40)) {
	/*NOTREACHED*/
	Tcl_Panic("<sys/stat.h> is not compatible with MSVC");
    }
#endif

    /*
     * Initialize support for namespaces and create the global namespace
     * (whose name is ""; an alias is "::"). This also initializes the Tcl
     * object type table and other object management code.
     */

    iPtr = (Interp *) ckalloc(sizeof(Interp));
    interp = (Tcl_Interp *) iPtr;

    iPtr->result = iPtr->resultSpace;
    iPtr->freeProc = NULL;
    iPtr->errorLine = 0;
    iPtr->objResultPtr = Tcl_NewObj();
    Tcl_IncrRefCount(iPtr->objResultPtr);
    iPtr->handle = TclHandleCreate(iPtr);
    iPtr->globalNsPtr = NULL;
    iPtr->hiddenCmdTablePtr = NULL;
    iPtr->interpInfo = NULL;

    iPtr->numLevels = 0;
    iPtr->maxNestingDepth = MAX_NESTING_DEPTH;
    iPtr->framePtr = NULL;	/* Initialise as soon as :: is available */
    iPtr->varFramePtr = NULL;	/* Initialise as soon as :: is available */

    /*
     * TIP #280 - Initialize the arrays used to extend the ByteCode and
     * Proc structures.
     */

    iPtr->cmdFramePtr = NULL;
    iPtr->linePBodyPtr = (Tcl_HashTable *) ckalloc(sizeof(Tcl_HashTable));
    iPtr->lineBCPtr = (Tcl_HashTable *) ckalloc(sizeof(Tcl_HashTable));
    iPtr->lineLAPtr = (Tcl_HashTable*) ckalloc (sizeof (Tcl_HashTable));
    iPtr->lineLABCPtr = (Tcl_HashTable*) ckalloc (sizeof (Tcl_HashTable));
    Tcl_InitHashTable(iPtr->linePBodyPtr, TCL_ONE_WORD_KEYS);
    Tcl_InitHashTable(iPtr->lineBCPtr, TCL_ONE_WORD_KEYS);
    Tcl_InitHashTable(iPtr->lineLAPtr, TCL_ONE_WORD_KEYS);
    Tcl_InitHashTable(iPtr->lineLABCPtr, TCL_ONE_WORD_KEYS);
    iPtr->scriptCLLocPtr = NULL;

    iPtr->activeVarTracePtr = NULL;

    iPtr->returnOpts = NULL;
    iPtr->errorInfo = NULL;
    TclNewLiteralStringObj(iPtr->eiVar, "::errorInfo");
    Tcl_IncrRefCount(iPtr->eiVar);
    iPtr->errorCode = NULL;
    TclNewLiteralStringObj(iPtr->ecVar, "::errorCode");
    Tcl_IncrRefCount(iPtr->ecVar);
    iPtr->returnLevel = 1;
    iPtr->returnCode = TCL_OK;

    iPtr->rootFramePtr = NULL;	/* Initialise as soon as :: is available */
    iPtr->lookupNsPtr = NULL;

    iPtr->appendResult = NULL;
    iPtr->appendAvl = 0;
    iPtr->appendUsed = 0;

    Tcl_InitHashTable(&iPtr->packageTable, TCL_STRING_KEYS);
    iPtr->packageUnknown = NULL;

    /* TIP #268 */
    if (getenv("TCL_PKG_PREFER_LATEST") == NULL) {
	iPtr->packagePrefer = PKG_PREFER_STABLE;
    } else {
	iPtr->packagePrefer = PKG_PREFER_LATEST;
    }

    iPtr->cmdCount = 0;
    TclInitLiteralTable(&(iPtr->literalTable));
    iPtr->compileEpoch = 0;
    iPtr->compiledProcPtr = NULL;
    iPtr->resolverPtr = NULL;
    iPtr->evalFlags = 0;
    iPtr->scriptFile = NULL;
    iPtr->flags = 0;
    iPtr->tracePtr = NULL;
    iPtr->tracesForbiddingInline = 0;
    iPtr->activeCmdTracePtr = NULL;
    iPtr->activeInterpTracePtr = NULL;
    iPtr->assocData = NULL;
    iPtr->execEnvPtr = NULL;	/* Set after namespaces initialized. */
    iPtr->emptyObjPtr = Tcl_NewObj();
				/* Another empty object. */
    Tcl_IncrRefCount(iPtr->emptyObjPtr);
    iPtr->resultSpace[0] = 0;
    iPtr->threadId = Tcl_GetCurrentThread();

    /* TIP #378 */
#ifdef TCL_INTERP_DEBUG_FRAME
    iPtr->flags |= INTERP_DEBUG_FRAME;
#else
    if (getenv("TCL_INTERP_DEBUG_FRAME") != NULL) {
        iPtr->flags |= INTERP_DEBUG_FRAME;
    }
#endif

    /*
     * Initialise the tables for variable traces and searches *before*
     * creating the global ns - so that the trace on errorInfo can be
     * recorded.
     */

    Tcl_InitHashTable(&iPtr->varTraces, TCL_ONE_WORD_KEYS);
    Tcl_InitHashTable(&iPtr->varSearches, TCL_ONE_WORD_KEYS);

    iPtr->globalNsPtr = NULL;	/* Force creation of global ns below. */
    iPtr->globalNsPtr = (Namespace *) Tcl_CreateNamespace(interp, "",
	    NULL, NULL);
    if (iPtr->globalNsPtr == NULL) {
	Tcl_Panic("Tcl_CreateInterp: can't create global namespace");
    }

    /*
     * Initialise the rootCallframe. It cannot be allocated on the stack, as
     * it has to be in place before TclCreateExecEnv tries to use a variable.
     */

    /* This is needed to satisfy GCC 3.3's strict aliasing rules */
    framePtr = (CallFrame *) ckalloc(sizeof(CallFrame));
    result = Tcl_PushCallFrame(interp, (Tcl_CallFrame *) framePtr,
	    (Tcl_Namespace *) iPtr->globalNsPtr, /*isProcCallFrame*/ 0);
    if (result != TCL_OK) {
	Tcl_Panic("Tcl_CreateInterp: failed to push the root stack frame");
    }
    framePtr->objc = 0;

    iPtr->framePtr = framePtr;
    iPtr->varFramePtr = framePtr;
    iPtr->rootFramePtr = framePtr;

    /*
     * Initialize support for code compilation and execution. We call
     * TclCreateExecEnv after initializing namespaces since it tries to
     * reference a Tcl variable (it links to the Tcl "tcl_traceExec"
     * variable).
     */

    iPtr->execEnvPtr = TclCreateExecEnv(interp);

    /*
     * TIP #219, Tcl Channel Reflection API support.
     */

    iPtr->chanMsg = NULL;

    /*
     * Initialize the compilation and execution statistics kept for this
     * interpreter.
     */

#ifdef TCL_COMPILE_STATS
    statsPtr = &(iPtr->stats);
    statsPtr->numExecutions = 0;
    statsPtr->numCompilations = 0;
    statsPtr->numByteCodesFreed = 0;
    (void) memset(statsPtr->instructionCount, 0,
	    sizeof(statsPtr->instructionCount));

    statsPtr->totalSrcBytes = 0.0;
    statsPtr->totalByteCodeBytes = 0.0;
    statsPtr->currentSrcBytes = 0.0;
    statsPtr->currentByteCodeBytes = 0.0;
    (void) memset(statsPtr->srcCount, 0, sizeof(statsPtr->srcCount));
    (void) memset(statsPtr->byteCodeCount, 0, sizeof(statsPtr->byteCodeCount));
    (void) memset(statsPtr->lifetimeCount, 0, sizeof(statsPtr->lifetimeCount));

    statsPtr->currentInstBytes = 0.0;
    statsPtr->currentLitBytes = 0.0;
    statsPtr->currentExceptBytes = 0.0;
    statsPtr->currentAuxBytes = 0.0;
    statsPtr->currentCmdMapBytes = 0.0;

    statsPtr->numLiteralsCreated = 0;
    statsPtr->totalLitStringBytes = 0.0;
    statsPtr->currentLitStringBytes = 0.0;
    (void) memset(statsPtr->literalCount, 0, sizeof(statsPtr->literalCount));
#endif /* TCL_COMPILE_STATS */

    /*
     * Initialise the stub table pointer.
     */

    iPtr->stubTable = &tclStubs;

    /*
     * Initialize the ensemble error message rewriting support.
     */

    iPtr->ensembleRewrite.sourceObjs = NULL;
    iPtr->ensembleRewrite.numRemovedObjs = 0;
    iPtr->ensembleRewrite.numInsertedObjs = 0;

    /*
     * TIP#143: Initialise the resource limit support.
     */

    TclInitLimitSupport(interp);

    /*
     * Initialise the thread-specific data ekeko.
     */

#if defined(TCL_THREADS) && defined(USE_THREAD_ALLOC)
    iPtr->allocCache = TclpGetAllocCache();
#else
    iPtr->allocCache = NULL;
#endif
    iPtr->pendingObjDataPtr = NULL;
    iPtr->asyncReadyPtr = TclGetAsyncReadyPtr();

    /*
     * Insure that the stack checking mechanism for this interp is
     * initialized.
     */

    GetCStackParams(iPtr);

    /*
     * Create the core commands. Do it here, rather than calling
     * Tcl_CreateCommand, because it's faster (there's no need to check for a
     * pre-existing command by the same name). If a command has a Tcl_CmdProc
     * but no Tcl_ObjCmdProc, set the Tcl_ObjCmdProc to
     * TclInvokeStringCommand. This is an object-based wrapper function that
     * extracts strings, calls the string function, and creates an object for
     * the result. Similarly, if a command has a Tcl_ObjCmdProc but no
     * Tcl_CmdProc, set the Tcl_CmdProc to TclInvokeObjectCommand.
     */

    for (cmdInfoPtr = builtInCmds;  cmdInfoPtr->name != NULL; cmdInfoPtr++) {
	int isNew;
	Tcl_HashEntry *hPtr;

	if ((cmdInfoPtr->objProc == NULL)
		&& (cmdInfoPtr->compileProc == NULL)) {
	    Tcl_Panic("builtin command with NULL object command proc and a NULL compile proc");
	}

	hPtr = Tcl_CreateHashEntry(&iPtr->globalNsPtr->cmdTable,
		cmdInfoPtr->name, &isNew);
	if (isNew) {
	    cmdPtr = (Command *) ckalloc(sizeof(Command));
	    cmdPtr->hPtr = hPtr;
	    cmdPtr->nsPtr = iPtr->globalNsPtr;
	    cmdPtr->refCount = 1;
	    cmdPtr->cmdEpoch = 0;
	    cmdPtr->compileProc = cmdInfoPtr->compileProc;
	    cmdPtr->proc = TclInvokeObjectCommand;
	    cmdPtr->clientData = cmdPtr;
	    cmdPtr->objProc = cmdInfoPtr->objProc;
	    cmdPtr->objClientData = NULL;
	    cmdPtr->deleteProc = NULL;
	    cmdPtr->deleteData = NULL;
	    cmdPtr->flags = 0;
	    cmdPtr->importRefPtr = NULL;
	    cmdPtr->tracePtr = NULL;
	    Tcl_SetHashValue(hPtr, cmdPtr);
	}
    }

    /*
     * Create the "chan", "dict", "info" and "string" ensembles. Note that all
     * these commands (and their subcommands that are not present in the
     * global namespace) are wholly safe.
     */

    TclInitChanCmd(interp);
    TclInitDictCmd(interp);
    TclInitInfoCmd(interp);
    TclInitStringCmd(interp);

    /*
     * Register "clock" subcommands. These *do* go through
     * Tcl_CreateObjCommand, since they aren't in the global namespace and
     * involve ensembles.
     */

    TclClockInit(interp);

    /*
     * Register the built-in functions. This is empty now that they are
     * implemented as commands in the ::tcl::mathfunc namespace.
     */

    /*
     * Register the default [interp bgerror] handler.
     */

    Tcl_CreateObjCommand(interp, "::tcl::Bgerror",
	    TclDefaultBgErrorHandlerObjCmd, NULL, NULL);

    /*
     * Create an unsupported command for debugging bytecode.
     */

    Tcl_CreateObjCommand(interp, "::tcl::unsupported::disassemble",
	    Tcl_DisassembleObjCmd, NULL, NULL);

#ifdef USE_DTRACE
    /*
     * Register the tcl::dtrace command.
     */

    Tcl_CreateObjCommand(interp, "::tcl::dtrace", DTraceObjCmd, NULL, NULL);
#endif /* USE_DTRACE */

    /*
     * Register the builtin math functions.
     */

    mathfuncNSPtr = Tcl_CreateNamespace(interp, "::tcl::mathfunc", NULL,NULL);
    if (mathfuncNSPtr == NULL) {
	Tcl_Panic("Can't create math function namespace");
    }
    strcpy(mathFuncName, "::tcl::mathfunc::");
#define MATH_FUNC_PREFIX_LEN 17 /* == strlen("::tcl::mathfunc::") */
    for (builtinFuncPtr = BuiltinFuncTable; builtinFuncPtr->name != NULL;
	    builtinFuncPtr++) {
	strcpy(mathFuncName+MATH_FUNC_PREFIX_LEN, builtinFuncPtr->name);
	Tcl_CreateObjCommand(interp, mathFuncName,
		builtinFuncPtr->objCmdProc, builtinFuncPtr->clientData, NULL);
	Tcl_Export(interp, mathfuncNSPtr, builtinFuncPtr->name, 0);
    }

    /*
     * Register the mathematical "operator" commands. [TIP #174]
     */

    mathopNSPtr = Tcl_CreateNamespace(interp, "::tcl::mathop", NULL, NULL);
#define MATH_OP_PREFIX_LEN 15 /* == strlen("::tcl::mathop::") */
    if (mathopNSPtr == NULL) {
	Tcl_Panic("can't create math operator namespace");
    }
    (void) Tcl_Export(interp, mathopNSPtr, "*", 1);
    strcpy(mathFuncName, "::tcl::mathop::");
    for (opcmdInfoPtr=mathOpCmds ; opcmdInfoPtr->name!=NULL ; opcmdInfoPtr++){
	TclOpCmdClientData *occdPtr = (TclOpCmdClientData *)
		ckalloc(sizeof(TclOpCmdClientData));

	occdPtr->op = opcmdInfoPtr->name;
	occdPtr->i.numArgs = opcmdInfoPtr->i.numArgs;
	occdPtr->expected = opcmdInfoPtr->expected;
	strcpy(mathFuncName + MATH_OP_PREFIX_LEN, opcmdInfoPtr->name);
	cmdPtr = (Command *) Tcl_CreateObjCommand(interp, mathFuncName,
		opcmdInfoPtr->objProc, occdPtr, DeleteOpCmdClientData);
	if (cmdPtr == NULL) {
	    Tcl_Panic("failed to create math operator %s",
		    opcmdInfoPtr->name);
	} else if (opcmdInfoPtr->compileProc != NULL) {
	    cmdPtr->compileProc = opcmdInfoPtr->compileProc;
	}
    }

    /*
     * Do Multiple/Safe Interps Tcl init stuff
     */

    TclInterpInit(interp);
    TclSetupEnv(interp);

    /*
     * TIP #59: Make embedded configuration information available.
     */

    TclInitEmbeddedConfigurationInformation(interp);

    /*
     * TIP #440: Declare the name of the script engine to be "Tcl".
     */

    Tcl_SetVar2(interp, "tcl_platform", "engine", "Tcl",
	    TCL_GLOBAL_ONLY);

    /*
     * Compute the byte order of this machine.
     */

    order.s = 1;
    Tcl_SetVar2(interp, "tcl_platform", "byteOrder",
	    ((order.c[0] == 1) ? "littleEndian" : "bigEndian"),
	    TCL_GLOBAL_ONLY);

    Tcl_SetVar2Ex(interp, "tcl_platform", "wordSize",
	    Tcl_NewLongObj((long) sizeof(long)), TCL_GLOBAL_ONLY);

    /* TIP #291 */
    Tcl_SetVar2Ex(interp, "tcl_platform", "pointerSize",
	    Tcl_NewLongObj((long) sizeof(void *)), TCL_GLOBAL_ONLY);

    /*
     * Set up other variables such as tcl_version and tcl_library
     */

    Tcl_SetVar(interp, "tcl_patchLevel", TCL_PATCH_LEVEL, TCL_GLOBAL_ONLY);
    Tcl_SetVar(interp, "tcl_version", TCL_VERSION, TCL_GLOBAL_ONLY);
    Tcl_TraceVar2(interp, "tcl_precision", NULL,
	    TCL_GLOBAL_ONLY|TCL_TRACE_READS|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    TclPrecTraceProc, NULL);
    TclpSetVariables(interp);

#ifdef TCL_THREADS
    /*
     * The existence of the "threaded" element of the tcl_platform array
     * indicates that this particular Tcl shell has been compiled with threads
     * turned on. Using "info exists tcl_platform(threaded)" a Tcl script can
     * introspect on the interpreter level of thread safety.
     */

    Tcl_SetVar2(interp, "tcl_platform", "threaded", "1", TCL_GLOBAL_ONLY);
#endif

    /*
     * Register Tcl's version number.
     * TIP #268: Full patchlevel instead of just major.minor
     */

    Tcl_PkgProvideEx(interp, "Tcl", TCL_PATCH_LEVEL, &tclStubs);

#ifdef Tcl_InitStubs
#undef Tcl_InitStubs
#endif
    Tcl_InitStubs(interp, TCL_VERSION, 1);

    if (TclTommath_Init(interp) != TCL_OK) {
	Tcl_Panic("%s", Tcl_GetString(Tcl_GetObjResult(interp)));
    }

    return interp;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole pinned jim.c retained`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_RegisterCoreCommands`, lines 16919–16928. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
void Jim_RegisterCoreCommands(Jim_Interp *interp)
{
    const struct jim_core_cmd_def_t *c;

    for (c = Jim_CoreCommandsTable; c->name; c++) {
        /* All core commands must have usage */
        assert(c->usage);
        Jim_RegisterCmd(interp, c->name, c->usage, c->minargs, c->maxargs, c->cmdProc, NULL, NULL, c->flags);
    }
}

```

## Consumer bindings

- [runtime/rust/src/cmd_oo/native_context.rs](../../../../runtime/rust/src/cmd_oo/native_context.rs), `cmd_oo::native_context::original_forward_prefix_tests::original_forward_definition_and_raw_report_match_original_member_stages` (linked): Finite source-defined control: original forward definition and raw report match original member stages. Fixed C86/C90/C91 ROW comparisons; presentation is an explicit later read. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_oo/native_context.rs](../../../../runtime/rust/src/cmd_oo/native_context.rs), `cmd_oo::native_context::original_forward_prefix_tests::original_forward_invocation_passes_exact_member_before_its_worker_getter` (linked): Finite source-defined control: original forward invocation passes exact member before its worker getter. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_oo/native_context.rs](../../../../runtime/rust/src/cmd_oo/native_context.rs), `cmd_oo::native_context::original_forward_prefix_tests::forward_declaration_clone_shares_the_retained_list_without_member_reference_donation` (linked): Finite source-defined control: forward declaration clone shares the retained list without member reference donation. Software sharing control for original CloneForwardMethod's source recipe; original nine-case rows contain no independent native clone observation. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_oo/native_context.rs](../../../../runtime/rust/src/cmd_oo/native_context.rs), `cmd_oo::native_context::original_definition_frame_tests::original_definition_entry_waits_for_a_report_and_preserves_raw_list_members` (linked): Finite source-defined control: original definition entry waits for a report and preserves raw list members. Match String/update phases and the report's native member +1 delta. The direct software frame fixture has no public EvalObjv argv +1 role. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-test-support/tests/data/native_resolution_reference_builds](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds)
- [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request)
- [rust/tcl-registry/tests/data/native_oo_operand_stages_original/source-windows](../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/source-windows)
