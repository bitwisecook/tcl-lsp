// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include "tcl.h"
int main(int argc, char **argv)
{
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    printf("HEADER|%s\n", TCL_PATCH_LEVEL);
    for (int mode = 0; mode < 5; mode++) {
        Tcl_Interp *interp = Tcl_CreateInterp();
        printf("VERSION|%d|%s\n", mode, Tcl_GetVar(interp, "tcl_patchLevel", TCL_GLOBAL_ONLY));
        if (mode == 2 && Tcl_Eval(interp, "array set 123 {member VALUE}") != TCL_OK) return 2;
        Tcl_Obj *names[] = {mode == 1 ? Tcl_NewStringObj("", 0) : Tcl_NewIntObj(123), Tcl_NewIntObj(456)};
        Tcl_Obj *unused[] = {Tcl_NewIntObj(777), Tcl_NewIntObj(888)};
        Tcl_Obj *vars = mode == 4 ? Tcl_NewStringObj("{", -1) : Tcl_NewListObj(2, names);
        Tcl_Obj *unused_vars = Tcl_NewListObj(2, unused);
        for (int i = 0; i < 2; i++) { Tcl_IncrRefCount(names[i]); Tcl_IncrRefCount(unused[i]); }
        Tcl_IncrRefCount(vars); Tcl_IncrRefCount(unused_vars);
        Tcl_Obj *words[] = {Tcl_NewStringObj("try", -1), Tcl_NewStringObj("set BODY RAN; error BOOM", -1),
            Tcl_NewStringObj("on", -1), Tcl_NewStringObj("error", -1), mode == 3 ? unused_vars : vars,
            Tcl_NewStringObj(mode == 3 ? "-" : "set HANDLED YES", -1),
            Tcl_NewStringObj("on", -1), Tcl_NewStringObj("error", -1), vars, Tcl_NewStringObj("set HANDLED YES", -1)};
        int count = mode == 3 ? 10 : 6;
        for (int i = 0; i < count; i++) Tcl_IncrRefCount(words[i]);
        int code = Tcl_EvalObjv(interp, count, words, TCL_EVAL_DIRECT);
        printf("RESULT|%d|%d|%d|%d\n", mode, code,
            Tcl_GetVar(interp, "BODY", TCL_GLOBAL_ONLY) != NULL, Tcl_GetVar(interp, "HANDLED", TCL_GLOBAL_ONLY) != NULL);
        for (int i = 0; i < 4; i++) {
            Tcl_Obj *object = i < 2 ? names[i] : unused[i - 2];
            printf("NAME|%d|%d|%d|%s\n", mode, i, object->bytes != NULL,
                object->typePtr ? object->typePtr->name : "none");
        }
        for (int i = 0; i < count; i++) Tcl_DecrRefCount(words[i]);
        Tcl_DecrRefCount(vars); Tcl_DecrRefCount(unused_vars);
        for (int i = 0; i < 2; i++) { Tcl_DecrRefCount(names[i]); Tcl_DecrRefCount(unused[i]); }
        Tcl_DeleteInterp(interp);
    }
    Tcl_Finalize();
    return 0;
}
