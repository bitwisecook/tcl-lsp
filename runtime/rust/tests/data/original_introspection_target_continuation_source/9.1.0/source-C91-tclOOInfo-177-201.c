TclOOGetClassFromObj(
    Tcl_Interp *interp,
    Tcl_Obj *objPtr)
{
    Object *oPtr = (Object *) Tcl_GetObjectFromObj(interp, objPtr);

    if (oPtr == NULL) {
	return NULL;
    }
    if (oPtr->classPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"\"%s\" is not a class", TclGetString(objPtr)));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "CLASS",
		TclGetString(objPtr), (char *)NULL);
	return NULL;
    }
    return oPtr->classPtr;
}

/*
 * ----------------------------------------------------------------------
 *
 * GetClassMethodFromObj --
 *
 *	Helper for looking up a class-defined method.
