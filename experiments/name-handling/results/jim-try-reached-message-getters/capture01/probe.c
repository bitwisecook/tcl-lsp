// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include <string.h>
#include "jim.h"
static Jim_Interp *actual;
static int change_result, getter_count;
static void hex(Jim_Obj *value) {
    if (!value) { printf("-"); return; }
    int length; const char *data = Jim_GetString(value, &length);
    for (int i = 0; i < length; i++) printf("%02x", (unsigned char)data[i]);
}
static void update(Jim_Obj *value) {
    const char *text = value->internalRep.ptr;
    getter_count++;
    if (change_result) Jim_SetResultString(actual, !strcmp(text, "777") ? "TRAP" : "CLAUSE", -1);
    value->length = strlen(text);
    value->bytes = Jim_Alloc(value->length + 1);
    memcpy(value->bytes, text, value->length + 1);
}
static const Jim_ObjType type = {"try-stage-probe", NULL, NULL, update, JIM_TYPE_NONE};
static Jim_Obj *word(const char *text, int controlled) {
    Jim_Obj *value = Jim_NewStringObj(actual, text, -1);
    if (controlled) { Jim_InvalidateStringRep(value); value->typePtr = &type; value->internalRep.ptr = (void *)text; }
    return value;
}
static int body(Jim_Interp *interp, int argc, Jim_Obj *const *argv) {
    (void)argc; (void)argv;
    Jim_Obj *member = Jim_NewStringObj(interp, "777", -1);
    Jim_Obj *errorcode = Jim_NewListObj(interp, &member, 1);
    if (Jim_SetGlobalVariableStr(interp, "errorCode", errorcode) != JIM_OK) return JIM_ERR;
    Jim_SetResultString(interp, "BODY", -1);
    return JIM_ERR;
}
int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    for (int mode = 0; mode < 9; mode++) {
        actual = Jim_CreateInterp(); Jim_RegisterCoreCommands(actual);
        if (Jim_InitStaticExtensions(actual) != JIM_OK) return 2;
        Jim_CreateCommand(actual, "body_error", body, NULL, NULL);
        change_result = mode == 2 || mode == 5 || mode == 6 || mode == 7 || mode == 8;
        getter_count = 0;
        int trap = mode >= 3;
        Jim_Obj *members[] = {word("777", mode >= 4), word("888", 0)};
        Jim_Obj *pattern = Jim_NewListObj(actual, members, mode == 8 ? 2 : 1);
        Jim_Obj *words[10] = {word("try", 0), word("body_error", 0),
            word(trap ? "trap" : "on", mode == 1 || mode == 2 || mode == 6),
            trap ? pattern : word("error", 0), word("message options", 0), word("set ::handled $::message", 0)};
        int count = 6;
        if (mode == 7) {
            words[2] = word("on", 0); words[3] = word("error", 0);
            words[6] = word("trap", 0); words[7] = pattern;
            words[8] = word("other options2", 0); words[9] = word("set ::handled WRONG", 0);
            count = 10;
        }
        for (int i = 0; i < count; i++) Jim_IncrRefCount(words[i]);
        int code = Jim_EvalObjVector(actual, count, words);
        Jim_Obj *result = Jim_GetResult(actual); Jim_IncrRefCount(result);
        printf("OBS|%d|%d|%d|", mode, code, getter_count); hex(result); printf("|");
        hex(Jim_GetGlobalVariableStr(actual, "message", JIM_NONE)); printf("|");
        hex(Jim_GetGlobalVariableStr(actual, "handled", JIM_NONE)); printf("|");
        hex(Jim_GetGlobalVariableStr(actual, "other", JIM_NONE)); printf("\n");
        Jim_DecrRefCount(actual, result);
        for (int i = 0; i < count; i++) Jim_DecrRefCount(actual, words[i]);
        Jim_FreeInterp(actual);
    }
    return 0;
}
