# naming.property.original-slot-storage-reference-order

Kind: `native-observation`

## Problem statement

Property slot replacement reaches a public helper-produced List, all incoming member references, old-storage retirement and checked object-hash getters in separate roles. Copying its public counts into a standalone storage helper can invent ownership or reorder reached effects.

## Question

For the initialized original C9.1 public readable/writable property slots, when are all incoming member references held relative to old storage/cache retirement and ObjHash String getters, and which original members do later info Lists retain?

## Conclusion

Six C9.1 public cases retain every incoming p,p,q header before retiring unheld old members, then reach key String getters in original order and deduplicate to the first p. Held old cache Lists delay old-member free until their release. ResolveAll helper-produced List membership, SetPropertyList incoming holds, transient hash-key refs and returned info List/cache roles remain distinct. Other five initialized providers supply selected core availability only. These public observations license neither an arbitrary external List/storage-helper equivalent nor Rust callback, property identity or Native callchain authority.

## Scope

Only509b is launched;509 is an unchanged fixed preparation retained by the request, not an executed attempt. Six original own dependency/compile/CLI-version/execute lanes close with unchanged pins. C91 has six cases,162 raw ROWs,48 EVENTs and12 LISTs; other five lanes observe availability. Raw callback/getter rows, public slot/helper return and later explicit RESULT String rendering retain separate phases. Pointer/member counts identify supplied original live headers in their original roles, not independent List issuance or application property/callchain generation. SDK source/library inputs are retained; configure/build processes lack replay receipts. Current software chronology and assertion outcomes remain independent.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original lazy property-name headers through public slot -set and ResolveAll helper-produced original-member List; raw callbacks/member refs, info List and later result presentation. Version: tcl8.4 8.4.20.

Initialized original 8.4.20 core reports selected property worker unavailable in this request. Only its actual availability/version lane is measured; no property chronology or optional-extension absence is inferred.

### tcl8.5

Status: `observed`. Channel: Original lazy property-name headers through public slot -set and ResolveAll helper-produced original-member List; raw callbacks/member refs, info List and later result presentation. Version: tcl8.5 8.5.19.

Initialized original 8.5.19 core reports selected property worker unavailable in this request. Only its actual availability/version lane is measured; no property chronology or optional-extension absence is inferred.

### tcl8.6

Status: `observed`. Channel: Original lazy property-name headers through public slot -set and ResolveAll helper-produced original-member List; raw callbacks/member refs, info List and later result presentation. Version: tcl8.6 8.6.18.

Initialized original 8.6.18 core reports selected property worker unavailable in this request. Only its actual availability/version lane is measured; no property chronology or optional-extension absence is inferred.

### tcl9.0

Status: `observed`. Channel: Original lazy property-name headers through public slot -set and ResolveAll helper-produced original-member List; raw callbacks/member refs, info List and later result presentation. Version: tcl9.0 9.0.4.

Initialized original 9.0.4 core reports selected property worker unavailable in this request. Only its actual availability/version lane is measured; no property chronology or optional-extension absence is inferred.

### tcl9.1

Status: `observed`. Channel: Original lazy property-name headers through public slot -set and ResolveAll helper-produced original-member List; raw callbacks/member refs, info List and later result presentation. Version: tcl9.1 9.1.0.

Six original public property-slot cases show ALL incoming p,p,q members retained with refs3/String absent when unheld old storage is retired, before the three checked key String getters. A held old cache delays old-member free until cache release. Getter order is first p, independent duplicate p, q; dedup keeps the first p. Returned info -all Lists hold the original first-p and q members. The public ResolveAll helper produces the original-member input List and holds members separately from SetPropertyList incoming refs and temporary ObjHash key refs. These public roles cannot be transferred to an arbitrary external List or isolated storage helper. Explicit result rendering is a later observation; no Rust or Native invocation identity follows.

### jim

Status: `observed`. Channel: Original lazy property-name headers through public slot -set and ResolveAll helper-produced original-member List; raw callbacks/member refs, info List and later result presentation. Version: jim 0.84.

Initialized original jim core reports selected property worker unavailable in this request. Only its actual availability/version lane is measured; no property chronology or optional-extension absence is inferred.

### bigip

Status: `not-tested`.

No original BIG-IP process for this finite public worker question.

## Source inspection

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `SetPropertyList(`, lines 924–1029. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
 *
 * ----------------------------------------------------------------------
 */
