# naming.embedding.original-command-recreation-discard

Kind: `native-observation`

## Problem statement

A public command delete callback may recreate the same name while an outer replacement is still running. That recreated publication has a different retirement purpose from ordinary deletion, active command retention or namespace currency. This fixed original source measures reached callbacks and invocation before software can choose a disposal contract.

## Question

What delete callbacks and reached replacement invocations occur when an original public object-command delete callback recreates that exact name once during an outer replacement, compared with a non-recreating delete callback, across each actual C Tcl provider and Jim?

## Conclusion

All six measured providers agree on these two idle object-command cases. The ordinary case reaches OLD delete once, FINAL invocation once and FINAL delete once. The once-recreating OLD callback additionally publishes MIDDLE, but MIDDLE invocation and MIDDLE delete remain 0 through outer replacement, FINAL invocation, explicit public deletion and full interpreter teardown. Every invocation returns the exact FINAL bytes. All 60 stage rows, reached callback order and public results are retained. Every provider dependency, compilation, CLI-version and main command exits 0 with empty stderr; the shared compiler-version command also exits 0. These public callback counts do not identify an incarnation from token addresses or certify an application implementation. The authored Runtime comparison retains all60 original public callback windows and exact invocation bytes as separate software definitions. The shared idle LegacyPublic retirement owner requires completed original deletion, an originally empty exact creation slot and uniquely owned intermediate object. Extra-held, Builtin/Proc, CountedEngine and OwnedRuntimeShim records retain their independent ordinary cleanup. A pure selected recipe supplies no actual generation, active-command or procedure identity.

## Scope

Exact unchanged C99 public object-command probe on measured C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9. Fresh fully initialized interpreters hold OLD/MIDDLE/FINAL client records through teardown, and OLD recreates the exact command name at most once. C uses Tcl_CreateObjCommand with its legacy int-count callback, including the retained Tcl9 compatibility adapter; the probe does not directly register through the Tcl9 size-count public callback family. Jim uses Jim_CreateCommand, whose public return is status, so address fields are unavailable. Exact held source objects enter Tcl_EvalObjEx or Jim_EvalObj; result bytes are queried after invocation. Diagnostic C token addresses remain raw observations with no generation/incarnation/procedure or active-command identity grant. Actual dependency/header/config/source/static-library/CLI/compiler/link associations and all 498 unchanged before/after paths are retained. Releases are measured through the probe VERSION/BUILD channels. Independent CLI version-query commands exit 0 but emit no stdout because their original scripts use a bare list; no fresh CLI-version output is claimed. SDK source/library inputs and provenance receipts are retained; configure/build processes lack replay receipts and source-to-library equivalence is unrecorded. External system toolchain/library binaries not already archived are hash-recorded only and must be independently supplied for replay; they are not silently reconstructed. No active command, namespace recreation, frame/source receipt, native compiler/procedure identity, application issuer, Rust assertion or BIG-IP outcome follows. Current direct Rust and Tokio test attributes and real module parents attach these source/API contracts only. They do not certify an executable inventory, passed assertion, complete Native comparison or changed-current-image result. Each original provider answer and independently closed software command retains its exact input/channel/source purpose. The Jim occupied-key software control retains the same original table key through once-only recreated publication and final invocation, then observes retirement at explicit public deletion through an allocation-only lease with no additional native reference. This definition attaches no matching current owned assertion outcome. The unchanged original486 experiment measures callback/invocation counts; it does not measure this key header/reference lifetime or authorize an active command, generation or procedure identity.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Actual public C/Jim object-command registration, one idle deletion-callback recreation, held original script object invocation, explicit public deletion and interpreter teardown; complete raw callback-stage/result channels. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

Ordinary OLD/FINAL delete counts are 1, FINAL invocation 1 and exact FINAL result. Once-recreating control publishes MIDDLE but observes its delete/invocation 0 through full teardown; OLD/FINAL delete 1 and FINAL invocation 1 remain. Both cases preserve all five original stage windows, reached event order, versions, ABI and diagnostic-token availability; all four provider commands exit 0 with empty stderr.

### bigip

Status: `not-tested`.

No original BIG-IP callback recreation, command replacement or teardown process was requested or executed.

## Source inspection

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_CreateObjCommand selected source`, lines 1760–1850. Retained reference `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c`.

```text
    ImportedCmdData *dataPtr;

    if (iPtr->flags & DELETED) {
	/*
	 * The interpreter is being deleted.  Don't create any new
	 * commands;  it's not safe to muck with the interpreter anymore.
	 */

	return (Tcl_Command) NULL;
    }

    /*
     * Determine where the command should reside. If its name contains 
     * namespace qualifiers, we put it in the specified namespace; 
     * otherwise, we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
       TclGetNamespaceForQualName(interp, cmdName, (Namespace *) NULL,
           CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
       if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

    hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &new);
    if (!new) {
	cmdPtr = (Command *) Tcl_GetHashValue(hPtr);

	/*
	 * Command already exists. If its object-based Tcl_ObjCmdProc is
	 * TclInvokeStringCommand, we just set its Tcl_ObjCmdProc to the
	 * argument "proc". Otherwise, we delete the old command. 
	 */

	if (cmdPtr->objProc == TclInvokeStringCommand) {
	    cmdPtr->objProc = proc;
	    cmdPtr->objClientData = clientData;
            cmdPtr->deleteProc = deleteProc;
            cmdPtr->deleteData = clientData;
	    return (Tcl_Command) cmdPtr;
	}

	/*
	 * Otherwise, we delete the old command.  Be careful to preserve
	 * any existing import links so we can restore them down below.
	 * That way, you can redefine a command and its import status
	 * will remain intact.
	 */

	oldRefPtr = cmdPtr->importRefPtr;
	cmdPtr->importRefPtr = NULL;

	Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &new);
	if (!new) {
	    /*
	     * If the deletion callback recreated the command, just throw
	     * away the new command (if we try to delete it again, we
	     * could get stuck in an infinite loop).
	     */

	     ckfree((char *) Tcl_GetHashValue(hPtr));
	}
    }
    cmdPtr = (Command *) ckalloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = (CompileProc *) NULL;
    cmdPtr->objProc = proc;
    cmdPtr->objClientData = clientData;
    cmdPtr->proc = TclInvokeObjectCommand;
    cmdPtr->clientData = (ClientData) cmdPtr;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;

    /*
     * Plug in any existing import references found above.  Be sure
     * to update all of these references to point to the new command.
     */

    if (oldRefPtr != NULL) {

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_CreateCommand and Tcl_CreateObjCommand selected source`, lines 1840–2130. Retained reference `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c`.

```text
    Tcl_Interp *interp,		/* Token for command interpreter returned by a
				 * previous call to Tcl_CreateInterp. */
    const char *cmdName,	/* Name of command. If it contains namespace
				 * qualifiers, the new command is put in the
				 * specified namespace; otherwise it is put in
				 * the global namespace. */
    Tcl_CmdProc *proc,		/* Function to associate with cmdName. */
    ClientData clientData,	/* Arbitrary value passed to string proc. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    Interp *iPtr = (Interp *) interp;
    ImportRef *oldRefPtr = NULL;
    Namespace *nsPtr, *dummy1, *dummy2;
    Command *cmdPtr, *refCmdPtr;
    Tcl_HashEntry *hPtr;
    const char *tail;
    int isNew;
    ImportedCmdData *dataPtr;

    if (iPtr->flags & DELETED) {
	/*
	 * The interpreter is being deleted. Don't create any new commands;
	 * it's not safe to muck with the interpreter anymore.
	 */

	return (Tcl_Command) NULL;
    }

    /*
     * Determine where the command should reside. If its name contains
     * namespace qualifiers, we put it in the specified namespace; otherwise,
     * we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
	TclGetNamespaceForQualName(interp, cmdName, NULL,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

    hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &isNew);
    if (!isNew) {
	/*
	 * Command already exists. Delete the old one. Be careful to preserve
	 * any existing import links so we can restore them down below. That
	 * way, you can redefine a command and its import status will remain
	 * intact.
	 */

	cmdPtr = Tcl_GetHashValue(hPtr);
	cmdPtr->refCount++;
	if (cmdPtr->importRefPtr) {
	    cmdPtr->flags |= CMD_REDEF_IN_PROGRESS;
	}

	Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);

	if (cmdPtr->flags & CMD_REDEF_IN_PROGRESS) {
	    oldRefPtr = cmdPtr->importRefPtr;
	    cmdPtr->importRefPtr = NULL;
	}
	TclCleanupCommandMacro(cmdPtr);

	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &isNew);
	if (!isNew) {
	    /*
	     * If the deletion callback recreated the command, just throw away
	     * the new command (if we try to delete it again, we could get
	     * stuck in an infinite loop).
	     */

	     ckfree((char *) Tcl_GetHashValue(hPtr));
	}
    } else {
	/*
	 * The list of command exported from the namespace might have changed.
	 * However, we do not need to recompute this just yet; next time we
	 * need the info will be soon enough.
	 */

	TclInvalidateNsCmdLookup(nsPtr);
	TclInvalidateNsPath(nsPtr);
    }
    cmdPtr = (Command *) ckalloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc = TclInvokeStringCommand;
    cmdPtr->objClientData = cmdPtr;
    cmdPtr->proc = proc;
    cmdPtr->clientData = clientData;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;

    /*
     * Plug in any existing import references found above. Be sure to update
     * all of these references to point to the new command.
     */

    if (oldRefPtr != NULL) {
	cmdPtr->importRefPtr = oldRefPtr;
	while (oldRefPtr != NULL) {
	    refCmdPtr = oldRefPtr->importedCmdPtr;
	    dataPtr = refCmdPtr->objClientData;
	    dataPtr->realCmdPtr = cmdPtr;
	    oldRefPtr = oldRefPtr->nextPtr;
	}
    }

    /*
     * We just created a command, so in its namespace and all of its parent
     * namespaces, it may shadow global commands with the same name. If any
     * shadowed commands are found, invalidate all cached command references
     * in the affected namespaces.
     */

    TclResetShadowedCmdRefs(interp, cmdPtr);
    return (Tcl_Command) cmdPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_CreateObjCommand --
 *
 *	Define a new object-based command in a command table.
 *
 * Results:
 *	The return value is a token for the command, which can be used in
 *	future calls to Tcl_GetCommandName.
 *
 * Side effects:
 *	If a command named "cmdName" already exists for interp, it is
 *	first deleted.  Then the new command is created from the arguments.
 *	[***] (See below for exception).
 *
 *	In the future, during bytecode evaluation when "cmdName" is seen as
 *	the name of a command by Tcl_EvalObj or Tcl_Eval, the object-based
 *	Tcl_ObjCmdProc proc will be called. When the command is deleted from
 *	the table, deleteProc will be called. See the manual entry for details
 *	on the calling sequence.
 *
 *----------------------------------------------------------------------
 */

