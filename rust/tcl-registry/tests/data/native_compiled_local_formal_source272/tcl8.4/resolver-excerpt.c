    for (localPtr = framePtr->procPtr->firstLocalPtr;
	 localPtr != NULL;
	 localPtr = localPtr->nextPtr) {

	/*
	 * Check to see if this local is affected by namespace or
	 * interp resolvers.  The resolver to use is cached for the
	 * next invocation of the procedure.
	 */

	if (!(localPtr->flags & (VAR_ARGUMENT|VAR_TEMPORARY|VAR_RESOLVED))
		&& (nsPtr->compiledVarResProc || iPtr->resolverPtr)) {
	    resPtr = iPtr->resolverPtr;

	    if (nsPtr->compiledVarResProc) {
		result = (*nsPtr->compiledVarResProc)(nsPtr->interp,
			localPtr->name, localPtr->nameLength,
			(Tcl_Namespace *) nsPtr, &vinfo);
	    } else {
		result = TCL_CONTINUE;
	    }

	    while ((result == TCL_CONTINUE) && resPtr) {
		if (resPtr->compiledVarResProc) {
		    result = (*resPtr->compiledVarResProc)(nsPtr->interp,
			    localPtr->name, localPtr->nameLength,
			    (Tcl_Namespace *) nsPtr, &vinfo);
		}
		resPtr = resPtr->nextPtr;
	    }
	    if (result == TCL_OK) {
		localPtr->resolveInfo = vinfo;
		localPtr->flags |= VAR_RESOLVED;
	    }
	}

	/*
	 * Now invoke the resolvers to determine the exact variables that
	 * should be used.
	 */
