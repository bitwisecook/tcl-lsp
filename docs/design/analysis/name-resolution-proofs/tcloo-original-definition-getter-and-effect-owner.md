# naming.tcloo.original-definition-getter-and-effect-owner

Kind: `implementation-contract`

## Problem statement

An original definition operand getter or retired method body can withdraw its selected Object/Class target, change operation currency or report the first Host refusal. Continuing with a fresh spelling lookup or applying later flags can act on the wrong target or erase a reached earlier effect.

## Question

Does the shared definition continuation retain its exact selected Object/Class target across original operand getters and outside-borrow retirement, preserve first typed refusal and already reached effects, and stop later getters/publication after withdrawal? Which C9.1 sequential export stages are inspected source facts rather than externally executed callback parity?

## Conclusion

The current source/API owner retains the selected definition target and object creation stamp, checks it at the described getter/retirement boundaries, and preserves the original argument currency and first typed refusal. The retained original C9.1 source shows context selection, object-hash String access before method flags, per-operand export/unexport effects and final epoch update. Six finite linked software controls are unrun on an attached matching owned image. All seven external execution statuses are not tested. No Native callback parity, Method record/name/key reference count, Class-facet allocation lifetime, frame entry, dynamic lookup or class-proof grant follows.

## Scope

Selected definition class operand, definition namespace operands, method deletion and visibility flag continuations, plus the exact Registry release schedule. Eight physical-LF C9.1 source joins are independently retained and byte-verified; source facts remain distinct from software Host/ABA/withdrawal controls and external execution. Runtime Object/Class target checks and flag sets do not reproduce the original C object-hash/Method storage roles. Compatibility C8.6/C9.0 schedules have no inspected chronology grant here. Class-facet lifecycle and introspection/report continuations require independent questions.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl8.4 not tested for this source/API question.

No original tcl8.4 process tests this definition target/getter/effect owner contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### tcl8.5

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl8.5 not tested for this source/API question.

No original tcl8.5 process tests this definition target/getter/effect owner contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### tcl8.6

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl8.6 not tested for this source/API question.

No original tcl8.6 process tests this definition target/getter/effect owner contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### tcl9.0

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl9.0 not tested for this source/API question.

No original tcl9.0 process tests this definition target/getter/effect owner contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### tcl9.1

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: tcl9.1 not tested for this source/API question.

No original tcl9.1 process tests this definition target/getter/effect owner contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### jim

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: jim not tested for this source/API question.

No original jim process tests this definition target/getter/effect owner contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

### bigip

Status: `not-tested`. Channel: Source/API inspection and authored software records only; no external worker input for this question. Version: bigip not tested for this source/API question.

No original bigip process tests this definition target/getter/effect owner contract. Retained source inspection and unrun authored controls establish neither Native callback outcomes nor physical context, frame, record identity or execution authority.

## Source inspection

tcl9.1 9.1.0, revision `Original C9.1 source tree joined by complete byte hash; source inspection only, no callback execution or binary/source equivalence receipt`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `GetOrCreateMethod and ExportMethod/UnexportMethod helpers`, lines 1228–1324. Retained reference `source-window-0`.

