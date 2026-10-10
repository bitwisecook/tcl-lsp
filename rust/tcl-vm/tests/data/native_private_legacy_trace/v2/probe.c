#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#ifdef JIM_PROBE
#include "jim.h"
#else
#include "tcl.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
#endif
static void hex(const char *p, long n) { for (long k=0;k<n;k++) printf("%02x",(unsigned char)p[k]); }
#ifdef JIM_PROBE
static void row(Jim_Interp *i,const char *label,int code) {int n;const char*p=Jim_GetString(Jim_GetResult(i),&n);printf("%s|%d|",label,code);hex(p,n);puts("");}
int main(void) {Jim_Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);if(Jim_InitStaticExtensions(i)!=JIM_OK)return 2;row(i,"VERSION",Jim_Eval(i,"info patchlevel"));row(i,"PRIVATE_AVAILABLE",Jim_Eval(i,"llength [info commands ::oo::define::private]"));row(i,"LEGACY_AVAILABLE",Jim_Eval(i,"trace variable v w callback"));Jim_FreeInterp(i);return 0;}
#else
static void row(Tcl_Interp*i,const char*label,int code){Size n;const char*p=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n);printf("%s|%d|",label,code);hex(p,(long)n);puts("");}
static Tcl_Interp *fresh(void){Tcl_Interp*i=Tcl_CreateInterp();if(Tcl_Init(i)!=TCL_OK){fprintf(stderr,"init failed: %s\n",Tcl_GetStringResult(i));return NULL;}return i;}
static int invoke(Tcl_Interp*i,int argc,Tcl_Obj**argv){for(int k=0;k<argc;k++)Tcl_IncrRefCount(argv[k]);int c=Tcl_EvalObjv(i,argc,argv,0);for(int k=0;k<argc;k++)Tcl_DecrRefCount(argv[k]);return c;}
int main(int argc,char**argv){(void)argc;Tcl_FindExecutable(argv[0]);Tcl_Interp*i=fresh();if(!i)return 2;row(i,"VERSION",Tcl_Eval(i,"info patchlevel"));row(i,"PRIVATE_AVAILABLE",Tcl_Eval(i,"llength [info commands ::oo::define::private]"));int available=atoi(Tcl_GetStringResult(i));if(available){row(i,"PRIVATE_QUERY",Tcl_Eval(i,"oo::class create C {set ::before [private];private {set ::inside [private]};set ::after [private]};list $::before $::inside $::after"));Tcl_Eval(i,"rename C {}");const char ff[]="private {method {p\xff} {} {return OK}}";const char surrogate[]="private {method {p\xed\xa0\x80} {} {return OK}}";const char zero[]="private {method {p\0tail} {} {return OK}}";const char*body[]={ff,surrogate,zero};Size lengths[]={sizeof(ff)-1,sizeof(surrogate)-1,sizeof(zero)-1};const char*labels[]={"PRIVATE_RAW_FF","PRIVATE_RAW_SURROGATE","PRIVATE_RAW_ZERO"};for(int k=0;k<3;k++){Tcl_Eval(i,"oo::class create C {}");Tcl_Obj*words[]={Tcl_NewStringObj("oo::define",-1),Tcl_NewStringObj("C",-1),Tcl_NewStringObj(body[k],lengths[k])};row(i,labels[k],invoke(i,3,words));char label[80];snprintf(label,sizeof(label),"%s_NAMES",labels[k]);row(i,label,Tcl_Eval(i,"info class methods C -private"));Tcl_Eval(i,"rename C {}");}}
Tcl_DeleteInterp(i);
const char plain[]={'w'},zero[]={'w',0,'b','a','d'},encoded[]={'w',(char)0xc0,(char)0x80,'b','a','d'},bad[]={'w',(char)0xff},onlyzero[]={0};const char*ops[]={plain,zero,encoded,bad,onlyzero};Size lengths[]={sizeof(plain),sizeof(zero),sizeof(encoded),sizeof(bad),sizeof(onlyzero)};const char*labels[]={"LEGACY_PLAIN","LEGACY_RAW_ZERO","LEGACY_ENCODED_ZERO","LEGACY_RAW_FF","LEGACY_ONLY_ZERO"};for(int k=0;k<5;k++){i=fresh();if(!i)return 2;Tcl_Obj*words[]={Tcl_NewStringObj("trace",-1),Tcl_NewStringObj("variable",-1),Tcl_NewStringObj("v",-1),Tcl_NewStringObj(ops[k],lengths[k]),Tcl_NewStringObj("callback",-1)};row(i,labels[k],invoke(i,5,words));char label[80];snprintf(label,sizeof(label),"%s_INFO",labels[k]);row(i,label,Tcl_Eval(i,"trace vinfo v"));Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}
#endif
