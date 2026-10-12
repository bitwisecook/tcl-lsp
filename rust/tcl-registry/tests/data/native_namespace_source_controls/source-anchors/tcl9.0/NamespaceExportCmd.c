NamespaceExportCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int firstArg, i;

    if (objc < 1) {
	Tcl_WrongNumArgs(interp, 1, objv, "?-clear? ?pattern pattern...?");
	return TCL_ERROR;
    }

    /*
     * If no pattern arguments are given, and "-clear" isn't specified, return
     * the namespace's current export pattern list.
     */

    if (objc == 1) {
	Tcl_Obj *listPtr;

	TclNewObj(listPtr);
	(void) Tcl_AppendExportList(interp, NULL, listPtr);
	Tcl_SetObjResult(interp, listPtr);
	return TCL_OK;
    }

    /*
     * Process the optional "-clear" argument.
     */

    firstArg = 1;
    if (strcmp("-clear", TclGetString(objv[firstArg])) == 0) {
	Tcl_Export(interp, NULL, "::", 1);
	Tcl_ResetResult(interp);
	firstArg++;
    }

    /*
     * Add each pattern to the namespace's export pattern list.
     */

    for (i = firstArg;  i < objc;  i++) {
	int result = Tcl_Export(interp, NULL, TclGetString(objv[i]), 0);
	if (result != TCL_OK) {
	    return result;
	}
    }
    return TCL_OK;
}