Tcl_Command
Tcl_CreateObjCommand(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * previous call to Tcl_CreateInterp). */
    const char *cmdName,	/* Name of command. If it contains namespace
				 * qualifiers, the new command is put in the
				 * specified namespace; otherwise it is put in
				 * the global namespace. */
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    ClientData clientData,	/* Arbitrary value to pass to object
    				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    Interp *iPtr = (Interp *) interp;
    ImportRef *oldRefPtr = NULL;
    Namespace *nsPtr, *dummy1, *dummy2;
    Command *cmdPtr, *refCmdPtr;
    Tcl_HashEntry *hPtr;
    const char *tail;
    int isNew;
    ImportedCmdData *dataPtr;

    if (iPtr->flags & DELETED) {
	/*
	 * The interpreter is being deleted. Don't create any new commands;
	 * it's not safe to muck with the interpreter anymore.
	 */

	return (Tcl_Command) NULL;
    }

    /*
     * Determine where the command should reside. If its name contains
     * namespace qualifiers, we put it in the specified namespace; otherwise,
     * we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
	TclGetNamespaceForQualName(interp, cmdName, NULL,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

    hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &isNew);
    TclInvalidateNsPath(nsPtr);
    if (!isNew) {
	cmdPtr = Tcl_GetHashValue(hPtr);

	/* Command already exists. */

	/*
	 * [***] This is wrong.  See Tcl Bug a16752c252.  
	 * However, this buggy behavior is kept under particular
	 * circumstances to accommodate deployed binaries of the
	 * "tclcompiler" program. http://sourceforge.net/projects/tclpro/
	 * that crash if the bug is fixed.
	 */

	if (cmdPtr->objProc == TclInvokeStringCommand
		&& cmdPtr->clientData == clientData
		&& cmdPtr->deleteData == clientData
		&& cmdPtr->deleteProc == deleteProc) {
	    cmdPtr->objProc = proc;
	    cmdPtr->objClientData = clientData;
	    return (Tcl_Command) cmdPtr;
	}

	/*
	 * Otherwise, we delete the old command. Be careful to preserve any
	 * existing import links so we can restore them down below. That way,
	 * you can redefine a command and its import status will remain
	 * intact.
	 */

	cmdPtr->refCount++;
	if (cmdPtr->importRefPtr) {
	    cmdPtr->flags |= CMD_REDEF_IN_PROGRESS;
	}

	Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);

	if (cmdPtr->flags & CMD_REDEF_IN_PROGRESS) {
	    oldRefPtr = cmdPtr->importRefPtr;
	    cmdPtr->importRefPtr = NULL;
	}
	TclCleanupCommandMacro(cmdPtr);

	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &isNew);
	if (!isNew) {
	    /*
	     * If the deletion callback recreated the command, just throw away
	     * the new command (if we try to delete it again, we could get
	     * stuck in an infinite loop).
	     */

	     ckfree(Tcl_GetHashValue(hPtr));
	}
    } else {
	/*
	 * The list of command exported from the namespace might have changed.
	 * However, we do not need to recompute this just yet; next time we
	 * need the info will be soon enough.
	 */

	TclInvalidateNsCmdLookup(nsPtr);
    }
    cmdPtr = (Command *) ckalloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc = proc;
    cmdPtr->objClientData = clientData;
    cmdPtr->proc = TclInvokeObjectCommand;
    cmdPtr->clientData = cmdPtr;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;

    /*
     * Plug in any existing import references found above. Be sure to update

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `TclCreateObjCommandInNs selected source`, lines 2340–2500. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    ClientData clientData,	/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    int deleted = 0, isNew = 0;
    Command *cmdPtr;
    ImportRef *oldRefPtr = NULL;
    ImportedCmdData *dataPtr;
    Tcl_HashEntry *hPtr;
    Namespace *nsPtr = (Namespace *) namesp;

    /*
     * If the command name we seek to create already exists, we need to delete
     * that first. That can be tricky in the presence of traces. Loop until we
     * no longer find an existing command in the way, or until we've deleted
     * one command and that didn't finish the job.
     */

    while (1) {
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, cmdName, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * [***] This is wrong.  See Tcl Bug a16752c252.
	 * However, this buggy behavior is kept under particular circumstances
	 * to accommodate deployed binaries of the "tclcompiler" program
	 * <http://sourceforge.net/projects/tclpro/> that crash if the bug is
	 * fixed.
	 */

	if (cmdPtr->objProc == TclInvokeStringCommand
		&& cmdPtr->clientData == clientData
		&& cmdPtr->deleteData == clientData
		&& cmdPtr->deleteProc == deleteProc) {
	    cmdPtr->objProc = proc;
	    cmdPtr->objClientData = clientData;
	    return (Tcl_Command)cmdPtr;
	}

	/*
	 * Otherwise, we delete the old command. Be careful to preserve any
	 * existing import links so we can restore them down below. That way,
	 * you can redefine a command and its import status will remain
	 * intact.
	 */

	cmdPtr->refCount++;
	if (cmdPtr->importRefPtr) {
	    cmdPtr->flags |= CMD_REDEF_IN_PROGRESS;
	}

	/*
	 * Make sure namespace doesn't get deallocated.
	 */

	cmdPtr->nsPtr->refCount++;

	Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	nsPtr = (Namespace *) TclEnsureNamespace(interp,
		(Tcl_Namespace *) cmdPtr->nsPtr);
	/* Note nsPtr may or may not be same as cmdPtr->nsPtr */
	TclNsDecrRefCount(cmdPtr->nsPtr);

	if (cmdPtr->flags & CMD_REDEF_IN_PROGRESS) {
	    oldRefPtr = cmdPtr->importRefPtr;
	    cmdPtr->importRefPtr = NULL;
	}
	TclCleanupCommandMacro(cmdPtr);
	deleted = 1;
    }
    if (!isNew) {
	/*
	 * If the deletion callback recreated the command, just throw away the
	 * new command (if we try to delete it again, we could get stuck in an
	 * infinite loop).
	 */

	ckfree(Tcl_GetHashValue(hPtr));
    }

    if (!deleted) {
	/*
	 * Command resolvers (per-interp, per-namespace) might have resolved
	 * to a command for the given namespace scope with this command not
	 * being registered with the namespace's command table. During BC
	 * compilation, the so-resolved command turns into a CmdName literal.
	 * Without invalidating a possible CmdName literal here explicitly,
	 * such literals keep being reused while pointing to overhauled
	 * commands.
	 */

	TclInvalidateCmdLiteral(interp, cmdName, nsPtr);

	/*
	 * The list of command exported from the namespace might have changed.
	 * However, we do not need to recompute this just yet; next time we
	 * need the info will be soon enough.
	 */

	TclInvalidateNsCmdLookup(nsPtr);
	TclInvalidateNsPath(nsPtr);
    }
    cmdPtr = (Command *)ckalloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc = proc;
    cmdPtr->objClientData = clientData;
    cmdPtr->proc = TclInvokeObjectCommand;
    cmdPtr->clientData = cmdPtr;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;
    cmdPtr->nreProc = NULL;

    /*
     * Plug in any existing import references found above. Be sure to update
     * all of these references to point to the new command.
     */

    if (oldRefPtr != NULL) {
	cmdPtr->importRefPtr = oldRefPtr;
	while (oldRefPtr != NULL) {
	    Command *refCmdPtr = oldRefPtr->importedCmdPtr;

	    dataPtr = (ImportedCmdData*)refCmdPtr->objClientData;
	    dataPtr->realCmdPtr = cmdPtr;
	    oldRefPtr = oldRefPtr->nextPtr;
	}
    }

    /*
     * We just created a command, so in its namespace and all of its parent
     * namespaces, it may shadow global commands with the same name. If any
     * shadowed commands are found, invalidate all cached command references
     * in the affected namespaces.
     */

    TclResetShadowedCmdRefs(interp, cmdPtr);

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_CreateCommand selected legacy string-command replacement source`, lines 2640–2790. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
	 * it's not safe to muck with the interpreter anymore.
	 */

	return (Tcl_Command) NULL;
    }

    /*
     * If the command name we seek to create already exists, we need to
     * delete that first.  That can be tricky in the presence of traces.
     * Loop until we no longer find an existing command in the way, or
     * until we've deleted one command and that didn't finish the job.
     */

    while (1) {
	/*
	 * Determine where the command should reside. If its name contains
	 * namespace qualifiers, we put it in the specified namespace;
	 * otherwise, we always put it in the global namespace.
	 */

	if (strstr(cmdName, "::") != NULL) {
	    Namespace *dummy1, *dummy2;

	    TclGetNamespaceForQualName(interp, cmdName, NULL,
		    TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	    if ((nsPtr == NULL) || (tail == NULL)) {
		return (Tcl_Command) NULL;
	    }
	} else {
	    nsPtr = iPtr->globalNsPtr;
	    tail = cmdName;
	}

	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * Be careful to preserve any existing import links so we can restore
	 * them down below. That way, you can redefine a command and its
	 * import status will remain intact.
	 */

	cmdPtr->refCount++;
	if (cmdPtr->importRefPtr) {
	    cmdPtr->flags |= CMD_REDEF_IN_PROGRESS;
	}

	Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);

	if (cmdPtr->flags & CMD_REDEF_IN_PROGRESS) {
	    oldRefPtr = cmdPtr->importRefPtr;
	    cmdPtr->importRefPtr = NULL;
	}
	TclCleanupCommandMacro(cmdPtr);
	deleted = 1;
    }

    if (!isNew) {
	/*
	 * If the deletion callback recreated the command, just throw away the
	 * new command (if we try to delete it again, we could get stuck in an
	 * infinite loop).
	 */

	Tcl_Free(Tcl_GetHashValue(hPtr));
    }

    if (!deleted) {
	/*
	 * Command resolvers (per-interp, per-namespace) might have resolved
	 * to a command for the given namespace scope with this command not
	 * being registered with the namespace's command table. During BC
	 * compilation, the so-resolved command turns into a CmdName literal.
	 * Without invalidating a possible CmdName literal here explicitly,
	 * such literals keep being reused while pointing to overhauled
	 * commands.
	 */

	TclInvalidateCmdLiteral(interp, tail, nsPtr);

	/*
	 * The list of command exported from the namespace might have changed.
	 * However, we do not need to recompute this just yet; next time we
	 * need the info will be soon enough.
	 */

	TclInvalidateNsCmdLookup(nsPtr);
	TclInvalidateNsPath(nsPtr);
    }
    cmdPtr = (Command *)Tcl_Alloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc = InvokeStringCommand;
    cmdPtr->objClientData = cmdPtr;
    cmdPtr->proc = proc;
    cmdPtr->clientData = clientData;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;
    cmdPtr->nreProc = NULL;

    /*
     * Plug in any existing import references found above. Be sure to update
     * all of these references to point to the new command.
     */

    if (oldRefPtr != NULL) {
	cmdPtr->importRefPtr = oldRefPtr;
	while (oldRefPtr != NULL) {
	    Command *refCmdPtr = oldRefPtr->importedCmdPtr;
	    dataPtr = (ImportedCmdData *)refCmdPtr->objClientData;
	    dataPtr->realCmdPtr = cmdPtr;
	    oldRefPtr = oldRefPtr->nextPtr;
	}
    }

    /*
     * We just created a command, so in its namespace and all of its parent
     * namespaces, it may shadow global commands with the same name. If any
     * shadowed commands are found, invalidate all cached command references
     * in the affected namespaces.
     */

    TclResetShadowedCmdRefs(interp, cmdPtr);
    return (Tcl_Command) cmdPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_CreateObjCommand --
 *

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_CreateCommand selected legacy string-command replacement source`, lines 2590–2740. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
	 * The interpreter is being deleted. Don't create any new commands;
	 * it's not safe to muck with the interpreter anymore.
	 */

	return (Tcl_Command) NULL;
    }

    /*
     * If the command name we seek to create already exists, we need to
     * delete that first.  That can be tricky in the presence of traces.
     * Loop until we no longer find an existing command in the way, or
     * until we've deleted one command and that didn't finish the job.
     */

    while (1) {
	/*
	 * Determine where the command should reside. If its name contains
	 * namespace qualifiers, we put it in the specified namespace;
	 * otherwise, we always put it in the global namespace.
	 */

	if (strstr(cmdName, "::") != NULL) {
	    Namespace *dummy1, *dummy2;

	    TclGetNamespaceForQualName(interp, cmdName, NULL,
		    TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	    if ((nsPtr == NULL) || (tail == NULL)) {
		return (Tcl_Command) NULL;
	    }
	} else {
	    nsPtr = iPtr->globalNsPtr;
	    tail = cmdName;
	}

	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * Be careful to preserve any existing import links so we can restore
	 * them down below. That way, you can redefine a command and its
	 * import status will remain intact.
	 */

	cmdPtr->refCount++;
	if (cmdPtr->importRefPtr) {
	    cmdPtr->flags |= CMD_REDEF_IN_PROGRESS;
	}

	Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);

	if (cmdPtr->flags & CMD_REDEF_IN_PROGRESS) {
	    oldRefPtr = cmdPtr->importRefPtr;
	    cmdPtr->importRefPtr = NULL;
	}
	TclCleanupCommandMacro(cmdPtr);
	deleted = true;
    }

    if (!isNew) {
	/*
	 * If the deletion callback recreated the command, just throw away the
	 * new command (if we try to delete it again, we could get stuck in an
	 * infinite loop).
	 */

	Tcl_Free(Tcl_GetHashValue(hPtr));
    }

    if (!deleted) {
	/*
	 * Command resolvers (per-interp, per-namespace) might have resolved
	 * to a command for the given namespace scope with this command not
	 * being registered with the namespace's command table. During BC
	 * compilation, the so-resolved command turns into a CmdName literal.
	 * Without invalidating a possible CmdName literal here explicitly,
	 * such literals keep being reused while pointing to overhauled
	 * commands.
	 */

	TclInvalidateCmdLiteral(interp, tail, nsPtr);

	/*
	 * The list of command exported from the namespace might have changed.
	 * However, we do not need to recompute this just yet; next time we
	 * need the info will be soon enough.
	 */

	TclInvalidateNsCmdLookup(nsPtr);
	TclInvalidateNsPath(nsPtr);
    }
    cmdPtr = (Command *)Tcl_Alloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc2 = InvokeStringCommand;
    cmdPtr->objClientData2 = cmdPtr;
    cmdPtr->proc = proc;
    cmdPtr->clientData = clientData;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;
    cmdPtr->nreProc2 = NULL;

    /*
     * Plug in any existing import references found above. Be sure to update
     * all of these references to point to the new command.
     */

    if (oldRefPtr != NULL) {
	cmdPtr->importRefPtr = oldRefPtr;
	while (oldRefPtr != NULL) {
	    Command *refCmdPtr = oldRefPtr->importedCmdPtr;
	    dataPtr = (ImportedCmdData *)refCmdPtr->objClientData2;
	    dataPtr->realCmdPtr = cmdPtr;
	    oldRefPtr = oldRefPtr->nextPtr;
	}
    }

    /*
     * We just created a command, so in its namespace and all of its parent
     * namespaces, it may shadow global commands with the same name. If any
     * shadowed commands are found, invalidate all cached command references
     * in the affected namespaces.
     */

    TclResetShadowedCmdRefs(interp, cmdPtr);
    return (Tcl_Command) cmdPtr;
}

