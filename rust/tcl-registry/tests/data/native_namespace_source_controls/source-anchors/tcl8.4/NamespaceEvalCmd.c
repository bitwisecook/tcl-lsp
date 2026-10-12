NamespaceEvalCmd(dummy, interp, objc, objv)
    ClientData dummy;		/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    Tcl_Namespace *namespacePtr;
    CallFrame frame;
    Tcl_Obj *objPtr;
    char *name;
    int length, result;

    if (objc < 4) {
        Tcl_WrongNumArgs(interp, 2, objv, "name arg ?arg...?");
        return TCL_ERROR;
    }

    /*
     * Try to resolve the namespace reference, caching the result in the
     * namespace object along the way.
     */

    result = GetNamespaceFromObj(interp, objv[2], &namespacePtr);
    if (result != TCL_OK) {
        return result;
    }

    /*
     * If the namespace wasn't found, try to create it.
     */
    
    if (namespacePtr == NULL) {
	name = Tcl_GetStringFromObj(objv[2], &length);
	namespacePtr = Tcl_CreateNamespace(interp, name, (ClientData) NULL, 
                (Tcl_NamespaceDeleteProc *) NULL);
	if (namespacePtr == NULL) {
	    return TCL_ERROR;
	}
    }

    /*
     * Make the specified namespace the current namespace and evaluate
     * the command(s).
     */

    result = Tcl_PushCallFrame(interp, (Tcl_CallFrame *) &frame, 
            namespacePtr, /*isProcCallFrame*/ 0);
    if (result != TCL_OK) {
        return TCL_ERROR;
    }
    frame.objc = objc;
    frame.objv = objv;  /* ref counts do not need to be incremented here */

    if (objc == 4) {
#ifndef TCL_TIP280
        result = Tcl_EvalObjEx(interp, objv[3], 0);
#else
        /* TIP #280 : Make actual argument location available to eval'd script */
        Interp* iPtr      = (Interp*) interp;
	CmdFrame* invoker = iPtr->cmdFramePtr;
	int word          = 3;
	TclArgumentGet (interp, objv[3], &invoker, &word);
        result = TclEvalObjEx(interp, objv[3], 0, invoker, word);
#endif
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result.  Tcl_EvalObjEx will delete
	 * the object when it decrements its refcount after eval'ing it.
	 */
        objPtr = Tcl_ConcatObj(objc-3, objv+3);
#ifndef TCL_TIP280
        result = Tcl_EvalObjEx(interp, objPtr, TCL_EVAL_DIRECT);
#else
	/* TIP #280. Make invoking context available to eval'd script */
	result = TclEvalObjEx(interp, objPtr, TCL_EVAL_DIRECT, NULL, 0);
#endif
    }
    if (result == TCL_ERROR) {
        char msg[256 + TCL_INTEGER_SPACE];
	
        sprintf(msg, "\n    (in namespace eval \"%.200s\" script line %d)",
            namespacePtr->fullName, interp->errorLine);
        Tcl_AddObjErrorInfo(interp, msg, -1);
    }

    /*
     * Restore the previous "current" namespace.
     */
    
    Tcl_PopCallFrame(interp);
    return result;
}
