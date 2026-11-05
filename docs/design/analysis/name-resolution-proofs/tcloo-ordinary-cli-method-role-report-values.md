# naming.tcloo.ordinary-cli-method-role-report-values

Kind: `native-observation`

## Problem statement

The public bytes reported by self method can equal a constructor/destructor literal for either a genuine special call or an ordinary named method. A printed-value experiment must retain that ambiguity and each provider channel without authenticating physical Foundation headers or call roles.

## Question

Which exact counted self method values do original ordinary C86/C90/C91 CLI scripts report for constructor/destructor next, a named literal and an active named rename, and what initialized command availability is actually observed on the other original CLIs?

## Conclusion

Original513 retains nineteen closed process records: all five C version/availability channels and eighteen counted values from three supported C providers, plus Jim ldd whose unprepinned zstd dependency withholds its CLI probes. Independent Jim-only513b retains three successful channels with version0.84 and initialized availability0/0/0. Supported C scripts report <constructor> for both constructor next calls and the named literal, moved for active rename, and <destructor> for both destructor next calls. C84/C85 retain availability0/0/0 without a value script. The twenty-two actual channels are individually retained; they are not a single whole-six successful513 request. Printed bytes establish no report-role identity, native header/refcount, Method record/clientData lifetime or Runtime/VM outcome.

## Scope

Two independently fixed original ordinary CLI requests and exact scripts, source excerpts, launch/dependency/version/availability/value streams, nested process receipts and unchanged input associations. Original513 Jim dependency refusal and absent probe channels remain explicit;513b supplies only its own Jim dependency/version/availability channels. Eighteen counted value rows from C86/C90/C91 remain public script results. Initialized availability is finite and does not establish global optional-package absence. Source/API Foundation header facts and the implementation-contract582 four unrun controls are separate, as are Method506 record/clientData observations. No native object identity/count, handler/compiler/frame/body entry, source edit, Rust assertion or measured performance result is inferred. SDK source/library inputs are retained; configure/build processes lack replay receipts.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original ordinary CLI argv/script values.tcl or initialized availability.tcl; no embedded public Method worker or physical object observer. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

The original version and initialized availability channels close with availability0/0/0. No OO value script executes. This finite initialized availability supplies no global optional-package absence, report value, header/count or role-identity semantics.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original ordinary CLI argv/script values.tcl or initialized availability.tcl; no embedded public Method worker or physical object observer. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The six exact public script results are <constructor> for derived/base constructors through next and the ordinary named literal; moved for the active renamed named method; and <destructor> for derived/base destructors through next. Initialized command availability is1/1/1. These counted printed values supply no report-role identity, native header/refcount or Method payload-lifetime observation.

### jim

Status: `observed`. Channel: Original ordinary CLI argv/script values.tcl or initialized availability.tcl; no embedded public Method worker or physical object observer. Version: jim 0.84.

Original513 closes only ldd: its real unprepinned zstd dependency prevents version, availability and value channels. Independent Jim-only513b closes three channels with actual version0.84 and initialized availability0/0/0; no OO value script executes. The two original requests remain independent and no whole-six513 success is inferred.

### bigip

Status: `not-tested`.

No original BIG-IP ordinary CLI method-role value/availability process.

