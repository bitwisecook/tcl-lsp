static int Jim_IfCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int boolean, retval, current = 1, falsebody = 0;

    while (1) {
        /* not enough arguments given! */
        if (current >= argc) {
            return JIM_USAGE;
        }
        if ((retval = Jim_GetBoolFromExpr(interp, argv[current++], &boolean))
            != JIM_OK)
            return retval;
        /* There lacks something, isn't it? */
        if (current >= argc) {
            return JIM_USAGE;
        }
        if (Jim_CompareStringImmediate(interp, argv[current], "then"))
            current++;
        /* Tsk tsk, no then-clause? */
        if (current >= argc) {
            return JIM_USAGE;
        }
        if (boolean)
            return Jim_EvalObj(interp, argv[current]);
        /* Ok: no else-clause follows */
        if (++current >= argc) {
            Jim_SetResult(interp, Jim_NewEmptyStringObj(interp));
            return JIM_OK;
        }
        falsebody = current++;
        if (Jim_CompareStringImmediate(interp, argv[falsebody], "else")) {
            /* IIICKS - else-clause isn't last cmd? */
            if (current != argc - 1) {
                return JIM_USAGE;
            }
            return Jim_EvalObj(interp, argv[current]);
        }
        else if (Jim_CompareStringImmediate(interp, argv[falsebody], "elseif"))
            /* Ok: elseif follows meaning all the stuff
             * again (how boring...) */
            continue;
        /* OOPS - else-clause is not last cmd? */
        else if (falsebody != argc - 1) {
            return JIM_USAGE;
        }
        return Jim_EvalObj(interp, argv[falsebody]);
    }
    return JIM_OK;
}
