// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include <string.h>
#include "jim.h"

static int global_mode;
static int reset_global(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    (void)argc; (void)argv;
    Jim_CallFrame *frame = interp->framePtr;
    Jim_Obj *ns = frame->nsObj;
    Jim_Obj *local = Jim_GetVariableStr(interp, "errorCode", JIM_ERRMSG);
    int code = Jim_SetGlobalVariableStr(interp, "errorCode", Jim_NewStringObj(interp, "NONE", -1));
    printf("GLOBAL_RESET|%d|%d|%d|%d|%d\n", global_mode, code,
        interp->framePtr == frame, interp->framePtr->nsObj == ns,
        Jim_GetVariableStr(interp, "errorCode", JIM_ERRMSG) == local);
    Jim_SetEmptyResult(interp);
    return JIM_OK;
}

static int global_reset(int mode)
{
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp) != JIM_OK) return 4;
    const char *setups[] = {"list", "array set ::errorCode {member VALUE}",
        "set scalar ODD; upvar #0 scalar(member) errorCode"};
    int code = Jim_Eval(interp, setups[mode]);
    printf("GLOBAL_SETUP|%d|%d\n", mode, code);
    if (code != JIM_OK) return 5;
    global_mode = mode;
    Jim_CreateCommand(interp, "reset_selected_error_code", reset_global, NULL, NULL);
    code = Jim_Eval(interp, "namespace eval CatchKey {proc p {} {set errorCode LOCAL; reset_selected_error_code}; p}");
    printf("GLOBAL_CALL|%d|%d\n", mode, code);
    Jim_FreeInterp(interp);
    return 0;
}

static int run(const char *command, int mode)
{
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
    if (mode == 2 && Jim_Eval(interp, "array set 123 {member VALUE}") != JIM_OK) return 3;
    if (mode == 3 && Jim_Eval(interp, "set scalar ODD; upvar #0 scalar(member) 123") != JIM_OK) return 6;
    Jim_Obj *message = mode == 1 ? Jim_NewStringObj(interp, "", 0) : Jim_NewIntObj(interp, 123);
    Jim_Obj *options = Jim_NewIntObj(interp, 456);
    Jim_IncrRefCount(message);
    Jim_IncrRefCount(options);
    Jim_Obj *argv[7];
    int argc = 0;
    argv[argc++] = Jim_NewStringObj(interp, command, -1);
    argv[argc++] = Jim_NewStringObj(interp, "error BOOM", -1);
    if (strcmp(command, "try") == 0) {
        argv[argc++] = Jim_NewStringObj(interp, "on", -1);
        argv[argc++] = Jim_NewStringObj(interp, "error", -1);
        Jim_Obj *names[] = {message, options};
        argv[argc++] = Jim_NewListObj(interp, names, 2);
        argv[argc++] = Jim_NewStringObj(interp, "list", -1);
    }
    else {
        argv[argc++] = message;
        argv[argc++] = options;
    }
    for (int n = 0; n < argc; n++) Jim_IncrRefCount(argv[n]);
    int code = Jim_EvalObjVector(interp, argc, argv);
    printf("RESULT|%s|%d|%d\n", command, mode, code);
    Jim_Obj *names[] = {message, options};
    const char *labels[] = {"MESSAGE", "OPTIONS"};
    for (int n = 0; n < 2; n++) {
        /* Observe type/representation before lookup can reach a name getter. */
        int string = names[n]->bytes != NULL;
        const char *type = names[n]->typePtr ? names[n]->typePtr->name : "none";
        Jim_Obj *lookup = Jim_NewStringObj(interp, n == 0 ? (mode == 1 ? "" : "123") : "456", -1);
        Jim_IncrRefCount(lookup);
        Jim_HashEntry *entry = Jim_FindHashEntry(&interp->topFramePtr->vars, lookup);
        printf("KEY|%s|%d|%s|%d|%s|%d|%d\n", command, mode, labels[n], string, type,
            entry != NULL, entry && Jim_GetHashEntryKey(entry) == names[n]);
        Jim_DecrRefCount(interp, lookup);
    }
    for (int n = 0; n < argc; n++) Jim_DecrRefCount(interp, argv[n]);
    Jim_DecrRefCount(interp, message);
    Jim_DecrRefCount(interp, options);
    Jim_FreeInterp(interp);
    return 0;
}
int main(void)
{
    const char *commands[] = {"catch", "try"};
    for (int n = 0; n < 2; n++) for (int mode = 0; mode < 4; mode++) {
        int code = run(commands[n], mode);
        if (code) return code;
    }
    for (int mode = 0; mode < 3; mode++) {
        int code = global_reset(mode);
        if (code) return code;
    }
    return 0;
}
