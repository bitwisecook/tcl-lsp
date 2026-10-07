#include <stdio.h>
#include <tcl.h>

int main(int argc, char **argv)
{
    Tcl_Interp *interp;
    Tcl_Obj *root, *element, *value, *selected;
    int flags;
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    interp = Tcl_CreateInterp();
    Tcl_Eval(interp, "info patchlevel");
    printf("engine_patchlevel=%s\n", Tcl_GetStringResult(interp));
    for (flags = 0; flags <= 1; ++flags) {
        root = Tcl_NewStringObj("::missing::arr", -1);
        element = Tcl_NewStringObj("k", -1);
        value = Tcl_NewStringObj("VALUE", -1);
        Tcl_IncrRefCount(root);
        Tcl_IncrRefCount(element);
        Tcl_IncrRefCount(value);
        Tcl_SetObjResult(interp, Tcl_NewStringObj("SENTINEL", -1));
        selected = Tcl_ObjSetVar2(interp, root, element, value,
            flags ? TCL_LEAVE_ERR_MSG : 0);
        printf("purpose=%s|selected=%s|result=%s\n",
            flags ? "Write" : "QuietWrite",
            selected ? "some" : "none", Tcl_GetStringResult(interp));
        Tcl_DecrRefCount(value);
        Tcl_DecrRefCount(element);
        Tcl_DecrRefCount(root);
    }
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return 0;
}
