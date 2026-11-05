# naming.tcloo.original-method-rename-operands-and-records

Kind: `native-observation`

## Problem statement

Method rename uses counted original source/destination names, a current method table and retained name/hash roles. Public Eval error reporting can read a later original argument even when the stock worker stops before it. Header ABI width and returned CmdInfo worker kind also represent different facts.

## Question

Within genuine original class/object definition contexts, which method-name getters and record/reference effects occur for missing/self/occupied/vacant destinations, and how do public EvalObjv and the actually returned stock-worker CmdInfo entry differ?

## Conclusion

The 32 reached510b cases per C86/C90/C91 preserve source-only stopping in the direct stock worker, separate public error-context destination getters, actual record-identity rejection, original visibility/clientData on successful move and counted-NUL lookup. Destination refs3 supply independent caller, Method-name and ObjHash-key roles. Public successful Eval clears seeded code; direct worker success keeps it. Actual CmdInfo reports kind1/int on C86/C90 and kind2/Tcl_Size on C91. Original510 keeps its C90 direct16 probe refusals unchanged; its process success does not establish those worker outcomes. C84/C85/Jim measure selected availability only. No Rust assertion, pointer generation or cross-channel error-state equivalence follows.

## Scope

Two independently fixed requests and all six own dependency/compile/version/execute captures are retained separately. Original510 attempts 32 cases per supported provider, with 16 C90 direct sentinel-712 refusals and no reached direct-worker result. Successor510b uses the actually returned CmdInfo kind and reaches all 32 supported cases. Eight source/destination scenarios span class/per-object scope and two authentic entry channels. Raw worker-return rows, public Eval error context, RESULT/ERRORCODE getters, outer definition results, subsequent invocation and interpreter teardown retain their explicit phases. Header Tcl_Size availability cannot select the function-pointer count ABI. Method/name/object-hash references have separate roles; arbitrary external Lists or application generation, current Native issuer, Body/Forward/Property payload lifetime, compiler/frame/edit permission and general OO availability are outside this finite question. SDK source/library inputs are retained; configure/build processes lack replay receipts.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original lazy counted method-name and token headers inside genuine class/object definition context; public EvalObjv and CmdInfo stock-worker calls have separate raw and presentation phases. Version: tcl8.4 8.4.20.

Initialized original 8.4.20 core/public SDK records selected TclOO rename worker availability only. No class/object rename semantics, getter chronology, optional-extension absence or Jim class simulation is inferred.

### tcl8.5

Status: `observed`. Channel: Original lazy counted method-name and token headers inside genuine class/object definition context; public EvalObjv and CmdInfo stock-worker calls have separate raw and presentation phases. Version: tcl8.5 8.5.19.

Initialized original 8.5.19 core/public SDK records selected TclOO rename worker availability only. No class/object rename semantics, getter chronology, optional-extension absence or Jim class simulation is inferred.

### tcl8.6

Status: `observed`. Channel: Original lazy counted method-name and token headers inside genuine class/object definition context; public EvalObjv and CmdInfo stock-worker calls have separate raw and presentation phases. Version: tcl8.6 8.6.18.

In510b, 32 fresh cases reach genuine original class/object method rename through separate public EvalObjv and CmdInfo-selected stock-worker channels. Missing source stops the direct worker before destination String; public Eval error context reaches destination before public return. Self-record and occupied destination fail without retiring clientData; vacant, uppercase, counted-NUL and literal qualified-looking destinations move the original method and retain its visibility/clientData. Successful destination refs3 comprise caller, Method-name and object-hash-key holds; failures leave caller refs1. Public successful Eval clears the seeded error-code option, while direct success retains SEEDED CODE. Later RESULT/ERRORCODE and invocation rendering are separate. Actual selected worker ABI is kind1/int. Original510 independently retains both reached channels.

### tcl9.0

Status: `observed`. Channel: Original lazy counted method-name and token headers inside genuine class/object definition context; public EvalObjv and CmdInfo stock-worker calls have separate raw and presentation phases. Version: tcl9.0 9.0.4.

In510b, 32 fresh cases reach genuine original class/object method rename through separate public EvalObjv and CmdInfo-selected stock-worker channels. Missing source stops the direct worker before destination String; public Eval error context reaches destination before public return. Self-record and occupied destination fail without retiring clientData; vacant, uppercase, counted-NUL and literal qualified-looking destinations move the original method and retain its visibility/clientData. Successful destination refs3 comprise caller, Method-name and object-hash-key holds; failures leave caller refs1. Public successful Eval clears the seeded error-code option, while direct success retains SEEDED CODE. Later RESULT/ERRORCODE and invocation rendering are separate. Actual selected worker ABI is kind1/int. Original510 public 16 cases reach their worker but its 16 direct cases stop at probe-sentinel-712; those records remain unreached and independent.

### tcl9.1

Status: `observed`. Channel: Original lazy counted method-name and token headers inside genuine class/object definition context; public EvalObjv and CmdInfo stock-worker calls have separate raw and presentation phases. Version: tcl9.1 9.1.0.

In510b, 32 fresh cases reach genuine original class/object method rename through separate public EvalObjv and CmdInfo-selected stock-worker channels. Missing source stops the direct worker before destination String; public Eval error context reaches destination before public return. Self-record and occupied destination fail without retiring clientData; vacant, uppercase, counted-NUL and literal qualified-looking destinations move the original method and retain its visibility/clientData. Successful destination refs3 comprise caller, Method-name and object-hash-key holds; failures leave caller refs1. Public successful Eval clears the seeded error-code option, while direct success retains SEEDED CODE. Later RESULT/ERRORCODE and invocation rendering are separate. Actual selected worker ABI is kind2/Tcl_Size. Original510 independently retains both reached channels.

### jim

Status: `observed`. Channel: Original lazy counted method-name and token headers inside genuine class/object definition context; public EvalObjv and CmdInfo stock-worker calls have separate raw and presentation phases. Version: jim 0.84.

Initialized original jim core/public SDK records selected TclOO rename worker availability only. No class/object rename semantics, getter chronology, optional-extension absence or Jim class simulation is inferred.

### bigip

Status: `not-tested`.

No original BIG-IP process for this finite TclOO public/stock-worker question.

## Source inspection

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 64–174. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
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
	return;
    }

    /*
     * Either there's no class (?!) or we're reconfiguring something that is
     * in use. Force regeneration of call chains.
     */

    TclOOGetFoundation(interp)->epoch++;
}

/*
 * ----------------------------------------------------------------------
 *

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 434–545. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
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
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Namespace *nsPtr = (Namespace *) Tcl_GetCurrentNamespace(interp);
    Tcl_HashSearch search;
    Tcl_HashEntry *hPtr;
    int soughtLen;
    const char *soughtStr, *matchedStr = NULL;

    if (objc < 2) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"bad call of unknown handler", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_UNKNOWN", (char *)NULL);
	return TCL_ERROR;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 1302–1412. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
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
    Tcl_Obj *const *objv)
{
    int isInstanceExport = (clientData != NULL);
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 1634–1744. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
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
     * Bump the right epoch if we actually changed anything.
     */

    if (changed) {
	if (isInstanceUnexport) {
	    oPtr->epoch++;
	} else {
	    BumpGlobalEpoch(interp, clsPtr);
	}
    }
    return TCL_OK;
}

/*

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 544–654. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
	Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_UNKNOWN", (char *)NULL);
	return TCL_ERROR;
    }
    if (TclOOGetDefineCmdContext(interp) == NULL) {
	return TCL_ERROR;
    }

    soughtStr = Tcl_GetStringFromObj(objv[1], &soughtLen);
    if (soughtLen == 0) {
	goto noMatch;
    }
    hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
    while (hPtr != NULL) {
	const char *nameStr = Tcl_GetHashKey(&nsPtr->cmdTable, hPtr);

	if (strncmp(soughtStr, nameStr, soughtLen) == 0) {
	    if (matchedStr != NULL) {
		goto noMatch;
	    }
	    matchedStr = nameStr;
	}
	hPtr = Tcl_NextHashEntry(&search);
    }

    if (matchedStr != NULL) {
	/*
	 * Got one match, and only one match!
	 */

	Tcl_Obj **newObjv =
		TclStackAlloc(interp, sizeof(Tcl_Obj*) * (objc - 1));
	int result;

	newObjv[0] = Tcl_NewStringObj(matchedStr, -1);
	Tcl_IncrRefCount(newObjv[0]);
	if (objc > 2) {
	    memcpy(newObjv + 1, objv + 2, sizeof(Tcl_Obj *) * (objc - 2));
	}
	result = Tcl_EvalObjv(interp, objc - 1, newObjv, 0);
	Tcl_DecrRefCount(newObjv[0]);
	TclStackFree(interp, newObjv);
	return result;
    }

  noMatch:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "invalid command name \"%s\"", soughtStr));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "COMMAND", soughtStr, (char *)NULL);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * FindCommand --
 *	Specialized version of Tcl_FindCommand that handles command prefixes
 *	and disallows namespace magic.
 *
 * ----------------------------------------------------------------------
 */

static Tcl_Command
FindCommand(
    Tcl_Interp *interp,
    Tcl_Obj *stringObj,
    Tcl_Namespace *const namespacePtr)
{
    int length;
    const char *nameStr, *string = Tcl_GetStringFromObj(stringObj, &length);
    Namespace *const nsPtr = (Namespace *) namespacePtr;
    FOREACH_HASH_DECLS;
    Tcl_Command cmd, cmd2;

    /*
     * If someone is playing games, we stop playing right now.
     */

    if (string[0] == '\0' || strstr(string, "::") != NULL) {
	return NULL;
    }

    /*
     * Do the exact lookup first.
     */

    cmd = Tcl_FindCommand(interp, string, namespacePtr, TCL_NAMESPACE_ONLY);
    if (cmd != NULL) {
	return cmd;
    }

    /*
     * Bother, need to perform an approximate match. Iterate across the hash
     * table of commands in the namespace.
     */

    FOREACH_HASH(nameStr, cmd2, &nsPtr->cmdTable) {
	if (strncmp(string, nameStr, length) == 0) {
	    if (cmd != NULL) {
		return NULL;
	    }
	    cmd = cmd2;
	}
    }

    /*
     * Either we found one thing or we found nothing. Either way, return it.
     */

    return cmd;
}


