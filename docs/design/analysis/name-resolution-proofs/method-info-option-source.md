# naming.tcloo.method-info-option-source

Kind: `source-anchor`

## Problem statement

Counted method option bytes, selected Index prefix/exactness, argument arity and target lookup order are independent purposes. A whole-byte String matcher or duplicated consumer table can select a different option or error boundary from the actual native C recipe.

## Question

What target/option order, release tables, sequential scope fold and local visibility mask do the pinned C8.6/C9 method handlers define?

## Conclusion

All six inspected handlers first require a target and resolve it before sequential original-object Index option lookup. C8.6 has -all/-localprivate/-private; C9 adds -scope and uses the last selected scope, clearing recursion after the loop. C9 legacy local matching excludes TRUE_PRIVATE_METHOD, while explicit scope matches SCOPE_FLAGS exactly. Original local result name objects and recursive roster/result protocols are separate from these option and flag recipes.

## Scope

Full pinned LF-only InfoObjectMethodsCmd/InfoClassMethodsCmd windows for C8.6.18/9.0.4/9.1.0. Source inspection only; no source-body inspection is treated as a guest launch, physical method inventory, target lifetime, cache or completion grant.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No inspected method handler for this provider.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No inspected method handler for this provider.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Pinned source inspection only; no executed source association is inferred.. Channel: Inspected full native method handler; no guest input.. Dialect: Tcl.

All six inspected handlers first require a target and resolve it before sequential original-object Index option lookup. C8.6 has -all/-localprivate/-private; C9 adds -scope and uses the last selected scope, clearing recursion after the loop. C9 legacy local matching excludes TRUE_PRIVATE_METHOD, while explicit scope matches SCOPE_FLAGS exactly. Original local result name objects and recursive roster/result protocols are separate from these option and flag recipes.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Pinned source inspection only; no executed source association is inferred.. Channel: Inspected full native method handler; no guest input.. Dialect: Tcl.

All six inspected handlers first require a target and resolve it before sequential original-object Index option lookup. C8.6 has -all/-localprivate/-private; C9 adds -scope and uses the last selected scope, clearing recursion after the loop. C9 legacy local matching excludes TRUE_PRIVATE_METHOD, while explicit scope matches SCOPE_FLAGS exactly. Original local result name objects and recursive roster/result protocols are separate from these option and flag recipes.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Pinned source inspection only; no executed source association is inferred.. Channel: Inspected full native method handler; no guest input.. Dialect: Tcl.

All six inspected handlers first require a target and resolve it before sequential original-object Index option lookup. C8.6 has -all/-localprivate/-private; C9 adds -scope and uses the last selected scope, clearing recursion after the loop. C9 legacy local matching excludes TRUE_PRIVATE_METHOD, while explicit scope matches SCOPE_FLAGS exactly. Original local result name objects and recursive roster/result protocols are separate from these option and flag recipes.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No inspected method handler for this provider.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance source inspection.

## Exact evidence

- `tcl8.6-InfoObjectMethodsCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl8.6-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl8.6-source.json). SHA-256 `5245ab65fdfccaf2a2f3e5998baa43f1958eeab53f7cadabac6f7c9d7e1bb470`. JSON pointer `/windows/0/snippet`. Pinned complete LF function, including target lookup, sequential original Index and local filter. 
- `tcl8.6-InfoClassMethodsCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl8.6-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl8.6-source.json). SHA-256 `5245ab65fdfccaf2a2f3e5998baa43f1958eeab53f7cadabac6f7c9d7e1bb470`. JSON pointer `/windows/1/snippet`. Pinned complete LF function, including target lookup, sequential original Index and local filter. 
- `tcl9.0-InfoObjectMethodsCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl9.0-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl9.0-source.json). SHA-256 `39af8f9a7bf993b7d819a4415b75563c1de155fe602b24ed14b9d5629efa5a46`. JSON pointer `/windows/0/snippet`. Pinned complete LF function, including target lookup, sequential original Index and local filter. 
- `tcl9.0-InfoClassMethodsCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl9.0-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl9.0-source.json). SHA-256 `39af8f9a7bf993b7d819a4415b75563c1de155fe602b24ed14b9d5629efa5a46`. JSON pointer `/windows/1/snippet`. Pinned complete LF function, including target lookup, sequential original Index and local filter. 
- `tcl9.1-InfoObjectMethodsCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl9.1-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl9.1-source.json). SHA-256 `657e0253bcfd76195b657454fb8ae2fc997c79924bc353e2817006b40df81517`. JSON pointer `/windows/0/snippet`. Pinned complete LF function, including target lookup, sequential original Index and local filter. 
- `tcl9.1-InfoClassMethodsCmd` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl9.1-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/tcl9.1-source.json). SHA-256 `657e0253bcfd76195b657454fb8ae2fc997c79924bc353e2817006b40df81517`. JSON pointer `/windows/1/snippet`. Pinned complete LF function, including target lookup, sequential original Index and local filter. 

## Source inspection

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInfo.c`, function `InfoObjectMethodsCmd`, lines 513–582. Full-source SHA-256 `309603344c3aeebe95832928b473a84ebc001b3d85c8489e15675448035de02b`; snippet SHA-256 `a41fdcf7953531f41d7061cc87892ac2d2a64556f763a14c7b30d628de829373`; retained evidence `tcl8.6-InfoObjectMethodsCmd`.

