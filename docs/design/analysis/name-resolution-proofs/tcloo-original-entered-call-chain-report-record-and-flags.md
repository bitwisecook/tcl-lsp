# naming.tcloo.original-entered-call-chain-report-record-and-flags

Kind: `implementation-contract`

## Problem statement

An entered call chain owns exact records whose current payload, name and flags may change while selector spellings remain the same. Classifying an ordinary method by constructor-like text or rebuilding a report from a mutable command table can substitute an unrelated role or record after deletion, rename or replacement.

## Question

Does an original entered call-chain report retain its exact Method/report owner, inspect CURRENT payload/type/private flags and genuine producer filter/unknown/facet/Named-Constructor-Destructor roles, and retain actual record/Foundation name List members until separately checked presentation?

## Conclusion

The current source/API continuation reports exact retained records and CURRENT payload/flags, using producer-issued roles and actual record/Foundation names. Five finite software definitions distinguish literal labels, filter/unknown/facet flags, table deletion/rename/payload replacement and first-cause presentation. Exact34 C9.0/C9.1 source joins remain static inspection only. All seven external execution statuses are not tested; no matching owned assertion result, Native Class/Method pointer lifetime or payload callback parity is attached.

## Scope

Actual retained MethodOwner and current payload/type/private flags; original entered caller filter/unknown/declaring-facet/Named-Constructor-Destructor roles; original record/Foundation name List members; checked original String presentation and first typed cause. Implicit stock dispatch report carriers provide no synthetic Method/cache authority. Foundation bootstrap-owned name headers, weak idle cache versus ALL entered-record lifetime, mutable payload retirement, arbitrary Class-facet pointer lifetime and VM semantics retain independent purposes. C9 source ordering is static inspection, separate from software Host/ABA controls and original executions.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl8.4 not tested for this source/API question.

No original tcl8.4 process tests this entered call-chain original report record, current payload and producer flags contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### tcl8.5

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl8.5 not tested for this source/API question.

No original tcl8.5 process tests this entered call-chain original report record, current payload and producer flags contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### tcl8.6

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl8.6 not tested for this source/API question.

No original tcl8.6 process tests this entered call-chain original report record, current payload and producer flags contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### tcl9.0

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl9.0 9.0.4.

Inspected C9.0.4 call/report sources select actual filter/unknown/private and declaring-facet flags, current mutable Method payload/type and actual record/Foundation name members. The retained34 total source extents inspect caller/installer/renderer roles. No original report worker execution, safe Class/Method pointer lifetime, displaced payload retention or callback parity is observed.

### tcl9.1

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl9.1 9.1.0.

Inspected C9.1.0 call/report sources select actual filter/unknown/private and declaring-facet flags, current mutable Method payload/type and actual record/Foundation name members. The retained34 total source extents inspect caller/installer/renderer roles. No original report worker execution, safe Class/Method pointer lifetime, displaced payload retention or callback parity is observed.

### jim

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: jim not tested for this source/API question.

No original jim process tests this entered call-chain original report record, current payload and producer flags contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### bigip

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: bigip not tested for this source/API question.

No original bigip process tests this entered call-chain original report record, current payload and producer flags contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

## Source inspection

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `Source/API extent: actual isFilter/unknown/private precedence, Foundation versus record name, actual declaring facet and CURRENT mutable payload descriptor; original List member roles`, lines 1850–1921. Retained reference `report-source-extent-0`.

```text
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
    Tcl_BounceRefCount(objectLiteral);
    Tcl_BounceRefCount(privateLiteral);

    /*
     * Finish building the description and return it.
     */

    resultObj = Tcl_NewListObj(callPtr->numChain, objv);
    TclStackFree(interp, objv);
    return resultObj;
}

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `Source/API extent: true private, unexported and public selection are independent native flags`, lines 68–90. Retained reference `report-source-extent-1`.

```text
 * Note that the flag bit PRIVATE_METHOD has a confusing name; it's just for
 * Itcl's special type of private.
 */

#define IS_PUBLIC(mPtr)				\
    (((mPtr)->flags & PUBLIC_METHOD) != 0)
