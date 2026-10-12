# naming.tcloo.explicit-variable-object-lookup-source

Kind: `source-anchor`

## Problem statement

The explicit variable and varname methods have different original-name extents and cell-creation flags; generic combined-name or AUTO-link rules cannot substitute for their selected native recipes.

## Question

Which original-object, CString, namespace-only lookup and name-construction purposes do the pinned stock LinkVar/VarName implementations select?

## Conclusion

LinkVar validates CString namespace separators, performs original namespace-only lookup with root creation and no element creation, marks the selected namespace variable and passes a CString local name to PtrMakeUpvar. Varname retains an absolute original operand or constructs a fresh qualified operand: counted append in C8.6/C9.1, CString interpolation in C9.0, independently applying C9 current-provider private-name correspondence. Its lookup creates root and element, follows aliases and reports the actual variable key. These structural recipes issue no frame, target, cache, body or Normal capability.

## Scope

Exact LF windows of TclOO_Object_LinkVar/TclOO_Object_VarName and C9 TclOOLookupObjectVar from pinned C8.6.18/9.0.4/9.1.0 source files. Source inspection only; getter/callback/materialisation and actual provider/frame/namespace inventory remain backend obligations.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Pinned source inspection; no executed build association from these excerpts.. Channel: Inspected stock method/lookup function windows, no guest input.. Dialect: Tcl.

Separate original namespace-only target/local alias and varname construction/creation recipes shown in exact functions.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Pinned source inspection; no executed build association from these excerpts.. Channel: Inspected stock method/lookup function windows, no guest input.. Dialect: Tcl.

Separate original namespace-only target/local alias and varname construction/creation recipes shown in exact functions.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Pinned source inspection; no executed build association from these excerpts.. Channel: Inspected stock method/lookup function windows, no guest input.. Dialect: Tcl.

Separate original namespace-only target/local alias and varname construction/creation recipes shown in exact functions.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `tcl8.6-TclOO_Object_LinkVar` (source-anchor): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl8.6-source.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl8.6-source.json). SHA-256 `1e59a32224a0aa5e24a76430cb53f148e399adc5f7856ebc040400d394dbb1c3`. JSON pointer `/windows/0/snippet`. Exact retained whole native function LF window.
- `tcl8.6-TclOO_Object_VarName` (source-anchor): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl8.6-source.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl8.6-source.json). SHA-256 `1e59a32224a0aa5e24a76430cb53f148e399adc5f7856ebc040400d394dbb1c3`. JSON pointer `/windows/1/snippet`. Exact retained whole native function LF window.
- `tcl9.0-TclOO_Object_LinkVar` (source-anchor): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.0-source.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.0-source.json). SHA-256 `de0ccdf3e41cb0575321f9d1b122909c7609440e9253f01402924a5110601cd9`. JSON pointer `/windows/0/snippet`. Exact retained whole native function LF window.
- `tcl9.0-TclOO_Object_VarName` (source-anchor): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.0-source.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.0-source.json). SHA-256 `de0ccdf3e41cb0575321f9d1b122909c7609440e9253f01402924a5110601cd9`. JSON pointer `/windows/1/snippet`. Exact retained whole native function LF window.
- `tcl9.0-TclOOLookupObjectVar` (source-anchor): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.0-source.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.0-source.json). SHA-256 `de0ccdf3e41cb0575321f9d1b122909c7609440e9253f01402924a5110601cd9`. JSON pointer `/windows/2/snippet`. Exact retained whole native function LF window.
- `tcl9.1-TclOO_Object_LinkVar` (source-anchor): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.1-source.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.1-source.json). SHA-256 `b40be910b05f8c76750b4ad776faa65e67701d3262a53d993fdc7165d8579197`. JSON pointer `/windows/0/snippet`. Exact retained whole native function LF window.
- `tcl9.1-TclOO_Object_VarName` (source-anchor): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.1-source.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.1-source.json). SHA-256 `b40be910b05f8c76750b4ad776faa65e67701d3262a53d993fdc7165d8579197`. JSON pointer `/windows/1/snippet`. Exact retained whole native function LF window.
- `tcl9.1-TclOOLookupObjectVar` (source-anchor): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.1-source.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/tcl9.1-source.json). SHA-256 `b40be910b05f8c76750b4ad776faa65e67701d3262a53d993fdc7165d8579197`. JSON pointer `/windows/2/snippet`. Exact retained whole native function LF window.

