# naming.tcloo.declared-variable-compiled-and-runtime-resolver

Kind: `native-observation`

## Problem statement

A counted TclOO variable declaration can retain bytes after raw zero while a method runtime name lookup reaches a CString resolver. Eagerly linking every declared full name makes dynamic access appear equivalent to a genuine compiler-selected local primary and can redirect writes into object storage. A successful declaration or a clipped variable presenter also cannot prove a compiled slot. The controls distinguish compiled and dynamic accesses to one declaring class and include a prefix-only negative and qualification after zero; they do not establish other object-private or compiler/header capabilities.

## Question

Do original counted declared member names resolve identically through literal compiled method accesses and dynamic set $target accesses, including raw zero, qualification after zero, surrogate and raw ff names?

## Conclusion

C8.6/C9.0/C9.1 literal compiled accesses to a matching raw-zero declaration read/write object STATIC, while dynamic full-name writes create ordinary locals, leave STATIC unchanged and fresh dynamic reads fail. Plain, surrogate and raw ff declarations resolve through both routes. A prefix-only declaration does not link the full raw-zero compiled name: its static reads fail, while a dynamic read after the dynamic write returns DYNAMIC. Qualification after zero is accepted by declaration, but the retained static and dynamic read controls fail and report no linked object variable. C8.6 info object vars clips the raw-zero presenter, whereas C9 retains it; that output alone does not identify the stored key. C8.4/C8.5/Jim lack OO in the observed command surface. No cache, original header, native local table or Rust execution authority is supplied.

The pure selected resolver distinguishes counted compiled-primary byte matching from runtime C-string boundaries and refuses unsupported recipes; source policy tests are independent of actual OO lookup/frame execution.

## Scope

Exact public original Tcl_NewStringObj(pointer,count)/Tcl_EvalObjv declaration, formal-list, method-body and dynamic name operands. ASCII setup via Tcl_Eval; compiled literal method bodies contain actual raw bytes, not Unicode/source escapes. Six names/negative arrangements and static/dynamic/read/object-vars chronology, fresh provider interpreters. Jim only executes startup/OO-availability source. Source windows explain full counted compiled matching versus runtime strlen; a caller still needs an independent actual compiler primary/context/lifetime owner.

Current direct test attributes and real module parents attach these source/API contracts only. They do not certify an executable inventory, passed assertion, complete Native comparison or changed-current-image result. Each original provider answer and independently closed software command retains its exact input/channel/source purpose.

## Provider answers

### tcl8.4, tcl8.5

Status: `unsupported`. Channel: Original counted Tcl objects passed to Tcl_EvalObjv; body source contains actual raw native bytes, ASCII setup via NUL-terminated Tcl_Eval. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Actual OO command query is empty; no TclOO member vector executes.

### tcl8.6

Status: `observed`. Channel: Original counted Tcl objects passed to Tcl_EvalObjv; body source contains actual raw native bytes, ASCII setup via NUL-terminated Tcl_Eval. Version: tcl8.6 8.6.18.

Matching raw-zero static object access stays STATIC after dynamic write; fresh dynamicread fails. Plain/FF/surrogate static reader changes to DYNAMIC after dynamic write. Prefix-only declaration and qualification-after-zero source controls do not link the compiled full name. The raw-zero object-vars presenter clips at zero.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted Tcl objects passed to Tcl_EvalObjv; body source contains actual raw native bytes, ASCII setup via NUL-terminated Tcl_Eval. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Matching raw-zero static object access stays STATIC after dynamic write; fresh dynamicread fails. Plain/FF/surrogate static reader changes to DYNAMIC after dynamic write. Prefix-only declaration and qualification-after-zero source controls do not link the compiled full name. The raw-zero object-vars presenter retains all counted bytes.

### jim

Status: `unsupported`. Channel: ASCII NUL-terminated Jim_Eval startup and OO-availability query only. Version: jim 0.84-9-g5bac7c9.

Actual OO command query is empty; no TclOO member vector executes.

### bigip

Status: `not-tested`.

No appliance observation for this exact counted TclOO resolver question.

## Source inspection

tcl8.6 8.6.18, revision `Pinned Tcl 8.6.18 release source`, `generic/tclOOMethod.c`, function `ProcedureMethodVarResolver / ProcedureMethodCompiledVarConnect / ProcedureMethodCompiledVarResolver`, lines 914–1075. Retained reference `tcl8.6-resolver-source`.

