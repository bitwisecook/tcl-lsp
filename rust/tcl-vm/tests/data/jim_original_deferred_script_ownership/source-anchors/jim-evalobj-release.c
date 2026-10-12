    /* Possibly add to the error stack trace */
    if (retcode == JIM_ERR) {
        JimSetErrorStack(interp, NULL);
    }

    JimPopEvalFrame(interp);

    /* Note that we don't have to decrement inUse, because the
     * following code transfers our use of the reference again to
     * the script object. */
    Jim_FreeIntRep(interp, scriptObjPtr);
    scriptObjPtr->typePtr = &scriptObjType;
    Jim_SetIntRepPtr(scriptObjPtr, script);
    Jim_DecrRefCount(interp, scriptObjPtr);

    return retcode;
}
