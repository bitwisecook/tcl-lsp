#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) {Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);if(Jim_InitStaticExtensions(i)!=JIM_OK)return NULL;return i;}
static Obj *string(Interp *i,const char*p,int n){return Jim_NewStringObj(i,p,n);}
static int eval(Interp*i,const char*p){return Jim_Eval(i,p);}
static int invoke(Interp*i,int n,Obj**v){for(int k=0;k<n;k++)Jim_IncrRefCount(v[k]);int c=Jim_EvalObjVector(i,n,v);for(int k=0;k<n;k++)Jim_DecrRefCount(i,v[k]);return c;}
static void row(Interp*i,const char*l,int c){int n;const char*p=Jim_GetString(Jim_GetResult(i),&n);printf("%s|%d|",l,c);for(int k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);puts("");}
static void destroy(Interp*i){Jim_FreeInterp(i);}
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
static Interp *fresh(void){Interp*i=Tcl_CreateInterp();if(Tcl_Init(i)!=TCL_OK)return NULL;return i;}
static Obj *string(Interp*i,const char*p,int n){(void)i;return Tcl_NewStringObj(p,n);}
static int eval(Interp*i,const char*p){return Tcl_Eval(i,p);}
static int invoke(Interp*i,int n,Obj**v){for(int k=0;k<n;k++)Tcl_IncrRefCount(v[k]);int c=Tcl_EvalObjv(i,n,v,0);for(int k=0;k<n;k++)Tcl_DecrRefCount(v[k]);return c;}
static void row(Interp*i,const char*l,int c){
#if TCL_MAJOR_VERSION >= 9
Tcl_Size n;
#else
int n;
#endif
const char*p=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n);printf("%s|%d|",l,c);for(long k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);puts("");}
static void destroy(Interp*i){Tcl_DeleteInterp(i);}
#endif
int main(int argc,char**argv){(void)argc;
#ifndef JIM_PROBE
Tcl_FindExecutable(argv[0]);
#else
(void)argv;
#endif
Interp*i=fresh();if(!i)return 2;row(i,"VERSION",eval(i,"info patchlevel"));
int available=eval(i,"info commands ::oo::class");
#ifdef JIM_PROBE
int n;const char*present=Jim_GetString(Jim_GetResult(i),&n);
#else
#if TCL_MAJOR_VERSION >= 9
Tcl_Size n;
#else
int n;
#endif
const char*present=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n);
#endif
(void)present;if(available||!n){row(i,"OO_UNAVAILABLE",eval(i,"set unavailable UNAVAILABLE"));destroy(i);return 0;}destroy(i);
const char option0[]="-private",option1[]="-p",option2[]="-private\0tail",option3[]="-private\xc0\x80tail",option4[]="-export",option5[]="-unexport",option6[]="--";
const char*options[]={option0,option1,option2,option3,option4,option5,option6};
int sizes[]={sizeof(option0)-1,sizeof(option1)-1,sizeof(option2)-1,sizeof(option3)-1,sizeof(option4)-1,sizeof(option5)-1,sizeof(option6)-1};
const char*labels[]={"PRIVATE_FULL","PRIVATE_PREFIX","PRIVATE_RAW_ZERO","PRIVATE_ENCODED_ZERO","EXPORT_FULL","UNEXPORT_FULL","INVALID_OPTION"};
for(int k=0;k<7;k++){i=fresh();if(!i)return 2;if(eval(i,"oo::class create C {}; oo::define C method invoke {} {my p}; C create c"))return 3;Obj*v[]={string(i,"oo::define",10),string(i,"C",1),string(i,"method",6),string(i,"p",1),string(i,options[k],sizes[k]),string(i,"",0),string(i,"return OK",9)};int c=invoke(i,7,v);char label[80];snprintf(label,sizeof(label),"%s_DEFINE",labels[k]);row(i,label,c);if(!c){snprintf(label,sizeof(label),"%s_NAMES",labels[k]);row(i,label,eval(i,"info class methods C -private"));snprintf(label,sizeof(label),"%s_EXTERNAL",labels[k]);row(i,label,eval(i,"c p"));snprintf(label,sizeof(label),"%s_INTERNAL",labels[k]);row(i,label,eval(i,"c invoke"));}destroy(i);}
const char*scripts[]={
"oo::class create C {method p {-arg} {return $-arg}}; C create c; c p VALUE",
"oo::class create C {private {method p {} {return PRIVATE}};method invoke {} {my p}};oo::class create D {superclass C;method child {} {my p}};D create d;d invoke",
"oo::class create C {private {method p {} {return PRIVATE}};method invoke {} {my p}};oo::class create D {superclass C;method child {} {my p}};D create d;d child",
"oo::class create C {private {forward p list PRIVATE};method invoke {} {my p}};C create c;list [info class methods C -private] [c invoke] [catch {c p} message] $message",
"oo::class create C {private {method p {} {return PRIVATE}};export p};C create c;c p",
"oo::class create C {method Hidden {} {return HIDDEN};method invoke {} {[self] Hidden}};C create c;c invoke",
"oo::class create C {method -export {} {return NAMED}};C create c;oo::define C export -export;c -export"
};
const char*cases[]={"DASHED_FORMAL_OPTION_FREE","INHERITED_OWN_PRIVATE","SUBCLASS_PRIVATE_MISS","PRIVATE_FORWARD","EXPORT_CONVERTS_PRIVATE","EXTERNAL_SELF_UNEXPORTED","OPTION_SPELLING_AS_NAME"};
for(int k=0;k<7;k++){i=fresh();if(!i)return 2;row(i,cases[k],eval(i,scripts[k]));destroy(i);}
#ifndef JIM_PROBE
Tcl_Finalize();
#endif
return 0;}
