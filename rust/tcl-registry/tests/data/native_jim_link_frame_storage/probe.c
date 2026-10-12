#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
static Interp *fresh(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK)return NULL; return i; }
static int eval(Interp *i,const char *source) { return Jim_Eval(i,source); }
static const char *result(Interp *i,long *length) { int n; const char *p=Jim_GetString(Jim_GetResult(i),&n); *length=n; return p; }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
static Interp *fresh(void) { Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK)return NULL; return i; }
static int eval(Interp *i,const char *source) { return Tcl_Eval(i,source); }
static const char *result(Interp *i,long *length) {
#if TCL_MAJOR_VERSION >= 9
 Tcl_Size n;
#else
 int n;
#endif
 const char *p=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n); *length=(long)n; return p;
}
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
#endif
static void hex(const char *bytes,long length) { for(long k=0;k<length;k++)printf("%02x",(unsigned char)bytes[k]); }
static void row(Interp *i,const char *label,int code) { long length; const char *bytes=result(i,&length); printf("%s|%d|",label,code); hex(bytes,length); puts(""); }
int main(int argc,char **argv) {
 (void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 const char *labels[]={"ACTIVE_LINK","RETIRED_SLOT_SELF_REUSE","NESTED_SLOT_REUSE","RETIRED_WRITE_THEN_READ","SCALAR_CELL_REFERENCE","GLOBAL_LINK"};
 const char *cases[]={
  "proc maker {} {set a KEEP;upvar 0 a x;proc p {} {&x} {set x};p};maker",
  "proc maker {} {set a KEEP;upvar 0 a x;proc p {} {&x} {set x}};maker;p",
  "proc maker {} {set a KEEP;upvar 0 a x;proc p {} {&x} {set x}};maker;proc unrelated {} {set a OTHER;p};unrelated",
  "proc maker {} {set a OLD;upvar 0 a x;proc p {} {&x} {set x MODIFIED};proc r {} {&x} {set x}};maker;p;r",
  "proc maker {} {set x KEEP;proc p {} {&x} {set x}};maker;p",
  "set a KEEP;upvar 0 a x;proc p {} {&x} {set x};set a CHANGED;p"
 };
 for(int k=0;k<6;k++) { Interp *i=fresh(); if(!i)return 2; row(i,"VERSION",eval(i,"info patchlevel")); printf("INPUT|%s|",labels[k]); hex(cases[k],(long)strlen(cases[k])); puts(""); row(i,labels[k],eval(i,cases[k])); destroy(i); }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