## Source inspection

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOBasic.c`, function `TclOO_Object_LinkVar`, lines 576–665. Full-source SHA-256 `1a8f47994517980c374d71d6914412ec3348cacf5e272b6d87c2d973626cdc98`; snippet SHA-256 `c2b48e19cc31fbdb0ce94323a86bd94a46fc67bb8677aa87fc40de29f4d94de5`; retained evidence `tcl8.6-TclOO_Object_LinkVar`.

```text
TclOO_Object_LinkVar(
    ClientData clientData,	/* Ignored. */
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Object object = Tcl_ObjectContextObject(context);
    Namespace *savedNsPtr;
    int i;

    if (objc-Tcl_ObjectContextSkippedArgs(context) < 0) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"?varName ...?");
	return TCL_ERROR;
    }

    /*
     * A sanity check. Shouldn't ever happen. (This is all that remains of a
     * more complex check inherited from [global] after we have applied the
     * fix for [Bug 2903811]; note that the fix involved *removing* code.)
     */

    if (iPtr->varFramePtr == NULL) {
	return TCL_OK;
    }

    for (i=Tcl_ObjectContextSkippedArgs(context) ; i<objc ; i++) {
	Var *varPtr, *aryPtr;
	const char *varName = TclGetString(objv[i]);

	/*
	 * The variable name must not contain a '::' since that's illegal in
	 * local names.
	 */

	if (strstr(varName, "::") != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "variable name \"%s\" illegal: must not contain namespace"
		    " separator", varName));
	    Tcl_SetErrorCode(interp, "TCL", "UPVAR", "INVERTED", (char *)NULL);
	    return TCL_ERROR;
	}

	/*
	 * Switch to the object's namespace for the duration of this call.
	 * Like this, the variable is looked up in the namespace of the
	 * object, and not in the namespace of the caller. Otherwise this
	 * would only work if the caller was a method of the object itself,
	 * which might not be true if the method was exported. This is a bit
	 * of a hack, but the simplest way to do this (pushing a stack frame
	 * would be horribly expensive by comparison).
	 */

	savedNsPtr = iPtr->varFramePtr->nsPtr;
	iPtr->varFramePtr->nsPtr = (Namespace *)
		Tcl_GetObjectNamespace(object);
	varPtr = TclObjLookupVar(interp, objv[i], NULL, TCL_NAMESPACE_ONLY,
		"define", 1, 0, &aryPtr);
	iPtr->varFramePtr->nsPtr = savedNsPtr;

	if (varPtr == NULL || aryPtr != NULL) {
	    /*
	     * Variable cannot be an element in an array. If aryPtr is not
	     * NULL, it is an element, so throw up an error and return.
	     */

	    TclVarErrMsg(interp, varName, NULL, "define",
		    "name refers to an element in an array");
	    Tcl_SetErrorCode(interp, "TCL", "UPVAR", "LOCAL_ELEMENT", (char *)NULL);
	    return TCL_ERROR;
	}

	/*
	 * Arrange for the lifetime of the variable to be correctly managed.
	 * This is copied out of Tcl_VariableObjCmd...
	 */

	if (!TclIsVarNamespaceVar(varPtr)) {
	    TclSetVarNamespaceVar(varPtr);
	}

	if (TclPtrMakeUpvar(interp, varPtr, varName, 0, -1) != TCL_OK) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOBasic.c`, function `TclOO_Object_VarName`, lines 678–776. Full-source SHA-256 `1a8f47994517980c374d71d6914412ec3348cacf5e272b6d87c2d973626cdc98`; snippet SHA-256 `e46d60160dc85d8db376acfe6bb2e6281bfc4dbc07b630d67f03e758d79e2af6`; retained evidence `tcl8.6-TclOO_Object_VarName`.

```text
TclOO_Object_VarName(
    ClientData clientData,	/* Ignored. */
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Var *varPtr, *aryVar;
    Tcl_Obj *varNamePtr, *argPtr;
    const char *arg;
    Tcl_Namespace *namespacePtr;

    if (Tcl_ObjectContextSkippedArgs(context)+1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"varName");
	return TCL_ERROR;
    }
    namespacePtr = Tcl_GetObjectNamespace(Tcl_ObjectContextObject(context));
    argPtr = objv[objc-1];
    arg = Tcl_GetString(argPtr);

    /*
     * Convert the variable name to fully-qualified form if it wasn't already.
     * This has to be done prior to lookup because we can run into problems
     * with resolvers otherwise. [Bug 3603695]
     *
     * We still need to do the lookup; the variable could be linked to another
     * variable and we want the target's name.
     */

    if (arg[0] == ':' && arg[1] == ':') {
	varNamePtr = argPtr;
    } else {
	varNamePtr = Tcl_NewStringObj(namespacePtr->fullName, -1);
	Tcl_AppendToObj(varNamePtr, "::", 2);
	Tcl_AppendObjToObj(varNamePtr, argPtr);
    }
    Tcl_IncrRefCount(varNamePtr);
    varPtr = TclObjLookupVar(interp, varNamePtr, NULL,
	    TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG, "refer to", 1, 1, &aryVar);
    Tcl_DecrRefCount(varNamePtr);
    if (varPtr == NULL) {
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "VARIABLE", arg, (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * The variable reference must not disappear too soon. [Bug 74b6110204]
     */
    if (!TclIsVarArrayElement(varPtr)) {
	TclSetVarNamespaceVar(varPtr);
    }

    /*
     * Now that we've pinned down what variable we're really talking about
     * (including traversing variable links), convert back to a name.
     */

    TclNewObj(varNamePtr);
    if (aryVar != NULL) {
	Tcl_HashEntry *hPtr;
	Tcl_HashSearch search;

	Tcl_GetVariableFullName(interp, (Tcl_Var) aryVar, varNamePtr);

	/*
	 * WARNING! This code pokes inside the implementation of hash tables!
	 */

	hPtr = Tcl_FirstHashEntry((Tcl_HashTable *) aryVar->value.tablePtr,
		&search);
	while (hPtr != NULL) {
	    if (varPtr == Tcl_GetHashValue(hPtr)) {
		Tcl_AppendPrintfToObj(varNamePtr, "(%s)",
			TclGetString(hPtr->key.objPtr));
		break;
	    }
	    hPtr = Tcl_NextHashEntry(&search);
	}
    } else if (!TclIsVarArrayElement(varPtr)) {
	Tcl_GetVariableFullName(interp, (Tcl_Var) varPtr, varNamePtr);
    } else {
	/*
	 * Target is an element of an array but we don't know which one.
	 * The name in the object's namespace is the best we can do.
	 * [Bug 2da1cb0c80]
	 */
	if (arg[0] == ':' && arg[1] == ':') {
	    Tcl_DecrRefCount(varNamePtr);
	    varNamePtr = argPtr;
	} else {
	    Tcl_AppendPrintfToObj(varNamePtr, "%s::%s",
		    namespacePtr->fullName, arg);
	}
    }
    Tcl_SetObjResult(interp, varNamePtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOBasic.c`, function `TclOO_Object_LinkVar`, lines 943–1032. Full-source SHA-256 `5782e57ef2753b052d904088cf26f43ceb96ab27afb7dbe99438e24a69e68e68`; snippet SHA-256 `979cdaaf1cd8e0a80213d7385c94ac8334a257844523f62e2402537e5f0224b0`; retained evidence `tcl9.0-TclOO_Object_LinkVar`.

```text
TclOO_Object_LinkVar(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Object object = Tcl_ObjectContextObject(context);
    Namespace *savedNsPtr;
    Tcl_Size i;

    if (objc < Tcl_ObjectContextSkippedArgs(context)) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"?varName ...?");
	return TCL_ERROR;
    }

    /*
     * A sanity check. Shouldn't ever happen. (This is all that remains of a
     * more complex check inherited from [global] after we have applied the
     * fix for [Bug 2903811]; note that the fix involved *removing* code.)
     */

    if (iPtr->varFramePtr == NULL) {
	return TCL_OK;
    }

    for (i = Tcl_ObjectContextSkippedArgs(context) ; i < objc ; i++) {
	Var *varPtr, *aryPtr;
	const char *varName = TclGetString(objv[i]);

	/*
	 * The variable name must not contain a '::' since that's illegal in
	 * local names.
	 */

	if (strstr(varName, "::") != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "variable name \"%s\" illegal: must not contain namespace"
		    " separator", varName));
	    Tcl_SetErrorCode(interp, "TCL", "UPVAR", "INVERTED", (char *)NULL);
	    return TCL_ERROR;
	}

	/*
	 * Switch to the object's namespace for the duration of this call.
	 * Like this, the variable is looked up in the namespace of the
	 * object, and not in the namespace of the caller. Otherwise this
	 * would only work if the caller was a method of the object itself,
	 * which might not be true if the method was exported. This is a bit
	 * of a hack, but the simplest way to do this (pushing a stack frame
	 * would be horribly expensive by comparison).
	 */

	savedNsPtr = iPtr->varFramePtr->nsPtr;
	iPtr->varFramePtr->nsPtr = (Namespace *)
		Tcl_GetObjectNamespace(object);
	varPtr = TclObjLookupVar(interp, objv[i], NULL, TCL_NAMESPACE_ONLY,
		"define", 1, 0, &aryPtr);
	iPtr->varFramePtr->nsPtr = savedNsPtr;

	if (varPtr == NULL || aryPtr != NULL) {
	    /*
	     * Variable cannot be an element in an array. If aryPtr is not
	     * NULL, it is an element, so throw up an error and return.
	     */

	    TclVarErrMsg(interp, varName, NULL, "define",
		    "name refers to an element in an array");
	    Tcl_SetErrorCode(interp, "TCL", "UPVAR", "LOCAL_ELEMENT", (char *)NULL);
	    return TCL_ERROR;
	}

	/*
	 * Arrange for the lifetime of the variable to be correctly managed.
	 * This is copied out of Tcl_VariableObjCmd...
	 */

	if (!TclIsVarNamespaceVar(varPtr)) {
	    TclSetVarNamespaceVar(varPtr);
	}

	if (TclPtrMakeUpvar(interp, varPtr, varName, 0, -1) != TCL_OK) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOBasic.c`, function `TclOO_Object_VarName`, lines 1160–1206. Full-source SHA-256 `5782e57ef2753b052d904088cf26f43ceb96ab27afb7dbe99438e24a69e68e68`; snippet SHA-256 `7f0ff9041a944d00216eefb2279f5754fda8278c237286104d14d8d45a51b328`; retained evidence `tcl9.0-TclOO_Object_VarName`.

```text
TclOO_Object_VarName(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Tcl_Var varPtr, aryVar;
    Tcl_Obj *varNamePtr;

    if ((int) Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"varName");
	return TCL_ERROR;
    }

    varPtr = TclOOLookupObjectVar(interp, Tcl_ObjectContextObject(context),
	    objv[objc - 1], &aryVar);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * The variable reference must not disappear too soon. [Bug 74b6110204]
     */
    if (!TclIsVarArrayElement((Var *) varPtr)) {
	TclSetVarNamespaceVar((Var *) varPtr);
    }

    /*
     * Now that we've pinned down what variable we're really talking about
     * (including traversing variable links), convert back to a name.
     */

    TclNewObj(varNamePtr);

    if (aryVar != NULL) {
	Tcl_GetVariableFullName(interp, aryVar, varNamePtr);
	Tcl_AppendPrintfToObj(varNamePtr, "(%s)", Tcl_GetString(
		VarHashGetKey(varPtr)));
    } else {
	Tcl_GetVariableFullName(interp, varPtr, varNamePtr);
    }
    Tcl_SetObjResult(interp, varNamePtr);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOBasic.c`, function `TclOOLookupObjectVar`, lines 1047–1147. Full-source SHA-256 `5782e57ef2753b052d904088cf26f43ceb96ab27afb7dbe99438e24a69e68e68`; snippet SHA-256 `b199d8173ed5f55768e98d22ee83ca7f44600ba4c7802924933e1efcc07bb0c3`; retained evidence `tcl9.0-TclOOLookupObjectVar`.

```text
TclOOLookupObjectVar(
    Tcl_Interp *interp,
    Tcl_Object object,		/* Object we're looking up within. */
    Tcl_Obj *varName,		/* User-visible name we're looking up. */
    Tcl_Var *aryPtr)		/* Where to write the handle to the array
				 * containing the element; if not an element,
				 * then the variable this points to is set to
				 * NULL. */
{
    const char *arg = TclGetString(varName);
    Tcl_Obj *varNamePtr;

    /*
     * Convert the variable name to fully-qualified form if it wasn't already.
     * This has to be done prior to lookup because we can run into problems
     * with resolvers otherwise. [Bug 3603695]
     *
     * We still need to do the lookup; the variable could be linked to another
     * variable and we want the target's name.
     */

    if (arg[0] == ':' && arg[1] == ':') {
	varNamePtr = varName;
    } else {
	Tcl_Namespace *namespacePtr = Tcl_GetObjectNamespace(object);
	CallFrame *framePtr = ((Interp *) interp)->varFramePtr;

	/*
	 * Private method handling. [TIP 500]
	 *
	 * If we're in a context that can see some private methods of an
	 * object, we may need to precede a variable name with its prefix.
	 * This is a little tricky as we need to check through the inheritance
	 * hierarchy when the method was declared by a class to see if the
	 * current object is an instance of that class.
	 */

	if (framePtr->isProcCallFrame & FRAME_IS_METHOD) {
	    Object *oPtr = (Object *) object;
	    CallContext *callerContext = (CallContext *) framePtr->clientData;
	    Method *mPtr = callerContext->callPtr->chain[
		    callerContext->index].mPtr;
	    PrivateVariableMapping *pvPtr;
	    Tcl_Size i;

	    if (mPtr->declaringObjectPtr == oPtr) {
		FOREACH_STRUCT(pvPtr, oPtr->privateVariables) {
		    if (!TclStringCmp(pvPtr->variableObj, varName, 1, 0,
			    TCL_INDEX_NONE)) {
			varName = pvPtr->fullNameObj;
			break;
		    }
		}
	    } else if (mPtr->declaringClassPtr &&
		    mPtr->declaringClassPtr->privateVariables.num) {
		Class *clsPtr = mPtr->declaringClassPtr;
		int isInstance = TclOOIsReachable(clsPtr, oPtr->selfCls);
		Class *mixinCls;

		if (!isInstance) {
		    FOREACH(mixinCls, oPtr->mixins) {
			if (TclOOIsReachable(clsPtr, mixinCls)) {
			    isInstance = 1;
			    break;
			}
		    }
		}
		if (isInstance) {
		    FOREACH_STRUCT(pvPtr, clsPtr->privateVariables) {
			if (!TclStringCmp(pvPtr->variableObj, varName, 1, 0,
				TCL_INDEX_NONE)) {
			    varName = pvPtr->fullNameObj;
			    break;
			}
		    }
		}
	    }
	}

	// The namespace isn't the global one; necessarily true for any object!
	varNamePtr = Tcl_ObjPrintf("%s::%s",
		namespacePtr->fullName, TclGetString(varName));
    }
    Tcl_IncrRefCount(varNamePtr);
    Tcl_Var var = (Tcl_Var) TclObjLookupVar(interp, varNamePtr, NULL,
	    TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG, "refer to", 1, 1,
	    (Var **) aryPtr);
    Tcl_DecrRefCount(varNamePtr);
    if (var == NULL) {
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "VARIABLE", arg, (void *) NULL);
    } else if (*aryPtr == NULL && TclIsVarArrayElement((Var *) var)) {
	/*
	 * If the varPtr points to an element of an array but we don't already
	 * have the array, find it now. Note that this can't be easily
	 * backported; the arrayPtr field is new in Tcl 9.0. [Bug 2da1cb0c80]
	 */
	*aryPtr = (Tcl_Var) TclVarParentArray(var);
    }

    return var;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOBasic.c`, function `TclOO_Object_LinkVar`, lines 998–1087. Full-source SHA-256 `d8b40b5adf0e96bbcac34fa6a7035dbc4ac431218c28924766c4c13532aff158`; snippet SHA-256 `bf0582a1c70a927f3021e259882ac3711a0684e9f910891ff84d9f9c53c43ed6`; retained evidence `tcl9.1-TclOO_Object_LinkVar`.

```text
TclOO_Object_LinkVar(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Object object = Tcl_ObjectContextObject(context);
    Namespace *savedNsPtr;
    Tcl_Size i;

    if (objc < Tcl_ObjectContextSkippedArgs(context)) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"?varName ...?");
	return TCL_ERROR;
    }

    /*
     * A sanity check. Shouldn't ever happen. (This is all that remains of a
     * more complex check inherited from [global] after we have applied the
     * fix for [Bug 2903811]; note that the fix involved *removing* code.)
     */

    if (iPtr->varFramePtr == NULL) {
	return TCL_OK;
    }

    for (i = Tcl_ObjectContextSkippedArgs(context) ; i < objc ; i++) {
	Var *varPtr, *aryPtr;
	const char *varName = TclGetString(objv[i]);

	/*
	 * The variable name must not contain a '::' since that's illegal in
	 * local names.
	 */

	if (strstr(varName, "::") != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "variable name \"%s\" illegal: must not contain namespace"
		    " separator", varName));
	    Tcl_SetErrorCode(interp, "TCL", "UPVAR", "INVERTED", (char *)NULL);
	    return TCL_ERROR;
	}

	/*
	 * Switch to the object's namespace for the duration of this call.
	 * Like this, the variable is looked up in the namespace of the
	 * object, and not in the namespace of the caller. Otherwise this
	 * would only work if the caller was a method of the object itself,
	 * which might not be true if the method was exported. This is a bit
	 * of a hack, but the simplest way to do this (pushing a stack frame
	 * would be horribly expensive by comparison).
	 */

	savedNsPtr = iPtr->varFramePtr->nsPtr;
	iPtr->varFramePtr->nsPtr = (Namespace *)
		Tcl_GetObjectNamespace(object);
	varPtr = TclObjLookupVar(interp, objv[i], NULL, TCL_NAMESPACE_ONLY,
		"define", 1, 0, &aryPtr);
	iPtr->varFramePtr->nsPtr = savedNsPtr;

	if (varPtr == NULL || aryPtr != NULL) {
	    /*
	     * Variable cannot be an element in an array. If aryPtr is not
	     * NULL, it is an element, so throw up an error and return.
	     */

	    TclVarErrMsg(interp, varName, NULL, "define",
		    "name refers to an element in an array");
	    Tcl_SetErrorCode(interp, "TCL", "UPVAR", "LOCAL_ELEMENT", (char *)NULL);
	    return TCL_ERROR;
	}

	/*
	 * Arrange for the lifetime of the variable to be correctly managed.
	 * This is copied out of Tcl_VariableObjCmd...
	 */

	if (!TclIsVarNamespaceVar(varPtr)) {
	    TclSetVarNamespaceVar(varPtr);
	}

	if (TclPtrMakeUpvar(interp, varPtr, varName, 0, -1) != TCL_OK) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOBasic.c`, function `TclOO_Object_VarName`, lines 1217–1263. Full-source SHA-256 `d8b40b5adf0e96bbcac34fa6a7035dbc4ac431218c28924766c4c13532aff158`; snippet SHA-256 `0bf7a895345db95744de9296f6857961afa70870cf7e285e75e2cd75f3a1383e`; retained evidence `tcl9.1-TclOO_Object_VarName`.

```text
TclOO_Object_VarName(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Tcl_Var varPtr, aryVar;
    Tcl_Obj *varNamePtr;

    if (Tcl_ObjectContextSkippedArgs(context) + 1 != objc) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"varName");
	return TCL_ERROR;
    }

    varPtr = TclOOLookupObjectVar(interp, Tcl_ObjectContextObject(context),
	    objv[objc - 1], &aryVar);
    if (varPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * The variable reference must not disappear too soon. [Bug 74b6110204]
     */
    if (!TclIsVarArrayElement((Var *) varPtr)) {
	TclSetVarNamespaceVar((Var *) varPtr);
    }

    /*
     * Now that we've pinned down what variable we're really talking about
     * (including traversing variable links), convert back to a name.
     */

    TclNewObj(varNamePtr);

    if (aryVar != NULL) {
	Tcl_GetVariableFullName(interp, aryVar, varNamePtr);
	Tcl_AppendPrintfToObj(varNamePtr, "(%s)", Tcl_GetString(
		VarHashGetKey(varPtr)));
    } else {
	Tcl_GetVariableFullName(interp, varPtr, varNamePtr);
    }
    Tcl_SetObjResult(interp, varNamePtr);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOBasic.c`, function `TclOOLookupObjectVar`, lines 1102–1204. Full-source SHA-256 `d8b40b5adf0e96bbcac34fa6a7035dbc4ac431218c28924766c4c13532aff158`; snippet SHA-256 `7859fc98401df3a549e1773dbe7eaf67737922ec396cab46540155899715b090`; retained evidence `tcl9.1-TclOOLookupObjectVar`.

```text
TclOOLookupObjectVar(
    Tcl_Interp *interp,
    Tcl_Object object,		/* Object we're looking up within. */
    Tcl_Obj *varName,		/* User-visible name we're looking up. */
    Tcl_Var *aryPtr)		/* Where to write the handle to the array
				 * containing the element; if not an element,
				 * then the variable this points to is set to
				 * NULL. */
{
    const char *arg = TclGetString(varName);
    Tcl_Obj *varNamePtr;

    /*
     * Convert the variable name to fully-qualified form if it wasn't already.
     * This has to be done prior to lookup because we can run into problems
     * with resolvers otherwise. [Bug 3603695]
     *
     * We still need to do the lookup; the variable could be linked to another
     * variable and we want the target's name.
     */

    if (arg[0] == ':' && arg[1] == ':') {
	varNamePtr = varName;
    } else {
	Tcl_DString ds;
	Tcl_DStringInit(&ds);
	Tcl_DStringAppend(&ds, 	Tcl_GetObjectNamespace(object)->fullName, -1);
	Tcl_DStringAppend(&ds, "::", 2);
	CallFrame *framePtr = ((Interp *) interp)->varFramePtr;

	/*
	 * Private method handling. [TIP 500]
	 *
	 * If we're in a context that can see some private methods of an
	 * object, we may need to precede a variable name with its prefix.
	 * This is a little tricky as we need to check through the inheritance
	 * hierarchy when the method was declared by a class to see if the
	 * current object is an instance of that class.
	 */

	if (framePtr->isProcCallFrame & FRAME_IS_METHOD) {
	    Object *oPtr = (Object *) object;
	    CallContext *callerContext = (CallContext *) framePtr->clientData;
	    Method *mPtr = CurrentlyInvoked(callerContext).mPtr;
	    PrivateVariableMapping *pvPtr;
	    Tcl_Size i;

	    if (mPtr->declaringObjectPtr == oPtr) {
		FOREACH_STRUCT(pvPtr, oPtr->privateVariables) {
		    if (!TclStringCmp(pvPtr->variableObj, varName, 1, 0,
			    TCL_INDEX_NONE)) {
			varName = pvPtr->fullNameObj;
			break;
		    }
		}
	    } else if (mPtr->declaringClassPtr &&
		    mPtr->declaringClassPtr->privateVariables.num) {
		Class *clsPtr = mPtr->declaringClassPtr;
		bool isInstance = TclOOIsReachable(clsPtr, oPtr->selfCls);
		Class *mixinCls;

		if (!isInstance) {
		    FOREACH(mixinCls, oPtr->mixins) {
			if (TclOOIsReachable(clsPtr, mixinCls)) {
			    isInstance = true;
			    break;
			}
		    }
		}
		if (isInstance) {
		    FOREACH_STRUCT(pvPtr, clsPtr->privateVariables) {
			if (!TclStringCmp(pvPtr->variableObj, varName, 1, 0,
				TCL_INDEX_NONE)) {
			    varName = pvPtr->fullNameObj;
			    break;
			}
		    }
		}
	    }
	}

	// The namespace isn't the global one; necessarily true for any object!
	TclDStringAppendObj(&ds, varName);
	varNamePtr = Tcl_DStringToObj(&ds);
    }
    Tcl_IncrRefCount(varNamePtr);
    Tcl_Var var = (Tcl_Var) TclObjLookupVar(interp, varNamePtr, NULL,
	    TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG, "refer to", 1, 1,
	    (Var **) aryPtr);
    Tcl_DecrRefCount(varNamePtr);
    if (var == NULL) {
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "VARIABLE", arg, (void *) NULL);
    } else if (*aryPtr == NULL && TclIsVarArrayElement((Var *) var)) {
	/*
	 * If the varPtr points to an element of an array but we don't already
	 * have the array, find it now. Note that this can't be easily
	 * backported; the arrayPtr field is new in Tcl 9.0. [Bug 2da1cb0c80]
	 */
	*aryPtr = (Tcl_Var) TclVarParentArray(var);
    }

    return var;
}

