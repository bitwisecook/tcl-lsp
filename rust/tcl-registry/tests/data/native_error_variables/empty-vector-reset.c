#include "tcl.h"
#include <stdio.h>

int main(int argc, char **argv) {
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *ip = Tcl_CreateInterp();
    for (int shared = 0; shared < 2; ++shared) {
        Tcl_SetObjResult(ip, Tcl_NewIntObj(17));
        Tcl_Obj *original = Tcl_GetObjResult(ip);
        if (shared) Tcl_IncrRefCount(original);
        int code = Tcl_EvalObjv(ip, 0, NULL, 0);
        Tcl_Obj *result = Tcl_GetObjResult(ip);
        printf("%d\t%d\t%d\t%d\t%d\t%s\t%d\t%d\n", shared,
            code, result == original, result->refCount, original->refCount,
            result->typePtr ? result->typePtr->name : "none",
            result->bytes != NULL, result->length);
        if (shared) Tcl_DecrRefCount(original);
    }
    Tcl_EvalEx(ip, "proc p {} {error FIRST_ERROR_LONG}", -1, 0);
    Tcl_Obj *head = Tcl_NewStringObj("p", -1);
    Tcl_IncrRefCount(head);
    int first = Tcl_EvalObjv(ip, 1, &head, 0);
    Tcl_Obj *original_info = Tcl_GetVar2Ex(ip, "errorInfo", NULL, TCL_GLOBAL_ONLY);
    if (!original_info) return 2;
    Tcl_IncrRefCount(original_info);
    int empty = Tcl_EvalObjv(ip, 0, NULL, 0);
    Tcl_Obj *current_info = Tcl_GetVar2Ex(ip, "errorInfo", NULL, TCL_GLOBAL_ONLY);
    printf("episode\t%d\t%d\t%d\t%d\n", first, empty,
        current_info == original_info, original_info->refCount);
    Tcl_DecrRefCount(original_info);
    Tcl_DecrRefCount(head);
    Tcl_DeleteInterp(ip);
    Tcl_Finalize();
    return 0;
}
