# naming.property.original-configure-retained-table-and-read-snapshot

Kind: `native-observation`

## Problem statement

A property setter or getter can change the property roster while configure remains active. Reusing an old selected name as though it were a current member, or rebuilding a configure-all read inventory after each getter, changes the selected property and returned snapshot. Public result equality must remain separate from the native table/header storage recipe.

## Question

For the two exact original configurable-class scripts, which setter executes after the first setter inserts b, and which properties and events does configure-all return when its first getter inserts b and an unrelated method?

## Conclusion

Tcl 9.0.4 and 9.1.0 each close both scripts exit0 with empty stderr. Multi-write reports events {a b}, while configure-all reports {-a A -z Z} and events {a z}, excluding newly added b; both final rosters include -a,-b,-z. Tcl 8.4.20, 8.5.19, 8.6.18 and Jim 0.84-9-g5bac7c9 reject oo::configurable before these workers. Two generic-class attempts reject property before the intended worker and supply no intended answer. Independent C9 source excerpts explain old cached name-table selection against current members and the retained configure-all List; C9.0 printed values do not grant the C9.1 software physical purpose. BIG-IP and the named Rust body have no execution outcome.

## Scope

Twelve exact configurable-script CLI observations, independent own version/process receipts, and two separately retained generic-class setup refusals. Script-visible code/result/event/roster bytes are measured; no custom header updater, pointer identity, cache primary, reference-count chronology or internal error-formatting stop is observed. Six invoked-file/downstream executable joins and whole lossless ELF bytes are retained. ldd reports dependencies without retained shared-library byte closure; inherited full environment and configure/build replay are absent. Eight exact source windows join whole C9.0/C9.1 tclOOProp.c bytes independently. One source-bound C9.1 software comparator remains unrun; no physical frame, Host metadata, Native admission, whole engine, BIG-IP/TMM or Rust implementation equivalence follows.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim

Status: `observed`. Channel: Two exact ordinary script files through recorded CLI argv; independent version script, whole public stdout/stderr and own closed process receipts. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; jim 0.84-9-g5bac7c9.

Both exact configurable scripts close exit1 with empty stdout and invalid command name "oo::configurable" before either intended configure worker is reached. This measures producer unavailability for these invocations, not the intended retained-table/read-snapshot behavior.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Two exact ordinary script files through recorded CLI argv; independent version script, whole public stdout/stderr and own closed process receipts. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Both exact configurable scripts close exit0 with empty stderr. Multi-write prints code 0, empty result, events {a b} and properties {-a -b -z}; configure-all prints code 0, result {-a A -z Z}, events {a z} and properties {-a -b -z}. The latter excludes newly added b from its read result. These public values alone establish no pointer/refcount/physical cache recipe or Rust assertion outcome.

### bigip

Status: `not-tested`. Channel: No BIG-IP property script response.

No original BIG-IP response is attached. Authored Runtime profiles and local Tcl/Jim observations establish no BIG-IP/TMM property behavior.

