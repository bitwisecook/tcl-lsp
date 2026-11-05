# naming.embedding.original-active-object-command-deletion

Kind: `native-observation`

## Problem statement

Deletion of an idle public command does not establish when its deletion callback runs during an active object-command invocation. Deferring the deletion callback until the active command returns changes its observable event sequence.

## Question

When an original publicly registered object command deletes itself inside its actual active callback, where does its deletion callback occur relative to deletion-API return, callback INVOKE_RETURN and first public evaluation return, and does a once-only same-name publication by that callback receive the second invocation?

## Conclusion

The five measured C providers reach DELETE_ORIGINAL before DELETE_API_RETURN and INVOKE_RETURN. Measured Jim0.84-9-g5bac7c9 reaches DELETE_ORIGINAL after INVOKE_RETURN and before the first public evaluation returns. All six return exact ORIGINAL_RETURN from that first evaluation. Without recreation the second evaluation fails with the same exact invalid-command result. With once-only recreation the same held caller word reaches the replacement once and returns exact REPLACEMENT on the second evaluation; original and replacement deletion each occur once through full interpreter teardown. These are138 ordered event/result rows across12 fresh active cases in6 closed probe processes. Each independent bare-list CLI query also exits0 but emits no version text; actual linked VERSION/BUILD/ABI rows provide provider attribution. Idle replacement measurements remain independent. No client-data free, reference-count, procedure, imported-command, recursive deletion, rename/hide, Rust or BIG-IP parity follows.

## Scope

Exact original C99 probe SHA 3572cff663d103e59a8e814a7b15002acd5c8320cbf8c415c8fabf04f48a9a05 on measured C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9. Mode0 actively deletes without recreation; mode1 recreates the same public command name once from the original deletion callback. Independent stack-backed client records stay alive through full teardown and are deliberately not freed by the callback. One caller-name object is held through both actual vector evaluations and then released; no extra command-record or protective reference is introduced. C uses Tcl_CreateObjCommand legacy int-count registration, including Tcl9 compatibility, not the size-count registration family. A third independent interpreter per probe supplies VERSION metadata; its result does not donate callback evidence. Full source/header/config/static-library/CLI/compiler/link input hashes and original before/after inventories are associated with their actual receipts. Whole source snippets use physical LF and preserve terminal LF and whitespace. SDK configure/build replay and source-to-library equivalence are unrecorded; external toolchain/system binaries lacking archived payloads remain hash-only. Application issuers, namespace/source/frame receipts, unmeasured command families, object-address/incarnation and general active lifetime claims remain unavailable.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original C Tcl_CreateObjCommand legacy int-count registration/Tcl_DeleteCommandFromToken/Tcl_EvalObjv or Jim_CreateCommand/Jim_DeleteCommand/Jim_EvalObjVector, two fresh initialized case interpreters, one held caller word through both evaluations, complete raw event/result and teardown streams. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

DELETE_ORIGINAL occurs before DELETE_API_RETURN and INVOKE_RETURN in both active cases. Both first evaluations return code0 and exact ORIGINAL_RETURN. Case0 second evaluation returns code1 and exact invalid command name "active_retiring". Case1 recreates once in the original deletion callback; the held caller word second evaluation returns code0 and exact REPLACEMENT, reaches replacement once, and full teardown reaches original and replacement deletion exactly once. All23 original event/result rows, including caller-word release and interpreter destruction, are retained.

### jim

Status: `observed`. Channel: Original C Tcl_CreateObjCommand legacy int-count registration/Tcl_DeleteCommandFromToken/Tcl_EvalObjv or Jim_CreateCommand/Jim_DeleteCommand/Jim_EvalObjVector, two fresh initialized case interpreters, one held caller word through both evaluations, complete raw event/result and teardown streams. Version: jim 0.84-9-g5bac7c9.

DELETE_API_RETURN and INVOKE_RETURN precede DELETE_ORIGINAL in both active cases; deletion and optional recreation finish before first-public-invocation-returned. Both first evaluations return code0 and exact ORIGINAL_RETURN. Case0 second evaluation returns code1 and exact invalid command name "active_retiring". Case1 recreates once in the original deletion callback; the held caller word second evaluation returns code0 and exact REPLACEMENT, reaches replacement once, and full teardown reaches original and replacement deletion exactly once. All23 original event/result rows, including caller-word release and interpreter destruction, are retained.

### bigip

Status: `not-tested`.

No BIG-IP execution or active command deletion result is attached. C Tcl/Jim callback ordering supplies no appliance parity.

## Source inspection

tcl8.4 8.4.20, revision `Exact release source and whole hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_DeleteCommandFromToken`, lines 2550–2696. Line unit: physical LF only. Original byte range [78942, 84048). Retained reference `active671-whole-source-tcl8.4`.

