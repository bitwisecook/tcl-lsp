#include <tcl.h>
#include <stdio.h>
#include <string.h>
static int origin(Tcl_Interp *i,Tcl_Obj *name) {
 Tcl_Obj *a[3];int code;
 a[0]=Tcl_NewStringObj("namespace",-1);a[1]=Tcl_NewStringObj("origin",-1);a[2]=name;
 Tcl_IncrRefCount(a[0]);Tcl_IncrRefCount(a[1]);
 code=Tcl_EvalObjv(i,3,a,0);
 printf("%d\t%s\t%d\t%s\n",code,name->typePtr?name->typePtr->name:"none",name->bytes!=NULL,Tcl_GetStringResult(i));
 Tcl_DecrRefCount(a[1]);Tcl_DecrRefCount(a[0]);return code;
}
int main(int argc,char **argv) {
 Tcl_Interp *i=Tcl_CreateInterp();Tcl_Obj *name;
 Tcl_Eval(i,"namespace eval src {proc p {} {return SOURCE};namespace export p};namespace eval mid {namespace import ::src::p;namespace export p};namespace eval dest {namespace import ::mid::p}");
 name=Tcl_NewStringObj("::dest::p",-1);Tcl_IncrRefCount(name);
 origin(i,name);
 if(argc>1 && strcmp(argv[1],"stringless")==0) {Tcl_InvalidateStringRep(name);origin(i,name);} else {Tcl_Eval(i,"rename ::src::p ::src::q");origin(i,name);Tcl_Eval(i,"rename ::dest::p {}; proc ::dest::p {} {return NEW}");origin(i,name);}
 Tcl_DecrRefCount(name);Tcl_DeleteInterp(i);return 0;
}
