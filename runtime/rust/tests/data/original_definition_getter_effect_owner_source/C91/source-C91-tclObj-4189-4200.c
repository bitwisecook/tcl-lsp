size_t
TclHashObjKey(
    TCL_UNUSED(Tcl_HashTable *),
    void *keyPtr)		/* Key from which to compute hash value. */
{
    Tcl_Obj *objPtr = (Tcl_Obj *)keyPtr;
    Tcl_Size length;
    const char *string = Tcl_GetStringFromObj(objPtr, &length);
    size_t result = 0;

    /*
     * I tried a zillion different hash functions and asked many other people