```text
Tcl_DeleteCommandFromToken(interp, cmd)
    Tcl_Interp *interp;		/* Token for command interpreter returned by
				 * a previous call to Tcl_CreateInterp. */
    Tcl_Command cmd;            /* Token for command to delete. */
{
    Interp *iPtr = (Interp *) interp;
    Command *cmdPtr = (Command *) cmd;
    ImportRef *refPtr, *nextRefPtr;
    Tcl_Command importCmd;

    /*
     * The code here is tricky.  We can't delete the hash table entry
     * before invoking the deletion callback because there are cases
     * where the deletion callback needs to invoke the command (e.g.
     * object systems such as OTcl). However, this means that the
     * callback could try to delete or rename the command. The deleted
     * flag allows us to detect these cases and skip nested deletes.
     */

    if (cmdPtr->flags & CMD_IS_DELETED) {
	/*
	 * Another deletion is already in progress.  Remove the hash
	 * table entry now, but don't invoke a callback or free the
	 * command structure.
	 */

        Tcl_DeleteHashEntry(cmdPtr->hPtr);
	cmdPtr->hPtr = NULL;
	return 0;
    }

    /* 
     * We must delete this command, even though both traces and
     * delete procs may try to avoid this (renaming the command etc).
     * Also traces and delete procs may try to delete the command
     * themsevles.  This flag declares that a delete is in progress
     * and that recursive deletes should be ignored.
     */
    cmdPtr->flags |= CMD_IS_DELETED;

    /*
     * Bump the command epoch counter. This will invalidate all cached
     * references that point to this command.
     */
    
    cmdPtr->cmdEpoch++;

    /*
     * Call trace procedures for the command being deleted. Then delete
     * its traces. 
     */

    if (cmdPtr->tracePtr != NULL) {
	CommandTrace *tracePtr;
	CallCommandTraces(iPtr,cmdPtr,NULL,NULL,TCL_TRACE_DELETE);
	/* Now delete these traces */
	tracePtr = cmdPtr->tracePtr;
	while (tracePtr != NULL) {
	    CommandTrace *nextPtr = tracePtr->nextPtr;
	    if ((--tracePtr->refCount) <= 0) {
		ckfree((char*)tracePtr);
	    }
	    tracePtr = nextPtr;
	}
	cmdPtr->tracePtr = NULL;
    }
    
    /*
     * If the command being deleted has a compile procedure, increment the
     * interpreter's compileEpoch to invalidate its compiled code. This
     * makes sure that we don't later try to execute old code compiled with
     * command-specific (i.e., inline) bytecodes for the now-deleted
     * command. This field is checked in Tcl_EvalObj and ObjInterpProc, and
     * code whose compilation epoch doesn't match is recompiled.
     */

    if (cmdPtr->compileProc != NULL) {
        iPtr->compileEpoch++;
    }

    if (cmdPtr->deleteProc != NULL) {
	/*
	 * Delete the command's client data. If this was an imported command
	 * created when a command was imported into a namespace, this client
	 * data will be a pointer to a ImportedCmdData structure describing
	 * the "real" command that this imported command refers to.
	 */
	
	/*
	 * If you are getting a crash during the call to deleteProc and
	 * cmdPtr->deleteProc is a pointer to the function free(), the
	 * most likely cause is that your extension allocated memory
	 * for the clientData argument to Tcl_CreateObjCommand() with
	 * the ckalloc() macro and you are now trying to deallocate
	 * this memory with free() instead of ckfree(). You should
	 * pass a pointer to your own method that calls ckfree().
	 */

	(*cmdPtr->deleteProc)(cmdPtr->deleteData);
    }

    /*
     * If this command was imported into other namespaces, then imported
     * commands were created that refer back to this command. Delete these
     * imported commands now.
     */

    for (refPtr = cmdPtr->importRefPtr;  refPtr != NULL;
            refPtr = nextRefPtr) {
	nextRefPtr = refPtr->nextPtr;
	importCmd = (Tcl_Command) refPtr->importedCmdPtr;
        Tcl_DeleteCommandFromToken(interp, importCmd);
    }

    /*
     * Don't use hPtr to delete the hash entry here, because it's
     * possible that the deletion callback renamed the command.
     * Instead, use cmdPtr->hptr, and make sure that no-one else
     * has already deleted the hash entry.
     */

    if (cmdPtr->hPtr != NULL) {
	Tcl_DeleteHashEntry(cmdPtr->hPtr);
    }

    /*
     * Mark the Command structure as no longer valid. This allows
     * TclExecuteByteCode to recognize when a Command has logically been
     * deleted and a pointer to this Command structure cached in a CmdName
     * object is invalid. TclExecuteByteCode will look up the command again
     * in the interpreter's command hashtable.
     */

    cmdPtr->objProc = NULL;

    /*
     * Now free the Command structure, unless there is another reference to
     * it from a CmdName Tcl object in some ByteCode code sequence. In that
     * case, delay the cleanup until all references are either discarded
     * (when a ByteCode is freed) or replaced by a new reference (when a
     * cached CmdName Command reference is found to be invalid and
     * TclExecuteByteCode looks up the command in the command hashtable).
     */
    
    TclCleanupCommand(cmdPtr);
    return 0;
}

```

tcl8.5 8.5.19, revision `Exact release source and whole hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_DeleteCommandFromToken`, lines 2774–2939. Line unit: physical LF only. Original byte range [84225, 89835). Retained reference `active671-whole-source-tcl8.5`.

