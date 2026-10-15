// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include <string.h>
#include "jim.h"
static Jim_Interp *actual;
static int mode, getters, is_try;
static void hex(Jim_Obj *value) {
    if (!value) { printf("-"); return; }
    int len; const char *bytes = Jim_GetString(value, &len);
    for (int i = 0; i < len; i++) printf("%02x", (unsigned char)bytes[i]);
}
static void update(Jim_Obj *value) {
    const char *name = value->internalRep.ptr;
    getters++;
    printf("BEFORE|%d|%d|%d|%d|", is_try, mode, getters, value->refCount);
    hex(Jim_GetResult(actual)); printf("\n");
    Jim_SetResultString(actual, mode == 1 ? "MSGNAME" : "OPTNAME", -1);
    if (mode == 3) {
        /* The captured code is unused because this options name is empty. */
        Jim_SetGlobalVariableStr(actual, "errorCode", Jim_NewStringObj(actual, "REPLACED", -1));
    }
    value->length = strlen(name);
    value->bytes = Jim_Alloc(value->length + 1);
    memcpy(value->bytes, name, value->length + 1);
    printf("AFTER|%d|%d|%d|", is_try, mode, getters);
    hex(Jim_GetResult(actual)); printf("\n");
}
static const Jim_ObjType type = {"catch-stage-probe", NULL, NULL, update, JIM_TYPE_NONE};
static Jim_Obj *word(const char *text, int custom) {
    Jim_Obj *value = Jim_NewStringObj(actual, text, -1);
    if (custom) {
        Jim_InvalidateStringRep(value);
        value->typePtr = &type;
        value->internalRep.ptr = (void *)text;
    }
    return value;
}
static int body(Jim_Interp *interp, int argc, Jim_Obj *const *argv) {
    (void)argc; (void)argv;
    Jim_Obj *member = Jim_NewStringObj(interp, "777", -1);
    Jim_Obj *code = Jim_NewListObj(interp, &member, 1);
    if (Jim_SetGlobalVariableStr(interp, "errorCode", code) != JIM_OK) return JIM_ERR;
    Jim_SetResultString(interp, "BODY", -1);
    return JIM_ERR;
}
int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    for (is_try = 0; is_try < 2; is_try++) for (mode = 0; mode < 4; mode++) {
        actual = Jim_CreateInterp(); Jim_RegisterCoreCommands(actual);
        if (Jim_InitStaticExtensions(actual) != JIM_OK) return 2;
        Jim_CreateCommand(actual, "body_error", body, NULL, NULL);
        getters = 0;
        Jim_Obj *message = word("message", mode == 1);
        Jim_Obj *options = word(mode == 3 ? "" : "options", mode >= 2);
        Jim_Obj *words[6] = {word(is_try ? "try" : "catch", 0), word("body_error", 0)};
        int count;
        if (is_try) {
            Jim_Obj *names[] = {message, options};
            words[2] = word("on", 0); words[3] = word("error", 0);
            words[4] = Jim_NewListObj(actual, names, 2);
            words[5] = word("set ::handled $::message", 0);
            count = 6;
        } else {
            words[2] = message; words[3] = options; count = 4;
        }
        for (int i = 0; i < count; i++) Jim_IncrRefCount(words[i]);
        int code = Jim_EvalObjVector(actual, count, words);
        Jim_Obj *result = Jim_GetResult(actual); Jim_IncrRefCount(result);
        printf("OBS|%d|%d|%d|%d|", is_try, mode, code, getters); hex(result); printf("|");
        hex(Jim_GetGlobalVariableStr(actual, "message", JIM_NONE)); printf("|");
        hex(Jim_GetGlobalVariableStr(actual, "handled", JIM_NONE)); printf("|");
        printf("%d\n", Jim_GetGlobalVariableStr(actual, "options", JIM_NONE) != NULL);
        Jim_DecrRefCount(actual, result);
        for (int i = 0; i < count; i++) Jim_DecrRefCount(actual, words[i]);
        Jim_FreeInterp(actual);
    }
    return 0;
}