#define IS_UNEXPORTED(mPtr)			\
    (((mPtr)->flags & SCOPE_FLAGS) == 0)
#define IS_ITCLPRIVATE(mPtr)				\
    (((mPtr)->flags & PRIVATE_METHOD) != 0)
#define IS_PRIVATE(mPtr)			\
    (((mPtr)->flags & TRUE_PRIVATE_METHOD) != 0)
#define WANT_PUBLIC(flags)			\
    (((flags) & PUBLIC_METHOD) != 0)
#define WANT_UNEXPORTED(flags)			\
    (((flags) & (PRIVATE_METHOD | TRUE_PRIVATE_METHOD)) == 0)
#define WANT_ITCLPRIVATE(flags)			\
    (((flags) & PRIVATE_METHOD) != 0)
#define WANT_PRIVATE(flags)			\
    (((flags) & TRUE_PRIVATE_METHOD) != 0)

/*
 * Name the bits used in the names table values.

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `Source/API extent: actual MInvoke isFilter producer, independent of selector spelling`, lines 1009–1057. Retained reference `report-source-extent-2`.

```text
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
    }
    callPtr->chain[i].mPtr = mPtr;
    callPtr->chain[i].isFilter = (doneFilters != NULL);
    callPtr->chain[i].filterDeclarer = filterDecl;
    callPtr->numChain++;
}

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `Source/API extent: actual filtered unknown fallback call-chain flag`, lines 1264–1290. Retained reference `report-source-extent-3`.

```text

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

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `Source/API extent: actual unfiltered unknown fallback call-chain flag`, lines 1345–1371. Retained reference `report-source-extent-4`.

```text
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

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOCall.c`, function `Source/API extent: actual class stereotype unknown fallback flag`, lines 1530–1559. Retained reference `report-source-extent-5`.

```text
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

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `Source/API extent: genuine stock method descriptors and installer visibility; class/configurable constructor descriptors remain distinct`, lines 124–181. Retained reference `report-source-extent-6`.

```text
 * Methods in the oo::object and oo::class classes. First, we define a helper
 * macro that makes building the method type declaration structure a lot
 * easier. No point in making life harder than it has to be!
 *
 * Note that the core methods don't need clone or free proc callbacks.
 */

