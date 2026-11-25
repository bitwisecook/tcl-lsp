InfoObjectMethodsCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,
    Tcl_Size objc,
    Tcl_Obj *const *objv)
{
    static const char *const options[] = {
	"-all", "-localprivate", "-private", "-scope", NULL
    };
    enum Options {
	OPT_ALL, OPT_LOCALPRIVATE, OPT_PRIVATE, OPT_SCOPE
    } idx;
    static const char *const scopes[] = {
	"private", "public", "unexported"
    };
    enum Scopes {
	SCOPE_PRIVATE, SCOPE_PUBLIC, SCOPE_UNEXPORTED,
	SCOPE_LOCALPRIVATE,
	SCOPE_DEFAULT = -1
    };
    Object *oPtr;
    int flag = PUBLIC_METHOD, scope = SCOPE_DEFAULT;
    bool recurse = false;
    FOREACH_HASH_DECLS;
    Tcl_Obj *namePtr, *resultObj;
    Method *mPtr;

    /*
     * Parse arguments.
     */

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "objName ?-option value ...?");
	return TCL_ERROR;
    }
    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[1]);
    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (objc != 2) {
	for (Tcl_Size i=2 ; i<objc ; i++) {
	    if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0,
		    &idx) != TCL_OK) {
		return TCL_ERROR;
	    }
	    switch (idx) {
	    case OPT_ALL:
		recurse = true;
		break;
	    case OPT_LOCALPRIVATE:
		flag = PRIVATE_METHOD;
		break;
	    case OPT_PRIVATE:
		flag = 0;
		break;
	    case OPT_SCOPE:
		if (++i >= objc) {
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "missing option for -scope"));
		    Tcl_SetErrorCode(interp, "TCL", "ARGUMENT", "MISSING",
			    (char *)NULL);
		    return TCL_ERROR;
		}
		if (Tcl_GetIndexFromObj(interp, objv[i], scopes, "scope", 0,
			&scope) != TCL_OK) {
		    return TCL_ERROR;
		}
		break;
	    default:
		TCL_UNREACHABLE();
	    }
	}
    }
    if (scope != SCOPE_DEFAULT) {
	recurse = false;
	switch (scope) {
	case SCOPE_PRIVATE:
	    flag = TRUE_PRIVATE_METHOD;
	    break;
	case SCOPE_PUBLIC:
	    flag = PUBLIC_METHOD;
	    break;
	case SCOPE_LOCALPRIVATE:
	    flag = PRIVATE_METHOD;
	    break;
	case SCOPE_UNEXPORTED:
	    flag = 0;
	    break;
	}
    }

    /*