```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 705–814. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
 */

Tcl_Object
TclOOGetDefineCmdContext(
    Tcl_Interp *interp)
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Object object;

    if ((iPtr->varFramePtr == NULL)
	    || (iPtr->varFramePtr->isProcCallFrame != FRAME_IS_OO_DEFINE)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"this command may only be called from within the context of"
		" an ::oo::define or ::oo::objdefine command", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return NULL;
    }
    object = iPtr->varFramePtr->clientData;
    if (Tcl_ObjectDeleted(object)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"this command cannot be called when the object has been"
		" deleted", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return NULL;
    }
    return object;
}

/*
 * ----------------------------------------------------------------------
 *
 * GetClassInOuterContext --
 *	Wrapper round Tcl_GetObjectFromObj to perform the lookup in the
 *	context that called oo::define (or equivalent). Note that this may
 *	have to go up multiple levels to get the level that we started doing
 *	definitions at.
 *
 * ----------------------------------------------------------------------
 */

static inline Class *
GetClassInOuterContext(
    Tcl_Interp *interp,
    Tcl_Obj *className,
    const char *errMsg)
{
    Interp *iPtr = (Interp *) interp;
    Object *oPtr;
    CallFrame *savedFramePtr = iPtr->varFramePtr;

    while (iPtr->varFramePtr->isProcCallFrame == FRAME_IS_OO_DEFINE) {
	if (iPtr->varFramePtr->callerVarPtr == NULL) {
	    Tcl_Panic("getting outer context when already in global context");
	}
	iPtr->varFramePtr = iPtr->varFramePtr->callerVarPtr;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, className);
    iPtr->varFramePtr = savedFramePtr;
    if (oPtr == NULL) {
	return NULL;
    }
    if (oPtr->classPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(errMsg, -1));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CLASS",
		TclGetString(className), (char *)NULL);
	return NULL;
    }
    return oPtr->classPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * GenerateErrorInfo --
 *	Factored out code to generate part of the error trace messages.
 *
 * ----------------------------------------------------------------------
 */

static inline void
GenerateErrorInfo(
    Tcl_Interp *interp,		/* Where to store the error info trace. */
    Object *oPtr,		/* What object (or class) was being configured
				 * when the error occurred? */
    Tcl_Obj *savedNameObj,	/* Name of object saved from before script was
				 * evaluated, which is needed if the object
				 * goes away part way through execution. OTOH,
				 * if the object isn't deleted then its
				 * current name (post-execution) has to be
				 * used. This matters, because the object
				 * could have been renamed... */
    const char *typeOfSubject)	/* Part of the message, saying whether it was
				 * an object, class or class-as-object that
				 * was being configured. */
{
    int length;
    Tcl_Obj *realNameObj = Tcl_ObjectDeleted((Tcl_Object) oPtr)
	    ? savedNameObj : TclOOObjectName(interp, oPtr);
    const char *objName = Tcl_GetStringFromObj(realNameObj, &length);
    int limit = OBJNAME_LENGTH_IN_ERRORINFO_LIMIT;
    int overflow = (length > limit);

    Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
	    "\n    (in definition script for %s \"%.*s%s\" line %d)",
	    typeOfSubject, (overflow ? limit : length), objName,
	    (overflow ? "..." : ""), Tcl_GetErrorLine(interp)));
}

/*
 * ----------------------------------------------------------------------

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1046–1157. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Make the oo::objdefine namespace the current namespace and evaluate the
     * command(s).
     */

    if (InitDefineContext(interp, fPtr->objdefNs, oPtr, objc,objv) != TCL_OK){
	return TCL_ERROR;
    }

    AddRef(oPtr);
    if (objc == 2) {
	Tcl_Obj *objNameObj = TclOOObjectName(interp, oPtr);

	Tcl_IncrRefCount(objNameObj);
	result = TclEvalObjEx(interp, objv[1], 0,
		((Interp *)interp)->cmdFramePtr, 2);
	if (result == TCL_ERROR) {
	    GenerateErrorInfo(interp, oPtr, objNameObj, "class object");
	}
	TclDecrRefCount(objNameObj);
    } else {
	result = MagicDefinitionInvoke(interp, fPtr->objdefNs, 1, objc, objv);
    }
    TclOODecrRefCount(oPtr);

    /*
     * Restore the previous "current" namespace.
     */

    TclPopStackFrame(interp);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineClassObjCmd --
 *	Implementation of the "class" subcommand of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineClassObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Class *clsPtr;
    Foundation *fPtr = TclOOGetFoundation(interp);
    int wasClass, willBeClass;

    /*
     * Parse the context to get the object to operate on.
     */

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (oPtr->flags & ROOT_OBJECT) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"may not modify the class of the root object class", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }
    if (oPtr->flags & ROOT_CLASS) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"may not modify the class of the class of classes", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
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
		"may not change classes into an instance of themselves", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Set the object's class.
     */

    wasClass = (oPtr->classPtr != NULL);
    willBeClass = (TclOOIsReachable(fPtr->classCls, clsPtr));

    if (oPtr->selfCls != clsPtr) {

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1110–1221. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
     * Parse the context to get the object to operate on.
     */

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (oPtr->flags & ROOT_OBJECT) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"may not modify the class of the root object class", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }
    if (oPtr->flags & ROOT_CLASS) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"may not modify the class of the class of classes", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
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
		"may not change classes into an instance of themselves", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Set the object's class.
     */

    wasClass = (oPtr->classPtr != NULL);
    willBeClass = (TclOOIsReachable(fPtr->classCls, clsPtr));

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
		ckfree(oPtr->classPtr);
		oPtr->classPtr = NULL;
	} else if (!wasClass && willBeClass) {
	    TclOOAllocClass(interp, oPtr);
	}

	if (oPtr->classPtr != NULL) {
	    BumpGlobalEpoch(interp, oPtr->classPtr);
	} else {
	    oPtr->epoch++;
	}
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineConstructorObjCmd --
 *	Implementation of the "constructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineConstructorObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Class *clsPtr;
    Tcl_Method method;
    int bodyLength;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "arguments body");
	return TCL_ERROR;
    }

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1224–1334. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
     * modify.
     */

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;

    Tcl_GetStringFromObj(objv[2], &bodyLength);
    if (bodyLength > 0) {
	/*
	 * Create the method structure.
	 */

	method = (Tcl_Method) TclOONewProcMethod(interp, clsPtr,
		PUBLIC_METHOD, NULL, objv[1], objv[2], NULL);
	if (method == NULL) {
	    return TCL_ERROR;
	}
    } else {
	/*
	 * Delete the constructor method record and set the field in the
	 * class record to NULL.
	 */

	method = NULL;
    }

    /*
     * Place the method structure in the class record. Note that we might not
     * immediately delete the constructor as this might be being done during
     * execution of the constructor itself.
     */

    Tcl_ClassSetConstructor(interp, (Tcl_Class) clsPtr, method);
    return TCL_OK;
}

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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1286–1396. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1343–1454. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
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
    if (!isInstanceExport && !clsPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Exporting is done by adding the PUBLIC_METHOD flag to the method
	 * record. If there is no such method in this object or class (i.e.
	 * the method comes from something inherited from or that we're an
	 * instance of) then we put in a blank record with that flag; such
	 * records are skipped over by the call chain engine *except* for
	 * their flags member.
	 */

	if (isInstanceExport) {
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1409–1520. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;
    if (!isInstanceExport && !clsPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Exporting is done by adding the PUBLIC_METHOD flag to the method
	 * record. If there is no such method in this object or class (i.e.
	 * the method comes from something inherited from or that we're an
	 * instance of) then we put in a blank record with that flag; such
	 * records are skipped over by the call chain engine *except* for
	 * their flags member.
	 */

	if (isInstanceExport) {
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
	if (isNew || !(mPtr->flags & PUBLIC_METHOD)) {
	    mPtr->flags |= PUBLIC_METHOD;
	    changed = 1;
	}
    }

    /*
     * Bump the right epoch if we actually changed anything.
     */

    if (changed) {
	if (isInstanceExport) {
	    oPtr->epoch++;
	} else {
	    BumpGlobalEpoch(interp, clsPtr);
	}
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineForwardObjCmd --
 *	Implementation of the "forward" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1502–1612. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
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

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineMethodObjCmd --
 *	Implementation of the "method" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
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

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineRenameMethodObjCmd --
 *	Implementation of the "renamemethod" subcommand of the "oo::define"
 *	and "oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineRenameMethodObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceRenameMethod = (clientData != NULL);
    Object *oPtr;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1560–1670. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
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

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineRenameMethodObjCmd --
 *	Implementation of the "renamemethod" subcommand of the "oo::define"
 *	and "oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineRenameMethodObjCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstanceRenameMethod = (clientData != NULL);
    Object *oPtr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "oldName newName");
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceRenameMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Delete the method entry from the appropriate hash table, and transfer
     * the thing it points to to its new entry. To do this, we first need to
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1616–1727. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceRenameMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Delete the method entry from the appropriate hash table, and transfer
     * the thing it points to to its new entry. To do this, we first need to
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1676–1787. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
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
     * Bump the right epoch if we actually changed anything.
     */

    if (changed) {
	if (isInstanceUnexport) {
	    oPtr->epoch++;
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

	/*
	 * Remember to invalidate the cached constructor chain for this class.
	 * [Bug 2531577]
	 */

	if (clsPtr->constructorChainPtr) {
	    TclOODeleteChain(clsPtr->constructorChainPtr);
	    clsPtr->constructorChainPtr = NULL;
	}
	BumpGlobalEpoch(interp, clsPtr);
    }
}

void
Tcl_ClassSetDestructor(
    Tcl_Interp *interp,
    Tcl_Class clazz,
    Tcl_Method method)
{
    Class *clsPtr = (Class *) clazz;

    if (method != (Tcl_Method) clsPtr->destructorPtr) {

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1856–1967. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj, *filterObj;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }
    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(filterObj, oPtr->classPtr->filters) {
	Tcl_ListObjAppendElement(NULL, resultObj, filterObj);
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ClassFilterSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int filterc;
    Tcl_Obj **filterv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &filterc,
	    &filterv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOClassSetFilters(interp, oPtr->classPtr, filterc, filterv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * ClassMixinGet, ClassMixinSet --
 *	Implementation of the "mixin" slot accessors of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ClassMixinGet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }
    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->classPtr->mixins) {
	Tcl_ListObjAppendElement(NULL, resultObj,
		TclOOObjectName(interp, mixinPtr->thisPtr));
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;

}

static int

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1890–2001. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int filterc;
    Tcl_Obj **filterv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &filterc,
	    &filterv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOClassSetFilters(interp, oPtr->classPtr, filterc, filterv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * ClassMixinGet, ClassMixinSet --
 *	Implementation of the "mixin" slot accessors of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ClassMixinGet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }
    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->classPtr->mixins) {
	Tcl_ListObjAppendElement(NULL, resultObj,
		TclOOObjectName(interp, mixinPtr->thisPtr));
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;

}

static int
ClassMixinSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int mixinc, i;
    Tcl_Obj **mixinv;
    Class **mixins;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &mixinc,
	    &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = TclStackAlloc(interp, sizeof(Class *) * mixinc);

    for (i = 0; i < mixinc; i++) {

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1935–2046. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }
    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->classPtr->mixins) {
	Tcl_ListObjAppendElement(NULL, resultObj,
		TclOOObjectName(interp, mixinPtr->thisPtr));
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;

}

static int
ClassMixinSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int mixinc, i;
    Tcl_Obj **mixinv;
    Class **mixins;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &mixinc,
	    &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = TclStackAlloc(interp, sizeof(Class *) * mixinc);

    for (i = 0; i < mixinc; i++) {
	mixins[i] = GetClassInOuterContext(interp, mixinv[i],
		"may only mix in classes");
	if (mixins[i] == NULL) {
	    i--;
	    goto freeAndError;
	}
	if (TclOOIsReachable(oPtr->classPtr, mixins[i])) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "may not mix a class into itself", -1));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "SELF_MIXIN", (char *)NULL);
	    goto freeAndError;
	}
    }

    TclOOClassSetMixins(interp, oPtr->classPtr, mixinc, mixins);
    TclStackFree(interp, mixins);
    return TCL_OK;

  freeAndError:
    TclStackFree(interp, mixins);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * ClassSuperGet, ClassSuperSet --
 *	Implementation of the "superclass" slot accessors of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ClassSuperGet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *superPtr;
    int i;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1972–2083. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int mixinc, i;
    Tcl_Obj **mixinv;
    Class **mixins;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &mixinc,
	    &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = TclStackAlloc(interp, sizeof(Class *) * mixinc);

    for (i = 0; i < mixinc; i++) {
	mixins[i] = GetClassInOuterContext(interp, mixinv[i],
		"may only mix in classes");
	if (mixins[i] == NULL) {
	    i--;
	    goto freeAndError;
	}
	if (TclOOIsReachable(oPtr->classPtr, mixins[i])) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "may not mix a class into itself", -1));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "SELF_MIXIN", (char *)NULL);
	    goto freeAndError;
	}
    }

    TclOOClassSetMixins(interp, oPtr->classPtr, mixinc, mixins);
    TclStackFree(interp, mixins);
    return TCL_OK;

  freeAndError:
    TclStackFree(interp, mixins);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * ClassSuperGet, ClassSuperSet --
 *	Implementation of the "superclass" slot accessors of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ClassSuperGet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *superPtr;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }
    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(superPtr, oPtr->classPtr->superclasses) {
	Tcl_ListObjAppendElement(NULL, resultObj,
		TclOOObjectName(interp, superPtr->thisPtr));
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ClassSuperSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int superc, i, j;
    Tcl_Obj **superv;
    Class **superclasses, *superPtr;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2040–2152. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *superPtr;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }
    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(superPtr, oPtr->classPtr->superclasses) {
	Tcl_ListObjAppendElement(NULL, resultObj,
		TclOOObjectName(interp, superPtr->thisPtr));
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ClassSuperSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int superc, i, j;
    Tcl_Obj **superv;
    Class **superclasses, *superPtr;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"superclassList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    } else if (oPtr == oPtr->fPtr->objectCls->thisPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"may not modify the superclass of the root object", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &superc,
	    &superv) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * Allocate some working space.
     */

    superclasses = (Class **) ckalloc(sizeof(Class *) * superc);

    /*
     * Parse the arguments to get the class to use as superclasses.
     *
     * Note that zero classes is special, as it is equivalent to just the
     * class of objects. [Bug 9d61624b3d]
     */

    if (superc == 0) {
	superclasses = ckrealloc(superclasses, sizeof(Class *));
	if (TclOOIsReachable(oPtr->fPtr->classCls, oPtr->classPtr)) {
	    superclasses[0] = oPtr->fPtr->classCls;
	} else {
	    superclasses[0] = oPtr->fPtr->objectCls;
	}
	superc = 1;
	AddRef(superclasses[0]->thisPtr);
    } else {
	for (i = 0; i < superc; i++) {
	    superclasses[i] = GetClassInOuterContext(interp, superv[i],
		    "only a class can be a superclass");
	    if (superclasses[i] == NULL) {
		goto failedAfterAlloc;
	    }
	    for (j = 0; j < i; j++) {
		if (superclasses[j] == superclasses[i]) {
		    Tcl_SetObjResult(interp, Tcl_NewStringObj(
			    "class should only be a direct superclass once",
			    -1));
		    Tcl_SetErrorCode(interp, "TCL", "OO", "REPETITIOUS", (char *)NULL);
		    goto failedAfterAlloc;
		}
	    }
	    if (TclOOIsReachable(oPtr->classPtr, superclasses[i])) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"attempt to form circular dependency graph", -1));
		Tcl_SetErrorCode(interp, "TCL", "OO", "CIRCULARITY", (char *)NULL);
	    failedAfterAlloc:
		for (; i-- > 0 ;) {
		    TclOODecrRefCount(superclasses[i]->thisPtr);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2076–2188. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int superc, i, j;
    Tcl_Obj **superv;
    Class **superclasses, *superPtr;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"superclassList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    } else if (oPtr == oPtr->fPtr->objectCls->thisPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"may not modify the superclass of the root object", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &superc,
	    &superv) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * Allocate some working space.
     */

    superclasses = (Class **) ckalloc(sizeof(Class *) * superc);

    /*
     * Parse the arguments to get the class to use as superclasses.
     *
     * Note that zero classes is special, as it is equivalent to just the
     * class of objects. [Bug 9d61624b3d]
     */

    if (superc == 0) {
	superclasses = ckrealloc(superclasses, sizeof(Class *));
	if (TclOOIsReachable(oPtr->fPtr->classCls, oPtr->classPtr)) {
	    superclasses[0] = oPtr->fPtr->classCls;
	} else {
	    superclasses[0] = oPtr->fPtr->objectCls;
	}
	superc = 1;
	AddRef(superclasses[0]->thisPtr);
    } else {
	for (i = 0; i < superc; i++) {
	    superclasses[i] = GetClassInOuterContext(interp, superv[i],
		    "only a class can be a superclass");
	    if (superclasses[i] == NULL) {
		goto failedAfterAlloc;
	    }
	    for (j = 0; j < i; j++) {
		if (superclasses[j] == superclasses[i]) {
		    Tcl_SetObjResult(interp, Tcl_NewStringObj(
			    "class should only be a direct superclass once",
			    -1));
		    Tcl_SetErrorCode(interp, "TCL", "OO", "REPETITIOUS", (char *)NULL);
		    goto failedAfterAlloc;
		}
	    }
	    if (TclOOIsReachable(oPtr->classPtr, superclasses[i])) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"attempt to form circular dependency graph", -1));
		Tcl_SetErrorCode(interp, "TCL", "OO", "CIRCULARITY", (char *)NULL);
	    failedAfterAlloc:
		for (; i-- > 0 ;) {
		    TclOODecrRefCount(superclasses[i]->thisPtr);
		}
		ckfree(superclasses);
		return TCL_ERROR;
	    }

	    /*
	     * Corresponding TclOODecrRefCount() is near the end of this
	     * function.
	     */

	    AddRef(superclasses[i]->thisPtr);
	}
    }

    /*
     * Install the list of superclasses into the class. Note that this also
     * involves splicing the class out of the superclasses' subclass list that
     * it used to be a member of and splicing it into the new superclasses'
     * subclass list.
     */

    if (oPtr->classPtr->superclasses.num != 0) {
	FOREACH(superPtr, oPtr->classPtr->superclasses) {
	    TclOORemoveFromSubclasses(oPtr->classPtr, superPtr);
	    TclOODecrRefCount(superPtr->thisPtr);
	}
	ckfree((char *) oPtr->classPtr->superclasses.list);
    }
    oPtr->classPtr->superclasses.list = superclasses;
    oPtr->classPtr->superclasses.num = superc;
    FOREACH(superPtr, oPtr->classPtr->superclasses) {
	TclOOAddToSubclasses(oPtr->classPtr, superPtr);
    }
    BumpGlobalEpoch(interp, oPtr->classPtr);

    return TCL_OK;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2206–2318. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj, *variableObj;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    }
    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(variableObj, oPtr->classPtr->variables) {
	Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ClassVarsSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int varc;
    Tcl_Obj **varv, *variableObj;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &varc,
	    &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    for (i = 0; i < varc; i++) {
	const char *varName = Tcl_GetString(varv[i]);

	if (strstr(varName, "::") != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "contain namespace separators"));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_DECLVAR", (char *)NULL);
	    return TCL_ERROR;
	}
	if (Tcl_StringMatch(varName, "*(*)")) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "refer to an array element"));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_DECLVAR", (char *)NULL);
	    return TCL_ERROR;
	}
    }

    for (i = 0; i < varc; i++) {
	Tcl_IncrRefCount(varv[i]);
    }
    FOREACH(variableObj, oPtr->classPtr->variables) {
	Tcl_DecrRefCount(variableObj);
    }
    if (i != varc) {
	if (varc == 0) {
	    ckfree((char *) oPtr->classPtr->variables.list);
	} else if (i) {
	    oPtr->classPtr->variables.list = (Tcl_Obj **)
		    ckrealloc((char *) oPtr->classPtr->variables.list,
		    sizeof(Tcl_Obj *) * varc);
	} else {
	    oPtr->classPtr->variables.list = (Tcl_Obj **)
		    ckalloc(sizeof(Tcl_Obj *) * varc);
	}
    }

    oPtr->classPtr->variables.num = 0;
    if (varc > 0) {
	int created, n;
	Tcl_HashTable uniqueTable;

	Tcl_InitObjHashTable(&uniqueTable);
	for (i = n = 0; i < varc; i++) {
	    Tcl_CreateHashEntry(&uniqueTable, varv[i], &created);
	    if (created) {
		oPtr->classPtr->variables.list[n++] = varv[i];
	    } else {
		Tcl_DecrRefCount(varv[i]);
	    }
	}

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2240–2351. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int varc;
    Tcl_Obj **varv, *variableObj;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "MONKEY_BUSINESS", (char *)NULL);
	return TCL_ERROR;
    } else if (TclListObjGetElements(interp, objv[0], &varc,
	    &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    for (i = 0; i < varc; i++) {
	const char *varName = Tcl_GetString(varv[i]);

	if (strstr(varName, "::") != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "contain namespace separators"));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_DECLVAR", (char *)NULL);
	    return TCL_ERROR;
	}
	if (Tcl_StringMatch(varName, "*(*)")) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "refer to an array element"));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_DECLVAR", (char *)NULL);
	    return TCL_ERROR;
	}
    }

    for (i = 0; i < varc; i++) {
	Tcl_IncrRefCount(varv[i]);
    }
    FOREACH(variableObj, oPtr->classPtr->variables) {
	Tcl_DecrRefCount(variableObj);
    }
    if (i != varc) {
	if (varc == 0) {
	    ckfree((char *) oPtr->classPtr->variables.list);
	} else if (i) {
	    oPtr->classPtr->variables.list = (Tcl_Obj **)
		    ckrealloc((char *) oPtr->classPtr->variables.list,
		    sizeof(Tcl_Obj *) * varc);
	} else {
	    oPtr->classPtr->variables.list = (Tcl_Obj **)
		    ckalloc(sizeof(Tcl_Obj *) * varc);
	}
    }

    oPtr->classPtr->variables.num = 0;
    if (varc > 0) {
	int created, n;
	Tcl_HashTable uniqueTable;

	Tcl_InitObjHashTable(&uniqueTable);
	for (i = n = 0; i < varc; i++) {
	    Tcl_CreateHashEntry(&uniqueTable, varv[i], &created);
	    if (created) {
		oPtr->classPtr->variables.list[n++] = varv[i];
	    } else {
		Tcl_DecrRefCount(varv[i]);
	    }
	}
	oPtr->classPtr->variables.num = n;

	/*
	 * Shouldn't be necessary, but maintain num/list invariant.
	 */

	oPtr->classPtr->variables.list = (Tcl_Obj **)
		ckrealloc((char *) oPtr->classPtr->variables.list,
		sizeof(Tcl_Obj *) * n);
	Tcl_DeleteHashTable(&uniqueTable);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjectFilterGet, ObjectFilterSet --
 *	Implementation of the "filter" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjFilterGet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2348–2459. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj, *filterObj;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(filterObj, oPtr->filters) {
	Tcl_ListObjAppendElement(NULL, resultObj, filterObj);
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjFilterSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int filterc;
    Tcl_Obj **filterv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &filterc,
	    &filterv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOObjectSetFilters(oPtr, filterc, filterv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjectMixinGet, ObjectMixinSet --
 *	Implementation of the "mixin" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjMixinGet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->mixins) {
	if (mixinPtr) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    TclOOObjectName(interp, mixinPtr->thisPtr));
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjMixinSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int mixinc;
    Tcl_Obj **mixinv;
    Class **mixins;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2376–2486. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int filterc;
    Tcl_Obj **filterv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &filterc,
	    &filterv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOObjectSetFilters(oPtr, filterc, filterv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjectMixinGet, ObjectMixinSet --
 *	Implementation of the "mixin" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjMixinGet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->mixins) {
	if (mixinPtr) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    TclOOObjectName(interp, mixinPtr->thisPtr));
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjMixinSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int mixinc;
    Tcl_Obj **mixinv;
    Class **mixins;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &mixinc,
	    &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = TclStackAlloc(interp, sizeof(Class *) * mixinc);

    for (i = 0; i < mixinc; i++) {
	mixins[i] = GetClassInOuterContext(interp, mixinv[i],
		"may only mix in classes");
	if (mixins[i] == NULL) {
	    TclStackFree(interp, mixins);
	    return TCL_ERROR;
	}
    }

    TclOOObjectSetMixins(oPtr, mixinc, mixins);
    TclStackFree(interp, mixins);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2415–2526. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->mixins) {
	if (mixinPtr) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    TclOOObjectName(interp, mixinPtr->thisPtr));
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjMixinSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int mixinc;
    Tcl_Obj **mixinv;
    Class **mixins;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &mixinc,
	    &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = TclStackAlloc(interp, sizeof(Class *) * mixinc);

    for (i = 0; i < mixinc; i++) {
	mixins[i] = GetClassInOuterContext(interp, mixinv[i],
		"may only mix in classes");
	if (mixins[i] == NULL) {
	    TclStackFree(interp, mixins);
	    return TCL_ERROR;
	}
    }

    TclOOObjectSetMixins(oPtr, mixinc, mixins);
    TclStackFree(interp, mixins);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjectVarsGet, ObjectVarsSet --
 *	Implementation of the "variable" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjVarsGet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj, *variableObj;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(variableObj, oPtr->variables) {
	Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjVarsSet(
    ClientData clientData,
    Tcl_Interp *interp,

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2447–2558. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int mixinc;
    Tcl_Obj **mixinv;
    Class **mixins;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &mixinc,
	    &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = TclStackAlloc(interp, sizeof(Class *) * mixinc);

    for (i = 0; i < mixinc; i++) {
	mixins[i] = GetClassInOuterContext(interp, mixinv[i],
		"may only mix in classes");
	if (mixins[i] == NULL) {
	    TclStackFree(interp, mixins);
	    return TCL_ERROR;
	}
    }

    TclOOObjectSetMixins(oPtr, mixinc, mixins);
    TclStackFree(interp, mixins);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjectVarsGet, ObjectVarsSet --
 *	Implementation of the "variable" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjVarsGet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj, *variableObj;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(variableObj, oPtr->variables) {
	Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjVarsSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int varc, i;
    Tcl_Obj **varv, *variableObj;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"variableList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &varc,
	    &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    for (i = 0; i < varc; i++) {
	const char *varName = Tcl_GetString(varv[i]);

	if (strstr(varName, "::") != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "contain namespace separators"));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_DECLVAR", (char *)NULL);
	    return TCL_ERROR;
	}
	if (Tcl_StringMatch(varName, "*(*)")) {

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2500–2612. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj, *variableObj;
    int i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(variableObj, oPtr->variables) {
	Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjVarsSet(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int varc, i;
    Tcl_Obj **varv, *variableObj;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"variableList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &varc,
	    &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    for (i = 0; i < varc; i++) {
	const char *varName = Tcl_GetString(varv[i]);

	if (strstr(varName, "::") != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "contain namespace separators"));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_DECLVAR", (char *)NULL);
	    return TCL_ERROR;
	}
	if (Tcl_StringMatch(varName, "*(*)")) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "refer to an array element"));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_DECLVAR", (char *)NULL);
	    return TCL_ERROR;
	}
    }
    for (i = 0; i < varc; i++) {
	Tcl_IncrRefCount(varv[i]);
    }

    FOREACH(variableObj, oPtr->variables) {
	Tcl_DecrRefCount(variableObj);
    }
    if (i != varc) {
	if (varc == 0) {
	    ckfree((char *) oPtr->variables.list);
	} else if (i) {
	    oPtr->variables.list = (Tcl_Obj **)
		    ckrealloc((char *) oPtr->variables.list,
		    sizeof(Tcl_Obj *) * varc);
	} else {
	    oPtr->variables.list = (Tcl_Obj **)
		    ckalloc(sizeof(Tcl_Obj *) * varc);
	}
    }
    oPtr->variables.num = 0;
    if (varc > 0) {
	int created, n;
	Tcl_HashTable uniqueTable;

	Tcl_InitObjHashTable(&uniqueTable);
	for (i = n = 0; i < varc; i++) {
	    Tcl_CreateHashEntry(&uniqueTable, varv[i], &created);
	    if (created) {
		oPtr->variables.list[n++] = varv[i];
	    } else {
		Tcl_DecrRefCount(varv[i]);
	    }
	}
	oPtr->variables.num = n;

	/*
	 * Shouldn't be necessary, but maintain num/list invariant.
	 */

	oPtr->variables.list = (Tcl_Obj **)
		ckrealloc((char *) oPtr->variables.list,
		sizeof(Tcl_Obj *) * n);
	Tcl_DeleteHashTable(&uniqueTable);
    }
    return TCL_OK;
}


```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2528–2619. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-8.6.18-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int varc, i;
    Tcl_Obj **varv, *variableObj;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"variableList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &varc,
	    &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    for (i = 0; i < varc; i++) {
	const char *varName = Tcl_GetString(varv[i]);

	if (strstr(varName, "::") != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "contain namespace separators"));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_DECLVAR", (char *)NULL);
	    return TCL_ERROR;
	}
	if (Tcl_StringMatch(varName, "*(*)")) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "refer to an array element"));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_DECLVAR", (char *)NULL);
	    return TCL_ERROR;
	}
    }
    for (i = 0; i < varc; i++) {
	Tcl_IncrRefCount(varv[i]);
    }

    FOREACH(variableObj, oPtr->variables) {
	Tcl_DecrRefCount(variableObj);
    }
    if (i != varc) {
	if (varc == 0) {
	    ckfree((char *) oPtr->variables.list);
	} else if (i) {
	    oPtr->variables.list = (Tcl_Obj **)
		    ckrealloc((char *) oPtr->variables.list,
		    sizeof(Tcl_Obj *) * varc);
	} else {
	    oPtr->variables.list = (Tcl_Obj **)
		    ckalloc(sizeof(Tcl_Obj *) * varc);
	}
    }
    oPtr->variables.num = 0;
    if (varc > 0) {
	int created, n;
	Tcl_HashTable uniqueTable;

	Tcl_InitObjHashTable(&uniqueTable);
	for (i = n = 0; i < varc; i++) {
	    Tcl_CreateHashEntry(&uniqueTable, varv[i], &created);
	    if (created) {
		oPtr->variables.list[n++] = varv[i];
	    } else {
		Tcl_DecrRefCount(varv[i]);
	    }
	}
	oPtr->variables.num = n;

	/*
	 * Shouldn't be necessary, but maintain num/list invariant.
	 */

	oPtr->variables.list = (Tcl_Obj **)
		ckrealloc((char *) oPtr->variables.list,
		sizeof(Tcl_Obj *) * n);
	Tcl_DeleteHashTable(&uniqueTable);
    }
    return TCL_OK;
}

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 202–311. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 303–413. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 631–742. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
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

    if (pmPtr->preCallProc != NULL) {
	int isFinished;

	result = pmPtr->preCallProc(pmPtr->clientData, interp, context,
		(Tcl_CallFrame *) fdPtr->framePtr, &isFinished);
	if (isFinished || result != TCL_OK) {
	    Tcl_PopCallFrame(interp);
	    TclStackFree(interp, fdPtr->framePtr);
	    if (pmPtr->refCount-- <= 1) {
		DeleteProcedureMethodRecord(pmPtr);
	    }
	    TclStackFree(interp, fdPtr);
	    return result;
	}
    }

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 1377–1486. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

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
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOGetProcFromMethod, TclOOGetFwdFromMethod --
 *
 *	Utility functions used for procedure-like and forwarding method
 *	introspection.
 *
 * ----------------------------------------------------------------------
 */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod(`, lines 133–244. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
 */

Tcl_Method
Tcl_NewInstanceMethod(
    Tcl_Interp *interp,		/* Unused? */
    Tcl_Object object,		/* The object that has the method attached to
				 * it. */
    Tcl_Obj *nameObj,		/* The name of the method. May be NULL; if so,
				 * up to caller to manage storage (e.g., when
				 * it is a constructor or destructor). */
    int flags,			/* Whether this is a public method. */
    const Tcl_MethodType *typePtr,
				/* The type of method this is, which defines
				 * how to invoke, delete and clone the
				 * method. */
    void *clientData)		/* Some data associated with the particular
				 * method to be created. */
{
    Object *oPtr = (Object *) object;
    Method *mPtr;
    Tcl_HashEntry *hPtr;
    int isNew;

    if (nameObj == NULL) {
	mPtr = (Method *)ckalloc(sizeof(Method));
	mPtr->namePtr = NULL;
	mPtr->refCount = 1;
	goto populate;
    }
    if (!oPtr->methodsPtr) {
	oPtr->methodsPtr = (Tcl_HashTable *)ckalloc(sizeof(Tcl_HashTable));
	Tcl_InitObjHashTable(oPtr->methodsPtr);
	oPtr->flags &= ~USE_CLASS_CACHE;
    }
    hPtr = Tcl_CreateHashEntry(oPtr->methodsPtr, (char *) nameObj, &isNew);
    if (isNew) {
	mPtr = (Method *)ckalloc(sizeof(Method));
	mPtr->namePtr = nameObj;
	mPtr->refCount = 1;
	Tcl_IncrRefCount(nameObj);
	Tcl_SetHashValue(hPtr, mPtr);
    } else {
	mPtr = (Method *)Tcl_GetHashValue(hPtr);
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
	mPtr->flags |= flags & (PUBLIC_METHOD | PRIVATE_METHOD);
    }
    oPtr->epoch++;
    return (Tcl_Method) mPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * Tcl_NewMethod --
 *
 *	Attach a method to a class.
 *
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod(`, lines 518–629. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
	}
    }

    return Tcl_NewInstanceMethod(interp, (Tcl_Object) oPtr, nameObj, flags,
	    typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOOMakeProcMethod --
 *
 *	The guts of the code to make a procedure-like method for a class.
 *	Split apart so that it is easier for other extensions to reuse (in
 *	particular, it frees them from having to pry so deeply into Tcl's
 *	guts).
 *
 * ----------------------------------------------------------------------
 */

Tcl_Method
TclOOMakeProcMethod(
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
    const Tcl_MethodType *typePtr,
				/* The type of the method to create. */
    void *clientData,	/* The per-method type-specific data. */
    Proc **procPtrPtr)		/* A pointer to the variable in which to write
				 * the procedure record reference. Presumably
				 * inside the structure indicated by the
				 * pointer in clientData. */
{
    Interp *iPtr = (Interp *) interp;
    Proc *procPtr;

    if (TclCreateProc(interp, NULL, namePtr, argsObj, bodyObj,
	    procPtrPtr) != TCL_OK) {
	return NULL;
    }
    procPtr = *procPtrPtr;
    procPtr->cmdPtr = NULL;

    if (iPtr->cmdFramePtr) {
	CmdFrame context = *iPtr->cmdFramePtr;

	if (context.type == TCL_LOCATION_BC) {
	    /*
	     * Retrieve source information from the bytecode, if possible. If
	     * the information is retrieved successfully, context.type will be
	     * TCL_LOCATION_SOURCE and the reference held by
	     * context.data.eval.path will be counted.
	     */

	    TclGetSrcInfoForPc(&context);
	} else if (context.type == TCL_LOCATION_SOURCE) {
	    /*
	     * The copy into 'context' up above has created another reference
	     * to 'context.data.eval.path'; account for it.
	     */

	    Tcl_IncrRefCount(context.data.eval.path);
	}

	if (context.type == TCL_LOCATION_SOURCE) {
	    /*
	     * We can account for source location within a proc only if the
	     * proc body was not created by substitution.
	     * (FIXME: check that this is sane and correct!)
	     */

	    if (context.line
		    && (context.nline >= 4) && (context.line[3] >= 0)) {
		int isNew;
		CmdFrame *cfPtr = (CmdFrame *)ckalloc(sizeof(CmdFrame));
		Tcl_HashEntry *hPtr;

		cfPtr->level = -1;
		cfPtr->type = context.type;
		cfPtr->line = (int *)ckalloc(sizeof(int));
		cfPtr->line[0] = context.line[3];
		cfPtr->nline = 1;
		cfPtr->framePtr = NULL;
		cfPtr->nextPtr = NULL;

		cfPtr->data.eval.path = context.data.eval.path;
		Tcl_IncrRefCount(cfPtr->data.eval.path);

		cfPtr->cmd = NULL;
		cfPtr->len = 0;

		hPtr = Tcl_CreateHashEntry(iPtr->linePBodyPtr,
			(char *) procPtr, &isNew);
		Tcl_SetHashValue(hPtr, cfPtr);
	    }

	    /*
	     * 'context' is going out of scope; account for the reference that
	     * it's holding to the path name.
	     */

	    Tcl_DecrRefCount(context.data.eval.path);

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod(`, lines 1338–1447. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
    fmPtr = (ForwardMethod *)ckalloc(sizeof(ForwardMethod));
    fmPtr->prefixObj = prefixObj;
    Tcl_IncrRefCount(prefixObj);
    return (Method *) Tcl_NewInstanceMethod(interp, (Tcl_Object) oPtr,
	    nameObj, flags, &fwdMethodType, fmPtr);
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOONewForwardMethod --
 *
 *	Create a new forwarded method for a class.
 *
 * ----------------------------------------------------------------------
 */

Method *
TclOONewForwardMethod(
    Tcl_Interp *interp,		/* Interpreter for error reporting. */
    Class *clsPtr,		/* The class to attach the method to. */
    int flags,			/* Whether the method is public or not. */
    Tcl_Obj *nameObj,		/* The name of the method. */
    Tcl_Obj *prefixObj)		/* List of arguments that form the command
				 * prefix to forward to. */
{
    int prefixLen;
    ForwardMethod *fmPtr;

    if (TclListObjLength(interp, prefixObj, &prefixLen) != TCL_OK) {
	return NULL;
    }
    if (prefixLen < 1) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"method forward prefix must be non-empty", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "BAD_FORWARD", (char *)NULL);
	return NULL;
    }

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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOMethod.c`, function `TclOODelMethodRef(`, lines 266–375. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--8.6.18--tclOOMethod.c`.

```text
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

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.c`, function `renamemethod`, lines 31–143. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-8.6.18-tclOO.c`.

```text
    {"export", TclOODefineExportObjCmd, 0},
    {"forward", TclOODefineForwardObjCmd, 0},
    {"method", TclOODefineMethodObjCmd, 0},
    {"renamemethod", TclOODefineRenameMethodObjCmd, 0},
    {"self", TclOODefineSelfObjCmd, 0},
    {"unexport", TclOODefineUnexportObjCmd, 0},
    {NULL, NULL, 0}
}, objdefCmds[] = {
    {"class", TclOODefineClassObjCmd, 1},
    {"deletemethod", TclOODefineDeleteMethodObjCmd, 1},
    {"export", TclOODefineExportObjCmd, 1},
    {"forward", TclOODefineForwardObjCmd, 1},
    {"method", TclOODefineMethodObjCmd, 1},
    {"renamemethod", TclOODefineRenameMethodObjCmd, 1},
    {"unexport", TclOODefineUnexportObjCmd, 1},
    {NULL, NULL, 0}
};

/*
 * What sort of size of things we like to allocate.
 */

#define ALLOC_CHUNK 8

/*
 * Function declarations for things defined in this file.
 */

static Object *		AllocObject(Tcl_Interp *interp, const char *nameStr,
			    Namespace *nsPtr, const char *nsNameStr);
static int		CloneClassMethod(Tcl_Interp *interp, Class *clsPtr,
			    Method *mPtr, Tcl_Obj *namePtr,
			    Method **newMPtrPtr);
static int		CloneObjectMethod(Tcl_Interp *interp, Object *oPtr,
			    Method *mPtr, Tcl_Obj *namePtr);
static void		DeletedDefineNamespace(ClientData clientData);
static void		DeletedObjdefNamespace(ClientData clientData);
static void		DeletedHelpersNamespace(ClientData clientData);
static Tcl_NRPostProc	FinalizeAlloc;
static Tcl_NRPostProc	FinalizeNext;
static Tcl_NRPostProc	FinalizeObjectCall;
static void		initClassPath(Tcl_Interp * interp, Class *clsPtr);
static int		InitFoundation(Tcl_Interp *interp);
static void		KillFoundation(ClientData clientData,
			    Tcl_Interp *interp);
static void		MyDeleted(ClientData clientData);
static void		ObjectNamespaceDeleted(ClientData clientData);
static void		ObjectRenamedTrace(ClientData clientData,
			    Tcl_Interp *interp, const char *oldName,
			    const char *newName, int flags);
static inline void	SquelchCachedName(Object *oPtr);

static int		PublicObjectCmd(ClientData clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
static int		PublicNRObjectCmd(ClientData clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
static int		PrivateObjectCmd(ClientData clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
static int		PrivateNRObjectCmd(ClientData clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
static void		RemoveClass(Class ** list, int num, int idx);
static void		RemoveObject(Object ** list, int num, int idx);

/*
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
    DCM("destroy", 1,	TclOO_Object_Destroy),
    DCM("eval", 0,	TclOO_Object_Eval),
    DCM("unknown", 0,	TclOO_Object_Unknown),
    DCM("variable", 0,	TclOO_Object_LinkVar),
    DCM("varname", 0,	TclOO_Object_VarName),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
}, clsMethods[] = {
    DCM("create", 1,	TclOO_Class_Create),
    DCM("new", 1,	TclOO_Class_New),
    DCM("createWithNamespace", 0, TclOO_Class_CreateNs),
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
 * Scripted parts of TclOO. First, the main script (cannot be outside this
 * file).
 */

static const char initScript[] =
"package ifneeded TclOO " TCLOO_PATCHLEVEL " {# Already present, OK?};"
"namespace eval ::oo { variable version " TCLOO_VERSION " };"
"namespace eval ::oo { variable patchlevel " TCLOO_PATCHLEVEL " };";
/* "tcl_findLibrary tcloo $oo::version $oo::version" */

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.c`, function `renamemethod`, lines 41–153. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-8.6.18-tclOO.c`.

```text
    {"export", TclOODefineExportObjCmd, 1},
    {"forward", TclOODefineForwardObjCmd, 1},
    {"method", TclOODefineMethodObjCmd, 1},
    {"renamemethod", TclOODefineRenameMethodObjCmd, 1},
    {"unexport", TclOODefineUnexportObjCmd, 1},
    {NULL, NULL, 0}
};

/*
 * What sort of size of things we like to allocate.
 */

#define ALLOC_CHUNK 8

/*
 * Function declarations for things defined in this file.
 */

static Object *		AllocObject(Tcl_Interp *interp, const char *nameStr,
			    Namespace *nsPtr, const char *nsNameStr);
static int		CloneClassMethod(Tcl_Interp *interp, Class *clsPtr,
			    Method *mPtr, Tcl_Obj *namePtr,
			    Method **newMPtrPtr);
static int		CloneObjectMethod(Tcl_Interp *interp, Object *oPtr,
			    Method *mPtr, Tcl_Obj *namePtr);
static void		DeletedDefineNamespace(ClientData clientData);
static void		DeletedObjdefNamespace(ClientData clientData);
static void		DeletedHelpersNamespace(ClientData clientData);
static Tcl_NRPostProc	FinalizeAlloc;
static Tcl_NRPostProc	FinalizeNext;
static Tcl_NRPostProc	FinalizeObjectCall;
static void		initClassPath(Tcl_Interp * interp, Class *clsPtr);
static int		InitFoundation(Tcl_Interp *interp);
static void		KillFoundation(ClientData clientData,
			    Tcl_Interp *interp);
static void		MyDeleted(ClientData clientData);
static void		ObjectNamespaceDeleted(ClientData clientData);
static void		ObjectRenamedTrace(ClientData clientData,
			    Tcl_Interp *interp, const char *oldName,
			    const char *newName, int flags);
static inline void	SquelchCachedName(Object *oPtr);

static int		PublicObjectCmd(ClientData clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
static int		PublicNRObjectCmd(ClientData clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
static int		PrivateObjectCmd(ClientData clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
static int		PrivateNRObjectCmd(ClientData clientData,
			    Tcl_Interp *interp, int objc,
			    Tcl_Obj *const *objv);
static void		RemoveClass(Class ** list, int num, int idx);
static void		RemoveObject(Object ** list, int num, int idx);

/*
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
    DCM("destroy", 1,	TclOO_Object_Destroy),
    DCM("eval", 0,	TclOO_Object_Eval),
    DCM("unknown", 0,	TclOO_Object_Unknown),
    DCM("variable", 0,	TclOO_Object_LinkVar),
    DCM("varname", 0,	TclOO_Object_VarName),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
}, clsMethods[] = {
    DCM("create", 1,	TclOO_Class_Create),
    DCM("new", 1,	TclOO_Class_New),
    DCM("createWithNamespace", 0, TclOO_Class_CreateNs),
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
 * Scripted parts of TclOO. First, the main script (cannot be outside this
 * file).
 */

static const char initScript[] =
"package ifneeded TclOO " TCLOO_PATCHLEVEL " {# Already present, OK?};"
"namespace eval ::oo { variable version " TCLOO_VERSION " };"
"namespace eval ::oo { variable patchlevel " TCLOO_PATCHLEVEL " };";
/* "tcl_findLibrary tcloo $oo::version $oo::version" */
/* " tcloo.tcl OO_LIBRARY oo::library;"; */

/*
 * The scripted part of the definitions of slots.
 */

static const char *slotScript =
"::oo::define ::oo::Slot {\n"
"    method Get {} {error unimplemented}\n"
"    method Set list {error unimplemented}\n"

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 77–147. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOO.h`.

```text

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

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 98–147. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOO.h`.

```text
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

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tcl.h`, function `Information about commands`, lines 952–1064. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-8.6.18-generic-tcl.h`.

```text

/*
 *----------------------------------------------------------------------------
 * Information about commands that is returned by Tcl_GetCommandInfo and
 * passed to Tcl_SetCommandInfo. objProc is an objc/objv object-based command
 * function while proc is a traditional Tcl argc/argv string-based function.
 * Tcl_CreateObjCommand and Tcl_CreateCommand ensure that both objProc and
 * proc are non-NULL and can be called to execute the command. However, it may
 * be faster to call one instead of the other. The member isNativeObjectProc
 * is set to 1 if an object-based function was registered by
 * Tcl_CreateObjCommand, and to 0 if a string-based function was registered by
 * Tcl_CreateCommand. The other function is typically set to a compatibility
 * wrapper that does string-to-object or object-to-string argument conversions
 * then calls the other function.
 */

typedef struct Tcl_CmdInfo {
    int isNativeObjectProc;	/* 1 if objProc was registered by a call to
				 * Tcl_CreateObjCommand; 0 otherwise.
				 * Tcl_SetCmdInfo does not modify this
				 * field. */
    Tcl_ObjCmdProc *objProc;	/* Command's object-based function. */
    ClientData objClientData;	/* ClientData for object proc. */
    Tcl_CmdProc *proc;		/* Command's string-based function. */
    ClientData clientData;	/* ClientData for string proc. */
    Tcl_CmdDeleteProc *deleteProc;
				/* Function to call when command is
				 * deleted. */
    ClientData deleteData;	/* Value to pass to deleteProc (usually the
				 * same as clientData). */
    Tcl_Namespace *namespacePtr;/* Points to the namespace that contains this
				 * command. Note that Tcl_SetCmdInfo will not
				 * change a command's namespace; use
				 * TclRenameCommand or Tcl_Eval (of 'rename')
				 * to do that. */
} Tcl_CmdInfo;

/*
 *----------------------------------------------------------------------------
 * The structure defined below is used to hold dynamic strings. The only
 * fields that clients should use are string and length, accessible via the
 * macros Tcl_DStringValue and Tcl_DStringLength.
 */

#define TCL_DSTRING_STATIC_SIZE 200
typedef struct Tcl_DString {
    char *string;		/* Points to beginning of string: either
				 * staticSpace below or a malloced array. */
    int length;			/* Number of non-NULL characters in the
				 * string. */
    int spaceAvl;		/* Total number of bytes available for the
				 * string and its terminating NULL char. */
    char staticSpace[TCL_DSTRING_STATIC_SIZE];
				/* Space to use in common case where string is
				 * small. */
} Tcl_DString;

#define Tcl_DStringLength(dsPtr) ((dsPtr)->length)
#define Tcl_DStringValue(dsPtr) ((dsPtr)->string)
#define Tcl_DStringTrunc Tcl_DStringSetLength

/*
 * Definitions for the maximum number of digits of precision that may be
 * specified in the "tcl_precision" variable, and the number of bytes of
 * buffer space required by Tcl_PrintDouble.
 */

#define TCL_MAX_PREC		17
#define TCL_DOUBLE_SPACE	(TCL_MAX_PREC+10)

/*
 * Definition for a number of bytes of buffer space sufficient to hold the
 * string representation of an integer in base 10 (assuming the existence of
 * 64-bit integers).
 */

#define TCL_INTEGER_SPACE	24

/*
 * Flag values passed to Tcl_ConvertElement.
 * TCL_DONT_USE_BRACES forces it not to enclose the element in braces, but to
 *	use backslash quoting instead.
 * TCL_DONT_QUOTE_HASH disables the default quoting of the '#' character. It
 *	is safe to leave the hash unquoted when the element is not the first
 *	element of a list, and this flag can be used by the caller to indicate
 *	that condition.
 */

#define TCL_DONT_USE_BRACES	1
#define TCL_DONT_QUOTE_HASH	8

/*
 * Flag that may be passed to Tcl_GetIndexFromObj to force it to disallow
 * abbreviated strings.
 */

#define TCL_EXACT	1

/*
 *----------------------------------------------------------------------------
 * Flag values passed to Tcl_RecordAndEval, Tcl_EvalObj, Tcl_EvalObjv.
 * WARNING: these bit choices must not conflict with the bit choices for
 * evalFlag bits in tclInt.h!
 *
 * Meanings:
 *	TCL_NO_EVAL:		Just record this command
 *	TCL_EVAL_GLOBAL:	Execute script in global namespace
 *	TCL_EVAL_DIRECT:	Do not compile this script
 *	TCL_EVAL_INVOKE:	Magical Tcl_EvalObjv mode for aliases/ensembles
 *				o Run in iPtr->lookupNsPtr or global namespace
 *				o Cut out of error traces
 *				o Don't reset the flags controlling ensemble
 *				  error message rewriting.

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tcl.h`, function `typedef struct Tcl_Obj`, lines 791–902. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-8.6.18-generic-tcl.h`.

```text
 * standard operations on objects of that type.
 */

typedef struct Tcl_ObjType {
    const char *name;		/* Name of the type, e.g. "int". */
    Tcl_FreeInternalRepProc *freeIntRepProc;
				/* Called to free any storage for the type's
				 * internal rep. NULL if the internal rep does
				 * not need freeing. */
    Tcl_DupInternalRepProc *dupIntRepProc;
				/* Called to create a new object as a copy of
				 * an existing object. */
    Tcl_UpdateStringProc *updateStringProc;
				/* Called to update the string rep from the
				 * type's internal representation. */
    Tcl_SetFromAnyProc *setFromAnyProc;
				/* Called to convert the object's internal rep
				 * to this type. Frees the internal rep of the
				 * old type. Returns TCL_ERROR on failure. */
} Tcl_ObjType;

/*
 * One of the following structures exists for each object in the Tcl system.
 * An object stores a value as either a string, some internal representation,
 * or both.
 */

typedef struct Tcl_Obj {
    int refCount;		/* When 0 the object will be freed. */
    char *bytes;		/* This points to the first byte of the
				 * object's string representation. The array
				 * must be followed by a null byte (i.e., at
				 * offset length) but may also contain
				 * embedded null characters. The array's
				 * storage is allocated by ckalloc. NULL means
				 * the string rep is invalid and must be
				 * regenerated from the internal rep.  Clients
				 * should use Tcl_GetStringFromObj or
				 * Tcl_GetString to get a pointer to the byte
				 * array as a readonly value. */
    int length;			/* The number of bytes at *bytes, not
				 * including the terminating null. */
    const Tcl_ObjType *typePtr;	/* Denotes the object's type. Always
				 * corresponds to the type of the object's
				 * internal rep. NULL indicates the object has
				 * no internal rep (has no type). */
    union {			/* The internal representation: */
	long longValue;		/*   - an long integer value. */
	double doubleValue;	/*   - a double-precision floating value. */
	void *otherValuePtr;	/*   - another, type-specific value,
	                       not used internally any more. */
	Tcl_WideInt wideValue;	/*   - a long long value. */
	struct {		/*   - internal rep as two pointers.
				 *     the main use of which is a bignum's
				 *     tightly packed fields, where the alloc,
				 *     used and signum flags are packed into
				 *     ptr2 with everything else hung off ptr1. */
	    void *ptr1;
	    void *ptr2;
	} twoPtrValue;
	struct {		/*   - internal rep as a pointer and a long,
	                       not used internally any more. */
	    void *ptr;
	    unsigned long value;
	} ptrAndLongRep;
    } internalRep;
} Tcl_Obj;

/*
 * Macros to increment and decrement a Tcl_Obj's reference count, and to test
 * whether an object is shared (i.e. has reference count > 1). Note: clients
 * should use Tcl_DecrRefCount() when they are finished using an object, and
 * should never call TclFreeObj() directly. TclFreeObj() is only defined and
 * made public in tcl.h to support Tcl_DecrRefCount's macro definition.
 */

void		Tcl_IncrRefCount(Tcl_Obj *objPtr);
void		Tcl_DecrRefCount(Tcl_Obj *objPtr);
int		Tcl_IsShared(Tcl_Obj *objPtr);

/*
 *----------------------------------------------------------------------------
 * The following structure contains the state needed by Tcl_SaveResult. No-one
 * outside of Tcl should access any of these fields. This structure is
 * typically allocated on the stack.
 */

typedef struct Tcl_SavedResult {
    char *result;
    Tcl_FreeProc *freeProc;
    Tcl_Obj *objResultPtr;
    char *appendResult;
    int appendAvl;
    int appendUsed;
    char resultSpace[TCL_RESULT_SIZE+1];
} Tcl_SavedResult;

/*
 *----------------------------------------------------------------------------
 * The following definitions support Tcl's namespace facility. Note: the first
 * five fields must match exactly the fields in a Namespace structure (see
 * tclInt.h).
 */

typedef struct Tcl_Namespace {
    char *name;			/* The namespace's name within its parent
				 * namespace. This contains no ::'s. The name
				 * of the global namespace is "" although "::"
				 * is an synonym. */
    char *fullName;		/* The namespace's fully qualified name. This
				 * starts with ::. */
    ClientData clientData;	/* Arbitrary value associated with this

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tcl.h`, function `typedef struct Tcl_Obj`, lines 815–926. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-8.6.18-generic-tcl.h`.

```text
 * or both.
 */

typedef struct Tcl_Obj {
    int refCount;		/* When 0 the object will be freed. */
    char *bytes;		/* This points to the first byte of the
				 * object's string representation. The array
				 * must be followed by a null byte (i.e., at
				 * offset length) but may also contain
				 * embedded null characters. The array's
				 * storage is allocated by ckalloc. NULL means
				 * the string rep is invalid and must be
				 * regenerated from the internal rep.  Clients
				 * should use Tcl_GetStringFromObj or
				 * Tcl_GetString to get a pointer to the byte
				 * array as a readonly value. */
    int length;			/* The number of bytes at *bytes, not
				 * including the terminating null. */
    const Tcl_ObjType *typePtr;	/* Denotes the object's type. Always
				 * corresponds to the type of the object's
				 * internal rep. NULL indicates the object has
				 * no internal rep (has no type). */
    union {			/* The internal representation: */
	long longValue;		/*   - an long integer value. */
	double doubleValue;	/*   - a double-precision floating value. */
	void *otherValuePtr;	/*   - another, type-specific value,
	                       not used internally any more. */
	Tcl_WideInt wideValue;	/*   - a long long value. */
	struct {		/*   - internal rep as two pointers.
				 *     the main use of which is a bignum's
				 *     tightly packed fields, where the alloc,
				 *     used and signum flags are packed into
				 *     ptr2 with everything else hung off ptr1. */
	    void *ptr1;
	    void *ptr2;
	} twoPtrValue;
	struct {		/*   - internal rep as a pointer and a long,
	                       not used internally any more. */
	    void *ptr;
	    unsigned long value;
	} ptrAndLongRep;
    } internalRep;
} Tcl_Obj;

/*
 * Macros to increment and decrement a Tcl_Obj's reference count, and to test
 * whether an object is shared (i.e. has reference count > 1). Note: clients
 * should use Tcl_DecrRefCount() when they are finished using an object, and
 * should never call TclFreeObj() directly. TclFreeObj() is only defined and
 * made public in tcl.h to support Tcl_DecrRefCount's macro definition.
 */

void		Tcl_IncrRefCount(Tcl_Obj *objPtr);
void		Tcl_DecrRefCount(Tcl_Obj *objPtr);
int		Tcl_IsShared(Tcl_Obj *objPtr);

/*
 *----------------------------------------------------------------------------
 * The following structure contains the state needed by Tcl_SaveResult. No-one
 * outside of Tcl should access any of these fields. This structure is
 * typically allocated on the stack.
 */

typedef struct Tcl_SavedResult {
    char *result;
    Tcl_FreeProc *freeProc;
    Tcl_Obj *objResultPtr;
    char *appendResult;
    int appendAvl;
    int appendUsed;
    char resultSpace[TCL_RESULT_SIZE+1];
} Tcl_SavedResult;

/*
 *----------------------------------------------------------------------------
 * The following definitions support Tcl's namespace facility. Note: the first
 * five fields must match exactly the fields in a Namespace structure (see
 * tclInt.h).
 */

typedef struct Tcl_Namespace {
    char *name;			/* The namespace's name within its parent
				 * namespace. This contains no ::'s. The name
				 * of the global namespace is "" although "::"
				 * is an synonym. */
    char *fullName;		/* The namespace's fully qualified name. This
				 * starts with ::. */
    ClientData clientData;	/* Arbitrary value associated with this
				 * namespace. */
    Tcl_NamespaceDeleteProc *deleteProc;
				/* Function invoked when deleting the
				 * namespace to, e.g., free clientData. */
    struct Tcl_Namespace *parentPtr;
				/* Points to the namespace that contains this
				 * one. NULL if this is the global
				 * namespace. */
} Tcl_Namespace;

/*
 *----------------------------------------------------------------------------
 * The following structure represents a call frame, or activation record. A
 * call frame defines a naming context for a procedure call: its local scope
 * (for local variables) and its namespace scope (used for non-local
 * variables; often the global :: namespace). A call frame can also define the
 * naming context for a namespace eval or namespace inscope command: the
 * namespace in which the command's code should execute. The Tcl_CallFrame
 * structures exist only while procedures or namespace eval/inscope's are
 * being executed, and provide a Tcl call stack.
 *
 * A call frame is initialized and pushed using Tcl_PushCallFrame and popped
 * using Tcl_PopCallFrame. Storage for a Tcl_CallFrame must be provided by the
 * Tcl_PushCallFrame caller, and callers typically allocate them on the C call

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_GetCommandInfo(`, lines 2935–3044. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
 */

int
Tcl_GetCommandInfo(
    Tcl_Interp *interp,		/* Interpreter in which to look for
				 * command. */
    const char *cmdName,	/* Name of desired command. */
    Tcl_CmdInfo *infoPtr)	/* Where to store information about
				 * command. */
{
    Tcl_Command cmd;

    cmd = Tcl_FindCommand(interp, cmdName, NULL, /*flags*/ 0);
    return Tcl_GetCommandInfoFromToken(cmd, infoPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandInfoFromToken --
 *
 *	Returns various information about a Tcl command.
 *
 * Results:
 *	Copies information from the command identified by 'cmd' into a
 *	caller-supplied structure and returns 1. If the 'cmd' is NULL, leaves
 *	the structure untouched and returns 0.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_GetCommandInfoFromToken(
    Tcl_Command cmd,
    Tcl_CmdInfo *infoPtr)
{
    Command *cmdPtr;		/* Internal representation of the command */

    if (cmd == NULL) {
	return 0;
    }

    /*
     * Set isNativeObjectProc 1 if objProc was registered by a call to
     * Tcl_CreateObjCommand. Otherwise set it to 0.
     */

    cmdPtr = (Command *) cmd;
    infoPtr->isNativeObjectProc =
	    (cmdPtr->objProc != TclInvokeStringCommand);
    infoPtr->objProc = cmdPtr->objProc;
    infoPtr->objClientData = cmdPtr->objClientData;
    infoPtr->proc = cmdPtr->proc;
    infoPtr->clientData = cmdPtr->clientData;
    infoPtr->deleteProc = cmdPtr->deleteProc;
    infoPtr->deleteData = cmdPtr->deleteData;
    infoPtr->namespacePtr = (Tcl_Namespace *) cmdPtr->nsPtr;
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandName --
 *
 *	Given a token returned by Tcl_CreateCommand, this function returns the
 *	current name of the command (which may have changed due to renaming).
 *
 * Results:
 *	The return value is the name of the given command.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_GetCommandName(
    Tcl_Interp *interp,		/* Interpreter containing the command. */
    Tcl_Command command)	/* Token for command returned by a previous
				 * call to Tcl_CreateCommand. The command must
				 * not have been deleted. */
{
    Command *cmdPtr = (Command *) command;

    if ((cmdPtr == NULL) || (cmdPtr->hPtr == NULL)) {
	/*
	 * This should only happen if command was "created" after the
	 * interpreter began to be deleted, so there isn't really any command.
	 * Just return an empty string.
	 */

	return "";
    }

    return (const char *)Tcl_GetHashKey(cmdPtr->hPtr->tablePtr, cmdPtr->hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFullName --
 *
 *	Given a token returned by, e.g., Tcl_CreateCommand or Tcl_FindCommand,
 *	this function appends to an object the command's full name, qualified
 *	by a sequence of parent namespace names. The command's fully-qualified

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_GetCommandInfoFromToken(`, lines 2945–3054. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
    Tcl_Command cmd;

    cmd = Tcl_FindCommand(interp, cmdName, NULL, /*flags*/ 0);
    return Tcl_GetCommandInfoFromToken(cmd, infoPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandInfoFromToken --
 *
 *	Returns various information about a Tcl command.
 *
 * Results:
 *	Copies information from the command identified by 'cmd' into a
 *	caller-supplied structure and returns 1. If the 'cmd' is NULL, leaves
 *	the structure untouched and returns 0.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_GetCommandInfoFromToken(
    Tcl_Command cmd,
    Tcl_CmdInfo *infoPtr)
{
    Command *cmdPtr;		/* Internal representation of the command */

    if (cmd == NULL) {
	return 0;
    }

    /*
     * Set isNativeObjectProc 1 if objProc was registered by a call to
     * Tcl_CreateObjCommand. Otherwise set it to 0.
     */

    cmdPtr = (Command *) cmd;
    infoPtr->isNativeObjectProc =
	    (cmdPtr->objProc != TclInvokeStringCommand);
    infoPtr->objProc = cmdPtr->objProc;
    infoPtr->objClientData = cmdPtr->objClientData;
    infoPtr->proc = cmdPtr->proc;
    infoPtr->clientData = cmdPtr->clientData;
    infoPtr->deleteProc = cmdPtr->deleteProc;
    infoPtr->deleteData = cmdPtr->deleteData;
    infoPtr->namespacePtr = (Tcl_Namespace *) cmdPtr->nsPtr;
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandName --
 *
 *	Given a token returned by Tcl_CreateCommand, this function returns the
 *	current name of the command (which may have changed due to renaming).
 *
 * Results:
 *	The return value is the name of the given command.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_GetCommandName(
    Tcl_Interp *interp,		/* Interpreter containing the command. */
    Tcl_Command command)	/* Token for command returned by a previous
				 * call to Tcl_CreateCommand. The command must
				 * not have been deleted. */
{
    Command *cmdPtr = (Command *) command;

    if ((cmdPtr == NULL) || (cmdPtr->hPtr == NULL)) {
	/*
	 * This should only happen if command was "created" after the
	 * interpreter began to be deleted, so there isn't really any command.
	 * Just return an empty string.
	 */

	return "";
    }

    return (const char *)Tcl_GetHashKey(cmdPtr->hPtr->tablePtr, cmdPtr->hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFullName --
 *
 *	Given a token returned by, e.g., Tcl_CreateCommand or Tcl_FindCommand,
 *	this function appends to an object the command's full name, qualified
 *	by a sequence of parent namespace names. The command's fully-qualified
 *	name may have changed due to renaming.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	The command's fully-qualified name is appended to the string
 *	representation of objPtr.
 *
 *----------------------------------------------------------------------

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_GetCommandInfoFromToken(`, lines 2967–3077. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
 */

int
Tcl_GetCommandInfoFromToken(
    Tcl_Command cmd,
    Tcl_CmdInfo *infoPtr)
{
    Command *cmdPtr;		/* Internal representation of the command */

    if (cmd == NULL) {
	return 0;
    }

    /*
     * Set isNativeObjectProc 1 if objProc was registered by a call to
     * Tcl_CreateObjCommand. Otherwise set it to 0.
     */

    cmdPtr = (Command *) cmd;
    infoPtr->isNativeObjectProc =
	    (cmdPtr->objProc != TclInvokeStringCommand);
    infoPtr->objProc = cmdPtr->objProc;
    infoPtr->objClientData = cmdPtr->objClientData;
    infoPtr->proc = cmdPtr->proc;
    infoPtr->clientData = cmdPtr->clientData;
    infoPtr->deleteProc = cmdPtr->deleteProc;
    infoPtr->deleteData = cmdPtr->deleteData;
    infoPtr->namespacePtr = (Tcl_Namespace *) cmdPtr->nsPtr;
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandName --
 *
 *	Given a token returned by Tcl_CreateCommand, this function returns the
 *	current name of the command (which may have changed due to renaming).
 *
 * Results:
 *	The return value is the name of the given command.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_GetCommandName(
    Tcl_Interp *interp,		/* Interpreter containing the command. */
    Tcl_Command command)	/* Token for command returned by a previous
				 * call to Tcl_CreateCommand. The command must
				 * not have been deleted. */
{
    Command *cmdPtr = (Command *) command;

    if ((cmdPtr == NULL) || (cmdPtr->hPtr == NULL)) {
	/*
	 * This should only happen if command was "created" after the
	 * interpreter began to be deleted, so there isn't really any command.
	 * Just return an empty string.
	 */

	return "";
    }

    return (const char *)Tcl_GetHashKey(cmdPtr->hPtr->tablePtr, cmdPtr->hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFullName --
 *
 *	Given a token returned by, e.g., Tcl_CreateCommand or Tcl_FindCommand,
 *	this function appends to an object the command's full name, qualified
 *	by a sequence of parent namespace names. The command's fully-qualified
 *	name may have changed due to renaming.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	The command's fully-qualified name is appended to the string
 *	representation of objPtr.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_GetCommandFullName(
    Tcl_Interp *interp,		/* Interpreter containing the command. */
    Tcl_Command command,	/* Token for command returned by a previous
				 * call to Tcl_CreateCommand. The command must
				 * not have been deleted. */
    Tcl_Obj *objPtr)		/* Points to the object onto which the
				 * command's full name is appended. */

{
    Interp *iPtr = (Interp *) interp;
    Command *cmdPtr = (Command *) command;
    char *name;

    /*
     * Add the full name of the containing namespace, followed by the "::"
     * separator, and the command name.
     */

    if (cmdPtr != NULL) {
	if (cmdPtr->nsPtr != NULL) {

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclObj.c`, function `AllocObjEntry(`, lines 225–337. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-8.6.18-tclObj.c`.

```text
 * Prototypes for the array hash key methods.
 */

static Tcl_HashEntry *	AllocObjEntry(Tcl_HashTable *tablePtr, void *keyPtr);

/*
 * Prototypes for the CommandName object type.
 */

static void		DupCmdNameInternalRep(Tcl_Obj *objPtr,
			    Tcl_Obj *copyPtr);
static void		FreeCmdNameInternalRep(Tcl_Obj *objPtr);
static int		SetCmdNameFromAny(Tcl_Interp *interp, Tcl_Obj *objPtr);

/*
 * The structures below defines the Tcl object types defined in this file by
 * means of functions that can be invoked by generic object code. See also
 * tclStringObj.c, tclListObj.c, tclByteCode.c for other type manager
 * implementations.
 */

static const Tcl_ObjType oldBooleanType = {
    "boolean",			/* name */
    NULL,			/* freeIntRepProc */
    NULL,			/* dupIntRepProc */
    NULL,			/* updateStringProc */
    TclSetBooleanFromAny		/* setFromAnyProc */
};
const Tcl_ObjType tclBooleanType = {
    "booleanString",		/* name */
    NULL,			/* freeIntRepProc */
    NULL,			/* dupIntRepProc */
    NULL,			/* updateStringProc */
    TclSetBooleanFromAny		/* setFromAnyProc */
};
const Tcl_ObjType tclDoubleType = {
    "double",			/* name */
    NULL,			/* freeIntRepProc */
    NULL,			/* dupIntRepProc */
    UpdateStringOfDouble,	/* updateStringProc */
    SetDoubleFromAny		/* setFromAnyProc */
};
const Tcl_ObjType tclIntType = {
    "int",			/* name */
    NULL,			/* freeIntRepProc */
    NULL,			/* dupIntRepProc */
    UpdateStringOfInt,		/* updateStringProc */
    SetIntFromAny		/* setFromAnyProc */
};
#ifndef TCL_WIDE_INT_IS_LONG
const Tcl_ObjType tclWideIntType = {
    "wideInt",			/* name */
    NULL,			/* freeIntRepProc */
    NULL,			/* dupIntRepProc */
    UpdateStringOfWideInt,	/* updateStringProc */
    SetWideIntFromAny		/* setFromAnyProc */
};
#endif
const Tcl_ObjType tclBignumType = {
    "bignum",			/* name */
    FreeBignum,			/* freeIntRepProc */
    DupBignum,			/* dupIntRepProc */
    UpdateStringOfBignum,	/* updateStringProc */
    NULL			/* setFromAnyProc */
};

/*
 * The structure below defines the Tcl obj hash key type.
 */

const Tcl_HashKeyType tclObjHashKeyType = {
    TCL_HASH_KEY_TYPE_VERSION,	/* version */
    TCL_HASH_KEY_DIRECT_COMPARE,/* allows compare keys by pointers */
    TclHashObjKey,		/* hashKeyProc */
    TclCompareObjKeys,		/* compareKeysProc */
    AllocObjEntry,		/* allocEntryProc */
    TclFreeObjEntry		/* freeEntryProc */
};

/*
 * The structure below defines the command name Tcl object type by means of
 * functions that can be invoked by generic object code. Objects of this type
 * cache the Command pointer that results from looking up command names in the
 * command hashtable. Such objects appear as the zeroth ("command name")
 * argument in a Tcl command.
 *
 * NOTE: the ResolvedCmdName that gets cached is stored in the
 * twoPtrValue.ptr1 field, and the twoPtrValue.ptr2 field is unused. You might
 * think you could use the simpler otherValuePtr field to store the single
 * ResolvedCmdName pointer, but DO NOT DO THIS. It seems that some extensions
 * use the second internal pointer field of the twoPtrValue field for their
 * own purposes.
 *
 * TRICKY POINT! Some extensions update this structure! (Notably, these
 * include TclBlend and TCom). This is highly ill-advised on their part, but
 * does allow them to delete a command when references to it are gone, which
 * is fragile but useful given their somewhat-OO style. Because of this, this
 * structure MUST NOT be const so that the C compiler puts the data in
 * writable memory. [Bug 2558422] [Bug 07d13d99b0a9]
 * TODO: Provide a better API for those extensions so that they can coexist...
 */

Tcl_ObjType tclCmdNameType = {
    "cmdName",			/* name */
    FreeCmdNameInternalRep,	/* freeIntRepProc */
    DupCmdNameInternalRep,	/* dupIntRepProc */
    NULL,			/* updateStringProc */
    SetCmdNameFromAny		/* setFromAnyProc */
};

/*
 * Structure containing a cached pointer to a command that is the result of
 * resolving the command's name in some namespace. It is the internal

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclObj.c`, function `AllocObjEntry(`, lines 3942–4051. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-8.6.18-tclObj.c`.

```text
 */

static Tcl_HashEntry *
AllocObjEntry(
    Tcl_HashTable *tablePtr,	/* Hash table. */
    void *keyPtr)		/* Key to store in the hash table entry. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    Tcl_HashEntry *hPtr = (Tcl_HashEntry *)ckalloc(sizeof(Tcl_HashEntry));

    hPtr->key.objPtr = objPtr;
    Tcl_IncrRefCount(objPtr);
    hPtr->clientData = NULL;

    return hPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * TclCompareObjKeys --
 *
 *	Compares two Tcl_Obj * keys.
 *
 * Results:
 *	The return value is 0 if they are different and 1 if they are the
 *	same.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

int
TclCompareObjKeys(
    void *keyPtr,		/* New key to compare. */
    Tcl_HashEntry *hPtr)	/* Existing key to compare. */
{
    Tcl_Obj *objPtr1 = (Tcl_Obj *)keyPtr;
    Tcl_Obj *objPtr2 = (Tcl_Obj *)hPtr->key.oneWordValue;
    const char *p1, *p2;
    size_t l1, l2;

    /*
     * If the object pointers are the same then they match.
     * OPT: this comparison was moved to the caller

       if (objPtr1 == objPtr2) {
	   return 1;
       }
    */

    /*
     * Don't use Tcl_GetStringFromObj as it would prevent l1 and l2 being
     * in a register.
     */

    p1 = TclGetString(objPtr1);
    l1 = objPtr1->length;
    p2 = TclGetString(objPtr2);
    l2 = objPtr2->length;

    /*
     * Only compare if the string representations are of the same length.
     */

    if (l1 == l2) {
	for (;; p1++, p2++, l1--) {
	    if (*p1 != *p2) {
		break;
	    }
	    if (l1 == 0) {
		return 1;
	    }
	}
    }

    return 0;
}

/*
 *----------------------------------------------------------------------
 *
 * TclFreeObjEntry --
 *
 *	Frees space for a Tcl_HashEntry containing the Tcl_Obj * key.
 *
 * Results:
 *	The return value is a pointer to the created entry.
 *
 * Side effects:
 *	Decrements the reference count of the object.
 *
 *----------------------------------------------------------------------
 */

void
TclFreeObjEntry(
    Tcl_HashEntry *hPtr)	/* Hash entry to free. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *) hPtr->key.oneWordValue;

    Tcl_DecrRefCount(objPtr);
    ckfree(hPtr);
}

/*
 *----------------------------------------------------------------------
 *

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclObj.c`, function `HashObjKey(`, lines 4065–4176. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-8.6.18-tclObj.c`.

```text
 */

unsigned int
TclHashObjKey(
    Tcl_HashTable *tablePtr,	/* Hash table. */
    void *keyPtr)		/* Key from which to compute hash value. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    int length;
    const char *string = TclGetStringFromObj(objPtr, &length);
    unsigned int result = 0;

    /*
     * I tried a zillion different hash functions and asked many other people
     * for advice. Many people had their own favorite functions, all
     * different, but no-one had much idea why they were good ones. I chose
     * the one below (multiply by 9 and add new character) because of the
     * following reasons:
     *
     * 1. Multiplying by 10 is perfect for keys that are decimal strings, and
     *    multiplying by 9 is just about as good.
     * 2. Times-9 is (shift-left-3) plus (old). This means that each
     *    character's bits hang around in the low-order bits of the hash value
     *    for ever, plus they spread fairly rapidly up to the high-order bits
     *    to fill out the hash value. This seems works well both for decimal
     *    and non-decimal strings.
     *
     * Note that this function is very weak against malicious strings; it's
     * very easy to generate multiple keys that have the same hashcode. On the
     * other hand, that hardly ever actually occurs and this function *is*
     * very cheap, even by comparison with industry-standard hashes like FNV.
     * If real strength of hash is required though, use a custom hash based on
     * Bob Jenkins's lookup3(), but be aware that it's significantly slower.
     * Tcl does not use that level of strength because it typically does not
     * need it (and some of the aspects of that strength are genuinely
     * unnecessary given the rest of Tcl's hash machinery, and the fact that
     * we do not either transfer hashes to another machine, use them as a true
     * substitute for equality, or attempt to minimize work in rebuilding the
     * hash table).
     *
     * See also HashStringKey in tclHash.c.
     * See also HashString in tclLiteral.c.
     *
     * See [tcl-Feature Request #2958832]
     */

    if (length > 0) {
	result = UCHAR(*string);
	while (--length) {
	    result += (result << 3) + UCHAR(*++string);
	}
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFromObj --
 *
 *	Returns the command specified by the name in a Tcl_Obj.
 *
 * Results:
 *	Returns a token for the command if it is found. Otherwise, if it can't
 *	be found or there is an error, returns NULL.
 *
 * Side effects:
 *	May update the internal representation for the object, caching the
 *	command reference so that the next time this function is called with
 *	the same object, the command can be found quickly.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
Tcl_GetCommandFromObj(
    Tcl_Interp *interp,		/* The interpreter in which to resolve the
				 * command and to report errors. */
    Tcl_Obj *objPtr)	/* The object containing the command's name.
				 * If the name starts with "::", will be
				 * looked up in global namespace. Else, looked
				 * up first in the current namespace, then in
				 * global namespace. */
{
    ResolvedCmdName *resPtr;

    /*
     * Get the internal representation, converting to a command type if
     * needed. The internal representation is a ResolvedCmdName that points to
     * the actual command.
     *
     * Check the context namespace and the namespace epoch of the resolved
     * symbol to make sure that it is fresh. Note that we verify that the
     * namespace id of the context namespace is the same as the one we cached;
     * this insures that the namespace wasn't deleted and a new one created at
     * the same address with the same command epoch. Note that fully qualified
     * names have a NULL refNsPtr, these checks needn't be made.
     *
     * Check also that the command's epoch is up to date, and that the command
     * is not deleted.
     *
     * If any check fails, then force another conversion to the command type,
     * to discard the old rep and create a new one.
     */

    resPtr = (ResolvedCmdName *)objPtr->internalRep.twoPtrValue.ptr1;
    if ((objPtr->typePtr == &tclCmdNameType) && (resPtr != NULL)) {
	Command *cmdPtr = resPtr->cmdPtr;

	if ((cmdPtr->cmdEpoch == resPtr->cmdEpoch)
		&& !(cmdPtr->flags & CMD_IS_DELETED)
		&& (interp == cmdPtr->nsPtr->interp)

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclObj.c`, function `CompareObjKeys(`, lines 3974–4084. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-8.6.18-tclObj.c`.

```text
 */

int
TclCompareObjKeys(
    void *keyPtr,		/* New key to compare. */
    Tcl_HashEntry *hPtr)	/* Existing key to compare. */
{
    Tcl_Obj *objPtr1 = (Tcl_Obj *)keyPtr;
    Tcl_Obj *objPtr2 = (Tcl_Obj *)hPtr->key.oneWordValue;
    const char *p1, *p2;
    size_t l1, l2;

    /*
     * If the object pointers are the same then they match.
     * OPT: this comparison was moved to the caller

       if (objPtr1 == objPtr2) {
	   return 1;
       }
    */

    /*
     * Don't use Tcl_GetStringFromObj as it would prevent l1 and l2 being
     * in a register.
     */

    p1 = TclGetString(objPtr1);
    l1 = objPtr1->length;
    p2 = TclGetString(objPtr2);
    l2 = objPtr2->length;

    /*
     * Only compare if the string representations are of the same length.
     */

    if (l1 == l2) {
	for (;; p1++, p2++, l1--) {
	    if (*p1 != *p2) {
		break;
	    }
	    if (l1 == 0) {
		return 1;
	    }
	}
    }

    return 0;
}

/*
 *----------------------------------------------------------------------
 *
 * TclFreeObjEntry --
 *
 *	Frees space for a Tcl_HashEntry containing the Tcl_Obj * key.
 *
 * Results:
 *	The return value is a pointer to the created entry.
 *
 * Side effects:
 *	Decrements the reference count of the object.
 *
 *----------------------------------------------------------------------
 */

void
TclFreeObjEntry(
    Tcl_HashEntry *hPtr)	/* Hash entry to free. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *) hPtr->key.oneWordValue;

    Tcl_DecrRefCount(objPtr);
    ckfree(hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclHashObjKey --
 *
 *	Compute a one-word summary of the string representation of the
 *	Tcl_Obj, which can be used to generate a hash index.
 *
 * Results:
 *	The return value is a one-word summary of the information in the
 *	string representation of the Tcl_Obj.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

unsigned int
TclHashObjKey(
    Tcl_HashTable *tablePtr,	/* Hash table. */
    void *keyPtr)		/* Key from which to compute hash value. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    int length;
    const char *string = TclGetStringFromObj(objPtr, &length);
    unsigned int result = 0;

    /*
     * I tried a zillion different hash functions and asked many other people
     * for advice. Many people had their own favorite functions, all
     * different, but no-one had much idea why they were good ones. I chose
     * the one below (multiply by 9 and add new character) because of the
     * following reasons:
     *
     * 1. Multiplying by 10 is perfect for keys that are decimal strings, and

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclObj.c`, function `FreeObjEntry(`, lines 4037–4147. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-8.6.18-tclObj.c`.

```text
 */

void
TclFreeObjEntry(
    Tcl_HashEntry *hPtr)	/* Hash entry to free. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *) hPtr->key.oneWordValue;

    Tcl_DecrRefCount(objPtr);
    ckfree(hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclHashObjKey --
 *
 *	Compute a one-word summary of the string representation of the
 *	Tcl_Obj, which can be used to generate a hash index.
 *
 * Results:
 *	The return value is a one-word summary of the information in the
 *	string representation of the Tcl_Obj.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

unsigned int
TclHashObjKey(
    Tcl_HashTable *tablePtr,	/* Hash table. */
    void *keyPtr)		/* Key from which to compute hash value. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    int length;
    const char *string = TclGetStringFromObj(objPtr, &length);
    unsigned int result = 0;

    /*
     * I tried a zillion different hash functions and asked many other people
     * for advice. Many people had their own favorite functions, all
     * different, but no-one had much idea why they were good ones. I chose
     * the one below (multiply by 9 and add new character) because of the
     * following reasons:
     *
     * 1. Multiplying by 10 is perfect for keys that are decimal strings, and
     *    multiplying by 9 is just about as good.
     * 2. Times-9 is (shift-left-3) plus (old). This means that each
     *    character's bits hang around in the low-order bits of the hash value
     *    for ever, plus they spread fairly rapidly up to the high-order bits
     *    to fill out the hash value. This seems works well both for decimal
     *    and non-decimal strings.
     *
     * Note that this function is very weak against malicious strings; it's
     * very easy to generate multiple keys that have the same hashcode. On the
     * other hand, that hardly ever actually occurs and this function *is*
     * very cheap, even by comparison with industry-standard hashes like FNV.
     * If real strength of hash is required though, use a custom hash based on
     * Bob Jenkins's lookup3(), but be aware that it's significantly slower.
     * Tcl does not use that level of strength because it typically does not
     * need it (and some of the aspects of that strength are genuinely
     * unnecessary given the rest of Tcl's hash machinery, and the fact that
     * we do not either transfer hashes to another machine, use them as a true
     * substitute for equality, or attempt to minimize work in rebuilding the
     * hash table).
     *
     * See also HashStringKey in tclHash.c.
     * See also HashString in tclLiteral.c.
     *
     * See [tcl-Feature Request #2958832]
     */

    if (length > 0) {
	result = UCHAR(*string);
	while (--length) {
	    result += (result << 3) + UCHAR(*++string);
	}
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFromObj --
 *
 *	Returns the command specified by the name in a Tcl_Obj.
 *
 * Results:
 *	Returns a token for the command if it is found. Otherwise, if it can't
 *	be found or there is an error, returns NULL.
 *
 * Side effects:
 *	May update the internal representation for the object, caching the
 *	command reference so that the next time this function is called with
 *	the same object, the command can be found quickly.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
Tcl_GetCommandFromObj(
    Tcl_Interp *interp,		/* The interpreter in which to resolve the
				 * command and to report errors. */
    Tcl_Obj *objPtr)	/* The object containing the command's name.
				 * If the name starts with "::", will be
				 * looked up in global namespace. Else, looked
				 * up first in the current namespace, then in
				 * global namespace. */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 86–198. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 740–851. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Namespace *nsPtr = (Namespace *) Tcl_GetCurrentNamespace(interp);
    FOREACH_HASH_DECLS;
    Tcl_Size soughtLen;
    const char *soughtStr, *nameStr, *matchedStr = NULL;

    if (objc < 2) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"bad call of unknown handler", TCL_AUTO_LENGTH));
	OO_ERROR(interp, BAD_UNKNOWN);
	return TCL_ERROR;
    }

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 2005–2115. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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
    int isInstanceExport = (clientData != NULL);
    int i, changed = 0;
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 2476–2586. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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

    if (method != (Tcl_Method) clsPtr->constructorPtr) {
	TclOODelMethodRef(clsPtr->constructorPtr);
	clsPtr->constructorPtr = (Method *) method;

	/*
	 * Remember to invalidate the cached constructor chain for this class.
	 * [Bug 2531577]
	 */

	if (clsPtr->constructorChainPtr) {
	    TclOODeleteChain(clsPtr->constructorChainPtr);
	    clsPtr->constructorChainPtr = NULL;
	}
	BumpGlobalEpoch(interp, clsPtr);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 849–959. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
	OO_ERROR(interp, BAD_UNKNOWN);
	return TCL_ERROR;
    }
    if (TclOOGetDefineCmdContext(interp) == NULL) {
	return TCL_ERROR;
    }

    soughtStr = TclGetStringFromObj(objv[1], &soughtLen);
    if (soughtLen == 0) {
	goto noMatch;
    }
    FOREACH_HASH_KEY(nameStr, &nsPtr->cmdTable) {
	if (strncmp(soughtStr, nameStr, soughtLen) == 0) {
	    if (matchedStr != NULL) {
		goto noMatch;
	    }
	    matchedStr = nameStr;
	}
    }

    if (matchedStr != NULL) {
	/*
	 * Got one match, and only one match!
	 */

	Tcl_Obj **newObjv = (Tcl_Obj **)
		TclStackAlloc(interp, sizeof(Tcl_Obj*) * (objc - 1));
	int result;

	newObjv[0] = Tcl_NewStringObj(matchedStr, TCL_AUTO_LENGTH);
	Tcl_IncrRefCount(newObjv[0]);
	if (objc > 2) {
	    memcpy(newObjv + 1, objv + 2, sizeof(Tcl_Obj *) * (objc - 2));
	}
	result = Tcl_EvalObjv(interp, objc - 1, newObjv, 0);
	Tcl_DecrRefCount(newObjv[0]);
	TclStackFree(interp, newObjv);
	return result;
    }

  noMatch:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "invalid command name \"%s\"", soughtStr));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "COMMAND", soughtStr, (char *)NULL);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * FindCommand --
 *
 *	Specialized version of Tcl_FindCommand that handles command prefixes
 *	and disallows namespace magic.
 *
 * ----------------------------------------------------------------------
 */

static Tcl_Command
FindCommand(
    Tcl_Interp *interp,
    Tcl_Obj *stringObj,
    Tcl_Namespace *const namespacePtr)
{
    Tcl_Size length;
    const char *nameStr, *string = TclGetStringFromObj(stringObj, &length);
    Namespace *const nsPtr = (Namespace *) namespacePtr;
    FOREACH_HASH_DECLS;
    Tcl_Command cmd, cmd2;

    /*
     * If someone is playing games, we stop playing right now.
     */

    if (string[0] == '\0' || strstr(string, "::") != NULL) {
	return NULL;
    }

    /*
     * Do the exact lookup first.
     */

    cmd = Tcl_FindCommand(interp, string, namespacePtr, TCL_NAMESPACE_ONLY);
    if (cmd != NULL) {
	return cmd;
    }

    /*
     * Bother, need to perform an approximate match. Iterate across the hash
     * table of commands in the namespace.
     */

    FOREACH_HASH(nameStr, cmd2, &nsPtr->cmdTable) {
	if (strncmp(string, nameStr, length) == 0) {
	    if (cmd != NULL) {
		return NULL;
	    }
	    cmd = cmd2;
	}
    }

    /*
     * Either we found one thing or we found nothing. Either way, return it.
     */

    return cmd;
}

/*
 * ----------------------------------------------------------------------
 *

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1008–1119. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
 */

Tcl_Object
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
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return NULL;
    }
    return oPtr->classPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * GetClassInOuterContext, GetNamespaceInOuterContext --
 *
 *	Wrappers round Tcl_GetObjectFromObj and TclGetNamespaceFromObj to
 *	perform the lookup in the context that called oo::define (or
 *	equivalent). Note that this may have to go up multiple levels to get
 *	the level that we started doing definitions at.
 *
 * ----------------------------------------------------------------------
 */

static inline Class *
GetClassInOuterContext(
    Tcl_Interp *interp,
    Tcl_Obj *className,
    const char *errMsg)
{
    Interp *iPtr = (Interp *) interp;
    Object *oPtr;
    CallFrame *savedFramePtr = iPtr->varFramePtr;

    while (iPtr->varFramePtr->isProcCallFrame == FRAME_IS_OO_DEFINE
	    || iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME) {
	if (iPtr->varFramePtr->callerVarPtr == NULL) {
	    Tcl_Panic("getting outer context when already in global context");
	}
	iPtr->varFramePtr = iPtr->varFramePtr->callerVarPtr;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, className);
    iPtr->varFramePtr = savedFramePtr;
    if (oPtr == NULL) {
	return NULL;
    }
    if (oPtr->classPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(errMsg, TCL_AUTO_LENGTH));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CLASS",
		TclGetString(className), (char *)NULL);
	return NULL;
    }
    return oPtr->classPtr;
}

static inline Tcl_Namespace *
GetNamespaceInOuterContext(
    Tcl_Interp *interp,
    Tcl_Obj *namespaceName)
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Namespace *nsPtr;
    int result;
    CallFrame *savedFramePtr = iPtr->varFramePtr;

    while (iPtr->varFramePtr->isProcCallFrame == FRAME_IS_OO_DEFINE
	    || iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME) {
	if (iPtr->varFramePtr->callerVarPtr == NULL) {
	    Tcl_Panic("getting outer context when already in global context");
	}
	iPtr->varFramePtr = iPtr->varFramePtr->callerVarPtr;
    }
    result = TclGetNamespaceFromObj(interp, namespaceName, &nsPtr);
    iPtr->varFramePtr = savedFramePtr;
    if (result != TCL_OK) {
	return NULL;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1039–1149. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
TclOOGetClassDefineCmdContext(
    Tcl_Interp *interp)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return NULL;
    }
    if (!oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return NULL;
    }
    return oPtr->classPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * GetClassInOuterContext, GetNamespaceInOuterContext --
 *
 *	Wrappers round Tcl_GetObjectFromObj and TclGetNamespaceFromObj to
 *	perform the lookup in the context that called oo::define (or
 *	equivalent). Note that this may have to go up multiple levels to get
 *	the level that we started doing definitions at.
 *
 * ----------------------------------------------------------------------
 */

static inline Class *
GetClassInOuterContext(
    Tcl_Interp *interp,
    Tcl_Obj *className,
    const char *errMsg)
{
    Interp *iPtr = (Interp *) interp;
    Object *oPtr;
    CallFrame *savedFramePtr = iPtr->varFramePtr;

    while (iPtr->varFramePtr->isProcCallFrame == FRAME_IS_OO_DEFINE
	    || iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME) {
	if (iPtr->varFramePtr->callerVarPtr == NULL) {
	    Tcl_Panic("getting outer context when already in global context");
	}
	iPtr->varFramePtr = iPtr->varFramePtr->callerVarPtr;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, className);
    iPtr->varFramePtr = savedFramePtr;
    if (oPtr == NULL) {
	return NULL;
    }
    if (oPtr->classPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(errMsg, TCL_AUTO_LENGTH));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CLASS",
		TclGetString(className), (char *)NULL);
	return NULL;
    }
    return oPtr->classPtr;
}

static inline Tcl_Namespace *
GetNamespaceInOuterContext(
    Tcl_Interp *interp,
    Tcl_Obj *namespaceName)
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Namespace *nsPtr;
    int result;
    CallFrame *savedFramePtr = iPtr->varFramePtr;

    while (iPtr->varFramePtr->isProcCallFrame == FRAME_IS_OO_DEFINE
	    || iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME) {
	if (iPtr->varFramePtr->callerVarPtr == NULL) {
	    Tcl_Panic("getting outer context when already in global context");
	}
	iPtr->varFramePtr = iPtr->varFramePtr->callerVarPtr;
    }
    result = TclGetNamespaceFromObj(interp, namespaceName, &nsPtr);
    iPtr->varFramePtr = savedFramePtr;
    if (result != TCL_OK) {
	return NULL;
    }
    return nsPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * GenerateErrorInfo --
 *
 *	Factored out code to generate part of the error trace messages.
 *
 * ----------------------------------------------------------------------
 */

static inline void
GenerateErrorInfo(
    Tcl_Interp *interp,		/* Where to store the error info trace. */
    Object *oPtr,		/* What object (or class) was being configured
				 * when the error occurred? */
    Tcl_Obj *savedNameObj,	/* Name of object saved from before script was
				 * evaluated, which is needed if the object
				 * goes away part way through execution. OTOH,
				 * if the object isn't deleted then its
				 * current name (post-execution) has to be
				 * used. This matters, because the object
				 * could have been renamed... */
    const char *typeOfSubject)	/* Part of the message, saying whether it was
				 * an object, class or class-as-object that
				 * was being configured. */
{

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1558–1668. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    Object *oPtr;
    int result, isPrivate;

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    if (objc < 2) {
	Tcl_SetObjResult(interp, TclOOObjectName(interp, oPtr));
	return TCL_OK;
    }

    isPrivate = IsPrivateDefine(interp);

    /*
     * Make the oo::objdefine namespace the current namespace and evaluate the
     * command(s).
     */

    nsPtr = TclOOGetDefineContextNamespace(interp, oPtr, 0);
    if (InitDefineContext(interp, nsPtr, oPtr, objc, objv) != TCL_OK) {
	return TCL_ERROR;
    }
    if (isPrivate) {
	((Interp *) interp)->varFramePtr->isProcCallFrame = PRIVATE_FRAME;
    }

    AddRef(oPtr);
    if (objc == 2) {
	Tcl_Obj *objNameObj = TclOOObjectName(interp, oPtr);

	Tcl_IncrRefCount(objNameObj);
	result = TclEvalObjEx(interp, objv[1], 0,
		((Interp *) interp)->cmdFramePtr, 1);
	if (result == TCL_ERROR) {
	    GenerateErrorInfo(interp, oPtr, objNameObj, "class object");
	}
	TclDecrRefCount(objNameObj);
    } else {
	result = MagicDefinitionInvoke(interp, nsPtr, 1, objc, objv);
    }
    TclOODecrRefCount(oPtr);

    /*
     * Restore the previous "current" namespace.
     */

    TclPopStackFrame(interp);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineObjSelfObjCmd --
 *
 *	Implementation of the "self" subcommand of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineObjSelfObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;

    if (objc != 1) {
	Tcl_WrongNumArgs(interp, 1, objv, NULL);
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, TclOOObjectName(interp, oPtr));
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefinePrivateObjCmd --
 *
 *	Implementation of the "private" subcommand of the "oo::define"
 *	and "oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefinePrivateObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstancePrivate = (clientData != NULL);
				/* Just so that we can generate the correct
				 * error message depending on the context of
				 * usage of this function. */
    Interp *iPtr = (Interp *) interp;
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int saved;			/* The saved flag. We restore it on exit so

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1632–1742. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, TclOOObjectName(interp, oPtr));
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefinePrivateObjCmd --
 *
 *	Implementation of the "private" subcommand of the "oo::define"
 *	and "oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefinePrivateObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    int isInstancePrivate = (clientData != NULL);
				/* Just so that we can generate the correct
				 * error message depending on the context of
				 * usage of this function. */
    Interp *iPtr = (Interp *) interp;
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int saved;			/* The saved flag. We restore it on exit so
				 * that [private private ...] doesn't make
				 * things go weird. */
    int result;

    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc == 1) {
	Tcl_SetObjResult(interp, Tcl_NewBooleanObj(IsPrivateDefine(interp)));
	return TCL_OK;
    }

    /*
     * Change the frame type flag while evaluating the body.
     */

    saved = iPtr->varFramePtr->isProcCallFrame;
    iPtr->varFramePtr->isProcCallFrame = PRIVATE_FRAME;

    /*
     * Evaluate the body; standard pattern.
     */

    AddRef(oPtr);
    if (objc == 2) {
	Tcl_Obj *objNameObj = TclOOObjectName(interp, oPtr);

	Tcl_IncrRefCount(objNameObj);
	result = TclEvalObjEx(interp, objv[1], 0, iPtr->cmdFramePtr, 1);
	if (result == TCL_ERROR) {
	    GenerateErrorInfo(interp, oPtr, objNameObj,
		    isInstancePrivate ? "object" : "class");
	}
	TclDecrRefCount(objNameObj);
    } else {
	result = MagicDefinitionInvoke(interp, TclGetCurrentNamespace(interp),
		1, objc, objv);
    }
    TclOODecrRefCount(oPtr);

    /*
     * Restore the frame type flag to what it was previously.
     */

    iPtr->varFramePtr->isProcCallFrame = saved;
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineClassObjCmd --
 *
 *	Implementation of the "class" subcommand of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineClassObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Class *clsPtr;
    Foundation *fPtr = TclOOGetFoundation(interp);
    int wasClass, willBeClass;

    /*
     * Parse the context to get the object to operate on.
     */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1664–1775. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
				 * error message depending on the context of
				 * usage of this function. */
    Interp *iPtr = (Interp *) interp;
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int saved;			/* The saved flag. We restore it on exit so
				 * that [private private ...] doesn't make
				 * things go weird. */
    int result;

    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc == 1) {
	Tcl_SetObjResult(interp, Tcl_NewBooleanObj(IsPrivateDefine(interp)));
	return TCL_OK;
    }

    /*
     * Change the frame type flag while evaluating the body.
     */

    saved = iPtr->varFramePtr->isProcCallFrame;
    iPtr->varFramePtr->isProcCallFrame = PRIVATE_FRAME;

    /*
     * Evaluate the body; standard pattern.
     */

    AddRef(oPtr);
    if (objc == 2) {
	Tcl_Obj *objNameObj = TclOOObjectName(interp, oPtr);

	Tcl_IncrRefCount(objNameObj);
	result = TclEvalObjEx(interp, objv[1], 0, iPtr->cmdFramePtr, 1);
	if (result == TCL_ERROR) {
	    GenerateErrorInfo(interp, oPtr, objNameObj,
		    isInstancePrivate ? "object" : "class");
	}
	TclDecrRefCount(objNameObj);
    } else {
	result = MagicDefinitionInvoke(interp, TclGetCurrentNamespace(interp),
		1, objc, objv);
    }
    TclOODecrRefCount(oPtr);

    /*
     * Restore the frame type flag to what it was previously.
     */

    iPtr->varFramePtr->isProcCallFrame = saved;
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineClassObjCmd --
 *
 *	Implementation of the "class" subcommand of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineClassObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;
    Class *clsPtr;
    Foundation *fPtr = TclOOGetFoundation(interp);
    int wasClass, willBeClass;

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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1741–1852. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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

    wasClass = (oPtr->classPtr != NULL);
    willBeClass = (TclOOIsReachable(fPtr->classCls, clsPtr));

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
 * ----------------------------------------------------------------------
 *
 * TclOODefineConstructorObjCmd --
 *
 *	Implementation of the "constructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineConstructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);
    Tcl_Method method;
    Tcl_Size bodyLength;

    if (clsPtr == NULL) {
	return TCL_ERROR;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1989–2099. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2108–2218. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    clsPtr = oPtr->classPtr;
    if (!isInstanceExport && !clsPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	/*
	 * Exporting is done by adding the PUBLIC_METHOD flag to the method
	 * record. If there is no such method in this object or class (i.e.
	 * the method comes from something inherited from or that we're an
	 * instance of) then we put in a blank record with that flag; such
	 * records are skipped over by the call chain engine *except* for
	 * their flags member.
	 */

	if (isInstanceExport) {
	    changed |= ExportInstanceMethod(oPtr, objv[i]);
	} else {
	    changed |= ExportMethod(clsPtr, objv[i]);
	}
    }

    /*
     * Bump the right epoch if we actually changed anything.
     */

    if (changed) {
	if (isInstanceExport) {
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
 * TclOODefineForwardObjCmd --
 *
 *	Implementation of the "forward" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
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

/*

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2180–2290. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineInitialiseObjCmd --
 *
 *	Implementation of the "initialise" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineInitialiseObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Tcl_Object object;
    Tcl_Obj *lambdaWords[3], *applyArgs[2];
    int result;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    /* Build the lambda */
    object = TclOOGetDefineCmdContext(interp);
    if (object == NULL) {
	return TCL_ERROR;
    }
    lambdaWords[0] = Tcl_NewObj();
    lambdaWords[1] = objv[1];
    lambdaWords[2] = TclNewNamespaceObj(Tcl_GetObjectNamespace(object));

    /* Delegate to [apply] to run it */
    applyArgs[0] = Tcl_NewStringObj("apply", -1);
    applyArgs[1] = Tcl_NewListObj(3, lambdaWords);
    Tcl_IncrRefCount(applyArgs[0]);
    Tcl_IncrRefCount(applyArgs[1]);
    result = Tcl_ApplyObjCmd(NULL, interp, 2, applyArgs);
    Tcl_DecrRefCount(applyArgs[0]);
    Tcl_DecrRefCount(applyArgs[1]);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineMethodObjCmd --
 *
 *	Implementation of the "method" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2243–2354. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    }

    /* Build the lambda */
    object = TclOOGetDefineCmdContext(interp);
    if (object == NULL) {
	return TCL_ERROR;
    }
    lambdaWords[0] = Tcl_NewObj();
    lambdaWords[1] = objv[1];
    lambdaWords[2] = TclNewNamespaceObj(Tcl_GetObjectNamespace(object));

    /* Delegate to [apply] to run it */
    applyArgs[0] = Tcl_NewStringObj("apply", -1);
    applyArgs[1] = Tcl_NewListObj(3, lambdaWords);
    Tcl_IncrRefCount(applyArgs[0]);
    Tcl_IncrRefCount(applyArgs[1]);
    result = Tcl_ApplyObjCmd(NULL, interp, 2, applyArgs);
    Tcl_DecrRefCount(applyArgs[0]);
    Tcl_DecrRefCount(applyArgs[1]);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineMethodObjCmd --
 *
 *	Implementation of the "method" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2305–2416. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineClassMethodObjCmd --
 *
 *	Implementation of the "classmethod" subcommand of the "oo::define"
 *	command. Defines a class method. See define(n) for details.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineClassMethodObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr;
    int isPublic;
    Tcl_Obj *forwardArgs[2], *prefixObj;
    Method *mPtr;

    if (objc != 2 && objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?args body?");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassDefineCmdContext(interp);
    if (!clsPtr) {
	return TCL_ERROR;
    }

    isPublic = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
	    ? PUBLIC_METHOD : 0;

    /*
     * Create the method on the delegate class if the caller gave arguments
     * and body.
     */
    if (objc == 4) {
	Tcl_Obj *delegateName = Tcl_ObjPrintf("%s:: oo ::delegate",
		clsPtr->thisPtr->namespacePtr->fullName);
	Class *delegatePtr = TclOOGetClassFromObj(interp, delegateName);

	Tcl_DecrRefCount(delegateName);
	if (!delegatePtr) {
	    return TCL_ERROR;
	}
	if (IsPrivateDefine(interp)) {
	    isPublic = 0;
	}
	if (TclOONewProcMethod(interp, delegatePtr, isPublic, objv[1],
		objv[2], objv[3], NULL) == NULL) {
	    return TCL_ERROR;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2458–2568. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceRenameMethod && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    /*
     * Delete the method entry from the appropriate hash table, and transfer
     * the thing it points to to its new entry. To do this, we first need to
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2517–2627. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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

    if (method != (Tcl_Method) clsPtr->constructorPtr) {
	TclOODelMethodRef(clsPtr->constructorPtr);
	clsPtr->constructorPtr = (Method *) method;

	/*
	 * Remember to invalidate the cached constructor chain for this class.
	 * [Bug 2531577]
	 */

	if (clsPtr->constructorChainPtr) {
	    TclOODeleteChain(clsPtr->constructorChainPtr);
	    clsPtr->constructorChainPtr = NULL;
	}
	BumpGlobalEpoch(interp, clsPtr);
    }
}

void
Tcl_ClassSetDestructor(
    Tcl_Interp *interp,
    Tcl_Class clazz,
    Tcl_Method method)
{
    Class *clsPtr = (Class *) clazz;

    if (method != (Tcl_Method) clsPtr->destructorPtr) {
	TclOODelMethodRef(clsPtr->destructorPtr);
	clsPtr->destructorPtr = (Method *) method;
	if (clsPtr->destructorChainPtr) {
	    TclOODeleteChain(clsPtr->destructorChainPtr);
	    clsPtr->destructorChainPtr = NULL;
	}
	BumpGlobalEpoch(interp, clsPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineSlots --
 *
 *	Create the "::oo::Slot" class and its standard instances. These are
 *	basically lists at the low level of TclOO; this provides a more
 *	consistent interface to them.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineSlots(
    Foundation *fPtr)
{
    Tcl_Interp *interp = fPtr->interp;
    Tcl_Object object = Tcl_NewObjectInstance(interp, (Tcl_Class)
	    fPtr->classCls, "::oo::Slot", NULL, TCL_INDEX_NONE, NULL, 0);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3675–3786. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj, *filterObj;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(filterObj, oPtr->filters) {
	Tcl_ListObjAppendElement(NULL, resultObj, filterObj);
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjFilter_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size filterc;
    Tcl_Obj **filterv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &filterc, &filterv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOObjectSetFilters(oPtr, filterc, filterv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjMixin_Get, ObjMixin_Set --
 *
 *	Implementation of the "mixin" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjMixin_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->mixins) {
	if (mixinPtr) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    TclOOObjectName(interp, mixinPtr->thisPtr));
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjMixin_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size mixinc, i;
    Tcl_Obj **mixinv;
    Class **mixins;		/* The references to the classes to actually
				 * install. */
    Tcl_HashTable uniqueCheck;	/* Note that this hash table is just used as a
				 * set of class references; it has no payload
				 * values and keys are always pointers. */
    int isNew;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3703–3814. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size filterc;
    Tcl_Obj **filterv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &filterc, &filterv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOObjectSetFilters(oPtr, filterc, filterv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjMixin_Get, ObjMixin_Set --
 *
 *	Implementation of the "mixin" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjMixin_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->mixins) {
	if (mixinPtr) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    TclOOObjectName(interp, mixinPtr->thisPtr));
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjMixin_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size mixinc, i;
    Tcl_Obj **mixinv;
    Class **mixins;		/* The references to the classes to actually
				 * install. */
    Tcl_HashTable uniqueCheck;	/* Note that this hash table is just used as a
				 * set of class references; it has no payload
				 * values and keys are always pointers. */
    int isNew;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &mixinc, &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = (Class **) TclStackAlloc(interp, sizeof(Class *) * mixinc);
    Tcl_InitHashTable(&uniqueCheck, TCL_ONE_WORD_KEYS);

    for (i = 0; i < mixinc; i++) {
	mixins[i] = GetClassInOuterContext(interp, mixinv[i],
		"may only mix in classes");
	if (mixins[i] == NULL) {
	    goto freeAndError;
	}
	(void) Tcl_CreateHashEntry(&uniqueCheck, (void *) mixins[i], &isNew);
	if (!isNew) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "class should only be a direct mixin once",
		    TCL_AUTO_LENGTH));
	    OO_ERROR(interp, REPETITIOUS);
	    goto freeAndError;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3742–3853. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->mixins) {
	if (mixinPtr) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    TclOOObjectName(interp, mixinPtr->thisPtr));
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjMixin_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size mixinc, i;
    Tcl_Obj **mixinv;
    Class **mixins;		/* The references to the classes to actually
				 * install. */
    Tcl_HashTable uniqueCheck;	/* Note that this hash table is just used as a
				 * set of class references; it has no payload
				 * values and keys are always pointers. */
    int isNew;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &mixinc, &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = (Class **) TclStackAlloc(interp, sizeof(Class *) * mixinc);
    Tcl_InitHashTable(&uniqueCheck, TCL_ONE_WORD_KEYS);

    for (i = 0; i < mixinc; i++) {
	mixins[i] = GetClassInOuterContext(interp, mixinv[i],
		"may only mix in classes");
	if (mixins[i] == NULL) {
	    goto freeAndError;
	}
	(void) Tcl_CreateHashEntry(&uniqueCheck, (void *) mixins[i], &isNew);
	if (!isNew) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "class should only be a direct mixin once",
		    TCL_AUTO_LENGTH));
	    OO_ERROR(interp, REPETITIOUS);
	    goto freeAndError;
	}
    }

    TclOOObjectSetMixins(oPtr, mixinc, mixins);
    TclStackFree(interp, mixins);
    Tcl_DeleteHashTable(&uniqueCheck);
    return TCL_OK;

  freeAndError:
    TclStackFree(interp, mixins);
    Tcl_DeleteHashTable(&uniqueCheck);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjVars_Get, ObjVars_Set --
 *
 *	Implementation of the "variable" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjVars_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3774–3885. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size mixinc, i;
    Tcl_Obj **mixinv;
    Class **mixins;		/* The references to the classes to actually
				 * install. */
    Tcl_HashTable uniqueCheck;	/* Note that this hash table is just used as a
				 * set of class references; it has no payload
				 * values and keys are always pointers. */
    int isNew;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &mixinc, &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = (Class **) TclStackAlloc(interp, sizeof(Class *) * mixinc);
    Tcl_InitHashTable(&uniqueCheck, TCL_ONE_WORD_KEYS);

    for (i = 0; i < mixinc; i++) {
	mixins[i] = GetClassInOuterContext(interp, mixinv[i],
		"may only mix in classes");
	if (mixins[i] == NULL) {
	    goto freeAndError;
	}
	(void) Tcl_CreateHashEntry(&uniqueCheck, (void *) mixins[i], &isNew);
	if (!isNew) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "class should only be a direct mixin once",
		    TCL_AUTO_LENGTH));
	    OO_ERROR(interp, REPETITIOUS);
	    goto freeAndError;
	}
    }

    TclOOObjectSetMixins(oPtr, mixinc, mixins);
    TclStackFree(interp, mixins);
    Tcl_DeleteHashTable(&uniqueCheck);
    return TCL_OK;

  freeAndError:
    TclStackFree(interp, mixins);
    Tcl_DeleteHashTable(&uniqueCheck);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjVars_Get, ObjVars_Set --
 *
 *	Implementation of the "variable" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjVars_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    if (IsPrivateDefine(interp)) {
	PrivateVariableMapping *privatePtr;

	FOREACH_STRUCT(privatePtr, oPtr->privateVariables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, privatePtr->variableObj);
	}
    } else {
	Tcl_Obj *variableObj;

	FOREACH(variableObj, oPtr->variables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjVars_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3845–3956. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    if (IsPrivateDefine(interp)) {
	PrivateVariableMapping *privatePtr;

	FOREACH_STRUCT(privatePtr, oPtr->privateVariables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, privatePtr->variableObj);
	}
    } else {
	Tcl_Obj *variableObj;

	FOREACH(variableObj, oPtr->variables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjVars_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size varc, i;
    Tcl_Obj **varv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"variableList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &varc, &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    for (i = 0; i < varc; i++) {
	const char *varName = TclGetString(varv[i]);

	if (strstr(varName, "::") != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "contain namespace separators"));
	    OO_ERROR(interp, BAD_DECLVAR);
	    return TCL_ERROR;
	}
	if (Tcl_StringMatch(varName, "*(*)")) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "refer to an array element"));
	    OO_ERROR(interp, BAD_DECLVAR);
	    return TCL_ERROR;
	}
    }

    if (IsPrivateDefine(interp)) {
	InstallPrivateVariableMapping(&oPtr->privateVariables, varc, varv,
		oPtr->creationEpoch);
    } else {
	InstallStandardVariableMapping(&oPtr->variables, varc, varv);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_ResolveClass --
 *
 *	Implementation of the "Resolve" support method for some slots (those
 *	that are slots around a list of classes). This resolves possible class
 *	names to their fully-qualified names if possible.
 *
 * ----------------------------------------------------------------------
 */

static int
Slot_ResolveClass(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    int idx = Tcl_ObjectContextSkippedArgs(context);
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Class *clsPtr;

    /*
     * Check if were called wrongly. The definition context isn't used...
     * except that GetClassInOuterContext() assumes that it is there.

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3883–3993. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size varc, i;
    Tcl_Obj **varv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"variableList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &varc, &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    for (i = 0; i < varc; i++) {
	const char *varName = TclGetString(varv[i]);

	if (strstr(varName, "::") != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "contain namespace separators"));
	    OO_ERROR(interp, BAD_DECLVAR);
	    return TCL_ERROR;
	}
	if (Tcl_StringMatch(varName, "*(*)")) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid declared variable name \"%s\": must not %s",
		    varName, "refer to an array element"));
	    OO_ERROR(interp, BAD_DECLVAR);
	    return TCL_ERROR;
	}
    }

    if (IsPrivateDefine(interp)) {
	InstallPrivateVariableMapping(&oPtr->privateVariables, varc, varv,
		oPtr->creationEpoch);
    } else {
	InstallStandardVariableMapping(&oPtr->variables, varc, varv);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_ResolveClass --
 *
 *	Implementation of the "Resolve" support method for some slots (those
 *	that are slots around a list of classes). This resolves possible class
 *	names to their fully-qualified names if possible.
 *
 * ----------------------------------------------------------------------
 */

static int
Slot_ResolveClass(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    int idx = Tcl_ObjectContextSkippedArgs(context);
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Class *clsPtr;

    /*
     * Check if were called wrongly. The definition context isn't used...
     * except that GetClassInOuterContext() assumes that it is there.
     */

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != idx + 1) {
	Tcl_WrongNumArgs(interp, idx, objv, "slotElement");
	return TCL_ERROR;
    }

    /*
     * Resolve the class if possible. If not, remove any resolution error and
     * return what we've got anyway as the failure might not be fatal overall.
     */

    clsPtr = GetClassInOuterContext(interp, objv[idx],
	    "USER SHOULD NOT SEE THIS MESSAGE");
    if (clsPtr == NULL) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, objv[idx]);
    } else {
	Tcl_SetObjResult(interp, TclOOObjectName(interp, clsPtr->thisPtr));
    }

    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Configurable_ClassReadableProps_Get, Configurable_ClassReadableProps_Set,
 * Configurable_ObjectReadableProps_Get, Configurable_ObjectReadableProps_Set --
 *
 *	Implementations of the "readableproperties" slot accessors for classes
 *	and instances.
 *
 * ----------------------------------------------------------------------
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3948–4059. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
    Tcl_Obj *const *objv)
{
    int idx = Tcl_ObjectContextSkippedArgs(context);
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Class *clsPtr;

    /*
     * Check if were called wrongly. The definition context isn't used...
     * except that GetClassInOuterContext() assumes that it is there.
     */

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != idx + 1) {
	Tcl_WrongNumArgs(interp, idx, objv, "slotElement");
	return TCL_ERROR;
    }

    /*
     * Resolve the class if possible. If not, remove any resolution error and
     * return what we've got anyway as the failure might not be fatal overall.
     */

    clsPtr = GetClassInOuterContext(interp, objv[idx],
	    "USER SHOULD NOT SEE THIS MESSAGE");
    if (clsPtr == NULL) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, objv[idx]);
    } else {
	Tcl_SetObjResult(interp, TclOOObjectName(interp, clsPtr->thisPtr));
    }

    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Configurable_ClassReadableProps_Get, Configurable_ClassReadableProps_Set,
 * Configurable_ObjectReadableProps_Get, Configurable_ObjectReadableProps_Set --
 *
 *	Implementations of the "readableproperties" slot accessors for classes
 *	and instances.
 *
 * ----------------------------------------------------------------------
 */

static int
Configurable_ClassReadableProps_Get(
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

    Tcl_SetObjResult(interp, TclOOGetPropertyList(&clsPtr->properties.readable));
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 4052–4163. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 4074–4185. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 4168–4279. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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
    int present = 0, changed = 0, i;
    Tcl_Obj *other;

    Tcl_SetListObj(listObj, 0, NULL);
    FOREACH(other, *propsList) {
	if (!TclStringCmp(propName, other, 1, 0, TCL_INDEX_NONE)) {
	    present = 1;
	    if (!addingProp) {
		changed = 1;
		continue;
	    }
	}
	Tcl_ListObjAppendElement(NULL, listObj, other);
    }
    if (!present && addingProp) {
	Tcl_ListObjAppendElement(NULL, listObj, propName);
	changed = 1;
    }
    return changed;
}

void
TclOORegisterInstanceProperty(
    Object *oPtr,		/* Object that owns the property slots. */
    Tcl_Obj *propName,		/* Property to add/remove. Must include the
				 * hyphen if one is desired; this is the value
				 * that is actually placed in the slot. */
    int registerReader,		/* True if we're adding the property name to
				 * the readable property slot. False if we're
				 * removing the property name from the slot. */
    int registerWriter)		/* True if we're adding the property name to
				 * the writable property slot. False if we're
				 * removing the property name from the slot. */
{
    Tcl_Obj *listObj = Tcl_NewObj();	/* Working buffer. */
    Tcl_Obj **objv;
    Tcl_Size count;

    if (BuildPropertyList(&oPtr->properties.readable, propName, registerReader,
	    listObj)) {
	TclListObjGetElements(NULL, listObj, &count, &objv);
	TclOOInstallReadableProperties(&oPtr->properties, count, objv);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 4190–4301. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.0.4-tclOODefineCmds.c`.

```text
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
    int present = 0, changed = 0, i;
    Tcl_Obj *other;

    Tcl_SetListObj(listObj, 0, NULL);
    FOREACH(other, *propsList) {
	if (!TclStringCmp(propName, other, 1, 0, TCL_INDEX_NONE)) {
	    present = 1;
	    if (!addingProp) {
		changed = 1;
		continue;
	    }
	}
	Tcl_ListObjAppendElement(NULL, listObj, other);
    }
    if (!present && addingProp) {
	Tcl_ListObjAppendElement(NULL, listObj, propName);
	changed = 1;
    }
    return changed;
}

void
TclOORegisterInstanceProperty(
    Object *oPtr,		/* Object that owns the property slots. */
    Tcl_Obj *propName,		/* Property to add/remove. Must include the
				 * hyphen if one is desired; this is the value
				 * that is actually placed in the slot. */
    int registerReader,		/* True if we're adding the property name to
				 * the readable property slot. False if we're
				 * removing the property name from the slot. */
    int registerWriter)		/* True if we're adding the property name to
				 * the writable property slot. False if we're
				 * removing the property name from the slot. */
{
    Tcl_Obj *listObj = Tcl_NewObj();	/* Working buffer. */
    Tcl_Obj **objv;
    Tcl_Size count;

    if (BuildPropertyList(&oPtr->properties.readable, propName, registerReader,
	    listObj)) {
	TclListObjGetElements(NULL, listObj, &count, &objv);
	TclOOInstallReadableProperties(&oPtr->properties, count, objv);
    }

    if (BuildPropertyList(&oPtr->properties.writable, propName, registerWriter,
	    listObj)) {
	TclListObjGetElements(NULL, listObj, &count, &objv);
	TclOOInstallWritableProperties(&oPtr->properties, count, objv);
    }
    Tcl_BounceRefCount(listObj);
}

void
TclOORegisterProperty(
    Class *clsPtr,		/* Class that owns the property slots. */
    Tcl_Obj *propName,		/* Property to add/remove. Must include the
				 * hyphen if one is desired; this is the value
				 * that is actually placed in the slot. */
    int registerReader,		/* True if we're adding the property name to
				 * the readable property slot. False if we're
				 * removing the property name from the slot. */
    int registerWriter)		/* True if we're adding the property name to
				 * the writable property slot. False if we're
				 * removing the property name from the slot. */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 310–419. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `Tcl_NewMethod2(`, lines 332–441. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod(`, lines 196–307. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
    return (Tcl_Method) mPtr;
}
Tcl_Method
Tcl_NewInstanceMethod(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Object object,		/* The object that has the method attached to
				 * it. */
    Tcl_Obj *nameObj,		/* The name of the method. May be NULL; if so,
				 * up to caller to manage storage (e.g., when
				 * it is a constructor or destructor). */
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
		"Tcl_NewInstanceMethod", "TCL_OO_METHOD_VERSION_1");
    }
    return TclNewInstanceMethod(NULL, object, nameObj, flags, typePtr,
	    clientData);
}
Tcl_Method
Tcl_NewInstanceMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Object object,		/* The object that has the method attached to
				 * it. */
    Tcl_Obj *nameObj,		/* The name of the method. May be NULL; if so,
				 * up to caller to manage storage (e.g., when
				 * it is a constructor or destructor). */
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
		"Tcl_NewInstanceMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewInstanceMethod(NULL, object, nameObj, flags,
	    (const Tcl_MethodType *) typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * Tcl_NewMethod --
 *
 *	Attach a method to a class.
 *
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod2(`, lines 219–330. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
	    clientData);
}
Tcl_Method
Tcl_NewInstanceMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Object object,		/* The object that has the method attached to
				 * it. */
    Tcl_Obj *nameObj,		/* The name of the method. May be NULL; if so,
				 * up to caller to manage storage (e.g., when
				 * it is a constructor or destructor). */
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
		"Tcl_NewInstanceMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewInstanceMethod(NULL, object, nameObj, flags,
	    (const Tcl_MethodType *) typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * Tcl_NewMethod --
 *
 *	Attach a method to a class.
 *
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOMethod.c`, function `TclOODelMethodRef(`, lines 365–474. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.0.4--tclOOMethod.c`.

```text
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `renamemethod`, lines 62–174. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclOO.c`.

```text
    {"initialize",	TclOODefineInitialiseObjCmd, 0},
    {"method",		TclOODefineMethodObjCmd, 0},
    {"private",		TclOODefinePrivateObjCmd, 0},
    {"renamemethod",	TclOODefineRenameMethodObjCmd, 0},
    {"self",		TclOODefineSelfObjCmd, 0},
    {"unexport",	TclOODefineUnexportObjCmd, 0},
    {NULL, NULL, 0}
}, objdefCmds[] = {
    {"class",		TclOODefineClassObjCmd, 1},
    {"deletemethod",	TclOODefineDeleteMethodObjCmd, 1},
    {"export",		TclOODefineExportObjCmd, 1},
    {"forward",		TclOODefineForwardObjCmd, 1},
    {"method",		TclOODefineMethodObjCmd, 1},
    {"private",		TclOODefinePrivateObjCmd, 1},
    {"renamemethod",	TclOODefineRenameMethodObjCmd, 1},
    {"self",		TclOODefineObjSelfObjCmd, 0},
    {"unexport",	TclOODefineUnexportObjCmd, 1},
    {NULL, NULL, 0}
};

/*
 * What sort of size of things we like to allocate.
 */

#define ALLOC_CHUNK 8

/*
 * Function declarations for things defined in this file.
 */

static Object *		AllocObject(Tcl_Interp *interp, const char *nameStr,
			    Namespace *nsPtr, const char *nsNameStr);
static int		CloneClassMethod(Tcl_Interp *interp, Class *clsPtr,
			    Method *mPtr, Tcl_Obj *namePtr,
			    Method **newMPtrPtr);
static int		CloneObjectMethod(Tcl_Interp *interp, Object *oPtr,
			    Method *mPtr, Tcl_Obj *namePtr);
static Tcl_NamespaceDeleteProc	DeletedHelpersNamespace;
static Tcl_NRPostProc	FinalizeAlloc;
static Tcl_NRPostProc	FinalizeNext;
static Tcl_NRPostProc	FinalizeObjectCall;
static inline void	InitClassPath(Tcl_Interp * interp, Class *clsPtr);
static void		InitClassSystemRoots(Tcl_Interp *interp,
			    Foundation *fPtr);
static int		InitFoundation(Tcl_Interp *interp);
static Tcl_InterpDeleteProc	KillFoundation;
static void		MakeAdditionalClasses(Foundation *fPtr,
			    Tcl_Namespace *defineNs,
			    Tcl_Namespace *objDefineNs);
static Tcl_CmdDeleteProc	MyDeleted;
static Tcl_NamespaceDeleteProc	ObjectNamespaceDeleted;
static Tcl_CommandTraceProc	ObjectRenamedTrace;
static inline void	RemoveClass(Class **list, size_t num, size_t idx);
static inline void	RemoveObject(Object **list, size_t num, size_t idx);
static inline void	SquelchCachedName(Object *oPtr);

static Tcl_ObjCmdProc	PublicNRObjectCmd;
static Tcl_ObjCmdProc	PrivateNRObjectCmd;
static Tcl_ObjCmdProc	MyClassNRObjCmd;
static Tcl_CmdDeleteProc	MyClassDeleted;

/*
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `renamemethod`, lines 73–185. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclOO.c`.

```text
    {"forward",		TclOODefineForwardObjCmd, 1},
    {"method",		TclOODefineMethodObjCmd, 1},
    {"private",		TclOODefinePrivateObjCmd, 1},
    {"renamemethod",	TclOODefineRenameMethodObjCmd, 1},
    {"self",		TclOODefineObjSelfObjCmd, 0},
    {"unexport",	TclOODefineUnexportObjCmd, 1},
    {NULL, NULL, 0}
};

/*
 * What sort of size of things we like to allocate.
 */

#define ALLOC_CHUNK 8

/*
 * Function declarations for things defined in this file.
 */

static Object *		AllocObject(Tcl_Interp *interp, const char *nameStr,
			    Namespace *nsPtr, const char *nsNameStr);
static int		CloneClassMethod(Tcl_Interp *interp, Class *clsPtr,
			    Method *mPtr, Tcl_Obj *namePtr,
			    Method **newMPtrPtr);
static int		CloneObjectMethod(Tcl_Interp *interp, Object *oPtr,
			    Method *mPtr, Tcl_Obj *namePtr);
static Tcl_NamespaceDeleteProc	DeletedHelpersNamespace;
static Tcl_NRPostProc	FinalizeAlloc;
static Tcl_NRPostProc	FinalizeNext;
static Tcl_NRPostProc	FinalizeObjectCall;
static inline void	InitClassPath(Tcl_Interp * interp, Class *clsPtr);
static void		InitClassSystemRoots(Tcl_Interp *interp,
			    Foundation *fPtr);
static int		InitFoundation(Tcl_Interp *interp);
static Tcl_InterpDeleteProc	KillFoundation;
static void		MakeAdditionalClasses(Foundation *fPtr,
			    Tcl_Namespace *defineNs,
			    Tcl_Namespace *objDefineNs);
static Tcl_CmdDeleteProc	MyDeleted;
static Tcl_NamespaceDeleteProc	ObjectNamespaceDeleted;
static Tcl_CommandTraceProc	ObjectRenamedTrace;
static inline void	RemoveClass(Class **list, size_t num, size_t idx);
static inline void	RemoveObject(Object **list, size_t num, size_t idx);
static inline void	SquelchCachedName(Object *oPtr);

static Tcl_ObjCmdProc	PublicNRObjectCmd;
static Tcl_ObjCmdProc	PrivateNRObjectCmd;
static Tcl_ObjCmdProc	MyClassNRObjCmd;
static Tcl_CmdDeleteProc	MyClassDeleted;

/*
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
 * The scripted part of TclOO: (legacy) package registration. There's no C API
 * at all for doing this, not even internally to Tcl.
 */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `typedef struct Tcl_MethodType`, lines 80–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text
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

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `typedef struct Tcl_MethodType`, lines 98–190. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text
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

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 82–190. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text

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

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 100–190. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

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

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 122–190. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text
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

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 126–190. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h`.

```text
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

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tcl.h`, function `Information about commands`, lines 831–943. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-9.0.4-generic-tcl.h`.

```text

/*
 *----------------------------------------------------------------------------
 * Information about commands that is returned by Tcl_GetCommandInfo and
 * passed to Tcl_SetCommandInfo. objProc is an objc/objv object-based command
 * function while proc is a traditional Tcl argc/argv string-based function.
 * Tcl_CreateObjCommand and Tcl_CreateCommand ensure that both objProc and
 * proc are non-NULL and can be called to execute the command. However, it may
 * be faster to call one instead of the other. The member isNativeObjectProc
 * is set to 1 if an object-based function was registered by
 * Tcl_CreateObjCommand, and to 0 if a string-based function was registered by
 * Tcl_CreateCommand. The other function is typically set to a compatibility
 * wrapper that does string-to-object or object-to-string argument conversions
 * then calls the other function.
 */

typedef struct {
    int isNativeObjectProc;	/* 1 if objProc was registered by a call to
				 * Tcl_CreateObjCommand; 2 if objProc was registered by
				 * a call to Tcl_CreateObjCommand2; 0 otherwise.
				 * Tcl_SetCmdInfo does not modify this field. */
    Tcl_ObjCmdProc *objProc;	/* Command's object-based function. */
    void *objClientData;	/* ClientData for object proc. */
    Tcl_CmdProc *proc;		/* Command's string-based function. */
    void *clientData;		/* ClientData for string proc. */
    Tcl_CmdDeleteProc *deleteProc;
				/* Function to call when command is
				 * deleted. */
    void *deleteData;		/* Value to pass to deleteProc (usually the
				 * same as clientData). */
    Tcl_Namespace *namespacePtr;/* Points to the namespace that contains this
				 * command. Note that Tcl_SetCmdInfo will not
				 * change a command's namespace; use
				 * TclRenameCommand or Tcl_Eval (of 'rename')
				 * to do that. */
#if TCL_MAJOR_VERSION > 8
    Tcl_ObjCmdProc2 *objProc2;	/* Command's object2-based function. */
    void *objClientData2;	/* ClientData for object2 proc. */
#endif
} Tcl_CmdInfo;

/*
 *----------------------------------------------------------------------------
 * The structure defined below is used to hold dynamic strings. The only
 * fields that clients should use are string and length, accessible via the
 * macros Tcl_DStringValue and Tcl_DStringLength.
 */

#define TCL_DSTRING_STATIC_SIZE 200
typedef struct Tcl_DString {
    char *string;		/* Points to beginning of string: either
				 * staticSpace below or a malloced array. */
    Tcl_Size length;		/* Number of bytes in string excluding
				 * terminating nul */
    Tcl_Size spaceAvl;		/* Total number of bytes available for the
				 * string and its terminating NULL char. */
    char staticSpace[TCL_DSTRING_STATIC_SIZE];
				/* Space to use in common case where string is
				 * small. */
} Tcl_DString;

#define Tcl_DStringLength(dsPtr) ((dsPtr)->length)
#define Tcl_DStringValue(dsPtr) ((dsPtr)->string)

/*
 * Definitions for the maximum number of digits of precision that may be
 * produced by Tcl_PrintDouble, and the number of bytes of buffer space
 * required by Tcl_PrintDouble.
 */

#define TCL_MAX_PREC		17
#define TCL_DOUBLE_SPACE	(TCL_MAX_PREC+10)

/*
 * Definition for a number of bytes of buffer space sufficient to hold the
 * string representation of an integer in base 10 (assuming the existence of
 * 64-bit integers).
 */

#define TCL_INTEGER_SPACE	(3*(int)sizeof(Tcl_WideInt))

/*
 *----------------------------------------------------------------------------
 * Type values returned by Tcl_GetNumberFromObj
 *	TCL_NUMBER_INT		Representation is a Tcl_WideInt
 *	TCL_NUMBER_BIG		Representation is an mp_int
 *	TCL_NUMBER_DOUBLE	Representation is a double
 *	TCL_NUMBER_NAN		Value is NaN.
 */

#define TCL_NUMBER_INT          2
#define TCL_NUMBER_BIG          3
#define TCL_NUMBER_DOUBLE       4
#define TCL_NUMBER_NAN          5

/*
 * Flag values passed to Tcl_ConvertElement.
 * TCL_DONT_USE_BRACES forces it not to enclose the element in braces, but to
 *	use backslash quoting instead.
 * TCL_DONT_QUOTE_HASH disables the default quoting of the '#' character. It
 *	is safe to leave the hash unquoted when the element is not the first
 *	element of a list, and this flag can be used by the caller to indicate
 *	that condition.
 */

#define TCL_DONT_USE_BRACES	1
#define TCL_DONT_QUOTE_HASH	8

/*
 * Flags that may be passed to Tcl_GetIndexFromObj.
 * TCL_EXACT disallows abbreviated strings.
 * TCL_NULL_OK allows the empty string or NULL to return TCL_OK.
 *      The returned value will be -1;

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tcl.h`, function `Tcl_ObjCmdProc2 *objProc2`, lines 864–976. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-9.0.4-generic-tcl.h`.

```text
				 * TclRenameCommand or Tcl_Eval (of 'rename')
				 * to do that. */
#if TCL_MAJOR_VERSION > 8
    Tcl_ObjCmdProc2 *objProc2;	/* Command's object2-based function. */
    void *objClientData2;	/* ClientData for object2 proc. */
#endif
} Tcl_CmdInfo;

/*
 *----------------------------------------------------------------------------
 * The structure defined below is used to hold dynamic strings. The only
 * fields that clients should use are string and length, accessible via the
 * macros Tcl_DStringValue and Tcl_DStringLength.
 */

#define TCL_DSTRING_STATIC_SIZE 200
typedef struct Tcl_DString {
    char *string;		/* Points to beginning of string: either
				 * staticSpace below or a malloced array. */
    Tcl_Size length;		/* Number of bytes in string excluding
				 * terminating nul */
    Tcl_Size spaceAvl;		/* Total number of bytes available for the
				 * string and its terminating NULL char. */
    char staticSpace[TCL_DSTRING_STATIC_SIZE];
				/* Space to use in common case where string is
				 * small. */
} Tcl_DString;

#define Tcl_DStringLength(dsPtr) ((dsPtr)->length)
#define Tcl_DStringValue(dsPtr) ((dsPtr)->string)

/*
 * Definitions for the maximum number of digits of precision that may be
 * produced by Tcl_PrintDouble, and the number of bytes of buffer space
 * required by Tcl_PrintDouble.
 */

#define TCL_MAX_PREC		17
#define TCL_DOUBLE_SPACE	(TCL_MAX_PREC+10)

/*
 * Definition for a number of bytes of buffer space sufficient to hold the
 * string representation of an integer in base 10 (assuming the existence of
 * 64-bit integers).
 */

#define TCL_INTEGER_SPACE	(3*(int)sizeof(Tcl_WideInt))

/*
 *----------------------------------------------------------------------------
 * Type values returned by Tcl_GetNumberFromObj
 *	TCL_NUMBER_INT		Representation is a Tcl_WideInt
 *	TCL_NUMBER_BIG		Representation is an mp_int
 *	TCL_NUMBER_DOUBLE	Representation is a double
 *	TCL_NUMBER_NAN		Value is NaN.
 */

#define TCL_NUMBER_INT          2
#define TCL_NUMBER_BIG          3
#define TCL_NUMBER_DOUBLE       4
#define TCL_NUMBER_NAN          5

/*
 * Flag values passed to Tcl_ConvertElement.
 * TCL_DONT_USE_BRACES forces it not to enclose the element in braces, but to
 *	use backslash quoting instead.
 * TCL_DONT_QUOTE_HASH disables the default quoting of the '#' character. It
 *	is safe to leave the hash unquoted when the element is not the first
 *	element of a list, and this flag can be used by the caller to indicate
 *	that condition.
 */

#define TCL_DONT_USE_BRACES	1
#define TCL_DONT_QUOTE_HASH	8

/*
 * Flags that may be passed to Tcl_GetIndexFromObj.
 * TCL_EXACT disallows abbreviated strings.
 * TCL_NULL_OK allows the empty string or NULL to return TCL_OK.
 *      The returned value will be -1;
 * TCL_INDEX_TEMP_TABLE disallows caching of lookups. A possible use case is
 *      a table that will not live long enough to make it worthwhile.
 */

#define TCL_EXACT		1
#define TCL_NULL_OK		32
#define TCL_INDEX_TEMP_TABLE	64

/*
 * Flags that may be passed to Tcl_UniCharToUtf.
 * TCL_COMBINE Combine surrogates
 */

#if TCL_MAJOR_VERSION > 8
#    define TCL_COMBINE		0x1000000
#else
#    define TCL_COMBINE		0
#endif
/*
 *----------------------------------------------------------------------------
 * Flag values passed to Tcl_RecordAndEval, Tcl_EvalObj, Tcl_EvalObjv.
 * WARNING: these bit choices must not conflict with the bit choices for
 * evalFlag bits in tclInt.h!
 *
 * Meanings:
 *	TCL_NO_EVAL:		Just record this command
 *	TCL_EVAL_GLOBAL:	Execute script in global namespace
 *	TCL_EVAL_DIRECT:	Do not compile this script
 *	TCL_EVAL_INVOKE:	Magical Tcl_EvalObjv mode for aliases/ensembles
 *				o Run in iPtr->lookupNsPtr or global namespace
 *				o Cut out of error traces
 *				o Don't reset the flags controlling ensemble
 *				  error message rewriting.

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tcl.h`, function `typedef struct Tcl_Obj`, lines 654–766. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-9.0.4-generic-tcl.h`.

```text
 * standard operations on objects of that type.
 */

typedef struct Tcl_ObjType {
    const char *name;		/* Name of the type, e.g. "int". */
    Tcl_FreeInternalRepProc *freeIntRepProc;
				/* Called to free any storage for the type's
				 * internal rep. NULL if the internal rep does
				 * not need freeing. */
    Tcl_DupInternalRepProc *dupIntRepProc;
				/* Called to create a new object as a copy of
				 * an existing object. */
    Tcl_UpdateStringProc *updateStringProc;
				/* Called to update the string rep from the
				 * type's internal representation. */
    Tcl_SetFromAnyProc *setFromAnyProc;
				/* Called to convert the object's internal rep
				 * to this type. Frees the internal rep of the
				 * old type. Returns TCL_ERROR on failure. */
#if TCL_MAJOR_VERSION > 8
    size_t version;		/* Version field for future-proofing. */

    /* List emulation functions - ObjType Version 1 */
    Tcl_ObjTypeLengthProc *lengthProc;
				/* Return the [llength] of the AbstractList */
    Tcl_ObjTypeIndexProc *indexProc;
				/* Return a value (Tcl_Obj) at a given index */
    Tcl_ObjTypeSliceProc *sliceProc;
				/* Return an AbstractList for
				 * [lrange $al $start $end] */
    Tcl_ObjTypeReverseProc *reverseProc;
				/* Return an AbstractList for [lreverse $al] */
    Tcl_ObjTypeGetElements *getElementsProc;
				/* Return an objv[] of all elements in the list */
    Tcl_ObjTypeSetElement *setElementProc;
				/* Replace the element at the indicies with the
				 * given valueObj. */
    Tcl_ObjTypeReplaceProc *replaceProc;
				/* Replace sublist with another sublist */
    Tcl_ObjTypeInOperatorProc *inOperProc;
				/* "in" and "ni" expr list operation.
				 * Determine if the given string value matches
				 * an element in the list. */
#endif
} Tcl_ObjType;

#if TCL_MAJOR_VERSION > 8
#   define TCL_OBJTYPE_V0 0, \
	   0,0,0,0,0,0,0,0	/* Pre-Tcl 9 */
#   define TCL_OBJTYPE_V1(a) offsetof(Tcl_ObjType, indexProc), \
	   a,0,0,0,0,0,0,0	/* Tcl 9 Version 1 */
#   define TCL_OBJTYPE_V2(a,b,c,d,e,f,g,h) sizeof(Tcl_ObjType),  \
	   a,b,c,d,e,f,g,h	/* Tcl 9 - AbstractLists */
#endif

/*
 * The following structure stores an internal representation (internalrep) for
 * a Tcl value. An internalrep is associated with an Tcl_ObjType when both
 * are stored in the same Tcl_Obj.  The routines of the Tcl_ObjType govern
 * the handling of the internalrep.
 */

typedef union Tcl_ObjInternalRep {	/* The internal representation: */
    long longValue;		/*   - an long integer value. */
    double doubleValue;		/*   - a double-precision floating value. */
    void *otherValuePtr;	/*   - another, type-specific value, */
				/*     not used internally any more. */
    Tcl_WideInt wideValue;	/*   - an integer value >= 64bits */
    struct {			/*   - internal rep as two pointers. */
	void *ptr1;
	void *ptr2;
    } twoPtrValue;
    struct {			/*   - internal rep as a pointer and a long, */
	void *ptr;		/*     not used internally any more. */
	unsigned long value;
    } ptrAndLongRep;
#if TCL_MAJOR_VERSION > 8
    struct {			/*   - use for pointer and length reps */
	void *ptr;
	Tcl_Size size;
    } ptrAndSize;
#endif
} Tcl_ObjInternalRep;

/*
 * One of the following structures exists for each object in the Tcl system.
 * An object stores a value as either a string, some internal representation,
 * or both.
 */

typedef struct Tcl_Obj {
    Tcl_Size refCount;		/* When 0 the object will be freed. */
    char *bytes;		/* This points to the first byte of the
				 * object's string representation. The array
				 * must be followed by a null byte (i.e., at
				 * offset length) but may also contain
				 * embedded null characters. The array's
				 * storage is allocated by Tcl_Alloc. NULL means
				 * the string rep is invalid and must be
				 * regenerated from the internal rep.  Clients
				 * should use Tcl_GetStringFromObj or
				 * Tcl_GetString to get a pointer to the byte
				 * array as a readonly value. */
    Tcl_Size length;		/* The number of bytes at *bytes, not
				 * including the terminating null. */
    const Tcl_ObjType *typePtr;	/* Denotes the object's type. Always
				 * corresponds to the type of the object's
				 * internal rep. NULL indicates the object has
				 * no internal rep (has no type). */
    Tcl_ObjInternalRep internalRep;
				/* The internal representation: */
} Tcl_Obj;


```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tcl.h`, function `typedef struct Tcl_Obj`, lines 741–852. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-9.0.4-generic-tcl.h`.

```text
 * or both.
 */

typedef struct Tcl_Obj {
    Tcl_Size refCount;		/* When 0 the object will be freed. */
    char *bytes;		/* This points to the first byte of the
				 * object's string representation. The array
				 * must be followed by a null byte (i.e., at
				 * offset length) but may also contain
				 * embedded null characters. The array's
				 * storage is allocated by Tcl_Alloc. NULL means
				 * the string rep is invalid and must be
				 * regenerated from the internal rep.  Clients
				 * should use Tcl_GetStringFromObj or
				 * Tcl_GetString to get a pointer to the byte
				 * array as a readonly value. */
    Tcl_Size length;		/* The number of bytes at *bytes, not
				 * including the terminating null. */
    const Tcl_ObjType *typePtr;	/* Denotes the object's type. Always
				 * corresponds to the type of the object's
				 * internal rep. NULL indicates the object has
				 * no internal rep (has no type). */
    Tcl_ObjInternalRep internalRep;
				/* The internal representation: */
} Tcl_Obj;

/*
 *----------------------------------------------------------------------------
 * The following definitions support Tcl's namespace facility. Note: the first
 * five fields must match exactly the fields in a Namespace structure (see
 * tclInt.h).
 */

typedef struct Tcl_Namespace {
    char *name;			/* The namespace's name within its parent
				 * namespace. This contains no ::'s. The name
				 * of the global namespace is "" although "::"
				 * is an synonym. */
    char *fullName;		/* The namespace's fully qualified name. This
				 * starts with ::. */
    void *clientData;		/* Arbitrary value associated with this
				 * namespace. */
    Tcl_NamespaceDeleteProc *deleteProc;
				/* Function invoked when deleting the
				 * namespace to, e.g., free clientData. */
    struct Tcl_Namespace *parentPtr;
				/* Points to the namespace that contains this
				 * one. NULL if this is the global
				 * namespace. */
} Tcl_Namespace;

/*
 *----------------------------------------------------------------------------
 * The following structure represents a call frame, or activation record. A
 * call frame defines a naming context for a procedure call: its local scope
 * (for local variables) and its namespace scope (used for non-local
 * variables; often the global :: namespace). A call frame can also define the
 * naming context for a namespace eval or namespace inscope command: the
 * namespace in which the command's code should execute. The Tcl_CallFrame
 * structures exist only while procedures or namespace eval/inscope's are
 * being executed, and provide a Tcl call stack.
 *
 * A call frame is initialized and pushed using Tcl_PushCallFrame and popped
 * using Tcl_PopCallFrame. Storage for a Tcl_CallFrame must be provided by the
 * Tcl_PushCallFrame caller, and callers typically allocate them on the C call
 * stack for efficiency. For this reason, Tcl_CallFrame is defined as a
 * structure and not as an opaque token. However, most Tcl_CallFrame fields
 * are hidden since applications should not access them directly; others are
 * declared as "dummyX".
 *
 * WARNING!! The structure definition must be kept consistent with the
 * CallFrame structure in tclInt.h. If you change one, change the other.
 */

typedef struct Tcl_CallFrame {
    Tcl_Namespace *nsPtr;	/* Current namespace for the call frame. */
    int dummy1;
    Tcl_Size dummy2;
    void *dummy3;
    void *dummy4;
    void *dummy5;
    Tcl_Size dummy6;
    void *dummy7;
    void *dummy8;
    Tcl_Size dummy9;
    void *dummy10;
    void *dummy11;
    void *dummy12;
    void *dummy13;
} Tcl_CallFrame;

/*
 *----------------------------------------------------------------------------
 * Information about commands that is returned by Tcl_GetCommandInfo and
 * passed to Tcl_SetCommandInfo. objProc is an objc/objv object-based command
 * function while proc is a traditional Tcl argc/argv string-based function.
 * Tcl_CreateObjCommand and Tcl_CreateCommand ensure that both objProc and
 * proc are non-NULL and can be called to execute the command. However, it may
 * be faster to call one instead of the other. The member isNativeObjectProc
 * is set to 1 if an object-based function was registered by
 * Tcl_CreateObjCommand, and to 0 if a string-based function was registered by
 * Tcl_CreateCommand. The other function is typically set to a compatibility
 * wrapper that does string-to-object or object-to-string argument conversions
 * then calls the other function.
 */

typedef struct {
    int isNativeObjectProc;	/* 1 if objProc was registered by a call to
				 * Tcl_CreateObjCommand; 2 if objProc was registered by
				 * a call to Tcl_CreateObjCommand2; 0 otherwise.
				 * Tcl_SetCmdInfo does not modify this field. */
    Tcl_ObjCmdProc *objProc;	/* Command's object-based function. */

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_GetCommandInfo(`, lines 3502–3612. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
 */

int
Tcl_GetCommandInfo(
    Tcl_Interp *interp,		/* Interpreter in which to look for
				 * command. */
    const char *cmdName,	/* Name of desired command. */
    Tcl_CmdInfo *infoPtr)	/* Where to store information about
				 * command. */
{
    Tcl_Command cmd;

    cmd = Tcl_FindCommand(interp, cmdName, NULL, /*flags*/ 0);
    return Tcl_GetCommandInfoFromToken(cmd, infoPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandInfoFromToken --
 *
 *	Returns various information about a Tcl command.
 *
 * Results:
 *	Copies information from the command identified by 'cmd' into a
 *	caller-supplied structure and returns 1. If the 'cmd' is NULL, leaves
 *	the structure untouched and returns 0.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_GetCommandInfoFromToken(
    Tcl_Command cmd,
    Tcl_CmdInfo *infoPtr)
{
    Command *cmdPtr;		/* Internal representation of the command */

    if (cmd == NULL) {
	return 0;
    }

    /*
     * Set isNativeObjectProc 1 if objProc was registered by a call to
     * Tcl_CreateObjCommand. Set isNativeObjectProc 2 if objProc was
     * registered by a call to Tcl_CreateObjCommand2. Otherwise set it to 0.
     */

    cmdPtr = (Command *) cmd;
    infoPtr->isNativeObjectProc =
	    (cmdPtr->objProc != InvokeStringCommand);
    infoPtr->objProc = cmdPtr->objProc;
    infoPtr->objClientData = cmdPtr->objClientData;
    infoPtr->proc = cmdPtr->proc;
    infoPtr->clientData = cmdPtr->clientData;
    if (cmdPtr->deleteProc == CmdWrapperDeleteProc) {
	CmdWrapperInfo *info = (CmdWrapperInfo *)cmdPtr->deleteData;
	infoPtr->deleteProc = info->deleteProc;
	infoPtr->deleteData = info->deleteData;
	infoPtr->objProc2 = info->proc;
	infoPtr->objClientData2 = info->clientData;
	if (cmdPtr->objProc == CmdWrapperProc) {
	    infoPtr->isNativeObjectProc = 2;
	}
    } else {
	infoPtr->deleteProc = cmdPtr->deleteProc;
	infoPtr->deleteData = cmdPtr->deleteData;
	infoPtr->objProc2 = CmdWrapper2Proc;
	infoPtr->objClientData2 = cmdPtr;
    }
    infoPtr->namespacePtr = (Tcl_Namespace *) cmdPtr->nsPtr;
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandName --
 *
 *	Given a token returned by Tcl_CreateObjCommand, this function returns the
 *	current name of the command (which may have changed due to renaming).
 *
 * Results:
 *	The return value is the name of the given command.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_GetCommandName(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Command command)	/* Token for command returned by a previous
				 * call to Tcl_CreateObjCommand. The command must
				 * not have been deleted. */
{
    Command *cmdPtr = (Command *) command;

    if ((cmdPtr == NULL) || (cmdPtr->hPtr == NULL)) {
	/*
	 * This should only happen if command was "created" after the
	 * interpreter began to be deleted, so there isn't really any command.
	 * Just return an empty string.
	 */

	return "";

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_GetCommandInfoFromToken(`, lines 1358–1468. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
    Tcl_CmdInfo info2;
    Tcl_Command buildInfoCmd = Tcl_CreateObjCommand(interp, "::tcl::build-info",
	    BuildInfoObjCmd, (void *)version, NULL);
    Tcl_GetCommandInfoFromToken(buildInfoCmd, &info2);
    info2.objProc2 = BuildInfoObjCmd2;
    info2.objClientData2 = (void *)version;
    Tcl_SetCommandInfoFromToken(buildInfoCmd, &info2);

    if (TclTommath_Init(interp) != TCL_OK) {
	Tcl_Panic("%s", Tcl_GetStringResult(interp));
    }

    if (TclOOInit(interp) != TCL_OK) {
	Tcl_Panic("%s", Tcl_GetStringResult(interp));
    }

    if (TclZlibInit(interp) != TCL_OK || TclZipfs_Init(interp) != TCL_OK) {
	Tcl_Panic("%s", Tcl_GetStringResult(interp));
    }

    TOP_CB(iPtr) = NULL;
    return interp;
}

static void
DeleteOpCmdClientData(
    void *clientData)
{
    TclOpCmdClientData *occdPtr = (TclOpCmdClientData *)clientData;

    Tcl_Free(occdPtr);
}

/*
 * ---------------------------------------------------------------------
 *
 * TclRegisterCommandTypeName, TclGetCommandTypeName --
 *
 *	Command type registration and lookup mechanism. Everything is keyed by
 *	the Tcl_ObjCmdProc for the command, and that is used as the *key* into
 *	the hash table that maps to constant strings that are names. (It is
 *	recommended that those names be ASCII.)
 *
 * ---------------------------------------------------------------------
 */

void
TclRegisterCommandTypeName(
    Tcl_ObjCmdProc *implementationProc,
    const char *nameStr)
{
    Tcl_HashEntry *hPtr;

    Tcl_MutexLock(&commandTypeLock);
    if (commandTypeInit == 0) {
	Tcl_InitHashTable(&commandTypeTable, TCL_ONE_WORD_KEYS);
	commandTypeInit = 1;
    }
    if (nameStr != NULL) {
	int isNew;

	hPtr = Tcl_CreateHashEntry(&commandTypeTable,
		(void *)implementationProc, &isNew);
	Tcl_SetHashValue(hPtr, nameStr);
    } else {
	hPtr = Tcl_FindHashEntry(&commandTypeTable,
		implementationProc);
	if (hPtr != NULL) {
	    Tcl_DeleteHashEntry(hPtr);
	}
    }
    Tcl_MutexUnlock(&commandTypeLock);
}

const char *
TclGetCommandTypeName(
    Tcl_Command command)
{
    Command *cmdPtr = (Command *) command;
    Tcl_ObjCmdProc *procPtr = cmdPtr->objProc;
    const char *name = "native";

    if (procPtr == NULL) {
	procPtr = cmdPtr->nreProc;
    }
    Tcl_MutexLock(&commandTypeLock);
    if (commandTypeInit) {
	Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&commandTypeTable, procPtr);

	if (hPtr && Tcl_GetHashValue(hPtr)) {
	    name = (const char *) Tcl_GetHashValue(hPtr);
	}
    }
    Tcl_MutexUnlock(&commandTypeLock);

    return name;
}

/*
 *----------------------------------------------------------------------
 *
 * TclHideUnsafeCommands --
 *
 *	Hides base commands that are not marked as safe from this interpreter.
 *
 * Results:
 *	TCL_OK if it succeeds, TCL_ERROR else.
 *
 * Side effects:
 *	Hides functionality in an interpreter.
 *

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_GetCommandInfoFromToken(`, lines 3512–3621. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
    Tcl_Command cmd;

    cmd = Tcl_FindCommand(interp, cmdName, NULL, /*flags*/ 0);
    return Tcl_GetCommandInfoFromToken(cmd, infoPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandInfoFromToken --
 *
 *	Returns various information about a Tcl command.
 *
 * Results:
 *	Copies information from the command identified by 'cmd' into a
 *	caller-supplied structure and returns 1. If the 'cmd' is NULL, leaves
 *	the structure untouched and returns 0.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_GetCommandInfoFromToken(
    Tcl_Command cmd,
    Tcl_CmdInfo *infoPtr)
{
    Command *cmdPtr;		/* Internal representation of the command */

    if (cmd == NULL) {
	return 0;
    }

    /*
     * Set isNativeObjectProc 1 if objProc was registered by a call to
     * Tcl_CreateObjCommand. Set isNativeObjectProc 2 if objProc was
     * registered by a call to Tcl_CreateObjCommand2. Otherwise set it to 0.
     */

    cmdPtr = (Command *) cmd;
    infoPtr->isNativeObjectProc =
	    (cmdPtr->objProc != InvokeStringCommand);
    infoPtr->objProc = cmdPtr->objProc;
    infoPtr->objClientData = cmdPtr->objClientData;
    infoPtr->proc = cmdPtr->proc;
    infoPtr->clientData = cmdPtr->clientData;
    if (cmdPtr->deleteProc == CmdWrapperDeleteProc) {
	CmdWrapperInfo *info = (CmdWrapperInfo *)cmdPtr->deleteData;
	infoPtr->deleteProc = info->deleteProc;
	infoPtr->deleteData = info->deleteData;
	infoPtr->objProc2 = info->proc;
	infoPtr->objClientData2 = info->clientData;
	if (cmdPtr->objProc == CmdWrapperProc) {
	    infoPtr->isNativeObjectProc = 2;
	}
    } else {
	infoPtr->deleteProc = cmdPtr->deleteProc;
	infoPtr->deleteData = cmdPtr->deleteData;
	infoPtr->objProc2 = CmdWrapper2Proc;
	infoPtr->objClientData2 = cmdPtr;
    }
    infoPtr->namespacePtr = (Tcl_Namespace *) cmdPtr->nsPtr;
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandName --
 *
 *	Given a token returned by Tcl_CreateObjCommand, this function returns the
 *	current name of the command (which may have changed due to renaming).
 *
 * Results:
 *	The return value is the name of the given command.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_GetCommandName(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Command command)	/* Token for command returned by a previous
				 * call to Tcl_CreateObjCommand. The command must
				 * not have been deleted. */
{
    Command *cmdPtr = (Command *) command;

    if ((cmdPtr == NULL) || (cmdPtr->hPtr == NULL)) {
	/*
	 * This should only happen if command was "created" after the
	 * interpreter began to be deleted, so there isn't really any command.
	 * Just return an empty string.
	 */

	return "";
    }

    return (const char *)Tcl_GetHashKey(cmdPtr->hPtr->tablePtr, cmdPtr->hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFullName --

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_GetCommandInfoFromToken(`, lines 3534–3644. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
 */

int
Tcl_GetCommandInfoFromToken(
    Tcl_Command cmd,
    Tcl_CmdInfo *infoPtr)
{
    Command *cmdPtr;		/* Internal representation of the command */

    if (cmd == NULL) {
	return 0;
    }

    /*
     * Set isNativeObjectProc 1 if objProc was registered by a call to
     * Tcl_CreateObjCommand. Set isNativeObjectProc 2 if objProc was
     * registered by a call to Tcl_CreateObjCommand2. Otherwise set it to 0.
     */

    cmdPtr = (Command *) cmd;
    infoPtr->isNativeObjectProc =
	    (cmdPtr->objProc != InvokeStringCommand);
    infoPtr->objProc = cmdPtr->objProc;
    infoPtr->objClientData = cmdPtr->objClientData;
    infoPtr->proc = cmdPtr->proc;
    infoPtr->clientData = cmdPtr->clientData;
    if (cmdPtr->deleteProc == CmdWrapperDeleteProc) {
	CmdWrapperInfo *info = (CmdWrapperInfo *)cmdPtr->deleteData;
	infoPtr->deleteProc = info->deleteProc;
	infoPtr->deleteData = info->deleteData;
	infoPtr->objProc2 = info->proc;
	infoPtr->objClientData2 = info->clientData;
	if (cmdPtr->objProc == CmdWrapperProc) {
	    infoPtr->isNativeObjectProc = 2;
	}
    } else {
	infoPtr->deleteProc = cmdPtr->deleteProc;
	infoPtr->deleteData = cmdPtr->deleteData;
	infoPtr->objProc2 = CmdWrapper2Proc;
	infoPtr->objClientData2 = cmdPtr;
    }
    infoPtr->namespacePtr = (Tcl_Namespace *) cmdPtr->nsPtr;
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandName --
 *
 *	Given a token returned by Tcl_CreateObjCommand, this function returns the
 *	current name of the command (which may have changed due to renaming).
 *
 * Results:
 *	The return value is the name of the given command.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_GetCommandName(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Command command)	/* Token for command returned by a previous
				 * call to Tcl_CreateObjCommand. The command must
				 * not have been deleted. */
{
    Command *cmdPtr = (Command *) command;

    if ((cmdPtr == NULL) || (cmdPtr->hPtr == NULL)) {
	/*
	 * This should only happen if command was "created" after the
	 * interpreter began to be deleted, so there isn't really any command.
	 * Just return an empty string.
	 */

	return "";
    }

    return (const char *)Tcl_GetHashKey(cmdPtr->hPtr->tablePtr, cmdPtr->hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFullName --
 *
 *	Given a token returned by, e.g., Tcl_CreateObjCommand or Tcl_FindCommand,
 *	this function appends to an object the command's full name, qualified
 *	by a sequence of parent namespace names. The command's fully-qualified
 *	name may have changed due to renaming.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	The command's fully-qualified name is appended to the string
 *	representation of objPtr.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_GetCommandFullName(
    Tcl_Interp *interp,		/* Interpreter containing the command. */
    Tcl_Command command,	/* Token for command returned by a previous
				 * call to Tcl_CreateObjCommand. The command must
				 * not have been deleted. */
    Tcl_Obj *objPtr)		/* Points to the object onto which the

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclObj.c`, function `AllocObjEntry(`, lines 206–318. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-9.0.4-tclObj.c`.

```text
 * Prototypes for the array hash key methods.
 */

static Tcl_HashEntry *	AllocObjEntry(Tcl_HashTable *tablePtr, void *keyPtr);

/*
 * Prototypes for the CommandName object type.
 */

static void		DupCmdNameInternalRep(Tcl_Obj *objPtr,
			    Tcl_Obj *copyPtr);
static void		FreeCmdNameInternalRep(Tcl_Obj *objPtr);
static int		SetCmdNameFromAny(Tcl_Interp *interp, Tcl_Obj *objPtr);

/*
 * The structures below defines the Tcl object types defined in this file by
 * means of functions that can be invoked by generic object code. See also
 * tclStringObj.c, tclListObj.c, tclByteCode.c for other type manager
 * implementations.
 */

const Tcl_ObjType tclBooleanType= {
    "boolean",			/* name */
    NULL,			/* freeIntRepProc */
    NULL,			/* dupIntRepProc */
    NULL,			/* updateStringProc */
    TclSetBooleanFromAny,	/* setFromAnyProc */
    TCL_OBJTYPE_V1(TclLengthOne)
};
const Tcl_ObjType tclDoubleType= {
    "double",			/* name */
    NULL,			/* freeIntRepProc */
    NULL,			/* dupIntRepProc */
    UpdateStringOfDouble,	/* updateStringProc */
    SetDoubleFromAny,		/* setFromAnyProc */
    TCL_OBJTYPE_V1(TclLengthOne)
};
const Tcl_ObjType tclIntType = {
    "int",			/* name */
    NULL,			/* freeIntRepProc */
    NULL,			/* dupIntRepProc */
    UpdateStringOfInt,		/* updateStringProc */
    SetIntFromAny,		/* setFromAnyProc */
    TCL_OBJTYPE_V1(TclLengthOne)
};
const Tcl_ObjType tclBignumType = {
    "bignum",			/* name */
    FreeBignum,			/* freeIntRepProc */
    DupBignum,			/* dupIntRepProc */
    UpdateStringOfBignum,	/* updateStringProc */
    NULL,			/* setFromAnyProc */
    TCL_OBJTYPE_V1(TclLengthOne)
};

/*
 * The structure below defines the Tcl obj hash key type.
 */

const Tcl_HashKeyType tclObjHashKeyType = {
    TCL_HASH_KEY_TYPE_VERSION,	/* version */
    TCL_HASH_KEY_DIRECT_COMPARE,/* allows compare keys by pointers */
    TclHashObjKey,		/* hashKeyProc */
    TclCompareObjKeys,		/* compareKeysProc */
    AllocObjEntry,		/* allocEntryProc */
    TclFreeObjEntry		/* freeEntryProc */
};

/*
 * The structure below defines the command name Tcl object type by means of
 * functions that can be invoked by generic object code. Objects of this type
 * cache the Command pointer that results from looking up command names in the
 * command hashtable. Such objects appear as the zeroth ("command name")
 * argument in a Tcl command.
 *
 * NOTE: the ResolvedCmdName that gets cached is stored in the
 * twoPtrValue.ptr1 field, and the twoPtrValue.ptr2 field is unused. You might
 * think you could use the simpler otherValuePtr field to store the single
 * ResolvedCmdName pointer, but DO NOT DO THIS. It seems that some extensions
 * use the second internal pointer field of the twoPtrValue field for their
 * own purposes.
 *
 * TRICKY POINT! Some extensions update this structure! (Notably, these
 * include TclBlend and TCom). This is highly ill-advised on their part, but
 * does allow them to delete a command when references to it are gone, which
 * is fragile but useful given their somewhat-OO style. Because of this, this
 * structure MUST NOT be const so that the C compiler puts the data in
 * writable memory. [Bug 2558422] [Bug 07d13d99b0a9]
 * TODO: Provide a better API for those extensions so that they can coexist...
 */

Tcl_ObjType tclCmdNameType = {
    "cmdName",			/* name */
    FreeCmdNameInternalRep,	/* freeIntRepProc */
    DupCmdNameInternalRep,	/* dupIntRepProc */
    NULL,			/* updateStringProc */
    SetCmdNameFromAny,		/* setFromAnyProc */
    TCL_OBJTYPE_V0
};

/*
 * Structure containing a cached pointer to a command that is the result of
 * resolving the command's name in some namespace. It is the internal
 * representation for a cmdName object. It contains the pointer along with
 * some information that is used to check the pointer's validity.
 */

typedef struct ResolvedCmdName {
    Command *cmdPtr;		/* A cached Command pointer. */
    Namespace *refNsPtr;	/* Points to the namespace containing the
				 * reference (not the namespace that contains
				 * the referenced command). NULL if the name
				 * is fully qualified.*/
    size_t refNsId;		/* refNsPtr's unique namespace id. Used to

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclObj.c`, function `AllocObjEntry(`, lines 4120–4229. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-9.0.4-tclObj.c`.

```text
 */

static Tcl_HashEntry *
AllocObjEntry(
    TCL_UNUSED(Tcl_HashTable *),
    void *keyPtr)		/* Key to store in the hash table entry. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    Tcl_HashEntry *hPtr = (Tcl_HashEntry *)Tcl_Alloc(sizeof(Tcl_HashEntry));

    hPtr->key.objPtr = objPtr;
    Tcl_IncrRefCount(objPtr);
    hPtr->clientData = NULL;

    return hPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * TclCompareObjKeys --
 *
 *	Compares two Tcl_Obj * keys.
 *
 * Results:
 *	The return value is 0 if they are different and 1 if they are the
 *	same.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

int
TclCompareObjKeys(
    void *keyPtr,		/* New key to compare. */
    Tcl_HashEntry *hPtr)	/* Existing key to compare. */
{
    Tcl_Obj *objPtr1 = (Tcl_Obj *)keyPtr;
    Tcl_Obj *objPtr2 = (Tcl_Obj *)hPtr->key.oneWordValue;
    const char *p1, *p2;
    size_t l1, l2;

    /*
     * If the object pointers are the same then they match.
     * OPT: this comparison was moved to the caller

       if (objPtr1 == objPtr2) {
	   return 1;
       }
    */

    /*
     * Don't use Tcl_GetStringFromObj as it would prevent l1 and l2 being
     * in a register.
     */

    p1 = TclGetString(objPtr1);
    l1 = objPtr1->length;
    p2 = TclGetString(objPtr2);
    l2 = objPtr2->length;

    /*
     * Only compare if the string representations are of the same length.
     */

    if (l1 == l2) {
	for (;; p1++, p2++, l1--) {
	    if (*p1 != *p2) {
		break;
	    }
	    if (l1 == 0) {
		return 1;
	    }
	}
    }

    return 0;
}

/*
 *----------------------------------------------------------------------
 *
 * TclFreeObjEntry --
 *
 *	Frees space for a Tcl_HashEntry containing the Tcl_Obj * key.
 *
 * Results:
 *	The return value is a pointer to the created entry.
 *
 * Side effects:
 *	Decrements the reference count of the object.
 *
 *----------------------------------------------------------------------
 */

void
TclFreeObjEntry(
    Tcl_HashEntry *hPtr)	/* Hash entry to free. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *) hPtr->key.oneWordValue;

    Tcl_DecrRefCount(objPtr);
    Tcl_Free(hPtr);
}

/*
 *----------------------------------------------------------------------
 *

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclObj.c`, function `HashObjKey(`, lines 4243–4354. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-9.0.4-tclObj.c`.

```text
 */

size_t
TclHashObjKey(
    TCL_UNUSED(Tcl_HashTable *),
    void *keyPtr)		/* Key from which to compute hash value. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    Tcl_Size length;
    const char *string = Tcl_GetStringFromObj(objPtr, &length);
    size_t result = 0;

    /*
     * I tried a zillion different hash functions and asked many other people
     * for advice. Many people had their own favorite functions, all
     * different, but no-one had much idea why they were good ones. I chose
     * the one below (multiply by 9 and add new character) because of the
     * following reasons:
     *
     * 1. Multiplying by 10 is perfect for keys that are decimal strings, and
     *    multiplying by 9 is just about as good.
     * 2. Times-9 is (shift-left-3) plus (old). This means that each
     *    character's bits hang around in the low-order bits of the hash value
     *    for ever, plus they spread fairly rapidly up to the high-order bits
     *    to fill out the hash value. This seems works well both for decimal
     *    and non-decimal strings.
     *
     * Note that this function is very weak against malicious strings; it's
     * very easy to generate multiple keys that have the same hashcode. On the
     * other hand, that hardly ever actually occurs and this function *is*
     * very cheap, even by comparison with industry-standard hashes like FNV.
     * If real strength of hash is required though, use a custom hash based on
     * Bob Jenkins's lookup3(), but be aware that it's significantly slower.
     * Tcl does not use that level of strength because it typically does not
     * need it (and some of the aspects of that strength are genuinely
     * unnecessary given the rest of Tcl's hash machinery, and the fact that
     * we do not either transfer hashes to another machine, use them as a true
     * substitute for equality, or attempt to minimize work in rebuilding the
     * hash table).
     *
     * See also HashStringKey in tclHash.c.
     * See also HashString in tclLiteral.c.
     *
     * See [tcl-Feature Request #2958832]
     */

    if (length > 0) {
	result = UCHAR(*string);
	while (--length) {
	    result += (result << 3) + UCHAR(*++string);
	}
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFromObj --
 *
 *	Returns the command specified by the name in a Tcl_Obj.
 *
 * Results:
 *	Returns a token for the command if it is found. Otherwise, if it can't
 *	be found or there is an error, returns NULL.
 *
 * Side effects:
 *	May update the internal representation for the object, caching the
 *	command reference so that the next time this function is called with
 *	the same object, the command can be found quickly.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
Tcl_GetCommandFromObj(
    Tcl_Interp *interp,		/* The interpreter in which to resolve the
				 * command and to report errors. */
    Tcl_Obj *objPtr)	/* The object containing the command's name.
				 * If the name starts with "::", will be
				 * looked up in global namespace. Else, looked
				 * up first in the current namespace, then in
				 * global namespace. */
{
    ResolvedCmdName *resPtr;

    /*
     * Get the internal representation, converting to a command type if
     * needed. The internal representation is a ResolvedCmdName that points to
     * the actual command.
     *
     * Check the context namespace and the namespace epoch of the resolved
     * symbol to make sure that it is fresh. Note that we verify that the
     * namespace id of the context namespace is the same as the one we cached;
     * this insures that the namespace wasn't deleted and a new one created at
     * the same address with the same command epoch. Note that fully qualified
     * names have a NULL refNsPtr, these checks needn't be made.
     *
     * Check also that the command's epoch is up to date, and that the command
     * is not deleted.
     *
     * If any check fails, then force another conversion to the command type,
     * to discard the old rep and create a new one.
     */

    resPtr = (ResolvedCmdName *)objPtr->internalRep.twoPtrValue.ptr1;
    if (TclHasInternalRep(objPtr, &tclCmdNameType)) {
	Command *cmdPtr = resPtr->cmdPtr;

	if ((cmdPtr->cmdEpoch == resPtr->cmdEpoch)
		&& (interp == cmdPtr->nsPtr->interp)
		&& !(cmdPtr->nsPtr->flags & NS_DYING)) {

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclObj.c`, function `CompareObjKeys(`, lines 4152–4262. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-9.0.4-tclObj.c`.

```text
 */

int
TclCompareObjKeys(
    void *keyPtr,		/* New key to compare. */
    Tcl_HashEntry *hPtr)	/* Existing key to compare. */
{
    Tcl_Obj *objPtr1 = (Tcl_Obj *)keyPtr;
    Tcl_Obj *objPtr2 = (Tcl_Obj *)hPtr->key.oneWordValue;
    const char *p1, *p2;
    size_t l1, l2;

    /*
     * If the object pointers are the same then they match.
     * OPT: this comparison was moved to the caller

       if (objPtr1 == objPtr2) {
	   return 1;
       }
    */

    /*
     * Don't use Tcl_GetStringFromObj as it would prevent l1 and l2 being
     * in a register.
     */

    p1 = TclGetString(objPtr1);
    l1 = objPtr1->length;
    p2 = TclGetString(objPtr2);
    l2 = objPtr2->length;

    /*
     * Only compare if the string representations are of the same length.
     */

    if (l1 == l2) {
	for (;; p1++, p2++, l1--) {
	    if (*p1 != *p2) {
		break;
	    }
	    if (l1 == 0) {
		return 1;
	    }
	}
    }

    return 0;
}

/*
 *----------------------------------------------------------------------
 *
 * TclFreeObjEntry --
 *
 *	Frees space for a Tcl_HashEntry containing the Tcl_Obj * key.
 *
 * Results:
 *	The return value is a pointer to the created entry.
 *
 * Side effects:
 *	Decrements the reference count of the object.
 *
 *----------------------------------------------------------------------
 */

void
TclFreeObjEntry(
    Tcl_HashEntry *hPtr)	/* Hash entry to free. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *) hPtr->key.oneWordValue;

    Tcl_DecrRefCount(objPtr);
    Tcl_Free(hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclHashObjKey --
 *
 *	Compute a one-word summary of the string representation of the
 *	Tcl_Obj, which can be used to generate a hash index.
 *
 * Results:
 *	The return value is a one-word summary of the information in the
 *	string representation of the Tcl_Obj.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

size_t
TclHashObjKey(
    TCL_UNUSED(Tcl_HashTable *),
    void *keyPtr)		/* Key from which to compute hash value. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    Tcl_Size length;
    const char *string = Tcl_GetStringFromObj(objPtr, &length);
    size_t result = 0;

    /*
     * I tried a zillion different hash functions and asked many other people
     * for advice. Many people had their own favorite functions, all
     * different, but no-one had much idea why they were good ones. I chose
     * the one below (multiply by 9 and add new character) because of the
     * following reasons:
     *
     * 1. Multiplying by 10 is perfect for keys that are decimal strings, and

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclObj.c`, function `FreeObjEntry(`, lines 4215–4325. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-9.0.4-tclObj.c`.

```text
 */

void
TclFreeObjEntry(
    Tcl_HashEntry *hPtr)	/* Hash entry to free. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *) hPtr->key.oneWordValue;

    Tcl_DecrRefCount(objPtr);
    Tcl_Free(hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclHashObjKey --
 *
 *	Compute a one-word summary of the string representation of the
 *	Tcl_Obj, which can be used to generate a hash index.
 *
 * Results:
 *	The return value is a one-word summary of the information in the
 *	string representation of the Tcl_Obj.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

size_t
TclHashObjKey(
    TCL_UNUSED(Tcl_HashTable *),
    void *keyPtr)		/* Key from which to compute hash value. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    Tcl_Size length;
    const char *string = Tcl_GetStringFromObj(objPtr, &length);
    size_t result = 0;

    /*
     * I tried a zillion different hash functions and asked many other people
     * for advice. Many people had their own favorite functions, all
     * different, but no-one had much idea why they were good ones. I chose
     * the one below (multiply by 9 and add new character) because of the
     * following reasons:
     *
     * 1. Multiplying by 10 is perfect for keys that are decimal strings, and
     *    multiplying by 9 is just about as good.
     * 2. Times-9 is (shift-left-3) plus (old). This means that each
     *    character's bits hang around in the low-order bits of the hash value
     *    for ever, plus they spread fairly rapidly up to the high-order bits
     *    to fill out the hash value. This seems works well both for decimal
     *    and non-decimal strings.
     *
     * Note that this function is very weak against malicious strings; it's
     * very easy to generate multiple keys that have the same hashcode. On the
     * other hand, that hardly ever actually occurs and this function *is*
     * very cheap, even by comparison with industry-standard hashes like FNV.
     * If real strength of hash is required though, use a custom hash based on
     * Bob Jenkins's lookup3(), but be aware that it's significantly slower.
     * Tcl does not use that level of strength because it typically does not
     * need it (and some of the aspects of that strength are genuinely
     * unnecessary given the rest of Tcl's hash machinery, and the fact that
     * we do not either transfer hashes to another machine, use them as a true
     * substitute for equality, or attempt to minimize work in rebuilding the
     * hash table).
     *
     * See also HashStringKey in tclHash.c.
     * See also HashString in tclLiteral.c.
     *
     * See [tcl-Feature Request #2958832]
     */

    if (length > 0) {
	result = UCHAR(*string);
	while (--length) {
	    result += (result << 3) + UCHAR(*++string);
	}
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFromObj --
 *
 *	Returns the command specified by the name in a Tcl_Obj.
 *
 * Results:
 *	Returns a token for the command if it is found. Otherwise, if it can't
 *	be found or there is an error, returns NULL.
 *
 * Side effects:
 *	May update the internal representation for the object, caching the
 *	command reference so that the next time this function is called with
 *	the same object, the command can be found quickly.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
Tcl_GetCommandFromObj(
    Tcl_Interp *interp,		/* The interpreter in which to resolve the
				 * command and to report errors. */
    Tcl_Obj *objPtr)	/* The object containing the command's name.
				 * If the name starts with "::", will be
				 * looked up in global namespace. Else, looked
				 * up first in the current namespace, then in
				 * global namespace. */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 86–196. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 723–834. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Namespace *nsPtr = (Namespace *) Tcl_GetCurrentNamespace(interp);
    FOREACH_HASH_DECLS;
    Tcl_Size soughtLen;
    const char *soughtStr, *nameStr, *matchedStr = NULL;

    if (objc < 2) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"bad call of unknown handler", TCL_AUTO_LENGTH));
	OO_ERROR(interp, BAD_UNKNOWN);
	return TCL_ERROR;
    }

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 1975–2085. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
	return TCL_ERROR;
    }

    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    Class *clsPtr = oPtr->classPtr;
    if (!isInstanceExport && !clsPtr) {
	return ReportAbuse(interp);
    }

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `RenameDeleteMethod(`, lines 2423–2533. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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

	/*
	 * Remember to invalidate the cached constructor chain for this class.
	 * [Bug 2531577]
	 */

	if (clsPtr->constructorChainPtr) {
	    TclOODeleteChain(clsPtr->constructorChainPtr);
	    clsPtr->constructorChainPtr = NULL;
	}
	BumpGlobalEpoch(interp, clsPtr);
    }
}

void

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 832–942. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
	OO_ERROR(interp, BAD_UNKNOWN);
	return TCL_ERROR;
    }
    if (TclOOGetDefineCmdContext(interp) == NULL) {
	return TCL_ERROR;
    }

    soughtStr = TclGetStringFromObj(objv[1], &soughtLen);
    if (soughtLen == 0) {
	goto noMatch;
    }
    FOREACH_HASH_KEY(nameStr, &nsPtr->cmdTable) {
	if (strncmp(soughtStr, nameStr, soughtLen) == 0) {
	    if (matchedStr != NULL) {
		goto noMatch;
	    }
	    matchedStr = nameStr;
	}
    }

    if (matchedStr != NULL) {
	/*
	 * Got one match, and only one match!
	 */

	Tcl_Obj **newObjv = (Tcl_Obj **)
		TclStackAlloc(interp, sizeof(Tcl_Obj *) * (objc - 1));
	int result;

	newObjv[0] = Tcl_NewStringObj(matchedStr, TCL_AUTO_LENGTH);
	Tcl_IncrRefCount(newObjv[0]);
	if (objc > 2) {
	    memcpy(newObjv + 1, objv + 2, sizeof(Tcl_Obj *) * (objc - 2));
	}
	result = Tcl_EvalObjv(interp, objc - 1, newObjv, 0);
	Tcl_DecrRefCount(newObjv[0]);
	TclStackFree(interp, newObjv);
	return result;
    }

  noMatch:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "invalid command name \"%s\"", soughtStr));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "COMMAND", soughtStr, (char *)NULL);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * FindCommand --
 *
 *	Specialized version of Tcl_FindCommand that handles command prefixes
 *	and disallows namespace magic.
 *
 * ----------------------------------------------------------------------
 */

static Tcl_Command
FindCommand(
    Tcl_Interp *interp,
    Tcl_Obj *stringObj,
    Tcl_Namespace *const namespacePtr)
{
    Tcl_Size length;
    const char *nameStr, *string = TclGetStringFromObj(stringObj, &length);
    Namespace *const nsPtr = (Namespace *) namespacePtr;
    FOREACH_HASH_DECLS;
    Tcl_Command cmd, cmd2;

    /*
     * If someone is playing games, we stop playing right now.
     */

    if (string[0] == '\0' || strstr(string, "::") != NULL) {
	return NULL;
    }

    /*
     * Do the exact lookup first.
     */

    cmd = Tcl_FindCommand(interp, string, namespacePtr, TCL_NAMESPACE_ONLY);
    if (cmd != NULL) {
	return cmd;
    }

    /*
     * Bother, need to perform an approximate match. Iterate across the hash
     * table of commands in the namespace.
     */

    FOREACH_HASH(nameStr, cmd2, &nsPtr->cmdTable) {
	if (strncmp(string, nameStr, length) == 0) {
	    if (cmd != NULL) {
		return NULL;
	    }
	    cmd = cmd2;
	}
    }

    /*
     * Either we found one thing or we found nothing. Either way, return it.
     */

    return cmd;
}

/*
 * ----------------------------------------------------------------------
 *

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 991–1102. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
 */

Tcl_Object
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

/*
 * ----------------------------------------------------------------------
 *
 * GetClassInOuterContext, GetNamespaceInOuterContext --
 *
 *	Wrappers round Tcl_GetObjectFromObj and TclGetNamespaceFromObj to
 *	perform the lookup in the context that called oo::define (or
 *	equivalent). Note that this may have to go up multiple levels to get
 *	the level that we started doing definitions at.
 *
 * ----------------------------------------------------------------------
 */

static inline Class *
GetClassInOuterContext(
    Tcl_Interp *interp,
    Tcl_Obj *className,
    const char *errMsg)
{
    Interp *iPtr = (Interp *) interp;
    Object *oPtr;
    CallFrame *savedFramePtr = iPtr->varFramePtr;

    while (iPtr->varFramePtr->isProcCallFrame == FRAME_IS_OO_DEFINE
	    || iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME) {
	if (iPtr->varFramePtr->callerVarPtr == NULL) {
	    Tcl_Panic("getting outer context when already in global context");
	}
	iPtr->varFramePtr = iPtr->varFramePtr->callerVarPtr;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, className);
    iPtr->varFramePtr = savedFramePtr;
    if (oPtr == NULL) {
	return NULL;
    }
    if (oPtr->classPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(errMsg, TCL_AUTO_LENGTH));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CLASS",
		TclGetString(className), (char *)NULL);
	return NULL;
    }
    return oPtr->classPtr;
}

static inline Tcl_Namespace *
GetNamespaceInOuterContext(
    Tcl_Interp *interp,
    Tcl_Obj *namespaceName)
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Namespace *nsPtr;
    int result;
    CallFrame *savedFramePtr = iPtr->varFramePtr;

    while (iPtr->varFramePtr->isProcCallFrame == FRAME_IS_OO_DEFINE
	    || iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME) {
	if (iPtr->varFramePtr->callerVarPtr == NULL) {
	    Tcl_Panic("getting outer context when already in global context");
	}
	iPtr->varFramePtr = iPtr->varFramePtr->callerVarPtr;
    }
    result = TclGetNamespaceFromObj(interp, namespaceName, &nsPtr);
    iPtr->varFramePtr = savedFramePtr;
    if (result != TCL_OK) {
	return NULL;
    }
    return nsPtr;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1022–1132. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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

/*
 * ----------------------------------------------------------------------
 *
 * GetClassInOuterContext, GetNamespaceInOuterContext --
 *
 *	Wrappers round Tcl_GetObjectFromObj and TclGetNamespaceFromObj to
 *	perform the lookup in the context that called oo::define (or
 *	equivalent). Note that this may have to go up multiple levels to get
 *	the level that we started doing definitions at.
 *
 * ----------------------------------------------------------------------
 */

static inline Class *
GetClassInOuterContext(
    Tcl_Interp *interp,
    Tcl_Obj *className,
    const char *errMsg)
{
    Interp *iPtr = (Interp *) interp;
    Object *oPtr;
    CallFrame *savedFramePtr = iPtr->varFramePtr;

    while (iPtr->varFramePtr->isProcCallFrame == FRAME_IS_OO_DEFINE
	    || iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME) {
	if (iPtr->varFramePtr->callerVarPtr == NULL) {
	    Tcl_Panic("getting outer context when already in global context");
	}
	iPtr->varFramePtr = iPtr->varFramePtr->callerVarPtr;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, className);
    iPtr->varFramePtr = savedFramePtr;
    if (oPtr == NULL) {
	return NULL;
    }
    if (oPtr->classPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(errMsg, TCL_AUTO_LENGTH));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CLASS",
		TclGetString(className), (char *)NULL);
	return NULL;
    }
    return oPtr->classPtr;
}

static inline Tcl_Namespace *
GetNamespaceInOuterContext(
    Tcl_Interp *interp,
    Tcl_Obj *namespaceName)
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Namespace *nsPtr;
    int result;
    CallFrame *savedFramePtr = iPtr->varFramePtr;

    while (iPtr->varFramePtr->isProcCallFrame == FRAME_IS_OO_DEFINE
	    || iPtr->varFramePtr->isProcCallFrame == PRIVATE_FRAME) {
	if (iPtr->varFramePtr->callerVarPtr == NULL) {
	    Tcl_Panic("getting outer context when already in global context");
	}
	iPtr->varFramePtr = iPtr->varFramePtr->callerVarPtr;
    }
    result = TclGetNamespaceFromObj(interp, namespaceName, &nsPtr);
    iPtr->varFramePtr = savedFramePtr;
    if (result != TCL_OK) {
	return NULL;
    }
    return nsPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * GenerateErrorInfo --
 *
 *	Factored out code to generate part of the error trace messages.
 *
 * ----------------------------------------------------------------------
 */

static inline void
GenerateErrorInfo(
    Tcl_Interp *interp,		/* Where to store the error info trace. */
    Object *oPtr,		/* What object (or class) was being configured
				 * when the error occurred? */
    Tcl_Obj *savedNameObj,	/* Name of object saved from before script was
				 * evaluated, which is needed if the object
				 * goes away part way through execution. OTOH,
				 * if the object isn't deleted then its
				 * current name (post-execution) has to be
				 * used. This matters, because the object
				 * could have been renamed... */
    const char *typeOfSubject)	/* Part of the message, saying whether it was
				 * an object, class or class-as-object that
				 * was being configured. */
{
    Tcl_Size length;
    Tcl_Obj *realNameObj = Tcl_ObjectDeleted((Tcl_Object) oPtr)

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1533–1643. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Object *oPtr;
    int result;

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    if (objc < 2) {
	Tcl_SetObjResult(interp, TclOOObjectName(interp, oPtr));
	return TCL_OK;
    }

    bool isPrivate = IsPrivateDefine(interp);

    /*
     * Make the oo::objdefine namespace the current namespace and evaluate the
     * command(s).
     */

    nsPtr = TclOOGetDefineContextNamespace(interp, oPtr, false);
    if (InitDefineContext(interp, nsPtr, oPtr, objc, objv) != TCL_OK) {
	return TCL_ERROR;
    }
    if (isPrivate) {
	((Interp *) interp)->varFramePtr->isProcCallFrame = PRIVATE_FRAME;
    }

    AddRef(oPtr);
    if (objc == 2) {
	Tcl_Obj *objNameObj = TclOOObjectName(interp, oPtr);

	Tcl_IncrRefCount(objNameObj);
	result = TclEvalObjEx(interp, objv[1], 0,
		((Interp *) interp)->cmdFramePtr, 1);
	if (result == TCL_ERROR) {
	    GenerateErrorInfo(interp, oPtr, objNameObj, "class object");
	}
	TclDecrRefCount(objNameObj);
    } else {
	result = MagicDefinitionInvoke(interp, nsPtr, 1, objc, objv);
    }
    TclOODecrRefCount(oPtr);

    /*
     * Restore the previous "current" namespace.
     */

    TclPopStackFrame(interp);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineObjSelfObjCmd --
 *
 *	Implementation of the "self" subcommand of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineObjSelfObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr;

    if (objc != 1) {
	Tcl_WrongNumArgs(interp, 1, objv, NULL);
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, TclOOObjectName(interp, oPtr));
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefinePrivateObjCmd --
 *
 *	Implementation of the "private" subcommand of the "oo::define"
 *	and "oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefinePrivateObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstancePrivate = (clientData != NULL);
				/* Just so that we can generate the correct
				 * error message depending on the context of
				 * usage of this function. */
    Interp *iPtr = (Interp *) interp;
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int saved;			/* The saved flag. We restore it on exit so

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1607–1717. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }

    Tcl_SetObjResult(interp, TclOOObjectName(interp, oPtr));
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefinePrivateObjCmd --
 *
 *	Implementation of the "private" subcommand of the "oo::define"
 *	and "oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefinePrivateObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstancePrivate = (clientData != NULL);
				/* Just so that we can generate the correct
				 * error message depending on the context of
				 * usage of this function. */
    Interp *iPtr = (Interp *) interp;
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int saved;			/* The saved flag. We restore it on exit so
				 * that [private private ...] doesn't make
				 * things go weird. */
    int result;

    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc == 1) {
	Tcl_SetObjResult(interp, Tcl_NewBooleanObj(IsPrivateDefine(interp)));
	return TCL_OK;
    }

    /*
     * Change the frame type flag while evaluating the body.
     */

    saved = iPtr->varFramePtr->isProcCallFrame;
    iPtr->varFramePtr->isProcCallFrame = PRIVATE_FRAME;

    /*
     * Evaluate the body; standard pattern.
     */

    AddRef(oPtr);
    if (objc == 2) {
	Tcl_Obj *objNameObj = TclOOObjectName(interp, oPtr);

	Tcl_IncrRefCount(objNameObj);
	result = TclEvalObjEx(interp, objv[1], 0, iPtr->cmdFramePtr, 1);
	if (result == TCL_ERROR) {
	    GenerateErrorInfo(interp, oPtr, objNameObj,
		    isInstancePrivate ? "object" : "class");
	}
	TclDecrRefCount(objNameObj);
    } else {
	result = MagicDefinitionInvoke(interp, TclGetCurrentNamespace(interp),
		1, objc, objv);
    }
    TclOODecrRefCount(oPtr);

    /*
     * Restore the frame type flag to what it was previously.
     */

    iPtr->varFramePtr->isProcCallFrame = saved;
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineClassObjCmd --
 *
 *	Implementation of the "class" subcommand of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1639–1750. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
				 * error message depending on the context of
				 * usage of this function. */
    Interp *iPtr = (Interp *) interp;
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    int saved;			/* The saved flag. We restore it on exit so
				 * that [private private ...] doesn't make
				 * things go weird. */
    int result;

    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc == 1) {
	Tcl_SetObjResult(interp, Tcl_NewBooleanObj(IsPrivateDefine(interp)));
	return TCL_OK;
    }

    /*
     * Change the frame type flag while evaluating the body.
     */

    saved = iPtr->varFramePtr->isProcCallFrame;
    iPtr->varFramePtr->isProcCallFrame = PRIVATE_FRAME;

    /*
     * Evaluate the body; standard pattern.
     */

    AddRef(oPtr);
    if (objc == 2) {
	Tcl_Obj *objNameObj = TclOOObjectName(interp, oPtr);

	Tcl_IncrRefCount(objNameObj);
	result = TclEvalObjEx(interp, objv[1], 0, iPtr->cmdFramePtr, 1);
	if (result == TCL_ERROR) {
	    GenerateErrorInfo(interp, oPtr, objNameObj,
		    isInstancePrivate ? "object" : "class");
	}
	TclDecrRefCount(objNameObj);
    } else {
	result = MagicDefinitionInvoke(interp, TclGetCurrentNamespace(interp),
		1, objc, objv);
    }
    TclOODecrRefCount(oPtr);

    /*
     * Restore the frame type flag to what it was previously.
     */

    iPtr->varFramePtr->isProcCallFrame = saved;
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineClassObjCmd --
 *
 *	Implementation of the "class" subcommand of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1715–1826. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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
 * ----------------------------------------------------------------------
 *
 * TclOODefineConstructorObjCmd --
 *
 *	Implementation of the "constructor" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineConstructorObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);
    Tcl_Method method;
    Tcl_Size bodyLength;

    if (clsPtr == NULL) {
	return TCL_ERROR;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 1962–2072. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2074–2184. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
	return TCL_ERROR;
    }

    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    Class *clsPtr = oPtr->classPtr;
    if (!isInstanceExport && !clsPtr) {
	return ReportAbuse(interp);
    }

    for (Tcl_Size i = 1; i < objc; i++) {
	/*
	 * Exporting is done by adding the PUBLIC_METHOD flag to the method
	 * record. If there is no such method in this object or class (i.e.
	 * the method comes from something inherited from or that we're an
	 * instance of) then we put in a blank record with that flag; such
	 * records are skipped over by the call chain engine *except* for
	 * their flags member.
	 */

	if (isInstanceExport) {
	    changed |= ExportInstanceMethod(oPtr, objv[i]);
	} else {
	    changed |= ExportMethod(clsPtr, objv[i]);
	}
    }

    /*
     * Bump the right epoch if we actually changed anything.
     */

    if (changed) {
	if (isInstanceExport) {
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
 * TclOODefineForwardObjCmd --
 *
 *	Implementation of the "forward" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
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

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineInitialiseObjCmd --
 *
 *	Implementation of the "initialise" subcommand of the "oo::define"
 *	command.

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2143–2253. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineInitialiseObjCmd --
 *
 *	Implementation of the "initialise" subcommand of the "oo::define"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineInitialiseObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "body");
	return TCL_ERROR;
    }

    // Build the lambda
    Tcl_Object object = TclOOGetDefineCmdContext(interp);
    if (object == NULL) {
	return TCL_ERROR;
    }
    Tcl_Obj *lambdaWords[] = {
	Tcl_NewObj(),
	objv[1],
	TclNewNamespaceObj(Tcl_GetObjectNamespace(object))
    };

    // Delegate to [apply] to run it
    Tcl_Obj *applyArgs[] = {
	Tcl_NewStringObj("apply", -1),
	Tcl_NewListObj(3, lambdaWords)
    };
    Tcl_IncrRefCount(applyArgs[0]);
    Tcl_IncrRefCount(applyArgs[1]);
    int result = Tcl_ApplyObjCmd(NULL, interp, 2, applyArgs);
    Tcl_DecrRefCount(applyArgs[0]);
    Tcl_DecrRefCount(applyArgs[1]);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineMethodObjCmd --
 *
 *	Implementation of the "method" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2199–2310. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    }

    // Build the lambda
    Tcl_Object object = TclOOGetDefineCmdContext(interp);
    if (object == NULL) {
	return TCL_ERROR;
    }
    Tcl_Obj *lambdaWords[] = {
	Tcl_NewObj(),
	objv[1],
	TclNewNamespaceObj(Tcl_GetObjectNamespace(object))
    };

    // Delegate to [apply] to run it
    Tcl_Obj *applyArgs[] = {
	Tcl_NewStringObj("apply", -1),
	Tcl_NewListObj(3, lambdaWords)
    };
    Tcl_IncrRefCount(applyArgs[0]);
    Tcl_IncrRefCount(applyArgs[1]);
    int result = Tcl_ApplyObjCmd(NULL, interp, 2, applyArgs);
    Tcl_DecrRefCount(applyArgs[0]);
    Tcl_DecrRefCount(applyArgs[1]);
    return result;
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineMethodObjCmd --
 *
 *	Implementation of the "method" subcommand of the "oo::define" and
 *	"oo::objdefine" commands.
 *
 * ----------------------------------------------------------------------
 */

