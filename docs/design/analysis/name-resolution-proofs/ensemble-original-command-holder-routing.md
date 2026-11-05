# naming.ensemble.original-command-holder-routing

Kind: `native-observation`

## Problem statement

Default and explicit ensemble command publication have different original naming purposes. Returned/displayed paths after namespace deletion and same-spelling recreation cannot establish private namespace or command identity; original full API evaluations must retain their provider-specific availability and completion results.

## Question

What exact public creation, existence, caught call and separate query results do default and explicit ensemble controls produce in live namespaces and after active ancestor deletion with same-spelling recreation?

## Conclusion

All 24 original external API case processes exit zero with empty stderr; six independent driver compilations exit zero. Each prelude returns guest code zero and the empty value. C8.4.20 returns code one for namespace ensemble in all four originals, and both later retained queries return code one because ::observed is absent. C8.5.19/C8.6.18 live default returns code zero with ::P::N 1 VALUE; live relative-qualified returns ::P::N::q::E 1 VALUE {}. Their two retained originals return code one with tried to manipulate ensemble of deleted namespace, and both independent queries return code one for absent ::observed. C9.0.4/C9.1.0 have those same successful live results. Their default retained original returns code zero with ::P::N::N 0 {} 1 {invalid command name "::P::N"}; explicit E returns ::P::N::E 0 {} 1 {invalid command name "::P::N::E"}. Their subsequent independent queries return code zero, retaining the corresponding original tuple followed by 0 0 NEW. Creation completion succeeds in those C9 retained cases while the measured rooted call fails; it is not successful rooted dispatch. Jim 0.84-9-g5bac7c9 returns code one with wrong # args: should be "ensemble command ?-automap prefix?" for all four exact C-syntax originals, and both retained queries fail for absent ::observed. Its separate ensemble API is not substituted. Whole original hex rows and exact strings remain authoritative. BIG-IP is not tested. Public names/results do not grant private parent/current namespace tokens, command allocations, incarnation, reference count, dispatcher identity, engine parity, arbitrary deletion law, Native compiler admission, successful Rust assertions or edit authority. Independent pinned source inspection records a separate implementation explanation: C8.5 NamespaceEnsembleCmd in tclNamesp.c and C8.6 TclNamespaceEnsembleCmd test NS_DYING, while C9.0/C9.1 entry tests NS_DEAD. C8.5 Tcl_CreateEnsemble constructs a qualified name and passes it through Tcl_CreateObjCommand. C8.6+ source default routing selects the actual parent while explicit source names use their selected current context; TclCreateEnsembleInNs retains independently supplied command/ensemble namespace arguments. Exact full sources, headers and line excerpts support these source facts only. They do not identify the private tokens/flags held during the recorded native processes, prove whole-binary source identity or add a native execution outcome.

## Scope

Four exact ASCII original source files share one exact independently evaluated prelude; two retained cases have one exact independent later query. Each fresh external driver uses full Tcl_Init/Tcl_EvalEx(flags0) or Jim core/static extensions and counted Jim_EvalObj, without joining or surrounding the prelude/body/query files with eval/catch/puts. Original caught call constructs remain unchanged source. This is the same exact native202 driver source under separately recorded actual compiler/run processes. C8.4 unavailability, C8.5/C8.6 deleted-namespace refusal, C9 creation completion with unavailable rooted dispatch, and Jim distinct API failure remain separate. Source/library/build associations do not establish private holder/token state or complete compiled-source identity.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh external full Tcl_Init/Tcl_EvalEx(flags0). Version: tcl8.4 8.4.20.

Actual stock 8.4.20 has unavailable namespace ensemble: all four exact original evaluations return code1 with its full bad-option table. Both retained-case later queries return code1 for absent ::observed. All preludes return code0/empty; no creation/dispatch result is inferred. All four case processes exit zero with empty stderr; guest errors remain measured results.

### tcl8.5

Status: `observed`. Channel: Fresh external full Tcl_Init/Tcl_EvalEx(flags0). Version: tcl8.5 8.5.19.

Actual stock 8.5.19 live default returns code0/::P::N 1 VALUE and relative-qualified returns code0/::P::N::q::E 1 VALUE {}. Both retained originals return code1/tried to manipulate ensemble of deleted namespace; both separate queries return code1 for absent ::observed. No retained creation or rooted dispatch succeeds in these measured cases. All four case processes exit zero with empty stderr; guest errors remain measured results.

