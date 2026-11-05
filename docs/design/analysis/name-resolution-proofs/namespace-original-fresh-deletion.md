# naming.namespace.original-fresh-namespace-deletion

Kind: `implementation-contract`

## Problem statement

The namespace command transfer retires its exact target before the variable transfer; a second lookup can lose that identity, while a publication row cannot prove cleanup or successful deletion.

## Question

Which independent same-operation conditions permit source namespace retirement and Normal completion without re-looking up retired names or donating native cleanup authority?

## Conclusion

A sealed post-command retirement handoff preserves actual target identities. A separate pre/post source envelope closes only fresh quiet namespaces with complete empty cell inventories and direct intrinsic source Proc cleanup. Missing or uncertain prerequisites remain terminal.

## Scope

The independent source completion receipt requires the current initial Registry handler, its complete original static operand vector, the same source/configuration/site and actual namespace targets. Every namespace in the removed subtree must have genuine source creation lineage. Current and caller variable tables must be closed and contain no definite or possible cells in that subtree; dynamic or uncertain observers, active namespace frames and pending retirements refuse the receipt. The actual command table may contain only direct source Proc tokens with exact current source allocations, intrinsic cleanup and no alias prefixes or runtime implementation. Missing, unknown, overlapping, imported, object or custom-cleanup targets remain outside this envelope. Native/host table and physical cleanup capability are excluded. Source anchors describe release algorithms without an execution claim. The selected recursive Delete transition additionally owns only the parent legacy InterpState-to-InterpreterPolicy write bridge. This source-qualified handoff preserves trace callbacks, reads, clobbers and all independently declared policy/host writes; a shape with no produced deletion transition retains the coarse bridge. Delete argument roles depend only on exact argv cardinality after the selected subcommand. Counted opaque values and fixed-count dynamic values retain NamespaceName roles; unknown expansion cardinality remains incomplete. Role selection cannot supply a name value, target identity, empty cells or successful completion.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No native execution receipt is attached to this source implementation contract.

## Source inspection

tcl8.4 8.4.20, revision `Retained release source identified by complete file digest`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `NamespaceDeleteCmd`, lines 2870–2916. Retained reference `delete-source-0`.

```text
NamespaceDeleteCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    Tcl_Namespace *namespacePtr;
    char *name;
    register int i;

    if (objc < 2) {
        Tcl_WrongNumArgs(interp, 2, objv, "?name name...?");
        return TCL_ERROR;
    }

    /*
     * Destroying one namespace may cause another to be destroyed. Break
     * this into two passes: first check to make sure that all namespaces on
     * the command line are valid, and report any errors.
     */

    for (i = 2;  i < objc;  i++) {
        name = Tcl_GetString(objv[i]);
	namespacePtr = Tcl_FindNamespace(interp, name,
		(Tcl_Namespace *) NULL, /*flags*/ 0);
	if (namespacePtr == NULL) {
	    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
                    "unknown namespace \"", Tcl_GetString(objv[i]),
		    "\" in namespace delete command", (char *) NULL);
            return TCL_ERROR;
        }
    }

    /*
     * Okay, now delete each namespace.
     */

    for (i = 2;  i < objc;  i++) {
        name = Tcl_GetString(objv[i]);
	namespacePtr = Tcl_FindNamespace(interp, name,
	    (Tcl_Namespace *) NULL, /* flags */ 0);
	if (namespacePtr) {
            Tcl_DeleteNamespace(namespacePtr);
        }
    }
    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `Retained release source identified by complete file digest`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `Tcl_DeleteNamespace`, lines 576–656. Retained reference `delete-source-1`.

```text
Tcl_DeleteNamespace(namespacePtr)
    Tcl_Namespace *namespacePtr;   /* Points to the namespace to delete. */
{
    register Namespace *nsPtr = (Namespace *) namespacePtr;
    Interp *iPtr = (Interp *) nsPtr->interp;
    Namespace *globalNsPtr =
	    (Namespace *) Tcl_GetGlobalNamespace((Tcl_Interp *) iPtr);
    Tcl_HashEntry *entryPtr;

    /*
     * If the namespace is on the call frame stack, it is marked as "dying"
     * (NS_DYING is OR'd into its flags): the namespace can't be looked up
     * by name but its commands and variables are still usable by those
     * active call frames. When all active call frames referring to the
     * namespace have been popped from the Tcl stack, Tcl_PopCallFrame will
     * call this procedure again to delete everything in the namespace.
     * If no nsName objects refer to the namespace (i.e., if its refCount 
     * is zero), its commands and variables are deleted and the storage for
     * its namespace structure is freed. Otherwise, if its refCount is
     * nonzero, the namespace's commands and variables are deleted but the
     * structure isn't freed. Instead, NS_DEAD is OR'd into the structure's
     * flags to allow the namespace resolution code to recognize that the
     * namespace is "deleted". The structure's storage is freed by
     * FreeNsNameInternalRep when its refCount reaches 0.
     */

    if (nsPtr->activationCount > 0) {
        nsPtr->flags |= NS_DYING;
        if (nsPtr->parentPtr != NULL) {
            entryPtr = Tcl_FindHashEntry(&nsPtr->parentPtr->childTable,
		    nsPtr->name);
            if (entryPtr != NULL) {
                Tcl_DeleteHashEntry(entryPtr);
            }
        }
        nsPtr->parentPtr = NULL;
    } else if (!(nsPtr->flags & NS_KILLED)) {
	/*
	 * Delete the namespace and everything in it. If this is the global
	 * namespace, then clear it but don't free its storage unless the
	 * interpreter is being torn down. Set the NS_KILLED flag to avoid
	 * recursive calls here - if the namespace is really in the process of
	 * being deleted, ignore any second call.
	 */

	nsPtr->flags |= (NS_DYING|NS_KILLED);
	
        TclTeardownNamespace(nsPtr);

        if ((nsPtr != globalNsPtr) || (iPtr->flags & DELETED)) {
            /*
	     * If this is the global namespace, then it may have residual
             * "errorInfo" and "errorCode" variables for errors that
             * occurred while it was being torn down.  Try to clear the
             * variable list one last time.
	     */

            TclDeleteNamespaceVars(nsPtr);
	    
            Tcl_DeleteHashTable(&nsPtr->childTable);
            Tcl_DeleteHashTable(&nsPtr->cmdTable);

            /*
             * If the reference count is 0, then discard the namespace.
             * Otherwise, mark it as "dead" so that it can't be used.
             */

            if (nsPtr->refCount == 0) {
                NamespaceFree(nsPtr);
            } else {
                nsPtr->flags |= NS_DEAD;
            }
        } else {
	    /*
	     * We didn't really kill it, so remove the KILLED marks, so
	     * it can get killed later, avoiding mem leaks
	     */
	     nsPtr->flags &= ~(NS_DYING|NS_KILLED);
	}
    }
}

