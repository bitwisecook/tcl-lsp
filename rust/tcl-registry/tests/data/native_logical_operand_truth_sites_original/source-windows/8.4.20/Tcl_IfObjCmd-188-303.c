Tcl_IfObjCmd(dummy, interp, objc, objv)
    ClientData dummy;			/* Not used. */
    Tcl_Interp *interp;			/* Current interpreter. */
    int objc;				/* Number of arguments. */
    Tcl_Obj *CONST objv[];		/* Argument objects. */
{
    int thenScriptIndex = 0;	/* then script to be evaled after syntax check */
#ifdef TCL_TIP280
    Interp* iPtr = (Interp*) interp;
#endif
    int i, result, value;
    char *clause;
    i = 1;
    while (1) {
	/*
	 * At this point in the loop, objv and objc refer to an expression
	 * to test, either for the main expression or an expression
	 * following an "elseif".  The arguments after the expression must
	 * be "then" (optional) and a script to execute if the expression is
	 * true.
	 */

	if (i >= objc) {
	    clause = Tcl_GetString(objv[i-1]);
	    Tcl_AppendResult(interp, "wrong # args: no expression after \"",
		    clause, "\" argument", (char *) NULL);
	    return TCL_ERROR;
	}
	if (!thenScriptIndex) {
	    result = Tcl_ExprBooleanObj(interp, objv[i], &value);
	    if (result != TCL_OK) {
		return result;
	    }
	}
	i++;
	if (i >= objc) {
	    missingScript:
	    clause = Tcl_GetString(objv[i-1]);
	    Tcl_AppendResult(interp, "wrong # args: no script following \"",
		    clause, "\" argument", (char *) NULL);
	    return TCL_ERROR;
	}
	clause = Tcl_GetString(objv[i]);
	if ((i < objc) && (strcmp(clause, "then") == 0)) {
	    i++;
	}
	if (i >= objc) {
	    goto missingScript;
	}
	if (value) {
	    thenScriptIndex = i;
	    value = 0;
	}
	
	/*
	 * The expression evaluated to false.  Skip the command, then
	 * see if there is an "else" or "elseif" clause.
	 */

	i++;
	if (i >= objc) {
	    if (thenScriptIndex) {
#ifndef TCL_TIP280
		return Tcl_EvalObjEx(interp, objv[thenScriptIndex], 0);
#else
		/* TIP #280. Make invoking context available to branch */
		return TclEvalObjEx(interp, objv[thenScriptIndex], 0,
				    iPtr->cmdFramePtr,thenScriptIndex);
#endif
	    }
	    return TCL_OK;
	}
	clause = Tcl_GetString(objv[i]);
	if ((clause[0] == 'e') && (strcmp(clause, "elseif") == 0)) {
	    i++;
	    continue;
	}
	break;
    }

    /*
     * Couldn't find a "then" or "elseif" clause to execute.  Check now
     * for an "else" clause.  We know that there's at least one more
     * argument when we get here.
     */

    if (strcmp(clause, "else") == 0) {
	i++;
	if (i >= objc) {
	    Tcl_AppendResult(interp,
		    "wrong # args: no script following \"else\" argument",
		    (char *) NULL);
	    return TCL_ERROR;
	}
    }
    if (i < objc - 1) {
	Tcl_AppendResult(interp,
		"wrong # args: extra words after \"else\" clause in \"if\" command",
		(char *) NULL);
	return TCL_ERROR;
    }
    if (thenScriptIndex) {
#ifndef TCL_TIP280
	return Tcl_EvalObjEx(interp, objv[thenScriptIndex], 0);
#else
	/* TIP #280. Make invoking context available to branch/else */
	return TclEvalObjEx(interp, objv[thenScriptIndex], 0,
			    iPtr->cmdFramePtr,thenScriptIndex);
#endif
    }
#ifndef TCL_TIP280
    return Tcl_EvalObjEx(interp, objv[i], 0);
#else
    return TclEvalObjEx(interp, objv[i], 0, iPtr->cmdFramePtr,i);
#endif
}
