Tcl_GetBooleanFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* The object from which to get boolean. */
    int *intPtr)	/* Place to store resulting boolean. */
{
    return Tcl_GetBoolFromObj(interp, objPtr, (TCL_NULL_OK-2)&(int)sizeof(int), (char *)(void *)intPtr);
}
