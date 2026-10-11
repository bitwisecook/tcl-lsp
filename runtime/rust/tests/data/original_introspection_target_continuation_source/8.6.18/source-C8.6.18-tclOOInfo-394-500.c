InfoObjectIsACmd(
    ClientData clientData,
    Tcl_Interp *interp,
    int objc,
    Tcl_Obj *const objv[])
{
    static const char *const categories[] = {
	"class", "metaclass", "mixin", "object", "typeof", NULL
    };
    enum IsACats {
	IsClass, IsMetaclass, IsMixin, IsObject, IsType
    };
    Object *oPtr, *o2Ptr;
    int idx, i, result = 0;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "category objName ?arg ...?");
	return TCL_ERROR;
    }
    if (Tcl_GetIndexFromObj(interp, objv[1], categories, "category", 0,
	    &idx) != TCL_OK) {
	return TCL_ERROR;
    }

    /*
     * Now we know what test we are doing, we can check we've got the right
     * number of arguments.
     */

    switch ((enum IsACats) idx) {
    case IsObject:
    case IsClass:
    case IsMetaclass:
	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "objName");
	    return TCL_ERROR;
	}
	break;
    case IsMixin:
    case IsType:
	if (objc != 4) {
	    Tcl_WrongNumArgs(interp, 2, objv, "objName className");
	    return TCL_ERROR;
	}
	break;
    }

    /*
     * Perform the check. Note that we can guarantee that we will not fail
     * from here on; "failures" result in a false-TCL_OK result.
     */

    oPtr = (Object *) Tcl_GetObjectFromObj(interp, objv[2]);
    if (oPtr == NULL) {
	goto failPrecondition;
    }

    switch ((enum IsACats) idx) {
    case IsObject:
	result = 1;
	break;
    case IsClass:
	result = (oPtr->classPtr != NULL);
	break;
    case IsMetaclass:
	if (oPtr->classPtr != NULL) {
	    result = TclOOIsReachable(TclOOGetFoundation(interp)->classCls,
		    oPtr->classPtr);
	}
	break;
    case IsMixin:
	o2Ptr = (Object *) Tcl_GetObjectFromObj(interp, objv[3]);
	if (o2Ptr == NULL) {
	    goto failPrecondition;
	}
	if (o2Ptr->classPtr != NULL) {
	    Class *mixinPtr;

	    FOREACH(mixinPtr, oPtr->mixins) {
		if (!mixinPtr) {
		    continue;
		}
		if (TclOOIsReachable(o2Ptr->classPtr, mixinPtr)) {
		    result = 1;
		    break;
		}
	    }
	}
	break;
    case IsType:
	o2Ptr = (Object *) Tcl_GetObjectFromObj(interp, objv[3]);
	if (o2Ptr == NULL) {
	    goto failPrecondition;
	}
	if (o2Ptr->classPtr != NULL) {
	    result = TclOOIsReachable(o2Ptr->classPtr, oPtr->selfCls);
	}
	break;
    }
    Tcl_SetObjResult(interp, Tcl_NewBooleanObj(result));
    return TCL_OK;

  failPrecondition:
    Tcl_ResetResult(interp);
    Tcl_SetObjResult(interp, Tcl_NewBooleanObj(0));
    return TCL_OK;
}
