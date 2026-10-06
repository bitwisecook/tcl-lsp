#include "tclOOInt.h"
#include <stdio.h>
#include <stdlib.h>
static void call(Tcl_Interp *ip,int n,Tcl_Obj **v){for(int x=0;x<n;x++)Tcl_IncrRefCount(v[x]);int c=Tcl_EvalObjv(ip,n,v,0);for(int x=0;x<n;x++)Tcl_DecrRefCount(v[x]);if(c!=TCL_OK){fprintf(stderr,"call %s\n",Tcl_GetStringResult(ip));exit(2);}}
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp*ip=Tcl_CreateInterp();if(Tcl_Init(ip)!=TCL_OK)return 2;Tcl_Eval(ip,"oo::configurable create C; C create o");
const char *names[]={"x\xff","x\0tail"};int lengths[]={2,6};for(int k=0;k<2;k++){
 Tcl_Obj *name=Tcl_NewStringObj(names[k],lengths[k]);Tcl_IncrRefCount(name);Tcl_Obj *v[]={Tcl_NewStringObj("oo::define",-1),Tcl_NewStringObj("C",-1),Tcl_NewStringObj("property",-1),name,Tcl_NewStringObj("-get",-1),Tcl_NewStringObj("return RAW",-1)};call(ip,6,v);
 char dashed[12];dashed[0]='-';memcpy(dashed+1,names[k],lengths[k]);Tcl_Obj *q=Tcl_NewStringObj(dashed,lengths[k]+1);Tcl_IncrRefCount(q);Tcl_Obj *read[]={Tcl_NewStringObj("o",-1),Tcl_NewStringObj("configure",-1),q};call(ip,3,read);
 printf("opaque-%d\t%s\t%s\n",k,Tcl_GetStringResult(ip),q->typePtr?q->typePtr->name:"none");Tcl_DecrRefCount(q);Tcl_DecrRefCount(name);
}Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;}
