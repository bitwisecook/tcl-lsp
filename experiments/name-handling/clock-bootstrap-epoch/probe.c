#include "tclInt.h"
#include <stdio.h>
#include <string.h>

static const char *names[] = {
    "::clock", "::tcl::clock::add", "::tcl::clock::format",
    "::tcl::clock::scan", "::tcl::clock::clicks",
    "::tcl::clock::seconds", "::tcl::clock::milliseconds",
    "::tcl::clock::microseconds", "::tcl::clock::getenv",
    "::tcl::clock::Oldscan", "::tcl::clock::ConvertLocalToUTC",
    "::tcl::clock::GetDateFields", "::tcl::clock::GetJulianDayFromEraYearMonthDay",
    "::tcl::clock::GetJulianDayFromEraYearWeekDay", "::tcl::clock::ParseFormatArgs",
    NULL
};

static void snapshot(Tcl_Interp *interp, const char *stage) {
    Interp *actual = (Interp *)interp;
    printf("EPOCH\t%s\t%d\t%d\n", stage, actual->compileEpoch,
        actual->globalNsPtr->resolverEpoch);
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
    int code;
    if (argc == 2 && strcmp(argv[1], "init") == 0) {
        code = Tcl_Init(interp);
    } else if (argc == 2) {
        char source[512];
        snprintf(source, sizeof(source),
            "proc boot {} {proc %s args {}; set marker after; puts $marker}\nboot\n", argv[1]);
        code = Tcl_EvalEx(interp, source, -1, TCL_EVAL_GLOBAL);
    } else {
        code = TCL_ERROR;
    }
    printf("RESULT\t%d\t%s\n", code, Tcl_GetStringResult(interp));
    snapshot(interp, "completed");
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return code != TCL_OK;
}