```text
ProcedureMethodVarResolver(
    Tcl_Interp *interp,
    const char *varName,
    Tcl_Namespace *contextNs,
    int flags,
    Tcl_Var *varPtr)
{
    int result;
    Tcl_ResolvedVarInfo *rPtr = NULL;

    result = ProcedureMethodCompiledVarResolver(interp, varName,
	    strlen(varName), contextNs, &rPtr);

    if (result != TCL_OK) {
	return result;
    }

    *varPtr = rPtr->fetchProc(interp, rPtr);

    /*
     * Must not retain reference to resolved information. [Bug 3105999]
     */

    rPtr->deleteProc(rPtr);
    return (*varPtr ? TCL_OK : TCL_CONTINUE);
}

static Tcl_Var
ProcedureMethodCompiledVarConnect(
    Tcl_Interp *interp,
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;
    Interp *iPtr = (Interp *) interp;
    CallFrame *framePtr = iPtr->varFramePtr;
    CallContext *contextPtr;
    Tcl_Obj *variableObj;
    Tcl_HashEntry *hPtr;
    int i, isNew, cacheIt, varLen, len;
    const char *match, *varName;

    /*
     * Check that the variable is being requested in a context that is also a
     * method call; if not (i.e. we're evaluating in the object's namespace or
     * in a procedure of that namespace) then we do nothing.
     */

    if (framePtr == NULL || !(framePtr->isProcCallFrame & FRAME_IS_METHOD)) {
	return NULL;
    }
    contextPtr = (CallContext *)framePtr->clientData;

    /*
     * If we've done the work before (in a comparable context) then reuse that
     * rather than performing resolution ourselves.
     */

    if (infoPtr->cachedObjectVar) {
	return infoPtr->cachedObjectVar;
    }

    /*
     * Check if the variable is one we want to resolve at all (i.e. whether it
     * is in the list provided by the user). If not, we mustn't do anything
     * either.
     */

    varName = TclGetStringFromObj(infoPtr->variableObj, &varLen);
    if (contextPtr->callPtr->chain[contextPtr->index]
	    .mPtr->declaringClassPtr != NULL) {
	FOREACH(variableObj, contextPtr->callPtr->chain[contextPtr->index]
		.mPtr->declaringClassPtr->variables) {
	    match = TclGetStringFromObj(variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		cacheIt = 0;
		goto gotMatch;
	    }
	}
    } else {
	FOREACH(variableObj, contextPtr->oPtr->variables) {
	    match = TclGetStringFromObj(variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		cacheIt = 1;
		goto gotMatch;
	    }
	}
    }
    return NULL;

    /*
     * It is a variable we want to resolve, so resolve it.
     */

  gotMatch:
    hPtr = Tcl_CreateHashEntry(TclVarTable(contextPtr->oPtr->namespacePtr),
	    (char *) variableObj, &isNew);
    if (isNew) {
	TclSetVarNamespaceVar((Var *) TclVarHashGetValue(hPtr));
    }
    if (cacheIt) {
	infoPtr->cachedObjectVar = TclVarHashGetValue(hPtr);

	/*
	 * We must keep a reference to the variable so everything will
	 * continue to work correctly even if it is unset; being unset does
	 * not end the life of the variable at this level. [Bug 3185009]
	 */

	VarHashRefCount(infoPtr->cachedObjectVar)++;
    }
    return TclVarHashGetValue(hPtr);
}

static void
ProcedureMethodCompiledVarDelete(
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;

    /*
     * Release the reference to the variable if we were holding it.
     */

    if (infoPtr->cachedObjectVar) {
	VarHashRefCount(infoPtr->cachedObjectVar)--;
	TclCleanupVar((Var *) infoPtr->cachedObjectVar, NULL);
    }
    Tcl_DecrRefCount(infoPtr->variableObj);
    ckfree(infoPtr);
}

static int
ProcedureMethodCompiledVarResolver(
    Tcl_Interp *interp,
    const char *varName,
    int length,
    Tcl_Namespace *contextNs,
    Tcl_ResolvedVarInfo **rPtrPtr)
{
    OOResVarInfo *infoPtr;
    Tcl_Obj *variableObj = Tcl_NewStringObj(varName, length);

    /*
     * Do not create resolvers for cases that contain namespace separators or
     * which look like array accesses. Both will lead us astray.
     */

    if (strstr(Tcl_GetString(variableObj), "::") != NULL ||
	    Tcl_StringMatch(Tcl_GetString(variableObj), "*(*)")) {
	Tcl_DecrRefCount(variableObj);
	return TCL_CONTINUE;
    }

    infoPtr = (OOResVarInfo *)ckalloc(sizeof(OOResVarInfo));
    infoPtr->info.fetchProc = ProcedureMethodCompiledVarConnect;
    infoPtr->info.deleteProc = ProcedureMethodCompiledVarDelete;
    infoPtr->cachedObjectVar = NULL;
    infoPtr->variableObj = variableObj;
    Tcl_IncrRefCount(variableObj);
    *rPtrPtr = &infoPtr->info;
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Pinned Tcl 9.0.4 release source`, `generic/tclOOMethod.c`, function `ProcedureMethodVarResolver / ProcedureMethodCompiledVarConnect / ProcedureMethodCompiledVarResolver`, lines 1067–1247. Retained reference `tcl9.0-resolver-source`.

