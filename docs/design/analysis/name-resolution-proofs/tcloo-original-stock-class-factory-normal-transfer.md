# naming.tcloo.original-stock-class-factory-normal-transfer

Kind: `implementation-contract`

## Problem statement

A stock class factory stores native method declarations and may create a configurable support relation before the next source command. A declaration inventory alone does not establish successful creation, and an unknown completion route discards later independently valid receiver publications.

## Question

Does a fresh quiet stock class factory retain normal completion only after independently validating its intrinsic constructor, complete original definition script, selected workers and resulting allocation?

## Conclusion

The source driver captures the current stock factory and any required support allocation before transfer. It requires exact static original operands, the selected release and manufacturer, a fresh valid destination in an existing namespace, and a complete definition script whose actual workers and stored formals are independently valid. After the existing transfer it checks the same source/site/configuration, current factory, class receipt and exact new allocation/publication. Only that complete pre/post transfer preserves the shared normal-handler certificate.

## Scope

Rust source-analysis contract for C Tcl 8.6/9.0/9.1 stock ordinary factories and C Tcl 9.0/9.1 stock configurable factories with fresh public slots, known unobserved source entry and represented static declaration scripts. Captured or dynamic operands, existing destination cleanup, unknown callbacks, custom or mutated factories, supplied Native entries and actual guest execution are excluded. Stored method and lifecycle bodies are not executed during registration. The selector is a validation obligation without an attached passing execution receipt.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No guest execution of this Rust implementation question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No guest execution of this Rust implementation question is claimed.

## Exact evidence

- `c-source` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/factory-transfer-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/factory-transfer-source-anchors.json). SHA-256 `53c976b6bf08cd637639877ddd0a2d0a99155a555765ced00e46b9f624500ee1`. JSON pointer `/source_anchors`. Complete release-specific stock create/constructor/post-constructor function excerpts with full original file digests. This is source inspection, not guest execution.

## Source inspection

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; full file digest identifies the source bytes`, `generic/tclOOBasic.c`, function `TclOO_Class_Create`, lines 153–205. Full-source SHA-256 `1a8f47994517980c374d71d6914412ec3348cacf5e272b6d87c2d973626cdc98`; snippet SHA-256 `99e5f9fd0bd9f8516122a699c44ef3d94fe975a6856a6e7f1a097cd0356025ea`; retained evidence `c-source`.

```text
TclOO_Class_Create(
    ClientData clientData,	/* Ignored. */
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    const char *objName;
    int len;

    /*
     * Sanity check; should not be possible to invoke this method on a
     * non-class.
     */

    if (oPtr->classPtr == NULL) {
	Tcl_Obj *cmdnameObj = TclOOObjectName(interp, oPtr);

	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"object \"%s\" is not a class", TclGetString(cmdnameObj)));
	Tcl_SetErrorCode(interp, "TCL", "OO", "INSTANTIATE_NONCLASS", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Check we have the right number of (sensible) arguments.
     */

    if (objc - Tcl_ObjectContextSkippedArgs(context) < 1) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"objectName ?arg ...?");
	return TCL_ERROR;
    }
    objName = Tcl_GetStringFromObj(
	    objv[Tcl_ObjectContextSkippedArgs(context)], &len);
    if (len == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"object name must not be empty", -1));
	Tcl_SetErrorCode(interp, "TCL", "OO", "EMPTY_NAME", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Make the object and return its name.
     */

    return TclNRNewObjectInstance(interp, (Tcl_Class) oPtr->classPtr,
	    objName, NULL, objc, objv,
	    Tcl_ObjectContextSkippedArgs(context)+1,
	    AddConstructionFinalizer(interp));
}

