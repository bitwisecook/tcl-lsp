	    if (localPtr->resolveInfo->deleteProc) {
		localPtr->resolveInfo->deleteProc(localPtr->resolveInfo);
	    } else {
		ckfree(localPtr->resolveInfo);
	    }
	    localPtr->resolveInfo = NULL;
	}
	localPtr->flags &= ~VAR_RESOLVED;

	if (haveResolvers &&
		!(localPtr->flags & (VAR_ARGUMENT|VAR_TEMPORARY))) {
	    ResolverScheme *resPtr = iPtr->resolverPtr;
	    Tcl_ResolvedVarInfo *vinfo;
	    int result;

	    if (nsPtr->compiledVarResProc) {
		result = nsPtr->compiledVarResProc(nsPtr->interp,
			localPtr->name, localPtr->nameLength,
			(Tcl_Namespace *) nsPtr, &vinfo);
	    } else {
		result = TCL_CONTINUE;
	    }

	    while ((result == TCL_CONTINUE) && resPtr) {
		if (resPtr->compiledVarResProc) {
		    result = resPtr->compiledVarResProc(nsPtr->interp,
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
    }
    localPtr = firstLocalPtr;
    codePtr->flags &= ~TCL_BYTECODE_RESOLVE_VARS;

