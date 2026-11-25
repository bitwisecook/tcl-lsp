Tcl_ResetResult(
    Tcl_Interp *interp)		/* Interpreter for which to clear result. */
{
    Interp *iPtr = (Interp *) interp;

    ResetObjResult(iPtr);
    if (iPtr->freeProc != NULL) {
	if (iPtr->freeProc == TCL_DYNAMIC) {
	    ckfree(iPtr->result);
	} else {
	    iPtr->freeProc(iPtr->result);
	}
	iPtr->freeProc = 0;
    }
    iPtr->result = iPtr->resultSpace;
    iPtr->resultSpace[0] = 0;
    if (iPtr->errorCode) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->ecVar, NULL,
		    iPtr->errorCode, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorCode);
	iPtr->errorCode = NULL;
    }
    if (iPtr->errorInfo) {
	/* Legacy support */
	if (iPtr->flags & ERR_LEGACY_COPY) {
	    Tcl_ObjSetVar2(interp, iPtr->eiVar, NULL,
		    iPtr->errorInfo, TCL_GLOBAL_ONLY);
	}
	Tcl_DecrRefCount(iPtr->errorInfo);
	iPtr->errorInfo = NULL;
    }
    iPtr->resetErrorStack = 1;
    iPtr->returnLevel = 1;
    iPtr->returnCode = TCL_OK;
    if (iPtr->returnOpts) {
	Tcl_DecrRefCount(iPtr->returnOpts);
	iPtr->returnOpts = NULL;
    }
    iPtr->flags &= ~(ERR_ALREADY_LOGGED | ERR_LEGACY_COPY);
}
