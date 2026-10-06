#include "tcl.h"
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *ip = Tcl_CreateInterp();
    Tcl_Obj *head = Tcl_NewStringObj("p", -1);
    Tcl_IncrRefCount(head);
    int definition = Tcl_EvalEx(ip, "proc p {} {error FIRST_ERROR_LONG}", -1, 0);
    int first = Tcl_EvalObjv(ip, 1, &head, 0);
    Tcl_Obj *original = Tcl_GetVar2Ex(ip, "errorInfo", NULL, TCL_GLOBAL_ONLY);
    if (!original) return 2;
    Tcl_IncrRefCount(original);
    Tcl_ResetResult(ip);
    Tcl_Obj *reset = Tcl_GetVar2Ex(ip, "errorInfo", NULL, TCL_GLOBAL_ONLY);
    int same_reset = reset == original;
    int reset_refs = original->refCount;
    int second_definition = Tcl_EvalEx(ip, "proc p {} {error X}", -1, 0);
    Tcl_Obj *after_definition = Tcl_GetVar2Ex(ip, "errorInfo", NULL, TCL_GLOBAL_ONLY);
    int same_definition = after_definition == original;
    int definition_refs = original->refCount;
    int second = Tcl_EvalObjv(ip, 1, &head, 0);
    Tcl_Obj *current = Tcl_GetVar2Ex(ip, "errorInfo", NULL, TCL_GLOBAL_ONLY);
    int fresh = current && current != original;
    int message = strcmp(Tcl_GetStringResult(ip), "X") == 0;
    int clean = current && !strstr(Tcl_GetString(current), "FIRST_ERROR_LONG");
    int third = Tcl_EvalObjv(ip, 1, &head, 0);
    current = Tcl_GetVar2Ex(ip, "errorInfo", NULL, TCL_GLOBAL_ONLY);
    int third_clean = current && !strstr(Tcl_GetString(current), "FIRST_ERROR_LONG");
    printf("%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\n",
        definition, first, same_reset, reset_refs, second_definition,
        same_definition, definition_refs, second, fresh, message, clean,
        third, third_clean);
    Tcl_DecrRefCount(original);
    Tcl_DecrRefCount(head);
    Tcl_DeleteInterp(ip);
    Tcl_Finalize();
    return 0;
}