```text
InfoObjectMethodsCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Object *oPtr;
    int flag = PUBLIC_METHOD, recurse = 0;
    FOREACH_HASH_DECLS;
    Tcl_Obj *namePtr, *resultObj;
    Method *mPtr;
    static const char *const options[] = {
	"-all", "-localprivate", "-private", NULL
    };
    enum Options {
	OPT_ALL, OPT_LOCALPRIVATE, OPT_PRIVATE
    };

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName ?-option value ...?");
	return TCL_ERROR;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc != 2) {
	int i, idx;

	for (i=2 ; i<objc ; i++) {
	    if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0,
		    &idx) != TCL_OK) {
		return TCL_ERROR;
	    }
	    switch ((enum Options) idx) {
	    case OPT_ALL:
		recurse = 1;
		break;
	    case OPT_LOCALPRIVATE:
		flag = PRIVATE_METHOD;
		break;
	    case OPT_PRIVATE:
		flag = 0;
		break;
	    }
	}
    }

    TclNewObj(resultObj);
    if (recurse) {
	const char **names;
	int i, numNames = TclOOGetSortedMethodList(oPtr, flag, &names);

	for (i=0 ; i<numNames ; i++) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    Tcl_NewStringObj(names[i], -1));
	}
	if (numNames > 0) {
	    ckfree(names);
	}
    } else if (oPtr->methodsPtr) {
	FOREACH_HASH(namePtr, mPtr, oPtr->methodsPtr) {
	    if (mPtr->typePtr && (mPtr->flags & flag) == flag) {
		Tcl_ListObjAppendElement(NULL, resultObj, namePtr);
	    }
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOOInfo.c`, function `InfoClassMethodsCmd`, lines 1137–1207. Full-source SHA-256 `309603344c3aeebe95832928b473a84ebc001b3d85c8489e15675448035de02b`; snippet SHA-256 `38e12e7351ff1b077d1181a2f6c248bbee1f94883e2217b348d9cf3aeb40213a`; retained evidence `tcl8.6-InfoClassMethodsCmd`.

```text
InfoClassMethodsCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    int flag = PUBLIC_METHOD, recurse = 0;
    Tcl_Obj *namePtr, *resultObj;
    Method *mPtr;
    Class *clsPtr;
    static const char *const options[] = {
	"-all", "-localprivate", "-private", NULL
    };
    enum Options {
	OPT_ALL, OPT_LOCALPRIVATE, OPT_PRIVATE
    };

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "className ?-option value ...?");
	return TCL_ERROR;
    }
    clsPtr = GetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc != 2) {
	int i, idx;

	for (i=2 ; i<objc ; i++) {
	    if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0,
		    &idx) != TCL_OK) {
		return TCL_ERROR;
	    }
	    switch ((enum Options) idx) {
	    case OPT_ALL:
		recurse = 1;
		break;
	    case OPT_LOCALPRIVATE:
		flag = PRIVATE_METHOD;
		break;
	    case OPT_PRIVATE:
		flag = 0;
		break;
	    }
	}
    }

    TclNewObj(resultObj);
    if (recurse) {
	const char **names;
	int i, numNames = TclOOGetSortedClassMethodList(clsPtr, flag, &names);

	for (i=0 ; i<numNames ; i++) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    Tcl_NewStringObj(names[i], -1));
	}
	if (numNames > 0) {
	    ckfree(names);
	}
    } else {
	FOREACH_HASH_DECLS;

	FOREACH_HASH(namePtr, mPtr, &clsPtr->classMethods) {
	    if (mPtr->typePtr && (mPtr->flags & flag) == flag) {
		Tcl_ListObjAppendElement(NULL, resultObj, namePtr);
	    }
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInfo.c`, function `InfoObjectMethodsCmd`, lines 566–697. Full-source SHA-256 `a1ed9fa857c89ba0cc5eea5a6f4e3be82783130823d5182c8e4a4abc5073158b`; snippet SHA-256 `152b47c5476d5c188a61fd56324603009f64561cd450897b6459e8c6c3af5f24`; retained evidence `tcl9.0-InfoObjectMethodsCmd`.