## Source inspection

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetPropertyName`, lines 274–343. Retained reference `source-window-0`.

```text
GetPropertyName(
    Tcl_Interp *interp,		/* Context and error reporting. */
    Object *oPtr,		/* Object to get property name from. */
    int flags,			/* Are we looking for a writable property?
				 * Can we do a fallback message?
				 * See GPNFlags for possible values */
    Tcl_Obj *namePtr,		/* The name supplied by the user. */
    GPNCache **cachePtr)	/* Where to cache the table, if the caller
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

```

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `ReleasePropertyNameCache`, lines 347–360. Retained reference `source-window-1`.

```text
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

```

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 365–409. Retained reference `source-window-2`.

```text
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

```

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 416–466. Retained reference `source-window-3`.

```text
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

```

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `GetPropertyName`, lines 163–232. Retained reference `source-window-4`.

```text
GetPropertyName(
    Tcl_Interp *interp,		/* Context and error reporting. */
    Object *oPtr,		/* Object to get property name from. */
    int flags,			/* Are we looking for a writable property?
				 * Can we do a fallback message?
				 * See GPNFlags for possible values */
    Tcl_Obj *namePtr,		/* The name supplied by the user. */
    GPNCache **cachePtr)	/* Where to cache the table, if the caller
				 * wants that. The contents are to be freed
				 * with Tcl_Free if the cache is used. */
{
    Tcl_Size objc, index, i;
    Tcl_Obj *listPtr = GetAllObjectProperties(
	    oPtr, flags & GPN_WRITABLE);
    Tcl_Obj **objv;
    GPNCache *tablePtr;

    (void) Tcl_ListObjGetElements(NULL, listPtr, &objc, &objv);
    if (cachePtr && *cachePtr) {
	tablePtr = *cachePtr;
    } else {
	tablePtr = (GPNCache *) TclStackAlloc(interp,
		offsetof(GPNCache, names) + sizeof(char *) * (objc + 1));

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

```

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `ReleasePropertyNameCache`, lines 236–249. Retained reference `source-window-5`.

```text
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

```

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 254–298. Retained reference `source-window-6`.

```text
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter used for the result, error
				 * reporting, etc. */
    Tcl_ObjectContext context,	/* The object/call context. */
    int objc,			/* Number of arguments. */
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
	 * Read all properties.
	 */

	Tcl_Obj *listPtr = GetAllObjectProperties(oPtr, 0);
	Tcl_Obj *resultPtr = Tcl_NewObj(), **namev;

	Tcl_IncrRefCount(listPtr);
	ListObjGetElements(listPtr, namec, namev);

	for (i = 0; i < namec; ) {
	    code = ReadProperty(interp, oPtr, TclGetString(namev[i]));
	    if (code != TCL_OK) {
		Tcl_DecrRefCount(resultPtr);
		break;
	    }
	    Tcl_DictObjPut(NULL, resultPtr, namev[i],
		    Tcl_GetObjResult(interp));
	    if (++i >= namec) {
		Tcl_SetObjResult(interp, resultPtr);

```

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 305–355. Retained reference `source-window-7`.

```text
    } else if (objc == 1) {
	/*
	 * Read a single named property.
	 */

	namePtr = GetPropertyName(interp, oPtr, 0, objv[0], NULL);
	if (namePtr == NULL) {
	    return TCL_ERROR;
	}
	return ReadProperty(interp, oPtr, TclGetString(namePtr));
    } else if (objc == 2) {
	/*
	 * Special case for writing to one property. Saves fiddling with the
	 * cache in this common case.
	 */

	namePtr = GetPropertyName(interp, oPtr, GPN_WRITABLE, objv[0], NULL);
	if (namePtr == NULL) {
	    return TCL_ERROR;
	}
	code = WriteProperty(interp, oPtr, TclGetString(namePtr), objv[1]);
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
	    code = WriteProperty(interp, oPtr, TclGetString(namePtr),
		    objv[i + 1]);
	    if (code != TCL_OK) {
		break;
	    }
	}
	if (code == TCL_OK) {
	    Tcl_ResetResult(interp);
	}
	ReleasePropertyNameCache(interp, &cache);
	return code;

```

## Consumer bindings

- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `lookup_property`: Selected software property lookup retains the old copied name table while mapping its selected index into the actual current property header. Actual currency/target/lifetime checks remain independent; public C9.0 values do not grant this C9.1 physical purpose.
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `configure_all_original`: Selected software configure-all retains the actual working property header/member inventory across accessor calls. The native two-script output comparison does not certify callback Host/refusal cleanup or physical reference counts.
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `configure_native_original`: Selected software multi-write carries a retained lookup table between actual sequential accessors; its issuer, actual object ownership and completion guards remain independent of printed result equality.
- [runtime/rust/src/cmd_oo/original_property_reporting_tests.rs](../../../../runtime/rust/src/cmd_oo/original_property_reporting_tests.rs), `cmd_oo::original_property_reporting_tests::materialized_property_reports_preserve_native_snapshot_and_table_index_roles` (linked): Two exact original configurable scripts and printed results/events/rosters supply a bounded software comparison under the actual C9.1 physical adapter purpose. C9.0 equal public results do not broaden that physical issuer. This body is unrun; independent sort/refusal/private-inventory controls and native pointer/refcount/lifetime equality are not inferred.
- [rust/tcl-vm/src/cmd_oo/native_properties/stage_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties/stage_tests.rs), `cmd_oo::native_properties::stage_tests::materialized_property_reports_preserve_native_snapshot_and_table_index_roles` (linked): Materialized property reports preserve native snapshot and table index roles. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.

## Evidence

- [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original)
- [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335)
- [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim)
- [runtime/rust/src/cmd_oo](../../../../runtime/rust/src/cmd_oo)
- [runtime/rust/src](../../../../runtime/rust/src)
- [rust/tcl-vm/src/cmd_oo/native_properties](../../../../rust/tcl-vm/src/cmd_oo/native_properties)
