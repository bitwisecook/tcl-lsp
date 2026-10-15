// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include "jim.h"
static void bytes(Jim_Obj *obj) {
    if (!obj) { printf("-"); return; }
    int length; const char *data = Jim_GetString(obj, &length);
    for (int n = 0; n < length; n++) printf("%02x", (unsigned char)data[n]);
}
int main(void) {
    const char *bodies[] = {
        "unset -nocomplain ::errorCode; error BODY",
        "set ::errorCode $::keep; error BODY",
        "unset -nocomplain ::errorCode; upvar #0 keep errorCode; error BODY",
        "unset -nocomplain ::errorCode; set ::scalar ODD; upvar #0 scalar(member) errorCode; error BODY",
        "set ::errorCode $::keep; set ::value OK"
    };
    const char *forms[] = {
        "catch {%s} message options",
        "try {%s} on error {message options} {set ::handler REACHED}",
        "try {%s} on error {message options} {set ::handler REACHED} finally {set ::finally REACHED}",
        "catch {%s} ::errorCode options",
        "try {%s} on ok {::errorCode options} {set ::handler REACHED}"
    };
    for (int form = 0; form < 5; form++) for (int mode = 0; mode < 5; mode++) {
        Jim_Interp *interp = Jim_CreateInterp(); Jim_RegisterCoreCommands(interp);
        if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
        if (Jim_Eval(interp, "set keep SAVED; set handler NONE; set finally NONE") != JIM_OK) return 3;
        Jim_Obj *saved = Jim_GetGlobalVariableStr(interp, "keep", JIM_ERRMSG);
        char script[1024]; snprintf(script, sizeof(script), forms[form], bodies[mode]);
        printf("INPUT|%d|%d|", form, mode);
        for (const unsigned char *p = (const unsigned char *)script; *p; p++) printf("%02x", *p);
        printf("\n");
        int code = Jim_Eval(interp, script);
        Jim_Obj *result = Jim_GetResult(interp); Jim_IncrRefCount(result);
        Jim_Obj *message = Jim_GetGlobalVariableStr(interp, form >= 3 ? "errorCode" : "message", JIM_NONE);
        Jim_Obj *options = Jim_GetGlobalVariableStr(interp, "options", JIM_NONE);
        Jim_Obj *errorcode = NULL;
        if (options) {
            Jim_Obj *key = Jim_NewStringObj(interp, "-errorcode", -1); Jim_IncrRefCount(key);
            Jim_DictKey(interp, options, key, &errorcode, JIM_NONE); Jim_DecrRefCount(interp, key);
        }
        printf("OBS|%d|%d|%d|", form, mode, code); bytes(result); printf("|"); bytes(message);
        printf("|"); bytes(options); printf("|%d|", errorcode == saved); bytes(errorcode);
        printf("|"); bytes(Jim_GetGlobalVariableStr(interp,"handler",JIM_NONE));
        printf("|"); bytes(Jim_GetGlobalVariableStr(interp,"finally",JIM_NONE)); printf("\n");
        Jim_DecrRefCount(interp, result); Jim_FreeInterp(interp);
    }
    return 0;
}
