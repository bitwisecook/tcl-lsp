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
