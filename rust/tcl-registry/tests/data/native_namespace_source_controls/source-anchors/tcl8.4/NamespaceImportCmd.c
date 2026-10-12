NamespaceImportCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int allowOverwrite = 0;
    char *string, *pattern;
    register int i, result;
    int firstArg;

    if (objc < 2) {
        Tcl_WrongNumArgs(interp, 2, objv,
	        "?-force? ?pattern pattern...?");
        return TCL_ERROR;
    }

    /*
     * Skip over the optional "-force" as the first argument.
     */

    firstArg = 2;
    if (firstArg < objc) {
	string = Tcl_GetString(objv[firstArg]);
	if ((*string == '-') && (strcmp(string, "-force") == 0)) {
	    allowOverwrite = 1;
	    firstArg++;
	}
    }

    /*
     * Handle the imports for each of the patterns.
     */

    for (i = firstArg;  i < objc;  i++) {
        pattern = Tcl_GetString(objv[i]);
	result = Tcl_Import(interp, (Tcl_Namespace *) NULL, pattern,
	        allowOverwrite);
        if (result != TCL_OK) {
            return result;
        }
    }
    return TCL_OK;
}
