Tcl_SetObjErrorCode(interp, errorObjPtr)
    Tcl_Interp *interp;
    Tcl_Obj *errorObjPtr;
{
    Interp *iPtr;
    
    iPtr = (Interp *) interp;
    Tcl_SetVar2Ex(interp, "errorCode", NULL, errorObjPtr, TCL_GLOBAL_ONLY);
    iPtr->flags |= ERROR_CODE_SET;
}
