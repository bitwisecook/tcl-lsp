static int Jim_EvalCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int rc;

    if (argc == 2) {
        rc = Jim_EvalObj(interp, argv[1]);
    }
    else {
        rc = Jim_EvalObj(interp, Jim_ConcatObj(interp, argc - 1, argv + 1));
    }

    return rc;
}
