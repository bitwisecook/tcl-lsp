# naming.namespace.original-byte-pattern-transfers

Kind: `implementation-contract`

## Problem statement

String export storage and UTF8-tail selection can collapse or discard original command patterns before import and forget select their actual token origins.

## Question

Do namespace pattern transfers retain selected original byte extents and use the canonical glob and token owners without recovering names from reporting text?

## Conclusion

Export alternatives retain NameBytes. Each actual pattern is selected from its retained original invocation operand, independently of logical transition advice. Leading clear selection and each pattern use the selected namespace purpose; import/export/forget matching delegates to NativeGlobProtocol. Imports retain their actual source tokens, and qualified forget joins the surviving source token even after movement. Unsupported or conflicting geometry withdraws rather than decoding a label.

## Scope

C Tcl source-model naming under Tcl 8.4, 8.5, 8.6, 9.0 and 9.1 selected policies, genuine original operands and independently retained namespace geometry. Opaque command/pattern units remain counted data under the shared selected native purposes. Original opaque namespace allocation belongs to the separate counted-allocation contract. Jim export is a selected no-op; its source-name import helper and unimplemented forget do not receive C token or completion authority. Supplied native entries, arbitrary preload helpers, replacement cleanup, unknown callbacks and physical compiler/cache/namespace authority are excluded.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No native execution of this Rust implementation question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No native execution of this Rust implementation question is claimed.

## Exact evidence

