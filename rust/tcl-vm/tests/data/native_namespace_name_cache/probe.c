#include "tcl.h"
#include <stdio.h>
#if TCL_MAJOR_VERSION >= 9
#define Count Tcl_Size
#else
#define Count int
#endif
static int probe(ClientData data, Tcl_Interp *i, Count objc, Tcl_Obj *const objv[]) {
    Tcl_Obj *value;
    if(objc != 2) return TCL_ERROR;
    value = Tcl_ObjGetVar2(i, objv[1], NULL, TCL_LEAVE_ERR_MSG);
    if(!value) return TCL_ERROR;
    printf("local|%s|%s\n", objv[1]->typePtr?objv[1]->typePtr->name:"NULL", Tcl_GetString(value));
    value = Tcl_ObjGetVar2(i, objv[1], NULL, TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG);
    if(!value) return TCL_ERROR;
    printf("namespace|%s|%s\n", objv[1]->typePtr?objv[1]->typePtr->name:"NULL", Tcl_GetString(value));
    if(Tcl_UpVar2(i, "#0", "v", NULL, "alias", 0)!=TCL_OK) return TCL_ERROR;
    value = Tcl_ObjGetVar2(i, objv[1], NULL, TCL_NAMESPACE_ONLY|TCL_LEAVE_ERR_MSG);
    if(!value) return TCL_ERROR;
    printf("linked|%s|%s\n", objv[1]->typePtr?objv[1]->typePtr->name:"NULL", Tcl_GetString(value));
    Tcl_ResetResult(i);
    return TCL_OK;
}
int main(int argc,char **argv) {
    Tcl_Interp *i;
    int code;
    Tcl_FindExecutable(argv[0]); i=Tcl_CreateInterp();
    #if TCL_MAJOR_VERSION >= 9
    Tcl_CreateObjCommand2(i,"probe",probe,NULL,NULL);
#else
    Tcl_CreateObjCommand(i,"probe",probe,NULL,NULL);
#endif
    code=Tcl_Eval(i,"set v GLOBAL; proc p {} {set v LOCAL; probe v; return $alias}; p");
    printf("completion|%d|%s\n",code,Tcl_GetStringResult(i));
    Tcl_DeleteInterp(i); Tcl_Finalize(); return code;
}
