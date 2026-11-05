# naming.namespace.original-quiet-pattern-completion

Kind: `implementation-contract`

## Problem statement

A publication or OnOk table transition cannot establish completion when import can call a preload helper or remove an occupied destination whose cleanup has not been retained.

## Question

Can a fresh quiet source-only pattern operation issue completion only after its independently closed handler and exact actual transfer are verified?

## Conclusion

The private completion envelope retains the actual initial handler, full original static argv, selected namespace and independently expected canonical table transfer. Import requires a proved absent global preload helper, known exports and fresh destinations; forget requires surviving direct token origins. Post-state correspondence, observer absence and current handler identity are checked before the existing common completion receipt is retained. Geometry and publication alone issue no completion. Private stock namespace workers retain their original selected parent descriptor through the canonical Registry registration lookup; an explicit authored descriptor keeps its own provenance and cannot borrow that parent pack.

## Scope

C Tcl source-model naming under Tcl 8.4, 8.5, 8.6, 9.0 and 9.1 selected policies, genuine original operands and independently retained ASCII namespace geometry. Opaque command/pattern units remain counted data under the shared selected native purposes. Jim export is a selected no-op; its source-name import helper and unimplemented forget do not receive C token or completion authority. Supplied native entries, generated opaque namespace allocation, arbitrary preload helpers, replacement cleanup, unknown callbacks and physical compiler/cache/namespace authority are excluded. The original registration source descriptor is metadata only. Modern private workers independently retain their actual same-site initial handler, current allocation, complete operand vector, exact policy and expected transfer before completion; descriptor projection supplies no token, compiler or Normal authority.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No native execution of this Rust implementation question is claimed.

## Source inspection

tcl8.4 8.4.20, revision `retained release source`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `Tcl_Import`, lines 1079–1303. Retained reference `tcl8.4-Tcl_Import-source`.

```text
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

```

tcl8.4 8.4.20, revision `retained release source`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `Tcl_ForgetImport`, lines 1334–1428. Retained reference `tcl8.4-Tcl_ForgetImport-source`.

```text
Tcl_ForgetImport(interp, namespacePtr, pattern)
    Tcl_Interp *interp;		 /* Current interpreter. */
    Tcl_Namespace *namespacePtr; /* Points to the namespace from which
				  * previously imported commands should be
				  * removed. NULL for current namespace. */
    CONST char *pattern;	 /* String pattern indicating which imported
				  * commands to remove. */
{
    Namespace *nsPtr, *sourceNsPtr, *dummyPtr;
    CONST char *simplePattern;
    char *cmdName;
    register Tcl_HashEntry *hPtr;
    Tcl_HashSearch search;

    /*
     * If the specified namespace is NULL, use the current namespace.
     */

    if (namespacePtr == NULL) {
        nsPtr = (Namespace *) Tcl_GetCurrentNamespace(interp);
    } else {
        nsPtr = (Namespace *) namespacePtr;
    }

    /*
     * Parse the pattern into its namespace-qualification (if any)
     * and the simple pattern.
     */

    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &sourceNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if (sourceNsPtr == NULL) {
        Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
		"unknown namespace in namespace forget pattern \"",
		pattern, "\"", (char *) NULL);
        return TCL_ERROR;
    }

    if (strcmp(pattern, simplePattern) == 0) {
	/*
	 * The pattern is simple.
	 * Delete any imported commands that match it.
	 */

	for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		(hPtr != NULL);
		hPtr = Tcl_NextHashEntry(&search)) {
	    Command *cmdPtr = (Command *) Tcl_GetHashValue(hPtr);
	    if (cmdPtr->deleteProc != DeleteImportedCmd) {
		continue;
	    }
	    cmdName = Tcl_GetHashKey(&nsPtr->cmdTable, hPtr);
	    if (Tcl_StringMatch(cmdName, simplePattern)) {
		Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	    }
	}
	return TCL_OK;
    }

    /* The pattern was namespace-qualified */

    for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search); (hPtr != NULL);
	    hPtr = Tcl_NextHashEntry(&search)) {
	Tcl_CmdInfo info;
	Tcl_Command token = (Tcl_Command) Tcl_GetHashValue(hPtr);
	Tcl_Command origin = TclGetOriginalCommand(token);

	if (Tcl_GetCommandInfoFromToken(origin, &info) == 0) {
	    continue;	/* Not an imported command */
	}
	if (info.namespacePtr != (Tcl_Namespace *) sourceNsPtr) {
	    /*
	     * Original not in namespace we're matching.
	     * Check the first link in the import chain.
	     */
	    Command *cmdPtr = (Command *) token;
	    ImportedCmdData *dataPtr =
		    (ImportedCmdData *) cmdPtr->objClientData;
	    Tcl_Command firstToken = (Tcl_Command) dataPtr->realCmdPtr;
	    if (firstToken == origin) {
		continue;
	    }
	    Tcl_GetCommandInfoFromToken(firstToken, &info);
	    if (info.namespacePtr != (Tcl_Namespace *) sourceNsPtr) {
		continue;
	    }
	    origin = firstToken;
	}
	if (Tcl_StringMatch(Tcl_GetCommandName(NULL, origin), simplePattern)) {
	    Tcl_DeleteCommandFromToken(interp, token);
	}
    }
    return TCL_OK;
}

```