## Source inspection

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.c`, function `TclNewLiteralStringObj(fPtr->constructorName`, lines 345–356. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-8.6.18-tclOO.c`.

```text
    fPtr->epoch = 0;
    fPtr->tsdPtr = tsdPtr;
    TclNewLiteralStringObj(fPtr->unknownMethodNameObj, "unknown");
    TclNewLiteralStringObj(fPtr->constructorName, "<constructor>");
    TclNewLiteralStringObj(fPtr->destructorName, "<destructor>");
    TclNewLiteralStringObj(fPtr->clonedName, "<cloned>");
    TclNewLiteralStringObj(fPtr->defineName, "::oo::define");
    Tcl_IncrRefCount(fPtr->unknownMethodNameObj);
    Tcl_IncrRefCount(fPtr->constructorName);
    Tcl_IncrRefCount(fPtr->destructorName);
    Tcl_IncrRefCount(fPtr->clonedName);
    Tcl_IncrRefCount(fPtr->defineName);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.c`, function `Tcl_IncrRefCount(fPtr->constructorName`, lines 350–361. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-8.6.18-tclOO.c`.

```text
    TclNewLiteralStringObj(fPtr->clonedName, "<cloned>");
    TclNewLiteralStringObj(fPtr->defineName, "::oo::define");
    Tcl_IncrRefCount(fPtr->unknownMethodNameObj);
    Tcl_IncrRefCount(fPtr->constructorName);
    Tcl_IncrRefCount(fPtr->destructorName);
    Tcl_IncrRefCount(fPtr->clonedName);
    Tcl_IncrRefCount(fPtr->defineName);
    Tcl_CreateObjCommand(interp, "::oo::UnknownDefinition",
	    TclOOUnknownDefinition, NULL, NULL);
    TclNewLiteralStringObj(namePtr, "::oo::UnknownDefinition");
    Tcl_SetNamespaceUnknownHandler(interp, fPtr->defineNs, namePtr);
    Tcl_SetNamespaceUnknownHandler(interp, fPtr->objdefNs, namePtr);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.c`, function `TclDecrRefCount(fPtr->constructorName`, lines 588–599. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-8.6.18-tclOO.c`.

```text
    Foundation *fPtr = GetFoundation(interp);

    TclDecrRefCount(fPtr->unknownMethodNameObj);
    TclDecrRefCount(fPtr->constructorName);
    TclDecrRefCount(fPtr->destructorName);
    TclDecrRefCount(fPtr->clonedName);
    TclDecrRefCount(fPtr->defineName);
    TclOODecrRefCount(fPtr->objectCls->thisPtr);
    TclOODecrRefCount(fPtr->classCls->thisPtr);

    ckfree(fPtr);
}

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.c`, function `TclOOGetCallContext(oPtr, NULL, DESTRUCTOR`, lines 1141–1152. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-8.6.18-tclOO.c`.

```text

    if (!Tcl_InterpDeleted(interp) && !(oPtr->flags & DESTRUCTOR_CALLED)) {
	CallContext *contextPtr =
		TclOOGetCallContext(oPtr, NULL, DESTRUCTOR, NULL);

	oPtr->flags |= DESTRUCTOR_CALLED;
	if (contextPtr != NULL) {
	    int result;
	    Tcl_InterpState state;

	    contextPtr->callPtr->flags |= DESTRUCTOR;
	    contextPtr->skip = 0;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.c`, function `TclOOGetCallContext(oPtr, NULL, CONSTRUCTOR`, lines 1663–1674. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-8.6.18-tclOO.c`.

```text

    if (objc >= 0) {
	CallContext *contextPtr =
		TclOOGetCallContext(oPtr, NULL, CONSTRUCTOR, NULL);

	if (contextPtr != NULL) {
	    int isRoot, result;
	    Tcl_InterpState state;

	    state = Tcl_SaveInterpState(interp, TCL_OK);
	    contextPtr->callPtr->flags |= CONSTRUCTOR;
	    contextPtr->skip = skip;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOBasic.c`, function `Exact selected report or original call-context source window`, lines 1030–1046. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOBasic.c`.

```text
	Tcl_SetObjResult(interp, TclOOObjectName(interp, clsPtr->thisPtr));
	return TCL_OK;
    }
    case SELF_METHOD:
	if (contextPtr->callPtr->flags & CONSTRUCTOR) {
	    Tcl_SetObjResult(interp, contextPtr->oPtr->fPtr->constructorName);
	} else if (contextPtr->callPtr->flags & DESTRUCTOR) {
	    Tcl_SetObjResult(interp, contextPtr->oPtr->fPtr->destructorName);
	} else {
	    Tcl_SetObjResult(interp,
		    CurrentlyInvoked(contextPtr).mPtr->namePtr);
	}
	return TCL_OK;
    case SELF_FILTER:
	if (!CurrentlyInvoked(contextPtr).isFilter) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "not inside a filtering context", -1));

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `Exact selected report or original call-context source window`, lines 899–910. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
     * there is a call into stereotypical object after it has finished running
     * its destructor phase. Such things can't be cached for a long time so the
     * epoch can be bogus. [Bug 7842f33a5c]
     */

    callPtr->flags = flags &
	    (PUBLIC_METHOD | PRIVATE_METHOD | SPECIAL | FILTER_HANDLING);
    if (oPtr->flags & USE_CLASS_CACHE) {
	oPtr = (oPtr->selfCls ? oPtr->selfCls->thisPtr : NULL);
	callPtr->flags |= USE_CLASS_CACHE;
    }
    if (oPtr) {

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `Exact selected report or original call-context source window`, lines 978–989. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
TclOOGetCallContext(
    Object *oPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags,			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
    Tcl_Obj *cacheInThisObj)	/* What object to cache in, or NULL if it is
				 * to be in the same object as the
				 * methodNameObj. */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `Exact selected report or original call-context source window`, lines 305–318. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

    /*
     * Run the method implementation.
     */

    return mPtr->typePtr->callProc(mPtr->clientData, interp,
	    (Tcl_ObjectContext) contextPtr, objc, objv);
}

static int

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `TclNewLiteralStringObj(fPtr->constructorName`, lines 423–434. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclOO.c`.

```text
    fPtr->tsdPtr = tsdPtr;

    TclNewLiteralStringObj(fPtr->unknownMethodNameObj, "unknown");
    TclNewLiteralStringObj(fPtr->constructorName, "<constructor>");
    TclNewLiteralStringObj(fPtr->destructorName, "<destructor>");
    TclNewLiteralStringObj(fPtr->clonedName, "<cloned>");
    TclNewLiteralStringObj(fPtr->defineName, "::oo::define");
    TclNewLiteralStringObj(fPtr->myName, "my");
    TclNewLiteralStringObj(fPtr->slotGetName, "Get");
    TclNewLiteralStringObj(fPtr->slotSetName, "Set");
    TclNewLiteralStringObj(fPtr->slotResolveName, "Resolve");
    TclNewLiteralStringObj(fPtr->slotDefOpName, "--default-operation");

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `Tcl_IncrRefCount(fPtr->constructorName`, lines 433–444. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclOO.c`.

```text
    TclNewLiteralStringObj(fPtr->slotResolveName, "Resolve");
    TclNewLiteralStringObj(fPtr->slotDefOpName, "--default-operation");
    Tcl_IncrRefCount(fPtr->unknownMethodNameObj);
    Tcl_IncrRefCount(fPtr->constructorName);
    Tcl_IncrRefCount(fPtr->destructorName);
    Tcl_IncrRefCount(fPtr->clonedName);
    Tcl_IncrRefCount(fPtr->defineName);
    Tcl_IncrRefCount(fPtr->myName);
    Tcl_IncrRefCount(fPtr->slotGetName);
    Tcl_IncrRefCount(fPtr->slotSetName);
    Tcl_IncrRefCount(fPtr->slotResolveName);
    Tcl_IncrRefCount(fPtr->slotDefOpName);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `TclDecrRefCount(fPtr->constructorName`, lines 818–829. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclOO.c`.

```text
    Foundation *fPtr = GetFoundation(interp);

    TclDecrRefCount(fPtr->unknownMethodNameObj);
    TclDecrRefCount(fPtr->constructorName);
    TclDecrRefCount(fPtr->destructorName);
    TclDecrRefCount(fPtr->clonedName);
    TclDecrRefCount(fPtr->defineName);
    TclDecrRefCount(fPtr->myName);
    TclDecrRefCount(fPtr->slotGetName);
    TclDecrRefCount(fPtr->slotSetName);
    TclDecrRefCount(fPtr->slotResolveName);
    TclDecrRefCount(fPtr->slotDefOpName);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `TclOOGetCallContext(oPtr, NULL, DESTRUCTOR`, lines 1439–1450. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclOO.c`.

```text

    if (!Tcl_InterpDeleted(interp) && !(oPtr->flags & DESTRUCTOR_CALLED)) {
	CallContext *contextPtr =
		TclOOGetCallContext(oPtr, NULL, DESTRUCTOR, NULL, NULL, NULL);

	oPtr->flags |= DESTRUCTOR_CALLED;
	if (contextPtr != NULL) {
	    int result;
	    Tcl_InterpState state;

	    contextPtr->callPtr->flags |= DESTRUCTOR;
	    contextPtr->skip = 0;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `TclOOGetCallContext(oPtr, NULL, CONSTRUCTOR`, lines 1990–2001. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclOO.c`.

```text

    if (objc != TCL_INDEX_NONE) {
	CallContext *contextPtr =
		TclOOGetCallContext(oPtr, NULL, CONSTRUCTOR, NULL, NULL, NULL);

	if (contextPtr != NULL) {
	    int isRoot, result;
	    Tcl_InterpState state;

	    state = Tcl_SaveInterpState(interp, TCL_OK);
	    contextPtr->callPtr->flags |= CONSTRUCTOR;
	    contextPtr->skip = skip;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOBasic.c`, function `Exact selected report or original call-context source window`, lines 1555–1570. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOBasic.c`.

```text
	Tcl_SetObjResult(interp, TclOOObjectName(interp, clsPtr->thisPtr));
	return TCL_OK;
    }
    case SELF_METHOD:
	if (contextPtr->callPtr->flags & CONSTRUCTOR) {
	    Tcl_SetObjResult(interp, contextPtr->oPtr->fPtr->constructorName);
	} else if (contextPtr->callPtr->flags & DESTRUCTOR) {
	    Tcl_SetObjResult(interp, contextPtr->oPtr->fPtr->destructorName);
	} else {
	    Tcl_SetObjResult(interp,
		    CurrentlyInvoked(contextPtr).mPtr->namePtr);
	}
	return TCL_OK;
    case SELF_FILTER:
	if (!CurrentlyInvoked(contextPtr).isFilter) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `TclNewLiteralStringObj(fPtr->constructorName`, lines 426–437. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclOO.c`.

```text
    fPtr->tsdPtr = tsdPtr;

    TclNewLiteralStringObj(fPtr->unknownMethodNameObj, "unknown");
    TclNewLiteralStringObj(fPtr->constructorName, "<constructor>");
    TclNewLiteralStringObj(fPtr->destructorName, "<destructor>");
    TclNewLiteralStringObj(fPtr->clonedName, "<cloned>");
    TclNewLiteralStringObj(fPtr->defineName, "::oo::define");
    TclNewLiteralStringObj(fPtr->myName, "my");
    TclNewLiteralStringObj(fPtr->slotGetName, "Get");
    TclNewLiteralStringObj(fPtr->slotSetName, "Set");
    TclNewLiteralStringObj(fPtr->slotResolveName, "Resolve");
    TclNewLiteralStringObj(fPtr->slotDefOpName, "--default-operation");

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `Tcl_IncrRefCount(fPtr->constructorName`, lines 436–447. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclOO.c`.

```text
    TclNewLiteralStringObj(fPtr->slotResolveName, "Resolve");
    TclNewLiteralStringObj(fPtr->slotDefOpName, "--default-operation");
    Tcl_IncrRefCount(fPtr->unknownMethodNameObj);
    Tcl_IncrRefCount(fPtr->constructorName);
    Tcl_IncrRefCount(fPtr->destructorName);
    Tcl_IncrRefCount(fPtr->clonedName);
    Tcl_IncrRefCount(fPtr->defineName);
    Tcl_IncrRefCount(fPtr->myName);
    Tcl_IncrRefCount(fPtr->slotGetName);
    Tcl_IncrRefCount(fPtr->slotSetName);
    Tcl_IncrRefCount(fPtr->slotResolveName);
    Tcl_IncrRefCount(fPtr->slotDefOpName);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `TclDecrRefCount(fPtr->constructorName`, lines 806–817. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclOO.c`.

```text
    Foundation *fPtr = GetFoundation(interp);

    TclDecrRefCount(fPtr->unknownMethodNameObj);
    TclDecrRefCount(fPtr->constructorName);
    TclDecrRefCount(fPtr->destructorName);
    TclDecrRefCount(fPtr->clonedName);
    TclDecrRefCount(fPtr->defineName);
    TclDecrRefCount(fPtr->myName);
    TclDecrRefCount(fPtr->slotGetName);
    TclDecrRefCount(fPtr->slotSetName);
    TclDecrRefCount(fPtr->slotResolveName);
    TclDecrRefCount(fPtr->slotDefOpName);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `TclOOGetCallContext(oPtr, NULL, DESTRUCTOR`, lines 1432–1443. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclOO.c`.

```text

    if (!Tcl_InterpDeleted(interp) && !(oPtr->flags & DESTRUCTOR_CALLED)) {
	CallContext *contextPtr =
		TclOOGetCallContext(oPtr, NULL, DESTRUCTOR, NULL, NULL, NULL);

	oPtr->flags |= DESTRUCTOR_CALLED;
	if (contextPtr != NULL) {
	    int result;
	    Tcl_InterpState state;

	    contextPtr->callPtr->flags |= DESTRUCTOR;
	    contextPtr->skip = 0;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `TclOOGetCallContext(oPtr, NULL, CONSTRUCTOR`, lines 1970–1981. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclOO.c`.

```text

    if (objc != TCL_INDEX_NONE) {
	CallContext *contextPtr =
		TclOOGetCallContext(oPtr, NULL, CONSTRUCTOR, NULL, NULL, NULL);

	if (contextPtr != NULL) {
	    int isRoot, result;
	    Tcl_InterpState state;

	    state = Tcl_SaveInterpState(interp, TCL_OK);
	    contextPtr->callPtr->flags |= CONSTRUCTOR;
	    contextPtr->skip = skip;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOBasic.c`, function `Exact selected report or original call-context source window`, lines 1510–1527. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOBasic.c`.

```text
 * ----------------------------------------------------------------------
 */
static inline Tcl_Obj *
MethodName(
    Foundation *fPtr,
    CallContext *contextPtr,
    Method *mPtr)
{
    switch (contextPtr->callPtr->flags & (CONSTRUCTOR | DESTRUCTOR)) {
    case CONSTRUCTOR:
	return fPtr->constructorName;
    case DESTRUCTOR:
	return fPtr->destructorName;
    default:
	return mPtr->namePtr;
    }
}


```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOBasic.c`, function `Exact selected report or original call-context source window`, lines 1625–1634. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOBasic.c`.

```text
	Tcl_SetObjResult(interp, TclOOObjectName(interp, clsPtr->thisPtr));
	return TCL_OK;
    }
    case SELF_METHOD:
	Tcl_SetObjResult(interp, MethodName(fPtr, contextPtr,
		CurrentlyInvoked(contextPtr).mPtr));
	return TCL_OK;
    case SELF_FILTER:
	if (!CurrentlyInvoked(contextPtr).isFilter) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOBasic.c`, function `TclOONextObjCmd(`, lines 791–824. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOBasic.c`.

```text
TclOONextObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Interp *iPtr = (Interp *) interp;
    CallFrame *framePtr = iPtr->varFramePtr;
    Tcl_ObjectContext context;

    /*
     * Start with sanity checks on the calling context to make sure that we
     * are invoked from a suitable method context. If so, we can safely
     * retrieve the handle to the object call context.
     */

    if (framePtr == NULL || !(framePtr->isProcCallFrame & FRAME_IS_METHOD)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"%s may only be called from inside a method",
		TclGetString(objv[0])));
	Tcl_SetErrorCode(interp, "TCL", "OO", "CONTEXT_REQUIRED", (char *)NULL);
	return TCL_ERROR;
    }
    context = framePtr->clientData;

    /*
     * Invoke the (advanced) method call context in the caller context. Note
     * that this is like [uplevel 1] and not [eval].
     */

    TclNRAddCallback(interp, NextRestoreFrame, framePtr, NULL,NULL,NULL);
    iPtr->varFramePtr = framePtr->callerVarPtr;
    return TclNRObjectContextInvokeNext(interp, context, objc, objv, 1);
}

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOBasic.c`, function `TclOONextObjCmd(`, lines 1317–1350. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOBasic.c`.

```text
TclOONextObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Interp *iPtr = (Interp *) interp;
    CallFrame *framePtr = iPtr->varFramePtr;
    Tcl_ObjectContext context;

    /*
     * Start with sanity checks on the calling context to make sure that we
     * are invoked from a suitable method context. If so, we can safely
     * retrieve the handle to the object call context.
     */

    if (framePtr == NULL || !(framePtr->isProcCallFrame & FRAME_IS_METHOD)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"%s may only be called from inside a method",
		TclGetString(objv[0])));
	OO_ERROR(interp, CONTEXT_REQUIRED);
	return TCL_ERROR;
    }
    context = (Tcl_ObjectContext) framePtr->clientData;

    /*
     * Invoke the (advanced) method call context in the caller context. Note
     * that this is like [uplevel 1] and not [eval].
     */

    TclNRAddCallback(interp, NextRestoreFrame, framePtr, NULL,NULL,NULL);
    iPtr->varFramePtr = framePtr->callerVarPtr;
    return TclNRObjectContextInvokeNext(interp, context, objc, objv, 1);
}

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOBasic.c`, function `TclOONextObjCmd(`, lines 1371–1400. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOBasic.c`.

```text
TclOONextObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Interp *iPtr = (Interp *) interp;
    CallFrame *framePtr = iPtr->varFramePtr;
    Tcl_ObjectContext context = (Tcl_ObjectContext)
	    TclOOGetContextFromCurrentStackFrame(interp, TclGetString(objv[0]));

    /*
     * Start with sanity checks on the calling context to make sure that we
     * are invoked from a suitable method context. If so, we can safely
     * retrieve the handle to the object call context.
     */

    if (!context) {
	return TCL_ERROR;
    }

    /*
     * Invoke the (advanced) method call context in the caller context. Note
     * that this is like [uplevel 1] and not [eval].
     */

    TclNRAddCallback(interp, NextRestoreFrame, framePtr, NULL,NULL,NULL);
    iPtr->varFramePtr = framePtr->callerVarPtr;
    return TclNRObjectContextInvokeNext(interp, context, objc, objv, 1);
}

```

## Consumer bindings

No implementation binding is claimed by this observation record.

## Separate experimental purposes

| Original channel | Finite observation |
| --- | --- |
| C86/C90/C91 values | Six exact counted script values per provider; public bytes only |
| C84/C85 availability | Initialized command availability0/0/0; no value script |
| Original513 Jim | ldd closes; real unprepinned zstd prevents version/availability/value probes |
| Independent513b Jim | Three own channels close; version0.84, availability0/0/0; no value script |
| Foundation inspection | Header holds and typed roles are a separate source/API contract |

The [selected Foundation role contract](tcloo-original-selected-method-role-report.md)
retains its inspected ownership facts and four unrun Runtime/VM controls.
Matching printed bytes do not identify a special call role or a physical
header. The [Method lifetime question](tcloo-original-method-cache-and-active-call-lifetime.md)
likewise retains its own record and mutable-clientData observations.
The original request IDs remain unchanged in both raw captures; their exact
mapping to this ordinary-CLI question grants no physical Foundation result.

## Evidence

- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
- [runtime/rust/tests/data/native_ordinary_cli_method_role_report_values_original/513/jim](../../../../runtime/rust/tests/data/native_ordinary_cli_method_role_report_values_original/513/jim)
- [rust/tcl-registry/tests/data/native_capi_scalar_publication_original](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original)
- [rust/tcl-registry/tests/data/native_oo_operand_stages_original](../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20)
- [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs)
- [rust/tcl-registry/tests/data/native_deep_import_chain_original](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original)
- [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim)
- [rust/tcl-registry/tests/data/native_child_alias_inventory298](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-test-support/tests/data/native_resolution_reference_builds](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds)
- [runtime/rust/tests/data/native_ordinary_cli_method_role_report_values_original/513b](../../../../runtime/rust/tests/data/native_ordinary_cli_method_role_report_values_original/513b)