int
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2265–2376. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineClassMethodObjCmd --
 *
 *	Implementation of the "classmethod" subcommand of the "oo::define"
 *	command. Defines a class method. See define(n) for details.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineClassMethodObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    if (objc != 2 && objc != 4) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?args body?");
	return TCL_ERROR;
    }
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);
    if (!clsPtr) {
	return TCL_ERROR;
    }

    int flags = Tcl_StringMatch(TclGetString(objv[1]), PUBLIC_PATTERN)
	    ? PUBLIC_METHOD : 0;

    // Create the method on the delegate class if the caller gave arguments and body
    if (objc == 4) {
	Tcl_Obj *delegateName = TclOOGetClassDelegateName(clsPtr->thisPtr);
	Class *delegatePtr = TclOOGetClassFromObj(interp, delegateName);
	Tcl_BounceRefCount(delegateName);
	if (!delegatePtr) {
	    return TCL_ERROR;
	}
	if (IsPrivateDefine(interp)) {
	    flags = 0;
	}
	if (TclOONewProcMethod(interp, delegatePtr, flags, objv[1],
		objv[2], objv[3], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }

    // Make the connection to the delegate by forwarding
    if (IsPrivateDefine(interp)) {
	flags = TRUE_PRIVATE_METHOD;
    }
    Tcl_Obj *forwardArgs[] = {
	Tcl_NewStringObj("myclass", -1),
	objv[1]
    };
    Tcl_Obj *prefixObj = Tcl_NewListObj(2, forwardArgs);
    Method *mPtr = TclOONewForwardMethod(interp, clsPtr, flags,

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2408–2518. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
	return TCL_ERROR;
    }

    oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!isInstanceRenameMethod && !oPtr->classPtr) {
	return ReportAbuse(interp);
    }

    /*
     * Delete the method entry from the appropriate hash table, and transfer
     * the thing it points to to its new entry. To do this, we first need to
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

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 2463–2573. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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

	/*
	 * Remember to invalidate the cached constructor chain for this class.
	 * [Bug 2531577]
	 */

	if (clsPtr->constructorChainPtr) {
	    TclOODeleteChain(clsPtr->constructorChainPtr);
	    clsPtr->constructorChainPtr = NULL;
	}
	BumpGlobalEpoch(interp, clsPtr);
    }
}