```text
Tcl_DeleteCommandFromToken(
    Tcl_Interp *interp,		/* Token for command interpreter returned by a
				 * previous call to Tcl_CreateInterp. */
    Tcl_Command cmd)		/* Token for command to delete. */
{
    Interp *iPtr = (Interp *) interp;
    Command *cmdPtr = (Command *) cmd;
    ImportRef *refPtr, *nextRefPtr;
    Tcl_Command importCmd;

    /*
     * Bump the command epoch counter. This will invalidate all cached
     * references that point to this command.
     */

    cmdPtr->cmdEpoch++;

    /*
     * The code here is tricky. We can't delete the hash table entry before
     * invoking the deletion callback because there are cases where the
     * deletion callback needs to invoke the command (e.g. object systems such
     * as OTcl). However, this means that the callback could try to delete or
     * rename the command. The deleted flag allows us to detect these cases
     * and skip nested deletes.
     */

    if (cmdPtr->flags & CMD_IS_DELETED) {
	/*
	 * Another deletion is already in progress. Remove the hash table
	 * entry now, but don't invoke a callback or free the command
	 * structure. Take care to only remove the hash entry if it has not
	 * already been removed; otherwise if we manage to hit this function
	 * three times, everything goes up in smoke. [Bug 1220058]
	 */

	if (cmdPtr->hPtr != NULL) {
	    Tcl_DeleteHashEntry(cmdPtr->hPtr);
	    cmdPtr->hPtr = NULL;
	}
	return 0;
    }

    /*
     * We must delete this command, even though both traces and delete procs
     * may try to avoid this (renaming the command etc). Also traces and
     * delete procs may try to delete the command themsevles. This flag
     * declares that a delete is in progress and that recursive deletes should
     * be ignored.
     */

    cmdPtr->flags |= CMD_IS_DELETED;

    /*
     * Call trace functions for the command being deleted. Then delete its
     * traces.
     */

    if (cmdPtr->tracePtr != NULL) {
	CommandTrace *tracePtr;
	CallCommandTraces(iPtr,cmdPtr,NULL,NULL,TCL_TRACE_DELETE);

	/*
	 * Now delete these traces.
	 */

	tracePtr = cmdPtr->tracePtr;
	while (tracePtr != NULL) {
	    CommandTrace *nextPtr = tracePtr->nextPtr;
	    if ((--tracePtr->refCount) <= 0) {
		ckfree((char *) tracePtr);
	    }
	    tracePtr = nextPtr;
	}
	cmdPtr->tracePtr = NULL;
    }

    /*
     * The list of command exported from the namespace might have changed.
     * However, we do not need to recompute this just yet; next time we need
     * the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(cmdPtr->nsPtr);

    /*
     * If the command being deleted has a compile function, increment the
     * interpreter's compileEpoch to invalidate its compiled code. This makes
     * sure that we don't later try to execute old code compiled with
     * command-specific (i.e., inline) bytecodes for the now-deleted command.
     * This field is checked in Tcl_EvalObj and ObjInterpProc, and code whose
     * compilation epoch doesn't match is recompiled.
     */

    if (cmdPtr->compileProc != NULL) {
	iPtr->compileEpoch++;
    }

    if (cmdPtr->deleteProc != NULL) {
	/*
	 * Delete the command's client data. If this was an imported command
	 * created when a command was imported into a namespace, this client
	 * data will be a pointer to a ImportedCmdData structure describing
	 * the "real" command that this imported command refers to.
	 */

	/*
	 * If you are getting a crash during the call to deleteProc and
	 * cmdPtr->deleteProc is a pointer to the function free(), the most
	 * likely cause is that your extension allocated memory for the
	 * clientData argument to Tcl_CreateObjCommand() with the ckalloc()
	 * macro and you are now trying to deallocate this memory with free()
	 * instead of ckfree(). You should pass a pointer to your own method
	 * that calls ckfree().
	 */

	(*cmdPtr->deleteProc)(cmdPtr->deleteData);
    }

    /*
     * If this command was imported into other namespaces, then imported
     * commands were created that refer back to this command. Delete these
     * imported commands now.
     */
    if (!(cmdPtr->flags & CMD_REDEF_IN_PROGRESS)) {
	for (refPtr = cmdPtr->importRefPtr;  refPtr != NULL;
		refPtr = nextRefPtr) {
	    nextRefPtr = refPtr->nextPtr;
	    importCmd = (Tcl_Command) refPtr->importedCmdPtr;
	    Tcl_DeleteCommandFromToken(interp, importCmd);
	}
    }

    /*
     * Don't use hPtr to delete the hash entry here, because it's possible
     * that the deletion callback renamed the command. Instead, use
     * cmdPtr->hptr, and make sure that no-one else has already deleted the
     * hash entry.
     */

    if (cmdPtr->hPtr != NULL) {
	Tcl_DeleteHashEntry(cmdPtr->hPtr);
	cmdPtr->hPtr = NULL;
    }

    /*
     * Mark the Command structure as no longer valid. This allows
     * TclExecuteByteCode to recognize when a Command has logically been
     * deleted and a pointer to this Command structure cached in a CmdName
     * object is invalid. TclExecuteByteCode will look up the command again in
     * the interpreter's command hashtable.
     */

    cmdPtr->objProc = NULL;

    /*
     * Now free the Command structure, unless there is another reference to it
     * from a CmdName Tcl object in some ByteCode code sequence. In that case,
     * delay the cleanup until all references are either discarded (when a
     * ByteCode is freed) or replaced by a new reference (when a cached
     * CmdName Command reference is found to be invalid and TclExecuteByteCode
     * looks up the command in the command hashtable).
     */

    TclCleanupCommandMacro(cmdPtr);
    return 0;
}

```

tcl8.6 8.6.18, revision `Exact release source and whole hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_DeleteCommandFromToken`, lines 3147–3320. Line unit: physical LF only. Original byte range [98088, 103755). Retained reference `active671-whole-source-tcl8.6`.

