static int Jim_UplevelCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int retcode;
    Jim_CallFrame *savedCallFrame, *targetCallFrame;
    const char *str;

    /* Save the old callframe pointer */
    savedCallFrame = interp->framePtr;

    /* Lookup the target frame pointer */
    str = Jim_String(argv[1]);
    if ((str[0] >= '0' && str[0] <= '9') || str[0] == '#') {
        targetCallFrame = Jim_GetCallFrameByLevel(interp, argv[1]);
        argc--;
        argv++;
    }
    else {
        targetCallFrame = Jim_GetCallFrameByLevel(interp, NULL);
    }
    if (targetCallFrame == NULL) {
        return JIM_ERR;
    }
    if (argc < 2) {
        return JIM_USAGE;
    }
    /* Eval the code in the target callframe. */
    interp->framePtr = targetCallFrame;
    if (argc == 2) {
        retcode = Jim_EvalObj(interp, argv[1]);
    }
    else {
        retcode = Jim_EvalObj(interp, Jim_ConcatObj(interp, argc - 1, argv + 1));
    }
    interp->framePtr = savedCallFrame;
    return retcode;
}
