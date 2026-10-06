#include "tcl.h"
#include <stdio.h>
#if TCL_MAJOR_VERSION >= 9
#define Count Tcl_Size
#else
#define Count int
#endif
static int probe(ClientData data, Tcl_Interp *i, Count objc, Tcl_Obj *const objv[]) {
    Tcl_Obj *v;
    if (objc != 2) return TCL_ERROR;
    v=Tcl_ObjGetVar2(i,objv[1],NULL,TCL_LEAVE_ERR_MSG);
    if(!v) return TCL_ERROR;
    printf("%s|%s\n",objv[1]->typePtr?objv[1]->typePtr->name:"NULL",Tcl_GetString(v));
    Tcl_ResetResult(i); return TCL_OK;
}
int main(int argc,char **argv) {
    Tcl_Interp *i; int c;
    Tcl_FindExecutable(argv[0]); i=Tcl_CreateInterp();
#if TCL_MAJOR_VERSION >= 9
    Tcl_CreateObjCommand2(i,"probe",probe,NULL,NULL);
#else
    Tcl_CreateObjCommand(i,"probe",probe,NULL,NULL);
#endif
    Tcl_SetVar2Ex(i,"name",NULL,Tcl_NewStringObj("v",1),TCL_GLOBAL_ONLY);
    c=Tcl_Eval(i,"set v GLOBAL;proc p {} {set v LOCAL;probe $::name;unset v;global $::name;probe $::name;return $v};p");
    printf("simple|%d|%s\n",c,Tcl_GetStringResult(i));
    c=Tcl_Eval(i,"namespace eval N {variable v QUALIFIED};set name ::N::v;p");
    printf("qualified|%d|%s\n",c,Tcl_GetStringResult(i));
    Tcl_DeleteInterp(i);Tcl_Finalize();return 0;
}