void
Tcl_ClassSetDestructor(
    Tcl_Interp *interp,
    Tcl_Class clazz,
    Tcl_Method method)
{
    Class *clsPtr = (Class *) clazz;

    if (method != (Tcl_Method) clsPtr->destructorPtr) {
	TclOODelMethodRef(clsPtr->destructorPtr);
	clsPtr->destructorPtr = (Method *) method;
	if (clsPtr->destructorChainPtr) {
	    TclOODeleteChain(clsPtr->destructorChainPtr);
	    clsPtr->destructorChainPtr = NULL;
	}
	BumpGlobalEpoch(interp, clsPtr);
    }
}

/*
 * ----------------------------------------------------------------------
 *
 * TclOODefineSlots --
 *
 *	Create the "::oo::Slot" class and its standard instances. These are
 *	basically lists at the low level of TclOO; this provides a more
 *	consistent interface to them.
 *
 * ----------------------------------------------------------------------
 */

int
TclOODefineSlots(
    Foundation *fPtr)
{
    Tcl_Interp *interp = fPtr->interp;
    Tcl_Object object = Tcl_NewObjectInstance(interp, (Tcl_Class)
	    fPtr->classCls, "::oo::Slot", NULL, TCL_INDEX_NONE, NULL, 0);

    if (object == NULL) {
	return TCL_ERROR;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3595–3706. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj, *filterObj;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(filterObj, oPtr->filters) {
	Tcl_ListObjAppendElement(NULL, resultObj, filterObj);
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjFilter_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size filterc;
    Tcl_Obj **filterv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &filterc, &filterv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOObjectSetFilters(oPtr, filterc, filterv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjMixin_Get, ObjMixin_Set --
 *
 *	Implementation of the "mixin" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjMixin_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->mixins) {
	if (mixinPtr) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    TclOOObjectName(interp, mixinPtr->thisPtr));
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjMixin_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size mixinc, i;
    Tcl_Obj **mixinv;
    Class **mixins;		/* The references to the classes to actually
				 * install. */
    Tcl_HashTable uniqueCheck;	/* Note that this hash table is just used as a
				 * set of class references; it has no payload
				 * values and keys are always pointers. */
    int isNew;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3623–3734. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size filterc;
    Tcl_Obj **filterv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"filterList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &filterc, &filterv) != TCL_OK) {
	return TCL_ERROR;
    }

    TclOOObjectSetFilters(oPtr, filterc, filterv);
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjMixin_Get, ObjMixin_Set --
 *
 *	Implementation of the "mixin" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjMixin_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->mixins) {
	if (mixinPtr) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    TclOOObjectName(interp, mixinPtr->thisPtr));
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjMixin_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size mixinc, i;
    Tcl_Obj **mixinv;
    Class **mixins;		/* The references to the classes to actually
				 * install. */
    Tcl_HashTable uniqueCheck;	/* Note that this hash table is just used as a
				 * set of class references; it has no payload
				 * values and keys are always pointers. */
    int isNew;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &mixinc, &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = (Class **) TclStackAlloc(interp, sizeof(Class *) * mixinc);
    Tcl_InitHashTable(&uniqueCheck, TCL_ONE_WORD_KEYS);

    for (i = 0; i < mixinc; i++) {
	mixins[i] = GetClassInOuterContext(interp, mixinv[i],
		"may only mix in classes");
	if (mixins[i] == NULL) {
	    goto freeAndError;
	}
	(void) Tcl_CreateHashEntry(&uniqueCheck, mixins[i], &isNew);
	if (!isNew) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "class should only be a direct mixin once",
		    TCL_AUTO_LENGTH));
	    OO_ERROR(interp, REPETITIOUS);
	    goto freeAndError;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3662–3773. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Class *mixinPtr;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    FOREACH(mixinPtr, oPtr->mixins) {
	if (mixinPtr) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    TclOOObjectName(interp, mixinPtr->thisPtr));
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjMixin_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size mixinc, i;
    Tcl_Obj **mixinv;
    Class **mixins;		/* The references to the classes to actually
				 * install. */
    Tcl_HashTable uniqueCheck;	/* Note that this hash table is just used as a
				 * set of class references; it has no payload
				 * values and keys are always pointers. */
    int isNew;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &mixinc, &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = (Class **) TclStackAlloc(interp, sizeof(Class *) * mixinc);
    Tcl_InitHashTable(&uniqueCheck, TCL_ONE_WORD_KEYS);

    for (i = 0; i < mixinc; i++) {
	mixins[i] = GetClassInOuterContext(interp, mixinv[i],
		"may only mix in classes");
	if (mixins[i] == NULL) {
	    goto freeAndError;
	}
	(void) Tcl_CreateHashEntry(&uniqueCheck, mixins[i], &isNew);
	if (!isNew) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "class should only be a direct mixin once",
		    TCL_AUTO_LENGTH));
	    OO_ERROR(interp, REPETITIOUS);
	    goto freeAndError;
	}
    }

    TclOOObjectSetMixins(oPtr, mixinc, mixins);
    TclStackFree(interp, mixins);
    Tcl_DeleteHashTable(&uniqueCheck);
    return TCL_OK;

  freeAndError:
    TclStackFree(interp, mixins);
    Tcl_DeleteHashTable(&uniqueCheck);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjVars_Get, ObjVars_Set --
 *
 *	Implementation of the "variable" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjVars_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3694–3805. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size mixinc, i;
    Tcl_Obj **mixinv;
    Class **mixins;		/* The references to the classes to actually
				 * install. */
    Tcl_HashTable uniqueCheck;	/* Note that this hash table is just used as a
				 * set of class references; it has no payload
				 * values and keys are always pointers. */
    int isNew;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"mixinList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &mixinc, &mixinv) != TCL_OK) {
	return TCL_ERROR;
    }

    mixins = (Class **) TclStackAlloc(interp, sizeof(Class *) * mixinc);
    Tcl_InitHashTable(&uniqueCheck, TCL_ONE_WORD_KEYS);

    for (i = 0; i < mixinc; i++) {
	mixins[i] = GetClassInOuterContext(interp, mixinv[i],
		"may only mix in classes");
	if (mixins[i] == NULL) {
	    goto freeAndError;
	}
	(void) Tcl_CreateHashEntry(&uniqueCheck, mixins[i], &isNew);
	if (!isNew) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "class should only be a direct mixin once",
		    TCL_AUTO_LENGTH));
	    OO_ERROR(interp, REPETITIOUS);
	    goto freeAndError;
	}
    }

    TclOOObjectSetMixins(oPtr, mixinc, mixins);
    TclStackFree(interp, mixins);
    Tcl_DeleteHashTable(&uniqueCheck);
    return TCL_OK;

  freeAndError:
    TclStackFree(interp, mixins);
    Tcl_DeleteHashTable(&uniqueCheck);
    return TCL_ERROR;
}

