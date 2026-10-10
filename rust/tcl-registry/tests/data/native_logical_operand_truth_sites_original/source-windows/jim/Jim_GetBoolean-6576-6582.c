int Jim_GetBoolean(Jim_Interp *interp, Jim_Obj *objPtr, int * booleanPtr)
{
    if (objPtr->typePtr != &intObjType && SetBooleanFromAny(interp, objPtr, JIM_ERRMSG) == JIM_ERR)
        return JIM_ERR;
    *booleanPtr = (int) JimWideValue(objPtr);
    return JIM_OK;
}
