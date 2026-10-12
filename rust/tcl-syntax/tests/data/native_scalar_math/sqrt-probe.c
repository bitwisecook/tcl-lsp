/* Selected native sqrt result and operand caches; no interpreter inference. */
#include <stdio.h>
#include <tcl.h>

static const char *type_name(Tcl_Obj *value) {
    return value->typePtr ? value->typePtr->name : "string";
}

int main(int argc, char **argv) {
    const char *labels[] = {"integer", "double", "string", "bad-string", "negative"};
    int i;
    if (argc != 2) return 2;
    Tcl_FindExecutable(argv[0]);
    for (i = 0; i < 5; ++i) {
        Tcl_Interp *interp = Tcl_CreateInterp();
        Tcl_Obj *input;
        const char *before;
        Tcl_Obj *retained_result;
        int code;
        Tcl_SetVar(interp, "tcl_library", argv[1], TCL_GLOBAL_ONLY);
        if (Tcl_Init(interp) != TCL_OK) return 3;
        switch (i) {
        case 0: input = Tcl_NewIntObj(4); break;
        case 1: input = Tcl_NewDoubleObj(4.0); break;
        case 2: input = Tcl_NewStringObj("4", -1); break;
        case 3: input = Tcl_NewStringObj("not-number", -1); break;
        default: input = Tcl_NewIntObj(-1); break;
        }
        Tcl_IncrRefCount(input);
        Tcl_SetVar2Ex(interp, "x", NULL, input, TCL_GLOBAL_ONLY);
        before = type_name(input);
        code = Tcl_Eval(interp, "set math_result [expr {sqrt($x)}]");
        retained_result = Tcl_GetVar2Ex(interp, "math_result", NULL, TCL_GLOBAL_ONLY);
        printf("%s code=%d before=%s after=%s result=%s\n",
            labels[i], code, before, type_name(input),
            type_name(retained_result ? retained_result : Tcl_GetObjResult(interp)));
        Tcl_DecrRefCount(input);
        Tcl_DeleteInterp(interp);
    }
    Tcl_Finalize();
    return 0;
}
