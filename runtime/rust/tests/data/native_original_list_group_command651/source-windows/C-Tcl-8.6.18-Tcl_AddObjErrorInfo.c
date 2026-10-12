Tcl_AddObjErrorInfo(
    Tcl_Interp *interp,		/* Interpreter to which error information
				 * pertains. */
    const char *message,	/* Points to the first byte of an array of
				 * bytes of the message. */
    int length)			/* The number of bytes in the message. If < 0,
				 * then append all bytes up to a NULL byte. */
{
    Interp *iPtr = (Interp *) interp;

    /*
     * If we are just starting to log an error, errorInfo is initialized from
     * the error message in the interpreter's result.
     */

    iPtr->flags |= ERR_LEGACY_COPY;
    if (iPtr->errorInfo == NULL) {
	if (iPtr->result[0] != 0) {
	    /*
	     * The interp's string result is set, apparently by some extension
	     * making a deprecated direct write to it. That extension may
	     * expect interp->result to continue to be set, so we'll take
	     * special pains to avoid clearing it, until we drop support for
	     * interp->result completely.
	     */

	    iPtr->errorInfo = Tcl_NewStringObj(iPtr->result, -1);
	} else {
	    iPtr->errorInfo = iPtr->objResultPtr;
	}
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
}
