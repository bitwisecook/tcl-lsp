# naming.tcloo.original-own-object-method-transfer

Kind: `implementation-contract`

## Problem statement

An own-object method can replace a class-provided method on one allocated instance while other instances continue to use the class entry. Joining the source declaration through the class object table, a reporting object name or a stale captured prefix can select a different implementation and can incorrectly borrow the class instance variable resolver.

## Question

Does a closed original own-object method setter retain the actual object allocation, selected worker and native-valid original fields, and do later method consumers use only that object's current own table?

## Conclusion

Ordinary three-operand deferred method setters compose selected intrinsic transfers into the allocated object's own table. Each represented setter advances only that object's retained table generation and restamps its known current stores. Actual method entries retain the object allocation separately from the class body context; captured prefixes keep their earlier generation, and unknown configurations or workers withdraw the view. Source bodies remain deferred, and own-object activation does not borrow class instance variable inventory.

## Scope

Current Compiler/Core implementation contract for source analysis with an explicit C Tcl 8.6, 9.0 or 9.1 policy, a current bounded source-created instance of the registered default factory, an unchanged ordinary object-definition worker, and a static script containing only exact three-operand method setters. Counted opaque method names and readonly anonymous object targets are retained independently. Inline/options/wrappers, arbitrary immediate definition scripts, custom or configurable factories, unknown prior methods, own-object variable inventory, foreign-source native allocation, compiler preparation and actual guest execution are excluded. The named Rust selectors are validation obligations; no passing test result is claimed.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust source transfer and provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust source transfer and provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust source transfer and provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust source transfer and provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust source transfer and provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This is a Rust source transfer and provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This is a Rust source transfer and provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Exact evidence

- `c-source` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_own_method_source/anchors.json](../../../../rust/tcl-registry/tests/data/native_tcloo_own_method_source/anchors.json). SHA-256 `1ec16579b42c78b8dc2012d2b1cd8b6ed57901487f6aabdef55c2949f333d512`. JSON pointer `/source_anchors`. Retained complete function snippets and full original C source file hashes for the setter, procedure method creation, own object table replacement and known procedure deletion boundaries.

## Source inspection

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source association; full source digest is authoritative`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 1548–1592. Full-source SHA-256 `56fb36d62f1ec51cca4cc5177bb90dc57986021206ea9700454ff67db7905e81`; snippet SHA-256 `149f61556901b38088f841f849e86b9e55a4b7d2730a1fa0ccd39871531f1ca4`; retained evidence `c-source`.

```text
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

```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `TclOOMakeProcInstanceMethod`, lines 430–523. Full-source SHA-256 `c9212d902416d74246b9c6283e17b094c749171b65c86213b7b0af590da7a728`; snippet SHA-256 `fedc40da244048f038d3711262486ee7da96b47622c70e68983d0c599a278fd2`; retained evidence `c-source`.

```text
TclOOMakeProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which _must not_ be
				 * NULL. */
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

    if (TclCreateProc(interp, NULL, TclGetString(nameObj), argsObj, bodyObj,
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
	    context.data.eval.path = NULL;
	}
    }

    return Tcl_NewInstanceMethod(interp, (Tcl_Object) oPtr, nameObj, flags,
	    typePtr, clientData);
}

```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod`, lines 136–192. Full-source SHA-256 `c9212d902416d74246b9c6283e17b094c749171b65c86213b7b0af590da7a728`; snippet SHA-256 `e53a1b9f6d25737a47b657a664438d3d48b31884a00e540c1ab0352c0eeab381`; retained evidence `c-source`.

```text
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

```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `DeleteProcedureMethod`, lines 1227–1235. Full-source SHA-256 `c9212d902416d74246b9c6283e17b094c749171b65c86213b7b0af590da7a728`; snippet SHA-256 `18171600f0682d3b789cc7439c9acea4b02f9afa5aa482c81bb5a737b5adb859`; retained evidence `c-source`.

```text
DeleteProcedureMethod(
    void *clientData)
{
    ProcedureMethod *pmPtr = (ProcedureMethod *)clientData;

    if (pmPtr->refCount-- <= 1) {
	DeleteProcedureMethodRecord(pmPtr);
    }
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source association; full source digest is authoritative`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 2277–2361. Full-source SHA-256 `1eb881c851f09719a9cb932c5ceb36034b96569affda2b2f870652a0bf29426d`; snippet SHA-256 `ca528ef582289d9f8483cc4767b04c7b0b97395acfa1a112abf3a591d5c9853f`; retained evidence `c-source`.

