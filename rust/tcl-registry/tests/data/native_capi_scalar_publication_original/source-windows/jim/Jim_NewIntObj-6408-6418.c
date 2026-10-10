Jim_Obj *Jim_NewIntObj(Jim_Interp *interp, jim_wide wideValue)
{
    Jim_Obj *objPtr;

    objPtr = Jim_NewObj(interp);
    objPtr->typePtr = &intObjType;
    objPtr->bytes = NULL;
    objPtr->taint = 0;
    objPtr->internalRep.wideValue = wideValue;
    return objPtr;
}
