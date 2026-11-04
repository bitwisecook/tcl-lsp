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

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Tcl.

All 30 attempted coroutine name/context controls reject because coroutine is unavailable; publication/resume/fullname semantics were not reached. Jim Missing namespace-exists queries are themselves unsupported, so they do not prove absence.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Tcl.

All 30 attempted coroutine name/context controls reject because coroutine is unavailable; publication/resume/fullname semantics were not reached. Jim Missing namespace-exists queries are themselves unsupported, so they do not prove absence.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Tcl.

C8.6.18/9.0.4/9.1.0 accept 26 of 30 original name/context pairs and reject four missing-holder pairs without creating Missing. The original name pointer reaches the driver unchanged. Raw zero clips the CString name; C080, FF and D800 remain separate accepted bytes. Empty names and existing Q:: empty tails are accepted. v2 info-coroutine results independently report the current holder for bare/relative names and the global holder for absolute names. C8.4/8.5 and current Jim report coroutine unavailable in these controls.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Tcl.

C8.6.18/9.0.4/9.1.0 accept 26 of 30 original name/context pairs and reject four missing-holder pairs without creating Missing. The original name pointer reaches the driver unchanged. Raw zero clips the CString name; C080, FF and D800 remain separate accepted bytes. Empty names and existing Q:: empty tails are accepted. v2 info-coroutine results independently report the current holder for bare/relative names and the global holder for absolute names. C8.4/8.5 and current Jim report coroutine unavailable in these controls.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Tcl.

C8.6.18/9.0.4/9.1.0 accept 26 of 30 original name/context pairs and reject four missing-holder pairs without creating Missing. The original name pointer reaches the driver unchanged. Raw zero clips the CString name; C080, FF and D800 remain separate accepted bytes. Empty names and existing Q:: empty tails are accepted. v2 info-coroutine results independently report the current holder for bare/relative names and the global holder for absolute names. C8.4/8.5 and current Jim report coroutine unavailable in these controls.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Jim.

All 30 attempted coroutine name/context controls reject because coroutine is unavailable; publication/resume/fullname semantics were not reached. Jim Missing namespace-exists queries are themselves unsupported, so they do not prove absence.

### bigip

Status: `not-tested`. Version: not measured. Build: not recorded. Channel: No input supplied. Dialect: BIG-IP.

No observation for this question.

## Historical references

- `probe-v1` (input): [rust/tcl-registry/tests/data/native_coroutine_publication/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/probe.c). Exact immutable original-object v1 probe source; count constructors, pointer timing, per-stage getters and untested API branches are inspectable.
- `probe-v2` (input): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c). Exact v2 probe, adding separate info-coroutine fullname controls without changing the v1 outcomes.
- `tcl8.4-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.4-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.4-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/stderr). Exact process stderr, separate from guest completion.
- `tcl8.4-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.4-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.4-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/stderr). Exact process stderr, separate from guest completion.
- `tcl8.5-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.5-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.5-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/stderr). Exact process stderr, separate from guest completion.
- `tcl8.5-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.5-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.5-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/stderr). Exact process stderr, separate from guest completion.
- `tcl8.6-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.6-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.6-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/stderr). Exact process stderr, separate from guest completion.
- `tcl8.6-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.6-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.6-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/stderr). Exact process stderr, separate from guest completion.
- `tcl9.0-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.0-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.0-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/stderr). Exact process stderr, separate from guest completion.
- `tcl9.0-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.0-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.0-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/stderr). Exact process stderr, separate from guest completion.
- `tcl9.1-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.1-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.1-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr). Exact process stderr, separate from guest completion.
- `tcl9.1-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.1-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.1-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr). Exact process stderr, separate from guest completion.
- `jim-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/jim/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `jim-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/jim/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `jim-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/jim/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/jim/stderr). Exact process stderr, separate from guest completion.
- `jim-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/receipt.json). Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `jim-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/stdout.tsv). Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `jim-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/stderr). Exact process stderr, separate from guest completion.
- `source-0` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json). JSON pointer `/0/snippet`. Exact pinned TclNRCoroutineObjCmd source window, independent of result observations.
- `source-1` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json). JSON pointer `/1/snippet`. Exact pinned TclNRCoroutineObjCmd source window, independent of result observations.
- `source-2` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json). JSON pointer `/2/snippet`. Exact pinned TclNRCoroutineObjCmd source window, independent of result observations.
- `coroutine-resume-registration-source-tcl8.6` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json). JSON pointer `/0/snippet`. Native resume consumes the selected coroutine clientData; argv[0] is diagnostic text only. No coroutine-name lookup is performed by this handler.
- `coroutine-resume-registration-source-tcl9.0` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json). JSON pointer `/1/snippet`. Native resume consumes the selected coroutine clientData; argv[0] is diagnostic text only. No coroutine-name lookup is performed by this handler.
- `coroutine-resume-registration-source-tcl9.1` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json). JSON pointer `/2/snippet`. Native resume consumes the selected coroutine clientData; argv[0] is diagnostic text only. No coroutine-name lookup is performed by this handler.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_coroutine_publication/verify.py"
]
```

Offline exact receipt/input/stream association verification only; zero native/compiler/Rust launches. Original capture.py and queue retain absolute provisioned input pins and immutable output paths; fresh recapture must use new output directories and recheck every required hash. Source windows reproduce separately from exact pinned full source LF ranges. No missing ELF is fabricated or claimed portable.
