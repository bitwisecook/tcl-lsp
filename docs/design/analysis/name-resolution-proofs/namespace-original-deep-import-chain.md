# naming.namespace.original-deep-import-chain

Kind: `native-observation`

## Problem statement

Can original command/procedure/ensemble introspection follow 200 imported bindings, including depths 63,64,65 and200, without a fixed64 hop truncation? What do actual providers report after source rename, an unrelated replacement at the vacated spelling, middle import replacement, source deletion/recreation and ensemble configuration through a deep import? Genuine generation/token lifetimes must not be replaced by textual spelling heuristics.

## Question

How do original namespace origin, procedure args/body/default/invocation and supported ensemble queries behave across depths 63,64,65 and 200 after source rename, vacated replacement, middle replacement and deletion/recreation?

## Conclusion

All five pinned C releases and current Jim complete the three procedure-chain setups with 200 namespace bindings. Public origin and invocation succeed at depths 63,64,65 and 200. C procedure args/body/default queries resolve the original procedure at those depths. After source rename and a new procedure at the vacated spelling, C deep imports report the moved origin and ORIGINAL; replacing the middle binding changes origin to n100 and returns REPLACED; deleting the source invalidates deep imports, and recreating its spelling does not revive them. Current Jim uses its scripted alias channel: initial origin/invocation succeeds at those depths, but args/body report that the alias is not a procedure and info default is unsupported. After source rename Jim observes the vacated replacement and returns VACATED; after deletion/recreation its alias target becomes RECREATED. Jim middle replacement also resolves n100. C8.5 through C9.1 additionally retain ensemble exists/map/invocation through all four depths, configuration via the deep import and source rename, while ensemble args remains a nonprocedure query. C8.4 lacks namespace ensemble; Jim rejects the measured create form and supports neither the measured exists nor configure form. Their failed ensemble setup rows remain purpose limitations, not successful chain observations. Every original process exits `0` with empty stderr and 95 public rows. The exact observations reject a 64-hop truncation for these successful purposes, but do not establish an arbitrary unbounded depth or private token/header identity.

Two additional marked Runtime source definitions are registered through the actual parent test module. The public comparator selects546 original CASE rows from the unchanged six-provider probe, keeping24 metadata rows outside its comparison and explicit Jim distribution helper selection. A separate deliberately malformed software generation cycle checks terminal refusal without a partial query origin.

## Scope

The immutable original ASCII LF source creates four independently named groups for rename/vacated replacement, middle replacement, deletion/recreation and ensemble queries. Depth samples are63,64,65 and 200. Every query records caught code and counted public result hex; mandatory setup rows gate each group, so unsupported C8.4/Jim ensemble setup cannot supply positive ensemble behavior. Version and executable queries are fresh public rows in each same original process; current Jim namespace origin/import helper bodies are independently returned by public info body. The listed 137 original CLI/header/static archive/configuration/source/distribution library inputs remain exact before and after. No new compile, private command token, object/header/cache, compiled opcode or Native frame is sampled. Twenty-three exact original source windows explain C original-command/procedure/namespace/ensemble routes and Jim scripted helper/alias behavior separately from observed public results. Recorded absolute paths, explicit TCL_LIBRARY overlays and original launch arguments are retained; compiler version, unlisted inherited environment and unrelated system libraries are unrecorded. Generic aliases, invocation identity, future lookup, handler entry, Normal beyond each caught query, editor equivalence and BIG-IP do not follow. Existing two-hop and other original records remain unchanged. Runtime traversal tests and current source contracts require independent software receipts; publication supplies no backend assertion PASS.

Module registration establishes source reachability only, not an executable inventory, launched assertion or passing comparison. The original six processes,570 raw rows and all provider answers remain unchanged. The software cycle has no Native counterpart, and Jim scripted alias observations cannot become200 genuine C-style Imported generations or private token/header identity. No arbitrary-depth, frame, handler or edit authority follows.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Exact original fixed ASCII LF distribution CLI source; independently set up 200-binding groups, public catch/result hex and same-process version/helper queries. Version: tcl8.4 8.4.20.

At all four depths origin/args/body/default/invoke resolve the original procedure. Source rename retains moved origin/ORIGINAL despite vacated replacement; middle replacement resolves n100/REPLACED; deletion and recreation leave old imports invalid. Measured namespace ensemble setup is unsupported; no positive ensemble chain result is inferred. Whole original process has 95 rows, exit `0` and empty stderr.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact original fixed ASCII LF distribution CLI source; independently set up 200-binding groups, public catch/result hex and same-process version/helper queries. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

At all four depths origin/args/body/default/invoke resolve the original procedure. Source rename retains moved origin/ORIGINAL despite vacated replacement; middle replacement resolves n100/REPLACED; deletion and recreation leave old imports invalid. Ensemble exists/map/invoke and configuration through depth 200 retain the same public target after source rename; args remains a nonprocedure query. Whole original process has 95 rows, exit `0` and empty stderr.

### jim

Status: `observed`. Channel: Exact original fixed ASCII LF distribution CLI source; independently set up 200-binding groups, public catch/result hex and same-process version/helper queries. Version: jim 0.84-9-g5bac7c9.

At all four depth samples public origin/invoke succeeds; args/body reports aliases are not procedures and info default is unsupported. Rename plus vacated replacement returns VACATED; middle replacement resolves n100/REPLACED; delete/recreate returns RECREATED. Measured ensemble setup/exists/configure is unsupported. Actual namespace helper bodies/version are separately returned in the same original process. Whole original process has 95 rows, exit `0` and empty stderr.

### bigip

Status: `not-tested`.

No BIG-IP process, import chain, query, token or behavior transfer is measured by this request.

## Source inspection

tcl8.4 8.4.20, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclNamesp.c`, function `TclGetOriginalCommand`, lines 1445–1548. Retained reference `native_ensemble_map_prefix312-map-source-8.4.20-tclNamesp.c`.

```text
 *	an imported command, the procedure returns NULL.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
