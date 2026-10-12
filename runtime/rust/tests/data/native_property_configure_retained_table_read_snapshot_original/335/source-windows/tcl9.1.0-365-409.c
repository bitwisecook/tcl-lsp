    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter used for the result, error
				 * reporting, etc. */
    Tcl_ObjectContext context,	/* The object/call context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    Tcl_Obj *namePtr;
    Tcl_Size i, namec;
    int code = TCL_OK;

    objc -= skip;
    if ((objc & 1) && (objc != 1)) {
	/*
	 * Bad (odd > 1) number of arguments.
	 */

	Tcl_WrongNumArgs(interp, skip, objv, "?-option value ...?");
	return TCL_ERROR;
    }

    objv += skip;
    if (objc == 0) {
	/*
	 * Read all properties.
	 */

	Tcl_Obj *listPtr = GetAllObjectProperties(oPtr, false);
	Tcl_Obj *resultPtr = Tcl_NewObj(), **namev;

	Tcl_IncrRefCount(listPtr);
	ListObjGetElements(listPtr, namec, namev);

	for (i = 0; i < namec; ) {
	    code = ReadProperty(interp, oPtr, namev[i]);
	    if (code != TCL_OK) {
		Tcl_DecrRefCount(resultPtr);
		break;
	    }
	    Tcl_DictObjPut(NULL, resultPtr, namev[i],
		    Tcl_GetObjResult(interp));
	    if (++i >= namec) {
		Tcl_SetObjResult(interp, resultPtr);