```text

// Make a blank method record or look up the existing one.
static inline Method *
GetOrCreateMethod(
    Tcl_HashTable *tablePtr,
    Tcl_Obj *namePtr,
    int *isNew)
{
    Tcl_HashEntry *hPtr = Tcl_CreateHashEntry(tablePtr, namePtr, isNew);
    if (*isNew) {
	Method *mPtr = (Method *) Tcl_Alloc(sizeof(Method));
	memset(mPtr, 0, sizeof(Method));
	mPtr->refCount = 1;
	mPtr->namePtr = namePtr;
	Tcl_IncrRefCount(namePtr);
	Tcl_SetHashValue(hPtr, mPtr);
	return mPtr;
    } else {
	return (Method *) Tcl_GetHashValue(hPtr);
    }
}

static bool
ExportMethod(
    Class *clsPtr,
    Tcl_Obj *namePtr)
{
    int isNew;
    Method *mPtr = GetOrCreateMethod(&clsPtr->classMethods, namePtr, &isNew);
    if (isNew || !(mPtr->flags & (PUBLIC_METHOD | PRIVATE_METHOD))) {
	mPtr->flags |= PUBLIC_METHOD;
	mPtr->flags &= ~TRUE_PRIVATE_METHOD;
	isNew = 1;
    }
    return isNew;
}

static bool
UnexportMethod(
    Class *clsPtr,
    Tcl_Obj *namePtr)
{
    int isNew;
    Method *mPtr = GetOrCreateMethod(&clsPtr->classMethods, namePtr, &isNew);
    if (isNew || mPtr->flags & (PUBLIC_METHOD | TRUE_PRIVATE_METHOD)) {
	mPtr->flags &= ~(PUBLIC_METHOD | TRUE_PRIVATE_METHOD);
	isNew = 1;
    }
    return isNew;
}

// Make the table of methods in the instance if it doesn't already exist.
static inline void
InitMethodTable(
    Object *oPtr)
{
    if (!oPtr->methodsPtr) {
	oPtr->methodsPtr = (Tcl_HashTable *) Tcl_Alloc(sizeof(Tcl_HashTable));
	Tcl_InitObjHashTable(oPtr->methodsPtr);
	oPtr->flags &= ~USE_CLASS_CACHE;
    }
}

static bool
ExportInstanceMethod(
    Object *oPtr,
    Tcl_Obj *namePtr)
{
    InitMethodTable(oPtr);

    int isNew;
    Method *mPtr = GetOrCreateMethod(oPtr->methodsPtr, namePtr, &isNew);
    if (isNew || !(mPtr->flags & (PUBLIC_METHOD | PRIVATE_METHOD))) {
	mPtr->flags |= PUBLIC_METHOD;
	mPtr->flags &= ~TRUE_PRIVATE_METHOD;
	isNew = 1;
    }
    return isNew;
}

static bool
UnexportInstanceMethod(
    Object *oPtr,
    Tcl_Obj *namePtr)
{
    InitMethodTable(oPtr);

    int isNew;
    Method *mPtr = GetOrCreateMethod(oPtr->methodsPtr, namePtr, &isNew);
    if (isNew || mPtr->flags & (PUBLIC_METHOD | TRUE_PRIVATE_METHOD)) {
	mPtr->flags &= ~(PUBLIC_METHOD | TRUE_PRIVATE_METHOD);
	isNew = 1;
    }
    return isNew;
}

bool

```

tcl9.1 9.1.0, revision `Original C9.1 source tree joined by complete byte hash; source inspection only, no callback execution or binary/source equivalence receipt`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOODefineClassObjCmd`, lines 1704–1795. Retained reference `source-window-1`.

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

```

tcl9.1 9.1.0, revision `Original C9.1 source tree joined by complete byte hash; source inspection only, no callback execution or binary/source equivalence receipt`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOODefineDefnNsObjCmd`, lines 1872–1940. Retained reference `source-window-2`.

```text

