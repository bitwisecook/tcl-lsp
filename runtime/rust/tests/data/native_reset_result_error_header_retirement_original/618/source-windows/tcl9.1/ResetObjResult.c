ResetObjResult(
    Interp *iPtr)		/* Points to the interpreter whose result
				 * object should be reset. */
{
    Tcl_Obj *objResultPtr = iPtr->objResultPtr;

    if (Tcl_IsShared(objResultPtr)) {
	TclDecrRefCount(objResultPtr);
	TclNewObj(objResultPtr);
	Tcl_IncrRefCount(objResultPtr);
	iPtr->objResultPtr = objResultPtr;
    } else {
	if (objResultPtr->bytes != &tclEmptyString) {
	    if (objResultPtr->bytes) {
		Tcl_Free(objResultPtr->bytes);
	    }
	    objResultPtr->bytes = &tclEmptyString;
	    objResultPtr->length = 0;
	}
	TclFreeInternalRep(objResultPtr);
    }
}
