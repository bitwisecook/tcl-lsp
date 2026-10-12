#include <stdio.h>
#include <tcl.h>
static int HostSum(ClientData unused, Tcl_Interp *interp, Tcl_Value *args, Tcl_Value *result) {
    (void)unused; (void)interp;
    result->type = TCL_INT;
    result->intValue = args[0].intValue + args[1].intValue;
    return TCL_OK;
}
static int ReplacementAbs(ClientData unused, Tcl_Interp *interp, Tcl_Value *args, Tcl_Value *result) {
    (void)unused; (void)interp; (void)args;
    result->type = TCL_INT; result->intValue = 999; return TCL_OK;
}
static int ReplaceAbs(ClientData unused, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
    Tcl_ValueType types[1] = {TCL_INT};
    (void)unused; (void)objc; (void)objv;
    Tcl_CreateMathFunc(interp, "abs", 1, types, ReplacementAbs, NULL);
    Tcl_SetObjResult(interp, Tcl_NewIntObj(-4));
    return TCL_OK;
}
int main(int argc, char **argv) {
    Tcl_Interp *interp;
    Tcl_ValueType types[2] = {TCL_INT, TCL_INT};
    const char *scripts[] = {
        "expr {hostsum(2,3)}",
        "set touched 0; set code [catch {expr {hostsum([incr touched])}} message]; list $code $touched $message",
        "namespace eval ::tcl::mathfunc {}; proc ::tcl::mathfunc::hostsum {a b} {return COMMAND_SHADOW}; expr {hostsum(2,3)}",
        "expr {abs([replace_abs])}",
        "expr {abs(-4)}"
    };
    size_t i;
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    interp = Tcl_CreateInterp();
    Tcl_CreateMathFunc(interp, "hostsum", 2, types, HostSum, NULL);
    Tcl_CreateObjCommand(interp, "replace_abs", ReplaceAbs, NULL, NULL);
    for (i = 0; i < sizeof(scripts)/sizeof(scripts[0]); ++i) {
        int code = Tcl_Eval(interp, scripts[i]);
        printf("%lu\t%d\t%s\n", (unsigned long)i, code, Tcl_GetStringResult(interp));
    }
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return 0;
}
