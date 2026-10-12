#include "tclInt.h"
#include <stdio.h>
#include <string.h>

static void snapshot(Tcl_Interp *interp, const char *stage) {
    Interp *actual = (Interp *)interp;
    printf("EPOCH\t%s\t%d\t%d\n", stage, actual->compileEpoch,
        actual->globalNsPtr->resolverEpoch);
    const char *names[] = {"::tcl::mathfunc::min", "::tcl::mathfunc::max", NULL};
    for (int n = 0; names[n]; ++n) {
        Command *command = (Command *)Tcl_FindCommand(interp, names[n], NULL, TCL_GLOBAL_ONLY);
        printf("COMMAND\t%s\t%s\t%d\t%d\t%d\n", stage, names[n],
            command != NULL, command ? command->compileProc != NULL : -1,
            command ? TclIsProc(command) != NULL : -1);
    }
    fflush(stdout);
}

int main(int argc, char **argv) {
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *interp = Tcl_CreateInterp();
    snapshot(interp, "created");
    int code = Tcl_EvalEx(interp, "lsort [info commands ::tcl::mathfunc::*]", -1, TCL_EVAL_GLOBAL);
    printf("ROSTER\t%d\t%s\n", code, Tcl_GetStringResult(interp));
    if (argc == 2 && strcmp(argv[1], "init") == 0) {
        code = Tcl_Init(interp);
    } else if (argc == 2) {
        FILE *file = fopen(argv[1], "rb");
        if (!file) return 2;
        fseek(file, 0, SEEK_END);
        long size = ftell(file);
        rewind(file);
        char *source = (char *)Tcl_Alloc((unsigned)size + 1);
        if (fread(source, 1, (size_t)size, file) != (size_t)size) return 2;
        fclose(file);
        source[size] = 0;
        code = Tcl_EvalEx(interp, source, (int)size, TCL_EVAL_GLOBAL);
        Tcl_Free(source);
    } else {
        code = TCL_ERROR;
    }
    printf("RESULT\t%d\t%s\n", code, Tcl_GetStringResult(interp));
    snapshot(interp, "completed");
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return code != TCL_OK;
}
