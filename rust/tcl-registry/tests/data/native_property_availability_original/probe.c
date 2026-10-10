#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK)return NULL; return i; }
static Obj *string(Interp *i,const char *p,int n) { return Jim_NewStringObj(i,p,n); }
static void hold(Obj *o) { Jim_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { Jim_DecrRefCount(i,o); }
static int eval(Interp *i,const char *p) { return Jim_Eval(i,p); }
static int invoke(Interp *i,int n,Obj **v) { for(int k=0;k<n;k++)hold(v[k]); int c=Jim_EvalObjVector(i,n,v); for(int k=0;k<n;k++)release(i,v[k]); return c; }
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static const char *bytes(Obj *o,long *n) { int size; const char *p=Jim_GetString(o,&size); *n=size; return p; }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
static Interp *fresh(void) { Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK)return NULL; return i; }
static Obj *string(Interp *i,const char *p,int n) { (void)i; return Tcl_NewStringObj(p,n); }
static void hold(Obj *o) { Tcl_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { (void)i; Tcl_DecrRefCount(o); }
static int eval(Interp *i,const char *p) { return Tcl_Eval(i,p); }
static int invoke(Interp *i,int n,Obj **v) { for(int k=0;k<n;k++)hold(v[k]); int c=Tcl_EvalObjv(i,n,v,0); for(int k=0;k<n;k++)release(i,v[k]); return c; }
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj *o,long *n) {
#if TCL_MAJOR_VERSION >= 9
Tcl_Size size;
#else
int size;
#endif
const char *p=Tcl_GetStringFromObj(o,&size); *n=(long)size; return p; }
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
#endif
static void hex(const char *p,long n) { for(long k=0;k<n;k++)printf("%02x",(unsigned char)p[k]); }
/* Pointer equality is sampled before the only result string getter. */
static void row(Interp *i,const char *label,int code,Obj *argument) { int same=argument && result(i)==argument; long n; const char *p=bytes(result(i),&n); printf("%s|%d|%d|",label,code,same); hex(p,n); puts(""); }

int main(int argc,char **argv) {
 (void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 Interp *i=fresh(); if(!i)return 2;
 row(i,"VERSION",eval(i,"info patchlevel"),NULL);
 Obj *availability[]={string(i,"info",4),string(i,"commands",8),string(i,"::oo::configurable",18)};
 row(i,"AVAILABILITY_OBJV",invoke(i,3,availability),NULL);
 Obj *factory[]={string(i,"oo::configurable",16),string(i,"create",6),string(i,"C",1)};
 int code=invoke(i,3,factory); row(i,"FACTORY_OBJV",code,NULL);
 if(!code) {
  Obj *object[]={string(i,"C",1),string(i,"create",6),string(i,"o",1)};
  row(i,"OBJECT_OBJV",invoke(i,3,object),NULL);
 }
 destroy(i);
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
