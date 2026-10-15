// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include "tcl.h"
int main(int argc, char **argv)
{
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    printf("HEADER|%s\n", TCL_PATCH_LEVEL);
    for (int mode = 0; mode < 3; mode++) {
        Tcl_Interp *interp = Tcl_CreateInterp();
        const char *version = Tcl_GetVar(interp, "tcl_patchLevel", TCL_GLOBAL_ONLY);
        printf("VERSION|%d|%s\n", mode, version ? version : "missing");
        if (mode == 2 && Tcl_Eval(interp, "array set 123 {member VALUE}") != TCL_OK) return 2;
        Tcl_Obj *name = mode == 1 ? Tcl_NewStringObj("", 0) : Tcl_NewIntObj(123);
        Tcl_Obj *options = Tcl_NewIntObj(456);
        Tcl_Obj *words[] = {Tcl_NewStringObj("catch", -1), Tcl_NewStringObj("error BOOM", -1), name, options};
        for (int n = 0; n < 4; n++) Tcl_IncrRefCount(words[n]);
        int count = TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4 ? 3 : 4;
        int code = Tcl_EvalObjv(interp, count, words, TCL_EVAL_DIRECT);
        printf("RESULT|%d|%d\n", mode, code);
        for (int n = 0; n < 2; n++) {
            Tcl_Obj *object = n == 0 ? name : options;
            printf("NAME|%d|%d|%d|%s\n", mode, n, object->bytes != NULL,
                object->typePtr ? object->typePtr->name : "none");
        }
        for (int n = 0; n < 4; n++) Tcl_DecrRefCount(words[n]);
        Tcl_DeleteInterp(interp);
    }
    Tcl_Finalize();
    return 0;
}
