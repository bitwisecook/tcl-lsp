InfoClassMethodTypeCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Method *mPtr;
    Class *clsPtr;

    if (objc != 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "className methodName");
	return TCL_ERROR;
    }
    clsPtr = TclOOGetClassFromObj(interp, objv[1]);
    if (clsPtr == NULL) {
	return TCL_ERROR;
    }

    mPtr = GetClassMethodFromObj(interp, clsPtr, objv[2]);
    if (mPtr == NULL) {
	return TCL_ERROR;
    }
    Tcl_SetObjResult(interp,
	    Tcl_NewStringObj(mPtr->type2Ptr->name, TCL_AUTO_LENGTH));
    return TCL_OK;
}
