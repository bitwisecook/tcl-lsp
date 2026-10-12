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

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native execution receipt is attached to this source implementation contract.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native execution receipt is attached to this source implementation contract.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native execution receipt is attached to this source implementation contract.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native execution receipt is attached to this source implementation contract.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native execution receipt is attached to this source implementation contract.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native execution receipt is attached to this source implementation contract.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native execution receipt is attached to this source implementation contract.

## Exact evidence

- `delete-source-0` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/0/snippet`. tcl8.4 exact NamespaceDeleteCmd source window, independent of Native admission and successful completion.
- `delete-source-1` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/1/snippet`. tcl8.4 exact Tcl_DeleteNamespace source window, independent of Native admission and successful completion.
- `delete-source-2` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/2/snippet`. tcl8.4 exact TclProcDeleteProc source window, independent of Native admission and successful completion.
- `delete-source-3` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/3/snippet`. tcl8.5 exact NamespaceDeleteCmd source window, independent of Native admission and successful completion.
- `delete-source-4` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/4/snippet`. tcl8.5 exact Tcl_DeleteNamespace source window, independent of Native admission and successful completion.
- `delete-source-5` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/5/snippet`. tcl8.5 exact TclProcDeleteProc source window, independent of Native admission and successful completion.
- `delete-source-6` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/6/snippet`. tcl8.6 exact NamespaceDeleteCmd source window, independent of Native admission and successful completion.
- `delete-source-7` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/7/snippet`. tcl8.6 exact Tcl_DeleteNamespace source window, independent of Native admission and successful completion.
- `delete-source-8` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/8/snippet`. tcl8.6 exact TclProcDeleteProc source window, independent of Native admission and successful completion.
- `delete-source-9` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/9/snippet`. tcl9.0 exact NamespaceDeleteCmd source window, independent of Native admission and successful completion.
- `delete-source-10` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/10/snippet`. tcl9.0 exact Tcl_DeleteNamespace source window, independent of Native admission and successful completion.
- `delete-source-11` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/11/snippet`. tcl9.0 exact TclProcDeleteProc source window, independent of Native admission and successful completion.
- `delete-source-12` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/12/snippet`. tcl9.1 exact NamespaceDeleteCmd source window, independent of Native admission and successful completion.
- `delete-source-13` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/13/snippet`. tcl9.1 exact Tcl_DeleteNamespace source window, independent of Native admission and successful completion.
- `delete-source-14` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-deletion-source-anchors.json). SHA-256 `7db407e31fcbbe8182bb36e51cf6bea479cfffc8369397c0aac4abebc9d7e93d`. JSON pointer `/source_anchors/14/snippet`. tcl9.1 exact TclProcDeleteProc source window, independent of Native admission and successful completion.

## Source inspection

tcl8.4 8.4.20, revision `Retained release source identified by complete file digest`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `NamespaceDeleteCmd`, lines 2870–2916. Full-source SHA-256 `cd23865ee8d4b0ce81874d519b24de5b84a16c116ed3593ead366946b0512cbb`; snippet SHA-256 `dd823315076c14b8bdc59661daaa6c0d5093202a207de84ad4acd6d52ecfdeb2`; retained evidence `delete-source-0`.

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

tcl8.4 8.4.20, revision `Retained release source identified by complete file digest`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `Tcl_DeleteNamespace`, lines 576–656. Full-source SHA-256 `cd23865ee8d4b0ce81874d519b24de5b84a16c116ed3593ead366946b0512cbb`; snippet SHA-256 `f8b63c78ebc07621717f46199046e1e69fbdcbff1785f4e34a072cb5d4438843`; retained evidence `delete-source-1`.

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

tcl8.4 8.4.20, revision `Retained release source identified by complete file digest`, `tmp/tcl8.4.20/generic/tclProc.c`, function `TclProcDeleteProc`, lines 1605–1614. Full-source SHA-256 `bfeecc7c08dabce16946efad9c0e7eb3cacf43d4a5376bf622fa4b69483eb879`; snippet SHA-256 `7b5b030e47c17f1d3dde75c7fde84d143eadd87e8d4c73d3cab9b5d02e9567d4`; retained evidence `delete-source-2`.

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

tcl8.5 8.5.19, revision `Retained release source identified by complete file digest`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceDeleteCmd`, lines 3177–3224. Full-source SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`; snippet SHA-256 `d024fde7fa0afa81770b0d31ec2ada21c8c20a9821a5897e9e0ae0126bf6b8f9`; retained evidence `delete-source-3`.

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