TclGetOriginalCommand(command)
    Tcl_Command command;	/* The imported command for which the
				 * original command should be returned. */
{
    register Command *cmdPtr = (Command *) command;
    ImportedCmdData *dataPtr;

    if (cmdPtr->deleteProc != DeleteImportedCmd) {
	return (Tcl_Command) NULL;
    }
    
    while (cmdPtr->deleteProc == DeleteImportedCmd) {
	dataPtr = (ImportedCmdData *) cmdPtr->objClientData;
	cmdPtr = dataPtr->realCmdPtr;
    }
    return (Tcl_Command) cmdPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * InvokeImportedCmd --
 *
 *	Invoked by Tcl whenever the user calls an imported command that
 *	was created by Tcl_Import. Finds the "real" command (in another
 *	namespace), and passes control to it.
 *
 * Results:
 *	Returns TCL_OK if successful, and  TCL_ERROR if anything goes wrong.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If anything
 *	goes wrong, the result object is set to an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InvokeImportedCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Points to the imported command's
				 * ImportedCmdData structure. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* The argument objects. */
{
    register ImportedCmdData *dataPtr = (ImportedCmdData *) clientData;
    register Command *realCmdPtr = dataPtr->realCmdPtr;

    return (*realCmdPtr->objProc)(realCmdPtr->objClientData, interp,
            objc, objv);
}

/*
 *----------------------------------------------------------------------
 *
 * DeleteImportedCmd --
 *
 *	Invoked by Tcl whenever an imported command is deleted. The "real"
 *	command keeps a list of all the imported commands that refer to it,
 *	so those imported commands can be deleted when the real command is
 *	deleted. This procedure removes the imported command reference from
 *	the real command's list, and frees up the memory associated with
 *	the imported command.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Removes the imported command from the real command's import list.
 *
 *----------------------------------------------------------------------
 */

static void
DeleteImportedCmd(clientData)
    ClientData clientData;	/* Points to the imported command's
				 * ImportedCmdData structure. */
{
    ImportedCmdData *dataPtr = (ImportedCmdData *) clientData;
    Command *realCmdPtr = dataPtr->realCmdPtr;
    Command *selfPtr = dataPtr->selfPtr;
    register ImportRef *refPtr, *prevPtr;

    prevPtr = NULL;
    for (refPtr = realCmdPtr->importRefPtr;  refPtr != NULL;
            refPtr = refPtr->nextPtr) {
	if (refPtr->importedCmdPtr == selfPtr) {
	    /*
	     * Remove *refPtr from real command's list of imported commands
	     * that refer to it.
	     */
	    
	    if (prevPtr == NULL) { /* refPtr is first in list */
		realCmdPtr->importRefPtr = refPtr->nextPtr;
	    } else {

```

tcl8.4 8.4.20, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclNamesp.c`, function `NamespaceOriginCmd`, lines 3478–3581. Retained reference `native_ensemble_map_prefix312-map-source-8.4.20-tclNamesp.c`.

```text
    }

    command = Tcl_GetCommandFromObj(interp, objv[2]);
    if (command == (Tcl_Command) NULL) {
	Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		"invalid command name \"", Tcl_GetString(objv[2]),
		"\"", (char *) NULL);
	return TCL_ERROR;
    }
    origCommand = TclGetOriginalCommand(command);
    if (origCommand == (Tcl_Command) NULL) {
	/*
	 * The specified command isn't an imported command. Return the
	 * command's name qualified by the full name of the namespace it
	 * was defined in.
	 */
	
	Tcl_GetCommandFullName(interp, command, Tcl_GetObjResult(interp));
    } else {
	Tcl_GetCommandFullName(interp, origCommand, Tcl_GetObjResult(interp));
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceParentCmd --
 *
 *	Invoked to implement the "namespace parent" command that returns the
 *	fully-qualified name of the parent namespace for a specified
 *	namespace. Handles the following syntax:
 *
 *	    namespace parent ?name?
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If anything
 *	goes wrong, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
NamespaceParentCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    Tcl_Namespace *nsPtr;
    int result;

    if (objc == 2) {
        nsPtr = Tcl_GetCurrentNamespace(interp);
    } else if (objc == 3) {
	result = GetNamespaceFromObj(interp, objv[2], &nsPtr);
        if (result != TCL_OK) {
            return result;
        }
        if (nsPtr == NULL) {
            Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
                    "unknown namespace \"", Tcl_GetString(objv[2]),
		    "\" in namespace parent command", (char *) NULL);
            return TCL_ERROR;
        }
    } else {
        Tcl_WrongNumArgs(interp, 2, objv, "?name?");
        return TCL_ERROR;
    }

    /*
     * Report the parent of the specified namespace.
     */

    if (nsPtr->parentPtr != NULL) {
        Tcl_SetStringObj(Tcl_GetObjResult(interp),
	        nsPtr->parentPtr->fullName, -1);
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceQualifiersCmd --
 *
 *	Invoked to implement the "namespace qualifiers" command that returns
 *	any leading namespace qualifiers in a string. These qualifiers are
 *	namespace names separated by "::"s. For example, for "::foo::p" this
 *	command returns "::foo", and for "::" it returns "". This command
 *	is the complement of the "namespace tail" command. Note that this
 *	command does not check whether the "namespace" names are, in fact,
 *	the names of currently defined namespaces. Handles the following
 *	syntax:
 *
 *	    namespace qualifiers string
 *
 * Results:
 *	Returns TCL_OK if successful, and  TCL_ERROR if anything goes wrong.
 *
 * Side effects:

```

tcl8.4 8.4.20, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclProc.c`, function `TclFindProc`, lines 808–941. Retained reference `native_child_colon_publication-full-source-8.4.20-tclProc.c`.

```text
 *	looked up using the usual rules: first in the current
 *	namespace and then in the global namespace.
 *
 * Results:
 *	NULL is returned if the name doesn't correspond to any
 *	procedure. Otherwise, the return value is a pointer to
 *	the procedure's record. If the name is found but refers
 *	to an imported command that points to a "real" procedure
 *	defined in another namespace, a pointer to that "real"
 *	procedure's structure is returned.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Proc *
TclFindProc(iPtr, procName)
    Interp *iPtr;		/* Interpreter in which to look. */
    CONST char *procName;		/* Name of desired procedure. */
{
    Tcl_Command cmd;
    Tcl_Command origCmd;
    Command *cmdPtr;

    cmd = Tcl_FindCommand((Tcl_Interp *) iPtr, procName,
            (Tcl_Namespace *) NULL, /*flags*/ 0);
    if (cmd == (Tcl_Command) NULL) {
        return NULL;
    }
    cmdPtr = (Command *) cmd;

    origCmd = TclGetOriginalCommand(cmd);
    if (origCmd != NULL) {
	cmdPtr = (Command *) origCmd;
    }
    if (cmdPtr->proc != TclProcInterpProc) {
	return NULL;
    }
    return (Proc *) cmdPtr->clientData;
}

/*
 *----------------------------------------------------------------------
 *
 * TclIsProc --
 *
 *	Tells whether a command is a Tcl procedure or not.
 *
 * Results:
 *	If the given command is actually a Tcl procedure, the
 *	return value is the address of the record describing
 *	the procedure.  Otherwise the return value is 0.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Proc *
TclIsProc(cmdPtr)
    Command *cmdPtr;		/* Command to test. */
{
    Tcl_Command origCmd;

    origCmd = TclGetOriginalCommand((Tcl_Command) cmdPtr);
    if (origCmd != NULL) {
	cmdPtr = (Command *) origCmd;
    }
    if (cmdPtr->proc == TclProcInterpProc) {
	return (Proc *) cmdPtr->clientData;
    }
    return (Proc *) 0;
}

/*
 *----------------------------------------------------------------------
 *
 * TclProcInterpProc --
 *
 *	When a Tcl procedure gets invoked with an argc/argv array of
 *	strings, this routine gets invoked to interpret the procedure.
 *
 * Results:
 *	A standard Tcl result value, usually TCL_OK.
 *
 * Side effects:
 *	Depends on the commands in the procedure.
 *
 *----------------------------------------------------------------------
 */

int
TclProcInterpProc(clientData, interp, argc, argv)
    ClientData clientData;	/* Record describing procedure to be
				 * interpreted. */
    Tcl_Interp *interp;		/* Interpreter in which procedure was
				 * invoked. */
    int argc;			/* Count of number of arguments to this
				 * procedure. */
    register CONST char **argv;	/* Argument values. */
{
    register Tcl_Obj *objPtr;
    register int i;
    int result;

    /*
     * This procedure generates an objv array for object arguments that hold
     * the argv strings. It starts out with stack-allocated space but uses
     * dynamically-allocated storage if needed.
     */

#define NUM_ARGS 20
    Tcl_Obj *(objStorage[NUM_ARGS]);
    register Tcl_Obj **objv = objStorage;

    /*
     * Create the object argument array "objv". Make sure objv is large
     * enough to hold the objc arguments plus 1 extra for the zero
     * end-of-objv word.
     */

    if ((argc + 1) > NUM_ARGS) {
	objv = (Tcl_Obj **)
	    ckalloc((unsigned)(argc + 1) * sizeof(Tcl_Obj *));
    }

    for (i = 0;  i < argc;  i++) {
	objv[i] = Tcl_NewStringObj(argv[i], -1);
	Tcl_IncrRefCount(objv[i]);
    }
    objv[argc] = 0;

```

tcl8.5 8.5.19, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `TclGetOriginalCommand`, lines 1867–1970. Retained reference `native_ensemble_holder_routing205-source-inspection208-8.5.19-tclNamesp.c`.

```text
 */

Tcl_Command
TclGetOriginalCommand(
    Tcl_Command command)	/* The imported command for which the original
				 * command should be returned. */
{
    register Command *cmdPtr = (Command *) command;
    ImportedCmdData *dataPtr;

    if (cmdPtr->deleteProc != DeleteImportedCmd) {
	return NULL;
    }

    while (cmdPtr->deleteProc == DeleteImportedCmd) {
	dataPtr = cmdPtr->objClientData;
	cmdPtr = dataPtr->realCmdPtr;
    }
    return (Tcl_Command) cmdPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * InvokeImportedCmd --
 *
 *	Invoked by Tcl whenever the user calls an imported command that was
 *	created by Tcl_Import. Finds the "real" command (in another
 *	namespace), and passes control to it.
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If anything goes
 *	wrong, the result object is set to an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InvokeImportedCmd(
    ClientData clientData,	/* Points to the imported command's
				 * ImportedCmdData structure. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* The argument objects. */
{
    register ImportedCmdData *dataPtr = clientData;
    register Command *realCmdPtr = dataPtr->realCmdPtr;

    return (*realCmdPtr->objProc)(realCmdPtr->objClientData, interp,
	    objc, objv);
}

/*
 *----------------------------------------------------------------------
 *
 * DeleteImportedCmd --
 *
 *	Invoked by Tcl whenever an imported command is deleted. The "real"
 *	command keeps a list of all the imported commands that refer to it, so
 *	those imported commands can be deleted when the real command is
 *	deleted. This function removes the imported command reference from the
 *	real command's list, and frees up the memory associated with the
 *	imported command.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Removes the imported command from the real command's import list.
 *
 *----------------------------------------------------------------------
 */

static void
DeleteImportedCmd(
    ClientData clientData)	/* Points to the imported command's
				 * ImportedCmdData structure. */
{
    ImportedCmdData *dataPtr = clientData;
    Command *realCmdPtr = dataPtr->realCmdPtr;
    Command *selfPtr = dataPtr->selfPtr;
    register ImportRef *refPtr, *prevPtr;

    prevPtr = NULL;
    for (refPtr = realCmdPtr->importRefPtr; refPtr != NULL;
	    refPtr = refPtr->nextPtr) {
	if (refPtr->importedCmdPtr == selfPtr) {
	    /*
	     * Remove *refPtr from real command's list of imported commands
	     * that refer to it.
	     */

	    if (prevPtr == NULL) { /* refPtr is first in list. */
		realCmdPtr->importRefPtr = refPtr->nextPtr;
	    } else {
		prevPtr->nextPtr = refPtr->nextPtr;
	    }
	    ckfree((char *) refPtr);
	    ckfree((char *) dataPtr);
	    return;
	}

```

tcl8.5 8.5.19, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceOriginCmd`, lines 3810–3913. Retained reference `native_ensemble_holder_routing205-source-inspection208-8.5.19-tclNamesp.c`.

```text
	Tcl_AppendResult(interp, "invalid command name \"",
		TclGetString(objv[2]), "\"", NULL);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "COMMAND",
		TclGetString(objv[2]), NULL);
	return TCL_ERROR;
    }
    origCommand = TclGetOriginalCommand(command);
    TclNewObj(resultPtr);
    if (origCommand == NULL) {
	/*
	 * The specified command isn't an imported command. Return the
	 * command's name qualified by the full name of the namespace it was
	 * defined in.
	 */

	Tcl_GetCommandFullName(interp, command, resultPtr);
    } else {
	Tcl_GetCommandFullName(interp, origCommand, resultPtr);
    }
    Tcl_SetObjResult(interp, resultPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceParentCmd --
 *
 *	Invoked to implement the "namespace parent" command that returns the
 *	fully-qualified name of the parent namespace for a specified
 *	namespace. Handles the following syntax:
 *
 *	    namespace parent ?name?
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If anything goes
 *	wrong, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
NamespaceParentCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Namespace *nsPtr;

    if (objc == 2) {
	nsPtr = TclGetCurrentNamespace(interp);
    } else if (objc == 3) {
	if (TclGetNamespaceFromObj(interp, objv[2], &nsPtr) != TCL_OK) {
	    return TCL_ERROR;
	}
    } else {
	Tcl_WrongNumArgs(interp, 2, objv, "?name?");
	return TCL_ERROR;
    }

    /*
     * Report the parent of the specified namespace.
     */

    if (nsPtr->parentPtr != NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		nsPtr->parentPtr->fullName, -1));
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespacePathCmd --
 *
 *	Invoked to implement the "namespace path" command that reads and
 *	writes the current namespace's command resolution path. Has one
 *	optional argument: if present, it is a list of named namespaces to set
 *	the path to, and if absent, the current path should be returned.
 *	Handles the following syntax:
 *
 *	    namespace path ?nsList?
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong
 *	(most notably if the namespace list contains the name of something
 *	other than a namespace). In the successful-exit case, may set the
 *	interpreter result to the list of names of the namespaces on the
 *	current namespace's path.
 *
 * Side effects:
 *	May update the namespace path (triggering a recomputing of all command
 *	names that depend on the namespace for resolution).
 *
 *----------------------------------------------------------------------
 */

static int
NamespacePathCmd(

```

tcl8.5 8.5.19, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclNamesp.c`, function `Tcl_FindEnsemble`, lines 5902–6039. Retained reference `native_ensemble_holder_routing205-source-inspection208-8.5.19-tclNamesp.c`.

```text
 *	ensemble commands as well.
 *
 * Results:
 *	Boolean value
 *
 * Side effects:
 *	None
 *
 *----------------------------------------------------------------------
 */

int
Tcl_IsEnsemble(
    Tcl_Command token)
{
    Command *cmdPtr = (Command *) token;
    if (cmdPtr->objProc == NsEnsembleImplementationCmd) {
	return 1;
    }
    cmdPtr = (Command *) TclGetOriginalCommand((Tcl_Command) cmdPtr);
    if (cmdPtr == NULL || cmdPtr->objProc != NsEnsembleImplementationCmd) {
	return 0;
    }
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * TclMakeEnsemble --
 *
 *	Create an ensemble from a table of implementation commands. The
 *	ensemble will be subject to (limited) compilation if any of the
 *	implementation commands are compilable.
 *
 * Results:
 *	Handle for the ensemble, or NULL if creation of it fails.
 *
 * Side effects:
 *	May advance bytecode compilation epoch.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
TclMakeEnsemble(
    Tcl_Interp *interp,
    const char *name,
    const EnsembleImplMap map[])
{
    Tcl_Command ensemble;	/* The overall ensemble. */
    Tcl_Namespace *tclNsPtr;	/* Reference to the "::tcl" namespace. */
    Tcl_DString buf;

    tclNsPtr = Tcl_FindNamespace(interp, "::tcl", NULL,
	    TCL_CREATE_NS_IF_UNKNOWN);
    if (tclNsPtr == NULL) {
	Tcl_Panic("unable to find or create ::tcl namespace!");
    }
    Tcl_DStringInit(&buf);
    Tcl_DStringAppend(&buf, "::tcl::", -1);
    Tcl_DStringAppend(&buf, name, -1);
    tclNsPtr = Tcl_FindNamespace(interp, Tcl_DStringValue(&buf), NULL,
	    TCL_CREATE_NS_IF_UNKNOWN);
    if (tclNsPtr == NULL) {
	Tcl_Panic("unable to find or create %s namespace!",
		Tcl_DStringValue(&buf));
    }
    ensemble = Tcl_CreateEnsemble(interp, Tcl_DStringValue(&buf)+5, tclNsPtr,
	    TCL_ENSEMBLE_PREFIX);
    Tcl_DStringAppend(&buf, "::", -1);
    if (ensemble != NULL) {
	Tcl_Obj *mapDict;
	int i, compile = 0;

	TclNewObj(mapDict);
	for (i=0 ; map[i].name != NULL ; i++) {
	    Tcl_Obj *fromObj, *toObj;
	    Command *cmdPtr;

	    fromObj = Tcl_NewStringObj(map[i].name, -1);
	    TclNewStringObj(toObj, Tcl_DStringValue(&buf),
		    Tcl_DStringLength(&buf));
	    Tcl_AppendToObj(toObj, map[i].name, -1);
	    Tcl_DictObjPut(NULL, mapDict, fromObj, toObj);
	    cmdPtr = (Command *) Tcl_CreateObjCommand(interp,
		    TclGetString(toObj), map[i].proc, NULL, NULL);
	    cmdPtr->compileProc = map[i].compileProc;
	    compile |= (map[i].compileProc != NULL);
	}
	Tcl_SetEnsembleMappingDict(interp, ensemble, mapDict);
	if (compile) {
	    Tcl_SetEnsembleFlags(interp, ensemble,
		    TCL_ENSEMBLE_PREFIX | ENSEMBLE_COMPILE);
	}
    }
    Tcl_DStringFree(&buf);

    return ensemble;
}

/*
 *----------------------------------------------------------------------
 *
 * NsEnsembleImplementationCmd --
 *
 *	Implements an ensemble of commands (being those exported by a
 *	namespace other than the global namespace) as a command with the same
 *	(short) name as the namespace in the parent namespace.
 *
 * Results:
 *	A standard Tcl result code. Will be TCL_ERROR if the command is not an
 *	unambiguous prefix of any command exported by the ensemble's
 *	namespace.
 *
 * Side effects:
 *	Depends on the command within the namespace that gets executed. If the
 *	ensemble itself returns TCL_ERROR, a descriptive error message will be
 *	placed in the interpreter's result.
 *
 *----------------------------------------------------------------------
 */

static int
NsEnsembleImplementationCmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    EnsembleConfig *ensemblePtr = clientData;
				/* The ensemble itself. */
    Tcl_Obj **tempObjv;		/* Space used to construct the list of
				 * arguments to pass to the command that
				 * implements the ensemble subcommand. */
    int result;			/* The result of the subcommand execution. */
    Tcl_Obj *prefixObj;		/* An object containing the prefix words of
				 * the command that implements the

```

tcl8.5 8.5.19, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclProc.c`, function `TclFindProc`, lines 978–1111. Retained reference `native_compiled_local_formal_source272-8.5.19-tclProc.c`.

```text
 *	usual rules: first in the current namespace and then in the global
 *	namespace.
 *
 * Results:
 *	NULL is returned if the name doesn't correspond to any procedure.
 *	Otherwise, the return value is a pointer to the procedure's record. If
 *	the name is found but refers to an imported command that points to a
 *	"real" procedure defined in another namespace, a pointer to that
 *	"real" procedure's structure is returned.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Proc *
TclFindProc(
    Interp *iPtr,		/* Interpreter in which to look. */
    CONST char *procName)	/* Name of desired procedure. */
{
    Tcl_Command cmd;
    Tcl_Command origCmd;
    Command *cmdPtr;

    cmd = Tcl_FindCommand((Tcl_Interp *) iPtr, procName, NULL, /*flags*/ 0);
    if (cmd == (Tcl_Command) NULL) {
	return NULL;
    }
    cmdPtr = (Command *) cmd;

    origCmd = TclGetOriginalCommand(cmd);
    if (origCmd != NULL) {
	cmdPtr = (Command *) origCmd;
    }
    if (cmdPtr->objProc != TclObjInterpProc) {
	return NULL;
    }
    return (Proc *) cmdPtr->objClientData;
}

/*
 *----------------------------------------------------------------------
 *
 * TclIsProc --
 *
 *	Tells whether a command is a Tcl procedure or not.
 *
 * Results:
 *	If the given command is actually a Tcl procedure, the return value is
 *	the address of the record describing the procedure. Otherwise the
 *	return value is 0.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Proc *
TclIsProc(
    Command *cmdPtr)		/* Command to test. */
{
    Tcl_Command origCmd;

    origCmd = TclGetOriginalCommand((Tcl_Command) cmdPtr);
    if (origCmd != NULL) {
	cmdPtr = (Command *) origCmd;
    }
    if (cmdPtr->objProc == TclObjInterpProc) {
	return (Proc *) cmdPtr->objClientData;
    }
    return (Proc *) 0;
}

/*
 *----------------------------------------------------------------------
 *
 * InitArgsAndLocals --
 *
 *	This routine is invoked in order to initialize the arguments and other
 *	compiled locals table for a new call frame.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	Allocates memory on the stack for the compiled local variables, the
 *	caller is responsible for freeing them. Initialises all variables. May
 *	invoke various name resolvers in order to determine which variables
 *	are being referenced at runtime.
 *
 *----------------------------------------------------------------------
 */

static int
ProcWrongNumArgs(
    Tcl_Interp *interp, int skip)
{
    CallFrame *framePtr = ((Interp *)interp)->varFramePtr;
    register Proc *procPtr = framePtr->procPtr;
    register Var *defPtr;
    int localCt = procPtr->numCompiledLocals, numArgs, i;
    Tcl_Obj **desiredObjs;
    const char *final = NULL;

    /*
     * Build up desired argument list for Tcl_WrongNumArgs
     */

    numArgs = framePtr->procPtr->numArgs;
    desiredObjs = (Tcl_Obj **) TclStackAlloc(interp,
	    (int) sizeof(Tcl_Obj *) * (numArgs+1));

    if (framePtr->isProcCallFrame & FRAME_IS_LAMBDA) {
	desiredObjs[0] = Tcl_NewStringObj("lambdaExpr", -1);
    } else {
	((Interp *) interp)->ensembleRewrite.numInsertedObjs -= skip - 1;

#ifdef AVOID_HACKS_FOR_ITCL
	desiredObjs[0] = framePtr->objv[skip-1];
#else
	desiredObjs[0] = Tcl_NewListObj(skip, framePtr->objv);
#endif /* AVOID_HACKS_FOR_ITCL */
    }
    Tcl_IncrRefCount(desiredObjs[0]);

    defPtr = (Var *) (&framePtr->localCachePtr->varName0 + localCt);
    for (i=1 ; i<=numArgs ; i++, defPtr++) {
	Tcl_Obj *argObj;
	Tcl_Obj *namePtr = localName(framePtr, i-1);

	if (defPtr->value.objPtr != NULL) {
	    TclNewObj(argObj);

```

tcl8.6 8.6.18, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclNamesp.c`, function `TclGetOriginalCommand`, lines 1918–2021. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclNamesp.c`.

```text

Tcl_Command
TclGetOriginalCommand(
    Tcl_Command command)	/* The imported command for which the original
				 * command should be returned. */
{
    Command *cmdPtr = (Command *) command;

    if (cmdPtr->deleteProc != DeleteImportedCmd) {
	return NULL;
    }

    while (cmdPtr->deleteProc == DeleteImportedCmd) {
	ImportedCmdData *dataPtr = (ImportedCmdData *)cmdPtr->objClientData;
	cmdPtr = dataPtr->realCmdPtr;
    }
    return (Tcl_Command) cmdPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * InvokeImportedCmd --
 *
 *	Invoked by Tcl whenever the user calls an imported command that was
 *	created by Tcl_Import. Finds the "real" command (in another
 *	namespace), and passes control to it.
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If anything goes
 *	wrong, the result object is set to an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InvokeImportedNRCmd(
    ClientData clientData,	/* Points to the imported command's
				 * ImportedCmdData structure. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* The argument objects. */
{
    ImportedCmdData *dataPtr = (ImportedCmdData *)clientData;
    Command *realCmdPtr = dataPtr->realCmdPtr;

    TclSkipTailcall(interp);
    return TclNREvalObjv(interp, objc, objv, TCL_EVAL_NOERR, realCmdPtr);
}

static int
InvokeImportedCmd(
    ClientData clientData,	/* Points to the imported command's
				 * ImportedCmdData structure. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* The argument objects. */
{
    return Tcl_NRCallObjProc(interp, InvokeImportedNRCmd, clientData,
	    objc, objv);
}

/*
 *----------------------------------------------------------------------
 *
 * DeleteImportedCmd --
 *
 *	Invoked by Tcl whenever an imported command is deleted. The "real"
 *	command keeps a list of all the imported commands that refer to it, so
 *	those imported commands can be deleted when the real command is
 *	deleted. This function removes the imported command reference from the
 *	real command's list, and frees up the memory associated with the
 *	imported command.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Removes the imported command from the real command's import list.
 *
 *----------------------------------------------------------------------
 */

static void
DeleteImportedCmd(
    ClientData clientData)	/* Points to the imported command's
				 * ImportedCmdData structure. */
{
    ImportedCmdData *dataPtr = (ImportedCmdData *)clientData;
    Command *realCmdPtr = dataPtr->realCmdPtr;
    Command *selfPtr = dataPtr->selfPtr;
    ImportRef *refPtr, *prevPtr;

    prevPtr = NULL;
    for (refPtr = realCmdPtr->importRefPtr; refPtr != NULL;
	    refPtr = refPtr->nextPtr) {
	if (refPtr->importedCmdPtr == selfPtr) {
	    /*
	     * Remove *refPtr from real command's list of imported commands
	     * that refer to it.
	     */

```

tcl8.6 8.6.18, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclNamesp.c`, function `NamespaceOriginCmd`, lines 3837–3940. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclNamesp.c`.

```text
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "COMMAND",
		TclGetString(objv[1]), (char *)NULL);
	return TCL_ERROR;
    }
    origCmd = TclGetOriginalCommand(cmd);
    TclNewObj(resultPtr);
    if (origCmd == NULL) {
	/*
	 * The specified command isn't an imported command. Return the
	 * command's name qualified by the full name of the namespace it was
	 * defined in.
	 */

	Tcl_GetCommandFullName(interp, cmd, resultPtr);
    } else {
	Tcl_GetCommandFullName(interp, origCmd, resultPtr);
    }
    Tcl_SetObjResult(interp, resultPtr);
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceParentCmd --
 *
 *	Invoked to implement the "namespace parent" command that returns the
 *	fully-qualified name of the parent namespace for a specified
 *	namespace. Handles the following syntax:
 *
 *	    namespace parent ?name?
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If anything goes
 *	wrong, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
NamespaceParentCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Namespace *nsPtr;

    if (objc == 1) {
	nsPtr = TclGetCurrentNamespace(interp);
    } else if (objc == 2) {
	if (TclGetNamespaceFromObj(interp, objv[1], &nsPtr) != TCL_OK) {
	    return TCL_ERROR;
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?name?");
	return TCL_ERROR;
    }

    /*
     * Report the parent of the specified namespace.
     */

    if (nsPtr->parentPtr != NULL) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		nsPtr->parentPtr->fullName, -1));
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespacePathCmd --
 *
 *	Invoked to implement the "namespace path" command that reads and
 *	writes the current namespace's command resolution path. Has one
 *	optional argument: if present, it is a list of named namespaces to set
 *	the path to, and if absent, the current path should be returned.
 *	Handles the following syntax:
 *
 *	    namespace path ?nsList?
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong
 *	(most notably if the namespace list contains the name of something
 *	other than a namespace). In the successful-exit case, may set the
 *	interpreter result to the list of names of the namespaces on the
 *	current namespace's path.
 *
 * Side effects:
 *	May update the namespace path (triggering a recomputing of all command
 *	names that depend on the namespace for resolution).
 *
 *----------------------------------------------------------------------
 */

static int
NamespacePathCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */

```

tcl8.6 8.6.18, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEnsemble.c`, function `Tcl_FindEnsemble`, lines 1402–1539. Retained reference `native_ensemble_holder_routing205-source-inspection208-8.6.18-tclEnsemble.c`.

```text
 *	None
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
Tcl_FindEnsemble(
    Tcl_Interp *interp,		/* Where to do the lookup, and where to write
				 * the errors if TCL_LEAVE_ERR_MSG is set in
				 * the flags. */
    Tcl_Obj *cmdNameObj,	/* Name of command to look up. */
    int flags)			/* Either 0 or TCL_LEAVE_ERR_MSG; other flags
				 * are probably not useful. */
{
    Command *cmdPtr;

    cmdPtr = (Command *)
	    Tcl_FindCommand(interp, TclGetString(cmdNameObj), NULL, flags);
    if (cmdPtr == NULL) {
	return NULL;
    }

    if (cmdPtr->objProc != NsEnsembleImplementationCmd) {
	/*
	 * Reuse existing infrastructure for following import link chains
	 * rather than duplicating it.
	 */

	cmdPtr = (Command *) TclGetOriginalCommand((Tcl_Command) cmdPtr);

	if (cmdPtr == NULL || cmdPtr->objProc != NsEnsembleImplementationCmd){
	    if (flags & TCL_LEAVE_ERR_MSG) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"\"%s\" is not an ensemble command",
			TclGetString(cmdNameObj)));
		Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ENSEMBLE",
			TclGetString(cmdNameObj), (char *)NULL);
	    }
	    return NULL;
	}
    }

    return (Tcl_Command) cmdPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_IsEnsemble --
 *
 *	Simple test for ensemble-hood that takes into account imported
 *	ensemble commands as well.
 *
 * Results:
 *	Boolean value
 *
 * Side effects:
 *	None
 *
 *----------------------------------------------------------------------
 */

int
Tcl_IsEnsemble(
    Tcl_Command token)
{
    Command *cmdPtr = (Command *) token;

    if (cmdPtr->objProc == NsEnsembleImplementationCmd) {
	return 1;
    }
    cmdPtr = (Command *) TclGetOriginalCommand((Tcl_Command) cmdPtr);
    if (cmdPtr == NULL || cmdPtr->objProc != NsEnsembleImplementationCmd) {
	return 0;
    }
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * TclMakeEnsemble --
 *
 *	Create an ensemble from a table of implementation commands. The
 *	ensemble will be subject to (limited) compilation if any of the
 *	implementation commands are compilable.
 *
 *	The 'name' parameter may be a single command name or a list if
 *	creating an ensemble subcommand (see the binary implementation).
 *
 *	Currently, the TCL_ENSEMBLE_PREFIX ensemble flag is only used on
 *	top-level ensemble commands.
 *
 * Results:
 *	Handle for the new ensemble, or NULL on failure.
 *
 * Side effects:
 *	May advance the bytecode compilation epoch.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
TclMakeEnsemble(
    Tcl_Interp *interp,
    const char *name,		 /* The ensemble name (as explained above) */
    const EnsembleImplMap map[]) /* The subcommands to create */
{
    Tcl_Command ensemble;
    Tcl_Namespace *ns;
    Tcl_DString buf, hiddenBuf;
    const char **nameParts = NULL;
    const char *cmdName = NULL;
    int i, nameCount = 0;
    int ensembleFlags = 0, hiddenLen;

    /*
     * Construct the path for the ensemble namespace and create it.
     */

    Tcl_DStringInit(&buf);
    Tcl_DStringInit(&hiddenBuf);
    TclDStringAppendLiteral(&hiddenBuf, "tcl:");
    Tcl_DStringAppend(&hiddenBuf, name, -1);
    TclDStringAppendLiteral(&hiddenBuf, ":");
    hiddenLen = Tcl_DStringLength(&hiddenBuf);
    if (name[0] == ':' && name[1] == ':') {
	/*
	 * An absolute name, so use it directly.
	 */

	cmdName = name;
	Tcl_DStringAppend(&buf, name, -1);
	ensembleFlags = TCL_ENSEMBLE_PREFIX;
    } else {
	/*
	 * Not an absolute name, so do munging of it. Note that this treats a
	 * multi-word list differently to a single word.

```

tcl8.6 8.6.18, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclProc.c`, function `TclFindProc`, lines 985–1118. Retained reference `native_compiled_local_formal_source272-8.6.18-tclProc.c`.

```text
 *	usual rules: first in the current namespace and then in the global
 *	namespace.
 *
 * Results:
 *	NULL is returned if the name doesn't correspond to any procedure.
 *	Otherwise, the return value is a pointer to the procedure's record. If
 *	the name is found but refers to an imported command that points to a
 *	"real" procedure defined in another namespace, a pointer to that
 *	"real" procedure's structure is returned.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Proc *
TclFindProc(
    Interp *iPtr,		/* Interpreter in which to look. */
    const char *procName)	/* Name of desired procedure. */
{
    Tcl_Command cmd;
    Command *cmdPtr;

    cmd = Tcl_FindCommand((Tcl_Interp *) iPtr, procName, NULL, /*flags*/ 0);
    if (cmd == (Tcl_Command) NULL) {
	return NULL;
    }
    cmdPtr = (Command *) cmd;

    return TclIsProc(cmdPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclIsProc --
 *
 *	Tells whether a command is a Tcl procedure or not.
 *
 * Results:
 *	If the given command is actually a Tcl procedure, the return value is
 *	the address of the record describing the procedure. Otherwise the
 *	return value is 0.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Proc *
TclIsProc(
    Command *cmdPtr)		/* Command to test. */
{
    Tcl_Command origCmd = TclGetOriginalCommand((Tcl_Command) cmdPtr);

    if (origCmd != NULL) {
	cmdPtr = (Command *) origCmd;
    }
    if (cmdPtr->deleteProc == TclProcDeleteProc) {
	return (Proc *)cmdPtr->objClientData;
    }
    return NULL;
}

static int
ProcWrongNumArgs(
    Tcl_Interp *interp,
    int skip)
{
    CallFrame *framePtr = ((Interp *)interp)->varFramePtr;
    Proc *procPtr = framePtr->procPtr;
    int localCt = procPtr->numCompiledLocals, numArgs, i;
    Tcl_Obj **desiredObjs;
    const char *final = NULL;

    /*
     * Build up desired argument list for Tcl_WrongNumArgs
     */

    numArgs = framePtr->procPtr->numArgs;
    desiredObjs = (Tcl_Obj **)TclStackAlloc(interp,
	    sizeof(Tcl_Obj *) * (numArgs+1));

    if (framePtr->isProcCallFrame & FRAME_IS_LAMBDA) {
	desiredObjs[0] = Tcl_NewStringObj("lambdaExpr", -1);
    } else {
#ifdef AVOID_HACKS_FOR_ITCL
	desiredObjs[0] = framePtr->objv[skip-1];
#else
	desiredObjs[0] = Tcl_NewListObj(1, framePtr->objv + skip - 1);
#endif /* AVOID_HACKS_FOR_ITCL */
    }
    Tcl_IncrRefCount(desiredObjs[0]);

    if (localCt > 0) {
	Var *defPtr = (Var *)(&framePtr->localCachePtr->varName0 + localCt);

	for (i=1 ; i<=numArgs ; i++, defPtr++) {
	    Tcl_Obj *argObj;
	    Tcl_Obj *namePtr = localName(framePtr, i-1);

	    if (defPtr->value.objPtr != NULL) {
		TclNewObj(argObj);
		Tcl_AppendStringsToObj(argObj, "?", TclGetString(namePtr), "?", (char *)NULL);
	    } else if (defPtr->flags & VAR_IS_ARGS) {
		numArgs--;
		final = "?arg ...?";
		break;
	    } else {
		argObj = namePtr;
		Tcl_IncrRefCount(namePtr);
	    }
	    desiredObjs[i] = argObj;
	}
    }

    Tcl_ResetResult(interp);
    Tcl_WrongNumArgs(interp, numArgs+1, desiredObjs, final);

    for (i=0 ; i<=numArgs ; i++) {
	Tcl_DecrRefCount(desiredObjs[i]);
    }
    TclStackFree(interp, desiredObjs);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * TclInitCompiledLocals --
 *
 *	This routine is invoked in order to initialize the compiled locals

```

tcl9.0 9.0.4, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclNamesp.c`, function `TclGetOriginalCommand`, lines 2077–2180. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclNamesp.c`.

```text
{
    Command *cmdPtr = (Command *) command;

    if (cmdPtr->deleteProc != DeleteImportedCmd) {
	return NULL;
    }

    while (cmdPtr->deleteProc == DeleteImportedCmd) {
	ImportedCmdData *dataPtr = (ImportedCmdData *) cmdPtr->objClientData;
	cmdPtr = dataPtr->realCmdPtr;
    }
    return (Tcl_Command) cmdPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * TclInvokeImportedCmd --
 *
 *	Invoked by Tcl whenever the user calls an imported command that was
 *	created by Tcl_Import. Finds the "real" command (in another
 *	namespace), and passes control to it.
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If anything goes
 *	wrong, the result object is set to an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InvokeImportedNRCmd(
    void *clientData,		/* Points to the imported command's
				 * ImportedCmdData structure. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* The argument objects. */
{
    ImportedCmdData *dataPtr = (ImportedCmdData *) clientData;
    Command *realCmdPtr = dataPtr->realCmdPtr;

    TclSkipTailcall(interp);
    return TclNREvalObjv(interp, objc, objv, TCL_EVAL_NOERR, realCmdPtr);
}

int
TclInvokeImportedCmd(
    void *clientData,		/* Points to the imported command's
				 * ImportedCmdData structure. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* The argument objects. */
{
    return Tcl_NRCallObjProc(interp, InvokeImportedNRCmd, clientData,
	    objc, objv);
}

/*
 *----------------------------------------------------------------------
 *
 * DeleteImportedCmd --
 *
 *	Invoked by Tcl whenever an imported command is deleted. The "real"
 *	command keeps a list of all the imported commands that refer to it, so
 *	those imported commands can be deleted when the real command is
 *	deleted. This function removes the imported command reference from the
 *	real command's list, and frees up the memory associated with the
 *	imported command.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Removes the imported command from the real command's import list.
 *
 *----------------------------------------------------------------------
 */

static void
DeleteImportedCmd(
    void *clientData)		/* Points to the imported command's
				 * ImportedCmdData structure. */
{
    ImportedCmdData *dataPtr = (ImportedCmdData *) clientData;
    Command *realCmdPtr = dataPtr->realCmdPtr;
    Command *selfPtr = dataPtr->selfPtr;
    ImportRef *refPtr, *prevPtr;

    prevPtr = NULL;
    for (refPtr = realCmdPtr->importRefPtr; refPtr != NULL;
	    refPtr = refPtr->nextPtr) {
	if (refPtr->importedCmdPtr == selfPtr) {
	    /*
	     * Remove *refPtr from real command's list of imported commands
	     * that refer to it.
	     */

	    if (prevPtr == NULL) { /* refPtr is first in list. */
		realCmdPtr->importRefPtr = refPtr->nextPtr;
	    } else {
		prevPtr->nextPtr = refPtr->nextPtr;

```

tcl9.0 9.0.4, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclNamesp.c`, function `NamespaceOriginCmd`, lines 4039–4142. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclNamesp.c`.

```text
    if (TclCheckEmptyString(resultPtr) == TCL_EMPTYSTRING_YES) {
	Tcl_DecrRefCount(resultPtr);
	goto namespaceOriginError;
    }
    Tcl_SetObjResult(interp, resultPtr);
    return TCL_OK;

  namespaceOriginError:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "invalid command name \"%s\"", TclGetString(objv[1])));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "COMMAND",
	    TclGetString(objv[1]), (char *)NULL);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceParentCmd --
 *
 *	Invoked to implement the "namespace parent" command that returns the
 *	fully-qualified name of the parent namespace for a specified
 *	namespace. Handles the following syntax:
 *
 *	    namespace parent ?name?
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If anything goes
 *	wrong, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
NamespaceParentCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Namespace *nsPtr;

    if (objc == 1) {
	nsPtr = TclGetCurrentNamespace(interp);
    } else if (objc == 2) {
	if (TclGetNamespaceFromObj(interp, objv[1], &nsPtr) != TCL_OK) {
	    return TCL_ERROR;
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?name?");
	return TCL_ERROR;
    }

    /*
     * Report the parent of the specified namespace.
     */

    if (nsPtr->parentPtr != NULL) {
	Tcl_SetObjResult(interp, TclNewNamespaceObj(nsPtr->parentPtr));
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespacePathCmd --
 *
 *	Invoked to implement the "namespace path" command that reads and
 *	writes the current namespace's command resolution path. Has one
 *	optional argument: if present, it is a list of named namespaces to set
 *	the path to, and if absent, the current path should be returned.
 *	Handles the following syntax:
 *
 *	    namespace path ?nsList?
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong
 *	(most notably if the namespace list contains the name of something
 *	other than a namespace). In the successful-exit case, may set the
 *	interpreter result to the list of names of the namespaces on the
 *	current namespace's path.
 *
 * Side effects:
 *	May update the namespace path (triggering a recomputing of all command
 *	names that depend on the namespace for resolution).
 *
 *----------------------------------------------------------------------
 */

static int
NamespacePathCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    Tcl_Size nsObjc, i;
    int result = TCL_ERROR;
    Tcl_Obj **nsObjv;

```

tcl9.0 9.0.4, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEnsemble.c`, function `Tcl_FindEnsemble`, lines 1470–1607. Retained reference `native_ensemble_holder_routing205-source-inspection208-9.0.4-tclEnsemble.c`.

```text
 */

Tcl_Command
Tcl_FindEnsemble(
    Tcl_Interp *interp,		/* Where to do the lookup, and where to write
				 * the errors if TCL_LEAVE_ERR_MSG is set in
				 * the flags. */
    Tcl_Obj *cmdNameObj,	/* Name of command to look up. */
    int flags)			/* Either 0 or TCL_LEAVE_ERR_MSG; other flags
				 * are probably not useful. */
{
    Tcl_Command token;

    token = Tcl_FindCommand(interp, TclGetString(cmdNameObj), NULL, flags);
    if (token == NULL) {
	return NULL;
    }

    if (((Command *) token)->objProc != TclEnsembleImplementationCmd) {
	/*
	 * Reuse existing infrastructure for following import link chains
	 * rather than duplicating it.
	 */

	token = TclGetOriginalCommand(token);

	if (token == NULL ||
		((Command *) token)->objProc != TclEnsembleImplementationCmd) {
	    if (flags & TCL_LEAVE_ERR_MSG) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"\"%s\" is not an ensemble command",
			TclGetString(cmdNameObj)));
		Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ENSEMBLE",
			TclGetString(cmdNameObj), (char *)NULL);
	    }
	    return NULL;
	}
    }

    return token;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_IsEnsemble --
 *
 *	Simple test for ensemble-hood that takes into account imported
 *	ensemble commands as well.
 *
 * Results:
 *	Boolean value
 *
 * Side effects:
 *	None
 *
 *----------------------------------------------------------------------
 */

int
Tcl_IsEnsemble(
    Tcl_Command token)		/* The command to check. */
{
    Command *cmdPtr = (Command *) token;

    if (cmdPtr->objProc == TclEnsembleImplementationCmd) {
	return 1;
    }
    cmdPtr = (Command *) TclGetOriginalCommand((Tcl_Command) cmdPtr);
    if (cmdPtr == NULL || cmdPtr->objProc != TclEnsembleImplementationCmd) {
	return 0;
    }
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * TclMakeEnsemble --
 *
 *	Create an ensemble from a table of implementation commands. The
 *	ensemble will be subject to (limited) compilation if any of the
 *	implementation commands are compilable.
 *
 *	The 'name' parameter may be a single command name or a list if
 *	creating an ensemble subcommand (see the binary implementation).
 *
 *	Currently, the TCL_ENSEMBLE_PREFIX ensemble flag is only used on
 *	top-level ensemble commands.
 *
 *	This code is not safe to run in Safe interpreter after user code has
 *	executed. That's OK right now because it's just used to set up Tcl,
 *	but it means we mustn't expose it at all, not even to Tk (until we can
 *	hide commands in namespaces directly).
 *
 * Results:
 *	Handle for the new ensemble, or NULL on failure.
 *
 * Side effects:
 *	May advance the bytecode compilation epoch.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
TclMakeEnsemble(
    Tcl_Interp *interp,
    const char *name,		/* The ensemble name (as explained above) */
    const EnsembleImplMap map[])/* The subcommands to create */
{
    Tcl_Command ensemble;
    Tcl_Namespace *ns;
    Tcl_DString buf, hiddenBuf;
    const char **nameParts = NULL;
    const char *cmdName = NULL;
    Tcl_Size i, nameCount = 0;
    int ensembleFlags = 0, hiddenLen;

    /*
     * Construct the path for the ensemble namespace and create it.
     */

    Tcl_DStringInit(&buf);
    Tcl_DStringInit(&hiddenBuf);
    TclDStringAppendLiteral(&hiddenBuf, "tcl:");
    Tcl_DStringAppend(&hiddenBuf, name, TCL_AUTO_LENGTH);
    TclDStringAppendLiteral(&hiddenBuf, ":");
    hiddenLen = Tcl_DStringLength(&hiddenBuf);
    if (name[0] == ':' && name[1] == ':') {
	/*
	 * An absolute name, so use it directly.
	 */

	cmdName = name;
	Tcl_DStringAppend(&buf, name, TCL_AUTO_LENGTH);
	ensembleFlags = TCL_ENSEMBLE_PREFIX;
    } else {
	/*

```

tcl9.0 9.0.4, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclProc.c`, function `TclFindProc`, lines 1010–1143. Retained reference `native_compiled_local_formal_source272-9.0.4-tclProc.c`.

```text
 *	usual rules: first in the current namespace and then in the global
 *	namespace.
 *
 * Results:
 *	NULL is returned if the name doesn't correspond to any procedure.
 *	Otherwise, the return value is a pointer to the procedure's record. If
 *	the name is found but refers to an imported command that points to a
 *	"real" procedure defined in another namespace, a pointer to that
 *	"real" procedure's structure is returned.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Proc *
TclFindProc(
    Interp *iPtr,		/* Interpreter in which to look. */
    const char *procName)	/* Name of desired procedure. */
{
    Tcl_Command cmd;
    Command *cmdPtr;

    cmd = Tcl_FindCommand((Tcl_Interp *) iPtr, procName, NULL, /*flags*/ 0);
    if (cmd == (Tcl_Command) NULL) {
	return NULL;
    }
    cmdPtr = (Command *) cmd;

    return TclIsProc(cmdPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclIsProc --
 *
 *	Tells whether a command is a Tcl procedure or not.
 *
 * Results:
 *	If the given command is actually a Tcl procedure, the return value is
 *	the address of the record describing the procedure. Otherwise the
 *	return value is 0.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Proc *
TclIsProc(
    Command *cmdPtr)		/* Command to test. */
{
    Tcl_Command origCmd = TclGetOriginalCommand((Tcl_Command) cmdPtr);

    if (origCmd != NULL) {
	cmdPtr = (Command *) origCmd;
    }
    if (cmdPtr->deleteProc == TclProcDeleteProc) {
	return (Proc *)cmdPtr->objClientData;
    }
    return NULL;
}

static int
ProcWrongNumArgs(
    Tcl_Interp *interp,
    int skip)
{
    CallFrame *framePtr = ((Interp *)interp)->varFramePtr;
    Proc *procPtr = framePtr->procPtr;
    Tcl_Size localCt = procPtr->numCompiledLocals, numArgs, i;
    Tcl_Obj **desiredObjs;
    const char *final = NULL;

    /*
     * Build up desired argument list for Tcl_WrongNumArgs
     */

    numArgs = framePtr->procPtr->numArgs;
    desiredObjs = (Tcl_Obj **)TclStackAlloc(interp,
	    sizeof(Tcl_Obj *) * (numArgs+1));

    if (framePtr->isProcCallFrame & FRAME_IS_LAMBDA) {
	desiredObjs[0] = Tcl_NewStringObj("lambdaExpr", -1);
    } else {
	desiredObjs[0] = framePtr->objv[skip-1];
    }
    Tcl_IncrRefCount(desiredObjs[0]);

    if (localCt > 0) {
	Var *defPtr = (Var *)(&framePtr->localCachePtr->varName0 + localCt);

	for (i=1 ; i<=numArgs ; i++, defPtr++) {
	    Tcl_Obj *argObj;
	    Tcl_Obj *namePtr = localName(framePtr, i-1);

	    if (defPtr->value.objPtr != NULL) {
		TclNewObj(argObj);
		Tcl_AppendStringsToObj(argObj, "?", TclGetString(namePtr), "?",
			(char *)NULL);
	    } else if (defPtr->flags & VAR_IS_ARGS) {
		numArgs--;
		final = "?arg ...?";
		break;
	    } else {
		argObj = namePtr;
		Tcl_IncrRefCount(namePtr);
	    }
	    desiredObjs[i] = argObj;
	}
    }

    Tcl_ResetResult(interp);
    Tcl_WrongNumArgs(interp, numArgs+1, desiredObjs, final);

    for (i=0 ; i<=numArgs ; i++) {
	Tcl_DecrRefCount(desiredObjs[i]);
    }
    TclStackFree(interp, desiredObjs);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * InitResolvedLocals --
 *
 *	This routine is invoked in order to initialize the compiled locals
 *	table for a new call frame.
 *
 * Results:

```

tcl9.1 9.1.0, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclNamesp.c`, function `TclGetOriginalCommand`, lines 2074–2177. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclNamesp.c`.

```text
{
    Command *cmdPtr = (Command *) command;

    if (cmdPtr->deleteProc != DeleteImportedCmd) {
	return NULL;
    }

    while (cmdPtr->deleteProc == DeleteImportedCmd) {
	ImportedCmdData *dataPtr = (ImportedCmdData *) cmdPtr->objClientData2;
	cmdPtr = dataPtr->realCmdPtr;
    }
    return (Tcl_Command) cmdPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * TclInvokeImportedCmd --
 *
 *	Invoked by Tcl whenever the user calls an imported command that was
 *	created by Tcl_Import. Finds the "real" command (in another
 *	namespace), and passes control to it.
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If anything goes
 *	wrong, the result object is set to an error message.
 *
 *----------------------------------------------------------------------
 */

static int
InvokeImportedNRCmd(
    void *clientData,		/* Points to the imported command's
				 * ImportedCmdData structure. */
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The argument objects. */
{
    ImportedCmdData *dataPtr = (ImportedCmdData *) clientData;
    Command *realCmdPtr = dataPtr->realCmdPtr;

    TclSkipTailcall(interp);
    return TclNREvalObjv(interp, objc, objv, TCL_EVAL_NOERR, realCmdPtr);
}

int
TclInvokeImportedCmd(
    void *clientData,		/* Points to the imported command's
				 * ImportedCmdData structure. */
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The argument objects. */
{
    return Tcl_NRCallObjProc2(interp, InvokeImportedNRCmd, clientData,
	    objc, objv);
}

/*
 *----------------------------------------------------------------------
 *
 * DeleteImportedCmd --
 *
 *	Invoked by Tcl whenever an imported command is deleted. The "real"
 *	command keeps a list of all the imported commands that refer to it, so
 *	those imported commands can be deleted when the real command is
 *	deleted. This function removes the imported command reference from the
 *	real command's list, and frees up the memory associated with the
 *	imported command.
 *
 * Results:
 *	None.
 *
 * Side effects:
 *	Removes the imported command from the real command's import list.
 *
 *----------------------------------------------------------------------
 */

static void
DeleteImportedCmd(
    void *clientData)		/* Points to the imported command's
				 * ImportedCmdData structure. */
{
    ImportedCmdData *dataPtr = (ImportedCmdData *) clientData;
    Command *realCmdPtr = dataPtr->realCmdPtr;
    Command *selfPtr = dataPtr->selfPtr;
    ImportRef *refPtr, *prevPtr;

    prevPtr = NULL;
    for (refPtr = realCmdPtr->importRefPtr; refPtr != NULL;
	    refPtr = refPtr->nextPtr) {
	if (refPtr->importedCmdPtr == selfPtr) {
	    /*
	     * Remove *refPtr from real command's list of imported commands
	     * that refer to it.
	     */

	    if (prevPtr == NULL) { /* refPtr is first in list. */
		realCmdPtr->importRefPtr = refPtr->nextPtr;
	    } else {
		prevPtr->nextPtr = refPtr->nextPtr;

```

tcl9.1 9.1.0, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclNamesp.c`, function `NamespaceOriginCmd`, lines 4011–4114. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclNamesp.c`.

```text
    Tcl_GetCommandFullName(interp, origCmd, resultPtr);
    if (TclCheckEmptyString(resultPtr) == TCL_EMPTYSTRING_YES) {
	Tcl_DecrRefCount(resultPtr);
	goto namespaceOriginError;
    }
    Tcl_SetObjResult(interp, resultPtr);
    return TCL_OK;

  namespaceOriginError:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "invalid command name \"%s\"", TclGetString(objv[1])));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "COMMAND",
	    TclGetString(objv[1]), (char *)NULL);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespaceParentCmd --
 *
 *	Invoked to implement the "namespace parent" command that returns the
 *	fully-qualified name of the parent namespace for a specified
 *	namespace. Handles the following syntax:
 *
 *	    namespace parent ?name?
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong.
 *
 * Side effects:
 *	Returns a result in the interpreter's result object. If anything goes
 *	wrong, the result is an error message.
 *
 *----------------------------------------------------------------------
 */

static int
NamespaceParentCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Namespace *nsPtr;

    if (objc == 1) {
	nsPtr = TclGetCurrentNamespace(interp);
    } else if (objc == 2) {
	if (TclGetNamespaceFromObj(interp, objv[1], &nsPtr) != TCL_OK) {
	    return TCL_ERROR;
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?name?");
	return TCL_ERROR;
    }

    /*
     * Report the parent of the specified namespace.
     */

    if (nsPtr->parentPtr != NULL) {
	Tcl_SetObjResult(interp, TclNewNamespaceObj(nsPtr->parentPtr));
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * NamespacePathCmd --
 *
 *	Invoked to implement the "namespace path" command that reads and
 *	writes the current namespace's command resolution path. Has one
 *	optional argument: if present, it is a list of named namespaces to set
 *	the path to, and if absent, the current path should be returned.
 *	Handles the following syntax:
 *
 *	    namespace path ?nsList?
 *
 * Results:
 *	Returns TCL_OK if successful, and TCL_ERROR if anything goes wrong
 *	(most notably if the namespace list contains the name of something
 *	other than a namespace). In the successful-exit case, may set the
 *	interpreter result to the list of names of the namespaces on the
 *	current namespace's path.
 *
 * Side effects:
 *	May update the namespace path (triggering a recomputing of all command
 *	names that depend on the namespace for resolution).
 *
 *----------------------------------------------------------------------
 */

static int
NamespacePathCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    Tcl_Size nsObjc, i;
    int result = TCL_ERROR;

```

tcl9.1 9.1.0, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEnsemble.c`, function `Tcl_FindEnsemble`, lines 1470–1607. Retained reference `native_ensemble_holder_routing205-source-inspection208-9.1.0-tclEnsemble.c`.

```text
 */

Tcl_Command
Tcl_FindEnsemble(
    Tcl_Interp *interp,		/* Where to do the lookup, and where to write
				 * the errors if TCL_LEAVE_ERR_MSG is set in
				 * the flags. */
    Tcl_Obj *cmdNameObj,	/* Name of command to look up. */
    int flags)			/* Either 0 or TCL_LEAVE_ERR_MSG; other flags
				 * are probably not useful. */
{
    Tcl_Command token;

    token = Tcl_FindCommand(interp, TclGetString(cmdNameObj), NULL, flags);
    if (token == NULL) {
	return NULL;
    }

    if (((Command *) token)->objProc2 != TclEnsembleImplementationCmd) {
	/*
	 * Reuse existing infrastructure for following import link chains
	 * rather than duplicating it.
	 */

	token = TclGetOriginalCommand(token);

	if (token == NULL ||
		((Command *) token)->objProc2 != TclEnsembleImplementationCmd) {
	    if (flags & TCL_LEAVE_ERR_MSG) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"\"%s\" is not an ensemble command",
			TclGetString(cmdNameObj)));
		Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "ENSEMBLE",
			TclGetString(cmdNameObj), (char *)NULL);
	    }
	    return NULL;
	}
    }

    return token;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_IsEnsemble --
 *
 *	Simple test for ensemble-hood that takes into account imported
 *	ensemble commands as well.
 *
 * Results:
 *	Boolean value
 *
 * Side effects:
 *	None
 *
 *----------------------------------------------------------------------
 */

int
Tcl_IsEnsemble(
    Tcl_Command token)		/* The command to check. */
{
    Command *cmdPtr = (Command *) token;

    if (cmdPtr->objProc2 == TclEnsembleImplementationCmd) {
	return 1;
    }
    cmdPtr = (Command *) TclGetOriginalCommand((Tcl_Command) cmdPtr);
    if (cmdPtr == NULL || cmdPtr->objProc2 != TclEnsembleImplementationCmd) {
	return 0;
    }
    return 1;
}

/*
 *----------------------------------------------------------------------
 *
 * TclMakeEnsemble --
 *
 *	Create an ensemble from a table of implementation commands. The
 *	ensemble will be subject to (limited) compilation if any of the
 *	implementation commands are compilable.
 *
 *	The 'name' parameter may be a single command name or a list if
 *	creating an ensemble subcommand (see the binary implementation).
 *
 *	Currently, the TCL_ENSEMBLE_PREFIX ensemble flag is only used on
 *	top-level ensemble commands.
 *
 *	This code is not safe to run in Safe interpreter after user code has
 *	executed. That's OK right now because it's just used to set up Tcl,
 *	but it means we mustn't expose it at all, not even to Tk (until we can
 *	hide commands in namespaces directly).
 *
 * Results:
 *	Handle for the new ensemble, or NULL on failure.
 *
 * Side effects:
 *	May advance the bytecode compilation epoch.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
TclMakeEnsemble(
    Tcl_Interp *interp,
    const char *name,		/* The ensemble name (as explained above) */
    const EnsembleImplMap map[])/* The subcommands to create */
{
    Tcl_Command ensemble;
    Tcl_Namespace *ns;
    Tcl_DString buf, hiddenBuf;
    const char **nameParts = NULL;
    const char *cmdName = NULL;
    Tcl_Size i, nameCount = 0;
    int ensembleFlags = 0;
    Tcl_Size hiddenLen;

    /*
     * Construct the path for the ensemble namespace and create it.
     */

    Tcl_DStringInit(&buf);
    Tcl_DStringInit(&hiddenBuf);
    TclDStringAppendLiteral(&hiddenBuf, "tcl:");
    Tcl_DStringAppend(&hiddenBuf, name, TCL_AUTO_LENGTH);
    TclDStringAppendLiteral(&hiddenBuf, ":");
    hiddenLen = Tcl_DStringLength(&hiddenBuf);
    if (name[0] == ':' && name[1] == ':') {
	/*
	 * An absolute name, so use it directly.
	 */

	cmdName = name;
	Tcl_DStringAppend(&buf, name, TCL_AUTO_LENGTH);
	ensembleFlags = TCL_ENSEMBLE_PREFIX;
    } else {

```

tcl9.1 9.1.0, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclProc.c`, function `TclFindProc`, lines 1005–1138. Retained reference `native_compiled_local_formal_source272-9.1.0-tclProc.c`.

```text
 *	usual rules: first in the current namespace and then in the global
 *	namespace.
 *
 * Results:
 *	NULL is returned if the name doesn't correspond to any procedure.
 *	Otherwise, the return value is a pointer to the procedure's record. If
 *	the name is found but refers to an imported command that points to a
 *	"real" procedure defined in another namespace, a pointer to that
 *	"real" procedure's structure is returned.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Proc *
TclFindProc(
    Interp *iPtr,		/* Interpreter in which to look. */
    const char *procName)	/* Name of desired procedure. */
{
    Tcl_Command cmd;
    Command *cmdPtr;

    cmd = Tcl_FindCommand((Tcl_Interp *) iPtr, procName, NULL, /*flags*/ 0);
    if (cmd == (Tcl_Command) NULL) {
	return NULL;
    }
    cmdPtr = (Command *) cmd;

    return TclIsProc(cmdPtr);
}

/*
 *----------------------------------------------------------------------
 *
 * TclIsProc --
 *
 *	Tells whether a command is a Tcl procedure or not.
 *
 * Results:
 *	If the given command is actually a Tcl procedure, the return value is
 *	the address of the record describing the procedure. Otherwise the
 *	return value is 0.
 *
 * Side effects:
 *	None.
 *
 *----------------------------------------------------------------------
 */

Proc *
TclIsProc(
    Command *cmdPtr)		/* Command to test. */
{
    Tcl_Command origCmd = TclGetOriginalCommand((Tcl_Command) cmdPtr);

    if (origCmd != NULL) {
	cmdPtr = (Command *) origCmd;
    }
    if (cmdPtr->deleteProc == TclProcDeleteProc) {
	return (Proc *)cmdPtr->objClientData2;
    }
    return NULL;
}

static int
ProcWrongNumArgs(
    Tcl_Interp *interp,
    Tcl_Size skip)
{
    CallFrame *framePtr = ((Interp *)interp)->varFramePtr;
    Proc *procPtr = framePtr->procPtr;
    Tcl_Size localCt = procPtr->numCompiledLocals, numArgs, i;
    Tcl_Obj **desiredObjs;
    const char *final = NULL;

    /*
     * Build up desired argument list for Tcl_WrongNumArgs
     */

    numArgs = framePtr->procPtr->numArgs;
    desiredObjs = (Tcl_Obj **)TclStackAlloc(interp,
	    sizeof(Tcl_Obj *) * (numArgs+1));

    if (framePtr->isProcCallFrame & FRAME_IS_LAMBDA) {
	desiredObjs[0] = Tcl_NewStringObj("lambdaExpr", -1);
    } else {
	desiredObjs[0] = framePtr->objv[skip-1];
    }
    Tcl_IncrRefCount(desiredObjs[0]);

    if (localCt > 0) {
	Var *defPtr = (Var *)(&framePtr->localCachePtr->varName0 + localCt);

	for (i=1 ; i<=numArgs ; i++, defPtr++) {
	    Tcl_Obj *argObj;
	    Tcl_Obj *namePtr = localName(framePtr, i-1);

	    if (defPtr->value.objPtr != NULL) {
		TclNewObj(argObj);
		Tcl_AppendStringsToObj(argObj, "?", TclGetString(namePtr), "?",
			(char *)NULL);
	    } else if (defPtr->flags & VAR_IS_ARGS) {
		numArgs--;
		final = "?arg ...?";
		break;
	    } else {
		argObj = namePtr;
		Tcl_IncrRefCount(namePtr);
	    }
	    desiredObjs[i] = argObj;
	}
    }

    Tcl_ResetResult(interp);
    Tcl_WrongNumArgs(interp, numArgs+1, desiredObjs, final);

    for (i=0 ; i<=numArgs ; i++) {
	Tcl_DecrRefCount(desiredObjs[i]);
    }
    TclStackFree(interp, desiredObjs);
    return TCL_ERROR;
}

/*
 *----------------------------------------------------------------------
 *
 * InitResolvedLocals --
 *
 *	This routine is invoked in order to initialize the compiled locals
 *	table for a new call frame.
 *
 * Results:

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/.proofs/native-providers/jimtcl/nshelper.tcl`, function `namespace-origin`, lines 14–31. Retained reference `native_jim_alias_introspection274-nshelper.tcl`.

```text
	}
}

proc {namespace origin} {name} {
	set nscanon [uplevel 1 [list ::namespace canon $name]]
	if {[exists -alias $nscanon]} {
		tailcall {namespace origin} [info alias $nscanon]
	}
	if {[exists -command $nscanon]} {
		return ::$nscanon
	}
	if {[exists -command $name]} {
		return ::$name
	}

	return -code error "invalid command name \"$name\""
}

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/.proofs/native-providers/jimtcl/nshelper.tcl`, function `namespace-import`, lines 58–86. Retained reference `native_jim_alias_introspection274-nshelper.tcl`.

```text
	tailcall namespace eval $name $arg $args
}

proc {namespace import} {args} {
	set current [uplevel 1 ::namespace canon]

	foreach pattern $args {
		foreach cmd [info commands [namespace canon $current $pattern]] {
			if {[namespace qualifiers $cmd] eq $current} {
				return -code error "import pattern \"$pattern\" tries to import from namespace \"$current\" into itself"
			}
			# What if this alias would create a loop?
			# follow the target alias chain to see if we are creating a loop
			set newcmd ${current}::[namespace tail $cmd]

			set alias $cmd
			while {[exists -alias $alias]} {
				set alias [info alias $alias]
				if {$alias eq $newcmd} {
					return -code error "import pattern \"$pattern\" would create a loop"
				}
			}

			alias $newcmd $cmd
		}
	}
}

# namespace-aware info commands: procs, channels, globals, locals, vars

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/.proofs/native-providers/jimtcl/nshelper.tcl`, function `namespace-ensemble`, lines 144–159. Retained reference `native_jim_alias_introspection274-nshelper.tcl`.

```text
	tailcall {*}$script
}

proc {namespace ensemble} {subcommand args} {
	if {$subcommand ne "create"} {
		return -code error "only \[namespace ensemble create\] is supported"
	}
	set ns [uplevel 1 namespace canon]
	set cmd $ns
	if {$ns eq ""} {
		return -code error "namespace ensemble create: must be called within a namespace"
	}

	# Create the mapping
	ensemble $cmd -automap ${ns}:: {*}$args
}

```

jim 0.84-9-g5bac7c9, revision `Exact pinned original source; source-control revision unrecorded. Current Jim patchlevel measured independently in the original process.`, `/workspace/.proofs/native-providers/jimtcl/jim-namespace.c`, function `JimNamespaceCmd`, lines 190–336. Retained reference `native_jim_alias_introspection274-jim-namespace.c`.

```text
    return Jim_EvalObjPrefix(interp, prefixObj, argc, argv);
}

static int JimNamespaceCmd(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    /* Must be kept in order with the array below */
    enum {
        OPT_CANONICAL,
        OPT_CODE,
        OPT_CURRENT,
        OPT_DELETE,
        OPT_ENSEMBLE,
        OPT_EVAL,
        OPT_EXPORT,
        OPT_IMPORT,
        OPT_INSCOPE,
        OPT_ORIGIN,
        OPT_PARENT,
        OPT_QUALIFIERS,
        OPT_TAIL,
        OPT_UPVAR,
        OPT_WHICH,
        OPT_COUNT,
    };
    static const jim_subcmd_type namespace_cmds[OPT_COUNT + 1] = {
        JIM_DEF_SUBCMD("canonical", "?current? ?name?", 0, 2),
        JIM_DEF_SUBCMD("code", "arg", 1, 1),
        JIM_DEF_SUBCMD("current", NULL, 0, 0),
        JIM_DEF_SUBCMD("delete", "?name ...?", 0, -1),
        JIM_DEF_SUBCMD("ensemble", "subcommand ?arg ...?", 1, -1),
        JIM_DEF_SUBCMD("eval", "name arg ?arg ...?", 2, -1),
        JIM_DEF_SUBCMD("export", "?...?", 0, -1),
        JIM_DEF_SUBCMD("import", "?...?", 0, -1),
        JIM_DEF_SUBCMD("inscope", "name arg ?arg ...?", 2, -1),
        JIM_DEF_SUBCMD("origin", "name", 1, 1),
        JIM_DEF_SUBCMD("parent", "?name?", 0, 1),
        JIM_DEF_SUBCMD("qualifiers", "string", 1, 1),
        JIM_DEF_SUBCMD("tail", "string", 1, 1),
        JIM_DEF_SUBCMD("upvar", "ns ?arg ...?", 1, -1),
        JIM_DEF_SUBCMD("which", "?-command|-variable? name", 1, 2),
        { NULL }
    };
    const jim_subcmd_type *ct = Jim_ParseSubCmd(interp, namespace_cmds, argc, argv);
    if (ct) {
        if (ct->function) {
            /* This is -help */
            return ct->function(interp, argc, argv);
        }

        /* (ct - namespace_cmds) is the index into the table */
        switch (ct - namespace_cmds) {
            case OPT_EVAL:
                {
                    Jim_Obj *nsObj;
                    Jim_Obj *objPtr;
                    if (argc == 4) {
                        objPtr = argv[3];
                    }
                    else {
                        objPtr = Jim_ConcatObj(interp, argc - 3, argv + 3);
                    }

                    nsObj = JimCanonicalNamespace(interp, interp->framePtr->nsObj, argv[2]);
                    return Jim_EvalNamespace(interp, objPtr, nsObj);
                }

            case OPT_CURRENT:
                Jim_SetResult(interp, JimNamespaceCurrent(interp));
                return JIM_OK;

            case OPT_CANONICAL:
                if (argc == 2) {
                    Jim_SetResult(interp, interp->framePtr->nsObj);
                }
                else if (argc == 3) {
                    Jim_SetResult(interp, JimCanonicalNamespace(interp, interp->framePtr->nsObj, argv[2]));
                }
                else {
                    Jim_SetResult(interp, JimCanonicalNamespace(interp, argv[2], argv[3]));
                }
                return JIM_OK;

            case OPT_QUALIFIERS:
                Jim_SetResult(interp, Jim_NamespaceQualifiers(interp, argv[2]));
                return JIM_OK;

            case OPT_EXPORT:
                return JIM_OK;

            case OPT_TAIL:
                if (argc != 3) {
                    Jim_WrongNumArgs(interp, 2, argv, "string");
                    return JIM_ERR;
                }
                Jim_SetResult(interp, Jim_NamespaceTail(interp, argv[2]));
                return JIM_OK;

            case OPT_PARENT:
                {
                    Jim_Obj *objPtr;
                    const char *name;

                    if (argc == 3) {
                        objPtr = argv[2];
                    }
                    else {
                        objPtr = interp->framePtr->nsObj;
                    }
                    if (Jim_Length(objPtr) == 0 || Jim_CompareStringImmediate(interp, objPtr, "::")) {
                        return JIM_OK;
                    }
                    objPtr = Jim_NamespaceQualifiers(interp, objPtr);

                    name = Jim_String(objPtr);

                    if (name[0] != ':' || name[1] != ':') {
                        /* Make it fully scoped */
                        Jim_SetResultString(interp, "::", 2);
                        Jim_AppendObj(interp, Jim_GetResult(interp), objPtr);
                        Jim_IncrRefCount(objPtr);
                        Jim_DecrRefCount(interp, objPtr);
                    }
                    else {
                        Jim_SetResult(interp, objPtr);
                    }
                }
                return JIM_OK;

                default:
                    /* Implemented as a Tcl helper proc.
                     * Note that calling a proc will change the current namespace,
                     * so helper procs must call [uplevel namespace canon] to get the callers
                     * namespace.
                     */
                    return Jim_EvalEnsemble(interp, "namespace", Jim_String(argv[1]), argc - 2, argv + 2);
            }
    }
    return JIM_ERR;
}

int Jim_namespaceInit(Jim_Interp *interp)
{
    Jim_PackageProvideCheck(interp, "namespace");
    Jim_RegisterSimpleCmd(interp, "namespace", "subcommand ?arg ...?", 1, -1, JimNamespaceCmd);
    Jim_RegisterSimpleCmd(interp, "variable", "name ?value?", 1, 2, JimVariableCmd);
    return JIM_OK;
}

```

## Consumer bindings

No implementation binding is claimed by this observation record.

- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::resolve_import_query`: Traverse actual retained Imported generations with visited-generation refusal and a purpose-specific terminal owner, preserving original alias/scripted purposes rather than truncating at64 hops.
- [runtime/rust/src/interp/native_import_chain_tests.rs](../../../../runtime/rust/src/interp/native_import_chain_tests.rs), `interp::native_import_chain_tests::original_deep_import_queries_match_all_546_c_and_jim_public_rows` (linked): The registered Runtime test child consumes the unchanged original depth200 probe and compares91 CASE rows per provider (546), excluding the four version/helper metadata rows from570 raw rows; selected Jim NamespaceEnsemble distribution helper is explicit and all original unsupported-purpose rows remain intact.
- [runtime/rust/src/interp/native_import_chain_tests.rs](../../../../runtime/rust/src/interp/native_import_chain_tests.rs), `interp::native_import_chain_tests::malformed_actual_import_generation_cycle_has_no_partial_query_origin` (linked): A deliberately malformed owned software Imported-generation cycle refuses every public query without returning a partial origin; this is an internal table invariant, not a cycle observed by the original Native request.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_deep_import_chain_original/providers](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/providers)
- [rust/tcl-registry/tests/data/native_deep_import_chain_original](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim)
- [rust/tcl-registry/tests/data/native_jim_alias_introspection274](../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274)
- [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/jim)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-registry/tests/data/native_mathop_jim_original_source/original-inputs](../../../../rust/tcl-registry/tests/data/native_mathop_jim_original_source/original-inputs)
