#include <stdio.h>
#include <stdlib.h>
#include "tcl.h"
int main(int argc, char **argv) {
    const char *values[] = {"9", "2147483649", "9223372036854775807"};
    const char *scripts[] = {"expr {sqrt($raw)}", "proc f {} {global raw; expr {sqrt($raw)}}; f"};
    int value, script, factory;
    Tcl_FindExecutable(argv[0]);
    for (factory=0; factory<2; factory++) for (value=0; value<3; value++) for (script=0; script<2; script++) {
        Tcl_Interp *interp = Tcl_CreateInterp();
        Tcl_Obj *before, *after;
        int prep, code;
        Tcl_SetVar(interp, "raw", values[value], TCL_GLOBAL_ONLY);
        prep = Tcl_Eval(interp, "incr raw 0");
        if (factory) Tcl_SetVar2Ex(interp, "raw", NULL, Tcl_NewWideIntObj((Tcl_WideInt)strtoll(values[value], NULL, 10)), TCL_GLOBAL_ONLY);
        before = Tcl_GetVar2Ex(interp, "raw", NULL, TCL_GLOBAL_ONLY);
        printf("factory=%s input=%s entry=%s prep=%d before=%s ", factory?"hostWide":"incr", values[value], script?"procedure":"direct", prep, before && before->typePtr ? before->typePtr->name : "string");
        code = Tcl_Eval(interp, scripts[script]);
        after = Tcl_GetVar2Ex(interp, "raw", NULL, TCL_GLOBAL_ONLY);
        printf("code=%d after=%s result=%s\n", code, after && after->typePtr ? after->typePtr->name : "string", Tcl_GetStringResult(interp));
        Tcl_DeleteInterp(interp);
    }
    Tcl_Finalize(); return 0;
}