/*
 *----------------------------------------------------------------------
 *
 * Tcl_CreateObjCommand --

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole original jim.c/jim.h retained`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_child_alias_inventory298/jim.c`, function `JimDecrCmdRefCount`, lines 3887–3911. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
static void JimDecrCmdRefCount(Jim_Interp *interp, Jim_Cmd *cmdPtr)
{
    if (--cmdPtr->inUse == 0) {
        if (cmdPtr->flags & JIM_CMD_ISPROC) {
            Jim_DecrRefCount(interp, cmdPtr->u.proc.argListObjPtr);
            Jim_DecrRefCount(interp, cmdPtr->u.proc.bodyObjPtr);
            Jim_DecrRefCount(interp, cmdPtr->u.proc.nsObj);
            if (cmdPtr->u.proc.staticVars) {
                Jim_FreeHashTable(cmdPtr->u.proc.staticVars);
                Jim_Free(cmdPtr->u.proc.staticVars);
            }
        }
        else {
            /* native (C) */
            if (cmdPtr->u.native.delProc) {
                cmdPtr->u.native.delProc(interp, cmdPtr->u.native.privData);
            }
        }
        if (cmdPtr->prevCmd) {
            /* Delete any pushed command too */
            JimDecrCmdRefCount(interp, cmdPtr->prevCmd);
        }

        /* Preserve the structure with inUse = 0 so that
         * cached references will continue to work.

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole original jim.c/jim.h retained`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_child_alias_inventory298/jim.c`, function `JimCreateCommand`, lines 4131–4180. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
static void JimCreateCommand(Jim_Interp *interp, Jim_Obj *nameObjPtr, Jim_Cmd *cmd)
{
    /* If the entry already exists, nameObjPtr will not be used,
     * so the refCount of nameObjPtr can't be zero, relying on this function to
     * release it in that case.
     */
    JimPanic((nameObjPtr->refCount == 0, "JimCreateCommand called with zero ref count name"));

    /* It may already exist, so we try to delete the old one.
     * Note that reference count means that it won't be deleted yet if
     * it exists in the call stack.
     *
     * BUT, if 'local' is in force, instead of deleting the existing
     * proc, we stash a reference to the old proc here.
     */
    if (interp->local) {
        Jim_HashEntry *he = Jim_FindHashEntry(&interp->commands, nameObjPtr);
        if (he) {
            /* Push this command over the top of the previous one */
            cmd->prevCmd = Jim_GetHashEntryVal(he);
            Jim_SetHashVal(&interp->commands, he, cmd);
            /* Need to increment the proc epoch here so that the new command will be used */
            Jim_InterpIncrProcEpoch(interp);
            return;
        }
    }

    /* Otherwise simply replace any existing command */

    /* Note that it is not necessary to increment the 'proc epoch' because any
     * existing command that is replaced will be held as a negative cache entry
     * until the next time the proc epoch is incremented.
     */
    Jim_ReplaceHashEntry(&interp->commands, nameObjPtr, cmd);
}

