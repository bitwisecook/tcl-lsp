NamespaceEvalCmd(
    void *clientData,		/* Arbitrary value passed to cmd. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    return Tcl_NRCallObjProc(interp, NRNamespaceEvalCmd, clientData, objc,
	    objv);
}
