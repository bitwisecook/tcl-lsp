#include "tclshim.h"

static const char *persistent_table[] = {"alpha", "beta", NULL};

static int index_command(ClientData mode, Tcl_Interp *interp, int objc,
        Tcl_Obj *const objv[]) {
    Tcl_Size index = -1;
    int flags = mode ? 64 : 0;
    if (objc != 2) return TCL_ERROR;
    if (Tcl_GetIndexFromObj(interp, objv[1], persistent_table, "option",
            flags, &index) != TCL_OK || index != 0) return TCL_ERROR;
    if (!mode && Tcl_GetIndexFromObj(interp, objv[1], persistent_table,
            "option", TCL_EXACT, &index) != TCL_OK) return TCL_ERROR;
    Tcl_SetObjResult(interp, objv[1]);
    return TCL_OK;
}

int IndexCache_Init(Tcl_Interp *interp) {
    persistent_table[0] = "alpha";
    Tcl_CreateObjCommand(interp, "original_index", index_command, NULL, NULL);
    Tcl_CreateObjCommand(interp, "temporary_index", index_command,
            (ClientData)1, NULL);
    return TCL_OK;
}

void IndexCache_ChangeTable(void) {
    persistent_table[0] = "changed";
}
