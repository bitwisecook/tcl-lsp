#include <stdio.h>
#include <string.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Interp Interp;
static Interp *make(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK){Jim_FreeInterp(i);return NULL;} return i; }
static int eval(Interp *i,const char *s){return Jim_Eval(i,s);}
static const char *result(Interp *i,long long *n){int length;const char *p=Jim_GetString(Jim_GetResult(i),&length);*n=length;return p;}
static void destroy(Interp *i){Jim_FreeInterp(i);}
#else
#include "tcl.h"
static Tcl_Interp *make(void) { Tcl_Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK){Tcl_DeleteInterp(i);return NULL;} return i; }
typedef Tcl_Interp Interp;
static int eval(Interp *i,const char *s){return Tcl_EvalEx(i,s,-1,TCL_EVAL_DIRECT);}
static const char *result(Interp *i,long long *n){
#if TCL_MAJOR_VERSION >= 9
Tcl_Size length;
#else
int length;
#endif
const char *p=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&length);*n=length;return p;}
static void destroy(Interp *i){Tcl_DeleteInterp(i);}
#endif
static void row(const char *kind,const char *label,int code,Interp *i){long long n;const char *p=result(i,&n);printf("%s|%s|%d|%lld|",kind,label,code,n);for(long long k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);puts("");fflush(stdout);}
struct Case {const char *name;const char *source;};
static const struct Case cases[]={
{"basic","set x OLD; after 0 {set x NEW}; vwait x; set x"},
{"lone-all","set {-all} OLD; after 0 {set {-all} NEW}; vwait -all; set {-all}"},
{"lone-signal","set {-signal} OLD; after 0 {set {-signal} NEW}; vwait -signal; set {-signal}"},
{"lone-end","set {--} OLD; after 0 {set {--} NEW}; vwait --"},
{"end-name","set {--} OLD; after 0 {set {--} NEW}; vwait -- --; set {--}"},
{"option-variable","set x OLD; after 0 {set x NEW}; vwait -variable x -timeout 100; set x"},
{"abbreviated-variable","set x OLD; after 0 {set x NEW}; vwait -v x -t 100; set x"},
{"timeout-or-jim-body","set {-timeout} OLD; after 0 {set {-timeout} NEW}; vwait -timeout 1"},
{"signal-name","set x OLD; after 0 {set x NEW}; vwait -signal x; set x"},
{"name-body","set x SAME; after 0 {set x SAME}; vwait x {break}"},
{"signal-name-body","set x SAME; after 0 {set x SAME}; vwait -signal x {break}"},
{"abbreviation-is-jim-name","set {-sig} OLD; after 0 {set {-sig} NEW}; vwait -sig x; set {-sig}"},
{"global-scope","set x OLD; namespace eval N {variable x LOCAL}; after 0 {set x GLOBAL}; namespace eval N {vwait x}; list [namespace eval N {set x}] $x"},
{"missing-option-value","vwait -variable x -timeout"},
{"missing-channel","vwait -readable no_such_original_channel"}
};
int main(int argc,char **argv){
#ifndef USE_JIM
Tcl_FindExecutable("original-vwait-forms");
#endif
Interp *i=make();if(!i)return 2;int rc=eval(i,"info patchlevel");row("V","actual-provider",rc,i);destroy(i);
for(unsigned k=0;k<sizeof(cases)/sizeof(cases[0]);k++){if(argc>1 && strcmp(argv[1],cases[k].name))continue;i=make();if(!i)return 3;rc=eval(i,cases[k].source);row("R",cases[k].name,rc,i);destroy(i);}
#ifndef USE_JIM
Tcl_Finalize();
#endif
return 0;}
