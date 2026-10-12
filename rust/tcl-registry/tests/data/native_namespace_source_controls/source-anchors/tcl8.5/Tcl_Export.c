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