```text
Tcl_DeleteCommandFromToken(
    Tcl_Interp *interp,		/* Token for command interpreter returned by a
				 * previous call to Tcl_CreateInterp. */
    Tcl_Command cmd)		/* Token for command to delete. */
{
    Interp *iPtr = (Interp *) interp;
    Command *cmdPtr = (Command *) cmd;
    ImportRef *refPtr, *nextRefPtr;
    Tcl_Command importCmd;

    /*
     * Bump the command epoch counter. This will invalidate all cached
     * references that point to this command.
     */

    cmdPtr->cmdEpoch++;

    /*
     * The code here is tricky. We can't delete the hash table entry before
     * invoking the deletion callback because there are cases where the
     * deletion callback needs to invoke the command (e.g. object systems such
     * as OTcl). However, this means that the callback could try to delete or
     * rename the command. The deleted flag allows us to detect these cases
     * and skip nested deletes.
     */

    if (cmdPtr->flags & CMD_IS_DELETED) {
	/*
	 * Another deletion is already in progress. Remove the hash table
	 * entry now, but don't invoke a callback or free the command
	 * structure. Take care to only remove the hash entry if it has not
	 * already been removed; otherwise if we manage to hit this function
	 * three times, everything goes up in smoke. [Bug 1220058]
	 */

	if (cmdPtr->hPtr != NULL) {
	    Tcl_DeleteHashEntry(cmdPtr->hPtr);
	    cmdPtr->hPtr = NULL;
	}
	return 0;
    }

    /*
     * We must delete this command, even though both traces and delete procs
     * may try to avoid this (renaming the command etc). Also traces and
     * delete procs may try to delete the command themselves. This flag
     * declares that a delete is in progress and that recursive deletes should
     * be ignored.
     */

    cmdPtr->flags |= CMD_IS_DELETED;

    /*
     * Call trace functions for the command being deleted. Then delete its
     * traces.
     */

    cmdPtr->nsPtr->refCount++;

    if (cmdPtr->tracePtr != NULL) {
	CommandTrace *tracePtr;
	CallCommandTraces(iPtr,cmdPtr,NULL,NULL,TCL_TRACE_DELETE);

	/*
	 * Now delete these traces.
	 */

	tracePtr = cmdPtr->tracePtr;
	while (tracePtr != NULL) {
	    CommandTrace *nextPtr = tracePtr->nextPtr;

	    if (tracePtr->refCount-- <= 1) {
		ckfree(tracePtr);
	    }
	    tracePtr = nextPtr;
	}
	cmdPtr->tracePtr = NULL;
    }

    /*
     * The list of commands exported from the namespace might have changed.
     * However, we do not need to recompute this just yet; next time we need
     * the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(cmdPtr->nsPtr);
    TclNsDecrRefCount(cmdPtr->nsPtr);

    /*
     * If the command being deleted has a compile function, increment the
     * interpreter's compileEpoch to invalidate its compiled code. This makes
     * sure that we don't later try to execute old code compiled with
     * command-specific (i.e., inline) bytecodes for the now-deleted command.
     * This field is checked in Tcl_EvalObj and ObjInterpProc, and code whose
     * compilation epoch doesn't match is recompiled.
     */

    if (cmdPtr->compileProc != NULL) {
	iPtr->compileEpoch++;
    }

    if (!(cmdPtr->flags & CMD_REDEF_IN_PROGRESS)) {
	/*
	 * Delete any imports of this routine before deleting this routine itself.
	 * See issue 688fcc7082fa.
	 */
	for (refPtr = cmdPtr->importRefPtr; refPtr != NULL;
		refPtr = nextRefPtr) {
	    nextRefPtr = refPtr->nextPtr;
	    importCmd = (Tcl_Command) refPtr->importedCmdPtr;
	    Tcl_DeleteCommandFromToken(interp, importCmd);
	}
    }

    if (cmdPtr->deleteProc != NULL) {
	/*
	 * Delete the command's client data. If this was an imported command
	 * created when a command was imported into a namespace, this client
	 * data will be a pointer to a ImportedCmdData structure describing
	 * the "real" command that this imported command refers to.
	 *
	 * If you are getting a crash during the call to deleteProc and
	 * cmdPtr->deleteProc is a pointer to the function free(), the most
	 * likely cause is that your extension allocated memory for the
	 * clientData argument to Tcl_CreateObjCommand with the ckalloc()
	 * macro and you are now trying to deallocate this memory with free()
	 * instead of ckfree(). You should pass a pointer to your own method
	 * that calls ckfree().
	 */

	cmdPtr->deleteProc(cmdPtr->deleteData);
    }

    /*
     * Don't use hPtr to delete the hash entry here, because it's possible
     * that the deletion callback renamed the command. Instead, use
     * cmdPtr->hptr, and make sure that no-one else has already deleted the
     * hash entry.
     */

    if (cmdPtr->hPtr != NULL) {
	Tcl_DeleteHashEntry(cmdPtr->hPtr);
	cmdPtr->hPtr = NULL;

	/*
	 * Bump the command epoch counter. This will invalidate all cached
	 * references that point to this command.
	 */

	cmdPtr->cmdEpoch++;
    }

    /*
     * A number of tests for particular kinds of commands are done by checking
     * whether the objProc field holds a known value. Set the field to NULL so
     * that such tests won't have false positives when applied to deleted
     * commands.
     */

    cmdPtr->objProc = NULL;

    /*
     * Now free the Command structure, unless there is another reference to it
     * from a CmdName Tcl object in some ByteCode code sequence. In that case,
     * delay the cleanup until all references are either discarded (when a
     * ByteCode is freed) or replaced by a new reference (when a cached
     * CmdName Command reference is found to be invalid and
     * TclNRExecuteByteCode looks up the command in the command hashtable).
     */

    cmdPtr->flags |= CMD_DEAD;
    TclCleanupCommandMacro(cmdPtr);
    return 0;
}

```