#define DCM(name,visibility,proc) \
    {name,visibility,\
	{TCL_OO_METHOD_VERSION_CURRENT,"core method: "#name,proc,NULL,NULL}}

static const DeclaredClassMethod objMethods[] = {
    DCM("<cloned>", 0,	TclOO_Object_Cloned),
    DCM("destroy", 1,	TclOO_Object_Destroy),
    DCM("eval", 0,	TclOO_Object_Eval),
    DCM("unknown", 0,	TclOO_Object_Unknown),
    DCM("variable", 0,	TclOO_Object_LinkVar),
    DCM("varname", 0,	TclOO_Object_VarName),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
}, clsMethods[] = {
    DCM("<cloned>", 0,	TclOO_Class_Cloned),
    DCM("create", 1,	TclOO_Class_Create),
    DCM("new", 1,	TclOO_Class_New),
    DCM("createWithNamespace", 0, TclOO_Class_CreateNs),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
}, cfgMethods[] = {
    DCM("configure", 1, TclOO_Configurable_Configure),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
}, singletonMethods[] = {
    DCM("new", 1,	TclOO_Singleton_New),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
}, singletonInstanceMethods[] = {
    DCM("<cloned>", 0,	TclOO_SingletonInstance_Cloned),
    DCM("destroy", 1,	TclOO_SingletonInstance_Destroy),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
};

/*
 * And for the oo::class constructor...
 */

static const Tcl_MethodType classConstructor = {
    TCL_OO_METHOD_VERSION_CURRENT,
    "oo::class constructor",
    TclOO_Class_Constructor, NULL, NULL
};

/*
 * And the oo::configurable constructor...
 */

static const Tcl_MethodType configurableConstructor = {
    TCL_OO_METHOD_VERSION_CURRENT,
    "oo::configurable constructor",
    TclOO_Configurable_Constructor, NULL, NULL
};

/*

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `Source/API extent: genuine Foundation unknown/constructor/destructor header allocation and independent +1 ownership`, lines 425–441. Retained reference `report-source-extent-7`.

```text
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
    Tcl_IncrRefCount(fPtr->unknownMethodNameObj);
    Tcl_IncrRefCount(fPtr->constructorName);
    Tcl_IncrRefCount(fPtr->destructorName);
    Tcl_IncrRefCount(fPtr->clonedName);
    Tcl_IncrRefCount(fPtr->defineName);
    Tcl_IncrRefCount(fPtr->myName);
    Tcl_IncrRefCount(fPtr->slotGetName);

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `Source/API extent: actual Foundation report-header retirement`, lines 817–826. Retained reference `report-source-extent-8`.

```text
{
    Foundation *fPtr = GetFoundation(interp);

    TclDecrRefCount(fPtr->unknownMethodNameObj);
    TclDecrRefCount(fPtr->constructorName);
    TclDecrRefCount(fPtr->destructorName);
    TclDecrRefCount(fPtr->clonedName);
    TclDecrRefCount(fPtr->defineName);
    TclDecrRefCount(fPtr->myName);
    TclDecrRefCount(fPtr->slotGetName);

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `Source/API extent: genuine procedure/forward payload descriptors`, lines 82–95. Retained reference `report-source-extent-9`.

```text
 * The types of methods defined by the core OO system.
 */

static const Tcl_MethodType procMethodType = {
    TCL_OO_METHOD_VERSION_CURRENT, "method",
    InvokeProcedureMethod, DeleteProcedureMethod, CloneProcedureMethod
};
static const Tcl_MethodType fwdMethodType = {
    TCL_OO_METHOD_VERSION_CURRENT, "forward",
    InvokeForwardMethod, DeleteForwardMethod, CloneForwardMethod
};

/*
 * Helper macros (derived from things private to tclVar.c)

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `Source/API extent: same exact Method record retains its name but resets flags/current mutable payload on replacement`, lines 157–195. Retained reference `report-source-extent-10`.

```text
    if (nameObj == NULL) {
	mPtr = (Method *) Tcl_Alloc(sizeof(Method));
	mPtr->namePtr = NULL;
	mPtr->refCount = 1;
	goto populate;
    }
    if (!oPtr->methodsPtr) {
	oPtr->methodsPtr = (Tcl_HashTable *) Tcl_Alloc(sizeof(Tcl_HashTable));
	Tcl_InitObjHashTable(oPtr->methodsPtr);
	oPtr->flags &= ~USE_CLASS_CACHE;
    }
    hPtr = Tcl_CreateHashEntry(oPtr->methodsPtr, nameObj, &isNew);
    if (isNew) {
	mPtr = (Method *) Tcl_Alloc(sizeof(Method));
	mPtr->namePtr = nameObj;
	mPtr->refCount = 1;
	Tcl_IncrRefCount(nameObj);
	Tcl_SetHashValue(hPtr, mPtr);
    } else {
	mPtr = (Method *) Tcl_GetHashValue(hPtr);
	if (mPtr->typePtr != NULL && mPtr->typePtr->deleteProc != NULL) {
	    mPtr->typePtr->deleteProc(mPtr->clientData);
	}
    }

  populate:
    mPtr->typePtr = typePtr;
    mPtr->clientData = clientData;
    mPtr->flags = 0;
    mPtr->declaringObjectPtr = oPtr;
    mPtr->declaringClassPtr = NULL;
    if (flags) {
	mPtr->flags |= flags &
		(PUBLIC_METHOD | PRIVATE_METHOD | TRUE_PRIVATE_METHOD);
	if (flags & TRUE_PRIVATE_METHOD) {
	    oPtr->flags |= HAS_PRIVATE_METHODS;
	}
    }
    oPtr->epoch++;

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `Source/API extent: class same-record current payload/flags replacement and real declaring Class join`, lines 280–314. Retained reference `report-source-extent-11`.

```text
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

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `Source/API extent: PropertyGetter/PropertySetter descriptor independently issued by property producer`, lines 62–81. Retained reference `report-source-extent-12`.

```text
 */

static const Tcl_MethodType GetterType = {
    TCL_OO_METHOD_VERSION_1,
    "PropertyGetter",
    Configurable_Getter,
    DetailsDeleter,
    DetailsCloner
};

static const Tcl_MethodType SetterType = {
    TCL_OO_METHOD_VERSION_1,
    "PropertySetter",
    Configurable_Setter,
    DetailsDeleter,
    DetailsCloner
};

/*
 * ----------------------------------------------------------------------

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `Source/API extent: slot accessor and operation descriptors supplied by actual installer data`, lines 28–61. Retained reference `report-source-extent-13`.

```text
 */

#define OBJNAME_LENGTH_IN_ERRORINFO_LIMIT 30

/*
 * Some things that make it easier to declare a slot.
 */
typedef struct DeclaredSlot {
    const char *name;
    const Tcl_MethodType getterType;
    const Tcl_MethodType setterType;
    const Tcl_MethodType resolverType;
    const char *defaultOp;	/* The default op, if not set by the class */
} DeclaredSlot;

#define SLOT(name,getter,setter,resolver,defOp) \
    {"::oo::" name,							\
	    {TCL_OO_METHOD_VERSION_1, "core method: " name " Getter",	\
		    getter, NULL, NULL},				\
	    {TCL_OO_METHOD_VERSION_1, "core method: " name " Setter",	\
		    setter, NULL, NULL},				\
	    {TCL_OO_METHOD_VERSION_1, "core method: " name " Resolver",	\
		    resolver, NULL, NULL}, (defOp)}

typedef struct DeclaredSlotMethod {
    const char *name;
    int flags;
    const Tcl_MethodType implType;
} DeclaredSlotMethod;

#define SLOT_METHOD(name,impl,flags) \
    {name, flags, {TCL_OO_METHOD_VERSION_1,				\
	    "core method: " name " slot", impl, NULL, NULL}}

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `Source/API extent: actual slot descriptor fragments, operation names and visibility`, lines 193–228. Retained reference `report-source-extent-14`.

```text
static const DeclaredSlot slots[] = {
    SLOT("define::filter",      ClassFilter_Get, ClassFilter_Set, NULL, NULL),
    SLOT("define::mixin",       ClassMixin_Get,  ClassMixin_Set, Slot_ResolveClass, "-set"),
    SLOT("define::superclass",  ClassSuper_Get,  ClassSuper_Set, Slot_ResolveClass, "-set"),
    SLOT("define::variable",    ClassVars_Get,   ClassVars_Set, NULL, NULL),
    SLOT("objdefine::filter",   ObjFilter_Get,   ObjFilter_Set, NULL, NULL),
    SLOT("objdefine::mixin",    ObjMixin_Get,    ObjMixin_Set, Slot_ResolveClass, "-set"),
    SLOT("objdefine::variable", ObjVars_Get,     ObjVars_Set, NULL, NULL),
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

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInfo.c`, function `Source/API extent: actual object report requests PUBLIC_METHOD, retains selected context then releases it after result`, lines 1742–1776. Retained reference `report-source-extent-15`.

```text
InfoObjectCallCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Object *oPtr;
    CallContext *contextPtr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName methodName");
	return TCL_ERROR;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Get the call context and render its call chain.
     */

    contextPtr = TclOOGetCallContext(oPtr, objv[2], PUBLIC_METHOD, NULL, NULL,
	    NULL);
    if (contextPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"cannot construct any call chain", TCL_AUTO_LENGTH));
	OO_ERROR(interp, BAD_CALL_CHAIN);
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp,
	    TclOORenderCallChain(interp, contextPtr->callPtr));
    TclOODeleteContext(contextPtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInfo.c`, function `Source/API extent: actual class report requests PUBLIC_METHOD and renders retained stereotype chain`, lines 1789–1821. Retained reference `report-source-extent-16`.

```text
InfoClassCallCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Class *clsPtr;
    CallChain *callPtr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "className methodName");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Get an render the stereotypical call chain.
     */

    callPtr = TclOOGetStereotypeCallChain(clsPtr, objv[2], PUBLIC_METHOD);
    if (callPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"cannot construct any call chain", TCL_AUTO_LENGTH));
	OO_ERROR(interp, BAD_CALL_CHAIN);
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, TclOORenderCallChain(interp, callPtr));
    TclOODeleteChain(callPtr);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `Source/API extent: actual isFilter/unknown/private precedence, Foundation versus record name, actual declaring facet and CURRENT mutable payload descriptor; original List member roles`, lines 1867–1938. Retained reference `report-source-extent-17`.

```text
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
    Tcl_BounceRefCount(objectLiteral);
    Tcl_BounceRefCount(privateLiteral);

    /*
     * Finish building the description and return it.
     */

    resultObj = Tcl_NewListObj(callPtr->numChain, objv);
    TclStackFree(interp, objv);
    return resultObj;
}

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `Source/API extent: true private, unexported and public selection are independent native flags`, lines 68–90. Retained reference `report-source-extent-18`.

```text
 * Note that the flag bit PRIVATE_METHOD has a confusing name; it's just for
 * Itcl's special type of private.
 */

#define IS_PUBLIC(mPtr)				\
    (((mPtr)->flags & PUBLIC_METHOD) != 0)
#define IS_UNEXPORTED(mPtr)			\
    (((mPtr)->flags & SCOPE_FLAGS) == 0)
#define IS_ITCLPRIVATE(mPtr)				\
    (((mPtr)->flags & PRIVATE_METHOD) != 0)
#define IS_PRIVATE(mPtr)			\
    (((mPtr)->flags & TRUE_PRIVATE_METHOD) != 0)
#define WANT_PUBLIC(flags)			\
    (((flags) & PUBLIC_METHOD) != 0)
#define WANT_UNEXPORTED(flags)			\
    (((flags) & (PRIVATE_METHOD | TRUE_PRIVATE_METHOD)) == 0)
#define WANT_ITCLPRIVATE(flags)			\
    (((flags) & PRIVATE_METHOD) != 0)
#define WANT_PRIVATE(flags)			\
    (((flags) & TRUE_PRIVATE_METHOD) != 0)

/*
 * Name the bits used in the names table values.

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `Source/API extent: actual MInvoke isFilter producer, independent of selector spelling`, lines 1027–1075. Retained reference `report-source-extent-19`.

```text
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
    }
    callPtr->chain[i].mPtr = mPtr;
    callPtr->chain[i].isFilter = (doneFilters != NULL);
    callPtr->chain[i].filterDeclarer = filterDecl;
    callPtr->numChain++;
}

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `Source/API extent: actual filtered unknown fallback call-chain flag`, lines 1282–1308. Retained reference `report-source-extent-20`.

