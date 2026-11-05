# naming.coroutine.root-frame-and-caller-lookup

Kind: `source-anchor`

## Problem statement

Coroutine command lookup uses the caller namespace, while a newly created coroutine starts on the interpreter root frame. Stamping its initial driver with the caller namespace makes actual namespace-currency checks reject a valid non-root creation.

## Question

Do the pinned C8.6/C9 coroutine creators use a root base frame separately from the caller namespace used for the original first command lookup?

## Conclusion

Each inspected TclNRCoroutineObjCmd saves lookupNsPtr from the caller varFrame namespace, sets running.framePtr and running.varFramePtr to rootFramePtr, clears cmdFramePtr, restores that root running context, and supplies the saved caller lookupNsPtr before evaluating the unchanged original argv List. Base execution context and first-command lookup context are distinct. No body, preparation, namespace existence, arbitrary frame or Normal permission follows.

## Scope

Source inspection of exact Tcl8.6.18/9.0.4/9.1.0 generic/tclBasic.c functions associated by full-file SHA with the immutable publication-v2 captures. C8.4/C8.5/Jim/F5 have no source conclusion in this question. These windows are not a new native execution or a physical coroutine/frame-pointer observation.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `not-tested`.

No observation for this question.

### tcl8.6, tcl9.0, tcl9.1

Status: `inspected`. Channel: Inspected TclNRCoroutineObjCmd function window, not a native invocation. Version: tcl8.6 8.6.18 pinned source; tcl9.0 9.0.4 pinned source; tcl9.1 9.1.0 pinned source.

Each inspected TclNRCoroutineObjCmd saves lookupNsPtr from the caller varFrame namespace, sets running.framePtr and running.varFramePtr to rootFramePtr, clears cmdFramePtr, restores that root running context, and supplies the saved caller lookupNsPtr before evaluating the unchanged original argv List. Base execution context and first-command lookup context are distinct. No body, preparation, namespace existence, arbitrary frame or Normal permission follows.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.6 8.6.18, revision `8.6.18`, `generic/tclBasic.c`, function `TclNRCoroutineObjCmd`, lines 9150–9277. Retained reference `root-frame-window-0`.