/*
 * ----------------------------------------------------------------------
 *
 * ObjVars_Get, ObjVars_Set --
 *
 *	Implementation of the "variable" slot accessors of the "oo::objdefine"
 *	command.
 *
 * ----------------------------------------------------------------------
 */

static int
ObjVars_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    if (IsPrivateDefine(interp)) {
	PrivateVariableMapping *privatePtr;

	FOREACH_STRUCT(privatePtr, oPtr->privateVariables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, privatePtr->variableObj);
	}
    } else {
	Tcl_Obj *variableObj;

	FOREACH(variableObj, oPtr->variables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjVars_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3765–3876. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Obj *resultObj;
    Tcl_Size i;

    if (Tcl_ObjectContextSkippedArgs(context) != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		NULL);
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }

    TclNewObj(resultObj);
    if (IsPrivateDefine(interp)) {
	PrivateVariableMapping *privatePtr;

	FOREACH_STRUCT(privatePtr, oPtr->privateVariables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, privatePtr->variableObj);
	}
    } else {
	Tcl_Obj *variableObj;

	FOREACH(variableObj, oPtr->variables) {
	    Tcl_ListObjAppendElement(NULL, resultObj, variableObj);
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

static int
ObjVars_Set(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size varc, i;
    Tcl_Obj **varv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"variableList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &varc, &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    for (i = 0; i < varc; i++) {
	if (ValidateVarNameForResolver(interp, varv[i]) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (IsPrivateDefine(interp)) {
	InstallPrivateVariableMapping(&oPtr->privateVariables, varc, varv,
		oPtr->creationEpoch);
    } else {
	InstallStandardVariableMapping(&oPtr->variables, varc, varv);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_ResolveClass --
 *
 *	Implementation of the "Resolve" support method for some slots (those
 *	that are slots around a list of classes). This resolves possible class
 *	names to their fully-qualified names if possible.
 *
 * ----------------------------------------------------------------------
 */

static int
Slot_ResolveClass(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Size idx = Tcl_ObjectContextSkippedArgs(context);
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Class *clsPtr;

    /*
     * Check if were called wrongly. The definition context isn't used...
     * except that GetClassInOuterContext() assumes that it is there.
     */

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != idx + 1) {
	Tcl_WrongNumArgs(interp, idx, objv, "slotElement");
	return TCL_ERROR;
    }

    /*
     * Resolve the class if possible. If not, remove any resolution error and
     * return what we've got anyway as the failure might not be fatal overall.
     */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3803–3913. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Tcl_Size varc, i;
    Tcl_Obj **varv;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"variableList");
	return TCL_ERROR;
    } else if (oPtr == NULL) {
	return TCL_ERROR;
    }
    objv += Tcl_ObjectContextSkippedArgs(context);
    if (TclListObjGetElements(interp, objv[0], &varc, &varv) != TCL_OK) {
	return TCL_ERROR;
    }

    for (i = 0; i < varc; i++) {
	if (ValidateVarNameForResolver(interp, varv[i]) != TCL_OK) {
	    return TCL_ERROR;
	}
    }

    if (IsPrivateDefine(interp)) {
	InstallPrivateVariableMapping(&oPtr->privateVariables, varc, varv,
		oPtr->creationEpoch);
    } else {
	InstallStandardVariableMapping(&oPtr->variables, varc, varv);
    }
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Slot_ResolveClass --
 *
 *	Implementation of the "Resolve" support method for some slots (those
 *	that are slots around a list of classes). This resolves possible class
 *	names to their fully-qualified names if possible.
 *
 * ----------------------------------------------------------------------
 */

static int
Slot_ResolveClass(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Tcl_Size idx = Tcl_ObjectContextSkippedArgs(context);
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Class *clsPtr;

    /*
     * Check if were called wrongly. The definition context isn't used...
     * except that GetClassInOuterContext() assumes that it is there.
     */

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != idx + 1) {
	Tcl_WrongNumArgs(interp, idx, objv, "slotElement");
	return TCL_ERROR;
    }

    /*
     * Resolve the class if possible. If not, remove any resolution error and
     * return what we've got anyway as the failure might not be fatal overall.
     */

    clsPtr = GetClassInOuterContext(interp, objv[idx],
	    "USER SHOULD NOT SEE THIS MESSAGE");
    if (clsPtr == NULL) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, objv[idx]);
    } else {
	Tcl_SetObjResult(interp, TclOOObjectName(interp, clsPtr->thisPtr));
    }

    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Configurable_ClassReadableProps_Get, Configurable_ClassReadableProps_Set,
 * Configurable_ObjectReadableProps_Get, Configurable_ObjectReadableProps_Set --
 *
 *	Implementations of the "readableproperties" slot accessors for classes
 *	and instances.
 *
 * ----------------------------------------------------------------------
 */

static int
Configurable_ClassReadableProps_Get(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);

    if (clsPtr == NULL) {
	return TCL_ERROR;

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3855–3966. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
    Tcl_Obj *const *objv)
{
    Tcl_Size idx = Tcl_ObjectContextSkippedArgs(context);
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    Class *clsPtr;

    /*
     * Check if were called wrongly. The definition context isn't used...
     * except that GetClassInOuterContext() assumes that it is there.
     */

    if (oPtr == NULL) {
	return TCL_ERROR;
    } else if (objc != idx + 1) {
	Tcl_WrongNumArgs(interp, idx, objv, "slotElement");
	return TCL_ERROR;
    }

    /*
     * Resolve the class if possible. If not, remove any resolution error and
     * return what we've got anyway as the failure might not be fatal overall.
     */

    clsPtr = GetClassInOuterContext(interp, objv[idx],
	    "USER SHOULD NOT SEE THIS MESSAGE");
    if (clsPtr == NULL) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, objv[idx]);
    } else {
	Tcl_SetObjResult(interp, TclOOObjectName(interp, clsPtr->thisPtr));
    }

    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------
 *
 * Configurable_ClassReadableProps_Get, Configurable_ClassReadableProps_Set,
 * Configurable_ObjectReadableProps_Get, Configurable_ObjectReadableProps_Set --
 *
 *	Implementations of the "readableproperties" slot accessors for classes
 *	and instances.
 *
 * ----------------------------------------------------------------------
 */

