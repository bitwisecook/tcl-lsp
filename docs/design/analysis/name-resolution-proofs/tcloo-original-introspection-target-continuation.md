# naming.tcloo.original-introspection-target-continuation

Kind: `implementation-contract`

## Problem statement

A Class facet can be replaced while its Object and operation context survive. Later report getters can also destroy or recreate an already selected receiver or instance namespace. Reselecting by display or treating a reached withdrawal as an initial missing receiver can report from the wrong owner.

## Question

Do Class allocation and introspection continuations retain the original Object, independently issued Class facet and selected namespace ledger across later operand getters, while initial missing isa receivers remain false without a secondary getter and reached owner withdrawal stops later report access?

## Conclusion

The current source/API owner distinguishes software Object creation from Class facet birth, retains the original receiver before report operands, and joins selected instance namespace access to the actual Object/token ledger. Initial missing isa and withdrawal of an already selected receiver remain separate. Six linked finite software controls have no attached matching owned assertion outcome. Sixteen exact source joins inspect C9.1 allocation/context/report order plus C8.6/C9.0 initial-missing isa order; they establish no callback, safe pointer/payload lifetime or physical parity. All seven external executions are not tested.

## Scope

Actual Runtime Class birth/copy and retained Class/Object receiver continuations for the described class reports, shared forward/definition/methodtype/call reports, object relation/method/variable/property reports and initial-missing isa behavior. The original source windows are inspected facts only; Host/ABA/destruction/rebirth controls remain software definitions. Aggregate method report headers, Method name/hash/refcounts, active chain/payload rendering, definition getter/effect order, Class variables and Native frame/source admission require independent purposes. Original command-object, token and import-lineage resolution uses its independent checked resolver. The retained target continuation grants no lookup or token purpose.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl8.4 not tested for this source/API question.

No original tcl8.4 process tests this introspection target continuation contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### tcl8.5

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl8.5 not tested for this source/API question.

No original tcl8.5 process tests this introspection target continuation contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### tcl8.6

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl8.6 not tested for this source/API question.

No original tcl8.6 process tests this introspection target continuation contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### tcl9.0

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl9.0 not tested for this source/API question.

No original tcl9.0 process tests this introspection target continuation contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### tcl9.1

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl9.1 not tested for this source/API question.

No original tcl9.1 process tests this introspection target continuation contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### jim

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: jim not tested for this source/API question.

No original jim process tests this introspection target continuation contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### bigip

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: bigip not tested for this source/API question.

No original bigip process tests this introspection target continuation contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

## Source inspection

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `TclOOAllocClass`, lines 1890–1932. Retained reference `source-window-0-0`.

```text
TclOOAllocClass(
    Tcl_Interp *interp,		/* Interpreter within which to allocate the
				 * class. */
    Object *useThisObj)		/* Object that is to act as the class
				 * representation. */
{
    Foundation *fPtr = GetFoundation(interp);
    Class *clsPtr = (Class *) Tcl_Alloc(sizeof(Class));

    memset(clsPtr, 0, sizeof(Class));
    clsPtr->thisPtr = useThisObj;

    /*
     * Configure the namespace path for the class's object.
     */

    InitClassPath(interp, clsPtr);

    /*
     * Classes are subclasses of oo::object, i.e. the objects they create are
     * objects.
     */

    clsPtr->superclasses.num = 1;
    clsPtr->superclasses.list = (Class **) Tcl_Alloc(sizeof(Class *));
    clsPtr->superclasses.list[0] = fPtr->objectCls;
    AddRef(fPtr->objectCls->thisPtr);

    /*
     * Finish connecting the class structure to the object structure.
     */

    clsPtr->thisPtr->classPtr = clsPtr;

    /*
     * That's the complicated bit. Now fill in the rest of the non-zero/NULL
     * fields.
     */

    Tcl_InitObjHashTable(&clsPtr->classMethods);
    return clsPtr;
}


```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext / TclOOGetClassDefineCmdContext`, lines 994–1034. Retained reference `source-window-0-1`.

```text
TclOOGetDefineCmdContext(
    Tcl_Interp *interp)
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Object object;

    if ((iPtr->varFramePtr == NULL)
	    || (iPtr->varFramePtr->isProcCallFrame != FRAME_IS_OO_DEFINE
	    && iPtr->varFramePtr->isProcCallFrame != PRIVATE_FRAME)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"this command may only be called from within the context of"
		" an ::oo::define or ::oo::objdefine command",
		TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return NULL;
    }
    object = (Tcl_Object) iPtr->varFramePtr->clientData;
    if (Tcl_ObjectDeleted(object)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"this command cannot be called when the object has been"
		" deleted", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return NULL;
    }
    return object;
}

