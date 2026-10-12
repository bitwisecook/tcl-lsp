TclOOAllocClass(
    Tcl_Interp *interp,		/* Interpreter within which to allocate the
				 * class. */
    Object *useThisObj)		/* Object that is to act as the class
				 * representation. */
{
    Foundation *fPtr = GetFoundation(interp);
    Class *clsPtr = (Class *) Tcl_Alloc(sizeof(Class));

    memset(clsPtr, 0, sizeof(Class));
    clsPtr->thisPtr = useThisObj;

    /*
     * Configure the namespace path for the class's object.
     */

    InitClassPath(interp, clsPtr);

    /*
     * Classes are subclasses of oo::object, i.e. the objects they create are
     * objects.
     */

    clsPtr->superclasses.num = 1;
    clsPtr->superclasses.list = (Class **) Tcl_Alloc(sizeof(Class *));
    clsPtr->superclasses.list[0] = fPtr->objectCls;
    AddRef(fPtr->objectCls->thisPtr);

    /*
     * Finish connecting the class structure to the object structure.
     */

    clsPtr->thisPtr->classPtr = clsPtr;

    /*
     * That's the complicated bit. Now fill in the rest of the non-zero/NULL
     * fields.
     */

    Tcl_InitObjHashTable(&clsPtr->classMethods);
    return clsPtr;
}