- `tcl8.4-Tcl_Export-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/Tcl_Export.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/Tcl_Export.c). SHA-256 `1ef0264e9f2140b123a26de23d2ba7b09b3278e4c671a37917b6f4069bc1b75a`. tcl8.4 retained Tcl_Export source; this is independent of Rust test or native invocation authority.
- `tcl8.4-Tcl_Import-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/Tcl_Import.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/Tcl_Import.c). SHA-256 `43a18400419ea1cc25a18b245d74ebd86337348652b72bcb18a648b2176d5079`. tcl8.4 retained Tcl_Import source; this is independent of Rust test or native invocation authority.
- `tcl8.4-Tcl_ForgetImport-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/Tcl_ForgetImport.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/Tcl_ForgetImport.c). SHA-256 `cdf4fc501f93d695978f1948b83163420b8495fe4bd60310dcb1a5e1c6d62beb`. tcl8.4 retained Tcl_ForgetImport source; this is independent of Rust test or native invocation authority.
- `tcl8.4-NamespaceExportCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/NamespaceExportCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/NamespaceExportCmd.c). SHA-256 `9c541d831ea43ef4ba2c37f20667a107b5a2ed3cd70bf555c16d1926bfe3bde5`. tcl8.4 retained NamespaceExportCmd source; this is independent of Rust test or native invocation authority.
- `tcl8.5-Tcl_Export-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/Tcl_Export.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/Tcl_Export.c). SHA-256 `5129be68867424f764783f3afee7e34f82333837eeff439f56594b0e69a8522b`. tcl8.5 retained Tcl_Export source; this is independent of Rust test or native invocation authority.
- `tcl8.5-Tcl_Import-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/Tcl_Import.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/Tcl_Import.c). SHA-256 `2fc6c9627aa0cdfd031caadd2a3255dd101d73cd9f419b03d0947c639c81b3f4`. tcl8.5 retained Tcl_Import source; this is independent of Rust test or native invocation authority.
- `tcl8.5-Tcl_ForgetImport-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/Tcl_ForgetImport.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/Tcl_ForgetImport.c). SHA-256 `833f6c7d8701b081d8fe56c780d8ba00babcc3ff0ab722f4472b77e858df220e`. tcl8.5 retained Tcl_ForgetImport source; this is independent of Rust test or native invocation authority.
- `tcl8.5-NamespaceExportCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/NamespaceExportCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/NamespaceExportCmd.c). SHA-256 `b8722c063d3bc30a50db02d29e98b1701b211a3fc1acdfc6b446f2096fa3db00`. tcl8.5 retained NamespaceExportCmd source; this is independent of Rust test or native invocation authority.
- `tcl8.6-Tcl_Export-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/Tcl_Export.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/Tcl_Export.c). SHA-256 `ae636aa95642db8fd71cfcfcc7c981878809280aeace9d139e5aff882775fdca`. tcl8.6 retained Tcl_Export source; this is independent of Rust test or native invocation authority.
- `tcl8.6-Tcl_Import-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/Tcl_Import.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/Tcl_Import.c). SHA-256 `3833d77e525a74dd85414d9f6896ce5763d01dc6bbf8dd45e727992f481538d9`. tcl8.6 retained Tcl_Import source; this is independent of Rust test or native invocation authority.
- `tcl8.6-Tcl_ForgetImport-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/Tcl_ForgetImport.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/Tcl_ForgetImport.c). SHA-256 `81f45437cb2e50407306616208b896466ea4de99111a8f421c4d06fbf6672cb6`. tcl8.6 retained Tcl_ForgetImport source; this is independent of Rust test or native invocation authority.
- `tcl8.6-NamespaceExportCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/NamespaceExportCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/NamespaceExportCmd.c). SHA-256 `91b0b058c25b41d06cef0bf75fc744f795ca28f4ec9e5efe0c7b1f791525aed3`. tcl8.6 retained NamespaceExportCmd source; this is independent of Rust test or native invocation authority.
- `tcl9.0-Tcl_Export-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/Tcl_Export.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/Tcl_Export.c). SHA-256 `0885125a66c98a77e4adf3ecfa583c65ea9e7fa3f64032f4999833ca56522960`. tcl9.0 retained Tcl_Export source; this is independent of Rust test or native invocation authority.
- `tcl9.0-Tcl_Import-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/Tcl_Import.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/Tcl_Import.c). SHA-256 `02c3008a3a4d21c10e74370f31f0df3ff6a2ab4c6b8dc3b99606d1f90aed670a`. tcl9.0 retained Tcl_Import source; this is independent of Rust test or native invocation authority.
- `tcl9.0-Tcl_ForgetImport-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/Tcl_ForgetImport.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/Tcl_ForgetImport.c). SHA-256 `536ff4f1c34a6a3583f728221650f9b40ca9401c0313ca0617b50cc084999e3e`. tcl9.0 retained Tcl_ForgetImport source; this is independent of Rust test or native invocation authority.
- `tcl9.0-NamespaceExportCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/NamespaceExportCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/NamespaceExportCmd.c). SHA-256 `5c9c5850b3955557e6e4142b49eefcee16ed0a9624dd586093a0f7709035791f`. tcl9.0 retained NamespaceExportCmd source; this is independent of Rust test or native invocation authority.
- `tcl9.1-Tcl_Export-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/Tcl_Export.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/Tcl_Export.c). SHA-256 `0885125a66c98a77e4adf3ecfa583c65ea9e7fa3f64032f4999833ca56522960`. tcl9.1 retained Tcl_Export source; this is independent of Rust test or native invocation authority.
- `tcl9.1-Tcl_Import-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/Tcl_Import.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/Tcl_Import.c). SHA-256 `1fdff5121de94b9f05dd08b947e2039d89d6daa02e435eb12c364d7e9d004532`. tcl9.1 retained Tcl_Import source; this is independent of Rust test or native invocation authority.
- `tcl9.1-Tcl_ForgetImport-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/Tcl_ForgetImport.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/Tcl_ForgetImport.c). SHA-256 `aa8db5fb3fb28787e9774bd108cb4521b07abe4f9728c6d343d863b90fcb72cf`. tcl9.1 retained Tcl_ForgetImport source; this is independent of Rust test or native invocation authority.
- `tcl9.1-NamespaceExportCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/NamespaceExportCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/NamespaceExportCmd.c). SHA-256 `b4bcd0ac8bb0294a307c3eaa34deeb2f76866de8c9d91e2972606765bcb10dd4`. tcl9.1 retained NamespaceExportCmd source; this is independent of Rust test or native invocation authority.
- `jim-JimNamespaceCmd-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/jim/JimNamespaceCmd.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/jim/JimNamespaceCmd.c). SHA-256 `68cf42ac5f1a1fadf45a6ef3275144f466cbfd548737a300cb791d25f30a07b5`. jim retained JimNamespaceCmd source; this is independent of Rust test or native invocation authority.

## Source inspection

tcl8.4 8.4.20, revision `retained release source`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `Tcl_Export`, lines 881–993. Full-source SHA-256 `cd23865ee8d4b0ce81874d519b24de5b84a16c116ed3593ead366946b0512cbb`; snippet SHA-256 `1ef0264e9f2140b123a26de23d2ba7b09b3278e4c671a37917b6f4069bc1b75a`; retained evidence `tcl8.4-Tcl_Export-source`.

```text
Tcl_Export(interp, namespacePtr, pattern, resetListFirst)
    Tcl_Interp *interp;		 /* Current interpreter. */
    Tcl_Namespace *namespacePtr; /* Points to the namespace from which 
				  * commands are to be exported. NULL for
                                  * the current namespace. */
    CONST char *pattern;         /* String pattern indicating which commands
                                  * to export. This pattern may not include
				  * any namespace qualifiers; only commands
				  * in the specified namespace may be
				  * exported. */
    int resetListFirst;		 /* If nonzero, resets the namespace's
				  * export list before appending. */
{
#define INIT_EXPORT_PATTERNS 5    
    Namespace *nsPtr, *exportNsPtr, *dummyPtr;
    Namespace *currNsPtr = (Namespace *) Tcl_GetCurrentNamespace(interp);
    CONST char *simplePattern;
    char *patternCpy;
    int neededElems, len, i;

    /*
     * If the specified namespace is NULL, use the current namespace.
     */

    if (namespacePtr == NULL) {
        nsPtr = (Namespace *) currNsPtr;
    } else {
        nsPtr = (Namespace *) namespacePtr;
    }

    /*
     * If resetListFirst is true (nonzero), clear the namespace's export
     * pattern list.
     */

    if (resetListFirst) {
	if (nsPtr->exportArrayPtr != NULL) {
	    for (i = 0;  i < nsPtr->numExportPatterns;  i++) {
		ckfree(nsPtr->exportArrayPtr[i]);
	    }
	    ckfree((char *) nsPtr->exportArrayPtr);
	    nsPtr->exportArrayPtr = NULL;
	    nsPtr->numExportPatterns = 0;
	    nsPtr->maxExportPatterns = 0;
	}
    }

    /*
     * Check that the pattern doesn't have namespace qualifiers.
     */

    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &exportNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if ((exportNsPtr != nsPtr) || (strcmp(pattern, simplePattern) != 0)) {
	Tcl_AppendStringsToObj(Tcl_GetObjResult(interp),
	        "invalid export pattern \"", pattern,
		"\": pattern can't specify a namespace",
		(char *) NULL);
	return TCL_ERROR;
    }

    /*
     * Make sure that we don't already have the pattern in the array
     */
    if (nsPtr->exportArrayPtr != NULL) {
	for (i = 0;  i < nsPtr->numExportPatterns;  i++) {
	    if (strcmp(pattern, nsPtr->exportArrayPtr[i]) == 0) {
		/*
		 * The pattern already exists in the list
		 */
		return TCL_OK;
	    }
	}
    }

    /*
     * Make sure there is room in the namespace's pattern array for the
     * new pattern.
     */

    neededElems = nsPtr->numExportPatterns + 1;
    if (nsPtr->exportArrayPtr == NULL) {
	nsPtr->exportArrayPtr = (char **)
	        ckalloc((unsigned) (INIT_EXPORT_PATTERNS * sizeof(char *)));
	nsPtr->numExportPatterns = 0;
	nsPtr->maxExportPatterns = INIT_EXPORT_PATTERNS;
    } else if (neededElems > nsPtr->maxExportPatterns) {
	int numNewElems = 2 * nsPtr->maxExportPatterns;
	size_t currBytes = nsPtr->numExportPatterns * sizeof(char *);
	size_t newBytes  = numNewElems * sizeof(char *);
	char **newPtr = (char **) ckalloc((unsigned) newBytes);

	memcpy((VOID *) newPtr, (VOID *) nsPtr->exportArrayPtr,
	        currBytes);
	ckfree((char *) nsPtr->exportArrayPtr);
	nsPtr->exportArrayPtr = (char **) newPtr;
	nsPtr->maxExportPatterns = numNewElems;
    }

    /*
     * Add the pattern to the namespace's array of export patterns.
     */

    len = strlen(pattern);
    patternCpy = (char *) ckalloc((unsigned) (len + 1));
    strcpy(patternCpy, pattern);
    
    nsPtr->exportArrayPtr[nsPtr->numExportPatterns] = patternCpy;
    nsPtr->numExportPatterns++;
    return TCL_OK;
#undef INIT_EXPORT_PATTERNS
}

