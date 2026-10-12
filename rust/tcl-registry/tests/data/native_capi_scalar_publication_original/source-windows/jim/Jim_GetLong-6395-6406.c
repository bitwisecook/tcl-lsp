int Jim_GetLong(Jim_Interp *interp, Jim_Obj *objPtr, long *longPtr)
{
    jim_wide wideValue;
    int retval;

    retval = Jim_GetWide(interp, objPtr, &wideValue);
    if (retval == JIM_OK) {
        *longPtr = (long)wideValue;
        return JIM_OK;
    }
    return JIM_ERR;
}
