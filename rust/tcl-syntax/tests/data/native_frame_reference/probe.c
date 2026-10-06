#include "tclInt.h"
#include <stdio.h>

static void record(const char *window, Tcl_Interp *interp, Tcl_Obj *level) {
    CallFrame *target = NULL;
    int code;
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
    code = TclGetFrame(interp, Tcl_GetString(level), &target);
#else
    code = TclObjGetFrame(interp, level, &target);
#endif
    printf("%s\t%d\t%d\t%s\t%d\t%d\t%d\t%d\t%d\t%d\n", window,
        code, code < 0 ? -1 : (target ? (int)target->level : 0),
        level->typePtr ? level->typePtr->name : "none", level->bytes != NULL,
        level->refCount,
        level->typePtr && level->typePtr->freeIntRepProc != NULL,
        level->typePtr && level->typePtr->dupIntRepProc != NULL,
        level->typePtr && level->typePtr->updateStringProc != NULL,
        level->typePtr && level->typePtr->setFromAnyProc != NULL);
}

int main(void) {
    Tcl_Interp *interp = Tcl_CreateInterp();
    Tcl_Interp *other = Tcl_CreateInterp();
    Tcl_Obj *level = Tcl_NewStringObj("#1", 2), *copy;
    Tcl_CallFrame first, replacement, foreign;
    Tcl_IncrRefCount(level);
    record("missing", interp, level);
    if (Tcl_PushCallFrame(interp, &first, Tcl_GetGlobalNamespace(interp), 1) != TCL_OK) return 2;
    record("first-frame", interp, level);
    Tcl_PopCallFrame(interp);
    record("retired-frame", interp, level);
    if (Tcl_PushCallFrame(interp, &replacement, Tcl_GetGlobalNamespace(interp), 1) != TCL_OK) return 3;
    record("replacement-frame", interp, level);
    if (Tcl_PushCallFrame(other, &foreign, Tcl_GetGlobalNamespace(other), 1) != TCL_OK) return 4;
    record("other-interpreter", other, level);
    copy = Tcl_DuplicateObj(level);
    Tcl_IncrRefCount(copy);
    Tcl_DecrRefCount(level);
    record("duplicate-after-original-release", other, copy);
    Tcl_DecrRefCount(copy);
    Tcl_PopCallFrame(other);
    Tcl_PopCallFrame(interp);
    Tcl_DeleteInterp(other);
    Tcl_DeleteInterp(interp);
    return 0;
}