int
TclOODefineDefnNsObjCmd(
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
    Class *clsPtr = TclOOGetClassDefineCmdContext(interp);
    Tcl_Namespace *nsPtr;
    Tcl_Obj *nsNamePtr, **storagePtr;

    if (clsPtr == NULL) {
	return TCL_ERROR;
    } else if (clsPtr->thisPtr->flags & (ROOT_OBJECT | ROOT_CLASS)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"may not modify the definition namespace of the root classes",
		TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    /*
     * Parse the arguments and work out what the user wants to do.
     */

    if (objc != 2 && objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "?kind? namespace");
	return TCL_ERROR;
    }
    if (objc == 3 && Tcl_GetIndexFromObj(interp, objv[1], kindList, "kind", 0,
	    &kind) != TCL_OK) {
	return TCL_ERROR;
    }
    if (!TclGetString(objv[objc - 1])[0]) {
	nsNamePtr = NULL;
    } else {
	nsPtr = GetNamespaceInOuterContext(interp, objv[objc - 1]);
	if (nsPtr == NULL) {
	    return TCL_ERROR;
	}
	nsNamePtr = TclNewNamespaceObj(nsPtr);
	Tcl_IncrRefCount(nsNamePtr);
    }

    /*
     * Update the correct field of the class definition.
     */

    if (kind) {			// -instance
	storagePtr = &clsPtr->objDefinitionNs;
    } else {			// -class
	storagePtr = &clsPtr->clsDefinitionNs;
    }
    if (*storagePtr != NULL) {
	Tcl_DecrRefCount(*storagePtr);
    }
    *storagePtr = nsNamePtr;
    return TCL_OK;
}

/*
 * ----------------------------------------------------------------------

```

tcl9.1 9.1.0, revision `Original C9.1 source tree joined by complete byte hash; source inspection only, no callback execution or binary/source equivalence receipt`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOODefineDeleteMethodObjCmd`, lines 1951–1990. Retained reference `source-window-3`.

```text
TclOODefineDeleteMethodObjCmd(
    void *clientData,
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    bool isInstanceDeleteMethod = (clientData != NULL);
    Object *oPtr;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name ?name ...?");
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

```

tcl9.1 9.1.0, revision `Original C9.1 source tree joined by complete byte hash; source inspection only, no callback execution or binary/source equivalence receipt`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOODefineExportObjCmd`, lines 2064–2115. Retained reference `source-window-4`.

```text
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

```

tcl9.1 9.1.0, revision `Original C9.1 source tree joined by complete byte hash; source inspection only, no callback execution or binary/source equivalence receipt`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOODefineCmds.c`, function `TclOODefineUnexportObjCmd`, lines 2451–2497. Retained reference `source-window-5`.

```text
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

```

tcl9.1 9.1.0, revision `Original C9.1 source tree joined by complete byte hash; source inspection only, no callback execution or binary/source equivalence receipt`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclObj.c`, function `tclObjHashKeyType`, lines 264–272. Retained reference `source-window-6`.

```text
const Tcl_HashKeyType tclObjHashKeyType = {
    TCL_HASH_KEY_TYPE_VERSION,	/* version */
    TCL_HASH_KEY_DIRECT_COMPARE,/* allows compare keys by pointers */
    TclHashObjKey,		/* hashKeyProc */
    TclCompareObjKeys,		/* compareKeysProc */
    AllocObjEntry,		/* allocEntryProc */
    TclFreeObjEntry		/* freeEntryProc */
};

```

tcl9.1 9.1.0, revision `Original C9.1 source tree joined by complete byte hash; source inspection only, no callback execution or binary/source equivalence receipt`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclObj.c`, function `TclHashObjKey`, lines 4189–4200. Retained reference `source-window-7`.

```text
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

```

## Consumer bindings

