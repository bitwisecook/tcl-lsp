Tcl_Import(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Namespace *namespacePtr,/* Points to the namespace into which the
				 * commands are to be imported. NULL for the
				 * current namespace. */
    const char *pattern,	/* String pattern indicating which commands to
				 * import. This pattern should be qualified by
				 * the name of the namespace from which to
				 * import the command(s). */
    int allowOverwrite)		/* If nonzero, allow existing commands to be
				 * overwritten by imported commands. If 0,
				 * return an error if an imported cmd
				 * conflicts with an existing one. */
{
    Namespace *nsPtr, *importNsPtr, *dummyPtr;
    const char *simplePattern;
    Tcl_HashEntry *hPtr;
    Tcl_HashSearch search;

    /*
     * If the specified namespace is NULL, use the current namespace.
     */

    if (namespacePtr == NULL) {
	nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    } else {
	nsPtr = (Namespace *) namespacePtr;
    }

    /*
     * First, invoke the "auto_import" command with the pattern being
     * imported. This command is part of the Tcl library. It looks for
     * imported commands in autoloaded libraries and loads them in. That way,
     * they will be found when we try to create links below.
     *
     * Note that we don't just call Tcl_EvalObjv() directly because we do not
     * want absence of the command to be a failure case.
     */

    if (Tcl_FindCommand(interp, "auto_import", NULL, TCL_GLOBAL_ONLY) != NULL) {
	Tcl_Obj *objv[2];
	int result;

	TclNewLiteralStringObj(objv[0], "auto_import");
	objv[1] = Tcl_NewStringObj(pattern, -1);

	Tcl_IncrRefCount(objv[0]);
	Tcl_IncrRefCount(objv[1]);
	result = Tcl_EvalObjv(interp, 2, objv, TCL_GLOBAL_ONLY);
	Tcl_DecrRefCount(objv[0]);
	Tcl_DecrRefCount(objv[1]);

	if (result != TCL_OK) {
	    return TCL_ERROR;
	}
	Tcl_ResetResult(interp);
    }

    /*
     * From the pattern, find the namespace from which we are importing and
     * get the simple pattern (no namespace qualifiers or ::'s) at the end.
     */

    if (strlen(pattern) == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj("empty import pattern",-1));
	Tcl_SetErrorCode(interp, "TCL", "IMPORT", "EMPTY", (char *)NULL);
	return TCL_ERROR;
    }
    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &importNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if (importNsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"unknown namespace in import pattern \"%s\"", pattern));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE", pattern, (char *)NULL);
	return TCL_ERROR;
    }
    if (importNsPtr == nsPtr) {
	if (pattern == simplePattern) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "no namespace specified in import pattern \"%s\"",
		    pattern));
	    Tcl_SetErrorCode(interp, "TCL", "IMPORT", "ORIGIN", (char *)NULL);
	} else {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "import pattern \"%s\" tries to import from namespace"
		    " \"%s\" into itself", pattern, importNsPtr->name));
	    Tcl_SetErrorCode(interp, "TCL", "IMPORT", "SELF", (char *)NULL);
	}
	return TCL_ERROR;
    }

    /*
     * Scan through the command table in the source namespace and look for
     * exported commands that match the string pattern. Create an "imported
     * command" in the current namespace for each imported command; these
     * commands redirect their invocations to the "real" command.
     */

    if ((simplePattern != NULL) && TclMatchIsTrivial(simplePattern)) {
	hPtr = Tcl_FindHashEntry(&importNsPtr->cmdTable, simplePattern);
	if (hPtr == NULL) {
	    return TCL_OK;
	}
	return DoImport(interp, nsPtr, hPtr, simplePattern, pattern,
		importNsPtr, allowOverwrite);
    }
    for (hPtr = Tcl_FirstHashEntry(&importNsPtr->cmdTable, &search);
	    (hPtr != NULL); hPtr = Tcl_NextHashEntry(&search)) {
	char *cmdName = (char *)Tcl_GetHashKey(&importNsPtr->cmdTable, hPtr);

	if (Tcl_StringMatch(cmdName, simplePattern) &&
		DoImport(interp, nsPtr, hPtr, cmdName, pattern, importNsPtr,
		allowOverwrite) == TCL_ERROR) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}
