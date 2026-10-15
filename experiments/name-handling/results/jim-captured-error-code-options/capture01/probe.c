// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include <string.h>
#include "jim.h"
#include "jim-subcmd.h"
static void hex(const char *value, int length)
{
    for (int i = 0; i < length; i++) printf("%02x", (unsigned char)value[i]);
}
int main(void)
{
    const char *body = "set ::errorCode SAVED; set ::keep $::errorCode; error BOOM";
    for (int mode = 0; mode < 2; mode++) {
        Jim_Interp *interp = Jim_CreateInterp();
        Jim_RegisterCoreCommands(interp);
        Jim_InitStaticExtensions(interp);
        Jim_Obj *words[] = {Jim_NewStringObj(interp, mode ? "try" : "catch", -1),
            Jim_NewStringObj(interp, body, -1), Jim_NewStringObj(interp, mode ? "on" : "::errorCode", -1),
            Jim_NewStringObj(interp, mode ? "error" : "opts", -1),
            Jim_NewStringObj(interp, "::errorCode opts", -1), Jim_NewStringObj(interp, "set HANDLED YES", -1)};
        int count = mode ? 6 : 4;
        for (int i = 0; i < count; i++) Jim_IncrRefCount(words[i]);
        int code = Jim_EvalObjVector(interp, count, words);
        int result_length;
        const char *result = Jim_GetString(Jim_GetResult(interp), &result_length);
        printf("RESULT|%d|%d|", mode, code); hex(result, result_length); printf("\n");
        Jim_Obj *kept = Jim_GetGlobalVariableStr(interp, "keep", JIM_NONE);
        Jim_Obj *current = Jim_GetGlobalVariableStr(interp, "errorCode", JIM_NONE);
        Jim_Obj *options = Jim_GetGlobalVariableStr(interp, "opts", JIM_NONE);
        Jim_Obj *captured = NULL;
        for (int i = 0; options && i + 1 < Jim_ListLength(interp, options); i += 2) {
            Jim_Obj *key = Jim_ListGetIndex(interp, options, i);
            if (!strcmp(Jim_String(key), "-errorcode")) captured = Jim_ListGetIndex(interp, options, i + 1);
        }
        printf("CAPTURE|%d|%d|%d|%d|", mode, captured != NULL, captured == kept, captured == current);
        int length;
        const char *value = captured ? Jim_GetString(captured, &length) : NULL;
        if (value) hex(value, length); else printf("-");
        printf("\n");
        printf("CURRENT|%d|", mode);
        value = current ? Jim_GetString(current, &length) : NULL;
        if (value) hex(value, length); else printf("-");
        printf("\n");
        for (int i = 0; i < count; i++) Jim_DecrRefCount(interp, words[i]);
        Jim_FreeInterp(interp);
    }
    return 0;
}