Class *
TclOOGetClassDefineCmdContext(
    Tcl_Interp *interp)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return NULL;
    }
    if (!oPtr->classPtr) {
	(void) ReportAbuse(interp);
	return NULL;
    }
    return oPtr->classPtr;
}

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOODefineClassObjCmd`, lines 1704–1803. Retained reference `source-window-0-2`.

```text
TclOODefineClassObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Class *clsPtr;
    Foundation *fPtr = TclOOGetFoundation(interp);

    /*
     * Parse the context to get the object to operate on.
     */

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (oPtr->flags & ROOT_OBJECT) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"may not modify the class of the root object class",
		TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }
    if (oPtr->flags & ROOT_CLASS) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"may not modify the class of the class of classes",
		TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    /*
     * Parse the argument to get the class to set the object's class to.
     */

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "className");
	return TCL_ERROR;
    }
    clsPtr = GetClassInOuterContext(interp, objv[1],
	    "the class of an object must be a class");
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }
    if (oPtr == clsPtr->thisPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"may not change classes into an instance of themselves",
		TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    /*
     * Set the object's class.
     */

    bool wasClass = (oPtr->classPtr != NULL);
    bool willBeClass = TclOOIsReachable(fPtr->classCls, clsPtr);

    if (oPtr->selfCls != clsPtr) {
	TclOORemoveFromInstances(oPtr, oPtr->selfCls);
	TclOODecrRefCount(oPtr->selfCls->thisPtr);
	oPtr->selfCls = clsPtr;
	AddRef(oPtr->selfCls->thisPtr);
	TclOOAddToInstances(oPtr, oPtr->selfCls);

	/*
	 * Create or delete the class guts if necessary.
	 */

	if (wasClass && !willBeClass) {
	    /*
	     * This is the most global of all epochs. Bump it! No cache can be
	     * trusted!
	     */

	    TclOORemoveFromMixins(oPtr->classPtr, oPtr);
	    oPtr->fPtr->epoch++;
	    oPtr->flags |= DONT_DELETE;
	    TclOODeleteDescendants(interp, oPtr);
	    oPtr->flags &= ~DONT_DELETE;
	    TclOOReleaseClassContents(interp, oPtr);
	    Tcl_Free(oPtr->classPtr);
	    oPtr->classPtr = NULL;
	} else if (!wasClass && willBeClass) {
	    TclOOAllocClass(interp, oPtr);
	}

	if (oPtr->classPtr != NULL) {
	    BumpGlobalEpoch(interp, oPtr->classPtr);
	} else {
	    BumpInstanceEpoch(oPtr);
	}
    }
    return TCL_OK;
}