```text
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
	if (TclOONewProcMethod(interp, oPtr->classPtr, isPublic, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `TclOOMakeProcInstanceMethod`, lines 615–651. Full-source SHA-256 `ad958b2a136ed614f53f2c1fc132a844738d4ff24298e098ce3bc5986525447c`; snippet SHA-256 `0d12907dc5ed11638a5557d82bd266c58a2564a8d010d240544984b72e1b10c3`; retained evidence `c-source`.

```text
TclOOMakeProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which _must not_ be
				 * NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which _must not_ be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which _must not_ be
				 * NULL. */
    const Tcl_MethodType *typePtr,
				/* The type of the method to create. */
    void *clientData,		/* The per-method type-specific data. */
    Proc **procPtrPtr)		/* A pointer to the variable in which to write
				 * the procedure record reference. Presumably
				 * inside the structure indicated by the
				 * pointer in clientData. */
{
    Interp *iPtr = (Interp *) interp;
    Proc *procPtr;

    if (typePtr->version > TCL_OO_METHOD_VERSION_1) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"TclOOMakeProcInstanceMethod", "TCL_OO_METHOD_VERSION_1");
    }
    if (TclCreateProc(interp, NULL, TclGetString(nameObj), argsObj, bodyObj,
	    procPtrPtr) != TCL_OK) {
	return NULL;
    }
    procPtr = *procPtrPtr;
    procPtr->cmdPtr = NULL;

    InitCmdFrame(iPtr, procPtr);

    return TclNewInstanceMethod(interp, (Tcl_Object) oPtr, nameObj, flags,
	    typePtr, clientData);
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod`, lines 199–220. Full-source SHA-256 `ad958b2a136ed614f53f2c1fc132a844738d4ff24298e098ce3bc5986525447c`; snippet SHA-256 `8ec2e9e7b1ee9765dcb56d1a99704765478cf65aeec0da73e34c4bd2639b1280`; retained evidence `c-source`.

```text
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

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `DeleteProcedureMethod`, lines 1401–1409. Full-source SHA-256 `ad958b2a136ed614f53f2c1fc132a844738d4ff24298e098ce3bc5986525447c`; snippet SHA-256 `bb88abeac6295b42cb4a9dad06019c03d53e4ccb00807898281b8fd1afb7a6fe`; retained evidence `c-source`.

```text
DeleteProcedureMethod(
    void *clientData)
{
    ProcedureMethod *pmPtr = (ProcedureMethod *) clientData;

    if (pmPtr->refCount-- <= 1) {
	DeleteProcedureMethodRecord(pmPtr);
    }
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source association; full source digest is authoritative`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 2237–2318. Full-source SHA-256 `bd254581a59362769c56269dfb009dd9ed5e5b9bcf00ac67b5738e97e50b5106`; snippet SHA-256 `a10f879e9ab4d7269c1f889ab63cb99cf586d0d07f8f5423c4b69c4d36c0e57b`; retained evidence `c-source`.

