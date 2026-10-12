NamespaceEvalCmd(
    void *clientData,		/* Arbitrary value passed to cmd. */
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    return Tcl_NRCallObjProc2(interp, NRNamespaceEvalCmd, clientData, objc,
	    objv);
}