```


## Consumer bindings

- [rust/tcl-syntax/src/naming/oo_variables.rs](../../../../rust/tcl-syntax/src/naming/oo_variables.rs), `native_oo_explicit_variable_local_name`: Purpose-separated pure byte selection or construction; no receiver or native object grant.
- [rust/tcl-syntax/src/naming/oo_variables.rs](../../../../rust/tcl-syntax/src/naming/oo_variables.rs), `native_oo_varname_lookup_bytes`: Purpose-separated pure byte selection or construction; no receiver or native object grant.
- [rust/tcl-syntax/src/naming/oo_variables.rs](../../../../rust/tcl-syntax/src/naming/oo_variables.rs), `native_oo_variable_key_report`: Purpose-separated pure byte selection or construction; no receiver or native object grant.
- [rust/tcl-syntax/src/naming/oo_variables.rs](../../../../rust/tcl-syntax/src/naming/oo_variables.rs), `native_oo_private_variable_matches`: Purpose-separated pure byte selection or construction; no receiver or native object grant.
- [rust/tcl-syntax/src/naming/oo_variables.rs](../../../../rust/tcl-syntax/src/naming/oo_variables.rs), `naming::oo_variables::explicit_variable_tests::explicit_local_and_varname_lookup_keep_their_independent_byte_extents` (linked): Separates local CString, counted lookup construction, private-name match purpose, reporting view and unavailable engines; no native operation pass.

A named test is a coverage binding, not a claim that it executed.

## Replay

Offline verify-only checked each immutable capture variant and all1740 protocol rows; zero native/compiler/Rust launches. Actual object namespace IDs belong to their individual fresh captures. V1 user varname recursion is a measured harness-input limitation, not guest unavailable. C8.4/Jim return-options C API not captured. Source inspection can be reproduced only by matching pinned full-file SHA and LF snippet windows; native runner cannot reinspect source.
