#include <tcl.h>
#include <stdio.h>
#include <string.h>
static Tcl_Obj *first;
static const char *mode;
static int calls;
static int capture(ClientData kind,Tcl_Interp *i,int objc,Tcl_Obj *const objv[]) {
    Tcl_Obj *v;
    (void)i;
    if(objc!=3 && kind) return TCL_ERROR;
    if(objc!=2 && !kind) return TCL_ERROR;
    v=kind?objv[1]:objv[0];
    printf("%s\t%d\t%s\t%d\n",mode,calls++,v->typePtr?v->typePtr->name:"none",first?first==v:-1);
    if(!first){first=v;Tcl_IncrRefCount(first);}
    return TCL_OK;
}
static void reset(const char *s){if(first)Tcl_DecrRefCount(first);first=NULL;calls=0;mode=s;}
static int run(Tcl_Interp *i,const char *s){int c=Tcl_Eval(i,s);if(c)fprintf(stderr,"%s\n",Tcl_GetStringResult(i));return c;}
int main(int argc,char **argv){
    Tcl_Interp *i;(void)argc;Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();
    Tcl_CreateObjCommand(i,"head",capture,NULL,NULL);Tcl_CreateObjCommand(i,"take",capture,(ClientData)1,NULL);
    reset("relative");
    if(run(i,"namespace eval N {}; proc p {} {head X}; proc N::q {} {head X}; proc d {} {take head X}; p; p; N::q; d"))return 1;
    reset("absolute");
    if(run(i,"proc p {} {::head X}; proc N::q {} {::head X}; proc d {} {take ::head X}; p; p; N::q; d"))return 1;
    reset("end");Tcl_DeleteInterp(i);Tcl_Finalize();return 0;
}