static int
Configurable_ClassReadableProps_Get(
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

    Tcl_SetObjResult(interp, TclOOGetPropertyList(&clsPtr->properties.readable));
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3959–4070. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 3981–4092. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 4075–4186. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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
    bool present = false, changed = false;
    Tcl_Size i;
    Tcl_Obj *other;

    Tcl_SetListObj(listObj, 0, NULL);
    FOREACH(other, *propsList) {
	if (!TclStringCmp(propName, other, 1, 0, TCL_INDEX_NONE)) {
	    present = true;
	    if (!addingProp) {
		changed = true;
		continue;
	    }
	}
	Tcl_ListObjAppendElement(NULL, listObj, other);
    }
    if (!present && addingProp) {
	Tcl_ListObjAppendElement(NULL, listObj, propName);
	changed = true;
    }
    return changed;
}

void
TclOORegisterInstanceProperty(
    Object *oPtr,		/* Object that owns the property slots. */
    Tcl_Obj *propName,		/* Property to add/remove. Must include the
				 * hyphen if one is desired; this is the value
				 * that is actually placed in the slot. */
    bool registerReader,	/* True if we're adding the property name to
				 * the readable property slot. False if we're
				 * removing the property name from the slot. */
    bool registerWriter)	/* True if we're adding the property name to
				 * the writable property slot. False if we're
				 * removing the property name from the slot. */
{
    Tcl_Obj *listObj = Tcl_NewObj();	/* Working buffer. */
    Tcl_Obj **objv;
    Tcl_Size count;

    if (BuildPropertyList(&oPtr->properties.readable, propName, registerReader,
	    listObj)) {
	TclListObjGetElements(NULL, listObj, &count, &objv);

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOOGetDefineCmdContext(`, lines 4097–4208. Retained reference `rust-tcl-registry-tests-data-native_empty_tcloo_lifecycle231-sources-9.1.0-tclOODefineCmds.c`.

```text
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
    bool present = false, changed = false;
    Tcl_Size i;
    Tcl_Obj *other;

    Tcl_SetListObj(listObj, 0, NULL);
    FOREACH(other, *propsList) {
	if (!TclStringCmp(propName, other, 1, 0, TCL_INDEX_NONE)) {
	    present = true;
	    if (!addingProp) {
		changed = true;
		continue;
	    }
	}
	Tcl_ListObjAppendElement(NULL, listObj, other);
    }
    if (!present && addingProp) {
	Tcl_ListObjAppendElement(NULL, listObj, propName);
	changed = true;
    }
    return changed;
}

void
TclOORegisterInstanceProperty(
    Object *oPtr,		/* Object that owns the property slots. */
    Tcl_Obj *propName,		/* Property to add/remove. Must include the
				 * hyphen if one is desired; this is the value
				 * that is actually placed in the slot. */
    bool registerReader,	/* True if we're adding the property name to
				 * the readable property slot. False if we're
				 * removing the property name from the slot. */
    bool registerWriter)	/* True if we're adding the property name to
				 * the writable property slot. False if we're
				 * removing the property name from the slot. */
{
    Tcl_Obj *listObj = Tcl_NewObj();	/* Working buffer. */
    Tcl_Obj **objv;
    Tcl_Size count;

    if (BuildPropertyList(&oPtr->properties.readable, propName, registerReader,
	    listObj)) {
	TclListObjGetElements(NULL, listObj, &count, &objv);
	TclOOInstallReadableProperties(&oPtr->properties, count, objv);
    }

    if (BuildPropertyList(&oPtr->properties.writable, propName, registerWriter,
	    listObj)) {
	TclListObjGetElements(NULL, listObj, &count, &objv);
	TclOOInstallWritableProperties(&oPtr->properties, count, objv);
    }
    Tcl_BounceRefCount(listObj);
}

void
TclOORegisterProperty(
    Class *clsPtr,		/* Class that owns the property slots. */
    Tcl_Obj *propName,		/* Property to add/remove. Must include the
				 * hyphen if one is desired; this is the value
				 * that is actually placed in the slot. */
    bool registerReader,	/* True if we're adding the property name to
				 * the readable property slot. False if we're
				 * removing the property name from the slot. */
    bool registerWriter)	/* True if we're adding the property name to
				 * the writable property slot. False if we're

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `Tcl_NewMethod(`, lines 318–427. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `Tcl_NewMethod2(`, lines 342–451. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod(`, lines 200–311. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
#ifndef TCL_NO_DEPRECATED
#undef Tcl_NewInstanceMethod
Tcl_Method
Tcl_NewInstanceMethod(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Object object,		/* The object that has the method attached to
				 * it. */
    Tcl_Obj *nameObj,		/* The name of the method. May be NULL; if so,
				 * up to caller to manage storage (e.g., when
				 * it is a constructor or destructor). */
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
		"Tcl_NewInstanceMethod", "TCL_OO_METHOD_VERSION_1");
    }
    return TclNewInstanceMethod(NULL, object, nameObj, flags,
	    (const Tcl_MethodType2 *)typePtr, clientData);
}
#endif /* TCL_NO_DEPRECATED */