```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; full file digest identifies the source bytes`, `generic/tclOOBasic.c`, function `TclOO_Class_Constructor`, lines 80–124. Full-source SHA-256 `1a8f47994517980c374d71d6914412ec3348cacf5e272b6d87c2d973626cdc98`; snippet SHA-256 `0bbcd8ff3a0e02589c3361b134e8e07934f8ebfeb152fe5a287cdb2590ad50d1`; retained evidence `c-source`.

```text
TclOO_Class_Constructor(
    ClientData clientData,
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Obj **invoke;

    if (objc-1 > Tcl_ObjectContextSkippedArgs(context)) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"?definitionScript?");
	return TCL_ERROR;
    } else if (objc == Tcl_ObjectContextSkippedArgs(context)) {
	return TCL_OK;
    }

    /*
     * Delegate to [oo::define] to do the work.
     */

    invoke = ckalloc(3 * sizeof(Tcl_Obj *));
    invoke[0] = oPtr->fPtr->defineName;
    invoke[1] = TclOOObjectName(interp, oPtr);
    invoke[2] = objv[objc-1];

    /*
     * Must add references or errors in configuration script will cause
     * trouble.
     */

    Tcl_IncrRefCount(invoke[0]);
    Tcl_IncrRefCount(invoke[1]);
    Tcl_IncrRefCount(invoke[2]);
    TclNRAddCallback(interp, DecrRefsPostClassConstructor,
	    invoke, NULL, NULL, NULL);

    /*
     * Tricky point: do not want the extra reported level in the Tcl stack
     * trace, so use TCL_EVAL_NOERR.
     */

    return TclNREvalObjv(interp, 3, invoke, TCL_EVAL_NOERR, NULL);
}

```

tcl8.6 8.6.18, revision `Retained Tcl 8.6.18 release source; full file digest identifies the source bytes`, `generic/tclOOBasic.c`, function `DecrRefsPostClassConstructor`, lines 127–139. Full-source SHA-256 `1a8f47994517980c374d71d6914412ec3348cacf5e272b6d87c2d973626cdc98`; snippet SHA-256 `8682a62f8de5e79d134109a6b99e30e50303861379b04c9d8ede7bca1f9c9c33`; retained evidence `c-source`.

```text
DecrRefsPostClassConstructor(
    ClientData data[],
    Tcl_Interp *interp,
    int result)
{
    Tcl_Obj **invoke = data[0];

    TclDecrRefCount(invoke[0]);
    TclDecrRefCount(invoke[1]);
    TclDecrRefCount(invoke[2]);
    ckfree(invoke);
    return result;
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; full file digest identifies the source bytes`, `generic/tclOOBasic.c`, function `TclOO_Class_Create`, lines 312–364. Full-source SHA-256 `5782e57ef2753b052d904088cf26f43ceb96ab27afb7dbe99438e24a69e68e68`; snippet SHA-256 `b69c78f5b99ba9872f968746fd4a207cc3cb069e062579185febf421aca46f5f`; retained evidence `c-source`.

```text
TclOO_Class_Create(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    const char *objName;
    Tcl_Size len;

    /*
     * Sanity check; should not be possible to invoke this method on a
     * non-class.
     */

    if (oPtr->classPtr == NULL) {
	Tcl_Obj *cmdnameObj = TclOOObjectName(interp, oPtr);

	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"object \"%s\" is not a class", TclGetString(cmdnameObj)));
	OO_ERROR(interp, INSTANTIATE_NONCLASS);
	return TCL_ERROR;
    }

    /*
     * Check we have the right number of (sensible) arguments.
     */

    if (objc < 1 + Tcl_ObjectContextSkippedArgs(context)) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"objectName ?arg ...?");
	return TCL_ERROR;
    }
    objName = Tcl_GetStringFromObj(
	    objv[Tcl_ObjectContextSkippedArgs(context)], &len);
    if (len == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"object name must not be empty", TCL_AUTO_LENGTH));
	OO_ERROR(interp, EMPTY_NAME);
	return TCL_ERROR;
    }

    /*
     * Make the object and return its name.
     */

    return TclNRNewObjectInstance(interp, (Tcl_Class) oPtr->classPtr,
	    objName, NULL, objc, objv,
	    Tcl_ObjectContextSkippedArgs(context)+1,
	    AddConstructionFinalizer(interp));
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; full file digest identifies the source bytes`, `generic/tclOOBasic.c`, function `TclOO_Class_Constructor`, lines 204–272. Full-source SHA-256 `5782e57ef2753b052d904088cf26f43ceb96ab27afb7dbe99438e24a69e68e68`; snippet SHA-256 `0cc16608cdf1925101ebe74e153f58685dcbbbc70ebdf40b8c73a57afed11209`; retained evidence `c-source`.