```text
int
TclNRCoroutineObjCmd(
    ClientData dummy,		/* Not used. */
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

    corPtr = (CoroutineData *)ckalloc(sizeof(CoroutineData));

    cmdPtr = (Command *) TclNRCreateCommandInNs(interp, simpleName,
	    (Tcl_Namespace *)nsPtr, /*objProc*/ NULL, TclNRInterpCoroutine,
	    corPtr, DeleteCoroutine);

    corPtr->cmdPtr = cmdPtr;
    cmdPtr->refCount++;

    /*
     * #280.
     * Provide the new coroutine with its own copy of the lineLABCPtr
     * hashtable for literal command arguments in bytecode. Note that that
     * CFWordBC chains are not duplicated, only the entrypoints to them. This
     * means that in the presence of coroutines each chain is potentially a
     * tree. Like the chain -> tree conversion of the CmdFrame stack.
     */

    {
	Tcl_HashSearch hSearch;
	Tcl_HashEntry *hePtr;

	corPtr->lineLABCPtr = (Tcl_HashTable *)ckalloc(sizeof(Tcl_HashTable));
	Tcl_InitHashTable(corPtr->lineLABCPtr, TCL_ONE_WORD_KEYS);

	for (hePtr = Tcl_FirstHashEntry(iPtr->lineLABCPtr,&hSearch);
		hePtr; hePtr = Tcl_NextHashEntry(&hSearch)) {
	    int isNew;
	    Tcl_HashEntry *newPtr =
		    Tcl_CreateHashEntry(corPtr->lineLABCPtr,
		    Tcl_GetHashKey(iPtr->lineLABCPtr, hePtr),
		    &isNew);

	    Tcl_SetHashValue(newPtr, Tcl_GetHashValue(hePtr));
	}
    }

    /*
     * Create the base context.
     */

    corPtr->running.framePtr = iPtr->rootFramePtr;
    corPtr->running.varFramePtr = iPtr->rootFramePtr;
    corPtr->running.cmdFramePtr = NULL;
    corPtr->running.lineLABCPtr = corPtr->lineLABCPtr;
    corPtr->stackLevel = NULL;
    corPtr->auxNumLevels = 0;

    /*
     * Create the coro's execEnv, switch to it to push the exit and coro
     * command callbacks, then switch back.
     */

    corPtr->eePtr = TclCreateExecEnv(interp, CORO_STACK_INITIAL_SIZE);
    corPtr->callerEEPtr = iPtr->execEnvPtr;
    corPtr->eePtr->corPtr = corPtr;

    SAVE_CONTEXT(corPtr->caller);
    corPtr->callerEEPtr = iPtr->execEnvPtr;
    RESTORE_CONTEXT(corPtr->running);
    iPtr->execEnvPtr = corPtr->eePtr;

    TclNRAddCallback(interp, NRCoroutineExitCallback, corPtr,
	    NULL, NULL, NULL);

    /*
     * Ensure that the command is looked up in the correct namespace.
     */

    iPtr->lookupNsPtr = lookupNsPtr;
    Tcl_NREvalObj(interp, Tcl_NewListObj(objc - 2, objv + 2), 0);
    iPtr->numLevels--;

    SAVE_CONTEXT(corPtr->running);
    RESTORE_CONTEXT(corPtr->caller);
    iPtr->execEnvPtr = corPtr->callerEEPtr;

    /*
     * Now just resume the coroutine.
     */

    TclNRAddCallback(interp, TclNRCoroutineActivateCallback, corPtr,
	    NULL, NULL, NULL);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `generic/tclBasic.c`, function `TclNRCoroutineObjCmd`, lines 9737–9865. Retained reference `root-frame-window-1`.

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
    cmdPtr->refCount++;

    /*
     * #280.
     * Provide the new coroutine with its own copy of the lineLABCPtr
     * hashtable for literal command arguments in bytecode. Note that
     * CFWordBC chains are not duplicated, only the entrypoints to them. This
     * means that in the presence of coroutines each chain is potentially a
     * tree. Like the chain -> tree conversion of the CmdFrame stack.
     */

    {
	Tcl_HashSearch hSearch;
	Tcl_HashEntry *hePtr;

	corPtr->lineLABCPtr = (Tcl_HashTable *)Tcl_Alloc(sizeof(Tcl_HashTable));
	Tcl_InitHashTable(corPtr->lineLABCPtr, TCL_ONE_WORD_KEYS);

	for (hePtr = Tcl_FirstHashEntry(iPtr->lineLABCPtr,&hSearch);
		hePtr; hePtr = Tcl_NextHashEntry(&hSearch)) {
	    int isNew;
	    Tcl_HashEntry *newPtr =
		    Tcl_CreateHashEntry(corPtr->lineLABCPtr,
		    Tcl_GetHashKey(iPtr->lineLABCPtr, hePtr),
		    &isNew);

	    Tcl_SetHashValue(newPtr, Tcl_GetHashValue(hePtr));
	}
    }

    /*
     * Create the base context.
     */

    corPtr->running.framePtr = iPtr->rootFramePtr;
    corPtr->running.varFramePtr = iPtr->rootFramePtr;
    corPtr->running.cmdFramePtr = NULL;
    corPtr->running.lineLABCPtr = corPtr->lineLABCPtr;
    corPtr->stackLevel = NULL;
    corPtr->auxNumLevels = 0;
    corPtr->yieldPtr = NULL;

    /*
     * Create the coro's execEnv, switch to it to push the exit and coro
     * command callbacks, then switch back.
     */

    corPtr->eePtr = TclCreateExecEnv(interp, CORO_STACK_INITIAL_SIZE);
    corPtr->callerEEPtr = iPtr->execEnvPtr;
    corPtr->eePtr->corPtr = corPtr;

    SAVE_CONTEXT(corPtr->caller);
    corPtr->callerEEPtr = iPtr->execEnvPtr;
    RESTORE_CONTEXT(corPtr->running);
    iPtr->execEnvPtr = corPtr->eePtr;

    TclNRAddCallback(interp, NRCoroutineExitCallback, corPtr,
	    NULL, NULL, NULL);

    /*
     * Ensure that the command is looked up in the correct namespace.
     */

    iPtr->lookupNsPtr = lookupNsPtr;
    Tcl_NREvalObj(interp, Tcl_NewListObj(objc - 2, objv + 2), 0);
    iPtr->numLevels--;

    SAVE_CONTEXT(corPtr->running);
    RESTORE_CONTEXT(corPtr->caller);
    iPtr->execEnvPtr = corPtr->callerEEPtr;

    /*
     * Now just resume the coroutine.
     */

    TclNRAddCallback(interp, TclNRCoroutineActivateCallback, corPtr,
	    NULL, NULL, NULL);
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `generic/tclBasic.c`, function `TclNRCoroutineObjCmd`, lines 10047–10174. Retained reference `root-frame-window-2`.

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
    cmdPtr->refCount++;

    /*
     * #280.
     * Provide the new coroutine with its own copy of the lineLABCPtr
     * hashtable for literal command arguments in bytecode. Note that
     * CFWordBC chains are not duplicated, only the entrypoints to them. This
     * means that in the presence of coroutines each chain is potentially a
     * tree. Like the chain -> tree conversion of the CmdFrame stack.
     */

    {
	Tcl_HashSearch hSearch;
	Tcl_HashEntry *hePtr;

	corPtr->lineLABCPtr = (Tcl_HashTable *)Tcl_Alloc(sizeof(Tcl_HashTable));
	Tcl_InitHashTable(corPtr->lineLABCPtr, TCL_ONE_WORD_KEYS);

	for (hePtr = Tcl_FirstHashEntry(iPtr->lineLABCPtr,&hSearch);
		hePtr; hePtr = Tcl_NextHashEntry(&hSearch)) {
	    Tcl_HashEntry *newPtr =
		    Tcl_CreateHashEntry(corPtr->lineLABCPtr,
		    Tcl_GetHashKey(iPtr->lineLABCPtr, hePtr),
		    NULL);

	    Tcl_SetHashValue(newPtr, Tcl_GetHashValue(hePtr));
	}
    }

    /*
     * Create the base context.
     */

    corPtr->running.framePtr = iPtr->rootFramePtr;
    corPtr->running.varFramePtr = iPtr->rootFramePtr;
    corPtr->running.cmdFramePtr = NULL;
    corPtr->running.lineLABCPtr = corPtr->lineLABCPtr;
    corPtr->stackLevel = NULL;
    corPtr->auxNumLevels = 0;
    corPtr->yieldPtr = NULL;

    /*
     * Create the coro's execEnv, switch to it to push the exit and coro
     * command callbacks, then switch back.
     */

    corPtr->eePtr = TclCreateExecEnv(interp, CORO_STACK_INITIAL_SIZE);
    corPtr->callerEEPtr = iPtr->execEnvPtr;
    corPtr->eePtr->corPtr = corPtr;

    SAVE_CONTEXT(corPtr->caller);
    corPtr->callerEEPtr = iPtr->execEnvPtr;
    RESTORE_CONTEXT(corPtr->running);
    iPtr->execEnvPtr = corPtr->eePtr;

    TclNRAddCallback(interp, NRCoroutineExitCallback, corPtr,
	    NULL, NULL, NULL);

    /*
     * Ensure that the command is looked up in the correct namespace.
     */

    iPtr->lookupNsPtr = lookupNsPtr;
    Tcl_NREvalObj(interp, Tcl_NewListObj(objc - 2, objv + 2), 0);
    iPtr->numLevels--;

    SAVE_CONTEXT(corPtr->running);
    RESTORE_CONTEXT(corPtr->caller);
    iPtr->execEnvPtr = corPtr->callerEEPtr;

    /*
     * Now just resume the coroutine.
     */

    TclNRAddCallback(interp, TclNRCoroutineActivateCallback, corPtr,
	    NULL, NULL, NULL);
    return TCL_OK;
}

```

## Consumer bindings

- [rust/tcl-vm/src/cmd_coro.rs](../../../../rust/tcl-vm/src/cmd_coro.rs), `cmd_coroutine`: Stamp the initial original-invocation driver with the actual shared root frame; retain caller namespace separately in the unchanged invocation List.