/*

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoClassDefnNsCmd`, lines 1152–1188. Retained reference `source-window-0-3`.

```text
InfoClassDefnNsCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    static const char *const kindList[] = {
	"-class",
	"-instance",
	NULL
    };
    int kind = 0;
    Tcl_Obj *nsNamePtr;
    Class *clsPtr;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "className ?kind?");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc == 3 && Tcl_GetIndexFromObj(interp, objv[2], kindList, "kind", 0,
	    &kind) != TCL_OK) {
	return TCL_ERROR;
    }

    if (kind) {			// -instance
	nsNamePtr = clsPtr->objDefinitionNs;
    } else {			// -class
	nsNamePtr = clsPtr->clsDefinitionNs;
    }
    if (nsNamePtr) {
	Tcl_SetObjResult(interp, nsNamePtr);
    }
    return TCL_OK;

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `TclOOGetClassFromObj`, lines 177–201. Retained reference `source-window-1-0`.

```text
TclOOGetClassFromObj(
    Tcl_Interp *interp,
    Tcl_Obj *objPtr)
{
    Object *oPtr = (Object *) Tcl_GetObjectFromObj(interp, objPtr);

    if (oPtr == NULL) {
	return NULL;
    }
    if (oPtr->classPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"\"%s\" is not a class", TclGetString(objPtr)));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CLASS",
		TclGetString(objPtr), (char *)NULL);
	return NULL;
    }
    return oPtr->classPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * GetClassMethodFromObj --
 *
 *	Helper for looking up a class-defined method.

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `GetClassMethodFromObj / GetInstanceMethodFromObj`, lines 202–278. Retained reference `source-window-1-1`.

```text
 *
 * ----------------------------------------------------------------------
 */
static inline Method *
GetClassMethodFromObj(
    Tcl_Interp *interp,
    Class *clsPtr,
    Tcl_Obj *methodName)
{
    Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&clsPtr->classMethods, methodName);
    Method *mPtr;

    if (hPtr == NULL) {
	goto unknownMethod;
    }
    mPtr = (Method *)Tcl_GetHashValue(hPtr);
    if (mPtr->type2Ptr == NULL) {
	/*
	 * Special entry for visibility control: pretend the method doesnt
	 * exist.
	 */

	goto unknownMethod;
    }
    return mPtr;

  unknownMethod:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "unknown method \"%s\"", TclGetString(methodName)));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "METHOD",
	    TclGetString(methodName), (char *)NULL);
    return NULL;
}

/*
 * ----------------------------------------------------------------------
 *
 * GetInstanceMethodFromObj --
 *
 *	Helper for looking up an object-instance-defined method.
 *
 * ----------------------------------------------------------------------
 */
