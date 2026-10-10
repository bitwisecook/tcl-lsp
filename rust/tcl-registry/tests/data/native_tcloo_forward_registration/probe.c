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

static void source_case(const char *label,const char *source) {
 Interp *i=fresh(); if(!i)return;
 row(i,label,eval(i,source),NULL); destroy(i);
}
int main(int argc,char **argv) {
 (void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 Interp *i=fresh(); if(!i)return 2;
 row(i,"VERSION",eval(i,"info patchlevel"),NULL);
 int code=eval(i,"info commands ::oo::class"); row(i,"AVAILABILITY",code,NULL);
 long n; bytes(result(i),&n); destroy(i);
 if(n) {
  source_case("MISSING_REGISTER", "oo::class create C {forward pass ::later FIXED};list [info class methods C] [info class forward C pass]");
  source_case("MISSING_INVOKE", "oo::class create C {forward pass ::later FIXED};C create object;object pass");
  source_case("PREFIX_CAPTURE", "set value INITIAL;oo::class create C {forward pass list $value};set value CHANGED;C create object;object pass EXTRA");
  source_case("NO_TARGET_CALL", "set hits 0;proc destination args {incr ::hits};oo::class create C {forward pass destination FIXED};set hits");
  source_case("SCRIPT_REPLACED", "oo::class create C {method pass {} {return OLD};forward pass ::later FIXED};info class forward C pass");
  source_case("FORWARD_REPLACED", "oo::class create C {forward pass ::later FIXED;method pass {} {return SCRIPT}};C create object;object pass");
  source_case("EMPTY_TARGET_REGISTER", "oo::class create C {forward pass {} FIXED};info class forward C pass");
  source_case("PREFIX_REQUIRED", "oo::class create C {forward pass}");
  source_case("SUPER_FORWARD", "oo::class create Base {forward a::b ::later};oo::class create C {superclass Base;method a::b {} {next}};list [info class methods C] [info class forward Base a::b]");
 }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
