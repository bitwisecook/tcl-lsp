# naming.tcloo.original-selected-method-role-report

Kind: `implementation-contract`

## Problem statement

Constructor and destructor Method records can be unnamed while self method reports a Foundation-owned literal. Inferring that role from printed text, or using a Method lifetime observation to authenticate a report header, conflates independent owners.

## Question

Does the selected actual call chain retain its issued Named, Constructor or Destructor report role through next, and does self method use the genuine separately owned Foundation report header or Value for a special role while refusing missing or unsupported owners?

## Conclusion

The inspected C86/C90/C91 source owns constructor/destructor literal objects in Foundation initialization, retains them with +1 holds and releases them at teardown. Selected self method reports the constructor/destructor Foundation object from actual call-context flags, or the current named Method name; next retains the actual context. The shared Rust role descriptor selects through original TclOO availability. Actual Runtime/VM bootstrap owners retain their own report objects, and real entry producers issue special chain roles that next inherits. Ordinary named sentinel-looking methods remain Named. Missing or unsupported owners refuse. Four genuine software definitions remain unexecuted; no external physical-header, refcount or role-identity measurement is attached.

## Scope

Exact pinned C86/C90/C91 source/API inspection plus current Registry/Runtime/VM owner declarations and four unexecuted software control definitions. Foundation object holds, active call role and Named Method reports are independent of Method cache/record/clientData lifetime. Runtime fixture refcounts and VM Value identities retain their separate software meanings. The ordinary CLI printed-value experiment is a separate question and supplies no header/refcount/role identity. No matching own Runtime/VM producer, inventory, assertion outcome, all-method payload lifetime, frame/compiler/Native entry, source edit or measured performance result is established.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`. Channel: Exact pinned source inspection and unexecuted software owner definitions.

Physical Foundation header/refcount/role identity is unmeasured. The separately scoped ordinary CLI value experiment does not execute this physical contract question.

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

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOBasic.c`, function `Exact selected Foundation report or original call-context source window`, lines 1030–1046. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOBasic.c`.

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

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `Exact selected Foundation report or original call-context source window`, lines 899–910. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

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

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `Exact selected Foundation report or original call-context source window`, lines 978–989. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

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

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `Exact selected Foundation report or original call-context source window`, lines 305–318. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

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

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOBasic.c`, function `Exact selected Foundation report or original call-context source window`, lines 1555–1570. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOBasic.c`.

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

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOBasic.c`, function `Exact selected Foundation report or original call-context source window`, lines 1510–1527. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOBasic.c`.

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

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOBasic.c`, function `Exact selected Foundation report or original call-context source window`, lines 1625–1634. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOBasic.c`.

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

- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `NativeTclooMethodReportRole`: Named, Constructor and Destructor roles are issued by the actual selected call producer; report text cannot authenticate a role.
- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `NativeTclooMethodReportProtocol`: Select through actual original TclOO declaration availability; literal Foundation report names do not authenticate physical ownership.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `FoundationMethodReports`: Own actual bootstrap-selected constructor/destructor report headers or Values independently of unnamed Method records; retain genuine missing-owner refusal.
- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `FoundationMethodReports`: Own actual bootstrap-selected constructor/destructor report headers or Values independently of unnamed Method records; retain genuine missing-owner refusal.
- [runtime/rust/src/cmd_oo/native_method_cache.rs](../../../../runtime/rust/src/cmd_oo/native_method_cache.rs), `ReachedMethodChain`: Retain the role issued by actual constructor/destructor entry and inherited next; literal names and unrelated method caches supply no role.
- [rust/tcl-vm/src/cmd_oo/native_method_cache.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_cache.rs), `ReachedMethodChain`: Retain the role issued by actual constructor/destructor entry and inherited next; literal names and unrelated method caches supply no role.
- [runtime/rust/src/cmd_oo/native_method_report_tests.rs](../../../../runtime/rust/src/cmd_oo/native_method_report_tests.rs), `cmd_oo::native_method_report_tests::original_foundation_headers_and_inherited_next_keep_genuine_report_roles` (linked): Genuine selected Runtime C86/C90/C91 software ingresses retain bootstrap-owned constructor/destructor headers independently of Method records. Actual constructor/destructor chains and inherited next report that header; sentinel-looking named methods remain Named. Actual Runtime fixture +1 Foundation and reached result holds are software count assertions only, without external physical-count measurement.
- [runtime/rust/src/cmd_oo/native_method_report_tests.rs](../../../../runtime/rust/src/cmd_oo/native_method_report_tests.rs), `cmd_oo::native_method_report_tests::original_foundation_report_refuses_missing_headers_and_unsupported_selection` (linked): The Runtime software definition withholds unavailable C84/C85/Jim report owners and removes a genuinely selected Foundation header. Missing owners transport the exact Host cause while retaining prior binary result/error stamp; neither spelling nor a fabricated Guest value repairs the owner.
- [rust/tcl-vm/src/cmd_oo/native_method_report_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_report_tests.rs), `cmd_oo::native_method_report_tests::original_foundation_value_headers_follow_constructor_destructor_next_roles` (linked): The VM software definition retains its own constructor/destructor Value identities through genuine selected chains and inherited next. Named sentinel-looking methods remain ordinary named reports; VM Value identity supplies no C refcount or header equivalence.
- [rust/tcl-vm/src/cmd_oo/native_method_report_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_report_tests.rs), `cmd_oo::native_method_report_tests::original_foundation_value_report_refuses_missing_and_unsupported_owners` (linked): The VM software definition withholds unavailable report selection and a genuinely missing Foundation Value owner, preserving the exact typed Host refusal and unavailable owner. It supplies no physical C header or whole-program execution result.

## Evidence

- [rust/tcl-registry](../../../../rust/tcl-registry)
- [runtime/rust](../../../../runtime/rust)
- [runtime/rust/src](../../../../runtime/rust/src)
- [rust/tcl-vm](../../../../rust/tcl-vm)
- [rust/tcl-vm/src](../../../../rust/tcl-vm/src)
- [docs/design](../..)
- [runtime/rust/tests](../../../../runtime/rust/tests)
- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [runtime/rust/tests/data/native_method_cache_active_call_lifetime_original](../../../../runtime/rust/tests/data/native_method_cache_active_call_lifetime_original)