tcl8.4 8.4.20, revision `retained release source`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `NamespaceImportCmd`, lines 3268–3311. Retained reference `tcl8.4-NamespaceImportCmd-source`.

```text
NamespaceImportCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int allowOverwrite = 0;
    char *string, *pattern;
    register int i, result;
    int firstArg;

    if (objc < 2) {
        Tcl_WrongNumArgs(interp, 2, objv,
	        "?-force? ?pattern pattern...?");
        return TCL_ERROR;
    }

    /*
     * Skip over the optional "-force" as the first argument.
     */

    firstArg = 2;
    if (firstArg < objc) {
	string = Tcl_GetString(objv[firstArg]);
	if ((*string == '-') && (strcmp(string, "-force") == 0)) {
	    allowOverwrite = 1;
	    firstArg++;
	}
    }

    /*
     * Handle the imports for each of the patterns.
     */

    for (i = firstArg;  i < objc;  i++) {
        pattern = Tcl_GetString(objv[i]);
	result = Tcl_Import(interp, (Tcl_Namespace *) NULL, pattern,
	        allowOverwrite);
        if (result != TCL_OK) {
            return result;
        }
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained release source`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `Tcl_Import`, lines 1460–1574. Retained reference `tcl8.5-Tcl_Import-source`.

```text
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
    register Tcl_HashEntry *hPtr;
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

    if (Tcl_FindCommand(interp,"auto_import",NULL,TCL_GLOBAL_ONLY) != NULL) {
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
	Tcl_SetObjResult(interp, Tcl_NewStringObj("empty import pattern", -1));
	return TCL_ERROR;
    }
    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &importNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if (importNsPtr == NULL) {
	Tcl_AppendResult(interp, "unknown namespace in import pattern \"",
		pattern, "\"", NULL);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE", pattern, NULL);
	return TCL_ERROR;
    }
    if (importNsPtr == nsPtr) {
	if (pattern == simplePattern) {
	    Tcl_AppendResult(interp,
		    "no namespace specified in import pattern \"", pattern,
		    "\"", NULL);
	} else {
	    Tcl_AppendResult(interp, "import pattern \"", pattern,
		    "\" tries to import from namespace \"",
		    importNsPtr->name, "\" into itself", NULL);
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
	char *cmdName = Tcl_GetHashKey(&importNsPtr->cmdTable, hPtr);
	if (Tcl_StringMatch(cmdName, simplePattern) &&
		DoImport(interp, nsPtr, hPtr, cmdName, pattern, importNsPtr,
		allowOverwrite) == TCL_ERROR) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained release source`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `Tcl_ForgetImport`, lines 1736–1844. Retained reference `tcl8.5-Tcl_ForgetImport-source`.

