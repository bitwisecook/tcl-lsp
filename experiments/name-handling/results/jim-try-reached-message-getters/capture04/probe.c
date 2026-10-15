// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include <string.h>
#include "jim.h"
static Jim_Interp *actual;
static int change_result, getter_count, current_mode;
static Jim_Obj *selected_clause, *selected_member;
static void hex(Jim_Obj *value) {
    if (!value) { printf("-"); return; }
    int length; const char *data = Jim_GetString(value, &length);
    for (int i = 0; i < length; i++) printf("%02x", (unsigned char)data[i]);
}
static void update(Jim_Obj *value) {
    const char *text = value->internalRep.ptr;
    getter_count++;
    printf("GETTER_BEFORE|%d|%d|%d|%d|%s|%d|", current_mode, getter_count,
        value == selected_clause, value == selected_member, value->typePtr->name, value->refCount);
    Jim_Obj *before = Jim_GetResult(actual);
    if (before->bytes) for (int i = 0; i < before->length; i++) printf("%02x", (unsigned char)before->bytes[i]);
    else printf("-");
    printf("\n");
    if (change_result) Jim_SetResultString(actual, !strcmp(text, "777") ? "TRAP" : "CLAUSE", -1);
    printf("GETTER_AFTER|%d|%d|", current_mode, getter_count);
    Jim_Obj *after = Jim_GetResult(actual);
    if (after->bytes) for (int i = 0; i < after->length; i++) printf("%02x", (unsigned char)after->bytes[i]);
    else printf("-");
    printf("\n");
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
    for (int mode = 0; mode < 13; mode++) {
        current_mode = mode;
        actual = Jim_CreateInterp(); Jim_RegisterCoreCommands(actual);
        if (Jim_InitStaticExtensions(actual) != JIM_OK) return 2;
        Jim_CreateCommand(actual, "body_error", body, NULL, NULL);
        change_result = mode == 2 || mode == 5 || mode == 6 || mode == 7 || mode == 8 || mode == 9 || mode == 10;
        getter_count = 0;
        int trap = mode >= 3;
        Jim_Obj *members[] = {word("777", mode >= 4), word("888", 0)};
        if (mode == 12) members[0] = Jim_NewIntObj(actual, 777);
        Jim_Obj *pattern = Jim_NewListObj(actual, members, mode == 8 || mode == 12 ? 2 : 1);
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
        if (mode == 9 || mode == 10 || mode == 11) {
            if (mode == 10 || mode == 11) { words[2] = word("on", 0); words[3] = word("error", 0); }
            words[6] = word("on", mode == 9); words[7] = word("error", 0);
            words[8] = word("other options2", 0); words[9] = word("set ::handled WRONG", mode == 10);
            if (mode == 11) words[9] = Jim_NewIntObj(actual, 777);
            count = 10;
        }
        selected_clause = mode == 9 ? words[6] : words[2]; selected_member = members[0];
        for (int i = 0; i < count; i++) Jim_IncrRefCount(words[i]);
        int code = Jim_EvalObjVector(actual, count, words);
        Jim_Obj *result = Jim_GetResult(actual); Jim_IncrRefCount(result);
        printf("OBS|%d|%d|%d|", mode, code, getter_count); hex(result); printf("|");
        hex(Jim_GetGlobalVariableStr(actual, "message", JIM_NONE)); printf("|");
        hex(Jim_GetGlobalVariableStr(actual, "handled", JIM_NONE)); printf("|");
        hex(Jim_GetGlobalVariableStr(actual, "other", JIM_NONE)); printf("\n");
        if (mode == 10 || mode == 11) printf("UNSELECTED_SCRIPT|%d|%d|%s\n", mode, words[9]->bytes != NULL, words[9]->typePtr ? words[9]->typePtr->name : "none");
        if (mode == 12) printf("UNREACHED_MEMBER|%d|%d|%s\n", mode, members[0]->bytes != NULL, members[0]->typePtr ? members[0]->typePtr->name : "none");
        Jim_DecrRefCount(actual, result);
        for (int i = 0; i < count; i++) Jim_DecrRefCount(actual, words[i]);
        Jim_FreeInterp(actual);
    }
    return 0;
}
