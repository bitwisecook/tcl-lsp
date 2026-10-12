static int Jim_ExprCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
#ifdef JIM_COMPAT
    if (argc > 2) {
        int retcode;
        Jim_Obj *objPtr;

        objPtr = Jim_ConcatObj(interp, argc - 1, argv + 1);
        Jim_IncrRefCount(objPtr);
        retcode = Jim_EvalExpression(interp, objPtr);
        Jim_DecrRefCount(interp, objPtr);

        return retcode;
    }
#endif
    return Jim_EvalExpression(interp, argv[1]);
}