```text
Tcl_ForgetImport(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Namespace *namespacePtr,/* Points to the namespace from which
				 * previously imported commands should be
				 * removed. NULL for current namespace. */
    const char *pattern)	/* String pattern indicating which imported
				 * commands to remove. */
{
    Namespace *nsPtr, *sourceNsPtr, *dummyPtr;
    const char *simplePattern;
    char *cmdName;
    register Tcl_HashEntry *hPtr;
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
     * Parse the pattern into its namespace-qualification (if any) and the
     * simple pattern.
     */

    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &sourceNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if (sourceNsPtr == NULL) {
	Tcl_AppendResult(interp,
		"unknown namespace in namespace forget pattern \"",
		pattern, "\"", NULL);
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE", pattern, NULL);
	return TCL_ERROR;
    }

    if (strcmp(pattern, simplePattern) == 0) {
	/*
	 * The pattern is simple. Delete any imported commands that match it.
	 */

	if (TclMatchIsTrivial(simplePattern)) {
	    Command *cmdPtr;

	    hPtr = Tcl_FindHashEntry(&nsPtr->cmdTable, simplePattern);
	    if ((hPtr != NULL)
		    && (cmdPtr = Tcl_GetHashValue(hPtr))
		    && (cmdPtr->deleteProc == DeleteImportedCmd)) {
		Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	    }
	    return TCL_OK;
	}
	for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		(hPtr != NULL); hPtr = Tcl_NextHashEntry(&search)) {
	    Command *cmdPtr = Tcl_GetHashValue(hPtr);

	    if (cmdPtr->deleteProc != DeleteImportedCmd) {
		continue;
	    }
	    cmdName = Tcl_GetHashKey(&nsPtr->cmdTable, hPtr);
	    if (Tcl_StringMatch(cmdName, simplePattern)) {
		Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	    }
	}
	return TCL_OK;
    }

    /*
     * The pattern was namespace-qualified.
     */

    for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search); (hPtr != NULL);
	    hPtr = Tcl_NextHashEntry(&search)) {
	Tcl_CmdInfo info;
	Tcl_Command token = Tcl_GetHashValue(hPtr);
	Tcl_Command origin = TclGetOriginalCommand(token);

	if (Tcl_GetCommandInfoFromToken(origin, &info) == 0) {
	    continue;			/* Not an imported command. */
	}
	if (info.namespacePtr != (Tcl_Namespace *) sourceNsPtr) {
	    /*
	     * Original not in namespace we're matching. Check the first link
	     * in the import chain.
	     */

	    Command *cmdPtr = (Command *) token;
	    ImportedCmdData *dataPtr = cmdPtr->objClientData;
	    Tcl_Command firstToken = (Tcl_Command) dataPtr->realCmdPtr;

	    if (firstToken == origin) {
		continue;
	    }
	    Tcl_GetCommandInfoFromToken(firstToken, &info);
	    if (info.namespacePtr != (Tcl_Namespace *) sourceNsPtr) {
		continue;
	    }
	    origin = firstToken;
	}
	if (Tcl_StringMatch(Tcl_GetCommandName(NULL, origin), simplePattern)) {
	    Tcl_DeleteCommandFromToken(interp, token);
	}
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained release source`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceImportCmd`, lines 3575–3639. Retained reference `tcl8.5-NamespaceImportCmd-source`.

```text
NamespaceImportCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int allowOverwrite = 0;
    char *string, *pattern;
    register int i, result;
    int firstArg;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 2, objv, "?-force? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * Skip over the optional "-force" as the first argument.
     */

    firstArg = 2;
    if (firstArg < objc) {
	string = TclGetString(objv[firstArg]);
	if ((*string == '-') && (strcmp(string, "-force") == 0)) {
	    allowOverwrite = 1;
	    firstArg++;
	}
    } else {
	/*
	 * When objc == 2, command is just [namespace import]. Introspection
	 * form to return list of imported commands.
	 */

	Tcl_HashEntry *hPtr;
	Tcl_HashSearch search;
	Namespace *nsPtr = (Namespace *) TclGetCurrentNamespace(interp);
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		hPtr != NULL; hPtr = Tcl_NextHashEntry(&search)) {
	    Command *cmdPtr = Tcl_GetHashValue(hPtr);

	    if (cmdPtr->deleteProc == DeleteImportedCmd) {
		Tcl_ListObjAppendElement(NULL, listPtr, Tcl_NewStringObj(
			Tcl_GetHashKey(&nsPtr->cmdTable, hPtr) ,-1));
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

```

tcl8.6 8.6.18, revision `retained release source`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `Tcl_Import`, lines 1502–1620. Retained reference `tcl8.6-Tcl_Import-source`.

```text
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

```

tcl8.6 8.6.18, revision `retained release source`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `Tcl_ForgetImport`, lines 1786–1894. Retained reference `tcl8.6-Tcl_ForgetImport-source`.

```text
Tcl_ForgetImport(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Namespace *namespacePtr,/* Points to the namespace from which
				 * previously imported commands should be
				 * removed. NULL for current namespace. */
    const char *pattern)	/* String pattern indicating which imported
				 * commands to remove. */
{
    Namespace *nsPtr, *sourceNsPtr, *dummyPtr;
    const char *simplePattern;
    char *cmdName;
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
     * Parse the pattern into its namespace-qualification (if any) and the
     * simple pattern.
     */

    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &sourceNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if (sourceNsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"unknown namespace in namespace forget pattern \"%s\"",
		pattern));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE", pattern, (char *)NULL);
	return TCL_ERROR;
    }

    if (strcmp(pattern, simplePattern) == 0) {
	/*
	 * The pattern is simple. Delete any imported commands that match it.
	 */

	if (TclMatchIsTrivial(simplePattern)) {
	    hPtr = Tcl_FindHashEntry(&nsPtr->cmdTable, simplePattern);
	    if (hPtr != NULL) {
		Command *cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

		if (cmdPtr && (cmdPtr->deleteProc == DeleteImportedCmd)) {
		    Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
		}
	    }
	    return TCL_OK;
	}
	for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		(hPtr != NULL); hPtr = Tcl_NextHashEntry(&search)) {
	    Command *cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	    if (cmdPtr->deleteProc != DeleteImportedCmd) {
		continue;
	    }
	    cmdName = (char *)Tcl_GetHashKey(&nsPtr->cmdTable, hPtr);
	    if (Tcl_StringMatch(cmdName, simplePattern)) {
		Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	    }
	}
	return TCL_OK;
    }

    /*
     * The pattern was namespace-qualified.
     */

    for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search); (hPtr != NULL);
	    hPtr = Tcl_NextHashEntry(&search)) {
	Tcl_CmdInfo info;
	Tcl_Command token = (Tcl_Command)Tcl_GetHashValue(hPtr);
	Tcl_Command origin = TclGetOriginalCommand(token);

	if (Tcl_GetCommandInfoFromToken(origin, &info) == 0) {
	    continue;			/* Not an imported command. */
	}
	if (info.namespacePtr != (Tcl_Namespace *) sourceNsPtr) {
	    /*
	     * Original not in namespace we're matching. Check the first link
	     * in the import chain.
	     */

	    Command *cmdPtr = (Command *) token;
	    ImportedCmdData *dataPtr = (ImportedCmdData *)cmdPtr->objClientData;
	    Tcl_Command firstToken = (Tcl_Command) dataPtr->realCmdPtr;

	    if (firstToken == origin) {
		continue;
	    }
	    Tcl_GetCommandInfoFromToken(firstToken, &info);
	    if (info.namespacePtr != (Tcl_Namespace *) sourceNsPtr) {
		continue;
	    }
	    origin = firstToken;
	}
	if (Tcl_StringMatch(Tcl_GetCommandName(NULL, origin), simplePattern)) {
	    Tcl_DeleteCommandFromToken(interp, token);
	}
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained release source`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `NamespaceImportCmd`, lines 3607–3671. Retained reference `tcl8.6-NamespaceImportCmd-source`.

```text
NamespaceImportCmd(
    ClientData dummy,		/* Not used. */
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
	    Command *cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	    if (cmdPtr->deleteProc == DeleteImportedCmd) {
		Tcl_ListObjAppendElement(NULL, listPtr, Tcl_NewStringObj(
			(char *)Tcl_GetHashKey(&nsPtr->cmdTable, hPtr), -1));
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

```

tcl9.0 9.0.4, revision `retained release source`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `Tcl_Import`, lines 1653–1771. Retained reference `tcl9.0-Tcl_Import-source`.

```text
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
	char *cmdName = (char *) Tcl_GetHashKey(&importNsPtr->cmdTable, hPtr);

	if (Tcl_StringMatch(cmdName, simplePattern) &&
		DoImport(interp, nsPtr, hPtr, cmdName, pattern, importNsPtr,
		allowOverwrite) == TCL_ERROR) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained release source`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `Tcl_ForgetImport`, lines 1939–2048. Retained reference `tcl9.0-Tcl_ForgetImport-source`.

```text
Tcl_ForgetImport(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Namespace *namespacePtr,/* Points to the namespace from which
				 * previously imported commands should be
				 * removed. NULL for current namespace. */
    const char *pattern)	/* String pattern indicating which imported
				 * commands to remove. */
{
    Namespace *nsPtr, *sourceNsPtr, *dummyPtr;
    const char *simplePattern;
    char *cmdName;
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
     * Parse the pattern into its namespace-qualification (if any) and the
     * simple pattern.
     */

    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &sourceNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if (sourceNsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"unknown namespace in namespace forget pattern \"%s\"",
		pattern));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE", pattern, (char *)NULL);
	return TCL_ERROR;
    }

    if (strcmp(pattern, simplePattern) == 0) {
	/*
	 * The pattern is simple. Delete any imported commands that match it.
	 */

	if (TclMatchIsTrivial(simplePattern)) {
	    hPtr = Tcl_FindHashEntry(&nsPtr->cmdTable, simplePattern);
	    if (hPtr != NULL) {
		Command *cmdPtr = (Command *) Tcl_GetHashValue(hPtr);

		if (cmdPtr && (cmdPtr->deleteProc == DeleteImportedCmd)) {
		    Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
		}
	    }
	    return TCL_OK;
	}
	for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		(hPtr != NULL); hPtr = Tcl_NextHashEntry(&search)) {
	    Command *cmdPtr = (Command *) Tcl_GetHashValue(hPtr);

	    if (cmdPtr->deleteProc != DeleteImportedCmd) {
		continue;
	    }
	    cmdName = (char *) Tcl_GetHashKey(&nsPtr->cmdTable, hPtr);
	    if (Tcl_StringMatch(cmdName, simplePattern)) {
		Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	    }
	}
	return TCL_OK;
    }

    /*
     * The pattern was namespace-qualified.
     */

    for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search); (hPtr != NULL);
	    hPtr = Tcl_NextHashEntry(&search)) {
	Tcl_CmdInfo info;
	Tcl_Command token = (Tcl_Command) Tcl_GetHashValue(hPtr);
	Tcl_Command origin = TclGetOriginalCommand(token);

	if (Tcl_GetCommandInfoFromToken(origin, &info) == 0) {
	    continue;			/* Not an imported command. */
	}
	if (info.namespacePtr != (Tcl_Namespace *) sourceNsPtr) {
	    /*
	     * Original not in namespace we're matching. Check the first link
	     * in the import chain.
	     */

	    Command *cmdPtr = (Command *) token;
	    ImportedCmdData *dataPtr = (ImportedCmdData *)
		    cmdPtr->objClientData;
	    Tcl_Command firstToken = (Tcl_Command) dataPtr->realCmdPtr;

	    if (firstToken == origin) {
		continue;
	    }
	    Tcl_GetCommandInfoFromToken(firstToken, &info);
	    if (info.namespacePtr != (Tcl_Namespace *) sourceNsPtr) {
		continue;
	    }
	    origin = firstToken;
	}
	if (Tcl_StringMatch(Tcl_GetCommandName(NULL, origin), simplePattern)) {
	    Tcl_DeleteCommandFromToken(interp, token);
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained release source`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `NamespaceImportCmd`, lines 3813–3877. Retained reference `tcl9.0-NamespaceImportCmd-source`.

```text
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

```

tcl9.1 9.1.0, revision `retained release source`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `Tcl_Import`, lines 1652–1770. Retained reference `tcl9.1-Tcl_Import-source`.

```text
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
		importNsPtr, allowOverwrite != 0);
    }
    for (hPtr = Tcl_FirstHashEntry(&importNsPtr->cmdTable, &search);
	    (hPtr != NULL); hPtr = Tcl_NextHashEntry(&search)) {
	char *cmdName = (char *) Tcl_GetHashKey(&importNsPtr->cmdTable, hPtr);

	if (Tcl_StringMatch(cmdName, simplePattern) &&
		DoImport(interp, nsPtr, hPtr, cmdName, pattern, importNsPtr,
		allowOverwrite != 0) == TCL_ERROR) {
	    return TCL_ERROR;
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained release source`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `Tcl_ForgetImport`, lines 1937–2045. Retained reference `tcl9.1-Tcl_ForgetImport-source`.

```text
Tcl_ForgetImport(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Namespace *namespacePtr,/* Points to the namespace from which
				 * previously imported commands should be
				 * removed. NULL for current namespace. */
    const char *pattern)	/* String pattern indicating which imported
				 * commands to remove. */
{
    Namespace *nsPtr, *sourceNsPtr, *dummyPtr;
    const char *simplePattern;
    char *cmdName;
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
     * Parse the pattern into its namespace-qualification (if any) and the
     * simple pattern.
     */

    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &sourceNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if (sourceNsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"unknown namespace in namespace forget pattern \"%s\"",
		pattern));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE", pattern, (char *)NULL);
	return TCL_ERROR;
    }

    if (strcmp(pattern, simplePattern) == 0) {
	/*
	 * The pattern is simple. Delete any imported commands that match it.
	 */

	if (TclMatchIsTrivial(simplePattern)) {
	    hPtr = Tcl_FindHashEntry(&nsPtr->cmdTable, simplePattern);
	    if (hPtr != NULL) {
		Command *cmdPtr = (Command *) Tcl_GetHashValue(hPtr);

		if (cmdPtr && (cmdPtr->deleteProc == DeleteImportedCmd)) {
		    Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
		}
	    }
	    return TCL_OK;
	}
	for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search);
		(hPtr != NULL); hPtr = Tcl_NextHashEntry(&search)) {
	    Command *cmdPtr = (Command *) Tcl_GetHashValue(hPtr);

	    if (cmdPtr->deleteProc != DeleteImportedCmd) {
		continue;
	    }
	    cmdName = (char *) Tcl_GetHashKey(&nsPtr->cmdTable, hPtr);
	    if (Tcl_StringMatch(cmdName, simplePattern)) {
		Tcl_DeleteCommandFromToken(interp, (Tcl_Command) cmdPtr);
	    }
	}
	return TCL_OK;
    }

    /*
     * The pattern was namespace-qualified.
     */

    for (hPtr = Tcl_FirstHashEntry(&nsPtr->cmdTable, &search); (hPtr != NULL);
	    hPtr = Tcl_NextHashEntry(&search)) {
	Tcl_CmdInfo info;
	Tcl_Command token = (Tcl_Command) Tcl_GetHashValue(hPtr);
	Tcl_Command origin = TclGetOriginalCommand(token);

	if (Tcl_GetCommandInfoFromToken(origin, &info) == 0) {
	    continue;			/* Not an imported command. */
	}
	if (info.namespacePtr != (Tcl_Namespace *) sourceNsPtr) {
	    /*
	     * Original not in namespace we're matching. Check the first link
	     * in the import chain.
	     */

	    Command *cmdPtr = (Command *) token;
	    ImportedCmdData *dataPtr = (ImportedCmdData *)cmdPtr->objClientData2;
	    Tcl_Command firstToken = (Tcl_Command) dataPtr->realCmdPtr;

	    if (firstToken == origin) {
		continue;
	    }
	    Tcl_GetCommandInfoFromToken(firstToken, &info);
	    if (info.namespacePtr != (Tcl_Namespace *) sourceNsPtr) {
		continue;
	    }
	    origin = firstToken;
	}
	if (Tcl_StringMatch(Tcl_GetCommandName(NULL, origin), simplePattern)) {
	    Tcl_DeleteCommandFromToken(interp, token);
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained release source`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `NamespaceImportCmd`, lines 3786–3850. Retained reference `tcl9.1-NamespaceImportCmd-source`.

```text
NamespaceImportCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    bool allowOverwrite = false;
    const char *string, *pattern;
    Tcl_Size i, firstArg;
    int result;

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
	    allowOverwrite = true;
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

```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-namespace.c`, function `JimNamespaceCmd`, lines 193–328. Retained reference `jim-JimNamespaceCmd-source`.

```text
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

```

## Consumer bindings

- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `NamespaceTransition::import_preload_command`: Explicit selected metadata proposal for the global C helper; actual absence is separately resolved.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `OriginalNamespaceBinding::capture`: Independent initial-handler, argv, quiet-state and transfer closure before the real operation.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `OriginalNamespaceBinding::completed`: Exact actual current transfer and original handler/site/config correspondence.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `import_is_fresh_and_closed`: Current absent preload, closed source/export roster and fresh destinations without replacement cleanup.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `import_origins_are_live`: Retain a surviving direct owner before forgetting an import, without last-reference cleanup inference.
- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `CommandRegistry::native_registration_source_descriptor`: Select only a canonical stock private registration parent or independently authored direct descriptor under the actual dialect and realm; preserve descriptor provenance without a live handler or completion grant.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `command_binding::original_namespace_binding::tests::original_namespace_import_and_forget_keep_actual_origin_with_closed_prelude` (linked): With the preload independently removed, two distinct opaque imports retain actual origins and qualified forget removes only the moved first token.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `command_binding::original_namespace_binding::tests::original_namespace_import_completion_requires_closed_prelude_and_destinations` (linked): Present preload, occupied forced destination, self import and qualified export provide no complete root world.
- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `registry::tests::original_registration_source_descriptor_retains_parent_and_override_provenance` (linked): Original public namespace metadata and modern private export registration retain the same stock parent descriptor, while older/unregistered/unknown mappings and explicit authored override provenance remain separate.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/jim/JimNamespaceCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/jim/JimNamespaceCmd.c)