```

tcl8.4 8.4.20, revision `retained release source`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `Tcl_Import`, lines 1079–1303. Full-source SHA-256 `cd23865ee8d4b0ce81874d519b24de5b84a16c116ed3593ead366946b0512cbb`; snippet SHA-256 `43a18400419ea1cc25a18b245d74ebd86337348652b72bcb18a648b2176d5079`; retained evidence `tcl8.4-Tcl_Import-source`.

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

tcl8.4 8.4.20, revision `retained release source`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `Tcl_ForgetImport`, lines 1334–1428. Full-source SHA-256 `cd23865ee8d4b0ce81874d519b24de5b84a16c116ed3593ead366946b0512cbb`; snippet SHA-256 `cdf4fc501f93d695978f1948b83163420b8495fe4bd60310dcb1a5e1c6d62beb`; retained evidence `tcl8.4-Tcl_ForgetImport-source`.

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

tcl8.4 8.4.20, revision `retained release source`, `tmp/tcl8.4.20/generic/tclNamesp.c`, function `NamespaceExportCmd`, lines 3124–3173. Full-source SHA-256 `cd23865ee8d4b0ce81874d519b24de5b84a16c116ed3593ead366946b0512cbb`; snippet SHA-256 `9c541d831ea43ef4ba2c37f20667a107b5a2ed3cd70bf555c16d1926bfe3bde5`; retained evidence `tcl8.4-NamespaceExportCmd-source`.

```text
NamespaceExportCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int firstArg, i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 2, objv,
	        "?-clear? ?pattern pattern...?");
        return TCL_ERROR;
    }

    /*
     * If no pattern arguments are given, and "-clear" isn't specified,
     * return the namespace's current export pattern list.
     */

    if (objc == 2) {
	Tcl_Obj *listPtr = Tcl_NewObj();

	(void) Tcl_AppendExportList(interp, NULL, listPtr);
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Process the optional "-clear" argument.
     */

    firstArg = 2;
    if (strcmp("-clear", Tcl_GetString(objv[firstArg])) == 0) {
	Tcl_Export(interp, NULL, "::", 1);
	Tcl_ResetResult(interp);
	firstArg++;
    }

    /*
     * Add each pattern to the namespace's export pattern list.
     */
    
    for (i = firstArg;  i < objc;  i++) {
	int result = Tcl_Export(interp, NULL, Tcl_GetString(objv[i]), 0);
        if (result != TCL_OK) {
            return result;
        }
    }
    return TCL_OK;
}