```text
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

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `Source/API extent: actual unfiltered unknown fallback call-chain flag`, lines 1360–1386. Retained reference `report-source-extent-21`.

```text
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

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOCall.c`, function `Source/API extent: actual class stereotype unknown fallback flag`, lines 1545–1574. Retained reference `report-source-extent-22`.

```text

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

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `Source/API extent: genuine stock method descriptors and installer visibility; class/configurable constructor descriptors remain distinct`, lines 124–181. Retained reference `report-source-extent-23`.

```text
/*
 * Methods in the oo::object and oo::class classes. First, we define a helper
 * macro that makes building the method type declaration structure a lot
 * easier. No point in making life harder than it has to be!
 *
 * Note that the core methods don't need clone or free proc callbacks.
 */

#define DCM(name,visibility,proc) \
    {name,visibility,\
	{TCL_OO_METHOD_VERSION_2,"core method: "#name,proc,NULL,NULL}}

static const DeclaredClassMethod objMethods[] = {
    DCM("<cloned>", false,	TclOO_Object_Cloned),
    DCM("destroy", true,	TclOO_Object_Destroy),
    DCM("eval", false,		TclOO_Object_Eval),
    DCM("unknown", false,	TclOO_Object_Unknown),
    DCM("variable", false,	TclOO_Object_LinkVar),
    DCM("varname", false,	TclOO_Object_VarName),
    {NULL, false, {0, NULL, NULL, NULL, NULL}}
}, clsMethods[] = {
    DCM("<cloned>", false,	TclOO_Class_Cloned),
    DCM("create", true,		TclOO_Class_Create),
    DCM("new", true,		TclOO_Class_New),
    DCM("createWithNamespace", false, TclOO_Class_CreateNs),
    {NULL, false, {0, NULL, NULL, NULL, NULL}}
}, cfgMethods[] = {
    DCM("configure", true,	TclOO_Configurable_Configure),
    {NULL, false, {0, NULL, NULL, NULL, NULL}}
}, singletonMethods[] = {
    DCM("new", true,		TclOO_Singleton_New),
    {NULL, false, {0, NULL, NULL, NULL, NULL}}
}, singletonInstanceMethods[] = {
    DCM("<cloned>", false,	TclOO_SingletonInstance_Cloned),
    DCM("destroy", true,	TclOO_SingletonInstance_Destroy),
    {NULL, false, {0, NULL, NULL, NULL, NULL}}
};

/*
 * And for the oo::class constructor...
 */

static const Tcl_MethodType2 classConstructor = {
    TCL_OO_METHOD_VERSION_2,
    "oo::class constructor",
    TclOO_Class_Constructor, NULL, NULL
};

/*
 * And the oo::configurable constructor...
 */

static const Tcl_MethodType2 configurableConstructor = {
    TCL_OO_METHOD_VERSION_2,
    "oo::configurable constructor",
    TclOO_Configurable_Constructor, NULL, NULL
};

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `Source/API extent: genuine Foundation unknown/constructor/destructor header allocation and independent +1 ownership`, lines 428–444. Retained reference `report-source-extent-24`.

```text
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
    Tcl_IncrRefCount(fPtr->unknownMethodNameObj);
    Tcl_IncrRefCount(fPtr->constructorName);
    Tcl_IncrRefCount(fPtr->destructorName);
    Tcl_IncrRefCount(fPtr->clonedName);
    Tcl_IncrRefCount(fPtr->defineName);
    Tcl_IncrRefCount(fPtr->myName);
    Tcl_IncrRefCount(fPtr->slotGetName);

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `Source/API extent: actual Foundation report-header retirement`, lines 805–814. Retained reference `report-source-extent-25`.