Tcl_Method
Tcl_NewInstanceMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Object object,		/* The object that has the method attached to
				 * it. */
    Tcl_Obj *nameObj,		/* The name of the method. May be NULL; if so,
				 * up to caller to manage storage (e.g., when
				 * it is a constructor or destructor). */
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
		"Tcl_NewInstanceMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewInstanceMethod(NULL, object, nameObj, flags,
	    typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * Tcl_NewMethod --
 *
 *	Attach a method to a class.
 *
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod2(`, lines 225–336. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
#endif /* TCL_NO_DEPRECATED */

Tcl_Method
Tcl_NewInstanceMethod2(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Object object,		/* The object that has the method attached to
				 * it. */
    Tcl_Obj *nameObj,		/* The name of the method. May be NULL; if so,
				 * up to caller to manage storage (e.g., when
				 * it is a constructor or destructor). */
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
		"Tcl_NewInstanceMethod2", "TCL_OO_METHOD_VERSION_2");
    }
    return TclNewInstanceMethod(NULL, object, nameObj, flags,
	    typePtr, clientData);
}

/*
 * ----------------------------------------------------------------------
 *
 * Tcl_NewMethod --
 *
 *	Attach a method to a class.
 *
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOMethod.c`, function `TclOODelMethodRef(`, lines 374–483. Retained reference `rust-tcl-registry-tests-data-native_oo_operand_stages_original-original-inputs-tmp--receiver-oo-operand-stages-request499--source-inspection--9.1.0--tclOOMethod.c`.

```text
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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `renamemethod`, lines 63–175. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclOO.c`.

```text
    {"initialize",	TclOODefineInitialiseObjCmd, 0},
    {"method",		TclOODefineMethodObjCmd, 0},
    {"private",		TclOODefinePrivateObjCmd, 0},
    {"renamemethod",	TclOODefineRenameMethodObjCmd, 0},
    {"self",		TclOODefineSelfObjCmd, 0},
    {"unexport",	TclOODefineUnexportObjCmd, 0},
    {NULL, NULL, 0}
}, objdefCmds[] = {
    {"class",		TclOODefineClassObjCmd, 1},
    {"deletemethod",	TclOODefineDeleteMethodObjCmd, 1},
    {"export",		TclOODefineExportObjCmd, 1},
    {"forward",		TclOODefineForwardObjCmd, 1},
    {"method",		TclOODefineMethodObjCmd, 1},
    {"private",		TclOODefinePrivateObjCmd, 1},
    {"renamemethod",	TclOODefineRenameMethodObjCmd, 1},
    {"self",		TclOODefineObjSelfObjCmd, 0},
    {"unexport",	TclOODefineUnexportObjCmd, 1},
    {NULL, NULL, 0}
};

/*
 * What sort of size of things we like to allocate.
 */

#define ALLOC_CHUNK 8

/*
 * Function declarations for things defined in this file.
 */

static Object *		AllocObject(Tcl_Interp *interp, const char *nameStr,
			    Namespace *nsPtr, const char *nsNameStr);
static int		CloneClassMethod(Tcl_Interp *interp, Class *clsPtr,
			    Method *mPtr, Tcl_Obj *namePtr,
			    Method **newMPtrPtr);
static int		CloneObjectMethod(Tcl_Interp *interp, Object *oPtr,
			    Method *mPtr, Tcl_Obj *namePtr);
static Tcl_NamespaceDeleteProc	DeletedHelpersNamespace;
static Tcl_NRPostProc	FinalizeAlloc;
static Tcl_NRPostProc	FinalizeNext;
static Tcl_NRPostProc	FinalizeObjectCall;
static inline void	InitClassPath(Tcl_Interp * interp, Class *clsPtr);
static void		InitClassSystemRoots(Tcl_Interp *interp,
			    Foundation *fPtr);
static int		InitFoundation(Tcl_Interp *interp);
static Tcl_InterpDeleteProc	KillFoundation;
static void		MakeAdditionalClasses(Foundation *fPtr,
			    Tcl_Namespace *defineNs,
			    Tcl_Namespace *objDefineNs);
static Tcl_CmdDeleteProc	MyDeleted;
static Tcl_NamespaceDeleteProc	ObjectNamespaceDeleted;
static Tcl_CommandTraceProc	ObjectRenamedTrace;
static inline void	RemoveClass(Class **list, size_t num, size_t idx);
static inline void	RemoveObject(Object **list, size_t num, size_t idx);
static inline void	SquelchCachedName(Object *oPtr);

static Tcl_ObjCmdProc2	PublicNRObjectCmd;
static Tcl_ObjCmdProc2	PrivateNRObjectCmd;
static Tcl_ObjCmdProc2	MyClassNRObjCmd;
static Tcl_CmdDeleteProc	MyClassDeleted;

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

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `renamemethod`, lines 74–186. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclOO.c`.

```text
    {"forward",		TclOODefineForwardObjCmd, 1},
    {"method",		TclOODefineMethodObjCmd, 1},
    {"private",		TclOODefinePrivateObjCmd, 1},
    {"renamemethod",	TclOODefineRenameMethodObjCmd, 1},
    {"self",		TclOODefineObjSelfObjCmd, 0},
    {"unexport",	TclOODefineUnexportObjCmd, 1},
    {NULL, NULL, 0}
};

/*
 * What sort of size of things we like to allocate.
 */

#define ALLOC_CHUNK 8

/*
 * Function declarations for things defined in this file.
 */

static Object *		AllocObject(Tcl_Interp *interp, const char *nameStr,
			    Namespace *nsPtr, const char *nsNameStr);
static int		CloneClassMethod(Tcl_Interp *interp, Class *clsPtr,
			    Method *mPtr, Tcl_Obj *namePtr,
			    Method **newMPtrPtr);
static int		CloneObjectMethod(Tcl_Interp *interp, Object *oPtr,
			    Method *mPtr, Tcl_Obj *namePtr);
static Tcl_NamespaceDeleteProc	DeletedHelpersNamespace;
static Tcl_NRPostProc	FinalizeAlloc;
static Tcl_NRPostProc	FinalizeNext;
static Tcl_NRPostProc	FinalizeObjectCall;
static inline void	InitClassPath(Tcl_Interp * interp, Class *clsPtr);
static void		InitClassSystemRoots(Tcl_Interp *interp,
			    Foundation *fPtr);
static int		InitFoundation(Tcl_Interp *interp);
static Tcl_InterpDeleteProc	KillFoundation;
static void		MakeAdditionalClasses(Foundation *fPtr,
			    Tcl_Namespace *defineNs,
			    Tcl_Namespace *objDefineNs);
static Tcl_CmdDeleteProc	MyDeleted;
static Tcl_NamespaceDeleteProc	ObjectNamespaceDeleted;
static Tcl_CommandTraceProc	ObjectRenamedTrace;
static inline void	RemoveClass(Class **list, size_t num, size_t idx);
static inline void	RemoveObject(Object **list, size_t num, size_t idx);
static inline void	SquelchCachedName(Object *oPtr);

static Tcl_ObjCmdProc2	PublicNRObjectCmd;
static Tcl_ObjCmdProc2	PrivateNRObjectCmd;
static Tcl_ObjCmdProc2	MyClassNRObjCmd;
static Tcl_CmdDeleteProc	MyClassDeleted;

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

/*
 * The scripted part of TclOO: (legacy) package registration. There's no C API
 * at all for doing this, not even internally to Tcl.
 */

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `typedef struct Tcl_MethodType`, lines 81–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
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

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `typedef struct Tcl_MethodType`, lines 99–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
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

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 83–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

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

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 101–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text

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

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 124–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
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

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.h`, function `TCL_OO_METHOD_VERSION`, lines 126–189. Retained reference `rust-tcl-registry-tests-data-native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h`.

```text
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

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tcl.h`, function `Information about commands`, lines 800–912. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-9.1.0-generic-tcl.h`.

```text

/*
 *----------------------------------------------------------------------------
 * Information about commands that is returned by Tcl_GetCommandInfo and
 * passed to Tcl_SetCommandInfo. objProc is an objc/objv object-based command
 * function while proc is a traditional Tcl argc/argv string-based function.
 * Tcl_CreateObjCommand and Tcl_CreateCommand ensure that both objProc and
 * proc are non-NULL and can be called to execute the command. However, it may
 * be faster to call one instead of the other. The member isNativeObjectProc
 * is set to 1 if an object-based function was registered by
 * Tcl_CreateObjCommand, and to 0 if a string-based function was registered by
 * Tcl_CreateCommand. The other function is typically set to a compatibility
 * wrapper that does string-to-object or object-to-string argument conversions
 * then calls the other function.
 */

typedef struct {
    int isNativeObjectProc;	/* 1 if objProc was registered by a call to
				 * Tcl_CreateObjCommand; 2 if objProc was registered by
				 * a call to Tcl_CreateObjCommand2; 0 otherwise.
				 * Tcl_SetCmdInfo does not modify this field. */
#ifdef TCL_NO_DEPRECATED
    void *objProcNotUsed;	/* Command's object-based function. */
    void *objClientDataNotUsed;	/* ClientData for object proc. */
#else
    Tcl_ObjCmdProc *objProc;	/* Command's object-based function. */
    void *objClientData;	/* ClientData for object proc. */
#endif
    Tcl_CmdProc *proc;		/* Command's string-based function. */
    void *clientData;		/* ClientData for string proc. */
    Tcl_CmdDeleteProc *deleteProc;
				/* Function to call when command is
				 * deleted. */
    void *deleteData;		/* Value to pass to deleteProc (usually the
				 * same as clientData). */
    Tcl_Namespace *namespacePtr;/* Points to the namespace that contains this
				 * command. Note that Tcl_SetCmdInfo will not
				 * change a command's namespace; use
				 * TclRenameCommand or Tcl_Eval (of 'rename')
				 * to do that. */
    Tcl_ObjCmdProc2 *objProc2;	/* Command's object2-based function. */
    void *objClientData2;	/* ClientData for object2 proc. */
} Tcl_CmdInfo;

/*
 *----------------------------------------------------------------------------
 * The structure defined below is used to hold dynamic strings. The only
 * fields that clients should use are string and length, accessible via the
 * macros Tcl_DStringValue and Tcl_DStringLength.
 */

#define TCL_DSTRING_STATIC_SIZE 200
typedef struct Tcl_DString {
    char *string;		/* Points to beginning of string: either
				 * staticSpace below or a malloced array. */
    Tcl_Size length;		/* Number of bytes in string excluding
				 * terminating nul */
    Tcl_Size spaceAvl;		/* Total number of bytes available for the
				 * string and its terminating NULL char. */
    char staticSpace[TCL_DSTRING_STATIC_SIZE];
				/* Space to use in common case where string is
				 * small. */
} Tcl_DString;

#define Tcl_DStringLength(dsPtr) ((dsPtr)->length)
#define Tcl_DStringValue(dsPtr) ((dsPtr)->string)

/*
 * Definitions for the maximum number of digits of precision that may be
 * produced by Tcl_PrintDouble, and the number of bytes of buffer space
 * required by Tcl_PrintDouble.
 */

#define TCL_MAX_PREC		17
#define TCL_DOUBLE_SPACE	(TCL_MAX_PREC+10)

/*
 * Definition for a number of bytes of buffer space sufficient to hold the
 * string representation of an integer in base 10 (assuming the existence of
 * 64-bit integers).
 */

#define TCL_INTEGER_SPACE	(3*(int)sizeof(Tcl_WideInt))

/*
 *----------------------------------------------------------------------------
 * Type values returned by Tcl_GetNumberFromObj
 *	TCL_NUMBER_INT		Representation is a Tcl_WideInt
 *	TCL_NUMBER_BIG		Representation is an mp_int
 *	TCL_NUMBER_DOUBLE	Representation is a double
 *	TCL_NUMBER_NAN		Value is NaN.
 */

#define TCL_NUMBER_INT		2
#define TCL_NUMBER_BIG		3
#define TCL_NUMBER_DOUBLE	4
#define TCL_NUMBER_NAN		5

/*
 * Flag values passed to Tcl_ConvertElement.
 * TCL_DONT_USE_BRACES forces it not to enclose the element in braces, but to
 *	use backslash quoting instead.
 * TCL_DONT_QUOTE_HASH disables the default quoting of the '#' character. It
 *	is safe to leave the hash unquoted when the element is not the first
 *	element of a list, and this flag can be used by the caller to indicate
 *	that condition.
 */

#define TCL_DONT_USE_BRACES	1
#define TCL_DONT_QUOTE_HASH	8

/*
 * Flags that may be passed to Tcl_GetIndexFromObj.

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tcl.h`, function `Tcl_ObjCmdProc2 *objProc2`, lines 837–949. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-9.1.0-generic-tcl.h`.

```text
				 * change a command's namespace; use
				 * TclRenameCommand or Tcl_Eval (of 'rename')
				 * to do that. */
    Tcl_ObjCmdProc2 *objProc2;	/* Command's object2-based function. */
    void *objClientData2;	/* ClientData for object2 proc. */
} Tcl_CmdInfo;

/*
 *----------------------------------------------------------------------------
 * The structure defined below is used to hold dynamic strings. The only
 * fields that clients should use are string and length, accessible via the
 * macros Tcl_DStringValue and Tcl_DStringLength.
 */

#define TCL_DSTRING_STATIC_SIZE 200
typedef struct Tcl_DString {
    char *string;		/* Points to beginning of string: either
				 * staticSpace below or a malloced array. */
    Tcl_Size length;		/* Number of bytes in string excluding
				 * terminating nul */
    Tcl_Size spaceAvl;		/* Total number of bytes available for the
				 * string and its terminating NULL char. */
    char staticSpace[TCL_DSTRING_STATIC_SIZE];
				/* Space to use in common case where string is
				 * small. */
} Tcl_DString;

#define Tcl_DStringLength(dsPtr) ((dsPtr)->length)
#define Tcl_DStringValue(dsPtr) ((dsPtr)->string)

/*
 * Definitions for the maximum number of digits of precision that may be
 * produced by Tcl_PrintDouble, and the number of bytes of buffer space
 * required by Tcl_PrintDouble.
 */

#define TCL_MAX_PREC		17
#define TCL_DOUBLE_SPACE	(TCL_MAX_PREC+10)

/*
 * Definition for a number of bytes of buffer space sufficient to hold the
 * string representation of an integer in base 10 (assuming the existence of
 * 64-bit integers).
 */

#define TCL_INTEGER_SPACE	(3*(int)sizeof(Tcl_WideInt))

/*
 *----------------------------------------------------------------------------
 * Type values returned by Tcl_GetNumberFromObj
 *	TCL_NUMBER_INT		Representation is a Tcl_WideInt
 *	TCL_NUMBER_BIG		Representation is an mp_int
 *	TCL_NUMBER_DOUBLE	Representation is a double
 *	TCL_NUMBER_NAN		Value is NaN.
 */

#define TCL_NUMBER_INT		2
#define TCL_NUMBER_BIG		3
#define TCL_NUMBER_DOUBLE	4
#define TCL_NUMBER_NAN		5

/*
 * Flag values passed to Tcl_ConvertElement.
 * TCL_DONT_USE_BRACES forces it not to enclose the element in braces, but to
 *	use backslash quoting instead.
 * TCL_DONT_QUOTE_HASH disables the default quoting of the '#' character. It
 *	is safe to leave the hash unquoted when the element is not the first
 *	element of a list, and this flag can be used by the caller to indicate
 *	that condition.
 */

#define TCL_DONT_USE_BRACES	1
#define TCL_DONT_QUOTE_HASH	8

/*
 * Flags that may be passed to Tcl_GetIndexFromObj.
 * TCL_EXACT disallows abbreviated strings.
 * TCL_NULL_OK allows the empty string or NULL to return TCL_OK.
 *      The returned value will be -1;
 * TCL_INDEX_TEMP_TABLE disallows caching of lookups. A possible use case is
 *      a table that will not live long enough to make it worthwhile.
 */

#define TCL_EXACT		1
#define TCL_NULL_OK		32
#define TCL_INDEX_TEMP_TABLE	64

/*
 * Flags that may be passed to Tcl_UniCharToUtf.
 * TCL_COMBINE Combine surrogates
 */

#define TCL_COMBINE		0x1000000
/*
 *----------------------------------------------------------------------------
 * Flag values passed to Tcl_RecordAndEval, Tcl_EvalObj, Tcl_EvalObjv.
 * WARNING: these bit choices must not conflict with the bit choices for
 * evalFlag bits in tclInt.h!
 *
 * Meanings:
 *	TCL_NO_EVAL:		Just record this command
 *	TCL_EVAL_GLOBAL:	Execute script in global namespace
 *	TCL_EVAL_DIRECT:	Do not compile this script
 *	TCL_EVAL_INVOKE:	Magical Tcl_EvalObjv mode for aliases/ensembles
 *				o Run in iPtr->lookupNsPtr or global namespace
 *				o Cut out of error traces
 *				o Don't reset the flags controlling ensemble
 *				  error message rewriting.
 *	TCL_CANCEL_UNWIND:	Magical Tcl_CancelEval mode that causes the
 *				stack for the script in progress to be
 *				completely unwound.
 *	TCL_EVAL_NOERR:		Do no exception reporting at all, just return
 *				as the caller will report.

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tcl.h`, function `typedef struct Tcl_Obj`, lines 608–720. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-9.1.0-generic-tcl.h`.

```text
	struct Tcl_Obj *listPtr, struct Tcl_Obj **newObjPtr);
typedef int (Tcl_ObjTypeGetElements) (Tcl_Interp *interp,
	struct Tcl_Obj *listPtr, Tcl_Size *objcptr, struct Tcl_Obj ***objvptr);
typedef struct Tcl_Obj *(Tcl_ObjTypeSetElement) (Tcl_Interp *interp,
	struct Tcl_Obj *listPtr, Tcl_Size indexCount,
	struct Tcl_Obj *const indexArray[], struct Tcl_Obj *valueObj);
typedef int (Tcl_ObjTypeReplaceProc) (Tcl_Interp *interp,
	struct Tcl_Obj *listObj, Tcl_Size first, Tcl_Size numToDelete,
	Tcl_Size numToInsert, struct Tcl_Obj *const insertObjs[]);
typedef int (Tcl_ObjTypeInOperatorProc) (Tcl_Interp *interp,
	struct Tcl_Obj *valueObj, struct Tcl_Obj *listObj, int *boolResult);

#ifndef TCL_NO_DEPRECATED
#   define Tcl_PackageInitProc Tcl_LibraryInitProc
#   define Tcl_PackageUnloadProc Tcl_LibraryUnloadProc
#endif

/*
 *----------------------------------------------------------------------------
 * The following structure represents a type of object, which is a particular
 * internal representation for an object plus a set of functions that provide
 * standard operations on objects of that type.
 */

typedef struct Tcl_ObjType {
    const char *name;		/* Name of the type, e.g. "int". */
    Tcl_FreeInternalRepProc *freeIntRepProc;
				/* Called to free any storage for the type's
				 * internal rep. NULL if the internal rep does
				 * not need freeing. */
    Tcl_DupInternalRepProc *dupIntRepProc;
				/* Called to create a new object as a copy of
				 * an existing object. */
    Tcl_UpdateStringProc *updateStringProc;
				/* Called to update the string rep from the
				 * type's internal representation. */
    Tcl_SetFromAnyProc *setFromAnyProc;
				/* Called to convert the object's internal rep
				 * to this type. Frees the internal rep of the
				 * old type. Returns TCL_ERROR on failure. */
    size_t version;		/* Version field for future-proofing. */

    /* List emulation functions - ObjType Version 1 */
    Tcl_ObjTypeLengthProc *lengthProc;
				/* Return the [llength] of the AbstractList */
    Tcl_ObjTypeIndexProc *indexProc;
				/* Return a value (Tcl_Obj) at a given index */
    Tcl_ObjTypeSliceProc *sliceProc;
				/* Return an AbstractList for
				 * [lrange $al $start $end] */
    Tcl_ObjTypeReverseProc *reverseProc;
				/* Return an AbstractList for [lreverse $al] */
    Tcl_ObjTypeGetElements *getElementsProc;
				/* Return an objv[] of all elements in the list */
    Tcl_ObjTypeSetElement *setElementProc;
				/* Replace the element at the indicies with the
				 * given valueObj. */
    Tcl_ObjTypeReplaceProc *replaceProc;
				/* Replace sublist with another sublist */
    Tcl_ObjTypeInOperatorProc *inOperProc;
				/* "in" and "ni" expr list operation.
				 * Determine if the given string value matches
				 * an element in the list. */
} Tcl_ObjType;

#define TCL_OBJTYPE_V0 0, \
	0,0,0,0,0,0,0,0 /* Pre-Tcl 9 */
#define TCL_OBJTYPE_V1(a) offsetof(Tcl_ObjType, indexProc), \
	a,0,0,0,0,0,0,0 /* Tcl 9 Version 1 */
#define TCL_OBJTYPE_V2(a,b,c,d,e,f,g,h) sizeof(Tcl_ObjType),  \
	a,b,c,d,e,f,g,h /* Tcl 9 - AbstractLists */

/*
 * The following structure stores an internal representation (internalrep) for
 * a Tcl value. An internalrep is associated with an Tcl_ObjType when both
 * are stored in the same Tcl_Obj.  The routines of the Tcl_ObjType govern
 * the handling of the internalrep.
 */

typedef union Tcl_ObjInternalRep {/* The internal representation: */
    long longValue;		/*   - an long integer value. */
    double doubleValue;		/*   - a double-precision floating value. */
    void *otherValuePtr;	/*   - another, type-specific value, */
				/*     not used internally any more. */
    Tcl_WideInt wideValue;	/*   - an integer value >= 64bits */
    struct {			/*   - internal rep as two pointers. */
	void *ptr1;
	void *ptr2;
    } twoPtrValue;
    struct {			/*   - internal rep as a pointer and a long, */
	void *ptr;		/*     not used internally any more. */
	unsigned long value;
    } ptrAndLongRep;
    struct {			/*   - use for pointer and length reps */
	void *ptr;
	Tcl_Size size;
    } ptrAndSize;
} Tcl_ObjInternalRep;

/*
 * One of the following structures exists for each object in the Tcl system.
 * An object stores a value as either a string, some internal representation,
 * or both.
 */