```text
ProcedureMethodVarResolver(
    Tcl_Interp *interp,
    const char *varName,
    Tcl_Namespace *contextNs,
    TCL_UNUSED(int) /*flags*/,	// Ignoring variable access flags (???)
    Tcl_Var *varPtr)
{
    int result;
    Tcl_ResolvedVarInfo *rPtr = NULL;

    result = ProcedureMethodCompiledVarResolver(interp, varName,
	    strlen(varName), contextNs, &rPtr);

    if (result != TCL_OK) {
	return result;
    }

    *varPtr = rPtr->fetchProc(interp, rPtr);

    /*
     * Must not retain reference to resolved information. [Bug 3105999]
     */

    rPtr->deleteProc(rPtr);
    return (*varPtr ? TCL_OK : TCL_CONTINUE);
}

static Tcl_Var
ProcedureMethodCompiledVarConnect(
    Tcl_Interp *interp,
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;
    Interp *iPtr = (Interp *) interp;
    CallFrame *framePtr = iPtr->varFramePtr;
    CallContext *contextPtr;
    Tcl_Obj *variableObj;
    PrivateVariableMapping *privateVar;
    Tcl_HashEntry *hPtr;
    int isNew, cacheIt;
    Tcl_Size i, varLen, len;
    const char *match, *varName;

    /*
     * Check that the variable is being requested in a context that is also a
     * method call; if not (i.e. we're evaluating in the object's namespace or
     * in a procedure of that namespace) then we do nothing.
     */

    if (framePtr == NULL || !(framePtr->isProcCallFrame & FRAME_IS_METHOD)) {
	return NULL;
    }
    contextPtr = (CallContext *) framePtr->clientData;

    /*
     * If we've done the work before (in a comparable context) then reuse that
     * rather than performing resolution ourselves.
     */

    if (infoPtr->cachedObjectVar) {
	return infoPtr->cachedObjectVar;
    }

    /*
     * Check if the variable is one we want to resolve at all (i.e. whether it
     * is in the list provided by the user). If not, we mustn't do anything
     * either.
     */

    varName = Tcl_GetStringFromObj(infoPtr->variableObj, &varLen);
    if (contextPtr->callPtr->chain[contextPtr->index]
	    .mPtr->declaringClassPtr != NULL) {
	FOREACH_STRUCT(privateVar, contextPtr->callPtr->chain[contextPtr->index]
		.mPtr->declaringClassPtr->privateVariables) {
	    match = Tcl_GetStringFromObj(privateVar->variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		variableObj = privateVar->fullNameObj;
		cacheIt = 0;
		goto gotMatch;
	    }
	}
	FOREACH(variableObj, contextPtr->callPtr->chain[contextPtr->index]
		.mPtr->declaringClassPtr->variables) {
	    match = Tcl_GetStringFromObj(variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		cacheIt = 0;
		goto gotMatch;
	    }
	}
    } else {
	FOREACH_STRUCT(privateVar, contextPtr->oPtr->privateVariables) {
	    match = Tcl_GetStringFromObj(privateVar->variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		variableObj = privateVar->fullNameObj;
		cacheIt = 1;
		goto gotMatch;
	    }
	}
	FOREACH(variableObj, contextPtr->oPtr->variables) {
	    match = Tcl_GetStringFromObj(variableObj, &len);
	    if ((len == varLen) && !memcmp(match, varName, len)) {
		cacheIt = 1;
		goto gotMatch;
	    }
	}
    }
    return NULL;

    /*
     * It is a variable we want to resolve, so resolve it.
     */

  gotMatch:
    hPtr = Tcl_CreateHashEntry(TclVarTable(contextPtr->oPtr->namespacePtr),
	    variableObj, &isNew);
    if (isNew) {
	TclSetVarNamespaceVar((Var *) TclVarHashGetValue(hPtr));
    }
    if (cacheIt) {
	infoPtr->cachedObjectVar = TclVarHashGetValue(hPtr);

	/*
	 * We must keep a reference to the variable so everything will
	 * continue to work correctly even if it is unset; being unset does
	 * not end the life of the variable at this level. [Bug 3185009]
	 */

	VarHashRefCount(infoPtr->cachedObjectVar)++;
    }
    return TclVarHashGetValue(hPtr);
}

static void
ProcedureMethodCompiledVarDelete(
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;

    /*
     * Release the reference to the variable if we were holding it.
     */

    if (infoPtr->cachedObjectVar) {
	VarHashRefCount(infoPtr->cachedObjectVar)--;
	TclCleanupVar((Var *) infoPtr->cachedObjectVar, NULL);
    }
    Tcl_DecrRefCount(infoPtr->variableObj);
    Tcl_Free(infoPtr);
}

static int
ProcedureMethodCompiledVarResolver(
    TCL_UNUSED(Tcl_Interp *),
    const char *varName,
    Tcl_Size length,
    TCL_UNUSED(Tcl_Namespace *),
    Tcl_ResolvedVarInfo **rPtrPtr)
{
    OOResVarInfo *infoPtr;
    Tcl_Obj *variableObj = Tcl_NewStringObj(varName, length);

    /*
     * Do not create resolvers for cases that contain namespace separators or
     * which look like array accesses. Both will lead us astray.
     */

    if (strstr(TclGetString(variableObj), "::") != NULL ||
	    Tcl_StringMatch(TclGetString(variableObj), "*(*)")) {
	Tcl_DecrRefCount(variableObj);
	return TCL_CONTINUE;
    }

    infoPtr = (OOResVarInfo *) Tcl_Alloc(sizeof(OOResVarInfo));
    infoPtr->info.fetchProc = ProcedureMethodCompiledVarConnect;
    infoPtr->info.deleteProc = ProcedureMethodCompiledVarDelete;
    infoPtr->cachedObjectVar = NULL;
    infoPtr->variableObj = variableObj;
    Tcl_IncrRefCount(variableObj);
    *rPtrPtr = &infoPtr->info;
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Pinned Tcl 9.1.0 release source`, `generic/tclOOMethod.c`, function `ProcedureMethodVarResolver / ProcedureMethodCompiledVarConnect / ProcedureMethodCompiledVarResolver / GetRealVarName`, lines 1095–1310. Retained reference `tcl9.1-resolver-source`.

