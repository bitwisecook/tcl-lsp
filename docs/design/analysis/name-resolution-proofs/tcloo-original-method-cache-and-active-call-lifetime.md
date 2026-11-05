# naming.tcloo.original-method-cache-and-active-call-lifetime

Kind: `native-observation`

## Problem statement

Idle method-name cache ownership, entered callchain Method-record lifetime and replacement clientData lifetime are separate original TclOO purposes. A cached selector or active record cannot establish a blanket payload-retention rule.

## Question

Under initialized original C TclOO public MethodType APIs, do idle cached selectors or duplicates retain deleted methods, which records survive an entered call and next, and when does same-record public replacement retire old clientData?

## Conclusion

C8.6/9.0/9.1 finite public cases distinguish weak idle method caches from all-record entered chain retention. Active deletion preserves original Method records through completion/next, while same-record replacement retires old clientData immediately even inside that active call. Declaring-class/interpreter teardown retires declarations despite held cache headers. C8.4/8.5/Jim lanes observe selected core SDK availability only. The interrupted506 outer attempt and complete independent506b capture remain separate; no address-generation, every method-kind payload, Rust assertion or BIG-IP equivalence is established.

## Scope

Original506 outer launcher closes SIGKILL with four completed provider lanes, preserved independently without all-six closure or established cause. Independent506b uses the byte-identical request/probe with a separately retained streaming launcher and all six own compile/dependency/version/execute closures. Supported C86/C90/C91 each run eight fresh cases and151 raw ROWs; older C/Jim are initialized availability-only. Public EvalObjv-return rows and explicit RESULT presentation are distinct; error-context conversion is not an internal worker stop receipt. The probe retains MethodData instrumentation through cleanup and caller token holds. Custom MethodType record retention supplies no blanket Body/Forward/Property per-call payload rule, namespace/frame/Native issuer or method-address generation.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original held method selector/duplicate and custom clientData token headers; separate raw rows, actual deletion callbacks, public EvalObjv and later result String presentation. Version: tcl8.4 8.4.20.

Initialized original 8.4.20 core/public SDK has no selected custom C TclOO MethodType worker in this fixed request. Only actual availability/version is observed; this does not establish optional-extension absence or Jim class simulation.

### tcl8.5

Status: `observed`. Channel: Original held method selector/duplicate and custom clientData token headers; separate raw rows, actual deletion callbacks, public EvalObjv and later result String presentation. Version: tcl8.5 8.5.19.

Initialized original 8.5.19 core/public SDK has no selected custom C TclOO MethodType worker in this fixed request. Only actual availability/version is observed; this does not establish optional-extension absence or Jim class simulation.

### tcl8.6

Status: `observed`. Channel: Original held method selector/duplicate and custom clientData token headers; separate raw rows, actual deletion callbacks, public EvalObjv and later result String presentation. Version: tcl8.6 8.6.18.

Eight independent original public MethodType v1/int cases show that held idle selector and duplicate caches do not retain a deleted Method record. Active self-deletion delays its deleteProc until entered completion; deleting an inherited record before next still permits that original entered record to run and releases it after the chain. Idle same-name replacement deletes old clientData immediately and dispatches new data. Same-record replacement inside an active call also deletes old clientData immediately, then nested invocation uses new data: record lifetime is distinct from mutable payload lifetime. Class/interpreter teardown deletes the declaration despite surviving caches, without repeat deletion on cache release. Original custom token String getters remain unreached in raw rows; explicit result presentation is separate. Diagnostic method addresses supply no generation or application issuer.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original held method selector/duplicate and custom clientData token headers; separate raw rows, actual deletion callbacks, public EvalObjv and later result String presentation. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Eight independent original public MethodType2/Tcl_Size cases show that held idle selector and duplicate caches do not retain a deleted Method record. Active self-deletion delays its deleteProc until entered completion; deleting an inherited record before next still permits that original entered record to run and releases it after the chain. Idle same-name replacement deletes old clientData immediately and dispatches new data. Same-record replacement inside an active call also deletes old clientData immediately, then nested invocation uses new data: record lifetime is distinct from mutable payload lifetime. Class/interpreter teardown deletes the declaration despite surviving caches, without repeat deletion on cache release. Original custom token String getters remain unreached in raw rows; explicit result presentation is separate. Diagnostic method addresses supply no generation or application issuer.

### jim

Status: `observed`. Channel: Original held method selector/duplicate and custom clientData token headers; separate raw rows, actual deletion callbacks, public EvalObjv and later result String presentation. Version: jim 0.84.

Initialized original jim core/public SDK has no selected custom C TclOO MethodType worker in this fixed request. Only actual availability/version is observed; this does not establish optional-extension absence or Jim class simulation.

### bigip

Status: `not-tested`.

No original BIG-IP process for this finite public worker question.

## Source inspection

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 107–202. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    ckfree(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	ckfree(callPtr->chain);
    }
    ckfree(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *
 *	Saves a reference to a method call context in a Tcl_Obj's internal
 *	representation.
 *
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    callPtr->refCount++;
    TclGetString(objPtr);
    TclFreeIntRep(objPtr);
    objPtr->typePtr = &methodNameType;
    objPtr->internalRep.twoPtrValue.ptr1 = callPtr;
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}


```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 138–233. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    ckfree(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	ckfree(callPtr->chain);
    }
    ckfree(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *
 *	Saves a reference to a method call context in a Tcl_Obj's internal
 *	representation.
 *
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    callPtr->refCount++;
    TclGetString(objPtr);
    TclFreeIntRep(objPtr);
    objPtr->typePtr = &methodNameType;
    objPtr->internalRep.twoPtrValue.ptr1 = callPtr;
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    CallChain *callPtr = srcPtr->internalRep.twoPtrValue.ptr1;

    dstPtr->typePtr = &methodNameType;
    dstPtr->internalRep.twoPtrValue.ptr1 = callPtr;
    callPtr->refCount++;
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    CallChain *callPtr = objPtr->internalRep.twoPtrValue.ptr1;

    TclOODeleteChain(callPtr);
    objPtr->typePtr = NULL;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 156–251. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	ckfree(callPtr->chain);
    }
    ckfree(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *
 *	Saves a reference to a method call context in a Tcl_Obj's internal
 *	representation.
 *
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    callPtr->refCount++;
    TclGetString(objPtr);
    TclFreeIntRep(objPtr);
    objPtr->typePtr = &methodNameType;
    objPtr->internalRep.twoPtrValue.ptr1 = callPtr;
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    CallChain *callPtr = srcPtr->internalRep.twoPtrValue.ptr1;

    dstPtr->typePtr = &methodNameType;
    dstPtr->internalRep.twoPtrValue.ptr1 = callPtr;
    callPtr->refCount++;
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    CallChain *callPtr = objPtr->internalRep.twoPtrValue.ptr1;

    TclOODeleteChain(callPtr);
    objPtr->typePtr = NULL;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,	/* The method call context. */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 228–325. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
    Tcl_Obj *objPtr)
{
    CallChain *callPtr = objPtr->internalRep.twoPtrValue.ptr1;

    TclOODeleteChain(callPtr);
    objPtr->typePtr = NULL;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,	/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *)clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	int i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL,NULL,NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags,contextPtr,NULL,NULL,NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
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
SetFilterFlags(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    CallContext *contextPtr = (CallContext *)data[0];

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1073–1171. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
		callPtr->refCount++;
		goto returnContext;
	    }
	    Tcl_SetHashValue(hPtr, NULL);
	    TclOODeleteChain(callPtr);
	}

	doFilters = 1;
    }

    callPtr = (CallChain *)ckalloc(sizeof(CallChain));
    InitCallChain(callPtr, oPtr, flags);

    cb.callChainPtr = callPtr;
    cb.filterLength = 0;
    cb.oPtr = oPtr;

    /*
     * If we're working with a forced use of unknown, do that now.
     */

    if (flags & FORCE_UNKNOWN) {
	AddSimpleChainToCallContext(oPtr, oPtr->fPtr->unknownMethodNameObj,
		&cb, NULL, BUILDING_MIXINS, NULL);
	AddSimpleChainToCallContext(oPtr, oPtr->fPtr->unknownMethodNameObj,
		&cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = -1;
	if (callPtr->numChain == 0) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	goto returnContext;
    }

    /*
     * Add all defined filters (if any, and if we're going to be processing
     * them; they're not processed for constructors, destructors or when we're
     * in the middle of processing a filter).
     */

    if (doFilters) {
	Tcl_Obj *filterObj;
	Class *mixinPtr;

	doFilters = 1;
	Tcl_InitObjHashTable(&doneFilters);
	FOREACH(mixinPtr, oPtr->mixins) {
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    TRAVERSED_MIXIN|BUILDING_MIXINS|OBJECT_MIXIN);
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    OBJECT_MIXIN);
	}
	FOREACH(filterObj, oPtr->filters) {
	    AddSimpleChainToCallContext(oPtr, filterObj, &cb, &doneFilters,
		    BUILDING_MIXINS, NULL);
	    AddSimpleChainToCallContext(oPtr, filterObj, &cb, &doneFilters, 0,
		    NULL);
	}
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		BUILDING_MIXINS);
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		0);
	Tcl_DeleteHashTable(&doneFilters);
    }
    count = cb.filterLength = callPtr->numChain;

    /*
     * Add the actual method implementations. We have to do this twice to
     * handle class mixins right.
     */

    AddSimpleChainToCallContext(oPtr, methodNameObj, &cb, NULL,
	    flags|BUILDING_MIXINS, NULL);
    AddSimpleChainToCallContext(oPtr, methodNameObj, &cb, NULL, flags, NULL);

    /*
     * Check to see if the method has no implementation. If so, we probably
     * need to add in a call to the unknown method. Otherwise, set up the
     * cacheing of the method implementation (if relevant).
     */

    if (count == callPtr->numChain) {
	/*
	 * Method does not actually exist. If we're dealing with constructors
	 * or destructors, this isn't a problem.
	 */

	if (flags & SPECIAL) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	AddSimpleChainToCallContext(oPtr, oPtr->fPtr->unknownMethodNameObj,
		&cb, NULL, BUILDING_MIXINS, NULL);
	AddSimpleChainToCallContext(oPtr, oPtr->fPtr->unknownMethodNameObj,
		&cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = -1;
	if (count == callPtr->numChain) {

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1098–1196. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
		&cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = -1;
	if (callPtr->numChain == 0) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	goto returnContext;
    }

    /*
     * Add all defined filters (if any, and if we're going to be processing
     * them; they're not processed for constructors, destructors or when we're
     * in the middle of processing a filter).
     */

    if (doFilters) {
	Tcl_Obj *filterObj;
	Class *mixinPtr;

	doFilters = 1;
	Tcl_InitObjHashTable(&doneFilters);
	FOREACH(mixinPtr, oPtr->mixins) {
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    TRAVERSED_MIXIN|BUILDING_MIXINS|OBJECT_MIXIN);
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    OBJECT_MIXIN);
	}
	FOREACH(filterObj, oPtr->filters) {
	    AddSimpleChainToCallContext(oPtr, filterObj, &cb, &doneFilters,
		    BUILDING_MIXINS, NULL);
	    AddSimpleChainToCallContext(oPtr, filterObj, &cb, &doneFilters, 0,
		    NULL);
	}
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		BUILDING_MIXINS);
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		0);
	Tcl_DeleteHashTable(&doneFilters);
    }
    count = cb.filterLength = callPtr->numChain;

    /*
     * Add the actual method implementations. We have to do this twice to
     * handle class mixins right.
     */

    AddSimpleChainToCallContext(oPtr, methodNameObj, &cb, NULL,
	    flags|BUILDING_MIXINS, NULL);
    AddSimpleChainToCallContext(oPtr, methodNameObj, &cb, NULL, flags, NULL);

    /*
     * Check to see if the method has no implementation. If so, we probably
     * need to add in a call to the unknown method. Otherwise, set up the
     * cacheing of the method implementation (if relevant).
     */

    if (count == callPtr->numChain) {
	/*
	 * Method does not actually exist. If we're dealing with constructors
	 * or destructors, this isn't a problem.
	 */

	if (flags & SPECIAL) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	AddSimpleChainToCallContext(oPtr, oPtr->fPtr->unknownMethodNameObj,
		&cb, NULL, BUILDING_MIXINS, NULL);
	AddSimpleChainToCallContext(oPtr, oPtr->fPtr->unknownMethodNameObj,
		&cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = -1;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else if (doFilters) {
	if (hPtr == NULL) {
	    if (oPtr->flags & USE_CLASS_CACHE) {
		if (oPtr->selfCls->classChainCache == NULL) {
		    oPtr->selfCls->classChainCache =
			    (Tcl_HashTable *)ckalloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->selfCls->classChainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->selfCls->classChainCache,
			(char *) methodNameObj, &i);
	    } else {
		if (oPtr->chainCache == NULL) {
		    oPtr->chainCache = (Tcl_HashTable *)ckalloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->chainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->chainCache,
			(char *) methodNameObj, &i);
	    }
	}
	callPtr->refCount++;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1158–1255. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
	 * or destructors, this isn't a problem.
	 */

	if (flags & SPECIAL) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	AddSimpleChainToCallContext(oPtr, oPtr->fPtr->unknownMethodNameObj,
		&cb, NULL, BUILDING_MIXINS, NULL);
	AddSimpleChainToCallContext(oPtr, oPtr->fPtr->unknownMethodNameObj,
		&cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = -1;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else if (doFilters) {
	if (hPtr == NULL) {
	    if (oPtr->flags & USE_CLASS_CACHE) {
		if (oPtr->selfCls->classChainCache == NULL) {
		    oPtr->selfCls->classChainCache =
			    (Tcl_HashTable *)ckalloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->selfCls->classChainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->selfCls->classChainCache,
			(char *) methodNameObj, &i);
	    } else {
		if (oPtr->chainCache == NULL) {
		    oPtr->chainCache = (Tcl_HashTable *)ckalloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->chainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->chainCache,
			(char *) methodNameObj, &i);
	    }
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    struct ChainBuilder cb;
    int i, count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1168–1265. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
		&cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = -1;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else if (doFilters) {
	if (hPtr == NULL) {
	    if (oPtr->flags & USE_CLASS_CACHE) {
		if (oPtr->selfCls->classChainCache == NULL) {
		    oPtr->selfCls->classChainCache =
			    (Tcl_HashTable *)ckalloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->selfCls->classChainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->selfCls->classChainCache,
			(char *) methodNameObj, &i);
	    } else {
		if (oPtr->chainCache == NULL) {
		    oPtr->chainCache = (Tcl_HashTable *)ckalloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->chainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->chainCache,
			(char *) methodNameObj, &i);
	    }
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    struct ChainBuilder cb;
    int i, count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply
     * can't get stereotypes. [Bug 7842f33a5c]
     */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1197–1294. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    struct ChainBuilder cb;
    int i, count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply
     * can't get stereotypes. [Bug 7842f33a5c]
     */

    if (clsPtr == NULL) {
	return NULL;
    }

    /*
     * Synthesize a temporary stereotypical object so that we can use existing
     * machinery to produce the stereotypical call chain.
     */

    memset(&obj, 0, sizeof(Object));
    obj.fPtr = fPtr;
    obj.selfCls = clsPtr;
    obj.refCount = 1;
    obj.flags = USE_CLASS_CACHE;

    /*
     * Check if we can get the chain out of the Tcl_Obj method name or out of
     * the cache. This is made a bit more complex by the fact that there are
     * multiple different layers of cache (in the Tcl_Obj, in the object, and
     * in the class).
     */

    if (clsPtr->classChainCache != NULL) {
	hPtr = Tcl_FindHashEntry(clsPtr->classChainCache,
		(char *) methodNameObj);
	if (hPtr != NULL && Tcl_GetHashValue(hPtr) != NULL) {
	    const int reuseMask =
		    ((flags & PUBLIC_METHOD) ? ~0 : ~PUBLIC_METHOD);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1203–1300. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    struct ChainBuilder cb;
    int i, count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply
     * can't get stereotypes. [Bug 7842f33a5c]
     */

    if (clsPtr == NULL) {
	return NULL;
    }

    /*
     * Synthesize a temporary stereotypical object so that we can use existing
     * machinery to produce the stereotypical call chain.
     */

    memset(&obj, 0, sizeof(Object));
    obj.fPtr = fPtr;
    obj.selfCls = clsPtr;
    obj.refCount = 1;
    obj.flags = USE_CLASS_CACHE;

    /*
     * Check if we can get the chain out of the Tcl_Obj method name or out of
     * the cache. This is made a bit more complex by the fact that there are
     * multiple different layers of cache (in the Tcl_Obj, in the object, and
     * in the class).
     */

    if (clsPtr->classChainCache != NULL) {
	hPtr = Tcl_FindHashEntry(clsPtr->classChainCache,
		(char *) methodNameObj);
	if (hPtr != NULL && Tcl_GetHashValue(hPtr) != NULL) {
	    const int reuseMask =
		    ((flags & PUBLIC_METHOD) ? ~0 : ~PUBLIC_METHOD);

	    callPtr = (CallChain *)Tcl_GetHashValue(hPtr);
	    if (IsStillValid(callPtr, &obj, flags, reuseMask)) {
		callPtr->refCount++;
		return callPtr;
	    }
	    Tcl_SetHashValue(hPtr, NULL);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1297–1394. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
		callPtr->refCount++;
		return callPtr;
	    }
	    Tcl_SetHashValue(hPtr, NULL);
	    TclOODeleteChain(callPtr);
	}
    } else {
	hPtr = NULL;
    }

    callPtr = (CallChain *)ckalloc(sizeof(CallChain));
    memset(callPtr, 0, sizeof(CallChain));
    callPtr->flags = flags & (PUBLIC_METHOD|PRIVATE_METHOD|FILTER_HANDLING);
    callPtr->epoch = fPtr->epoch;
    callPtr->objectCreationEpoch = fPtr->tsdPtr->nsCount;
    callPtr->objectEpoch = clsPtr->thisPtr->epoch;
    callPtr->refCount = 1;
    callPtr->chain = callPtr->staticChain;

    cb.callChainPtr = callPtr;
    cb.filterLength = 0;
    cb.oPtr = &obj;

    /*
     * Add all defined filters (if any, and if we're going to be processing
     * them; they're not processed for constructors, destructors or when we're
     * in the middle of processing a filter).
     */

    Tcl_InitObjHashTable(&doneFilters);
    AddClassFiltersToCallContext(&obj, clsPtr, &cb, &doneFilters,
	    BUILDING_MIXINS);
    AddClassFiltersToCallContext(&obj, clsPtr, &cb, &doneFilters, 0);
    Tcl_DeleteHashTable(&doneFilters);
    count = cb.filterLength = callPtr->numChain;

    /*
     * Add the actual method implementations.
     */

    AddSimpleChainToCallContext(&obj, methodNameObj, &cb, NULL,
	    flags|BUILDING_MIXINS, NULL);
    AddSimpleChainToCallContext(&obj, methodNameObj, &cb, NULL, flags, NULL);

    /*
     * Check to see if the method has no implementation. If so, we probably
     * need to add in a call to the unknown method. Otherwise, set up the
     * caching of the method implementation (if relevant).
     */

    if (count == callPtr->numChain) {
	AddSimpleChainToCallContext(&obj, fPtr->unknownMethodNameObj, &cb,
		NULL, BUILDING_MIXINS, NULL);
	AddSimpleChainToCallContext(&obj, fPtr->unknownMethodNameObj, &cb,
		NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = -1;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else {
	if (hPtr == NULL) {
	    if (clsPtr->classChainCache == NULL) {
		clsPtr->classChainCache = (Tcl_HashTable *)ckalloc(sizeof(Tcl_HashTable));
		Tcl_InitObjHashTable(clsPtr->classChainCache);
	    }
	    hPtr = Tcl_CreateHashEntry(clsPtr->classChainCache,
		    (char *) methodNameObj, &i);
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(methodNameObj, callPtr);
    }
    return callPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddClassFiltersToCallContext --
 *
 *	Logic to make extracting all the filters from the class context much
 *	easier.
 *
 * ----------------------------------------------------------------------
 */

static void
AddClassFiltersToCallContext(
    Object *const oPtr,		/* Object that the filters operate on. */
    Class *clsPtr,		/* Class to get the filters from. */
    struct ChainBuilder *const cbPtr,
				/* Context to fill with call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. Keys are objects, values are
				 * ignored. */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1351–1448. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
		NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = -1;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else {
	if (hPtr == NULL) {
	    if (clsPtr->classChainCache == NULL) {
		clsPtr->classChainCache = (Tcl_HashTable *)ckalloc(sizeof(Tcl_HashTable));
		Tcl_InitObjHashTable(clsPtr->classChainCache);
	    }
	    hPtr = Tcl_CreateHashEntry(clsPtr->classChainCache,
		    (char *) methodNameObj, &i);
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(methodNameObj, callPtr);
    }
    return callPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddClassFiltersToCallContext --
 *
 *	Logic to make extracting all the filters from the class context much
 *	easier.
 *
 * ----------------------------------------------------------------------
 */

static void
AddClassFiltersToCallContext(
    Object *const oPtr,		/* Object that the filters operate on. */
    Class *clsPtr,		/* Class to get the filters from. */
    struct ChainBuilder *const cbPtr,
				/* Context to fill with call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. Keys are objects, values are
				 * ignored. */
    int flags)			/* Whether we've gone along a mixin link
				 * yet. */
{
    int i;
    int clearedFlags =
	    flags & ~(TRAVERSED_MIXIN|OBJECT_MIXIN|BUILDING_MIXINS);
    Class *superPtr, *mixinPtr;
    Tcl_Obj *filterObj;

  tailRecurse:
    if (clsPtr == NULL) {
	return;
    }

    /*
     * Add all the filters defined by classes mixed into the main class
     * hierarchy.
     */

    FOREACH(mixinPtr, clsPtr->mixins) {
	AddClassFiltersToCallContext(oPtr, mixinPtr, cbPtr, doneFilters,
		flags|TRAVERSED_MIXIN);
    }

    /*
     * Add all the class filters from the current class. Note that the filters
     * are added starting at the object root, as this allows the object to
     * override how filters work to extend their behaviour.
     */

    if (MIXIN_CONSISTENT(flags)) {
	FOREACH(filterObj, clsPtr->filters) {
	    int isNew;

	    (void) Tcl_CreateHashEntry(doneFilters, (char *) filterObj,
		    &isNew);
	    if (isNew) {
		AddSimpleChainToCallContext(oPtr, filterObj, cbPtr,
			doneFilters, clearedFlags|BUILDING_MIXINS, clsPtr);
		AddSimpleChainToCallContext(oPtr, filterObj, cbPtr,
			doneFilters, clearedFlags, clsPtr);
	    }
	}
    }

    /*
     * Now process the recursive case. Notice the tail-call optimization.
     */

    switch (clsPtr->superclasses.num) {
    case 1:
	clsPtr = clsPtr->superclasses.list[0];
	goto tailRecurse;
    default:

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `StashCallChain(`, lines 76–171. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
static inline int	IsStillValid(CallChain *callPtr, Object *oPtr,
			    int flags, int reuseMask);
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */

static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,
    NULL
};


/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    ckfree(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	ckfree(callPtr->chain);
    }
    ckfree(callPtr);
}


```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `StashCallChain(`, lines 180–276. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    callPtr->refCount++;
    TclGetString(objPtr);
    TclFreeIntRep(objPtr);
    objPtr->typePtr = &methodNameType;
    objPtr->internalRep.twoPtrValue.ptr1 = callPtr;
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    CallChain *callPtr = srcPtr->internalRep.twoPtrValue.ptr1;

    dstPtr->typePtr = &methodNameType;
    dstPtr->internalRep.twoPtrValue.ptr1 = callPtr;
    callPtr->refCount++;
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    CallChain *callPtr = objPtr->internalRep.twoPtrValue.ptr1;

    TclOODeleteChain(callPtr);
    objPtr->typePtr = NULL;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,	/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *)clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	int i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `StashCallChain(`, lines 196–292. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    CallChain *callPtr = srcPtr->internalRep.twoPtrValue.ptr1;

    dstPtr->typePtr = &methodNameType;
    dstPtr->internalRep.twoPtrValue.ptr1 = callPtr;
    callPtr->refCount++;
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    CallChain *callPtr = objPtr->internalRep.twoPtrValue.ptr1;

    TclOODeleteChain(callPtr);
    objPtr->typePtr = NULL;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,	/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *)clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	int i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `StashCallChain(`, lines 1194–1291. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
	    }
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    struct ChainBuilder cb;
    int i, count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply
     * can't get stereotypes. [Bug 7842f33a5c]
     */

    if (clsPtr == NULL) {
	return NULL;
    }

    /*
     * Synthesize a temporary stereotypical object so that we can use existing
     * machinery to produce the stereotypical call chain.
     */

    memset(&obj, 0, sizeof(Object));
    obj.fPtr = fPtr;
    obj.selfCls = clsPtr;
    obj.refCount = 1;
    obj.flags = USE_CLASS_CACHE;

    /*
     * Check if we can get the chain out of the Tcl_Obj method name or out of
     * the cache. This is made a bit more complex by the fact that there are
     * multiple different layers of cache (in the Tcl_Obj, in the object, and
     * in the class).
     */

    if (clsPtr->classChainCache != NULL) {
	hPtr = Tcl_FindHashEntry(clsPtr->classChainCache,
		(char *) methodNameObj);
	if (hPtr != NULL && Tcl_GetHashValue(hPtr) != NULL) {

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `StashCallChain(`, lines 1365–1461. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
		    (char *) methodNameObj, &i);
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(methodNameObj, callPtr);
    }
    return callPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddClassFiltersToCallContext --
 *
 *	Logic to make extracting all the filters from the class context much
 *	easier.
 *
 * ----------------------------------------------------------------------
 */

static void
AddClassFiltersToCallContext(
    Object *const oPtr,		/* Object that the filters operate on. */
    Class *clsPtr,		/* Class to get the filters from. */
    struct ChainBuilder *const cbPtr,
				/* Context to fill with call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. Keys are objects, values are
				 * ignored. */
    int flags)			/* Whether we've gone along a mixin link
				 * yet. */
{
    int i;
    int clearedFlags =
	    flags & ~(TRAVERSED_MIXIN|OBJECT_MIXIN|BUILDING_MIXINS);
    Class *superPtr, *mixinPtr;
    Tcl_Obj *filterObj;

  tailRecurse:
    if (clsPtr == NULL) {
	return;
    }

    /*
     * Add all the filters defined by classes mixed into the main class
     * hierarchy.
     */

    FOREACH(mixinPtr, clsPtr->mixins) {
	AddClassFiltersToCallContext(oPtr, mixinPtr, cbPtr, doneFilters,
		flags|TRAVERSED_MIXIN);
    }

    /*
     * Add all the class filters from the current class. Note that the filters
     * are added starting at the object root, as this allows the object to
     * override how filters work to extend their behaviour.
     */

    if (MIXIN_CONSISTENT(flags)) {
	FOREACH(filterObj, clsPtr->filters) {
	    int isNew;

	    (void) Tcl_CreateHashEntry(doneFilters, (char *) filterObj,
		    &isNew);
	    if (isNew) {
		AddSimpleChainToCallContext(oPtr, filterObj, cbPtr,
			doneFilters, clearedFlags|BUILDING_MIXINS, clsPtr);
		AddSimpleChainToCallContext(oPtr, filterObj, cbPtr,
			doneFilters, clearedFlags, clsPtr);
	    }
	}
    }

    /*
     * Now process the recursive case. Notice the tail-call optimization.
     */

    switch (clsPtr->superclasses.num) {
    case 1:
	clsPtr = clsPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, clsPtr->superclasses) {
	    AddClassFiltersToCallContext(oPtr, superPtr, cbPtr, doneFilters,
		    flags);
	}
    case 0:
	return;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * AddSimpleClassChainToCallContext --

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `FreeMethodNameRep(`, lines 71–166. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
			    Class *const filterDecl);
static int		CmpStr(const void *ptr1, const void *ptr2);
static void		DupMethodNameRep(Tcl_Obj *srcPtr, Tcl_Obj *dstPtr);
static Tcl_NRPostProc	FinalizeMethodRefs;
static void		FreeMethodNameRep(Tcl_Obj *objPtr);
static inline int	IsStillValid(CallChain *callPtr, Object *oPtr,
			    int flags, int reuseMask);
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */

static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,
    NULL
};


/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    ckfree(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `FreeMethodNameRep(`, lines 223–320. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
    callPtr->refCount++;
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    CallChain *callPtr = objPtr->internalRep.twoPtrValue.ptr1;

    TclOODeleteChain(callPtr);
    objPtr->typePtr = NULL;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,	/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *)clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	int i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL,NULL,NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags,contextPtr,NULL,NULL,NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
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
SetFilterFlags(
    void *data[],

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `FreeMethodNameRep(`, lines 1039–1137. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
	    if (IsStillValid(callPtr, oPtr, flags, reuseMask)) {
		callPtr->refCount++;
		goto returnContext;
	    }
	    FreeMethodNameRep(cacheInThisObj);
	}

	/*
	 * Note that it's possible to end up with a NULL oPtr->selfCls here if
	 * there is a call into stereotypical object after it has finished
	 * running its destructor phase. It's quite a tangle, but at that
	 * point, we simply can't get stereotypes from the cache.
	 * [Bug 7842f33a5c]
	 */

	if (oPtr->flags & USE_CLASS_CACHE && oPtr->selfCls) {
	    if (oPtr->selfCls->classChainCache) {
		hPtr = Tcl_FindHashEntry(oPtr->selfCls->classChainCache,
			(char *) methodNameObj);
	    } else {
		hPtr = NULL;
	    }
	} else {
	    if (oPtr->chainCache != NULL) {
		hPtr = Tcl_FindHashEntry(oPtr->chainCache,
			(char *) methodNameObj);
	    } else {
		hPtr = NULL;
	    }
	}

	if (hPtr != NULL && Tcl_GetHashValue(hPtr) != NULL) {
	    callPtr = (CallChain *)Tcl_GetHashValue(hPtr);
	    if (IsStillValid(callPtr, oPtr, flags, reuseMask)) {
		callPtr->refCount++;
		goto returnContext;
	    }
	    Tcl_SetHashValue(hPtr, NULL);
	    TclOODeleteChain(callPtr);
	}

	doFilters = 1;
    }

    callPtr = (CallChain *)ckalloc(sizeof(CallChain));
    InitCallChain(callPtr, oPtr, flags);

    cb.callChainPtr = callPtr;
    cb.filterLength = 0;
    cb.oPtr = oPtr;

    /*
     * If we're working with a forced use of unknown, do that now.
     */

    if (flags & FORCE_UNKNOWN) {
	AddSimpleChainToCallContext(oPtr, oPtr->fPtr->unknownMethodNameObj,
		&cb, NULL, BUILDING_MIXINS, NULL);
	AddSimpleChainToCallContext(oPtr, oPtr->fPtr->unknownMethodNameObj,
		&cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = -1;
	if (callPtr->numChain == 0) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	goto returnContext;
    }

    /*
     * Add all defined filters (if any, and if we're going to be processing
     * them; they're not processed for constructors, destructors or when we're
     * in the middle of processing a filter).
     */

    if (doFilters) {
	Tcl_Obj *filterObj;
	Class *mixinPtr;

	doFilters = 1;
	Tcl_InitObjHashTable(&doneFilters);
	FOREACH(mixinPtr, oPtr->mixins) {
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    TRAVERSED_MIXIN|BUILDING_MIXINS|OBJECT_MIXIN);
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    OBJECT_MIXIN);
	}
	FOREACH(filterObj, oPtr->filters) {
	    AddSimpleChainToCallContext(oPtr, filterObj, &cb, &doneFilters,
		    BUILDING_MIXINS, NULL);
	    AddSimpleChainToCallContext(oPtr, filterObj, &cb, &doneFilters, 0,
		    NULL);
	}
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		BUILDING_MIXINS);
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		0);
	Tcl_DeleteHashTable(&doneFilters);
    }

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `DupMethodNameRep(`, lines 69–164. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
			    struct ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters, int flags,
			    Class *const filterDecl);
static int		CmpStr(const void *ptr1, const void *ptr2);
static void		DupMethodNameRep(Tcl_Obj *srcPtr, Tcl_Obj *dstPtr);
static Tcl_NRPostProc	FinalizeMethodRefs;
static void		FreeMethodNameRep(Tcl_Obj *objPtr);
static inline int	IsStillValid(CallChain *callPtr, Object *oPtr,
			    int flags, int reuseMask);
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */

static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,
    NULL
};


/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    ckfree(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `DupMethodNameRep(`, lines 211–308. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    CallChain *callPtr = srcPtr->internalRep.twoPtrValue.ptr1;

    dstPtr->typePtr = &methodNameType;
    dstPtr->internalRep.twoPtrValue.ptr1 = callPtr;
    callPtr->refCount++;
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    CallChain *callPtr = objPtr->internalRep.twoPtrValue.ptr1;

    TclOODeleteChain(callPtr);
    objPtr->typePtr = NULL;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,	/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *)clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	int i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL,NULL,NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags,contextPtr,NULL,NULL,NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `TclOOInvokeContext(`, lines 246–344. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,	/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *)clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	int i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL,NULL,NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags,contextPtr,NULL,NULL,NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
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
SetFilterFlags(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    CallContext *contextPtr = (CallContext *)data[0];

    contextPtr->oPtr->flags |= FILTER_HANDLING;
    return result;
}

static int
ResetFilterFlags(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    CallContext *contextPtr = (CallContext *)data[0];

    contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    return result;
}

static int
FinalizeMethodRefs(
    void *data[],

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `FinalizeMethodRefs(`, lines 339–436. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
    return result;
}

static int
FinalizeMethodRefs(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    CallContext *contextPtr = (CallContext *)data[0];
    int i;

    for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	TclOODelMethodRef(contextPtr->callPtr->chain[i].mPtr);
    }
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetSortedMethodList, TclOOGetSortedClassMethodList --
 *
 *	Discovers the list of method names supported by an object or class.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOGetSortedMethodList(
    Object *oPtr,		/* The object to get the method names for. */
    int flags,			/* Whether we just want the public method
				 * names. */
    const char ***stringsPtr)	/* Where to write a pointer to the array of
				 * strings to. */
{
    Tcl_HashTable names;	/* Tcl_Obj* method name to "wanted in list"
				 * mapping. */
    Tcl_HashTable examinedClasses;
				/* Used to track what classes have been looked
				 * at. Is set-like in nature and keyed by
				 * pointer to class. */
    FOREACH_HASH_DECLS;
    int i;
    Class *mixinPtr;
    Tcl_Obj *namePtr;
    Method *mPtr;
    int isWantedIn;
    void *isWanted;

    Tcl_InitObjHashTable(&names);
    Tcl_InitHashTable(&examinedClasses, TCL_ONE_WORD_KEYS);

    /*
     * Name the bits used in the names table values.
     */
#define IN_LIST 1
#define NO_IMPLEMENTATION 2

    /*
     * Process method names due to the object.
     */

    if (oPtr->methodsPtr) {
	FOREACH_HASH(namePtr, mPtr, oPtr->methodsPtr) {
	    int isNew;

	    if ((mPtr->flags & PRIVATE_METHOD) && !(flags & PRIVATE_METHOD)) {
		continue;
	    }
	    hPtr = Tcl_CreateHashEntry(&names, (char *) namePtr, &isNew);
	    if (isNew) {
		isWantedIn = ((!(flags & PUBLIC_METHOD)
			|| mPtr->flags & PUBLIC_METHOD) ? IN_LIST : 0);
		isWantedIn |= (mPtr->typePtr == NULL ? NO_IMPLEMENTATION : 0);
		Tcl_SetHashValue(hPtr, INT2PTR(isWantedIn));
	    }
	}
    }

    /*
     * Process method names due to private methods on the object's class.
     */

    if (flags & PRIVATE_METHOD) {
	FOREACH_HASH(namePtr, mPtr, &oPtr->selfCls->classMethods) {
	    if (mPtr->flags & PRIVATE_METHOD) {
		int isNew;

		hPtr = Tcl_CreateHashEntry(&names, (char *) namePtr, &isNew);
		if (isNew) {
		    isWantedIn = IN_LIST;
		    if (mPtr->typePtr == NULL) {
			isWantedIn |= NO_IMPLEMENTATION;
		    }
		    Tcl_SetHashValue(hPtr, INT2PTR(isWantedIn));
		} else if (mPtr->typePtr != NULL) {
		    isWantedIn = PTR2INT(Tcl_GetHashValue(hPtr));

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 54–149. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
			    Tcl_HashTable *const doneFilters, int flags);
static void		AddClassMethodNames(Class *clsPtr, int flags,
			    Tcl_HashTable *const namesPtr,
			    Tcl_HashTable *const examinedClassesPtr);
static inline void	AddMethodToCallChain(Method *const mPtr,
			    struct ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters,
			    Class *const filterDecl, int flags);
static inline void	AddSimpleChainToCallContext(Object *const oPtr,
			    Tcl_Obj *const methodNameObj,
			    struct ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters, int flags,
			    Class *const filterDecl);
static void		AddSimpleClassChainToCallContext(Class *classPtr,
			    Tcl_Obj *const methodNameObj,
			    struct ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters, int flags,
			    Class *const filterDecl);
static int		CmpStr(const void *ptr1, const void *ptr2);
static void		DupMethodNameRep(Tcl_Obj *srcPtr, Tcl_Obj *dstPtr);
static Tcl_NRPostProc	FinalizeMethodRefs;
static void		FreeMethodNameRep(Tcl_Obj *objPtr);
static inline int	IsStillValid(CallChain *callPtr, Object *oPtr,
			    int flags, int reuseMask);
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */

static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,
    NULL
};


/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    ckfree(tablePtr);
}

/*

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 750–847. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
	}
	if (oPtr->methodsPtr) {
	    hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, (char*) methodNameObj);
	    if (hPtr != NULL) {
		AddMethodToCallChain((Method *)Tcl_GetHashValue(hPtr), cbPtr,
			doneFilters, filterDecl, flags);
	    }
	}
    }
    if (!oPtr->selfCls) {
	return;
    }
    AddSimpleClassChainToCallContext(oPtr->selfCls, methodNameObj, cbPtr,
	    doneFilters, flags, filterDecl);
}

/*
 * ----------------------------------------------------------------------
 *
 * AddMethodToCallChain --
 *
 *	Utility method that manages the adding of a particular method
 *	implementation to a call-chain.
 *
 * ----------------------------------------------------------------------
 */

static inline void
AddMethodToCallChain(
    Method *const mPtr,		/* Actual method implementation to add to call
				 * chain (or NULL, a no-op). */
    struct ChainBuilder *const cbPtr,
				/* The call chain to add the method
				 * implementation to. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. If NULL, not processing filters.
				 * Note that this function does not update
				 * this hashtable. */
    Class *const filterDecl,	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
    int flags)			/* Used to check if we're mixin-consistent
				 * only. Mixin-consistent means that either
				 * we're looking to add things from a mixin
				 * and we have passed a mixin, or we're not
				 * looking to add things from a mixin and have
				 * not passed a mixin. */
{
    CallChain *callPtr = cbPtr->callChainPtr;
    int i;

    /*
     * Return if this is just an entry used to record whether this is a public
     * method. If so, there's nothing real to call and so nothing to add to
     * the call chain.
     *
     * This is also where we enforce mixin-consistency.
     */

    if (mPtr == NULL || mPtr->typePtr == NULL || !MIXIN_CONSISTENT(flags)) {
	return;
    }

    /*
     * Enforce real private method handling here. We will skip adding this
     * method IF
     *  1) we are not allowing private methods, AND
     *  2) this is a private method, AND
     *  3) this is a class method, AND
     *  4) this method was not declared by the class of the current object.
     *
     * This does mean that only classes really handle private methods. This
     * should be sufficient for [incr Tcl] support though.
     */

    if (!(callPtr->flags & PRIVATE_METHOD)
	    && (mPtr->flags & PRIVATE_METHOD)
	    && (mPtr->declaringClassPtr != NULL)
	    && (mPtr->declaringClassPtr != cbPtr->oPtr->selfCls)) {
	return;
    }

    /*
     * First test whether the method is already in the call chain. Skip over
     * any leading filters.
     */

    for (i = cbPtr->filterLength ; i < callPtr->numChain ; i++) {
	if (callPtr->chain[i].mPtr == mPtr &&
		callPtr->chain[i].isFilter == (doneFilters != NULL)) {
	    /*
	     * Call chain semantics states that methods come as *late* in the
	     * call chain as possible. This is done by copying down the
	     * following methods. Note that this does not change the number of
	     * method invocations in the call chain; it just rearranges them.
	     */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 774–872. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

static inline void
AddMethodToCallChain(
    Method *const mPtr,		/* Actual method implementation to add to call
				 * chain (or NULL, a no-op). */
    struct ChainBuilder *const cbPtr,
				/* The call chain to add the method
				 * implementation to. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. If NULL, not processing filters.
				 * Note that this function does not update
				 * this hashtable. */
    Class *const filterDecl,	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
    int flags)			/* Used to check if we're mixin-consistent
				 * only. Mixin-consistent means that either
				 * we're looking to add things from a mixin
				 * and we have passed a mixin, or we're not
				 * looking to add things from a mixin and have
				 * not passed a mixin. */
{
    CallChain *callPtr = cbPtr->callChainPtr;
    int i;

    /*
     * Return if this is just an entry used to record whether this is a public
     * method. If so, there's nothing real to call and so nothing to add to
     * the call chain.
     *
     * This is also where we enforce mixin-consistency.
     */

    if (mPtr == NULL || mPtr->typePtr == NULL || !MIXIN_CONSISTENT(flags)) {
	return;
    }

    /*
     * Enforce real private method handling here. We will skip adding this
     * method IF
     *  1) we are not allowing private methods, AND
     *  2) this is a private method, AND
     *  3) this is a class method, AND
     *  4) this method was not declared by the class of the current object.
     *
     * This does mean that only classes really handle private methods. This
     * should be sufficient for [incr Tcl] support though.
     */

    if (!(callPtr->flags & PRIVATE_METHOD)
	    && (mPtr->flags & PRIVATE_METHOD)
	    && (mPtr->declaringClassPtr != NULL)
	    && (mPtr->declaringClassPtr != cbPtr->oPtr->selfCls)) {
	return;
    }

    /*
     * First test whether the method is already in the call chain. Skip over
     * any leading filters.
     */

    for (i = cbPtr->filterLength ; i < callPtr->numChain ; i++) {
	if (callPtr->chain[i].mPtr == mPtr &&
		callPtr->chain[i].isFilter == (doneFilters != NULL)) {
	    /*
	     * Call chain semantics states that methods come as *late* in the
	     * call chain as possible. This is done by copying down the
	     * following methods. Note that this does not change the number of
	     * method invocations in the call chain; it just rearranges them.
	     */

	    Class *declCls = callPtr->chain[i].filterDeclarer;

	    for (; i + 1 < callPtr->numChain ; i++) {
		callPtr->chain[i] = callPtr->chain[i + 1];
	    }
	    callPtr->chain[i].mPtr = mPtr;
	    callPtr->chain[i].isFilter = (doneFilters != NULL);
	    callPtr->chain[i].filterDeclarer = declCls;
	    return;
	}
    }

    /*
     * Need to really add the method. This is made a bit more complex by the
     * fact that we are using some "static" space initially, and only start
     * realloc-ing if the chain gets long.
     */

    if (callPtr->numChain == CALL_CHAIN_STATIC_SIZE) {
	callPtr->chain =
		(struct MInvoke *)ckalloc(sizeof(struct MInvoke) * (callPtr->numChain + 1));
	memcpy(callPtr->chain, callPtr->staticChain,
		sizeof(struct MInvoke) * callPtr->numChain);
    } else if (callPtr->numChain > CALL_CHAIN_STATIC_SIZE) {
	callPtr->chain = (struct MInvoke *)ckrealloc(callPtr->chain,

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 1505–1602. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
		doneFilters, flags|TRAVERSED_MIXIN, filterDecl);
    }

    if (flags & CONSTRUCTOR) {
	AddMethodToCallChain(classPtr->constructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else if (flags & DESTRUCTOR) {
	AddMethodToCallChain(classPtr->destructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else {
	Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&classPtr->classMethods,
		(char *) methodNameObj);

	if (hPtr != NULL) {
	    Method *mPtr = (Method *)Tcl_GetHashValue(hPtr);

	    if (!(flags & KNOWN_STATE)) {
		if (flags & PUBLIC_METHOD) {
		    if (mPtr->flags & PUBLIC_METHOD) {
			flags |= DEFINITE_PUBLIC;
		    } else {
			return;
		    }
		} else {
		    flags |= DEFINITE_PROTECTED;
		}
	    }
	    AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl, flags);
	}
    }

    switch (classPtr->superclasses.num) {
    case 1:
	classPtr = classPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, classPtr->superclasses) {
	    AddSimpleClassChainToCallContext(superPtr, methodNameObj, cbPtr,
		    doneFilters, flags, filterDecl);
	}
	/* FALLTHRU */
    case 0:
	return;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOORenderCallChain --
 *
 *	Create a description of a call chain. Used in [info object call],
 *	[info class call], and [self call].
 *
 * ----------------------------------------------------------------------
 */

Tcl_Obj *
TclOORenderCallChain(
    Tcl_Interp *interp,
    CallChain *callPtr)
{
    Tcl_Obj *filterLiteral, *methodLiteral, *objectLiteral;
    Tcl_Obj *resultObj, *descObjs[4], **objv;
    Foundation *fPtr = TclOOGetFoundation(interp);
    int i;

    /*
     * Allocate the literals (potentially) used in our description.
     */

    filterLiteral = Tcl_NewStringObj("filter", -1);
    Tcl_IncrRefCount(filterLiteral);
    methodLiteral = Tcl_NewStringObj("method", -1);
    Tcl_IncrRefCount(methodLiteral);
    objectLiteral = Tcl_NewStringObj("object", -1);
    Tcl_IncrRefCount(objectLiteral);

    /*
     * Do the actual construction of the descriptions. They consist of a list
     * of triples that describe the details of how a method is understood. For
     * each triple, the first word is the type of invocation ("method" is
     * normal, "unknown" is special because it adds the method name as an
     * extra argument when handled by some method types, and "filter" is
     * special because it's a filter method). The second word is the name of
     * the method in question (which differs for "unknown" and "filter" types)
     * and the third word is the full name of the class that declares the
     * method (or "object" if it is declared on the instance).
     */

    objv = (Tcl_Obj **)TclStackAlloc(interp, callPtr->numChain * sizeof(Tcl_Obj *));
    for (i = 0 ; i < callPtr->numChain ; i++) {
	struct MInvoke *miPtr = &callPtr->chain[i];

	descObjs[0] =
	    miPtr->isFilter ? filterLiteral :
	    callPtr->flags & OO_UNKNOWN_METHOD ? fPtr->unknownMethodNameObj :
		    methodLiteral;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 1508–1605. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
    if (flags & CONSTRUCTOR) {
	AddMethodToCallChain(classPtr->constructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else if (flags & DESTRUCTOR) {
	AddMethodToCallChain(classPtr->destructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else {
	Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&classPtr->classMethods,
		(char *) methodNameObj);

	if (hPtr != NULL) {
	    Method *mPtr = (Method *)Tcl_GetHashValue(hPtr);

	    if (!(flags & KNOWN_STATE)) {
		if (flags & PUBLIC_METHOD) {
		    if (mPtr->flags & PUBLIC_METHOD) {
			flags |= DEFINITE_PUBLIC;
		    } else {
			return;
		    }
		} else {
		    flags |= DEFINITE_PROTECTED;
		}
	    }
	    AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl, flags);
	}
    }

    switch (classPtr->superclasses.num) {
    case 1:
	classPtr = classPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, classPtr->superclasses) {
	    AddSimpleClassChainToCallContext(superPtr, methodNameObj, cbPtr,
		    doneFilters, flags, filterDecl);
	}
	/* FALLTHRU */
    case 0:
	return;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOORenderCallChain --
 *
 *	Create a description of a call chain. Used in [info object call],
 *	[info class call], and [self call].
 *
 * ----------------------------------------------------------------------
 */

Tcl_Obj *
TclOORenderCallChain(
    Tcl_Interp *interp,
    CallChain *callPtr)
{
    Tcl_Obj *filterLiteral, *methodLiteral, *objectLiteral;
    Tcl_Obj *resultObj, *descObjs[4], **objv;
    Foundation *fPtr = TclOOGetFoundation(interp);
    int i;

    /*
     * Allocate the literals (potentially) used in our description.
     */

    filterLiteral = Tcl_NewStringObj("filter", -1);
    Tcl_IncrRefCount(filterLiteral);
    methodLiteral = Tcl_NewStringObj("method", -1);
    Tcl_IncrRefCount(methodLiteral);
    objectLiteral = Tcl_NewStringObj("object", -1);
    Tcl_IncrRefCount(objectLiteral);

    /*
     * Do the actual construction of the descriptions. They consist of a list
     * of triples that describe the details of how a method is understood. For
     * each triple, the first word is the type of invocation ("method" is
     * normal, "unknown" is special because it adds the method name as an
     * extra argument when handled by some method types, and "filter" is
     * special because it's a filter method). The second word is the name of
     * the method in question (which differs for "unknown" and "filter" types)
     * and the third word is the full name of the class that declares the
     * method (or "object" if it is declared on the instance).
     */

    objv = (Tcl_Obj **)TclStackAlloc(interp, callPtr->numChain * sizeof(Tcl_Obj *));
    for (i = 0 ; i < callPtr->numChain ; i++) {
	struct MInvoke *miPtr = &callPtr->chain[i];

	descObjs[0] =
	    miPtr->isFilter ? filterLiteral :
	    callPtr->flags & OO_UNKNOWN_METHOD ? fPtr->unknownMethodNameObj :
		    methodLiteral;
	descObjs[1] =
	    callPtr->flags & CONSTRUCTOR ? fPtr->constructorName :
	    callPtr->flags & DESTRUCTOR ? fPtr->destructorName :

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 1528–1625. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
		} else {
		    flags |= DEFINITE_PROTECTED;
		}
	    }
	    AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl, flags);
	}
    }

    switch (classPtr->superclasses.num) {
    case 1:
	classPtr = classPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, classPtr->superclasses) {
	    AddSimpleClassChainToCallContext(superPtr, methodNameObj, cbPtr,
		    doneFilters, flags, filterDecl);
	}
	/* FALLTHRU */
    case 0:
	return;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOORenderCallChain --
 *
 *	Create a description of a call chain. Used in [info object call],
 *	[info class call], and [self call].
 *
 * ----------------------------------------------------------------------
 */

Tcl_Obj *
TclOORenderCallChain(
    Tcl_Interp *interp,
    CallChain *callPtr)
{
    Tcl_Obj *filterLiteral, *methodLiteral, *objectLiteral;
    Tcl_Obj *resultObj, *descObjs[4], **objv;
    Foundation *fPtr = TclOOGetFoundation(interp);
    int i;

    /*
     * Allocate the literals (potentially) used in our description.
     */

    filterLiteral = Tcl_NewStringObj("filter", -1);
    Tcl_IncrRefCount(filterLiteral);
    methodLiteral = Tcl_NewStringObj("method", -1);
    Tcl_IncrRefCount(methodLiteral);
    objectLiteral = Tcl_NewStringObj("object", -1);
    Tcl_IncrRefCount(objectLiteral);

    /*
     * Do the actual construction of the descriptions. They consist of a list
     * of triples that describe the details of how a method is understood. For
     * each triple, the first word is the type of invocation ("method" is
     * normal, "unknown" is special because it adds the method name as an
     * extra argument when handled by some method types, and "filter" is
     * special because it's a filter method). The second word is the name of
     * the method in question (which differs for "unknown" and "filter" types)
     * and the third word is the full name of the class that declares the
     * method (or "object" if it is declared on the instance).
     */

    objv = (Tcl_Obj **)TclStackAlloc(interp, callPtr->numChain * sizeof(Tcl_Obj *));
    for (i = 0 ; i < callPtr->numChain ; i++) {
	struct MInvoke *miPtr = &callPtr->chain[i];

	descObjs[0] =
	    miPtr->isFilter ? filterLiteral :
	    callPtr->flags & OO_UNKNOWN_METHOD ? fPtr->unknownMethodNameObj :
		    methodLiteral;
	descObjs[1] =
	    callPtr->flags & CONSTRUCTOR ? fPtr->constructorName :
	    callPtr->flags & DESTRUCTOR ? fPtr->destructorName :
		    miPtr->mPtr->namePtr;
	descObjs[2] = miPtr->mPtr->declaringClassPtr
		? Tcl_GetObjectName(interp,
			(Tcl_Object) miPtr->mPtr->declaringClassPtr->thisPtr)
		: objectLiteral;
	descObjs[3] = Tcl_NewStringObj(miPtr->mPtr->typePtr->name, -1);

	objv[i] = Tcl_NewListObj(4, descObjs);
    }

    /*
     * Drop the local references to the literals; if they're actually used,
     * they'll live on the description itself.
     */

    Tcl_DecrRefCount(filterLiteral);
    Tcl_DecrRefCount(methodLiteral);
    Tcl_DecrRefCount(objectLiteral);

    /*

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOCall.c`, function `methodNameType =`, lines 82–176. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--8.6.18--tclOOCall.c`.

```text
/*
 * Object type used to manage type caches attached to method names.
 */

static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,
    NULL
};


/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    ckfree(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	ckfree(callPtr->chain);
    }
    ckfree(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 201–297. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
 * ----------------------------------------------------------------------
 */

Tcl_Method
Tcl_NewMethod(
    Tcl_Interp *interp,		/* The interpreter containing the class. */
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    Class *clsPtr = (Class *) cls;
    Method *mPtr;
    Tcl_HashEntry *hPtr;
    int isNew;

    if (nameObj == NULL) {
	mPtr = (Method *)ckalloc(sizeof(Method));
	mPtr->namePtr = NULL;
	mPtr->refCount = 1;
	goto populate;
    }
    hPtr = Tcl_CreateHashEntry(&clsPtr->classMethods, (char *)nameObj,&isNew);
    if (isNew) {
	mPtr = (Method *)ckalloc(sizeof(Method));
	mPtr->refCount = 1;
	mPtr->namePtr = nameObj;
	Tcl_IncrRefCount(nameObj);
	Tcl_SetHashValue(hPtr, mPtr);
    } else {
	mPtr = (Method *)Tcl_GetHashValue(hPtr);
	if (mPtr->typePtr != NULL && mPtr->typePtr->deleteProc != NULL) {
	    mPtr->typePtr->deleteProc(mPtr->clientData);
	}
    }

  populate:
    clsPtr->thisPtr->fPtr->epoch++;
    mPtr->typePtr = typePtr;
    mPtr->clientData = clientData;
    mPtr->flags = 0;
    mPtr->declaringObjectPtr = NULL;
    mPtr->declaringClassPtr = clsPtr;
    if (flags) {
	mPtr->flags |= flags & (PUBLIC_METHOD | PRIVATE_METHOD);
    }

    return (Tcl_Method) mPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODelMethodRef --
 *
 *	How to delete a method.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->typePtr != NULL && mPtr->typePtr->deleteProc != NULL) {
	    mPtr->typePtr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	ckfree(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewBasicMethod --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOONewBasicMethod(
    Tcl_Interp *interp,

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 302–398. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
{
    Tcl_Obj *namePtr = Tcl_NewStringObj(dcm->name, -1);

    Tcl_IncrRefCount(namePtr);
    Tcl_NewMethod(interp, (Tcl_Class) clsPtr, namePtr,
	    (dcm->isPublic ? PUBLIC_METHOD : 0), &dcm->definition, NULL);
    Tcl_DecrRefCount(namePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which must not be
				 * NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which must not be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which must not be
				 * NULL. */
    ProcedureMethod **pmPtrPtr)	/* Place to write pointer to procedure method
				 * structure to allow for deeper tuning of the
				 * structure's contents. NULL if caller is not
				 * interested. */
{
    int argsLen;
    ProcedureMethod *pmPtr;
    Tcl_Method method;

    if (TclListObjLength(interp, argsObj, &argsLen) != TCL_OK) {
	return NULL;
    }
    pmPtr = AllocProcedureMethodRecord(flags);
    method = TclOOMakeProcInstanceMethod(interp, oPtr, flags, nameObj,
	    argsObj, bodyObj, &procMethodType, pmPtr, &pmPtr->procPtr);
    if (method == NULL) {
	ckfree(pmPtr);
    } else if (pmPtrPtr != NULL) {
	*pmPtrPtr = pmPtr;
    }
    return (Method *) method;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcMethod --
 *
 *	Create a new procedure-like method for a class.
 *
 * ----------------------------------------------------------------------
 */

Method *
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 630–727. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
	    context.data.eval.path = NULL;
	}
    }

    return Tcl_NewMethod(interp, (Tcl_Class) clsPtr, nameObj, flags, typePtr,
	    clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * InvokeProcedureMethod, PushMethodCallFrame --
 *
 *	How to invoke a procedure-like method.
 *
 * ----------------------------------------------------------------------
 */

static int
InvokeProcedureMethod(
    void *clientData,		/* Pointer to some per-method context. */
    Tcl_Interp *interp,
    Tcl_ObjectContext context,	/* The method calling context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments as actually seen. */
{
    ProcedureMethod *pmPtr = (ProcedureMethod *)clientData;
    int result;
    PMFrameData *fdPtr;		/* Important data that has to have a lifetime
				 * matched by this function (or rather, by the
				 * call frame's lifetime). */

    /*
     * If the object namespace (or interpreter) were deleted, we just skip to
     * the next thing in the chain.
     */

    if (TclOOObjectDestroyed(((CallContext *)context)->oPtr) ||
	Tcl_InterpDeleted(interp)
    ) {
	return TclNRObjectContextInvokeNext(interp, context, objc, objv,
		Tcl_ObjectContextSkippedArgs(context));
    }

    /*
     * Finishes filling out the extra frame info so that [info frame] works if
     * that is not already set up.
     */

    if (pmPtr->efi.length == 0) {
	Tcl_Method method = Tcl_ObjectContextMethod(context);

	pmPtr->efi.length = 2;
	pmPtr->efi.fields[0].name = "method";
	pmPtr->efi.fields[0].proc = RenderMethodName;
	pmPtr->efi.fields[0].clientData = pmPtr;
	pmPtr->callSiteFlags = ((CallContext *)
		context)->callPtr->flags & (CONSTRUCTOR | DESTRUCTOR);
	pmPtr->interp = interp;
	pmPtr->method = method;
	if (pmPtr->gfivProc != NULL) {
	    pmPtr->efi.fields[1].name = "";
	    pmPtr->efi.fields[1].proc = pmPtr->gfivProc;
	    pmPtr->efi.fields[1].clientData = pmPtr;
	} else {
	    if (Tcl_MethodDeclarerObject(method) != NULL) {
		pmPtr->efi.fields[1].name = "object";
	    } else {
		pmPtr->efi.fields[1].name = "class";
	    }
	    pmPtr->efi.fields[1].proc = RenderDeclarerName;
	    pmPtr->efi.fields[1].clientData = pmPtr;
	}
    }

    /*
     * Allocate the special frame data.
     */

    fdPtr = (PMFrameData *)TclStackAlloc(interp, sizeof(PMFrameData));

    /*
     * Create a call frame for this method.
     */

    result = PushMethodCallFrame(interp, (CallContext *) context, pmPtr,
	    objc, objv, fdPtr);
    if (result != TCL_OK) {
	TclStackFree(interp, fdPtr);
	return result;
    }
    pmPtr->refCount++;

    /*
     * Give the pre-call callback a chance to do some setup and, possibly,
     * veto the call.
     */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 1376–1472. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text

    fmPtr = (ForwardMethod *)ckalloc(sizeof(ForwardMethod));
    fmPtr->prefixObj = prefixObj;
    Tcl_IncrRefCount(prefixObj);
    return (Method *) Tcl_NewMethod(interp, (Tcl_Class) clsPtr, nameObj,
	    flags, &fwdMethodType, fmPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * InvokeForwardMethod --
 *
 *	How to invoke a forwarded method. Works by doing some ensemble-like
 *	command rearranging and then invokes some other Tcl command.
 *
 * ----------------------------------------------------------------------
 */

static int
InvokeForwardMethod(
    void *clientData,	/* Pointer to some per-method context. */
    Tcl_Interp *interp,
    Tcl_ObjectContext context,	/* The method calling context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments as actually seen. */
{
    CallContext *contextPtr = (CallContext *) context;
    ForwardMethod *fmPtr = (ForwardMethod *)clientData;
    Tcl_Obj **argObjs, **prefixObjs;
    int numPrefixes, len, skip = contextPtr->skip;

    /*
     * Build the real list of arguments to use. Note that we know that the
     * prefixObj field of the ForwardMethod structure holds a reference to a
     * non-empty list, so there's a whole class of failures ("not a list") we
     * can ignore here.
     */

    TclListObjGetElements(NULL, fmPtr->prefixObj, &numPrefixes, &prefixObjs);
    argObjs = InitEnsembleRewrite(interp, objc, objv, skip,
	    numPrefixes, prefixObjs, &len);
    Tcl_NRAddCallback(interp, FinalizeForwardCall, argObjs, NULL, NULL, NULL);
    /*
     * NOTE: The combination of direct set of iPtr->lookupNsPtr and the use
     * of the TCL_EVAL_NOERR flag results in an evaluation configuration
     * very much like TCL_EVAL_INVOKE.
     */
    ((Interp *)interp)->lookupNsPtr
	    = (Namespace *) contextPtr->oPtr->namespacePtr;
    return TclNREvalObjv(interp, len, argObjs, TCL_EVAL_NOERR, NULL);
}

static int
FinalizeForwardCall(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    Tcl_Obj **argObjs = (Tcl_Obj **)data[0];

    TclStackFree(interp, argObjs);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * DeleteForwardMethod, CloneForwardMethod --
 *
 *	How to delete and clone forwarded methods.
 *
 * ----------------------------------------------------------------------
 */

static void
DeleteForwardMethod(
    void *clientData)
{
    ForwardMethod *fmPtr = (ForwardMethod *)clientData;

    Tcl_DecrRefCount(fmPtr->prefixObj);
    ckfree(fmPtr);
}

static int
CloneForwardMethod(
    Tcl_Interp *interp,
    void *clientData,
    void **newClientData)
{
    ForwardMethod *fmPtr = (ForwardMethod *)clientData;
    ForwardMethod *fm2Ptr = (ForwardMethod *)ckalloc(sizeof(ForwardMethod));

    fm2Ptr->prefixObj = fmPtr->prefixObj;
    Tcl_IncrRefCount(fm2Ptr->prefixObj);
    *newClientData = fm2Ptr;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `TclOODelMethodRef(`, lines 265–360. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->typePtr != NULL && mPtr->typePtr->deleteProc != NULL) {
	    mPtr->typePtr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	ckfree(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewBasicMethod --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOONewBasicMethod(
    Tcl_Interp *interp,
    Class *clsPtr,		/* Class to attach the method to. */
    const DeclaredClassMethod *dcm)
				/* Name of the method, whether it is public,
				 * and the function to implement it. */
{
    Tcl_Obj *namePtr = Tcl_NewStringObj(dcm->name, -1);

    Tcl_IncrRefCount(namePtr);
    Tcl_NewMethod(interp, (Tcl_Class) clsPtr, namePtr,
	    (dcm->isPublic ? PUBLIC_METHOD : 0), &dcm->definition, NULL);
    Tcl_DecrRefCount(namePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which must not be
				 * NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which must not be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which must not be
				 * NULL. */
    ProcedureMethod **pmPtrPtr)	/* Place to write pointer to procedure method
				 * structure to allow for deeper tuning of the
				 * structure's contents. NULL if caller is not
				 * interested. */
{
    int argsLen;
    ProcedureMethod *pmPtr;
    Tcl_Method method;

    if (TclListObjLength(interp, argsObj, &argsLen) != TCL_OK) {
	return NULL;
    }
    pmPtr = AllocProcedureMethodRecord(flags);
    method = TclOOMakeProcInstanceMethod(interp, oPtr, flags, nameObj,
	    argsObj, bodyObj, &procMethodType, pmPtr, &pmPtr->procPtr);
    if (method == NULL) {
	ckfree(pmPtr);
    } else if (pmPtrPtr != NULL) {
	*pmPtrPtr = pmPtr;
    }
    return (Method *) method;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcMethod --
 *
 *	Create a new procedure-like method for a class.

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInt.h`, function `#define AddRef`, lines 559–617. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h`.

```text
/*
 * Alternatives to Tcl_Preserve/Tcl_EventuallyFree/Tcl_Release.
 */

#define AddRef(ptr) ((ptr)->refCount++)

/*
 * A convenience macro for iterating through the lists used in the internal
 * memory management of objects.
 * REQUIRES DECLARATION: int i;
 */

#define FOREACH(var,ary) \
    for(i=0 ; i<(ary).num; i++) if ((ary).list[i] == NULL) { \
	continue; \
    } else if ((var) = (ary).list[i], 1)

/*
 * Convenience macros for iterating through hash tables. FOREACH_HASH_DECLS
 * sets up the declarations needed for the main macro, FOREACH_HASH, which
 * does the actual iteration. FOREACH_HASH_VALUE is a restricted version that
 * only iterates over values.
 */

#define FOREACH_HASH_DECLS \
    Tcl_HashEntry *hPtr;Tcl_HashSearch search
#define FOREACH_HASH(key,val,tablePtr) \
    for(hPtr=Tcl_FirstHashEntry((tablePtr),&search); hPtr!=NULL ? \
	    ((key)=(void *)Tcl_GetHashKey((tablePtr),hPtr),\
	    (val)=Tcl_GetHashValue(hPtr),1):0; hPtr=Tcl_NextHashEntry(&search))
#define FOREACH_HASH_VALUE(val,tablePtr) \
    for(hPtr=Tcl_FirstHashEntry((tablePtr),&search); hPtr!=NULL ? \
	    ((val)=Tcl_GetHashValue(hPtr),1):0;hPtr=Tcl_NextHashEntry(&search))

/*
 * Convenience macro for duplicating a list. Needs no external declaration,
 * but all arguments are used multiple times and so must have no side effects.
 */

#undef DUPLICATE /* prevent possible conflict with definition in WINAPI nb30.h */
#define DUPLICATE(target,source,type) \
    do { \
	size_t len = sizeof(type) * ((target).num=(source).num);\
	if (len != 0) { \
	    memcpy(((target).list=(type*)ckalloc(len)), (source).list, len); \
	} else { \
	    (target).list = NULL; \
	} \
    } while(0)

#endif /* TCL_OO_INTERNAL_H */

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInt.h`, function `struct Method`, lines 40–138. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h`.

```text
 * information about all sorts of methods, including forwards, constructors
 * and destructors.
 */

typedef struct Method {
    const Tcl_MethodType *typePtr;
				/* The type of method. If NULL, this is a
				 * special flag record which is just used for
				 * the setting of the flags field. */
    int refCount;
    void *clientData;	/* Type-specific data. */
    Tcl_Obj *namePtr;		/* Name of the method. */
    struct Object *declaringObjectPtr;
				/* The object that declares this method, or
				 * NULL if it was declared by a class. */
    struct Class *declaringClassPtr;
				/* The class that declares this method, or
				 * NULL if it was declared directly on an
				 * object. */
    int flags;			/* Assorted flags. Includes whether this
				 * method is public/exported or not. */
} Method;

/*
 * Pre- and post-call callbacks, to allow procedure-like methods to be fine
 * tuned in their behaviour.
 */

typedef int (TclOO_PreCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_CallFrame *framePtr, int *isFinished);
typedef int (TclOO_PostCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_Namespace *namespacePtr, int result);
typedef void (TclOO_PmCDDeleteProc)(void *clientData);
typedef void *(TclOO_PmCDCloneProc)(void *clientData);

/*
 * Procedure-like methods have the following extra information.
 */

typedef struct ProcedureMethod {
    int version;		/* Version of this structure. Currently must
				 * be 0. */
    Proc *procPtr;		/* Core of the implementation of the method;
				 * includes the argument definition and the
				 * body bytecodes. */
    int flags;			/* Flags to control features. */
    int refCount;
    void *clientData;
    TclOO_PmCDDeleteProc *deleteClientdataProc;
    TclOO_PmCDCloneProc *cloneClientdataProc;
    ProcErrorProc *errProc;	/* Replacement error handler. */
    TclOO_PreCallProc *preCallProc;
				/* Callback to allow for additional setup
				 * before the method executes. */
    TclOO_PostCallProc *postCallProc;
				/* Callback to allow for additional cleanup
				 * after the method executes. */
    GetFrameInfoValueProc *gfivProc;
				/* Callback to allow for fine tuning of how
				 * the method reports itself. */
    Command cmd;		/* Space used to connect to [info frame] */
    ExtraFrameInfo efi;		/* Space used to store data for [info frame] */
    Tcl_Interp *interp;		/* Interpreter in which to compute the name of
				 * the method. */
    Tcl_Method method;		/* Method to compute the name of. */
    int callSiteFlags;		/* Flags from the call chain. Only interested
				 * in whether this is a constructor or
				 * destructor, which we can't know until then
				 * for messy reasons. Other flags are variable
				 * but not used. */
} ProcedureMethod;

#define TCLOO_PROCEDURE_METHOD_VERSION 0

/*
 * Flags for use in a ProcedureMethod.
 *
 * When the USE_DECLARER_NS flag is set, the method will use the namespace of
 * the object or class that declared it (or the clone of it, if it was from
 * such that the implementation of the method came to the particular use)
 * instead of the namespace of the object on which the method was invoked.
 * This flag must be distinct from all others that are associated with
 * methods.
 */

#define USE_DECLARER_NS		0x80

/*
 * Forwarded methods have the following extra information.
 */

typedef struct ForwardMethod {
    Tcl_Obj *prefixObj;		/* The list of values to use to replace the
				 * object and method name with. Will be a
				 * non-empty list. */
} ForwardMethod;

/*
 * Helper definitions that declare a "list" array. The two varieties are

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInt.h`, function `struct CallChain`, lines 29–127. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h`.

```text
/*
 * Forward declarations.
 */

struct CallChain;
struct Class;
struct Foundation;
struct Object;

/*
 * The data that needs to be stored per method. This record is used to collect
 * information about all sorts of methods, including forwards, constructors
 * and destructors.
 */

typedef struct Method {
    const Tcl_MethodType *typePtr;
				/* The type of method. If NULL, this is a
				 * special flag record which is just used for
				 * the setting of the flags field. */
    int refCount;
    void *clientData;	/* Type-specific data. */
    Tcl_Obj *namePtr;		/* Name of the method. */
    struct Object *declaringObjectPtr;
				/* The object that declares this method, or
				 * NULL if it was declared by a class. */
    struct Class *declaringClassPtr;
				/* The class that declares this method, or
				 * NULL if it was declared directly on an
				 * object. */
    int flags;			/* Assorted flags. Includes whether this
				 * method is public/exported or not. */
} Method;

/*
 * Pre- and post-call callbacks, to allow procedure-like methods to be fine
 * tuned in their behaviour.
 */

typedef int (TclOO_PreCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_CallFrame *framePtr, int *isFinished);
typedef int (TclOO_PostCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_Namespace *namespacePtr, int result);
typedef void (TclOO_PmCDDeleteProc)(void *clientData);
typedef void *(TclOO_PmCDCloneProc)(void *clientData);

/*
 * Procedure-like methods have the following extra information.
 */

typedef struct ProcedureMethod {
    int version;		/* Version of this structure. Currently must
				 * be 0. */
    Proc *procPtr;		/* Core of the implementation of the method;
				 * includes the argument definition and the
				 * body bytecodes. */
    int flags;			/* Flags to control features. */
    int refCount;
    void *clientData;
    TclOO_PmCDDeleteProc *deleteClientdataProc;
    TclOO_PmCDCloneProc *cloneClientdataProc;
    ProcErrorProc *errProc;	/* Replacement error handler. */
    TclOO_PreCallProc *preCallProc;
				/* Callback to allow for additional setup
				 * before the method executes. */
    TclOO_PostCallProc *postCallProc;
				/* Callback to allow for additional cleanup
				 * after the method executes. */
    GetFrameInfoValueProc *gfivProc;
				/* Callback to allow for fine tuning of how
				 * the method reports itself. */
    Command cmd;		/* Space used to connect to [info frame] */
    ExtraFrameInfo efi;		/* Space used to store data for [info frame] */
    Tcl_Interp *interp;		/* Interpreter in which to compute the name of
				 * the method. */
    Tcl_Method method;		/* Method to compute the name of. */
    int callSiteFlags;		/* Flags from the call chain. Only interested
				 * in whether this is a constructor or
				 * destructor, which we can't know until then
				 * for messy reasons. Other flags are variable
				 * but not used. */
} ProcedureMethod;

#define TCLOO_PROCEDURE_METHOD_VERSION 0

/*
 * Flags for use in a ProcedureMethod.
 *
 * When the USE_DECLARER_NS flag is set, the method will use the namespace of
 * the object or class that declared it (or the clone of it, if it was from
 * such that the implementation of the method came to the particular use)
 * instead of the namespace of the object on which the method was invoked.
 * This flag must be distinct from all others that are associated with
 * methods.
 */

#define USE_DECLARER_NS		0x80

/*

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInt.h`, function `struct CallChain`, lines 265–363. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h`.

```text
				 * the ClientData values that are the values
				 * of each piece of attached metadata. This
				 * field starts out as NULL and is only
				 * allocated if metadata is attached. */
    struct CallChain *constructorChainPtr;
    struct CallChain *destructorChainPtr;
    Tcl_HashTable *classChainCache;
				/* Places where call chains are stored. For
				 * constructors, the class chain is always
				 * used. For destructors and ordinary methods,
				 * the class chain is only used when the
				 * object doesn't override with its own mixins
				 * (and filters and method implementations for
				 * when getting method chains). */
    LIST_STATIC(Tcl_Obj *) variables;
} Class;

/*
 * The foundation of the object system within an interpreter contains
 * references to the key classes and namespaces, together with a few other
 * useful bits and pieces. Probably ought to eventually go in the Interp
 * structure itself.
 */

typedef struct ThreadLocalData {
    int nsCount;		/* Epoch counter is used for keeping
				 * the values used in Tcl_Obj internal
				 * representations sane. Must be thread-local
				 * because Tcl_Objs can cross interpreter
				 * boundaries within a thread (objects don't
				 * generally cross threads). */
} ThreadLocalData;

typedef struct Foundation {
    Tcl_Interp *interp;
    Class *objectCls;		/* The root of the object system. */
    Class *classCls;		/* The class of all classes. */
    Tcl_Namespace *ooNs;	/* ::oo namespace. */
    Tcl_Namespace *defineNs;	/* Namespace containing special commands for
				 * manipulating objects and classes. The
				 * "oo::define" command acts as a special kind
				 * of ensemble for this namespace. */
    Tcl_Namespace *objdefNs;	/* Namespace containing special commands for
				 * manipulating objects and classes. The
				 * "oo::objdefine" command acts as a special
				 * kind of ensemble for this namespace. */
    Tcl_Namespace *helpersNs;	/* Namespace containing the commands that are
				 * only valid when executing inside a
				 * procedural method. */
    int epoch;			/* Used to invalidate method chains when the
				 * class structure changes. */
    ThreadLocalData *tsdPtr;	/* Counter so we can allocate a unique
				 * namespace to each object. */
    Tcl_Obj *unknownMethodNameObj;
				/* Shared object containing the name of the
				 * unknown method handler method. */
    Tcl_Obj *constructorName;	/* Shared object containing the "name" of a
				 * constructor. */
    Tcl_Obj *destructorName;	/* Shared object containing the "name" of a
				 * destructor. */
    Tcl_Obj *clonedName;	/* Shared object containing the name of a
				 * "<cloned>" pseudo-constructor. */
    Tcl_Obj *defineName;	/* Fully qualified name of oo::define. */
} Foundation;

/*
 * A call context structure is built when a method is called. It contains the
 * chain of method implementations that are to be invoked by a particular
 * call, and the process of calling walks the chain, with the [next] command
 * proceeding to the next entry in the chain.
 */

#define CALL_CHAIN_STATIC_SIZE 4

struct MInvoke {
    Method *mPtr;		/* Reference to the method implementation
				 * record. */
    int isFilter;		/* Whether this is a filter invocation. */
    Class *filterDeclarer;	/* What class decided to add the filter; if
				 * NULL, it was added by the object. */
};

typedef struct CallChain {
    int objectCreationEpoch;	/* The object's creation epoch. Note that the
				 * object reference is not stored in the call
				 * chain; it is in the call context. */
    int objectEpoch;		/* Local (object structure) epoch counter
				 * snapshot. */
    int epoch;			/* Global (class structure) epoch counter
				 * snapshot. */
    int flags;			/* Assorted flags, see below. */
    int refCount;		/* Reference count. */
    int numChain;		/* Size of the call chain. */
    struct MInvoke *chain;	/* Array of call chain entries. May point to
				 * staticChain if the number of entries is
				 * small. */
    struct MInvoke staticChain[CALL_CHAIN_STATIC_SIZE];
} CallChain;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInt.h`, function `struct CallChain`, lines 266–364. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h`.

```text
				 * of each piece of attached metadata. This
				 * field starts out as NULL and is only
				 * allocated if metadata is attached. */
    struct CallChain *constructorChainPtr;
    struct CallChain *destructorChainPtr;
    Tcl_HashTable *classChainCache;
				/* Places where call chains are stored. For
				 * constructors, the class chain is always
				 * used. For destructors and ordinary methods,
				 * the class chain is only used when the
				 * object doesn't override with its own mixins
				 * (and filters and method implementations for
				 * when getting method chains). */
    LIST_STATIC(Tcl_Obj *) variables;
} Class;

/*
 * The foundation of the object system within an interpreter contains
 * references to the key classes and namespaces, together with a few other
 * useful bits and pieces. Probably ought to eventually go in the Interp
 * structure itself.
 */

typedef struct ThreadLocalData {
    int nsCount;		/* Epoch counter is used for keeping
				 * the values used in Tcl_Obj internal
				 * representations sane. Must be thread-local
				 * because Tcl_Objs can cross interpreter
				 * boundaries within a thread (objects don't
				 * generally cross threads). */
} ThreadLocalData;

typedef struct Foundation {
    Tcl_Interp *interp;
    Class *objectCls;		/* The root of the object system. */
    Class *classCls;		/* The class of all classes. */
    Tcl_Namespace *ooNs;	/* ::oo namespace. */
    Tcl_Namespace *defineNs;	/* Namespace containing special commands for
				 * manipulating objects and classes. The
				 * "oo::define" command acts as a special kind
				 * of ensemble for this namespace. */
    Tcl_Namespace *objdefNs;	/* Namespace containing special commands for
				 * manipulating objects and classes. The
				 * "oo::objdefine" command acts as a special
				 * kind of ensemble for this namespace. */
    Tcl_Namespace *helpersNs;	/* Namespace containing the commands that are
				 * only valid when executing inside a
				 * procedural method. */
    int epoch;			/* Used to invalidate method chains when the
				 * class structure changes. */
    ThreadLocalData *tsdPtr;	/* Counter so we can allocate a unique
				 * namespace to each object. */
    Tcl_Obj *unknownMethodNameObj;
				/* Shared object containing the name of the
				 * unknown method handler method. */
    Tcl_Obj *constructorName;	/* Shared object containing the "name" of a
				 * constructor. */
    Tcl_Obj *destructorName;	/* Shared object containing the "name" of a
				 * destructor. */
    Tcl_Obj *clonedName;	/* Shared object containing the name of a
				 * "<cloned>" pseudo-constructor. */
    Tcl_Obj *defineName;	/* Fully qualified name of oo::define. */
} Foundation;

/*
 * A call context structure is built when a method is called. It contains the
 * chain of method implementations that are to be invoked by a particular
 * call, and the process of calling walks the chain, with the [next] command
 * proceeding to the next entry in the chain.
 */

#define CALL_CHAIN_STATIC_SIZE 4

struct MInvoke {
    Method *mPtr;		/* Reference to the method implementation
				 * record. */
    int isFilter;		/* Whether this is a filter invocation. */
    Class *filterDeclarer;	/* What class decided to add the filter; if
				 * NULL, it was added by the object. */
};

typedef struct CallChain {
    int objectCreationEpoch;	/* The object's creation epoch. Note that the
				 * object reference is not stored in the call
				 * chain; it is in the call context. */
    int objectEpoch;		/* Local (object structure) epoch counter
				 * snapshot. */
    int epoch;			/* Global (class structure) epoch counter
				 * snapshot. */
    int flags;			/* Assorted flags, see below. */
    int refCount;		/* Reference count. */
    int numChain;		/* Size of the call chain. */
    struct MInvoke *chain;	/* Array of call chain entries. May point to
				 * staticChain if the number of entries is
				 * small. */
    struct MInvoke staticChain[CALL_CHAIN_STATIC_SIZE];
} CallChain;

typedef struct CallContext {

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInt.h`, function `struct CallChain`, lines 343–440. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h`.

```text
    Class *filterDeclarer;	/* What class decided to add the filter; if
				 * NULL, it was added by the object. */
};

typedef struct CallChain {
    int objectCreationEpoch;	/* The object's creation epoch. Note that the
				 * object reference is not stored in the call
				 * chain; it is in the call context. */
    int objectEpoch;		/* Local (object structure) epoch counter
				 * snapshot. */
    int epoch;			/* Global (class structure) epoch counter
				 * snapshot. */
    int flags;			/* Assorted flags, see below. */
    int refCount;		/* Reference count. */
    int numChain;		/* Size of the call chain. */
    struct MInvoke *chain;	/* Array of call chain entries. May point to
				 * staticChain if the number of entries is
				 * small. */
    struct MInvoke staticChain[CALL_CHAIN_STATIC_SIZE];
} CallChain;

typedef struct CallContext {
    Object *oPtr;		/* The object associated with this call. */
    int index;			/* Index into the call chain of the currently
				 * executing method implementation. */
    int skip;			/* Current number of arguments to skip; can
				 * vary depending on whether it is a direct
				 * method call or a continuation via the
				 * [next] command. */
    CallChain *callPtr;		/* The actual call chain. */
} CallContext;

/*
 * Bits for the 'flags' field of the call chain.
 */

#define PUBLIC_METHOD     0x01	/* This is a public (exported) method. */
#define PRIVATE_METHOD    0x02	/* This is a private (class's direct instances
				 * only) method. Supports itcl. */
#define OO_UNKNOWN_METHOD 0x04	/* This is an unknown method. */
#define CONSTRUCTOR	  0x08	/* This is a constructor. */
#define DESTRUCTOR	  0x10	/* This is a destructor. */

/*
 * Structure containing definition information about basic class methods.
 */

typedef struct {
    const char *name;		/* Name of the method in question. */
    int isPublic;		/* Whether the method is public by default. */
    Tcl_MethodType definition;	/* How to call the method. */
} DeclaredClassMethod;

/*
 *----------------------------------------------------------------
 * Commands relating to OO support.
 *----------------------------------------------------------------
 */

MODULE_SCOPE int	TclOOInit(Tcl_Interp *interp);
MODULE_SCOPE int	TclOODefineObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOOObjDefObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineConstructorObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineDeleteMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineDestructorObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineExportObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineForwardObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineRenameMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineUnexportObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineClassObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineSelfObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOOUnknownDefinition(void *clientData,
			    Tcl_Interp *interp, int objc,

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInt.h`, function `struct MInvoke`, lines 335–432. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h`.

```text
 */

#define CALL_CHAIN_STATIC_SIZE 4

struct MInvoke {
    Method *mPtr;		/* Reference to the method implementation
				 * record. */
    int isFilter;		/* Whether this is a filter invocation. */
    Class *filterDeclarer;	/* What class decided to add the filter; if
				 * NULL, it was added by the object. */
};

typedef struct CallChain {
    int objectCreationEpoch;	/* The object's creation epoch. Note that the
				 * object reference is not stored in the call
				 * chain; it is in the call context. */
    int objectEpoch;		/* Local (object structure) epoch counter
				 * snapshot. */
    int epoch;			/* Global (class structure) epoch counter
				 * snapshot. */
    int flags;			/* Assorted flags, see below. */
    int refCount;		/* Reference count. */
    int numChain;		/* Size of the call chain. */
    struct MInvoke *chain;	/* Array of call chain entries. May point to
				 * staticChain if the number of entries is
				 * small. */
    struct MInvoke staticChain[CALL_CHAIN_STATIC_SIZE];
} CallChain;

typedef struct CallContext {
    Object *oPtr;		/* The object associated with this call. */
    int index;			/* Index into the call chain of the currently
				 * executing method implementation. */
    int skip;			/* Current number of arguments to skip; can
				 * vary depending on whether it is a direct
				 * method call or a continuation via the
				 * [next] command. */
    CallChain *callPtr;		/* The actual call chain. */
} CallContext;

/*
 * Bits for the 'flags' field of the call chain.
 */

#define PUBLIC_METHOD     0x01	/* This is a public (exported) method. */
#define PRIVATE_METHOD    0x02	/* This is a private (class's direct instances
				 * only) method. Supports itcl. */
#define OO_UNKNOWN_METHOD 0x04	/* This is an unknown method. */
#define CONSTRUCTOR	  0x08	/* This is a constructor. */
#define DESTRUCTOR	  0x10	/* This is a destructor. */

/*
 * Structure containing definition information about basic class methods.
 */

typedef struct {
    const char *name;		/* Name of the method in question. */
    int isPublic;		/* Whether the method is public by default. */
    Tcl_MethodType definition;	/* How to call the method. */
} DeclaredClassMethod;

/*
 *----------------------------------------------------------------
 * Commands relating to OO support.
 *----------------------------------------------------------------
 */

MODULE_SCOPE int	TclOOInit(Tcl_Interp *interp);
MODULE_SCOPE int	TclOODefineObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOOObjDefObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineConstructorObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineDeleteMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineDestructorObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineExportObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineForwardObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineRenameMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineUnexportObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInt.h`, function `struct MInvoke`, lines 354–451. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h`.

```text
				 * snapshot. */
    int flags;			/* Assorted flags, see below. */
    int refCount;		/* Reference count. */
    int numChain;		/* Size of the call chain. */
    struct MInvoke *chain;	/* Array of call chain entries. May point to
				 * staticChain if the number of entries is
				 * small. */
    struct MInvoke staticChain[CALL_CHAIN_STATIC_SIZE];
} CallChain;

typedef struct CallContext {
    Object *oPtr;		/* The object associated with this call. */
    int index;			/* Index into the call chain of the currently
				 * executing method implementation. */
    int skip;			/* Current number of arguments to skip; can
				 * vary depending on whether it is a direct
				 * method call or a continuation via the
				 * [next] command. */
    CallChain *callPtr;		/* The actual call chain. */
} CallContext;

/*
 * Bits for the 'flags' field of the call chain.
 */

#define PUBLIC_METHOD     0x01	/* This is a public (exported) method. */
#define PRIVATE_METHOD    0x02	/* This is a private (class's direct instances
				 * only) method. Supports itcl. */
#define OO_UNKNOWN_METHOD 0x04	/* This is an unknown method. */
#define CONSTRUCTOR	  0x08	/* This is a constructor. */
#define DESTRUCTOR	  0x10	/* This is a destructor. */

/*
 * Structure containing definition information about basic class methods.
 */

typedef struct {
    const char *name;		/* Name of the method in question. */
    int isPublic;		/* Whether the method is public by default. */
    Tcl_MethodType definition;	/* How to call the method. */
} DeclaredClassMethod;

/*
 *----------------------------------------------------------------
 * Commands relating to OO support.
 *----------------------------------------------------------------
 */

MODULE_SCOPE int	TclOOInit(Tcl_Interp *interp);
MODULE_SCOPE int	TclOODefineObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOOObjDefObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineConstructorObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineDeleteMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineDestructorObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineExportObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineForwardObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineRenameMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineUnexportObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineClassObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineSelfObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOOUnknownDefinition(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOOCopyObjectCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOONextObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOONextToObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOOSelfObjCmd(void *clientData,

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInt.h`, function `struct MInvoke`, lines 357–454. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h`.

```text
    int numChain;		/* Size of the call chain. */
    struct MInvoke *chain;	/* Array of call chain entries. May point to
				 * staticChain if the number of entries is
				 * small. */
    struct MInvoke staticChain[CALL_CHAIN_STATIC_SIZE];
} CallChain;

typedef struct CallContext {
    Object *oPtr;		/* The object associated with this call. */
    int index;			/* Index into the call chain of the currently
				 * executing method implementation. */
    int skip;			/* Current number of arguments to skip; can
				 * vary depending on whether it is a direct
				 * method call or a continuation via the
				 * [next] command. */
    CallChain *callPtr;		/* The actual call chain. */
} CallContext;

/*
 * Bits for the 'flags' field of the call chain.
 */

#define PUBLIC_METHOD     0x01	/* This is a public (exported) method. */
#define PRIVATE_METHOD    0x02	/* This is a private (class's direct instances
				 * only) method. Supports itcl. */
#define OO_UNKNOWN_METHOD 0x04	/* This is an unknown method. */
#define CONSTRUCTOR	  0x08	/* This is a constructor. */
#define DESTRUCTOR	  0x10	/* This is a destructor. */

/*
 * Structure containing definition information about basic class methods.
 */

typedef struct {
    const char *name;		/* Name of the method in question. */
    int isPublic;		/* Whether the method is public by default. */
    Tcl_MethodType definition;	/* How to call the method. */
} DeclaredClassMethod;

/*
 *----------------------------------------------------------------
 * Commands relating to OO support.
 *----------------------------------------------------------------
 */

MODULE_SCOPE int	TclOOInit(Tcl_Interp *interp);
MODULE_SCOPE int	TclOODefineObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOOObjDefObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineConstructorObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineDeleteMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineDestructorObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineExportObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineForwardObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineRenameMethodObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineUnexportObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineClassObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOODefineSelfObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOOUnknownDefinition(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOOCopyObjectCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOONextObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOONextToObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
MODULE_SCOPE int	TclOOSelfObjCmd(void *clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.h`, function `typedef int (Tcl_MethodCallProc`, lines 59–147. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOO.h`.

```text
 * implementation. These are used to implement custom types of method calls
 * and to allow the attachment of arbitrary data to objects and classes.
 */

typedef int (Tcl_MethodCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext objectContext, int objc, Tcl_Obj *const *objv);
typedef void (Tcl_MethodDeleteProc)(void *clientData);
typedef int (Tcl_CloneProc)(Tcl_Interp *interp, void *oldClientData,
	void **newClientData);
typedef void (Tcl_ObjectMetadataDeleteProc)(void *clientData);
typedef int (Tcl_ObjectMapMethodNameProc)(Tcl_Interp *interp,
	Tcl_Object object, Tcl_Class *startClsPtr, Tcl_Obj *methodNameObj);

/*
 * The type of a method implementation. This describes how to call the method
 * implementation, how to delete it (when the object or class is deleted) and
 * how to create a clone of it (when the object or class is copied).
 */

typedef struct {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_CURRENT in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType;

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */

#define TCL_OO_METHOD_VERSION_CURRENT 1

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

#define TCL_OO_METADATA_VERSION_CURRENT 1

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 76–147. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOO.h`.

```text
 */

typedef struct {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_CURRENT in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType;

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */

#define TCL_OO_METHOD_VERSION_CURRENT 1

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

#define TCL_OO_METADATA_VERSION_CURRENT 1

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 97–147. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOO.h`.

```text
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */

#define TCL_OO_METHOD_VERSION_CURRENT 1

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

#define TCL_OO_METADATA_VERSION_CURRENT 1

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 61–159. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOODecls.h`.

```text
				Tcl_Object object, Tcl_Obj *nameObj,
				int isPublic, const Tcl_MethodType *typePtr,
				void *clientData);
/* 12 */
TCLAPI Tcl_Method	Tcl_NewMethod(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int isPublic,
				const Tcl_MethodType *typePtr,
				void *clientData);
/* 13 */
TCLAPI Tcl_Object	Tcl_NewObjectInstance(Tcl_Interp *interp,
				Tcl_Class cls, const char *nameStr,
				const char *nsNameStr, int objc,
				Tcl_Obj *const *objv, int skip);
/* 14 */
TCLAPI int		Tcl_ObjectDeleted(Tcl_Object object);
/* 15 */
TCLAPI int		Tcl_ObjectContextIsFiltering(
				Tcl_ObjectContext context);
/* 16 */
TCLAPI Tcl_Method	Tcl_ObjectContextMethod(Tcl_ObjectContext context);
/* 17 */
TCLAPI Tcl_Object	Tcl_ObjectContextObject(Tcl_ObjectContext context);
/* 18 */
TCLAPI int		Tcl_ObjectContextSkippedArgs(
				Tcl_ObjectContext context);
/* 19 */
TCLAPI void *		Tcl_ClassGetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr);
/* 20 */
TCLAPI void		Tcl_ClassSetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 21 */
TCLAPI void *		Tcl_ObjectGetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr);
/* 22 */
TCLAPI void		Tcl_ObjectSetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 23 */
TCLAPI int		Tcl_ObjectContextInvokeNext(Tcl_Interp *interp,
				Tcl_ObjectContext context, int objc,
				Tcl_Obj *const *objv, int skip);
/* 24 */
TCLAPI Tcl_ObjectMapMethodNameProc * Tcl_ObjectGetMethodNameMapper(
				Tcl_Object object);
/* 25 */
TCLAPI void		Tcl_ObjectSetMethodNameMapper(Tcl_Object object,
				Tcl_ObjectMapMethodNameProc *mapMethodNameProc);
/* 26 */
TCLAPI void		Tcl_ClassSetConstructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 27 */
TCLAPI void		Tcl_ClassSetDestructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 28 */
TCLAPI Tcl_Obj *	Tcl_GetObjectName(Tcl_Interp *interp,
				Tcl_Object object);
/* Slot 29 is reserved */
/* Slot 30 is reserved */
/* Slot 31 is reserved */
/* Slot 32 is reserved */
/* Slot 33 is reserved */
/* 34 */
TCLAPI void		TclOOUnusedStubEntry(void);

typedef struct {
    const struct TclOOIntStubs *tclOOIntStubs;
} TclOOStubHooks;

typedef struct TclOOStubs {
    int magic;
    const TclOOStubHooks *hooks;

    Tcl_Object (*tcl_CopyObjectInstance) (Tcl_Interp *interp, Tcl_Object sourceObject, const char *targetName, const char *targetNamespaceName); /* 0 */
    Tcl_Object (*tcl_GetClassAsObject) (Tcl_Class clazz); /* 1 */
    Tcl_Class (*tcl_GetObjectAsClass) (Tcl_Object object); /* 2 */
    Tcl_Command (*tcl_GetObjectCommand) (Tcl_Object object); /* 3 */
    Tcl_Object (*tcl_GetObjectFromObj) (Tcl_Interp *interp, Tcl_Obj *objPtr); /* 4 */
    Tcl_Namespace * (*tcl_GetObjectNamespace) (Tcl_Object object); /* 5 */
    Tcl_Class (*tcl_MethodDeclarerClass) (Tcl_Method method); /* 6 */
    Tcl_Object (*tcl_MethodDeclarerObject) (Tcl_Method method); /* 7 */
    int (*tcl_MethodIsPublic) (Tcl_Method method); /* 8 */
    int (*tcl_MethodIsType) (Tcl_Method method, const Tcl_MethodType *typePtr, void **clientDataPtr); /* 9 */
    Tcl_Obj * (*tcl_MethodName) (Tcl_Method method); /* 10 */
    Tcl_Method (*tcl_NewInstanceMethod) (Tcl_Interp *interp, Tcl_Object object, Tcl_Obj *nameObj, int isPublic, const Tcl_MethodType *typePtr, void *clientData); /* 11 */
    Tcl_Method (*tcl_NewMethod) (Tcl_Interp *interp, Tcl_Class cls, Tcl_Obj *nameObj, int isPublic, const Tcl_MethodType *typePtr, void *clientData); /* 12 */
    Tcl_Object (*tcl_NewObjectInstance) (Tcl_Interp *interp, Tcl_Class cls, const char *nameStr, const char *nsNameStr, int objc, Tcl_Obj *const *objv, int skip); /* 13 */
    int (*tcl_ObjectDeleted) (Tcl_Object object); /* 14 */
    int (*tcl_ObjectContextIsFiltering) (Tcl_ObjectContext context); /* 15 */
    Tcl_Method (*tcl_ObjectContextMethod) (Tcl_ObjectContext context); /* 16 */
    Tcl_Object (*tcl_ObjectContextObject) (Tcl_ObjectContext context); /* 17 */
    int (*tcl_ObjectContextSkippedArgs) (Tcl_ObjectContext context); /* 18 */
    void * (*tcl_ClassGetMetadata) (Tcl_Class clazz, const Tcl_ObjectMetadataType *typePtr); /* 19 */
    void (*tcl_ClassSetMetadata) (Tcl_Class clazz, const Tcl_ObjectMetadataType *typePtr, void *metadata); /* 20 */
    void * (*tcl_ObjectGetMetadata) (Tcl_Object object, const Tcl_ObjectMetadataType *typePtr); /* 21 */
    void (*tcl_ObjectSetMetadata) (Tcl_Object object, const Tcl_ObjectMetadataType *typePtr, void *metadata); /* 22 */
    int (*tcl_ObjectContextInvokeNext) (Tcl_Interp *interp, Tcl_ObjectContext context, int objc, Tcl_Obj *const *objv, int skip); /* 23 */
    Tcl_ObjectMapMethodNameProc * (*tcl_ObjectGetMethodNameMapper) (Tcl_Object object); /* 24 */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 204–256. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOODecls.h`.

```text
#define Tcl_MethodName \
	(tclOOStubsPtr->tcl_MethodName) /* 10 */
#define Tcl_NewInstanceMethod \
	(tclOOStubsPtr->tcl_NewInstanceMethod) /* 11 */
#define Tcl_NewMethod \
	(tclOOStubsPtr->tcl_NewMethod) /* 12 */
#define Tcl_NewObjectInstance \
	(tclOOStubsPtr->tcl_NewObjectInstance) /* 13 */
#define Tcl_ObjectDeleted \
	(tclOOStubsPtr->tcl_ObjectDeleted) /* 14 */
#define Tcl_ObjectContextIsFiltering \
	(tclOOStubsPtr->tcl_ObjectContextIsFiltering) /* 15 */
#define Tcl_ObjectContextMethod \
	(tclOOStubsPtr->tcl_ObjectContextMethod) /* 16 */
#define Tcl_ObjectContextObject \
	(tclOOStubsPtr->tcl_ObjectContextObject) /* 17 */
#define Tcl_ObjectContextSkippedArgs \
	(tclOOStubsPtr->tcl_ObjectContextSkippedArgs) /* 18 */
#define Tcl_ClassGetMetadata \
	(tclOOStubsPtr->tcl_ClassGetMetadata) /* 19 */
#define Tcl_ClassSetMetadata \
	(tclOOStubsPtr->tcl_ClassSetMetadata) /* 20 */
#define Tcl_ObjectGetMetadata \
	(tclOOStubsPtr->tcl_ObjectGetMetadata) /* 21 */
#define Tcl_ObjectSetMetadata \
	(tclOOStubsPtr->tcl_ObjectSetMetadata) /* 22 */
#define Tcl_ObjectContextInvokeNext \
	(tclOOStubsPtr->tcl_ObjectContextInvokeNext) /* 23 */
#define Tcl_ObjectGetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectGetMethodNameMapper) /* 24 */
#define Tcl_ObjectSetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectSetMethodNameMapper) /* 25 */
#define Tcl_ClassSetConstructor \
	(tclOOStubsPtr->tcl_ClassSetConstructor) /* 26 */
#define Tcl_ClassSetDestructor \
	(tclOOStubsPtr->tcl_ClassSetDestructor) /* 27 */
#define Tcl_GetObjectName \
	(tclOOStubsPtr->tcl_GetObjectName) /* 28 */
/* Slot 29 is reserved */
/* Slot 30 is reserved */
/* Slot 31 is reserved */
/* Slot 32 is reserved */
/* Slot 33 is reserved */
#define TclOOUnusedStubEntry \
	(tclOOStubsPtr->tclOOUnusedStubEntry) /* 34 */

#endif /* defined(USE_TCLOO_STUBS) */

/* !END!: Do not edit above this line. */

#undef TclOOUnusedStubEntry

#endif /* _TCLOODECLS */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODecls.h`, function `Tcl_GetObjectFromObj`, lines 39–137. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOODecls.h`.

```text
TCLAPI Tcl_Class	Tcl_GetObjectAsClass(Tcl_Object object);
/* 3 */
TCLAPI Tcl_Command	Tcl_GetObjectCommand(Tcl_Object object);
/* 4 */
TCLAPI Tcl_Object	Tcl_GetObjectFromObj(Tcl_Interp *interp,
				Tcl_Obj *objPtr);
/* 5 */
TCLAPI Tcl_Namespace *	Tcl_GetObjectNamespace(Tcl_Object object);
/* 6 */
TCLAPI Tcl_Class	Tcl_MethodDeclarerClass(Tcl_Method method);
/* 7 */
TCLAPI Tcl_Object	Tcl_MethodDeclarerObject(Tcl_Method method);
/* 8 */
TCLAPI int		Tcl_MethodIsPublic(Tcl_Method method);
/* 9 */
TCLAPI int		Tcl_MethodIsType(Tcl_Method method,
				const Tcl_MethodType *typePtr,
				void **clientDataPtr);
/* 10 */
TCLAPI Tcl_Obj *	Tcl_MethodName(Tcl_Method method);
/* 11 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int isPublic, const Tcl_MethodType *typePtr,
				void *clientData);
/* 12 */
TCLAPI Tcl_Method	Tcl_NewMethod(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int isPublic,
				const Tcl_MethodType *typePtr,
				void *clientData);
/* 13 */
TCLAPI Tcl_Object	Tcl_NewObjectInstance(Tcl_Interp *interp,
				Tcl_Class cls, const char *nameStr,
				const char *nsNameStr, int objc,
				Tcl_Obj *const *objv, int skip);
/* 14 */
TCLAPI int		Tcl_ObjectDeleted(Tcl_Object object);
/* 15 */
TCLAPI int		Tcl_ObjectContextIsFiltering(
				Tcl_ObjectContext context);
/* 16 */
TCLAPI Tcl_Method	Tcl_ObjectContextMethod(Tcl_ObjectContext context);
/* 17 */
TCLAPI Tcl_Object	Tcl_ObjectContextObject(Tcl_ObjectContext context);
/* 18 */
TCLAPI int		Tcl_ObjectContextSkippedArgs(
				Tcl_ObjectContext context);
/* 19 */
TCLAPI void *		Tcl_ClassGetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr);
/* 20 */
TCLAPI void		Tcl_ClassSetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 21 */
TCLAPI void *		Tcl_ObjectGetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr);
/* 22 */
TCLAPI void		Tcl_ObjectSetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 23 */
TCLAPI int		Tcl_ObjectContextInvokeNext(Tcl_Interp *interp,
				Tcl_ObjectContext context, int objc,
				Tcl_Obj *const *objv, int skip);
/* 24 */
TCLAPI Tcl_ObjectMapMethodNameProc * Tcl_ObjectGetMethodNameMapper(
				Tcl_Object object);
/* 25 */
TCLAPI void		Tcl_ObjectSetMethodNameMapper(Tcl_Object object,
				Tcl_ObjectMapMethodNameProc *mapMethodNameProc);
/* 26 */
TCLAPI void		Tcl_ClassSetConstructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 27 */
TCLAPI void		Tcl_ClassSetDestructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 28 */
TCLAPI Tcl_Obj *	Tcl_GetObjectName(Tcl_Interp *interp,
				Tcl_Object object);
/* Slot 29 is reserved */
/* Slot 30 is reserved */
/* Slot 31 is reserved */
/* Slot 32 is reserved */
/* Slot 33 is reserved */
/* 34 */
TCLAPI void		TclOOUnusedStubEntry(void);

typedef struct {
    const struct TclOOIntStubs *tclOOIntStubs;
} TclOOStubHooks;

typedef struct TclOOStubs {
    int magic;
    const TclOOStubHooks *hooks;

    Tcl_Object (*tcl_CopyObjectInstance) (Tcl_Interp *interp, Tcl_Object sourceObject, const char *targetName, const char *targetNamespaceName); /* 0 */
    Tcl_Object (*tcl_GetClassAsObject) (Tcl_Class clazz); /* 1 */
    Tcl_Class (*tcl_GetObjectAsClass) (Tcl_Object object); /* 2 */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODecls.h`, function `Tcl_GetObjectFromObj`, lines 188–256. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOODecls.h`.

```text
#define Tcl_GetObjectAsClass \
	(tclOOStubsPtr->tcl_GetObjectAsClass) /* 2 */
#define Tcl_GetObjectCommand \
	(tclOOStubsPtr->tcl_GetObjectCommand) /* 3 */
#define Tcl_GetObjectFromObj \
	(tclOOStubsPtr->tcl_GetObjectFromObj) /* 4 */
#define Tcl_GetObjectNamespace \
	(tclOOStubsPtr->tcl_GetObjectNamespace) /* 5 */
#define Tcl_MethodDeclarerClass \
	(tclOOStubsPtr->tcl_MethodDeclarerClass) /* 6 */
#define Tcl_MethodDeclarerObject \
	(tclOOStubsPtr->tcl_MethodDeclarerObject) /* 7 */
#define Tcl_MethodIsPublic \
	(tclOOStubsPtr->tcl_MethodIsPublic) /* 8 */
#define Tcl_MethodIsType \
	(tclOOStubsPtr->tcl_MethodIsType) /* 9 */
#define Tcl_MethodName \
	(tclOOStubsPtr->tcl_MethodName) /* 10 */
#define Tcl_NewInstanceMethod \
	(tclOOStubsPtr->tcl_NewInstanceMethod) /* 11 */
#define Tcl_NewMethod \
	(tclOOStubsPtr->tcl_NewMethod) /* 12 */
#define Tcl_NewObjectInstance \
	(tclOOStubsPtr->tcl_NewObjectInstance) /* 13 */
#define Tcl_ObjectDeleted \
	(tclOOStubsPtr->tcl_ObjectDeleted) /* 14 */
#define Tcl_ObjectContextIsFiltering \
	(tclOOStubsPtr->tcl_ObjectContextIsFiltering) /* 15 */
#define Tcl_ObjectContextMethod \
	(tclOOStubsPtr->tcl_ObjectContextMethod) /* 16 */
#define Tcl_ObjectContextObject \
	(tclOOStubsPtr->tcl_ObjectContextObject) /* 17 */
#define Tcl_ObjectContextSkippedArgs \
	(tclOOStubsPtr->tcl_ObjectContextSkippedArgs) /* 18 */
#define Tcl_ClassGetMetadata \
	(tclOOStubsPtr->tcl_ClassGetMetadata) /* 19 */
#define Tcl_ClassSetMetadata \
	(tclOOStubsPtr->tcl_ClassSetMetadata) /* 20 */
#define Tcl_ObjectGetMetadata \
	(tclOOStubsPtr->tcl_ObjectGetMetadata) /* 21 */
#define Tcl_ObjectSetMetadata \
	(tclOOStubsPtr->tcl_ObjectSetMetadata) /* 22 */
#define Tcl_ObjectContextInvokeNext \
	(tclOOStubsPtr->tcl_ObjectContextInvokeNext) /* 23 */
#define Tcl_ObjectGetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectGetMethodNameMapper) /* 24 */
#define Tcl_ObjectSetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectSetMethodNameMapper) /* 25 */
#define Tcl_ClassSetConstructor \
	(tclOOStubsPtr->tcl_ClassSetConstructor) /* 26 */
#define Tcl_ClassSetDestructor \
	(tclOOStubsPtr->tcl_ClassSetDestructor) /* 27 */
#define Tcl_GetObjectName \
	(tclOOStubsPtr->tcl_GetObjectName) /* 28 */
/* Slot 29 is reserved */
/* Slot 30 is reserved */
/* Slot 31 is reserved */
/* Slot 32 is reserved */
/* Slot 33 is reserved */
#define TclOOUnusedStubEntry \
	(tclOOStubsPtr->tclOOUnusedStubEntry) /* 34 */

#endif /* defined(USE_TCLOO_STUBS) */

/* !END!: Do not edit above this line. */

#undef TclOOUnusedStubEntry

#endif /* _TCLOODECLS */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODecls.h`, function `Tcl_GetObjectAsClass`, lines 35–133. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOODecls.h`.

```text
				const char *targetNamespaceName);
/* 1 */
TCLAPI Tcl_Object	Tcl_GetClassAsObject(Tcl_Class clazz);
/* 2 */
TCLAPI Tcl_Class	Tcl_GetObjectAsClass(Tcl_Object object);
/* 3 */
TCLAPI Tcl_Command	Tcl_GetObjectCommand(Tcl_Object object);
/* 4 */
TCLAPI Tcl_Object	Tcl_GetObjectFromObj(Tcl_Interp *interp,
				Tcl_Obj *objPtr);
/* 5 */
TCLAPI Tcl_Namespace *	Tcl_GetObjectNamespace(Tcl_Object object);
/* 6 */
TCLAPI Tcl_Class	Tcl_MethodDeclarerClass(Tcl_Method method);
/* 7 */
TCLAPI Tcl_Object	Tcl_MethodDeclarerObject(Tcl_Method method);
/* 8 */
TCLAPI int		Tcl_MethodIsPublic(Tcl_Method method);
/* 9 */
TCLAPI int		Tcl_MethodIsType(Tcl_Method method,
				const Tcl_MethodType *typePtr,
				void **clientDataPtr);
/* 10 */
TCLAPI Tcl_Obj *	Tcl_MethodName(Tcl_Method method);
/* 11 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int isPublic, const Tcl_MethodType *typePtr,
				void *clientData);
/* 12 */
TCLAPI Tcl_Method	Tcl_NewMethod(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int isPublic,
				const Tcl_MethodType *typePtr,
				void *clientData);
/* 13 */
TCLAPI Tcl_Object	Tcl_NewObjectInstance(Tcl_Interp *interp,
				Tcl_Class cls, const char *nameStr,
				const char *nsNameStr, int objc,
				Tcl_Obj *const *objv, int skip);
/* 14 */
TCLAPI int		Tcl_ObjectDeleted(Tcl_Object object);
/* 15 */
TCLAPI int		Tcl_ObjectContextIsFiltering(
				Tcl_ObjectContext context);
/* 16 */
TCLAPI Tcl_Method	Tcl_ObjectContextMethod(Tcl_ObjectContext context);
/* 17 */
TCLAPI Tcl_Object	Tcl_ObjectContextObject(Tcl_ObjectContext context);
/* 18 */
TCLAPI int		Tcl_ObjectContextSkippedArgs(
				Tcl_ObjectContext context);
/* 19 */
TCLAPI void *		Tcl_ClassGetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr);
/* 20 */
TCLAPI void		Tcl_ClassSetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 21 */
TCLAPI void *		Tcl_ObjectGetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr);
/* 22 */
TCLAPI void		Tcl_ObjectSetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 23 */
TCLAPI int		Tcl_ObjectContextInvokeNext(Tcl_Interp *interp,
				Tcl_ObjectContext context, int objc,
				Tcl_Obj *const *objv, int skip);
/* 24 */
TCLAPI Tcl_ObjectMapMethodNameProc * Tcl_ObjectGetMethodNameMapper(
				Tcl_Object object);
/* 25 */
TCLAPI void		Tcl_ObjectSetMethodNameMapper(Tcl_Object object,
				Tcl_ObjectMapMethodNameProc *mapMethodNameProc);
/* 26 */
TCLAPI void		Tcl_ClassSetConstructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 27 */
TCLAPI void		Tcl_ClassSetDestructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 28 */
TCLAPI Tcl_Obj *	Tcl_GetObjectName(Tcl_Interp *interp,
				Tcl_Object object);
/* Slot 29 is reserved */
/* Slot 30 is reserved */
/* Slot 31 is reserved */
/* Slot 32 is reserved */
/* Slot 33 is reserved */
/* 34 */
TCLAPI void		TclOOUnusedStubEntry(void);

typedef struct {
    const struct TclOOIntStubs *tclOOIntStubs;
} TclOOStubHooks;

typedef struct TclOOStubs {
    int magic;
    const TclOOStubHooks *hooks;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODecls.h`, function `Tcl_GetObjectAsClass`, lines 184–256. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOODecls.h`.

```text
#define Tcl_CopyObjectInstance \
	(tclOOStubsPtr->tcl_CopyObjectInstance) /* 0 */
#define Tcl_GetClassAsObject \
	(tclOOStubsPtr->tcl_GetClassAsObject) /* 1 */
#define Tcl_GetObjectAsClass \
	(tclOOStubsPtr->tcl_GetObjectAsClass) /* 2 */
#define Tcl_GetObjectCommand \
	(tclOOStubsPtr->tcl_GetObjectCommand) /* 3 */
#define Tcl_GetObjectFromObj \
	(tclOOStubsPtr->tcl_GetObjectFromObj) /* 4 */
#define Tcl_GetObjectNamespace \
	(tclOOStubsPtr->tcl_GetObjectNamespace) /* 5 */
#define Tcl_MethodDeclarerClass \
	(tclOOStubsPtr->tcl_MethodDeclarerClass) /* 6 */
#define Tcl_MethodDeclarerObject \
	(tclOOStubsPtr->tcl_MethodDeclarerObject) /* 7 */
#define Tcl_MethodIsPublic \
	(tclOOStubsPtr->tcl_MethodIsPublic) /* 8 */
#define Tcl_MethodIsType \
	(tclOOStubsPtr->tcl_MethodIsType) /* 9 */
#define Tcl_MethodName \
	(tclOOStubsPtr->tcl_MethodName) /* 10 */
#define Tcl_NewInstanceMethod \
	(tclOOStubsPtr->tcl_NewInstanceMethod) /* 11 */
#define Tcl_NewMethod \
	(tclOOStubsPtr->tcl_NewMethod) /* 12 */
#define Tcl_NewObjectInstance \
	(tclOOStubsPtr->tcl_NewObjectInstance) /* 13 */
#define Tcl_ObjectDeleted \
	(tclOOStubsPtr->tcl_ObjectDeleted) /* 14 */
#define Tcl_ObjectContextIsFiltering \
	(tclOOStubsPtr->tcl_ObjectContextIsFiltering) /* 15 */
#define Tcl_ObjectContextMethod \
	(tclOOStubsPtr->tcl_ObjectContextMethod) /* 16 */
#define Tcl_ObjectContextObject \
	(tclOOStubsPtr->tcl_ObjectContextObject) /* 17 */
#define Tcl_ObjectContextSkippedArgs \
	(tclOOStubsPtr->tcl_ObjectContextSkippedArgs) /* 18 */
#define Tcl_ClassGetMetadata \
	(tclOOStubsPtr->tcl_ClassGetMetadata) /* 19 */
#define Tcl_ClassSetMetadata \
	(tclOOStubsPtr->tcl_ClassSetMetadata) /* 20 */
#define Tcl_ObjectGetMetadata \
	(tclOOStubsPtr->tcl_ObjectGetMetadata) /* 21 */
#define Tcl_ObjectSetMetadata \
	(tclOOStubsPtr->tcl_ObjectSetMetadata) /* 22 */
#define Tcl_ObjectContextInvokeNext \
	(tclOOStubsPtr->tcl_ObjectContextInvokeNext) /* 23 */
#define Tcl_ObjectGetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectGetMethodNameMapper) /* 24 */
#define Tcl_ObjectSetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectSetMethodNameMapper) /* 25 */
#define Tcl_ClassSetConstructor \
	(tclOOStubsPtr->tcl_ClassSetConstructor) /* 26 */
#define Tcl_ClassSetDestructor \
	(tclOOStubsPtr->tcl_ClassSetDestructor) /* 27 */
#define Tcl_GetObjectName \
	(tclOOStubsPtr->tcl_GetObjectName) /* 28 */
/* Slot 29 is reserved */
/* Slot 30 is reserved */
/* Slot 31 is reserved */
/* Slot 32 is reserved */
/* Slot 33 is reserved */
#define TclOOUnusedStubEntry \
	(tclOOStubsPtr->tclOOUnusedStubEntry) /* 34 */

#endif /* defined(USE_TCLOO_STUBS) */

/* !END!: Do not edit above this line. */

#undef TclOOUnusedStubEntry

#endif /* _TCLOODECLS */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 63–160. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
static inline int	InitDefineContext(Tcl_Interp *interp,
			    Tcl_Namespace *namespacePtr, Object *oPtr,
			    int objc, Tcl_Obj *const objv[]);
static inline void	RecomputeClassCacheFlag(Object *oPtr);
static int		RenameDeleteMethod(Tcl_Interp *interp, Object *oPtr,
			    int useClass, Tcl_Obj *const fromPtr,
			    Tcl_Obj *const toPtr);
static int		ClassFilterGet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassFilterSet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassMixinGet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassMixinSet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassSuperGet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassSuperSet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassVarsGet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassVarsSet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjFilterGet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjFilterSet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjMixinGet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjMixinSet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjVarsGet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjVarsSet(ClientData clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);

/*
 * Now define the slots used in declarations.
 */

static const struct DeclaredSlot slots[] = {
    SLOT("define::filter",      ClassFilterGet, ClassFilterSet),
    SLOT("define::mixin",       ClassMixinGet,  ClassMixinSet),
    SLOT("define::superclass",  ClassSuperGet,  ClassSuperSet),
    SLOT("define::variable",    ClassVarsGet,   ClassVarsSet),
    SLOT("objdefine::filter",   ObjFilterGet,   ObjFilterSet),
    SLOT("objdefine::mixin",    ObjMixinGet,    ObjMixinSet),
    SLOT("objdefine::variable", ObjVarsGet,     ObjVarsSet),
    {NULL, {0, 0, 0, 0, 0}, {0, 0, 0, 0, 0}}
};

/*
 * ----------------------------------------------------------------------
 *
 * BumpGlobalEpoch --
 *	Utility that ensures that call chains that are invalid will get thrown
 *	away at an appropriate time. Note that exactly which epoch gets
 *	advanced will depend on exactly what the class is tangled up in; in
 *	the worst case, the simplest option is to advance the global epoch,
 *	causing *everything* to be thrown away on next usage.
 *
 * ----------------------------------------------------------------------
 */

static inline void
BumpGlobalEpoch(
    Tcl_Interp *interp,
    Class *classPtr)
{
    if (classPtr != NULL
	    && classPtr->subclasses.num == 0
	    && classPtr->instances.num == 0
	    && classPtr->mixinSubs.num == 0) {
	/*
	 * If a class has no subclasses or instances, and is not mixed into
	 * anything, a change to its structure does not require us to
	 * invalidate any call chains. Note that we still bump our object's
	 * epoch if it has any mixins; the relation between a class and its
	 * representative object is special. But it won't hurt.
	 */

	if (classPtr->thisPtr->mixins.num > 0) {
	    classPtr->thisPtr->epoch++;
	}

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 426–523. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text

/*
 * ----------------------------------------------------------------------
 *
 * RenameDeleteMethod --
 *	Core of the code to rename and delete methods.
 *
 * ----------------------------------------------------------------------
 */

static int
RenameDeleteMethod(
    Tcl_Interp *interp,
    Object *oPtr,
    int useClass,
    Tcl_Obj *const fromPtr,
    Tcl_Obj *const toPtr)
{
    Tcl_HashEntry *hPtr, *newHPtr = NULL;
    Method *mPtr;
    int isNew;

    if (!useClass) {
	if (!oPtr->methodsPtr) {
	noSuchMethod:
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "method %s does not exist", TclGetString(fromPtr)));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "METHOD",
		    TclGetString(fromPtr), (char *)NULL);
	    return TCL_ERROR;
	}
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, (char *) fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(oPtr->methodsPtr, (char *) toPtr,
		    &isNew);
	    if (hPtr == newHPtr) {
	    renameToSelf:
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"cannot rename method to itself", -1));
		Tcl_SetErrorCode(interp, "TCL", "OO", "RENAME_TO_SELF", (char *)NULL);
		return TCL_ERROR;
	    } else if (!isNew) {
	    renameToExisting:
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"method called %s already exists",
			TclGetString(toPtr)));
		Tcl_SetErrorCode(interp, "TCL", "OO", "RENAME_OVER", (char *)NULL);
		return TCL_ERROR;
	    }
	}
    } else {
	hPtr = Tcl_FindHashEntry(&oPtr->classPtr->classMethods,
		(char *) fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(&oPtr->classPtr->classMethods,
		    (char *) toPtr, &isNew);
	    if (hPtr == newHPtr) {
		goto renameToSelf;
	    } else if (!isNew) {
		goto renameToExisting;
	    }
	}
    }

    /*
     * Complete the splicing by changing the method's name.
     */

    mPtr = Tcl_GetHashValue(hPtr);
    if (toPtr) {
	Tcl_IncrRefCount(toPtr);
	Tcl_DecrRefCount(mPtr->namePtr);
	mPtr->namePtr = toPtr;
	Tcl_SetHashValue(newHPtr, mPtr);
    } else {
	if (!useClass) {
	    RecomputeClassCacheFlag(oPtr);
	}
	TclOODelMethodRef(mPtr);
    }
    Tcl_DeleteHashEntry(hPtr);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOUnknownDefinition --
 *	Handles what happens when an unknown command is encountered during the
 *	processing of a definition script. Works by finding a command in the
 *	operating definition namespace that the requested command is a unique
 *	prefix of.

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 433–530. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
 * ----------------------------------------------------------------------
 */

static int
RenameDeleteMethod(
    Tcl_Interp *interp,
    Object *oPtr,
    int useClass,
    Tcl_Obj *const fromPtr,
    Tcl_Obj *const toPtr)
{
    Tcl_HashEntry *hPtr, *newHPtr = NULL;
    Method *mPtr;
    int isNew;

    if (!useClass) {
	if (!oPtr->methodsPtr) {
	noSuchMethod:
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "method %s does not exist", TclGetString(fromPtr)));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "METHOD",
		    TclGetString(fromPtr), (char *)NULL);
	    return TCL_ERROR;
	}
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, (char *) fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(oPtr->methodsPtr, (char *) toPtr,
		    &isNew);
	    if (hPtr == newHPtr) {
	    renameToSelf:
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"cannot rename method to itself", -1));
		Tcl_SetErrorCode(interp, "TCL", "OO", "RENAME_TO_SELF", (char *)NULL);
		return TCL_ERROR;
	    } else if (!isNew) {
	    renameToExisting:
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"method called %s already exists",
			TclGetString(toPtr)));
		Tcl_SetErrorCode(interp, "TCL", "OO", "RENAME_OVER", (char *)NULL);
		return TCL_ERROR;
	    }
	}
    } else {
	hPtr = Tcl_FindHashEntry(&oPtr->classPtr->classMethods,
		(char *) fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(&oPtr->classPtr->classMethods,
		    (char *) toPtr, &isNew);
	    if (hPtr == newHPtr) {
		goto renameToSelf;
	    } else if (!isNew) {
		goto renameToExisting;
	    }
	}
    }

    /*
     * Complete the splicing by changing the method's name.
     */

    mPtr = Tcl_GetHashValue(hPtr);
    if (toPtr) {
	Tcl_IncrRefCount(toPtr);
	Tcl_DecrRefCount(mPtr->namePtr);
	mPtr->namePtr = toPtr;
	Tcl_SetHashValue(newHPtr, mPtr);
    } else {
	if (!useClass) {
	    RecomputeClassCacheFlag(oPtr);
	}
	TclOODelMethodRef(mPtr);
    }
    Tcl_DeleteHashEntry(hPtr);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOUnknownDefinition --
 *	Handles what happens when an unknown command is encountered during the
 *	processing of a definition script. Works by finding a command in the
 *	operating definition namespace that the requested command is a unique
 *	prefix of.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOUnknownDefinition(
    ClientData clientData,

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1262–1359. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDeleteMethodObjCmd --
 *	Implementation of the "deletemethod" subcommand of the "oo::define"
 *	and "oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDeleteMethodObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceDeleteMethod = (clientData != NULL);
    Object *oPtr;
    int i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	oPtr->epoch++;
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Class *clsPtr;
    Tcl_Method method;
    int bodyLength;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;

    Tcl_GetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1270–1367. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
 * ----------------------------------------------------------------------
 */

int
TclOODefineDeleteMethodObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceDeleteMethod = (clientData != NULL);
    Object *oPtr;
    int i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	oPtr->epoch++;
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Class *clsPtr;
    Tcl_Method method;
    int bodyLength;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;

    Tcl_GetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1276–1373. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceDeleteMethod = (clientData != NULL);
    Object *oPtr;
    int i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	oPtr->epoch++;
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Class *clsPtr;
    Tcl_Method method;
    int bodyLength;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;

    Tcl_GetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1289–1385. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	oPtr->epoch++;
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Class *clsPtr;
    Tcl_Method method;
    int bodyLength;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;

    Tcl_GetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a
     * destructor during a destructor is fairly dumb anyway.
     */

    Tcl_ClassSetDestructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1301–1397. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	oPtr->epoch++;
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Class *clsPtr;
    Tcl_Method method;
    int bodyLength;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;

    Tcl_GetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a
     * destructor during a destructor is fairly dumb anyway.
     */

    Tcl_ClassSetDestructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineExportObjCmd --
 *	Implementation of the "export" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineExportObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1307–1403. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	oPtr->epoch++;
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Class *clsPtr;
    Tcl_Method method;
    int bodyLength;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;

    Tcl_GetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a
     * destructor during a destructor is fairly dumb anyway.
     */

    Tcl_ClassSetDestructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineExportObjCmd --
 *	Implementation of the "export" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineExportObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceExport = (clientData != NULL);
    Object *oPtr;
    Method *mPtr;
    Tcl_HashEntry *hPtr;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1633–1730. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
     * get the entries from the appropriate hash tables (this can generate a
     * range of errors...)
     */

    if (RenameDeleteMethod(interp, oPtr, !isInstanceRenameMethod,
	    objv[1], objv[2]) != TCL_OK) {
	return TCL_ERROR;
    }

    if (isInstanceRenameMethod) {
	oPtr->epoch++;
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineUnexportObjCmd --
 *	Implementation of the "unexport" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineUnexportObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceUnexport = (clientData != NULL);
    Object *oPtr;
    Method *mPtr;
    Tcl_HashEntry *hPtr;
    Class *clsPtr;
    int i, isNew, changed = 0;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;
    if (!isInstanceUnexport && !clsPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Unexporting is done by removing the PUBLIC_METHOD flag from the
	 * method record. If there is no such method in this object or class
	 * (i.e. the method comes from something inherited from or that we're
	 * an instance of) then we put in a blank record without that flag;
	 * such records are skipped over by the call chain engine *except* for
	 * their flags member.
	 */

	if (isInstanceUnexport) {
	    if (!oPtr->methodsPtr) {
		oPtr->methodsPtr = ckalloc(sizeof(Tcl_HashTable));
		Tcl_InitObjHashTable(oPtr->methodsPtr);
		oPtr->flags &= ~USE_CLASS_CACHE;
	    }
	    hPtr = Tcl_CreateHashEntry(oPtr->methodsPtr, (char *) objv[i],
		    &isNew);
	} else {
	    hPtr = Tcl_CreateHashEntry(&clsPtr->classMethods, (char*) objv[i],
		    &isNew);
	}

	if (isNew) {
	    mPtr = ckalloc(sizeof(Method));
	    memset(mPtr, 0, sizeof(Method));
	    mPtr->refCount = 1;
	    mPtr->namePtr = objv[i];
	    Tcl_IncrRefCount(objv[i]);
	    Tcl_SetHashValue(hPtr, mPtr);
	} else {
	    mPtr = Tcl_GetHashValue(hPtr);
	}
	if (isNew || mPtr->flags & PUBLIC_METHOD) {
	    mPtr->flags &= ~PUBLIC_METHOD;
	    changed = 1;
	}
    }

    /*

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 179–274. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	Tcl_Free(callPtr->chain);
    }
    Tcl_Free(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *
 *	Saves a reference to a method call context in a Tcl_Obj's internal
 *	representation.
 *
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    Tcl_ObjInternalRep ir;

    callPtr->refCount++;
    TclGetString(objPtr);
    ir.twoPtrValue.ptr1 = callPtr;
    Tcl_StoreInternalRep(objPtr, &methodNameType, &ir);
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 210–304. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	Tcl_Free(callPtr->chain);
    }
    Tcl_Free(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *
 *	Saves a reference to a method call context in a Tcl_Obj's internal
 *	representation.
 *
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    Tcl_ObjInternalRep ir;

    callPtr->refCount++;
    TclGetString(objPtr);
    ir.twoPtrValue.ptr1 = callPtr;
    Tcl_StoreInternalRep(objPtr, &methodNameType, &ir);
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 228–323. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	Tcl_Free(callPtr->chain);
    }
    Tcl_Free(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *
 *	Saves a reference to a method call context in a Tcl_Obj's internal
 *	representation.
 *
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    Tcl_ObjInternalRep ir;

    callPtr->refCount++;
    TclGetString(objPtr);
    ir.twoPtrValue.ptr1 = callPtr;
    Tcl_StoreInternalRep(objPtr, &methodNameType, &ir);
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 296–393. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL,NULL,NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags,contextPtr,NULL,NULL,NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

    /*
     * Run the method implementation.
     */

    if (mPtr->typePtr->version < TCL_OO_METHOD_VERSION_2) {
	return (mPtr->typePtr->callProc)(mPtr->clientData, interp,
		(Tcl_ObjectContext) contextPtr, objc, objv);
    }
    return (mPtr->type2Ptr->callProc)(mPtr->clientData, interp,
	    (Tcl_ObjectContext) contextPtr, objc, objv);
}

static int
SetFilterFlags(
    void *data[],
    TCL_UNUSED(Tcl_Interp *),

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1258–1356. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
		callPtr->refCount++;
		goto returnContext;
	    }
	    Tcl_SetHashValue(hPtr, NULL);
	    TclOODeleteChain(callPtr);
	}

	doFilters = 1;
    }

    callPtr = (CallChain *) Tcl_Alloc(sizeof(CallChain));
    InitCallChain(callPtr, oPtr, flags);

    cb.callChainPtr = callPtr;
    cb.filterLength = 0;
    cb.oPtr = oPtr;

    /*
     * If we're working with a forced use of unknown, do that now.
     */

    if (flags & FORCE_UNKNOWN) {
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, BUILDING_MIXINS,
		NULL);
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (callPtr->numChain == 0) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	goto returnContext;
    }

    /*
     * Add all defined filters (if any, and if we're going to be processing
     * them; they're not processed for constructors, destructors or when we're
     * in the middle of processing a filter).
     */

    if (doFilters) {
	Tcl_Obj *filterObj;
	Class *mixinPtr;

	doFilters = 1;
	Tcl_InitObjHashTable(&doneFilters);
	FOREACH(mixinPtr, oPtr->mixins) {
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    TRAVERSED_MIXIN|BUILDING_MIXINS|OBJECT_MIXIN);
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    OBJECT_MIXIN);
	}
	FOREACH(filterObj, oPtr->filters) {
	    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
		    filterObj, &cb, &doneFilters, BUILDING_MIXINS, NULL);
	    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
		    filterObj, &cb, &doneFilters, 0, NULL);
	}
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		BUILDING_MIXINS);
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		0);
	Tcl_DeleteHashTable(&doneFilters);
    }
    count = cb.filterLength = callPtr->numChain;

    /*
     * Add the actual method implementations. We have to do this twice to
     * handle class mixins right.
     */

    if (oPtr == contextObj) {
	donePrivate |= AddInstancePrivateToCallContext(oPtr, methodNameObj,
		&cb, flags);
	donePrivate |= (contextObj->flags & HAS_PRIVATE_METHODS);
    }
    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
	    methodNameObj, &cb, NULL, flags|BUILDING_MIXINS, NULL);
    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
	    methodNameObj, &cb, NULL, flags, NULL);

    /*
     * Check to see if the method has no implementation. If so, we probably
     * need to add in a call to the unknown method. Otherwise, set up the
     * cacheing of the method implementation (if relevant).
     */

    if (count == callPtr->numChain) {
	/*
	 * Method does not actually exist. If we're dealing with constructors
	 * or destructors, this isn't a problem.
	 */

	if (flags & SPECIAL) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1284–1382. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (callPtr->numChain == 0) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	goto returnContext;
    }

    /*
     * Add all defined filters (if any, and if we're going to be processing
     * them; they're not processed for constructors, destructors or when we're
     * in the middle of processing a filter).
     */

    if (doFilters) {
	Tcl_Obj *filterObj;
	Class *mixinPtr;

	doFilters = 1;
	Tcl_InitObjHashTable(&doneFilters);
	FOREACH(mixinPtr, oPtr->mixins) {
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    TRAVERSED_MIXIN|BUILDING_MIXINS|OBJECT_MIXIN);
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    OBJECT_MIXIN);
	}
	FOREACH(filterObj, oPtr->filters) {
	    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
		    filterObj, &cb, &doneFilters, BUILDING_MIXINS, NULL);
	    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
		    filterObj, &cb, &doneFilters, 0, NULL);
	}
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		BUILDING_MIXINS);
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		0);
	Tcl_DeleteHashTable(&doneFilters);
    }
    count = cb.filterLength = callPtr->numChain;

    /*
     * Add the actual method implementations. We have to do this twice to
     * handle class mixins right.
     */

    if (oPtr == contextObj) {
	donePrivate |= AddInstancePrivateToCallContext(oPtr, methodNameObj,
		&cb, flags);
	donePrivate |= (contextObj->flags & HAS_PRIVATE_METHODS);
    }
    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
	    methodNameObj, &cb, NULL, flags|BUILDING_MIXINS, NULL);
    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
	    methodNameObj, &cb, NULL, flags, NULL);

    /*
     * Check to see if the method has no implementation. If so, we probably
     * need to add in a call to the unknown method. Otherwise, set up the
     * cacheing of the method implementation (if relevant).
     */

    if (count == callPtr->numChain) {
	/*
	 * Method does not actually exist. If we're dealing with constructors
	 * or destructors, this isn't a problem.
	 */

	if (flags & SPECIAL) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, BUILDING_MIXINS,
		NULL);
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else if (doFilters && !donePrivate) {
	if (hPtr == NULL) {
	    int isNew;
	    if (oPtr->flags & USE_CLASS_CACHE) {
		if (oPtr->selfCls->classChainCache == NULL) {
		    oPtr->selfCls->classChainCache = (Tcl_HashTable *)
			    Tcl_Alloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->selfCls->classChainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->selfCls->classChainCache,
			methodNameObj, &isNew);
	    } else {
		if (oPtr->chainCache == NULL) {
		    oPtr->chainCache = (Tcl_HashTable *)

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1350–1447. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
	 * or destructors, this isn't a problem.
	 */

	if (flags & SPECIAL) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, BUILDING_MIXINS,
		NULL);
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else if (doFilters && !donePrivate) {
	if (hPtr == NULL) {
	    int isNew;
	    if (oPtr->flags & USE_CLASS_CACHE) {
		if (oPtr->selfCls->classChainCache == NULL) {
		    oPtr->selfCls->classChainCache = (Tcl_HashTable *)
			    Tcl_Alloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->selfCls->classChainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->selfCls->classChainCache,
			methodNameObj, &isNew);
	    } else {
		if (oPtr->chainCache == NULL) {
		    oPtr->chainCache = (Tcl_HashTable *)
			    Tcl_Alloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->chainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->chainCache,
			methodNameObj, &isNew);
	    }
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)
	    TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1361–1458. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else if (doFilters && !donePrivate) {
	if (hPtr == NULL) {
	    int isNew;
	    if (oPtr->flags & USE_CLASS_CACHE) {
		if (oPtr->selfCls->classChainCache == NULL) {
		    oPtr->selfCls->classChainCache = (Tcl_HashTable *)
			    Tcl_Alloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->selfCls->classChainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->selfCls->classChainCache,
			methodNameObj, &isNew);
	    } else {
		if (oPtr->chainCache == NULL) {
		    oPtr->chainCache = (Tcl_HashTable *)
			    Tcl_Alloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->chainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->chainCache,
			methodNameObj, &isNew);
	    }
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)
	    TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    ChainBuilder cb;
    Tcl_Size count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1392–1489. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)
	    TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    ChainBuilder cb;
    Tcl_Size count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply
     * can't get stereotypes. [Bug 7842f33a5c]
     */

    if (clsPtr == NULL) {
	return NULL;
    }

    /*
     * Synthesize a temporary stereotypical object so that we can use existing
     * machinery to produce the stereotypical call chain.
     */

    memset(&obj, 0, sizeof(Object));
    obj.fPtr = fPtr;
    obj.selfCls = clsPtr;
    obj.refCount = 1;
    obj.flags = USE_CLASS_CACHE;

    /*
     * Check if we can get the chain out of the Tcl_Obj method name or out of
     * the cache. This is made a bit more complex by the fact that there are
     * multiple different layers of cache (in the Tcl_Obj, in the object, and
     * in the class).
     */

    if (clsPtr->classChainCache != NULL) {
	hPtr = Tcl_FindHashEntry(clsPtr->classChainCache,
		methodNameObj);
	if (hPtr != NULL && Tcl_GetHashValue(hPtr) != NULL) {
	    const int reuseMask = (WANT_PUBLIC(flags) ? ~0 : ~PUBLIC_METHOD);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1398–1495. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)
	    TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    ChainBuilder cb;
    Tcl_Size count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply
     * can't get stereotypes. [Bug 7842f33a5c]
     */

    if (clsPtr == NULL) {
	return NULL;
    }

    /*
     * Synthesize a temporary stereotypical object so that we can use existing
     * machinery to produce the stereotypical call chain.
     */

    memset(&obj, 0, sizeof(Object));
    obj.fPtr = fPtr;
    obj.selfCls = clsPtr;
    obj.refCount = 1;
    obj.flags = USE_CLASS_CACHE;

    /*
     * Check if we can get the chain out of the Tcl_Obj method name or out of
     * the cache. This is made a bit more complex by the fact that there are
     * multiple different layers of cache (in the Tcl_Obj, in the object, and
     * in the class).
     */

    if (clsPtr->classChainCache != NULL) {
	hPtr = Tcl_FindHashEntry(clsPtr->classChainCache,
		methodNameObj);
	if (hPtr != NULL && Tcl_GetHashValue(hPtr) != NULL) {
	    const int reuseMask = (WANT_PUBLIC(flags) ? ~0 : ~PUBLIC_METHOD);

	    callPtr = (CallChain *) Tcl_GetHashValue(hPtr);
	    if (IsStillValid(callPtr, &obj, flags, reuseMask)) {
		callPtr->refCount++;
		return callPtr;
	    }
	    Tcl_SetHashValue(hPtr, NULL);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1492–1589. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
		callPtr->refCount++;
		return callPtr;
	    }
	    Tcl_SetHashValue(hPtr, NULL);
	    TclOODeleteChain(callPtr);
	}
    } else {
	hPtr = NULL;
    }

    callPtr = (CallChain *) Tcl_Alloc(sizeof(CallChain));
    memset(callPtr, 0, sizeof(CallChain));
    callPtr->flags = flags & (PUBLIC_METHOD|PRIVATE_METHOD|FILTER_HANDLING);
    callPtr->epoch = fPtr->epoch;
    callPtr->objectCreationEpoch = fPtr->tsdPtr->nsCount;
    callPtr->objectEpoch = clsPtr->thisPtr->epoch;
    callPtr->refCount = 1;
    callPtr->chain = callPtr->staticChain;

    cb.callChainPtr = callPtr;
    cb.filterLength = 0;
    cb.oPtr = &obj;

    /*
     * Add all defined filters (if any, and if we're going to be processing
     * them; they're not processed for constructors, destructors or when we're
     * in the middle of processing a filter).
     */

    Tcl_InitObjHashTable(&doneFilters);
    AddClassFiltersToCallContext(&obj, clsPtr, &cb, &doneFilters,
	    BUILDING_MIXINS);
    AddClassFiltersToCallContext(&obj, clsPtr, &cb, &doneFilters, 0);
    Tcl_DeleteHashTable(&doneFilters);
    count = cb.filterLength = callPtr->numChain;

    /*
     * Add the actual method implementations.
     */

    AddSimpleChainToCallContext(&obj, NULL, methodNameObj, &cb, NULL,
	    flags|BUILDING_MIXINS, NULL);
    AddSimpleChainToCallContext(&obj, NULL, methodNameObj, &cb, NULL, flags,
	    NULL);

    /*
     * Check to see if the method has no implementation. If so, we probably
     * need to add in a call to the unknown method. Otherwise, set up the
     * caching of the method implementation (if relevant).
     */

    if (count == callPtr->numChain) {
	AddSimpleChainToCallContext(&obj, NULL, fPtr->unknownMethodNameObj,
		&cb, NULL, BUILDING_MIXINS, NULL);
	AddSimpleChainToCallContext(&obj, NULL, fPtr->unknownMethodNameObj,
		&cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else {
	if (hPtr == NULL) {
	    int isNew;
	    if (clsPtr->classChainCache == NULL) {
		clsPtr->classChainCache = (Tcl_HashTable *)
			Tcl_Alloc(sizeof(Tcl_HashTable));
		Tcl_InitObjHashTable(clsPtr->classChainCache);
	    }
	    hPtr = Tcl_CreateHashEntry(clsPtr->classChainCache,
		    methodNameObj, &isNew);
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(methodNameObj, callPtr);
    }
    return callPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddClassFiltersToCallContext --
 *
 *	Logic to make extracting all the filters from the class context much
 *	easier.
 *
 * ----------------------------------------------------------------------
 */

static void
AddClassFiltersToCallContext(
    Object *const oPtr,		/* Object that the filters operate on. */
    Class *clsPtr,		/* Class to get the filters from. */
    ChainBuilder *const cbPtr,	/* Context to fill with call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1547–1644. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
		&cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else {
	if (hPtr == NULL) {
	    int isNew;
	    if (clsPtr->classChainCache == NULL) {
		clsPtr->classChainCache = (Tcl_HashTable *)
			Tcl_Alloc(sizeof(Tcl_HashTable));
		Tcl_InitObjHashTable(clsPtr->classChainCache);
	    }
	    hPtr = Tcl_CreateHashEntry(clsPtr->classChainCache,
		    methodNameObj, &isNew);
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(methodNameObj, callPtr);
    }
    return callPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddClassFiltersToCallContext --
 *
 *	Logic to make extracting all the filters from the class context much
 *	easier.
 *
 * ----------------------------------------------------------------------
 */

static void
AddClassFiltersToCallContext(
    Object *const oPtr,		/* Object that the filters operate on. */
    Class *clsPtr,		/* Class to get the filters from. */
    ChainBuilder *const cbPtr,	/* Context to fill with call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. Keys are objects, values are
				 * ignored. */
    int flags)			/* Whether we've gone along a mixin link
				 * yet. */
{
    Tcl_Size i;
    int clearedFlags =
	    flags & ~(TRAVERSED_MIXIN|OBJECT_MIXIN|BUILDING_MIXINS);
    Class *superPtr, *mixinPtr;
    Tcl_Obj *filterObj;

  tailRecurse:
    if (clsPtr == NULL) {
	return;
    }

    /*
     * Add all the filters defined by classes mixed into the main class
     * hierarchy.
     */

    FOREACH(mixinPtr, clsPtr->mixins) {
	AddClassFiltersToCallContext(oPtr, mixinPtr, cbPtr, doneFilters,
		flags|TRAVERSED_MIXIN);
    }

    /*
     * Add all the class filters from the current class. Note that the filters
     * are added starting at the object root, as this allows the object to
     * override how filters work to extend their behaviour.
     */

    if (MIXIN_CONSISTENT(flags)) {
	FOREACH(filterObj, clsPtr->filters) {
	    int isNew;

	    (void) Tcl_CreateHashEntry(doneFilters, filterObj, &isNew);
	    if (isNew) {
		AddSimpleChainToCallContext(oPtr, NULL, filterObj, cbPtr,
			doneFilters, clearedFlags|BUILDING_MIXINS, clsPtr);
		AddSimpleChainToCallContext(oPtr, NULL, filterObj, cbPtr,
			doneFilters, clearedFlags, clsPtr);
	    }
	}
    }

    /*
     * Now process the recursive case. Notice the tail-call optimization.
     */

    switch (clsPtr->superclasses.num) {
    case 1:
	clsPtr = clsPtr->superclasses.list[0];
	goto tailRecurse;
    default:

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `StashCallChain(`, lines 148–243. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static size_t		SortMethodNames(Tcl_HashTable *namesPtr, int flags,
			    const char ***stringsPtr);
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */

static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,
    NULL,
    TCL_OBJTYPE_V0
};

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	Tcl_Free(callPtr->chain);
    }
    Tcl_Free(callPtr);
}


```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `StashCallChain(`, lines 252–348. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    Tcl_ObjInternalRep ir;

    callPtr->refCount++;
    TclGetString(objPtr);
    ir.twoPtrValue.ptr1 = callPtr;
    Tcl_StoreInternalRep(objPtr, &methodNameType, &ir);
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `StashCallChain(`, lines 269–365. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `StashCallChain(`, lines 288–385. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL,NULL,NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags,contextPtr,NULL,NULL,NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

    /*
     * Run the method implementation.
     */

    if (mPtr->typePtr->version < TCL_OO_METHOD_VERSION_2) {
	return (mPtr->typePtr->callProc)(mPtr->clientData, interp,
		(Tcl_ObjectContext) contextPtr, objc, objv);
    }

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `StashCallChain(`, lines 1389–1486. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
	    }
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)
	    TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    ChainBuilder cb;
    Tcl_Size count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply
     * can't get stereotypes. [Bug 7842f33a5c]
     */

    if (clsPtr == NULL) {
	return NULL;
    }

    /*
     * Synthesize a temporary stereotypical object so that we can use existing
     * machinery to produce the stereotypical call chain.
     */

    memset(&obj, 0, sizeof(Object));
    obj.fPtr = fPtr;
    obj.selfCls = clsPtr;
    obj.refCount = 1;
    obj.flags = USE_CLASS_CACHE;

    /*
     * Check if we can get the chain out of the Tcl_Obj method name or out of
     * the cache. This is made a bit more complex by the fact that there are
     * multiple different layers of cache (in the Tcl_Obj, in the object, and
     * in the class).
     */

    if (clsPtr->classChainCache != NULL) {
	hPtr = Tcl_FindHashEntry(clsPtr->classChainCache,
		methodNameObj);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `StashCallChain(`, lines 1563–1659. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
		    methodNameObj, &isNew);
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(methodNameObj, callPtr);
    }
    return callPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddClassFiltersToCallContext --
 *
 *	Logic to make extracting all the filters from the class context much
 *	easier.
 *
 * ----------------------------------------------------------------------
 */

static void
AddClassFiltersToCallContext(
    Object *const oPtr,		/* Object that the filters operate on. */
    Class *clsPtr,		/* Class to get the filters from. */
    ChainBuilder *const cbPtr,	/* Context to fill with call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. Keys are objects, values are
				 * ignored. */
    int flags)			/* Whether we've gone along a mixin link
				 * yet. */
{
    Tcl_Size i;
    int clearedFlags =
	    flags & ~(TRAVERSED_MIXIN|OBJECT_MIXIN|BUILDING_MIXINS);
    Class *superPtr, *mixinPtr;
    Tcl_Obj *filterObj;

  tailRecurse:
    if (clsPtr == NULL) {
	return;
    }

    /*
     * Add all the filters defined by classes mixed into the main class
     * hierarchy.
     */

    FOREACH(mixinPtr, clsPtr->mixins) {
	AddClassFiltersToCallContext(oPtr, mixinPtr, cbPtr, doneFilters,
		flags|TRAVERSED_MIXIN);
    }

    /*
     * Add all the class filters from the current class. Note that the filters
     * are added starting at the object root, as this allows the object to
     * override how filters work to extend their behaviour.
     */

    if (MIXIN_CONSISTENT(flags)) {
	FOREACH(filterObj, clsPtr->filters) {
	    int isNew;

	    (void) Tcl_CreateHashEntry(doneFilters, filterObj, &isNew);
	    if (isNew) {
		AddSimpleChainToCallContext(oPtr, NULL, filterObj, cbPtr,
			doneFilters, clearedFlags|BUILDING_MIXINS, clsPtr);
		AddSimpleChainToCallContext(oPtr, NULL, filterObj, cbPtr,
			doneFilters, clearedFlags, clsPtr);
	    }
	}
    }

    /*
     * Now process the recursive case. Notice the tail-call optimization.
     */

    switch (clsPtr->superclasses.num) {
    case 1:
	clsPtr = clsPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, clsPtr->superclasses) {
	    AddClassFiltersToCallContext(oPtr, superPtr, cbPtr, doneFilters,
		    flags);
	}
	TCL_FALLTHROUGH();
    case 0:
	return;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * AddPrivatesFromClassChainToCallContext --
 *

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `FreeMethodNameRep(`, lines 141–236. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
			    DefineChain *const definePtr, int flags);
static int		CmpStr(const void *ptr1, const void *ptr2);
static void		DupMethodNameRep(Tcl_Obj *srcPtr, Tcl_Obj *dstPtr);
static Tcl_NRPostProc	FinalizeMethodRefs;
static void		FreeMethodNameRep(Tcl_Obj *objPtr);
static inline int	IsStillValid(CallChain *callPtr, Object *oPtr,
			    int flags, int reuseMask);
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static size_t		SortMethodNames(Tcl_HashTable *namesPtr, int flags,
			    const char ***stringsPtr);
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */

static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,
    NULL,
    TCL_OBJTYPE_V0
};

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `FreeMethodNameRep(`, lines 293–390. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL,NULL,NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags,contextPtr,NULL,NULL,NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

    /*
     * Run the method implementation.
     */

    if (mPtr->typePtr->version < TCL_OO_METHOD_VERSION_2) {
	return (mPtr->typePtr->callProc)(mPtr->clientData, interp,
		(Tcl_ObjectContext) contextPtr, objc, objv);
    }
    return (mPtr->type2Ptr->callProc)(mPtr->clientData, interp,
	    (Tcl_ObjectContext) contextPtr, objc, objv);
}

static int

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `DupMethodNameRep(`, lines 139–234. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
			    DefineChain *const definePtr, int flags);
static inline void	AddSimpleDefineNamespaces(Object *const oPtr,
			    DefineChain *const definePtr, int flags);
static int		CmpStr(const void *ptr1, const void *ptr2);
static void		DupMethodNameRep(Tcl_Obj *srcPtr, Tcl_Obj *dstPtr);
static Tcl_NRPostProc	FinalizeMethodRefs;
static void		FreeMethodNameRep(Tcl_Obj *objPtr);
static inline int	IsStillValid(CallChain *callPtr, Object *oPtr,
			    int flags, int reuseMask);
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static size_t		SortMethodNames(Tcl_HashTable *namesPtr, int flags,
			    const char ***stringsPtr);
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */

static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,
    NULL,
    TCL_OBJTYPE_V0
};

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `DupMethodNameRep(`, lines 284–381. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL,NULL,NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags,contextPtr,NULL,NULL,NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

    /*
     * Run the method implementation.
     */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `TclOOInvokeContext(`, lines 314–412. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    int objc,			/* The number of arguments. */
    Tcl_Obj *const objv[])	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const int isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL,NULL,NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags,contextPtr,NULL,NULL,NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

    /*
     * Run the method implementation.
     */

    if (mPtr->typePtr->version < TCL_OO_METHOD_VERSION_2) {
	return (mPtr->typePtr->callProc)(mPtr->clientData, interp,
		(Tcl_ObjectContext) contextPtr, objc, objv);
    }
    return (mPtr->type2Ptr->callProc)(mPtr->clientData, interp,
	    (Tcl_ObjectContext) contextPtr, objc, objv);
}

static int
SetFilterFlags(
    void *data[],
    TCL_UNUSED(Tcl_Interp *),
    int result)
{
    CallContext *contextPtr = (CallContext *) data[0];

    contextPtr->oPtr->flags |= FILTER_HANDLING;
    return result;
}

static int
ResetFilterFlags(
    void *data[],
    TCL_UNUSED(Tcl_Interp *),
    int result)
{
    CallContext *contextPtr = (CallContext *) data[0];

    contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    return result;
}

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `FinalizeMethodRefs(`, lines 411–508. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
    return result;
}

static int
FinalizeMethodRefs(
    void *data[],
    TCL_UNUSED(Tcl_Interp *),
    int result)
{
    CallContext *contextPtr = (CallContext *) data[0];
    Tcl_Size i;

    for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	TclOODelMethodRef(contextPtr->callPtr->chain[i].mPtr);
    }
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetSortedMethodList, TclOOGetSortedClassMethodList --
 *
 *	Discovers the list of method names supported by an object or class.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOGetSortedMethodList(
    Object *oPtr,		/* The object to get the method names for. */
    Object *contextObj,		/* From what context object we are inquiring.
				 * NULL when the context shouldn't see
				 * object-level private methods. Note that
				 * flags can override this. */
    Class *contextCls,		/* From what context class we are inquiring.
				 * NULL when the context shouldn't see
				 * class-level private methods. Note that
				 * flags can override this. */
    int flags,			/* Whether we just want the public method
				 * names. */
    const char ***stringsPtr)	/* Where to write a pointer to the array of
				 * strings to. */
{
    Tcl_HashTable names;	/* Tcl_Obj* method name to "wanted in list"
				 * mapping. */
    Tcl_HashTable examinedClasses;
				/* Used to track what classes have been looked
				 * at. Is set-like in nature and keyed by
				 * pointer to class. */
    FOREACH_HASH_DECLS;
    Tcl_Size i, numStrings;
    Class *mixinPtr;
    Tcl_Obj *namePtr;
    Method *mPtr;

    Tcl_InitObjHashTable(&names);
    Tcl_InitHashTable(&examinedClasses, TCL_ONE_WORD_KEYS);

    /*
     * Process method names due to the object.
     */

    if (oPtr->methodsPtr) {
	FOREACH_HASH(namePtr, mPtr, oPtr->methodsPtr) {
	    if (IS_PRIVATE(mPtr)) {
		continue;
	    }
	    if (IS_UNEXPORTED(mPtr) && !WANT_UNEXPORTED(flags)) {
		continue;
	    }
	    AddStandardMethodName(flags, namePtr, mPtr, &names);
	}
    }

    /*
     * Process method names due to private methods on the object's class.
     */

    if (WANT_UNEXPORTED(flags)) {
	FOREACH_HASH(namePtr, mPtr, &oPtr->selfCls->classMethods) {
	    if (IS_UNEXPORTED(mPtr)) {
		AddStandardMethodName(flags, namePtr, mPtr, &names);
	    }
	}
    }

    /*
     * Process method names due to private methods on the context's object or
     * class. Which must be correct if either are not NULL.
     */

    if (contextObj && contextObj->methodsPtr) {
	AddPrivateMethodNames(contextObj->methodsPtr, &names);
    }
    if (contextCls) {
	AddPrivateMethodNames(&contextCls->classMethods, &names);
    }

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 106–202. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
			    Tcl_HashTable *const examinedClassesPtr);
static inline void	AddDefinitionNamespaceToChain(Class *const definerCls,
			    Tcl_Obj *const namespaceName,
			    DefineChain *const definePtr, int flags);
static inline void	AddMethodToCallChain(Method *const mPtr,
			    ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters,
			    Class *const filterDecl, int flags);
static inline int	AddInstancePrivateToCallContext(Object *const oPtr,
			    Tcl_Obj *const methodNameObj,
			    ChainBuilder *const cbPtr, int flags);
static inline void	AddStandardMethodName(int flags, Tcl_Obj *namePtr,
			    Method *mPtr, Tcl_HashTable *namesPtr);
static inline void	AddPrivateMethodNames(Tcl_HashTable *methodsTablePtr,
			    Tcl_HashTable *namesPtr);
static inline int	AddSimpleChainToCallContext(Object *const oPtr,
			    Class *const contextCls,
			    Tcl_Obj *const methodNameObj,
			    ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters, int flags,
			    Class *const filterDecl);
static int		AddPrivatesFromClassChainToCallContext(Class *classPtr,
			    Class *const contextCls,
			    Tcl_Obj *const methodNameObj,
			    ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters, int flags,
			    Class *const filterDecl);
static int		AddSimpleClassChainToCallContext(Class *classPtr,
			    Tcl_Obj *const methodNameObj,
			    ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters, int flags,
			    Class *const filterDecl);
static void		AddSimpleClassDefineNamespaces(Class *classPtr,
			    DefineChain *const definePtr, int flags);
static inline void	AddSimpleDefineNamespaces(Object *const oPtr,
			    DefineChain *const definePtr, int flags);
static int		CmpStr(const void *ptr1, const void *ptr2);
static void		DupMethodNameRep(Tcl_Obj *srcPtr, Tcl_Obj *dstPtr);
static Tcl_NRPostProc	FinalizeMethodRefs;
static void		FreeMethodNameRep(Tcl_Obj *objPtr);
static inline int	IsStillValid(CallChain *callPtr, Object *oPtr,
			    int flags, int reuseMask);
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static size_t		SortMethodNames(Tcl_HashTable *namesPtr, int flags,
			    const char ***stringsPtr);
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */

static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,
    NULL,
    TCL_OBJTYPE_V0
};

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 840–937. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodName);
	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (IS_PRIVATE(mPtr)) {
		AddMethodToCallChain(mPtr, cbPtr, NULL, NULL, flags);
		donePrivate = 1;
	    }
	}
    }
    return donePrivate;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddSimpleChainToCallContext --
 *
 *	The core of the call-chain construction engine, this handles calling a
 *	particular method on a particular object. Note that filters and
 *	unknown handling are already handled by the logic that uses this
 *	function. Returns true if a private method was one of those found.
 *
 * ----------------------------------------------------------------------
 */

static inline int
AddSimpleChainToCallContext(
    Object *const oPtr,		/* Object to add call chain entries for. */
    Class *const contextCls,	/* Context class; the currently considered
				 * class is equal to this, private methods may
				 * also be added. [TIP 500] */
    Tcl_Obj *const methodNameObj,
				/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what call chain entries
				 * have been processed. */
    int flags,			/* What sort of call chain are we building. */
    Class *const filterDecl)	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
{
    Tcl_Size i;
    int foundPrivate = 0, blockedUnexported = 0;
    Tcl_HashEntry *hPtr;
    Method *mPtr;

    if (!(flags & (KNOWN_STATE | SPECIAL)) && oPtr->methodsPtr) {
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);

	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (!IS_PRIVATE(mPtr)) {
		if (WANT_PUBLIC(flags)) {
		    if (!IS_PUBLIC(mPtr)) {
			blockedUnexported = 1;
		    } else {
			flags |= DEFINITE_PUBLIC;
		    }
		} else {
		    flags |= DEFINITE_PROTECTED;
		}
	    }
	}
    }
    if (!(flags & SPECIAL)) {
	Class *mixinPtr;

	FOREACH(mixinPtr, oPtr->mixins) {
	    if (contextCls) {
		foundPrivate |= AddPrivatesFromClassChainToCallContext(
			mixinPtr, contextCls, methodNameObj, cbPtr,
			doneFilters, flags|TRAVERSED_MIXIN, filterDecl);
	    }
	    foundPrivate |= AddSimpleClassChainToCallContext(mixinPtr,
		    methodNameObj, cbPtr, doneFilters,
		    flags | TRAVERSED_MIXIN, filterDecl);
	}
	if (oPtr->methodsPtr && !blockedUnexported) {
	    hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);
	    if (hPtr != NULL) {
		mPtr = (Method *) Tcl_GetHashValue(hPtr);
		if (!IS_PRIVATE(mPtr)) {
		    AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			    flags);
		}
	    }
	}
    }
    if (!oPtr->selfCls) {
	return foundPrivate;
    }
    if (contextCls) {
	foundPrivate |= AddPrivatesFromClassChainToCallContext(oPtr->selfCls,
		contextCls, methodNameObj, cbPtr, doneFilters, flags,
		filterDecl);
    }

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 920–1017. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
	    hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);
	    if (hPtr != NULL) {
		mPtr = (Method *) Tcl_GetHashValue(hPtr);
		if (!IS_PRIVATE(mPtr)) {
		    AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			    flags);
		}
	    }
	}
    }
    if (!oPtr->selfCls) {
	return foundPrivate;
    }
    if (contextCls) {
	foundPrivate |= AddPrivatesFromClassChainToCallContext(oPtr->selfCls,
		contextCls, methodNameObj, cbPtr, doneFilters, flags,
		filterDecl);
    }
    if (!blockedUnexported) {
	foundPrivate |= AddSimpleClassChainToCallContext(oPtr->selfCls,
		methodNameObj, cbPtr, doneFilters, flags, filterDecl);
    }
    return foundPrivate;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddMethodToCallChain --
 *
 *	Utility method that manages the adding of a particular method
 *	implementation to a call-chain.
 *
 * ----------------------------------------------------------------------
 */

static inline void
AddMethodToCallChain(
    Method *const mPtr,		/* Actual method implementation to add to call
				 * chain (or NULL, a no-op). */
    ChainBuilder *const cbPtr,	/* The call chain to add the method
				 * implementation to. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. If NULL, not processing filters.
				 * Note that this function does not update
				 * this hashtable. */
    Class *const filterDecl,	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
    int flags)			/* Used to check if we're mixin-consistent
				 * only. Mixin-consistent means that either
				 * we're looking to add things from a mixin
				 * and we have passed a mixin, or we're not
				 * looking to add things from a mixin and have
				 * not passed a mixin. */
{
    CallChain *callPtr = cbPtr->callChainPtr;
    Tcl_Size i;

    /*
     * Return if this is just an entry used to record whether this is a public
     * method. If so, there's nothing real to call and so nothing to add to
     * the call chain.
     *
     * This is also where we enforce mixin-consistency.
     */

    if (mPtr == NULL || mPtr->typePtr == NULL || !MIXIN_CONSISTENT(flags)) {
	return;
    }

    /*
     * Enforce real private method handling here. We will skip adding this
     * method IF
     *  1) we are not allowing private methods, AND
     *  2) this is a private method, AND
     *  3) this is a class method, AND
     *  4) this method was not declared by the class of the current object.
     *
     * This does mean that only classes really handle private methods. This
     * should be sufficient for [incr Tcl] support though.
     */

    if (!WANT_UNEXPORTED(callPtr->flags)
	    && IS_UNEXPORTED(mPtr)
	    && (mPtr->declaringClassPtr != NULL)
	    && (mPtr->declaringClassPtr != cbPtr->oPtr->selfCls)) {
	return;
    }

    /*
     * First test whether the method is already in the call chain. Skip over
     * any leading filters.
     */

    for (i = cbPtr->filterLength ; i < callPtr->numChain ; i++) {
	if (callPtr->chain[i].mPtr == mPtr &&

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 953–1051. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

static inline void
AddMethodToCallChain(
    Method *const mPtr,		/* Actual method implementation to add to call
				 * chain (or NULL, a no-op). */
    ChainBuilder *const cbPtr,	/* The call chain to add the method
				 * implementation to. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. If NULL, not processing filters.
				 * Note that this function does not update
				 * this hashtable. */
    Class *const filterDecl,	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
    int flags)			/* Used to check if we're mixin-consistent
				 * only. Mixin-consistent means that either
				 * we're looking to add things from a mixin
				 * and we have passed a mixin, or we're not
				 * looking to add things from a mixin and have
				 * not passed a mixin. */
{
    CallChain *callPtr = cbPtr->callChainPtr;
    Tcl_Size i;

    /*
     * Return if this is just an entry used to record whether this is a public
     * method. If so, there's nothing real to call and so nothing to add to
     * the call chain.
     *
     * This is also where we enforce mixin-consistency.
     */

    if (mPtr == NULL || mPtr->typePtr == NULL || !MIXIN_CONSISTENT(flags)) {
	return;
    }

    /*
     * Enforce real private method handling here. We will skip adding this
     * method IF
     *  1) we are not allowing private methods, AND
     *  2) this is a private method, AND
     *  3) this is a class method, AND
     *  4) this method was not declared by the class of the current object.
     *
     * This does mean that only classes really handle private methods. This
     * should be sufficient for [incr Tcl] support though.
     */

    if (!WANT_UNEXPORTED(callPtr->flags)
	    && IS_UNEXPORTED(mPtr)
	    && (mPtr->declaringClassPtr != NULL)
	    && (mPtr->declaringClassPtr != cbPtr->oPtr->selfCls)) {
	return;
    }

    /*
     * First test whether the method is already in the call chain. Skip over
     * any leading filters.
     */

    for (i = cbPtr->filterLength ; i < callPtr->numChain ; i++) {
	if (callPtr->chain[i].mPtr == mPtr &&
		callPtr->chain[i].isFilter == (doneFilters != NULL)) {
	    /*
	     * Call chain semantics states that methods come as *late* in the
	     * call chain as possible. This is done by copying down the
	     * following methods. Note that this does not change the number of
	     * method invocations in the call chain; it just rearranges them.
	     */

	    Class *declCls = callPtr->chain[i].filterDeclarer;

	    for (; i + 1 < callPtr->numChain ; i++) {
		callPtr->chain[i] = callPtr->chain[i + 1];
	    }
	    callPtr->chain[i].mPtr = mPtr;
	    callPtr->chain[i].isFilter = (doneFilters != NULL);
	    callPtr->chain[i].filterDeclarer = declCls;
	    return;
	}
    }

    /*
     * Need to really add the method. This is made a bit more complex by the
     * fact that we are using some "static" space initially, and only start
     * realloc-ing if the chain gets long.
     */

    if (callPtr->numChain == CALL_CHAIN_STATIC_SIZE) {
	callPtr->chain = (MInvoke *)
		Tcl_Alloc(sizeof(MInvoke) * (callPtr->numChain + 1));
	memcpy(callPtr->chain, callPtr->staticChain,
		sizeof(MInvoke) * callPtr->numChain);
    } else if (callPtr->numChain > CALL_CHAIN_STATIC_SIZE) {
	callPtr->chain = (MInvoke *) Tcl_Realloc(callPtr->chain,
		sizeof(MInvoke) * (callPtr->numChain + 1));

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 1715–1812. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
	if (hPtr != NULL) {
	    Method *mPtr = (Method *) Tcl_GetHashValue(hPtr);

	    if (IS_PRIVATE(mPtr)) {
		AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			flags);
		return 1;
	    }
	}
    }

    switch (classPtr->superclasses.num) {
    case 1:
	classPtr = classPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, classPtr->superclasses) {
	    if (AddPrivatesFromClassChainToCallContext(superPtr, contextCls,
		    methodName, cbPtr, doneFilters, flags, filterDecl)) {
		return 1;
	    }
	}
	TCL_FALLTHROUGH();
    case 0:
	return 0;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * AddSimpleClassChainToCallContext --
 *
 *	Construct a call-chain from a class hierarchy.
 *
 * ----------------------------------------------------------------------
 */

static int
AddSimpleClassChainToCallContext(
    Class *classPtr,		/* Class to add the call chain entries for. */
    Tcl_Obj *const methodNameObj,
				/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what call chain entries
				 * have been processed. */
    int flags,			/* What sort of call chain are we building. */
    Class *const filterDecl)	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
{
    Tcl_Size i;
    int privateDanger = 0;
    Class *superPtr;

    /*
     * We hard-code the tail-recursive form. It's by far the most common case
     * *and* it is much more gentle on the stack.
     *
     * Note that mixins must be processed before the main class hierarchy.
     * [Bug 1998221]
     */

  tailRecurse:
    if (classPtr == NULL) {
	return privateDanger;
    }
    FOREACH(superPtr, classPtr->mixins) {
	privateDanger |= AddSimpleClassChainToCallContext(superPtr,
		methodNameObj, cbPtr, doneFilters, flags | TRAVERSED_MIXIN,
		filterDecl);
    }

    if (flags & CONSTRUCTOR) {
	AddMethodToCallChain(classPtr->constructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else if (flags & DESTRUCTOR) {
	AddMethodToCallChain(classPtr->destructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else {
	Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&classPtr->classMethods,
		methodNameObj);

	if (classPtr->flags & HAS_PRIVATE_METHODS) {
	    privateDanger |= 1;
	}
	if (hPtr != NULL) {
	    Method *mPtr = (Method *) Tcl_GetHashValue(hPtr);

	    if (!IS_PRIVATE(mPtr)) {
		if (!(flags & KNOWN_STATE)) {
		    if (flags & PUBLIC_METHOD) {
			if (!IS_PUBLIC(mPtr)) {
			    return privateDanger;
			}
			flags |= DEFINITE_PUBLIC;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 1787–1884. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
		filterDecl);
    }

    if (flags & CONSTRUCTOR) {
	AddMethodToCallChain(classPtr->constructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else if (flags & DESTRUCTOR) {
	AddMethodToCallChain(classPtr->destructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else {
	Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&classPtr->classMethods,
		methodNameObj);

	if (classPtr->flags & HAS_PRIVATE_METHODS) {
	    privateDanger |= 1;
	}
	if (hPtr != NULL) {
	    Method *mPtr = (Method *) Tcl_GetHashValue(hPtr);

	    if (!IS_PRIVATE(mPtr)) {
		if (!(flags & KNOWN_STATE)) {
		    if (flags & PUBLIC_METHOD) {
			if (!IS_PUBLIC(mPtr)) {
			    return privateDanger;
			}
			flags |= DEFINITE_PUBLIC;
		    } else {
			flags |= DEFINITE_PROTECTED;
		    }
		}
		AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			flags);
	    }
	}
    }

    switch (classPtr->superclasses.num) {
    case 1:
	classPtr = classPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, classPtr->superclasses) {
	    privateDanger |= AddSimpleClassChainToCallContext(superPtr,
		    methodNameObj, cbPtr, doneFilters, flags, filterDecl);
	}
	TCL_FALLTHROUGH();
    case 0:
	return privateDanger;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOORenderCallChain --
 *
 *	Create a description of a call chain. Used in [info object call],
 *	[info class call], and [self call].
 *
 * ----------------------------------------------------------------------
 */

Tcl_Obj *
TclOORenderCallChain(
    Tcl_Interp *interp,
    CallChain *callPtr)
{
    Tcl_Obj *filterLiteral, *methodLiteral, *objectLiteral, *privateLiteral;
    Tcl_Obj *resultObj, *descObjs[4], **objv;
    Foundation *fPtr = TclOOGetFoundation(interp);
    Tcl_Size i;

    /*
     * Allocate the literals (potentially) used in our description.
     */

    TclNewLiteralStringObj(filterLiteral, "filter");
    TclNewLiteralStringObj(methodLiteral, "method");
    TclNewLiteralStringObj(objectLiteral, "object");
    TclNewLiteralStringObj(privateLiteral, "private");

    /*
     * Do the actual construction of the descriptions. They consist of a list
     * of triples that describe the details of how a method is understood. For
     * each triple, the first word is the type of invocation ("method" is
     * normal, "unknown" is special because it adds the method name as an
     * extra argument when handled by some method types, and "filter" is
     * special because it's a filter method). The second word is the name of
     * the method in question (which differs for "unknown" and "filter" types)
     * and the third word is the full name of the class that declares the
     * method (or "object" if it is declared on the instance).
     */

    objv = (Tcl_Obj **)
	    TclStackAlloc(interp, callPtr->numChain * sizeof(Tcl_Obj *));
    for (i = 0 ; i < callPtr->numChain ; i++) {
	MInvoke *miPtr = &callPtr->chain[i];

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 1790–1887. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
    if (flags & CONSTRUCTOR) {
	AddMethodToCallChain(classPtr->constructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else if (flags & DESTRUCTOR) {
	AddMethodToCallChain(classPtr->destructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else {
	Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&classPtr->classMethods,
		methodNameObj);

	if (classPtr->flags & HAS_PRIVATE_METHODS) {
	    privateDanger |= 1;
	}
	if (hPtr != NULL) {
	    Method *mPtr = (Method *) Tcl_GetHashValue(hPtr);

	    if (!IS_PRIVATE(mPtr)) {
		if (!(flags & KNOWN_STATE)) {
		    if (flags & PUBLIC_METHOD) {
			if (!IS_PUBLIC(mPtr)) {
			    return privateDanger;
			}
			flags |= DEFINITE_PUBLIC;
		    } else {
			flags |= DEFINITE_PROTECTED;
		    }
		}
		AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			flags);
	    }
	}
    }

    switch (classPtr->superclasses.num) {
    case 1:
	classPtr = classPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, classPtr->superclasses) {
	    privateDanger |= AddSimpleClassChainToCallContext(superPtr,
		    methodNameObj, cbPtr, doneFilters, flags, filterDecl);
	}
	TCL_FALLTHROUGH();
    case 0:
	return privateDanger;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOORenderCallChain --
 *
 *	Create a description of a call chain. Used in [info object call],
 *	[info class call], and [self call].
 *
 * ----------------------------------------------------------------------
 */

Tcl_Obj *
TclOORenderCallChain(
    Tcl_Interp *interp,
    CallChain *callPtr)
{
    Tcl_Obj *filterLiteral, *methodLiteral, *objectLiteral, *privateLiteral;
    Tcl_Obj *resultObj, *descObjs[4], **objv;
    Foundation *fPtr = TclOOGetFoundation(interp);
    Tcl_Size i;

    /*
     * Allocate the literals (potentially) used in our description.
     */

    TclNewLiteralStringObj(filterLiteral, "filter");
    TclNewLiteralStringObj(methodLiteral, "method");
    TclNewLiteralStringObj(objectLiteral, "object");
    TclNewLiteralStringObj(privateLiteral, "private");

    /*
     * Do the actual construction of the descriptions. They consist of a list
     * of triples that describe the details of how a method is understood. For
     * each triple, the first word is the type of invocation ("method" is
     * normal, "unknown" is special because it adds the method name as an
     * extra argument when handled by some method types, and "filter" is
     * special because it's a filter method). The second word is the name of
     * the method in question (which differs for "unknown" and "filter" types)
     * and the third word is the full name of the class that declares the
     * method (or "object" if it is declared on the instance).
     */

    objv = (Tcl_Obj **)
	    TclStackAlloc(interp, callPtr->numChain * sizeof(Tcl_Obj *));
    for (i = 0 ; i < callPtr->numChain ; i++) {
	MInvoke *miPtr = &callPtr->chain[i];

	descObjs[0] =
	    miPtr->isFilter ? filterLiteral :
	    callPtr->flags & OO_UNKNOWN_METHOD ? fPtr->unknownMethodNameObj :

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 1813–1910. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
		    } else {
			flags |= DEFINITE_PROTECTED;
		    }
		}
		AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			flags);
	    }
	}
    }

    switch (classPtr->superclasses.num) {
    case 1:
	classPtr = classPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, classPtr->superclasses) {
	    privateDanger |= AddSimpleClassChainToCallContext(superPtr,
		    methodNameObj, cbPtr, doneFilters, flags, filterDecl);
	}
	TCL_FALLTHROUGH();
    case 0:
	return privateDanger;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOORenderCallChain --
 *
 *	Create a description of a call chain. Used in [info object call],
 *	[info class call], and [self call].
 *
 * ----------------------------------------------------------------------
 */

Tcl_Obj *
TclOORenderCallChain(
    Tcl_Interp *interp,
    CallChain *callPtr)
{
    Tcl_Obj *filterLiteral, *methodLiteral, *objectLiteral, *privateLiteral;
    Tcl_Obj *resultObj, *descObjs[4], **objv;
    Foundation *fPtr = TclOOGetFoundation(interp);
    Tcl_Size i;

    /*
     * Allocate the literals (potentially) used in our description.
     */

    TclNewLiteralStringObj(filterLiteral, "filter");
    TclNewLiteralStringObj(methodLiteral, "method");
    TclNewLiteralStringObj(objectLiteral, "object");
    TclNewLiteralStringObj(privateLiteral, "private");

    /*
     * Do the actual construction of the descriptions. They consist of a list
     * of triples that describe the details of how a method is understood. For
     * each triple, the first word is the type of invocation ("method" is
     * normal, "unknown" is special because it adds the method name as an
     * extra argument when handled by some method types, and "filter" is
     * special because it's a filter method). The second word is the name of
     * the method in question (which differs for "unknown" and "filter" types)
     * and the third word is the full name of the class that declares the
     * method (or "object" if it is declared on the instance).
     */

    objv = (Tcl_Obj **)
	    TclStackAlloc(interp, callPtr->numChain * sizeof(Tcl_Obj *));
    for (i = 0 ; i < callPtr->numChain ; i++) {
	MInvoke *miPtr = &callPtr->chain[i];

	descObjs[0] =
	    miPtr->isFilter ? filterLiteral :
	    callPtr->flags & OO_UNKNOWN_METHOD ? fPtr->unknownMethodNameObj :
	    IS_PRIVATE(miPtr->mPtr) ? privateLiteral :
		    methodLiteral;
	descObjs[1] =
	    callPtr->flags & CONSTRUCTOR ? fPtr->constructorName :
	    callPtr->flags & DESTRUCTOR ? fPtr->destructorName :
		    miPtr->mPtr->namePtr;
	descObjs[2] = miPtr->mPtr->declaringClassPtr
		? Tcl_GetObjectName(interp,
			(Tcl_Object) miPtr->mPtr->declaringClassPtr->thisPtr)
		: objectLiteral;
	descObjs[3] = Tcl_NewStringObj(miPtr->mPtr->typePtr->name,
		TCL_AUTO_LENGTH);

	objv[i] = Tcl_NewListObj(4, descObjs);
    }

    /*
     * Drop the local references to the literals; if they're actually used,
     * they'll live on the description itself.
     */

    Tcl_BounceRefCount(filterLiteral);
    Tcl_BounceRefCount(methodLiteral);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `methodNameType =`, lines 154–248. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.0.4--tclOOCall.c`.

```text
/*
 * Object type used to manage type caches attached to method names.
 */

static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,
    NULL,
    TCL_OBJTYPE_V0
};

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	Tcl_Free(callPtr->chain);
    }
    Tcl_Free(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 252–350. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
 * ----------------------------------------------------------------------
 */

Tcl_Method
TclNewMethod(
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    Class *clsPtr = (Class *) cls;
    Method *mPtr;
    Tcl_HashEntry *hPtr;
    int isNew;

    if (nameObj == NULL) {
	mPtr = (Method *) Tcl_Alloc(sizeof(Method));
	mPtr->namePtr = NULL;
	mPtr->refCount = 1;
	goto populate;
    }
    hPtr = Tcl_CreateHashEntry(&clsPtr->classMethods, nameObj, &isNew);
    if (isNew) {
	mPtr = (Method *) Tcl_Alloc(sizeof(Method));
	mPtr->refCount = 1;
	mPtr->namePtr = nameObj;
	Tcl_IncrRefCount(nameObj);
	Tcl_SetHashValue(hPtr, mPtr);
    } else {
	mPtr = (Method *) Tcl_GetHashValue(hPtr);
	if (mPtr->typePtr != NULL && mPtr->typePtr->deleteProc != NULL) {
	    mPtr->typePtr->deleteProc(mPtr->clientData);
	}
    }

  populate:
    clsPtr->thisPtr->fPtr->epoch++;
    mPtr->typePtr = typePtr;
    mPtr->clientData = clientData;
    mPtr->flags = 0;
    mPtr->declaringObjectPtr = NULL;
    mPtr->declaringClassPtr = clsPtr;
    if (flags) {
	mPtr->flags |= flags &
		(PUBLIC_METHOD | PRIVATE_METHOD | TRUE_PRIVATE_METHOD);
	if (flags & TRUE_PRIVATE_METHOD) {
	    clsPtr->flags |= HAS_PRIVATE_METHODS;
	}
    }

    return (Tcl_Method) mPtr;
}

Tcl_Method
Tcl_NewMethod(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    if (typePtr->version > TCL_OO_METHOD_VERSION_1) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod", "TCL_OO_METHOD_VERSION_1");
    }
    return TclNewMethod(cls, nameObj, flags, typePtr, clientData);
}

Tcl_Method
Tcl_NewMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType2 *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    if (typePtr->version < TCL_OO_METHOD_VERSION_2) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 327–422. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
    if (typePtr->version > TCL_OO_METHOD_VERSION_1) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod", "TCL_OO_METHOD_VERSION_1");
    }
    return TclNewMethod(cls, nameObj, flags, typePtr, clientData);
}

Tcl_Method
Tcl_NewMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType2 *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    if (typePtr->version < TCL_OO_METHOD_VERSION_2) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewMethod(cls, nameObj, flags,
	    (const Tcl_MethodType *) typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODelMethodRef --
 *
 *	How to delete a method.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->typePtr != NULL && mPtr->typePtr->deleteProc != NULL) {
	    mPtr->typePtr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	Tcl_Free(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineBasicMethods --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODefineBasicMethods(
    Class *clsPtr,		/* Class to attach the methods to. */
    const DeclaredClassMethod *dcmAry)
				/* Static table of method definitions. */
{
    int i;

    for (i = 0 ; dcmAry[i].name ; i++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcmAry[i].name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,
		(dcmAry[i].isPublic ? PUBLIC_METHOD : 0),
		&dcmAry[i].definition, NULL);
	Tcl_BounceRefCount(namePtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 349–444. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
    if (typePtr->version < TCL_OO_METHOD_VERSION_2) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewMethod(cls, nameObj, flags,
	    (const Tcl_MethodType *) typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODelMethodRef --
 *
 *	How to delete a method.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->typePtr != NULL && mPtr->typePtr->deleteProc != NULL) {
	    mPtr->typePtr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	Tcl_Free(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineBasicMethods --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODefineBasicMethods(
    Class *clsPtr,		/* Class to attach the methods to. */
    const DeclaredClassMethod *dcmAry)
				/* Static table of method definitions. */
{
    int i;

    for (i = 0 ; dcmAry[i].name ; i++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcmAry[i].name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,
		(dcmAry[i].isPublic ? PUBLIC_METHOD : 0),
		&dcmAry[i].definition, NULL);
	Tcl_BounceRefCount(namePtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which must not be
				 * NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which must not be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which must not be
				 * NULL. */
    ProcedureMethod **pmPtrPtr)	/* Place to write pointer to procedure method
				 * structure to allow for deeper tuning of the
				 * structure's contents. NULL if caller is not
				 * interested. */
{
    Tcl_Size argsLen;
    ProcedureMethod *pmPtr;
    Tcl_Method method;

    if (TclListObjLength(interp, argsObj, &argsLen) != TCL_OK) {
	return NULL;
    }

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 401–497. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text

    for (i = 0 ; dcmAry[i].name ; i++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcmAry[i].name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,
		(dcmAry[i].isPublic ? PUBLIC_METHOD : 0),
		&dcmAry[i].definition, NULL);
	Tcl_BounceRefCount(namePtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which must not be
				 * NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which must not be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which must not be
				 * NULL. */
    ProcedureMethod **pmPtrPtr)	/* Place to write pointer to procedure method
				 * structure to allow for deeper tuning of the
				 * structure's contents. NULL if caller is not
				 * interested. */
{
    Tcl_Size argsLen;
    ProcedureMethod *pmPtr;
    Tcl_Method method;

    if (TclListObjLength(interp, argsObj, &argsLen) != TCL_OK) {
	return NULL;
    }
    pmPtr = AllocProcedureMethodRecord(flags);
    method = TclOOMakeProcInstanceMethod(interp, oPtr, flags, nameObj,
	    argsObj, bodyObj, &procMethodType, pmPtr, &pmPtr->procPtr);
    if (method == NULL) {
	Tcl_Free(pmPtr);
    } else if (pmPtrPtr != NULL) {
	*pmPtrPtr = pmPtr;
    }
    return (Method *) method;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcMethod --
 *
 *	Create a new procedure-like method for a class.
 *
 * ----------------------------------------------------------------------
 */

Method *
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 740–837. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
    procPtr->cmdPtr = NULL;

    InitCmdFrame(iPtr, procPtr);

    return TclNewMethod(
	    (Tcl_Class) clsPtr, nameObj, flags, typePtr, clientData);
}

Tcl_Method
TclOOMakeProcMethod2(
    Tcl_Interp *interp,		/* The interpreter containing the class. */
    Class *clsPtr,		/* The class to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which may be NULL;
				 * if so, up to caller to manage storage
				 * (e.g., because it is a constructor or
				 * destructor). */
    const char *namePtr,	/* The name of the method as a string, which
				 * _must not_ be NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which _must not_ be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which _must not_ be
				 * NULL. */
    const Tcl_MethodType2 *typePtr,
				/* The type of the method to create. */
    void *clientData,		/* The per-method type-specific data. */
    Proc **procPtrPtr)		/* A pointer to the variable in which to write
				 * the procedure record reference. Presumably
				 * inside the structure indicated by the
				 * pointer in clientData. */
{
    Interp *iPtr = (Interp *) interp;
    Proc *procPtr;

    if (typePtr->version < TCL_OO_METHOD_VERSION_2) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"TclOOMakeProcMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    if (TclCreateProc(interp, NULL, namePtr, argsObj, bodyObj,
	    procPtrPtr) != TCL_OK) {
	return NULL;
    }
    procPtr = *procPtrPtr;
    procPtr->cmdPtr = NULL;

    InitCmdFrame(iPtr, procPtr);

    return TclNewMethod(
	    (Tcl_Class) clsPtr, nameObj, flags, (const Tcl_MethodType *)typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * InvokeProcedureMethod, FinalizePMCall, PushMethodCallFrame --
 *
 *	How to invoke a procedure-like method.
 *
 * ----------------------------------------------------------------------
 */

static int
InvokeProcedureMethod(
    void *clientData,		/* Pointer to the per-method record. */
    Tcl_Interp *interp,
    Tcl_ObjectContext context,	/* The method calling context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments as actually seen. */
{
    ProcedureMethod *pmPtr = (ProcedureMethod *) clientData;
    int result;
    PMFrameData *fdPtr;		/* Important data that has to have a lifetime
				 * matched by this function (or rather, by the
				 * call frame's lifetime). */

    /*
     * If the object namespace (or interpreter) were deleted, we just skip to
     * the next thing in the chain.
     */

    if (TclOOObjectDestroyed(((CallContext *) context)->oPtr)
	    || Tcl_InterpDeleted(interp)) {
	return TclNRObjectContextInvokeNext(interp, context, objc, objv,
		Tcl_ObjectContextSkippedArgs(context));
    }

    /*
     * Finishes filling out the extra frame info so that [info frame] works if
     * that is not already set up.
     */

    if (pmPtr->efi.length == 0) {
	Tcl_Method method = Tcl_ObjectContextMethod(context);

	pmPtr->efi.length = 2;
	pmPtr->efi.fields[0].name = "method";
	pmPtr->efi.fields[0].proc = RenderMethodName;
	pmPtr->efi.fields[0].clientData = pmPtr;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 783–880. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
    procPtr->cmdPtr = NULL;

    InitCmdFrame(iPtr, procPtr);

    return TclNewMethod(
	    (Tcl_Class) clsPtr, nameObj, flags, (const Tcl_MethodType *)typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * InvokeProcedureMethod, FinalizePMCall, PushMethodCallFrame --
 *
 *	How to invoke a procedure-like method.
 *
 * ----------------------------------------------------------------------
 */

static int
InvokeProcedureMethod(
    void *clientData,		/* Pointer to the per-method record. */
    Tcl_Interp *interp,
    Tcl_ObjectContext context,	/* The method calling context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments as actually seen. */
{
    ProcedureMethod *pmPtr = (ProcedureMethod *) clientData;
    int result;
    PMFrameData *fdPtr;		/* Important data that has to have a lifetime
				 * matched by this function (or rather, by the
				 * call frame's lifetime). */

    /*
     * If the object namespace (or interpreter) were deleted, we just skip to
     * the next thing in the chain.
     */

    if (TclOOObjectDestroyed(((CallContext *) context)->oPtr)
	    || Tcl_InterpDeleted(interp)) {
	return TclNRObjectContextInvokeNext(interp, context, objc, objv,
		Tcl_ObjectContextSkippedArgs(context));
    }

    /*
     * Finishes filling out the extra frame info so that [info frame] works if
     * that is not already set up.
     */

    if (pmPtr->efi.length == 0) {
	Tcl_Method method = Tcl_ObjectContextMethod(context);

	pmPtr->efi.length = 2;
	pmPtr->efi.fields[0].name = "method";
	pmPtr->efi.fields[0].proc = RenderMethodName;
	pmPtr->efi.fields[0].clientData = pmPtr;
	pmPtr->callSiteFlags = ((CallContext *)
		context)->callPtr->flags & (CONSTRUCTOR | DESTRUCTOR);
	pmPtr->interp = interp;
	pmPtr->method = method;
	if (pmPtr->gfivProc != NULL) {
	    pmPtr->efi.fields[1].name = "";
	    pmPtr->efi.fields[1].proc = pmPtr->gfivProc;
	    pmPtr->efi.fields[1].clientData = pmPtr;
	} else {
	    if (Tcl_MethodDeclarerObject(method) != NULL) {
		pmPtr->efi.fields[1].name = "object";
	    } else {
		pmPtr->efi.fields[1].name = "class";
	    }
	    pmPtr->efi.fields[1].proc = RenderDeclarerName;
	    pmPtr->efi.fields[1].clientData = pmPtr;
	}
    }

    /*
     * Allocate the special frame data.
     */

    fdPtr = (PMFrameData *) TclStackAlloc(interp, sizeof(PMFrameData));

    /*
     * Create a call frame for this method.
     */

    result = PushMethodCallFrame(interp, (CallContext *) context, pmPtr,
	    objc, objv, fdPtr);
    if (result != TCL_OK) {
	TclStackFree(interp, fdPtr);
	return result;
    }
    pmPtr->refCount++;

    /*
     * Give the pre-call callback a chance to do some setup and, possibly,
     * veto the call.
     */

    if (pmPtr->preCallProc != NULL) {

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 1550–1646. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text

    fmPtr = (ForwardMethod *) Tcl_Alloc(sizeof(ForwardMethod));
    fmPtr->prefixObj = prefixObj;
    Tcl_IncrRefCount(prefixObj);
    return (Method *) TclNewMethod((Tcl_Class) clsPtr, nameObj,
	    flags, &fwdMethodType, fmPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * InvokeForwardMethod, FinalizeForwardCall --
 *
 *	How to invoke a forwarded method. Works by doing some ensemble-like
 *	command rearranging and then invokes some other Tcl command.
 *
 * ----------------------------------------------------------------------
 */

static int
InvokeForwardMethod(
    void *clientData,		/* Pointer to some per-method context. */
    Tcl_Interp *interp,
    Tcl_ObjectContext context,	/* The method calling context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments as actually seen. */
{
    CallContext *contextPtr = (CallContext *) context;
    ForwardMethod *fmPtr = (ForwardMethod *) clientData;
    Tcl_Obj **argObjs, **prefixObjs;
    Tcl_Size numPrefixes, skip = contextPtr->skip;
    int len;

    /*
     * Build the real list of arguments to use. Note that we know that the
     * prefixObj field of the ForwardMethod structure holds a reference to a
     * non-empty list, so there's a whole class of failures ("not a list") we
     * can ignore here.
     */

    TclListObjGetElements(NULL, fmPtr->prefixObj, &numPrefixes, &prefixObjs);
    argObjs = InitEnsembleRewrite(interp, objc, objv, skip,
	    numPrefixes, prefixObjs, &len);
    TclNRAddCallback(interp, FinalizeForwardCall, argObjs, NULL, NULL, NULL);
    /*
     * NOTE: The combination of direct set of iPtr->lookupNsPtr and the use
     * of the TCL_EVAL_NOERR flag results in an evaluation configuration
     * very much like TCL_EVAL_INVOKE.
     */
    ((Interp *) interp)->lookupNsPtr = (Namespace *)
	    contextPtr->oPtr->namespacePtr;
    return TclNREvalObjv(interp, len, argObjs, TCL_EVAL_NOERR, NULL);
}

static int
FinalizeForwardCall(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    Tcl_Obj **argObjs = (Tcl_Obj **) data[0];

    TclStackFree(interp, argObjs);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * DeleteForwardMethod, CloneForwardMethod --
 *
 *	How to delete and clone forwarded methods.
 *
 * ----------------------------------------------------------------------
 */

static void
DeleteForwardMethod(
    void *clientData)
{
    ForwardMethod *fmPtr = (ForwardMethod *) clientData;

    Tcl_DecrRefCount(fmPtr->prefixObj);
    Tcl_Free(fmPtr);
}

static int
CloneForwardMethod(
    TCL_UNUSED(Tcl_Interp *),
    void *clientData,
    void **newClientData)
{
    ForwardMethod *fmPtr = (ForwardMethod *) clientData;
    ForwardMethod *fm2Ptr = (ForwardMethod *) Tcl_Alloc(sizeof(ForwardMethod));

    fm2Ptr->prefixObj = fmPtr->prefixObj;
    Tcl_IncrRefCount(fm2Ptr->prefixObj);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 309–405. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
    return (Tcl_Method) mPtr;
}

Tcl_Method
Tcl_NewMethod(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    if (typePtr->version > TCL_OO_METHOD_VERSION_1) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod", "TCL_OO_METHOD_VERSION_1");
    }
    return TclNewMethod(cls, nameObj, flags, typePtr, clientData);
}

Tcl_Method
Tcl_NewMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType2 *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    if (typePtr->version < TCL_OO_METHOD_VERSION_2) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewMethod(cls, nameObj, flags,
	    (const Tcl_MethodType *) typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODelMethodRef --
 *
 *	How to delete a method.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->typePtr != NULL && mPtr->typePtr->deleteProc != NULL) {
	    mPtr->typePtr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	Tcl_Free(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineBasicMethods --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODefineBasicMethods(
    Class *clsPtr,		/* Class to attach the methods to. */
    const DeclaredClassMethod *dcmAry)
				/* Static table of method definitions. */
{
    int i;

    for (i = 0 ; dcmAry[i].name ; i++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcmAry[i].name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `Tcl_NewMethod2(`, lines 331–426. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
    return TclNewMethod(cls, nameObj, flags, typePtr, clientData);
}

Tcl_Method
Tcl_NewMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType2 *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    if (typePtr->version < TCL_OO_METHOD_VERSION_2) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewMethod(cls, nameObj, flags,
	    (const Tcl_MethodType *) typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODelMethodRef --
 *
 *	How to delete a method.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->typePtr != NULL && mPtr->typePtr->deleteProc != NULL) {
	    mPtr->typePtr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	Tcl_Free(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineBasicMethods --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODefineBasicMethods(
    Class *clsPtr,		/* Class to attach the methods to. */
    const DeclaredClassMethod *dcmAry)
				/* Static table of method definitions. */
{
    int i;

    for (i = 0 ; dcmAry[i].name ; i++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcmAry[i].name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,
		(dcmAry[i].isPublic ? PUBLIC_METHOD : 0),
		&dcmAry[i].definition, NULL);
	Tcl_BounceRefCount(namePtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `TclOODelMethodRef(`, lines 364–459. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->typePtr != NULL && mPtr->typePtr->deleteProc != NULL) {
	    mPtr->typePtr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	Tcl_Free(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineBasicMethods --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODefineBasicMethods(
    Class *clsPtr,		/* Class to attach the methods to. */
    const DeclaredClassMethod *dcmAry)
				/* Static table of method definitions. */
{
    int i;

    for (i = 0 ; dcmAry[i].name ; i++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcmAry[i].name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,
		(dcmAry[i].isPublic ? PUBLIC_METHOD : 0),
		&dcmAry[i].definition, NULL);
	Tcl_BounceRefCount(namePtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which must not be
				 * NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which must not be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which must not be
				 * NULL. */
    ProcedureMethod **pmPtrPtr)	/* Place to write pointer to procedure method
				 * structure to allow for deeper tuning of the
				 * structure's contents. NULL if caller is not
				 * interested. */
{
    Tcl_Size argsLen;
    ProcedureMethod *pmPtr;
    Tcl_Method method;

    if (TclListObjLength(interp, argsObj, &argsLen) != TCL_OK) {
	return NULL;
    }
    pmPtr = AllocProcedureMethodRecord(flags);
    method = TclOOMakeProcInstanceMethod(interp, oPtr, flags, nameObj,
	    argsObj, bodyObj, &procMethodType, pmPtr, &pmPtr->procPtr);
    if (method == NULL) {
	Tcl_Free(pmPtr);
    } else if (pmPtrPtr != NULL) {
	*pmPtrPtr = pmPtr;
    }
    return (Method *) method;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcMethod --

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInt.h`, function `#define AddRef`, lines 668–754. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOInt.h`.

```text
/*
 * Alternatives to Tcl_Preserve/Tcl_EventuallyFree/Tcl_Release.
 */

#define AddRef(ptr) ((ptr)->refCount++)

/*
 * A convenience macro for iterating through the lists used in the internal
 * memory management of objects.
 * REQUIRES DECLARATION: Tcl_Size i;
 */

#define FOREACH(var,ary) \
    for(i=0 ; i<(ary).num; i++) if ((ary).list[i] == NULL) { \
	continue; \
    } else if ((var) = (ary).list[i], 1)

/*
 * A variation where the array is an array of structs. There's no issue with
 * possible NULLs; every element of the array will be iterated over and the
 * variable set to a pointer to each of those elements in turn.
 * REQUIRES DECLARATION: Tcl_Size i; See [96551aca55] for more FOREACH_STRUCT details.
 */

#define FOREACH_STRUCT(var,ary) \
    if (i=0, (ary).num>0) for(; var=&((ary).list[i]), i<(ary).num; i++)

/*
 * Convenience macros for iterating through hash tables. FOREACH_HASH_DECLS
 * sets up the declarations needed for the main macro, FOREACH_HASH, which
 * does the actual iteration. FOREACH_HASH_KEY and FOREACH_HASH_VALUE are
 * restricted versions that only iterate over keys or values respectively.
 * REQUIRES DECLARATION: FOREACH_HASH_DECLS;
 */

#define FOREACH_HASH_DECLS \
    Tcl_HashEntry *hPtr;Tcl_HashSearch search
#define FOREACH_HASH(key, val, tablePtr) \
    for(hPtr = Tcl_FirstHashEntry((tablePtr), &search); hPtr != NULL ? \
	    (*(void **)&(key) = Tcl_GetHashKey((tablePtr), hPtr), \
	    *(void **)&(val) = Tcl_GetHashValue(hPtr), 1) : 0; \
	    hPtr = Tcl_NextHashEntry(&search))
#define FOREACH_HASH_KEY(key, tablePtr) \
    for(hPtr = Tcl_FirstHashEntry((tablePtr), &search); hPtr != NULL ? \
	    (*(void **)&(key) = Tcl_GetHashKey((tablePtr), hPtr), 1) : 0; \
	    hPtr = Tcl_NextHashEntry(&search))
#define FOREACH_HASH_VALUE(val, tablePtr) \
    for(hPtr = Tcl_FirstHashEntry((tablePtr), &search); hPtr != NULL ? \
	    (*(void **)&(val) = Tcl_GetHashValue(hPtr), 1) : 0; \
	    hPtr = Tcl_NextHashEntry(&search))

/*
 * Convenience macro for duplicating a list. Needs no external declaration,
 * but all arguments are used multiple times and so must have no side effects.
 */

#undef DUPLICATE /* prevent possible conflict with definition in WINAPI nb30.h */
#define DUPLICATE(target,source,type) \
    do { \
	size_t len = sizeof(type) * ((target).num=(source).num);\
	if (len != 0) { \
	    memcpy(((target).list=(type*)Tcl_Alloc(len)), (source).list, len); \
	} else { \
	    (target).list = NULL; \
	} \
    } while(0)

/*
 * Convenience macro for generating error codes.
 */
#define OO_ERROR(interp, code) \
    Tcl_SetErrorCode((interp), "TCL", "OO", #code, (char *)NULL)

#ifdef _TCLSIZEHANDLED
#   undef _TCLSIZEHANDLED
#   undef Tcl_Size
#endif

#endif /* TCL_OO_INTERNAL_H */

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInt.h`, function `struct Method`, lines 35–133. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOInt.h`.

```text
typedef struct Class Class;
typedef struct DeclaredClassMethod DeclaredClassMethod;
typedef struct ForwardMethod ForwardMethod;
typedef struct Foundation Foundation;
typedef struct Method Method;
typedef struct MInvoke MInvoke;
typedef struct Object Object;
typedef struct PrivateVariableMapping PrivateVariableMapping;
typedef struct ProcedureMethod ProcedureMethod;
typedef struct PropertyStorage PropertyStorage;

#if (TCL_MAJOR_VERSION < 9) && !defined(Tcl_Size)
#   define Tcl_Size int
#   define _TCLSIZEHANDLED
# endif

/*
 * The data that needs to be stored per method. This record is used to collect
 * information about all sorts of methods, including forwards, constructors
 * and destructors.
 */
struct Method {
    union {
	const Tcl_MethodType *typePtr;
#if TCL_MAJOR_VERSION > 8
	const Tcl_MethodType2 *type2Ptr;
#endif
    };				/* The type of method. If NULL, this is a
				 * special flag record which is just used for
				 * the setting of the flags field. Note that
				 * this is a union of two pointer types that
				 * have the same layout at least as far as the
				 * internal version field. */
    Tcl_Size refCount;
    void *clientData;		/* Type-specific data. */
    Tcl_Obj *namePtr;		/* Name of the method. */
    Object *declaringObjectPtr;	/* The object that declares this method, or
				 * NULL if it was declared by a class. */
    Class *declaringClassPtr;	/* The class that declares this method, or
				 * NULL if it was declared directly on an
				 * object. */
    int flags;			/* Assorted flags. Includes whether this
				 * method is public/exported or not. */
};

/*
 * Pre- and post-call callbacks, to allow procedure-like methods to be fine
 * tuned in their behaviour.
 */

typedef int (TclOO_PreCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_CallFrame *framePtr, int *isFinished);
typedef int (TclOO_PostCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_Namespace *namespacePtr, int result);
typedef void (TclOO_PmCDDeleteProc)(void *clientData);
typedef void *(TclOO_PmCDCloneProc)(void *clientData);

/*
 * Procedure-like methods have the following extra information.
 */
struct ProcedureMethod {
    int version;		/* Version of this structure. Currently must
				 * be TCLOO_PROCEDURE_METHOD_VERSION_1. */
    Proc *procPtr;		/* Core of the implementation of the method;
				 * includes the argument definition and the
				 * body bytecodes. */
    int flags;			/* Flags to control features. */
    Tcl_Size refCount;
    void *clientData;
    TclOO_PmCDDeleteProc *deleteClientdataProc;
    TclOO_PmCDCloneProc *cloneClientdataProc;
    ProcErrorProc *errProc;	/* Replacement error handler. */
    TclOO_PreCallProc *preCallProc;
				/* Callback to allow for additional setup
				 * before the method executes. */
    TclOO_PostCallProc *postCallProc;
				/* Callback to allow for additional cleanup
				 * after the method executes. */
    GetFrameInfoValueProc *gfivProc;
				/* Callback to allow for fine tuning of how
				 * the method reports itself. */
    Command cmd;		/* Space used to connect to [info frame] */
    ExtraFrameInfo efi;		/* Space used to store data for [info frame] */
    Tcl_Interp *interp;		/* Interpreter in which to compute the name of
				 * the method. */
    Tcl_Method method;		/* Method to compute the name of. */
    int callSiteFlags;		/* Flags from the call chain. Only interested
				 * in whether this is a constructor or
				 * destructor, which we can't know until then
				 * for messy reasons. Other flags are variable
				 * but not used. */
};

enum ProcedureMethodVersion {
    TCLOO_PROCEDURE_METHOD_VERSION_1 = 0
};
#define TCLOO_PROCEDURE_METHOD_VERSION TCLOO_PROCEDURE_METHOD_VERSION_1

/*

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInt.h`, function `struct Method`, lines 52–150. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOInt.h`.

```text
 * The data that needs to be stored per method. This record is used to collect
 * information about all sorts of methods, including forwards, constructors
 * and destructors.
 */
struct Method {
    union {
	const Tcl_MethodType *typePtr;
#if TCL_MAJOR_VERSION > 8
	const Tcl_MethodType2 *type2Ptr;
#endif
    };				/* The type of method. If NULL, this is a
				 * special flag record which is just used for
				 * the setting of the flags field. Note that
				 * this is a union of two pointer types that
				 * have the same layout at least as far as the
				 * internal version field. */
    Tcl_Size refCount;
    void *clientData;		/* Type-specific data. */
    Tcl_Obj *namePtr;		/* Name of the method. */
    Object *declaringObjectPtr;	/* The object that declares this method, or
				 * NULL if it was declared by a class. */
    Class *declaringClassPtr;	/* The class that declares this method, or
				 * NULL if it was declared directly on an
				 * object. */
    int flags;			/* Assorted flags. Includes whether this
				 * method is public/exported or not. */
};

/*
 * Pre- and post-call callbacks, to allow procedure-like methods to be fine
 * tuned in their behaviour.
 */

typedef int (TclOO_PreCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_CallFrame *framePtr, int *isFinished);
typedef int (TclOO_PostCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_Namespace *namespacePtr, int result);
typedef void (TclOO_PmCDDeleteProc)(void *clientData);
typedef void *(TclOO_PmCDCloneProc)(void *clientData);

/*
 * Procedure-like methods have the following extra information.
 */
struct ProcedureMethod {
    int version;		/* Version of this structure. Currently must
				 * be TCLOO_PROCEDURE_METHOD_VERSION_1. */
    Proc *procPtr;		/* Core of the implementation of the method;
				 * includes the argument definition and the
				 * body bytecodes. */
    int flags;			/* Flags to control features. */
    Tcl_Size refCount;
    void *clientData;
    TclOO_PmCDDeleteProc *deleteClientdataProc;
    TclOO_PmCDCloneProc *cloneClientdataProc;
    ProcErrorProc *errProc;	/* Replacement error handler. */
    TclOO_PreCallProc *preCallProc;
				/* Callback to allow for additional setup
				 * before the method executes. */
    TclOO_PostCallProc *postCallProc;
				/* Callback to allow for additional cleanup
				 * after the method executes. */
    GetFrameInfoValueProc *gfivProc;
				/* Callback to allow for fine tuning of how
				 * the method reports itself. */
    Command cmd;		/* Space used to connect to [info frame] */
    ExtraFrameInfo efi;		/* Space used to store data for [info frame] */
    Tcl_Interp *interp;		/* Interpreter in which to compute the name of
				 * the method. */
    Tcl_Method method;		/* Method to compute the name of. */
    int callSiteFlags;		/* Flags from the call chain. Only interested
				 * in whether this is a constructor or
				 * destructor, which we can't know until then
				 * for messy reasons. Other flags are variable
				 * but not used. */
};

enum ProcedureMethodVersion {
    TCLOO_PROCEDURE_METHOD_VERSION_1 = 0
};
#define TCLOO_PROCEDURE_METHOD_VERSION TCLOO_PROCEDURE_METHOD_VERSION_1

/*
 * Flags for use in a ProcedureMethod.
 *
 */
enum ProceudreMethodFlags {
    USE_DECLARER_NS = 0x80	/* When set, the method will use the namespace
				 * of the object or class that declared it (or
				 * the clone of it, if it was from such that
				 * the implementation of the method came to the
				 * particular use) instead of the namespace of
				 * the object on which the method was invoked.
				 * This flag must be distinct from all others
				 * that are associated with methods. */
};

/*
 * Forwarded methods have the following extra information.
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInt.h`, function `struct CallChain`, lines 29–127. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOInt.h`.

```text
/*
 * Forward declarations.
 */

typedef struct CallChain CallChain;
typedef struct CallContext CallContext;
typedef struct Class Class;
typedef struct DeclaredClassMethod DeclaredClassMethod;
typedef struct ForwardMethod ForwardMethod;
typedef struct Foundation Foundation;
typedef struct Method Method;
typedef struct MInvoke MInvoke;
typedef struct Object Object;
typedef struct PrivateVariableMapping PrivateVariableMapping;
typedef struct ProcedureMethod ProcedureMethod;
typedef struct PropertyStorage PropertyStorage;

#if (TCL_MAJOR_VERSION < 9) && !defined(Tcl_Size)
#   define Tcl_Size int
#   define _TCLSIZEHANDLED
# endif

/*
 * The data that needs to be stored per method. This record is used to collect
 * information about all sorts of methods, including forwards, constructors
 * and destructors.
 */
struct Method {
    union {
	const Tcl_MethodType *typePtr;
#if TCL_MAJOR_VERSION > 8
	const Tcl_MethodType2 *type2Ptr;
#endif
    };				/* The type of method. If NULL, this is a
				 * special flag record which is just used for
				 * the setting of the flags field. Note that
				 * this is a union of two pointer types that
				 * have the same layout at least as far as the
				 * internal version field. */
    Tcl_Size refCount;
    void *clientData;		/* Type-specific data. */
    Tcl_Obj *namePtr;		/* Name of the method. */
    Object *declaringObjectPtr;	/* The object that declares this method, or
				 * NULL if it was declared by a class. */
    Class *declaringClassPtr;	/* The class that declares this method, or
				 * NULL if it was declared directly on an
				 * object. */
    int flags;			/* Assorted flags. Includes whether this
				 * method is public/exported or not. */
};

/*
 * Pre- and post-call callbacks, to allow procedure-like methods to be fine
 * tuned in their behaviour.
 */

typedef int (TclOO_PreCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_CallFrame *framePtr, int *isFinished);
typedef int (TclOO_PostCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_Namespace *namespacePtr, int result);
typedef void (TclOO_PmCDDeleteProc)(void *clientData);
typedef void *(TclOO_PmCDCloneProc)(void *clientData);

/*
 * Procedure-like methods have the following extra information.
 */
struct ProcedureMethod {
    int version;		/* Version of this structure. Currently must
				 * be TCLOO_PROCEDURE_METHOD_VERSION_1. */
    Proc *procPtr;		/* Core of the implementation of the method;
				 * includes the argument definition and the
				 * body bytecodes. */
    int flags;			/* Flags to control features. */
    Tcl_Size refCount;
    void *clientData;
    TclOO_PmCDDeleteProc *deleteClientdataProc;
    TclOO_PmCDCloneProc *cloneClientdataProc;
    ProcErrorProc *errProc;	/* Replacement error handler. */
    TclOO_PreCallProc *preCallProc;
				/* Callback to allow for additional setup
				 * before the method executes. */
    TclOO_PostCallProc *postCallProc;
				/* Callback to allow for additional cleanup
				 * after the method executes. */
    GetFrameInfoValueProc *gfivProc;
				/* Callback to allow for fine tuning of how
				 * the method reports itself. */
    Command cmd;		/* Space used to connect to [info frame] */
    ExtraFrameInfo efi;		/* Space used to store data for [info frame] */
    Tcl_Interp *interp;		/* Interpreter in which to compute the name of
				 * the method. */
    Tcl_Method method;		/* Method to compute the name of. */
    int callSiteFlags;		/* Flags from the call chain. Only interested
				 * in whether this is a constructor or
				 * destructor, which we can't know until then
				 * for messy reasons. Other flags are variable
				 * but not used. */
};

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInt.h`, function `struct CallChain`, lines 439–536. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOInt.h`.

```text

/*
 * The cacheable part of a call context.
 */
struct CallChain {
    Tcl_Size objectCreationEpoch;/* The object's creation epoch. Note that the
				 * object reference is not stored in the call
				 * chain; it is in the call context. */
    Tcl_Size objectEpoch;	/* Local (object structure) epoch counter
				 * snapshot. */
    Tcl_Size epoch;		/* Global (class structure) epoch counter
				 * snapshot. */
    int flags;			/* Assorted flags, see below. */
    Tcl_Size refCount;		/* Reference count. */
    Tcl_Size numChain;		/* Size of the call chain. */
    MInvoke *chain;		/* Array of call chain entries. May point to
				 * staticChain if the number of entries is
				 * small. */
    MInvoke staticChain[CALL_CHAIN_STATIC_SIZE];
};

/*
 * A call context structure is built when a method is called. It contains the
 * chain of method implementations that are to be invoked by a particular
 * call, and the process of calling walks the chain, with the [next] command
 * proceeding to the next entry in the chain.
 */
struct CallContext {
    Object *oPtr;		/* The object associated with this call. */
    Tcl_Size index;		/* Index into the call chain of the currently
				 * executing method implementation. */
    Tcl_Size skip;		/* Current number of arguments to skip; can
				 * vary depending on whether it is a direct
				 * method call or a continuation via the
				 * [next] command. */
    CallChain *callPtr;		/* The actual call chain. */
};

/*
 * Bits for the 'flags' field of the call chain.
 */
enum TclOOCallChainFlags {
    PUBLIC_METHOD = 0x01,	/* This is a public (exported) method. */
    PRIVATE_METHOD = 0x02,	/* This is a private (class's direct instances
				 * only) method. Supports itcl. */
    OO_UNKNOWN_METHOD = 0x04,	/* This is an unknown method. */
    CONSTRUCTOR = 0x08,		/* This is a constructor. */
    DESTRUCTOR = 0x10,		/* This is a destructor. */
    TRUE_PRIVATE_METHOD = 0x20	/* This is a private method only accessible
				 * from other methods defined on this class
				 * or instance. [TIP #500] */
};
#define SCOPE_FLAGS (PUBLIC_METHOD | PRIVATE_METHOD | TRUE_PRIVATE_METHOD)

/*
 * Structure containing definition information about basic class methods.
 */
struct DeclaredClassMethod {
    const char *name;		/* Name of the method in question. */
    int isPublic;		/* Whether the method is public by default. */
    Tcl_MethodType definition;	/* How to call the method. */
};

/*
 *----------------------------------------------------------------
 * Commands relating to OO support.
 *----------------------------------------------------------------
 */

MODULE_SCOPE int		TclOOInit(Tcl_Interp *interp);
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOOObjDefObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineClassMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineConstructorObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineDefnNsObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineDeleteMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineDestructorObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineExportObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineForwardObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineInitialiseObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineRenameMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineUnexportObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineClassObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineSelfObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineObjSelfObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefinePrivateObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefinePropertyCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOOUnknownDefinition;
MODULE_SCOPE Tcl_ObjCmdProc	TclOOCallbackObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOOClassVariableObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOOCopyObjectCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODelegateNameObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOOLinkObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOONextObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOONextToObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOOSelfObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOOInfoObjectPropCmd;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInt.h`, function `struct MInvoke`, lines 36–134. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOInt.h`.

```text
typedef struct DeclaredClassMethod DeclaredClassMethod;
typedef struct ForwardMethod ForwardMethod;
typedef struct Foundation Foundation;
typedef struct Method Method;
typedef struct MInvoke MInvoke;
typedef struct Object Object;
typedef struct PrivateVariableMapping PrivateVariableMapping;
typedef struct ProcedureMethod ProcedureMethod;
typedef struct PropertyStorage PropertyStorage;

#if (TCL_MAJOR_VERSION < 9) && !defined(Tcl_Size)
#   define Tcl_Size int
#   define _TCLSIZEHANDLED
# endif

/*
 * The data that needs to be stored per method. This record is used to collect
 * information about all sorts of methods, including forwards, constructors
 * and destructors.
 */
struct Method {
    union {
	const Tcl_MethodType *typePtr;
#if TCL_MAJOR_VERSION > 8
	const Tcl_MethodType2 *type2Ptr;
#endif
    };				/* The type of method. If NULL, this is a
				 * special flag record which is just used for
				 * the setting of the flags field. Note that
				 * this is a union of two pointer types that
				 * have the same layout at least as far as the
				 * internal version field. */
    Tcl_Size refCount;
    void *clientData;		/* Type-specific data. */
    Tcl_Obj *namePtr;		/* Name of the method. */
    Object *declaringObjectPtr;	/* The object that declares this method, or
				 * NULL if it was declared by a class. */
    Class *declaringClassPtr;	/* The class that declares this method, or
				 * NULL if it was declared directly on an
				 * object. */
    int flags;			/* Assorted flags. Includes whether this
				 * method is public/exported or not. */
};

/*
 * Pre- and post-call callbacks, to allow procedure-like methods to be fine
 * tuned in their behaviour.
 */

typedef int (TclOO_PreCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_CallFrame *framePtr, int *isFinished);
typedef int (TclOO_PostCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_Namespace *namespacePtr, int result);
typedef void (TclOO_PmCDDeleteProc)(void *clientData);
typedef void *(TclOO_PmCDCloneProc)(void *clientData);

/*
 * Procedure-like methods have the following extra information.
 */
struct ProcedureMethod {
    int version;		/* Version of this structure. Currently must
				 * be TCLOO_PROCEDURE_METHOD_VERSION_1. */
    Proc *procPtr;		/* Core of the implementation of the method;
				 * includes the argument definition and the
				 * body bytecodes. */
    int flags;			/* Flags to control features. */
    Tcl_Size refCount;
    void *clientData;
    TclOO_PmCDDeleteProc *deleteClientdataProc;
    TclOO_PmCDCloneProc *cloneClientdataProc;
    ProcErrorProc *errProc;	/* Replacement error handler. */
    TclOO_PreCallProc *preCallProc;
				/* Callback to allow for additional setup
				 * before the method executes. */
    TclOO_PostCallProc *postCallProc;
				/* Callback to allow for additional cleanup
				 * after the method executes. */
    GetFrameInfoValueProc *gfivProc;
				/* Callback to allow for fine tuning of how
				 * the method reports itself. */
    Command cmd;		/* Space used to connect to [info frame] */
    ExtraFrameInfo efi;		/* Space used to store data for [info frame] */
    Tcl_Interp *interp;		/* Interpreter in which to compute the name of
				 * the method. */
    Tcl_Method method;		/* Method to compute the name of. */
    int callSiteFlags;		/* Flags from the call chain. Only interested
				 * in whether this is a constructor or
				 * destructor, which we can't know until then
				 * for messy reasons. Other flags are variable
				 * but not used. */
};

enum ProcedureMethodVersion {
    TCLOO_PROCEDURE_METHOD_VERSION_1 = 0
};
#define TCLOO_PROCEDURE_METHOD_VERSION TCLOO_PROCEDURE_METHOD_VERSION_1

/*
 * Flags for use in a ProcedureMethod.

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInt.h`, function `struct MInvoke`, lines 428–525. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOInt.h`.

```text
/*
 * Information relating to the invocation of a particular method implementation
 * in a call chain.
 */
struct MInvoke {
    Method *mPtr;		/* Reference to the method implementation
				 * record. */
    int isFilter;		/* Whether this is a filter invocation. */
    Class *filterDeclarer;	/* What class decided to add the filter; if
				 * NULL, it was added by the object. */
};

/*
 * The cacheable part of a call context.
 */
struct CallChain {
    Tcl_Size objectCreationEpoch;/* The object's creation epoch. Note that the
				 * object reference is not stored in the call
				 * chain; it is in the call context. */
    Tcl_Size objectEpoch;	/* Local (object structure) epoch counter
				 * snapshot. */
    Tcl_Size epoch;		/* Global (class structure) epoch counter
				 * snapshot. */
    int flags;			/* Assorted flags, see below. */
    Tcl_Size refCount;		/* Reference count. */
    Tcl_Size numChain;		/* Size of the call chain. */
    MInvoke *chain;		/* Array of call chain entries. May point to
				 * staticChain if the number of entries is
				 * small. */
    MInvoke staticChain[CALL_CHAIN_STATIC_SIZE];
};

/*
 * A call context structure is built when a method is called. It contains the
 * chain of method implementations that are to be invoked by a particular
 * call, and the process of calling walks the chain, with the [next] command
 * proceeding to the next entry in the chain.
 */
struct CallContext {
    Object *oPtr;		/* The object associated with this call. */
    Tcl_Size index;		/* Index into the call chain of the currently
				 * executing method implementation. */
    Tcl_Size skip;		/* Current number of arguments to skip; can
				 * vary depending on whether it is a direct
				 * method call or a continuation via the
				 * [next] command. */
    CallChain *callPtr;		/* The actual call chain. */
};

/*
 * Bits for the 'flags' field of the call chain.
 */
enum TclOOCallChainFlags {
    PUBLIC_METHOD = 0x01,	/* This is a public (exported) method. */
    PRIVATE_METHOD = 0x02,	/* This is a private (class's direct instances
				 * only) method. Supports itcl. */
    OO_UNKNOWN_METHOD = 0x04,	/* This is an unknown method. */
    CONSTRUCTOR = 0x08,		/* This is a constructor. */
    DESTRUCTOR = 0x10,		/* This is a destructor. */
    TRUE_PRIVATE_METHOD = 0x20	/* This is a private method only accessible
				 * from other methods defined on this class
				 * or instance. [TIP #500] */
};
#define SCOPE_FLAGS (PUBLIC_METHOD | PRIVATE_METHOD | TRUE_PRIVATE_METHOD)

/*
 * Structure containing definition information about basic class methods.
 */
struct DeclaredClassMethod {
    const char *name;		/* Name of the method in question. */
    int isPublic;		/* Whether the method is public by default. */
    Tcl_MethodType definition;	/* How to call the method. */
};

/*
 *----------------------------------------------------------------
 * Commands relating to OO support.
 *----------------------------------------------------------------
 */

MODULE_SCOPE int		TclOOInit(Tcl_Interp *interp);
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOOObjDefObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineClassMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineConstructorObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineDefnNsObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineDeleteMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineDestructorObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineExportObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineForwardObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineInitialiseObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineRenameMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineUnexportObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineClassObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineSelfObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefineObjSelfObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc	TclOODefinePrivateObjCmd;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `typedef int (Tcl_MethodCallProc`, lines 60–156. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text
 * implementation. These are used to implement custom types of method calls
 * and to allow the attachment of arbitrary data to objects and classes.
 */

typedef int (Tcl_MethodCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext objectContext, int objc, Tcl_Obj *const *objv);
#if TCL_MAJOR_VERSION > 8
typedef int (Tcl_MethodCallProc2)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext objectContext, Tcl_Size objc, Tcl_Obj *const *objv);
#endif
typedef void (Tcl_MethodDeleteProc)(void *clientData);
typedef int (Tcl_CloneProc)(Tcl_Interp *interp, void *oldClientData,
	void **newClientData);
typedef void (Tcl_ObjectMetadataDeleteProc)(void *clientData);
typedef int (Tcl_ObjectMapMethodNameProc)(Tcl_Interp *interp,
	Tcl_Object object, Tcl_Class *startClsPtr, Tcl_Obj *methodNameObj);

/*
 * The type of a method implementation. This describes how to call the method
 * implementation, how to delete it (when the object or class is deleted) and
 * how to create a clone of it (when the object or class is copied).
 */

typedef struct Tcl_MethodType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_(1|CURRENT) in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType;

#if TCL_MAJOR_VERSION > 8
typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;
#endif

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
    TCL_OO_METHOD_VERSION_CURRENT = 1
#if TCL_MAJOR_VERSION > 8
    ,
    TCL_OO_METHOD_VERSION_1 = 1,
    TCL_OO_METHOD_VERSION_2 = 2
#endif
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `typedef int (Tcl_MethodCallProc`, lines 63–159. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text

typedef int (Tcl_MethodCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext objectContext, int objc, Tcl_Obj *const *objv);
#if TCL_MAJOR_VERSION > 8
typedef int (Tcl_MethodCallProc2)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext objectContext, Tcl_Size objc, Tcl_Obj *const *objv);
#endif
typedef void (Tcl_MethodDeleteProc)(void *clientData);
typedef int (Tcl_CloneProc)(Tcl_Interp *interp, void *oldClientData,
	void **newClientData);
typedef void (Tcl_ObjectMetadataDeleteProc)(void *clientData);
typedef int (Tcl_ObjectMapMethodNameProc)(Tcl_Interp *interp,
	Tcl_Object object, Tcl_Class *startClsPtr, Tcl_Obj *methodNameObj);

/*
 * The type of a method implementation. This describes how to call the method
 * implementation, how to delete it (when the object or class is deleted) and
 * how to create a clone of it (when the object or class is copied).
 */

typedef struct Tcl_MethodType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_(1|CURRENT) in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType;

#if TCL_MAJOR_VERSION > 8
typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;
#endif

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
    TCL_OO_METHOD_VERSION_CURRENT = 1
#if TCL_MAJOR_VERSION > 8
    ,
    TCL_OO_METHOD_VERSION_1 = 1,
    TCL_OO_METHOD_VERSION_2 = 2
#endif
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `typedef struct Tcl_MethodType`, lines 79–175. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text
 * implementation, how to delete it (when the object or class is deleted) and
 * how to create a clone of it (when the object or class is copied).
 */

typedef struct Tcl_MethodType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_(1|CURRENT) in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType;

#if TCL_MAJOR_VERSION > 8
typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;
#endif

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
    TCL_OO_METHOD_VERSION_CURRENT = 1
#if TCL_MAJOR_VERSION > 8
    ,
    TCL_OO_METHOD_VERSION_1 = 1,
    TCL_OO_METHOD_VERSION_2 = 2
#endif
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `typedef struct Tcl_MethodType`, lines 97–190. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text
				 * be copied directly. */
} Tcl_MethodType;

#if TCL_MAJOR_VERSION > 8
typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;
#endif

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
    TCL_OO_METHOD_VERSION_CURRENT = 1
#if TCL_MAJOR_VERSION > 8
    ,
    TCL_OO_METHOD_VERSION_1 = 1,
    TCL_OO_METHOD_VERSION_2 = 2
#endif
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 81–177. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text
 */

typedef struct Tcl_MethodType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_(1|CURRENT) in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType;

#if TCL_MAJOR_VERSION > 8
typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;
#endif

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
    TCL_OO_METHOD_VERSION_CURRENT = 1
#if TCL_MAJOR_VERSION > 8
    ,
    TCL_OO_METHOD_VERSION_1 = 1,
    TCL_OO_METHOD_VERSION_2 = 2
#endif
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 99–190. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text

#if TCL_MAJOR_VERSION > 8
typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;
#endif

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
    TCL_OO_METHOD_VERSION_CURRENT = 1
#if TCL_MAJOR_VERSION > 8
    ,
    TCL_OO_METHOD_VERSION_1 = 1,
    TCL_OO_METHOD_VERSION_2 = 2
#endif
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 121–190. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
    TCL_OO_METHOD_VERSION_CURRENT = 1
#if TCL_MAJOR_VERSION > 8
    ,
    TCL_OO_METHOD_VERSION_1 = 1,
    TCL_OO_METHOD_VERSION_2 = 2
#endif
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 124–190. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text
enum TclOOMethodVersion {
    TCL_OO_METHOD_VERSION_CURRENT = 1
#if TCL_MAJOR_VERSION > 8
    ,
    TCL_OO_METHOD_VERSION_1 = 1,
    TCL_OO_METHOD_VERSION_2 = 2
#endif
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 125–190. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text
    TCL_OO_METHOD_VERSION_CURRENT = 1
#if TCL_MAJOR_VERSION > 8
    ,
    TCL_OO_METHOD_VERSION_1 = 1,
    TCL_OO_METHOD_VERSION_2 = 2
#endif
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 69–167. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h`.

```text
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType *typePtr,
				void *clientData);
/* 12 */
TCLAPI Tcl_Method	Tcl_NewMethod(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,
				const Tcl_MethodType *typePtr,
				void *clientData);
/* 13 */
TCLAPI Tcl_Object	Tcl_NewObjectInstance(Tcl_Interp *interp,
				Tcl_Class cls, const char *nameStr,
				const char *nsNameStr, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 14 */
TCLAPI int		Tcl_ObjectDeleted(Tcl_Object object);
/* 15 */
TCLAPI int		Tcl_ObjectContextIsFiltering(
				Tcl_ObjectContext context);
/* 16 */
TCLAPI Tcl_Method	Tcl_ObjectContextMethod(Tcl_ObjectContext context);
/* 17 */
TCLAPI Tcl_Object	Tcl_ObjectContextObject(Tcl_ObjectContext context);
/* 18 */
TCLAPI Tcl_Size		Tcl_ObjectContextSkippedArgs(
				Tcl_ObjectContext context);
/* 19 */
TCLAPI void *		Tcl_ClassGetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr);
/* 20 */
TCLAPI void		Tcl_ClassSetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 21 */
TCLAPI void *		Tcl_ObjectGetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr);
/* 22 */
TCLAPI void		Tcl_ObjectSetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 23 */
TCLAPI int		Tcl_ObjectContextInvokeNext(Tcl_Interp *interp,
				Tcl_ObjectContext context, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 24 */
TCLAPI Tcl_ObjectMapMethodNameProc * Tcl_ObjectGetMethodNameMapper(
				Tcl_Object object);
/* 25 */
TCLAPI void		Tcl_ObjectSetMethodNameMapper(Tcl_Object object,
				Tcl_ObjectMapMethodNameProc *mapMethodNameProc);
/* 26 */
TCLAPI void		Tcl_ClassSetConstructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 27 */
TCLAPI void		Tcl_ClassSetDestructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 28 */
TCLAPI Tcl_Obj *	Tcl_GetObjectName(Tcl_Interp *interp,
				Tcl_Object object);
/* 29 */
TCLAPI int		Tcl_MethodIsPrivate(Tcl_Method method);
/* 30 */
TCLAPI Tcl_Class	Tcl_GetClassOfObject(Tcl_Object object);
/* 31 */
TCLAPI Tcl_Obj *	Tcl_GetObjectClassName(Tcl_Interp *interp,
				Tcl_Object object);
/* 32 */
TCLAPI int		Tcl_MethodIsType2(Tcl_Method method,
				const Tcl_MethodType2 *typePtr,
				void **clientDataPtr);
/* 33 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod2(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType2 *typePtr,
				void *clientData);
/* 34 */
TCLAPI Tcl_Method	Tcl_NewMethod2(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,
				const Tcl_MethodType2 *typePtr,
				void *clientData);

typedef struct {
    const struct TclOOIntStubs *tclOOIntStubs;
} TclOOStubHooks;

typedef struct TclOOStubs {
    int magic;
    const TclOOStubHooks *hooks;

    Tcl_Object (*tcl_CopyObjectInstance) (Tcl_Interp *interp, Tcl_Object sourceObject, const char *targetName, const char *targetNamespaceName); /* 0 */
    Tcl_Object (*tcl_GetClassAsObject) (Tcl_Class clazz); /* 1 */
    Tcl_Class (*tcl_GetObjectAsClass) (Tcl_Object object); /* 2 */
    Tcl_Command (*tcl_GetObjectCommand) (Tcl_Object object); /* 3 */
    Tcl_Object (*tcl_GetObjectFromObj) (Tcl_Interp *interp, Tcl_Obj *objPtr); /* 4 */
    Tcl_Namespace * (*tcl_GetObjectNamespace) (Tcl_Object object); /* 5 */
    Tcl_Class (*tcl_MethodDeclarerClass) (Tcl_Method method); /* 6 */
    Tcl_Object (*tcl_MethodDeclarerObject) (Tcl_Method method); /* 7 */
    int (*tcl_MethodIsPublic) (Tcl_Method method); /* 8 */
    int (*tcl_MethodIsType) (Tcl_Method method, const Tcl_MethodType *typePtr, void **clientDataPtr); /* 9 */
    Tcl_Obj * (*tcl_MethodName) (Tcl_Method method); /* 10 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 140–238. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h`.

```text
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType2 *typePtr,
				void *clientData);
/* 34 */
TCLAPI Tcl_Method	Tcl_NewMethod2(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,
				const Tcl_MethodType2 *typePtr,
				void *clientData);

typedef struct {
    const struct TclOOIntStubs *tclOOIntStubs;
} TclOOStubHooks;

typedef struct TclOOStubs {
    int magic;
    const TclOOStubHooks *hooks;

    Tcl_Object (*tcl_CopyObjectInstance) (Tcl_Interp *interp, Tcl_Object sourceObject, const char *targetName, const char *targetNamespaceName); /* 0 */
    Tcl_Object (*tcl_GetClassAsObject) (Tcl_Class clazz); /* 1 */
    Tcl_Class (*tcl_GetObjectAsClass) (Tcl_Object object); /* 2 */
    Tcl_Command (*tcl_GetObjectCommand) (Tcl_Object object); /* 3 */
    Tcl_Object (*tcl_GetObjectFromObj) (Tcl_Interp *interp, Tcl_Obj *objPtr); /* 4 */
    Tcl_Namespace * (*tcl_GetObjectNamespace) (Tcl_Object object); /* 5 */
    Tcl_Class (*tcl_MethodDeclarerClass) (Tcl_Method method); /* 6 */
    Tcl_Object (*tcl_MethodDeclarerObject) (Tcl_Method method); /* 7 */
    int (*tcl_MethodIsPublic) (Tcl_Method method); /* 8 */
    int (*tcl_MethodIsType) (Tcl_Method method, const Tcl_MethodType *typePtr, void **clientDataPtr); /* 9 */
    Tcl_Obj * (*tcl_MethodName) (Tcl_Method method); /* 10 */
    Tcl_Method (*tcl_NewInstanceMethod) (Tcl_Interp *interp, Tcl_Object object, Tcl_Obj *nameObj, int flags, const Tcl_MethodType *typePtr, void *clientData); /* 11 */
    Tcl_Method (*tcl_NewMethod) (Tcl_Interp *interp, Tcl_Class cls, Tcl_Obj *nameObj, int flags, const Tcl_MethodType *typePtr, void *clientData); /* 12 */
    Tcl_Object (*tcl_NewObjectInstance) (Tcl_Interp *interp, Tcl_Class cls, const char *nameStr, const char *nsNameStr, Tcl_Size objc, Tcl_Obj *const *objv, Tcl_Size skip); /* 13 */
    int (*tcl_ObjectDeleted) (Tcl_Object object); /* 14 */
    int (*tcl_ObjectContextIsFiltering) (Tcl_ObjectContext context); /* 15 */
    Tcl_Method (*tcl_ObjectContextMethod) (Tcl_ObjectContext context); /* 16 */
    Tcl_Object (*tcl_ObjectContextObject) (Tcl_ObjectContext context); /* 17 */
    Tcl_Size (*tcl_ObjectContextSkippedArgs) (Tcl_ObjectContext context); /* 18 */
    void * (*tcl_ClassGetMetadata) (Tcl_Class clazz, const Tcl_ObjectMetadataType *typePtr); /* 19 */
    void (*tcl_ClassSetMetadata) (Tcl_Class clazz, const Tcl_ObjectMetadataType *typePtr, void *metadata); /* 20 */
    void * (*tcl_ObjectGetMetadata) (Tcl_Object object, const Tcl_ObjectMetadataType *typePtr); /* 21 */
    void (*tcl_ObjectSetMetadata) (Tcl_Object object, const Tcl_ObjectMetadataType *typePtr, void *metadata); /* 22 */
    int (*tcl_ObjectContextInvokeNext) (Tcl_Interp *interp, Tcl_ObjectContext context, Tcl_Size objc, Tcl_Obj *const *objv, Tcl_Size skip); /* 23 */
    Tcl_ObjectMapMethodNameProc * (*tcl_ObjectGetMethodNameMapper) (Tcl_Object object); /* 24 */
    void (*tcl_ObjectSetMethodNameMapper) (Tcl_Object object, Tcl_ObjectMapMethodNameProc *mapMethodNameProc); /* 25 */
    void (*tcl_ClassSetConstructor) (Tcl_Interp *interp, Tcl_Class clazz, Tcl_Method method); /* 26 */
    void (*tcl_ClassSetDestructor) (Tcl_Interp *interp, Tcl_Class clazz, Tcl_Method method); /* 27 */
    Tcl_Obj * (*tcl_GetObjectName) (Tcl_Interp *interp, Tcl_Object object); /* 28 */
    int (*tcl_MethodIsPrivate) (Tcl_Method method); /* 29 */
    Tcl_Class (*tcl_GetClassOfObject) (Tcl_Object object); /* 30 */
    Tcl_Obj * (*tcl_GetObjectClassName) (Tcl_Interp *interp, Tcl_Object object); /* 31 */
    int (*tcl_MethodIsType2) (Tcl_Method method, const Tcl_MethodType2 *typePtr, void **clientDataPtr); /* 32 */
    Tcl_Method (*tcl_NewInstanceMethod2) (Tcl_Interp *interp, Tcl_Object object, Tcl_Obj *nameObj, int flags, const Tcl_MethodType2 *typePtr, void *clientData); /* 33 */
    Tcl_Method (*tcl_NewMethod2) (Tcl_Interp *interp, Tcl_Class cls, Tcl_Obj *nameObj, int flags, const Tcl_MethodType2 *typePtr, void *clientData); /* 34 */
} TclOOStubs;

extern const TclOOStubs *tclOOStubsPtr;

#ifdef __cplusplus
}
#endif

#if defined(USE_TCLOO_STUBS)

/*
 * Inline function declarations:
 */

#define Tcl_CopyObjectInstance \
	(tclOOStubsPtr->tcl_CopyObjectInstance) /* 0 */
#define Tcl_GetClassAsObject \
	(tclOOStubsPtr->tcl_GetClassAsObject) /* 1 */
#define Tcl_GetObjectAsClass \
	(tclOOStubsPtr->tcl_GetObjectAsClass) /* 2 */
#define Tcl_GetObjectCommand \
	(tclOOStubsPtr->tcl_GetObjectCommand) /* 3 */
#define Tcl_GetObjectFromObj \
	(tclOOStubsPtr->tcl_GetObjectFromObj) /* 4 */
#define Tcl_GetObjectNamespace \
	(tclOOStubsPtr->tcl_GetObjectNamespace) /* 5 */
#define Tcl_MethodDeclarerClass \
	(tclOOStubsPtr->tcl_MethodDeclarerClass) /* 6 */
#define Tcl_MethodDeclarerObject \
	(tclOOStubsPtr->tcl_MethodDeclarerObject) /* 7 */
#define Tcl_MethodIsPublic \
	(tclOOStubsPtr->tcl_MethodIsPublic) /* 8 */
#define Tcl_MethodIsType \
	(tclOOStubsPtr->tcl_MethodIsType) /* 9 */
#define Tcl_MethodName \
	(tclOOStubsPtr->tcl_MethodName) /* 10 */
#define Tcl_NewInstanceMethod \
	(tclOOStubsPtr->tcl_NewInstanceMethod) /* 11 */
#define Tcl_NewMethod \
	(tclOOStubsPtr->tcl_NewMethod) /* 12 */
#define Tcl_NewObjectInstance \
	(tclOOStubsPtr->tcl_NewObjectInstance) /* 13 */
#define Tcl_ObjectDeleted \
	(tclOOStubsPtr->tcl_ObjectDeleted) /* 14 */
#define Tcl_ObjectContextIsFiltering \
	(tclOOStubsPtr->tcl_ObjectContextIsFiltering) /* 15 */
#define Tcl_ObjectContextMethod \

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 226–293. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h`.

```text
#define Tcl_MethodName \
	(tclOOStubsPtr->tcl_MethodName) /* 10 */
#define Tcl_NewInstanceMethod \
	(tclOOStubsPtr->tcl_NewInstanceMethod) /* 11 */
#define Tcl_NewMethod \
	(tclOOStubsPtr->tcl_NewMethod) /* 12 */
#define Tcl_NewObjectInstance \
	(tclOOStubsPtr->tcl_NewObjectInstance) /* 13 */
#define Tcl_ObjectDeleted \
	(tclOOStubsPtr->tcl_ObjectDeleted) /* 14 */
#define Tcl_ObjectContextIsFiltering \
	(tclOOStubsPtr->tcl_ObjectContextIsFiltering) /* 15 */
#define Tcl_ObjectContextMethod \
	(tclOOStubsPtr->tcl_ObjectContextMethod) /* 16 */
#define Tcl_ObjectContextObject \
	(tclOOStubsPtr->tcl_ObjectContextObject) /* 17 */
#define Tcl_ObjectContextSkippedArgs \
	(tclOOStubsPtr->tcl_ObjectContextSkippedArgs) /* 18 */
#define Tcl_ClassGetMetadata \
	(tclOOStubsPtr->tcl_ClassGetMetadata) /* 19 */
#define Tcl_ClassSetMetadata \
	(tclOOStubsPtr->tcl_ClassSetMetadata) /* 20 */
#define Tcl_ObjectGetMetadata \
	(tclOOStubsPtr->tcl_ObjectGetMetadata) /* 21 */
#define Tcl_ObjectSetMetadata \
	(tclOOStubsPtr->tcl_ObjectSetMetadata) /* 22 */
#define Tcl_ObjectContextInvokeNext \
	(tclOOStubsPtr->tcl_ObjectContextInvokeNext) /* 23 */
#define Tcl_ObjectGetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectGetMethodNameMapper) /* 24 */
#define Tcl_ObjectSetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectSetMethodNameMapper) /* 25 */
#define Tcl_ClassSetConstructor \
	(tclOOStubsPtr->tcl_ClassSetConstructor) /* 26 */
#define Tcl_ClassSetDestructor \
	(tclOOStubsPtr->tcl_ClassSetDestructor) /* 27 */
#define Tcl_GetObjectName \
	(tclOOStubsPtr->tcl_GetObjectName) /* 28 */
#define Tcl_MethodIsPrivate \
	(tclOOStubsPtr->tcl_MethodIsPrivate) /* 29 */
#define Tcl_GetClassOfObject \
	(tclOOStubsPtr->tcl_GetClassOfObject) /* 30 */
#define Tcl_GetObjectClassName \
	(tclOOStubsPtr->tcl_GetObjectClassName) /* 31 */
#define Tcl_MethodIsType2 \
	(tclOOStubsPtr->tcl_MethodIsType2) /* 32 */
#define Tcl_NewInstanceMethod2 \
	(tclOOStubsPtr->tcl_NewInstanceMethod2) /* 33 */
#define Tcl_NewMethod2 \
	(tclOOStubsPtr->tcl_NewMethod2) /* 34 */

#endif /* defined(USE_TCLOO_STUBS) */

/* !END!: Do not edit above this line. */

#if TCL_MAJOR_VERSION < 9
    /* TIP #630 */
#   undef Tcl_MethodType2
#   undef Tcl_MethodIsType2
#   undef Tcl_NewInstanceMethod2
#   undef Tcl_NewMethod2
#endif
#ifdef _TCLSIZEHANDLED
#   undef _TCLSIZEHANDLED
#   undef Tcl_Size
#endif

#endif /* _TCLOODECLS */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 270–293. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h`.

```text
#define Tcl_MethodIsType2 \
	(tclOOStubsPtr->tcl_MethodIsType2) /* 32 */
#define Tcl_NewInstanceMethod2 \
	(tclOOStubsPtr->tcl_NewInstanceMethod2) /* 33 */
#define Tcl_NewMethod2 \
	(tclOOStubsPtr->tcl_NewMethod2) /* 34 */

#endif /* defined(USE_TCLOO_STUBS) */

/* !END!: Do not edit above this line. */

#if TCL_MAJOR_VERSION < 9
    /* TIP #630 */
#   undef Tcl_MethodType2
#   undef Tcl_MethodIsType2
#   undef Tcl_NewInstanceMethod2
#   undef Tcl_NewMethod2
#endif
#ifdef _TCLSIZEHANDLED
#   undef _TCLSIZEHANDLED
#   undef Tcl_Size
#endif

#endif /* _TCLOODECLS */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 282–293. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h`.

```text
    /* TIP #630 */
#   undef Tcl_MethodType2
#   undef Tcl_MethodIsType2
#   undef Tcl_NewInstanceMethod2
#   undef Tcl_NewMethod2
#endif
#ifdef _TCLSIZEHANDLED
#   undef _TCLSIZEHANDLED
#   undef Tcl_Size
#endif

#endif /* _TCLOODECLS */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODecls.h`, function `Tcl_GetObjectFromObj`, lines 47–145. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h`.

```text
TCLAPI Tcl_Class	Tcl_GetObjectAsClass(Tcl_Object object);
/* 3 */
TCLAPI Tcl_Command	Tcl_GetObjectCommand(Tcl_Object object);
/* 4 */
TCLAPI Tcl_Object	Tcl_GetObjectFromObj(Tcl_Interp *interp,
				Tcl_Obj *objPtr);
/* 5 */
TCLAPI Tcl_Namespace *	Tcl_GetObjectNamespace(Tcl_Object object);
/* 6 */
TCLAPI Tcl_Class	Tcl_MethodDeclarerClass(Tcl_Method method);
/* 7 */
TCLAPI Tcl_Object	Tcl_MethodDeclarerObject(Tcl_Method method);
/* 8 */
TCLAPI int		Tcl_MethodIsPublic(Tcl_Method method);
/* 9 */
TCLAPI int		Tcl_MethodIsType(Tcl_Method method,
				const Tcl_MethodType *typePtr,
				void **clientDataPtr);
/* 10 */
TCLAPI Tcl_Obj *	Tcl_MethodName(Tcl_Method method);
/* 11 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType *typePtr,
				void *clientData);
/* 12 */
TCLAPI Tcl_Method	Tcl_NewMethod(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,
				const Tcl_MethodType *typePtr,
				void *clientData);
/* 13 */
TCLAPI Tcl_Object	Tcl_NewObjectInstance(Tcl_Interp *interp,
				Tcl_Class cls, const char *nameStr,
				const char *nsNameStr, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 14 */
TCLAPI int		Tcl_ObjectDeleted(Tcl_Object object);
/* 15 */
TCLAPI int		Tcl_ObjectContextIsFiltering(
				Tcl_ObjectContext context);
/* 16 */
TCLAPI Tcl_Method	Tcl_ObjectContextMethod(Tcl_ObjectContext context);
/* 17 */
TCLAPI Tcl_Object	Tcl_ObjectContextObject(Tcl_ObjectContext context);
/* 18 */
TCLAPI Tcl_Size		Tcl_ObjectContextSkippedArgs(
				Tcl_ObjectContext context);
/* 19 */
TCLAPI void *		Tcl_ClassGetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr);
/* 20 */
TCLAPI void		Tcl_ClassSetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 21 */
TCLAPI void *		Tcl_ObjectGetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr);
/* 22 */
TCLAPI void		Tcl_ObjectSetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 23 */
TCLAPI int		Tcl_ObjectContextInvokeNext(Tcl_Interp *interp,
				Tcl_ObjectContext context, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 24 */
TCLAPI Tcl_ObjectMapMethodNameProc * Tcl_ObjectGetMethodNameMapper(
				Tcl_Object object);
/* 25 */
TCLAPI void		Tcl_ObjectSetMethodNameMapper(Tcl_Object object,
				Tcl_ObjectMapMethodNameProc *mapMethodNameProc);
/* 26 */
TCLAPI void		Tcl_ClassSetConstructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 27 */
TCLAPI void		Tcl_ClassSetDestructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 28 */
TCLAPI Tcl_Obj *	Tcl_GetObjectName(Tcl_Interp *interp,
				Tcl_Object object);
/* 29 */
TCLAPI int		Tcl_MethodIsPrivate(Tcl_Method method);
/* 30 */
TCLAPI Tcl_Class	Tcl_GetClassOfObject(Tcl_Object object);
/* 31 */
TCLAPI Tcl_Obj *	Tcl_GetObjectClassName(Tcl_Interp *interp,
				Tcl_Object object);
/* 32 */
TCLAPI int		Tcl_MethodIsType2(Tcl_Method method,
				const Tcl_MethodType2 *typePtr,
				void **clientDataPtr);
/* 33 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod2(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType2 *typePtr,
				void *clientData);
/* 34 */
TCLAPI Tcl_Method	Tcl_NewMethod2(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODecls.h`, function `Tcl_GetObjectFromObj`, lines 210–293. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h`.

```text
#define Tcl_GetObjectAsClass \
	(tclOOStubsPtr->tcl_GetObjectAsClass) /* 2 */
#define Tcl_GetObjectCommand \
	(tclOOStubsPtr->tcl_GetObjectCommand) /* 3 */
#define Tcl_GetObjectFromObj \
	(tclOOStubsPtr->tcl_GetObjectFromObj) /* 4 */
#define Tcl_GetObjectNamespace \
	(tclOOStubsPtr->tcl_GetObjectNamespace) /* 5 */
#define Tcl_MethodDeclarerClass \
	(tclOOStubsPtr->tcl_MethodDeclarerClass) /* 6 */
#define Tcl_MethodDeclarerObject \
	(tclOOStubsPtr->tcl_MethodDeclarerObject) /* 7 */
#define Tcl_MethodIsPublic \
	(tclOOStubsPtr->tcl_MethodIsPublic) /* 8 */
#define Tcl_MethodIsType \
	(tclOOStubsPtr->tcl_MethodIsType) /* 9 */
#define Tcl_MethodName \
	(tclOOStubsPtr->tcl_MethodName) /* 10 */
#define Tcl_NewInstanceMethod \
	(tclOOStubsPtr->tcl_NewInstanceMethod) /* 11 */
#define Tcl_NewMethod \
	(tclOOStubsPtr->tcl_NewMethod) /* 12 */
#define Tcl_NewObjectInstance \
	(tclOOStubsPtr->tcl_NewObjectInstance) /* 13 */
#define Tcl_ObjectDeleted \
	(tclOOStubsPtr->tcl_ObjectDeleted) /* 14 */
#define Tcl_ObjectContextIsFiltering \
	(tclOOStubsPtr->tcl_ObjectContextIsFiltering) /* 15 */
#define Tcl_ObjectContextMethod \
	(tclOOStubsPtr->tcl_ObjectContextMethod) /* 16 */
#define Tcl_ObjectContextObject \
	(tclOOStubsPtr->tcl_ObjectContextObject) /* 17 */
#define Tcl_ObjectContextSkippedArgs \
	(tclOOStubsPtr->tcl_ObjectContextSkippedArgs) /* 18 */
#define Tcl_ClassGetMetadata \
	(tclOOStubsPtr->tcl_ClassGetMetadata) /* 19 */
#define Tcl_ClassSetMetadata \
	(tclOOStubsPtr->tcl_ClassSetMetadata) /* 20 */
#define Tcl_ObjectGetMetadata \
	(tclOOStubsPtr->tcl_ObjectGetMetadata) /* 21 */
#define Tcl_ObjectSetMetadata \
	(tclOOStubsPtr->tcl_ObjectSetMetadata) /* 22 */
#define Tcl_ObjectContextInvokeNext \
	(tclOOStubsPtr->tcl_ObjectContextInvokeNext) /* 23 */
#define Tcl_ObjectGetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectGetMethodNameMapper) /* 24 */
#define Tcl_ObjectSetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectSetMethodNameMapper) /* 25 */
#define Tcl_ClassSetConstructor \
	(tclOOStubsPtr->tcl_ClassSetConstructor) /* 26 */
#define Tcl_ClassSetDestructor \
	(tclOOStubsPtr->tcl_ClassSetDestructor) /* 27 */
#define Tcl_GetObjectName \
	(tclOOStubsPtr->tcl_GetObjectName) /* 28 */
#define Tcl_MethodIsPrivate \
	(tclOOStubsPtr->tcl_MethodIsPrivate) /* 29 */
#define Tcl_GetClassOfObject \
	(tclOOStubsPtr->tcl_GetClassOfObject) /* 30 */
#define Tcl_GetObjectClassName \
	(tclOOStubsPtr->tcl_GetObjectClassName) /* 31 */
#define Tcl_MethodIsType2 \
	(tclOOStubsPtr->tcl_MethodIsType2) /* 32 */
#define Tcl_NewInstanceMethod2 \
	(tclOOStubsPtr->tcl_NewInstanceMethod2) /* 33 */
#define Tcl_NewMethod2 \
	(tclOOStubsPtr->tcl_NewMethod2) /* 34 */

#endif /* defined(USE_TCLOO_STUBS) */

/* !END!: Do not edit above this line. */

#if TCL_MAJOR_VERSION < 9
    /* TIP #630 */
#   undef Tcl_MethodType2
#   undef Tcl_MethodIsType2
#   undef Tcl_NewInstanceMethod2
#   undef Tcl_NewMethod2
#endif
#ifdef _TCLSIZEHANDLED
#   undef _TCLSIZEHANDLED
#   undef Tcl_Size
#endif

#endif /* _TCLOODECLS */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODecls.h`, function `Tcl_GetObjectAsClass`, lines 43–141. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h`.

```text
				const char *targetNamespaceName);
/* 1 */
TCLAPI Tcl_Object	Tcl_GetClassAsObject(Tcl_Class clazz);
/* 2 */
TCLAPI Tcl_Class	Tcl_GetObjectAsClass(Tcl_Object object);
/* 3 */
TCLAPI Tcl_Command	Tcl_GetObjectCommand(Tcl_Object object);
/* 4 */
TCLAPI Tcl_Object	Tcl_GetObjectFromObj(Tcl_Interp *interp,
				Tcl_Obj *objPtr);
/* 5 */
TCLAPI Tcl_Namespace *	Tcl_GetObjectNamespace(Tcl_Object object);
/* 6 */
TCLAPI Tcl_Class	Tcl_MethodDeclarerClass(Tcl_Method method);
/* 7 */
TCLAPI Tcl_Object	Tcl_MethodDeclarerObject(Tcl_Method method);
/* 8 */
TCLAPI int		Tcl_MethodIsPublic(Tcl_Method method);
/* 9 */
TCLAPI int		Tcl_MethodIsType(Tcl_Method method,
				const Tcl_MethodType *typePtr,
				void **clientDataPtr);
/* 10 */
TCLAPI Tcl_Obj *	Tcl_MethodName(Tcl_Method method);
/* 11 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType *typePtr,
				void *clientData);
/* 12 */
TCLAPI Tcl_Method	Tcl_NewMethod(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,
				const Tcl_MethodType *typePtr,
				void *clientData);
/* 13 */
TCLAPI Tcl_Object	Tcl_NewObjectInstance(Tcl_Interp *interp,
				Tcl_Class cls, const char *nameStr,
				const char *nsNameStr, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 14 */
TCLAPI int		Tcl_ObjectDeleted(Tcl_Object object);
/* 15 */
TCLAPI int		Tcl_ObjectContextIsFiltering(
				Tcl_ObjectContext context);
/* 16 */
TCLAPI Tcl_Method	Tcl_ObjectContextMethod(Tcl_ObjectContext context);
/* 17 */
TCLAPI Tcl_Object	Tcl_ObjectContextObject(Tcl_ObjectContext context);
/* 18 */
TCLAPI Tcl_Size		Tcl_ObjectContextSkippedArgs(
				Tcl_ObjectContext context);
/* 19 */
TCLAPI void *		Tcl_ClassGetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr);
/* 20 */
TCLAPI void		Tcl_ClassSetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 21 */
TCLAPI void *		Tcl_ObjectGetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr);
/* 22 */
TCLAPI void		Tcl_ObjectSetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 23 */
TCLAPI int		Tcl_ObjectContextInvokeNext(Tcl_Interp *interp,
				Tcl_ObjectContext context, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 24 */
TCLAPI Tcl_ObjectMapMethodNameProc * Tcl_ObjectGetMethodNameMapper(
				Tcl_Object object);
/* 25 */
TCLAPI void		Tcl_ObjectSetMethodNameMapper(Tcl_Object object,
				Tcl_ObjectMapMethodNameProc *mapMethodNameProc);
/* 26 */
TCLAPI void		Tcl_ClassSetConstructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 27 */
TCLAPI void		Tcl_ClassSetDestructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 28 */
TCLAPI Tcl_Obj *	Tcl_GetObjectName(Tcl_Interp *interp,
				Tcl_Object object);
/* 29 */
TCLAPI int		Tcl_MethodIsPrivate(Tcl_Method method);
/* 30 */
TCLAPI Tcl_Class	Tcl_GetClassOfObject(Tcl_Object object);
/* 31 */
TCLAPI Tcl_Obj *	Tcl_GetObjectClassName(Tcl_Interp *interp,
				Tcl_Object object);
/* 32 */
TCLAPI int		Tcl_MethodIsType2(Tcl_Method method,
				const Tcl_MethodType2 *typePtr,
				void **clientDataPtr);
/* 33 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod2(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType2 *typePtr,

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODecls.h`, function `Tcl_GetObjectAsClass`, lines 206–293. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h`.

```text
#define Tcl_CopyObjectInstance \
	(tclOOStubsPtr->tcl_CopyObjectInstance) /* 0 */
#define Tcl_GetClassAsObject \
	(tclOOStubsPtr->tcl_GetClassAsObject) /* 1 */
#define Tcl_GetObjectAsClass \
	(tclOOStubsPtr->tcl_GetObjectAsClass) /* 2 */
#define Tcl_GetObjectCommand \
	(tclOOStubsPtr->tcl_GetObjectCommand) /* 3 */
#define Tcl_GetObjectFromObj \
	(tclOOStubsPtr->tcl_GetObjectFromObj) /* 4 */
#define Tcl_GetObjectNamespace \
	(tclOOStubsPtr->tcl_GetObjectNamespace) /* 5 */
#define Tcl_MethodDeclarerClass \
	(tclOOStubsPtr->tcl_MethodDeclarerClass) /* 6 */
#define Tcl_MethodDeclarerObject \
	(tclOOStubsPtr->tcl_MethodDeclarerObject) /* 7 */
#define Tcl_MethodIsPublic \
	(tclOOStubsPtr->tcl_MethodIsPublic) /* 8 */
#define Tcl_MethodIsType \
	(tclOOStubsPtr->tcl_MethodIsType) /* 9 */
#define Tcl_MethodName \
	(tclOOStubsPtr->tcl_MethodName) /* 10 */
#define Tcl_NewInstanceMethod \
	(tclOOStubsPtr->tcl_NewInstanceMethod) /* 11 */
#define Tcl_NewMethod \
	(tclOOStubsPtr->tcl_NewMethod) /* 12 */
#define Tcl_NewObjectInstance \
	(tclOOStubsPtr->tcl_NewObjectInstance) /* 13 */
#define Tcl_ObjectDeleted \
	(tclOOStubsPtr->tcl_ObjectDeleted) /* 14 */
#define Tcl_ObjectContextIsFiltering \
	(tclOOStubsPtr->tcl_ObjectContextIsFiltering) /* 15 */
#define Tcl_ObjectContextMethod \
	(tclOOStubsPtr->tcl_ObjectContextMethod) /* 16 */
#define Tcl_ObjectContextObject \
	(tclOOStubsPtr->tcl_ObjectContextObject) /* 17 */
#define Tcl_ObjectContextSkippedArgs \
	(tclOOStubsPtr->tcl_ObjectContextSkippedArgs) /* 18 */
#define Tcl_ClassGetMetadata \
	(tclOOStubsPtr->tcl_ClassGetMetadata) /* 19 */
#define Tcl_ClassSetMetadata \
	(tclOOStubsPtr->tcl_ClassSetMetadata) /* 20 */
#define Tcl_ObjectGetMetadata \
	(tclOOStubsPtr->tcl_ObjectGetMetadata) /* 21 */
#define Tcl_ObjectSetMetadata \
	(tclOOStubsPtr->tcl_ObjectSetMetadata) /* 22 */
#define Tcl_ObjectContextInvokeNext \
	(tclOOStubsPtr->tcl_ObjectContextInvokeNext) /* 23 */
#define Tcl_ObjectGetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectGetMethodNameMapper) /* 24 */
#define Tcl_ObjectSetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectSetMethodNameMapper) /* 25 */
#define Tcl_ClassSetConstructor \
	(tclOOStubsPtr->tcl_ClassSetConstructor) /* 26 */
#define Tcl_ClassSetDestructor \
	(tclOOStubsPtr->tcl_ClassSetDestructor) /* 27 */
#define Tcl_GetObjectName \
	(tclOOStubsPtr->tcl_GetObjectName) /* 28 */
#define Tcl_MethodIsPrivate \
	(tclOOStubsPtr->tcl_MethodIsPrivate) /* 29 */
#define Tcl_GetClassOfObject \
	(tclOOStubsPtr->tcl_GetClassOfObject) /* 30 */
#define Tcl_GetObjectClassName \
	(tclOOStubsPtr->tcl_GetObjectClassName) /* 31 */
#define Tcl_MethodIsType2 \
	(tclOOStubsPtr->tcl_MethodIsType2) /* 32 */
#define Tcl_NewInstanceMethod2 \
	(tclOOStubsPtr->tcl_NewInstanceMethod2) /* 33 */
#define Tcl_NewMethod2 \
	(tclOOStubsPtr->tcl_NewMethod2) /* 34 */

#endif /* defined(USE_TCLOO_STUBS) */

/* !END!: Do not edit above this line. */

#if TCL_MAJOR_VERSION < 9
    /* TIP #630 */
#   undef Tcl_MethodType2
#   undef Tcl_MethodIsType2
#   undef Tcl_NewInstanceMethod2
#   undef Tcl_NewMethod2
#endif
#ifdef _TCLSIZEHANDLED
#   undef _TCLSIZEHANDLED
#   undef Tcl_Size
#endif

#endif /* _TCLOODECLS */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 85–183. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
static inline int	InitDefineContext(Tcl_Interp *interp,
			    Tcl_Namespace *namespacePtr, Object *oPtr,
			    int objc, Tcl_Obj *const objv[]);
static inline void	RecomputeClassCacheFlag(Object *oPtr);
static int		RenameDeleteMethod(Tcl_Interp *interp, Object *oPtr,
			    int useClass, Tcl_Obj *const fromPtr,
			    Tcl_Obj *const toPtr);
static int		Slot_Append(void *,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Slot_AppendNew(void *,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Slot_Clear(void *,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Slot_Prepend(void *,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Slot_Remove(void *,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Slot_Resolve(void *,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Slot_Set(void *,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Slot_Unimplemented(void *,
			    Tcl_Interp *interp, Tcl_ObjectContext,
			    int, Tcl_Obj *const *);
static int		Slot_Unknown(void *,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassFilter_Get(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassFilter_Set(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassMixin_Get(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassMixin_Set(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassSuper_Get(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassSuper_Set(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassVars_Get(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ClassVars_Set(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjFilter_Get(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjFilter_Set(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjMixin_Get(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjMixin_Set(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjVars_Get(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		ObjVars_Set(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Configurable_ClassReadableProps_Get(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Configurable_ClassReadableProps_Set(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Configurable_ClassWritableProps_Get(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Configurable_ClassWritableProps_Set(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Configurable_ObjectReadableProps_Get(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Configurable_ObjectReadableProps_Set(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Configurable_ObjectWritableProps_Get(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);
static int		Configurable_ObjectWritableProps_Set(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 731–828. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text

/*
 * ----------------------------------------------------------------------
 *
 * RenameDeleteMethod --
 *
 *	Core of the code to rename and delete methods.
 *
 * ----------------------------------------------------------------------
 */

static int
RenameDeleteMethod(
    Tcl_Interp *interp,
    Object *oPtr,
    int useClass,
    Tcl_Obj *const fromPtr,
    Tcl_Obj *const toPtr)
{
    Tcl_HashEntry *hPtr, *newHPtr = NULL;
    Method *mPtr;
    int isNew;

    if (!useClass) {
	if (!oPtr->methodsPtr) {
	noSuchMethod:
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "method %s does not exist", TclGetString(fromPtr)));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "METHOD",
		    TclGetString(fromPtr), (char *)NULL);
	    return TCL_ERROR;
	}
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(oPtr->methodsPtr, toPtr,
		    &isNew);
	    if (hPtr == newHPtr) {
	    renameToSelf:
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"cannot rename method to itself", TCL_AUTO_LENGTH));
		OO_ERROR(interp, RENAME_TO_SELF);
		return TCL_ERROR;
	    } else if (!isNew) {
	    renameToExisting:
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"method called %s already exists",
			TclGetString(toPtr)));
		OO_ERROR(interp, RENAME_OVER);
		return TCL_ERROR;
	    }
	}
    } else {
	hPtr = Tcl_FindHashEntry(&oPtr->classPtr->classMethods, fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(&oPtr->classPtr->classMethods,
		    (char *) toPtr, &isNew);
	    if (hPtr == newHPtr) {
		goto renameToSelf;
	    } else if (!isNew) {
		goto renameToExisting;
	    }
	}
    }

    /*
     * Complete the splicing by changing the method's name.
     */

    mPtr = (Method *) Tcl_GetHashValue(hPtr);
    if (toPtr) {
	Tcl_IncrRefCount(toPtr);
	Tcl_DecrRefCount(mPtr->namePtr);
	mPtr->namePtr = toPtr;
	Tcl_SetHashValue(newHPtr, mPtr);
    } else {
	if (!useClass) {
	    RecomputeClassCacheFlag(oPtr);
	}
	TclOODelMethodRef(mPtr);
    }
    Tcl_DeleteHashEntry(hPtr);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOUnknownDefinition --
 *
 *	Handles what happens when an unknown command is encountered during the
 *	processing of a definition script. Works by finding a command in the
 *	operating definition namespace that the requested command is a unique

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 739–836. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
 * ----------------------------------------------------------------------
 */

static int
RenameDeleteMethod(
    Tcl_Interp *interp,
    Object *oPtr,
    int useClass,
    Tcl_Obj *const fromPtr,
    Tcl_Obj *const toPtr)
{
    Tcl_HashEntry *hPtr, *newHPtr = NULL;
    Method *mPtr;
    int isNew;

    if (!useClass) {
	if (!oPtr->methodsPtr) {
	noSuchMethod:
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "method %s does not exist", TclGetString(fromPtr)));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "METHOD",
		    TclGetString(fromPtr), (char *)NULL);
	    return TCL_ERROR;
	}
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(oPtr->methodsPtr, toPtr,
		    &isNew);
	    if (hPtr == newHPtr) {
	    renameToSelf:
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"cannot rename method to itself", TCL_AUTO_LENGTH));
		OO_ERROR(interp, RENAME_TO_SELF);
		return TCL_ERROR;
	    } else if (!isNew) {
	    renameToExisting:
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"method called %s already exists",
			TclGetString(toPtr)));
		OO_ERROR(interp, RENAME_OVER);
		return TCL_ERROR;
	    }
	}
    } else {
	hPtr = Tcl_FindHashEntry(&oPtr->classPtr->classMethods, fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(&oPtr->classPtr->classMethods,
		    (char *) toPtr, &isNew);
	    if (hPtr == newHPtr) {
		goto renameToSelf;
	    } else if (!isNew) {
		goto renameToExisting;
	    }
	}
    }

    /*
     * Complete the splicing by changing the method's name.
     */

    mPtr = (Method *) Tcl_GetHashValue(hPtr);
    if (toPtr) {
	Tcl_IncrRefCount(toPtr);
	Tcl_DecrRefCount(mPtr->namePtr);
	mPtr->namePtr = toPtr;
	Tcl_SetHashValue(newHPtr, mPtr);
    } else {
	if (!useClass) {
	    RecomputeClassCacheFlag(oPtr);
	}
	TclOODelMethodRef(mPtr);
    }
    Tcl_DeleteHashEntry(hPtr);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOUnknownDefinition --
 *
 *	Handles what happens when an unknown command is encountered during the
 *	processing of a definition script. Works by finding a command in the
 *	operating definition namespace that the requested command is a unique
 *	prefix of.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOUnknownDefinition(
    TCL_UNUSED(void *),

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1964–2061. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDeleteMethodObjCmd --
 *
 *	Implementation of the "deletemethod" subcommand of the "oo::define"
 *	and "oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDeleteMethodObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceDeleteMethod = (clientData != NULL);
    Object *oPtr;
    int i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1973–2070. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
 * ----------------------------------------------------------------------
 */

int
TclOODefineDeleteMethodObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceDeleteMethod = (clientData != NULL);
    Object *oPtr;
    int i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1979–2076. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceDeleteMethod = (clientData != NULL);
    Object *oPtr;
    int i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a
     * destructor during a destructor is fairly dumb anyway.

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1992–2088. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a
     * destructor during a destructor is fairly dumb anyway.
     */

    Tcl_ClassSetDestructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineExportObjCmd --
 *
 *	Implementation of the "export" subcommand of the "oo::define" and

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 2004–2100. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a
     * destructor during a destructor is fairly dumb anyway.
     */

    Tcl_ClassSetDestructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineExportObjCmd --
 *
 *	Implementation of the "export" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineExportObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 2010–2106. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a
     * destructor during a destructor is fairly dumb anyway.
     */

    Tcl_ClassSetDestructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineExportObjCmd --
 *
 *	Implementation of the "export" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineExportObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceExport = (clientData != NULL);
    int i, changed = 0;
    Object *oPtr;
    Class *clsPtr;

    if (objc < 2) {

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 2475–2571. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
     * get the entries from the appropriate hash tables (this can generate a
     * range of errors...)
     */

    if (RenameDeleteMethod(interp, oPtr, !isInstanceRenameMethod,
	    objv[1], objv[2]) != TCL_OK) {
	return TCL_ERROR;
    }

    if (isInstanceRenameMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineUnexportObjCmd --
 *
 *	Implementation of the "unexport" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineUnexportObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceUnexport = (clientData != NULL);
    Object *oPtr;
    Class *clsPtr;
    int i, changed = 0;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;
    if (!isInstanceUnexport && !clsPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	if (isInstanceUnexport) {
	    changed |= UnexportInstanceMethod(oPtr, objv[i]);
	} else {
	    changed |= UnexportMethod(clsPtr, objv[i]);
	}
    }

    /*
     * Bump the right epoch if we actually changed anything.
     */

    if (changed) {
	if (isInstanceUnexport) {
	    BumpInstanceEpoch(oPtr);
	} else {
	    BumpGlobalEpoch(interp, clsPtr);
	}
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Tcl_ClassSetConstructor, Tcl_ClassSetDestructor --
 *
 *	How to install a constructor or destructor into a class; API to call
 *	from C.
 *
 * ----------------------------------------------------------------------
 */

void
Tcl_ClassSetConstructor(
    Tcl_Interp *interp,
    Tcl_Class clazz,
    Tcl_Method method)
{
    Class *clsPtr = (Class *) clazz;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 177–271. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	Tcl_Free(callPtr->chain);
    }
    Tcl_Free(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *
 *	Saves a reference to a method call context in a Tcl_Obj's internal
 *	representation.
 *
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    Tcl_ObjInternalRep ir;

    callPtr->refCount++;
    TclGetString(objPtr);
    ir.twoPtrValue.ptr1 = callPtr;
    Tcl_StoreInternalRep(objPtr, &methodNameType, &ir);
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}


```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 207–301. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	Tcl_Free(callPtr->chain);
    }
    Tcl_Free(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *
 *	Saves a reference to a method call context in a Tcl_Obj's internal
 *	representation.
 *
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    Tcl_ObjInternalRep ir;

    callPtr->refCount++;
    TclGetString(objPtr);
    ir.twoPtrValue.ptr1 = callPtr;
    Tcl_StoreInternalRep(objPtr, &methodNameType, &ir);
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 224–319. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	Tcl_Free(callPtr->chain);
    }
    Tcl_Free(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *
 *	Saves a reference to a method call context in a Tcl_Obj's internal
 *	representation.
 *
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    Tcl_ObjInternalRep ir;

    callPtr->refCount++;
    TclGetString(objPtr);
    ir.twoPtrValue.ptr1 = callPtr;
    Tcl_StoreInternalRep(objPtr, &methodNameType, &ir);
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */
int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    Tcl_Size objc,		/* The number of arguments. */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 292–389. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */
int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    Tcl_Size objc,		/* The number of arguments. */
    Tcl_Obj *const *objv)	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const bool isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL, NULL, NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags, contextPtr, NULL, NULL, NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

    /*
     * Run the method implementation.
     */

#ifndef TCL_NO_DEPRECATED
    if (mPtr->typePtr->version < TCL_OO_METHOD_VERSION_2) {
	return (mPtr->typePtr->callProc)(mPtr->clientData, interp,
		(Tcl_ObjectContext) contextPtr, (int)objc, objv);
    }
#endif /* TCL_NO_DEPRECATED */
    return (mPtr->type2Ptr->callProc)(mPtr->clientData, interp,
	    (Tcl_ObjectContext) contextPtr, objc, objv);
}

static int
SetFilterFlags(
    void *data[],

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1277–1375. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
		callPtr->refCount++;
		goto returnContext;
	    }
	    Tcl_SetHashValue(hPtr, NULL);
	    TclOODeleteChain(callPtr);
	}

	doFilters = true;
    }

    callPtr = (CallChain *) Tcl_Alloc(sizeof(CallChain));
    InitCallChain(callPtr, oPtr, flags);

    cb.callChainPtr = callPtr;
    cb.filterLength = 0;
    cb.oPtr = oPtr;

    /*
     * If we're working with a forced use of unknown, do that now.
     */

    if (flags & FORCE_UNKNOWN) {
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, BUILDING_MIXINS,
		NULL);
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (callPtr->numChain == 0) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	goto returnContext;
    }

    /*
     * Add all defined filters (if any, and if we're going to be processing
     * them; they're not processed for constructors, destructors or when we're
     * in the middle of processing a filter).
     */

    if (doFilters) {
	Tcl_Obj *filterObj;
	Class *mixinPtr;

	Tcl_InitObjHashTable(&doneFilters);
	FOREACH(mixinPtr, oPtr->mixins) {
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    TRAVERSED_MIXIN|BUILDING_MIXINS|OBJECT_MIXIN);
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    OBJECT_MIXIN);
	}
	FOREACH(filterObj, oPtr->filters) {
	    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
		    filterObj, &cb, &doneFilters, BUILDING_MIXINS, NULL);
	    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
		    filterObj, &cb, &doneFilters, 0, NULL);
	}
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		BUILDING_MIXINS);
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		0);
	Tcl_DeleteHashTable(&doneFilters);
    }
    count = cb.filterLength = callPtr->numChain;

    /*
     * Add the actual method implementations. We have to do this twice to
     * handle class mixins right.
     */

    if (oPtr == contextObj) {
	donePrivate |= AddInstancePrivateToCallContext(oPtr, methodNameObj,
		&cb, flags);
	donePrivate |= (contextObj->flags & HAS_PRIVATE_METHODS);
    }
    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
	    methodNameObj, &cb, NULL, flags|BUILDING_MIXINS, NULL);
    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
	    methodNameObj, &cb, NULL, flags, NULL);

    /*
     * Check to see if the method has no implementation. If so, we probably
     * need to add in a call to the unknown method. Otherwise, set up the
     * cacheing of the method implementation (if relevant).
     */

    if (count == callPtr->numChain) {
	/*
	 * Method does not actually exist. If we're dealing with constructors
	 * or destructors, this isn't a problem.
	 */

	if (flags & SPECIAL) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	AddSimpleChainToCallContext(oPtr, NULL,

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1303–1401. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (callPtr->numChain == 0) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	goto returnContext;
    }

    /*
     * Add all defined filters (if any, and if we're going to be processing
     * them; they're not processed for constructors, destructors or when we're
     * in the middle of processing a filter).
     */

    if (doFilters) {
	Tcl_Obj *filterObj;
	Class *mixinPtr;

	Tcl_InitObjHashTable(&doneFilters);
	FOREACH(mixinPtr, oPtr->mixins) {
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    TRAVERSED_MIXIN|BUILDING_MIXINS|OBJECT_MIXIN);
	    AddClassFiltersToCallContext(oPtr, mixinPtr, &cb, &doneFilters,
		    OBJECT_MIXIN);
	}
	FOREACH(filterObj, oPtr->filters) {
	    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
		    filterObj, &cb, &doneFilters, BUILDING_MIXINS, NULL);
	    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
		    filterObj, &cb, &doneFilters, 0, NULL);
	}
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		BUILDING_MIXINS);
	AddClassFiltersToCallContext(oPtr, oPtr->selfCls, &cb, &doneFilters,
		0);
	Tcl_DeleteHashTable(&doneFilters);
    }
    count = cb.filterLength = callPtr->numChain;

    /*
     * Add the actual method implementations. We have to do this twice to
     * handle class mixins right.
     */

    if (oPtr == contextObj) {
	donePrivate |= AddInstancePrivateToCallContext(oPtr, methodNameObj,
		&cb, flags);
	donePrivate |= (contextObj->flags & HAS_PRIVATE_METHODS);
    }
    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
	    methodNameObj, &cb, NULL, flags|BUILDING_MIXINS, NULL);
    donePrivate |= AddSimpleChainToCallContext(oPtr, contextCls,
	    methodNameObj, &cb, NULL, flags, NULL);

    /*
     * Check to see if the method has no implementation. If so, we probably
     * need to add in a call to the unknown method. Otherwise, set up the
     * cacheing of the method implementation (if relevant).
     */

    if (count == callPtr->numChain) {
	/*
	 * Method does not actually exist. If we're dealing with constructors
	 * or destructors, this isn't a problem.
	 */

	if (flags & SPECIAL) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, BUILDING_MIXINS,
		NULL);
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else if (doFilters && !donePrivate) {
	if (hPtr == NULL) {
	    int isNew;
	    if (oPtr->flags & USE_CLASS_CACHE) {
		if (oPtr->selfCls->classChainCache == NULL) {
		    oPtr->selfCls->classChainCache = (Tcl_HashTable *)
			    Tcl_Alloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->selfCls->classChainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->selfCls->classChainCache,
			methodNameObj, &isNew);
	    } else {
		if (oPtr->chainCache == NULL) {
		    oPtr->chainCache = (Tcl_HashTable *)
			    Tcl_Alloc(sizeof(Tcl_HashTable));

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1368–1465. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
	 * or destructors, this isn't a problem.
	 */

	if (flags & SPECIAL) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, BUILDING_MIXINS,
		NULL);
	AddSimpleChainToCallContext(oPtr, NULL,
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else if (doFilters && !donePrivate) {
	if (hPtr == NULL) {
	    int isNew;
	    if (oPtr->flags & USE_CLASS_CACHE) {
		if (oPtr->selfCls->classChainCache == NULL) {
		    oPtr->selfCls->classChainCache = (Tcl_HashTable *)
			    Tcl_Alloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->selfCls->classChainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->selfCls->classChainCache,
			methodNameObj, &isNew);
	    } else {
		if (oPtr->chainCache == NULL) {
		    oPtr->chainCache = (Tcl_HashTable *)
			    Tcl_Alloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->chainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->chainCache,
			methodNameObj, &isNew);
	    }
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)
	    TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1379–1476. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
		oPtr->fPtr->unknownMethodNameObj, &cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else if (doFilters && !donePrivate) {
	if (hPtr == NULL) {
	    int isNew;
	    if (oPtr->flags & USE_CLASS_CACHE) {
		if (oPtr->selfCls->classChainCache == NULL) {
		    oPtr->selfCls->classChainCache = (Tcl_HashTable *)
			    Tcl_Alloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->selfCls->classChainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->selfCls->classChainCache,
			methodNameObj, &isNew);
	    } else {
		if (oPtr->chainCache == NULL) {
		    oPtr->chainCache = (Tcl_HashTable *)
			    Tcl_Alloc(sizeof(Tcl_HashTable));

		    Tcl_InitObjHashTable(oPtr->chainCache);
		}
		hPtr = Tcl_CreateHashEntry(oPtr->chainCache,
			methodNameObj, &isNew);
	    }
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)
	    TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    ChainBuilder cb;
    Tcl_Size count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1410–1507. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)
	    TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    ChainBuilder cb;
    Tcl_Size count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply
     * can't get stereotypes. [Bug 7842f33a5c]
     */

    if (clsPtr == NULL) {
	return NULL;
    }

    /*
     * Synthesize a temporary stereotypical object so that we can use existing
     * machinery to produce the stereotypical call chain.
     */

    memset(&obj, 0, sizeof(Object));
    obj.fPtr = fPtr;
    obj.selfCls = clsPtr;
    obj.refCount = 1;
    obj.flags = USE_CLASS_CACHE;

    /*
     * Check if we can get the chain out of the Tcl_Obj method name or out of
     * the cache. This is made a bit more complex by the fact that there are
     * multiple different layers of cache (in the Tcl_Obj, in the object, and
     * in the class).
     */

    if (clsPtr->classChainCache != NULL) {
	hPtr = Tcl_FindHashEntry(clsPtr->classChainCache,
		methodNameObj);
	if (hPtr != NULL && Tcl_GetHashValue(hPtr) != NULL) {
	    const int reuseMask = (WANT_PUBLIC(flags) ? ~0 : ~PUBLIC_METHOD);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1416–1513. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)
	    TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    ChainBuilder cb;
    Tcl_Size count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply
     * can't get stereotypes. [Bug 7842f33a5c]
     */

    if (clsPtr == NULL) {
	return NULL;
    }

    /*
     * Synthesize a temporary stereotypical object so that we can use existing
     * machinery to produce the stereotypical call chain.
     */

    memset(&obj, 0, sizeof(Object));
    obj.fPtr = fPtr;
    obj.selfCls = clsPtr;
    obj.refCount = 1;
    obj.flags = USE_CLASS_CACHE;

    /*
     * Check if we can get the chain out of the Tcl_Obj method name or out of
     * the cache. This is made a bit more complex by the fact that there are
     * multiple different layers of cache (in the Tcl_Obj, in the object, and
     * in the class).
     */

    if (clsPtr->classChainCache != NULL) {
	hPtr = Tcl_FindHashEntry(clsPtr->classChainCache,
		methodNameObj);
	if (hPtr != NULL && Tcl_GetHashValue(hPtr) != NULL) {
	    const int reuseMask = (WANT_PUBLIC(flags) ? ~0 : ~PUBLIC_METHOD);

	    callPtr = (CallChain *) Tcl_GetHashValue(hPtr);
	    if (IsStillValid(callPtr, &obj, flags, reuseMask)) {
		callPtr->refCount++;
		return callPtr;
	    }
	    Tcl_SetHashValue(hPtr, NULL);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1510–1607. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
		callPtr->refCount++;
		return callPtr;
	    }
	    Tcl_SetHashValue(hPtr, NULL);
	    TclOODeleteChain(callPtr);
	}
    } else {
	hPtr = NULL;
    }

    callPtr = (CallChain *) Tcl_Alloc(sizeof(CallChain));
    memset(callPtr, 0, sizeof(CallChain));
    callPtr->flags = flags & (PUBLIC_METHOD|PRIVATE_METHOD|FILTER_HANDLING);
    callPtr->epoch = fPtr->epoch;
    callPtr->objectCreationEpoch = fPtr->tsdPtr->nsCount;
    callPtr->objectEpoch = clsPtr->thisPtr->epoch;
    callPtr->refCount = 1;
    callPtr->chain = callPtr->staticChain;

    cb.callChainPtr = callPtr;
    cb.filterLength = 0;
    cb.oPtr = &obj;

    /*
     * Add all defined filters (if any, and if we're going to be processing
     * them; they're not processed for constructors, destructors or when we're
     * in the middle of processing a filter).
     */

    Tcl_InitObjHashTable(&doneFilters);
    AddClassFiltersToCallContext(&obj, clsPtr, &cb, &doneFilters,
	    BUILDING_MIXINS);
    AddClassFiltersToCallContext(&obj, clsPtr, &cb, &doneFilters, 0);
    Tcl_DeleteHashTable(&doneFilters);
    count = cb.filterLength = callPtr->numChain;

    /*
     * Add the actual method implementations.
     */

    AddSimpleChainToCallContext(&obj, NULL, methodNameObj, &cb, NULL,
	    flags|BUILDING_MIXINS, NULL);
    AddSimpleChainToCallContext(&obj, NULL, methodNameObj, &cb, NULL, flags,
	    NULL);

    /*
     * Check to see if the method has no implementation. If so, we probably
     * need to add in a call to the unknown method. Otherwise, set up the
     * caching of the method implementation (if relevant).
     */

    if (count == callPtr->numChain) {
	AddSimpleChainToCallContext(&obj, NULL, fPtr->unknownMethodNameObj,
		&cb, NULL, BUILDING_MIXINS, NULL);
	AddSimpleChainToCallContext(&obj, NULL, fPtr->unknownMethodNameObj,
		&cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else {
	if (hPtr == NULL) {
	    if (clsPtr->classChainCache == NULL) {
		clsPtr->classChainCache = (Tcl_HashTable *)
			Tcl_Alloc(sizeof(Tcl_HashTable));
		Tcl_InitObjHashTable(clsPtr->classChainCache);
	    }
	    hPtr = Tcl_CreateHashEntry(clsPtr->classChainCache,
		    methodNameObj, NULL);
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(methodNameObj, callPtr);
    }
    return callPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddClassFiltersToCallContext --
 *
 *	Logic to make extracting all the filters from the class context much
 *	easier.
 *
 * ----------------------------------------------------------------------
 */

static void
AddClassFiltersToCallContext(
    Object *const oPtr,		/* Object that the filters operate on. */
    Class *clsPtr,		/* Class to get the filters from. */
    ChainBuilder *const cbPtr,	/* Context to fill with call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. Keys are objects, values are

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOODeleteChain(`, lines 1565–1662. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
		&cb, NULL, 0, NULL);
	callPtr->flags |= OO_UNKNOWN_METHOD;
	callPtr->epoch = 0;
	if (count == callPtr->numChain) {
	    TclOODeleteChain(callPtr);
	    return NULL;
	}
    } else {
	if (hPtr == NULL) {
	    if (clsPtr->classChainCache == NULL) {
		clsPtr->classChainCache = (Tcl_HashTable *)
			Tcl_Alloc(sizeof(Tcl_HashTable));
		Tcl_InitObjHashTable(clsPtr->classChainCache);
	    }
	    hPtr = Tcl_CreateHashEntry(clsPtr->classChainCache,
		    methodNameObj, NULL);
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(methodNameObj, callPtr);
    }
    return callPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddClassFiltersToCallContext --
 *
 *	Logic to make extracting all the filters from the class context much
 *	easier.
 *
 * ----------------------------------------------------------------------
 */

static void
AddClassFiltersToCallContext(
    Object *const oPtr,		/* Object that the filters operate on. */
    Class *clsPtr,		/* Class to get the filters from. */
    ChainBuilder *const cbPtr,	/* Context to fill with call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. Keys are objects, values are
				 * ignored. */
    int flags)			/* Whether we've gone along a mixin link
				 * yet. */
{
    Tcl_Size i;
    int clearedFlags =
	    flags & ~(TRAVERSED_MIXIN|OBJECT_MIXIN|BUILDING_MIXINS);
    Class *superPtr, *mixinPtr;
    Tcl_Obj *filterObj;

  tailRecurse:
    if (clsPtr == NULL) {
	return;
    }

    /*
     * Add all the filters defined by classes mixed into the main class
     * hierarchy.
     */

    FOREACH(mixinPtr, clsPtr->mixins) {
	AddClassFiltersToCallContext(oPtr, mixinPtr, cbPtr, doneFilters,
		flags|TRAVERSED_MIXIN);
    }

    /*
     * Add all the class filters from the current class. Note that the filters
     * are added starting at the object root, as this allows the object to
     * override how filters work to extend their behaviour.
     */

    if (MIXIN_CONSISTENT(flags)) {
	FOREACH(filterObj, clsPtr->filters) {
	    int isNew;

	    (void) Tcl_CreateHashEntry(doneFilters, filterObj, &isNew);
	    if (isNew) {
		AddSimpleChainToCallContext(oPtr, NULL, filterObj, cbPtr,
			doneFilters, clearedFlags|BUILDING_MIXINS, clsPtr);
		AddSimpleChainToCallContext(oPtr, NULL, filterObj, cbPtr,
			doneFilters, clearedFlags, clsPtr);
	    }
	}
    }

    /*
     * Now process the recursive case. Notice the tail-call optimization.
     */

    switch (clsPtr->superclasses.num) {
    case 1:
	clsPtr = clsPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, clsPtr->superclasses) {

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `StashCallChain(`, lines 148–242. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static Tcl_Size	SortMethodNames(Tcl_HashTable *namesPtr, int flags,
			    Tcl_Obj ***stringsPtr);
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */
static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,			// UpdateString
    NULL,			// SetFromAny
    TCL_OBJTYPE_V0
};

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	Tcl_Free(callPtr->chain);
    }
    Tcl_Free(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `StashCallChain(`, lines 248–344. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

static inline void
StashCallChain(
    Tcl_Obj *objPtr,
    CallChain *callPtr)
{
    Tcl_ObjInternalRep ir;

    callPtr->refCount++;
    TclGetString(objPtr);
    ir.twoPtrValue.ptr1 = callPtr;
    Tcl_StoreInternalRep(objPtr, &methodNameType, &ir);
}

void
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */
int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    Tcl_Size objc,		/* The number of arguments. */
    Tcl_Obj *const *objv)	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const bool isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `StashCallChain(`, lines 265–361. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
TclOOStashContext(
    Tcl_Obj *objPtr,
    CallContext *contextPtr)
{
    StashCallChain(objPtr, contextPtr->callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * DupMethodNameRep, FreeMethodNameRep --
 *
 *	Functions to implement the required parts of the Tcl_Obj guts needed
 *	for caching of method contexts in Tcl_Objs.
 *
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */
int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    Tcl_Size objc,		/* The number of arguments. */
    Tcl_Obj *const *objv)	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const bool isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `StashCallChain(`, lines 284–381. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */
int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    Tcl_Size objc,		/* The number of arguments. */
    Tcl_Obj *const *objv)	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const bool isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL, NULL, NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags, contextPtr, NULL, NULL, NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

    /*
     * Run the method implementation.
     */

#ifndef TCL_NO_DEPRECATED
    if (mPtr->typePtr->version < TCL_OO_METHOD_VERSION_2) {
	return (mPtr->typePtr->callProc)(mPtr->clientData, interp,
		(Tcl_ObjectContext) contextPtr, (int)objc, objv);
    }

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `StashCallChain(`, lines 1407–1504. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
	    }
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(cacheInThisObj, callPtr);
    } else if (flags & CONSTRUCTOR) {
	if (oPtr->selfCls->constructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->constructorChainPtr);
	}
	oPtr->selfCls->constructorChainPtr = callPtr;
	callPtr->refCount++;
    } else if ((flags & DESTRUCTOR) && oPtr->mixins.num == 0) {
	if (oPtr->selfCls->destructorChainPtr) {
	    TclOODeleteChain(oPtr->selfCls->destructorChainPtr);
	}
	oPtr->selfCls->destructorChainPtr = callPtr;
	callPtr->refCount++;
    }

  returnContext:
    contextPtr = (CallContext *)
	    TclStackAlloc(oPtr->fPtr->interp, sizeof(CallContext));
    contextPtr->oPtr = oPtr;

    /*
     * Corresponding TclOODecrRefCount() in TclOODeleteContext
     */

    AddRef(oPtr);
    contextPtr->callPtr = callPtr;
    contextPtr->skip = 2;
    contextPtr->index = 0;
    return contextPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetStereotypeCallChain --
 *
 *	Construct a call-chain for a method that would be used by a
 *	stereotypical instance of the given class (i.e., where the object has
 *	no definitions special to itself).
 *
 * ----------------------------------------------------------------------
 */

CallChain *
TclOOGetStereotypeCallChain(
    Class *clsPtr,		/* The object to get the context for. */
    Tcl_Obj *methodNameObj,	/* The name of the method to get the context
				 * for. NULL when getting a constructor or
				 * destructor chain. */
    int flags)			/* What sort of context are we looking for.
				 * Only the bits PUBLIC_METHOD, CONSTRUCTOR,
				 * PRIVATE_METHOD, DESTRUCTOR and
				 * FILTER_HANDLING are useful. */
{
    CallChain *callPtr;
    ChainBuilder cb;
    Tcl_Size count;
    Foundation *fPtr = clsPtr->thisPtr->fPtr;
    Tcl_HashEntry *hPtr;
    Tcl_HashTable doneFilters;
    Object obj;

    /*
     * Note that it's possible to end up with a NULL clsPtr here if there is
     * a call into stereotypical object after it has finished running its
     * destructor phase. It's quite a tangle, but at that point, we simply
     * can't get stereotypes. [Bug 7842f33a5c]
     */

    if (clsPtr == NULL) {
	return NULL;
    }

    /*
     * Synthesize a temporary stereotypical object so that we can use existing
     * machinery to produce the stereotypical call chain.
     */

    memset(&obj, 0, sizeof(Object));
    obj.fPtr = fPtr;
    obj.selfCls = clsPtr;
    obj.refCount = 1;
    obj.flags = USE_CLASS_CACHE;

    /*
     * Check if we can get the chain out of the Tcl_Obj method name or out of
     * the cache. This is made a bit more complex by the fact that there are
     * multiple different layers of cache (in the Tcl_Obj, in the object, and
     * in the class).
     */

    if (clsPtr->classChainCache != NULL) {
	hPtr = Tcl_FindHashEntry(clsPtr->classChainCache,
		methodNameObj);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `StashCallChain(`, lines 1580–1676. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
		    methodNameObj, NULL);
	}
	callPtr->refCount++;
	Tcl_SetHashValue(hPtr, callPtr);
	StashCallChain(methodNameObj, callPtr);
    }
    return callPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddClassFiltersToCallContext --
 *
 *	Logic to make extracting all the filters from the class context much
 *	easier.
 *
 * ----------------------------------------------------------------------
 */

static void
AddClassFiltersToCallContext(
    Object *const oPtr,		/* Object that the filters operate on. */
    Class *clsPtr,		/* Class to get the filters from. */
    ChainBuilder *const cbPtr,	/* Context to fill with call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. Keys are objects, values are
				 * ignored. */
    int flags)			/* Whether we've gone along a mixin link
				 * yet. */
{
    Tcl_Size i;
    int clearedFlags =
	    flags & ~(TRAVERSED_MIXIN|OBJECT_MIXIN|BUILDING_MIXINS);
    Class *superPtr, *mixinPtr;
    Tcl_Obj *filterObj;

  tailRecurse:
    if (clsPtr == NULL) {
	return;
    }

    /*
     * Add all the filters defined by classes mixed into the main class
     * hierarchy.
     */

    FOREACH(mixinPtr, clsPtr->mixins) {
	AddClassFiltersToCallContext(oPtr, mixinPtr, cbPtr, doneFilters,
		flags|TRAVERSED_MIXIN);
    }

    /*
     * Add all the class filters from the current class. Note that the filters
     * are added starting at the object root, as this allows the object to
     * override how filters work to extend their behaviour.
     */

    if (MIXIN_CONSISTENT(flags)) {
	FOREACH(filterObj, clsPtr->filters) {
	    int isNew;

	    (void) Tcl_CreateHashEntry(doneFilters, filterObj, &isNew);
	    if (isNew) {
		AddSimpleChainToCallContext(oPtr, NULL, filterObj, cbPtr,
			doneFilters, clearedFlags|BUILDING_MIXINS, clsPtr);
		AddSimpleChainToCallContext(oPtr, NULL, filterObj, cbPtr,
			doneFilters, clearedFlags, clsPtr);
	    }
	}
    }

    /*
     * Now process the recursive case. Notice the tail-call optimization.
     */

    switch (clsPtr->superclasses.num) {
    case 1:
	clsPtr = clsPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, clsPtr->superclasses) {
	    AddClassFiltersToCallContext(oPtr, superPtr, cbPtr, doneFilters,
		    flags);
	}
	TCL_FALLTHROUGH();
    case 0:
	return;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * AddPrivatesFromClassChainToCallContext --
 *

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `FreeMethodNameRep(`, lines 141–236. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
			    DefineChain *const definePtr, int flags);
static int		CmpNames(const void *ptr1, const void *ptr2);
static void		DupMethodNameRep(Tcl_Obj *srcPtr, Tcl_Obj *dstPtr);
static Tcl_NRPostProc	FinalizeMethodRefs;
static void		FreeMethodNameRep(Tcl_Obj *objPtr);
static inline bool	IsStillValid(CallChain *callPtr, Object *oPtr,
			    int flags, int reuseMask);
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static Tcl_Size	SortMethodNames(Tcl_HashTable *namesPtr, int flags,
			    Tcl_Obj ***stringsPtr);
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */
static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,			// UpdateString
    NULL,			// SetFromAny
    TCL_OBJTYPE_V0
};

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	Tcl_Free(callPtr->chain);
    }

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `FreeMethodNameRep(`, lines 289–386. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */
int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    Tcl_Size objc,		/* The number of arguments. */
    Tcl_Obj *const *objv)	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const bool isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL, NULL, NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags, contextPtr, NULL, NULL, NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

    /*
     * Run the method implementation.
     */

#ifndef TCL_NO_DEPRECATED
    if (mPtr->typePtr->version < TCL_OO_METHOD_VERSION_2) {
	return (mPtr->typePtr->callProc)(mPtr->clientData, interp,
		(Tcl_ObjectContext) contextPtr, (int)objc, objv);
    }
#endif /* TCL_NO_DEPRECATED */
    return (mPtr->type2Ptr->callProc)(mPtr->clientData, interp,
	    (Tcl_ObjectContext) contextPtr, objc, objv);
}

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `DupMethodNameRep(`, lines 139–234. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
			    DefineChain *const definePtr, int flags);
static inline void	AddSimpleDefineNamespaces(Object *const oPtr,
			    DefineChain *const definePtr, int flags);
static int		CmpNames(const void *ptr1, const void *ptr2);
static void		DupMethodNameRep(Tcl_Obj *srcPtr, Tcl_Obj *dstPtr);
static Tcl_NRPostProc	FinalizeMethodRefs;
static void		FreeMethodNameRep(Tcl_Obj *objPtr);
static inline bool	IsStillValid(CallChain *callPtr, Object *oPtr,
			    int flags, int reuseMask);
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static Tcl_Size	SortMethodNames(Tcl_HashTable *namesPtr, int flags,
			    Tcl_Obj ***stringsPtr);
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */
static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,			// UpdateString
    NULL,			// SetFromAny
    TCL_OBJTYPE_V0
};

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `DupMethodNameRep(`, lines 280–377. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
 * ----------------------------------------------------------------------
 */

static void
DupMethodNameRep(
    Tcl_Obj *srcPtr,
    Tcl_Obj *dstPtr)
{
    StashCallChain(dstPtr, (CallChain *)
	    TclFetchInternalRep(srcPtr, &methodNameType)->twoPtrValue.ptr1);
}

static void
FreeMethodNameRep(
    Tcl_Obj *objPtr)
{
    TclOODeleteChain((CallChain *)
	    TclFetchInternalRep(objPtr, &methodNameType)->twoPtrValue.ptr1);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInvokeContext --
 *
 *	Invokes a single step along a method call-chain context. Note that the
 *	invocation of a step along the chain can cause further steps along the
 *	chain to be invoked. Note that this function is written to be as light
 *	in stack usage as possible.
 *
 * ----------------------------------------------------------------------
 */
int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    Tcl_Size objc,		/* The number of arguments. */
    Tcl_Obj *const *objv)	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const bool isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL, NULL, NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags, contextPtr, NULL, NULL, NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

    /*
     * Run the method implementation.
     */

#ifndef TCL_NO_DEPRECATED

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `TclOOInvokeContext(`, lines 309–407. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
 *
 * ----------------------------------------------------------------------
 */
int
TclOOInvokeContext(
    void *clientData,		/* The method call context. */
    Tcl_Interp *interp,		/* Interpreter for error reporting, and many
				 * other sorts of context handling (e.g.,
				 * commands, variables) depending on method
				 * implementation. */
    Tcl_Size objc,		/* The number of arguments. */
    Tcl_Obj *const *objv)	/* The arguments as actually seen. */
{
    CallContext *const contextPtr = (CallContext *) clientData;
    Method *const mPtr = contextPtr->callPtr->chain[contextPtr->index].mPtr;
    const bool isFilter =
	    contextPtr->callPtr->chain[contextPtr->index].isFilter;

    /*
     * If this is the first step along the chain, we preserve the method
     * entries in the chain so that they do not get deleted out from under our
     * feet.
     */

    if (contextPtr->index == 0) {
	Tcl_Size i;

	for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	    AddRef(contextPtr->callPtr->chain[i].mPtr);
	}

	/*
	 * Ensure that the method name itself is part of the arguments when
	 * we're doing unknown processing.
	 */

	if (contextPtr->callPtr->flags & OO_UNKNOWN_METHOD) {
	    contextPtr->skip--;
	}

	/*
	 * Add a callback to ensure that method references are dropped once
	 * this call is finished.
	 */

	TclNRAddCallback(interp, FinalizeMethodRefs, contextPtr, NULL, NULL,
		NULL);
    }

    /*
     * Save whether we were in a filter and set up whether we are now.
     */

    if (contextPtr->oPtr->flags & FILTER_HANDLING) {
	TclNRAddCallback(interp, SetFilterFlags, contextPtr, NULL, NULL, NULL);
    } else {
	TclNRAddCallback(interp, ResetFilterFlags, contextPtr, NULL, NULL, NULL);
    }
    if (isFilter || contextPtr->callPtr->flags & FILTER_HANDLING) {
	contextPtr->oPtr->flags |= FILTER_HANDLING;
    } else {
	contextPtr->oPtr->flags &= ~FILTER_HANDLING;
    }

    /*
     * Run the method implementation.
     */

#ifndef TCL_NO_DEPRECATED
    if (mPtr->typePtr->version < TCL_OO_METHOD_VERSION_2) {
	return (mPtr->typePtr->callProc)(mPtr->clientData, interp,
		(Tcl_ObjectContext) contextPtr, (int)objc, objv);
    }
#endif /* TCL_NO_DEPRECATED */
    return (mPtr->type2Ptr->callProc)(mPtr->clientData, interp,
	    (Tcl_ObjectContext) contextPtr, objc, objv);
}

static int
SetFilterFlags(
    void *data[],
    TCL_UNUSED(Tcl_Interp *),
    int result)
{
    CallContext *contextPtr = (CallContext *) data[0];

    contextPtr->oPtr->flags |= FILTER_HANDLING;
    return result;
}

static int
ResetFilterFlags(
    void *data[],
    TCL_UNUSED(Tcl_Interp *),
    int result)
{
    CallContext *contextPtr = (CallContext *) data[0];

    contextPtr->oPtr->flags &= ~FILTER_HANDLING;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `FinalizeMethodRefs(`, lines 408–504. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
    return result;
}

static int
FinalizeMethodRefs(
    void *data[],
    TCL_UNUSED(Tcl_Interp *),
    int result)
{
    CallContext *contextPtr = (CallContext *) data[0];
    Tcl_Size i;

    for (i = 0 ; i < contextPtr->callPtr->numChain ; i++) {
	TclOODelMethodRef(contextPtr->callPtr->chain[i].mPtr);
    }
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOContextTypeName --
 *
 *	Get the name of the (high-level) type of method that a context is
 *	processing. Used for error message generation.
 *
 * ----------------------------------------------------------------------
 */
const char *
TclOOContextTypeName(
    CallContext *contextPtr)
{
    if (contextPtr->callPtr->flags & CONSTRUCTOR) {
	return "constructor";
    } else if (contextPtr->callPtr->flags & DESTRUCTOR) {
	return "destructor";
    } else {
	return "method";
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetSortedMethodList, TclOOGetSortedClassMethodList --
 *
 *	Discovers the list of method names supported by an object or class.
 *
 * ----------------------------------------------------------------------
 */

Tcl_Size
TclOOGetSortedMethodList(
    Object *oPtr,		/* The object to get the method names for. */
    Object *contextObj,		/* From what context object we are inquiring.
				 * NULL when the context shouldn't see
				 * object-level private methods. Note that
				 * flags can override this. */
    Class *contextCls,		/* From what context class we are inquiring.
				 * NULL when the context shouldn't see
				 * class-level private methods. Note that
				 * flags can override this. */
    int flags,			/* Whether we just want the public method
				 * names. */
    Tcl_Obj ***namesLstPtr)	/* Where to write a pointer to the array of
				 * names to. */
{
    Tcl_HashTable names;	/* Tcl_Obj * method name to "wanted in list"
				 * mapping. */
    Tcl_HashTable examinedClasses;
				/* Used to track what classes have been looked
				 * at. Is set-like in nature and keyed by
				 * pointer to class. */
    FOREACH_HASH_DECLS;
    Tcl_Size i, numStrings;
    Class *mixinPtr;
    Tcl_Obj *namePtr;
    Method *mPtr;

    Tcl_InitObjHashTable(&names);
    Tcl_InitHashTable(&examinedClasses, TCL_ONE_WORD_KEYS);

    /*
     * Process method names due to the object.
     */

    if (oPtr->methodsPtr) {
	FOREACH_HASH(namePtr, mPtr, oPtr->methodsPtr) {
	    if (IS_PRIVATE(mPtr)) {
		continue;
	    }
	    if (IS_UNEXPORTED(mPtr) && !WANT_UNEXPORTED(flags)) {
		continue;
	    }
	    AddStandardMethodName(flags, namePtr, mPtr, &names);
	}
    }

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 106–202. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
			    Tcl_HashTable *const examinedClassesPtr);
static inline void	AddDefinitionNamespaceToChain(Class *const definerCls,
			    Tcl_Obj *const namespaceName,
			    DefineChain *const definePtr, int flags);
static inline void	AddMethodToCallChain(Method *const mPtr,
			    ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters,
			    Class *const filterDecl, int flags);
static inline bool	AddInstancePrivateToCallContext(Object *const oPtr,
			    Tcl_Obj *const methodNameObj,
			    ChainBuilder *const cbPtr, int flags);
static inline void	AddStandardMethodName(int flags, Tcl_Obj *namePtr,
			    Method *mPtr, Tcl_HashTable *namesPtr);
static inline void	AddPrivateMethodNames(Tcl_HashTable *methodsTablePtr,
			    Tcl_HashTable *namesPtr);
static inline bool	AddSimpleChainToCallContext(Object *const oPtr,
			    Class *const contextCls,
			    Tcl_Obj *const methodNameObj,
			    ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters, int flags,
			    Class *const filterDecl);
static bool		AddPrivatesFromClassChainToCallContext(Class *classPtr,
			    Class *const contextCls,
			    Tcl_Obj *const methodNameObj,
			    ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters, int flags,
			    Class *const filterDecl);
static bool		AddSimpleClassChainToCallContext(Class *classPtr,
			    Tcl_Obj *const methodNameObj,
			    ChainBuilder *const cbPtr,
			    Tcl_HashTable *const doneFilters, int flags,
			    Class *const filterDecl);
static void		AddSimpleClassDefineNamespaces(Class *classPtr,
			    DefineChain *const definePtr, int flags);
static inline void	AddSimpleDefineNamespaces(Object *const oPtr,
			    DefineChain *const definePtr, int flags);
static int		CmpNames(const void *ptr1, const void *ptr2);
static void		DupMethodNameRep(Tcl_Obj *srcPtr, Tcl_Obj *dstPtr);
static Tcl_NRPostProc	FinalizeMethodRefs;
static void		FreeMethodNameRep(Tcl_Obj *objPtr);
static inline bool	IsStillValid(CallChain *callPtr, Object *oPtr,
			    int flags, int reuseMask);
static Tcl_NRPostProc	ResetFilterFlags;
static Tcl_NRPostProc	SetFilterFlags;
static Tcl_Size	SortMethodNames(Tcl_HashTable *namesPtr, int flags,
			    Tcl_Obj ***stringsPtr);
static inline void	StashCallChain(Tcl_Obj *objPtr, CallChain *callPtr);

/*
 * Object type used to manage type caches attached to method names.
 */
static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,			// UpdateString
    NULL,			// SetFromAny
    TCL_OBJTYPE_V0
};

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */
void

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 860–957. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodName);
	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (IS_PRIVATE(mPtr)) {
		AddMethodToCallChain(mPtr, cbPtr, NULL, NULL, flags);
		donePrivate = true;
	    }
	}
    }
    return donePrivate;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddSimpleChainToCallContext --
 *
 *	The core of the call-chain construction engine, this handles calling a
 *	particular method on a particular object. Note that filters and
 *	unknown handling are already handled by the logic that uses this
 *	function. Returns true if a private method was one of those found.
 *
 * ----------------------------------------------------------------------
 */
static inline bool
AddSimpleChainToCallContext(
    Object *const oPtr,		/* Object to add call chain entries for. */
    Class *const contextCls,	/* Context class; the currently considered
				 * class is equal to this, private methods may
				 * also be added. [TIP 500] */
    Tcl_Obj *const methodNameObj,
				/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what call chain entries
				 * have been processed. */
    int flags,			/* What sort of call chain are we building. */
    Class *const filterDecl)	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
{
    Tcl_Size i;
    bool foundPrivate = false, blockedUnexported = false;
    Tcl_HashEntry *hPtr;
    Method *mPtr;

    if (!(flags & (KNOWN_STATE | SPECIAL)) && oPtr->methodsPtr) {
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);

	if (hPtr != NULL) {
	    mPtr = (Method *) Tcl_GetHashValue(hPtr);
	    if (!IS_PRIVATE(mPtr)) {
		if (WANT_PUBLIC(flags)) {
		    if (!IS_PUBLIC(mPtr)) {
			blockedUnexported = true;
		    } else {
			flags |= DEFINITE_PUBLIC;
		    }
		} else {
		    flags |= DEFINITE_PROTECTED;
		}
	    }
	}
    }
    if (!(flags & SPECIAL)) {
	Class *mixinPtr;

	FOREACH(mixinPtr, oPtr->mixins) {
	    if (contextCls) {
		foundPrivate |= AddPrivatesFromClassChainToCallContext(
			mixinPtr, contextCls, methodNameObj, cbPtr,
			doneFilters, flags|TRAVERSED_MIXIN, filterDecl);
	    }
	    foundPrivate |= AddSimpleClassChainToCallContext(mixinPtr,
		    methodNameObj, cbPtr, doneFilters,
		    flags | TRAVERSED_MIXIN, filterDecl);
	}
	if (oPtr->methodsPtr && !blockedUnexported) {
	    hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);
	    if (hPtr != NULL) {
		mPtr = (Method *) Tcl_GetHashValue(hPtr);
		if (!IS_PRIVATE(mPtr)) {
		    AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			    flags);
		}
	    }
	}
    }
    if (!oPtr->selfCls) {
	return foundPrivate;
    }
    if (contextCls) {
	foundPrivate |= AddPrivatesFromClassChainToCallContext(oPtr->selfCls,
		contextCls, methodNameObj, cbPtr, doneFilters, flags,
		filterDecl);
    }
    if (!blockedUnexported) {

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 939–1036. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
	    hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodNameObj);
	    if (hPtr != NULL) {
		mPtr = (Method *) Tcl_GetHashValue(hPtr);
		if (!IS_PRIVATE(mPtr)) {
		    AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			    flags);
		}
	    }
	}
    }
    if (!oPtr->selfCls) {
	return foundPrivate;
    }
    if (contextCls) {
	foundPrivate |= AddPrivatesFromClassChainToCallContext(oPtr->selfCls,
		contextCls, methodNameObj, cbPtr, doneFilters, flags,
		filterDecl);
    }
    if (!blockedUnexported) {
	foundPrivate |= AddSimpleClassChainToCallContext(oPtr->selfCls,
		methodNameObj, cbPtr, doneFilters, flags, filterDecl);
    }
    return foundPrivate;
}

/*
 * ----------------------------------------------------------------------
 *
 * AddMethodToCallChain --
 *
 *	Utility method that manages the adding of a particular method
 *	implementation to a call-chain.
 *
 * ----------------------------------------------------------------------
 */
static inline void
AddMethodToCallChain(
    Method *const mPtr,		/* Actual method implementation to add to call
				 * chain (or NULL, a no-op). */
    ChainBuilder *const cbPtr,	/* The call chain to add the method
				 * implementation to. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. If NULL, not processing filters.
				 * Note that this function does not update
				 * this hashtable. */
    Class *const filterDecl,	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
    int flags)			/* Used to check if we're mixin-consistent
				 * only. Mixin-consistent means that either
				 * we're looking to add things from a mixin
				 * and we have passed a mixin, or we're not
				 * looking to add things from a mixin and have
				 * not passed a mixin. */
{
    CallChain *callPtr = cbPtr->callChainPtr;
    Tcl_Size i;

    /*
     * Return if this is just an entry used to record whether this is a public
     * method. If so, there's nothing real to call and so nothing to add to
     * the call chain.
     *
     * This is also where we enforce mixin-consistency.
     */

    if (mPtr == NULL || mPtr->type2Ptr == NULL || !MIXIN_CONSISTENT(flags)) {
	return;
    }

    /*
     * Enforce real private method handling here. We will skip adding this
     * method IF
     *  1) we are not allowing private methods, AND
     *  2) this is a private method, AND
     *  3) this is a class method, AND
     *  4) this method was not declared by the class of the current object.
     *
     * This does mean that only classes really handle private methods. This
     * should be sufficient for [incr Tcl] support though.
     */

    if (!WANT_UNEXPORTED(callPtr->flags)
	    && IS_UNEXPORTED(mPtr)
	    && (mPtr->declaringClassPtr != NULL)
	    && (mPtr->declaringClassPtr != cbPtr->oPtr->selfCls)) {
	return;
    }

    /*
     * First test whether the method is already in the call chain. Skip over
     * any leading filters.
     */

    for (i = cbPtr->filterLength ; i < callPtr->numChain ; i++) {
	if (callPtr->chain[i].mPtr == mPtr &&
		callPtr->chain[i].isFilter == (doneFilters != NULL)) {

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 971–1069. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
 *
 * ----------------------------------------------------------------------
 */
static inline void
AddMethodToCallChain(
    Method *const mPtr,		/* Actual method implementation to add to call
				 * chain (or NULL, a no-op). */
    ChainBuilder *const cbPtr,	/* The call chain to add the method
				 * implementation to. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what filters have been
				 * processed. If NULL, not processing filters.
				 * Note that this function does not update
				 * this hashtable. */
    Class *const filterDecl,	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
    int flags)			/* Used to check if we're mixin-consistent
				 * only. Mixin-consistent means that either
				 * we're looking to add things from a mixin
				 * and we have passed a mixin, or we're not
				 * looking to add things from a mixin and have
				 * not passed a mixin. */
{
    CallChain *callPtr = cbPtr->callChainPtr;
    Tcl_Size i;

    /*
     * Return if this is just an entry used to record whether this is a public
     * method. If so, there's nothing real to call and so nothing to add to
     * the call chain.
     *
     * This is also where we enforce mixin-consistency.
     */

    if (mPtr == NULL || mPtr->type2Ptr == NULL || !MIXIN_CONSISTENT(flags)) {
	return;
    }

    /*
     * Enforce real private method handling here. We will skip adding this
     * method IF
     *  1) we are not allowing private methods, AND
     *  2) this is a private method, AND
     *  3) this is a class method, AND
     *  4) this method was not declared by the class of the current object.
     *
     * This does mean that only classes really handle private methods. This
     * should be sufficient for [incr Tcl] support though.
     */

    if (!WANT_UNEXPORTED(callPtr->flags)
	    && IS_UNEXPORTED(mPtr)
	    && (mPtr->declaringClassPtr != NULL)
	    && (mPtr->declaringClassPtr != cbPtr->oPtr->selfCls)) {
	return;
    }

    /*
     * First test whether the method is already in the call chain. Skip over
     * any leading filters.
     */

    for (i = cbPtr->filterLength ; i < callPtr->numChain ; i++) {
	if (callPtr->chain[i].mPtr == mPtr &&
		callPtr->chain[i].isFilter == (doneFilters != NULL)) {
	    /*
	     * Call chain semantics states that methods come as *late* in the
	     * call chain as possible. This is done by copying down the
	     * following methods. Note that this does not change the number of
	     * method invocations in the call chain; it just rearranges them.
	     */

	    Class *declCls = callPtr->chain[i].filterDeclarer;

	    for (; i + 1 < callPtr->numChain ; i++) {
		callPtr->chain[i] = callPtr->chain[i + 1];
	    }
	    callPtr->chain[i].mPtr = mPtr;
	    callPtr->chain[i].isFilter = (doneFilters != NULL);
	    callPtr->chain[i].filterDeclarer = declCls;
	    return;
	}
    }

    /*
     * Need to really add the method. This is made a bit more complex by the
     * fact that we are using some "static" space initially, and only start
     * realloc-ing if the chain gets long.
     */

    if (callPtr->numChain == CALL_CHAIN_STATIC_SIZE) {
	callPtr->chain = (MInvoke *)
		Tcl_Alloc(sizeof(MInvoke) * (callPtr->numChain + 1));
	memcpy(callPtr->chain, callPtr->staticChain,
		sizeof(MInvoke) * callPtr->numChain);
    } else if (callPtr->numChain > CALL_CHAIN_STATIC_SIZE) {
	callPtr->chain = (MInvoke *) Tcl_Realloc(callPtr->chain,
		sizeof(MInvoke) * (callPtr->numChain + 1));

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 1732–1829. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
	if (hPtr != NULL) {
	    Method *mPtr = (Method *) Tcl_GetHashValue(hPtr);

	    if (IS_PRIVATE(mPtr)) {
		AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			flags);
		return true;
	    }
	}
    }

    switch (classPtr->superclasses.num) {
    case 1:
	classPtr = classPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, classPtr->superclasses) {
	    if (AddPrivatesFromClassChainToCallContext(superPtr, contextCls,
		    methodName, cbPtr, doneFilters, flags, filterDecl)) {
		return true;
	    }
	}
	TCL_FALLTHROUGH();
    case 0:
	return false;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * AddSimpleClassChainToCallContext --
 *
 *	Construct a call-chain from a class hierarchy.
 *
 * ----------------------------------------------------------------------
 */

static bool
AddSimpleClassChainToCallContext(
    Class *classPtr,		/* Class to add the call chain entries for. */
    Tcl_Obj *const methodNameObj,
				/* Name of method to add the call chain
				 * entries for. */
    ChainBuilder *const cbPtr,	/* Where to add the call chain entries. */
    Tcl_HashTable *const doneFilters,
				/* Where to record what call chain entries
				 * have been processed. */
    int flags,			/* What sort of call chain are we building. */
    Class *const filterDecl)	/* The class that declared the filter. If
				 * NULL, either the filter was declared by the
				 * object or this isn't a filter. */
{
    Tcl_Size i;
    bool privateDanger = false;
    Class *superPtr;

    /*
     * We hard-code the tail-recursive form. It's by far the most common case
     * *and* it is much more gentle on the stack.
     *
     * Note that mixins must be processed before the main class hierarchy.
     * [Bug 1998221]
     */

  tailRecurse:
    if (classPtr == NULL) {
	return privateDanger;
    }
    FOREACH(superPtr, classPtr->mixins) {
	privateDanger |= AddSimpleClassChainToCallContext(superPtr,
		methodNameObj, cbPtr, doneFilters, flags | TRAVERSED_MIXIN,
		filterDecl);
    }

    if (flags & CONSTRUCTOR) {
	AddMethodToCallChain(classPtr->constructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else if (flags & DESTRUCTOR) {
	AddMethodToCallChain(classPtr->destructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else {
	Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&classPtr->classMethods,
		methodNameObj);

	if (classPtr->flags & HAS_PRIVATE_METHODS) {
	    privateDanger |= true;
	}
	if (hPtr != NULL) {
	    Method *mPtr = (Method *) Tcl_GetHashValue(hPtr);

	    if (!IS_PRIVATE(mPtr)) {
		if (!(flags & KNOWN_STATE)) {
		    if (flags & PUBLIC_METHOD) {
			if (!IS_PUBLIC(mPtr)) {
			    return privateDanger;
			}
			flags |= DEFINITE_PUBLIC;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 1804–1901. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
		filterDecl);
    }

    if (flags & CONSTRUCTOR) {
	AddMethodToCallChain(classPtr->constructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else if (flags & DESTRUCTOR) {
	AddMethodToCallChain(classPtr->destructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else {
	Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&classPtr->classMethods,
		methodNameObj);

	if (classPtr->flags & HAS_PRIVATE_METHODS) {
	    privateDanger |= true;
	}
	if (hPtr != NULL) {
	    Method *mPtr = (Method *) Tcl_GetHashValue(hPtr);

	    if (!IS_PRIVATE(mPtr)) {
		if (!(flags & KNOWN_STATE)) {
		    if (flags & PUBLIC_METHOD) {
			if (!IS_PUBLIC(mPtr)) {
			    return privateDanger;
			}
			flags |= DEFINITE_PUBLIC;
		    } else {
			flags |= DEFINITE_PROTECTED;
		    }
		}
		AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			flags);
	    }
	}
    }

    switch (classPtr->superclasses.num) {
    case 1:
	classPtr = classPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, classPtr->superclasses) {
	    privateDanger |= AddSimpleClassChainToCallContext(superPtr,
		    methodNameObj, cbPtr, doneFilters, flags, filterDecl);
	}
	TCL_FALLTHROUGH();
    case 0:
	return privateDanger;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOORenderCallChain --
 *
 *	Create a description of a call chain. Used in [info object call],
 *	[info class call], and [self call].
 *
 * ----------------------------------------------------------------------
 */

Tcl_Obj *
TclOORenderCallChain(
    Tcl_Interp *interp,
    CallChain *callPtr)
{
    Tcl_Obj *filterLiteral, *methodLiteral, *objectLiteral, *privateLiteral;
    Tcl_Obj *resultObj, *descObjs[4], **objv;
    Foundation *fPtr = TclOOGetFoundation(interp);
    Tcl_Size i;

    /*
     * Allocate the literals (potentially) used in our description.
     */

    TclNewLiteralStringObj(filterLiteral, "filter");
    TclNewLiteralStringObj(methodLiteral, "method");
    TclNewLiteralStringObj(objectLiteral, "object");
    TclNewLiteralStringObj(privateLiteral, "private");

    /*
     * Do the actual construction of the descriptions. They consist of a list
     * of triples that describe the details of how a method is understood. For
     * each triple, the first word is the type of invocation ("method" is
     * normal, "unknown" is special because it adds the method name as an
     * extra argument when handled by some method types, and "filter" is
     * special because it's a filter method). The second word is the name of
     * the method in question (which differs for "unknown" and "filter" types)
     * and the third word is the full name of the class that declares the
     * method (or "object" if it is declared on the instance).
     */

    objv = (Tcl_Obj **)
	    TclStackAlloc(interp, callPtr->numChain * sizeof(Tcl_Obj *));
    for (i = 0 ; i < callPtr->numChain ; i++) {
	MInvoke *miPtr = &callPtr->chain[i];

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 1807–1904. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
    if (flags & CONSTRUCTOR) {
	AddMethodToCallChain(classPtr->constructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else if (flags & DESTRUCTOR) {
	AddMethodToCallChain(classPtr->destructorPtr, cbPtr, doneFilters,
		filterDecl, flags);
    } else {
	Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&classPtr->classMethods,
		methodNameObj);

	if (classPtr->flags & HAS_PRIVATE_METHODS) {
	    privateDanger |= true;
	}
	if (hPtr != NULL) {
	    Method *mPtr = (Method *) Tcl_GetHashValue(hPtr);

	    if (!IS_PRIVATE(mPtr)) {
		if (!(flags & KNOWN_STATE)) {
		    if (flags & PUBLIC_METHOD) {
			if (!IS_PUBLIC(mPtr)) {
			    return privateDanger;
			}
			flags |= DEFINITE_PUBLIC;
		    } else {
			flags |= DEFINITE_PROTECTED;
		    }
		}
		AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			flags);
	    }
	}
    }

    switch (classPtr->superclasses.num) {
    case 1:
	classPtr = classPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, classPtr->superclasses) {
	    privateDanger |= AddSimpleClassChainToCallContext(superPtr,
		    methodNameObj, cbPtr, doneFilters, flags, filterDecl);
	}
	TCL_FALLTHROUGH();
    case 0:
	return privateDanger;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOORenderCallChain --
 *
 *	Create a description of a call chain. Used in [info object call],
 *	[info class call], and [self call].
 *
 * ----------------------------------------------------------------------
 */

Tcl_Obj *
TclOORenderCallChain(
    Tcl_Interp *interp,
    CallChain *callPtr)
{
    Tcl_Obj *filterLiteral, *methodLiteral, *objectLiteral, *privateLiteral;
    Tcl_Obj *resultObj, *descObjs[4], **objv;
    Foundation *fPtr = TclOOGetFoundation(interp);
    Tcl_Size i;

    /*
     * Allocate the literals (potentially) used in our description.
     */

    TclNewLiteralStringObj(filterLiteral, "filter");
    TclNewLiteralStringObj(methodLiteral, "method");
    TclNewLiteralStringObj(objectLiteral, "object");
    TclNewLiteralStringObj(privateLiteral, "private");

    /*
     * Do the actual construction of the descriptions. They consist of a list
     * of triples that describe the details of how a method is understood. For
     * each triple, the first word is the type of invocation ("method" is
     * normal, "unknown" is special because it adds the method name as an
     * extra argument when handled by some method types, and "filter" is
     * special because it's a filter method). The second word is the name of
     * the method in question (which differs for "unknown" and "filter" types)
     * and the third word is the full name of the class that declares the
     * method (or "object" if it is declared on the instance).
     */

    objv = (Tcl_Obj **)
	    TclStackAlloc(interp, callPtr->numChain * sizeof(Tcl_Obj *));
    for (i = 0 ; i < callPtr->numChain ; i++) {
	MInvoke *miPtr = &callPtr->chain[i];

	descObjs[0] =
		miPtr->isFilter ? filterLiteral :
		callPtr->flags & OO_UNKNOWN_METHOD ? fPtr->unknownMethodNameObj :

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `AddMethodToCallChain(`, lines 1830–1927. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text
		    } else {
			flags |= DEFINITE_PROTECTED;
		    }
		}
		AddMethodToCallChain(mPtr, cbPtr, doneFilters, filterDecl,
			flags);
	    }
	}
    }

    switch (classPtr->superclasses.num) {
    case 1:
	classPtr = classPtr->superclasses.list[0];
	goto tailRecurse;
    default:
	FOREACH(superPtr, classPtr->superclasses) {
	    privateDanger |= AddSimpleClassChainToCallContext(superPtr,
		    methodNameObj, cbPtr, doneFilters, flags, filterDecl);
	}
	TCL_FALLTHROUGH();
    case 0:
	return privateDanger;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOORenderCallChain --
 *
 *	Create a description of a call chain. Used in [info object call],
 *	[info class call], and [self call].
 *
 * ----------------------------------------------------------------------
 */

Tcl_Obj *
TclOORenderCallChain(
    Tcl_Interp *interp,
    CallChain *callPtr)
{
    Tcl_Obj *filterLiteral, *methodLiteral, *objectLiteral, *privateLiteral;
    Tcl_Obj *resultObj, *descObjs[4], **objv;
    Foundation *fPtr = TclOOGetFoundation(interp);
    Tcl_Size i;

    /*
     * Allocate the literals (potentially) used in our description.
     */

    TclNewLiteralStringObj(filterLiteral, "filter");
    TclNewLiteralStringObj(methodLiteral, "method");
    TclNewLiteralStringObj(objectLiteral, "object");
    TclNewLiteralStringObj(privateLiteral, "private");

    /*
     * Do the actual construction of the descriptions. They consist of a list
     * of triples that describe the details of how a method is understood. For
     * each triple, the first word is the type of invocation ("method" is
     * normal, "unknown" is special because it adds the method name as an
     * extra argument when handled by some method types, and "filter" is
     * special because it's a filter method). The second word is the name of
     * the method in question (which differs for "unknown" and "filter" types)
     * and the third word is the full name of the class that declares the
     * method (or "object" if it is declared on the instance).
     */

    objv = (Tcl_Obj **)
	    TclStackAlloc(interp, callPtr->numChain * sizeof(Tcl_Obj *));
    for (i = 0 ; i < callPtr->numChain ; i++) {
	MInvoke *miPtr = &callPtr->chain[i];

	descObjs[0] =
		miPtr->isFilter ? filterLiteral :
		callPtr->flags & OO_UNKNOWN_METHOD ? fPtr->unknownMethodNameObj :
		IS_PRIVATE(miPtr->mPtr) ? privateLiteral :
			methodLiteral;
	descObjs[1] =
		callPtr->flags & CONSTRUCTOR ? fPtr->constructorName :
		callPtr->flags & DESTRUCTOR ? fPtr->destructorName :
			miPtr->mPtr->namePtr;
	descObjs[2] = miPtr->mPtr->declaringClassPtr
		? Tcl_GetObjectName(interp,
			(Tcl_Object) miPtr->mPtr->declaringClassPtr->thisPtr)
		: objectLiteral;
	descObjs[3] = Tcl_NewStringObj(miPtr->mPtr->type2Ptr->name,
		TCL_AUTO_LENGTH);

	objv[i] = Tcl_NewListObj(4, descObjs);
    }

    /*
     * Drop the local references to the literals; if they're actually used,
     * they'll live on the description itself.
     */

    Tcl_BounceRefCount(filterLiteral);
    Tcl_BounceRefCount(methodLiteral);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `methodNameType =`, lines 153–247. Retained reference `runtime-rust-tests-data-native_method_cache_active_call_lifetime_original-original-inputs-workspace--.proofs--native-method-lifetime506--original-request--source-inspection--9.1.0--tclOOCall.c`.

```text

/*
 * Object type used to manage type caches attached to method names.
 */
static const Tcl_ObjType methodNameType = {
    "TclOO method name",
    FreeMethodNameRep,
    DupMethodNameRep,
    NULL,			// UpdateString
    NULL,			// SetFromAny
    TCL_OBJTYPE_V0
};

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteContext --
 *
 *	Destroys a method call-chain context, which should not be in use.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteContext(
    CallContext *contextPtr)
{
    Object *oPtr = contextPtr->oPtr;

    TclOODeleteChain(contextPtr->callPtr);
    if (oPtr != NULL) {
	TclStackFree(oPtr->fPtr->interp, contextPtr);

	/*
	 * Corresponding AddRef() in TclOO.c/TclOOObjectCmdCore
	 */

	TclOODecrRefCount(oPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChainCache --
 *
 *	Destroy the cache of method call-chains.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChainCache(
    Tcl_HashTable *tablePtr)
{
    FOREACH_HASH_DECLS;
    CallChain *callPtr;

    FOREACH_HASH_VALUE(callPtr, tablePtr) {
	if (callPtr) {
	    TclOODeleteChain(callPtr);
	}
    }
    Tcl_DeleteHashTable(tablePtr);
    Tcl_Free(tablePtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODeleteChain --
 *
 *	Destroys a method call-chain.
 *
 * ----------------------------------------------------------------------
 */
void
TclOODeleteChain(
    CallChain *callPtr)
{
    if (callPtr == NULL || callPtr->refCount-- > 1) {
	return;
    }
    if (callPtr->chain != callPtr->staticChain) {
	Tcl_Free(callPtr->chain);
    }
    Tcl_Free(callPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOStashContext --
 *
 *	Saves a reference to a method call context in a Tcl_Obj's internal
 *	representation.
 *

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 258–356. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
 * ----------------------------------------------------------------------
 */

Tcl_Method
TclNewMethod(
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType2 *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    Class *clsPtr = (Class *) cls;
    Method *mPtr;
    Tcl_HashEntry *hPtr;
    int isNew;

    if (nameObj == NULL) {
	mPtr = (Method *) Tcl_Alloc(sizeof(Method));
	mPtr->namePtr = NULL;
	mPtr->refCount = 1;
	goto populate;
    }
    hPtr = Tcl_CreateHashEntry(&clsPtr->classMethods, nameObj, &isNew);
    if (isNew) {
	mPtr = (Method *) Tcl_Alloc(sizeof(Method));
	mPtr->refCount = 1;
	mPtr->namePtr = nameObj;
	Tcl_IncrRefCount(nameObj);
	Tcl_SetHashValue(hPtr, mPtr);
    } else {
	mPtr = (Method *) Tcl_GetHashValue(hPtr);
	if (mPtr->type2Ptr != NULL && mPtr->type2Ptr->deleteProc != NULL) {
	    mPtr->type2Ptr->deleteProc(mPtr->clientData);
	}
    }

  populate:
    clsPtr->thisPtr->fPtr->epoch++;
    mPtr->type2Ptr = typePtr;
    mPtr->clientData = clientData;
    mPtr->flags = 0;
    mPtr->declaringObjectPtr = NULL;
    mPtr->declaringClassPtr = clsPtr;
    if (flags) {
	mPtr->flags |= flags &
		(PUBLIC_METHOD | PRIVATE_METHOD | TRUE_PRIVATE_METHOD);
	if (flags & TRUE_PRIVATE_METHOD) {
	    clsPtr->flags |= HAS_PRIVATE_METHODS;
	}
    }

    return (Tcl_Method) mPtr;
}

#ifndef TCL_NO_DEPRECATED
#undef Tcl_NewMethod
Tcl_Method
Tcl_NewMethod(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    if (typePtr->version > TCL_OO_METHOD_VERSION_1) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod", "TCL_OO_METHOD_VERSION_1");
    }
    return TclNewMethod(cls, nameObj, flags,
	    (const Tcl_MethodType2 *)typePtr, clientData);
}
#endif /* TCL_NO_DEPRECATED */

Tcl_Method
Tcl_NewMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType2 *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 335–430. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
    if (typePtr->version > TCL_OO_METHOD_VERSION_1) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod", "TCL_OO_METHOD_VERSION_1");
    }
    return TclNewMethod(cls, nameObj, flags,
	    (const Tcl_MethodType2 *)typePtr, clientData);
}
#endif /* TCL_NO_DEPRECATED */

Tcl_Method
Tcl_NewMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType2 *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    if (typePtr->version < TCL_OO_METHOD_VERSION_2) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewMethod(cls, nameObj, flags, typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODelMethodRef --
 *
 *	How to delete a method.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->type2Ptr != NULL && mPtr->type2Ptr->deleteProc != NULL) {
	    mPtr->type2Ptr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	Tcl_Free(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineBasicMethods --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODefineBasicMethods(
    Class *clsPtr,		/* Class to attach the methods to. */
    const DeclaredClassMethod *dcmAry)
				/* Static table of method definitions. */
{
    for (const DeclaredClassMethod *dcm = dcmAry; dcm->name ; dcm++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcm->name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,
		(dcm->isPublic ? PUBLIC_METHOD : 0), &dcm->definition, NULL);
	Tcl_BounceRefCount(namePtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 359–454. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
    if (typePtr->version < TCL_OO_METHOD_VERSION_2) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewMethod(cls, nameObj, flags, typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODelMethodRef --
 *
 *	How to delete a method.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->type2Ptr != NULL && mPtr->type2Ptr->deleteProc != NULL) {
	    mPtr->type2Ptr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	Tcl_Free(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineBasicMethods --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODefineBasicMethods(
    Class *clsPtr,		/* Class to attach the methods to. */
    const DeclaredClassMethod *dcmAry)
				/* Static table of method definitions. */
{
    for (const DeclaredClassMethod *dcm = dcmAry; dcm->name ; dcm++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcm->name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,
		(dcm->isPublic ? PUBLIC_METHOD : 0), &dcm->definition, NULL);
	Tcl_BounceRefCount(namePtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which must not be
				 * NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which must not be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which must not be
				 * NULL. */
    ProcedureMethod **pmPtrPtr)	/* Place to write pointer to procedure method
				 * structure to allow for deeper tuning of the
				 * structure's contents. NULL if caller is not
				 * interested. */
{
    Tcl_Size argsLen;
    ProcedureMethod *pmPtr;
    Tcl_Method method;

    if (TclListObjLength(interp, argsObj, &argsLen) != TCL_OK) {
	return NULL;
    }
    pmPtr = AllocProcedureMethodRecord(flags);
    method = TclOOMakeProcInstanceMethod2(interp, oPtr, flags, nameObj,
	    argsObj, bodyObj, &procMethodType, pmPtr, &pmPtr->procPtr);
    if (method == NULL) {

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 408–504. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
{
    for (const DeclaredClassMethod *dcm = dcmAry; dcm->name ; dcm++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcm->name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,
		(dcm->isPublic ? PUBLIC_METHOD : 0), &dcm->definition, NULL);
	Tcl_BounceRefCount(namePtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which must not be
				 * NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which must not be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which must not be
				 * NULL. */
    ProcedureMethod **pmPtrPtr)	/* Place to write pointer to procedure method
				 * structure to allow for deeper tuning of the
				 * structure's contents. NULL if caller is not
				 * interested. */
{
    Tcl_Size argsLen;
    ProcedureMethod *pmPtr;
    Tcl_Method method;

    if (TclListObjLength(interp, argsObj, &argsLen) != TCL_OK) {
	return NULL;
    }
    pmPtr = AllocProcedureMethodRecord(flags);
    method = TclOOMakeProcInstanceMethod2(interp, oPtr, flags, nameObj,
	    argsObj, bodyObj, &procMethodType, pmPtr, &pmPtr->procPtr);
    if (method == NULL) {
	Tcl_Free(pmPtr);
    } else if (pmPtrPtr != NULL) {
	*pmPtrPtr = pmPtr;
    }
    return (Method *) method;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcMethod --
 *
 *	Create a new procedure-like method for a class.
 *
 * ----------------------------------------------------------------------
 */

Method *
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 750–847. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
    procPtr->cmdPtr = NULL;

    InitCmdFrame(iPtr, procPtr);

    return TclNewMethod(
	    (Tcl_Class) clsPtr, nameObj, flags, (const Tcl_MethodType2 *)typePtr, clientData);
}
#endif /* TCL_NO_DEPRECATED */

Tcl_Method
TclOOMakeProcMethod2(
    Tcl_Interp *interp,		/* The interpreter containing the class. */
    Class *clsPtr,		/* The class to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which may be NULL;
				 * if so, up to caller to manage storage
				 * (e.g., because it is a constructor or
				 * destructor). */
    const char *namePtr,	/* The name of the method as a string, which
				 * _must not_ be NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which _must not_ be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which _must not_ be
				 * NULL. */
    const Tcl_MethodType2 *typePtr,
				/* The type of the method to create. */
    void *clientData,		/* The per-method type-specific data. */
    Proc **procPtrPtr)		/* A pointer to the variable in which to write
				 * the procedure record reference. Presumably
				 * inside the structure indicated by the
				 * pointer in clientData. */
{
    Interp *iPtr = (Interp *) interp;
    Proc *procPtr;

    if (typePtr->version < TCL_OO_METHOD_VERSION_2) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"TclOOMakeProcMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    if (TclCreateProc(interp, NULL, namePtr, argsObj, bodyObj,
	    procPtrPtr) != TCL_OK) {
	return NULL;
    }
    procPtr = *procPtrPtr;
    procPtr->cmdPtr = NULL;

    InitCmdFrame(iPtr, procPtr);

    return TclNewMethod((Tcl_Class) clsPtr, nameObj, flags,
	    typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * InvokeProcedureMethod, FinalizePMCall, PushMethodCallFrame --
 *
 *	How to invoke a procedure-like method.
 *
 * ----------------------------------------------------------------------
 */

static int
InvokeProcedureMethod(
    void *clientData,		/* Pointer to the per-method record. */
    Tcl_Interp *interp,
    Tcl_ObjectContext context,	/* The method calling context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments as actually seen. */
{
    ProcedureMethod *pmPtr = (ProcedureMethod *) clientData;
    int result;
    PMFrameData *fdPtr;		/* Important data that has to have a lifetime
				 * matched by this function (or rather, by the
				 * call frame's lifetime). */

    /*
     * If the object namespace (or interpreter) were deleted, we just skip to
     * the next thing in the chain.
     */

    if (TclOOObjectDestroyed(((CallContext *) context)->oPtr)
	    || Tcl_InterpDeleted(interp)) {
	return TclNRObjectContextInvokeNext(interp, context, objc, objv,
		Tcl_ObjectContextSkippedArgs(context));
    }

    /*
     * Finishes filling out the extra frame info so that [info frame] works if
     * that is not already set up.
     */

    if (pmPtr->efi.length == 0) {
	Tcl_Method method = Tcl_ObjectContextMethod(context);

	pmPtr->efi.length = 2;
	pmPtr->efi.fields[0].name = "method";
	pmPtr->efi.fields[0].proc = RenderMethodName;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 794–891. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
    procPtr->cmdPtr = NULL;

    InitCmdFrame(iPtr, procPtr);

    return TclNewMethod((Tcl_Class) clsPtr, nameObj, flags,
	    typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * InvokeProcedureMethod, FinalizePMCall, PushMethodCallFrame --
 *
 *	How to invoke a procedure-like method.
 *
 * ----------------------------------------------------------------------
 */

static int
InvokeProcedureMethod(
    void *clientData,		/* Pointer to the per-method record. */
    Tcl_Interp *interp,
    Tcl_ObjectContext context,	/* The method calling context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments as actually seen. */
{
    ProcedureMethod *pmPtr = (ProcedureMethod *) clientData;
    int result;
    PMFrameData *fdPtr;		/* Important data that has to have a lifetime
				 * matched by this function (or rather, by the
				 * call frame's lifetime). */

    /*
     * If the object namespace (or interpreter) were deleted, we just skip to
     * the next thing in the chain.
     */

    if (TclOOObjectDestroyed(((CallContext *) context)->oPtr)
	    || Tcl_InterpDeleted(interp)) {
	return TclNRObjectContextInvokeNext(interp, context, objc, objv,
		Tcl_ObjectContextSkippedArgs(context));
    }

    /*
     * Finishes filling out the extra frame info so that [info frame] works if
     * that is not already set up.
     */

    if (pmPtr->efi.length == 0) {
	Tcl_Method method = Tcl_ObjectContextMethod(context);

	pmPtr->efi.length = 2;
	pmPtr->efi.fields[0].name = "method";
	pmPtr->efi.fields[0].proc = RenderMethodName;
	pmPtr->efi.fields[0].clientData = pmPtr;
	pmPtr->callSiteFlags = ((CallContext *)
		context)->callPtr->flags & (CONSTRUCTOR | DESTRUCTOR);
	pmPtr->interp = interp;
	pmPtr->method = method;
	if (pmPtr->gfivProc != NULL) {
	    pmPtr->efi.fields[1].name = "";
	    pmPtr->efi.fields[1].proc = pmPtr->gfivProc;
	    pmPtr->efi.fields[1].clientData = pmPtr;
	} else {
	    if (Tcl_MethodDeclarerObject(method) != NULL) {
		pmPtr->efi.fields[1].name = "object";
	    } else {
		pmPtr->efi.fields[1].name = "class";
	    }
	    pmPtr->efi.fields[1].proc = RenderDeclarerName;
	    pmPtr->efi.fields[1].clientData = pmPtr;
	}
    }

    /*
     * Allocate the special frame data.
     */

    fdPtr = (PMFrameData *) TclStackAlloc(interp, sizeof(PMFrameData));

    /*
     * Create a call frame for this method.
     */

    result = PushMethodCallFrame(interp, (CallContext *) context, pmPtr,
	    objc, objv, fdPtr);
    if (result != TCL_OK) {
	TclStackFree(interp, fdPtr);
	return result;
    }
    pmPtr->refCount++;

    /*
     * Give the pre-call callback a chance to do some setup and, possibly,
     * veto the call.
     */

    if (pmPtr->preCallProc != NULL) {

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `TclNewMethod(`, lines 1613–1709. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text

    fmPtr = (ForwardMethod *) Tcl_Alloc(sizeof(ForwardMethod));
    fmPtr->prefixObj = prefixObj;
    Tcl_IncrRefCount(prefixObj);
    return (Method *) TclNewMethod((Tcl_Class) clsPtr, nameObj,
	    flags, &fwdMethodType, fmPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * InvokeForwardMethod, FinalizeForwardCall --
 *
 *	How to invoke a forwarded method. Works by doing some ensemble-like
 *	command rearranging and then invokes some other Tcl command.
 *
 * ----------------------------------------------------------------------
 */

static int
InvokeForwardMethod(
    void *clientData,		/* Pointer to some per-method context. */
    Tcl_Interp *interp,
    Tcl_ObjectContext context,	/* The method calling context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments as actually seen. */
{
    CallContext *contextPtr = (CallContext *) context;
    ForwardMethod *fmPtr = (ForwardMethod *) clientData;
    Tcl_Obj **argObjs, **prefixObjs;
    Tcl_Size numPrefixes, skip = contextPtr->skip;
    Tcl_Size len;

    /*
     * Build the real list of arguments to use. Note that we know that the
     * prefixObj field of the ForwardMethod structure holds a reference to a
     * non-empty list, so there's a whole class of failures ("not a list") we
     * can ignore here.
     */

    TclListObjGetElements(NULL, fmPtr->prefixObj, &numPrefixes, &prefixObjs);
    argObjs = InitEnsembleRewrite(interp, objc, objv, skip,
	    numPrefixes, prefixObjs, &len);
    TclNRAddCallback(interp, FinalizeForwardCall, argObjs, NULL, NULL, NULL);
    /*
     * NOTE: The combination of direct set of iPtr->lookupNsPtr and the use
     * of the TCL_EVAL_NOERR flag results in an evaluation configuration
     * very much like TCL_EVAL_INVOKE.
     */
    ((Interp *) interp)->lookupNsPtr = (Namespace *)
	    contextPtr->oPtr->namespacePtr;
    return TclNREvalObjv(interp, len, argObjs, TCL_EVAL_NOERR, NULL);
}

static int
FinalizeForwardCall(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    Tcl_Obj **argObjs = (Tcl_Obj **) data[0];

    TclStackFree(interp, argObjs);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * DeleteForwardMethod, CloneForwardMethod --
 *
 *	How to delete and clone forwarded methods.
 *
 * ----------------------------------------------------------------------
 */

static void
DeleteForwardMethod(
    void *clientData)
{
    ForwardMethod *fmPtr = (ForwardMethod *) clientData;

    Tcl_DecrRefCount(fmPtr->prefixObj);
    Tcl_Free(fmPtr);
}

static int
CloneForwardMethod(
    TCL_UNUSED(Tcl_Interp *),
    void *clientData,
    void **newClientData)
{
    ForwardMethod *fmPtr = (ForwardMethod *) clientData;
    ForwardMethod *fm2Ptr = (ForwardMethod *) Tcl_Alloc(sizeof(ForwardMethod));

    fm2Ptr->prefixObj = fmPtr->prefixObj;
    Tcl_IncrRefCount(fm2Ptr->prefixObj);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 317–413. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text

#ifndef TCL_NO_DEPRECATED
#undef Tcl_NewMethod
Tcl_Method
Tcl_NewMethod(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    if (typePtr->version > TCL_OO_METHOD_VERSION_1) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod", "TCL_OO_METHOD_VERSION_1");
    }
    return TclNewMethod(cls, nameObj, flags,
	    (const Tcl_MethodType2 *)typePtr, clientData);
}
#endif /* TCL_NO_DEPRECATED */

Tcl_Method
Tcl_NewMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType2 *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    if (typePtr->version < TCL_OO_METHOD_VERSION_2) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewMethod(cls, nameObj, flags, typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODelMethodRef --
 *
 *	How to delete a method.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->type2Ptr != NULL && mPtr->type2Ptr->deleteProc != NULL) {
	    mPtr->type2Ptr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	Tcl_Free(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineBasicMethods --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODefineBasicMethods(
    Class *clsPtr,		/* Class to attach the methods to. */
    const DeclaredClassMethod *dcmAry)
				/* Static table of method definitions. */
{
    for (const DeclaredClassMethod *dcm = dcmAry; dcm->name ; dcm++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcm->name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,
		(dcm->isPublic ? PUBLIC_METHOD : 0), &dcm->definition, NULL);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `Tcl_NewMethod2(`, lines 341–436. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
}
#endif /* TCL_NO_DEPRECATED */

Tcl_Method
Tcl_NewMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Class cls,		/* The class to attach the method to. */
    Tcl_Obj *nameObj,		/* The name of the object. May be NULL (e.g.,
				 * for constructors or destructors); if so, up
				 * to caller to manage storage. */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType2 *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    if (typePtr->version < TCL_OO_METHOD_VERSION_2) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"Tcl_NewMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewMethod(cls, nameObj, flags, typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODelMethodRef --
 *
 *	How to delete a method.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->type2Ptr != NULL && mPtr->type2Ptr->deleteProc != NULL) {
	    mPtr->type2Ptr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	Tcl_Free(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineBasicMethods --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODefineBasicMethods(
    Class *clsPtr,		/* Class to attach the methods to. */
    const DeclaredClassMethod *dcmAry)
				/* Static table of method definitions. */
{
    for (const DeclaredClassMethod *dcm = dcmAry; dcm->name ; dcm++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcm->name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,
		(dcm->isPublic ? PUBLIC_METHOD : 0), &dcm->definition, NULL);
	Tcl_BounceRefCount(namePtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which must not be
				 * NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which must not be NULL. */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `TclOODelMethodRef(`, lines 373–468. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
 * ----------------------------------------------------------------------
 */

void
TclOODelMethodRef(
    Method *mPtr)
{
    if ((mPtr != NULL) && (mPtr->refCount-- <= 1)) {
	if (mPtr->type2Ptr != NULL && mPtr->type2Ptr->deleteProc != NULL) {
	    mPtr->type2Ptr->deleteProc(mPtr->clientData);
	}
	if (mPtr->namePtr != NULL) {
	    Tcl_DecrRefCount(mPtr->namePtr);
	}

	Tcl_Free(mPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineBasicMethods --
 *
 *	Helper that makes it cleaner to create very simple methods during
 *	basic system initialization. Not suitable for general use.
 *
 * ----------------------------------------------------------------------
 */

void
TclOODefineBasicMethods(
    Class *clsPtr,		/* Class to attach the methods to. */
    const DeclaredClassMethod *dcmAry)
				/* Static table of method definitions. */
{
    for (const DeclaredClassMethod *dcm = dcmAry; dcm->name ; dcm++) {
	Tcl_Obj *namePtr = Tcl_NewStringObj(dcm->name, TCL_AUTO_LENGTH);

	TclNewMethod((Tcl_Class) clsPtr, namePtr,
		(dcm->isPublic ? PUBLIC_METHOD : 0), &dcm->definition, NULL);
	Tcl_BounceRefCount(namePtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcInstanceMethod --
 *
 *	Create a new procedure-like method for an object.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which must not be
				 * NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which must not be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which must not be
				 * NULL. */
    ProcedureMethod **pmPtrPtr)	/* Place to write pointer to procedure method
				 * structure to allow for deeper tuning of the
				 * structure's contents. NULL if caller is not
				 * interested. */
{
    Tcl_Size argsLen;
    ProcedureMethod *pmPtr;
    Tcl_Method method;

    if (TclListObjLength(interp, argsObj, &argsLen) != TCL_OK) {
	return NULL;
    }
    pmPtr = AllocProcedureMethodRecord(flags);
    method = TclOOMakeProcInstanceMethod2(interp, oPtr, flags, nameObj,
	    argsObj, bodyObj, &procMethodType, pmPtr, &pmPtr->procPtr);
    if (method == NULL) {
	Tcl_Free(pmPtr);
    } else if (pmPtrPtr != NULL) {
	*pmPtrPtr = pmPtr;
    }
    return (Method *) method;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewProcMethod --
 *
 *	Create a new procedure-like method for a class.
 *

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInt.h`, function `#define AddRef`, lines 711–792. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOInt.h`.

```text
/*
 * Alternatives to Tcl_Preserve/Tcl_EventuallyFree/Tcl_Release.
 */

#define AddRef(ptr) ((ptr)->refCount++)

/*
 * A convenience macro for iterating through the lists used in the internal
 * memory management of objects.
 * REQUIRES DECLARATION: Tcl_Size i;
 */

#define FOREACH(var,ary) \
    for (i=0 ; i<(ary).num; i++) if ((ary).list[i] == NULL) { \
	continue; \
    } else if ((var) = (ary).list[i], 1)

/*
 * A variation where the array is an array of structs. There's no issue with
 * possible NULLs; every element of the array will be iterated over and the
 * variable set to a pointer to each of those elements in turn.
 * REQUIRES DECLARATION: Tcl_Size i; See [96551aca55] for more FOREACH_STRUCT details.
 */

#define FOREACH_STRUCT(var,ary) \
    if (i=0, (ary).num>0) for (; var=&((ary).list[i]), i<(ary).num; i++)

/*
 * Convenience macros for iterating through hash tables. FOREACH_HASH_DECLS
 * sets up the declarations needed for the main macro, FOREACH_HASH, which
 * does the actual iteration. FOREACH_HASH_KEY and FOREACH_HASH_VALUE are
 * restricted versions that only iterate over keys or values respectively.
 * REQUIRES DECLARATION: FOREACH_HASH_DECLS;
 */

#define FOREACH_HASH_DECLS \
    Tcl_HashEntry *hPtr;Tcl_HashSearch search
#define FOREACH_HASH(key, val, tablePtr) \
    for (hPtr = Tcl_FirstHashEntry((tablePtr), &search); hPtr != NULL ? \
	    (*(void **)&(key) = Tcl_GetHashKey((tablePtr), hPtr), \
	    *(void **)&(val) = Tcl_GetHashValue(hPtr), 1) : 0; \
	    hPtr = Tcl_NextHashEntry(&search))
#define FOREACH_HASH_KEY(key, tablePtr) \
    for (hPtr = Tcl_FirstHashEntry((tablePtr), &search); hPtr != NULL ? \
	    (*(void **)&(key) = Tcl_GetHashKey((tablePtr), hPtr), 1) : 0; \
	    hPtr = Tcl_NextHashEntry(&search))
#define FOREACH_HASH_VALUE(val, tablePtr) \
    for (hPtr = Tcl_FirstHashEntry((tablePtr), &search); hPtr != NULL ? \
	    (*(void **)&(val) = Tcl_GetHashValue(hPtr), 1) : 0; \
	    hPtr = Tcl_NextHashEntry(&search))

/*
 * Convenience macro for duplicating a list. Needs no external declaration,
 * but all arguments are used multiple times and so must have no side effects.
 */

#undef DUPLICATE /* prevent possible conflict with definition in WINAPI nb30.h */
#define DUPLICATE(target,source,type) \
    do { \
	size_t len = sizeof(type) * ((target).num=(source).num);\
	if (len != 0) { \
	    memcpy(((target).list=(type*)Tcl_Alloc(len)), (source).list, len); \
	} else { \
	    (target).list = NULL; \
	} \
    } while(0)

/*
 * Convenience macro for generating error codes.
 */
#define OO_ERROR(interp, code) \
    Tcl_SetErrorCode((interp), "TCL", "OO", #code, (char *)NULL)

#endif /* TCL_OO_INTERNAL_H */

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInt.h`, function `struct Method`, lines 35–133. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOInt.h`.

```text
typedef struct Class Class;
typedef struct DeclaredClassMethod DeclaredClassMethod;
typedef struct ForwardMethod ForwardMethod;
typedef struct Foundation Foundation;
typedef struct Method Method;
typedef struct MInvoke MInvoke;
typedef struct Object Object;
typedef struct PrivateVariableMapping PrivateVariableMapping;
typedef struct ProcedureMethod ProcedureMethod;
typedef struct PropertyStorage PropertyStorage;

/*
 * The data that needs to be stored per method. This record is used to collect
 * information about all sorts of methods, including forwards, constructors
 * and destructors.
 */
struct Method {
    union {
#ifndef TCL_NO_DEPRECATED
	const Tcl_MethodType *typePtr;
#endif /* TCL_NO_DEPRECATED */
	const Tcl_MethodType2 *type2Ptr;
    };				/* The type of method. If NULL, this is a
				 * special flag record which is just used for
				 * the setting of the flags field. Note that
				 * this is a union of two pointer types that
				 * have the same layout at least as far as the
				 * internal version field. */
    Tcl_Size refCount;
    void *clientData;		/* Type-specific data. */
    Tcl_Obj *namePtr;		/* Name of the method. */
    Object *declaringObjectPtr;	/* The object that declares this method, or
				 * NULL if it was declared by a class. */
    Class *declaringClassPtr;	/* The class that declares this method, or
				 * NULL if it was declared directly on an
				 * object. */
    int flags;			/* Assorted flags. Includes whether this
				 * method is public/exported or not. */
};

/*
 * Pre- and post-call callbacks, to allow procedure-like methods to be fine
 * tuned in their behaviour.
 */

typedef int (TclOO_PreCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_CallFrame *framePtr, int *isFinished);
typedef int (TclOO_PostCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_Namespace *namespacePtr, int result);
typedef void (TclOO_PmCDDeleteProc)(void *clientData);
typedef void *(TclOO_PmCDCloneProc)(void *clientData);

/*
 * Procedure-like methods have the following extra information.
 */
struct ProcedureMethod {
    int version;		/* Version of this structure. Currently must
				 * be TCLOO_PROCEDURE_METHOD_VERSION_1. */
    Proc *procPtr;		/* Core of the implementation of the method;
				 * includes the argument definition and the
				 * body bytecodes. */
    int flags;			/* Flags to control features. */
    Tcl_Size refCount;
    void *clientData;
    TclOO_PmCDDeleteProc *deleteClientdataProc;
    TclOO_PmCDCloneProc *cloneClientdataProc;
    ProcErrorProc *errProc;	/* Replacement error handler. */
    TclOO_PreCallProc *preCallProc;
				/* Callback to allow for additional setup
				 * before the method executes. */
    TclOO_PostCallProc *postCallProc;
				/* Callback to allow for additional cleanup
				 * after the method executes. */
    GetFrameInfoValueProc *gfivProc;
				/* Callback to allow for fine tuning of how
				 * the method reports itself. */
    Command cmd;		/* Space used to connect to [info frame] */
    ExtraFrameInfo efi;		/* Space used to store data for [info frame] */
    Tcl_Interp *interp;		/* Interpreter in which to compute the name of
				 * the method. */
    Tcl_Method method;		/* Method to compute the name of. */
    int callSiteFlags;		/* Flags from the call chain. Only interested
				 * in whether this is a constructor or
				 * destructor, which we can't know until then
				 * for messy reasons. Other flags are variable
				 * but not used. */
};

enum ProcedureMethodVersion {
    TCLOO_PROCEDURE_METHOD_VERSION_1 = 0
};
#define TCLOO_PROCEDURE_METHOD_VERSION TCLOO_PROCEDURE_METHOD_VERSION_1

/*
 * Flags for use in a ProcedureMethod.
 *
 */
enum ProceudreMethodFlags {
    USE_DECLARER_NS = 0x80	/* When set, the method will use the namespace

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInt.h`, function `struct Method`, lines 47–145. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOInt.h`.

```text
 * The data that needs to be stored per method. This record is used to collect
 * information about all sorts of methods, including forwards, constructors
 * and destructors.
 */
struct Method {
    union {
#ifndef TCL_NO_DEPRECATED
	const Tcl_MethodType *typePtr;
#endif /* TCL_NO_DEPRECATED */
	const Tcl_MethodType2 *type2Ptr;
    };				/* The type of method. If NULL, this is a
				 * special flag record which is just used for
				 * the setting of the flags field. Note that
				 * this is a union of two pointer types that
				 * have the same layout at least as far as the
				 * internal version field. */
    Tcl_Size refCount;
    void *clientData;		/* Type-specific data. */
    Tcl_Obj *namePtr;		/* Name of the method. */
    Object *declaringObjectPtr;	/* The object that declares this method, or
				 * NULL if it was declared by a class. */
    Class *declaringClassPtr;	/* The class that declares this method, or
				 * NULL if it was declared directly on an
				 * object. */
    int flags;			/* Assorted flags. Includes whether this
				 * method is public/exported or not. */
};

/*
 * Pre- and post-call callbacks, to allow procedure-like methods to be fine
 * tuned in their behaviour.
 */

typedef int (TclOO_PreCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_CallFrame *framePtr, int *isFinished);
typedef int (TclOO_PostCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_Namespace *namespacePtr, int result);
typedef void (TclOO_PmCDDeleteProc)(void *clientData);
typedef void *(TclOO_PmCDCloneProc)(void *clientData);

/*
 * Procedure-like methods have the following extra information.
 */
struct ProcedureMethod {
    int version;		/* Version of this structure. Currently must
				 * be TCLOO_PROCEDURE_METHOD_VERSION_1. */
    Proc *procPtr;		/* Core of the implementation of the method;
				 * includes the argument definition and the
				 * body bytecodes. */
    int flags;			/* Flags to control features. */
    Tcl_Size refCount;
    void *clientData;
    TclOO_PmCDDeleteProc *deleteClientdataProc;
    TclOO_PmCDCloneProc *cloneClientdataProc;
    ProcErrorProc *errProc;	/* Replacement error handler. */
    TclOO_PreCallProc *preCallProc;
				/* Callback to allow for additional setup
				 * before the method executes. */
    TclOO_PostCallProc *postCallProc;
				/* Callback to allow for additional cleanup
				 * after the method executes. */
    GetFrameInfoValueProc *gfivProc;
				/* Callback to allow for fine tuning of how
				 * the method reports itself. */
    Command cmd;		/* Space used to connect to [info frame] */
    ExtraFrameInfo efi;		/* Space used to store data for [info frame] */
    Tcl_Interp *interp;		/* Interpreter in which to compute the name of
				 * the method. */
    Tcl_Method method;		/* Method to compute the name of. */
    int callSiteFlags;		/* Flags from the call chain. Only interested
				 * in whether this is a constructor or
				 * destructor, which we can't know until then
				 * for messy reasons. Other flags are variable
				 * but not used. */
};

enum ProcedureMethodVersion {
    TCLOO_PROCEDURE_METHOD_VERSION_1 = 0
};
#define TCLOO_PROCEDURE_METHOD_VERSION TCLOO_PROCEDURE_METHOD_VERSION_1

/*
 * Flags for use in a ProcedureMethod.
 *
 */
enum ProceudreMethodFlags {
    USE_DECLARER_NS = 0x80	/* When set, the method will use the namespace
				 * of the object or class that declared it (or
				 * the clone of it, if it was from such that
				 * the implementation of the method came to the
				 * particular use) instead of the namespace of
				 * the object on which the method was invoked.
				 * This flag must be distinct from all others
				 * that are associated with methods. */
};

/*
 * Forwarded methods have the following extra information.
 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInt.h`, function `struct CallChain`, lines 29–127. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOInt.h`.

```text
/*
 * Forward declarations.
 */

typedef struct CallChain CallChain;
typedef struct CallContext CallContext;
typedef struct Class Class;
typedef struct DeclaredClassMethod DeclaredClassMethod;
typedef struct ForwardMethod ForwardMethod;
typedef struct Foundation Foundation;
typedef struct Method Method;
typedef struct MInvoke MInvoke;
typedef struct Object Object;
typedef struct PrivateVariableMapping PrivateVariableMapping;
typedef struct ProcedureMethod ProcedureMethod;
typedef struct PropertyStorage PropertyStorage;

/*
 * The data that needs to be stored per method. This record is used to collect
 * information about all sorts of methods, including forwards, constructors
 * and destructors.
 */
struct Method {
    union {
#ifndef TCL_NO_DEPRECATED
	const Tcl_MethodType *typePtr;
#endif /* TCL_NO_DEPRECATED */
	const Tcl_MethodType2 *type2Ptr;
    };				/* The type of method. If NULL, this is a
				 * special flag record which is just used for
				 * the setting of the flags field. Note that
				 * this is a union of two pointer types that
				 * have the same layout at least as far as the
				 * internal version field. */
    Tcl_Size refCount;
    void *clientData;		/* Type-specific data. */
    Tcl_Obj *namePtr;		/* Name of the method. */
    Object *declaringObjectPtr;	/* The object that declares this method, or
				 * NULL if it was declared by a class. */
    Class *declaringClassPtr;	/* The class that declares this method, or
				 * NULL if it was declared directly on an
				 * object. */
    int flags;			/* Assorted flags. Includes whether this
				 * method is public/exported or not. */
};

/*
 * Pre- and post-call callbacks, to allow procedure-like methods to be fine
 * tuned in their behaviour.
 */

typedef int (TclOO_PreCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_CallFrame *framePtr, int *isFinished);
typedef int (TclOO_PostCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_Namespace *namespacePtr, int result);
typedef void (TclOO_PmCDDeleteProc)(void *clientData);
typedef void *(TclOO_PmCDCloneProc)(void *clientData);

/*
 * Procedure-like methods have the following extra information.
 */
struct ProcedureMethod {
    int version;		/* Version of this structure. Currently must
				 * be TCLOO_PROCEDURE_METHOD_VERSION_1. */
    Proc *procPtr;		/* Core of the implementation of the method;
				 * includes the argument definition and the
				 * body bytecodes. */
    int flags;			/* Flags to control features. */
    Tcl_Size refCount;
    void *clientData;
    TclOO_PmCDDeleteProc *deleteClientdataProc;
    TclOO_PmCDCloneProc *cloneClientdataProc;
    ProcErrorProc *errProc;	/* Replacement error handler. */
    TclOO_PreCallProc *preCallProc;
				/* Callback to allow for additional setup
				 * before the method executes. */
    TclOO_PostCallProc *postCallProc;
				/* Callback to allow for additional cleanup
				 * after the method executes. */
    GetFrameInfoValueProc *gfivProc;
				/* Callback to allow for fine tuning of how
				 * the method reports itself. */
    Command cmd;		/* Space used to connect to [info frame] */
    ExtraFrameInfo efi;		/* Space used to store data for [info frame] */
    Tcl_Interp *interp;		/* Interpreter in which to compute the name of
				 * the method. */
    Tcl_Method method;		/* Method to compute the name of. */
    int callSiteFlags;		/* Flags from the call chain. Only interested
				 * in whether this is a constructor or
				 * destructor, which we can't know until then
				 * for messy reasons. Other flags are variable
				 * but not used. */
};

enum ProcedureMethodVersion {
    TCLOO_PROCEDURE_METHOD_VERSION_1 = 0
};
#define TCLOO_PROCEDURE_METHOD_VERSION TCLOO_PROCEDURE_METHOD_VERSION_1

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInt.h`, function `struct CallChain`, lines 441–538. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOInt.h`.

```text

/*
 * The cacheable part of a call context.
 */
struct CallChain {
    Tcl_Size objectCreationEpoch;/* The object's creation epoch. Note that the
				 * object reference is not stored in the call
				 * chain; it is in the call context. */
    Tcl_Size objectEpoch;	/* Local (object structure) epoch counter
				 * snapshot. */
    Tcl_Size epoch;		/* Global (class structure) epoch counter
				 * snapshot. */
    int flags;			/* Assorted flags, see below. */
    Tcl_Size refCount;		/* Reference count. */
    Tcl_Size numChain;		/* Size of the call chain. */
    MInvoke *chain;		/* Array of call chain entries. May point to
				 * staticChain if the number of entries is
				 * small. */
    MInvoke staticChain[CALL_CHAIN_STATIC_SIZE];
};

/*
 * A call context structure is built when a method is called. It contains the
 * chain of method implementations that are to be invoked by a particular
 * call, and the process of calling walks the chain, with the [next] command
 * proceeding to the next entry in the chain.
 */
struct CallContext {
    Object *oPtr;		/* The object associated with this call. */
    Tcl_Size index;		/* Index into the call chain of the currently
				 * executing method implementation. */
    Tcl_Size skip;		/* Current number of arguments to skip; can
				 * vary depending on whether it is a direct
				 * method call or a continuation via the
				 * [next] command. */
    CallChain *callPtr;		/* The actual call chain. */
};

/*
 * Bits for the 'flags' field of the call chain.
 */
enum TclOOCallChainFlags {
    PUBLIC_METHOD = 0x01,	/* This is a public (exported) method. */
    PRIVATE_METHOD = 0x02,	/* This is a private (class's direct instances
				 * only) method. Supports itcl. */
    OO_UNKNOWN_METHOD = 0x04,	/* This is an unknown method. */
    CONSTRUCTOR = 0x08,		/* This is a constructor. */
    DESTRUCTOR = 0x10,		/* This is a destructor. */
    TRUE_PRIVATE_METHOD = 0x20	/* This is a private method only accessible
				 * from other methods defined on this class
				 * or instance. [TIP #500] */
};
#define SCOPE_FLAGS (PUBLIC_METHOD | PRIVATE_METHOD | TRUE_PRIVATE_METHOD)

/*
 * Structure containing definition information about basic class methods.
 */
struct DeclaredClassMethod {
    const char *name;		/* Name of the method in question. */
    bool isPublic;		/* Whether the method is public by default. */
    Tcl_MethodType2 definition;	/* How to call the method. */
};

/*
 *----------------------------------------------------------------
 * Commands relating to OO support.
 *----------------------------------------------------------------
 */

MODULE_SCOPE int		TclOOInit(Tcl_Interp *interp);
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOOObjDefObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineClassMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineConstructorObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineDefnNsObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineDeleteMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineDestructorObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineExportObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineForwardObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineInitialiseObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineRenameMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineUnexportObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineClassObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineSelfObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineObjSelfObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefinePrivateObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefinePropertyCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOOUnknownDefinition;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOOCallbackObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOOClassVariableObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOOCopyObjectCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODelegateNameObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOOLinkObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOONextObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOONextToObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOOSelfObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOOInfoObjectPropCmd;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInt.h`, function `struct MInvoke`, lines 36–134. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOInt.h`.

```text
typedef struct DeclaredClassMethod DeclaredClassMethod;
typedef struct ForwardMethod ForwardMethod;
typedef struct Foundation Foundation;
typedef struct Method Method;
typedef struct MInvoke MInvoke;
typedef struct Object Object;
typedef struct PrivateVariableMapping PrivateVariableMapping;
typedef struct ProcedureMethod ProcedureMethod;
typedef struct PropertyStorage PropertyStorage;

/*
 * The data that needs to be stored per method. This record is used to collect
 * information about all sorts of methods, including forwards, constructors
 * and destructors.
 */
struct Method {
    union {
#ifndef TCL_NO_DEPRECATED
	const Tcl_MethodType *typePtr;
#endif /* TCL_NO_DEPRECATED */
	const Tcl_MethodType2 *type2Ptr;
    };				/* The type of method. If NULL, this is a
				 * special flag record which is just used for
				 * the setting of the flags field. Note that
				 * this is a union of two pointer types that
				 * have the same layout at least as far as the
				 * internal version field. */
    Tcl_Size refCount;
    void *clientData;		/* Type-specific data. */
    Tcl_Obj *namePtr;		/* Name of the method. */
    Object *declaringObjectPtr;	/* The object that declares this method, or
				 * NULL if it was declared by a class. */
    Class *declaringClassPtr;	/* The class that declares this method, or
				 * NULL if it was declared directly on an
				 * object. */
    int flags;			/* Assorted flags. Includes whether this
				 * method is public/exported or not. */
};

/*
 * Pre- and post-call callbacks, to allow procedure-like methods to be fine
 * tuned in their behaviour.
 */

typedef int (TclOO_PreCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_CallFrame *framePtr, int *isFinished);
typedef int (TclOO_PostCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext context, Tcl_Namespace *namespacePtr, int result);
typedef void (TclOO_PmCDDeleteProc)(void *clientData);
typedef void *(TclOO_PmCDCloneProc)(void *clientData);

/*
 * Procedure-like methods have the following extra information.
 */
struct ProcedureMethod {
    int version;		/* Version of this structure. Currently must
				 * be TCLOO_PROCEDURE_METHOD_VERSION_1. */
    Proc *procPtr;		/* Core of the implementation of the method;
				 * includes the argument definition and the
				 * body bytecodes. */
    int flags;			/* Flags to control features. */
    Tcl_Size refCount;
    void *clientData;
    TclOO_PmCDDeleteProc *deleteClientdataProc;
    TclOO_PmCDCloneProc *cloneClientdataProc;
    ProcErrorProc *errProc;	/* Replacement error handler. */
    TclOO_PreCallProc *preCallProc;
				/* Callback to allow for additional setup
				 * before the method executes. */
    TclOO_PostCallProc *postCallProc;
				/* Callback to allow for additional cleanup
				 * after the method executes. */
    GetFrameInfoValueProc *gfivProc;
				/* Callback to allow for fine tuning of how
				 * the method reports itself. */
    Command cmd;		/* Space used to connect to [info frame] */
    ExtraFrameInfo efi;		/* Space used to store data for [info frame] */
    Tcl_Interp *interp;		/* Interpreter in which to compute the name of
				 * the method. */
    Tcl_Method method;		/* Method to compute the name of. */
    int callSiteFlags;		/* Flags from the call chain. Only interested
				 * in whether this is a constructor or
				 * destructor, which we can't know until then
				 * for messy reasons. Other flags are variable
				 * but not used. */
};

enum ProcedureMethodVersion {
    TCLOO_PROCEDURE_METHOD_VERSION_1 = 0
};
#define TCLOO_PROCEDURE_METHOD_VERSION TCLOO_PROCEDURE_METHOD_VERSION_1

/*
 * Flags for use in a ProcedureMethod.
 *
 */
enum ProceudreMethodFlags {
    USE_DECLARER_NS = 0x80	/* When set, the method will use the namespace
				 * of the object or class that declared it (or

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInt.h`, function `struct MInvoke`, lines 430–527. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOInt.h`.

```text
/*
 * Information relating to the invocation of a particular method implementation
 * in a call chain.
 */
struct MInvoke {
    Method *mPtr;		/* Reference to the method implementation
				 * record. */
    int isFilter;		/* Whether this is a filter invocation. */
    Class *filterDeclarer;	/* What class decided to add the filter; if
				 * NULL, it was added by the object. */
};

/*
 * The cacheable part of a call context.
 */
struct CallChain {
    Tcl_Size objectCreationEpoch;/* The object's creation epoch. Note that the
				 * object reference is not stored in the call
				 * chain; it is in the call context. */
    Tcl_Size objectEpoch;	/* Local (object structure) epoch counter
				 * snapshot. */
    Tcl_Size epoch;		/* Global (class structure) epoch counter
				 * snapshot. */
    int flags;			/* Assorted flags, see below. */
    Tcl_Size refCount;		/* Reference count. */
    Tcl_Size numChain;		/* Size of the call chain. */
    MInvoke *chain;		/* Array of call chain entries. May point to
				 * staticChain if the number of entries is
				 * small. */
    MInvoke staticChain[CALL_CHAIN_STATIC_SIZE];
};

/*
 * A call context structure is built when a method is called. It contains the
 * chain of method implementations that are to be invoked by a particular
 * call, and the process of calling walks the chain, with the [next] command
 * proceeding to the next entry in the chain.
 */
struct CallContext {
    Object *oPtr;		/* The object associated with this call. */
    Tcl_Size index;		/* Index into the call chain of the currently
				 * executing method implementation. */
    Tcl_Size skip;		/* Current number of arguments to skip; can
				 * vary depending on whether it is a direct
				 * method call or a continuation via the
				 * [next] command. */
    CallChain *callPtr;		/* The actual call chain. */
};

/*
 * Bits for the 'flags' field of the call chain.
 */
enum TclOOCallChainFlags {
    PUBLIC_METHOD = 0x01,	/* This is a public (exported) method. */
    PRIVATE_METHOD = 0x02,	/* This is a private (class's direct instances
				 * only) method. Supports itcl. */
    OO_UNKNOWN_METHOD = 0x04,	/* This is an unknown method. */
    CONSTRUCTOR = 0x08,		/* This is a constructor. */
    DESTRUCTOR = 0x10,		/* This is a destructor. */
    TRUE_PRIVATE_METHOD = 0x20	/* This is a private method only accessible
				 * from other methods defined on this class
				 * or instance. [TIP #500] */
};
#define SCOPE_FLAGS (PUBLIC_METHOD | PRIVATE_METHOD | TRUE_PRIVATE_METHOD)

/*
 * Structure containing definition information about basic class methods.
 */
struct DeclaredClassMethod {
    const char *name;		/* Name of the method in question. */
    bool isPublic;		/* Whether the method is public by default. */
    Tcl_MethodType2 definition;	/* How to call the method. */
};

/*
 *----------------------------------------------------------------
 * Commands relating to OO support.
 *----------------------------------------------------------------
 */

MODULE_SCOPE int		TclOOInit(Tcl_Interp *interp);
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOOObjDefObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineClassMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineConstructorObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineDefnNsObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineDeleteMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineDestructorObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineExportObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineForwardObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineInitialiseObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineRenameMethodObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineUnexportObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineClassObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineSelfObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefineObjSelfObjCmd;
MODULE_SCOPE Tcl_ObjCmdProc2	TclOODefinePrivateObjCmd;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `typedef int (Tcl_MethodCallProc`, lines 61–157. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
 * and to allow the attachment of arbitrary data to objects and classes.
 */

#ifndef TCL_NO_DEPRECATED
typedef int (Tcl_MethodCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext objectContext, int objc, Tcl_Obj *const *objv);
#endif /* TCL_NO_DEPRECATED */
typedef int (Tcl_MethodCallProc2)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext objectContext, Tcl_Size objc, Tcl_Obj *const *objv);
typedef void (Tcl_MethodDeleteProc)(void *clientData);
typedef int (Tcl_CloneProc)(Tcl_Interp *interp, void *oldClientData,
	void **newClientData);
typedef void (Tcl_ObjectMetadataDeleteProc)(void *clientData);
typedef int (Tcl_ObjectMapMethodNameProc)(Tcl_Interp *interp,
	Tcl_Object object, Tcl_Class *startClsPtr, Tcl_Obj *methodNameObj);

/*
 * The type of a method implementation. This describes how to call the method
 * implementation, how to delete it (when the object or class is deleted) and
 * how to create a clone of it (when the object or class is copied).
 */

#ifndef TCL_NO_DEPRECATED
typedef struct Tcl_MethodType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_(1|CURRENT) in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType;
#endif /* TCL_NO_DEPRECATED */

typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
#ifndef TCL_NO_DEPRECATED
    TCL_OO_METHOD_VERSION_CURRENT = 1,
    TCL_OO_METHOD_VERSION_1 = 1,
#endif /* TCL_NO_DEPRECATED */
    TCL_OO_METHOD_VERSION_2 = 2
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `typedef int (Tcl_MethodCallProc`, lines 64–160. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
#ifndef TCL_NO_DEPRECATED
typedef int (Tcl_MethodCallProc)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext objectContext, int objc, Tcl_Obj *const *objv);
#endif /* TCL_NO_DEPRECATED */
typedef int (Tcl_MethodCallProc2)(void *clientData, Tcl_Interp *interp,
	Tcl_ObjectContext objectContext, Tcl_Size objc, Tcl_Obj *const *objv);
typedef void (Tcl_MethodDeleteProc)(void *clientData);
typedef int (Tcl_CloneProc)(Tcl_Interp *interp, void *oldClientData,
	void **newClientData);
typedef void (Tcl_ObjectMetadataDeleteProc)(void *clientData);
typedef int (Tcl_ObjectMapMethodNameProc)(Tcl_Interp *interp,
	Tcl_Object object, Tcl_Class *startClsPtr, Tcl_Obj *methodNameObj);

/*
 * The type of a method implementation. This describes how to call the method
 * implementation, how to delete it (when the object or class is deleted) and
 * how to create a clone of it (when the object or class is copied).
 */

#ifndef TCL_NO_DEPRECATED
typedef struct Tcl_MethodType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_(1|CURRENT) in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType;
#endif /* TCL_NO_DEPRECATED */

typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
#ifndef TCL_NO_DEPRECATED
    TCL_OO_METHOD_VERSION_CURRENT = 1,
    TCL_OO_METHOD_VERSION_1 = 1,
#endif /* TCL_NO_DEPRECATED */
    TCL_OO_METHOD_VERSION_2 = 2
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `typedef struct Tcl_MethodType`, lines 80–176. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
 * how to create a clone of it (when the object or class is copied).
 */

#ifndef TCL_NO_DEPRECATED
typedef struct Tcl_MethodType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_(1|CURRENT) in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType;
#endif /* TCL_NO_DEPRECATED */

typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
#ifndef TCL_NO_DEPRECATED
    TCL_OO_METHOD_VERSION_CURRENT = 1,
    TCL_OO_METHOD_VERSION_1 = 1,
#endif /* TCL_NO_DEPRECATED */
    TCL_OO_METHOD_VERSION_2 = 2
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `typedef struct Tcl_MethodType`, lines 98–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
				 * be copied directly. */
} Tcl_MethodType;
#endif /* TCL_NO_DEPRECATED */

typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
#ifndef TCL_NO_DEPRECATED
    TCL_OO_METHOD_VERSION_CURRENT = 1,
    TCL_OO_METHOD_VERSION_1 = 1,
#endif /* TCL_NO_DEPRECATED */
    TCL_OO_METHOD_VERSION_2 = 2
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 82–178. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text

#ifndef TCL_NO_DEPRECATED
typedef struct Tcl_MethodType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_(1|CURRENT) in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType;
#endif /* TCL_NO_DEPRECATED */

typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
#ifndef TCL_NO_DEPRECATED
    TCL_OO_METHOD_VERSION_CURRENT = 1,
    TCL_OO_METHOD_VERSION_1 = 1,
#endif /* TCL_NO_DEPRECATED */
    TCL_OO_METHOD_VERSION_2 = 2
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 100–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
#endif /* TCL_NO_DEPRECATED */

typedef struct Tcl_MethodType2 {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METHOD_VERSION_2 in
				 * declarations. */
    const char *name;		/* Name of this type of method, mostly for
				 * debugging purposes. */
    Tcl_MethodCallProc2 *callProc;
				/* How to invoke this method. */
    Tcl_MethodDeleteProc *deleteProc;
				/* How to delete this method's type-specific
				 * data, or NULL if the type-specific data
				 * does not need deleting. */
    Tcl_CloneProc *cloneProc;	/* How to copy this method's type-specific
				 * data, or NULL if the type-specific data can
				 * be copied directly. */
} Tcl_MethodType2;

/*
 * The correct value for the version field of the Tcl_MethodType structure.
 * This allows new versions of the structure to be introduced without breaking
 * binary compatibility.
 */
enum TclOOMethodVersion {
#ifndef TCL_NO_DEPRECATED
    TCL_OO_METHOD_VERSION_CURRENT = 1,
    TCL_OO_METHOD_VERSION_1 = 1,
#endif /* TCL_NO_DEPRECATED */
    TCL_OO_METHOD_VERSION_2 = 2
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 122–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
 * binary compatibility.
 */
enum TclOOMethodVersion {
#ifndef TCL_NO_DEPRECATED
    TCL_OO_METHOD_VERSION_CURRENT = 1,
    TCL_OO_METHOD_VERSION_1 = 1,
#endif /* TCL_NO_DEPRECATED */
    TCL_OO_METHOD_VERSION_2 = 2
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 123–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
 */
enum TclOOMethodVersion {
#ifndef TCL_NO_DEPRECATED
    TCL_OO_METHOD_VERSION_CURRENT = 1,
    TCL_OO_METHOD_VERSION_1 = 1,
#endif /* TCL_NO_DEPRECATED */
    TCL_OO_METHOD_VERSION_2 = 2
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 125–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
#ifndef TCL_NO_DEPRECATED
    TCL_OO_METHOD_VERSION_CURRENT = 1,
    TCL_OO_METHOD_VERSION_1 = 1,
#endif /* TCL_NO_DEPRECATED */
    TCL_OO_METHOD_VERSION_2 = 2
};

/*
 * Visibility constants for the flags parameter to Tcl_NewMethod and
 * Tcl_NewInstanceMethod.
 */
enum TclOOMethodVisibilityFlags {
    TCL_OO_METHOD_PUBLIC = 1,
    TCL_OO_METHOD_UNEXPORTED = 0,
    TCL_OO_METHOD_PRIVATE = 0x20
};

/*
 * The type of some object (or class) metadata. This describes how to delete
 * the metadata (when the object or class is deleted) and how to create a
 * clone of it (when the object or class is copied).
 */

typedef struct Tcl_ObjectMetadataType {
    int version;		/* Structure version field. Always to be equal
				 * to TCL_OO_METADATA_VERSION_CURRENT in
				 * declarations. */
    const char *name;
    Tcl_ObjectMetadataDeleteProc *deleteProc;
				/* How to delete the metadata. This must not
				 * be NULL. */
    Tcl_CloneProc *cloneProc;	/* How to copy the metadata, or NULL if the
				 * type-specific data can be copied
				 * directly. */
} Tcl_ObjectMetadataType;

/*
 * The correct value for the version field of the Tcl_ObjectMetadataType
 * structure. This allows new versions of the structure to be introduced
 * without breaking binary compatibility.
 */

enum TclOOMetadataVersion {
    TCL_OO_METADATA_VERSION_CURRENT = 1,
    TCL_OO_METADATA_VERSION_1 = 1
};

/*
 * Include all the public API, generated from tclOO.decls.
 */

#include "tclOODecls.h"  /* IWYU pragma: export */

#ifdef __cplusplus
}
#endif
#endif

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 65–163. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType *typePtr,
				void *clientData);
/* 12 */
TCLAPI Tcl_Method	Tcl_NewMethod(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,
				const Tcl_MethodType *typePtr,
				void *clientData);
/* 13 */
TCLAPI Tcl_Object	Tcl_NewObjectInstance(Tcl_Interp *interp,
				Tcl_Class cls, const char *nameStr,
				const char *nsNameStr, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 14 */
TCLAPI int		Tcl_ObjectDeleted(Tcl_Object object);
/* 15 */
TCLAPI int		Tcl_ObjectContextIsFiltering(
				Tcl_ObjectContext context);
/* 16 */
TCLAPI Tcl_Method	Tcl_ObjectContextMethod(Tcl_ObjectContext context);
/* 17 */
TCLAPI Tcl_Object	Tcl_ObjectContextObject(Tcl_ObjectContext context);
/* 18 */
TCLAPI Tcl_Size		Tcl_ObjectContextSkippedArgs(
				Tcl_ObjectContext context);
/* 19 */
TCLAPI void *		Tcl_ClassGetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr);
/* 20 */
TCLAPI void		Tcl_ClassSetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 21 */
TCLAPI void *		Tcl_ObjectGetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr);
/* 22 */
TCLAPI void		Tcl_ObjectSetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 23 */
TCLAPI int		Tcl_ObjectContextInvokeNext(Tcl_Interp *interp,
				Tcl_ObjectContext context, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 24 */
TCLAPI Tcl_ObjectMapMethodNameProc * Tcl_ObjectGetMethodNameMapper(
				Tcl_Object object);
/* 25 */
TCLAPI void		Tcl_ObjectSetMethodNameMapper(Tcl_Object object,
				Tcl_ObjectMapMethodNameProc *mapMethodNameProc);
/* 26 */
TCLAPI void		Tcl_ClassSetConstructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 27 */
TCLAPI void		Tcl_ClassSetDestructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 28 */
TCLAPI Tcl_Obj *	Tcl_GetObjectName(Tcl_Interp *interp,
				Tcl_Object object);
/* 29 */
TCLAPI int		Tcl_MethodIsPrivate(Tcl_Method method);
/* 30 */
TCLAPI Tcl_Class	Tcl_GetClassOfObject(Tcl_Object object);
/* 31 */
TCLAPI Tcl_Obj *	Tcl_GetObjectClassName(Tcl_Interp *interp,
				Tcl_Object object);
/* 32 */
TCLAPI int		Tcl_MethodIsType2(Tcl_Method method,
				const Tcl_MethodType2 *typePtr,
				void **clientDataPtr);
/* 33 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod2(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType2 *typePtr,
				void *clientData);
/* 34 */
TCLAPI Tcl_Method	Tcl_NewMethod2(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,
				const Tcl_MethodType2 *typePtr,
				void *clientData);

typedef struct {
    const struct TclOOIntStubs *tclOOIntStubs;
} TclOOStubHooks;

typedef struct TclOOStubs {
    int magic;
    const TclOOStubHooks *hooks;

    Tcl_Object (*tcl_CopyObjectInstance) (Tcl_Interp *interp, Tcl_Object sourceObject, const char *targetName, const char *targetNamespaceName); /* 0 */
    Tcl_Object (*tcl_GetClassAsObject) (Tcl_Class clazz); /* 1 */
    Tcl_Class (*tcl_GetObjectAsClass) (Tcl_Object object); /* 2 */
    Tcl_Command (*tcl_GetObjectCommand) (Tcl_Object object); /* 3 */
    Tcl_Object (*tcl_GetObjectFromObj) (Tcl_Interp *interp, Tcl_Obj *objPtr); /* 4 */
    Tcl_Namespace * (*tcl_GetObjectNamespace) (Tcl_Object object); /* 5 */
    Tcl_Class (*tcl_MethodDeclarerClass) (Tcl_Method method); /* 6 */
    Tcl_Object (*tcl_MethodDeclarerObject) (Tcl_Method method); /* 7 */
    int (*tcl_MethodIsPublic) (Tcl_Method method); /* 8 */
    int (*tcl_MethodIsType) (Tcl_Method method, const Tcl_MethodType *typePtr, void **clientDataPtr); /* 9 */
    Tcl_Obj * (*tcl_MethodName) (Tcl_Method method); /* 10 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 136–234. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType2 *typePtr,
				void *clientData);
/* 34 */
TCLAPI Tcl_Method	Tcl_NewMethod2(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,
				const Tcl_MethodType2 *typePtr,
				void *clientData);

typedef struct {
    const struct TclOOIntStubs *tclOOIntStubs;
} TclOOStubHooks;

typedef struct TclOOStubs {
    int magic;
    const TclOOStubHooks *hooks;

    Tcl_Object (*tcl_CopyObjectInstance) (Tcl_Interp *interp, Tcl_Object sourceObject, const char *targetName, const char *targetNamespaceName); /* 0 */
    Tcl_Object (*tcl_GetClassAsObject) (Tcl_Class clazz); /* 1 */
    Tcl_Class (*tcl_GetObjectAsClass) (Tcl_Object object); /* 2 */
    Tcl_Command (*tcl_GetObjectCommand) (Tcl_Object object); /* 3 */
    Tcl_Object (*tcl_GetObjectFromObj) (Tcl_Interp *interp, Tcl_Obj *objPtr); /* 4 */
    Tcl_Namespace * (*tcl_GetObjectNamespace) (Tcl_Object object); /* 5 */
    Tcl_Class (*tcl_MethodDeclarerClass) (Tcl_Method method); /* 6 */
    Tcl_Object (*tcl_MethodDeclarerObject) (Tcl_Method method); /* 7 */
    int (*tcl_MethodIsPublic) (Tcl_Method method); /* 8 */
    int (*tcl_MethodIsType) (Tcl_Method method, const Tcl_MethodType *typePtr, void **clientDataPtr); /* 9 */
    Tcl_Obj * (*tcl_MethodName) (Tcl_Method method); /* 10 */
    Tcl_Method (*tcl_NewInstanceMethod) (Tcl_Interp *interp, Tcl_Object object, Tcl_Obj *nameObj, int flags, const Tcl_MethodType *typePtr, void *clientData); /* 11 */
    Tcl_Method (*tcl_NewMethod) (Tcl_Interp *interp, Tcl_Class cls, Tcl_Obj *nameObj, int flags, const Tcl_MethodType *typePtr, void *clientData); /* 12 */
    Tcl_Object (*tcl_NewObjectInstance) (Tcl_Interp *interp, Tcl_Class cls, const char *nameStr, const char *nsNameStr, Tcl_Size objc, Tcl_Obj *const *objv, Tcl_Size skip); /* 13 */
    int (*tcl_ObjectDeleted) (Tcl_Object object); /* 14 */
    int (*tcl_ObjectContextIsFiltering) (Tcl_ObjectContext context); /* 15 */
    Tcl_Method (*tcl_ObjectContextMethod) (Tcl_ObjectContext context); /* 16 */
    Tcl_Object (*tcl_ObjectContextObject) (Tcl_ObjectContext context); /* 17 */
    Tcl_Size (*tcl_ObjectContextSkippedArgs) (Tcl_ObjectContext context); /* 18 */
    void * (*tcl_ClassGetMetadata) (Tcl_Class clazz, const Tcl_ObjectMetadataType *typePtr); /* 19 */
    void (*tcl_ClassSetMetadata) (Tcl_Class clazz, const Tcl_ObjectMetadataType *typePtr, void *metadata); /* 20 */
    void * (*tcl_ObjectGetMetadata) (Tcl_Object object, const Tcl_ObjectMetadataType *typePtr); /* 21 */
    void (*tcl_ObjectSetMetadata) (Tcl_Object object, const Tcl_ObjectMetadataType *typePtr, void *metadata); /* 22 */
    int (*tcl_ObjectContextInvokeNext) (Tcl_Interp *interp, Tcl_ObjectContext context, Tcl_Size objc, Tcl_Obj *const *objv, Tcl_Size skip); /* 23 */
    Tcl_ObjectMapMethodNameProc * (*tcl_ObjectGetMethodNameMapper) (Tcl_Object object); /* 24 */
    void (*tcl_ObjectSetMethodNameMapper) (Tcl_Object object, Tcl_ObjectMapMethodNameProc *mapMethodNameProc); /* 25 */
    void (*tcl_ClassSetConstructor) (Tcl_Interp *interp, Tcl_Class clazz, Tcl_Method method); /* 26 */
    void (*tcl_ClassSetDestructor) (Tcl_Interp *interp, Tcl_Class clazz, Tcl_Method method); /* 27 */
    Tcl_Obj * (*tcl_GetObjectName) (Tcl_Interp *interp, Tcl_Object object); /* 28 */
    int (*tcl_MethodIsPrivate) (Tcl_Method method); /* 29 */
    Tcl_Class (*tcl_GetClassOfObject) (Tcl_Object object); /* 30 */
    Tcl_Obj * (*tcl_GetObjectClassName) (Tcl_Interp *interp, Tcl_Object object); /* 31 */
    int (*tcl_MethodIsType2) (Tcl_Method method, const Tcl_MethodType2 *typePtr, void **clientDataPtr); /* 32 */
    Tcl_Method (*tcl_NewInstanceMethod2) (Tcl_Interp *interp, Tcl_Object object, Tcl_Obj *nameObj, int flags, const Tcl_MethodType2 *typePtr, void *clientData); /* 33 */
    Tcl_Method (*tcl_NewMethod2) (Tcl_Interp *interp, Tcl_Class cls, Tcl_Obj *nameObj, int flags, const Tcl_MethodType2 *typePtr, void *clientData); /* 34 */
} TclOOStubs;

extern const TclOOStubs *tclOOStubsPtr;

#ifdef __cplusplus
}
#endif

#if defined(USE_TCLOO_STUBS)

/*
 * Inline function declarations:
 */

#define Tcl_CopyObjectInstance \
	(tclOOStubsPtr->tcl_CopyObjectInstance) /* 0 */
#define Tcl_GetClassAsObject \
	(tclOOStubsPtr->tcl_GetClassAsObject) /* 1 */
#define Tcl_GetObjectAsClass \
	(tclOOStubsPtr->tcl_GetObjectAsClass) /* 2 */
#define Tcl_GetObjectCommand \
	(tclOOStubsPtr->tcl_GetObjectCommand) /* 3 */
#define Tcl_GetObjectFromObj \
	(tclOOStubsPtr->tcl_GetObjectFromObj) /* 4 */
#define Tcl_GetObjectNamespace \
	(tclOOStubsPtr->tcl_GetObjectNamespace) /* 5 */
#define Tcl_MethodDeclarerClass \
	(tclOOStubsPtr->tcl_MethodDeclarerClass) /* 6 */
#define Tcl_MethodDeclarerObject \
	(tclOOStubsPtr->tcl_MethodDeclarerObject) /* 7 */
#define Tcl_MethodIsPublic \
	(tclOOStubsPtr->tcl_MethodIsPublic) /* 8 */
#define Tcl_MethodIsType \
	(tclOOStubsPtr->tcl_MethodIsType) /* 9 */
#define Tcl_MethodName \
	(tclOOStubsPtr->tcl_MethodName) /* 10 */
#define Tcl_NewInstanceMethod \
	(tclOOStubsPtr->tcl_NewInstanceMethod) /* 11 */
#define Tcl_NewMethod \
	(tclOOStubsPtr->tcl_NewMethod) /* 12 */
#define Tcl_NewObjectInstance \
	(tclOOStubsPtr->tcl_NewObjectInstance) /* 13 */
#define Tcl_ObjectDeleted \
	(tclOOStubsPtr->tcl_ObjectDeleted) /* 14 */
#define Tcl_ObjectContextIsFiltering \
	(tclOOStubsPtr->tcl_ObjectContextIsFiltering) /* 15 */
#define Tcl_ObjectContextMethod \

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 222–319. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
#define Tcl_MethodName \
	(tclOOStubsPtr->tcl_MethodName) /* 10 */
#define Tcl_NewInstanceMethod \
	(tclOOStubsPtr->tcl_NewInstanceMethod) /* 11 */
#define Tcl_NewMethod \
	(tclOOStubsPtr->tcl_NewMethod) /* 12 */
#define Tcl_NewObjectInstance \
	(tclOOStubsPtr->tcl_NewObjectInstance) /* 13 */
#define Tcl_ObjectDeleted \
	(tclOOStubsPtr->tcl_ObjectDeleted) /* 14 */
#define Tcl_ObjectContextIsFiltering \
	(tclOOStubsPtr->tcl_ObjectContextIsFiltering) /* 15 */
#define Tcl_ObjectContextMethod \
	(tclOOStubsPtr->tcl_ObjectContextMethod) /* 16 */
#define Tcl_ObjectContextObject \
	(tclOOStubsPtr->tcl_ObjectContextObject) /* 17 */
#define Tcl_ObjectContextSkippedArgs \
	(tclOOStubsPtr->tcl_ObjectContextSkippedArgs) /* 18 */
#define Tcl_ClassGetMetadata \
	(tclOOStubsPtr->tcl_ClassGetMetadata) /* 19 */
#define Tcl_ClassSetMetadata \
	(tclOOStubsPtr->tcl_ClassSetMetadata) /* 20 */
#define Tcl_ObjectGetMetadata \
	(tclOOStubsPtr->tcl_ObjectGetMetadata) /* 21 */
#define Tcl_ObjectSetMetadata \
	(tclOOStubsPtr->tcl_ObjectSetMetadata) /* 22 */
#define Tcl_ObjectContextInvokeNext \
	(tclOOStubsPtr->tcl_ObjectContextInvokeNext) /* 23 */
#define Tcl_ObjectGetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectGetMethodNameMapper) /* 24 */
#define Tcl_ObjectSetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectSetMethodNameMapper) /* 25 */
#define Tcl_ClassSetConstructor \
	(tclOOStubsPtr->tcl_ClassSetConstructor) /* 26 */
#define Tcl_ClassSetDestructor \
	(tclOOStubsPtr->tcl_ClassSetDestructor) /* 27 */
#define Tcl_GetObjectName \
	(tclOOStubsPtr->tcl_GetObjectName) /* 28 */
#define Tcl_MethodIsPrivate \
	(tclOOStubsPtr->tcl_MethodIsPrivate) /* 29 */
#define Tcl_GetClassOfObject \
	(tclOOStubsPtr->tcl_GetClassOfObject) /* 30 */
#define Tcl_GetObjectClassName \
	(tclOOStubsPtr->tcl_GetObjectClassName) /* 31 */
#define Tcl_MethodIsType2 \
	(tclOOStubsPtr->tcl_MethodIsType2) /* 32 */
#define Tcl_NewInstanceMethod2 \
	(tclOOStubsPtr->tcl_NewInstanceMethod2) /* 33 */
#define Tcl_NewMethod2 \
	(tclOOStubsPtr->tcl_NewMethod2) /* 34 */

#endif /* defined(USE_TCLOO_STUBS) */

/* !END!: Do not edit above this line. */

#ifdef TCL_NO_DEPRECATED
#   undef Tcl_MethodType
#   undef Tcl_MethodIsType
#   undef Tcl_NewInstanceMethod
#   undef Tcl_NewMethod
#endif

#if defined(__STDC_VERSION__) && (__STDC_VERSION__ >= 201112L)
#ifndef TclOOGeneric
/* Select method based on type of argument. */
#define TclOOGeneric(typePtr, impl) \
    _Generic(typePtr, default: impl, const Tcl_MethodType2 *: impl ## 2)
#endif

#ifdef USE_TCLOO_STUBS

#undef Tcl_MethodIsType
#define Tcl_MethodIsType(method, typePtr, clientDataPtr) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_MethodIsType) \
	((method), (typePtr), (clientDataPtr)))
#undef Tcl_NewInstanceMethod
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))
#undef Tcl_NewMethod
#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#else
#define Tcl_MethodIsType(method, typePtr, clientDataPtr) \
    (TclOOGeneric((typePtr), Tcl_MethodIsType) \
	((method), (typePtr), (clientDataPtr)))
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))

#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#endif
#endif

#endif /* _TCLOODECLS */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 266–319. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
#define Tcl_MethodIsType2 \
	(tclOOStubsPtr->tcl_MethodIsType2) /* 32 */
#define Tcl_NewInstanceMethod2 \
	(tclOOStubsPtr->tcl_NewInstanceMethod2) /* 33 */
#define Tcl_NewMethod2 \
	(tclOOStubsPtr->tcl_NewMethod2) /* 34 */

#endif /* defined(USE_TCLOO_STUBS) */

/* !END!: Do not edit above this line. */

#ifdef TCL_NO_DEPRECATED
#   undef Tcl_MethodType
#   undef Tcl_MethodIsType
#   undef Tcl_NewInstanceMethod
#   undef Tcl_NewMethod
#endif

#if defined(__STDC_VERSION__) && (__STDC_VERSION__ >= 201112L)
#ifndef TclOOGeneric
/* Select method based on type of argument. */
#define TclOOGeneric(typePtr, impl) \
    _Generic(typePtr, default: impl, const Tcl_MethodType2 *: impl ## 2)
#endif

#ifdef USE_TCLOO_STUBS

#undef Tcl_MethodIsType
#define Tcl_MethodIsType(method, typePtr, clientDataPtr) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_MethodIsType) \
	((method), (typePtr), (clientDataPtr)))
#undef Tcl_NewInstanceMethod
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))
#undef Tcl_NewMethod
#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#else
#define Tcl_MethodIsType(method, typePtr, clientDataPtr) \
    (TclOOGeneric((typePtr), Tcl_MethodIsType) \
	((method), (typePtr), (clientDataPtr)))
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))

#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#endif
#endif

#endif /* _TCLOODECLS */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 277–319. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
#ifdef TCL_NO_DEPRECATED
#   undef Tcl_MethodType
#   undef Tcl_MethodIsType
#   undef Tcl_NewInstanceMethod
#   undef Tcl_NewMethod
#endif

#if defined(__STDC_VERSION__) && (__STDC_VERSION__ >= 201112L)
#ifndef TclOOGeneric
/* Select method based on type of argument. */
#define TclOOGeneric(typePtr, impl) \
    _Generic(typePtr, default: impl, const Tcl_MethodType2 *: impl ## 2)
#endif

#ifdef USE_TCLOO_STUBS

#undef Tcl_MethodIsType
#define Tcl_MethodIsType(method, typePtr, clientDataPtr) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_MethodIsType) \
	((method), (typePtr), (clientDataPtr)))
#undef Tcl_NewInstanceMethod
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))
#undef Tcl_NewMethod
#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#else
#define Tcl_MethodIsType(method, typePtr, clientDataPtr) \
    (TclOOGeneric((typePtr), Tcl_MethodIsType) \
	((method), (typePtr), (clientDataPtr)))
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))

#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#endif
#endif

#endif /* _TCLOODECLS */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 297–319. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
#undef Tcl_NewInstanceMethod
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))
#undef Tcl_NewMethod
#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#else
#define Tcl_MethodIsType(method, typePtr, clientDataPtr) \
    (TclOOGeneric((typePtr), Tcl_MethodIsType) \
	((method), (typePtr), (clientDataPtr)))
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))

#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#endif
#endif

#endif /* _TCLOODECLS */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 298–319. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))
#undef Tcl_NewMethod
#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#else
#define Tcl_MethodIsType(method, typePtr, clientDataPtr) \
    (TclOOGeneric((typePtr), Tcl_MethodIsType) \
	((method), (typePtr), (clientDataPtr)))
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))

#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#endif
#endif

#endif /* _TCLOODECLS */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 309–319. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))

#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#endif
#endif

#endif /* _TCLOODECLS */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_NewMethod`, lines 310–319. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
    (TclOOGeneric((typePtr), Tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))

#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), Tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))
#endif
#endif

#endif /* _TCLOODECLS */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_GetObjectFromObj`, lines 43–141. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
TCLAPI Tcl_Class	Tcl_GetObjectAsClass(Tcl_Object object);
/* 3 */
TCLAPI Tcl_Command	Tcl_GetObjectCommand(Tcl_Object object);
/* 4 */
TCLAPI Tcl_Object	Tcl_GetObjectFromObj(Tcl_Interp *interp,
				Tcl_Obj *objPtr);
/* 5 */
TCLAPI Tcl_Namespace *	Tcl_GetObjectNamespace(Tcl_Object object);
/* 6 */
TCLAPI Tcl_Class	Tcl_MethodDeclarerClass(Tcl_Method method);
/* 7 */
TCLAPI Tcl_Object	Tcl_MethodDeclarerObject(Tcl_Method method);
/* 8 */
TCLAPI int		Tcl_MethodIsPublic(Tcl_Method method);
/* 9 */
TCLAPI int		Tcl_MethodIsType(Tcl_Method method,
				const Tcl_MethodType *typePtr,
				void **clientDataPtr);
/* 10 */
TCLAPI Tcl_Obj *	Tcl_MethodName(Tcl_Method method);
/* 11 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType *typePtr,
				void *clientData);
/* 12 */
TCLAPI Tcl_Method	Tcl_NewMethod(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,
				const Tcl_MethodType *typePtr,
				void *clientData);
/* 13 */
TCLAPI Tcl_Object	Tcl_NewObjectInstance(Tcl_Interp *interp,
				Tcl_Class cls, const char *nameStr,
				const char *nsNameStr, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 14 */
TCLAPI int		Tcl_ObjectDeleted(Tcl_Object object);
/* 15 */
TCLAPI int		Tcl_ObjectContextIsFiltering(
				Tcl_ObjectContext context);
/* 16 */
TCLAPI Tcl_Method	Tcl_ObjectContextMethod(Tcl_ObjectContext context);
/* 17 */
TCLAPI Tcl_Object	Tcl_ObjectContextObject(Tcl_ObjectContext context);
/* 18 */
TCLAPI Tcl_Size		Tcl_ObjectContextSkippedArgs(
				Tcl_ObjectContext context);
/* 19 */
TCLAPI void *		Tcl_ClassGetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr);
/* 20 */
TCLAPI void		Tcl_ClassSetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 21 */
TCLAPI void *		Tcl_ObjectGetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr);
/* 22 */
TCLAPI void		Tcl_ObjectSetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 23 */
TCLAPI int		Tcl_ObjectContextInvokeNext(Tcl_Interp *interp,
				Tcl_ObjectContext context, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 24 */
TCLAPI Tcl_ObjectMapMethodNameProc * Tcl_ObjectGetMethodNameMapper(
				Tcl_Object object);
/* 25 */
TCLAPI void		Tcl_ObjectSetMethodNameMapper(Tcl_Object object,
				Tcl_ObjectMapMethodNameProc *mapMethodNameProc);
/* 26 */
TCLAPI void		Tcl_ClassSetConstructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 27 */
TCLAPI void		Tcl_ClassSetDestructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 28 */
TCLAPI Tcl_Obj *	Tcl_GetObjectName(Tcl_Interp *interp,
				Tcl_Object object);
/* 29 */
TCLAPI int		Tcl_MethodIsPrivate(Tcl_Method method);
/* 30 */
TCLAPI Tcl_Class	Tcl_GetClassOfObject(Tcl_Object object);
/* 31 */
TCLAPI Tcl_Obj *	Tcl_GetObjectClassName(Tcl_Interp *interp,
				Tcl_Object object);
/* 32 */
TCLAPI int		Tcl_MethodIsType2(Tcl_Method method,
				const Tcl_MethodType2 *typePtr,
				void **clientDataPtr);
/* 33 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod2(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType2 *typePtr,
				void *clientData);
/* 34 */
TCLAPI Tcl_Method	Tcl_NewMethod2(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_GetObjectFromObj`, lines 206–304. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
#define Tcl_GetObjectAsClass \
	(tclOOStubsPtr->tcl_GetObjectAsClass) /* 2 */
#define Tcl_GetObjectCommand \
	(tclOOStubsPtr->tcl_GetObjectCommand) /* 3 */
#define Tcl_GetObjectFromObj \
	(tclOOStubsPtr->tcl_GetObjectFromObj) /* 4 */
#define Tcl_GetObjectNamespace \
	(tclOOStubsPtr->tcl_GetObjectNamespace) /* 5 */
#define Tcl_MethodDeclarerClass \
	(tclOOStubsPtr->tcl_MethodDeclarerClass) /* 6 */
#define Tcl_MethodDeclarerObject \
	(tclOOStubsPtr->tcl_MethodDeclarerObject) /* 7 */
#define Tcl_MethodIsPublic \
	(tclOOStubsPtr->tcl_MethodIsPublic) /* 8 */
#define Tcl_MethodIsType \
	(tclOOStubsPtr->tcl_MethodIsType) /* 9 */
#define Tcl_MethodName \
	(tclOOStubsPtr->tcl_MethodName) /* 10 */
#define Tcl_NewInstanceMethod \
	(tclOOStubsPtr->tcl_NewInstanceMethod) /* 11 */
#define Tcl_NewMethod \
	(tclOOStubsPtr->tcl_NewMethod) /* 12 */
#define Tcl_NewObjectInstance \
	(tclOOStubsPtr->tcl_NewObjectInstance) /* 13 */
#define Tcl_ObjectDeleted \
	(tclOOStubsPtr->tcl_ObjectDeleted) /* 14 */
#define Tcl_ObjectContextIsFiltering \
	(tclOOStubsPtr->tcl_ObjectContextIsFiltering) /* 15 */
#define Tcl_ObjectContextMethod \
	(tclOOStubsPtr->tcl_ObjectContextMethod) /* 16 */
#define Tcl_ObjectContextObject \
	(tclOOStubsPtr->tcl_ObjectContextObject) /* 17 */
#define Tcl_ObjectContextSkippedArgs \
	(tclOOStubsPtr->tcl_ObjectContextSkippedArgs) /* 18 */
#define Tcl_ClassGetMetadata \
	(tclOOStubsPtr->tcl_ClassGetMetadata) /* 19 */
#define Tcl_ClassSetMetadata \
	(tclOOStubsPtr->tcl_ClassSetMetadata) /* 20 */
#define Tcl_ObjectGetMetadata \
	(tclOOStubsPtr->tcl_ObjectGetMetadata) /* 21 */
#define Tcl_ObjectSetMetadata \
	(tclOOStubsPtr->tcl_ObjectSetMetadata) /* 22 */
#define Tcl_ObjectContextInvokeNext \
	(tclOOStubsPtr->tcl_ObjectContextInvokeNext) /* 23 */
#define Tcl_ObjectGetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectGetMethodNameMapper) /* 24 */
#define Tcl_ObjectSetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectSetMethodNameMapper) /* 25 */
#define Tcl_ClassSetConstructor \
	(tclOOStubsPtr->tcl_ClassSetConstructor) /* 26 */
#define Tcl_ClassSetDestructor \
	(tclOOStubsPtr->tcl_ClassSetDestructor) /* 27 */
#define Tcl_GetObjectName \
	(tclOOStubsPtr->tcl_GetObjectName) /* 28 */
#define Tcl_MethodIsPrivate \
	(tclOOStubsPtr->tcl_MethodIsPrivate) /* 29 */
#define Tcl_GetClassOfObject \
	(tclOOStubsPtr->tcl_GetClassOfObject) /* 30 */
#define Tcl_GetObjectClassName \
	(tclOOStubsPtr->tcl_GetObjectClassName) /* 31 */
#define Tcl_MethodIsType2 \
	(tclOOStubsPtr->tcl_MethodIsType2) /* 32 */
#define Tcl_NewInstanceMethod2 \
	(tclOOStubsPtr->tcl_NewInstanceMethod2) /* 33 */
#define Tcl_NewMethod2 \
	(tclOOStubsPtr->tcl_NewMethod2) /* 34 */

#endif /* defined(USE_TCLOO_STUBS) */

/* !END!: Do not edit above this line. */

#ifdef TCL_NO_DEPRECATED
#   undef Tcl_MethodType
#   undef Tcl_MethodIsType
#   undef Tcl_NewInstanceMethod
#   undef Tcl_NewMethod
#endif

#if defined(__STDC_VERSION__) && (__STDC_VERSION__ >= 201112L)
#ifndef TclOOGeneric
/* Select method based on type of argument. */
#define TclOOGeneric(typePtr, impl) \
    _Generic(typePtr, default: impl, const Tcl_MethodType2 *: impl ## 2)
#endif

#ifdef USE_TCLOO_STUBS

#undef Tcl_MethodIsType
#define Tcl_MethodIsType(method, typePtr, clientDataPtr) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_MethodIsType) \
	((method), (typePtr), (clientDataPtr)))
#undef Tcl_NewInstanceMethod
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))
#undef Tcl_NewMethod
#define Tcl_NewMethod(interp, cls, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewMethod) \
	((interp), (cls), (nameObj), (flags), (typePtr), (clientData)))

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_GetObjectAsClass`, lines 39–137. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
				const char *targetNamespaceName);
/* 1 */
TCLAPI Tcl_Object	Tcl_GetClassAsObject(Tcl_Class clazz);
/* 2 */
TCLAPI Tcl_Class	Tcl_GetObjectAsClass(Tcl_Object object);
/* 3 */
TCLAPI Tcl_Command	Tcl_GetObjectCommand(Tcl_Object object);
/* 4 */
TCLAPI Tcl_Object	Tcl_GetObjectFromObj(Tcl_Interp *interp,
				Tcl_Obj *objPtr);
/* 5 */
TCLAPI Tcl_Namespace *	Tcl_GetObjectNamespace(Tcl_Object object);
/* 6 */
TCLAPI Tcl_Class	Tcl_MethodDeclarerClass(Tcl_Method method);
/* 7 */
TCLAPI Tcl_Object	Tcl_MethodDeclarerObject(Tcl_Method method);
/* 8 */
TCLAPI int		Tcl_MethodIsPublic(Tcl_Method method);
/* 9 */
TCLAPI int		Tcl_MethodIsType(Tcl_Method method,
				const Tcl_MethodType *typePtr,
				void **clientDataPtr);
/* 10 */
TCLAPI Tcl_Obj *	Tcl_MethodName(Tcl_Method method);
/* 11 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType *typePtr,
				void *clientData);
/* 12 */
TCLAPI Tcl_Method	Tcl_NewMethod(Tcl_Interp *interp, Tcl_Class cls,
				Tcl_Obj *nameObj, int flags,
				const Tcl_MethodType *typePtr,
				void *clientData);
/* 13 */
TCLAPI Tcl_Object	Tcl_NewObjectInstance(Tcl_Interp *interp,
				Tcl_Class cls, const char *nameStr,
				const char *nsNameStr, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 14 */
TCLAPI int		Tcl_ObjectDeleted(Tcl_Object object);
/* 15 */
TCLAPI int		Tcl_ObjectContextIsFiltering(
				Tcl_ObjectContext context);
/* 16 */
TCLAPI Tcl_Method	Tcl_ObjectContextMethod(Tcl_ObjectContext context);
/* 17 */
TCLAPI Tcl_Object	Tcl_ObjectContextObject(Tcl_ObjectContext context);
/* 18 */
TCLAPI Tcl_Size		Tcl_ObjectContextSkippedArgs(
				Tcl_ObjectContext context);
/* 19 */
TCLAPI void *		Tcl_ClassGetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr);
/* 20 */
TCLAPI void		Tcl_ClassSetMetadata(Tcl_Class clazz,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 21 */
TCLAPI void *		Tcl_ObjectGetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr);
/* 22 */
TCLAPI void		Tcl_ObjectSetMetadata(Tcl_Object object,
				const Tcl_ObjectMetadataType *typePtr,
				void *metadata);
/* 23 */
TCLAPI int		Tcl_ObjectContextInvokeNext(Tcl_Interp *interp,
				Tcl_ObjectContext context, Tcl_Size objc,
				Tcl_Obj *const *objv, Tcl_Size skip);
/* 24 */
TCLAPI Tcl_ObjectMapMethodNameProc * Tcl_ObjectGetMethodNameMapper(
				Tcl_Object object);
/* 25 */
TCLAPI void		Tcl_ObjectSetMethodNameMapper(Tcl_Object object,
				Tcl_ObjectMapMethodNameProc *mapMethodNameProc);
/* 26 */
TCLAPI void		Tcl_ClassSetConstructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 27 */
TCLAPI void		Tcl_ClassSetDestructor(Tcl_Interp *interp,
				Tcl_Class clazz, Tcl_Method method);
/* 28 */
TCLAPI Tcl_Obj *	Tcl_GetObjectName(Tcl_Interp *interp,
				Tcl_Object object);
/* 29 */
TCLAPI int		Tcl_MethodIsPrivate(Tcl_Method method);
/* 30 */
TCLAPI Tcl_Class	Tcl_GetClassOfObject(Tcl_Object object);
/* 31 */
TCLAPI Tcl_Obj *	Tcl_GetObjectClassName(Tcl_Interp *interp,
				Tcl_Object object);
/* 32 */
TCLAPI int		Tcl_MethodIsType2(Tcl_Method method,
				const Tcl_MethodType2 *typePtr,
				void **clientDataPtr);
/* 33 */
TCLAPI Tcl_Method	Tcl_NewInstanceMethod2(Tcl_Interp *interp,
				Tcl_Object object, Tcl_Obj *nameObj,
				int flags, const Tcl_MethodType2 *typePtr,

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODecls.h`, function `Tcl_GetObjectAsClass`, lines 202–300. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h`.

```text
#define Tcl_CopyObjectInstance \
	(tclOOStubsPtr->tcl_CopyObjectInstance) /* 0 */
#define Tcl_GetClassAsObject \
	(tclOOStubsPtr->tcl_GetClassAsObject) /* 1 */
#define Tcl_GetObjectAsClass \
	(tclOOStubsPtr->tcl_GetObjectAsClass) /* 2 */
#define Tcl_GetObjectCommand \
	(tclOOStubsPtr->tcl_GetObjectCommand) /* 3 */
#define Tcl_GetObjectFromObj \
	(tclOOStubsPtr->tcl_GetObjectFromObj) /* 4 */
#define Tcl_GetObjectNamespace \
	(tclOOStubsPtr->tcl_GetObjectNamespace) /* 5 */
#define Tcl_MethodDeclarerClass \
	(tclOOStubsPtr->tcl_MethodDeclarerClass) /* 6 */
#define Tcl_MethodDeclarerObject \
	(tclOOStubsPtr->tcl_MethodDeclarerObject) /* 7 */
#define Tcl_MethodIsPublic \
	(tclOOStubsPtr->tcl_MethodIsPublic) /* 8 */
#define Tcl_MethodIsType \
	(tclOOStubsPtr->tcl_MethodIsType) /* 9 */
#define Tcl_MethodName \
	(tclOOStubsPtr->tcl_MethodName) /* 10 */
#define Tcl_NewInstanceMethod \
	(tclOOStubsPtr->tcl_NewInstanceMethod) /* 11 */
#define Tcl_NewMethod \
	(tclOOStubsPtr->tcl_NewMethod) /* 12 */
#define Tcl_NewObjectInstance \
	(tclOOStubsPtr->tcl_NewObjectInstance) /* 13 */
#define Tcl_ObjectDeleted \
	(tclOOStubsPtr->tcl_ObjectDeleted) /* 14 */
#define Tcl_ObjectContextIsFiltering \
	(tclOOStubsPtr->tcl_ObjectContextIsFiltering) /* 15 */
#define Tcl_ObjectContextMethod \
	(tclOOStubsPtr->tcl_ObjectContextMethod) /* 16 */
#define Tcl_ObjectContextObject \
	(tclOOStubsPtr->tcl_ObjectContextObject) /* 17 */
#define Tcl_ObjectContextSkippedArgs \
	(tclOOStubsPtr->tcl_ObjectContextSkippedArgs) /* 18 */
#define Tcl_ClassGetMetadata \
	(tclOOStubsPtr->tcl_ClassGetMetadata) /* 19 */
#define Tcl_ClassSetMetadata \
	(tclOOStubsPtr->tcl_ClassSetMetadata) /* 20 */
#define Tcl_ObjectGetMetadata \
	(tclOOStubsPtr->tcl_ObjectGetMetadata) /* 21 */
#define Tcl_ObjectSetMetadata \
	(tclOOStubsPtr->tcl_ObjectSetMetadata) /* 22 */
#define Tcl_ObjectContextInvokeNext \
	(tclOOStubsPtr->tcl_ObjectContextInvokeNext) /* 23 */
#define Tcl_ObjectGetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectGetMethodNameMapper) /* 24 */
#define Tcl_ObjectSetMethodNameMapper \
	(tclOOStubsPtr->tcl_ObjectSetMethodNameMapper) /* 25 */
#define Tcl_ClassSetConstructor \
	(tclOOStubsPtr->tcl_ClassSetConstructor) /* 26 */
#define Tcl_ClassSetDestructor \
	(tclOOStubsPtr->tcl_ClassSetDestructor) /* 27 */
#define Tcl_GetObjectName \
	(tclOOStubsPtr->tcl_GetObjectName) /* 28 */
#define Tcl_MethodIsPrivate \
	(tclOOStubsPtr->tcl_MethodIsPrivate) /* 29 */
#define Tcl_GetClassOfObject \
	(tclOOStubsPtr->tcl_GetClassOfObject) /* 30 */
#define Tcl_GetObjectClassName \
	(tclOOStubsPtr->tcl_GetObjectClassName) /* 31 */
#define Tcl_MethodIsType2 \
	(tclOOStubsPtr->tcl_MethodIsType2) /* 32 */
#define Tcl_NewInstanceMethod2 \
	(tclOOStubsPtr->tcl_NewInstanceMethod2) /* 33 */
#define Tcl_NewMethod2 \
	(tclOOStubsPtr->tcl_NewMethod2) /* 34 */

#endif /* defined(USE_TCLOO_STUBS) */

/* !END!: Do not edit above this line. */

#ifdef TCL_NO_DEPRECATED
#   undef Tcl_MethodType
#   undef Tcl_MethodIsType
#   undef Tcl_NewInstanceMethod
#   undef Tcl_NewMethod
#endif

#if defined(__STDC_VERSION__) && (__STDC_VERSION__ >= 201112L)
#ifndef TclOOGeneric
/* Select method based on type of argument. */
#define TclOOGeneric(typePtr, impl) \
    _Generic(typePtr, default: impl, const Tcl_MethodType2 *: impl ## 2)
#endif

#ifdef USE_TCLOO_STUBS

#undef Tcl_MethodIsType
#define Tcl_MethodIsType(method, typePtr, clientDataPtr) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_MethodIsType) \
	((method), (typePtr), (clientDataPtr)))
#undef Tcl_NewInstanceMethod
#define Tcl_NewInstanceMethod(interp, object, nameObj, flags, typePtr, clientData) \
    (TclOOGeneric((typePtr), tclOOStubsPtr->tcl_NewInstanceMethod) \
	((interp), (object), (nameObj), (flags), (typePtr), (clientData)))

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 85–182. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
static inline int	InitDefineContext(Tcl_Interp *interp,
			    Tcl_Namespace *namespacePtr, Object *oPtr,
			    Tcl_Size objc, Tcl_Obj *const *objv);
static inline void	RecomputeClassCacheFlag(Object *oPtr);
static int		RenameDeleteMethod(Tcl_Interp *interp, Object *oPtr,
			    bool useClass, Tcl_Obj *const fromPtr,
			    Tcl_Obj *const toPtr);
static Tcl_MethodCallProc2 Slot_Append;
static Tcl_MethodCallProc2 Slot_AppendNew;
static Tcl_MethodCallProc2 Slot_Clear;
static Tcl_MethodCallProc2 Slot_Prepend;
static Tcl_MethodCallProc2 Slot_Remove;
static Tcl_MethodCallProc2 Slot_Resolve;
static Tcl_MethodCallProc2 Slot_ResolveClass;
static Tcl_MethodCallProc2 Slot_Set;
static Tcl_MethodCallProc2 Slot_Unimplemented;
static Tcl_MethodCallProc2 Slot_Unknown;
static Tcl_MethodCallProc2 ClassFilter_Get, ClassFilter_Set;
static Tcl_MethodCallProc2 ClassMixin_Get, ClassMixin_Set;
static Tcl_MethodCallProc2 ClassSuper_Get, ClassSuper_Set;
static Tcl_MethodCallProc2 ClassVars_Get, ClassVars_Set;
static Tcl_MethodCallProc2 ObjFilter_Get, ObjFilter_Set;
static Tcl_MethodCallProc2 ObjMixin_Get, ObjMixin_Set;
static Tcl_MethodCallProc2 ObjVars_Get, ObjVars_Set;
static Tcl_MethodCallProc2 Configurable_ClassReadableProps_Get;
static Tcl_MethodCallProc2 Configurable_ClassReadableProps_Set;
static Tcl_MethodCallProc2 Configurable_ClassWritableProps_Get;
static Tcl_MethodCallProc2 Configurable_ClassWritableProps_Set;
static Tcl_MethodCallProc2 Configurable_ObjectReadableProps_Get;
static Tcl_MethodCallProc2 Configurable_ObjectReadableProps_Set;
static Tcl_MethodCallProc2 Configurable_ObjectWritableProps_Get;
static Tcl_MethodCallProc2 Configurable_ObjectWritableProps_Set;

/*
 * Now define the slots used in declarations.
 */

static const DeclaredSlot slots[] = {
    SLOT("define::filter",	ClassFilter_Get, ClassFilter_Set, NULL, NULL),
    SLOT("define::mixin",	ClassMixin_Get,  ClassMixin_Set, Slot_ResolveClass, "-set"),
    SLOT("define::superclass",	ClassSuper_Get,  ClassSuper_Set, Slot_ResolveClass, "-set"),
    SLOT("define::variable",	ClassVars_Get,   ClassVars_Set, NULL, NULL),
    SLOT("objdefine::filter",	ObjFilter_Get,   ObjFilter_Set, NULL, NULL),
    SLOT("objdefine::mixin",	ObjMixin_Get,    ObjMixin_Set, Slot_ResolveClass, "-set"),
    SLOT("objdefine::variable",	ObjVars_Get,     ObjVars_Set, NULL, NULL),
    SLOT("configuresupport::readableproperties",
	    Configurable_ClassReadableProps_Get,
	    Configurable_ClassReadableProps_Set, NULL, NULL),
    SLOT("configuresupport::writableproperties",
	    Configurable_ClassWritableProps_Get,
	    Configurable_ClassWritableProps_Set, NULL, NULL),
    SLOT("configuresupport::objreadableproperties",
	    Configurable_ObjectReadableProps_Get,
	    Configurable_ObjectReadableProps_Set, NULL, NULL),
    SLOT("configuresupport::objwritableproperties",
	    Configurable_ObjectWritableProps_Get,
	    Configurable_ObjectWritableProps_Set, NULL, NULL),
    {NULL, {0, 0, 0, 0, 0}, {0, 0, 0, 0, 0}, {0, 0, 0, 0, 0}, 0}
};

static const DeclaredSlotMethod slotMethods[] = {
    SLOT_METHOD("Get",		Slot_Unimplemented, 0),
    SLOT_METHOD("Resolve",	Slot_Resolve,	0),
    SLOT_METHOD("Set",		Slot_Unimplemented, 0),
    SLOT_METHOD("-append",	Slot_Append,	PUBLIC_METHOD),
    SLOT_METHOD("-appendifnew",	Slot_AppendNew,	PUBLIC_METHOD),
    SLOT_METHOD("-clear",	Slot_Clear,	PUBLIC_METHOD),
    SLOT_METHOD("-prepend",	Slot_Prepend,	PUBLIC_METHOD),
    SLOT_METHOD("-remove",	Slot_Remove,	PUBLIC_METHOD),
    SLOT_METHOD("-set",		Slot_Set,	PUBLIC_METHOD),
    SLOT_METHOD("unknown",	Slot_Unknown,	0),
    {NULL, 0, {0, 0, 0, 0, 0}}
};

/*
 * How to build the in-namespace name of a private variable. This is a pattern
 * used with Tcl_ObjPrintf().
 */

#define PRIVATE_VARIABLE_PATTERN "%" TCL_Z_MODIFIER "d : %s"

/*
 * ----------------------------------------------------------------------
 *
 * IsPrivateDefine --
 *
 *	Extracts whether the current context is handling private definitions.
 *
 * ----------------------------------------------------------------------
 */

static inline bool
IsPrivateDefine(
    Tcl_Interp *interp)
{
    Interp *iPtr = (Interp *) interp;

    if (!iPtr->varFramePtr) {

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 714–811. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text

/*
 * ----------------------------------------------------------------------
 *
 * RenameDeleteMethod --
 *
 *	Core of the code to rename and delete methods.
 *
 * ----------------------------------------------------------------------
 */

static int
RenameDeleteMethod(
    Tcl_Interp *interp,
    Object *oPtr,
    bool useClass,
    Tcl_Obj *const fromPtr,
    Tcl_Obj *const toPtr)
{
    Tcl_HashEntry *hPtr, *newHPtr = NULL;
    Method *mPtr;
    int isNew;

    if (!useClass) {
	if (!oPtr->methodsPtr) {
	noSuchMethod:
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "method %s does not exist", TclGetString(fromPtr)));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "METHOD",
		    TclGetString(fromPtr), (char *)NULL);
	    return TCL_ERROR;
	}
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(oPtr->methodsPtr, toPtr,
		    &isNew);
	    if (hPtr == newHPtr) {
	    renameToSelf:
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"cannot rename method to itself", TCL_AUTO_LENGTH));
		OO_ERROR(interp, RENAME_TO_SELF);
		return TCL_ERROR;
	    } else if (!isNew) {
	    renameToExisting:
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"method called %s already exists",
			TclGetString(toPtr)));
		OO_ERROR(interp, RENAME_OVER);
		return TCL_ERROR;
	    }
	}
    } else {
	hPtr = Tcl_FindHashEntry(&oPtr->classPtr->classMethods, fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(&oPtr->classPtr->classMethods,
		    (char *) toPtr, &isNew);
	    if (hPtr == newHPtr) {
		goto renameToSelf;
	    } else if (!isNew) {
		goto renameToExisting;
	    }
	}
    }

    /*
     * Complete the splicing by changing the method's name.
     */

    mPtr = (Method *) Tcl_GetHashValue(hPtr);
    if (toPtr) {
	Tcl_IncrRefCount(toPtr);
	Tcl_DecrRefCount(mPtr->namePtr);
	mPtr->namePtr = toPtr;
	Tcl_SetHashValue(newHPtr, mPtr);
    } else {
	if (!useClass) {
	    RecomputeClassCacheFlag(oPtr);
	}
	TclOODelMethodRef(mPtr);
    }
    Tcl_DeleteHashEntry(hPtr);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOUnknownDefinition --
 *
 *	Handles what happens when an unknown command is encountered during the
 *	processing of a definition script. Works by finding a command in the
 *	operating definition namespace that the requested command is a unique

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 722–819. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
 * ----------------------------------------------------------------------
 */

static int
RenameDeleteMethod(
    Tcl_Interp *interp,
    Object *oPtr,
    bool useClass,
    Tcl_Obj *const fromPtr,
    Tcl_Obj *const toPtr)
{
    Tcl_HashEntry *hPtr, *newHPtr = NULL;
    Method *mPtr;
    int isNew;

    if (!useClass) {
	if (!oPtr->methodsPtr) {
	noSuchMethod:
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "method %s does not exist", TclGetString(fromPtr)));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "METHOD",
		    TclGetString(fromPtr), (char *)NULL);
	    return TCL_ERROR;
	}
	hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(oPtr->methodsPtr, toPtr,
		    &isNew);
	    if (hPtr == newHPtr) {
	    renameToSelf:
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"cannot rename method to itself", TCL_AUTO_LENGTH));
		OO_ERROR(interp, RENAME_TO_SELF);
		return TCL_ERROR;
	    } else if (!isNew) {
	    renameToExisting:
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"method called %s already exists",
			TclGetString(toPtr)));
		OO_ERROR(interp, RENAME_OVER);
		return TCL_ERROR;
	    }
	}
    } else {
	hPtr = Tcl_FindHashEntry(&oPtr->classPtr->classMethods, fromPtr);
	if (hPtr == NULL) {
	    goto noSuchMethod;
	}
	if (toPtr) {
	    newHPtr = Tcl_CreateHashEntry(&oPtr->classPtr->classMethods,
		    (char *) toPtr, &isNew);
	    if (hPtr == newHPtr) {
		goto renameToSelf;
	    } else if (!isNew) {
		goto renameToExisting;
	    }
	}
    }

    /*
     * Complete the splicing by changing the method's name.
     */

    mPtr = (Method *) Tcl_GetHashValue(hPtr);
    if (toPtr) {
	Tcl_IncrRefCount(toPtr);
	Tcl_DecrRefCount(mPtr->namePtr);
	mPtr->namePtr = toPtr;
	Tcl_SetHashValue(newHPtr, mPtr);
    } else {
	if (!useClass) {
	    RecomputeClassCacheFlag(oPtr);
	}
	TclOODelMethodRef(mPtr);
    }
    Tcl_DeleteHashEntry(hPtr);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOUnknownDefinition --
 *
 *	Handles what happens when an unknown command is encountered during the
 *	processing of a definition script. Works by finding a command in the
 *	operating definition namespace that the requested command is a unique
 *	prefix of.
 *
 * ----------------------------------------------------------------------
 */

int
TclOOUnknownDefinition(
    TCL_UNUSED(void *),

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1938–2035. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDeleteMethodObjCmd --
 *
 *	Implementation of the "deletemethod" subcommand of the "oo::define"
 *	and "oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDeleteMethodObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstanceDeleteMethod = (clientData != NULL);
    Object *oPtr;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	return ReportAbuse(interp);
    }

    for (Tcl_Size i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1947–2044. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
 * ----------------------------------------------------------------------
 */

int
TclOODefineDeleteMethodObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstanceDeleteMethod = (clientData != NULL);
    Object *oPtr;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	return ReportAbuse(interp);
    }

    for (Tcl_Size i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1953–2050. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstanceDeleteMethod = (clientData != NULL);
    Object *oPtr;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	return ReportAbuse(interp);
    }

    for (Tcl_Size i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a
     * destructor during a destructor is fairly dumb anyway.
     */

    Tcl_ClassSetDestructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1965–2061. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceDeleteMethod && !oPtr->classPtr) {
	return ReportAbuse(interp);
    }

    for (Tcl_Size i = 1; i < objc; i++) {
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a
     * destructor during a destructor is fairly dumb anyway.
     */

    Tcl_ClassSetDestructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineExportObjCmd --
 *
 *	Implementation of the "export" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1974–2070. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
	/*
	 * Delete the method structure from the appropriate hash table.
	 */

	if (RenameDeleteMethod(interp, oPtr, !isInstanceDeleteMethod,
		objv[i], NULL) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a
     * destructor during a destructor is fairly dumb anyway.
     */

    Tcl_ClassSetDestructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineExportObjCmd --
 *
 *	Implementation of the "export" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineExportObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstanceExport = (clientData != NULL), changed = false;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 1980–2076. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
	    return TCL_ERROR;
	}
    }

    if (isInstanceDeleteMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineDestructorObjCmd --
 *
 *	Implementation of the "destructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineDestructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Method method;
    Tcl_Size bodyLength;
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    (void) TclGetStringFromObj(objv[1], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, NULL, objv[1], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the destructor method record and set the field in the class
	 * record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the destructor as this might be being done during
     * execution of the destructor itself. Also note that setting a
     * destructor during a destructor is fairly dumb anyway.
     */

    Tcl_ClassSetDestructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineExportObjCmd --
 *
 *	Implementation of the "export" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineExportObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstanceExport = (clientData != NULL), changed = false;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `DeleteMethod`, lines 2422–2518. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
     * get the entries from the appropriate hash tables (this can generate a
     * range of errors...)
     */

    if (RenameDeleteMethod(interp, oPtr, !isInstanceRenameMethod,
	    objv[1], objv[2]) != TCL_OK) {
	return TCL_ERROR;
    }

    if (isInstanceRenameMethod) {
	BumpInstanceEpoch(oPtr);
    } else {
	BumpGlobalEpoch(interp, oPtr->classPtr);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineUnexportObjCmd --
 *
 *	Implementation of the "unexport" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineUnexportObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstanceUnexport = (clientData != NULL), changed = false;
    Object *oPtr;
    Class *clsPtr;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;
    if (!isInstanceUnexport && !clsPtr) {
	return ReportAbuse(interp);
    }

    for (Tcl_Size i = 1; i < objc; i++) {
	if (isInstanceUnexport) {
	    changed |= UnexportInstanceMethod(oPtr, objv[i]);
	} else {
	    changed |= UnexportMethod(clsPtr, objv[i]);
	}
    }

    /*
     * Bump the right epoch if we actually changed anything.
     */

    if (changed) {
	if (isInstanceUnexport) {
	    BumpInstanceEpoch(oPtr);
	} else {
	    BumpGlobalEpoch(interp, clsPtr);
	}
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Tcl_ClassSetConstructor, Tcl_ClassSetDestructor --
 *
 *	How to install a constructor or destructor into a class; API to call
 *	from C.
 *
 * ----------------------------------------------------------------------
 */

void
Tcl_ClassSetConstructor(
    Tcl_Interp *interp,
    Tcl_Class clazz,
    Tcl_Method method)
{
    Class *clsPtr = (Class *) clazz;

    if (method != (Tcl_Method) clsPtr->constructorPtr) {
	TclOODelMethodRef(clsPtr->constructorPtr);
	clsPtr->constructorPtr = (Method *) method;

```

## Consumer bindings

- [rust/tcl-vm/src/cmd_oo/native_method_cache/owner_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_cache/owner_tests.rs), `cmd_oo::native_method_cache::owner_tests::original_vm_idle_selector_cache_does_not_retain_deleted_method_records` (linked): Finite source-defined control: original vm idle selector cache does not retain deleted method records. Native506b idle case0 is a separate physical MethodType observation. These assertions cover the VM's exact Rc record and selector cache roles. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-vm/src/cmd_oo/native_method_cache/owner_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_cache/owner_tests.rs), `cmd_oo::native_method_cache::owner_tests::original_vm_entered_chain_keeps_all_records_and_reads_current_same_record_payload` (linked): Finite source-defined control: original vm entered chain keeps all records and reads current same record payload. Real VM command callbacks and method bodies; no custom C MethodType grant. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-vm/src/cmd_oo/native_method_cache/owner_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_cache/owner_tests.rs), `cmd_oo::native_method_cache::owner_tests::original_vm_cache_world_does_not_adopt_a_fresh_stamp_after_actual_mutation` (linked): Finite source-defined control: original vm cache world does not adopt a fresh stamp after actual mutation. Guest mutation is not context ABA. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-vm/src/cmd_oo/native_method_cache/owner_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_cache/owner_tests.rs), `cmd_oo::native_method_cache::owner_tests::original_vm_method_callback_context_restore_keeps_first_host_and_reached_cache` (linked): Finite source-defined control: original vm method callback context restore keeps first host and reached cache. Context refusal at a genuine VM NativeCommand callback, not a simulated C free hook. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-vm/src/cmd_oo/native_method_cache/owner_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_cache/owner_tests.rs), `cmd_oo::native_method_cache::owner_tests::original_vm_method_key_refuses_inherited_context_before_string_and_cache` (linked): Finite source-defined control: original vm method key refuses inherited context before string and cache. A real installed context change/restore precedes the original key getter. This is VM ingress ordering, not a C3 extension String-updater observation. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_oo/native_method_cache.rs](../../../../runtime/rust/src/cmd_oo/native_method_cache.rs), `cmd_oo::native_method_cache::original_ownership_tests::original_idle_method_cache_observes_retirement_without_retaining_payload` (linked): Finite source-defined control: original idle method cache observes retirement without retaining payload. Runtime method-record ownership; procedure clientData has its own roles. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_oo/native_method_cache.rs](../../../../runtime/rust/src/cmd_oo/native_method_cache.rs), `cmd_oo::native_method_cache::original_ownership_tests::original_entered_chain_retains_every_inherited_record_until_last_next_owner` (linked): Finite source-defined control: original entered chain retains every inherited record until last next owner. This tests the validated entered-record role, not an entered proc body. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_oo/native_method_cache.rs](../../../../runtime/rust/src/cmd_oo/native_method_cache.rs), `cmd_oo::native_method_cache::original_ownership_tests::original_same_record_replacement_retires_unentered_old_payload_immediately` (linked): Finite source-defined control: original same record replacement retires unentered old payload immediately. Strong record ownership is distinct from a proc's real entered-body role. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_oo/native_method_cache.rs](../../../../runtime/rust/src/cmd_oo/native_method_cache.rs), `cmd_oo::native_method_cache::original_ownership_tests::original_missing_cached_record_cannot_adopt_a_recreated_name` (linked): Finite source-defined control: original missing cached record cannot adopt a recreated name. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-vm/src/cmd_oo/native_method_cache/string_stage_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_cache/string_stage_tests.rs), `cmd_oo::native_method_cache::string_stage_tests::original_method_cache_string_uses_captured_formatter_before_all_record_upgrade` (linked): Original method cache string uses captured formatter before all record upgrade. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.
- [rust/tcl-vm/src/cmd_oo/original_method_retirement_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/original_method_retirement_tests.rs), `cmd_oo::original_method_retirement_tests::original_method_table_replacement_preserves_record_and_retires_original_payload` (linked): Original method table replacement preserves record and retires original payload. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.
- [rust/tcl-vm/src/cmd_oo/original_method_retirement_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/original_method_retirement_tests.rs), `cmd_oo::original_method_retirement_tests::original_unnamed_slot_replacement_keeps_only_genuine_entered_original_owner` (linked): Original unnamed slot replacement keeps only genuine entered original owner. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.

## Evidence

- [runtime/rust/tests/data/native_method_cache_active_call_lifetime_original/506](../../../../runtime/rust/tests/data/native_method_cache_active_call_lifetime_original/506)
- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request)
- [runtime/rust/tests/data/native_method_cache_active_call_lifetime_original/506b](../../../../runtime/rust/tests/data/native_method_cache_active_call_lifetime_original/506b)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-test-support/tests/data/native_resolution_reference_builds](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds)
- [runtime/rust/tests/data/native_method_cache_active_call_lifetime_original](../../../../runtime/rust/tests/data/native_method_cache_active_call_lifetime_original)
- [rust/tcl-vm/src/cmd_oo/native_method_cache](../../../../rust/tcl-vm/src/cmd_oo/native_method_cache)
- [rust/tcl-vm/src/cmd_oo](../../../../rust/tcl-vm/src/cmd_oo)