typedef struct Tcl_Obj {
    Tcl_Size refCount;		/* When 0 the object will be freed. */
    char *bytes;		/* This points to the first byte of the
				 * object's string representation. The array
				 * must be followed by a null byte (i.e., at
				 * offset length) but may also contain
				 * embedded null characters. The array's
				 * storage is allocated by Tcl_Alloc. NULL means

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tcl.h`, function `typedef struct Tcl_Obj`, lines 629–740. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-9.1.0-generic-tcl.h`.

```text
 * standard operations on objects of that type.
 */

typedef struct Tcl_ObjType {
    const char *name;		/* Name of the type, e.g. "int". */
    Tcl_FreeInternalRepProc *freeIntRepProc;
				/* Called to free any storage for the type's
				 * internal rep. NULL if the internal rep does
				 * not need freeing. */
    Tcl_DupInternalRepProc *dupIntRepProc;
				/* Called to create a new object as a copy of
				 * an existing object. */
    Tcl_UpdateStringProc *updateStringProc;
				/* Called to update the string rep from the
				 * type's internal representation. */
    Tcl_SetFromAnyProc *setFromAnyProc;
				/* Called to convert the object's internal rep
				 * to this type. Frees the internal rep of the
				 * old type. Returns TCL_ERROR on failure. */
    size_t version;		/* Version field for future-proofing. */

    /* List emulation functions - ObjType Version 1 */
    Tcl_ObjTypeLengthProc *lengthProc;
				/* Return the [llength] of the AbstractList */
    Tcl_ObjTypeIndexProc *indexProc;
				/* Return a value (Tcl_Obj) at a given index */
    Tcl_ObjTypeSliceProc *sliceProc;
				/* Return an AbstractList for
				 * [lrange $al $start $end] */
    Tcl_ObjTypeReverseProc *reverseProc;
				/* Return an AbstractList for [lreverse $al] */
    Tcl_ObjTypeGetElements *getElementsProc;
				/* Return an objv[] of all elements in the list */
    Tcl_ObjTypeSetElement *setElementProc;
				/* Replace the element at the indicies with the
				 * given valueObj. */
    Tcl_ObjTypeReplaceProc *replaceProc;
				/* Replace sublist with another sublist */
    Tcl_ObjTypeInOperatorProc *inOperProc;
				/* "in" and "ni" expr list operation.
				 * Determine if the given string value matches
				 * an element in the list. */
} Tcl_ObjType;

#define TCL_OBJTYPE_V0 0, \
	0,0,0,0,0,0,0,0 /* Pre-Tcl 9 */
#define TCL_OBJTYPE_V1(a) offsetof(Tcl_ObjType, indexProc), \
	a,0,0,0,0,0,0,0 /* Tcl 9 Version 1 */
#define TCL_OBJTYPE_V2(a,b,c,d,e,f,g,h) sizeof(Tcl_ObjType),  \
	a,b,c,d,e,f,g,h /* Tcl 9 - AbstractLists */

/*
 * The following structure stores an internal representation (internalrep) for
 * a Tcl value. An internalrep is associated with an Tcl_ObjType when both
 * are stored in the same Tcl_Obj.  The routines of the Tcl_ObjType govern
 * the handling of the internalrep.
 */

typedef union Tcl_ObjInternalRep {/* The internal representation: */
    long longValue;		/*   - an long integer value. */
    double doubleValue;		/*   - a double-precision floating value. */
    void *otherValuePtr;	/*   - another, type-specific value, */
				/*     not used internally any more. */
    Tcl_WideInt wideValue;	/*   - an integer value >= 64bits */
    struct {			/*   - internal rep as two pointers. */
	void *ptr1;
	void *ptr2;
    } twoPtrValue;
    struct {			/*   - internal rep as a pointer and a long, */
	void *ptr;		/*     not used internally any more. */
	unsigned long value;
    } ptrAndLongRep;
    struct {			/*   - use for pointer and length reps */
	void *ptr;
	Tcl_Size size;
    } ptrAndSize;
} Tcl_ObjInternalRep;

/*
 * One of the following structures exists for each object in the Tcl system.
 * An object stores a value as either a string, some internal representation,
 * or both.
 */

typedef struct Tcl_Obj {
    Tcl_Size refCount;		/* When 0 the object will be freed. */
    char *bytes;		/* This points to the first byte of the
				 * object's string representation. The array
				 * must be followed by a null byte (i.e., at
				 * offset length) but may also contain
				 * embedded null characters. The array's
				 * storage is allocated by Tcl_Alloc. NULL means
				 * the string rep is invalid and must be
				 * regenerated from the internal rep.  Clients
				 * should use Tcl_GetStringFromObj or
				 * Tcl_GetString to get a pointer to the byte
				 * array as a readonly value. */
    Tcl_Size length;		/* The number of bytes at *bytes, not
				 * including the terminating null. */
    const Tcl_ObjType *typePtr;	/* Denotes the object's type. Always
				 * corresponds to the type of the object's
				 * internal rep. NULL indicates the object has
				 * no internal rep (has no type). */
    Tcl_ObjInternalRep internalRep;
				/* The internal representation: */
} Tcl_Obj;

/*
 *----------------------------------------------------------------------------
 * The following definitions support Tcl's namespace facility. Note: the first
 * five fields must match exactly the fields in a Namespace structure (see
 * tclInt.h).

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tcl.h`, function `typedef struct Tcl_Obj`, lines 710–821. Retained reference `rust-tcl-registry-tests-data-native_dict_update_write_error_lifetime398-request-sdk-9.1.0-generic-tcl.h`.

```text
 * or both.
 */

typedef struct Tcl_Obj {
    Tcl_Size refCount;		/* When 0 the object will be freed. */
    char *bytes;		/* This points to the first byte of the
				 * object's string representation. The array
				 * must be followed by a null byte (i.e., at
				 * offset length) but may also contain
				 * embedded null characters. The array's
				 * storage is allocated by Tcl_Alloc. NULL means
				 * the string rep is invalid and must be
				 * regenerated from the internal rep.  Clients
				 * should use Tcl_GetStringFromObj or
				 * Tcl_GetString to get a pointer to the byte
				 * array as a readonly value. */
    Tcl_Size length;		/* The number of bytes at *bytes, not
				 * including the terminating null. */
    const Tcl_ObjType *typePtr;	/* Denotes the object's type. Always
				 * corresponds to the type of the object's
				 * internal rep. NULL indicates the object has
				 * no internal rep (has no type). */
    Tcl_ObjInternalRep internalRep;
				/* The internal representation: */
} Tcl_Obj;

/*
 *----------------------------------------------------------------------------
 * The following definitions support Tcl's namespace facility. Note: the first
 * five fields must match exactly the fields in a Namespace structure (see
 * tclInt.h).
 */

typedef struct Tcl_Namespace {
    char *name;			/* The namespace's name within its parent
				 * namespace. This contains no ::'s. The name
				 * of the global namespace is "" although "::"
				 * is an synonym. */
    char *fullName;		/* The namespace's fully qualified name. This
				 * starts with ::. */
    void *clientData;		/* Arbitrary value associated with this
				 * namespace. */
    Tcl_NamespaceDeleteProc *deleteProc;
				/* Function invoked when deleting the
				 * namespace to, e.g., free clientData. */
    struct Tcl_Namespace *parentPtr;
				/* Points to the namespace that contains this
				 * one. NULL if this is the global
				 * namespace. */
} Tcl_Namespace;

/*
 *----------------------------------------------------------------------------
 * The following structure represents a call frame, or activation record. A
 * call frame defines a naming context for a procedure call: its local scope
 * (for local variables) and its namespace scope (used for non-local
 * variables; often the global :: namespace). A call frame can also define the
 * naming context for a namespace eval or namespace inscope command: the
 * namespace in which the command's code should execute. The Tcl_CallFrame
 * structures exist only while procedures or namespace eval/inscope's are
 * being executed, and provide a Tcl call stack.
 *
 * A call frame is initialized and pushed using Tcl_PushCallFrame and popped
 * using Tcl_PopCallFrame. Storage for a Tcl_CallFrame must be provided by the
 * Tcl_PushCallFrame caller, and callers typically allocate them on the C call
 * stack for efficiency. For this reason, Tcl_CallFrame is defined as a
 * structure and not as an opaque token. However, most Tcl_CallFrame fields
 * are hidden since applications should not access them directly; others are
 * declared as "dummyX".
 *
 * WARNING!! The structure definition must be kept consistent with the
 * CallFrame structure in tclInt.h. If you change one, change the other.
 */

typedef struct Tcl_CallFrame {
    Tcl_Namespace *nsPtr;	/* Current namespace for the call frame. */
    int dummy1;
    Tcl_Size dummy2;
    void *dummy3;
    void *dummy4;
    void *dummy5;
    Tcl_Size dummy6;
    void *dummy7;
    void *dummy8;
    Tcl_Size dummy9;
    void *dummy10;
    void *dummy11;
    void *dummy12;
    void *dummy13;
} Tcl_CallFrame;

/*
 *----------------------------------------------------------------------------
 * Information about commands that is returned by Tcl_GetCommandInfo and
 * passed to Tcl_SetCommandInfo. objProc is an objc/objv object-based command
 * function while proc is a traditional Tcl argc/argv string-based function.
 * Tcl_CreateObjCommand and Tcl_CreateCommand ensure that both objProc and
 * proc are non-NULL and can be called to execute the command. However, it may
 * be faster to call one instead of the other. The member isNativeObjectProc
 * is set to 1 if an object-based function was registered by
 * Tcl_CreateObjCommand, and to 0 if a string-based function was registered by
 * Tcl_CreateCommand. The other function is typically set to a compatibility
 * wrapper that does string-to-object or object-to-string argument conversions
 * then calls the other function.
 */

typedef struct {
    int isNativeObjectProc;	/* 1 if objProc was registered by a call to
				 * Tcl_CreateObjCommand; 2 if objProc was registered by
				 * a call to Tcl_CreateObjCommand2; 0 otherwise.
				 * Tcl_SetCmdInfo does not modify this field. */
#ifdef TCL_NO_DEPRECATED

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_GetCommandInfo(`, lines 3467–3577. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
 */

int
Tcl_GetCommandInfo(
    Tcl_Interp *interp,		/* Interpreter in which to look for
				 * command. */
    const char *cmdName,	/* Name of desired command. */
    Tcl_CmdInfo *infoPtr)	/* Where to store information about
				 * command. */
{
    Tcl_Command cmd;

    cmd = Tcl_FindCommand(interp, cmdName, NULL, /*flags*/ 0);
    return Tcl_GetCommandInfoFromToken(cmd, infoPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandInfoFromToken --
 *
 *	Returns various information about a Tcl command.
 *
 * Results:
 *	Copies information from the command identified by 'cmd' into a
 *	caller-supplied structure and returns 1. If the 'cmd' is NULL, leaves
 *	the structure untouched and returns 0.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_GetCommandInfoFromToken(
    Tcl_Command cmd,
    Tcl_CmdInfo *infoPtr)
{
    Command *cmdPtr;		/* Internal representation of the command */

    if (cmd == NULL) {
	return 0;
    }

    /*
     * Set isNativeObjectProc 1 if objProc was registered by a call to
     * Tcl_CreateObjCommand. Set isNativeObjectProc 2 if objProc was
     * registered by a call to Tcl_CreateObjCommand2. Otherwise set it to 0.
     */

    cmdPtr = (Command *) cmd;
    infoPtr->isNativeObjectProc =
	    (cmdPtr->objProc2 != InvokeStringCommand) ? 2 : 0;
    infoPtr->objProc2 = cmdPtr->objProc2;
    infoPtr->objClientData2 = cmdPtr->objClientData2;
    infoPtr->proc = cmdPtr->proc;
    infoPtr->clientData = cmdPtr->clientData;
#ifndef TCL_NO_DEPRECATED
    if (cmdPtr->deleteProc == CmdWrapperDeleteProc) {
	CmdWrapperInfo *info = (CmdWrapperInfo *)cmdPtr->deleteData;
	infoPtr->deleteProc = info->deleteProc;
	infoPtr->deleteData = info->deleteData;
	infoPtr->objProc = info->proc;
	infoPtr->objClientData = info->clientData;
	if (cmdPtr->objProc2 == CmdWrapperProc) {
	    infoPtr->isNativeObjectProc = 1;
	}
    } else
#endif
    {
	infoPtr->deleteProc = cmdPtr->deleteProc;
	infoPtr->deleteData = cmdPtr->deleteData;
#ifndef TCL_NO_DEPRECATED
	infoPtr->objProc = CmdWrapper2Proc;
	infoPtr->objClientData = cmdPtr;
#endif
    }
    infoPtr->namespacePtr = (Tcl_Namespace *) cmdPtr->nsPtr;
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandName --
 *
 *	Given a token returned by Tcl_CreateObjCommand, this function returns the
 *	current name of the command (which may have changed due to renaming).
 *
 * Results:
 *	The return value is the name of the given command.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_GetCommandName(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Command command)	/* Token for command returned by a previous
				 * call to Tcl_CreateObjCommand. The command must
				 * not have been deleted. */
{
    Command *cmdPtr = (Command *) command;

    if ((cmdPtr == NULL) || (cmdPtr->hPtr == NULL)) {
	/*
	 * This should only happen if command was "created" after the

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_GetCommandInfoFromToken(`, lines 1297–1407. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
    Tcl_CmdInfo info2;
    Tcl_Command buildInfoCmd = Tcl_CreateObjCommand(interp, "::tcl::build-info",
	    BuildInfoObjCmd, (void *)version, NULL);
    Tcl_GetCommandInfoFromToken(buildInfoCmd, &info2);
    info2.objProc2 = BuildInfoObjCmd2;
    info2.objClientData2 = (void *)version;
    Tcl_SetCommandInfoFromToken(buildInfoCmd, &info2);
#endif

    if (TclTommath_Init(interp) != TCL_OK) {
	Tcl_Panic("%s", Tcl_GetStringResult(interp));
    }

    if (TclOOInit(interp) != TCL_OK) {
	Tcl_Panic("%s", Tcl_GetStringResult(interp));
    }

    if (TclZlibInit(interp) != TCL_OK || TclZipfsInitInterp(interp) != TCL_OK) {
	Tcl_Panic("%s", Tcl_GetStringResult(interp));
    }

#if defined(_WIN32) || defined(__CYGWIN__)
    /* Ignore failures, let [package require registry] fail instead */
    (void) Registry_Init(interp);
#endif

    TOP_CB(iPtr) = NULL;
    return interp;
}

static void
DeleteOpCmdClientData(
    void *clientData)
{
    TclOpCmdClientData *occdPtr = (TclOpCmdClientData *)clientData;

    Tcl_Free(occdPtr);
}

/*
 * ---------------------------------------------------------------------
 *
 * TclRegisterCommandTypeName, TclGetCommandTypeName --
 *
 *	Command type registration and lookup mechanism. Everything is keyed by
 *	the Tcl_ObjCmdProc for the command, and that is used as the *key* into
 *	the hash table that maps to constant strings that are names. (It is
 *	recommended that those names be ASCII.)
 *
 * ---------------------------------------------------------------------
 */

void
TclRegisterCommandTypeName(
    Tcl_ObjCmdProc2 *implementationProc,
    const char *nameStr)
{
    Tcl_HashEntry *hPtr;

    Tcl_MutexLock(&commandTypeLock);
    if (commandTypeInit == 0) {
	Tcl_InitHashTable(&commandTypeTable, TCL_ONE_WORD_KEYS);
	commandTypeInit = 1;
    }
    if (nameStr != NULL) {
	int isNew;

	hPtr = Tcl_CreateHashEntry(&commandTypeTable,
		(void *)implementationProc, &isNew);
	Tcl_SetHashValue(hPtr, nameStr);
    } else {
	hPtr = Tcl_FindHashEntry(&commandTypeTable,
		implementationProc);
	if (hPtr != NULL) {
	    Tcl_DeleteHashEntry(hPtr);
	}
    }
    Tcl_MutexUnlock(&commandTypeLock);
}

const char *
TclGetCommandTypeName(
    Tcl_Command command)
{
    Command *cmdPtr = (Command *) command;
    Tcl_ObjCmdProc2 *procPtr = cmdPtr->objProc2;
    const char *name = "native";

    if (procPtr == NULL) {
	procPtr = cmdPtr->nreProc2;
    }
    Tcl_MutexLock(&commandTypeLock);
    if (commandTypeInit) {
	Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&commandTypeTable, procPtr);

	if (hPtr && Tcl_GetHashValue(hPtr)) {
	    name = (const char *) Tcl_GetHashValue(hPtr);
	}
    }
    Tcl_MutexUnlock(&commandTypeLock);

    return name;
}

/*
 *----------------------------------------------------------------------
 *
 * TclHideUnsafeCommands --
 *
 *	Hides base commands that are not marked as safe from this interpreter.
 *

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_GetCommandInfoFromToken(`, lines 3477–3587. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
    Tcl_Command cmd;

    cmd = Tcl_FindCommand(interp, cmdName, NULL, /*flags*/ 0);
    return Tcl_GetCommandInfoFromToken(cmd, infoPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandInfoFromToken --
 *
 *	Returns various information about a Tcl command.
 *
 * Results:
 *	Copies information from the command identified by 'cmd' into a
 *	caller-supplied structure and returns 1. If the 'cmd' is NULL, leaves
 *	the structure untouched and returns 0.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

int
Tcl_GetCommandInfoFromToken(
    Tcl_Command cmd,
    Tcl_CmdInfo *infoPtr)
{
    Command *cmdPtr;		/* Internal representation of the command */

    if (cmd == NULL) {
	return 0;
    }

    /*
     * Set isNativeObjectProc 1 if objProc was registered by a call to
     * Tcl_CreateObjCommand. Set isNativeObjectProc 2 if objProc was
     * registered by a call to Tcl_CreateObjCommand2. Otherwise set it to 0.
     */

    cmdPtr = (Command *) cmd;
    infoPtr->isNativeObjectProc =
	    (cmdPtr->objProc2 != InvokeStringCommand) ? 2 : 0;
    infoPtr->objProc2 = cmdPtr->objProc2;
    infoPtr->objClientData2 = cmdPtr->objClientData2;
    infoPtr->proc = cmdPtr->proc;
    infoPtr->clientData = cmdPtr->clientData;
#ifndef TCL_NO_DEPRECATED
    if (cmdPtr->deleteProc == CmdWrapperDeleteProc) {
	CmdWrapperInfo *info = (CmdWrapperInfo *)cmdPtr->deleteData;
	infoPtr->deleteProc = info->deleteProc;
	infoPtr->deleteData = info->deleteData;
	infoPtr->objProc = info->proc;
	infoPtr->objClientData = info->clientData;
	if (cmdPtr->objProc2 == CmdWrapperProc) {
	    infoPtr->isNativeObjectProc = 1;
	}
    } else
#endif
    {
	infoPtr->deleteProc = cmdPtr->deleteProc;
	infoPtr->deleteData = cmdPtr->deleteData;
#ifndef TCL_NO_DEPRECATED
	infoPtr->objProc = CmdWrapper2Proc;
	infoPtr->objClientData = cmdPtr;
#endif
    }
    infoPtr->namespacePtr = (Tcl_Namespace *) cmdPtr->nsPtr;
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandName --
 *
 *	Given a token returned by Tcl_CreateObjCommand, this function returns the
 *	current name of the command (which may have changed due to renaming).
 *
 * Results:
 *	The return value is the name of the given command.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_GetCommandName(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Command command)	/* Token for command returned by a previous
				 * call to Tcl_CreateObjCommand. The command must
				 * not have been deleted. */
{
    Command *cmdPtr = (Command *) command;

    if ((cmdPtr == NULL) || (cmdPtr->hPtr == NULL)) {
	/*
	 * This should only happen if command was "created" after the
	 * interpreter began to be deleted, so there isn't really any command.
	 * Just return an empty string.
	 */

	return "";
    }

    return (const char *)Tcl_GetHashKey(cmdPtr->hPtr->tablePtr, cmdPtr->hPtr);
}


```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_GetCommandInfoFromToken(`, lines 3499–3609. Retained reference `rust-tcl-registry-tests-data-native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
 */

int
Tcl_GetCommandInfoFromToken(
    Tcl_Command cmd,
    Tcl_CmdInfo *infoPtr)
{
    Command *cmdPtr;		/* Internal representation of the command */

    if (cmd == NULL) {
	return 0;
    }

    /*
     * Set isNativeObjectProc 1 if objProc was registered by a call to
     * Tcl_CreateObjCommand. Set isNativeObjectProc 2 if objProc was
     * registered by a call to Tcl_CreateObjCommand2. Otherwise set it to 0.
     */

    cmdPtr = (Command *) cmd;
    infoPtr->isNativeObjectProc =
	    (cmdPtr->objProc2 != InvokeStringCommand) ? 2 : 0;
    infoPtr->objProc2 = cmdPtr->objProc2;
    infoPtr->objClientData2 = cmdPtr->objClientData2;
    infoPtr->proc = cmdPtr->proc;
    infoPtr->clientData = cmdPtr->clientData;
#ifndef TCL_NO_DEPRECATED
    if (cmdPtr->deleteProc == CmdWrapperDeleteProc) {
	CmdWrapperInfo *info = (CmdWrapperInfo *)cmdPtr->deleteData;
	infoPtr->deleteProc = info->deleteProc;
	infoPtr->deleteData = info->deleteData;
	infoPtr->objProc = info->proc;
	infoPtr->objClientData = info->clientData;
	if (cmdPtr->objProc2 == CmdWrapperProc) {
	    infoPtr->isNativeObjectProc = 1;
	}
    } else
#endif
    {
	infoPtr->deleteProc = cmdPtr->deleteProc;
	infoPtr->deleteData = cmdPtr->deleteData;
#ifndef TCL_NO_DEPRECATED
	infoPtr->objProc = CmdWrapper2Proc;
	infoPtr->objClientData = cmdPtr;
#endif
    }
    infoPtr->namespacePtr = (Tcl_Namespace *) cmdPtr->nsPtr;
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandName --
 *
 *	Given a token returned by Tcl_CreateObjCommand, this function returns the
 *	current name of the command (which may have changed due to renaming).
 *
 * Results:
 *	The return value is the name of the given command.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

const char *
Tcl_GetCommandName(
    TCL_UNUSED(Tcl_Interp *),
    Tcl_Command command)	/* Token for command returned by a previous
				 * call to Tcl_CreateObjCommand. The command must
				 * not have been deleted. */
{
    Command *cmdPtr = (Command *) command;

    if ((cmdPtr == NULL) || (cmdPtr->hPtr == NULL)) {
	/*
	 * This should only happen if command was "created" after the
	 * interpreter began to be deleted, so there isn't really any command.
	 * Just return an empty string.
	 */

	return "";
    }

    return (const char *)Tcl_GetHashKey(cmdPtr->hPtr->tablePtr, cmdPtr->hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFullName --
 *
 *	Given a token returned by, e.g., Tcl_CreateObjCommand or Tcl_FindCommand,
 *	this function appends to an object the command's full name, qualified
 *	by a sequence of parent namespace names. The command's fully-qualified
 *	name may have changed due to renaming.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	The command's fully-qualified name is appended to the string
 *	representation of objPtr.
 *
 *----------------------------------------------------------------------
 */

void
Tcl_GetCommandFullName(

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclObj.c`, function `AllocObjEntry(`, lines 206–318. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-9.1.0-tclObj.c`.

```text
 * Prototypes for the array hash key methods.
 */

static Tcl_HashEntry *	AllocObjEntry(Tcl_HashTable *tablePtr, void *keyPtr);

/*
 * Prototypes for the CommandName object type.
 */

static void		DupCmdNameInternalRep(Tcl_Obj *objPtr,
			    Tcl_Obj *copyPtr);
static void		FreeCmdNameInternalRep(Tcl_Obj *objPtr);
static int		SetCmdNameFromAny(Tcl_Interp *interp, Tcl_Obj *objPtr);

/*
 * The structures below defines the Tcl object types defined in this file by
 * means of functions that can be invoked by generic object code. See also
 * tclStringObj.c, tclListObj.c, tclByteCode.c for other type manager
 * implementations.
 */

const Tcl_ObjType tclBooleanType= {
    "boolean",
    NULL,			// FreeIntRep
    NULL,			// DupIntRep
    NULL,			// UpdateString
    TclSetBooleanFromAny,
    TCL_OBJTYPE_V1(TclLengthOne)
};
const Tcl_ObjType tclDoubleType= {
    "double",
    NULL,			// FreeIntRep
    NULL,			// DupIntRep
    UpdateStringOfDouble,
    SetDoubleFromAny,
    TCL_OBJTYPE_V1(TclLengthOne)
};
const Tcl_ObjType tclIntType = {
    "int",
    NULL,			// FreeIntRep
    NULL,			// DupIntRep
    UpdateStringOfInt,
    SetIntFromAny,
    TCL_OBJTYPE_V1(TclLengthOne)
};
const Tcl_ObjType tclBignumType = {
    "bignum",
    FreeBignum,
    DupBignum,
    UpdateStringOfBignum,
    NULL,			// SetFromAny
    TCL_OBJTYPE_V1(TclLengthOne)
};

/*
 * The structure below defines the Tcl obj hash key type.
 */

const Tcl_HashKeyType tclObjHashKeyType = {
    TCL_HASH_KEY_TYPE_VERSION,	/* version */
    TCL_HASH_KEY_DIRECT_COMPARE,/* allows compare keys by pointers */
    TclHashObjKey,		/* hashKeyProc */
    TclCompareObjKeys,		/* compareKeysProc */
    AllocObjEntry,		/* allocEntryProc */
    TclFreeObjEntry		/* freeEntryProc */
};

/*
 * The structure below defines the command name Tcl object type by means of
 * functions that can be invoked by generic object code. Objects of this type
 * cache the Command pointer that results from looking up command names in the
 * command hashtable. Such objects appear as the zeroth ("command name")
 * argument in a Tcl command.
 *
 * NOTE: the ResolvedCmdName that gets cached is stored in the
 * twoPtrValue.ptr1 field, and the twoPtrValue.ptr2 field is unused. You might
 * think you could use the simpler otherValuePtr field to store the single
 * ResolvedCmdName pointer, but DO NOT DO THIS. It seems that some extensions
 * use the second internal pointer field of the twoPtrValue field for their
 * own purposes.
 *
 * TRICKY POINT! Some extensions update this structure! (Notably, these
 * include TclBlend and TCom). This is highly ill-advised on their part, but
 * does allow them to delete a command when references to it are gone, which
 * is fragile but useful given their somewhat-OO style. Because of this, this
 * structure MUST NOT be const so that the C compiler puts the data in
 * writable memory. [Bug 2558422] [Bug 07d13d99b0a9]
 * TODO: Provide a better API for those extensions so that they can coexist...
 */

Tcl_ObjType tclCmdNameType = {
    "cmdName",
    FreeCmdNameInternalRep,
    DupCmdNameInternalRep,
    NULL,			// UpdateString
    SetCmdNameFromAny,
    TCL_OBJTYPE_V0
};

/*
 * Structure containing a cached pointer to a command that is the result of
 * resolving the command's name in some namespace. It is the internal
 * representation for a cmdName object. It contains the pointer along with
 * some information that is used to check the pointer's validity.
 */

typedef struct ResolvedCmdName {
    Command *cmdPtr;		/* A cached Command pointer. */
    Namespace *refNsPtr;	/* Points to the namespace containing the
				 * reference (not the namespace that contains
				 * the referenced command). NULL if the name
				 * is fully qualified.*/
    size_t refNsId;		/* refNsPtr's unique namespace id. Used to

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclObj.c`, function `AllocObjEntry(`, lines 4062–4171. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-9.1.0-tclObj.c`.

```text
 */

static Tcl_HashEntry *
AllocObjEntry(
    TCL_UNUSED(Tcl_HashTable *),
    void *keyPtr)		/* Key to store in the hash table entry. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    Tcl_HashEntry *hPtr = (Tcl_HashEntry *)Tcl_AttemptAlloc(sizeof(Tcl_HashEntry));

    if (hPtr) {
	hPtr->key.objPtr = objPtr;
	Tcl_IncrRefCount(objPtr);
	hPtr->clientData = NULL;
    }

    return hPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * TclCompareObjKeys --
 *
 *	Compares two Tcl_Obj * keys.
 *
 * Results:
 *	The return value is 0 if they are different and 1 if they are the
 *	same.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

int
TclCompareObjKeys(
    void *keyPtr,		/* New key to compare. */
    Tcl_HashEntry *hPtr)	/* Existing key to compare. */
{
    Tcl_Obj *objPtr1 = (Tcl_Obj *)keyPtr;
    Tcl_Obj *objPtr2 = (Tcl_Obj *)hPtr->key.oneWordValue;
    const char *p1, *p2;
    size_t l1, l2;

    /*
     * If the object pointers are the same then they match.
     * OPT: this comparison was moved to the caller

	if (objPtr1 == objPtr2) {
	    return 1;
	}
     */

    /*
     * Don't use Tcl_GetStringFromObj as it would prevent l1 and l2 being
     * in a register.
     */

    p1 = TclGetString(objPtr1);
    l1 = objPtr1->length;
    p2 = TclGetString(objPtr2);
    l2 = objPtr2->length;

    /*
     * Only compare if the string representations are of the same length.
     */

    if (l1 == l2) {
	for (;; p1++, p2++, l1--) {
	    if (*p1 != *p2) {
		break;
	    }
	    if (l1 == 0) {
		return 1;
	    }
	}
    }

    return 0;
}

/*
 *----------------------------------------------------------------------
 *
 * TclFreeObjEntry --
 *
 *	Frees space for a Tcl_HashEntry containing the Tcl_Obj * key.
 *
 * Results:
 *	The return value is a pointer to the created entry.
 *
 * Side effects:
 *	Decrements the reference count of the object.
 *
 *----------------------------------------------------------------------
 */

void
TclFreeObjEntry(
    Tcl_HashEntry *hPtr)	/* Hash entry to free. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *) hPtr->key.oneWordValue;

    Tcl_DecrRefCount(objPtr);
    Tcl_Free(hPtr);
}

/*

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclObj.c`, function `HashObjKey(`, lines 4187–4298. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-9.1.0-tclObj.c`.

```text
 */

size_t
TclHashObjKey(
    TCL_UNUSED(Tcl_HashTable *),
    void *keyPtr)		/* Key from which to compute hash value. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    Tcl_Size length;
    const char *string = Tcl_GetStringFromObj(objPtr, &length);
    size_t result = 0;

    /*
     * I tried a zillion different hash functions and asked many other people
     * for advice. Many people had their own favorite functions, all
     * different, but no-one had much idea why they were good ones. I chose
     * the one below (multiply by 9 and add new character) because of the
     * following reasons:
     *
     * 1. Multiplying by 10 is perfect for keys that are decimal strings, and
     *    multiplying by 9 is just about as good.
     * 2. Times-9 is (shift-left-3) plus (old). This means that each
     *    character's bits hang around in the low-order bits of the hash value
     *    for ever, plus they spread fairly rapidly up to the high-order bits
     *    to fill out the hash value. This seems works well both for decimal
     *    and non-decimal strings.
     *
     * Note that this function is very weak against malicious strings; it's
     * very easy to generate multiple keys that have the same hashcode. On the
     * other hand, that hardly ever actually occurs and this function *is*
     * very cheap, even by comparison with industry-standard hashes like FNV.
     * If real strength of hash is required though, use a custom hash based on
     * Bob Jenkins's lookup3(), but be aware that it's significantly slower.
     * Tcl does not use that level of strength because it typically does not
     * need it (and some of the aspects of that strength are genuinely
     * unnecessary given the rest of Tcl's hash machinery, and the fact that
     * we do not either transfer hashes to another machine, use them as a true
     * substitute for equality, or attempt to minimize work in rebuilding the
     * hash table).
     *
     * See also HashStringKey in tclHash.c.
     * See also HashString in tclLiteral.c.
     *
     * See [tcl-Feature Request #2958832]
     */

    if (length > 0) {
	result = UCHAR(*string);
	while (--length) {
	    result += (result << 3) + UCHAR(*++string);
	}
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFromObj --
 *
 *	Returns the command specified by the name in a Tcl_Obj.
 *
 * Results:
 *	Returns a token for the command if it is found. Otherwise, if it can't
 *	be found or there is an error, returns NULL.
 *
 * Side effects:
 *	May update the internal representation for the object, caching the
 *	command reference so that the next time this function is called with
 *	the same object, the command can be found quickly.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
Tcl_GetCommandFromObj(
    Tcl_Interp *interp,		/* The interpreter in which to resolve the
				 * command and to report errors. */
    Tcl_Obj *objPtr)		/* The object containing the command's name.
				 * If the name starts with "::", will be
				 * looked up in global namespace. Else, looked
				 * up first in the current namespace, then in
				 * global namespace. */
{
    ResolvedCmdName *resPtr;

    /*
     * Get the internal representation, converting to a command type if
     * needed. The internal representation is a ResolvedCmdName that points to
     * the actual command.
     *
     * Check the context namespace and the namespace epoch of the resolved
     * symbol to make sure that it is fresh. Note that we verify that the
     * namespace id of the context namespace is the same as the one we cached;
     * this insures that the namespace wasn't deleted and a new one created at
     * the same address with the same command epoch. Note that fully qualified
     * names have a NULL refNsPtr, these checks needn't be made.
     *
     * Check also that the command's epoch is up to date, and that the command
     * is not deleted.
     *
     * If any check fails, then force another conversion to the command type,
     * to discard the old rep and create a new one.
     */

    resPtr = (ResolvedCmdName *)objPtr->internalRep.twoPtrValue.ptr1;
    if (TclHasInternalRep(objPtr, &tclCmdNameType)) {
	Command *cmdPtr = resPtr->cmdPtr;

	if ((cmdPtr->cmdEpoch == resPtr->cmdEpoch)
		&& (interp == cmdPtr->nsPtr->interp)
		&& !(cmdPtr->nsPtr->flags & NS_DYING)) {

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclObj.c`, function `CompareObjKeys(`, lines 4096–4206. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-9.1.0-tclObj.c`.

```text
 */

int
TclCompareObjKeys(
    void *keyPtr,		/* New key to compare. */
    Tcl_HashEntry *hPtr)	/* Existing key to compare. */
{
    Tcl_Obj *objPtr1 = (Tcl_Obj *)keyPtr;
    Tcl_Obj *objPtr2 = (Tcl_Obj *)hPtr->key.oneWordValue;
    const char *p1, *p2;
    size_t l1, l2;

    /*
     * If the object pointers are the same then they match.
     * OPT: this comparison was moved to the caller

	if (objPtr1 == objPtr2) {
	    return 1;
	}
     */

    /*
     * Don't use Tcl_GetStringFromObj as it would prevent l1 and l2 being
     * in a register.
     */

    p1 = TclGetString(objPtr1);
    l1 = objPtr1->length;
    p2 = TclGetString(objPtr2);
    l2 = objPtr2->length;

    /*
     * Only compare if the string representations are of the same length.
     */

    if (l1 == l2) {
	for (;; p1++, p2++, l1--) {
	    if (*p1 != *p2) {
		break;
	    }
	    if (l1 == 0) {
		return 1;
	    }
	}
    }

    return 0;
}

/*
 *----------------------------------------------------------------------
 *
 * TclFreeObjEntry --
 *
 *	Frees space for a Tcl_HashEntry containing the Tcl_Obj * key.
 *
 * Results:
 *	The return value is a pointer to the created entry.
 *
 * Side effects:
 *	Decrements the reference count of the object.
 *
 *----------------------------------------------------------------------
 */

void
TclFreeObjEntry(
    Tcl_HashEntry *hPtr)	/* Hash entry to free. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *) hPtr->key.oneWordValue;

    Tcl_DecrRefCount(objPtr);
    Tcl_Free(hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclHashObjKey --
 *
 *	Compute a one-word summary of the string representation of the
 *	Tcl_Obj, which can be used to generate a hash index.
 *
 * Results:
 *	The return value is a one-word summary of the information in the
 *	string representation of the Tcl_Obj.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

size_t
TclHashObjKey(
    TCL_UNUSED(Tcl_HashTable *),
    void *keyPtr)		/* Key from which to compute hash value. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    Tcl_Size length;
    const char *string = Tcl_GetStringFromObj(objPtr, &length);
    size_t result = 0;

    /*
     * I tried a zillion different hash functions and asked many other people
     * for advice. Many people had their own favorite functions, all
     * different, but no-one had much idea why they were good ones. I chose
     * the one below (multiply by 9 and add new character) because of the
     * following reasons:
     *
     * 1. Multiplying by 10 is perfect for keys that are decimal strings, and

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclObj.c`, function `FreeObjEntry(`, lines 4159–4269. Retained reference `rust-tcl-registry-tests-data-native_ensemble_map_prefix312-map-source-9.1.0-tclObj.c`.

```text
 */

void
TclFreeObjEntry(
    Tcl_HashEntry *hPtr)	/* Hash entry to free. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *) hPtr->key.oneWordValue;

    Tcl_DecrRefCount(objPtr);
    Tcl_Free(hPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclHashObjKey --
 *
 *	Compute a one-word summary of the string representation of the
 *	Tcl_Obj, which can be used to generate a hash index.
 *
 * Results:
 *	The return value is a one-word summary of the information in the
 *	string representation of the Tcl_Obj.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

size_t
TclHashObjKey(
    TCL_UNUSED(Tcl_HashTable *),
    void *keyPtr)		/* Key from which to compute hash value. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    Tcl_Size length;
    const char *string = Tcl_GetStringFromObj(objPtr, &length);
    size_t result = 0;

    /*
     * I tried a zillion different hash functions and asked many other people
     * for advice. Many people had their own favorite functions, all
     * different, but no-one had much idea why they were good ones. I chose
     * the one below (multiply by 9 and add new character) because of the
     * following reasons:
     *
     * 1. Multiplying by 10 is perfect for keys that are decimal strings, and
     *    multiplying by 9 is just about as good.
     * 2. Times-9 is (shift-left-3) plus (old). This means that each
     *    character's bits hang around in the low-order bits of the hash value
     *    for ever, plus they spread fairly rapidly up to the high-order bits
     *    to fill out the hash value. This seems works well both for decimal
     *    and non-decimal strings.
     *
     * Note that this function is very weak against malicious strings; it's
     * very easy to generate multiple keys that have the same hashcode. On the
     * other hand, that hardly ever actually occurs and this function *is*
     * very cheap, even by comparison with industry-standard hashes like FNV.
     * If real strength of hash is required though, use a custom hash based on
     * Bob Jenkins's lookup3(), but be aware that it's significantly slower.
     * Tcl does not use that level of strength because it typically does not
     * need it (and some of the aspects of that strength are genuinely
     * unnecessary given the rest of Tcl's hash machinery, and the fact that
     * we do not either transfer hashes to another machine, use them as a true
     * substitute for equality, or attempt to minimize work in rebuilding the
     * hash table).
     *
     * See also HashStringKey in tclHash.c.
     * See also HashString in tclLiteral.c.
     *
     * See [tcl-Feature Request #2958832]
     */

    if (length > 0) {
	result = UCHAR(*string);
	while (--length) {
	    result += (result << 3) + UCHAR(*++string);
	}
    }
    return result;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_GetCommandFromObj --
 *
 *	Returns the command specified by the name in a Tcl_Obj.
 *
 * Results:
 *	Returns a token for the command if it is found. Otherwise, if it can't
 *	be found or there is an error, returns NULL.
 *
 * Side effects:
 *	May update the internal representation for the object, caching the
 *	command reference so that the next time this function is called with
 *	the same object, the command can be found quickly.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
Tcl_GetCommandFromObj(
    Tcl_Interp *interp,		/* The interpreter in which to resolve the
				 * command and to report errors. */
    Tcl_Obj *objPtr)		/* The object containing the command's name.
				 * If the name starts with "::", will be
				 * looked up in global namespace. Else, looked
				 * up first in the current namespace, then in
				 * global namespace. */

```

## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `native_tcloo_method_definition::tests::original_method_rename_verdict_keeps_entry_identity_and_diagnostic_extent_separate` (linked): Finite source-defined control: original method rename verdict keeps entry identity and diagnostic extent separate. Pure verdict over supplied actual entry facts; no table or invocation admission. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_oo/native_method_rename_tests.rs](../../../../runtime/rust/src/cmd_oo/native_method_rename_tests.rs), `cmd_oo::native_method_rename_tests::original_named_declarations_keep_record_name_and_table_key_roles_separate` (linked): Finite source-defined control: original named declarations keep record name and table key roles separate. Genuine authored/generated declaration roles; shallow Rust observations are not Native copies. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_oo/native_method_rename_tests.rs](../../../../runtime/rust/src/cmd_oo/native_method_rename_tests.rs), `cmd_oo::native_method_rename_tests::original_rename_verdict_and_counted_headers_match_reached_stock_worker_stages` (linked): Finite source-defined control: original rename verdict and counted headers match reached stock worker stages. Match stock worker name/verdict/role phases only; the Runtime forward payload is independent. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/cmd_oo/native_method_rename_tests.rs](../../../../runtime/rust/src/cmd_oo/native_method_rename_tests.rs), `cmd_oo::native_method_rename_tests::original_self_method_report_keeps_current_entered_name_and_unnamed_roles_distinct` (linked): Finite source-defined control: original self method report keeps current entered name and unnamed roles distinct. Source-selected SELF_METHOD name role; no measured custom MethodType payload/body equivalence. Unnamed slots retain no named-record role. Foundation-owned report headers are a separate consumer purpose, not supplied by this owner. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-vm/src/cmd_oo/native_method_cache/rename_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_cache/rename_tests.rs), `cmd_oo::native_method_cache::rename_tests::original_vm_rename_missing_source_does_not_reach_destination_string` (linked): Original vm rename missing source does not reach destination string. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.
- [rust/tcl-vm/src/cmd_oo/native_method_cache/rename_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_cache/rename_tests.rs), `cmd_oo::native_method_cache::rename_tests::original_vm_rename_moves_same_record_and_observes_reached_payload_changes` (linked): Original vm rename moves same record and observes reached payload changes. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.

## Evidence

- [runtime/rust/tests/data/native_method_rename_operands_records_original/510](../../../../runtime/rust/tests/data/native_method_rename_operands_records_original/510)
- [runtime/rust/tests/data](../../../../runtime/rust/tests/data)
- [rust/tcl-registry/tests/data/native_capi_scalar_publication_original](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original)
- [rust/tcl-registry/tests/data/native_oo_operand_stages_original](../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original)
- [rust/tcl-registry/tests/data/native_command_recreation_original](../../../../rust/tcl-registry/tests/data/native_command_recreation_original)
- [runtime/rust/tests/data/native_method_cache_active_call_lifetime_original](../../../../runtime/rust/tests/data/native_method_cache_active_call_lifetime_original)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20)
- [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0)
- [rust/tcl-registry/tests/data/native_deep_import_chain_original](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original)
- [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim)
- [rust/tcl-registry/tests/data/native_child_alias_inventory298](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-test-support/tests/data/native_resolution_reference_builds](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds)
- [runtime/rust/tests/data/native_method_rename_operands_records_original/510b](../../../../runtime/rust/tests/data/native_method_rename_operands_records_original/510b)
- [rust/tcl-vm/src/cmd_oo/native_method_cache](../../../../rust/tcl-vm/src/cmd_oo/native_method_cache)
