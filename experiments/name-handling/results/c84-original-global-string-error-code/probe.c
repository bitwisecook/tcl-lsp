// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include <string.h>
#include "tclInt.h"
static int mode, trace_count;
static CallFrame *caller;
static Namespace *space;
static void hex(const char *bytes, int length) {
    for (int i = 0; i < length; i++) printf("%02x", (unsigned char)bytes[i]);
}
static void object(Tcl_Obj *value) {
    if (!value) { printf("-"); return; }
    int length; const char *data = Tcl_GetStringFromObj(value, &length); hex(data, length);
}
static char *trace(ClientData data, Tcl_Interp *interp, CONST char *name1, CONST char *name2, int flags) {
    (void)data;
    Interp *actual = (Interp *)interp; trace_count++;
    printf("TRACE|%d|%d|", mode, trace_count); hex(name1, strlen(name1)); printf("|");
    if (name2) hex(name2, strlen(name2)); else printf("-");
    printf("|%d|%d|%d|%d\n", !!(flags & TCL_GLOBAL_ONLY), actual->varFramePtr == caller,
        actual->varFramePtr->nsPtr == space, !!(actual->flags & ERROR_CODE_SET));
    if (mode == 4) Tcl_SetVar2Ex(interp, "errorCode", NULL, Tcl_NewStringObj("REPLACED", -1), TCL_GLOBAL_ONLY);
    if (mode == 5) Tcl_SetObjResult(interp, Tcl_NewStringObj("TRACE_RESULT", -1));
    return mode == 2 || mode == 5 ? "TRACEFAIL" : NULL;
}
static int probe(ClientData data, Tcl_Interp *interp, int argc, Tcl_Obj *CONST argv[]) {
    (void)data; (void)argc; (void)argv;
    Interp *actual = (Interp *)interp; caller = actual->varFramePtr; space = caller->nsPtr;
    Tcl_Obj *local = Tcl_GetVar2Ex(interp, "errorCode", NULL, 0);
    Tcl_Obj *supplied = Tcl_NewIntObj(777); Tcl_IncrRefCount(supplied);
    Tcl_SetObjResult(interp, Tcl_NewStringObj("MARKER", -1));
    int before_flag = !!(actual->flags & ERROR_CODE_SET);
    Tcl_SetObjErrorCode(interp, supplied);
    Tcl_Obj *global = Tcl_GetVar2Ex(interp, "errorCode", NULL, TCL_GLOBAL_ONLY);
    printf("OBS|%d|%d|%d|%d|%d|%d|%d|%d|%s|%d|", mode, trace_count, before_flag,
        !!(actual->flags & ERROR_CODE_SET), actual->varFramePtr == caller,
        actual->varFramePtr->nsPtr == space, Tcl_GetVar2Ex(interp, "errorCode", NULL, 0) == local,
        global == supplied, supplied->typePtr ? supplied->typePtr->name : "none", supplied->bytes != NULL);
    object(Tcl_GetObjResult(interp)); printf("|"); object(global); printf("\n");
    Tcl_DecrRefCount(supplied);
    return TCL_OK;
}
int main(int argc, char **argv) {
    (void)argc; Tcl_FindExecutable(argv[0]); setvbuf(stdout, NULL, _IONBF, 0);
    for (mode = 0; mode < 6; mode++) {
        Tcl_Interp *interp = Tcl_CreateInterp(); if (Tcl_Init(interp) != TCL_OK) { object(Tcl_GetObjResult(interp)); return 2; }
        if (Tcl_Eval(interp, "unset -nocomplain ::errorCode; namespace eval N {variable errorCode NAMESPACE; proc p {} {set errorCode LOCAL; probe; return $errorCode}}") != TCL_OK) return 3;
        if (mode == 3 && Tcl_Eval(interp, "array set ::errorCode {key VALUE}") != TCL_OK) return 4;
        trace_count = 0;
        if (mode != 0 && mode != 3 && Tcl_TraceVar(interp, "errorCode", TCL_GLOBAL_ONLY|TCL_TRACE_WRITES, trace, NULL) != TCL_OK) return 5;
        Tcl_CreateObjCommand(interp, "probe", probe, NULL, NULL);
        int code = Tcl_Eval(interp, "N::p");
        printf("CALL|%d|%d|", mode, code); object(Tcl_GetObjResult(interp)); printf("|");
        object(Tcl_GetVar2Ex(interp, "N::errorCode", NULL, TCL_GLOBAL_ONLY)); printf("\n");
        Tcl_DeleteInterp(interp);
    }
    Tcl_Finalize(); return 0;
}