- [runtime/rust/src/cmd_oo/native_context.rs](../../../../runtime/rust/src/cmd_oo/native_context.rs), `OriginalOoTarget`: Retain the exact selected Object/Class definition target and its original object creation stamp; check current existence/torn-down state and required class membership instead of reconstructing by spelling. This is not an independent Class-facet allocation, frame, lookup or invocation grant.
- [runtime/rust/src/cmd_oo/native_context.rs](../../../../runtime/rust/src/cmd_oo/native_context.rs), `with_original_arguments`: Retain the entered original argument tape and operation currency while getters and the selected definition worker run; completion checks do not manufacture a Native issuer.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `def_class`: Check the retained definition target after the selected original class operand and before target effects; actual withdrawal and typed first refusal remain distinct.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `def_definitionnamespace`: Retain the selected definition target across kind and namespace operand getters and before namespace publication.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `def_deletemethod`: Preserve sequential original name/removal stages and check the original target after outside-borrow method retirement before continuing.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `def_export`: Keep the selected original operand getter/effect schedule, reached early flags and first refusal; Runtime flag sets do not attest original C Method record/name/object-hash key roles.
- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `NativeTclooMethodExportOperandOrder`: Describe the independently selected per-operand C9.1 source schedule versus explicitly unmeasured compatibility recipes, without issuing method/flag or Native execution authority.
- [runtime/rust/src/cmd_oo/original_definition_stage_tests.rs](../../../../runtime/rust/src/cmd_oo/original_definition_stage_tests.rs), `cmd_oo::original_definition_stage_tests::class_operand_withdrawal_cannot_recreate_a_class_facet_on_the_original_object` (linked): The actual selected object definition target is retained before the class-name getter. Getter Host/ABA and actual object withdrawal preserve reached bytes and stop later class-facet effects; ordinary success remains a separate software branch. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_definition_stage_tests.rs](../../../../runtime/rust/src/cmd_oo/original_definition_stage_tests.rs), `cmd_oo::original_definition_stage_tests::definition_namespace_checks_original_target_after_kind_and_namespace_getters` (linked): Actual kind and namespace getters preserve original argument slots, retained target checks, first typed refusal and unreached later getter/publication; ordinary metadata edits remain separate from target withdrawal. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_definition_stage_tests.rs](../../../../runtime/rust/src/cmd_oo/original_definition_stage_tests.rs), `cmd_oo::original_definition_stage_tests::method_deletion_distinguishes_withdrawn_target_before_later_lookup_or_retirement` (linked): Actual Object/Class method removal checks the retained definition target after original name or outside-borrow body retirement; target withdrawal stops later lookup/getter and keeps the software deleted-target Guest presentation distinct from Host refusal. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_definition_stage_tests.rs](../../../../runtime/rust/src/cmd_oo/original_definition_stage_tests.rs), `cmd_oo::original_definition_stage_tests::selected_c91_visibility_keeps_reached_first_flag_when_a_later_operand_refuses` (linked): The selected C9.1 software schedule preserves the first reached export/unexport flag when a later original header updater yields Host/ABA refusal; later operand getters and flag effects stay unreached. This callback mode is not measured in original C. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [runtime/rust/src/cmd_oo/original_definition_stage_tests.rs](../../../../runtime/rust/src/cmd_oo/original_definition_stage_tests.rs), `cmd_oo::original_definition_stage_tests::visibility_compatibility_and_ordinary_metadata_changes_keep_their_selected_stages` (linked): Finite authored C8.6/C9.0 compatibility and C9.1 sequential schedules distinguish ordinary metadata edits from first typed refusal without asserting inspected chronology for the compatibility releases. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs), `native_tcloo_method_definition::export_order_tests::selected_export_operand_order_does_not_grant_uninspected_releases` (linked): The selected Registry protocol confines OperandThenEffect to the represented C9.1 release and keeps older supported releases on the explicitly unmeasured CompatibilityReadAll recipe. Protocol labels issue no Method record, object-hash key or callback execution authority. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.

## Evidence

- [runtime/rust/tests/data/original_definition_getter_effect_owner_source/C91](../../../../runtime/rust/tests/data/original_definition_getter_effect_owner_source/C91)
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs)
- [runtime/rust/src/cmd_oo/native_context.rs](../../../../runtime/rust/src/cmd_oo/native_context.rs)
- [runtime/rust/src/cmd_oo/original_definition_stage_tests.rs](../../../../runtime/rust/src/cmd_oo/original_definition_stage_tests.rs)
- [rust/tcl-registry/src/native_tcloo_method_definition.rs](../../../../rust/tcl-registry/src/native_tcloo_method_definition.rs)
