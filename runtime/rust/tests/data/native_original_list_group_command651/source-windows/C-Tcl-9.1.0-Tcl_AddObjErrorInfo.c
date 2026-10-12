Tcl_AppendObjToErrorInfo(
    Tcl_Interp *interp,		/* Interpreter to which error information
				 * pertains. */
    Tcl_Obj *objPtr)		/* Message to record. */
{
    Tcl_Size length;
    const char *message = TclGetStringFromObj(objPtr, &length);
    Interp *iPtr = (Interp *) interp;

    Tcl_IncrRefCount(objPtr);

    /*
     * If we are just starting to log an error, errorInfo is initialized from
     * the error message in the interpreter's result.
     */

    iPtr->flags |= ERR_LEGACY_COPY;
    if (iPtr->errorInfo == NULL) {
	iPtr->errorInfo = iPtr->objResultPtr;
	Tcl_IncrRefCount(iPtr->errorInfo);
	if (!iPtr->errorCode) {
	    Tcl_SetErrorCode(interp, "NONE", (char *)NULL);
	}
    }

    /*
     * Now append "message" to the end of errorInfo.
     */

    if (length != 0) {
	if (Tcl_IsShared(iPtr->errorInfo)) {
	    Tcl_DecrRefCount(iPtr->errorInfo);
	    iPtr->errorInfo = Tcl_DuplicateObj(iPtr->errorInfo);
	    Tcl_IncrRefCount(iPtr->errorInfo);
	}
	Tcl_AppendToObj(iPtr->errorInfo, message, length);
    }
    Tcl_DecrRefCount(objPtr);
}