/* commands */
Jim_Cmd *Jim_RegisterCommand(Jim_Interp *interp, Jim_Obj *cmdNameObj,
    Jim_CmdProc *cmdProc,
    Jim_DelCmdProc *delProc,
    const char *usage,
    const char *help,
    short minargs,
    short maxargs,
    int flags,
    void *privData)
{
    JimPanic(((flags & JIM_CMD_ISPROC), "Jim_RegisterCommand called with JIM_CMD_ISPROC flag"));
    Jim_Cmd *cmdPtr = Jim_Alloc(sizeof(*cmdPtr));

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole original jim.c/jim.h retained`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_child_alias_inventory298/jim.c`, function `Jim_CreateCommand`, lines 4204–4210. Retained reference `native_jim_alias_error_extent279-request-jim.c`.

```text
int Jim_CreateCommand(Jim_Interp *interp, const char *cmdNameStr,
    Jim_CmdProc *cmdProc, void *privData, Jim_DelCmdProc *delProc)
{
    Jim_RegisterCmd(interp, cmdNameStr, NULL, 0, -1, cmdProc, delProc, privData, 0);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole original jim.c/jim.h retained`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_child_alias_inventory298/jim.h`, function `Jim public command callback declarations`, lines 505–510. Retained reference `native_jim_core_alias_prefix273-jim.h`.

```text
/* The cmd structure. */
typedef int Jim_CmdProc(struct Jim_Interp *interp, int argc,
    Jim_Obj *const *argv);
typedef void Jim_DelCmdProc(struct Jim_Interp *interp, void *privData);

/* The dict structure. It uses the same approach as Python OrderedDict

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact whole original jim.c/jim.h retained`, `/workspace/tcl-lsp/rust/tcl-registry/tests/data/native_child_alias_inventory298/jim.h`, function `Jim public command callback declarations`, lines 927–943. Retained reference `native_jim_core_alias_prefix273-jim.h`.

```text
JIM_EXPORT int Jim_CreateCommand (Jim_Interp *interp,
        const char *cmdName, Jim_CmdProc *cmdProc, void *privData,
         Jim_DelCmdProc *delProc);
/* Simplify creating commands that specify minargs, maxargs and usage but
 * don't need delProc or privData
 */
#define Jim_RegisterSimpleCmd(interp, name, usage, minargs, maxargs, cmdproc) \
        Jim_RegisterCommand(interp, Jim_NewStringObj(interp, name, -1), cmdproc, NULL, usage, NULL, minargs, maxargs, 0, NULL)
/* And also slightly more complex where delProc, privData and flags may be needed */
#define Jim_RegisterCmd(interp, name, usage, minargs, maxargs, cmdproc, delproc, privdata, flags) \
        Jim_RegisterCommand(interp, Jim_NewStringObj(interp, name, -1), cmdproc, delproc, usage, NULL, minargs, maxargs, flags, privdata)

/** Register the built-in core commands in an interpreter. */
JIM_EXPORT void Jim_RegisterCoreCommands (Jim_Interp *interp);
/** Delete a command by name object. */
JIM_EXPORT int Jim_DeleteCommand (Jim_Interp *interp,
        Jim_Obj *cmdNameObj);

```

tcl8.4 8.4.20, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_CreateObjCommand`, lines 1737–1869. Retained reference `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c`.

```text
Tcl_Command
Tcl_CreateObjCommand(interp, cmdName, proc, clientData, deleteProc)
    Tcl_Interp *interp;		/* Token for command interpreter (returned
				 * by previous call to Tcl_CreateInterp). */
    CONST char *cmdName;	/* Name of command. If it contains namespace
				 * qualifiers, the new command is put in the
				 * specified namespace; otherwise it is put
				 * in the global namespace. */
    Tcl_ObjCmdProc *proc;	/* Object-based procedure to associate with
				 * name. */
    ClientData clientData;	/* Arbitrary value to pass to object
    				 * procedure. */
    Tcl_CmdDeleteProc *deleteProc;
				/* If not NULL, gives a procedure to call
				 * when this command is deleted. */
{
    Interp *iPtr = (Interp *) interp;
    ImportRef *oldRefPtr = NULL;
    Namespace *nsPtr, *dummy1, *dummy2;
    Command *cmdPtr, *refCmdPtr;
    Tcl_HashEntry *hPtr;
    CONST char *tail;
    int new;
    ImportedCmdData *dataPtr;

    if (iPtr->flags & DELETED) {
	/*
	 * The interpreter is being deleted.  Don't create any new
	 * commands;  it's not safe to muck with the interpreter anymore.
	 */

	return (Tcl_Command) NULL;
    }

    /*
     * Determine where the command should reside. If its name contains 
     * namespace qualifiers, we put it in the specified namespace; 
     * otherwise, we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
       TclGetNamespaceForQualName(interp, cmdName, (Namespace *) NULL,
           CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
       if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

    hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &new);
    if (!new) {
	cmdPtr = (Command *) Tcl_GetHashValue(hPtr);

	/*
	 * Command already exists. If its object-based Tcl_ObjCmdProc is
	 * TclInvokeStringCommand, we just set its Tcl_ObjCmdProc to the
	 * argument "proc". Otherwise, we delete the old command. 
	 */

	if (cmdPtr->objProc == TclInvokeStringCommand) {
	    cmdPtr->objProc = proc;
	    cmdPtr->objClientData = clientData;
            cmdPtr->deleteProc = deleteProc;
            cmdPtr->deleteData = clientData;
	    return (Tcl_Command) cmdPtr;
	}

	/*
	 * Otherwise, we delete the old command.  Be careful to preserve
	 * any existing import links so we can restore them down below.
	 * That way, you can redefine a command and its import status
	 * will remain intact.
	 */

	oldRefPtr = cmdPtr->importRefPtr;
	cmdPtr->importRefPtr = NULL;

	Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &new);
	if (!new) {
	    /*
	     * If the deletion callback recreated the command, just throw
	     * away the new command (if we try to delete it again, we
	     * could get stuck in an infinite loop).
	     */

	     ckfree((char *) Tcl_GetHashValue(hPtr));
	}
    }
    cmdPtr = (Command *) ckalloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = (CompileProc *) NULL;
    cmdPtr->objProc = proc;
    cmdPtr->objClientData = clientData;
    cmdPtr->proc = TclInvokeObjectCommand;
    cmdPtr->clientData = (ClientData) cmdPtr;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;

    /*
     * Plug in any existing import references found above.  Be sure
     * to update all of these references to point to the new command.
     */

    if (oldRefPtr != NULL) {
	cmdPtr->importRefPtr = oldRefPtr;
	while (oldRefPtr != NULL) {
	    refCmdPtr = oldRefPtr->importedCmdPtr;
	    dataPtr = (ImportedCmdData*)refCmdPtr->objClientData;
	    dataPtr->realCmdPtr = cmdPtr;
	    oldRefPtr = oldRefPtr->nextPtr;
	}
    }
    
    /*
     * We just created a command, so in its namespace and all of its parent
     * namespaces, it may shadow global commands with the same name. If any
     * shadowed commands are found, invalidate all cached command references
     * in the affected namespaces.
     */
    
    TclResetShadowedCmdRefs(interp, cmdPtr);
    return (Tcl_Command) cmdPtr;
}

```

tcl8.5 8.5.19, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_CreateObjCommand`, lines 1998–2153. Retained reference `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c`.

```text
Tcl_Command
Tcl_CreateObjCommand(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * previous call to Tcl_CreateInterp). */
    const char *cmdName,	/* Name of command. If it contains namespace
				 * qualifiers, the new command is put in the
				 * specified namespace; otherwise it is put in
				 * the global namespace. */
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    ClientData clientData,	/* Arbitrary value to pass to object
    				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    Interp *iPtr = (Interp *) interp;
    ImportRef *oldRefPtr = NULL;
    Namespace *nsPtr, *dummy1, *dummy2;
    Command *cmdPtr, *refCmdPtr;
    Tcl_HashEntry *hPtr;
    const char *tail;
    int isNew;
    ImportedCmdData *dataPtr;

    if (iPtr->flags & DELETED) {
	/*
	 * The interpreter is being deleted. Don't create any new commands;
	 * it's not safe to muck with the interpreter anymore.
	 */

	return (Tcl_Command) NULL;
    }

    /*
     * Determine where the command should reside. If its name contains
     * namespace qualifiers, we put it in the specified namespace; otherwise,
     * we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
	TclGetNamespaceForQualName(interp, cmdName, NULL,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

    hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &isNew);
    TclInvalidateNsPath(nsPtr);
    if (!isNew) {
	cmdPtr = Tcl_GetHashValue(hPtr);

	/* Command already exists. */

	/*
	 * [***] This is wrong.  See Tcl Bug a16752c252.  
	 * However, this buggy behavior is kept under particular
	 * circumstances to accommodate deployed binaries of the
	 * "tclcompiler" program. http://sourceforge.net/projects/tclpro/
	 * that crash if the bug is fixed.
	 */

	if (cmdPtr->objProc == TclInvokeStringCommand
		&& cmdPtr->clientData == clientData
		&& cmdPtr->deleteData == clientData
		&& cmdPtr->deleteProc == deleteProc) {
	    cmdPtr->objProc = proc;
	    cmdPtr->objClientData = clientData;
	    return (Tcl_Command) cmdPtr;
	}

	/*
	 * Otherwise, we delete the old command. Be careful to preserve any
	 * existing import links so we can restore them down below. That way,
	 * you can redefine a command and its import status will remain
	 * intact.
	 */

	cmdPtr->refCount++;
	if (cmdPtr->importRefPtr) {
	    cmdPtr->flags |= CMD_REDEF_IN_PROGRESS;
	}

	Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);

	if (cmdPtr->flags & CMD_REDEF_IN_PROGRESS) {
	    oldRefPtr = cmdPtr->importRefPtr;
	    cmdPtr->importRefPtr = NULL;
	}
	TclCleanupCommandMacro(cmdPtr);

	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, tail, &isNew);
	if (!isNew) {
	    /*
	     * If the deletion callback recreated the command, just throw away
	     * the new command (if we try to delete it again, we could get
	     * stuck in an infinite loop).
	     */

	     ckfree(Tcl_GetHashValue(hPtr));
	}
    } else {
	/*
	 * The list of command exported from the namespace might have changed.
	 * However, we do not need to recompute this just yet; next time we
	 * need the info will be soon enough.
	 */

	TclInvalidateNsCmdLookup(nsPtr);
    }
    cmdPtr = (Command *) ckalloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc = proc;
    cmdPtr->objClientData = clientData;
    cmdPtr->proc = TclInvokeObjectCommand;
    cmdPtr->clientData = cmdPtr;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;

    /*
     * Plug in any existing import references found above. Be sure to update
     * all of these references to point to the new command.
     */

    if (oldRefPtr != NULL) {
	cmdPtr->importRefPtr = oldRefPtr;
	while (oldRefPtr != NULL) {
	    refCmdPtr = oldRefPtr->importedCmdPtr;
	    dataPtr = refCmdPtr->objClientData;
	    dataPtr->realCmdPtr = cmdPtr;
	    oldRefPtr = oldRefPtr->nextPtr;
	}
    }

    /*
     * We just created a command, so in its namespace and all of its parent
     * namespaces, it may shadow global commands with the same name. If any
     * shadowed commands are found, invalidate all cached command references
     * in the affected namespaces.
     */

    TclResetShadowedCmdRefs(interp, cmdPtr);
    return (Tcl_Command) cmdPtr;
}

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_CreateObjCommand`, lines 2283–2332. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
Tcl_Command
Tcl_CreateObjCommand(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * previous call to Tcl_CreateInterp). */
    const char *cmdName,	/* Name of command. If it contains namespace
				 * qualifiers, the new command is put in the
				 * specified namespace; otherwise it is put in
				 * the global namespace. */
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    ClientData clientData,	/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    Interp *iPtr = (Interp *) interp;
    Namespace *nsPtr;
    const char *tail;

    if (iPtr->flags & DELETED) {
	/*
	 * The interpreter is being deleted. Don't create any new commands;
	 * it's not safe to muck with the interpreter anymore.
	 */
	return (Tcl_Command) NULL;
    }

    /*
     * Determine where the command should reside. If its name contains
     * namespace qualifiers, we put it in the specified namespace;
     * otherwise, we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
	Namespace *dummy1, *dummy2;

	TclGetNamespaceForQualName(interp, cmdName, NULL,
	    TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

    return TclCreateObjCommandInNs(interp, tail, (Tcl_Namespace *) nsPtr,
	proc, clientData, deleteProc);
}

```