```text
TclOO_Class_Constructor(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    int objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    size_t skip = Tcl_ObjectContextSkippedArgs(context);
    Tcl_Obj **invoke, *delegateName;

    if ((size_t) objc > skip + 1) {
	Tcl_WrongNumArgs(interp, skip, objv,
		"?definitionScript?");
	return TCL_ERROR;
    }

    /*
     * Make the class definition delegate. This is special; it doesn't reenter
     * here (and the class definition delegate doesn't run any constructors).
     *
     * This needs to be done before consideration of whether to pass the script
     * argument to [oo::define]. [Bug 680503]
     */

    delegateName = Tcl_ObjPrintf("%s:: oo ::delegate",
	    oPtr->namespacePtr->fullName);
    Tcl_IncrRefCount(delegateName);
    Tcl_NewObjectInstance(interp, (Tcl_Class) oPtr->fPtr->classCls,
	    TclGetString(delegateName), NULL, TCL_INDEX_NONE, NULL, 0);

    /*
     * If there's nothing else to do, we're done.
     */

    if ((size_t) objc == skip) {
	Tcl_InterpState saved = Tcl_SaveInterpState(interp, TCL_OK);
	MixinClassDelegates(interp, oPtr, delegateName);
	Tcl_DecrRefCount(delegateName);
	return Tcl_RestoreInterpState(interp, saved);
    }

    /*
     * Delegate to [oo::define] to do the work.
     */

    invoke = (Tcl_Obj **) TclStackAlloc(interp, 3 * sizeof(Tcl_Obj *));
    invoke[0] = oPtr->fPtr->defineName;
    invoke[1] = TclOOObjectName(interp, oPtr);
    invoke[2] = objv[objc - 1];

    /*
     * Must add references or errors in configuration script will cause
     * trouble.
     */

    Tcl_IncrRefCount(invoke[0]);
    Tcl_IncrRefCount(invoke[1]);
    Tcl_IncrRefCount(invoke[2]);
    TclNRAddCallback(interp, PostClassConstructor,
	    invoke, oPtr, delegateName, NULL);

    /*
     * Tricky point: do not want the extra reported level in the Tcl stack
     * trace, so use TCL_EVAL_NOERR.
     */

    return TclNREvalObjv(interp, 3, invoke, TCL_EVAL_NOERR, NULL);
}

```

tcl9.0 9.0.4, revision `Retained Tcl 9.0.4 release source; full file digest identifies the source bytes`, `generic/tclOOBasic.c`, function `PostClassConstructor`, lines 279–298. Full-source SHA-256 `5782e57ef2753b052d904088cf26f43ceb96ab27afb7dbe99438e24a69e68e68`; snippet SHA-256 `15fb3b57a11e456c4c9f6744fc532a06728bb140d8250425cd1c0b7892fed8a9`; retained evidence `c-source`.

