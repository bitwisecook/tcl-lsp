NamespaceImportCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int allowOverwrite = 0;
    const char *string, *pattern;
    int i, result;
    int firstArg;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-force? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * Skip over the optional "-force" as the first argument.
     */

    firstArg = 1;
    if (firstArg < objc) {
	string = TclGetString(objv[firstArg]);
	if ((*string == '-') && (strcmp(string, "-force") == 0)) {
	    allowOverwrite = 1;
	    firstArg++;
	}
    } else {
	/*
	 * When objc == 1, command is just [namespace import]. Introspection
	 * form to return list of imported commands.
	 */

	Tcl_HashEntry *hPtr;
	Tcl_HashSearch search;
	Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		hPtr != NULL; hPtr = Tcl_NextHashEntry(&search)) {
	    Command *cmdPtr = (Command *) Tcl_GetHashValue(hPtr);

	    if (cmdPtr->deleteProc == DeleteImportedCmd) {
		Tcl_ListObjAppendElement(NULL, listPtr, Tcl_NewStringObj(
			(char *) Tcl_GetHashKey(&nsPtr->cmdTable, hPtr), -1));
	    }
	}
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Handle the imports for each of the patterns.
     */

    for (i = firstArg;  i < objc;  i++) {
	pattern = TclGetString(objv[i]);
	result = Tcl_Import(interp, NULL, pattern, allowOverwrite);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}