static inline Method *
GetInstanceMethodFromObj(
    Tcl_Interp *interp,
    Object *oPtr,
    Tcl_Obj *methodName)
{
    Tcl_HashEntry *hPtr;
    Method *mPtr;

    if (!oPtr->methodsPtr) {
	goto unknownMethod;
    }
    hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodName);
    if (hPtr == NULL) {
	goto unknownMethod;
    }
    mPtr = (Method *)Tcl_GetHashValue(hPtr);
    if (mPtr->type2Ptr == NULL) {
	/*
	 * Special entry for visibility control: pretend the method doesnt
	 * exist.
	 */

	goto unknownMethod;
    }
    return mPtr;

  unknownMethod:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "unknown method \"%s\"", TclGetString(methodName)));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "METHOD",
	    TclGetString(methodName), (char *)NULL);
    return NULL;
}

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoObjectClassCmd`, lines 340–383. Retained reference `source-window-1-2`.

```text
static int
InfoObjectClassCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName ?className?");
	return TCL_ERROR;
    }

    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    if (objc == 2) {
	Tcl_SetObjResult(interp,
		TclOOObjectName(interp, oPtr->selfCls->thisPtr));
	return TCL_OK;
    } else {
	Class *mixinPtr, *o2clsPtr;
	Tcl_Size i;

	o2clsPtr = TclOOGetClassFromObj(interp, objv[2]);
	if (o2clsPtr == NULL) {
	    return TCL_ERROR;
	}

	FOREACH(mixinPtr, oPtr->mixins) {
	    if (!mixinPtr) {
		continue;
	    }
	    if (TclOOIsReachable(o2clsPtr, mixinPtr)) {
		Tcl_SetObjResult(interp, Tcl_NewBooleanObj(1));
		return TCL_OK;
	    }
	}
	Tcl_SetObjResult(interp, Tcl_NewBooleanObj(
		TclOOIsReachable(o2clsPtr, oPtr->selfCls)));
	return TCL_OK;

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoObjectIsACmd`, lines 535–645. Retained reference `source-window-1-3`.

```text
InfoObjectIsACmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    static const char *const categories[] = {
	"class", "metaclass", "mixin", "object", "typeof", NULL
    };
    enum IsACats {
	IsClass, IsMetaclass, IsMixin, IsObject, IsType
    } idx;
    Object *oPtr, *o2Ptr;
    int result = 0;
    Tcl_Size i;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "category objName ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], categories, "category", 0,
	    &idx) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * Now we know what test we are doing, we can check we've got the right
     * number of arguments.
     */

    switch (idx) {
    case IsObject:
    case IsClass:
    case IsMetaclass:
	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "objName");
	    return TCL_ERROR;
	}
	break;
    case IsMixin:
    case IsType:
	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "objName className");
	    return TCL_ERROR;
	}
	break;
    default:
	TCL_UNREACHABLE();
    }

    /*
     * Perform the check. Note that we can guarantee that we will not fail
     * from here on; "failures" result in a false-TCL_OK result.
     */

    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[2]);
    if (oPtr == NULL) {
	goto failPrecondition;
    }

    switch (idx) {
    case IsObject:
	result = 1;
	break;
    case IsClass:
	result = (oPtr->classPtr != NULL);
	break;
    case IsMetaclass:
	if (oPtr->classPtr != NULL) {
	    result = TclOOIsReachable(TclOOGetFoundation(interp)->classCls,
		    oPtr->classPtr);
	}
	break;
    case IsMixin:
	o2Ptr = (Object *) Tcl_GetObjectFromObj(interp, objv[3]);
	if (o2Ptr == NULL) {
	    goto failPrecondition;
	}
	if (o2Ptr->classPtr != NULL) {
	    Class *mixinPtr;

	    FOREACH(mixinPtr, oPtr->mixins) {
		if (!mixinPtr) {
		    continue;
		}
		if (TclOOIsReachable(o2Ptr->classPtr, mixinPtr)) {
		    result = 1;
		    break;
		}
	    }
	}
	break;
    case IsType:
	o2Ptr = (Object *) Tcl_GetObjectFromObj(interp, objv[3]);
	if (o2Ptr == NULL) {
	    goto failPrecondition;
	}
	if (o2Ptr->classPtr != NULL) {
	    result = TclOOIsReachable(o2Ptr->classPtr, oPtr->selfCls);
	}
	break;
    default:
	TCL_UNREACHABLE();
    }
    Tcl_SetObjResult(interp, Tcl_NewBooleanObj(result));
    return TCL_OK;

  failPrecondition:
    Tcl_ResetResult(interp);
    Tcl_SetObjResult(interp, Tcl_NewBooleanObj(0));
    return TCL_OK;

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoObjectMethodsCmd`, lines 659–750. Retained reference `source-window-1-4`.

```text
InfoObjectMethodsCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    static const char *const options[] = {
	"-all", "-localprivate", "-private", "-scope", NULL
    };
    enum Options {
	OPT_ALL, OPT_LOCALPRIVATE, OPT_PRIVATE, OPT_SCOPE
    } idx;
    static const char *const scopes[] = {
	"private", "public", "unexported"
    };
    enum Scopes {
	SCOPE_PRIVATE, SCOPE_PUBLIC, SCOPE_UNEXPORTED,
	SCOPE_LOCALPRIVATE,
	SCOPE_DEFAULT = -1
    };
    Object *oPtr;
    int flag = PUBLIC_METHOD, scope = SCOPE_DEFAULT;
    bool recurse = false;
    FOREACH_HASH_DECLS;
    Tcl_Obj *namePtr, *resultObj;
    Method *mPtr;

    /*
     * Parse arguments.
     */

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName ?-option value ...?");
	return TCL_ERROR;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc != 2) {
	for (Tcl_Size i=2 ; i<objc ; i++) {
	    if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0,
		    &idx) != TCL_OK) {
		return TCL_ERROR;
	    }
	    switch (idx) {
	    case OPT_ALL:
		recurse = true;
		break;
	    case OPT_LOCALPRIVATE:
		flag = PRIVATE_METHOD;
		break;
	    case OPT_PRIVATE:
		flag = 0;
		break;
	    case OPT_SCOPE:
		if (++i >= objc) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "missing option for -scope"));
		    Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING",
			    (char *)NULL);
		    return TCL_ERROR;
		}
		if (Tcl_GetIndexFromObj(interp, objv[i], scopes, "scope", 0,
			&scope) != TCL_OK) {
		    return TCL_ERROR;
		}
		break;
	    default:
		TCL_UNREACHABLE();
	    }
	}
    }
    if (scope != SCOPE_DEFAULT) {
	recurse = false;
	switch (scope) {
	case SCOPE_PRIVATE:
	    flag = TRUE_PRIVATE_METHOD;
	    break;
	case SCOPE_PUBLIC:
	    flag = PUBLIC_METHOD;
	    break;
	case SCOPE_LOCALPRIVATE:
	    flag = PRIVATE_METHOD;
	    break;
	case SCOPE_UNEXPORTED:
	    flag = 0;
	    break;
	}
    }

    /*

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoObjectVarsCmd`, lines 1003–1060. Retained reference `source-window-1-5`.

```text
InfoObjectVarsCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    const char *pattern = NULL;
    FOREACH_HASH_DECLS;
    VarInHash *vihPtr;
    Tcl_Obj *nameObj, *resultObj;

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName ?pattern?");
	return TCL_ERROR;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc == 3) {
	pattern = TclGetString(objv[2]);
    }
    TclNewObj(resultObj);

    /*
     * Extract the information we need from the object's namespace's table of
     * variables. Note that this involves horrific knowledge of the guts of
     * tclVar.c, so we can't leverage our hash-iteration macros properly.
     */

    FOREACH_HASH_VALUE(vihPtr,
	    &((Namespace *) oPtr->namespacePtr)->varTable.table) {
	nameObj = vihPtr->entry.key.objPtr;

	if (TclIsVarUndefined(&vihPtr->var)
		|| !TclIsVarNamespaceVar(&vihPtr->var)) {
	    continue;
	}
	if (pattern != NULL
		&& !Tcl_StringMatch(TclGetString(nameObj), pattern)) {
	    continue;
	}
	Tcl_ListObjAppendElement(NULL, resultObj, nameObj);
    }

    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * InfoClassConstrCmd --
 *
 *	Implements [info class constructor $clsName]
 *
 * ----------------------------------------------------------------------

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoClassDefnCmd`, lines 1107–1138. Retained reference `source-window-1-6`.

```text
InfoClassDefnCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Method *mPtr;
    Proc *procPtr;
    Tcl_Obj *resultObjs[2];
    Class *clsPtr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "className methodName");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }
    mPtr = GetClassMethodFromObj(interp, clsPtr, objv[2]);
    if (mPtr == NULL) {
	return TCL_ERROR;
    }
    procPtr = GetProcFromMethod(interp, mPtr);
    if (procPtr == NULL) {
	return TCL_ERROR;
    }

    resultObjs[0] = DescribeMethodArgs(procPtr);
    resultObjs[1] = TclOOGetMethodBody(mPtr);
    Tcl_SetObjResult(interp, Tcl_NewListObj(2, resultObjs));
    return TCL_OK;

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoClassForwardCmd`, lines 1281–1309. Retained reference `source-window-1-7`.

```text
InfoClassForwardCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Method *mPtr;
    Tcl_Obj *prefixObj;
    Class *clsPtr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "className methodName");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }
    mPtr = GetClassMethodFromObj(interp, clsPtr, objv[2]);
    if (mPtr == NULL) {
	return TCL_ERROR;
    }
    prefixObj = GetForwardFromMethod(interp, mPtr);
    if (prefixObj == NULL) {
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, prefixObj);
    return TCL_OK;

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoClassMethodTypeCmd`, lines 1501–1526. Retained reference `source-window-1-8`.

```text
InfoClassMethodTypeCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Method *mPtr;
    Class *clsPtr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "className methodName");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }

    mPtr = GetClassMethodFromObj(interp, clsPtr, objv[2]);
    if (mPtr == NULL) {
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp,
	    Tcl_NewStringObj(mPtr->type2Ptr->name, TCL_AUTO_LENGTH));
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoObjectCallCmd / InfoClassCallCmd`, lines 1734–1815. Retained reference `source-window-1-9`.

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

/*
 * ----------------------------------------------------------------------
 *
 * InfoClassCallCmd --
 *
 *	Implements [info class call $clsName $methodName]
 *
 * ----------------------------------------------------------------------
 */

static int
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

/*

```

