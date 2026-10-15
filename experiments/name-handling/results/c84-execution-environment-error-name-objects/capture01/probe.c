// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include <string.h>
#include "tclInt.h"
static void hex(const char *bytes, int length) {
    for (int i = 0; i < length; i++) printf("%02x", (unsigned char)bytes[i]);
}
static void header(Tcl_Obj *value) {
    printf("%d|%s|%d|", value->refCount,
        value->typePtr ? value->typePtr->name : "none", value->bytes != NULL);
    if (value->bytes) hex(value->bytes, value->length); else printf("-");
}
static void environment(const char *phase, Tcl_Interp *interp, Tcl_Obj *info, Tcl_Obj *code) {
    Interp *actual = (Interp *)interp;
    ExecEnv *env = actual->execEnvPtr;
    printf("ENV|%s|%d|%d|%d|%d|", phase, env != NULL, actual->globalNsPtr != NULL,
        env->errorInfo == info, env->errorCode == code);
    header(env->errorInfo); printf("|"); header(env->errorCode);
    printf("|%d\n", env->stackTop);
}
int main(int argc, char **argv) {
    (void)argc; Tcl_FindExecutable(argv[0]); setvbuf(stdout, NULL, _IONBF, 0);
    Tcl_Interp *interp = Tcl_CreateInterp();
    ExecEnv *env = ((Interp *)interp)->execEnvPtr;
    Tcl_Obj *info = env->errorInfo, *code = env->errorCode;
    environment("birth-before-init", interp, info, code);
    printf("DISTINCT|same-interpreter|%d\n", info != code);
    if (Tcl_Init(interp) != TCL_OK) return 2;
    environment("after-init", interp, info, code);
    Tcl_SetObjResult(interp, Tcl_NewIntObj(777));
    Tcl_AddObjErrorInfo(interp, " INFO", -1);
    environment("after-seed-default-append", interp, info, code);
    Tcl_AddObjErrorInfo(interp, " MORE", -1);
    environment("after-second-append", interp, info, code);
    Tcl_ResetResult(interp);
    environment("after-reset", interp, info, code);
    int status = Tcl_Eval(interp, "namespace eval N {proc p {} {error FAIL}}; N::p");
    printf("SCRIPT|nested-error|%d\n", status);
    environment("after-nested-error", interp, info, code);
    Tcl_Interp *child = Tcl_CreateInterp();
    ExecEnv *child_env = ((Interp *)child)->execEnvPtr;
    environment("independent-birth", child, child_env->errorInfo, child_env->errorCode);
    printf("DISTINCT|independent-interpreters|%d|%d|%d|%d\n",
        info != child_env->errorInfo, info != child_env->errorCode,
        code != child_env->errorInfo, code != child_env->errorCode);
    Tcl_DeleteInterp(child);
    printf("DELETE|independent-interpreter|completed\n");
    /* Explicit observation roles keep headers alive after their owner dies. */
    Tcl_IncrRefCount(info); Tcl_IncrRefCount(code);
    environment("with-observation-roles", interp, info, code);
    Tcl_DeleteInterp(interp);
    printf("AFTER_DELETE|observer-held|1|1|");
    header(info); printf("|"); header(code); printf("\n");
    Tcl_DecrRefCount(info); Tcl_DecrRefCount(code);
    Tcl_Finalize(); return 0;
}
