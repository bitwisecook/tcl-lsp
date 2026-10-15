// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

#include <stdio.h>
#include <string.h>
#include "jim.h"

static int run(const char *kind, int all, int mode)
{
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
    if (Jim_Eval(interp, "proc 123 {} {}; proc 456 {} {}; alias 789 123") != JIM_OK) return 3;
    const char *name = strcmp(kind, "aliases") ? "123" : "789";
    Jim_Obj *lookup = Jim_NewStringObj(interp, name, -1);
    Jim_IncrRefCount(lookup);
    Jim_HashEntry *entry = Jim_FindHashEntry(&interp->commands, lookup);
    if (!entry) return 4;
    Jim_Obj *key = Jim_GetHashEntryKey(entry);
    Jim_IncrRefCount(key);
    Jim_DecrRefCount(interp, lookup);
    jim_wide wide;
    if (Jim_GetWide(interp, key, &wide) != JIM_OK) return 5;
    Jim_InvalidateStringRep(key);
    int refs = key->refCount;
    int before_bytes = key->bytes != NULL;
    const char *before_type = key->typePtr ? key->typePtr->name : "none";
    const char *pattern = mode == 1 ? name : mode == 2 ? "456" :
        strcmp(kind, "aliases") ? "1*" : "7*";
    Jim_Obj *argv[5];
    int argc = 0;
    argv[argc++] = Jim_NewStringObj(interp, "info", -1);
    argv[argc++] = Jim_NewStringObj(interp, "-nons", -1);
    argv[argc++] = Jim_NewStringObj(interp, kind, -1);
    if (all) argv[argc++] = Jim_NewStringObj(interp, "-all", -1);
    if (mode) argv[argc++] = Jim_NewStringObj(interp, pattern, -1);
    for (int n = 0; n < argc; n++) Jim_IncrRefCount(argv[n]);
    int code = Jim_EvalObjVector(interp, argc, argv);
    Jim_Obj *result = Jim_GetResult(interp);
    int count = Jim_ListLength(interp, result);
    int found = 0;
    for (int n = 0; n < count; n++) {
        Jim_Obj *member;
        if (Jim_ListIndex(interp, result, n, &member, JIM_ERRMSG) != JIM_OK) return 6;
        if (member == key) found++;
    }
    printf("KEY|%s|%d|%d|%d|%d|%s|%d|%d|%s|%d|%d|%d",
        kind, all, mode, code, before_bytes, before_type, refs,
        key->bytes != NULL, key->typePtr ? key->typePtr->name : "none",
        key->refCount, found, count);
    Jim_SetResultString(interp, "", 0);
    printf("|%d\n", key->refCount);
    for (int n = argc - 1; n >= 0; n--) Jim_DecrRefCount(interp, argv[n]);
    Jim_DecrRefCount(interp, key);
    Jim_FreeInterp(interp);
    return 0;
}

int main(void)
{
    const char *kinds[] = {"commands", "procs", "aliases"};
    for (int kind = 0; kind < 3; kind++) {
        for (int all = 0; all < 2; all++) {
            for (int mode = 0; mode < 4; mode++) {
                int code = run(kinds[kind], all, mode);
                if (code) return code;
            }
        }
    }
    return 0;
}