tcl8.6 8.6.18, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInfo.c`, function `InfoObjectIsACmd`, lines 394–500. Retained reference `source-window-1-10`.

```text
InfoObjectIsACmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    static const char *const categories[] = {
	"class", "metaclass", "mixin", "object", "typeof", NULL
    };
    enum IsACats {
	IsClass, IsMetaclass, IsMixin, IsObject, IsType
    };
    Object *oPtr, *o2Ptr;
    int idx, i, result = 0;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "category objName ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], categories, "category", 0,
	    &idx) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * Now we know what test we are doing, we can check we've got the right
     * number of arguments.
     */

    switch ((enum IsACats) idx) {
    case IsObject:
    case IsClass:
    case IsMetaclass:
	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "objName");
	    return TCL_ERROR;
	}
	break;
    case IsMixin:
    case IsType:
	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "objName className");
	    return TCL_ERROR;
	}
	break;
    }

    /*
     * Perform the check. Note that we can guarantee that we will not fail
     * from here on; "failures" result in a false-TCL_OK result.
     */

    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[2]);
    if (oPtr == NULL) {
	goto failPrecondition;
    }

    switch ((enum IsACats) idx) {
    case IsObject:
	result = 1;
	break;
    case IsClass:
	result = (oPtr->classPtr != NULL);
	break;
    case IsMetaclass:
	if (oPtr->classPtr != NULL) {
	    result = TclOOIsReachable(TclOOGetFoundation(interp)->classCls,
		    oPtr->classPtr);
	}
	break;
    case IsMixin:
	o2Ptr = (Object *) Tcl_GetObjectFromObj(interp, objv[3]);
	if (o2Ptr == NULL) {
	    goto failPrecondition;
	}
	if (o2Ptr->classPtr != NULL) {
	    Class *mixinPtr;

	    FOREACH(mixinPtr, oPtr->mixins) {
		if (!mixinPtr) {
		    continue;
		}
		if (TclOOIsReachable(o2Ptr->classPtr, mixinPtr)) {
		    result = 1;
		    break;
		}
	    }
	}
	break;
    case IsType:
	o2Ptr = (Object *) Tcl_GetObjectFromObj(interp, objv[3]);
	if (o2Ptr == NULL) {
	    goto failPrecondition;
	}
	if (o2Ptr->classPtr != NULL) {
	    result = TclOOIsReachable(o2Ptr->classPtr, oPtr->selfCls);
	}
	break;
    }
    Tcl_SetObjResult(interp, Tcl_NewBooleanObj(result));
    return TCL_OK;

  failPrecondition:
    Tcl_ResetResult(interp);
    Tcl_SetObjResult(interp, Tcl_NewBooleanObj(0));
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Original source joined by complete bytes and physical LF extents; source inspection only, without external callback or safe pointer-lifetime execution`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInfo.c`, function `InfoObjectIsACmd`, lines 442–553. Retained reference `source-window-1-11`.