```text
GetRealVarName(
    CallContext *contextPtr,
    const char *varName,
    Tcl_Size varNameLen,
    bool *isInstanceVar)
{
    const Class *clsPtr = contextPtr->callPtr->chain[contextPtr->index]
	    .mPtr->declaringClassPtr;
    const PrivateVariableMapping *privateVar;
    Tcl_Size i;
    Tcl_Obj *variableObj;
    const char *name;
    Tcl_Size len;

    if (clsPtr) {
	FOREACH_STRUCT(privateVar, clsPtr->privateVariables) {
	    name = TclGetStringFromObj(privateVar->variableObj, &len);
	    if (varNameLen == len && !memcmp(varName, name, len)) {
		if (isInstanceVar) {
		    *isInstanceVar = false;
		}
		return privateVar->fullNameObj;
	    }
	}
	FOREACH(variableObj, clsPtr->variables) {
	    name = TclGetStringFromObj(variableObj, &len);
	    if (varNameLen == len && !memcmp(varName, name, len)) {
		if (isInstanceVar) {
		    *isInstanceVar = false;
		}
		return variableObj;
	    }
	}
    } else {
	const Object *oPtr = contextPtr->oPtr;
	FOREACH_STRUCT(privateVar, oPtr->privateVariables) {
	    name = TclGetStringFromObj(privateVar->variableObj, &len);
	    if (varNameLen == len && !memcmp(varName, name, len)) {
		if (isInstanceVar) {
		    *isInstanceVar = true;
		}
		return privateVar->fullNameObj;
	    }
	}
	FOREACH(variableObj, oPtr->variables) {
	    name = TclGetStringFromObj(variableObj, &len);
	    if (varNameLen == len && !memcmp(varName, name, len)) {
		if (isInstanceVar) {
		    *isInstanceVar = true;
		}
		return variableObj;
	    }
	}
    }
    if (isInstanceVar) {
	*isInstanceVar = false;
    }
    return NULL;
}

// Get or allocate a variable in an object.
static inline Tcl_Var
GetObjectVar(
    Object *oPtr,
    Tcl_Obj *variableObj)
{
    int isNew;
    Tcl_HashEntry *hPtr = Tcl_CreateHashEntry(
	    TclVarTable(oPtr->namespacePtr), variableObj, &isNew);
    Tcl_Var var = TclVarHashGetValue(hPtr);
    if (isNew) {
	TclSetVarNamespaceVar((Var *) var);
    }
    return var;
}

// Called on entry to a compiled context to connect the local variables to
// be resolved to the actual variables in the object instance. If we want to
// connect it, we return the variable; otherwise NULL.
// This is the core of the variable resolver.
static Tcl_Var
ProcedureMethodCompiledVarConnect(
    Tcl_Interp *interp,
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;

    /*
     * Check that the variable is being requested in a context that is also a
     * method call; if not (i.e. we're evaluating in the object's namespace or
     * in a procedure of that namespace) then we do nothing.
     */

    CallContext *contextPtr = GetCurrentCallContext(interp);
    if (!contextPtr) {
	return NULL;
    }

    /*
     * If we've done the work before (in a comparable context) then reuse that
     * rather than performing resolution ourselves.
     */

    if (infoPtr->cachedObjectVar) {
	return infoPtr->cachedObjectVar;
    }

    /*
     * Check if the variable is one we want to resolve at all (i.e. whether it
     * is in the list provided by the user). If not, we mustn't do anything
     * either.
     */

    bool cacheIt;
    Tcl_Obj *variableObj = GetRealVarName(contextPtr, infoPtr->varName,
	    infoPtr->varNameLen, &cacheIt);
    if (!variableObj) {
	return NULL;
    }

    /*
     * It is a variable we want to resolve, so resolve it.
     */

    Tcl_Var var = GetObjectVar(contextPtr->oPtr, variableObj);
    if (cacheIt) {
	infoPtr->cachedObjectVar = var;

	/*
	 * We must keep a reference to the variable so everything will
	 * continue to work correctly even if it is unset; being unset does
	 * not end the life of the variable at this level. [Bug 3185009]
	 */

	VarHashRefCount(infoPtr->cachedObjectVar)++;
    }
    return var;
}

static void
ProcedureMethodCompiledVarDelete(
    Tcl_ResolvedVarInfo *rPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) rPtr;

    /*
     * Release the reference to the variable if we were holding it.
     */

    if (infoPtr->cachedObjectVar) {
	VarHashRefCount(infoPtr->cachedObjectVar)--;
	TclCleanupVar((Var *) infoPtr->cachedObjectVar, NULL);
    }
    Tcl_Free(infoPtr);
}

static int
ProcedureMethodVarResolver(
    Tcl_Interp *interp,		// Calling context holder.
    const char *varName,	// Variable name to look for. Not array element.
    TCL_UNUSED(Tcl_Namespace *) /*contextNs*/,
    TCL_UNUSED(int) /*flags*/,	// Ignoring variable access flags (???)
    Tcl_Var *varPtr)		// Where to write found variable.
{
    /*
     * Check that the variable is being requested in a context that is also a
     * method call; if not (i.e. we're evaluating in the object's namespace or
     * in a procedure of that namespace) then we do nothing.
     */

    CallContext *contextPtr = GetCurrentCallContext(interp);
    if (!contextPtr) {
	return TCL_CONTINUE;
    }

    /*
     * Do not resolve for cases that contain namespace separators.
     */

    if ((varName[0] == ':' && varName[1] == ':')
	    || strstr(varName, "::") != NULL) {
	return TCL_CONTINUE;
    }

    Tcl_Obj *realVarObj = GetRealVarName(contextPtr, varName,
	    (Tcl_Size) strlen(varName), NULL);

    Tcl_Var var = NULL;
    if (realVarObj) {
	/*
	* It is a variable we want to resolve, so resolve it.
	*/

	*varPtr = var = GetObjectVar(contextPtr->oPtr, realVarObj);
    }
    return (var ? TCL_OK : TCL_CONTINUE);
}

static int
ProcedureMethodCompiledVarResolver(
    TCL_UNUSED(Tcl_Interp *)/*interp*/,
    const char *varName,
    Tcl_Size length,
    TCL_UNUSED(Tcl_Namespace *)/*contextNs*/,
    Tcl_ResolvedVarInfo **rPtrPtr)
{
    OOResVarInfo *infoPtr = (OOResVarInfo *) Tcl_Alloc(
	    offsetof(OOResVarInfo, varName) + sizeof(char) * (length + 1));
    infoPtr->info.fetchProc = ProcedureMethodCompiledVarConnect;
    infoPtr->info.deleteProc = ProcedureMethodCompiledVarDelete;
    infoPtr->cachedObjectVar = NULL;
    infoPtr->varNameLen = length;
    memcpy(infoPtr->varName, varName, sizeof(char) * (length + 1));
    *rPtrPtr = &infoPtr->info;
    return TCL_OK;
}

```

