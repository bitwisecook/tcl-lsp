#include <tcl.h>
static int Exact(ClientData data, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
    (void)data; (void)objc; (void)objv;
    Tcl_SetObjResult(interp, Tcl_NewStringObj("ORIGINAL", -1));
    return TCL_OK;
}
static int Install(ClientData data, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
    (void)data; (void)objc; (void)objv;
    if (!Tcl_CreateObjCommand(interp, "x", Exact, NULL, NULL)) return TCL_ERROR;
    if (!Tcl_CreateObjCommand(interp, "child::y", Exact, NULL, NULL)) return TCL_ERROR;
    return TCL_OK;
}
int Inventorycontext_Init(Tcl_Interp *interp) {
    if (!Tcl_CreateObjCommand(interp, "inventoryInstallOriginal", Install, NULL, NULL)) return TCL_ERROR;
    return TCL_OK;
}
