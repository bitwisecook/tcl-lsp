# naming.namespace.original-qualified-forget-publications

Kind: `implementation-contract`

## Problem statement

Qualified forget patterns require the same live original site, implementation allocation and selected policy in both binding and publication inventories. Withdrawing a publication independently of that exact match can lose closed-world and current handler correspondence.

## Question

Does the publication ledger remove only imports whose independently retained current origin matches a qualified forget pattern, including source and alias moves?

## Conclusion

Qualified forgetting joins each imported publication to the unique surviving direct publication with the same original site, implementation allocation and naming policy. It matches the current origin namespace and simple bytes through the selected ForgetOriginFilter. Unqualified patterns continue to match current own import names through ForgetOwnSearch. Missing or ambiguous origins withdraw; no reporting name supplies a slot.

## Scope

Readonly modeled C Tcl source publication transfer under Tcl 8.4, 8.5, 8.6, 9.0 and 9.1 naming policies. Positive source fixtures explicitly select the Direct/ScriptCode authoring driver and independently move the global auto_import preload before imports. An Unknown compilation-mode control still has no completed root world. The real canonical binding transfer and independent intrinsic completion receipt remain required; publication matching does not grant execution, Normal completion, namespace existence, native token/header or editor authority. Intrinsic completion remains scoped to direct surviving source owners; intermediate import-chain fallback and Jim qualified forgetting remain unavailable.

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

- `tcl8.4-Tcl_ForgetImport-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/Tcl_ForgetImport.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.4/Tcl_ForgetImport.c). SHA-256 `cdf4fc501f93d695978f1948b83163420b8495fe4bd60310dcb1a5e1c6d62beb`. tcl8.4 retained Tcl_ForgetImport source; this is independent of Rust test or native invocation authority.
- `tcl8.5-Tcl_ForgetImport-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/Tcl_ForgetImport.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.5/Tcl_ForgetImport.c). SHA-256 `833f6c7d8701b081d8fe56c780d8ba00babcc3ff0ab722f4472b77e858df220e`. tcl8.5 retained Tcl_ForgetImport source; this is independent of Rust test or native invocation authority.
- `tcl8.6-Tcl_ForgetImport-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/Tcl_ForgetImport.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl8.6/Tcl_ForgetImport.c). SHA-256 `81f45437cb2e50407306616208b896466ea4de99111a8f421c4d06fbf6672cb6`. tcl8.6 retained Tcl_ForgetImport source; this is independent of Rust test or native invocation authority.
- `tcl9.0-Tcl_ForgetImport-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/Tcl_ForgetImport.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.0/Tcl_ForgetImport.c). SHA-256 `536ff4f1c34a6a3583f728221650f9b40ca9401c0313ca0617b50cc084999e3e`. tcl9.0 retained Tcl_ForgetImport source; this is independent of Rust test or native invocation authority.
- `tcl9.1-Tcl_ForgetImport-source` (source-anchor): [rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/Tcl_ForgetImport.c](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/source-anchors/tcl9.1/Tcl_ForgetImport.c). SHA-256 `aa8db5fb3fb28787e9774bd108cb4521b07abe4f9728c6d343d863b90fcb72cf`. tcl9.1 retained Tcl_ForgetImport source; this is independent of Rust test or native invocation authority.

## Source inspection

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


## Consumer bindings

- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `OriginalSourceCommandWorld::forget`: Join retained origin site and implementation allocation to unique current source publication before selected qualified glob matching.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `OriginalNamespaceBinding::completed`: Independent actual canonical binding, handler, source, argv and namespace transfer checks remain required.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `command_binding::source_command_world::tests::original_qualified_forget_joins_current_origin_after_source_and_alias_moves` (linked): A moved source origin and moved own import keep exact opaque identity; qualified p* forgetting removes only the import whose current source name still matches, and preserves both direct sources.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `command_binding::source_command_world::tests::original_namespace_world_requires_the_explicit_source_compilation_mode` (linked): An unchanged namespace declaration under Unknown compiler mode supplies no complete source world.

A named test is a coverage binding, not a claim that it executed.

## Replay

The listed Rust selectors are implementation obligations without an attached execution receipt. Independently captured namespace source controls and retained Tcl_ForgetImport sources remain separate native questions.