```text
{
    Foundation *fPtr = GetFoundation(interp);

    TclDecrRefCount(fPtr->unknownMethodNameObj);
    TclDecrRefCount(fPtr->constructorName);
    TclDecrRefCount(fPtr->destructorName);
    TclDecrRefCount(fPtr->clonedName);
    TclDecrRefCount(fPtr->defineName);
    TclDecrRefCount(fPtr->myName);
    TclDecrRefCount(fPtr->slotGetName);

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `Source/API extent: genuine procedure/forward payload descriptors`, lines 82–95. Retained reference `report-source-extent-26`.

```text
/*
 * The types of methods defined by the core OO system.
 */

static const Tcl_MethodType2 procMethodType = {
    TCL_OO_METHOD_VERSION_2, "method",
    InvokeProcedureMethod, DeleteProcedureMethod, CloneProcedureMethod
};
static const Tcl_MethodType2 fwdMethodType = {
    TCL_OO_METHOD_VERSION_2, "forward",
    InvokeForwardMethod, DeleteForwardMethod, CloneForwardMethod
};

/*

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `Source/API extent: same exact Method record retains its name but resets flags/current mutable payload on replacement`, lines 157–195. Retained reference `report-source-extent-27`.

```text

    if (nameObj == NULL) {
	mPtr = (Method *) Tcl_Alloc(sizeof(Method));
	mPtr->namePtr = NULL;
	mPtr->refCount = 1;
	goto populate;
    }
    if (!oPtr->methodsPtr) {
	oPtr->methodsPtr = (Tcl_HashTable *) Tcl_Alloc(sizeof(Tcl_HashTable));
	Tcl_InitObjHashTable(oPtr->methodsPtr);
	oPtr->flags &= ~USE_CLASS_CACHE;
    }
    hPtr = Tcl_CreateHashEntry(oPtr->methodsPtr, nameObj, &isNew);
    if (isNew) {
	mPtr = (Method *) Tcl_Alloc(sizeof(Method));
	mPtr->namePtr = nameObj;
	mPtr->refCount = 1;
	Tcl_IncrRefCount(nameObj);
	Tcl_SetHashValue(hPtr, mPtr);
    } else {
	mPtr = (Method *) Tcl_GetHashValue(hPtr);
	if (mPtr->type2Ptr != NULL && mPtr->type2Ptr->deleteProc != NULL) {
	    mPtr->type2Ptr->deleteProc(mPtr->clientData);
	}
    }

  populate:
    mPtr->type2Ptr = typePtr;
    mPtr->clientData = clientData;
    mPtr->flags = 0;
    mPtr->declaringObjectPtr = oPtr;
    mPtr->declaringClassPtr = NULL;
    if (flags) {
	mPtr->flags |= flags &
		(PUBLIC_METHOD | PRIVATE_METHOD | TRUE_PRIVATE_METHOD);
	if (flags & TRUE_PRIVATE_METHOD) {
	    oPtr->flags |= HAS_PRIVATE_METHODS;
	}
    }

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `Source/API extent: class same-record current payload/flags replacement and real declaring Class join`, lines 280–314. Retained reference `report-source-extent-28`.

```text
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

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `Source/API extent: PropertyGetter/PropertySetter descriptor independently issued by property producer`, lines 62–81. Retained reference `report-source-extent-29`.

```text
/*
 * Method descriptors
 */

