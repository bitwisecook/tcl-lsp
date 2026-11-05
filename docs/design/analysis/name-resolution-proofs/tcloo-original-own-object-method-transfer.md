# naming.tcloo.original-own-object-method-transfer

Kind: `implementation-contract`

## Problem statement

An own-object method can replace a class-provided method on one allocated instance while other instances continue to use the class entry. Joining the source declaration through the class object table, a reporting object name or a stale captured prefix can select a different implementation and can incorrectly borrow the class instance variable resolver.

## Question

Does a closed original own-object method setter retain the actual object allocation, selected worker and native-valid original fields, and do later method consumers use only that object's current own table?

## Conclusion

Ordinary three-operand deferred method setters compose selected intrinsic transfers into the allocated object's own table. Each represented setter advances only that object's retained table generation and restamps its known current stores. Actual method entries retain the object allocation separately from the class body context; captured prefixes keep their earlier generation, and unknown configurations or workers withdraw the view. Source bodies remain deferred, and own-object activation does not borrow class instance variable inventory.

## Scope

Current Compiler/Core implementation contract for source analysis with an explicit C Tcl 8.6, 9.0 or 9.1 policy, a current bounded source-created instance of the registered default factory, an unchanged ordinary object-definition worker, and either a static script containing only exact three-operand method setters or one such setter written inline whose literal source member words re-parse unchanged as that script. Counted opaque method names and readonly anonymous object targets are retained independently. Options/wrappers, substituted or expanded inline member words, arbitrary immediate definition scripts, custom or configurable factories, unknown prior methods, own-object variable inventory, foreign-source native allocation, compiler preparation and actual guest execution are excluded. The named Rust selectors are validation obligations; no passing test result is claimed.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a Rust source transfer and provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Source inspection

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source association; full source digest is authoritative`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 1548–1592. Retained reference `c-source`.

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

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `TclOOMakeProcInstanceMethod`, lines 430–523. Retained reference `c-source`.

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

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod`, lines 136–192. Retained reference `c-source`.

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

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `DeleteProcedureMethod`, lines 1227–1235. Retained reference `c-source`.

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

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source association; full source digest is authoritative`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 2277–2361. Retained reference `c-source`.

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

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `TclOOMakeProcInstanceMethod`, lines 615–651. Retained reference `c-source`.

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

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod`, lines 199–220. Retained reference `c-source`.

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

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `DeleteProcedureMethod`, lines 1401–1409. Retained reference `c-source`.

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

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source association; full source digest is authoritative`, `generic/tclOODefineCmds.c`, function `TclOODefineMethodObjCmd`, lines 2237–2318. Retained reference `c-source`.

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

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `TclOOMakeProcInstanceMethod`, lines 622–658. Retained reference `c-source`.

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

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `Tcl_NewInstanceMethod`, lines 203–224. Retained reference `c-source`.

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

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source association; full source digest is authoritative`, `generic/tclOOMethod.c`, function `DeleteProcedureMethod`, lines 1464–1472. Retained reference `c-source`.

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
- [rust/tcl-compiler/src/command_binding/own_object_configuration.rs](../../../../rust/tcl-compiler/src/command_binding/own_object_configuration.rs), `command_binding::own_object_configuration::tests::original_own_object_inline_member_installs_the_script_layout_entry` (linked): Checks the actual allocation/worker/native-field/current-table axis stated in this contract, with unsupported cases remaining terminal.
- [rust/tcl-lsp-core/src/receiver_identity.rs](../../../../rust/tcl-lsp-core/src/receiver_identity.rs), `receiver_identity::tests::original_own_object_navigation_joins_the_actual_configuration_allocation` (linked): Checks the actual allocation/worker/native-field/current-table axis stated in this contract, with unsupported cases remaining terminal.
- [rust/tcl-compiler/src/command_binding/own_object_configuration.rs](../../../../rust/tcl-compiler/src/command_binding/own_object_configuration.rs), `command_binding::own_object_configuration::original_configuration_body_purpose_tests::original_own_object_configuration_keeps_body_bom_and_parent_policy_separate` (linked): Finite source-defined control: original own object configuration keeps body bom and parent policy separate. Actual original stock own-object transfer, whole prologue versus BOM-prefixed worker body, unchanged parent full config/source and foreign source refusal. No physical method-entry authority. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

## Exact original-source Compiler software outcomes

The table reports only admitted selectors attached to this question by exact comments in the retained original test bodies. Each result is limited to its named source snapshot, owned Compiler image and selector invocation. No other source/image, unselected assertion, external-provider observation or performance conclusion follows.

| Original operation | Passed | Failed | Incomplete |
| --- | ---: | ---: | ---: |
| `integration-compiler-focused-consumer-tests344` | 1 | 0 | 0 |

Exact original-source body hashes, receipts, streams, inventories and owned images preserve the per-selector events and independent producer/admission limits. The existing provider answers retain their own experimental scope.

## Exact original-source347 software outcomes

These results belong only to the retained source347 body/question pairs and their owning Compiler or Core image. They do not establish an outcome for current source, unselected controls or external providers.

| Original operation | Passed | Failed |
| --- | ---: | ---: |
| `integration-compiler-focused-consumer-tests347` | 1 | 0 |

`Exact original body hashes, producer/list/admission receipts, raw logs and owned images` retain the finite per-selector results and independent failed Runtime producer.

## Evidence

- [rust/tcl-registry/tests/data/native_tcloo_own_method_source/anchors.json](../../../../rust/tcl-registry/tests/data/native_tcloo_own_method_source/anchors.json)
