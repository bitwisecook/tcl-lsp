Tcl_Import(interp, namespacePtr, pattern, allowOverwrite)
    Tcl_Interp *interp;		 /* Current interpreter. */
    Tcl_Namespace *namespacePtr; /* Points to the namespace into which the
				  * commands are to be imported. NULL for
                                  * the current namespace. */
    CONST char *pattern;         /* String pattern indicating which commands
                                  * to import. This pattern should be
				  * qualified by the name of the namespace
				  * from which to import the command(s). */
    int allowOverwrite;		 /* If nonzero, allow existing commands to
				  * be overwritten by imported commands.
				  * If 0, return an error if an imported
				  * cmd conflicts with an existing one. */
{
    Interp *iPtr = (Interp *) interp;
    Namespace *nsPtr, *importNsPtr, *dummyPtr;
    Namespace *currNsPtr = (Namespace *) Tcl_GetCurrentNamespace(interp);
    CONST char *simplePattern;
    char *cmdName;
    register Tcl_HashEntry *hPtr;
    Tcl_HashSearch search;
    Command *cmdPtr;
    ImportRef *refPtr;
    Tcl_Command autoCmd, importedCmd;
    ImportedCmdData *dataPtr;
    int wasExported, i, result;

    /*
     * If the specified namespace is NULL, use the current namespace.
     */

    if (namespacePtr == NULL) {
        nsPtr = (Namespace *) currNsPtr;
    } else {
        nsPtr = (Namespace *) namespacePtr;
    }
 
    /*
     * First, invoke the "auto_import" command with the pattern
     * being imported.  This command is part of the Tcl library.
     * It looks for imported commands in autoloaded libraries and
     * loads them in.  That way, they will be found when we try
     * to create links below.
     */
    
    autoCmd = Tcl_FindCommand(interp, "auto_import",
 	    (Tcl_Namespace *) NULL, /*flags*/ TCL_GLOBAL_ONLY);
 
    if (autoCmd != NULL) {
	Tcl_Obj *objv[2];
 
	objv[0] = Tcl_NewStringObj("auto_import", -1);
	Tcl_IncrRefCount(objv[0]);
	objv[1] = Tcl_NewStringObj(pattern, -1);
	Tcl_IncrRefCount(objv[1]);
 
	cmdPtr = (Command *) autoCmd;
	result = (*cmdPtr->objProc)(cmdPtr->objClientData, interp,
		2, objv);
 
	Tcl_DecrRefCount(objv[0]);
	Tcl_DecrRefCount(objv[1]);
 
	if (result != TCL_OK) {
	    return TCL_ERROR;
	}
	Tcl_ResetResult(interp);
    }

    /*
     * From the pattern, find the namespace from which we are importing
     * and get the simple pattern (no namespace qualifiers or ::'s) at
     * the end.
     */

    if (strlen(pattern) == 0) {
	Tcl_SetStringObj(Tcl_GetObjResult(interp),
	        "empty import pattern", -1);
        return TCL_ERROR;
    }
    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &importNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if (importNsPtr == NULL) {
	Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		"unknown namespace in import pattern \"",
		pattern, "\"", (char *) NULL);
        return TCL_ERROR;
    }
    if (importNsPtr == nsPtr) {
	if (pattern == simplePattern) {
	    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		    "no namespace specified in import pattern \"", pattern,
		    "\"", (char *) NULL);
	} else {
	    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		    "import pattern \"", pattern,
		    "\" tries to import from namespace \"",
		    importNsPtr->name, "\" into itself", (char *) NULL);
	}
        return TCL_ERROR;
    }

    /*
     * Scan through the command table in the source namespace and look for
     * exported commands that match the string pattern. Create an "imported
     * command" in the current namespace for each imported command; these
     * commands redirect their invocations to the "real" command.
     */

    for (hPtr = Tcl_FirstHashEntry(&importNsPtr->cmdTable, &search);
	    (hPtr != NULL);
	    hPtr = Tcl_NextHashEntry(&search)) {
        cmdName = Tcl_GetHashKey(&importNsPtr->cmdTable, hPtr);
        if (Tcl_StringMatch(cmdName, simplePattern)) {
	    /*
	     * The command cmdName in the source namespace matches the
	     * pattern. Check whether it was exported. If it wasn't,
	     * we ignore it.
	     */
	    Tcl_HashEntry *found;

	    wasExported = 0;
	    for (i = 0;  i < importNsPtr->numExportPatterns;  i++) {
		if (Tcl_StringMatch(cmdName,
			importNsPtr->exportArrayPtr[i])) {
		    wasExported = 1;
		    break;
		}
	    }
	    if (!wasExported) {
		continue;
            }

	    /*
	     * Unless there is a name clash, create an imported command
	     * in the current namespace that refers to cmdPtr.
	     */

	    found = Tcl_FindHashEntry(&nsPtr->cmdTable, cmdName);
	    if ((found == NULL) || allowOverwrite) {
		/*
		 * Create the imported command and its client data.
		 * To create the new command in the current namespace, 
		 * generate a fully qualified name for it.
		 */

		Tcl_DString ds;

		Tcl_DStringInit(&ds);
		Tcl_DStringAppend(&ds, nsPtr->fullName, -1);
		if (nsPtr != iPtr->globalNsPtr) {
		    Tcl_DStringAppend(&ds, "::", 2);
		}
		Tcl_DStringAppend(&ds, cmdName, -1);

		/*
		 * Check whether creating the new imported command in the
		 * current namespace would create a cycle of imported
		 * command references.
		 */

		cmdPtr = (Command *) Tcl_GetHashValue(hPtr);
		if ((found != NULL)
			&& cmdPtr->deleteProc == DeleteImportedCmd) {

		    Command *overwrite = (Command *) Tcl_GetHashValue(found);
		    Command *link = cmdPtr;
		    while (link->deleteProc == DeleteImportedCmd) {
			ImportedCmdData *dataPtr;
		       
			dataPtr = (ImportedCmdData *) link->objClientData;
			link = dataPtr->realCmdPtr;
			if (overwrite == link) {
			    Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
				    "import pattern \"", pattern,
				    "\" would create a loop containing ",
				    "command \"", Tcl_DStringValue(&ds),
				    "\"", (char *) NULL);
			    Tcl_DStringFree(&ds);
			    return TCL_ERROR;
			}
		    }
		}

		dataPtr = (ImportedCmdData *)
		        ckalloc(sizeof(ImportedCmdData));
                importedCmd = Tcl_CreateObjCommand(interp, 
                        Tcl_DStringValue(&ds), InvokeImportedCmd,
                        (ClientData) dataPtr, DeleteImportedCmd);
		dataPtr->realCmdPtr = cmdPtr;
		dataPtr->selfPtr = (Command *) importedCmd;
		dataPtr->selfPtr->compileProc = cmdPtr->compileProc;
		Tcl_DStringFree(&ds);

		/*
		 * Create an ImportRef structure describing this new import
		 * command and add it to the import ref list in the "real"
		 * command.
		 */

                refPtr = (ImportRef *) ckalloc(sizeof(ImportRef));
                refPtr->importedCmdPtr = (Command *) importedCmd;
                refPtr->nextPtr = cmdPtr->importRefPtr;
                cmdPtr->importRefPtr = refPtr;
            } else {
		Command *overwrite = (Command *) Tcl_GetHashValue(found);
		if (overwrite->deleteProc == DeleteImportedCmd) {
		    ImportedCmdData *dataPtr =
			    (ImportedCmdData *) overwrite->objClientData;
		    if (dataPtr->realCmdPtr
			    == (Command *) Tcl_GetHashValue(hPtr)) {
			/* Repeated import of same command -- acceptable */
			return TCL_OK;
		    }
		}
		Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		        "can't import command \"", cmdName,
			"\": already exists", (char *) NULL);
                return TCL_ERROR;
            }
        }
    }
    return TCL_OK;
}