static const Tcl_MethodType2 GetterType = {
    TCL_OO_METHOD_VERSION_2,
    "PropertyGetter",
    Configurable_Getter,
    DetailsDeleter,
    DetailsCloner
};

static const Tcl_MethodType2 SetterType = {
    TCL_OO_METHOD_VERSION_2,
    "PropertySetter",
    Configurable_Setter,
    DetailsDeleter,
    DetailsCloner
};

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `Source/API extent: slot accessor and operation descriptors supplied by actual installer data`, lines 28–61. Retained reference `report-source-extent-30`.

```text
 */

#define OBJNAME_LENGTH_IN_ERRORINFO_LIMIT 30

/*
 * Some things that make it easier to declare a slot.
 */
typedef struct DeclaredSlot {
    const char *name;
    const Tcl_MethodType2 getterType;
    const Tcl_MethodType2 setterType;
    const Tcl_MethodType2 resolverType;
    const char *defaultOp;	// The default op, if not set by the class
} DeclaredSlot;

#define SLOT(name,getter,setter,resolver,defOp) \
    {"::oo::" name,							\
	    {TCL_OO_METHOD_VERSION_2, "core method: " name " Getter",	\
		    getter, NULL, NULL},				\
	    {TCL_OO_METHOD_VERSION_2, "core method: " name " Setter",	\
		    setter, NULL, NULL},				\
	    {TCL_OO_METHOD_VERSION_2, "core method: " name " Resolver",	\
		    resolver, NULL, NULL}, (defOp)}

typedef struct DeclaredSlotMethod {
    const char *name;
    int flags;
    const Tcl_MethodType2 implType;
} DeclaredSlotMethod;

#define SLOT_METHOD(name,impl,flags) \
    {name, flags, {TCL_OO_METHOD_VERSION_2,				\
	    "core method: " name " slot", impl, NULL, NULL}}

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `Source/API extent: actual slot descriptor fragments, operation names and visibility`, lines 122–157. Retained reference `report-source-extent-31`.

```text
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

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `Source/API extent: actual object report requests PUBLIC_METHOD, retains selected context then releases it after result`, lines 1734–1768. Retained reference `report-source-extent-32`.

