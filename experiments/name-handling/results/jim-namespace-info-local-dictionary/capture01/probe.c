// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include "jim.h"
static const char *type(Jim_Obj *obj) {return obj->typePtr ? obj->typePtr->name : "none";}
static void bytes(Jim_Obj *obj) {int length; const char *s = Jim_GetString(obj, &length); for (int n=0; n<length; n++) printf("%02x", (unsigned char)s[n]);}
static void observe(Jim_Interp *interp, const char *label, int code, Jim_Obj *original)
{
    Jim_Obj *result = Jim_GetResult(interp);
    printf("HEADER|%s|%d|%s|%d|%d\n", label, code, type(result), result->bytes != NULL, result->refCount);
    int count = Jim_ListLength(interp, result);
    printf("LIST|%s|%d\n", label, count);
    for (int n=0; n<count; n++) {
        Jim_Obj *member = Jim_ListGetIndex(interp, result, n);
        printf("MEMBER|%s|%d|%s|%d|%d|%d|", label, n, type(member), member->bytes != NULL, member->refCount, original ? member == original : -1);
        bytes(member); printf("\n");
    }
    printf("RESULT|%s|", label); bytes(result); printf("\n");
}
static int local(const char *label, int empty)
{
    Jim_Interp *interp = Jim_CreateInterp(); Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
    const char *setup = "proc LocalReceiptP {} {}; namespace eval LocalReceiptScope {proc LocalReceiptP {} {}}";
    if (Jim_Eval(interp, setup) != JIM_OK) return 3;
    const char *input = empty ? "namespace eval LocalReceiptScope {{namespace info} commands MissingReceiptP*}" : "namespace eval LocalReceiptScope {{namespace info} commands LocalReceiptP*}";
    observe(interp, label, Jim_Eval(interp, input), NULL);
    Jim_FreeInterp(interp); return 0;
}
static int dictionary(const char *label, int pattern)
{
    Jim_Interp *interp = Jim_CreateInterp(); Jim_RegisterCoreCommands(interp);
    Jim_Obj *member = Jim_NewIntObj(interp, 123);
    Jim_Obj *key = Jim_NewListObj(interp, &member, 1);
    Jim_Obj *dict = Jim_NewDictObj(interp, NULL, 0);
    if (Jim_DictAddElement(interp, dict, key, Jim_NewIntObj(interp, 1)) != JIM_OK) return 4;
    if (Jim_SetVariableStr(interp, "d", dict) != JIM_OK) return 5;
    /* Original public API deliberately removes only this real table key's String cache. */
    Jim_InvalidateStringRep(key);
    printf("KEY_BEFORE|%s|%s|%d|%d\n", label, type(key), key->bytes != NULL, key->refCount);
    observe(interp, label, Jim_Eval(interp, pattern ? "dict keys $d *" : "dict keys $d"), key);
    Jim_FreeInterp(interp); return 0;
}
int main(void)
{
    int code;
    if ((code=local("LOCAL_DUPLICATE",0))) return code;
    if ((code=local("LOCAL_EMPTY",1))) return code;
    if ((code=dictionary("KEYS_NO_PATTERN",0))) return code;
    return dictionary("KEYS_PATTERN",1);
}
