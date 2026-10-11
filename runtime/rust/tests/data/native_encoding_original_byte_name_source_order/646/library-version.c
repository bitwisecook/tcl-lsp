#include <stdio.h>
#ifdef PROBE_JIM
#include "jim.h"
int main(void) {
    Jim_Interp *interp = Jim_CreateInterp();
    int code;
    Jim_RegisterCoreCommands(interp);
    code = Jim_Eval(interp, "info patchlevel");
    printf("code\t%d\npatchlevel\t%s\n", code, Jim_String(interp->result));
    Jim_FreeInterp(interp);
    return code == JIM_OK ? 0 : 1;
}
#else
#include "tcl.h"
int main(int argc, char **argv) {
    Tcl_Interp *interp;
    int code, major, minor, patch, kind;
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    Tcl_GetVersion(&major, &minor, &patch, &kind);
    printf("library-version\t%d\t%d\t%d\t%d\n", major, minor, patch, kind);
    interp = Tcl_CreateInterp();
    code = Tcl_EvalEx(interp, "info patchlevel", -1, TCL_EVAL_GLOBAL);
    printf("code\t%d\npatchlevel\t%s\n", code, Tcl_GetStringResult(interp));
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return code == TCL_OK ? 0 : 1;
}
#endif
