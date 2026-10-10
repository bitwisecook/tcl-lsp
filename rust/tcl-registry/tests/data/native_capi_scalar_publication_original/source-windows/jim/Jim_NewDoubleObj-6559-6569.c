Jim_Obj *Jim_NewDoubleObj(Jim_Interp *interp, double doubleValue)
{
    Jim_Obj *objPtr;

    objPtr = Jim_NewObj(interp);
    objPtr->typePtr = &doubleObjType;
    objPtr->bytes = NULL;
    objPtr->taint = 0;
    objPtr->internalRep.doubleValue = doubleValue;
    return objPtr;
}
