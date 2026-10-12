Tcl_SetObjErrorCode(
    Tcl_Interp *interp,
    Tcl_Obj *errorObjPtr)
{
    Interp *iPtr = (Interp *) interp;

    if (iPtr->errorCode) {
	Tcl_DecrRefCount(iPtr->errorCode);
    }
    iPtr->errorCode = errorObjPtr;
    Tcl_IncrRefCount(iPtr->errorCode);
}
