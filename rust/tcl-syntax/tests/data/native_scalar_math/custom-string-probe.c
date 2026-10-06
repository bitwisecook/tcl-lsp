/* A custom scalar's string updater can mutate the current interpreter frame. */
#include <stdio.h>
#include <string.h>
#include <tcl.h>

static void update_scalar(Tcl_Obj *value) {
    Tcl_Interp *interp = (Tcl_Interp *)value->internalRep.twoPtrValue.ptr1;
    Tcl_SetVar(interp, "keep", "MUTATED", 0);
    value->bytes = Tcl_Alloc(2);
    memcpy(value->bytes, "4", 2);
    value->length = 1;
}

static Tcl_ObjType custom_scalar = {
    "custom-scalar", NULL, NULL, update_scalar, NULL
};

static int make_scalar(ClientData unused, Tcl_Interp *interp,
        int objc, Tcl_Obj *const objv[]) {
    Tcl_Obj *value = Tcl_NewObj();
    (void)unused; (void)objc; (void)objv;
    Tcl_InvalidateStringRep(value);
    value->typePtr = &custom_scalar;
    value->internalRep.twoPtrValue.ptr1 = interp;
    Tcl_SetObjResult(interp, value);
    return TCL_OK;
}

int main(int argc, char **argv) {
    Tcl_Interp *interp;
    int code;
    if (argc != 2) return 2;
    Tcl_FindExecutable(argv[0]);
    interp = Tcl_CreateInterp();
    Tcl_SetVar(interp, "tcl_library", argv[1], TCL_GLOBAL_ONLY);
    if (Tcl_Init(interp) != TCL_OK) return 3;
    Tcl_CreateObjCommand(interp, "custom_scalar", make_scalar, NULL, NULL);
    code = Tcl_Eval(interp,
        "proc f {} {set keep SAFE; set input [custom_scalar]; "
        "set result [expr {sqrt($input)}]; list $result $keep}; f");
    printf("code=%d result=%s\n", code, Tcl_GetStringResult(interp));
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return 0;
}
