#ifdef USE_JIM
#include <jim.h>
typedef Jim_Interp I;typedef Jim_Obj O;
#define INC(o) Jim_IncrRefCount(o)
#define DEC(i,o) Jim_DecrRefCount(i,o)
#define WORD(i,s) Jim_NewStringObj(i,s,-1)
#define LIST(i,n,v) Jim_NewListObj(i,v,n)
#define EVAL(i,n,v) Jim_EvalObjVector(i,n,v)
#define RESULT(i) Jim_GetResult(i)
#define SET(i,n) Jim_SetResultInt(i,n)
#define STRING(o) Jim_String(o)
#else
#include <tcl.h>
typedef Tcl_Interp I;typedef Tcl_Obj O;
#define INC(o) Tcl_IncrRefCount(o)
#define DEC(i,o) Tcl_DecrRefCount(o)
#define WORD(i,s) Tcl_NewStringObj(s,-1)
#define LIST(i,n,v) Tcl_NewListObj(n,v)
#define EVAL(i,n,v) Tcl_EvalObjv(i,n,v,TCL_EVAL_GLOBAL)
#define RESULT(i) Tcl_GetObjResult(i)
#define SET(i,n) Tcl_SetObjResult(i,Tcl_NewIntObj(n))
#define STRING(o) Tcl_GetString(o)
#endif
#include <stdio.h>
static O*original;
#ifdef USE_JIM
static int capture(I*i,int n,O*const*v){
#else
static int capture(ClientData cd,I*i,int n,O*const*v){(void)cd;
#endif
if(n!=2)return 1;printf("{\"same_object\":%s,\"cache\":\"%s\"}\n",v[1]==original?"true":"false",v[1]->typePtr?v[1]->typePtr->name:"string");SET(i,v[1]==original);return 0;}
int main(){
#ifdef USE_JIM
I*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_CreateCommand(i,"capture",capture,NULL,NULL);original=Jim_NewDoubleObj(i,1.5);
#else
I*i=Tcl_CreateInterp();Tcl_CreateObjCommand(i,"capture",capture,NULL,NULL);original=Tcl_NewDoubleObj(1.5);
#endif
INC(original);O*fields[]={WORD(i,"arg"),original};O*pair=LIST(i,2,fields);INC(pair);O*params=LIST(i,1,&pair);INC(params);O*define[]={WORD(i,"proc"),WORD(i,"p"),params,WORD(i,"capture $arg")};for(int k=0;k<4;k++)INC(define[k]);int rc=EVAL(i,4,define);if(rc!=0){fprintf(stderr,"%s\n",STRING(RESULT(i)));return 1;}for(int k=0;k<4;k++)DEC(i,define[k]);O*call=WORD(i,"p");INC(call);rc=EVAL(i,1,&call);DEC(i,call);DEC(i,params);DEC(i,pair);DEC(i,original);
#ifdef USE_JIM
Jim_FreeInterp(i);
#else
Tcl_DeleteInterp(i);
#endif
return rc;}