tcl8.6 8.6.18, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2334–2502. Retained reference `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
Tcl_Command
TclCreateObjCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,	/* Name of command, without any namespace
				 * components. */
    Tcl_Namespace *namesp,	/* The namespace to create the command in */
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    ClientData clientData,	/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    int deleted = 0, isNew = 0;
    Command *cmdPtr;
    ImportRef *oldRefPtr = NULL;
    ImportedCmdData *dataPtr;
    Tcl_HashEntry *hPtr;
    Namespace *nsPtr = (Namespace *) namesp;

    /*
     * If the command name we seek to create already exists, we need to delete
     * that first. That can be tricky in the presence of traces. Loop until we
     * no longer find an existing command in the way, or until we've deleted
     * one command and that didn't finish the job.
     */

    while (1) {
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, cmdName, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * [***] This is wrong.  See Tcl Bug a16752c252.
	 * However, this buggy behavior is kept under particular circumstances
	 * to accommodate deployed binaries of the "tclcompiler" program
	 * <http://sourceforge.net/projects/tclpro/> that crash if the bug is
	 * fixed.
	 */

	if (cmdPtr->objProc == TclInvokeStringCommand
		&& cmdPtr->clientData == clientData
		&& cmdPtr->deleteData == clientData
		&& cmdPtr->deleteProc == deleteProc) {
	    cmdPtr->objProc = proc;
	    cmdPtr->objClientData = clientData;
	    return (Tcl_Command)cmdPtr;
	}

	/*
	 * Otherwise, we delete the old command. Be careful to preserve any
	 * existing import links so we can restore them down below. That way,
	 * you can redefine a command and its import status will remain
	 * intact.
	 */

	cmdPtr->refCount++;
	if (cmdPtr->importRefPtr) {
	    cmdPtr->flags |= CMD_REDEF_IN_PROGRESS;
	}

	/*
	 * Make sure namespace doesn't get deallocated.
	 */

	cmdPtr->nsPtr->refCount++;

	Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	nsPtr = (Namespace *) TclEnsureNamespace(interp,
		(Tcl_Namespace *) cmdPtr->nsPtr);
	/* Note nsPtr may or may not be same as cmdPtr->nsPtr */
	TclNsDecrRefCount(cmdPtr->nsPtr);

	if (cmdPtr->flags & CMD_REDEF_IN_PROGRESS) {
	    oldRefPtr = cmdPtr->importRefPtr;
	    cmdPtr->importRefPtr = NULL;
	}
	TclCleanupCommandMacro(cmdPtr);
	deleted = 1;
    }
    if (!isNew) {
	/*
	 * If the deletion callback recreated the command, just throw away the
	 * new command (if we try to delete it again, we could get stuck in an
	 * infinite loop).
	 */

	ckfree(Tcl_GetHashValue(hPtr));
    }

    if (!deleted) {
	/*
	 * Command resolvers (per-interp, per-namespace) might have resolved
	 * to a command for the given namespace scope with this command not
	 * being registered with the namespace's command table. During BC
	 * compilation, the so-resolved command turns into a CmdName literal.
	 * Without invalidating a possible CmdName literal here explicitly,
	 * such literals keep being reused while pointing to overhauled
	 * commands.
	 */

	TclInvalidateCmdLiteral(interp, cmdName, nsPtr);

	/*
	 * The list of command exported from the namespace might have changed.
	 * However, we do not need to recompute this just yet; next time we
	 * need the info will be soon enough.
	 */

	TclInvalidateNsCmdLookup(nsPtr);
	TclInvalidateNsPath(nsPtr);
    }
    cmdPtr = (Command *)ckalloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc = proc;
    cmdPtr->objClientData = clientData;
    cmdPtr->proc = TclInvokeObjectCommand;
    cmdPtr->clientData = cmdPtr;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;
    cmdPtr->nreProc = NULL;

    /*
     * Plug in any existing import references found above. Be sure to update
     * all of these references to point to the new command.
     */

    if (oldRefPtr != NULL) {
	cmdPtr->importRefPtr = oldRefPtr;
	while (oldRefPtr != NULL) {
	    Command *refCmdPtr = oldRefPtr->importedCmdPtr;

	    dataPtr = (ImportedCmdData*)refCmdPtr->objClientData;
	    dataPtr->realCmdPtr = cmdPtr;
	    oldRefPtr = oldRefPtr->nextPtr;
	}
    }

    /*
     * We just created a command, so in its namespace and all of its parent
     * namespaces, it may shadow global commands with the same name. If any
     * shadowed commands are found, invalidate all cached command references
     * in the affected namespaces.
     */

    TclResetShadowedCmdRefs(interp, cmdPtr);
    return (Tcl_Command) cmdPtr;
}

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_CreateObjCommand`, lines 2873–2922. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
Tcl_Command
Tcl_CreateObjCommand(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * previous call to Tcl_CreateInterp). */
    const char *cmdName,	/* Name of command. If it contains namespace
				 * qualifiers, the new command is put in the
				 * specified namespace; otherwise it is put in
				 * the global namespace. */
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    void *clientData,		/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    Interp *iPtr = (Interp *)interp;
    Namespace *nsPtr;
    const char *tail;

    if (iPtr->flags & DELETED) {
	/*
	 * The interpreter is being deleted. Don't create any new commands;
	 * it's not safe to muck with the interpreter anymore.
	 */
	return NULL;
    }

    /*
     * Determine where the command should reside. If its name contains
     * namespace qualifiers, we put it in the specified namespace;
     * otherwise, we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
	Namespace *dummy1, *dummy2;

	TclGetNamespaceForQualName(interp, cmdName, NULL,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

    return TclCreateObjCommandInNs(interp, tail, (Tcl_Namespace *) nsPtr,
	proc, clientData, deleteProc);
}

```

tcl9.0 9.0.4, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2924–3077. Retained reference `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
Tcl_Command
TclCreateObjCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,	/* Name of command, without any namespace
				 * components. */
    Tcl_Namespace *namesp,	/* The namespace to create the command in */
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    void *clientData,		/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    int deleted = 0, isNew = 0;
    Command *cmdPtr;
    ImportRef *oldRefPtr = NULL;
    ImportedCmdData *dataPtr;
    Tcl_HashEntry *hPtr;
    Namespace *nsPtr = (Namespace *) namesp;

    /*
     * If the command name we seek to create already exists, we need to delete
     * that first. That can be tricky in the presence of traces. Loop until we
     * no longer find an existing command in the way, or until we've deleted
     * one command and that didn't finish the job.
     */

    while (1) {
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, cmdName, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * Command already exists; delete it. Be careful to preserve any
	 * existing import links so we can restore them down below. That way,
	 * you can redefine a command and its import status will remain
	 * intact.
	 */

	cmdPtr->refCount++;
	if (cmdPtr->importRefPtr) {
	    cmdPtr->flags |= CMD_REDEF_IN_PROGRESS;
	}

	/*
	 * Make sure namespace doesn't get deallocated.
	 */

	cmdPtr->nsPtr->refCount++;

	Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	nsPtr = (Namespace *) TclEnsureNamespace(interp,
		(Tcl_Namespace *) cmdPtr->nsPtr);
	/* Note nsPtr may or may not be same as cmdPtr->nsPtr */
	TclNsDecrRefCount(cmdPtr->nsPtr);

	if (cmdPtr->flags & CMD_REDEF_IN_PROGRESS) {
	    oldRefPtr = cmdPtr->importRefPtr;
	    cmdPtr->importRefPtr = NULL;
	}
	TclCleanupCommandMacro(cmdPtr);
	deleted = 1;
    }
    if (!isNew) {
	/*
	 * If the deletion callback recreated the command, just throw away the
	 * new command (if we try to delete it again, we could get stuck in an
	 * infinite loop).
	 */

	Tcl_Free(Tcl_GetHashValue(hPtr));
    }

    if (!deleted) {
	/*
	 * Command resolvers (per-interp, per-namespace) might have resolved
	 * to a command for the given namespace scope with this command not
	 * being registered with the namespace's command table. During BC
	 * compilation, the so-resolved command turns into a CmdName literal.
	 * Without invalidating a possible CmdName literal here explicitly,
	 * such literals keep being reused while pointing to overhauled
	 * commands.
	 */

	TclInvalidateCmdLiteral(interp, cmdName, nsPtr);

	/*
	 * The list of command exported from the namespace might have changed.
	 * However, we do not need to recompute this just yet; next time we
	 * need the info will be soon enough.
	 */

	TclInvalidateNsCmdLookup(nsPtr);
	TclInvalidateNsPath(nsPtr);
    }
    cmdPtr = (Command *)Tcl_Alloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc = proc;
    cmdPtr->objClientData = clientData;
    cmdPtr->proc = NULL;
    cmdPtr->clientData = NULL;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;
    cmdPtr->nreProc = NULL;

    /*
     * Plug in any existing import references found above. Be sure to update
     * all of these references to point to the new command.
     */

    if (oldRefPtr != NULL) {
	cmdPtr->importRefPtr = oldRefPtr;
	while (oldRefPtr != NULL) {
	    Command *refCmdPtr = oldRefPtr->importedCmdPtr;

	    dataPtr = (ImportedCmdData*)refCmdPtr->objClientData;
	    cmdPtr->refCount++;
	    TclCleanupCommandMacro(dataPtr->realCmdPtr);
	    dataPtr->realCmdPtr = cmdPtr;
	    oldRefPtr = oldRefPtr->nextPtr;
	}
    }

    /*
     * We just created a command, so in its namespace and all of its parent
     * namespaces, it may shadow global commands with the same name. If any
     * shadowed commands are found, invalidate all cached command references
     * in the affected namespaces.
     */

    TclResetShadowedCmdRefs(interp, cmdPtr);
    return (Tcl_Command) cmdPtr;
}

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_CreateObjCommand`, lines 2800–2825. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
Tcl_Command
Tcl_CreateObjCommand(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * previous call to Tcl_CreateInterp). */
    const char *cmdName,	/* Name of command. If it contains namespace
				 * qualifiers, the new command is put in the
				 * specified namespace; otherwise it is put in
				 * the global namespace. */
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    void *clientData,		/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    CmdWrapperInfo *info = (CmdWrapperInfo *)Tcl_Alloc(sizeof(CmdWrapperInfo));
    info->proc = proc;
    info->clientData = clientData;
    info->deleteProc = deleteProc;
    info->deleteData = clientData;

    return Tcl_CreateObjCommand2(interp, cmdName,
	    (proc ? CmdWrapperProc : NULL),
	    info, CmdWrapperDeleteProc);
}

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_CreateObjCommand2`, lines 2828–2877. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
Tcl_Command
Tcl_CreateObjCommand2(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * previous call to Tcl_CreateInterp). */
    const char *cmdName,	/* Name of command. If it contains namespace
				 * qualifiers, the new command is put in the
				 * specified namespace; otherwise it is put in
				 * the global namespace. */
    Tcl_ObjCmdProc2 *proc,	/* Object-based function to associate with
				 * name. */
    void *clientData,		/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    Interp *iPtr = (Interp *)interp;
    Namespace *nsPtr;
    const char *tail;

    if (iPtr->flags & DELETED) {
	/*
	 * The interpreter is being deleted. Don't create any new commands;
	 * it's not safe to muck with the interpreter anymore.
	 */
	return NULL;
    }

    /*
     * Determine where the command should reside. If its name contains
     * namespace qualifiers, we put it in the specified namespace;
     * otherwise, we always put it in the global namespace.
     */

    if (strstr(cmdName, "::") != NULL) {
	Namespace *dummy1, *dummy2;

	TclGetNamespaceForQualName(interp, cmdName, NULL,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy1, &dummy2, &tail);
	if ((nsPtr == NULL) || (tail == NULL)) {
	    return (Tcl_Command) NULL;
	}
    } else {
	nsPtr = iPtr->globalNsPtr;
	tail = cmdName;
    }

    return TclCreateObjCommandInNs(interp, tail, (Tcl_Namespace *) nsPtr,
	proc, clientData, deleteProc);
}