```

tcl8.4 8.4.20, revision `Retained release source identified by complete file digest`, `tmp/tcl8.4.20/generic/tclProc.c`, function `TclProcDeleteProc`, lines 1605–1614. Retained reference `delete-source-2`.

```text
TclProcDeleteProc(clientData)
    ClientData clientData;		/* Procedure to be deleted. */
{
    Proc *procPtr = (Proc *) clientData;

    procPtr->refCount--;
    if (procPtr->refCount <= 0) {
	TclProcCleanupProc(procPtr);
    }
}

```

tcl8.5 8.5.19, revision `Retained release source identified by complete file digest`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceDeleteCmd`, lines 3177–3224. Retained reference `delete-source-3`.

```text
NamespaceDeleteCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Namespace *namespacePtr;
    char *name;
    register int i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 2, objv, "?name name...?");
	return TCL_ERROR;
    }

    /*
     * Destroying one namespace may cause another to be destroyed. Break this
     * into two passes: first check to make sure that all namespaces on the
     * command line are valid, and report any errors.
     */

    for (i = 2;  i < objc;  i++) {
	name = TclGetString(objv[i]);
	namespacePtr = Tcl_FindNamespace(interp, name, NULL, /*flags*/ 0);
	if ((namespacePtr == NULL)
		|| (((Namespace *)namespacePtr)->flags & NS_KILLED)) {
	    Tcl_AppendResult(interp, "unknown namespace \"",
		    TclGetString(objv[i]),
		    "\" in namespace delete command", NULL);
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE",
		    TclGetString(objv[i]), NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * Okay, now delete each namespace.
     */

    for (i = 2;  i < objc;  i++) {
	name = TclGetString(objv[i]);
	namespacePtr = Tcl_FindNamespace(interp, name, NULL, /* flags */ 0);
	if (namespacePtr) {
	    Tcl_DeleteNamespace(namespacePtr);
	}
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `Retained release source identified by complete file digest`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `Tcl_DeleteNamespace`, lines 945–1062. Retained reference `delete-source-4`.

```text
Tcl_DeleteNamespace(
    Tcl_Namespace *namespacePtr)/* Points to the namespace to delete. */
{
    register Namespace *nsPtr = (Namespace *) namespacePtr;
    Interp *iPtr = (Interp *) nsPtr->interp;
    Namespace *globalNsPtr = (Namespace *)
	    TclGetGlobalNamespace((Tcl_Interp *) iPtr);
    Tcl_HashEntry *entryPtr;

    /*
     * If the namespace has associated ensemble commands, delete them first.
     * This leaves the actual contents of the namespace alone (unless they are
     * linked ensemble commands, of course). Note that this code is actually
     * reentrant so command delete traces won't purturb things badly.
     */

    while (nsPtr->ensembles != NULL) {
	EnsembleConfig *ensemblePtr = (EnsembleConfig *) nsPtr->ensembles;

	/*
	 * Splice out and link to indicate that we've already been killed.
	 */

	nsPtr->ensembles = (Tcl_Ensemble *) ensemblePtr->next;
	ensemblePtr->next = ensemblePtr;
	Tcl_DeleteCommandFromToken(nsPtr->interp, ensemblePtr->token);
    }

    /*
     * If the namespace has a registered unknown handler (TIP 181), then free
     * it here.
     */

    if (nsPtr->unknownHandlerPtr != NULL) {
	Tcl_DecrRefCount(nsPtr->unknownHandlerPtr);
	nsPtr->unknownHandlerPtr = NULL;
    }

    /*
     * If the namespace is on the call frame stack, it is marked as "dying"
     * (NS_DYING is OR'd into its flags): the namespace can't be looked up by
     * name but its commands and variables are still usable by those active
     * call frames. When all active call frames referring to the namespace
     * have been popped from the Tcl stack, Tcl_PopCallFrame will call this
     * function again to delete everything in the namespace. If no nsName
     * objects refer to the namespace (i.e., if its refCount is zero), its
     * commands and variables are deleted and the storage for its namespace
     * structure is freed. Otherwise, if its refCount is nonzero, the
     * namespace's commands and variables are deleted but the structure isn't
     * freed. Instead, NS_DEAD is OR'd into the structure's flags to allow the
     * namespace resolution code to recognize that the namespace is "deleted".
     * The structure's storage is freed by FreeNsNameInternalRep when its
     * refCount reaches 0.
     */

    if (nsPtr->activationCount - (nsPtr == globalNsPtr) > 0) {
	nsPtr->flags |= NS_DYING;
	if (nsPtr->parentPtr != NULL) {
	    entryPtr = Tcl_FindHashEntry(&nsPtr->parentPtr->childTable,
		    nsPtr->name);
	    if (entryPtr != NULL) {
		Tcl_DeleteHashEntry(entryPtr);
	    }
	}
	nsPtr->parentPtr = NULL;
    } else if (!(nsPtr->flags & NS_KILLED)) {
	/*
	 * Delete the namespace and everything in it. If this is the global
	 * namespace, then clear it but don't free its storage unless the
	 * interpreter is being torn down. Set the NS_KILLED flag to avoid
	 * recursive calls here - if the namespace is really in the process of
	 * being deleted, ignore any second call.
	 */

	nsPtr->flags |= (NS_DYING|NS_KILLED);

	TclTeardownNamespace(nsPtr);

	if ((nsPtr != globalNsPtr) || (iPtr->flags & DELETED)) {
	    /*
	     * If this is the global namespace, then it may have residual
	     * "errorInfo" and "errorCode" variables for errors that occurred
	     * while it was being torn down. Try to clear the variable list
	     * one last time.
	     */

	    TclDeleteNamespaceVars(nsPtr);

	    Tcl_DeleteHashTable(&nsPtr->childTable);
	    Tcl_DeleteHashTable(&nsPtr->cmdTable);

	    /*
	     * If the reference count is 0, then discard the namespace.
	     * Otherwise, mark it as "dead" so that it can't be used.
	     */

	    if (nsPtr->refCount == 0) {
		NamespaceFree(nsPtr);
	    } else {
		nsPtr->flags |= NS_DEAD;
	    }
	} else {
	    /*
	     * Restore the ::errorInfo and ::errorCode traces.
	     */

	    EstablishErrorInfoTraces(NULL, nsPtr->interp, NULL, NULL, 0);
	    EstablishErrorCodeTraces(NULL, nsPtr->interp, NULL, NULL, 0);

	    /*
	     * We didn't really kill it, so remove the KILLED marks, so it can
	     * get killed later, avoiding mem leaks.
	     */

	    nsPtr->flags &= ~(NS_DYING|NS_KILLED);
	}
    }
}