tcl9.0 9.0.4, revision `Exact release source and whole hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_DeleteCommandFromToken`, lines 3727–3902. Line unit: physical LF only. Original byte range [115770, 121494). Retained reference `active671-whole-source-tcl9.0`.

```text
Tcl_DeleteCommandFromToken(
    Tcl_Interp *interp,		/* Token for command interpreter returned by a
				 * previous call to Tcl_CreateInterp. */
    Tcl_Command cmd)		/* Token for command to delete. */
{
    Interp *iPtr = (Interp *) interp;
    Command *cmdPtr = (Command *) cmd;
    ImportRef *refPtr, *nextRefPtr;
    Tcl_Command importCmd;

    /*
     * The code here is tricky. We can't delete the hash table entry before
     * invoking the deletion callback because there are cases where the
     * deletion callback needs to invoke the command (e.g. object systems such
     * as OTcl). However, this means that the callback could try to delete or
     * rename the command. The deleted flag allows us to detect these cases
     * and skip nested deletes.
     */

    if (cmdPtr->flags & CMD_DYING) {
	/*
	 * Another deletion is already in progress. Remove the hash table
	 * entry now, but don't invoke a callback or free the command
	 * structure. Take care to only remove the hash entry if it has not
	 * already been removed; otherwise if we manage to hit this function
	 * three times, everything goes up in smoke. [Bug 1220058]
	 */

	if (cmdPtr->hPtr != NULL) {
	    Tcl_DeleteHashEntry(cmdPtr->hPtr);
	    cmdPtr->hPtr = NULL;
	}

	/*
	 * Bump the command epoch counter. This will invalidate all cached
	 * references that point to this command.
	 */

	cmdPtr->cmdEpoch++;

	return 0;
    }

    /*
     * We must delete this command, even though both traces and delete procs
     * may try to avoid this (renaming the command etc). Also traces and
     * delete procs may try to delete the command themselves. This flag
     * declares that a delete is in progress and that recursive deletes should
     * be ignored.
     */

    cmdPtr->flags |= CMD_DYING;

    /*
     * Call each functions and then delete the trace.
     */

    cmdPtr->nsPtr->refCount++;

    if (cmdPtr->tracePtr != NULL) {
	CommandTrace *tracePtr;
	/* CallCommandTraces() does not cmdPtr, that's
	 * done just before Tcl_DeleteCommandFromToken() returns */
	CallCommandTraces(iPtr,cmdPtr,NULL,NULL,TCL_TRACE_DELETE);

	/*
	 * Now delete these traces.
	 */

	tracePtr = cmdPtr->tracePtr;
	while (tracePtr != NULL) {
	    CommandTrace *nextPtr = tracePtr->nextPtr;

	    if (tracePtr->refCount-- <= 1) {
		Tcl_Free(tracePtr);
	    }
	    tracePtr = nextPtr;
	}
	cmdPtr->tracePtr = NULL;
    }

    /*
     * The list of commands exported from the namespace might have changed.
     * However, we do not need to recompute this just yet; next time we need
     * the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(cmdPtr->nsPtr);
    TclNsDecrRefCount(cmdPtr->nsPtr);

    /*
     * If the command being deleted has a compile function, increment the
     * interpreter's compileEpoch to invalidate its compiled code. This makes
     * sure that we don't later try to execute old code compiled with
     * command-specific (i.e., inline) bytecodes for the now-deleted command.
     * This field is checked in Tcl_EvalObj and ObjInterpProc, and code whose
     * compilation epoch doesn't match is recompiled.
     */

    if (cmdPtr->compileProc != NULL) {
	iPtr->compileEpoch++;
    }

    if (!(cmdPtr->flags & CMD_REDEF_IN_PROGRESS)) {
	/*
	 * Delete any imports of this routine before deleting this routine itself.
	 * See issue 688fcc7082fa.
	 */
	for (refPtr = cmdPtr->importRefPtr; refPtr != NULL;
		refPtr = nextRefPtr) {
	    nextRefPtr = refPtr->nextPtr;
	    importCmd = (Tcl_Command) refPtr->importedCmdPtr;
	    Tcl_DeleteCommandFromToken(interp, importCmd);
	}
    }

    if (cmdPtr->deleteProc != NULL) {
	/*
	 * Delete the command's client data. If this was an imported command
	 * created when a command was imported into a namespace, this client
	 * data will be a pointer to a ImportedCmdData structure describing
	 * the "real" command that this imported command refers to.
	 *
	 * If you are getting a crash during the call to deleteProc and
	 * cmdPtr->deleteProc is a pointer to the function free(), the most
	 * likely cause is that your extension allocated memory for the
	 * clientData argument to Tcl_CreateObjCommand with the Tcl_Alloc()
	 * macro and you are now trying to deallocate this memory with free()
	 * instead of Tcl_Free(). You should pass a pointer to your own method
	 * that calls Tcl_Free().
	 */

	cmdPtr->deleteProc(cmdPtr->deleteData);
    }

    /*
     * Don't use hPtr to delete the hash entry here, because it's possible
     * that the deletion callback renamed the command. Instead, use
     * cmdPtr->hptr, and make sure that no-one else has already deleted the
     * hash entry.
     */

    if (cmdPtr->hPtr != NULL) {
	Tcl_DeleteHashEntry(cmdPtr->hPtr);
	cmdPtr->hPtr = NULL;

	/*
	 * Bump the command epoch counter. This will invalidate all cached
	 * references that point to this command.
	 */

	cmdPtr->cmdEpoch++;
    }

    /*
     * A number of tests for particular kinds of commands are done by checking
     * whether the objProc field holds a known value. Set the field to NULL so
     * that such tests won't have false positives when applied to deleted
     * commands.
     */

    cmdPtr->objProc = NULL;

    /*
     * Now free the Command structure, unless there is another reference to it
     * from a CmdName Tcl object in some ByteCode code sequence. In that case,
     * delay the cleanup until all references are either discarded (when a
     * ByteCode is freed) or replaced by a new reference (when a cached
     * CmdName Command reference is found to be invalid and
     * TclNRExecuteByteCode looks up the command in the command hashtable).
     */

    cmdPtr->flags |= CMD_DEAD;
    TclCleanupCommandMacro(cmdPtr);
    return 0;
}

```