## Consumer bindings

- [rust/tcl-vm/src/cmd_oo/native_variables.rs](../../../../rust/tcl-vm/src/cmd_oo/native_variables.rs), `runtime_target`: Selects declaring-provider runtime namespace storage with the independent RuntimeRoot CString recipe; it does not create a compiled local.
- [runtime/rust/src/cmd_oo/native_variables.rs](../../../../runtime/rust/src/cmd_oo/native_variables.rs), `NativeOoVariableResolver::runtime_target`: Retains an actual reached method frame and current declaring inventory before selected runtime root comparison.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::add_tcloo_compiled_instance_link`: Connects an independently admitted compiled local ordinal to actual object namespace storage; declaration equality alone cannot issue a slot.
- [rust/tcl-vm/src/cmd_oo/native_variables.rs](../../../../rust/tcl-vm/src/cmd_oo/native_variables.rs), `cmd_oo::native_variables::tests::compiled_and_dynamic_declared_variables_match_original_native_controls` (linked): Compares all 48 retained counted declaration/static/dynamic/read/object-vars rows for each of C8.6/C9.0/C9.1 through independently selected native VM or Runtime interpreter contexts. It does not grant compiler preparation to source analysis or assert test execution from the native capture.
- [runtime/rust/src/cmd_oo/native_variables.rs](../../../../runtime/rust/src/cmd_oo/native_variables.rs), `cmd_oo::native_variables::tests::compiled_and_dynamic_declared_variables_match_original_native_controls` (linked): Compares all 48 retained counted declaration/static/dynamic/read/object-vars rows for each of C8.6/C9.0/C9.1 through independently selected native VM or Runtime interpreter contexts. It does not grant compiler preparation to source analysis or assert test execution from the native capture.

- [rust/tcl-syntax/src/naming/oo_variables.rs](../../../../rust/tcl-syntax/src/naming/oo_variables.rs), `naming::oo_variables::tests::declared_variable_resolver_selects_counted_primary_and_runtime_cstring_independently` (linked): The pure selected OO-variable resolver distinguishes counted compiled-primary bytes from runtime C-string boundaries, retains NUL/surrogate/opaque test inputs and refuses unsupported recipes; no external OO lookup/frame execution is attached.

Current source linkage supplies no execution or assertion credit.

## Evidence

- [rust/tcl-vm/tests/data/native_oo_variable_resolver](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver)
- [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.4.20](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.4.20)
- [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.5.19](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.5.19)
- [rust/tcl-vm/tests/data/native_oo_variable_resolver/8.6.18](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/8.6.18)
- [rust/tcl-vm/tests/data/native_oo_variable_resolver/9.0.4](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/9.0.4)
- [rust/tcl-vm/tests/data/native_oo_variable_resolver/9.1.0](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/9.1.0)
- [rust/tcl-vm/tests/data/native_oo_variable_resolver/jim](../../../../rust/tcl-vm/tests/data/native_oo_variable_resolver/jim)
