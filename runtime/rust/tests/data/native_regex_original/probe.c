#include "tcl.h"
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#define TRACE "trace add variable m write observe; "
#else
typedef int Count;
#define TRACE "trace variable m w observe; "
#endif
#if TCL_MAJOR_VERSION >= 9 && TCL_MINOR_VERSION >= 1
typedef Tcl_Size CallbackCount;
#else
typedef int CallbackCount;
#endif
static Tcl_Obj *seed;
static int keep(ClientData client,Tcl_Interp *i, CallbackCount objc,Tcl_Obj *const objv[]) {
 printf("callback\t%d\t%d\t%s\t%d\t%d\t%s\t%d\n",objv[1]==seed,seed->refCount,seed->typePtr?seed->typePtr->name:"none",seed->bytes!=NULL,objv[2]->refCount,objv[2]->typePtr?objv[2]->typePtr->name:"none",objv[2]->bytes!=NULL);
 Tcl_SetObjResult(i,seed); return TCL_OK;
}
int main(int argc,char **argv) {
 int which=argc>1?atoi(argv[1]):0; Tcl_FindExecutable(argv[0]); Tcl_Interp *i=Tcl_CreateInterp();
 const char *cases[]={
 "set log {}; proc observe {name index op} {upvar 1 $name v; lappend ::log $v}; " TRACE "regexp -all {(.)} abc m s; list $log $m $s",
 "set log {}; proc observe {name index op} {upvar 1 $name v; lappend ::log $v; if {$v == \"b\"} {error STOP}}; " TRACE "set rc [catch {regexp -all {(.)} abc m s} msg]; list $rc $log $msg $s $::errorCode",
 "set a scalar; set rc [catch {regsub a a b a(k)} msg]; list $rc $msg $::errorCode",
 "set a scalar; regsub a a b a(k)"
 };
 int code;
 if(which==5) {
 Tcl_CreateObjCommand(i,"keep",keep,NULL,NULL);
 Tcl_Obj *seedChild=Tcl_NewStringObj("S",-1); seed=Tcl_NewListObj(1,&seedChild);
 Tcl_Obj *head=Tcl_NewStringObj("keep",-1); Tcl_Obj *members[2]={head,seed}; Tcl_Obj *prefix=Tcl_NewListObj(2,members); Tcl_IncrRefCount(prefix);
 Tcl_Obj *words[6]={Tcl_NewStringObj("regsub",-1),Tcl_NewStringObj("-all",-1),Tcl_NewStringObj("-command",-1),Tcl_NewStringObj("(.)",-1),Tcl_NewStringObj("ab",-1),prefix};
 int n;for(n=0;n<5;n++)Tcl_IncrRefCount(words[n]);code=Tcl_EvalObjv(i,6,words,0);
 for(n=0;n<5;n++)Tcl_DecrRefCount(words[n]);Tcl_DecrRefCount(prefix);
 } else if(which==3) {
 Tcl_Obj *child=Tcl_NewStringObj("untouched",-1); Tcl_Obj *name=Tcl_NewListObj(1,&child); Tcl_IncrRefCount(name);
 Tcl_Obj *words[4]={Tcl_NewStringObj("regexp",-1),Tcl_NewStringObj("z",-1),Tcl_NewStringObj("abc",-1),name};
 int n;for(n=0;n<3;n++)Tcl_IncrRefCount(words[n]);
 code=Tcl_EvalObjv(i,4,words,0);
 printf("target\t%d\t%s\t%d\n",name->bytes!=NULL,name->typePtr?name->typePtr->name:"none",name->refCount);
 for(n=0;n<3;n++)Tcl_DecrRefCount(words[n]);Tcl_DecrRefCount(name);
 } else { code=Tcl_EvalEx(i,cases[which==4?3:which],(int)strlen(cases[which==4?3:which]),0); }
 Tcl_Obj *result=Tcl_GetObjResult(i); const char *type=result->typePtr?result->typePtr->name:"none";int resident=result->bytes!=NULL,refs=result->refCount;
 Count length; const unsigned char *bytes=(const unsigned char*)Tcl_GetStringFromObj(result,&length);printf("result\t%d\t%s\t%d\t%d\t",code,type,resident,refs);int n;for(n=0;n<length;n++)printf("%02x",bytes[n]);puts("");
 Tcl_DeleteInterp(i); Tcl_Finalize(); return 0;
}
