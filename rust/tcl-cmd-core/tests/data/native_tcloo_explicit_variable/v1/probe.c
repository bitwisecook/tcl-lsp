#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK) return NULL; return i; }
static Obj *string(Interp *i,const char *p,int n) { return Jim_NewStringObj(i,p,n); }
static void retain(Obj *o) { Jim_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { Jim_DecrRefCount(i,o); }
static int invoke(Interp *i,int n,Obj **v) { return Jim_EvalObjVector(i,n,v); }
static int source(Interp *i,const char *p,int n) { Obj *o=string(i,p,n); retain(o); int c=Jim_EvalObj(i,o); release(i,o); return c; }
static void result(Interp *i,const char *label,int c) { int n; const char *p=Jim_GetString(Jim_GetResult(i),&n); printf("%s|%d|",label,c); for(int k=0;k<n;k++) printf("%02x",(unsigned char)p[k]); puts(""); }
static void options(Interp *i,const char *label,int c) { (void)i; printf("%s_OPTIONS_NOT_TESTED|%d|no-C-return-options-api\n",label,c); }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static Interp *fresh(void) { Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK) return NULL; return i; }
static Obj *string(Interp *i,const char *p,int n) { (void)i; return Tcl_NewStringObj(p,n); }
static void retain(Obj *o) { Tcl_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { (void)i; Tcl_DecrRefCount(o); }
static int invoke(Interp *i,int n,Obj **v) { return Tcl_EvalObjv(i,n,v,0); }
static int source(Interp *i,const char *p,int n) { return Tcl_EvalEx(i,p,n,0); }
static void result(Interp *i,const char *label,int c) { Count n; const char *p=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n); printf("%s|%d|",label,c); for(Count k=0;k<n;k++) printf("%02x",(unsigned char)p[k]); puts(""); }
static void options(Interp *i,const char *label,int c) {
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
 (void)i; printf("%s_OPTIONS_NOT_TESTED|%d|no-Tcl_GetReturnOptions\n",label,c);
#else
 Obj *o=Tcl_GetReturnOptions(i,c); retain(o); Count n; const char *p=Tcl_GetStringFromObj(o,&n); printf("%s_OPTIONS|%d|",label,c); for(Count k=0;k<n;k++) printf("%02x",(unsigned char)p[k]); puts(""); release(i,o);
#endif
}
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
#endif

static void input(const char *label,const char *p,int n) { printf("%s_INPUT|0|",label); for(int k=0;k<n;k++) printf("%02x",(unsigned char)p[k]); puts(""); }
static int vector(Interp *i,const char *command,const char *member,Obj *operand) {
 Obj *v[]={string(i,command,(int)strlen(command)),string(i,member,(int)strlen(member)),operand};
 retain(v[0]); retain(v[1]); int c=invoke(i,3,v); release(i,v[1]); release(i,v[0]); return c;
}
static void inspect_namespace(Interp *i,const char *label) {
 const char *script="namespace eval [info object namespace O] {list [info vars] [array exists a] [info exists k]}";
 int c=source(i,script,(int)strlen(script)); result(i,label,c); options(i,label,c);
}
struct Case { const char *label; const char *name; int length; };
static const struct Case cases[]={
 {"ASCII","k",1},
 {"RAW_ZERO","k\0tail",6},
 {"ENCODED_ZERO","k\xc0\x80tail",7},
 {"ARRAY_ELEMENT","a(k)",4},
 {"QUALIFIER_AFTER_ZERO","k\0::Q",5},
 {"QUALIFIED","N::k",4},
 {"ABSOLUTE_RAW_ZERO","::global\0tail",13},
 {"OPAQUE_FF","k\xff",2}
};
int main(int argc,char **argv) {
 (void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 Interp *i=fresh(); if(!i) return 2; int c=source(i,"info patchlevel",15); result(i,"VERSION",c); destroy(i);
 const char *members[]={"link","varname","write"};
 const char *setup="oo::class create C {method link {name} {set c [catch {my variable $name} r]; list $c $r [info vars]}; method varname {name} {my varname $name}; method write {name} {my variable $name; set $name VALUE; my varname $name}}; C create O";
 for(unsigned x=0;x<sizeof(cases)/sizeof(cases[0]);x++) for(int operation=0;operation<3;operation++) {
  char label[100],aux[120]; snprintf(label,sizeof(label),"%s_%s",members[operation],cases[x].label);
  input(label,cases[x].name,cases[x].length);
  i=fresh(); if(!i) return 2;
  c=source(i,setup,(int)strlen(setup)); snprintf(aux,sizeof(aux),"%s_SETUP",label); result(i,aux,c);
  if(c!=0) { printf("%s_NOT_ATTEMPTED|%d|setup-unavailable\n",label,c); destroy(i); continue; }
  snprintf(aux,sizeof(aux),"%s_BEFORE",label); inspect_namespace(i,aux);
  Obj *original=string(i,cases[x].name,cases[x].length); retain(original);
  c=vector(i,"O",members[operation],original); result(i,label,c); options(i,label,c);
  snprintf(aux,sizeof(aux),"%s_ORIGINAL_AFTER",label);
#ifdef JIM_PROBE
  int n; const char *p=Jim_GetString(original,&n); input(aux,p,n);
#else
  Count n; const char *p=Tcl_GetStringFromObj(original,&n); input(aux,p,(int)n);
#endif
  snprintf(aux,sizeof(aux),"%s_AFTER",label); inspect_namespace(i,aux);
  release(i,original); destroy(i);
 }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