```

tcl8.5 8.5.19, revision `retained release source`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `Tcl_Export`, lines 1263–1374. Full-source SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`; snippet SHA-256 `5129be68867424f764783f3afee7e34f82333837eeff439f56594b0e69a8522b`; retained evidence `tcl8.5-Tcl_Export-source`.

```text
Tcl_Export(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Namespace *namespacePtr,/* Points to the namespace from which commands
				 * are to be exported. NULL for the current
				 * namespace. */
    const char *pattern,	/* String pattern indicating which commands to
				 * export. This pattern may not include any
				 * namespace qualifiers; only commands in the
				 * specified namespace may be exported. */
    int resetListFirst)		/* If nonzero, resets the namespace's export
				 * list before appending. */
{
#define INIT_EXPORT_PATTERNS 5
    Namespace *nsPtr, *exportNsPtr, *dummyPtr;
    Namespace *currNsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    const char *simplePattern;
    char *patternCpy;
    int neededElems, len, i;

    /*
     * If the specified namespace is NULL, use the current namespace.
     */

    if (namespacePtr == NULL) {
	nsPtr = (Namespace *) currNsPtr;
    } else {
	nsPtr = (Namespace *) namespacePtr;
    }

    /*
     * If resetListFirst is true (nonzero), clear the namespace's export
     * pattern list.
     */

    if (resetListFirst) {
	if (nsPtr->exportArrayPtr != NULL) {
	    for (i = 0;  i < nsPtr->numExportPatterns;  i++) {
		ckfree(nsPtr->exportArrayPtr[i]);
	    }
	    ckfree((char *) nsPtr->exportArrayPtr);
	    nsPtr->exportArrayPtr = NULL;
	    TclInvalidateNsCmdLookup(nsPtr);
	    nsPtr->numExportPatterns = 0;
	    nsPtr->maxExportPatterns = 0;
	}
    }

    /*
     * Check that the pattern doesn't have namespace qualifiers.
     */

    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &exportNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if ((exportNsPtr != nsPtr) || (strcmp(pattern, simplePattern) != 0)) {
	Tcl_AppendResult(interp, "invalid export pattern \"", pattern,
		"\": pattern can't specify a namespace", NULL);
	return TCL_ERROR;
    }

    /*
     * Make sure that we don't already have the pattern in the array
     */

    if (nsPtr->exportArrayPtr != NULL) {
	for (i = 0;  i < nsPtr->numExportPatterns;  i++) {
	    if (strcmp(pattern, nsPtr->exportArrayPtr[i]) == 0) {
		/*
		 * The pattern already exists in the list.
		 */

		return TCL_OK;
	    }
	}
    }

    /*
     * Make sure there is room in the namespace's pattern array for the new
     * pattern.
     */

    neededElems = nsPtr->numExportPatterns + 1;
    if (neededElems > nsPtr->maxExportPatterns) {
	nsPtr->maxExportPatterns = nsPtr->maxExportPatterns ?
		2 * nsPtr->maxExportPatterns : INIT_EXPORT_PATTERNS;
	nsPtr->exportArrayPtr = (char **)
		ckrealloc((char *) nsPtr->exportArrayPtr,
		sizeof(char *) * nsPtr->maxExportPatterns);
    }

    /*
     * Add the pattern to the namespace's array of export patterns.
     */

    len = strlen(pattern);
    patternCpy = ckalloc((unsigned) (len + 1));
    memcpy(patternCpy, pattern, (unsigned) len + 1);

    nsPtr->exportArrayPtr[nsPtr->numExportPatterns] = patternCpy;
    nsPtr->numExportPatterns++;

    /*
     * The list of commands actually exported from the namespace might have
     * changed (probably will have!) However, we do not need to recompute this
     * just yet; next time we need the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(nsPtr);

    return TCL_OK;
#undef INIT_EXPORT_PATTERNS
}

```

tcl8.5 8.5.19, revision `retained release source`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `Tcl_Import`, lines 1460–1574. Full-source SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`; snippet SHA-256 `2fc6c9627aa0cdfd031caadd2a3255dd101d73cd9f419b03d0947c639c81b3f4`; retained evidence `tcl8.5-Tcl_Import-source`.

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

tcl8.5 8.5.19, revision `retained release source`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `Tcl_ForgetImport`, lines 1736–1844. Full-source SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`; snippet SHA-256 `833f6c7d8701b081d8fe56c780d8ba00babcc3ff0ab722f4472b77e858df220e`; retained evidence `tcl8.5-Tcl_ForgetImport-source`.

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