```

tcl8.5 8.5.19, revision `Retained release source identified by complete file digest`, `tmp/tcl8.5.19/generic/tclProc.c`, function `TclProcDeleteProc`, lines 2130–2139. Retained reference `delete-source-5`.

```text
TclProcDeleteProc(
    ClientData clientData)	/* Procedure to be deleted. */
{
    Proc *procPtr = (Proc *) clientData;

    procPtr->refCount--;
    if (procPtr->refCount <= 0) {
	TclProcCleanupProc(procPtr);
    }
}

```

tcl8.6 8.6.18, revision `Retained release source identified by complete file digest`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `NamespaceDeleteCmd`, lines 3185–3232. Retained reference `delete-source-6`.

```text
NamespaceDeleteCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Namespace *namespacePtr;
    const char *name;
    int i;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?name name...?");
	return TCL_ERROR;
    }

    /*
     * Destroying one namespace may cause another to be destroyed. Break this
     * into two passes: first check to make sure that all namespaces on the
     * command line are valid, and report any errors.
     */

    for (i = 1;  i < objc;  i++) {
	name = TclGetString(objv[i]);
	namespacePtr = Tcl_FindNamespace(interp, name, NULL, /*flags*/ 0);
	if ((namespacePtr == NULL)
		|| (((Namespace *) namespacePtr)->flags & NS_KILLED)) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "unknown namespace \"%s\" in namespace delete command",
		    TclGetString(objv[i])));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE",
		    TclGetString(objv[i]), (char *)NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * Okay, now delete each namespace.
     */

    for (i = 1;  i < objc;  i++) {
	name = TclGetString(objv[i]);
	namespacePtr = Tcl_FindNamespace(interp, name, NULL, /* flags */ 0);
	if (namespacePtr) {
	    Tcl_DeleteNamespace(namespacePtr);
	}
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `Retained release source identified by complete file digest`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `Tcl_DeleteNamespace`, lines 882–1040. Retained reference `delete-source-7`.

```text
Tcl_DeleteNamespace(
    Tcl_Namespace *namespacePtr)/* Points to the namespace to delete. */
{
    Namespace *nsPtr = (Namespace *) namespacePtr;
    Tcl_Interp *interp = nsPtr->interp;
    Namespace *globalNsPtr = (Namespace *) TclGetGlobalNamespace(interp);
    Tcl_HashEntry *entryPtr;
    Tcl_HashSearch search;
    Command *cmdPtr;

    /*
     * Ensure that this namespace doesn't get deallocated in the meantime.
     */
    nsPtr->refCount++;

    /*
     * Give anyone interested - notably TclOO - a chance to use this namespace
     * normally despite the fact that the namespace is going to go. Allows the
     * calling of destructors. Will only be called once (unless re-established
     * by the called function). [Bug 2950259]
     *
     * Note that setting this field requires access to the internal definition
     * of namespaces, so it should only be accessed by code that knows about
     * being careful with reentrancy.
     */

    if (nsPtr->earlyDeleteProc != NULL) {
	Tcl_NamespaceDeleteProc *earlyDeleteProc = nsPtr->earlyDeleteProc;

	nsPtr->earlyDeleteProc = NULL;
	nsPtr->activationCount++;
	earlyDeleteProc(nsPtr->clientData);
	nsPtr->activationCount--;
    }

    /*
     * Delete all coroutine commands now: break the circular ref cycle between
     * the namespace and the coroutine command [Bug 2724403]. This code is
     * essentially duplicated in TclTeardownNamespace() for all other
     * commands. Don't optimize to Tcl_NextHashEntry() because of traces.
     *
     * NOTE: we could avoid traversing the ns's command list by keeping a
     * separate list of coros.
     */

    for (entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
	    entryPtr != NULL;) {
	cmdPtr = (Command *)Tcl_GetHashValue(entryPtr);
	if (cmdPtr->nreProc == TclNRInterpCoroutine) {
	    Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	    entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
	} else {
	    entryPtr = Tcl_NextHashEntry(&search);
	}
    }

    /*
     * If the namespace has associated ensemble commands, delete them first.
     * This leaves the actual contents of the namespace alone (unless they are
     * linked ensemble commands, of course). Note that this code is actually
     * reentrant so command delete traces won't purturb things badly.
     */

    while (nsPtr->ensembles != NULL) {
	EnsembleConfig *ensemblePtr = (EnsembleConfig *) nsPtr->ensembles;

	/*
	 * Splice out and link to indicate that we've already been killed.
	 */

	nsPtr->ensembles = (Tcl_Ensemble *) ensemblePtr->next;
	ensemblePtr->next = ensemblePtr;
	Tcl_DeleteCommandFromToken(interp, ensemblePtr->token);
    }

    /*
     * If the namespace has a registered unknown handler (TIP 181), then free
     * it here.
     */

    if (nsPtr->unknownHandlerPtr != NULL) {
	Tcl_DecrRefCount(nsPtr->unknownHandlerPtr);
	nsPtr->unknownHandlerPtr = NULL;
    }

    /*
     * If the namespace is on the call frame stack, it is marked as "dying"
     * (NS_DYING is OR'd into its flags):  Contents of the namespace are
     * still available and visible until the namespace is later marked as
     * NS_DEAD, and its commands and variables are still usable by any
     * active call frames referring to th namespace. When all active call
     * frames referring to the namespace have been popped from the Tcl
     * stack, Tcl_PopCallFrame calls Tcl_DeleteNamespace again. If no
     * nsName objects refer to the namespace (i.e., if its refCount is
     * zero), its commands and variables are deleted and the storage for
     * its namespace structure is freed.  Otherwise, if its refCount is
     * nonzero, the namespace's commands and variables are deleted but the
     * structure isn't freed. Instead, NS_DEAD is OR'd into the structure's
     * flags to allow the namespace resolution code to recognize that the
     * namespace is "deleted".  The structure's storage is freed by
     * FreeNsNameInternalRep when its refCount reaches 0.
     */

    if (nsPtr->activationCount > (nsPtr == globalNsPtr)) {
	nsPtr->flags |= NS_DYING;
	if (nsPtr->parentPtr != NULL) {
	    entryPtr = Tcl_FindHashEntry(
		    TclGetNamespaceChildTable((Tcl_Namespace *)
			    nsPtr->parentPtr), nsPtr->name);
	    if (entryPtr != NULL) {
		Tcl_DeleteHashEntry(entryPtr);
	    }
	}
	nsPtr->parentPtr = NULL;
    } else if (!(nsPtr->flags & NS_KILLED)) {
	/*
	 * Delete the namespace and everything in it. If this is the global
	 * namespace, then clear it but don't free its storage unless the
	 * interpreter is being torn down. Set the NS_KILLED flag to avoid
	 * recursive calls here - if the namespace is really in the process of
	 * being deleted, ignore any second call.
	 */

	nsPtr->flags |= (NS_DYING | NS_KILLED);

	TclTeardownNamespace(nsPtr);

	if ((nsPtr != globalNsPtr) || (((Interp *) interp)->flags & DELETED)) {
	    /*
	     * If this is the global namespace, then it may have residual
	     * "errorInfo" and "errorCode" variables for errors that occurred
	     * while it was being torn down. Try to clear the variable list
	     * one last time.
	     */

	    TclDeleteNamespaceVars(nsPtr);

	    Tcl_DeleteHashTable(&nsPtr->childTable);
	    Tcl_DeleteHashTable(&nsPtr->cmdTable);

	    nsPtr ->flags |= NS_DEAD;
	} else {
	    /*
	     * Restore the ::errorInfo and ::errorCode traces.
	     */

	    EstablishErrorInfoTraces(NULL, interp, NULL, NULL, 0);
	    EstablishErrorCodeTraces(NULL, interp, NULL, NULL, 0);

	    /*
	     * We didn't really kill it, so remove the KILLED marks, so it can
	     * get killed later, avoiding mem leaks.
	     */

	    nsPtr->flags &= ~(NS_DYING|NS_KILLED);
	}
    }
    TclNsDecrRefCount(nsPtr);
}

```

tcl8.6 8.6.18, revision `Retained release source identified by complete file digest`, `tmp/tcl8.6.18/generic/tclProc.c`, function `TclProcDeleteProc`, lines 2106–2114. Retained reference `delete-source-8`.

```text
TclProcDeleteProc(
    ClientData clientData)	/* Procedure to be deleted. */
{
    Proc *procPtr = (Proc *)clientData;

    if (procPtr->refCount-- <= 1) {
	TclProcCleanupProc(procPtr);
    }
}

```

tcl9.0 9.0.4, revision `Retained release source identified by complete file digest`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `NamespaceDeleteCmd`, lines 3391–3438. Retained reference `delete-source-9`.

```text
NamespaceDeleteCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Namespace *namespacePtr;
    const char *name;
    int i;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?name name...?");
	return TCL_ERROR;
    }

    /*
     * Destroying one namespace may cause another to be destroyed. Break this
     * into two passes: first check to make sure that all namespaces on the
     * command line are valid, and report any errors.
     */

    for (i = 1;  i < objc;  i++) {
	name = TclGetString(objv[i]);
	namespacePtr = Tcl_FindNamespace(interp, name, NULL, /*flags*/ 0);
	if ((namespacePtr == NULL)
		|| (((Namespace *) namespacePtr)->flags & NS_TEARDOWN)) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "unknown namespace \"%s\" in namespace delete command",
		    TclGetString(objv[i])));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE",
		    TclGetString(objv[i]), (char *)NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * Okay, now delete each namespace.
     */

    for (i = 1;  i < objc;  i++) {
	name = TclGetString(objv[i]);
	namespacePtr = Tcl_FindNamespace(interp, name, NULL, /* flags */ 0);
	if (namespacePtr) {
	    Tcl_DeleteNamespace(namespacePtr);
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Retained release source identified by complete file digest`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `Tcl_DeleteNamespace`, lines 1012–1175. Retained reference `delete-source-10`.

```text
Tcl_DeleteNamespace(
    Tcl_Namespace *namespacePtr)/* Points to the namespace to delete. */
{
    Namespace *nsPtr = (Namespace *) namespacePtr;
    Tcl_Interp *interp = nsPtr->interp;
    Namespace *globalNsPtr = (Namespace *) TclGetGlobalNamespace(interp);
    Tcl_HashEntry *entryPtr;
    Tcl_HashSearch search;
    Command *cmdPtr;

    /*
     * Ensure that this namespace doesn't get deallocated in the meantime.
     */
    nsPtr->refCount++;

    /*
     * Give anyone interested - notably TclOO - a chance to use this namespace
     * normally despite the fact that the namespace is going to go. Allows the
     * calling of destructors. Will only be called once (unless re-established
     * by the called function). [Bug 2950259]
     *
     * Note that setting this field requires access to the internal definition
     * of namespaces, so it should only be accessed by code that knows about
     * being careful with reentrancy.
     */

    if (nsPtr->earlyDeleteProc != NULL) {
	Tcl_NamespaceDeleteProc *earlyDeleteProc = nsPtr->earlyDeleteProc;

	nsPtr->earlyDeleteProc = NULL;
	nsPtr->activationCount++;
	earlyDeleteProc(nsPtr->clientData);
	nsPtr->activationCount--;
    }

    /*
     * Delete all coroutine commands now: break the circular ref cycle between
     * the namespace and the coroutine command [Bug 2724403]. This code is
     * essentially duplicated in TclTeardownNamespace() for all other
     * commands. Don't optimize to Tcl_NextHashEntry() because of traces.
     *
     * NOTE: we could avoid traversing the ns's command list by keeping a
     * separate list of coros.
     */

    for (entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
	    entryPtr != NULL;) {
	cmdPtr = (Command *) Tcl_GetHashValue(entryPtr);
	if (cmdPtr->nreProc == TclNRInterpCoroutine) {
	    Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	    entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
	} else {
	    entryPtr = Tcl_NextHashEntry(&search);
	}
    }

    /*
     * If the namespace has associated ensemble commands, delete them first.
     * This leaves the actual contents of the namespace alone (unless they are
     * linked ensemble commands, of course). Note that this code is actually
     * reentrant so command delete traces won't purturb things badly.
     */

    while (nsPtr->ensembles != NULL) {
	EnsembleConfig *ensemblePtr = (EnsembleConfig *) nsPtr->ensembles;

	/*
	 * Splice out and link to indicate that we've already been killed.
	 */

	nsPtr->ensembles = (Tcl_Ensemble *) ensemblePtr->next;
	ensemblePtr->next = ensemblePtr;
	Tcl_DeleteCommandFromToken(interp, ensemblePtr->token);
    }

    /*
     * If the namespace has a registered unknown handler (TIP 181), then free
     * it here.
     */

    if (nsPtr->unknownHandlerPtr != NULL) {
	Tcl_DecrRefCount(nsPtr->unknownHandlerPtr);
	nsPtr->unknownHandlerPtr = NULL;
    }

    /*
     * If the namespace is on the call frame stack, it is marked as "dying"
     * (NS_DYING is OR'd into its flags):  Contents of the namespace are
     * still available and visible until the namespace is later marked as
     * NS_DEAD, and its commands and variables are still usable by any
     * active call frames referring to th namespace. When all active call
     * frames referring to the namespace have been popped from the Tcl
     * stack, Tcl_PopCallFrame calls Tcl_DeleteNamespace again. If no
     * nsName objects refer to the namespace (i.e., if its refCount is
     * zero), its commands and variables are deleted and the storage for
     * its namespace structure is freed.  Otherwise, if its refCount is
     * nonzero, the namespace's commands and variables are deleted but the
     * structure isn't freed. Instead, NS_DEAD is OR'd into the structure's
     * flags to allow the namespace resolution code to recognize that the
     * namespace is "deleted".  The structure's storage is freed by
     * FreeNsNameInternalRep when its refCount reaches 0.
     */

    if (nsPtr->activationCount > (nsPtr == globalNsPtr)) {
	nsPtr->flags |= NS_DYING;
	if (nsPtr->parentPtr != NULL) {
	    entryPtr = FindChildEntry(nsPtr->parentPtr, nsPtr->name);
	    if (entryPtr != NULL) {
		Tcl_DeleteHashEntry(entryPtr);
	    }
	}
	nsPtr->parentPtr = NULL;
    } else if (!(nsPtr->flags & NS_TEARDOWN)) {
	/*
	 * Delete the namespace and everything in it. If this is the global
	 * namespace, then clear it but don't free its storage unless the
	 * interpreter is being torn down. Set the NS_TEARDOWN flag to avoid
	 * recursive calls here - if the namespace is really in the process of
	 * being deleted, ignore any second call.
	 */

	nsPtr->flags |= NS_DYING | NS_TEARDOWN;

	TclTeardownNamespace(nsPtr);

	if ((nsPtr != globalNsPtr) || (((Interp *) interp)->flags & DELETED)) {
	    /*
	     * If this is the global namespace, then it may have residual
	     * "errorInfo" and "errorCode" variables for errors that occurred
	     * while it was being torn down. Try to clear the variable list
	     * one last time.
	     */

	    TclDeleteNamespaceVars(nsPtr);

#ifndef BREAK_NAMESPACE_COMPAT
	    Tcl_DeleteHashTable(&nsPtr->childTable);
#else
	    if (nsPtr->childTablePtr != NULL) {
		Tcl_DeleteHashTable(nsPtr->childTablePtr);
		Tcl_Free(nsPtr->childTablePtr);
	    }
#endif
	    Tcl_DeleteHashTable(&nsPtr->cmdTable);

	    nsPtr ->flags |= NS_DEAD;
	} else {
	    /*
	     * Restore the ::errorInfo and ::errorCode traces.
	     */

	    EstablishErrorInfoTraces(NULL, interp, NULL, NULL, 0);
	    EstablishErrorCodeTraces(NULL, interp, NULL, NULL, 0);

	    /*
	     * We didn't really kill it, so remove the KILLED marks, so it can
	     * get killed later, avoiding mem leaks.
	     */

	    nsPtr->flags &= ~(NS_DYING|NS_TEARDOWN);
	}
    }
    TclNsDecrRefCount(nsPtr);
}

```

tcl9.0 9.0.4, revision `Retained release source identified by complete file digest`, `tmp/tcl9.0.4/generic/tclProc.c`, function `TclProcDeleteProc`, lines 2124–2132. Retained reference `delete-source-11`.

```text
TclProcDeleteProc(
    void *clientData)		/* Procedure to be deleted. */
{
    Proc *procPtr = (Proc *)clientData;

    if (procPtr->refCount-- <= 1) {
	TclProcCleanupProc(procPtr);
    }
}

```

tcl9.1 9.1.0, revision `Retained release source identified by complete file digest`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `NamespaceDeleteCmd`, lines 3363–3410. Retained reference `delete-source-12`.

```text
NamespaceDeleteCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Namespace *namespacePtr;
    const char *name;
    Tcl_Size i;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?name name...?");
	return TCL_ERROR;
    }

    /*
     * Destroying one namespace may cause another to be destroyed. Break this
     * into two passes: first check to make sure that all namespaces on the
     * command line are valid, and report any errors.
     */

    for (i = 1;  i < objc;  i++) {
	name = TclGetString(objv[i]);
	namespacePtr = Tcl_FindNamespace(interp, name, NULL, /*flags*/ 0);
	if ((namespacePtr == NULL)
		|| (((Namespace *) namespacePtr)->flags & NS_TEARDOWN)) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "unknown namespace \"%s\" in namespace delete command",
		    TclGetString(objv[i])));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE",
		    TclGetString(objv[i]), (char *)NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * Okay, now delete each namespace.
     */

    for (i = 1;  i < objc;  i++) {
	name = TclGetString(objv[i]);
	namespacePtr = Tcl_FindNamespace(interp, name, NULL, /* flags */ 0);
	if (namespacePtr) {
	    Tcl_DeleteNamespace(namespacePtr);
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `Retained release source identified by complete file digest`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `Tcl_DeleteNamespace`, lines 1011–1174. Retained reference `delete-source-13`.

```text
Tcl_DeleteNamespace(
    Tcl_Namespace *namespacePtr)/* Points to the namespace to delete. */
{
    Namespace *nsPtr = (Namespace *) namespacePtr;
    Tcl_Interp *interp = nsPtr->interp;
    Namespace *globalNsPtr = (Namespace *) TclGetGlobalNamespace(interp);
    Tcl_HashEntry *entryPtr;
    Tcl_HashSearch search;
    Command *cmdPtr;

    /*
     * Ensure that this namespace doesn't get deallocated in the meantime.
     */
    nsPtr->refCount++;

    /*
     * Give anyone interested - notably TclOO - a chance to use this namespace
     * normally despite the fact that the namespace is going to go. Allows the
     * calling of destructors. Will only be called once (unless re-established
     * by the called function). [Bug 2950259]
     *
     * Note that setting this field requires access to the internal definition
     * of namespaces, so it should only be accessed by code that knows about
     * being careful with reentrancy.
     */

    if (nsPtr->earlyDeleteProc != NULL) {
	Tcl_NamespaceDeleteProc *earlyDeleteProc = nsPtr->earlyDeleteProc;

	nsPtr->earlyDeleteProc = NULL;
	nsPtr->activationCount++;
	earlyDeleteProc(nsPtr->clientData);
	nsPtr->activationCount--;
    }

    /*
     * Delete all coroutine commands now: break the circular ref cycle between
     * the namespace and the coroutine command [Bug 2724403]. This code is
     * essentially duplicated in TclTeardownNamespace() for all other
     * commands. Don't optimize to Tcl_NextHashEntry() because of traces.
     *
     * NOTE: we could avoid traversing the ns's command list by keeping a
     * separate list of coros.
     */

    for (entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
	    entryPtr != NULL;) {
	cmdPtr = (Command *) Tcl_GetHashValue(entryPtr);
	if (cmdPtr->nreProc2 == TclNRInterpCoroutine) {
	    Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	    entryPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
	} else {
	    entryPtr = Tcl_NextHashEntry(&search);
	}
    }

    /*
     * If the namespace has associated ensemble commands, delete them first.
     * This leaves the actual contents of the namespace alone (unless they are
     * linked ensemble commands, of course). Note that this code is actually
     * reentrant so command delete traces won't purturb things badly.
     */

    while (nsPtr->ensembles != NULL) {
	EnsembleConfig *ensemblePtr = (EnsembleConfig *) nsPtr->ensembles;

	/*
	 * Splice out and link to indicate that we've already been killed.
	 */

	nsPtr->ensembles = (Tcl_Ensemble *) ensemblePtr->next;
	ensemblePtr->next = ensemblePtr;
	Tcl_DeleteCommandFromToken(interp, ensemblePtr->token);
    }

    /*
     * If the namespace has a registered unknown handler (TIP 181), then free
     * it here.
     */

    if (nsPtr->unknownHandlerPtr != NULL) {
	Tcl_DecrRefCount(nsPtr->unknownHandlerPtr);
	nsPtr->unknownHandlerPtr = NULL;
    }

    /*
     * If the namespace is on the call frame stack, it is marked as "dying"
     * (NS_DYING is OR'd into its flags):  Contents of the namespace are
     * still available and visible until the namespace is later marked as
     * NS_DEAD, and its commands and variables are still usable by any
     * active call frames referring to th namespace. When all active call
     * frames referring to the namespace have been popped from the Tcl
     * stack, Tcl_PopCallFrame calls Tcl_DeleteNamespace again. If no
     * nsName objects refer to the namespace (i.e., if its refCount is
     * zero), its commands and variables are deleted and the storage for
     * its namespace structure is freed.  Otherwise, if its refCount is
     * nonzero, the namespace's commands and variables are deleted but the
     * structure isn't freed. Instead, NS_DEAD is OR'd into the structure's
     * flags to allow the namespace resolution code to recognize that the
     * namespace is "deleted".  The structure's storage is freed by
     * FreeNsNameInternalRep when its refCount reaches 0.
     */

    if (nsPtr->activationCount > (nsPtr == globalNsPtr)) {
	nsPtr->flags |= NS_DYING;
	if (nsPtr->parentPtr != NULL) {
	    entryPtr = FindChildEntry(nsPtr->parentPtr, nsPtr->name);
	    if (entryPtr != NULL) {
		Tcl_DeleteHashEntry(entryPtr);
	    }
	}
	nsPtr->parentPtr = NULL;
    } else if (!(nsPtr->flags & NS_TEARDOWN)) {
	/*
	 * Delete the namespace and everything in it. If this is the global
	 * namespace, then clear it but don't free its storage unless the
	 * interpreter is being torn down. Set the NS_TEARDOWN flag to avoid
	 * recursive calls here - if the namespace is really in the process of
	 * being deleted, ignore any second call.
	 */

	nsPtr->flags |= NS_DYING | NS_TEARDOWN;

	TclTeardownNamespace(nsPtr);

	if ((nsPtr != globalNsPtr) || (((Interp *) interp)->flags & DELETED)) {
	    /*
	     * If this is the global namespace, then it may have residual
	     * "errorInfo" and "errorCode" variables for errors that occurred
	     * while it was being torn down. Try to clear the variable list
	     * one last time.
	     */

	    TclDeleteNamespaceVars(nsPtr);

#ifndef BREAK_NAMESPACE_COMPAT
	    Tcl_DeleteHashTable(&nsPtr->childTable);
#else
	    if (nsPtr->childTablePtr != NULL) {
		Tcl_DeleteHashTable(nsPtr->childTablePtr);
		Tcl_Free(nsPtr->childTablePtr);
	    }
#endif
	    Tcl_DeleteHashTable(&nsPtr->cmdTable);

	    nsPtr ->flags |= NS_DEAD;
	} else {
	    /*
	     * Restore the ::errorInfo and ::errorCode traces.
	     */

	    EstablishErrorInfoTraces(NULL, interp, NULL, NULL, 0);
	    EstablishErrorCodeTraces(NULL, interp, NULL, NULL, 0);

	    /*
	     * We didn't really kill it, so remove the KILLED marks, so it can
	     * get killed later, avoiding mem leaks.
	     */

	    nsPtr->flags &= ~(NS_DYING|NS_TEARDOWN);
	}
    }
    TclNsDecrRefCount(nsPtr);
}

```

tcl9.1 9.1.0, revision `Retained release source identified by complete file digest`, `tmp/tcl9.1.0/generic/tclProc.c`, function `TclProcDeleteProc`, lines 2122–2130. Retained reference `delete-source-14`.

```text
TclProcDeleteProc(
    void *clientData)		/* Procedure to be deleted. */
{
    Proc *procPtr = (Proc *)clientData;

    if (procPtr->refCount-- <= 1) {
	TclProcCleanupProc(procPtr);
    }
}

```

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs), `OriginalNamespaceDeletionTransfer::capture`: Exact pre-command target identities from original operand and current table.
- [rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs), `OriginalNamespaceDeletionTransfer::after_command_transfer`: Same-operation retirement handoff, no completion or Native grant.
- [rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs), `AppliedOriginalNamespaceDeletions::matches`: Retained target/input/ordinal/site joins at variable transfer.
- [rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs), `OriginalNamespaceDeletion::capture`: Independent quiet current handler, fresh namespace, empty cells and intrinsic cleanup conditions.
- [rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs), `OriginalNamespaceDeletion::completed`: Actual normal successor and same-site post-transfer retirement.
- [rust/tcl-registry/src/commands/tcl/namespace_.rs](../../../../rust/tcl-registry/src/commands/tcl/namespace_.rs), `namespace_delete_count_arg_roles`: Exact cardinality selects the variadic NamespaceName roles without treating opaque bytes or computed values as unavailable text.
- [rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs), `command_binding::original_namespace_deletion::tests::original_fresh_namespace_delete_closes_only_empty_cells_and_intrinsic_procedures` (linked): Five pinned C authoring contexts retain fresh empty namespace retirement and direct intrinsic procedure cleanup, including opaque names and multiple disjoint targets; source creation lineage and same-operation identity remain distinct from native cleanup.
- [rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_deletion.rs), `command_binding::original_namespace_deletion::tests::original_namespace_delete_refuses_missing_unknown_and_overlapping_targets` (linked): Five pinned C authoring contexts refuse missing, dynamic, duplicate, overlapping parent/child targets and namespaces with values; no failed lookup or uncertain cleanup becomes Normal.
- [rust/tcl-registry/src/commands/tcl/namespace_.rs](../../../../rust/tcl-registry/src/commands/tcl/namespace_.rs), `commands::tcl::namespace_::tests::original_namespace_delete_effects_hand_only_legacy_state_to_the_recursive_transition` (linked): Five selected C releases hand only the parent legacy interpreter-state bridge to an actual recursive deletion transition; trace callbacks and legacy provenance remain. Zero target transitions retain the bridge.
- [rust/tcl-registry/src/commands/tcl/namespace_.rs](../../../../rust/tcl-registry/src/commands/tcl/namespace_.rs), `commands::tcl::namespace_::tests::original_namespace_delete_keeps_independent_policy_writes_and_trace_callbacks` (linked): Five selected C releases preserve an independently authored InterpreterPolicy write alongside recursive deletion and the actual TRACE callback footprint.
- [rust/tcl-registry/src/commands/tcl/namespace_.rs](../../../../rust/tcl-registry/src/commands/tcl/namespace_.rs), `commands::tcl::namespace_::tests::original_namespace_delete_roles_require_cardinality_without_requiring_text_values` (linked): Five selected C contexts retain NamespaceName roles for an actual counted opaque operand and a fixed-count dynamic operand; unknown expansion cardinality remains incomplete with no target identity or completion grant.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json)