tcl9.1 9.1.0, revision `Exact release source and whole hash; source-control revision unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_DeleteCommandFromToken`, lines 3697–3872. Line unit: physical LF only. Original byte range [116033, 121758). Retained reference `active671-whole-source-tcl9.1`.

```text
Tcl_DeleteCommandFromToken(
    Tcl_Interp *interp,		/* Token for command interpreter returned by a
				 * previous call to Tcl_CreateInterp. */
    Tcl_Command cmd)		/* Token for command to delete. */
{
    Interp *iPtr = (Interp *) interp;
    Command *cmdPtr = (Command *) cmd;
    ImportRef *refPtr, *nextRefPtr;
    Tcl_Command importCmd;

    /*
     * The code here is tricky. We can't delete the hash table entry before
     * invoking the deletion callback because there are cases where the
     * deletion callback needs to invoke the command (e.g. object systems such
     * as OTcl). However, this means that the callback could try to delete or
     * rename the command. The deleted flag allows us to detect these cases
     * and skip nested deletes.
     */

    if (cmdPtr->flags & CMD_DYING) {
	/*
	 * Another deletion is already in progress. Remove the hash table
	 * entry now, but don't invoke a callback or free the command
	 * structure. Take care to only remove the hash entry if it has not
	 * already been removed; otherwise if we manage to hit this function
	 * three times, everything goes up in smoke. [Bug 1220058]
	 */

	if (cmdPtr->hPtr != NULL) {
	    Tcl_DeleteHashEntry(cmdPtr->hPtr);
	    cmdPtr->hPtr = NULL;
	}

	/*
	 * Bump the command epoch counter. This will invalidate all cached
	 * references that point to this command.
	 */

	cmdPtr->cmdEpoch++;

	return 0;
    }

    /*
     * We must delete this command, even though both traces and delete procs
     * may try to avoid this (renaming the command etc). Also traces and
     * delete procs may try to delete the command themselves. This flag
     * declares that a delete is in progress and that recursive deletes should
     * be ignored.
     */

    cmdPtr->flags |= CMD_DYING;

    /*
     * Call each functions and then delete the trace.
     */

    cmdPtr->nsPtr->refCount++;

    if (cmdPtr->tracePtr != NULL) {
	CommandTrace *tracePtr;
	/* CallCommandTraces() does not cmdPtr, that's
	 * done just before Tcl_DeleteCommandFromToken() returns */
	CallCommandTraces(iPtr,cmdPtr,NULL,NULL,TCL_TRACE_DELETE);

	/*
	 * Now delete these traces.
	 */

	tracePtr = cmdPtr->tracePtr;
	while (tracePtr != NULL) {
	    CommandTrace *nextPtr = tracePtr->nextPtr;

	    if (tracePtr->refCount-- <= 1) {
		Tcl_Free(tracePtr);
	    }
	    tracePtr = nextPtr;
	}
	cmdPtr->tracePtr = NULL;
    }

    /*
     * The list of commands exported from the namespace might have changed.
     * However, we do not need to recompute this just yet; next time we need
     * the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(cmdPtr->nsPtr);
    TclNsDecrRefCount(cmdPtr->nsPtr);

    /*
     * If the command being deleted has a compile function, increment the
     * interpreter's compileEpoch to invalidate its compiled code. This makes
     * sure that we don't later try to execute old code compiled with
     * command-specific (i.e., inline) bytecodes for the now-deleted command.
     * This field is checked in Tcl_EvalObj and ObjInterpProc, and code whose
     * compilation epoch doesn't match is recompiled.
     */

    if (cmdPtr->compileProc != NULL) {
	iPtr->compileEpoch++;
    }

    if (!(cmdPtr->flags & CMD_REDEF_IN_PROGRESS)) {
	/*
	 * Delete any imports of this routine before deleting this routine itself.
	 * See issue 688fcc7082fa.
	 */
	for (refPtr = cmdPtr->importRefPtr; refPtr != NULL;
		refPtr = nextRefPtr) {
	    nextRefPtr = refPtr->nextPtr;
	    importCmd = (Tcl_Command) refPtr->importedCmdPtr;
	    Tcl_DeleteCommandFromToken(interp, importCmd);
	}
    }

    if (cmdPtr->deleteProc != NULL) {
	/*
	 * Delete the command's client data. If this was an imported command
	 * created when a command was imported into a namespace, this client
	 * data will be a pointer to a ImportedCmdData structure describing
	 * the "real" command that this imported command refers to.
	 *
	 * If you are getting a crash during the call to deleteProc and
	 * cmdPtr->deleteProc is a pointer to the function free(), the most
	 * likely cause is that your extension allocated memory for the
	 * clientData argument to Tcl_CreateObjCommand with the Tcl_Alloc()
	 * macro and you are now trying to deallocate this memory with free()
	 * instead of Tcl_Free(). You should pass a pointer to your own method
	 * that calls Tcl_Free().
	 */

	cmdPtr->deleteProc(cmdPtr->deleteData);
    }

    /*
     * Don't use hPtr to delete the hash entry here, because it's possible
     * that the deletion callback renamed the command. Instead, use
     * cmdPtr->hptr, and make sure that no-one else has already deleted the
     * hash entry.
     */

    if (cmdPtr->hPtr != NULL) {
	Tcl_DeleteHashEntry(cmdPtr->hPtr);
	cmdPtr->hPtr = NULL;

	/*
	 * Bump the command epoch counter. This will invalidate all cached
	 * references that point to this command.
	 */

	cmdPtr->cmdEpoch++;
    }

    /*
     * A number of tests for particular kinds of commands are done by checking
     * whether the objProc field holds a known value. Set the field to NULL so
     * that such tests won't have false positives when applied to deleted
     * commands.
     */

    cmdPtr->objProc2 = NULL;

    /*
     * Now free the Command structure, unless there is another reference to it
     * from a CmdName Tcl object in some ByteCode code sequence. In that case,
     * delay the cleanup until all references are either discarded (when a
     * ByteCode is freed) or replaced by a new reference (when a cached
     * CmdName Command reference is found to be invalid and
     * TclNRExecuteByteCode looks up the command in the command hashtable).
     */

    cmdPtr->flags |= CMD_DEAD;
    TclCleanupCommandMacro(cmdPtr);
    return 0;
}

```