```text
InfoObjectMethodsCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
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
    int flag = PUBLIC_METHOD, recurse = 0, scope = SCOPE_DEFAULT;
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
	int i;

	for (i=2 ; i<objc ; i++) {
	    if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0,
		    &idx) != TCL_OK) {
		return TCL_ERROR;
	    }
	    switch (idx) {
	    case OPT_ALL:
		recurse = 1;
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
	recurse = 0;
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
     * List matching methods.
     */

    TclNewObj(resultObj);
    if (recurse) {
	const char **names;
	int i, numNames = TclOOGetSortedMethodList(oPtr, NULL, NULL, flag,
		&names);

	for (i=0 ; i<numNames ; i++) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    Tcl_NewStringObj(names[i], TCL_AUTO_LENGTH));
	}
	if (numNames > 0) {
	    Tcl_Free((void *)names);
	}
    } else if (oPtr->methodsPtr) {
	if (scope == SCOPE_DEFAULT) {
	    /*
	     * Handle legacy-mode matching. [Bug 36e5517a6850]
	     */
	    int scopeFilter = flag | TRUE_PRIVATE_METHOD;

	    FOREACH_HASH(namePtr, mPtr, oPtr->methodsPtr) {
		if (mPtr->typePtr && (mPtr->flags & scopeFilter) == flag) {
		    Tcl_ListObjAppendElement(NULL, resultObj, namePtr);
		}
	    }
	} else {
	    FOREACH_HASH(namePtr, mPtr, oPtr->methodsPtr) {
		if (mPtr->typePtr && (mPtr->flags & SCOPE_FLAGS) == flag) {
		    Tcl_ListObjAppendElement(NULL, resultObj, namePtr);
		}
	    }
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOInfo.c`, function `InfoClassMethodsCmd`, lines 1358–1479. Full-source SHA-256 `a1ed9fa857c89ba0cc5eea5a6f4e3be82783130823d5182c8e4a4abc5073158b`; snippet SHA-256 `bd87a6b2dc576fda793f0c4176744e6613fac7a8036e6a28f6ee7fe64f9dc314`; retained evidence `tcl9.0-InfoClassMethodsCmd`.