tcl8.5 8.5.19, revision `retained release source`, `tmp/tcl8.5.19/generic/tclNamesp.c`, function `NamespaceExportCmd`, lines 3429–3477. Full-source SHA-256 `60f944eb9ab669e183a546297c5aa6c8b62af0f79cdab115c74d37497c1fd83f`; snippet SHA-256 `b8722c063d3bc30a50db02d29e98b1701b211a3fc1acdfc6b446f2096fa3db00`; retained evidence `tcl8.5-NamespaceExportCmd-source`.

```text
NamespaceExportCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int firstArg, i;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 2, objv, "?-clear? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * If no pattern arguments are given, and "-clear" isn't specified, return
     * the namespace's current export pattern list.
     */

    if (objc == 2) {
	Tcl_Obj *listPtr = Tcl_NewObj();

	(void) Tcl_AppendExportList(interp, NULL, listPtr);
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Process the optional "-clear" argument.
     */

    firstArg = 2;
    if (strcmp("-clear", Tcl_GetString(objv[firstArg])) == 0) {
	Tcl_Export(interp, NULL, "::", 1);
	Tcl_ResetResult(interp);
	firstArg++;
    }

    /*
     * Add each pattern to the namespace's export pattern list.
     */

    for (i = firstArg;  i < objc;  i++) {
	int result = Tcl_Export(interp, NULL, Tcl_GetString(objv[i]), 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

```

