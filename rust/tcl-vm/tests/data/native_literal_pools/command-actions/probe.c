#include <tcl.h>
#include <stdio.h>
static Tcl_Obj *held;
static const char *scenario;
static void show(const char *stage) {
    printf("%s\t%s\t%s\n",scenario,stage,held && held->typePtr ? held->typePtr->name : "none");
}
static int head(ClientData data,Tcl_Interp *i,int objc,Tcl_Obj *const objv[]) {
    (void)data;(void)objc;(void)objv;Tcl_ResetResult(i);return TCL_OK;
}
static int probe(ClientData data,Tcl_Interp *i,int objc,Tcl_Obj *const objv[]) {
    (void)data;
    if(objc!=2) return TCL_ERROR;
    if(held) Tcl_DecrRefCount(held);
    held=objv[1];Tcl_IncrRefCount(held);show("probe");Tcl_ResetResult(i);return TCL_OK;
}
static int run(const char *label,const char *body) {
    Tcl_Interp *i=Tcl_CreateInterp();int code;
    scenario=label;held=NULL;
    Tcl_CreateObjCommand(i,"head",head,NULL,NULL);
    Tcl_CreateObjCommand(i,"probe",probe,NULL,NULL);
    code=Tcl_Eval(i,body);
    if(code!=TCL_OK){fprintf(stderr,"%s: %s\n",label,Tcl_GetStringResult(i));return 1;}
    if(!held)return 2;
    Tcl_DeleteCommand(i,"head");show("deleted");
    Tcl_CreateObjCommand(i,"head",head,NULL,NULL);show("recreated");
    Tcl_DecrRefCount(held);held=NULL;Tcl_DeleteInterp(i);return 0;
}
int main(int argc,char **argv) {
    int result=0;(void)argc;Tcl_FindExecutable(argv[0]);
    result|=run("data-relative","proc f {} {probe head; head}; f");
    result|=run("command-relative","proc f {} {head; probe head}; f");
    result|=run("data-absolute","proc f {} {probe ::head; ::head}; f");
    result|=run("command-absolute","proc f {} {::head; probe ::head}; f");
    Tcl_Finalize();return result;
}