```text
InfoObjectIsACmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    static const char *const categories[] = {
	"class", "metaclass", "mixin", "object", "typeof", NULL
    };
    enum IsACats {
	IsClass, IsMetaclass, IsMixin, IsObject, IsType
    } idx;
    Object *oPtr, *o2Ptr;
    int result = 0;
    Tcl_Size i;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "category objName ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], categories, "category", 0,
	    &idx) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * Now we know what test we are doing, we can check we've got the right
     * number of arguments.
     */

    switch (idx) {
    case IsObject:
    case IsClass:
    case IsMetaclass:
	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "objName");
	    return TCL_ERROR;
	}
	break;
    case IsMixin:
    case IsType:
	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "objName className");
	    return TCL_ERROR;
	}
	break;
    default:
	TCL_UNREACHABLE();
    }

    /*
     * Perform the check. Note that we can guarantee that we will not fail
     * from here on; "failures" result in a false-TCL_OK result.
     */

    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[2]);
    if (oPtr == NULL) {
	goto failPrecondition;
    }

    switch (idx) {
    case IsObject:
	result = 1;
	break;
    case IsClass:
	result = (oPtr->classPtr != NULL);
	break;
    case IsMetaclass:
	if (oPtr->classPtr != NULL) {
	    result = TclOOIsReachable(TclOOGetFoundation(interp)->classCls,
		    oPtr->classPtr);
	}
	break;
    case IsMixin:
	o2Ptr = (Object *) Tcl_GetObjectFromObj(interp, objv[3]);
	if (o2Ptr == NULL) {
	    goto failPrecondition;
	}
	if (o2Ptr->classPtr != NULL) {
	    Class *mixinPtr;

	    FOREACH(mixinPtr, oPtr->mixins) {
		if (!mixinPtr) {
		    continue;
		}
		if (TclOOIsReachable(o2Ptr->classPtr, mixinPtr)) {
		    result = 1;
		    break;
		}
	    }
	}
	break;
    case IsType:
	o2Ptr = (Object *) Tcl_GetObjectFromObj(interp, objv[3]);
	if (o2Ptr == NULL) {
	    goto failPrecondition;
	}
	if (o2Ptr->classPtr != NULL) {
	    result = TclOOIsReachable(o2Ptr->classPtr, oPtr->selfCls);
	}
	break;
    default:
	TCL_UNREACHABLE();
    }
    Tcl_SetObjResult(interp, Tcl_NewBooleanObj(result));
    return TCL_OK;

  failPrecondition:
    Tcl_ResetResult(interp);
    Tcl_SetObjResult(interp, Tcl_NewBooleanObj(0));
    return TCL_OK;
}

```