```text
InfoObjectCallCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    CallContext *contextPtr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName methodName");
	return TCL_ERROR;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Get the call context and render its call chain.
     */

    contextPtr = TclOOGetCallContext(oPtr, objv[2], PUBLIC_METHOD, NULL, NULL,
	    NULL);
    if (contextPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"cannot construct any call chain", TCL_AUTO_LENGTH));
	OO_ERROR(interp, BAD_CALL_CHAIN);
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp,
	    TclOORenderCallChain(interp, contextPtr->callPtr));
    TclOODeleteContext(contextPtr);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Complete exact source hash and independently joined physical-LF extent; static inspection only`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `Source/API extent: actual class report requests PUBLIC_METHOD and renders retained stereotype chain`, lines 1781–1813. Retained reference `report-source-extent-33`.

```text
InfoClassCallCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr;
    CallChain *callPtr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "className methodName");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Get an render the stereotypical call chain.
     */

    callPtr = TclOOGetStereotypeCallChain(clsPtr, objv[2], PUBLIC_METHOD);
    if (callPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"cannot construct any call chain", TCL_AUTO_LENGTH));
	OO_ERROR(interp, BAD_CALL_CHAIN);
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, TclOORenderCallChain(interp, callPtr));
    TclOODeleteChain(callPtr);
    return TCL_OK;
}

```

