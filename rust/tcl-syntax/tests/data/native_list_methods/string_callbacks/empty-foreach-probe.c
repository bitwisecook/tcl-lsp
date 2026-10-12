/* Native length callbacks from custom scalar and ordinary container members. */
#include <stdio.h>
#include <string.h>
#include <tcl.h>
static void update_scalar(Tcl_Obj *value) {
    Tcl_Interp *interp=(Tcl_Interp *)value->internalRep.twoPtrValue.ptr1;
    Tcl_SetVar(interp,"keep","MUTATED",0);
    value->bytes=Tcl_Alloc(1);value->bytes[0]=0;value->length=0;
}
static Tcl_ObjType custom_scalar={"custom-scalar",NULL,NULL,update_scalar,NULL};
static int make_input(ClientData ignored,Tcl_Interp *interp,int objc,Tcl_Obj *const objv[]) {
    Tcl_Obj *value=Tcl_NewObj(), *result; const char *kind=Tcl_GetString(objv[1]);
    (void)ignored;(void)objc;
    Tcl_InvalidateStringRep(value); value->typePtr=&custom_scalar;
    value->internalRep.twoPtrValue.ptr1=interp;
    if(strcmp(kind,"list")==0) result=Tcl_NewListObj(1,&value);
#if TCL_MAJOR_VERSION >= 9 || TCL_MINOR_VERSION >= 5
    else if(strcmp(kind,"dict")==0) { result=Tcl_NewDictObj(); Tcl_DictObjPut(interp,result,Tcl_NewStringObj("key",-1),value); }
#endif
    else result=value;
    Tcl_SetObjResult(interp,result);return TCL_OK;
}
static int show_type(ClientData ignored,Tcl_Interp *interp,int objc,Tcl_Obj *const objv[]) {
    const char *type=objv[1]->typePtr ? objv[1]->typePtr->name:"string";
    (void)ignored;(void)objc; Tcl_SetObjResult(interp,Tcl_NewStringObj(type,-1));return TCL_OK;
}
int main(int argc,char **argv) {
    const char *kinds[]={"scalar"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<1;kind++) {
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION < 5
        if(kind==2)continue;
#endif
        Tcl_Interp *interp=Tcl_CreateInterp();char script[512];int code;
        Tcl_SetVar(interp,"tcl_library",argv[1],TCL_GLOBAL_ONLY);if(Tcl_Init(interp)!=TCL_OK)return 3;
        Tcl_CreateObjCommand(interp,"make_input",make_input,NULL,NULL);Tcl_CreateObjCommand(interp,"show_type",show_type,NULL,NULL);
        Tcl_Eval(interp,"interp alias {} runtime_iteration {} foreach");
        snprintf(script,sizeof(script),"proc f {} {set input [make_input %s]; set keep SAFE; %s item $input {}; list $keep}; f",kinds[kind],route?"runtime_iteration":"foreach");
        code=Tcl_Eval(interp,script); printf("route=%s kind=%s code=%d result=%s\n",route?"generic":"compiled",kinds[kind],code,Tcl_GetStringResult(interp));
        Tcl_DeleteInterp(interp);
    }
    Tcl_Finalize();return 0;
}
