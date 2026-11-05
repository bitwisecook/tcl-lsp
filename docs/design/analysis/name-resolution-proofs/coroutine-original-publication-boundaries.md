# naming.coroutine.original-publication-boundaries

Kind: `native-observation`

## Problem statement

String-only coroutine creation can reject opaque native bytes, choose the wrong holder or create namespaces the native coroutine handler requires to exist.

## Question

Which name bytes, namespace holder, missing-namespace errors and empty simple names does coroutine creation select in the finite original-object controls?

## Conclusion

C8.6.18/9.0.4/9.1.0 accept 26 of 30 original name/context pairs and reject four missing-holder pairs without creating Missing. The original name pointer reaches the driver unchanged. Raw zero clips the CString name; C080, FF and D800 remain separate accepted bytes. Empty names and existing Q:: empty tails are accepted. v2 info-coroutine results independently report the current holder for bare/relative names and the global holder for absolute names. C8.4/8.5 and current Jim report coroutine unavailable in these controls.

## Scope

Two immutable all-six variants, 15 original names in global and N contexts. v1 start/resume and inventory cannot alone establish publication holder; v2 info-coroutine fullname controls supply the independent holder discriminator. Setup creates N, N::Q and Q only. Original Tcl string objects carry explicitly counted bytes, not ByteArray getters or Document/source UTF re-encoding. Supported C85+ options are post-result observations. No command cache/compiler/preparation, fresh holder creation, lifecycle or body Normal grant.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `unsupported`. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

All 30 attempted coroutine name/context controls reject because coroutine is unavailable; publication/resume/fullname semantics were not reached. Jim Missing namespace-exists queries are themselves unsupported, so they do not prove absence.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

C8.6.18/9.0.4/9.1.0 accept 26 of 30 original name/context pairs and reject four missing-holder pairs without creating Missing. The original name pointer reaches the driver unchanged. Raw zero clips the CString name; C080, FF and D800 remain separate accepted bytes. Empty names and existing Q:: empty tails are accepted. v2 info-coroutine results independently report the current holder for bare/relative names and the global holder for absolute names. C8.4/8.5 and current Jim report coroutine unavailable in these controls.

### bigip

Status: `not-tested`. Channel: No input supplied.

No observation for this question.

## Source inspection

tcl9.1 9.1.0, revision `Exact pinned source associated by immutable native coroutine publication receipts`, `generic/tclBasic.c`, function `TclNRInterpCoroutine`, lines 9983–10034. Retained reference `coroutine-resume-registration-source-tcl9.1`.

```text
int
TclNRInterpCoroutine(
    void *clientData,
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    CoroutineData *corPtr = (CoroutineData *)clientData;

    if (!COR_IS_SUSPENDED(corPtr)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"coroutine \"%s\" is already running",
		TclGetString(objv[0])));
	Tcl_SetErrorCode(interp, "TCL", "COROUTINE", "BUSY", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Parse all the arguments to work out what to feed as the result of the
     * [yield]. TRICKY POINT: objc==0 happens here! It occurs when a coroutine
     * is deleted!
     */

    switch (corPtr->nargs) {
    case COROUTINE_ARGUMENTS_SINGLE_OPTIONAL:
	if (objc == 2) {
	    Tcl_SetObjResult(interp, objv[1]);
	} else if (objc > 2) {
	    Tcl_WrongNumArgs(interp, 1, objv, "?arg?");
	    return TCL_ERROR;
	}
	break;
    default:
	if (corPtr->nargs + 1 != objc) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "wrong coro nargs; how did we get here? "
		    "not implemented!", TCL_INDEX_NONE));
	    Tcl_SetErrorCode(interp, "TCL", "WRONGARGS", (char *)NULL);
	    return TCL_ERROR;
	}
	TCL_FALLTHROUGH();
    case COROUTINE_ARGUMENTS_ARBITRARY:
	if (objc > 1) {
	    Tcl_SetObjResult(interp, Tcl_NewListObj(objc - 1, objv + 1));
	}
	break;
    }

    TclNRAddCallback(interp, TclNRCoroutineActivateCallback, corPtr,
	    NULL, NULL, NULL);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Pinned full source, independently associated by captured receipt required_sha256`, `generic/tclBasic.c`, function `TclNRCoroutineObjCmd`, lines 9737–9786. Retained reference `source-1`.

```text
int
TclNRCoroutineObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Command *cmdPtr;
    CoroutineData *corPtr;
    const char *procName, *simpleName;
    Namespace *nsPtr, *altNsPtr, *cxtNsPtr,
	*inNsPtr = (Namespace *)TclGetCurrentNamespace(interp);
    Namespace *lookupNsPtr = iPtr->varFramePtr->nsPtr;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name cmd ?arg ...?");
	return TCL_ERROR;
    }

    procName = TclGetString(objv[1]);
    TclGetNamespaceForQualName(interp, procName, inNsPtr, 0,
	    &nsPtr, &altNsPtr, &cxtNsPtr, &simpleName);

    if (nsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": unknown namespace",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE", (char *)NULL);
	return TCL_ERROR;
    }
    if (simpleName == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": bad procedure name",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "COMMAND", procName, (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * We ARE creating the coroutine command: allocate the corresponding
     * struct and create the corresponding command.
     */

    corPtr = (CoroutineData *)Tcl_Alloc(sizeof(CoroutineData));

    cmdPtr = (Command *) TclNRCreateCommandInNs(interp, simpleName,
	    (Tcl_Namespace *)nsPtr, /*objProc*/ NULL, TclNRInterpCoroutine,
	    corPtr, DeleteCoroutine);

    corPtr->cmdPtr = cmdPtr;

```