## Consumer bindings

- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `build_call_chain`: Retain exact selected chain steps and original producer flags rather than reselecting roles from displayed names.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `render_call_chain`: Report exact retained record CURRENT payload/type/private and genuine filter/unknown/facet flags, preserving actual name members before checked presentation.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `retain_original_terminal_report`: Keep the selected stock report-only terminus without fabricating a Method/cache owner.
- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `NativeTclooCallReportKind`: Pure actual descriptor/flag report classification; no physical call-chain owner or entered authority issued.
- [runtime/rust/src/cmd_oo/original_call_report_tests.rs](../../../../runtime/rust/src/cmd_oo/original_call_report_tests.rs), `cmd_oo::original_call_report_tests::original_call_report_uses_payload_descriptors_before_literal_names_and_stock_fallbacks` (linked): Selected software reports inspect actual current payload/type descriptors before literal selector labels, preserving ordinary Forward methods named constructor/destructor/empty and stock/property descriptor precedence without synthesizing a Method owner. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_call_report_tests.rs](../../../../runtime/rust/src/cmd_oo/original_call_report_tests.rs), `cmd_oo::original_call_report_tests::original_call_report_keeps_filter_unknown_and_declaring_facet_flags_independent` (linked): Actual producer filter, unknown dispatch, declaring Object/Class facet and Named/Constructor/Destructor roles remain independent of selector spelling, using the original record or actual Foundation name header as selected List members. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_call_report_tests.rs](../../../../runtime/rust/src/cmd_oo/original_call_report_tests.rs), `cmd_oo::original_call_report_tests::original_call_report_keeps_exact_entered_record_through_deletion_rename_and_payload_replacement` (linked): An entered exact software Method record survives table deletion as its original report owner, while active rename and same-record replacement report CURRENT name/payload/private flags. Retaining a record does not retain every displaced mutable payload or permit name-based revival. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_call_report_tests.rs](../../../../runtime/rust/src/cmd_oo/original_call_report_tests.rs), `cmd_oo::original_call_report_tests::original_call_report_retains_original_name_until_checked_presentation_and_first_cause` (linked): Actual original record/Foundation name membership remains lazy until independently checked String presentation; first Host or context ABA during that updater preserves reached effects and prevents later report continuation. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `native_tcloo_method_definition::call_report_tests::original_call_report_flags_and_payload_types_do_not_classify_selector_spellings` (linked): The pure Registry protocol classifies actual payload descriptor and producer filter/unknown/private/facet/role flags, preserving independent List member roles without turning selector spelling into an entered Method/cache authority. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.

## Evidence

- [runtime/rust/tests/data/original_entered_call_chain_report_source/sources](../../../../runtime/rust/tests/data/original_entered_call_chain_report_source/sources)
- [runtime/rust/tests/data/original_entered_call_chain_report_source](../../../../runtime/rust/tests/data/original_entered_call_chain_report_source)
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs)
- [runtime/rust/src/cmd_oo/native_method_cache.rs](../../../../runtime/rust/src/cmd_oo/native_method_cache.rs)
- [runtime/rust/src/cmd_oo/original_call_report_tests.rs](../../../../runtime/rust/src/cmd_oo/original_call_report_tests.rs)
- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs)