static inline void
SetPropertyList(
    PropertyList *propList,	/* The property list to write. Replaces the
				 * property list's contents. */
    Tcl_Size objc,		/* Number of property names. */
    Tcl_Obj *const *objv)	/* Property names. */
{
    Tcl_Size i, n;
    Tcl_Obj *propObj;
    int isNew;
    Tcl_HashTable uniqueTable;

    for (i=0 ; i<objc ; i++) {
	Tcl_IncrRefCount(objv[i]);
    }
    FOREACH(propObj, *propList) {
	Tcl_DecrRefCount(propObj);
    }
    if (i != objc) {
	if (objc == 0) {
	    Tcl_Free(propList->list);
	} else if (i) {
	    propList->list = (Tcl_Obj **)
		    Tcl_Realloc(propList->list, sizeof(Tcl_Obj *) * ((size_t)objc));
	} else {
	    propList->list = (Tcl_Obj **)
		    Tcl_Alloc(sizeof(Tcl_Obj *) * ((size_t)objc));
	}
    }
    propList->num = 0;
    if (objc > 0) {
	Tcl_InitObjHashTable(&uniqueTable);
	for (i=n=0 ; i<objc ; i++) {
	    Tcl_CreateHashEntry(&uniqueTable, objv[i], &isNew);
	    if (isNew) {
		propList->list[n++] = objv[i];
	    } else {
		Tcl_DecrRefCount(objv[i]);
	    }
	}
	propList->num = n;

	/*
	 * Shouldn't be necessary, but maintain num/list invariant.
	 */

	if (n != objc) {
	    propList->list = (Tcl_Obj **)
		    Tcl_Realloc(propList->list, sizeof(Tcl_Obj *) * ((size_t)n));
	}
	Tcl_DeleteHashTable(&uniqueTable);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInstallReadableProperties --
 *
 *	Helper for writing the readable property list (which is actually a set)
 *	that includes flushing the name cache.
 *
 * ----------------------------------------------------------------------
 */
void
TclOOInstallReadableProperties(
    PropertyStorage *props,	/* Which property list to install into. */
    Tcl_Size objc,		/* Number of property names. */
    Tcl_Obj *const *objv)	/* Property names. */
{
    if (props->allReadableCache) {
	Tcl_DecrRefCount(props->allReadableCache);
	props->allReadableCache = NULL;
    }

    SetPropertyList(&props->readable, objc, objv);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInstallWritableProperties --
 *
 *	Helper for writing the writable property list (which is actually a set)
 *	that includes flushing the name cache.
 *
 * ----------------------------------------------------------------------
 */
void
TclOOInstallWritableProperties(
    PropertyStorage *props,	/* Which property list to install into. */
    Tcl_Size objc,		/* Number of property names. */
    Tcl_Obj *const *objv)	/* Property names. */
{
    if (props->allWritableCache) {
	Tcl_DecrRefCount(props->allWritableCache);
	props->allWritableCache = NULL;
    }

    SetPropertyList(&props->writable, objc, objv);
}

/*

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `SetPropertyList(`, lines 998–1103. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
	Tcl_DecrRefCount(props->allReadableCache);
	props->allReadableCache = NULL;
    }

    SetPropertyList(&props->readable, objc, objv);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInstallWritableProperties --
 *
 *	Helper for writing the writable property list (which is actually a set)
 *	that includes flushing the name cache.
 *
 * ----------------------------------------------------------------------
 */
void
TclOOInstallWritableProperties(
    PropertyStorage *props,	/* Which property list to install into. */
    Tcl_Size objc,		/* Number of property names. */
    Tcl_Obj *const *objv)	/* Property names. */
{
    if (props->allWritableCache) {
	Tcl_DecrRefCount(props->allWritableCache);
	props->allWritableCache = NULL;
    }

    SetPropertyList(&props->writable, objc, objv);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetPropertyList --
 *
 *	Helper for reading a property list.
 *
 * ----------------------------------------------------------------------
 */
Tcl_Obj *
TclOOGetPropertyList(
    PropertyList *propList)	/* The property list to read. */
{
    Tcl_Obj *resultObj, *propNameObj;
    Tcl_Size i;

    TclNewObj(resultObj);
    FOREACH(propNameObj, *propList) {
	Tcl_ListObjAppendElement(NULL, resultObj, propNameObj);
    }
    return resultObj;
}

/*
 * ----------------------------------------------------------------------
 *
 * InstallStdPropertyImpls --
 *
 *	Validates a (dashless) property name, and installs implementation
 *	methods if asked to do so (readable and writable flags).
 *
 * ----------------------------------------------------------------------
 */
static int
InstallStdPropertyImpls(
    void *useInstance,
    Tcl_Interp *interp,
    Tcl_Obj *propName,
    bool readable,
    bool writable)
{
    const char *name, *reason;
    Tcl_Size len;
    char flag = TCL_DONT_QUOTE_HASH;

    /*
     * Validate the property name. Note that just calling TclScanElement() is
     * cheaper than actually formatting a list and comparing the string
     * version of that with the original, as TclScanElement() is one of the
     * core parts of doing that; this skips a whole load of irrelevant memory
     * allocations!
     */

    name = Tcl_GetStringFromObj(propName, &len);
    if (Tcl_StringMatch(name, "-*")) {
	reason = "must not begin with -";
	goto badProp;
    }
    if (TclScanElement(name, len, &flag) != len) {
	reason = "must be a simple word";
	goto badProp;
    }
    if (Tcl_StringMatch(name, "*::*")) {
	reason = "must not contain namespace separators";
	goto badProp;
    }
    if (Tcl_StringMatch(name, "*[()]*")) {
	reason = "must not contain parentheses";
	goto badProp;
    }

    /*
     * Install the implementations... if asked to do so.
     */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `SetPropertyList(`, lines 1022–1127. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
	Tcl_DecrRefCount(props->allWritableCache);
	props->allWritableCache = NULL;
    }

    SetPropertyList(&props->writable, objc, objv);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetPropertyList --
 *
 *	Helper for reading a property list.
 *
 * ----------------------------------------------------------------------
 */
Tcl_Obj *
TclOOGetPropertyList(
    PropertyList *propList)	/* The property list to read. */
{
    Tcl_Obj *resultObj, *propNameObj;
    Tcl_Size i;

    TclNewObj(resultObj);
    FOREACH(propNameObj, *propList) {
	Tcl_ListObjAppendElement(NULL, resultObj, propNameObj);
    }
    return resultObj;
}

/*
 * ----------------------------------------------------------------------
 *
 * InstallStdPropertyImpls --
 *
 *	Validates a (dashless) property name, and installs implementation
 *	methods if asked to do so (readable and writable flags).
 *
 * ----------------------------------------------------------------------
 */
static int
InstallStdPropertyImpls(
    void *useInstance,
    Tcl_Interp *interp,
    Tcl_Obj *propName,
    bool readable,
    bool writable)
{
    const char *name, *reason;
    Tcl_Size len;
    char flag = TCL_DONT_QUOTE_HASH;

    /*
     * Validate the property name. Note that just calling TclScanElement() is
     * cheaper than actually formatting a list and comparing the string
     * version of that with the original, as TclScanElement() is one of the
     * core parts of doing that; this skips a whole load of irrelevant memory
     * allocations!
     */

    name = Tcl_GetStringFromObj(propName, &len);
    if (Tcl_StringMatch(name, "-*")) {
	reason = "must not begin with -";
	goto badProp;
    }
    if (TclScanElement(name, len, &flag) != len) {
	reason = "must be a simple word";
	goto badProp;
    }
    if (Tcl_StringMatch(name, "*::*")) {
	reason = "must not contain namespace separators";
	goto badProp;
    }
    if (Tcl_StringMatch(name, "*[()]*")) {
	reason = "must not contain parentheses";
	goto badProp;
    }

    /*
     * Install the implementations... if asked to do so.
     */

    if (useInstance) {
	Tcl_Object object = TclOOGetDefineCmdContext(interp);
	if (!object) {
	    return TCL_ERROR;
	}
	ImplementObjectProperty(object, propName, readable, writable);
    } else {
	Tcl_Class cls = (Tcl_Class) TclOOGetClassDefineCmdContext(interp);
	if (!cls) {
	    return TCL_ERROR;
	}
	ImplementClassProperty(cls, propName, readable, writable);
    }
    return TCL_OK;

  badProp:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "bad property name \"%s\": %s", name, reason));
    OO_ERROR(interp, PROPERTY_FORMAT);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `TclOOInstallReadableProperties(`, lines 988–1093. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
 *
 * ----------------------------------------------------------------------
 */
void
TclOOInstallReadableProperties(
    PropertyStorage *props,	/* Which property list to install into. */
    Tcl_Size objc,		/* Number of property names. */
    Tcl_Obj *const *objv)	/* Property names. */
{
    if (props->allReadableCache) {
	Tcl_DecrRefCount(props->allReadableCache);
	props->allReadableCache = NULL;
    }

    SetPropertyList(&props->readable, objc, objv);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOInstallWritableProperties --
 *
 *	Helper for writing the writable property list (which is actually a set)
 *	that includes flushing the name cache.
 *
 * ----------------------------------------------------------------------
 */
void
TclOOInstallWritableProperties(
    PropertyStorage *props,	/* Which property list to install into. */
    Tcl_Size objc,		/* Number of property names. */
    Tcl_Obj *const *objv)	/* Property names. */
{
    if (props->allWritableCache) {
	Tcl_DecrRefCount(props->allWritableCache);
	props->allWritableCache = NULL;
    }

    SetPropertyList(&props->writable, objc, objv);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetPropertyList --
 *
 *	Helper for reading a property list.
 *
 * ----------------------------------------------------------------------
 */
Tcl_Obj *
TclOOGetPropertyList(
    PropertyList *propList)	/* The property list to read. */
{
    Tcl_Obj *resultObj, *propNameObj;
    Tcl_Size i;

    TclNewObj(resultObj);
    FOREACH(propNameObj, *propList) {
	Tcl_ListObjAppendElement(NULL, resultObj, propNameObj);
    }
    return resultObj;
}

/*
 * ----------------------------------------------------------------------
 *
 * InstallStdPropertyImpls --
 *
 *	Validates a (dashless) property name, and installs implementation
 *	methods if asked to do so (readable and writable flags).
 *
 * ----------------------------------------------------------------------
 */
static int
InstallStdPropertyImpls(
    void *useInstance,
    Tcl_Interp *interp,
    Tcl_Obj *propName,
    bool readable,
    bool writable)
{
    const char *name, *reason;
    Tcl_Size len;
    char flag = TCL_DONT_QUOTE_HASH;

    /*
     * Validate the property name. Note that just calling TclScanElement() is
     * cheaper than actually formatting a list and comparing the string
     * version of that with the original, as TclScanElement() is one of the
     * core parts of doing that; this skips a whole load of irrelevant memory
     * allocations!
     */

    name = Tcl_GetStringFromObj(propName, &len);
    if (Tcl_StringMatch(name, "-*")) {
	reason = "must not begin with -";
	goto badProp;
    }
    if (TclScanElement(name, len, &flag) != len) {
	reason = "must be a simple word";
	goto badProp;
    }
    if (Tcl_StringMatch(name, "*::*")) {
	reason = "must not contain namespace separators";
	goto badProp;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `TclOOInstallWritableProperties(`, lines 1012–1118. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
 *
 * ----------------------------------------------------------------------
 */
void
TclOOInstallWritableProperties(
    PropertyStorage *props,	/* Which property list to install into. */
    Tcl_Size objc,		/* Number of property names. */
    Tcl_Obj *const *objv)	/* Property names. */
{
    if (props->allWritableCache) {
	Tcl_DecrRefCount(props->allWritableCache);
	props->allWritableCache = NULL;
    }

    SetPropertyList(&props->writable, objc, objv);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetPropertyList --
 *
 *	Helper for reading a property list.
 *
 * ----------------------------------------------------------------------
 */
Tcl_Obj *
TclOOGetPropertyList(
    PropertyList *propList)	/* The property list to read. */
{
    Tcl_Obj *resultObj, *propNameObj;
    Tcl_Size i;

    TclNewObj(resultObj);
    FOREACH(propNameObj, *propList) {
	Tcl_ListObjAppendElement(NULL, resultObj, propNameObj);
    }
    return resultObj;
}

/*
 * ----------------------------------------------------------------------
 *
 * InstallStdPropertyImpls --
 *
 *	Validates a (dashless) property name, and installs implementation
 *	methods if asked to do so (readable and writable flags).
 *
 * ----------------------------------------------------------------------
 */
static int
InstallStdPropertyImpls(
    void *useInstance,
    Tcl_Interp *interp,
    Tcl_Obj *propName,
    bool readable,
    bool writable)
{
    const char *name, *reason;
    Tcl_Size len;
    char flag = TCL_DONT_QUOTE_HASH;

    /*
     * Validate the property name. Note that just calling TclScanElement() is
     * cheaper than actually formatting a list and comparing the string
     * version of that with the original, as TclScanElement() is one of the
     * core parts of doing that; this skips a whole load of irrelevant memory
     * allocations!
     */

    name = Tcl_GetStringFromObj(propName, &len);
    if (Tcl_StringMatch(name, "-*")) {
	reason = "must not begin with -";
	goto badProp;
    }
    if (TclScanElement(name, len, &flag) != len) {
	reason = "must be a simple word";
	goto badProp;
    }
    if (Tcl_StringMatch(name, "*::*")) {
	reason = "must not contain namespace separators";
	goto badProp;
    }
    if (Tcl_StringMatch(name, "*[()]*")) {
	reason = "must not contain parentheses";
	goto badProp;
    }

    /*
     * Install the implementations... if asked to do so.
     */

    if (useInstance) {
	Tcl_Object object = TclOOGetDefineCmdContext(interp);
	if (!object) {
	    return TCL_ERROR;
	}
	ImplementObjectProperty(object, propName, readable, writable);
    } else {
	Tcl_Class cls = (Tcl_Class) TclOOGetClassDefineCmdContext(interp);
	if (!cls) {
	    return TCL_ERROR;
	}
	ImplementClassProperty(cls, propName, readable, writable);
    }
    return TCL_OK;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetAllClassProperties(`, lines 733–840. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
 *
 * ----------------------------------------------------------------------
 */
static Tcl_Obj *
GetAllClassProperties(
    Class *clsPtr,		/* The class to inspect. Must exist. */
    bool writable,		/* Whether to get writable properties. If
				 * false, readable properties will be returned
				 * instead. */
    bool *allocated)		/* Address of variable to set to true if a
				 * Tcl_Obj was allocated and may be safely
				 * modified by the caller. */
{
    Tcl_HashTable hashTable;
    FOREACH_HASH_DECLS;
    Tcl_Obj *propName, *result;

    /*
     * Look in the cache.
     */

    if (clsPtr->properties.epoch == clsPtr->thisPtr->fPtr->epoch) {
	if (writable) {
	    if (clsPtr->properties.allWritableCache) {
		*allocated = false;
		return clsPtr->properties.allWritableCache;
	    }
	} else {
	    if (clsPtr->properties.allReadableCache) {
		*allocated = false;
		return clsPtr->properties.allReadableCache;
	    }
	}
    }

    /*
     * Gather the information. Unsorted! (Caller will sort.)
     */

    *allocated = true;
    Tcl_InitObjHashTable(&hashTable);
    FindClassProps(clsPtr, writable, &hashTable);
    TclNewObj(result);
    FOREACH_HASH_KEY(propName, &hashTable) {
	Tcl_ListObjAppendElement(NULL, result, propName);
    }
    Tcl_DeleteHashTable(&hashTable);

    /*
     * Cache the information. Also purges the cache.
     */

    if (clsPtr->properties.epoch != clsPtr->thisPtr->fPtr->epoch) {
	if (clsPtr->properties.allWritableCache) {
	    Tcl_DecrRefCount(clsPtr->properties.allWritableCache);
	    clsPtr->properties.allWritableCache = NULL;
	}
	if (clsPtr->properties.allReadableCache) {
	    Tcl_DecrRefCount(clsPtr->properties.allReadableCache);
	    clsPtr->properties.allReadableCache = NULL;
	}
    }
    clsPtr->properties.epoch = clsPtr->thisPtr->fPtr->epoch;
    if (writable) {
	clsPtr->properties.allWritableCache = result;
    } else {
	clsPtr->properties.allReadableCache = result;
    }
    Tcl_IncrRefCount(result);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * SortPropList --
 *
 *	Sort a list of names of properties. Simple support function. Assumes
 *	that the list Tcl_Obj is unshared and doesn't have a string
 *	representation.
 *
 * ----------------------------------------------------------------------
 */

static int
PropNameCompare(
    const void *a,
    const void *b)
{
    Tcl_Obj *first = *(Tcl_Obj **) a;
    Tcl_Obj *second = *(Tcl_Obj **) b;

    return TclStringCmp(first, second, 0, 0, TCL_INDEX_NONE);
}

static inline void
SortPropList(
    Tcl_Obj *list)
{
    Tcl_Size ec;
    Tcl_Obj **ev;

    if (Tcl_IsShared(list)) {
	Tcl_Panic("shared property list cannot be sorted");
    }
    Tcl_ListObjGetElements(NULL, list, &ec, &ev);
    TclInvalidateStringRep(list);
    qsort(ev, (unsigned)ec, sizeof(Tcl_Obj *), PropNameCompare);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetAllClassProperties(`, lines 1344–1451. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
     * Get the properties.
     */

    if (all) {
	result = GetAllClassProperties(clsPtr, writable, &allocated);
	if (allocated) {
	    SortPropList(result);
	}
    } else {
	if (writable) {
	    result = TclOOGetPropertyList(&clsPtr->properties.writable);
	} else {
	    result = TclOOGetPropertyList(&clsPtr->properties.readable);
	}
	SortPropList(result);
    }
    Tcl_SetObjResult(interp, result);
    return TCL_OK;
}

int
TclOOInfoObjectPropCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Tcl_Size i;
    bool all = false, writable = false;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName ?options...?");
	return TCL_ERROR;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    for (i = 2; i < objc; i++) {
	int idx;
	if (Tcl_GetIndexFromObj(interp, objv[i], propOptNames, "option", 0,
		&idx) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (idx) {
	case PROP_ALL:
	    all = true;
	    break;
	case PROP_READABLE:
	    writable = false;
	    break;
	case PROP_WRITABLE:
	    writable = true;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

    /*
     * Get the properties.
     */

    Tcl_Obj *result;
    if (all) {
	result = GetAllObjectProperties(oPtr, writable);
    } else {
	if (writable) {
	    result = TclOOGetPropertyList(&oPtr->properties.writable);
	} else {
	    result = TclOOGetPropertyList(&oPtr->properties.readable);
	}
	SortPropList(result);
    }
    Tcl_SetObjResult(interp, result);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOReleasePropertyStorage --
 *
 *	Delete the memory associated with a class or object's properties.
 *
 * ----------------------------------------------------------------------
 */

static inline void
ReleasePropertyList(
    PropertyList *propList)
{
    Tcl_Obj *propertyObj;
    Tcl_Size i;

    FOREACH(propertyObj, *propList) {
	Tcl_DecrRefCount(propertyObj);
    }
    Tcl_Free(propList->list);
    propList->list = NULL;
    propList->num = 0;
}

void
TclOOReleasePropertyStorage(
    PropertyStorage *propsPtr)
{

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetAllObjectProperties(`, lines 47–154. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
			    Tcl_Size objc, Tcl_Obj *const *objv);
static void		DetailsDeleter(void *clientData);
static int		DetailsCloner(Tcl_Interp *, void *oldClientData,
			    void **newClientData);
static Tcl_Obj		*GetAllObjectProperties(Object *oPtr,
			    bool writable);
static void		ImplementObjectProperty(Tcl_Object targetObject,
			    Tcl_Obj *propNamePtr, bool installGetter,
			    bool installSetter);
static void		ImplementClassProperty(Tcl_Class targetObject,
			    Tcl_Obj *propNamePtr, bool installGetter,
			    bool installSetter);
static void		FreePropName(Tcl_Obj *objPtr);
static void		DupPropName(Tcl_Obj *srcPtr, Tcl_Obj *copyPtr);

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

/*
 * Cache for longer-term handling of property implementation names. This is
 * the interior representation of the cache attached to the property names
 * used by users. It allows the version of the names after they're mapped to
 * TclOO method names to also be cached.
 */
typedef struct PropertyName {
    // If the property is '-foo', this is '<ReadProp-foo>'
    Tcl_Obj *readerName;
    // If the property is '-foo', this is '<WriteProp-foo>'
    Tcl_Obj *writerName;
} PropertyName;

/*
 * The object type descriptor for the property implementation cache type.
 */
static const Tcl_ObjType propNameType = {
    "tcl::oo property name",
    FreePropName,		// FreeIntRep
    DupPropName,		// DupIntRep
    NULL,			// UpdateString
    NULL,			// SetFromAnyProc
    TCL_OBJTYPE_V0
};

/*
 * How to install a cache value into a Tcl_Obj internal representation.
 */
#define SET_PROPERTY_NAME(objPtr, namePtr) \
    do {							\
	Tcl_ObjInternalRep ir;					\
	ir.twoPtrValue.ptr1 = (namePtr);			\
	ir.twoPtrValue.ptr2 = NULL;				\
	Tcl_StoreInternalRep((objPtr), &propNameType, &ir);	\
    } while (0)

/*
 * How to retrieve a cache value from a Tcl_Obj internal representation.
 */
#define GET_PROPERTY_NAME(irPtr) \
    ((PropertyName *) (irPtr)->twoPtrValue.ptr1)

/*
 * ----------------------------------------------------------------------
 *
 * DupPropName, FreePropName, GetImplNamesForProperty --
 *
 *	A caching scheme for the actual property implementation names so
 *	that don't have to reallocate the method name each time we call the
 *	implementations.
 *
 *	Note that this scheme is NOT used by the property configuration code,
 *	where the starting names have a slightly different structure.
 *
 * ----------------------------------------------------------------------
 */

static void
FreePropName(
    Tcl_Obj *objPtr)
{
    PropertyName *namePtr = GET_PROPERTY_NAME(&objPtr->internalRep);
    TclDecrRefCount(namePtr->readerName);
    TclDecrRefCount(namePtr->writerName);
    Tcl_Free((void *) namePtr);
    objPtr->typePtr = NULL;
}

static void
DupPropName(
    Tcl_Obj *srcPtr,
    Tcl_Obj *copyPtr)
{

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetAllObjectProperties(`, lines 282–390. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
				 * wants that. The contents are to be freed
				 * with Tcl_Free if the cache is used. */
{
    Tcl_Size objc, index, i;
    Tcl_Obj *listPtr = GetAllObjectProperties(
	    oPtr, (flags & GPN_WRITABLE) != 0);
    Tcl_Obj **objv;
    GPNCache *tablePtr;

    (void) Tcl_ListObjGetElements(NULL, listPtr, &objc, &objv);
    if (cachePtr && *cachePtr) {
	tablePtr = *cachePtr;
    } else {
	tablePtr = (GPNCache *) TclStackAlloc(interp,
		offsetof(GPNCache, names) + sizeof(char *) * ((size_t)objc + 1));

	for (i = 0; i < objc; i++) {
	    tablePtr->names[i] = TclGetString(objv[i]);
	}
	tablePtr->names[objc] = NULL;
	if (cachePtr) {
	    /*
	     * Have a cache, but nothing in it so far.
	     *
	     * We cache the list here so it doesn't vanish from under our
	     * feet if a property implementation does something crazy like
	     * changing the set of properties. The type of copy this does
	     * means that the copy holds the references to the names in the
	     * table.
	     */
	    tablePtr->listPtr = TclListObjCopy(NULL, listPtr);
	    Tcl_IncrRefCount(tablePtr->listPtr);
	    *cachePtr = tablePtr;
	} else {
	    tablePtr->listPtr = NULL;
	}
    }
    int result = Tcl_GetIndexFromObjStruct(interp, namePtr, tablePtr->names,
	    sizeof(char *), "property", TCL_INDEX_TEMP_TABLE, &index);
    if (result == TCL_ERROR && !(flags & GPN_FALLING_BACK)) {
	/*
	 * If property can be accessed the other way, use a special message.
	 * We use a recursive call to look this up.
	 */

	Tcl_InterpState state = Tcl_SaveInterpState(interp, result);
	Tcl_Obj *otherName = GetPropertyName(interp, oPtr,
		flags ^ (GPN_WRITABLE | GPN_FALLING_BACK), namePtr, NULL);
	result = Tcl_RestoreInterpState(interp, state);
	if (otherName != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "property \"%s\" is %s only",
		    TclGetString(otherName),
		    (flags & GPN_WRITABLE) ? "read" : "write"));
	}
    }
    if (!cachePtr) {
	TclStackFree(interp, tablePtr);
    }
    if (result != TCL_OK) {
	return NULL;
    }
    return objv[index];
}

/* Release the cache made by GetPropertyName(). */
static inline void
ReleasePropertyNameCache(
    Tcl_Interp *interp,
    GPNCache **cachePtr)
{
    if (*cachePtr) {
	GPNCache *tablePtr = *cachePtr;
	if (tablePtr->listPtr) {
	    Tcl_DecrRefCount(tablePtr->listPtr);
	}
	TclStackFree(interp, tablePtr);
	*cachePtr = NULL;
    }
}

int
TclOO_Configurable_Configure(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter used for the result, error
				 * reporting, etc. */
    Tcl_ObjectContext context,	/* The object/call context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    Tcl_Obj *namePtr;
    Tcl_Size i, namec;
    int code = TCL_OK;

    objc -= skip;
    if ((objc & 1) && (objc != 1)) {
	/*
	 * Bad (odd > 1) number of arguments.
	 */

	Tcl_WrongNumArgs(interp, skip, objv, "?-option value ...?");
	return TCL_ERROR;
    }

    objv += skip;
    if (objc == 0) {
	/*

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetAllObjectProperties(`, lines 390–497. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
	/*
	 * Read all properties.
	 */

	Tcl_Obj *listPtr = GetAllObjectProperties(oPtr, false);
	Tcl_Obj *resultPtr = Tcl_NewObj(), **namev;

	Tcl_IncrRefCount(listPtr);
	ListObjGetElements(listPtr, namec, namev);

	for (i = 0; i < namec; ) {
	    code = ReadProperty(interp, oPtr, namev[i]);
	    if (code != TCL_OK) {
		Tcl_DecrRefCount(resultPtr);
		break;
	    }
	    Tcl_DictObjPut(NULL, resultPtr, namev[i],
		    Tcl_GetObjResult(interp));
	    if (++i >= namec) {
		Tcl_SetObjResult(interp, resultPtr);
		break;
	    }
	    Tcl_SetObjResult(interp, Tcl_NewObj());
	}
	Tcl_DecrRefCount(listPtr);
	return code;
    } else if (objc == 1) {
	/*
	 * Read a single named property.
	 */

	namePtr = GetPropertyName(interp, oPtr, 0, objv[0], NULL);
	if (namePtr == NULL) {
	    return TCL_ERROR;
	}
	return ReadProperty(interp, oPtr, namePtr);
    } else if (objc == 2) {
	/*
	 * Special case for writing to one property. Saves fiddling with the
	 * cache in this common case.
	 */

	namePtr = GetPropertyName(interp, oPtr, GPN_WRITABLE, objv[0], NULL);
	if (namePtr == NULL) {
	    return TCL_ERROR;
	}
	code = WriteProperty(interp, oPtr, namePtr, objv[1]);
	if (code == TCL_OK) {
	    Tcl_ResetResult(interp);
	}
	return code;
    } else {
	/*
	 * Write properties. Slightly tricky because we want to cache the
	 * table of property names.
	 */
	GPNCache *cache = NULL;

	code = TCL_OK;
	for (i = 0; i < objc; i += 2) {
	    namePtr = GetPropertyName(interp, oPtr, GPN_WRITABLE, objv[i],
		    &cache);
	    if (namePtr == NULL) {
		code = TCL_ERROR;
		break;
	    }
	    code = WriteProperty(interp, oPtr, namePtr, objv[i + 1]);
	    if (code != TCL_OK) {
		break;
	    }
	}
	if (code == TCL_OK) {
	    Tcl_ResetResult(interp);
	}
	ReleasePropertyNameCache(interp, &cache);
	return code;
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * Configurable_Getter, Configurable_Setter --
 *
 *	Standard property implementation. The clientData is a simple Tcl_Obj *
 *	that contains the name of the property.
 *
 * ----------------------------------------------------------------------
 */

static int
Configurable_Getter(
    void *clientData,		/* Which property to read. Actually a Tcl_Obj *
				 * reference that is the name of the variable
				 * in the cpntext object. */
    Tcl_Interp *interp,		/* Interpreter used for the result, error
				 * reporting, etc. */
    Tcl_ObjectContext context,	/* The object/call context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Tcl_Obj *propNamePtr = (Tcl_Obj *) clientData;
    Tcl_Var varPtr, aryVar;
    Tcl_Obj *valuePtr;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context),
		objv, NULL);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetAllObjectProperties(`, lines 851–958. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
 * ----------------------------------------------------------------------
 */

Tcl_Obj *
GetAllObjectProperties(
    Object *oPtr,		/* The object to inspect. Must exist. */
    bool writable)		/* Whether to get writable properties. If
				 * false, readable properties will be returned
				 * instead. */
{
    Tcl_HashTable hashTable;
    FOREACH_HASH_DECLS;
    Tcl_Obj *propName, *result;

    /*
     * Look in the cache.
     */

    if (oPtr->properties.epoch == oPtr->fPtr->epoch) {
	if (writable) {
	    if (oPtr->properties.allWritableCache) {
		return oPtr->properties.allWritableCache;
	    }
	} else {
	    if (oPtr->properties.allReadableCache) {
		return oPtr->properties.allReadableCache;
	    }
	}
    }

    /*
     * Gather the information. Unsorted! (Caller will sort.)
     */

    Tcl_InitObjHashTable(&hashTable);
    FindObjectProps(oPtr, writable, &hashTable);
    TclNewObj(result);
    FOREACH_HASH_KEY(propName, &hashTable) {
	Tcl_ListObjAppendElement(NULL, result, propName);
    }
    Tcl_DeleteHashTable(&hashTable);
    SortPropList(result);

    /*
     * Cache the information.
     */

    if (oPtr->properties.epoch != oPtr->fPtr->epoch) {
	if (oPtr->properties.allWritableCache) {
	    Tcl_DecrRefCount(oPtr->properties.allWritableCache);
	    oPtr->properties.allWritableCache = NULL;
	}
	if (oPtr->properties.allReadableCache) {
	    Tcl_DecrRefCount(oPtr->properties.allReadableCache);
	    oPtr->properties.allReadableCache = NULL;
	}
    }
    oPtr->properties.epoch = oPtr->fPtr->epoch;
    if (writable) {
	oPtr->properties.allWritableCache = result;
    } else {
	oPtr->properties.allReadableCache = result;
    }
    Tcl_IncrRefCount(result);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * SetPropertyList --
 *
 *	Helper for writing a property list (which is actually a set).
 *
 * ----------------------------------------------------------------------
 */
static inline void
SetPropertyList(
    PropertyList *propList,	/* The property list to write. Replaces the
				 * property list's contents. */
    Tcl_Size objc,		/* Number of property names. */
    Tcl_Obj *const *objv)	/* Property names. */
{
    Tcl_Size i, n;
    Tcl_Obj *propObj;
    int isNew;
    Tcl_HashTable uniqueTable;

    for (i=0 ; i<objc ; i++) {
	Tcl_IncrRefCount(objv[i]);
    }
    FOREACH(propObj, *propList) {
	Tcl_DecrRefCount(propObj);
    }
    if (i != objc) {
	if (objc == 0) {
	    Tcl_Free(propList->list);
	} else if (i) {
	    propList->list = (Tcl_Obj **)
		    Tcl_Realloc(propList->list, sizeof(Tcl_Obj *) * ((size_t)objc));
	} else {
	    propList->list = (Tcl_Obj **)
		    Tcl_Alloc(sizeof(Tcl_Obj *) * ((size_t)objc));
	}
    }
    propList->num = 0;
    if (objc > 0) {
	Tcl_InitObjHashTable(&uniqueTable);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetAllObjectProperties(`, lines 1406–1472. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
     */

    Tcl_Obj *result;
    if (all) {
	result = GetAllObjectProperties(oPtr, writable);
    } else {
	if (writable) {
	    result = TclOOGetPropertyList(&oPtr->properties.writable);
	} else {
	    result = TclOOGetPropertyList(&oPtr->properties.readable);
	}
	SortPropList(result);
    }
    Tcl_SetObjResult(interp, result);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOReleasePropertyStorage --
 *
 *	Delete the memory associated with a class or object's properties.
 *
 * ----------------------------------------------------------------------
 */

static inline void
ReleasePropertyList(
    PropertyList *propList)
{
    Tcl_Obj *propertyObj;
    Tcl_Size i;

    FOREACH(propertyObj, *propList) {
	Tcl_DecrRefCount(propertyObj);
    }
    Tcl_Free(propList->list);
    propList->list = NULL;
    propList->num = 0;
}

void
TclOOReleasePropertyStorage(
    PropertyStorage *propsPtr)
{
    if (propsPtr->allReadableCache) {
	Tcl_DecrRefCount(propsPtr->allReadableCache);
    }
    if (propsPtr->allWritableCache) {
	Tcl_DecrRefCount(propsPtr->allWritableCache);
    }
    if (propsPtr->readable.num) {
	ReleasePropertyList(&propsPtr->readable);
    }
    if (propsPtr->writable.num) {
	ReleasePropertyList(&propsPtr->writable);
    }
}

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `TclOOInfoClassPropCmd(`, lines 1299–1407. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
 * ----------------------------------------------------------------------
 */

int
TclOOInfoClassPropCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr;
    Tcl_Size i;
    int idx;
    bool all = false, writable = false, allocated = false;
    Tcl_Obj *result;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "className ?options...?");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }
    for (i = 2; i < objc; i++) {
	if (Tcl_GetIndexFromObj(interp, objv[i], propOptNames, "option", 0,
		&idx) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (idx) {
	case PROP_ALL:
	    all = true;
	    break;
	case PROP_READABLE:
	    writable = false;
	    break;
	case PROP_WRITABLE:
	    writable = true;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

    /*
     * Get the properties.
     */

    if (all) {
	result = GetAllClassProperties(clsPtr, writable, &allocated);
	if (allocated) {
	    SortPropList(result);
	}
    } else {
	if (writable) {
	    result = TclOOGetPropertyList(&clsPtr->properties.writable);
	} else {
	    result = TclOOGetPropertyList(&clsPtr->properties.readable);
	}
	SortPropList(result);
    }
    Tcl_SetObjResult(interp, result);
    return TCL_OK;
}

int
TclOOInfoObjectPropCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Tcl_Size i;
    bool all = false, writable = false;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName ?options...?");
	return TCL_ERROR;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    for (i = 2; i < objc; i++) {
	int idx;
	if (Tcl_GetIndexFromObj(interp, objv[i], propOptNames, "option", 0,
		&idx) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (idx) {
	case PROP_ALL:
	    all = true;
	    break;
	case PROP_READABLE:
	    writable = false;
	    break;
	case PROP_WRITABLE:
	    writable = true;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

    /*
     * Get the properties.
     */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `TclOOInfoObjectPropCmd(`, lines 1361–1467. Retained reference `runtime-rust-tests-data-native_property_slot_reference_order_original-original-inputs-workspace--.proofs--native-property-order509b--original-request--source-inspection--9.1.0--tclOOProp.c`.

```text
    return TCL_OK;
}

int
TclOOInfoObjectPropCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Tcl_Size i;
    bool all = false, writable = false;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName ?options...?");
	return TCL_ERROR;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    for (i = 2; i < objc; i++) {
	int idx;
	if (Tcl_GetIndexFromObj(interp, objv[i], propOptNames, "option", 0,
		&idx) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (idx) {
	case PROP_ALL:
	    all = true;
	    break;
	case PROP_READABLE:
	    writable = false;
	    break;
	case PROP_WRITABLE:
	    writable = true;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

    /*
     * Get the properties.
     */

    Tcl_Obj *result;
    if (all) {
	result = GetAllObjectProperties(oPtr, writable);
    } else {
	if (writable) {
	    result = TclOOGetPropertyList(&oPtr->properties.writable);
	} else {
	    result = TclOOGetPropertyList(&oPtr->properties.readable);
	}
	SortPropList(result);
    }
    Tcl_SetObjResult(interp, result);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOReleasePropertyStorage --
 *
 *	Delete the memory associated with a class or object's properties.
 *
 * ----------------------------------------------------------------------
 */

static inline void
ReleasePropertyList(
    PropertyList *propList)
{
    Tcl_Obj *propertyObj;
    Tcl_Size i;

    FOREACH(propertyObj, *propList) {
	Tcl_DecrRefCount(propertyObj);
    }
    Tcl_Free(propList->list);
    propList->list = NULL;
    propList->num = 0;
}

void
TclOOReleasePropertyStorage(
    PropertyStorage *propsPtr)
{
    if (propsPtr->allReadableCache) {
	Tcl_DecrRefCount(propsPtr->allReadableCache);
    }
    if (propsPtr->allWritableCache) {
	Tcl_DecrRefCount(propsPtr->allWritableCache);
    }
    if (propsPtr->readable.num) {
	ReleasePropertyList(&propsPtr->readable);
    }
    if (propsPtr->writable.num) {
	ReleasePropertyList(&propsPtr->writable);
    }
}

/*
 * Local Variables:

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `Slot_Set(`, lines 3033–3138. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    Tcl_Obj *list;

    // Resolve all values
    if (skip == objc) {
	list = Tcl_NewObj();
    } else {
	list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
	if (list == NULL) {
	    return TCL_ERROR;
	}
    }
    Tcl_IncrRefCount(list);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unimplemented --
 *
 *	Default implementation of the "Get" and "Set" slot accessors. Just
 *	returns an error; actual slots must override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unimplemented(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    TCL_UNUSED(Tcl_ObjectContext),
    TCL_UNUSED(Tcl_Size),
    TCL_UNUSED(Tcl_Obj *const *))
{
    Tcl_SetObjResult(interp, Tcl_NewStringObj("unimplemented", -1));
    OO_ERROR(interp, ABSTRACT_SLOT);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unknown --
 *
 *	Unknown method name handler for slots. Delegates to the default slot
 *	operation (--default-operation forwarded method) unless the first
 *	argument starts with a dash.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unknown(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip >= objc) {
	Tcl_Obj *args[] = {
	    oPtr->fPtr->myName,
	    oPtr->fPtr->slotDefOpName
	};
	return TclOOPrivateObjectCmd(oPtr, interp, 2, args);
    } else if (TclGetString(objv[skip])[0] != '-') {
	Tcl_Obj **args = (Tcl_Obj **) TclStackAlloc(interp,
		sizeof(Tcl_Obj *) * (objc - skip + 2));
	args[0] = oPtr->fPtr->myName;
	args[1] = oPtr->fPtr->slotDefOpName;
	memcpy(args+2, objv+skip, sizeof(Tcl_Obj *) * (objc - skip));
	int code = TclOOPrivateObjectCmd(oPtr, interp, objc - skip + 2, args);
	TclStackFree(interp, args);
	return code;
    }
    return TclNRObjectContextInvokeNext(interp, context, objc, objv, skip);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOSetSuperclasses --
 *
 *	Core of the "superclass" slot setter. Caller must AddRef() the objects
 *	holding the classes to set before calling this. The 'superclasses'
 *	argument must be allocated with Tcl_Alloc(); this function takes
 *	ownership.
 *
 * ----------------------------------------------------------------------

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 2688–2794. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
}

// Call [$slot Resolve $item] for each of a whole list of items.
static inline Tcl_Obj *
ResolveAll(
    Tcl_Interp *interp,
    Object *slot,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Obj **resolvedItems = (Tcl_Obj **) TclStackAlloc(interp,
	    sizeof(Tcl_Obj *) * objc);
    for (Tcl_Size i = 0; i < objc; i++) {
	resolvedItems[i] = CallSlotResolve(interp, slot, objv[i]);
	if (resolvedItems[i] == NULL) {
	    for (Tcl_Size j = 0; j < i; j++) {
		Tcl_DecrRefCount(resolvedItems[j]);
	    }
	    TclStackFree(interp, (void *) resolvedItems);
	    return NULL;
	}
	Tcl_IncrRefCount(resolvedItems[i]);
	Tcl_ResetResult(interp);
    }
    Tcl_Obj *resolvedList = Tcl_NewListObj(objc, resolvedItems);
    for (Tcl_Size i = 0; i < objc; i++) {
	TclDecrRefCount(resolvedItems[i]);
    }
    TclStackFree(interp, (void *) resolvedItems);
    return resolvedList;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Append --
 *
 *	Implementation of the "-append" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Append(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip == objc) {
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    // Get slot contents; store in list
    Tcl_Obj *list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

    // Append
    if (Tcl_IsShared(list)) {
	Tcl_Obj *dup = Tcl_DuplicateObj(list);
	Tcl_IncrRefCount(dup);
	Tcl_DecrRefCount(list);
	list = dup;
    }
    if (Tcl_ListObjAppendList(interp, list, resolved) != TCL_OK) {
	Tcl_DecrRefCount(list);
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_DecrRefCount(resolved);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_AppendNew --
 *
 *	Implementation of the "-appendifnew" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_AppendNew(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 2740–2847. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    // Get slot contents; store in list
    Tcl_Obj *list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

    // Append
    if (Tcl_IsShared(list)) {
	Tcl_Obj *dup = Tcl_DuplicateObj(list);
	Tcl_IncrRefCount(dup);
	Tcl_DecrRefCount(list);
	list = dup;
    }
    if (Tcl_ListObjAppendList(interp, list, resolved) != TCL_OK) {
	Tcl_DecrRefCount(list);
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_DecrRefCount(resolved);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_AppendNew --
 *
 *	Implementation of the "-appendifnew" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_AppendNew(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip == objc) {
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    // Get slot contents; store in list
    Tcl_Obj *list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

    // Prepare a set of items in the list to set
    Tcl_Size listc;
    Tcl_Obj **listv;
    if (TclListObjGetElements(interp, list, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(list);
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_HashTable unique;
    Tcl_InitObjHashTable(&unique);
    for (Tcl_Size i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&unique, listv[i], NULL);
    }

    // Append the new items if they're not already there
    if (Tcl_IsShared(list)) {
	Tcl_Obj *dup = Tcl_DuplicateObj(list);
	Tcl_IncrRefCount(dup);
	Tcl_DecrRefCount(list);
	list = dup;
    }
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    for (Tcl_Size i=0 ; i<listc; i++) {
	int isNew;
	Tcl_CreateHashEntry(&unique, listv[i], &isNew);
	if (isNew) {
	    Tcl_ListObjAppendElement(interp, list, listv[i]);
	}
    }
    Tcl_DecrRefCount(resolved);
    Tcl_DeleteHashTable(&unique);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 2798–2904. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    // Get slot contents; store in list
    Tcl_Obj *list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

    // Prepare a set of items in the list to set
    Tcl_Size listc;
    Tcl_Obj **listv;
    if (TclListObjGetElements(interp, list, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(list);
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_HashTable unique;
    Tcl_InitObjHashTable(&unique);
    for (Tcl_Size i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&unique, listv[i], NULL);
    }

    // Append the new items if they're not already there
    if (Tcl_IsShared(list)) {
	Tcl_Obj *dup = Tcl_DuplicateObj(list);
	Tcl_IncrRefCount(dup);
	Tcl_DecrRefCount(list);
	list = dup;
    }
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    for (Tcl_Size i=0 ; i<listc; i++) {
	int isNew;
	Tcl_CreateHashEntry(&unique, listv[i], &isNew);
	if (isNew) {
	    Tcl_ListObjAppendElement(interp, list, listv[i]);
	}
    }
    Tcl_DecrRefCount(resolved);
    Tcl_DeleteHashTable(&unique);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Clear --
 *
 *	Implementation of the "-clear" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Clear(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip != objc) {
	Tcl_WrongNumArgs(interp, skip, objv, NULL);
	return TCL_ERROR;
    }
    Tcl_Obj *list = Tcl_NewObj();
    Tcl_IncrRefCount(list);
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Prepend --
 *
 *	Implementation of the "-prepend" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Prepend(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip == objc) {
	return TCL_OK;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 2904–3010. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (list == NULL) {
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);

    // Get slot contents and append to list
    Tcl_Obj *oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(list);
	return TCL_ERROR;
    }
    Tcl_ListObjAppendList(NULL, list, oldList);
    Tcl_ResetResult(interp);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Remove --
 *
 *	Implementation of the "-remove" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Remove(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip == objc) {
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    // Get slot contents; store in list
    Tcl_Obj *oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(oldList);
    Tcl_ResetResult(interp);

    // Prepare a set of items in the list to remove
    Tcl_Size listc;
    Tcl_Obj **listv;
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    Tcl_HashTable removeSet;
    Tcl_InitObjHashTable(&removeSet);
    for (Tcl_Size i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&removeSet, listv[i], NULL);
    }
    Tcl_DecrRefCount(resolved);

    // Append the new items from the old items if they're not in the remove set
    if (TclListObjGetElements(interp, oldList, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(oldList);
	Tcl_DeleteHashTable(&removeSet);
	return TCL_ERROR;
    }
    Tcl_Obj *newList = Tcl_NewObj();
    for (Tcl_Size i=0 ; i<listc; i++) {
	if (Tcl_FindHashEntry(&removeSet, listv[i]) == NULL) {
	    Tcl_ListObjAppendElement(NULL, newList, listv[i]);
	}
    }
    Tcl_DecrRefCount(oldList);
    Tcl_DeleteHashTable(&removeSet);

    // Set slot contents
    Tcl_IncrRefCount(newList);
    int code = CallSlotSet(interp, oPtr, newList);
    Tcl_DecrRefCount(newList);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Resolve --
 *
 *	Default implementation of the "Resolve" slot accessor. Just returns
 *	its argument unchanged; particular slots may override.
 *
 * ----------------------------------------------------------------------
 */
static int

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 2949–3055. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    // Get slot contents; store in list
    Tcl_Obj *oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(oldList);
    Tcl_ResetResult(interp);

    // Prepare a set of items in the list to remove
    Tcl_Size listc;
    Tcl_Obj **listv;
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    Tcl_HashTable removeSet;
    Tcl_InitObjHashTable(&removeSet);
    for (Tcl_Size i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&removeSet, listv[i], NULL);
    }
    Tcl_DecrRefCount(resolved);

    // Append the new items from the old items if they're not in the remove set
    if (TclListObjGetElements(interp, oldList, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(oldList);
	Tcl_DeleteHashTable(&removeSet);
	return TCL_ERROR;
    }
    Tcl_Obj *newList = Tcl_NewObj();
    for (Tcl_Size i=0 ; i<listc; i++) {
	if (Tcl_FindHashEntry(&removeSet, listv[i]) == NULL) {
	    Tcl_ListObjAppendElement(NULL, newList, listv[i]);
	}
    }
    Tcl_DecrRefCount(oldList);
    Tcl_DeleteHashTable(&removeSet);

    // Set slot contents
    Tcl_IncrRefCount(newList);
    int code = CallSlotSet(interp, oPtr, newList);
    Tcl_DecrRefCount(newList);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Resolve --
 *
 *	Default implementation of the "Resolve" slot accessor. Just returns
 *	its argument unchanged; particular slots may override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Resolve(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip + 1 != objc) {
	Tcl_WrongNumArgs(interp, skip, objv, "list");
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, objv[objc - 1]);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Set --
 *
 *	Implementation of the "-set" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    Tcl_Obj *list;

    // Resolve all values
    if (skip == objc) {
	list = Tcl_NewObj();
    } else {
	list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
	if (list == NULL) {
	    return TCL_ERROR;
	}

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 3048–3153. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    // Resolve all values
    if (skip == objc) {
	list = Tcl_NewObj();
    } else {
	list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
	if (list == NULL) {
	    return TCL_ERROR;
	}
    }
    Tcl_IncrRefCount(list);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unimplemented --
 *
 *	Default implementation of the "Get" and "Set" slot accessors. Just
 *	returns an error; actual slots must override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unimplemented(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    TCL_UNUSED(Tcl_ObjectContext),
    TCL_UNUSED(Tcl_Size),
    TCL_UNUSED(Tcl_Obj *const *))
{
    Tcl_SetObjResult(interp, Tcl_NewStringObj("unimplemented", -1));
    OO_ERROR(interp, ABSTRACT_SLOT);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unknown --
 *
 *	Unknown method name handler for slots. Delegates to the default slot
 *	operation (--default-operation forwarded method) unless the first
 *	argument starts with a dash.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unknown(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip >= objc) {
	Tcl_Obj *args[] = {
	    oPtr->fPtr->myName,
	    oPtr->fPtr->slotDefOpName
	};
	return TclOOPrivateObjectCmd(oPtr, interp, 2, args);
    } else if (TclGetString(objv[skip])[0] != '-') {
	Tcl_Obj **args = (Tcl_Obj **) TclStackAlloc(interp,
		sizeof(Tcl_Obj *) * (objc - skip + 2));
	args[0] = oPtr->fPtr->myName;
	args[1] = oPtr->fPtr->slotDefOpName;
	memcpy(args+2, objv+skip, sizeof(Tcl_Obj *) * (objc - skip));
	int code = TclOOPrivateObjectCmd(oPtr, interp, objc - skip + 2, args);
	TclStackFree(interp, args);
	return code;
    }
    return TclNRObjectContextInvokeNext(interp, context, objc, objv, skip);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOSetSuperclasses --
 *
 *	Core of the "superclass" slot setter. Caller must AddRef() the objects
 *	holding the classes to set before calling this. The 'superclasses'
 *	argument must be allocated with Tcl_Alloc(); this function takes
 *	ownership.
 *
 * ----------------------------------------------------------------------
 */
void
TclOOSetSuperclasses(
    Class *clsPtr,
    Tcl_Size superc,
    Class **superclasses)
{
    Tcl_Size i;
    Class *superPtr;

    if (clsPtr->superclasses.num != 0) {
	FOREACH(superPtr, clsPtr->superclasses) {
	    TclOORemoveFromSubclasses(clsPtr, superPtr);
	    TclOODecrRefCount(superPtr->thisPtr);
	}

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 2654–2761. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
}

// Call [$slot Set $list] to set the list of contents of the slot.
static inline int
CallSlotSet(
    Tcl_Interp *interp,
    Object *slot,
    Tcl_Obj *list)
{
    Tcl_Obj *setArgs[] = {
	slot->fPtr->myName,
	slot->fPtr->slotSetName,
	list
    };
    return TclOOPrivateObjectCmd(slot, interp, 3, setArgs);
}

// Call [$slot Resolve $item] to convert a slot item into canonical form.
static inline Tcl_Obj *
CallSlotResolve(
    Tcl_Interp *interp,
    Object *slot,
    Tcl_Obj *item)
{
    Tcl_Obj *resolveArgs[] = {
	slot->fPtr->myName,
	slot->fPtr->slotResolveName,
	item
    };
    int code = TclOOPrivateObjectCmd(slot, interp, 3, resolveArgs);
    if (code != TCL_OK) {
	return NULL;
    }
    return Tcl_GetObjResult(interp);
}

// Call [$slot Resolve $item] for each of a whole list of items.
static inline Tcl_Obj *
ResolveAll(
    Tcl_Interp *interp,
    Object *slot,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Obj **resolvedItems = (Tcl_Obj **) TclStackAlloc(interp,
	    sizeof(Tcl_Obj *) * objc);
    for (Tcl_Size i = 0; i < objc; i++) {
	resolvedItems[i] = CallSlotResolve(interp, slot, objv[i]);
	if (resolvedItems[i] == NULL) {
	    for (Tcl_Size j = 0; j < i; j++) {
		Tcl_DecrRefCount(resolvedItems[j]);
	    }
	    TclStackFree(interp, (void *) resolvedItems);
	    return NULL;
	}
	Tcl_IncrRefCount(resolvedItems[i]);
	Tcl_ResetResult(interp);
    }
    Tcl_Obj *resolvedList = Tcl_NewListObj(objc, resolvedItems);
    for (Tcl_Size i = 0; i < objc; i++) {
	TclDecrRefCount(resolvedItems[i]);
    }
    TclStackFree(interp, (void *) resolvedItems);
    return resolvedList;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Append --
 *
 *	Implementation of the "-append" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Append(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip == objc) {
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    // Get slot contents; store in list
    Tcl_Obj *list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

    // Append
    if (Tcl_IsShared(list)) {
	Tcl_Obj *dup = Tcl_DuplicateObj(list);
	Tcl_IncrRefCount(dup);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 2769–2875. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    }
    Tcl_DecrRefCount(resolved);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_AppendNew --
 *
 *	Implementation of the "-appendifnew" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_AppendNew(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip == objc) {
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    // Get slot contents; store in list
    Tcl_Obj *list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

    // Prepare a set of items in the list to set
    Tcl_Size listc;
    Tcl_Obj **listv;
    if (TclListObjGetElements(interp, list, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(list);
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_HashTable unique;
    Tcl_InitObjHashTable(&unique);
    for (Tcl_Size i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&unique, listv[i], NULL);
    }

    // Append the new items if they're not already there
    if (Tcl_IsShared(list)) {
	Tcl_Obj *dup = Tcl_DuplicateObj(list);
	Tcl_IncrRefCount(dup);
	Tcl_DecrRefCount(list);
	list = dup;
    }
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    for (Tcl_Size i=0 ; i<listc; i++) {
	int isNew;
	Tcl_CreateHashEntry(&unique, listv[i], &isNew);
	if (isNew) {
	    Tcl_ListObjAppendElement(interp, list, listv[i]);
	}
    }
    Tcl_DecrRefCount(resolved);
    Tcl_DeleteHashTable(&unique);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Clear --
 *
 *	Implementation of the "-clear" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Clear(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip != objc) {
	Tcl_WrongNumArgs(interp, skip, objv, NULL);
	return TCL_ERROR;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 2845–2950. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Tcl_DecrRefCount(resolved);
    Tcl_DeleteHashTable(&unique);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Clear --
 *
 *	Implementation of the "-clear" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Clear(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip != objc) {
	Tcl_WrongNumArgs(interp, skip, objv, NULL);
	return TCL_ERROR;
    }
    Tcl_Obj *list = Tcl_NewObj();
    Tcl_IncrRefCount(list);
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Prepend --
 *
 *	Implementation of the "-prepend" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Prepend(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip == objc) {
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (list == NULL) {
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);

    // Get slot contents and append to list
    Tcl_Obj *oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(list);
	return TCL_ERROR;
    }
    Tcl_ListObjAppendList(NULL, list, oldList);
    Tcl_ResetResult(interp);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Remove --
 *
 *	Implementation of the "-remove" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Remove(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip == objc) {
	return TCL_OK;
    }

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 2875–2981. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
	return TCL_ERROR;
    }
    Tcl_Obj *list = Tcl_NewObj();
    Tcl_IncrRefCount(list);
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Prepend --
 *
 *	Implementation of the "-prepend" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Prepend(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip == objc) {
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (list == NULL) {
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);

    // Get slot contents and append to list
    Tcl_Obj *oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(list);
	return TCL_ERROR;
    }
    Tcl_ListObjAppendList(NULL, list, oldList);
    Tcl_ResetResult(interp);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Remove --
 *
 *	Implementation of the "-remove" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Remove(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip == objc) {
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    // Get slot contents; store in list
    Tcl_Obj *oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(oldList);
    Tcl_ResetResult(interp);

    // Prepare a set of items in the list to remove
    Tcl_Size listc;
    Tcl_Obj **listv;
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    Tcl_HashTable removeSet;
    Tcl_InitObjHashTable(&removeSet);
    for (Tcl_Size i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&removeSet, listv[i], NULL);
    }
    Tcl_DecrRefCount(resolved);

    // Append the new items from the old items if they're not in the remove set
    if (TclListObjGetElements(interp, oldList, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(oldList);
	Tcl_DeleteHashTable(&removeSet);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 2920–3026. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Tcl_ListObjAppendList(NULL, list, oldList);
    Tcl_ResetResult(interp);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Remove --
 *
 *	Implementation of the "-remove" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Remove(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip == objc) {
	return TCL_OK;
    }

    // Resolve all values
    Tcl_Obj *resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    // Get slot contents; store in list
    Tcl_Obj *oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(oldList);
    Tcl_ResetResult(interp);

    // Prepare a set of items in the list to remove
    Tcl_Size listc;
    Tcl_Obj **listv;
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    Tcl_HashTable removeSet;
    Tcl_InitObjHashTable(&removeSet);
    for (Tcl_Size i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&removeSet, listv[i], NULL);
    }
    Tcl_DecrRefCount(resolved);

    // Append the new items from the old items if they're not in the remove set
    if (TclListObjGetElements(interp, oldList, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(oldList);
	Tcl_DeleteHashTable(&removeSet);
	return TCL_ERROR;
    }
    Tcl_Obj *newList = Tcl_NewObj();
    for (Tcl_Size i=0 ; i<listc; i++) {
	if (Tcl_FindHashEntry(&removeSet, listv[i]) == NULL) {
	    Tcl_ListObjAppendElement(NULL, newList, listv[i]);
	}
    }
    Tcl_DecrRefCount(oldList);
    Tcl_DeleteHashTable(&removeSet);

    // Set slot contents
    Tcl_IncrRefCount(newList);
    int code = CallSlotSet(interp, oPtr, newList);
    Tcl_DecrRefCount(newList);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Resolve --
 *
 *	Default implementation of the "Resolve" slot accessor. Just returns
 *	its argument unchanged; particular slots may override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Resolve(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip + 1 != objc) {
	Tcl_WrongNumArgs(interp, skip, objv, "list");
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, objv[objc - 1]);
    return TCL_OK;
}


```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 2991–3095. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Tcl_DeleteHashTable(&removeSet);

    // Set slot contents
    Tcl_IncrRefCount(newList);
    int code = CallSlotSet(interp, oPtr, newList);
    Tcl_DecrRefCount(newList);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Resolve --
 *
 *	Default implementation of the "Resolve" slot accessor. Just returns
 *	its argument unchanged; particular slots may override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Resolve(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip + 1 != objc) {
	Tcl_WrongNumArgs(interp, skip, objv, "list");
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, objv[objc - 1]);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Set --
 *
 *	Implementation of the "-set" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    Tcl_Obj *list;

    // Resolve all values
    if (skip == objc) {
	list = Tcl_NewObj();
    } else {
	list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
	if (list == NULL) {
	    return TCL_ERROR;
	}
    }
    Tcl_IncrRefCount(list);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unimplemented --
 *
 *	Default implementation of the "Get" and "Set" slot accessors. Just
 *	returns an error; actual slots must override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unimplemented(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    TCL_UNUSED(Tcl_ObjectContext),
    TCL_UNUSED(Tcl_Size),
    TCL_UNUSED(Tcl_Obj *const *))
{
    Tcl_SetObjResult(interp, Tcl_NewStringObj("unimplemented", -1));
    OO_ERROR(interp, ABSTRACT_SLOT);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unknown --
 *
 *	Unknown method name handler for slots. Delegates to the default slot
 *	operation (--default-operation forwarded method) unless the first
 *	argument starts with a dash.

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 3056–3161. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    }
    Tcl_IncrRefCount(list);

    // Set slot contents
    int code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unimplemented --
 *
 *	Default implementation of the "Get" and "Set" slot accessors. Just
 *	returns an error; actual slots must override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unimplemented(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    TCL_UNUSED(Tcl_ObjectContext),
    TCL_UNUSED(Tcl_Size),
    TCL_UNUSED(Tcl_Obj *const *))
{
    Tcl_SetObjResult(interp, Tcl_NewStringObj("unimplemented", -1));
    OO_ERROR(interp, ABSTRACT_SLOT);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unknown --
 *
 *	Unknown method name handler for slots. Delegates to the default slot
 *	operation (--default-operation forwarded method) unless the first
 *	argument starts with a dash.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unknown(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip >= objc) {
	Tcl_Obj *args[] = {
	    oPtr->fPtr->myName,
	    oPtr->fPtr->slotDefOpName
	};
	return TclOOPrivateObjectCmd(oPtr, interp, 2, args);
    } else if (TclGetString(objv[skip])[0] != '-') {
	Tcl_Obj **args = (Tcl_Obj **) TclStackAlloc(interp,
		sizeof(Tcl_Obj *) * (objc - skip + 2));
	args[0] = oPtr->fPtr->myName;
	args[1] = oPtr->fPtr->slotDefOpName;
	memcpy(args+2, objv+skip, sizeof(Tcl_Obj *) * (objc - skip));
	int code = TclOOPrivateObjectCmd(oPtr, interp, objc - skip + 2, args);
	TclStackFree(interp, args);
	return code;
    }
    return TclNRObjectContextInvokeNext(interp, context, objc, objv, skip);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOSetSuperclasses --
 *
 *	Core of the "superclass" slot setter. Caller must AddRef() the objects
 *	holding the classes to set before calling this. The 'superclasses'
 *	argument must be allocated with Tcl_Alloc(); this function takes
 *	ownership.
 *
 * ----------------------------------------------------------------------
 */
void
TclOOSetSuperclasses(
    Class *clsPtr,
    Tcl_Size superc,
    Class **superclasses)
{
    Tcl_Size i;
    Class *superPtr;

    if (clsPtr->superclasses.num != 0) {
	FOREACH(superPtr, clsPtr->superclasses) {
	    TclOORemoveFromSubclasses(clsPtr, superPtr);
	    TclOODecrRefCount(superPtr->thisPtr);
	}
	Tcl_Free(clsPtr->superclasses.list);
    }
    clsPtr->superclasses.list = superclasses;
    clsPtr->superclasses.num = superc;
    FOREACH(superPtr, clsPtr->superclasses) {
	TclOOAddToSubclasses(clsPtr, superPtr);
    }
}

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `Configurable_ClassReadableProps_Set(`, lines 3921–4028. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    return TCL_OK;
}

static int
Configurable_ClassReadableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (TclListObjGetElements(interp, objv[0], &varc, &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallReadableProperties(&clsPtr->properties, varc, varv);
    BumpGlobalEpoch(interp, clsPtr);
    return TCL_OK;
}

static int
Configurable_ObjectReadableProps_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, TclOOGetPropertyList(&oPtr->properties.readable));
    return TCL_OK;
}

static int
Configurable_ObjectReadableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &varc,
	    &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallReadableProperties(&oPtr->properties, varc, varv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Configurable_ClassWritableProps_Get, Configurable_ClassWritableProps_Set,
 * Configurable_ObjectWritableProps_Get, Configurable_ObjectWritableProps_Set --
 *
 *	Implementations of the "writableproperties" slot accessors for classes
 *	and instances.
 *
 * ----------------------------------------------------------------------
 */

static int
Configurable_ClassWritableProps_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `Configurable_ObjectReadableProps_Set(`, lines 3973–4080. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    return TCL_OK;
}

static int
Configurable_ObjectReadableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &varc,
	    &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallReadableProperties(&oPtr->properties, varc, varv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Configurable_ClassWritableProps_Get, Configurable_ClassWritableProps_Set,
 * Configurable_ObjectWritableProps_Get, Configurable_ObjectWritableProps_Set --
 *
 *	Implementations of the "writableproperties" slot accessors for classes
 *	and instances.
 *
 * ----------------------------------------------------------------------
 */

static int
Configurable_ClassWritableProps_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, TclOOGetPropertyList(&clsPtr->properties.writable));
    return TCL_OK;
}

static int
Configurable_ClassWritableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"propertyList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (TclListObjGetElements(interp, objv[0], &varc, &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallWritableProperties(&clsPtr->properties, varc, varv);
    BumpGlobalEpoch(interp, clsPtr);
    return TCL_OK;
}

static int
Configurable_ObjectWritableProps_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);

    if (oPtr == NULL) {

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `Configurable_ClassWritableProps_Set(`, lines 4037–4144. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    return TCL_OK;
}

static int
Configurable_ClassWritableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"propertyList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (TclListObjGetElements(interp, objv[0], &varc, &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallWritableProperties(&clsPtr->properties, varc, varv);
    BumpGlobalEpoch(interp, clsPtr);
    return TCL_OK;
}

static int
Configurable_ObjectWritableProps_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, TclOOGetPropertyList(&oPtr->properties.writable));
    return TCL_OK;
}

static int
Configurable_ObjectWritableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"propertyList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &varc,
	    &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallWritableProperties(&oPtr->properties, varc, varv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOORegisterProperty, TclOORegisterInstanceProperty --
 *
 *	Helpers to add or remove a name from the property slots of a class or
 *	instance.
 *
 * BuildPropertyList --
 *
 *	Helper for the helpers. Scans a property list and does the filtering
 *	or adding of the property to add or remove
 *
 * ----------------------------------------------------------------------
 */

static bool
BuildPropertyList(
    PropertyList *propsList,	/* Property list to scan. */
    Tcl_Obj *propName,		/* Property to add/remove. */
    bool addingProp,		/* True if we're adding, false if removing. */
    Tcl_Obj *listObj)		/* The list of property names we're building */
{

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `configuresupport::readableproperties`, lines 126–232. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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
	return false;
    }
    return iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME;
}

/*
 * ----------------------------------------------------------------------
 *
 * BumpGlobalEpoch --
 *
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

	    /*
	     * Invalidate the property caches directly.
	     */

	    if (classPtr->properties.allReadableCache) {
		Tcl_DecrRefCount(classPtr->properties.allReadableCache);
		classPtr->properties.allReadableCache = NULL;
	    }
	    if (classPtr->properties.allWritableCache) {
		Tcl_DecrRefCount(classPtr->properties.allWritableCache);
		classPtr->properties.allWritableCache = NULL;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `Slot_Set(`, lines 106–214. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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
			    int objc, Tcl_Obj *const *objv);
static int		Slot_ResolveClass(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);

/*
 * Now define the slots used in declarations.
 */

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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `Slot_Set(`, lines 3099–3204. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    Tcl_Obj *list;

    /* Resolve all values */
    if (skip == objc) {
	list = Tcl_NewObj();
    } else {
	list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
	if (list == NULL) {
	    return TCL_ERROR;
	}
    }
    Tcl_IncrRefCount(list);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unimplemented --
 *
 *	Default implementation of the "Get" and "Set" slot accessors. Just
 *	returns an error; actual slots must override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unimplemented(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    TCL_UNUSED(Tcl_ObjectContext),
    TCL_UNUSED(int),
    TCL_UNUSED(Tcl_Obj *const *))
{
    Tcl_SetObjResult(interp, Tcl_NewStringObj("unimplemented", -1));
    OO_ERROR(interp, ABSTRACT_SLOT);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unknown --
 *
 *	Unknown method name handler for slots. Delegates to the default slot
 *	operation (--default-operation forwarded method) unless the first
 *	argument starts with a dash.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unknown(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    if (skip >= objc) {
	Tcl_Obj *args[2];
	args[0] = oPtr->fPtr->myName;
	args[1] = oPtr->fPtr->slotDefOpName;
	return TclOOPrivateObjectCmd(oPtr, interp, 2, args);
    } else if (TclGetString(objv[skip])[0] != '-') {
	Tcl_Obj **args = (Tcl_Obj **) TclStackAlloc(interp,
		sizeof(Tcl_Obj *) * (objc - skip + 2));
	args[0] = oPtr->fPtr->myName;
	args[1] = oPtr->fPtr->slotDefOpName;
	memcpy(args+2, objv+skip, sizeof(Tcl_Obj*) * (objc - skip));
	code = TclOOPrivateObjectCmd(oPtr, interp, objc - skip + 2, args);
	TclStackFree(interp, args);
	return code;
    }
    return TclNRObjectContextInvokeNext(interp, context, objc, objv, skip);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOSetSuperclasses --
 *
 *	Core of the "superclass" slot setter. Caller must AddRef() the objects
 *	holding the classes to set before calling this. The 'superclasses'
 *	argument must be allocated with Tcl_Alloc(); this function takes
 *	ownership.
 *
 * ----------------------------------------------------------------------
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 2745–2851. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
}

/* Call [$slot Resolve $item] for each of a whole list of items. */
static inline Tcl_Obj *
ResolveAll(
    Tcl_Interp *interp,
    Object *slot,
    int objc,
    Tcl_Obj *const *objv)
{
    Tcl_Obj **resolvedItems = (Tcl_Obj **) TclStackAlloc(interp,
	    sizeof(Tcl_Obj *) * objc);
    Tcl_Obj *resolvedList;
    int i;

    for (i = 0; i < objc; i++) {
	resolvedItems[i] = CallSlotResolve(interp, slot, objv[i]);
	if (resolvedItems[i] == NULL) {
	    for (int j = 0; j < i; j++) {
		Tcl_DecrRefCount(resolvedItems[j]);
	    }
	    TclStackFree(interp, (void *) resolvedItems);
	    return NULL;
	}
	Tcl_IncrRefCount(resolvedItems[i]);
	Tcl_ResetResult(interp);
    }
    resolvedList = Tcl_NewListObj(objc, resolvedItems);
    for (i = 0; i < objc; i++) {
	TclDecrRefCount(resolvedItems[i]);
    }
    TclStackFree(interp, (void *) resolvedItems);
    return resolvedList;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Append --
 *
 *	Implementation of the "-append" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Append(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    Tcl_Obj *resolved, *list;

    if (skip == objc) {
	return TCL_OK;
    }

    /* Resolve all values */
    resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    /* Get slot contents; store in list */
    list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

    /* Append */
    if (Tcl_IsShared(list)) {
	Tcl_Obj *dup = Tcl_DuplicateObj(list);
	Tcl_IncrRefCount(dup);
	Tcl_DecrRefCount(list);
	list = dup;
    }
    if (Tcl_ListObjAppendList(interp, list, resolved) != TCL_OK) {
	Tcl_DecrRefCount(list);
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_DecrRefCount(resolved);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_AppendNew --
 *
 *	Implementation of the "-appendifnew" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_AppendNew(
    TCL_UNUSED(void *),

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 2802–2909. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
	return TCL_OK;
    }

    /* Resolve all values */
    resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    /* Get slot contents; store in list */
    list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

    /* Append */
    if (Tcl_IsShared(list)) {
	Tcl_Obj *dup = Tcl_DuplicateObj(list);
	Tcl_IncrRefCount(dup);
	Tcl_DecrRefCount(list);
	list = dup;
    }
    if (Tcl_ListObjAppendList(interp, list, resolved) != TCL_OK) {
	Tcl_DecrRefCount(list);
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_DecrRefCount(resolved);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_AppendNew --
 *
 *	Implementation of the "-appendifnew" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_AppendNew(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code, isNew;
    Tcl_Obj *resolved, *list, **listv;
    Tcl_Size listc, i;
    Tcl_HashTable unique;

    if (skip == objc) {
	return TCL_OK;
    }

    /* Resolve all values */
    resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    /* Get slot contents; store in list */
    list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

    /* Prepare a set of items in the list to set */
    if (TclListObjGetElements(interp, list, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(list);
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_InitObjHashTable(&unique);
    for (i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&unique, listv[i], &isNew);
    }

    /* Append the new items if they're not already there */
    if (Tcl_IsShared(list)) {
	Tcl_Obj *dup = Tcl_DuplicateObj(list);
	Tcl_IncrRefCount(dup);
	Tcl_DecrRefCount(list);
	list = dup;
    }
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    for (i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&unique, listv[i], &isNew);
	if (isNew) {
	    Tcl_ListObjAppendElement(interp, list, listv[i]);
	}
    }
    Tcl_DecrRefCount(resolved);
    Tcl_DeleteHashTable(&unique);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 2864–2970. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
	return TCL_OK;
    }

    /* Resolve all values */
    resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    /* Get slot contents; store in list */
    list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

    /* Prepare a set of items in the list to set */
    if (TclListObjGetElements(interp, list, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(list);
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_InitObjHashTable(&unique);
    for (i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&unique, listv[i], &isNew);
    }

    /* Append the new items if they're not already there */
    if (Tcl_IsShared(list)) {
	Tcl_Obj *dup = Tcl_DuplicateObj(list);
	Tcl_IncrRefCount(dup);
	Tcl_DecrRefCount(list);
	list = dup;
    }
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    for (i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&unique, listv[i], &isNew);
	if (isNew) {
	    Tcl_ListObjAppendElement(interp, list, listv[i]);
	}
    }
    Tcl_DecrRefCount(resolved);
    Tcl_DeleteHashTable(&unique);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Clear --
 *
 *	Implementation of the "-clear" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Clear(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    Tcl_Obj *list;

    if (skip != objc) {
	Tcl_WrongNumArgs(interp, skip, objv, NULL);
	return TCL_ERROR;
    }
    list = Tcl_NewObj();
    Tcl_IncrRefCount(list);
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Prepend --
 *
 *	Implementation of the "-prepend" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Prepend(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    Tcl_Obj *list, *oldList;
    if (skip == objc) {
	return TCL_OK;
    }

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 2969–3075. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
	return TCL_OK;
    }

    /* Resolve all values */
    list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (list == NULL) {
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);

    /* Get slot contents and append to list */
    oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(list);
	return TCL_ERROR;
    }
    Tcl_ListObjAppendList(NULL, list, oldList);
    Tcl_ResetResult(interp);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Remove --
 *
 *	Implementation of the "-remove" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Remove(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code, isNew;
    Tcl_Size listc, i;
    Tcl_Obj *resolved, *oldList, *newList, **listv;
    Tcl_HashTable removeSet;

    if (skip == objc) {
	return TCL_OK;
    }

    /* Resolve all values */
    resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    /* Get slot contents; store in list */
    oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(oldList);
    Tcl_ResetResult(interp);

    /* Prepare a set of items in the list to remove */
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    Tcl_InitObjHashTable(&removeSet);
    for (i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&removeSet, listv[i], &isNew);
    }
    Tcl_DecrRefCount(resolved);

    /* Append the new items from the old items if they're not in the remove set */
    if (TclListObjGetElements(interp, oldList, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(oldList);
	Tcl_DeleteHashTable(&removeSet);
	return TCL_ERROR;
    }
    newList = Tcl_NewObj();
    for (i=0 ; i<listc; i++) {
	if (Tcl_FindHashEntry(&removeSet, listv[i]) == NULL) {
	    Tcl_ListObjAppendElement(NULL, newList, listv[i]);
	}
    }
    Tcl_DecrRefCount(oldList);
    Tcl_DeleteHashTable(&removeSet);

    /* Set slot contents */
    Tcl_IncrRefCount(newList);
    code = CallSlotSet(interp, oPtr, newList);
    Tcl_DecrRefCount(newList);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Resolve --
 *
 *	Default implementation of the "Resolve" slot accessor. Just returns
 *	its argument unchanged; particular slots may override.
 *
 * ----------------------------------------------------------------------
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 3018–3124. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
	return TCL_OK;
    }

    /* Resolve all values */
    resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    /* Get slot contents; store in list */
    oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(oldList);
    Tcl_ResetResult(interp);

    /* Prepare a set of items in the list to remove */
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    Tcl_InitObjHashTable(&removeSet);
    for (i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&removeSet, listv[i], &isNew);
    }
    Tcl_DecrRefCount(resolved);

    /* Append the new items from the old items if they're not in the remove set */
    if (TclListObjGetElements(interp, oldList, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(oldList);
	Tcl_DeleteHashTable(&removeSet);
	return TCL_ERROR;
    }
    newList = Tcl_NewObj();
    for (i=0 ; i<listc; i++) {
	if (Tcl_FindHashEntry(&removeSet, listv[i]) == NULL) {
	    Tcl_ListObjAppendElement(NULL, newList, listv[i]);
	}
    }
    Tcl_DecrRefCount(oldList);
    Tcl_DeleteHashTable(&removeSet);

    /* Set slot contents */
    Tcl_IncrRefCount(newList);
    code = CallSlotSet(interp, oPtr, newList);
    Tcl_DecrRefCount(newList);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Resolve --
 *
 *	Default implementation of the "Resolve" slot accessor. Just returns
 *	its argument unchanged; particular slots may override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Resolve(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    int skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip + 1 != objc) {
	Tcl_WrongNumArgs(interp, skip, objv, "list");
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, objv[objc - 1]);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Set --
 *
 *	Implementation of the "-set" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    Tcl_Obj *list;

    /* Resolve all values */
    if (skip == objc) {
	list = Tcl_NewObj();
    } else {
	list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
	if (list == NULL) {
	    return TCL_ERROR;
	}
    }
    Tcl_IncrRefCount(list);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `ResolveAll(`, lines 3114–3219. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    /* Resolve all values */
    if (skip == objc) {
	list = Tcl_NewObj();
    } else {
	list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
	if (list == NULL) {
	    return TCL_ERROR;
	}
    }
    Tcl_IncrRefCount(list);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unimplemented --
 *
 *	Default implementation of the "Get" and "Set" slot accessors. Just
 *	returns an error; actual slots must override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unimplemented(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    TCL_UNUSED(Tcl_ObjectContext),
    TCL_UNUSED(int),
    TCL_UNUSED(Tcl_Obj *const *))
{
    Tcl_SetObjResult(interp, Tcl_NewStringObj("unimplemented", -1));
    OO_ERROR(interp, ABSTRACT_SLOT);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unknown --
 *
 *	Unknown method name handler for slots. Delegates to the default slot
 *	operation (--default-operation forwarded method) unless the first
 *	argument starts with a dash.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unknown(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    if (skip >= objc) {
	Tcl_Obj *args[2];
	args[0] = oPtr->fPtr->myName;
	args[1] = oPtr->fPtr->slotDefOpName;
	return TclOOPrivateObjectCmd(oPtr, interp, 2, args);
    } else if (TclGetString(objv[skip])[0] != '-') {
	Tcl_Obj **args = (Tcl_Obj **) TclStackAlloc(interp,
		sizeof(Tcl_Obj *) * (objc - skip + 2));
	args[0] = oPtr->fPtr->myName;
	args[1] = oPtr->fPtr->slotDefOpName;
	memcpy(args+2, objv+skip, sizeof(Tcl_Obj*) * (objc - skip));
	code = TclOOPrivateObjectCmd(oPtr, interp, objc - skip + 2, args);
	TclStackFree(interp, args);
	return code;
    }
    return TclNRObjectContextInvokeNext(interp, context, objc, objv, skip);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOSetSuperclasses --
 *
 *	Core of the "superclass" slot setter. Caller must AddRef() the objects
 *	holding the classes to set before calling this. The 'superclasses'
 *	argument must be allocated with Tcl_Alloc(); this function takes
 *	ownership.
 *
 * ----------------------------------------------------------------------
 */
void
TclOOSetSuperclasses(
    Class *clsPtr,
    Tcl_Size superc,
    Class **superclasses)
{
    Tcl_Size i;
    Class *superPtr;

    if (clsPtr->superclasses.num != 0) {
	FOREACH(superPtr, clsPtr->superclasses) {
	    TclOORemoveFromSubclasses(clsPtr, superPtr);
	    TclOODecrRefCount(superPtr->thisPtr);
	}
	Tcl_Free(clsPtr->superclasses.list);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 2711–2818. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
}

/* Call [$slot Set $list] to set the list of contents of the slot. */
static inline int
CallSlotSet(
    Tcl_Interp *interp,
    Object *slot,
    Tcl_Obj *list)
{
    Tcl_Obj *setArgs[3];
    setArgs[0] = slot->fPtr->myName;
    setArgs[1] = slot->fPtr->slotSetName;
    setArgs[2] = list;
    return TclOOPrivateObjectCmd(slot, interp, 3, setArgs);
}

/* Call [$slot Resolve $item] to convert a slot item into canonical form. */
static inline Tcl_Obj *
CallSlotResolve(
    Tcl_Interp *interp,
    Object *slot,
    Tcl_Obj *item)
{
    Tcl_Obj *resolveArgs[3];
    int code;

    resolveArgs[0] = slot->fPtr->myName;
    resolveArgs[1] = slot->fPtr->slotResolveName;
    resolveArgs[2] = item;
    code = TclOOPrivateObjectCmd(slot, interp, 3, resolveArgs);
    if (code != TCL_OK) {
	return NULL;
    }
    return Tcl_GetObjResult(interp);
}

/* Call [$slot Resolve $item] for each of a whole list of items. */
static inline Tcl_Obj *
ResolveAll(
    Tcl_Interp *interp,
    Object *slot,
    int objc,
    Tcl_Obj *const *objv)
{
    Tcl_Obj **resolvedItems = (Tcl_Obj **) TclStackAlloc(interp,
	    sizeof(Tcl_Obj *) * objc);
    Tcl_Obj *resolvedList;
    int i;

    for (i = 0; i < objc; i++) {
	resolvedItems[i] = CallSlotResolve(interp, slot, objv[i]);
	if (resolvedItems[i] == NULL) {
	    for (int j = 0; j < i; j++) {
		Tcl_DecrRefCount(resolvedItems[j]);
	    }
	    TclStackFree(interp, (void *) resolvedItems);
	    return NULL;
	}
	Tcl_IncrRefCount(resolvedItems[i]);
	Tcl_ResetResult(interp);
    }
    resolvedList = Tcl_NewListObj(objc, resolvedItems);
    for (i = 0; i < objc; i++) {
	TclDecrRefCount(resolvedItems[i]);
    }
    TclStackFree(interp, (void *) resolvedItems);
    return resolvedList;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Append --
 *
 *	Implementation of the "-append" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Append(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    Tcl_Obj *resolved, *list;

    if (skip == objc) {
	return TCL_OK;
    }

    /* Resolve all values */
    resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    /* Get slot contents; store in list */
    list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 2831–2937. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    }
    Tcl_DecrRefCount(resolved);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_AppendNew --
 *
 *	Implementation of the "-appendifnew" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_AppendNew(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code, isNew;
    Tcl_Obj *resolved, *list, **listv;
    Tcl_Size listc, i;
    Tcl_HashTable unique;

    if (skip == objc) {
	return TCL_OK;
    }

    /* Resolve all values */
    resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    /* Get slot contents; store in list */
    list = CallSlotGet(interp, oPtr);
    if (list == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);
    Tcl_ResetResult(interp);

    /* Prepare a set of items in the list to set */
    if (TclListObjGetElements(interp, list, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(list);
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_InitObjHashTable(&unique);
    for (i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&unique, listv[i], &isNew);
    }

    /* Append the new items if they're not already there */
    if (Tcl_IsShared(list)) {
	Tcl_Obj *dup = Tcl_DuplicateObj(list);
	Tcl_IncrRefCount(dup);
	Tcl_DecrRefCount(list);
	list = dup;
    }
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    for (i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&unique, listv[i], &isNew);
	if (isNew) {
	    Tcl_ListObjAppendElement(interp, list, listv[i]);
	}
    }
    Tcl_DecrRefCount(resolved);
    Tcl_DeleteHashTable(&unique);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Clear --
 *
 *	Implementation of the "-clear" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Clear(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    Tcl_Obj *list;

    if (skip != objc) {

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 2907–3012. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    Tcl_DecrRefCount(resolved);
    Tcl_DeleteHashTable(&unique);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Clear --
 *
 *	Implementation of the "-clear" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Clear(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    Tcl_Obj *list;

    if (skip != objc) {
	Tcl_WrongNumArgs(interp, skip, objv, NULL);
	return TCL_ERROR;
    }
    list = Tcl_NewObj();
    Tcl_IncrRefCount(list);
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Prepend --
 *
 *	Implementation of the "-prepend" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Prepend(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    Tcl_Obj *list, *oldList;
    if (skip == objc) {
	return TCL_OK;
    }

    /* Resolve all values */
    list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (list == NULL) {
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);

    /* Get slot contents and append to list */
    oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(list);
	return TCL_ERROR;
    }
    Tcl_ListObjAppendList(NULL, list, oldList);
    Tcl_ResetResult(interp);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Remove --
 *
 *	Implementation of the "-remove" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Remove(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code, isNew;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 2939–3045. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
	return TCL_ERROR;
    }
    list = Tcl_NewObj();
    Tcl_IncrRefCount(list);
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Prepend --
 *
 *	Implementation of the "-prepend" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Prepend(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    Tcl_Obj *list, *oldList;
    if (skip == objc) {
	return TCL_OK;
    }

    /* Resolve all values */
    list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (list == NULL) {
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(list);

    /* Get slot contents and append to list */
    oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(list);
	return TCL_ERROR;
    }
    Tcl_ListObjAppendList(NULL, list, oldList);
    Tcl_ResetResult(interp);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Remove --
 *
 *	Implementation of the "-remove" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Remove(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code, isNew;
    Tcl_Size listc, i;
    Tcl_Obj *resolved, *oldList, *newList, **listv;
    Tcl_HashTable removeSet;

    if (skip == objc) {
	return TCL_OK;
    }

    /* Resolve all values */
    resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    /* Get slot contents; store in list */
    oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(oldList);
    Tcl_ResetResult(interp);

    /* Prepare a set of items in the list to remove */
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    Tcl_InitObjHashTable(&removeSet);
    for (i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&removeSet, listv[i], &isNew);
    }
    Tcl_DecrRefCount(resolved);

    /* Append the new items from the old items if they're not in the remove set */
    if (TclListObjGetElements(interp, oldList, &listc, &listv) != TCL_OK) {

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 2985–3091. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    Tcl_ListObjAppendList(NULL, list, oldList);
    Tcl_ResetResult(interp);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Remove --
 *
 *	Implementation of the "-remove" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Remove(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code, isNew;
    Tcl_Size listc, i;
    Tcl_Obj *resolved, *oldList, *newList, **listv;
    Tcl_HashTable removeSet;

    if (skip == objc) {
	return TCL_OK;
    }

    /* Resolve all values */
    resolved = ResolveAll(interp, oPtr, objc - skip, objv + skip);
    if (resolved == NULL) {
	return TCL_ERROR;
    }

    /* Get slot contents; store in list */
    oldList = CallSlotGet(interp, oPtr);
    if (oldList == NULL) {
	Tcl_DecrRefCount(resolved);
	return TCL_ERROR;
    }
    Tcl_IncrRefCount(oldList);
    Tcl_ResetResult(interp);

    /* Prepare a set of items in the list to remove */
    TclListObjGetElements(NULL, resolved, &listc, &listv);
    Tcl_InitObjHashTable(&removeSet);
    for (i=0 ; i<listc; i++) {
	Tcl_CreateHashEntry(&removeSet, listv[i], &isNew);
    }
    Tcl_DecrRefCount(resolved);

    /* Append the new items from the old items if they're not in the remove set */
    if (TclListObjGetElements(interp, oldList, &listc, &listv) != TCL_OK) {
	Tcl_DecrRefCount(oldList);
	Tcl_DeleteHashTable(&removeSet);
	return TCL_ERROR;
    }
    newList = Tcl_NewObj();
    for (i=0 ; i<listc; i++) {
	if (Tcl_FindHashEntry(&removeSet, listv[i]) == NULL) {
	    Tcl_ListObjAppendElement(NULL, newList, listv[i]);
	}
    }
    Tcl_DecrRefCount(oldList);
    Tcl_DeleteHashTable(&removeSet);

    /* Set slot contents */
    Tcl_IncrRefCount(newList);
    code = CallSlotSet(interp, oPtr, newList);
    Tcl_DecrRefCount(newList);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Resolve --
 *
 *	Default implementation of the "Resolve" slot accessor. Just returns
 *	its argument unchanged; particular slots may override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Resolve(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    int skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip + 1 != objc) {
	Tcl_WrongNumArgs(interp, skip, objv, "list");
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, objv[objc - 1]);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 3057–3161. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    Tcl_DeleteHashTable(&removeSet);

    /* Set slot contents */
    Tcl_IncrRefCount(newList);
    code = CallSlotSet(interp, oPtr, newList);
    Tcl_DecrRefCount(newList);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Resolve --
 *
 *	Default implementation of the "Resolve" slot accessor. Just returns
 *	its argument unchanged; particular slots may override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Resolve(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    int skip = Tcl_ObjectContextSkippedArgs(context);
    if (skip + 1 != objc) {
	Tcl_WrongNumArgs(interp, skip, objv, "list");
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp, objv[objc - 1]);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Set --
 *
 *	Implementation of the "-set" slot operation.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    Tcl_Obj *list;

    /* Resolve all values */
    if (skip == objc) {
	list = Tcl_NewObj();
    } else {
	list = ResolveAll(interp, oPtr, objc - skip, objv + skip);
	if (list == NULL) {
	    return TCL_ERROR;
	}
    }
    Tcl_IncrRefCount(list);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unimplemented --
 *
 *	Default implementation of the "Get" and "Set" slot accessors. Just
 *	returns an error; actual slots must override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unimplemented(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    TCL_UNUSED(Tcl_ObjectContext),
    TCL_UNUSED(int),
    TCL_UNUSED(Tcl_Obj *const *))
{
    Tcl_SetObjResult(interp, Tcl_NewStringObj("unimplemented", -1));
    OO_ERROR(interp, ABSTRACT_SLOT);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unknown --
 *
 *	Unknown method name handler for slots. Delegates to the default slot
 *	operation (--default-operation forwarded method) unless the first
 *	argument starts with a dash.

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `CallSlotSet(`, lines 3122–3227. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    }
    Tcl_IncrRefCount(list);

    /* Set slot contents */
    code = CallSlotSet(interp, oPtr, list);
    Tcl_DecrRefCount(list);
    return code;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unimplemented --
 *
 *	Default implementation of the "Get" and "Set" slot accessors. Just
 *	returns an error; actual slots must override.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unimplemented(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    TCL_UNUSED(Tcl_ObjectContext),
    TCL_UNUSED(int),
    TCL_UNUSED(Tcl_Obj *const *))
{
    Tcl_SetObjResult(interp, Tcl_NewStringObj("unimplemented", -1));
    OO_ERROR(interp, ABSTRACT_SLOT);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_Unknown --
 *
 *	Unknown method name handler for slots. Delegates to the default slot
 *	operation (--default-operation forwarded method) unless the first
 *	argument starts with a dash.
 *
 * ----------------------------------------------------------------------
 */
static int
Slot_Unknown(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    int skip = Tcl_ObjectContextSkippedArgs(context), code;
    if (skip >= objc) {
	Tcl_Obj *args[2];
	args[0] = oPtr->fPtr->myName;
	args[1] = oPtr->fPtr->slotDefOpName;
	return TclOOPrivateObjectCmd(oPtr, interp, 2, args);
    } else if (TclGetString(objv[skip])[0] != '-') {
	Tcl_Obj **args = (Tcl_Obj **) TclStackAlloc(interp,
		sizeof(Tcl_Obj *) * (objc - skip + 2));
	args[0] = oPtr->fPtr->myName;
	args[1] = oPtr->fPtr->slotDefOpName;
	memcpy(args+2, objv+skip, sizeof(Tcl_Obj*) * (objc - skip));
	code = TclOOPrivateObjectCmd(oPtr, interp, objc - skip + 2, args);
	TclStackFree(interp, args);
	return code;
    }
    return TclNRObjectContextInvokeNext(interp, context, objc, objv, skip);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOSetSuperclasses --
 *
 *	Core of the "superclass" slot setter. Caller must AddRef() the objects
 *	holding the classes to set before calling this. The 'superclasses'
 *	argument must be allocated with Tcl_Alloc(); this function takes
 *	ownership.
 *
 * ----------------------------------------------------------------------
 */
void
TclOOSetSuperclasses(
    Class *clsPtr,
    Tcl_Size superc,
    Class **superclasses)
{
    Tcl_Size i;
    Class *superPtr;

    if (clsPtr->superclasses.num != 0) {
	FOREACH(superPtr, clsPtr->superclasses) {
	    TclOORemoveFromSubclasses(clsPtr, superPtr);
	    TclOODecrRefCount(superPtr->thisPtr);
	}
	Tcl_Free(clsPtr->superclasses.list);
    }
    clsPtr->superclasses.list = superclasses;
    clsPtr->superclasses.num = superc;
    FOREACH(superPtr, clsPtr->superclasses) {
	TclOOAddToSubclasses(clsPtr, superPtr);
    }
}


```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `Configurable_ClassReadableProps_Set(`, lines 160–266. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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
			    int objc, Tcl_Obj *const *objv);
static int		Slot_ResolveClass(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);

/*
 * Now define the slots used in declarations.
 */

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

/*
 * How to build the in-namespace name of a private variable. This is a pattern
 * used with Tcl_ObjPrintf().
 */

#define PRIVATE_VARIABLE_PATTERN "%d : %s"

/*
 * ----------------------------------------------------------------------
 *
 * IsPrivateDefine --
 *
 *	Extracts whether the current context is handling private definitions.
 *
 * ----------------------------------------------------------------------
 */

static inline int
IsPrivateDefine(
    Tcl_Interp *interp)
{
    Interp *iPtr = (Interp *) interp;

    if (!iPtr->varFramePtr) {
	return 0;
    }
    return iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME;
}

/*
 * ----------------------------------------------------------------------
 *
 * BumpGlobalEpoch --
 *
 *	Utility that ensures that call chains that are invalid will get thrown
 *	away at an appropriate time. Note that exactly which epoch gets
 *	advanced will depend on exactly what the class is tangled up in; in

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `Configurable_ClassReadableProps_Set(`, lines 4014–4121. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    return TCL_OK;
}

static int
Configurable_ClassReadableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (TclListObjGetElements(interp, objv[0], &varc, &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallReadableProperties(&clsPtr->properties, varc, varv);
    BumpGlobalEpoch(interp, clsPtr);
    return TCL_OK;
}

static int
Configurable_ObjectReadableProps_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, TclOOGetPropertyList(&oPtr->properties.readable));
    return TCL_OK;
}

static int
Configurable_ObjectReadableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &varc,
	    &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallReadableProperties(&oPtr->properties, varc, varv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Configurable_ClassWritableProps_Get, Configurable_ClassWritableProps_Set,
 * Configurable_ObjectWritableProps_Get, Configurable_ObjectWritableProps_Set --
 *
 *	Implementations of the "writableproperties" slot accessors for classes
 *	and instances.
 *
 * ----------------------------------------------------------------------
 */

static int
Configurable_ClassWritableProps_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `Configurable_ObjectReadableProps_Set(`, lines 172–278. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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
			    int objc, Tcl_Obj *const *objv);
static int		Slot_ResolveClass(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);

/*
 * Now define the slots used in declarations.
 */

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

/*
 * How to build the in-namespace name of a private variable. This is a pattern
 * used with Tcl_ObjPrintf().
 */

#define PRIVATE_VARIABLE_PATTERN "%d : %s"

/*
 * ----------------------------------------------------------------------
 *
 * IsPrivateDefine --
 *
 *	Extracts whether the current context is handling private definitions.
 *
 * ----------------------------------------------------------------------
 */

static inline int
IsPrivateDefine(
    Tcl_Interp *interp)
{
    Interp *iPtr = (Interp *) interp;

    if (!iPtr->varFramePtr) {
	return 0;
    }
    return iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME;
}

/*
 * ----------------------------------------------------------------------
 *
 * BumpGlobalEpoch --
 *
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `Configurable_ObjectReadableProps_Set(`, lines 4066–4173. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    return TCL_OK;
}

static int
Configurable_ObjectReadableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &varc,
	    &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallReadableProperties(&oPtr->properties, varc, varv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Configurable_ClassWritableProps_Get, Configurable_ClassWritableProps_Set,
 * Configurable_ObjectWritableProps_Get, Configurable_ObjectWritableProps_Set --
 *
 *	Implementations of the "writableproperties" slot accessors for classes
 *	and instances.
 *
 * ----------------------------------------------------------------------
 */

static int
Configurable_ClassWritableProps_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, TclOOGetPropertyList(&clsPtr->properties.writable));
    return TCL_OK;
}

static int
Configurable_ClassWritableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"propertyList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (TclListObjGetElements(interp, objv[0], &varc, &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallWritableProperties(&clsPtr->properties, varc, varv);
    BumpGlobalEpoch(interp, clsPtr);
    return TCL_OK;
}

static int
Configurable_ObjectWritableProps_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);

    if (oPtr == NULL) {

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `Configurable_ClassWritableProps_Set(`, lines 166–272. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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
			    int objc, Tcl_Obj *const *objv);
static int		Slot_ResolveClass(void *clientData,
			    Tcl_Interp *interp, Tcl_ObjectContext context,
			    int objc, Tcl_Obj *const *objv);

/*
 * Now define the slots used in declarations.
 */

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

/*
 * How to build the in-namespace name of a private variable. This is a pattern
 * used with Tcl_ObjPrintf().
 */

#define PRIVATE_VARIABLE_PATTERN "%d : %s"

/*
 * ----------------------------------------------------------------------
 *
 * IsPrivateDefine --
 *
 *	Extracts whether the current context is handling private definitions.
 *
 * ----------------------------------------------------------------------
 */

static inline int
IsPrivateDefine(
    Tcl_Interp *interp)
{
    Interp *iPtr = (Interp *) interp;

    if (!iPtr->varFramePtr) {
	return 0;
    }
    return iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME;
}

/*
 * ----------------------------------------------------------------------
 *
 * BumpGlobalEpoch --
 *
 *	Utility that ensures that call chains that are invalid will get thrown
 *	away at an appropriate time. Note that exactly which epoch gets
 *	advanced will depend on exactly what the class is tangled up in; in
 *	the worst case, the simplest option is to advance the global epoch,
 *	causing *everything* to be thrown away on next usage.
 *
 * ----------------------------------------------------------------------
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `Configurable_ClassWritableProps_Set(`, lines 4130–4237. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    return TCL_OK;
}

static int
Configurable_ClassWritableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"propertyList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (TclListObjGetElements(interp, objv[0], &varc, &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallWritableProperties(&clsPtr->properties, varc, varv);
    BumpGlobalEpoch(interp, clsPtr);
    return TCL_OK;
}

static int
Configurable_ObjectWritableProps_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, TclOOGetPropertyList(&oPtr->properties.writable));
    return TCL_OK;
}

static int
Configurable_ObjectWritableProps_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size varc;
    Tcl_Obj **varv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"propertyList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &varc,
	    &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOInstallWritableProperties(&oPtr->properties, varc, varv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOORegisterProperty, TclOORegisterInstanceProperty --
 *
 *	Helpers to add or remove a name from the property slots of a class or
 *	instance.
 *
 * BuildPropertyList --
 *
 *	Helper for the helpers. Scans a property list and does the filtering
 *	or adding of the property to add or remove
 *
 * ----------------------------------------------------------------------
 */

static int
BuildPropertyList(
    PropertyList *propsList,	/* Property list to scan. */
    Tcl_Obj *propName,		/* Property to add/remove. */
    int addingProp,		/* True if we're adding, false if removing. */
    Tcl_Obj *listObj)		/* The list of property names we're building */
{

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `configuresupport::readableproperties`, lines 197–303. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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

/*
 * How to build the in-namespace name of a private variable. This is a pattern
 * used with Tcl_ObjPrintf().
 */

#define PRIVATE_VARIABLE_PATTERN "%d : %s"

/*
 * ----------------------------------------------------------------------
 *
 * IsPrivateDefine --
 *
 *	Extracts whether the current context is handling private definitions.
 *
 * ----------------------------------------------------------------------
 */

static inline int
IsPrivateDefine(
    Tcl_Interp *interp)
{
    Interp *iPtr = (Interp *) interp;

    if (!iPtr->varFramePtr) {
	return 0;
    }
    return iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME;
}

/*
 * ----------------------------------------------------------------------
 *
 * BumpGlobalEpoch --
 *
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

	    /*
	     * Invalidate the property caches directly.
	     */

	    if (classPtr->properties.allReadableCache) {
		Tcl_DecrRefCount(classPtr->properties.allReadableCache);
		classPtr->properties.allReadableCache = NULL;
	    }
	    if (classPtr->properties.allWritableCache) {
		Tcl_DecrRefCount(classPtr->properties.allWritableCache);
		classPtr->properties.allWritableCache = NULL;

```

## Consumer bindings

- [runtime/rust/src/cmd_oo/original_property_retirement_tests.rs](../../../../runtime/rust/src/cmd_oo/original_property_retirement_tests.rs), `cmd_oo::original_property_retirement_tests::mandatory_property_retirement_releases_both_caches_before_original_members` (linked): Finite source-defined control: mandatory property retirement releases both caches before original members. Release order is inspected C91 source, not a measured teardown callback tape. Direct mandatory cleanup retains its own original operation before callbacks. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_oo/native_properties/original_order_tests.rs](../../../../runtime/rust/src/cmd_oo/native_properties/original_order_tests.rs), `cmd_oo::native_properties::original_order_tests::original_property_replacement_matches_incoming_hash_and_cache_reference_stages` (linked): Finite source-defined control: original property replacement matches incoming hash and cache reference stages. Only matched C9.1 worker, held-cache and raw List stages; presentation is separate. Actual info issued the result role; the explicit caller hold is independent. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

## Evidence

- [runtime/rust/tests/data/native_property_slot_reference_order_original](../../../../runtime/rust/tests/data/native_property_slot_reference_order_original)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [runtime/rust/tests/data/native_method_cache_active_call_lifetime_original/original-inputs](../../../../runtime/rust/tests/data/native_method_cache_active_call_lifetime_original/original-inputs)
- [runtime/rust/tests/data/native_method_cache_active_call_lifetime_original/506](../../../../runtime/rust/tests/data/native_method_cache_active_call_lifetime_original/506)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-test-support/tests/data/native_resolution_reference_builds](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds)
