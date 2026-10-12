#include "tclInt.h"
#include <stdio.h>
#include <string.h>

static int invoke(Tcl_Interp *i, const char *command, const char *argument) {
    Tcl_Obj *words[2] = {Tcl_NewStringObj(command, -1), Tcl_NewStringObj(argument, -1)};
    Tcl_IncrRefCount(words[0]);
    Tcl_IncrRefCount(words[1]);
    int code = Tcl_EvalObjv(i, 2, words, TCL_EVAL_GLOBAL);
    Tcl_DecrRefCount(words[0]);
    Tcl_DecrRefCount(words[1]);
    return code;
}

int main(int argc, char **argv) {
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *i = Tcl_CreateInterp();
    if (Tcl_Init(i) != TCL_OK) {
        fprintf(stderr, "%s\n", Tcl_GetStringResult(i));
        return 1;
    }
    const char *parser = "::tcl::clock::ParseFormatArgs";
    const char *getter = "::tcl::clock::getenv";
    Command *p = (Command *)Tcl_FindCommand(i, parser, NULL, TCL_GLOBAL_ONLY);
    Command *g = (Command *)Tcl_FindCommand(i, getter, NULL, TCL_GLOBAL_ONLY);
    Tcl_CmdInfo pi, gi;
    Tcl_GetCommandInfo(i, parser, &pi);
    Tcl_GetCommandInfo(i, getter, &gi);
    printf("present %d %d null-hooks %d %d same-clientdata %d\n",
        p != NULL, g != NULL, p && !p->compileProc, g && !g->compileProc,
        pi.objClientData == gi.objClientData);
    const char *others[] = {"Oldscan", "ConvertLocalToUTC", "GetDateFields",
        "GetJulianDayFromEraYearMonthDay", "GetJulianDayFromEraYearWeekDay"};
    for (int n = 0; n < 5; ++n) {
        char name[100];
        snprintf(name, sizeof(name), "::tcl::clock::%s", others[n]);
        if (Tcl_DeleteCommand(i, name) != TCL_OK) return 2;
    }
    puts("explicitly-retired-other-five");
    if (invoke(i, parser, "0") != TCL_OK) return 3;
    Tcl_Obj *first = Tcl_GetObjResult(i);
    Tcl_IncrRefCount(first);
    int count;
    Tcl_Obj **members;
    Tcl_ListObjGetElements(i, first, &count, &members);
    printf("first-length %d literal-refcount %d\n", count, members[0]->refCount);
    if (invoke(i, parser, "0") != TCL_OK) return 4;
    Tcl_Obj *second = Tcl_GetObjResult(i);
    Tcl_IncrRefCount(second);
    Tcl_Obj **again;
    Tcl_ListObjGetElements(i, second, &count, &again);
    printf("same-default-originals %d literal-refcount %d\n",
        members[0] == again[0] && members[1] == again[1] && members[2] == again[2],
        members[0]->refCount);
    Tcl_ResetResult(i);
    int getterFirst = argc > 1 && !strcmp(argv[1], "getenv-first");
    if (Tcl_DeleteCommand(i, getterFirst ? getter : parser) != TCL_OK) return 5;
    printf("first-retirement %s literal-refcount %d\n",
        getterFirst ? "getenv" : "ParseFormatArgs", members[0]->refCount);
    if (invoke(i, getterFirst ? parser : getter,
            getterFirst ? "0" : "NH_CLOCK_GETENV_TEST") != TCL_OK) return 6;
    printf("surviving-worker-result %s\n", Tcl_GetStringResult(i));
    Tcl_ResetResult(i);
    if (Tcl_DeleteCommand(i, getterFirst ? parser : getter) != TCL_OK) return 7;
    printf("last-retirement literal-refcount %d retained-value %s\n",
        members[0]->refCount, Tcl_GetString(members[0]));
    Tcl_DecrRefCount(first);
    Tcl_DecrRefCount(second);
    Tcl_DeleteInterp(i);
    Tcl_Finalize();
    return 0;
}
