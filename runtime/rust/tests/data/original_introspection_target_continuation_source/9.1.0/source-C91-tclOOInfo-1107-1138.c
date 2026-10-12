InfoClassDefnCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    Method *mPtr;
    Proc *procPtr;
    Tcl_Obj *resultObjs[2];
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
    procPtr = GetProcFromMethod(interp, mPtr);
    if (procPtr == NULL) {
	return TCL_ERROR;
    }

    resultObjs[0] = DescribeMethodArgs(procPtr);
    resultObjs[1] = TclOOGetMethodBody(mPtr);
    Tcl_SetObjResult(interp, Tcl_NewListObj(2, resultObjs));
    return TCL_OK;
