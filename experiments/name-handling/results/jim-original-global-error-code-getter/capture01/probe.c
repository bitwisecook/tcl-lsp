// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include "jim.h"
static int mode;
static Jim_Obj *expected;
static void bytes(Jim_Obj *value)
{
    int length;
    const char *string = Jim_GetString(value, &length);
    for (int n = 0; n < length; n++) printf("%02x", (unsigned char)string[n]);
}
static int read_global(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    (void)argc; (void)argv;
    Jim_CallFrame *frame = interp->framePtr;
    Jim_Obj *ns = frame->nsObj;
    Jim_Obj *local = Jim_GetVariableStr(interp, "errorCode", JIM_ERRMSG);
    Jim_SetResultString(interp, "MARKER", -1);
    Jim_Obj *value = Jim_GetGlobalVariableStr(interp, "errorCode", JIM_NONE);
    printf("GLOBAL_READ|%d|%d|%d|%d|%d|%d|", mode, value != NULL,
        value && value == expected, interp->framePtr == frame,
        interp->framePtr->nsObj == ns,
        Jim_GetVariableStr(interp, "errorCode", JIM_ERRMSG) == local);
    if (value) printf("%d|%s|", value->bytes != NULL, value->typePtr ? value->typePtr->name : "none");
    else printf("-|-|");
    bytes(Jim_GetResult(interp));
    printf("\n");
    return JIM_OK;
}
int main(void)
{
    const char *setups[] = {"unset -nocomplain errorCode", "list",
        "upvar #0 target errorCode", "set scalar ODD; upvar #0 scalar(member) errorCode"};
    for (mode = 0; mode < 4; mode++) {
        Jim_Interp *interp = Jim_CreateInterp();
        Jim_RegisterCoreCommands(interp);
        if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
        expected = NULL;
        if (mode == 1 || mode == 2) {
            expected = Jim_NewIntObj(interp, 777);
            if (Jim_SetGlobalVariableStr(interp, mode == 1 ? "errorCode" : "target", expected) != JIM_OK) return 3;
        }
        int code = Jim_Eval(interp, setups[mode]);
        printf("SETUP|%d|%d\n", mode, code);
        if (code != JIM_OK) return 4;
        Jim_CreateCommand(interp, "read_selected_error_code", read_global, NULL, NULL);
        code = Jim_Eval(interp, "namespace eval ReadKey {proc p {} {set errorCode LOCAL; read_selected_error_code}; p}");
        printf("CALL|%d|%d\n", mode, code);
        Jim_FreeInterp(interp);
    }
    return 0;
}
