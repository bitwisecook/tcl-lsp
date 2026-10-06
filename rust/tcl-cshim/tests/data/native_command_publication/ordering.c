#include <tcl.h>
#include <stdio.h>
#include <string.h>
static Tcl_Interp *active;
static int mode;
static int answer(ClientData data, Tcl_Interp *interp, int count, Tcl_Obj *const values[]) {
    (void)count; (void)values;
    Tcl_SetObjResult(interp, Tcl_NewStringObj((const char *)data, -1));
    return TCL_OK;
}
static void report(const char *label) {
    Tcl_CmdInfo info;
    int exists = Tcl_GetCommandInfo(active, "::n::x", &info);
    printf("%s exists=%d value=%s\n", label, exists,
        exists && info.objProc == answer ? (const char *)info.objClientData : "none");
}
static void deleted(ClientData data) {
    (void)data;
    report("delete-enter");
    if (mode >= 2) {
        Tcl_CreateObjCommand(active, "::n::x", answer, (ClientData)"nested", NULL);
        report("delete-created");
    }
}
static int seed(ClientData data, Tcl_Interp *interp, int count, Tcl_Obj *const values[]) {
    (void)data; (void)count; (void)values;
    Tcl_CreateObjCommand(interp, "::n::x", answer, (ClientData)"old", deleted);
    return TCL_OK;
}
int main(void) {
    Tcl_FindExecutable("registration-order");
    for (mode = 0; mode != 4; ++mode) {
        active = Tcl_CreateInterp();
        Tcl_CreateObjCommand(active, "seed", seed, NULL, NULL);
        int code = Tcl_Eval(active, "namespace eval n {::seed}");
        printf("case=%d seed=%d\n", mode, code);
        report("before");
        if (mode == 1 || mode == 3) {
            int status = Tcl_DeleteCommand(active, "::n::x");
            printf("delete-status=%d\n", status);
        } else {
            Tcl_Command token = Tcl_CreateObjCommand(active, "::n::x", answer,
                (ClientData)"outer", NULL);
            printf("replace-token=%d\n", token != NULL);
        }
        report("after");
        Tcl_DeleteInterp(active);
    }
    Tcl_Finalize();
    return 0;
}
