 *
 * ----------------------------------------------------------------------
 */
static inline Method *
GetClassMethodFromObj(
    Tcl_Interp *interp,
    Class *clsPtr,
    Tcl_Obj *methodName)
{
    Tcl_HashEntry *hPtr = Tcl_FindHashEntry(&clsPtr->classMethods, methodName);
    Method *mPtr;

    if (hPtr == NULL) {
	goto unknownMethod;
    }
    mPtr = (Method *)Tcl_GetHashValue(hPtr);
    if (mPtr->type2Ptr == NULL) {
	/*
	 * Special entry for visibility control: pretend the method doesnt
	 * exist.
	 */

	goto unknownMethod;
    }
    return mPtr;

  unknownMethod:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "unknown method \"%s\"", TclGetString(methodName)));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "METHOD",
	    TclGetString(methodName), (char *)NULL);
    return NULL;
}

/*
 * ----------------------------------------------------------------------
 *
 * GetInstanceMethodFromObj --
 *
 *	Helper for looking up an object-instance-defined method.
 *
 * ----------------------------------------------------------------------
 */
static inline Method *
GetInstanceMethodFromObj(
    Tcl_Interp *interp,
    Object *oPtr,
    Tcl_Obj *methodName)
{
    Tcl_HashEntry *hPtr;
    Method *mPtr;

    if (!oPtr->methodsPtr) {
	goto unknownMethod;
    }
    hPtr = Tcl_FindHashEntry(oPtr->methodsPtr, methodName);
    if (hPtr == NULL) {
	goto unknownMethod;
    }
    mPtr = (Method *)Tcl_GetHashValue(hPtr);
    if (mPtr->type2Ptr == NULL) {
	/*
	 * Special entry for visibility control: pretend the method doesnt
	 * exist.
	 */

	goto unknownMethod;
    }
    return mPtr;

  unknownMethod:
    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
	    "unknown method \"%s\"", TclGetString(methodName)));
    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "METHOD",
	    TclGetString(methodName), (char *)NULL);
    return NULL;
}
