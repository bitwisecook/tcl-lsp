/* Native old-object hook effects before Integer read/modify/write. */
#include <stdio.h>
#include <string.h>
#include <tcl.h>
static int replacement(ClientData unused, Tcl_Interp *i, int n, Tcl_Obj *const v[]) {
    (void)unused; (void)n; (void)v;
    Tcl_SetObjResult(i,Tcl_NewStringObj("CHANGED",-1)); return TCL_OK;
}
static void mutate(Tcl_Interp *i,const char *what) {
    Tcl_SetVar(i,"keep",what,0);
    Tcl_DeleteCommand(i,"worker");
    Tcl_CreateObjCommand(i,"worker",replacement,NULL,NULL);
}
static void update(Tcl_Obj *o) {
    mutate((Tcl_Interp *)o->internalRep.twoPtrValue.ptr1,"UPDATED");
    o->bytes=Tcl_Alloc(2); memcpy(o->bytes,"3",2); o->length=1;
}
static void free_rep(Tcl_Obj *o) {
    mutate((Tcl_Interp *)o->internalRep.twoPtrValue.ptr1,"FREED");
}
static Tcl_ObjType updater={"increment-updater",NULL,NULL,update,NULL};
static Tcl_ObjType freer={"increment-free",free_rep,NULL,NULL,NULL};
static int make_input(ClientData unused,Tcl_Interp *i,int n,Tcl_Obj *const v[]) {
    Tcl_Obj *o; const char *mode=Tcl_GetString(v[1]); (void)unused;(void)n;
    if(strcmp(mode,"free")==0) {
        o=Tcl_NewStringObj("3",-1); o->typePtr=&freer;
    } else { o=Tcl_NewObj(); Tcl_InvalidateStringRep(o); o->typePtr=&updater; }
    o->internalRep.twoPtrValue.ptr1=i; Tcl_SetObjResult(i,o); return TCL_OK;
}
int main(int n,char **v) {
    const char *modes[]={"update","free"}; unsigned mode,route;
    if(n!=2)return 2; Tcl_FindExecutable(v[0]);
    for(route=0;route<2;route++)for(mode=0;mode<2;mode++) {
        Tcl_Interp *i=Tcl_CreateInterp(); char script[600]; int code;
        Tcl_SetVar(i,"tcl_library",v[1],TCL_GLOBAL_ONLY);
        if(Tcl_Init(i)!=TCL_OK) { fprintf(stderr,"init: %s\n",Tcl_GetStringResult(i)); return 3; }
        Tcl_CreateObjCommand(i,"make_input",make_input,NULL,NULL);
        Tcl_Eval(i,"interp alias {} runtime_incr {} incr; proc worker {} {return OLD}");
        snprintf(script,sizeof(script),"proc f {} {set input [make_input %s]; set keep SAFE; set result [%s input]; list $result $keep [worker]}; f",modes[mode],route?"runtime_incr":"incr");
        code=Tcl_Eval(i,script);
        printf("route=%s hook=%s code=%d result=%s\n",route?"generic":"compiled",modes[mode],code,Tcl_GetStringResult(i));
        Tcl_DeleteInterp(i);
    }
    Tcl_Finalize(); return 0;
}