tcl9.1 9.1.0, revision `Pinned full source, independently associated by captured receipt required_sha256`, `generic/tclBasic.c`, function `TclNRCoroutineObjCmd`, lines 10047–10096. Retained reference `source-2`.

```text
int
TclNRCoroutineObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Command *cmdPtr;
    CoroutineData *corPtr;
    const char *procName, *simpleName;
    Namespace *nsPtr, *altNsPtr, *cxtNsPtr;
    Namespace *inNsPtr = (Namespace *)TclGetCurrentNamespace(interp);
    Namespace *lookupNsPtr = iPtr->varFramePtr->nsPtr;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name cmd ?arg ...?");
	return TCL_ERROR;
    }

    procName = TclGetString(objv[1]);
    TclGetNamespaceForQualName(interp, procName, inNsPtr, 0,
	    &nsPtr, &altNsPtr, &cxtNsPtr, &simpleName);

    if (nsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": unknown namespace",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE", (char *)NULL);
	return TCL_ERROR;
    }
    if (simpleName == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": bad procedure name",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "COMMAND", procName, (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * We ARE creating the coroutine command: allocate the corresponding
     * struct and create the corresponding command.
     */

    corPtr = (CoroutineData *)Tcl_Alloc(sizeof(CoroutineData));

    cmdPtr = (Command *) TclNRCreateCommandInNs(interp, simpleName,
	    (Tcl_Namespace *)nsPtr, /*objProc*/ NULL, TclNRInterpCoroutine,
	    corPtr, DeleteCoroutine);

    corPtr->cmdPtr = cmdPtr;

```

## Consumer bindings

- [rust/tcl-vm/src/interp/native_coroutine_names.rs](../../../../rust/tcl-vm/src/interp/native_coroutine_names.rs), `original_coroutine_names_match_native_publication_controls`: Actual original-object comparison harness; v2 fullname supplement independent of v1 relative lookup.
- [runtime/rust/src/interp/native_coroutine_names.rs](../../../../runtime/rust/src/interp/native_coroutine_names.rs), `original_coroutine_names_match_native_publication_controls`: Actual original-object comparison harness; v2 fullname supplement independent of v1 relative lookup.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::coroutine_publication_slot`: Independently selected name extent only; actual holder and publication remain backend-owned.
- [runtime/rust/src/interp/native_compilation.rs](../../../../runtime/rust/src/interp/native_compilation.rs), `Interp::enter_native_builtin`: Retains actual selected registration generation separately from optional stock identity, with existing activation guard/context lifetime; direct compiled helpers carry no generation.
- [runtime/rust/src/interp/native_compilation.rs](../../../../runtime/rust/src/interp/native_compilation.rs), `Interp::active_builtin_command_generation`: Read the actual selected builtin activation's registration generation independently of original argv, display identity and stock metadata.
- [runtime/rust/src/cmd_coro.rs](../../../../runtime/rust/src/cmd_coro.rs), `coro_resume_command`: Uses actual selected coroutine placement while retaining unchanged caller argument objects. Does not reparse bytes after the native name extent.
- [rust/tcl-vm/src/cmd_coro.rs](../../../../rust/tcl-vm/src/cmd_coro.rs), `coro_resume`: Uses the actual selected command sidecar and retained original argv[0]; no text representation or command-name relookup selects the coroutine.
- [rust/tcl-vm/src/interp/native_coroutine_names.rs](../../../../rust/tcl-vm/src/interp/native_coroutine_names.rs), `interp::native_coroutine_names::tests::original_coroutine_names_match_native_publication_controls` (linked): Original starts/resumes, exact missing-namespace errors and no Missing creation; v2 comparison supplement adds separately measured info-coroutine published fullname. No test execution claimed.
- [runtime/rust/src/interp/native_coroutine_names.rs](../../../../runtime/rust/src/interp/native_coroutine_names.rs), `interp::native_coroutine_names::tests::original_coroutine_names_match_native_publication_controls` (linked): Original starts/resumes, exact missing-namespace errors and no Missing creation; v2 comparison supplement adds separately measured info-coroutine published fullname. No test execution claimed.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::coroutine_publication_tests::coroutine_slots_keep_current_holders_and_native_byte_extents` (linked): Pure selected CString slot geometry, including existing holders and raw-zero clipping; no handler or availability grant.

## Evidence

- [rust/tcl-registry/tests/data/native_coroutine_publication](../../../../rust/tcl-registry/tests/data/native_coroutine_publication)
- [rust/tcl-registry/tests/data/native_coroutine_publication/v2](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2)