```

tcl9.1 9.1.0, revision `Exact release source/hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2879–3033. Retained reference `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
Tcl_Command
TclCreateObjCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,	/* Name of command, without any namespace
				 * components. */
    Tcl_Namespace *namesp,	/* The namespace to create the command in */
    Tcl_ObjCmdProc2 *proc,	/* Object-based function to associate with
				 * name. */
    void *clientData,		/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    bool deleted = false;
    int isNew = 0;
    Command *cmdPtr;
    ImportRef *oldRefPtr = NULL;
    ImportedCmdData *dataPtr;
    Tcl_HashEntry *hPtr;
    Namespace *nsPtr = (Namespace *) namesp;

    /*
     * If the command name we seek to create already exists, we need to delete
     * that first. That can be tricky in the presence of traces. Loop until we
     * no longer find an existing command in the way, or until we've deleted
     * one command and that didn't finish the job.
     */

    while (1) {
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, cmdName, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * Command already exists; delete it. Be careful to preserve any
	 * existing import links so we can restore them down below. That way,
	 * you can redefine a command and its import status will remain
	 * intact.
	 */

	cmdPtr->refCount++;
	if (cmdPtr->importRefPtr) {
	    cmdPtr->flags |= CMD_REDEF_IN_PROGRESS;
	}

	/*
	 * Make sure namespace doesn't get deallocated.
	 */

	cmdPtr->nsPtr->refCount++;

	Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	nsPtr = (Namespace *) TclEnsureNamespace(interp,
		(Tcl_Namespace *) cmdPtr->nsPtr);
	/* Note nsPtr may or may not be same as cmdPtr->nsPtr */
	TclNsDecrRefCount(cmdPtr->nsPtr);

	if (cmdPtr->flags & CMD_REDEF_IN_PROGRESS) {
	    oldRefPtr = cmdPtr->importRefPtr;
	    cmdPtr->importRefPtr = NULL;
	}
	TclCleanupCommandMacro(cmdPtr);
	deleted = true;
    }
    if (!isNew) {
	/*
	 * If the deletion callback recreated the command, just throw away the
	 * new command (if we try to delete it again, we could get stuck in an
	 * infinite loop).
	 */

	Tcl_Free(Tcl_GetHashValue(hPtr));
    }

    if (!deleted) {
	/*
	 * Command resolvers (per-interp, per-namespace) might have resolved
	 * to a command for the given namespace scope with this command not
	 * being registered with the namespace's command table. During BC
	 * compilation, the so-resolved command turns into a CmdName literal.
	 * Without invalidating a possible CmdName literal here explicitly,
	 * such literals keep being reused while pointing to overhauled
	 * commands.
	 */

	TclInvalidateCmdLiteral(interp, cmdName, nsPtr);

	/*
	 * The list of command exported from the namespace might have changed.
	 * However, we do not need to recompute this just yet; next time we
	 * need the info will be soon enough.
	 */

	TclInvalidateNsCmdLookup(nsPtr);
	TclInvalidateNsPath(nsPtr);
    }
    cmdPtr = (Command *)Tcl_Alloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc2 = proc;
    cmdPtr->objClientData2 = clientData;
    cmdPtr->proc = NULL;
    cmdPtr->clientData = NULL;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;
    cmdPtr->nreProc2 = NULL;

    /*
     * Plug in any existing import references found above. Be sure to update
     * all of these references to point to the new command.
     */

    if (oldRefPtr != NULL) {
	cmdPtr->importRefPtr = oldRefPtr;
	while (oldRefPtr != NULL) {
	    Command *refCmdPtr = oldRefPtr->importedCmdPtr;

	    dataPtr = (ImportedCmdData*)refCmdPtr->objClientData2;
	    cmdPtr->refCount++;
	    TclCleanupCommandMacro(dataPtr->realCmdPtr);
	    dataPtr->realCmdPtr = cmdPtr;
	    oldRefPtr = oldRefPtr->nextPtr;
	}
    }

    /*
     * We just created a command, so in its namespace and all of its parent
     * namespaces, it may shadow global commands with the same name. If any
     * shadowed commands are found, invalidate all cached command references
     * in the affected namespaces.
     */

    TclResetShadowedCmdRefs(interp, cmdPtr);
    return (Tcl_Command) cmdPtr;
}

```