tcl8.5 8.5.19, revision `Retained release source identified by complete file digest`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `Tcl_DeleteNamespace`, lines 945–1062. Full-source SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`; snippet SHA-256 `a13b9b9393b1f4818381bf426cda0ee888d8816b54f989f63233a81aae6c0342`; retained evidence `delete-source-4`.

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

tcl8.5 8.5.19, revision `Retained release source identified by complete file digest`, `tmp/tcl8.5.19/generic/tclProc.c`, function `TclProcDeleteProc`, lines 2130–2139. Full-source SHA-256 `9adc0419de1af0c81c7718e1e682fb50f4e2873f7b2d07a0f3cbe380f7d2dce6`; snippet SHA-256 `f44ba1ba2b25da80d92c5182de7839e3fb10f877781652f8a602362543d030c6`; retained evidence `delete-source-5`.

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

tcl8.6 8.6.18, revision `Retained release source identified by complete file digest`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `NamespaceDeleteCmd`, lines 3185–3232. Full-source SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`; snippet SHA-256 `d15a5140e95b53e0193c02372e1e51843801c74a298c2cd36fa40b37f31d8d1e`; retained evidence `delete-source-6`.

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

tcl8.6 8.6.18, revision `Retained release source identified by complete file digest`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `Tcl_DeleteNamespace`, lines 882–1040. Full-source SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`; snippet SHA-256 `e29adfd56e61548caa6529324244f690dc3c9984a23e341d378e54d01552e837`; retained evidence `delete-source-7`.

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

tcl8.6 8.6.18, revision `Retained release source identified by complete file digest`, `tmp/tcl8.6.18/generic/tclProc.c`, function `TclProcDeleteProc`, lines 2106–2114. Full-source SHA-256 `c1ddd801a69b0e39bf923e97838134489ca4937fb7d081eccc0814b6811f71b3`; snippet SHA-256 `dec1e7fcb9de3af28b8e02be00acf2750cc82af95386a5ebf3189d1cffd1038d`; retained evidence `delete-source-8`.

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

tcl9.0 9.0.4, revision `Retained release source identified by complete file digest`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `NamespaceDeleteCmd`, lines 3391–3438. Full-source SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`; snippet SHA-256 `aff64b3315c8da516cf06d7b6054e5397ea067c32a6a4b5f771f3bc4340ef0ce`; retained evidence `delete-source-9`.

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

tcl9.0 9.0.4, revision `Retained release source identified by complete file digest`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `Tcl_DeleteNamespace`, lines 1012–1175. Full-source SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`; snippet SHA-256 `8bc75e485083864ec8cf1307512f59d3545d7d2632447debf34988382de6e7ae`; retained evidence `delete-source-10`.

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

tcl9.0 9.0.4, revision `Retained release source identified by complete file digest`, `tmp/tcl9.0.4/generic/tclProc.c`, function `TclProcDeleteProc`, lines 2124–2132. Full-source SHA-256 `ef5ef608fbb22c68d35317e09637f099380457e5d01ff8e1b67f2a8eeb70c110`; snippet SHA-256 `17200e534b0f3185ba7cb1f6b896c812094a24b1daaf034824259fb69b97102c`; retained evidence `delete-source-11`.

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

tcl9.1 9.1.0, revision `Retained release source identified by complete file digest`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `NamespaceDeleteCmd`, lines 3363–3410. Full-source SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`; snippet SHA-256 `2ea84b5b6d7c10b50af49cfd740f1859ab8641fc19fb6b32a4fc4aec2ed4df1d`; retained evidence `delete-source-12`.

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

tcl9.1 9.1.0, revision `Retained release source identified by complete file digest`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `Tcl_DeleteNamespace`, lines 1011–1174. Full-source SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`; snippet SHA-256 `f50a1411dda58926a03dc4ef88d4048b3ebcb9c9c5b39bbe0d5aaaf7108639dc`; retained evidence `delete-source-13`.

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

tcl9.1 9.1.0, revision `Retained release source identified by complete file digest`, `tmp/tcl9.1.0/generic/tclProc.c`, function `TclProcDeleteProc`, lines 2122–2130. Full-source SHA-256 `2c5cf8968a3176aa8e5c316063594186f50109cecd2fbe3c3dff2f3035cefd1a`; snippet SHA-256 `17200e534b0f3185ba7cb1f6b896c812094a24b1daaf034824259fb69b97102c`; retained evidence `delete-source-14`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Rust selectors require the genuine complete source driver and selected Registry/configuration/frame. No executed Rust or Native completion receipt is attached. All unknown cells, observers, cleanup and Native entries remain refusals.