## Consumer bindings

- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `Class`: Keep a distinct software Class facet birth for genuine allocation, independent of Object creation and shallow observation clones.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `oo_copy_cmd`: Replace the shallow Class observation birth with a fresh birth before genuine destination insertion.
- [runtime/rust/src/cmd_oo/native_context.rs](../../../../runtime/rust/src/cmd_oo/native_context.rs), `OriginalOoTarget`: Retain the exact Object creation and weak Class facet birth across actual report getters; equal display or surviving Object does not authenticate a reborn facet.
- [runtime/rust/src/cmd_oo/native_context.rs](../../../../runtime/rust/src/cmd_oo/native_context.rs), `OriginalOoTarget::check_instance_namespace`: Join the selected instance namespace to the original Object and actual namespace-name token ledger before later access; no physical original C namespace recipe is issued.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `info_class`: Retain the selected original Object/Class receiver across the described later getter and current report/table derivation, without Native Method header, pointer-lifetime or callback execution authority.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `info_object`: Retain the selected original Object/Class receiver across the described later getter and current report/table derivation, without Native Method header, pointer-lifetime or callback execution authority.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `info_forward`: Retain the selected original Object/Class receiver across the described later getter and current report/table derivation, without Native Method header, pointer-lifetime or callback execution authority.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `info_definition`: Retain the selected original Object/Class receiver across the described later getter and current report/table derivation, without Native Method header, pointer-lifetime or callback execution authority.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `info_methodtype`: Retain the selected original Object/Class receiver across the described later getter and current report/table derivation, without Native Method header, pointer-lifetime or callback execution authority.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `info_call`: Retain the selected original Object/Class receiver across the described later getter and current report/table derivation, without Native Method header, pointer-lifetime or callback execution authority.
- [runtime/rust/src/cmd_oo/original_class_facet_tests.rs](../../../../runtime/rust/src/cmd_oo/original_class_facet_tests.rs), `cmd_oo::original_class_facet_tests::actual_class_allocation_and_copy_issue_fresh_births_while_observations_share_identity` (linked): Actual Runtime Class allocation and copy issue fresh software births, while shallow observation clones share the same birth. The finite control checks actual root/default/copy producers without claiming original C Class pointer identity or payload lifetime. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_class_facet_tests.rs](../../../../runtime/rust/src/cmd_oo/original_class_facet_tests.rs), `cmd_oo::original_class_facet_tests::demotion_and_promotion_cannot_refresh_an_original_class_facet_continuation` (linked): Genuine demotion and promotion keep the original Object/context but issue a different Class facet birth. A retained weak original facet cannot become the new facet; this is software continuation withdrawal, not an original C callback safety observation. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_class_facet_tests.rs](../../../../runtime/rust/src/cmd_oo/original_class_facet_tests.rs), `cmd_oo::original_class_facet_tests::original_class_reports_check_reached_operands_before_current_facet_access` (linked): Selected Class reports retain the original receiver/facet before kind, method, property-option and pattern getters. Ordinary metadata changes remain allowed, while Host/ABA, Object withdrawal and reborn facets stop later current table/report access. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_info_target_tests.rs](../../../../runtime/rust/src/cmd_oo/original_info_target_tests.rs), `cmd_oo::original_info_target_tests::shared_method_reports_keep_original_receiver_before_name_getters_and_table_access` (linked): Object and Class forward/definition/methodtype/call reports retain the original receiver before method-name getters and later table derivation. Reached bytes and first refusal remain separate from missing-method presentation or fresh receiver reselection. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_info_target_tests.rs](../../../../runtime/rust/src/cmd_oo/original_info_target_tests.rs), `cmd_oo::original_info_target_tests::object_reports_refuse_withdrawn_receivers_before_relation_namespace_or_report_derivation` (linked): Object class-membership, methods, variables and property reports preserve original receiver checks after reached later operands. The selected instance namespace is joined to its actual original object and token ledger before access; software destruction/recreation and ordinary mutations remain distinct. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_info_target_tests.rs](../../../../runtime/rust/src/cmd_oo/original_info_target_tests.rs), `cmd_oo::original_info_target_tests::isa_missing_precondition_leaves_relation_unread_but_selected_receiver_withdrawal_refuses` (linked): Initial missing isa receiver returns false without reading the secondary relation operand, separately from an initially selected live receiver withdrawn during a later getter. The latter preserves reached bytes/first refusal and cannot be repaired by a fresh false or spelling lookup. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.

## Evidence

- [runtime/rust/tests/data/original_introspection_target_continuation_source/9.1.0](../../../../runtime/rust/tests/data/original_introspection_target_continuation_source/9.1.0)
- [runtime/rust/tests/data/original_introspection_target_continuation_source/8.6.18/source-C8.6.18-tclOOInfo-394-500.c](../../../../runtime/rust/tests/data/original_introspection_target_continuation_source/8.6.18/source-C8.6.18-tclOOInfo-394-500.c)
- [runtime/rust/tests/data/original_introspection_target_continuation_source/9.0.4/source-C9.0.4-tclOOInfo-442-553.c](../../../../runtime/rust/tests/data/original_introspection_target_continuation_source/9.0.4/source-C9.0.4-tclOOInfo-442-553.c)
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs)
- [runtime/rust/src/cmd_oo](../../../../runtime/rust/src/cmd_oo)