```text
InfoClassMethodsCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
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
	SCOPE_DEFAULT = -1
    };
    int flag = PUBLIC_METHOD, recurse = 0, scope = SCOPE_DEFAULT;
    Tcl_Obj *namePtr, *resultObj;
    Method *mPtr;
    Class *clsPtr;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "className ?-option value ...?");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc != 2) {
	int i;

	for (i=2 ; i<objc ; i++) {
	    if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0,
		    &idx) != TCL_OK) {
		return TCL_ERROR;
	    }
	    switch (idx) {
	    case OPT_ALL:
		recurse = 1;
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
	recurse = 0;
	switch (scope) {
	case SCOPE_PRIVATE:
	    flag = TRUE_PRIVATE_METHOD;
	    break;
	case SCOPE_PUBLIC:
	    flag = PUBLIC_METHOD;
	    break;
	case SCOPE_UNEXPORTED:
	    flag = 0;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

    TclNewObj(resultObj);
    if (recurse) {
	const char **names;
	Tcl_Size i, numNames = TclOOGetSortedClassMethodList(clsPtr, flag, &names);

	for (i=0 ; i<numNames ; i++) {
	    Tcl_ListObjAppendElement(NULL, resultObj,
		    Tcl_NewStringObj(names[i], TCL_AUTO_LENGTH));
	}
	if (numNames > 0) {
	    Tcl_Free((void *)names);
	}
    } else {
	FOREACH_HASH_DECLS;

	if (scope == SCOPE_DEFAULT) {
	    /*
	     * Handle legacy-mode matching. [Bug 36e5517a6850]
	     */
	    int scopeFilter = flag | TRUE_PRIVATE_METHOD;

	    FOREACH_HASH(namePtr, mPtr, &clsPtr->classMethods) {
		if (mPtr->typePtr && (mPtr->flags & scopeFilter) == flag) {
		    Tcl_ListObjAppendElement(NULL, resultObj, namePtr);
		}
	    }
	} else {
	    FOREACH_HASH(namePtr, mPtr, &clsPtr->classMethods) {
		if (mPtr->typePtr && (mPtr->flags & SCOPE_FLAGS) == flag) {
		    Tcl_ListObjAppendElement(NULL, resultObj, namePtr);
		}
	    }
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoObjectMethodsCmd`, lines 659–785. Full-source SHA-256 `5f46472c0039bfd89a5ed06aba1df376942136c6ddf02d5c6a0b2cf0bce2a5a9`; snippet SHA-256 `50915c6ff31a21d69bf1bd5bf04c20a6f8f1168c3667155d48166ff856693191`; retained evidence `tcl9.1-InfoObjectMethodsCmd`.

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
     * List matching methods.
     */

    TclNewObj(resultObj);
    if (recurse) {
	Tcl_Obj **names;
	int numNames = TclOOGetSortedMethodList(oPtr, NULL, NULL, flag, &names);

	TclListObjAppendElements(NULL, resultObj, numNames, names);
	if (numNames > 0) {
	    Tcl_Free((void *)names);
	}
    } else if (oPtr->methodsPtr) {
	if (scope == SCOPE_DEFAULT) {
	    /*
	     * Handle legacy-mode matching. [Bug 36e5517a6850]
	     */
	    int scopeFilter = flag | TRUE_PRIVATE_METHOD;

	    FOREACH_HASH(namePtr, mPtr, oPtr->methodsPtr) {
		if (mPtr->type2Ptr && (mPtr->flags & scopeFilter) == flag) {
		    Tcl_ListObjAppendElement(NULL, resultObj, namePtr);
		}
	    }
	} else {
	    FOREACH_HASH(namePtr, mPtr, oPtr->methodsPtr) {
		if (mPtr->type2Ptr && (mPtr->flags & SCOPE_FLAGS) == flag) {
		    Tcl_ListObjAppendElement(NULL, resultObj, namePtr);
		}
	    }
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOInfo.c`, function `InfoClassMethodsCmd`, lines 1371–1488. Full-source SHA-256 `5f46472c0039bfd89a5ed06aba1df376942136c6ddf02d5c6a0b2cf0bce2a5a9`; snippet SHA-256 `c67ca370d34809831a88bb8014f95eb847aa28d60fcb3251a95f6209edb39836`; retained evidence `tcl9.1-InfoClassMethodsCmd`.

```text
InfoClassMethodsCmd(
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
	SCOPE_DEFAULT = -1
    };
    int flag = PUBLIC_METHOD, scope = SCOPE_DEFAULT;
    bool recurse = false;
    Tcl_Obj *namePtr, *resultObj;
    Method *mPtr;
    Class *clsPtr;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "className ?-option value ...?");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
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
	case SCOPE_UNEXPORTED:
	    flag = 0;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

    TclNewObj(resultObj);
    if (recurse) {
	Tcl_Obj **names;
	Tcl_Size numNames = TclOOGetSortedClassMethodList(clsPtr, flag, &names);

	TclListObjAppendElements(NULL, resultObj, numNames, names);
	if (numNames > 0) {
	    Tcl_Free((void *)names);
	}
    } else {
	FOREACH_HASH_DECLS;

	if (scope == SCOPE_DEFAULT) {
	    /*
	     * Handle legacy-mode matching. [Bug 36e5517a6850]
	     */
	    int scopeFilter = flag | TRUE_PRIVATE_METHOD;

	    FOREACH_HASH(namePtr, mPtr, &clsPtr->classMethods) {
		if (mPtr->type2Ptr && (mPtr->flags & scopeFilter) == flag) {
		    Tcl_ListObjAppendElement(NULL, resultObj, namePtr);
		}
	    }
	} else {
	    FOREACH_HASH(namePtr, mPtr, &clsPtr->classMethods) {
		if (mPtr->type2Ptr && (mPtr->flags & SCOPE_FLAGS) == flag) {
		    Tcl_ListObjAppendElement(NULL, resultObj, namePtr);
		}
	    }
	}
    }
    Tcl_SetObjResult(interp, resultObj);
    return TCL_OK;
}

```


## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_info/methods.rs](../../../../rust/tcl-registry/src/native_tcloo_info/methods.rs), `NativeTclooMethodInfoSelection::matches_local`: Pure local flags mask, no method inventory or body/name object grant.
- [rust/tcl-registry/src/native_tcloo_info/methods.rs](../../../../rust/tcl-registry/src/native_tcloo_info/methods.rs), `native_tcloo_info::methods::tests::method_options_keep_cstring_prefix_and_sequential_scope_selection` (linked): Pure selected CString prefix/ordering/filter recipe and unsupported release negatives, separate from original Index/cache and target authority.

A named test is a coverage binding, not a claim that it executed.

## Replay

Reproduce exact LF ranges and full/snippet digests from each pinned source path; native replay does not reinspect source.