tcl8.6 8.6.18, revision `retained release source`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `Tcl_Export`, lines 1303–1414. Full-source SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`; snippet SHA-256 `ae636aa95642db8fd71cfcfcc7c981878809280aeace9d139e5aff882775fdca`; retained evidence `tcl8.6-Tcl_Export-source`.

```text
Tcl_Export(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Namespace *namespacePtr,/* Points to the namespace from which commands
				 * are to be exported. NULL for the current
				 * namespace. */
    const char *pattern,	/* String pattern indicating which commands to
				 * export. This pattern may not include any
				 * namespace qualifiers; only commands in the
				 * specified namespace may be exported. */
    int resetListFirst)		/* If nonzero, resets the namespace's export
				 * list before appending. */
{
#define INIT_EXPORT_PATTERNS 5
    Namespace *nsPtr, *exportNsPtr, *dummyPtr;
    Namespace *currNsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    const char *simplePattern;
    char *patternCpy;
    int neededElems, len, i;

    /*
     * If the specified namespace is NULL, use the current namespace.
     */

    if (namespacePtr == NULL) {
	nsPtr = (Namespace *) currNsPtr;
    } else {
	nsPtr = (Namespace *) namespacePtr;
    }

    /*
     * If resetListFirst is true (nonzero), clear the namespace's export
     * pattern list.
     */

    if (resetListFirst) {
	if (nsPtr->exportArrayPtr != NULL) {
	    for (i = 0;  i < nsPtr->numExportPatterns;  i++) {
		ckfree(nsPtr->exportArrayPtr[i]);
	    }
	    ckfree(nsPtr->exportArrayPtr);
	    nsPtr->exportArrayPtr = NULL;
	    TclInvalidateNsCmdLookup(nsPtr);
	    nsPtr->numExportPatterns = 0;
	    nsPtr->maxExportPatterns = 0;
	}
    }

    /*
     * Check that the pattern doesn't have namespace qualifiers.
     */

    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &exportNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if ((exportNsPtr != nsPtr) || (strcmp(pattern, simplePattern) != 0)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("invalid export pattern"
		" \"%s\": pattern can't specify a namespace", pattern));
	Tcl_SetErrorCode(interp, "TCL", "EXPORT", "INVALID", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Make sure that we don't already have the pattern in the array
     */

    if (nsPtr->exportArrayPtr != NULL) {
	for (i = 0;  i < nsPtr->numExportPatterns;  i++) {
	    if (strcmp(pattern, nsPtr->exportArrayPtr[i]) == 0) {
		/*
		 * The pattern already exists in the list.
		 */

		return TCL_OK;
	    }
	}
    }

    /*
     * Make sure there is room in the namespace's pattern array for the new
     * pattern.
     */

    neededElems = nsPtr->numExportPatterns + 1;
    if (neededElems > nsPtr->maxExportPatterns) {
	nsPtr->maxExportPatterns = nsPtr->maxExportPatterns ?
		2 * nsPtr->maxExportPatterns : INIT_EXPORT_PATTERNS;
	nsPtr->exportArrayPtr = (char **)ckrealloc(nsPtr->exportArrayPtr,
		sizeof(char *) * nsPtr->maxExportPatterns);
    }

    /*
     * Add the pattern to the namespace's array of export patterns.
     */

    len = strlen(pattern);
    patternCpy = (char *)ckalloc(len + 1);
    memcpy(patternCpy, pattern, len + 1);

    nsPtr->exportArrayPtr[nsPtr->numExportPatterns] = patternCpy;
    nsPtr->numExportPatterns++;

    /*
     * The list of commands actually exported from the namespace might have
     * changed (probably will have!) However, we do not need to recompute this
     * just yet; next time we need the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(nsPtr);

    return TCL_OK;
#undef INIT_EXPORT_PATTERNS
}

```

tcl8.6 8.6.18, revision `retained release source`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `Tcl_Import`, lines 1502–1620. Full-source SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`; snippet SHA-256 `3833d77e525a74dd85414d9f6896ce5763d01dc6bbf8dd45e727992f481538d9`; retained evidence `tcl8.6-Tcl_Import-source`.

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

tcl8.6 8.6.18, revision `retained release source`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `Tcl_ForgetImport`, lines 1786–1894. Full-source SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`; snippet SHA-256 `81f45437cb2e50407306616208b896466ea4de99111a8f421c4d06fbf6672cb6`; retained evidence `tcl8.6-Tcl_ForgetImport-source`.

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

tcl8.6 8.6.18, revision `retained release source`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `NamespaceExportCmd`, lines 3460–3509. Full-source SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`; snippet SHA-256 `91b0b058c25b41d06cef0bf75fc744f795ca28f4ec9e5efe0c7b1f791525aed3`; retained evidence `tcl8.6-NamespaceExportCmd-source`.

```text
NamespaceExportCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int firstArg, i;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-clear? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * If no pattern arguments are given, and "-clear" isn't specified, return
     * the namespace's current export pattern list.
     */

    if (objc == 1) {
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	(void)Tcl_AppendExportList(interp, NULL, listPtr);
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Process the optional "-clear" argument.
     */

    firstArg = 1;
    if (strcmp("-clear", TclGetString(objv[firstArg])) == 0) {
	Tcl_Export(interp, NULL, "::", 1);
	Tcl_ResetResult(interp);
	firstArg++;
    }

    /*
     * Add each pattern to the namespace's export pattern list.
     */

    for (i = firstArg;  i < objc;  i++) {
	int result = Tcl_Export(interp, NULL, TclGetString(objv[i]), 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `retained release source`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `Tcl_Export`, lines 1454–1565. Full-source SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`; snippet SHA-256 `0885125a66c98a77e4adf3ecfa583c65ea9e7fa3f64032f4999833ca56522960`; retained evidence `tcl9.0-Tcl_Export-source`.

```text
Tcl_Export(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Namespace *namespacePtr,/* Points to the namespace from which commands
				 * are to be exported. NULL for the current
				 * namespace. */
    const char *pattern,	/* String pattern indicating which commands to
				 * export. This pattern may not include any
				 * namespace qualifiers; only commands in the
				 * specified namespace may be exported. */
    int resetListFirst)		/* If nonzero, resets the namespace's export
				 * list before appending. */
{
#define INIT_EXPORT_PATTERNS 5
    Namespace *nsPtr, *exportNsPtr, *dummyPtr;
    Namespace *currNsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    const char *simplePattern;
    char *patternCpy;
    Tcl_Size neededElems, len, i;

    /*
     * If the specified namespace is NULL, use the current namespace.
     */

    if (namespacePtr == NULL) {
	nsPtr = (Namespace *) currNsPtr;
    } else {
	nsPtr = (Namespace *) namespacePtr;
    }

    /*
     * If resetListFirst is true (nonzero), clear the namespace's export
     * pattern list.
     */

    if (resetListFirst) {
	if (nsPtr->exportArrayPtr != NULL) {
	    for (i = 0;  i < nsPtr->numExportPatterns;  i++) {
		Tcl_Free(nsPtr->exportArrayPtr[i]);
	    }
	    Tcl_Free(nsPtr->exportArrayPtr);
	    nsPtr->exportArrayPtr = NULL;
	    TclInvalidateNsCmdLookup(nsPtr);
	    nsPtr->numExportPatterns = 0;
	    nsPtr->maxExportPatterns = 0;
	}
    }

    /*
     * Check that the pattern doesn't have namespace qualifiers.
     */

    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &exportNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if ((exportNsPtr != nsPtr) || (strcmp(pattern, simplePattern) != 0)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("invalid export pattern"
		" \"%s\": pattern can't specify a namespace", pattern));
	Tcl_SetErrorCode(interp, "TCL", "EXPORT", "INVALID", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Make sure that we don't already have the pattern in the array
     */

    if (nsPtr->exportArrayPtr != NULL) {
	for (i = 0;  i < nsPtr->numExportPatterns;  i++) {
	    if (strcmp(pattern, nsPtr->exportArrayPtr[i]) == 0) {
		/*
		 * The pattern already exists in the list.
		 */

		return TCL_OK;
	    }
	}
    }

    /*
     * Make sure there is room in the namespace's pattern array for the new
     * pattern.
     */

    neededElems = nsPtr->numExportPatterns + 1;
    if (neededElems > nsPtr->maxExportPatterns) {
	nsPtr->maxExportPatterns = nsPtr->maxExportPatterns ?
		2 * nsPtr->maxExportPatterns : INIT_EXPORT_PATTERNS;
	nsPtr->exportArrayPtr = (char **) Tcl_Realloc(nsPtr->exportArrayPtr,
		sizeof(char *) * nsPtr->maxExportPatterns);
    }

    /*
     * Add the pattern to the namespace's array of export patterns.
     */

    len = strlen(pattern);
    patternCpy = (char *) Tcl_Alloc(len + 1);
    memcpy(patternCpy, pattern, len + 1);

    nsPtr->exportArrayPtr[nsPtr->numExportPatterns] = patternCpy;
    nsPtr->numExportPatterns++;

    /*
     * The list of commands actually exported from the namespace might have
     * changed (probably will have!) However, we do not need to recompute this
     * just yet; next time we need the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(nsPtr);

    return TCL_OK;
#undef INIT_EXPORT_PATTERNS
}

```

tcl9.0 9.0.4, revision `retained release source`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `Tcl_Import`, lines 1653–1771. Full-source SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`; snippet SHA-256 `02c3008a3a4d21c10e74370f31f0df3ff6a2ab4c6b8dc3b99606d1f90aed670a`; retained evidence `tcl9.0-Tcl_Import-source`.

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

tcl9.0 9.0.4, revision `retained release source`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `Tcl_ForgetImport`, lines 1939–2048. Full-source SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`; snippet SHA-256 `536ff4f1c34a6a3583f728221650f9b40ca9401c0313ca0617b50cc084999e3e`; retained evidence `tcl9.0-Tcl_ForgetImport-source`.

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

tcl9.0 9.0.4, revision `retained release source`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `NamespaceExportCmd`, lines 3666–3715. Full-source SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`; snippet SHA-256 `5c9c5850b3955557e6e4142b49eefcee16ed0a9624dd586093a0f7709035791f`; retained evidence `tcl9.0-NamespaceExportCmd-source`.

```text
NamespaceExportCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int firstArg, i;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-clear? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * If no pattern arguments are given, and "-clear" isn't specified, return
     * the namespace's current export pattern list.
     */

    if (objc == 1) {
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	(void) Tcl_AppendExportList(interp, NULL, listPtr);
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Process the optional "-clear" argument.
     */

    firstArg = 1;
    if (strcmp("-clear", TclGetString(objv[firstArg])) == 0) {
	Tcl_Export(interp, NULL, "::", 1);
	Tcl_ResetResult(interp);
	firstArg++;
    }

    /*
     * Add each pattern to the namespace's export pattern list.
     */

    for (i = firstArg;  i < objc;  i++) {
	int result = Tcl_Export(interp, NULL, TclGetString(objv[i]), 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

```

tcl9.1 9.1.0, revision `retained release source`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `Tcl_Export`, lines 1453–1564. Full-source SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`; snippet SHA-256 `0885125a66c98a77e4adf3ecfa583c65ea9e7fa3f64032f4999833ca56522960`; retained evidence `tcl9.1-Tcl_Export-source`.

```text
Tcl_Export(
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Namespace *namespacePtr,/* Points to the namespace from which commands
				 * are to be exported. NULL for the current
				 * namespace. */
    const char *pattern,	/* String pattern indicating which commands to
				 * export. This pattern may not include any
				 * namespace qualifiers; only commands in the
				 * specified namespace may be exported. */
    int resetListFirst)		/* If nonzero, resets the namespace's export
				 * list before appending. */
{
#define INIT_EXPORT_PATTERNS 5
    Namespace *nsPtr, *exportNsPtr, *dummyPtr;
    Namespace *currNsPtr = (Namespace *) TclGetCurrentNamespace(interp);
    const char *simplePattern;
    char *patternCpy;
    Tcl_Size neededElems, len, i;

    /*
     * If the specified namespace is NULL, use the current namespace.
     */

    if (namespacePtr == NULL) {
	nsPtr = (Namespace *) currNsPtr;
    } else {
	nsPtr = (Namespace *) namespacePtr;
    }

    /*
     * If resetListFirst is true (nonzero), clear the namespace's export
     * pattern list.
     */

    if (resetListFirst) {
	if (nsPtr->exportArrayPtr != NULL) {
	    for (i = 0;  i < nsPtr->numExportPatterns;  i++) {
		Tcl_Free(nsPtr->exportArrayPtr[i]);
	    }
	    Tcl_Free(nsPtr->exportArrayPtr);
	    nsPtr->exportArrayPtr = NULL;
	    TclInvalidateNsCmdLookup(nsPtr);
	    nsPtr->numExportPatterns = 0;
	    nsPtr->maxExportPatterns = 0;
	}
    }

    /*
     * Check that the pattern doesn't have namespace qualifiers.
     */

    TclGetNamespaceForQualName(interp, pattern, nsPtr, TCL_NAMESPACE_ONLY,
	    &exportNsPtr, &dummyPtr, &dummyPtr, &simplePattern);

    if ((exportNsPtr != nsPtr) || (strcmp(pattern, simplePattern) != 0)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("invalid export pattern"
		" \"%s\": pattern can't specify a namespace", pattern));
	Tcl_SetErrorCode(interp, "TCL", "EXPORT", "INVALID", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Make sure that we don't already have the pattern in the array
     */

    if (nsPtr->exportArrayPtr != NULL) {
	for (i = 0;  i < nsPtr->numExportPatterns;  i++) {
	    if (strcmp(pattern, nsPtr->exportArrayPtr[i]) == 0) {
		/*
		 * The pattern already exists in the list.
		 */

		return TCL_OK;
	    }
	}
    }

    /*
     * Make sure there is room in the namespace's pattern array for the new
     * pattern.
     */

    neededElems = nsPtr->numExportPatterns + 1;
    if (neededElems > nsPtr->maxExportPatterns) {
	nsPtr->maxExportPatterns = nsPtr->maxExportPatterns ?
		2 * nsPtr->maxExportPatterns : INIT_EXPORT_PATTERNS;
	nsPtr->exportArrayPtr = (char **) Tcl_Realloc(nsPtr->exportArrayPtr,
		sizeof(char *) * nsPtr->maxExportPatterns);
    }

    /*
     * Add the pattern to the namespace's array of export patterns.
     */

    len = strlen(pattern);
    patternCpy = (char *) Tcl_Alloc(len + 1);
    memcpy(patternCpy, pattern, len + 1);

    nsPtr->exportArrayPtr[nsPtr->numExportPatterns] = patternCpy;
    nsPtr->numExportPatterns++;

    /*
     * The list of commands actually exported from the namespace might have
     * changed (probably will have!) However, we do not need to recompute this
     * just yet; next time we need the info will be soon enough.
     */

    TclInvalidateNsCmdLookup(nsPtr);

    return TCL_OK;
#undef INIT_EXPORT_PATTERNS
}

```

tcl9.1 9.1.0, revision `retained release source`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `Tcl_Import`, lines 1652–1770. Full-source SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`; snippet SHA-256 `1fdff5121de94b9f05dd08b947e2039d89d6daa02e435eb12c364d7e9d004532`; retained evidence `tcl9.1-Tcl_Import-source`.

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

tcl9.1 9.1.0, revision `retained release source`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `Tcl_ForgetImport`, lines 1937–2045. Full-source SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`; snippet SHA-256 `aa8db5fb3fb28787e9774bd108cb4521b07abe4f9728c6d343d863b90fcb72cf`; retained evidence `tcl9.1-Tcl_ForgetImport-source`.

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

tcl9.1 9.1.0, revision `retained release source`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `NamespaceExportCmd`, lines 3638–3687. Full-source SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`; snippet SHA-256 `b4bcd0ac8bb0294a307c3eaa34deeb2f76866de8c9d91e2972606765bcb10dd4`; retained evidence `tcl9.1-NamespaceExportCmd-source`.

```text
NamespaceExportCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size firstArg, i;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-clear? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * If no pattern arguments are given, and "-clear" isn't specified, return
     * the namespace's current export pattern list.
     */

    if (objc == 1) {
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	(void) Tcl_AppendExportList(interp, NULL, listPtr);
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Process the optional "-clear" argument.
     */

    firstArg = 1;
    if (strcmp("-clear", TclGetString(objv[firstArg])) == 0) {
	Tcl_Export(interp, NULL, "::", 1);
	Tcl_ResetResult(interp);
	firstArg++;
    }

    /*
     * Add each pattern to the namespace's export pattern list.
     */

    for (i = firstArg;  i < objc;  i++) {
	int result = Tcl_Export(interp, NULL, TclGetString(objv[i]), 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-namespace.c`, function `JimNamespaceCmd`, lines 193–328. Full-source SHA-256 `22bb6fb5fcea8ce8ec7e3eecbfe5ebd5ee290b9b725101ff4e4a1c0e9abed7f3`; snippet SHA-256 `68cf42ac5f1a1fadf45a6ef3275144f466cbfd548737a300cb791d25f30a07b5`; retained evidence `jim-JimNamespaceCmd-source`.

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

- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `NamespaceTransition::export_pattern_byte_operands`: Select the leading control with the shared independent namespace pattern purpose.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `apply_exports`: Counted selected export pattern storage with qualification checks and ordered clear.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `apply_import`: Actual source cells, export filters and imported token links selected through the canonical byte owners.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `apply_forget`: Local or original-token pattern matching without reporting-name reconstruction.
- [rust/tcl-syntax/src/native_glob.rs](../../../../rust/tcl-syntax/src/native_glob.rs), `NativeGlobProtocol::match_name_pattern`: Independent import/export/forget matching under the selected naming recipe.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `state_transition::tests::original_namespace_export_control_uses_the_selected_cstring_extent` (linked): Only the leading selected C CString extent controls clear; modified zero stays data and Jim receives no C control recipe.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `command_binding::original_namespace_binding::tests::original_namespace_export_storage_keeps_exact_opaque_units` (linked): A source export preserves the exact D800 pattern while its D801 sibling remains distinct under all five C source policies.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `state_transition::tests::original_namespace_patterns_close_only_their_represented_byte_ordinals` (linked): The selected export/import/forget transition retains exact byte pattern ordinals across C5; those represented identities avoid duplicate dynamic widening while unknown operands still widen. Import derives its force field from the separately selected pattern CString extent.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact named Rust selectors are implementation coverage obligations without an attached execution receipt. The independently retained source controls and source excerpts do not grant Rust handler selection, completion or physical native authority.