```text
PostClassConstructor(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    Tcl_Obj **invoke = (Tcl_Obj **) data[0];
    Object *oPtr = (Object *) data[1];
    Tcl_Obj *delegateName = (Tcl_Obj *) data[2];
    Tcl_InterpState saved;

    TclDecrRefCount(invoke[0]);
    TclDecrRefCount(invoke[1]);
    TclDecrRefCount(invoke[2]);
    TclStackFree(interp, invoke);

    saved = Tcl_SaveInterpState(interp, result);
    MixinClassDelegates(interp, oPtr, delegateName);
    Tcl_DecrRefCount(delegateName);
    return Tcl_RestoreInterpState(interp, saved);
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; full file digest identifies the source bytes`, `generic/tclOOBasic.c`, function `TclOO_Class_Create`, lines 398–444. Full-source SHA-256 `d8b40b5adf0e96bbcac34fa6a7035dbc4ac431218c28924766c4c13532aff158`; snippet SHA-256 `1422da8d6b4068c37acc67fbf55aec1a85d2e3fc3b7a39626bcc241e1b6f1d4a`; retained evidence `c-source`.

```text
TclOO_Class_Create(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter in which to create the object;
				 * also used for error reporting. */
    Tcl_ObjectContext context,	/* The object/call context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Tcl_Class cls = ContextClass(interp, context);
    const char *objName;
    Tcl_Size len;

    /*
     * Sanity check; should not be possible to invoke this method on a
     * non-class.
     */

    if (!cls) {
	return TCL_ERROR;
    }

    /*
     * Check we have the right number of (sensible) arguments.
     */

    if (objc < 1 + Tcl_ObjectContextSkippedArgs(context)) {
	Tcl_WrongNumArgs(interp, Tcl_ObjectContextSkippedArgs(context), objv,
		"objectName ?arg ...?");
	return TCL_ERROR;
    }
    objName = Tcl_GetStringFromObj(
	    objv[Tcl_ObjectContextSkippedArgs(context)], &len);
    if (len == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"object name must not be empty", TCL_AUTO_LENGTH));
	OO_ERROR(interp, EMPTY_NAME);
	return TCL_ERROR;
    }

    /*
     * Make the object and return its name.
     */

    return TclNRNewObjectInstance(interp, cls, objName, NULL, objc, objv,
	    Tcl_ObjectContextSkippedArgs(context)+1,
	    AddConstructionFinalizer(interp));
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; full file digest identifies the source bytes`, `generic/tclOOBasic.c`, function `TclOO_Class_Constructor`, lines 248–313. Full-source SHA-256 `d8b40b5adf0e96bbcac34fa6a7035dbc4ac431218c28924766c4c13532aff158`; snippet SHA-256 `07566792891a1d7d3e089a4011a1c48f9c3b7af69924bbbec9d10a407bcb3f69`; retained evidence `c-source`.

```text
TclOO_Class_Constructor(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_ObjectContext context,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    if (objc > skip + 1) {
	Tcl_WrongNumArgs(interp, skip, objv,
		"?definitionScript?");
	return TCL_ERROR;
    }

    /*
     * Make the class definition delegate. This is special; it doesn't reenter
     * here (and the class definition delegate doesn't run any constructors).
     *
     * This needs to be done before consideration of whether to pass the script
     * argument to [oo::define]. [Bug 680503]
     */

    Tcl_Obj *delegateName = TclOOGetClassDelegateName(oPtr);
    Tcl_IncrRefCount(delegateName);
    Tcl_NewObjectInstance(interp, (Tcl_Class) oPtr->fPtr->classCls,
	    TclGetString(delegateName), NULL, TCL_INDEX_NONE, NULL, 0);

    /*
     * If there's nothing else to do, we're done.
     */

    if (objc == skip) {
	Tcl_InterpState saved = Tcl_SaveInterpState(interp, TCL_OK);
	MixinClassDelegates(interp, oPtr, delegateName);
	Tcl_DecrRefCount(delegateName);
	return Tcl_RestoreInterpState(interp, saved);
    }

    /*
     * Delegate to [oo::define] to do the work.
     */

    Tcl_Obj **invoke = (Tcl_Obj **) TclStackAlloc(interp, 3 * sizeof(Tcl_Obj *));
    invoke[0] = oPtr->fPtr->defineName;
    invoke[1] = TclOOObjectName(interp, oPtr);
    invoke[2] = objv[objc - 1];

    /*
     * Must add references or errors in configuration script will cause
     * trouble.
     */

    Tcl_IncrRefCount(invoke[0]);
    Tcl_IncrRefCount(invoke[1]);
    Tcl_IncrRefCount(invoke[2]);
    TclNRAddCallback(interp, PostClassConstructor,
	    invoke, oPtr, delegateName, NULL);

    /*
     * Tricky point: do not want the extra reported level in the Tcl stack
     * trace, so use TCL_EVAL_NOERR.
     */

    return TclNREvalObjv(interp, 3, invoke, TCL_EVAL_NOERR, NULL);
}

```

