static int SetBooleanFromAny(Jim_Interp *interp, Jim_Obj *objPtr, int flags)
{
    int index = Jim_FindByName(Jim_String(objPtr), jim_true_false_strings,
        sizeof(jim_true_false_strings) / sizeof(*jim_true_false_strings));
    if (index < 0) {
        if (flags & JIM_ERRMSG) {
            Jim_SetResultFormatted(interp, "expected boolean but got \"%#s\"", objPtr);
        }
        return JIM_ERR;
    }

    /* Free the old internal repr and set the new one. */
    Jim_FreeIntRep(interp, objPtr);
    objPtr->typePtr = &intObjType;
    /* 4 true values in jim_true_false_strings */
    objPtr->internalRep.wideValue = index < 4 ? 1 : 0;
    return JIM_OK;
}