### tcl8.6

Status: `observed`. Channel: Fresh external full Tcl_Init/Tcl_EvalEx(flags0). Version: tcl8.6 8.6.18.

Actual stock 8.6.18 live default returns code0/::P::N 1 VALUE and relative-qualified returns code0/::P::N::q::E 1 VALUE {}. Both retained originals return code1/tried to manipulate ensemble of deleted namespace; both separate queries return code1 for absent ::observed. No retained creation or rooted dispatch succeeds in these measured cases. All four case processes exit zero with empty stderr; guest errors remain measured results.

### tcl9.0

Status: `observed`. Channel: Fresh external full Tcl_Init/Tcl_EvalEx(flags0). Version: tcl9.0 9.0.4.

Actual stock 9.0.4 live default returns code0/::P::N 1 VALUE and relative-qualified returns code0/::P::N::q::E 1 VALUE {}. Retained default original returns code0/::P::N::N 0 {} 1 {invalid command name "::P::N"}; retained E returns code0/::P::N::E 0 {} 1 {invalid command name "::P::N::E"}. Each independent later query returns its original tuple followed by 0 0 NEW. Successful original creation completion remains distinct from failed rooted call and zero existence fields; private parent/current tokens are not observed. All four case processes exit zero with empty stderr; guest errors remain measured results.

### tcl9.1

Status: `observed`. Channel: Fresh external full Tcl_Init/Tcl_EvalEx(flags0). Version: tcl9.1 9.1.0.

Actual stock 9.1.0 live default returns code0/::P::N 1 VALUE and relative-qualified returns code0/::P::N::q::E 1 VALUE {}. Retained default original returns code0/::P::N::N 0 {} 1 {invalid command name "::P::N"}; retained E returns code0/::P::N::E 0 {} 1 {invalid command name "::P::N::E"}. Each independent later query returns its original tuple followed by 0 0 NEW. Successful original creation completion remains distinct from failed rooted call and zero existence fields; private parent/current tokens are not observed. All four case processes exit zero with empty stderr; guest errors remain measured results.

### jim

Status: `observed`. Channel: Fresh external Jim core/static extensions and counted original Jim_EvalObj; independent prelude/original/query files. Version: jim 0.84-9-g5bac7c9.

Actual Jim 0.84-9-g5bac7c9 all four original C namespace-ensemble syntax controls return code1 with wrong # args: should be "ensemble command ?-automap prefix?". Both retained-case queries return code1 for absent ::observed. All preludes return code0/empty. Jim own API is not substituted and no namespace-token or creation parity is inferred. All four case processes exit zero with empty stderr; guest errors remain measured results.

### bigip

Status: `not-tested`.

No appliance execution answers this exact independently evaluated source/API question.

## Source inspection

