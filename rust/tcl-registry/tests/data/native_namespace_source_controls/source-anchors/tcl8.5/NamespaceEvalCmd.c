NamespaceEvalCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_Namespace *namespacePtr;
    CallFrame *framePtr, **framePtrPtr;
    Tcl_Obj *objPtr;
    int result;

    if (objc < 4) {
	Tcl_WrongNumArgs(interp, 2, objv, "name arg ?arg...?");
	return TCL_ERROR;
    }

    /*
     * Try to resolve the namespace reference, caching the result in the
     * namespace object along the way.
     */

    result = GetNamespaceFromObj(interp, objv[2], &namespacePtr);

    /*
     * If the namespace wasn't found, try to create it.
     */

    if (result == TCL_ERROR) {
	char *name = TclGetString(objv[2]);

	namespacePtr = Tcl_CreateNamespace(interp, name, NULL, NULL);
	if (namespacePtr == NULL) {
	    return TCL_ERROR;
	}
    }

    /*
     * Make the specified namespace the current namespace and evaluate the
     * command(s).
     */

    /* This is needed to satisfy GCC 3.3's strict aliasing rules */
    framePtrPtr = &framePtr;
    result = TclPushStackFrame(interp, (Tcl_CallFrame **) framePtrPtr,
	    namespacePtr, /*isProcCallFrame*/ 0);
    if (result != TCL_OK) {
	return TCL_ERROR;
    }

    framePtr->objc = objc;
    framePtr->objv = objv;

    if (objc == 4) {
	/*
	 * TIP #280: Make actual argument location available to eval'd script.
	 */

	Interp *iPtr      = (Interp *) interp;
	CmdFrame* invoker = iPtr->cmdFramePtr;
	int word          = 3;

	TclArgumentGet (interp, objv[3], &invoker, &word);
	result = TclEvalObjEx(interp, objv[3], 0, invoker, word);
    } else {
	/*
	 * More than one argument: concatenate them together with spaces
	 * between, then evaluate the result. Tcl_EvalObjEx will delete the
	 * object when it decrements its refcount after eval'ing it.
	 */

	objPtr = Tcl_ConcatObj(objc-3, objv+3);

	/*
	 * TIP #280: Make invoking context available to eval'd script.
	 */

	result = TclEvalObjEx(interp, objPtr, TCL_EVAL_DIRECT, NULL, 0);
    }

    if (result == TCL_ERROR) {
	int length = strlen(namespacePtr->fullName);
	int limit = 200;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (in namespace eval \"%.*s%s\" script line %d)",
		(overflow ? limit : length), namespacePtr->fullName,
		(overflow ? "..." : ""), interp->errorLine));
    }

    /*
     * Restore the previous "current" namespace.
     */

    TclPopStackFrame(interp);
    return result;
}
