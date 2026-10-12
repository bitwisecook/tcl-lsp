Tcl_AddObjErrorInfo(interp, message, length)
    Tcl_Interp *interp;		/* Interpreter to which error information
				 * pertains. */
    CONST char *message;	/* Points to the first byte of an array of
				 * bytes of the message. */
    int length;			/* The number of bytes in the message.
				 * If < 0, then append all bytes up to a
				 * NULL byte. */
{
    register Interp *iPtr = (Interp *) interp;
    Tcl_Obj *objPtr;
    
    /*
     * If we are just starting to log an error, errorInfo is initialized
     * from the error message in the interpreter's result.
     */

    if (!(iPtr->flags & ERR_IN_PROGRESS)) { /* just starting to log error */
	iPtr->flags |= ERR_IN_PROGRESS;

	if (iPtr->result[0] == 0) {
	    Tcl_ObjSetVar2(interp, iPtr->execEnvPtr->errorInfo, NULL, 
	            iPtr->objResultPtr, TCL_GLOBAL_ONLY);
	} else {		/* use the string result */
	    objPtr = Tcl_NewStringObj(interp->result, -1);
	    Tcl_IncrRefCount(objPtr);
	    Tcl_ObjSetVar2(interp, iPtr->execEnvPtr->errorInfo, NULL, 
	            objPtr, TCL_GLOBAL_ONLY);
	    Tcl_DecrRefCount(objPtr);
	}

	/*
	 * If the errorCode variable wasn't set by the code that generated
	 * the error, set it to "NONE".
	 */

	if (!(iPtr->flags & ERROR_CODE_SET)) {
	    objPtr = Tcl_NewStringObj("NONE", -1);
	    Tcl_IncrRefCount(objPtr);
	    Tcl_ObjSetVar2(interp, iPtr->execEnvPtr->errorCode, NULL, 
	            objPtr, TCL_GLOBAL_ONLY);
	    Tcl_DecrRefCount(objPtr);
	}
    }

    /*
     * Now append "message" to the end of errorInfo.
     */

    if (length != 0) {
	objPtr = Tcl_NewStringObj(message, length);
	Tcl_IncrRefCount(objPtr);
	Tcl_ObjSetVar2(interp, iPtr->execEnvPtr->errorInfo, NULL, 
	        objPtr, (TCL_GLOBAL_ONLY | TCL_APPEND_VALUE));
	Tcl_DecrRefCount(objPtr); /* free msg object appended above */
    }
}