```text
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
    } else {
	if (TclOONewProcMethod(interp, oPtr->classPtr, flags, objv[1],
		objv[objc - 2], objv[objc - 1], NULL) == NULL) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `TclOOMakeProcInstanceMethod`, lines 622–658. Full-source SHA-256 `1424048c7a16891c68535ae215359d440f28654a72f72eefadc9b5aada54f1d1`; snippet SHA-256 `c4294003964f6e418f9425a0e88ebc734dcf1349a6a39d149270ca240e1ea125`; retained evidence `c-source`.

```text
TclOOMakeProcInstanceMethod(
    Tcl_Interp *interp,		/* The interpreter containing the object. */
    Object *oPtr,		/* The object to modify. */
    int flags,			/* Whether this is a public method. */
    Tcl_Obj *nameObj,		/* The name of the method, which _must not_ be
				 * NULL. */
    Tcl_Obj *argsObj,		/* The formal argument list for the method,
				 * which _must not_ be NULL. */
    Tcl_Obj *bodyObj,		/* The body of the method, which _must not_ be
				 * NULL. */
    const Tcl_MethodType *typePtr,
				/* The type of the method to create. */
    void *clientData,		/* The per-method type-specific data. */
    Proc **procPtrPtr)		/* A pointer to the variable in which to write
				 * the procedure record reference. Presumably
				 * inside the structure indicated by the
				 * pointer in clientData. */
{
    Interp *iPtr = (Interp *) interp;
    Proc *procPtr;

    if (typePtr->version > TCL_OO_METHOD_VERSION_1) {
	Tcl_Panic("%s: Wrong version in typePtr->version, should be %s",
		"TclOOMakeProcInstanceMethod", "TCL_OO_METHOD_VERSION_1");
    }
    if (TclCreateProc(interp, NULL, TclGetString(nameObj), argsObj, bodyObj,
	    procPtrPtr) != TCL_OK) {
	return NULL;
    }
    procPtr = *procPtrPtr;
    procPtr->cmdPtr = NULL;

    InitCmdFrame(iPtr, procPtr);

    return TclNewInstanceMethod(interp, (Tcl_Object) oPtr, nameObj, flags,
	    (const Tcl_MethodType2 *)typePtr, clientData);
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod`, lines 203–224. Full-source SHA-256 `1424048c7a16891c68535ae215359d440f28654a72f72eefadc9b5aada54f1d1`; snippet SHA-256 `40a8bfd03b8fd168229f697f0e19abfafd4fc3794080b91cb4733cbbaf5ffcda`; retained evidence `c-source`.

```text
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

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `DeleteProcedureMethod`, lines 1464–1472. Full-source SHA-256 `1424048c7a16891c68535ae215359d440f28654a72f72eefadc9b5aada54f1d1`; snippet SHA-256 `bb88abeac6295b42cb4a9dad06019c03d53e4ccb00807898281b8fd1afb7a6fe`; retained evidence `c-source`.

```text
DeleteProcedureMethod(
    void *clientData)
{
    ProcedureMethod *pmPtr = (ProcedureMethod *) clientData;

    if (pmPtr->refCount-- <= 1) {
	DeleteProcedureMethodRecord(pmPtr);
    }
}

```


## Consumer bindings

- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `native_deferred_method_setter`: Selects the intrinsic exact setter storage contract independently of metadata and construction effects.
- [rust/tcl-compiler/src/command_binding/own_object_configuration.rs](../../../../rust/tcl-compiler/src/command_binding/own_object_configuration.rs), `walk_closed_own_object_method_configuration`: Validates original operands and actual worker/object ownership, then composes represented intrinsic transfers through SourceOutcomes.
- [rust/tcl-compiler/src/command_binding/receiver_self.rs](../../../../rust/tcl-compiler/src/command_binding/receiver_self.rs), `retained_instance_method_entry`: Selects the current object own table before the selected class instance table; no class-object table donation.
- [rust/tcl-lsp-core/src/receiver_identity.rs](../../../../rust/tcl-lsp-core/src/receiver_identity.rs), `original_method`: Joins the exact own configuration allocation and canonical source declaration independently of class reporting maps.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `definer::tests::original_deferred_method_setter_keeps_native_storage_and_body_execution_separate` (linked): Checks the actual allocation/worker/native-field/current-table axis stated in this contract, with unsupported cases remaining terminal.
- [rust/tcl-compiler/src/command_binding/own_object_configuration.rs](../../../../rust/tcl-compiler/src/command_binding/own_object_configuration.rs), `command_binding::own_object_configuration::tests::original_own_object_entries_keep_allocation_and_counted_names_separate` (linked): Checks the actual allocation/worker/native-field/current-table axis stated in this contract, with unsupported cases remaining terminal.
- [rust/tcl-compiler/src/command_binding/own_object_configuration.rs](../../../../rust/tcl-compiler/src/command_binding/own_object_configuration.rs), `command_binding::own_object_configuration::tests::original_own_object_readonly_target_retains_no_generated_name` (linked): Checks the actual allocation/worker/native-field/current-table axis stated in this contract, with unsupported cases remaining terminal.
- [rust/tcl-compiler/src/command_binding/own_object_configuration.rs](../../../../rust/tcl-compiler/src/command_binding/own_object_configuration.rs), `command_binding::own_object_configuration::tests::original_own_object_unknown_or_changed_workers_withdraw_entries` (linked): Checks the actual allocation/worker/native-field/current-table axis stated in this contract, with unsupported cases remaining terminal.
- [rust/tcl-compiler/src/command_binding/own_object_configuration.rs](../../../../rust/tcl-compiler/src/command_binding/own_object_configuration.rs), `command_binding::own_object_configuration::tests::original_own_object_body_does_not_borrow_class_instance_variables` (linked): Checks the actual allocation/worker/native-field/current-table axis stated in this contract, with unsupported cases remaining terminal.
- [rust/tcl-compiler/src/command_binding/own_object_configuration.rs](../../../../rust/tcl-compiler/src/command_binding/own_object_configuration.rs), `command_binding::own_object_configuration::tests::original_own_object_replacement_retires_prior_captured_prefix` (linked): Checks the actual allocation/worker/native-field/current-table axis stated in this contract, with unsupported cases remaining terminal.
- [rust/tcl-lsp-core/src/receiver_identity.rs](../../../../rust/tcl-lsp-core/src/receiver_identity.rs), `receiver_identity::tests::original_own_object_navigation_joins_the_actual_configuration_allocation` (linked): Checks the actual allocation/worker/native-field/current-table axis stated in this contract, with unsupported cases remaining terminal.

A named test is a coverage binding, not a claim that it executed.

## Replay

Run the named selectors on the coherent compiled workspace before claiming a passing result. C source inspection is separate from native observations and native compilation/object/body capabilities.
