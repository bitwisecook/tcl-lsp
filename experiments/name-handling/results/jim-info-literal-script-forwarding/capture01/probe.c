// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include <string.h>
#include "jim.h"
extern int Jim_namespaceInit(Jim_Interp *interp);
static int member, mode;
static void object_hex(Jim_Obj *object)
{
    int length;
    const char *value = Jim_GetString(object, &length);
    for (int i = 0; i < length; i++) printf("%02x", (unsigned char)value[i]);
}
static int observe(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_EvalFrame *frame = interp->evalFrame;
    int line;
    Jim_Obj *filename = Jim_GetSourceInfo(interp, frame->scriptObj, &line);
    printf("OBS|%d|%d|%d|", member, mode, argc);
    object_hex(argv[0]); printf("|"); object_hex(interp->framePtr->nsObj);
    printf("|%d|%d|", frame->level, frame->parent ? frame->parent->argc : -1);
    if (frame->parent && frame->parent->argc) object_hex(frame->parent->argv[0]); else printf("-");
    printf("|%s|%d|", frame->scriptObj->typePtr ? frame->scriptObj->typePtr->name : "none", frame->scriptObj->refCount);
    object_hex(filename); printf("|%d\n", line);
    Jim_SetResultInt(interp, mode == 4 ? 654 : 321);
    return mode == 3 ? JIM_ERR : mode == 6 ? JIM_BREAK : JIM_OK;
}
int main(void)
{
    const char *members[] = {"nameofexecutable", "hostname"};
    const char *helpers[] = {"info nameofexecutable", "os.gethostname"};
    for (member = 0; member < 2; member++) for (mode = 0; mode < 9; mode++) {
        Jim_Interp *interp = Jim_CreateInterp();
        Jim_RegisterCoreCommands(interp);
        int setup = JIM_OK;
        if (mode != 0) Jim_CreateCommand(interp, helpers[member], observe, NULL, NULL);
        if (mode == 2) {
            Jim_namespaceInit(interp);
            char name[80]; snprintf(name, sizeof(name), "N::%s", helpers[member]);
            Jim_CreateCommand(interp, name, observe, NULL, NULL);
        }
        if (mode == 4) Jim_CreateCommand(interp, helpers[member], observe, NULL, NULL);
        if (mode == 5 || mode == 7) {
            char script[160];
            snprintf(script, sizeof(script), "rename {%s} held", helpers[member]);
            setup = Jim_Eval(interp, script);
            if (setup == JIM_OK && mode == 7) {
                snprintf(script, sizeof(script), "proc {%s} {} {return SCRIPTED}", helpers[member]);
                setup = Jim_Eval(interp, script);
            }
        }
        if (mode == 8) setup = Jim_Eval(interp, "rename info inspect");
        printf("SETUP|%d|%d|%d\n", member, mode, setup);
        if (setup != JIM_OK) return 2;
        int code;
        if (mode == 2) {
            char script[120]; snprintf(script, sizeof(script), "namespace eval N {info %s}", members[member]);
            code = Jim_Eval(interp, script);
        }
        else {
            Jim_Obj *words[] = {Jim_NewStringObj(interp, mode == 8 ? "inspect" : "info", -1), Jim_NewStringObj(interp, members[member], -1)};
            for (int i = 0; i < 2; i++) Jim_IncrRefCount(words[i]);
            code = Jim_EvalObjVector(interp, 2, words);
            for (int i = 0; i < 2; i++) Jim_DecrRefCount(interp, words[i]);
        }
        printf("RESULT|%d|%d|%d|", member, mode, code);
        object_hex(Jim_GetResult(interp)); printf("\n");
        Jim_FreeInterp(interp);
    }
    return 0;
}