jim 0.84-9-g5bac7c9, revision `Jim commit5bac7c9 and captured whole source hash`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimIncrCmdRefCount`, lines 3881–3884. Line unit: physical LF only. Original byte range [115019, 115092). Retained reference `active671-whole-source-jim`.

```text
static void JimIncrCmdRefCount(Jim_Cmd *cmdPtr)
{
    cmdPtr->inUse++;
}

```

jim 0.84-9-g5bac7c9, revision `Jim commit5bac7c9 and captured whole source hash`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimDecrCmdRefCount`, lines 3887–3922. Line unit: physical LF only. Original byte range [115176, 116564). Retained reference `active671-whole-source-jim`.

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
         * These will be discarding at the next procEpoch increment
         * or once 1000 have been accumulated (unless quitting
         * since we can only be unwinding in that case).
         */
        cmdPtr->prevCmd = interp->oldCmdCache;
        interp->oldCmdCache = cmdPtr;
        if (!interp->quitting && ++interp->oldCmdCacheSize >= 1000) {
            Jim_InterpIncrProcEpoch(interp);
        }
    }
}

```

jim 0.84-9-g5bac7c9, revision `Jim commit5bac7c9 and captured whole source hash`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_DeleteCommand`, lines 4447–4460. Line unit: physical LF only. Original byte range [133824, 134210). Retained reference `active671-whole-source-jim`.

```text
int Jim_DeleteCommand(Jim_Interp *interp, Jim_Obj *nameObj)
{
    int ret = JIM_OK;

    nameObj = JimQualifyName(interp, nameObj);

    if (Jim_DeleteHashEntry(&interp->commands, nameObj) == JIM_ERR) {
        Jim_SetResultFormatted(interp, "can't delete \"%#s\": command doesn't exist", nameObj);
        ret = JIM_ERR;
    }
    Jim_DecrRefCount(interp, nameObj);

    return ret;
}

```