## Consumer bindings

- [runtime/rust/src/interp/native_command_recreation.rs](../../../../runtime/rust/src/interp/native_command_recreation.rs), `interp::native_command_recreation::tests::original_idle_object_recreation_matches_all_six_public_callback_windows` (linked): The authored Runtime comparison keeps all60 original public callback counter windows from six canonical streams, two exact idle cases and unchanged invocation bytes. OLD/FINAL deletion, MIDDLE discard and FINAL result fields are compared through teardown. Runtime identities remain independently distinct; diagnostic native addresses, generation/active/procedure parity and whole backend equivalence are not asserted. This comparison definition is unexecuted.
- [runtime/rust/src/interp/native_command_recreation.rs](../../../../runtime/rust/src/interp/native_command_recreation.rs), `interp::native_command_recreation::tests::original_idle_retirement_requires_completed_original_delete_and_exact_creation_slot` (linked): Retained software ownership keeps original deletion incomplete; only actual owner retirement and an originally empty exact slot admit the separate witness. Wrong kind, generation and namespace/name entry refuse independently. Counted records and procedures receive no public idle discard authority.
- [runtime/rust/src/interp/native_command_recreation.rs](../../../../runtime/rust/src/interp/native_command_recreation.rs), `interp::native_command_recreation::tests::original_owned_shim_purpose_survives_the_public_constructor_facade` (linked): The actual owned Runtime shim retains its issuing purpose through the public facade. It cannot issue an idle public-retirement witness; replacement refuses after reached OLD callback effects and its recreated record receives ordinary owned teardown cleanup.
- [runtime/rust/src/interp/native_command_recreation.rs](../../../../runtime/rust/src/interp/native_command_recreation.rs), `interp::native_command_recreation::tests::original_recreation_extra_owner_and_non_object_purposes_cannot_borrow_idle_discard` (linked): Actual software extra-held records, Builtin/Proc records, CountedEngine object producers and OwnedRuntimeShim producers cannot borrow the idle LegacyPublic discard purpose. Typed refusal withholds FINAL publication while those independently owned intermediate records retain ordinary cleanup.
- [rust/tcl-registry/src/native_command_creation.rs](../../../../rust/tcl-registry/src/native_command_creation.rs), `native_command_creation::tests::original_idle_recreation_recipe_requires_a_supported_native_point` (linked): The pure Registry recipe selects LegacyPublicObjectCommand only at supported actual C/Jim naming points. CountedEngine and OwnedRuntimeShim, authored/Logical context, absent point and conflicting release refuse; selecting a recipe issues no original current table, active frame or object ownership.
- [rust/tcl-registry/src/native_command_creation.rs](../../../../rust/tcl-registry/src/native_command_creation.rs), `native_command_creation::tests::original_public_registration_selects_c_namespace_and_jim_root_cstring_purposes` (linked): Finite source-defined control: original public registration selects c namespace and jim root cstring purposes. Pure source-selected registration units; no live table/header or counted Jim API equivalence is inferred from the retirement rows. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/interp/native_command_recreation.rs](../../../../runtime/rust/src/interp/native_command_recreation.rs), `interp::native_command_recreation::tests::original_jim_public_constructor_owns_root_cstring_key_and_retains_occupied_key` (linked): Finite source-defined control: original jim public constructor owns root cstring key and retains occupied key. Actual software CString/key/client-data roles, independent of the native recreation probe's finite callback-count comparison. SAFETY: the live original table owns this borrowed key header. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [runtime/rust/src/interp/native_command_recreation.rs](../../../../runtime/rust/src/interp/native_command_recreation.rs), `interp::native_command_recreation::tests::original_idle_jim_recreation_keeps_the_occupied_key_until_public_delete` (linked): Actual Jim software occupied original key survives reached once-only recreation and final invocation, then retires at explicit public delete. Its allocation-only lease adds no Native reference. Native486 observes callback/invocation counts, not this key header/refcount lifetime. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs)
- [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source)
- [rust/tcl-test-support/tests/data/native_resolution_reference_builds](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds)
- [rust/tcl-registry/tests/data/native_info_inventory_original/sources](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources)
- [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix)
- [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix)
- [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix)
- [rust/tcl-registry/tests/data/native_command_recreation_original/source-windows/8.4.20](../../../../rust/tcl-registry/tests/data/native_command_recreation_original/source-windows/8.4.20)
- [rust/tcl-registry/tests/data/native_command_recreation_original/source-windows/8.5.19](../../../../rust/tcl-registry/tests/data/native_command_recreation_original/source-windows/8.5.19)
