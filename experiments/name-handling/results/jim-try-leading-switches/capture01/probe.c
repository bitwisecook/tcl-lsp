// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include "jim.h"
static Jim_Interp *actual;
static int case_no, body_code, bodies, handlers, getters;
static void hex(Jim_Obj *value) {
    if (!value) { printf("-"); return; }
    int len; const char *bytes = Jim_GetString(value, &len);
    for (int i = 0; i < len; i++) printf("%02x", (unsigned char)bytes[i]);
}
static int body(Jim_Interp *interp, int argc, Jim_Obj *const *argv) {
    (void)argc; (void)argv; bodies++;
    Jim_SetResultString(interp, "BODY", -1); return body_code;
}
static int handler(Jim_Interp *interp, int argc, Jim_Obj *const *argv) {
    (void)argc; (void)argv; handlers++;
    Jim_SetResultString(interp, "HANDLED", -1); return JIM_OK;
}
static void update(Jim_Obj *value) {
    const char *bytes = value->internalRep.ptr;
    getters++;
    printf("GETTER|%d|%d|%d|%d|%s|", case_no, body_code, getters, value->refCount, bytes);
    hex(Jim_GetResult(actual)); printf("\n");
    Jim_SetResultString(actual, "GETTER", -1);
    value->length = strlen(bytes); value->bytes = Jim_Alloc(value->length + 1);
    memcpy(value->bytes, bytes, value->length + 1);
}
static const Jim_ObjType type = {"try-switch-probe", NULL, NULL, update, JIM_TYPE_NONE};
static Jim_Obj *word(const char *bytes, int len, int custom) {
    Jim_Obj *value = Jim_NewStringObj(actual, bytes, len);
    if (custom) {
        Jim_InvalidateStringRep(value); value->typePtr = &type;
        value->internalRep.ptr = (void *)bytes;
    }
    return value;
}
struct Case { const char *bytes; int len, custom_selector, custom_body, custom_handler; };
#define CASE(s) {s, sizeof(s)-1, 0, 0, 0}
int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    static const struct Case cases[] = {
        CASE("-no0"), CASE("-0"), CASE("-no64"), CASE("-64"),
        CASE("-no128"), CASE("-128"), CASE("-no63"), CASE("-63"),
        CASE("-no-1"), CASE("--"), CASE("-noerror"), CASE("-error"),
        CASE("-no 1"), CASE("-no+1"), CASE("-no0x1"),
        CASE("-no9223372036854775807"), CASE("-no9223372036854775808"),
        CASE("-no18446744073709551615"), CASE("-no0\0tail"), CASE("--\0tail"),
        {"-no0", 4, 1, 1, 0}, {"--", 2, 1, 1, 0}, {"-bad", 4, 1, 1, 1}
    };
    printf("CONFIG|%d|%d\n", (int)(sizeof(jim_wide)*CHAR_BIT), (int)(sizeof(void*)*CHAR_BIT));
    for (case_no=0; case_no<(int)(sizeof(cases)/sizeof(cases[0])); case_no++) {
        const int codes[] = {0, 1, 63, 64};
        for (int c=0; c<4; c++) {
            body_code=codes[c]; bodies=handlers=getters=0;
            actual=Jim_CreateInterp(); Jim_RegisterCoreCommands(actual);
            if (Jim_InitStaticExtensions(actual)!=JIM_OK) return 2;
            Jim_CreateCommand(actual,"body_probe",body,NULL,NULL);
            Jim_CreateCommand(actual,"handler_probe",handler,NULL,NULL);
            const struct Case *spec=&cases[case_no];
            char code[32]; snprintf(code,sizeof(code),"%d",body_code);
            Jim_Obj *words[]={word("try",-1,0),word(spec->bytes,spec->len,spec->custom_selector),
                word("body_probe",-1,spec->custom_body),word("on",-1,spec->custom_handler),
                word(code,-1,0),word("message options",-1,0),word("handler_probe",-1,0)};
            for(int i=0;i<7;i++) Jim_IncrRefCount(words[i]);
            Jim_SetResultString(actual,"MARKER",-1);
            int reached=Jim_EvalObjVector(actual,7,words);
            Jim_Obj *result=Jim_GetResult(actual); Jim_IncrRefCount(result);
            printf("OBS|%d|%d|%d|%d|%d|%d|",case_no,body_code,reached,bodies,handlers,getters);
            hex(result); printf("|%d|%d|%s\n",Jim_GetGlobalVariableStr(actual,"message",JIM_NONE)!=NULL,
                words[3]->bytes!=NULL,words[3]->typePtr?words[3]->typePtr->name:"none");
            Jim_DecrRefCount(actual,result);
            for(int i=0;i<7;i++) Jim_DecrRefCount(actual,words[i]);
            Jim_FreeInterp(actual);
        }
    }
    return 0;
}
