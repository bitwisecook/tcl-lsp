int Jim_EvalObj(Jim_Interp *interp, Jim_Obj *scriptObjPtr)
{
    int i;
    ScriptObj *script;
    ScriptToken *token;
    int retcode = JIM_OK;
    Jim_Obj *sargv[JIM_EVAL_SARGV_LEN], **argv = NULL;
    Jim_EvalFrame frame;

    /* If the object is of type "list", with no string rep we can call
     * a specialized version of Jim_EvalObj() */
    if (Jim_IsList(scriptObjPtr) && scriptObjPtr->bytes == NULL) {
        return JimEvalObjList(interp, scriptObjPtr);
    }

    Jim_IncrRefCount(scriptObjPtr);     /* Make sure it's shared. */
    script = JimGetScript(interp, scriptObjPtr);
    if (JimParseCheckMissing(interp, script->missing) == JIM_ERR) {
        JimSetErrorStack(interp, script);
        Jim_DecrRefCount(interp, scriptObjPtr);
        return JIM_ERR;
    }

    /* Reset the interpreter result. This is useful to
     * return the empty result in the case of empty program. */
    Jim_SetEmptyResult(interp);

