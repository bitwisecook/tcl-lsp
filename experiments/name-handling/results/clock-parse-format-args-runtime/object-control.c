#include "tclInt.h"
#include <stdio.h>

int main(int argc, char **argv) {
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *i = Tcl_CreateInterp();
    if (Tcl_Init(i) != TCL_OK) {
        fprintf(stderr, "%s\n", Tcl_GetStringResult(i));
        return 1;
    }
    const char *name = "::tcl::clock::ParseFormatArgs";
    Command *worker = (Command *)Tcl_FindCommand(i, name, NULL, TCL_GLOBAL_ONLY);
    printf("worker-present %d null-hook %d\n", worker != NULL, worker && !worker->compileProc);
    Tcl_Obj *defaults[2] = {Tcl_NewStringObj(name, -1), Tcl_NewIntObj(0)};
    Tcl_IncrRefCount(defaults[0]);
    Tcl_IncrRefCount(defaults[1]);
    if (Tcl_EvalObjv(i, 2, defaults, TCL_EVAL_GLOBAL) != TCL_OK) return 2;
    Tcl_Obj *first = Tcl_GetObjResult(i);
    Tcl_IncrRefCount(first);
    Tcl_Obj **members;
    int count;
    Tcl_ListObjGetElements(i, first, &count, &members);
    printf("first-length %d literal-refcount %d\n", count, members[0]->refCount);
    if (Tcl_EvalObjv(i, 2, defaults, TCL_EVAL_GLOBAL) != TCL_OK) return 3;
    Tcl_Obj *second = Tcl_GetObjResult(i);
    Tcl_IncrRefCount(second);
    Tcl_Obj **again;
    Tcl_ListObjGetElements(i, second, &count, &again);
    printf("same-literals %d literal-refcount %d\n",
        again[0] == members[0] && again[1] == members[1] && again[2] == members[2], members[0]->refCount);
    const char *texts[] = {name, "0", "-format", "supplied", "-locale", "custom", "-timezone", ":UTC"};
    Tcl_Obj *words[8];
    for (int n = 0; n < 8; ++n) {
        words[n] = Tcl_NewStringObj(texts[n], -1);
        Tcl_IncrRefCount(words[n]);
    }
    if (Tcl_EvalObjv(i, 8, words, TCL_EVAL_GLOBAL) != TCL_OK) return 4;
    Tcl_Obj *supplied = Tcl_GetObjResult(i);
    Tcl_IncrRefCount(supplied);
    Tcl_Obj **originals;
    Tcl_ListObjGetElements(i, supplied, &count, &originals);
    printf("same-supplied %d\n", originals[0] == words[3] && originals[1] == words[5] && originals[2] == words[7]);
    Tcl_DeleteCommand(i, name);
    printf("after-worker-delete literal-refcount %d value %s supplied %s\n",
        members[0]->refCount, Tcl_GetString(members[0]), Tcl_GetString(originals[0]));
    const char *others[] = {"getenv", "Oldscan", "ConvertLocalToUTC", "GetDateFields", "GetJulianDayFromEraYearMonthDay", "GetJulianDayFromEraYearWeekDay"};
    for (int n = 0; n < 6; ++n) {
        char path[100];
        snprintf(path, sizeof(path), "::tcl::clock::%s", others[n]);
        Tcl_DeleteCommand(i, path);
    }
    printf("after-pool-retirement literal-refcount %d value %s supplied %s\n",
        members[0]->refCount, Tcl_GetString(members[0]), Tcl_GetString(originals[0]));
    for (int n = 0; n < 8; ++n) Tcl_DecrRefCount(words[n]);
    Tcl_DecrRefCount(defaults[0]);
    Tcl_DecrRefCount(defaults[1]);
    Tcl_DecrRefCount(first);
    Tcl_DecrRefCount(second);
    Tcl_DecrRefCount(supplied);
    Tcl_DeleteInterp(i);
    Tcl_Finalize();
    return 0;
}