tcl9.1 9.1.0, revision `Retained Tcl 9.1.0 release source; full file digest identifies the source bytes`, `generic/tclOOBasic.c`, function `PostClassConstructor`, lines 326–344. Full-source SHA-256 `d8b40b5adf0e96bbcac34fa6a7035dbc4ac431218c28924766c4c13532aff158`; snippet SHA-256 `aa27bb613bdb340f6dca9b34a038144a57277531b8b9067b19ee4a676be0ef06`; retained evidence `c-source`.

```text
PostClassConstructor(
    void *data[],
    Tcl_Interp *interp,
    int result)
{
    Tcl_Obj **invoke = (Tcl_Obj **) data[0];
    Object *oPtr = (Object *) data[1];
    Tcl_Obj *delegateName = (Tcl_Obj *) data[2];

    TclDecrRefCount(invoke[0]);
    TclDecrRefCount(invoke[1]);
    TclDecrRefCount(invoke[2]);
    TclStackFree(interp, invoke);

    Tcl_InterpState saved = Tcl_SaveInterpState(interp, result);
    MixinClassDelegates(interp, oPtr, delegateName);
    Tcl_DecrRefCount(delegateName);
    return Tcl_RestoreInterpState(interp, saved);
}

```


## Consumer bindings

- [rust/tcl-compiler/src/command_binding/original_class_factory_transfer.rs](../../../../rust/tcl-compiler/src/command_binding/original_class_factory_transfer.rs), `OriginalClassFactoryTransfer::capture`: Independent intrinsic creation premises: exact original argv, selected stock constructor and current workers, fresh valid slot, complete stored declarations.
- [rust/tcl-compiler/src/command_binding/original_class_factory_transfer.rs](../../../../rust/tcl-compiler/src/command_binding/original_class_factory_transfer.rs), `OriginalClassFactoryTransfer::completed`: Same actual operation, resulting allocation and immutable class/publication correspondence after the canonical transfer.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `SourceCommandBindings::prepare_native_definition`: Compose complete factory transfer with the shared SourceOutcomes normal-handler certificate; declaration metadata cannot issue it.
- [rust/tcl-compiler/src/command_binding/deferred_method.rs](../../../../rust/tcl-compiler/src/command_binding/deferred_method.rs), `closed_definition_member`: Existing shared selected-worker, native formal, property and inheritance declaration closure.
- [rust/tcl-compiler/src/command_binding/original_class_factory_transfer.rs](../../../../rust/tcl-compiler/src/command_binding/original_class_factory_transfer.rs), `command_binding::original_class_factory_transfer::tests::original_stock_class_factory_normal_requires_fresh_valid_intrinsic_definition` (linked): Fresh static ordinary and configurable declaration scripts require complete Normal; occupied/missing destinations, invalid native formals/variables and arbitrary immediate scripts cannot issue that receipt.

A named test is a coverage binding, not a claim that it executed.

## Replay

No passing Rust or guest execution receipt is attached. Full command-vector/source ownership, current factory/support allocations, observer closure, native declaration validity and exact post-transfer allocation remain independent required checks.
