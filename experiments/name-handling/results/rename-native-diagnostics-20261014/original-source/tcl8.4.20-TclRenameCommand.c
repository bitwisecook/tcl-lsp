int
TclRenameCommand(interp, oldName, newName)
    Tcl_Interp *interp;                 /* Current interpreter. */
    char *oldName;                      /* Existing command name. */
    char *newName;                      /* New command name. */
{
    Interp *iPtr = (Interp *) interp;
    CONST char *newTail;
    Namespace *cmdNsPtr, *newNsPtr, *dummy1, *dummy2;
    Tcl_Command cmd;
    Command *cmdPtr;
    Tcl_HashEntry *hPtr, *oldHPtr;
    int new, result;
    Tcl_Obj* oldFullName;
    Tcl_DString newFullName;

    /*
     * Find the existing command. An error is returned if cmdName can't
     * be found.
     */

    cmd = Tcl_FindCommand(interp, oldName, (Tcl_Namespace *) NULL,
	/*flags*/ 0);
    cmdPtr = (Command *) cmd;
    if (cmdPtr == NULL) {
	Tcl_AppendStringsToObj(Tcl_GetObjResult(interp), "can't ",
                ((newName == NULL)||(*newName == '\0'))? "delete":"rename",
                " \"", oldName, "\": command doesn't exist", (char *) NULL);
	return TCL_ERROR;
    }
    cmdNsPtr = cmdPtr->nsPtr;
    oldFullName = Tcl_NewObj();
    Tcl_IncrRefCount( oldFullName );
    Tcl_GetCommandFullName( interp, cmd, oldFullName );

    /*
     * If the new command name is NULL or empty, delete the command. Do this
     * with Tcl_DeleteCommandFromToken, since we already have the command.
     */
    
    if ((newName == NULL) || (*newName == '\0')) {
	Tcl_DeleteCommandFromToken(interp, cmd);
	result = TCL_OK;
	goto done;
    }

    /*
     * Make sure that the destination command does not already exist.
     * The rename operation is like creating a command, so we should
     * automatically create the containing namespaces just like
     * Tcl_CreateCommand would.
     */

    TclGetNamespaceForQualName(interp, newName, (Namespace *) NULL,
       CREATE_NS_IF_UNKNOWN, &newNsPtr, &dummy1, &dummy2, &newTail);

    if ((newNsPtr == NULL) || (newTail == NULL)) {
	Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		 "can't rename to \"", newName, "\": bad command name",
    	    	 (char *) NULL);
	result = TCL_ERROR;
	goto done;
    }
    if (Tcl_FindHashEntry(&newNsPtr->cmdTable, newTail) != NULL) {
	Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		 "can't rename to \"", newName,
		 "\": command already exists", (char *) NULL);
	result = TCL_ERROR;
	goto done;
    }

    /*
     * Warning: any changes done in the code here are likely
     * to be needed in Tcl_HideCommand() code too.
     * (until the common parts are extracted out)     --dl
     */

    /*
     * Put the command in the new namespace so we can check for an alias
     * loop. Since we are adding a new command to a namespace, we must
     * handle any shadowing of the global commands that this might create.
     */
    
    oldHPtr = cmdPtr->hPtr;
    hPtr = Tcl_CreateHashEntry(&newNsPtr->cmdTable, newTail, &new);
    Tcl_SetHashValue(hPtr, (ClientData) cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = newNsPtr;
    TclResetShadowedCmdRefs(interp, cmdPtr);

    /*
     * Now check for an alias loop. If we detect one, put everything back
     * the way it was and report the error.
     */

    result = TclPreventAliasLoop(interp, interp, (Tcl_Command) cmdPtr);
    if (result != TCL_OK) {
        Tcl_DeleteHashEntry(cmdPtr->hPtr);
        cmdPtr->hPtr = oldHPtr;
        cmdPtr->nsPtr = cmdNsPtr;
	goto done;
    }

    /*
     * Script for rename traces can delete the command "oldName".
     * Therefore increment the reference count for cmdPtr so that
     * it's Command structure is freed only towards the end of this
     * function by calling TclCleanupCommand.
     *
     * The trace procedure needs to get a fully qualified name for
     * old and new commands [Tcl bug #651271], or else there's no way
     * for the trace procedure to get the namespace from which the old
     * command is being renamed!
     */

    Tcl_DStringInit( &newFullName );
    Tcl_DStringAppend( &newFullName, newNsPtr->fullName, -1 );
    if ( newNsPtr != iPtr->globalNsPtr ) {
	Tcl_DStringAppend( &newFullName, "::", 2 );
    }
    Tcl_DStringAppend( &newFullName, newTail, -1 );
    cmdPtr->refCount++;
    CallCommandTraces( iPtr, cmdPtr,
		       Tcl_GetString( oldFullName ),
		       Tcl_DStringValue( &newFullName ),
		       TCL_TRACE_RENAME);
    Tcl_DStringFree( &newFullName );

    /*
     * The new command name is okay, so remove the command from its
     * current namespace. This is like deleting the command, so bump
     * the cmdEpoch to invalidate any cached references to the command.
     */
    
    Tcl_DeleteHashEntry(oldHPtr);
    cmdPtr->cmdEpoch++;

    /*
     * If the command being renamed has a compile procedure, increment the
     * interpreter's compileEpoch to invalidate its compiled code. This
     * makes sure that we don't later try to execute old code compiled for
     * the now-renamed command.
     */

    if (cmdPtr->compileProc != NULL) {
	iPtr->compileEpoch++;
    }

    /*
     * Now free the Command structure, if the "oldName" command has
     * been deleted by invocation of rename traces.
     */
    TclCleanupCommand(cmdPtr);
    result = TCL_OK;

    done:
    TclDecrRefCount( oldFullName );
    return result;
}