jim 0.84-9-g5bac7c9, revision `Jim commit5bac7c9 and captured whole source hash`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimInvokeCommand`, lines 11101–11215. Line unit: physical LF only. Original byte range [350892, 354599). Retained reference `active671-whole-source-jim`.

```text
static int JimInvokeCommand(Jim_Interp *interp, int objc, Jim_Obj *const *objv)
{
    int retcode;
    Jim_Cmd *cmdPtr;
    void *prevPrivData;
    Jim_Obj *tailcallObj = NULL;

#if 0
    printf("invoke");
    int j;
    for (j = 0; j < objc; j++) {
        printf(" '%s'", Jim_String(objv[j]));
    }
    printf("\n");
#endif

    cmdPtr = Jim_GetCommand(interp, objv[0], JIM_ERRMSG);
    if (cmdPtr == NULL) {
        return JimUnknown(interp, objc, objv);
    }
    JimIncrCmdRefCount(cmdPtr);

    if (interp->evalDepth == interp->maxEvalDepth) {
        Jim_SetResultString(interp, "Infinite eval recursion", -1);
        retcode = JIM_ERR;
        goto out;
    }
    interp->evalDepth++;
    prevPrivData = interp->cmdPrivData;

tailcall:

    interp->evalFrame->argc = objc;
    interp->evalFrame->argv = objv;
    interp->evalFrame->cmd = cmdPtr;

    if (!interp->traceCmdObj ||
        (retcode = JimTraceCallback(interp, "cmd", objc, objv)) == JIM_OK) {
        /* Call it -- Make sure result is an empty object. */
        Jim_SetEmptyResult(interp);
        interp->taint = Jim_CalcTaint(objc, objv);
        if (cmdPtr->flags & JIM_CMD_ISPROC) {
            retcode = JimCallProcedure(interp, cmdPtr, objc, objv);
        }
        else if ((cmdPtr->flags & JIM_CMD_NOTAINT) && Jim_CheckTaint(interp, JIM_TAINT_ANY)) {
            Jim_SetTaintError(interp, 1, objv);
            retcode = JIM_ERR;
        }
        else {
            retcode = JimCallNative(interp, cmdPtr, objc, objv);
            /* XXX Could it return -2 for tainted? */
        }
        if (retcode == JIM_ERR) {
            JimSetErrorStack(interp, NULL);
        }
    }

    if (tailcallObj) {
        /* clean up previous tailcall if we were invoking one */
        Jim_DecrRefCount(interp, tailcallObj);
        tailcallObj = NULL;
    }

    /* These are now invalid */
    interp->evalFrame->argc = 0;
    interp->evalFrame->argv = NULL;

    /* If a tailcall is returned for this frame, loop to invoke the new command */
    if (retcode == JIM_EVAL && interp->framePtr->tailcallObj) {
        JimDecrCmdRefCount(interp, cmdPtr);

        /* Replace the current command with the new tailcall command */
        cmdPtr = interp->framePtr->tailcallCmd;
        interp->framePtr->tailcallCmd = NULL;
        tailcallObj = interp->framePtr->tailcallObj;
        interp->framePtr->tailcallObj = NULL;
        /* We can access the internal rep here because the object can only
         * be constructed by the tailcall command
         */
        objc = tailcallObj->internalRep.listValue.len;
        objv = tailcallObj->internalRep.listValue.ele;
        goto tailcall;
    }

    interp->cmdPrivData = prevPrivData;
    interp->evalDepth--;

out:
    JimDecrCmdRefCount(interp, cmdPtr);

    if (retcode == JIM_ERR) {
        JimSetErrorStack(interp, NULL);
    }

    if (interp->framePtr->tailcallObj) {
        /* We might have skipped invoking a tailcall, perhaps because of an error
         * in defer handling so cleanup now
         */
        JimDecrCmdRefCount(interp, interp->framePtr->tailcallCmd);
        Jim_DecrRefCount(interp, interp->framePtr->tailcallObj);
        interp->framePtr->tailcallCmd = NULL;
        interp->framePtr->tailcallObj = NULL;
    }

    return retcode;
}

/* Eval the object vector 'objv' composed of 'objc' elements.
 * Every element is used as single argument.
 * Jim_EvalObj() will call this function every time its object
 * argument is of "list" type, with no string representation.
 *
 * This is possible because the string representation of a
 * list object generated by the UpdateStringOfList is made
 * in a way that ensures that every list element is a different

```

## Consumer bindings

- [runtime/rust/src/interp/native_command_retirement.rs](../../../../runtime/rust/src/interp/native_command_retirement.rs), `Interp::retire_original_command_generation`: Capture the actual command generation, original operation and physical frame/namespace before callbacks; use the canonical release-order driver for mandatory implementation/import retirement and retain the first full cause through matching unlink.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `ObjCommand::retire_original_delete_procedure`: Withdraw the actual once-only delete procedure before callback entry independently of the existing dispatch Rc. Runtime-owned HostEntry availability is withdrawn before its private backing destructor; Jim final-record retirement remains separate.
- [runtime/rust/src/interp/native_command_retirement/tests.rs](../../../../runtime/rust/src/interp/native_command_retirement/tests.rs), `interp::native_command_retirement::tests::original_active_object_deletion_separates_callback_from_dispatch_role_and_recreation` (linked): Actual registered software ObjCommand callbacks distinguish selected C immediate delete-procedure withdrawal from Jim final-dispatch-record retirement, check once-only deletion and preserve a callback-created same-name replacement for a later fresh-script invocation. Allocation-only Weak observations and software dispatch cleanup do not establish Native held-word cache, client-data freeing, reference counts or general active-command lifetime. The retained Native probe separately holds one caller word through two actual vector evaluations and keeps independent stack client records alive; this software control uses a fresh script for its later invocation and does not reproduce the held-word or freed-address experiment. This binds the current source definition only; no passed assertion or new Native observation is attached.

## Evidence

- [rust/tcl-registry/tests/data/native_active_object_command_deletion/671](../../../../rust/tcl-registry/tests/data/native_active_object_command_deletion/671)
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs)
- [runtime/rust/src/interp/native_command_retirement.rs](../../../../runtime/rust/src/interp/native_command_retirement.rs)
- [runtime/rust/src/interp/native_command_retirement/tests.rs](../../../../runtime/rust/src/interp/native_command_retirement/tests.rs)