tcl8.5 8.5.19, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceEnsembleCmd`, lines 4780–4810. Retained reference `ensemble-source208-8.5.19-tclNamesp.c-4780-4810.txt`.

```text
    };
    enum EnsCreateOpts {
	CRT_CMD, CRT_MAP, CRT_PREFIX, CRT_SUBCMDS, CRT_UNKNOWN
    };
    static const char *configOptions[] = {
	"-map", "-namespace", "-prefixes", "-subcommands", "-unknown", NULL
    };
    enum EnsConfigOpts {
	CONF_MAP, CONF_NAMESPACE, CONF_PREFIX, CONF_SUBCMDS, CONF_UNKNOWN
    };
    int index;

    nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    if (nsPtr == NULL || nsPtr->flags & NS_DYING) {
	if (!Tcl_InterpDeleted(interp)) {
	    Tcl_AppendResult(interp,
		    "tried to manipulate ensemble of deleted namespace", NULL);
	}
	return TCL_ERROR;
    }

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 2, objv, "subcommand ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[2], subcommands, "subcommand", 0,
	    &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum EnsSubcmds) index) {

```

tcl8.5 8.5.19, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceEnsembleCmd`, lines 4835–4858. Retained reference `ensemble-source208-8.5.19-tclNamesp.c-4835-4858.txt`.

```text
	/*
	 * Parse the option list, applying type checks as we go. Note that we
	 * are not incrementing any reference counts in the objects at this
	 * stage, so the presence of an option multiple times won't cause any
	 * memory leaks.
	 */

	for (; objc>1 ; objc-=2,objv+=2 ) {
	    if (Tcl_GetIndexFromObj(interp, objv[0], createOptions, "option",
		    0, &index) != TCL_OK) {
		if (allocatedMapFlag) {
		    Tcl_DecrRefCount(mapObj);
		}
		return TCL_ERROR;
	    }
	    switch ((enum EnsCreateOpts) index) {
	    case CRT_CMD:
		name = TclGetString(objv[1]);
		continue;
	    case CRT_SUBCMDS:
		if (TclListObjLength(interp, objv[1], &len) != TCL_OK) {
		    if (allocatedMapFlag) {
			Tcl_DecrRefCount(mapObj);
		    }

```

tcl8.5 8.5.19, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceEnsembleCmd`, lines 4948–4980. Retained reference `ensemble-source208-8.5.19-tclNamesp.c-4948-4980.txt`.

```text
			Tcl_DecrRefCount(mapObj);
		    }
		    return TCL_ERROR;
		}
		unknownObj = (len > 0 ? objv[1] : NULL);
		continue;
	    }
	}

	/*
	 * Create the ensemble. Note that this might delete another ensemble
	 * linked to the same namespace, so we must be careful. However, we
	 * should be OK because we only link the namespace into the list once
	 * we've created it (and after any deletions have occurred.)
	 */

	token = Tcl_CreateEnsemble(interp, name, NULL,
		(permitPrefix ? TCL_ENSEMBLE_PREFIX : 0));
	Tcl_SetEnsembleSubcommandList(interp, token, subcmdObj);
	Tcl_SetEnsembleMappingDict(interp, token, mapObj);
	Tcl_SetEnsembleUnknownHandler(interp, token, unknownObj);

	/*
	 * Tricky! Must ensure that the result is not shared (command delete
	 * traces could have corrupted the pristine object that we started
	 * with). [Snit test rename-1.5]
	 */

	Tcl_ResetResult(interp);
	Tcl_GetCommandFullName(interp, token, Tcl_GetObjResult(interp));
	return TCL_OK;
    }

```

tcl8.5 8.5.19, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `Tcl_CreateEnsemble`, lines 5273–5337. Retained reference `ensemble-source208-8.5.19-tclNamesp.c-5273-5337.txt`.

```text
Tcl_CreateEnsemble(
    Tcl_Interp *interp,
    const char *name,
    Tcl_Namespace *namespacePtr,
    int flags)
{
    Namespace *nsPtr = (Namespace *) namespacePtr;
    EnsembleConfig *ensemblePtr = (EnsembleConfig *)
	    ckalloc(sizeof(EnsembleConfig));
    Tcl_Obj *nameObj = NULL;

    if (nsPtr == NULL) {
	nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    }

    /*
     * Make the name of the ensemble into a fully qualified name. This might
     * allocate a temporary object.
     */

    if (!(name[0] == ':' && name[1] == ':')) {
	nameObj = Tcl_NewStringObj(nsPtr->fullName, -1);
	if (nsPtr->parentPtr == NULL) {
	    Tcl_AppendStringsToObj(nameObj, name, NULL);
	} else {
	    Tcl_AppendStringsToObj(nameObj, "::", name, NULL);
	}
	Tcl_IncrRefCount(nameObj);
	name = TclGetString(nameObj);
    }

    ensemblePtr->nsPtr = nsPtr;
    ensemblePtr->epoch = 0;
    Tcl_InitHashTable(&ensemblePtr->subcommandTable, TCL_STRING_KEYS);
    ensemblePtr->subcommandArrayPtr = NULL;
    ensemblePtr->subcmdList = NULL;
    ensemblePtr->subcommandDict = NULL;
    ensemblePtr->flags = flags;
    ensemblePtr->unknownHandler = NULL;
    ensemblePtr->token = Tcl_CreateObjCommand(interp, name,
	    NsEnsembleImplementationCmd, ensemblePtr, DeleteEnsembleConfig);
    ensemblePtr->next = (EnsembleConfig *) nsPtr->ensembles;
    nsPtr->ensembles = (Tcl_Ensemble *) ensemblePtr;

    /*
     * Trigger an eventual recomputation of the ensemble command set. Note
     * that this is slightly tricky, as it means that we are not actually
     * counting the number of namespace export actions, but it is the simplest
     * way to go!
     */

    nsPtr->exportLookupEpoch++;

    if (flags & ENSEMBLE_COMPILE) {
	((Command *) ensemblePtr->token)->compileProc = TclCompileEnsemble;
    }

    if (nameObj != NULL) {
	TclDecrRefCount(nameObj);
    }
    return ensemblePtr->token;
}

