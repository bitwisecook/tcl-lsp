TclOOGetDefineCmdContext(
    Tcl_Interp *interp)
{
    Interp *iPtr = (Interp *) interp;
    Tcl_Object object;

    if ((iPtr->varFramePtr == NULL)
	    || (iPtr->varFramePtr->isProcCallFrame != FRAME_IS_OO_DEFINE
	    && iPtr->varFramePtr->isProcCallFrame != PRIVATE_FRAME)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"this command may only be called from within the context of"
		" an ::oo::define or ::oo::objdefine command",
		TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return NULL;
    }
    object = (Tcl_Object) iPtr->varFramePtr->clientData;
    if (Tcl_ObjectDeleted(object)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"this command cannot be called when the object has been"
		" deleted", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return NULL;
    }
    return object;
}

Class *
TclOOGetClassDefineCmdContext(
    Tcl_Interp *interp)
{
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);
    if (oPtr == NULL) {
	return NULL;
    }
    if (!oPtr->classPtr) {
	(void) ReportAbuse(interp);
	return NULL;
    }
    return oPtr->classPtr;
}
