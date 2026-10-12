TclOODefinePropertyCmd(
    void *useInstance,		/* NULL for class, non-NULL for object. */
    Tcl_Interp *interp,		/* For error reporting and lookup. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Arguments. */
{
    int i;
    static const char *const options[] = {
	"-get", "-kind", "-set", NULL
    };
    enum Options {
	OPT_GET, OPT_KIND, OPT_SET
    };
    static const char *const kinds[] = {
	"readable", "readwrite", "writable", NULL
    };
    enum Kinds {
	KIND_RO, KIND_RW, KIND_WO
    };
    Object *oPtr = (Object *) TclOOGetDefineCmdContext(interp);

    if (oPtr == NULL) {
	return TCL_ERROR;
    }
    if (!useInstance && !oPtr->classPtr) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to misuse API", TCL_AUTO_LENGTH));
	OO_ERROR(interp, MONKEY_BUSINESS);
	return TCL_ERROR;
    }

    for (i = 1; i < objc; i++) {
	Tcl_Obj *propObj = objv[i], *nextObj, *argObj, *hyphenated;
	Tcl_Obj *getterScript = NULL, *setterScript = NULL;

	/*
	 * Parse the extra options for the property.
	 */

	int kind = KIND_RW;
	while (i + 1 < objc) {
	    int option;

	    nextObj = objv[i + 1];
	    if (TclGetString(nextObj)[0] != '-') {
		break;
	    }
	    if (Tcl_GetIndexFromObj(interp, nextObj, options, "option", 0,
		    &option) != TCL_OK) {
		return TCL_ERROR;
	    }
	    if (i + 2 >= objc) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"missing %s to go with %s option",
			(option == OPT_KIND ? "kind value" : "body"),
			options[option]));
		Tcl_SetErrorCode(interp, "TCL", "WRONGARGS", (char *)NULL);
		return TCL_ERROR;
	    }
	    argObj = objv[i + 2];
	    i += 2;
	    switch (option) {
	    case OPT_GET:
		getterScript = argObj;
		break;
	    case OPT_SET:
		setterScript = argObj;
		break;
	    case OPT_KIND:
		if (Tcl_GetIndexFromObj(interp, argObj, kinds, "kind", 0,
			&kind) != TCL_OK) {
		    return TCL_ERROR;
		}
		break;
	    default:
		TCL_UNREACHABLE();
	    }
	}

	/*
	 * Install the property. Note that InstallStdPropertyImpls
	 * validates the property name as well.
	 */

	if (InstallStdPropertyImpls(useInstance, interp, propObj,
		kind != KIND_WO && getterScript == NULL,
		kind != KIND_RO && setterScript == NULL) != TCL_OK) {
	    return TCL_ERROR;
	}

	hyphenated = Tcl_ObjPrintf("-%s", TclGetString(propObj));
	if (useInstance) {
	    TclOORegisterInstanceProperty(oPtr, hyphenated,
		    kind != KIND_WO, kind != KIND_RO);
	} else {
	    TclOORegisterProperty(oPtr->classPtr, hyphenated,
		    kind != KIND_WO, kind != KIND_RO);
	}
	Tcl_BounceRefCount(hyphenated);