/*
 *----------------------------------------------------------------------

```

tcl8.6 8.6.18, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEnsemble.c`, function `TclNamespaceEnsembleCmd`, lines 145–175. Retained reference `ensemble-source208-8.6.18-tclEnsemble.c-145-175.txt`.

```text
{
    Tcl_Namespace *namespacePtr;
    Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp), *cxtPtr,
	    *foundNsPtr, *altFoundNsPtr, *actualCxtPtr;
    Tcl_Command token;
    Tcl_DictSearch search;
    Tcl_Obj *listObj;
    const char *simpleName;
    int index;
    int done;

    if (nsPtr == NULL || nsPtr->flags & NS_DYING) {
	if (!Tcl_InterpDeleted(interp)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "tried to manipulate ensemble of deleted namespace",
		    -1));
	    Tcl_SetErrorCode(interp, "TCL", "ENSEMBLE", "DEAD", (char *)NULL);
	}
	return TCL_ERROR;
    }

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "subcommand ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], ensembleSubcommands,
	    "subcommand", 0, &index) != TCL_OK) {
	return TCL_ERROR;
    }

    switch ((enum EnsSubcmds) index) {

```

tcl8.6 8.6.18, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEnsemble.c`, function `TclNamespaceEnsembleCmd`, lines 190–225. Retained reference `ensemble-source208-8.6.18-tclEnsemble.c-190-225.txt`.

```text
	 * Check that we've got option-value pairs... [Bug 1558654]
	 */

	if (objc & 1) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?option value ...?");
	    return TCL_ERROR;
	}
	objv += 2;
	objc -= 2;

	name = nsPtr->name;
	cxtPtr = (Namespace *) nsPtr->parentPtr;

	/*
	 * Parse the option list, applying type checks as we go. Note that we
	 * are not incrementing any reference counts in the objects at this
	 * stage, so the presence of an option multiple times won't cause any
	 * memory leaks.
	 */

	for (; objc>1 ; objc-=2,objv+=2) {
	    if (Tcl_GetIndexFromObj(interp, objv[0], ensembleCreateOptions,
		    "option", 0, &index) != TCL_OK) {
		if (allocatedMapFlag) {
		    Tcl_DecrRefCount(mapObj);
		}
		return TCL_ERROR;
	    }
	    switch ((enum EnsCreateOpts) index) {
	    case CRT_CMD:
		name = TclGetString(objv[1]);
		cxtPtr = nsPtr;
		continue;
	    case CRT_SUBCMDS:
		if (TclListObjLength(interp, objv[1], &len) != TCL_OK) {
		    if (allocatedMapFlag) {

```

tcl8.6 8.6.18, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEnsemble.c`, function `TclNamespaceEnsembleCmd`, lines 333–359. Retained reference `ensemble-source208-8.6.18-tclEnsemble.c-333-359.txt`.

```text
		}
		unknownObj = (len > 0 ? objv[1] : NULL);
		continue;
	    }
	}

	TclGetNamespaceForQualName(interp, name, cxtPtr,
		TCL_CREATE_NS_IF_UNKNOWN, &foundNsPtr, &altFoundNsPtr,
		&actualCxtPtr, &simpleName);

	/*
	 * Create the ensemble. Note that this might delete another ensemble
	 * linked to the same namespace, so we must be careful. However, we
	 * should be OK because we only link the namespace into the list once
	 * we've created it (and after any deletions have occurred.)
	 */

	token = TclCreateEnsembleInNs(interp, simpleName,
	     (Tcl_Namespace *) foundNsPtr, (Tcl_Namespace *) nsPtr,
	     (permitPrefix ? TCL_ENSEMBLE_PREFIX : 0));
	Tcl_SetEnsembleSubcommandList(interp, token, subcmdObj);
	Tcl_SetEnsembleMappingDict(interp, token, mapObj);
	Tcl_SetEnsembleUnknownHandler(interp, token, unknownObj);
	Tcl_SetEnsembleParameterList(interp, token, paramObj);

	/*
	 * Tricky! Must ensure that the result is not shared (command delete

```

tcl8.6 8.6.18, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEnsemble.c`, function `Tcl_CreateEnsemble`, lines 725–745. Retained reference `ensemble-source208-8.6.18-tclEnsemble.c-725-745.txt`.

```text
Tcl_Command
Tcl_CreateEnsemble(
    Tcl_Interp *interp,
    const char *name,
    Tcl_Namespace *namespacePtr,
    int flags)
{
    Namespace *nsPtr = (Namespace *)namespacePtr, *foundNsPtr, *altNsPtr,
	    *actualNsPtr;
    const char * simpleName;

    if (nsPtr == NULL) {
	nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    }

    TclGetNamespaceForQualName(interp, name, nsPtr, TCL_CREATE_NS_IF_UNKNOWN,
	    &foundNsPtr, &altNsPtr, &actualNsPtr, &simpleName);
    return TclCreateEnsembleInNs(interp, simpleName,
	    (Tcl_Namespace *) foundNsPtr, (Tcl_Namespace *) nsPtr, flags);
}


```

tcl9.0 9.0.4, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEnsemble.c`, function `TclNamespaceEnsembleCmd`, lines 151–176. Retained reference `ensemble-source208-9.0.4-tclEnsemble.c-151-176.txt`.

```text
TclNamespaceEnsembleCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    Tcl_Command token;		/* The ensemble command. */
    enum EnsSubcmds index;

    if (nsPtr == NULL || nsPtr->flags & NS_DEAD) {
	if (!Tcl_InterpDeleted(interp)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "tried to manipulate ensemble of deleted namespace",
		    TCL_AUTO_LENGTH));
	    Tcl_SetErrorCode(interp, "TCL", "ENSEMBLE", "DEAD", (char *)NULL);
	}
	return TCL_ERROR;
    }

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "subcommand ?arg ...?");
	return TCL_ERROR;
    } else if (Tcl_GetIndexFromObj(interp, objv[1], ensembleSubcommands,
	    "subcommand", 0, &index) != TCL_OK) {
	return TCL_ERROR;

```

tcl9.0 9.0.4, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEnsemble.c`, function `InitEnsembleFromOptions`, lines 255–301. Retained reference `ensemble-source208-9.0.4-tclEnsemble.c-255-301.txt`.

```text
 *
 *----------------------------------------------------------------------
 */
static Tcl_Command
InitEnsembleFromOptions(
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    Namespace *cxtPtr = nsPtr->parentPtr;
    Namespace *altFoundNsPtr, *actualCxtPtr;
    const char *name = nsPtr->name;
    Tcl_Size len;
    int allocatedMapFlag = 0;
    enum EnsCreateOpts index;
    Tcl_Command token;		/* The created ensemble command. */
    Namespace *foundNsPtr;
    const char *simpleName;
    /*
     * Defaults
     */
    Tcl_Obj *subcmdObj = NULL;
    Tcl_Obj *mapObj = NULL;
    int permitPrefix = 1;
    Tcl_Obj *unknownObj = NULL;
    Tcl_Obj *paramObj = NULL;

    /*
     * Parse the option list, applying type checks as we go. Note that we are
     * not incrementing any reference counts in the objects at this stage, so
     * the presence of an option multiple times won't cause any memory leaks.
     */

    for (; objc>1 ; objc-=2,objv+=2) {
	if (Tcl_GetIndexFromObj(interp, objv[0], ensembleCreateOptions,
		"option", 0, &index) != TCL_OK) {
	    goto error;
	}
	switch (index) {
	case CRT_CMD:
	    name = TclGetString(objv[1]);
	    cxtPtr = nsPtr;
	    continue;
	case CRT_SUBCMDS:
	    if (TclListObjLength(interp, objv[1], &len) != TCL_OK) {
		goto error;

```

tcl9.0 9.0.4, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEnsemble.c`, function `InitEnsembleFromOptions`, lines 393–414. Retained reference `ensemble-source208-9.0.4-tclEnsemble.c-393-414.txt`.

```text

    TclGetNamespaceForQualName(interp, name, cxtPtr,
	    TCL_CREATE_NS_IF_UNKNOWN, &foundNsPtr, &altFoundNsPtr,
	    &actualCxtPtr, &simpleName);

    /*
     * Create the ensemble. Note that this might delete another ensemble
     * linked to the same namespace, so we must be careful. However, we
     * should be OK because we only link the namespace into the list once
     * we've created it (and after any deletions have occurred.)
     */

    token = TclCreateEnsembleInNs(interp, simpleName,
	    (Tcl_Namespace *) foundNsPtr, (Tcl_Namespace *) nsPtr,
	    (permitPrefix ? TCL_ENSEMBLE_PREFIX : 0));
    Tcl_SetEnsembleSubcommandList(interp, token, subcmdObj);
    Tcl_SetEnsembleMappingDict(interp, token, mapObj);
    Tcl_SetEnsembleUnknownHandler(interp, token, unknownObj);
    Tcl_SetEnsembleParameterList(interp, token, paramObj);
    return token;

  error:

```

tcl9.0 9.0.4, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEnsemble.c`, function `Tcl_CreateEnsemble`, lines 830–851. Retained reference `ensemble-source208-9.0.4-tclEnsemble.c-830-851.txt`.

```text
Tcl_Command
Tcl_CreateEnsemble(
    Tcl_Interp *interp,
    const char *name,		/* The ensemble name. */
    Tcl_Namespace *namespacePtr,/* Context namespace. */
    int flags)			/* Whether we need exact matching and whether
				 * we bytecode-compile the ensemble's uses. */
{
    Namespace *nsPtr = (Namespace *) namespacePtr, *foundNsPtr, *altNsPtr,
	    *actualNsPtr;
    const char * simpleName;

    if (nsPtr == NULL) {
	nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    }

    TclGetNamespaceForQualName(interp, name, nsPtr, TCL_CREATE_NS_IF_UNKNOWN,
	    &foundNsPtr, &altNsPtr, &actualNsPtr, &simpleName);
    return TclCreateEnsembleInNs(interp, simpleName,
	    (Tcl_Namespace *) foundNsPtr, (Tcl_Namespace *) nsPtr, flags);
}


```

tcl9.1 9.1.0, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEnsemble.c`, function `TclNamespaceEnsembleCmd`, lines 151–176. Retained reference `ensemble-source208-9.1.0-tclEnsemble.c-151-176.txt`.

```text
TclNamespaceEnsembleCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    Tcl_Command token;		/* The ensemble command. */
    enum EnsSubcmds index;

    if (nsPtr == NULL || nsPtr->flags & NS_DEAD) {
	if (!Tcl_InterpDeleted(interp)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "tried to manipulate ensemble of deleted namespace",
		    TCL_AUTO_LENGTH));
	    Tcl_SetErrorCode(interp, "TCL", "ENSEMBLE", "DEAD", (char *)NULL);
	}
	return TCL_ERROR;
    }

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "subcommand ?arg ...?");
	return TCL_ERROR;
    } else if (Tcl_GetIndexFromObj(interp, objv[1], ensembleSubcommands,
	    "subcommand", 0, &index) != TCL_OK) {
	return TCL_ERROR;

```

tcl9.1 9.1.0, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEnsemble.c`, function `InitEnsembleFromOptions`, lines 255–301. Retained reference `ensemble-source208-9.1.0-tclEnsemble.c-255-301.txt`.

```text
 *
 *----------------------------------------------------------------------
 */
static Tcl_Command
InitEnsembleFromOptions(
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    Namespace *cxtPtr = nsPtr->parentPtr;
    Namespace *altFoundNsPtr, *actualCxtPtr;
    const char *name = nsPtr->name;
    Tcl_Size len;
    int allocatedMapFlag = 0;
    enum EnsCreateOpts index;
    Tcl_Command token;		/* The created ensemble command. */
    Namespace *foundNsPtr;
    const char *simpleName;
    /*
     * Defaults
     */
    Tcl_Obj *subcmdObj = NULL;
    Tcl_Obj *mapObj = NULL;
    int permitPrefix = 1;
    Tcl_Obj *unknownObj = NULL;
    Tcl_Obj *paramObj = NULL;

    /*
     * Parse the option list, applying type checks as we go. Note that we are
     * not incrementing any reference counts in the objects at this stage, so
     * the presence of an option multiple times won't cause any memory leaks.
     */

    for (; objc>1 ; objc-=2,objv+=2) {
	if (Tcl_GetIndexFromObj(interp, objv[0], ensembleCreateOptions,
		"option", 0, &index) != TCL_OK) {
	    goto error;
	}
	switch (index) {
	case CRT_CMD:
	    name = TclGetString(objv[1]);
	    cxtPtr = nsPtr;
	    continue;
	case CRT_SUBCMDS:
	    if (TclListObjLength(interp, objv[1], &len) != TCL_OK) {
		goto error;

```

tcl9.1 9.1.0, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEnsemble.c`, function `InitEnsembleFromOptions`, lines 393–414. Retained reference `ensemble-source208-9.1.0-tclEnsemble.c-393-414.txt`.

```text

    TclGetNamespaceForQualName(interp, name, cxtPtr,
	    TCL_CREATE_NS_IF_UNKNOWN, &foundNsPtr, &altFoundNsPtr,
	    &actualCxtPtr, &simpleName);

    /*
     * Create the ensemble. Note that this might delete another ensemble
     * linked to the same namespace, so we must be careful. However, we
     * should be OK because we only link the namespace into the list once
     * we've created it (and after any deletions have occurred.)
     */

    token = TclCreateEnsembleInNs(interp, simpleName,
	    (Tcl_Namespace *) foundNsPtr, (Tcl_Namespace *) nsPtr,
	    (permitPrefix ? TCL_ENSEMBLE_PREFIX : 0));
    Tcl_SetEnsembleSubcommandList(interp, token, subcmdObj);
    Tcl_SetEnsembleMappingDict(interp, token, mapObj);
    Tcl_SetEnsembleUnknownHandler(interp, token, unknownObj);
    Tcl_SetEnsembleParameterList(interp, token, paramObj);
    return token;

  error:

```

tcl9.1 9.1.0, revision `Source-control revision unrecorded; exact source/header hashes and release path retained independently.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEnsemble.c`, function `Tcl_CreateEnsemble`, lines 830–851. Retained reference `ensemble-source208-9.1.0-tclEnsemble.c-830-851.txt`.

```text
Tcl_Command
Tcl_CreateEnsemble(
    Tcl_Interp *interp,
    const char *name,		/* The ensemble name. */
    Tcl_Namespace *namespacePtr,/* Context namespace. */
    int flags)			/* Whether we need exact matching and whether
				 * we bytecode-compile the ensemble's uses. */
{
    Namespace *nsPtr = (Namespace *) namespacePtr, *foundNsPtr, *altNsPtr,
	    *actualNsPtr;
    const char * simpleName;

    if (nsPtr == NULL) {
	nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    }

    TclGetNamespaceForQualName(interp, name, nsPtr, TCL_CREATE_NS_IF_UNKNOWN,
	    &foundNsPtr, &altNsPtr, &actualNsPtr, &simpleName);
    return TclCreateEnsembleInNs(interp, simpleName,
	    (Tcl_Namespace *) foundNsPtr, (Tcl_Namespace *) nsPtr, flags);
}


```

## Consumer bindings

- [runtime/rust/src/cmd_namespace/native_holder_routing.rs](../../../../runtime/rust/src/cmd_namespace/native_holder_routing.rs), `cmd_namespace::native_holder_routing::original_ensemble_holder_source_controls_match_all_native_providers` (linked): Runtime compares all-six original four-case public source completions with separate prelude/body/query evaluations and unchanged provider-specific outputs.
- [rust/tcl-vm/src/interp/native_command_holder_tests.rs](../../../../rust/tcl-vm/src/interp/native_command_holder_tests.rs), `interp::native_command_holder_tests::original_ensemble_holder_source_controls_match_all_native_providers` (linked): VM compares the exact independently evaluated original all-six four-case completions without private token identity inference.
- [rust/tcl-syntax/src/naming/native/routing.rs](../../../../rust/tcl-syntax/src/naming/native/routing.rs), `naming::native::routing::tests::ensemble_default_parent_and_explicit_current_routing_remain_distinct` (linked): Pure selected recipe geometry retains actual-parent default versus explicit-current routing; this does not execute a provider or prove token state.
- [rust/tcl-registry/src/native_ensemble.rs](../../../../rust/tcl-registry/src/native_ensemble.rs), `native_ensemble::configuration_surface_tests::actual_namespace_lifetime_policy_matches_pinned_ensemble_entry` (linked): Selected source policy retains the independent C85/C86 NS_DYING versus C9 NS_DEAD entry distinction without observing a particular native namespace.

## Evidence

- [rust/tcl-registry/tests/data/native_ensemble_holder_routing205](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205)
- [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/8.4.20](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/8.4.20)
- [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/8.5.19](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/8.5.19)
- [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/8.6.18](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/8.6.18)
- [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/9.0.4](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/9.0.4)
- [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/9.1.0](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/jim](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/jim)
