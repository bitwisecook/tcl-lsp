#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
typedef int Count;
#define OK JIM_OK
static Obj *string(Interp *i,const char *p,int n) {return Jim_NewStringObj(i,p,n);}
static void retain(Obj *o) {Jim_IncrRefCount(o);}
static void release(Interp *i,Obj *o) {Jim_DecrRefCount(i,o);}
static int vector(Interp *i,int n,Obj **v) {return Jim_EvalObjVector(i,n,v);}
static int eval(Interp *i,const char *s) {return Jim_Eval(i,s);}
static Obj *result(Interp *i) {return Jim_GetResult(i);}
static const char *bytes(Obj *o,Count *n) {return Jim_GetString(o,n);}
static Obj *variable(Interp *i,const char *name) {return Jim_GetGlobalVariableStr(i,name,0);}
static Interp *create(void) {Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);if(Jim_InitStaticExtensions(i)!=JIM_OK)exit(3);return i;}
static void destroy(Interp *i) {Jim_FreeInterp(i);}
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION>=9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
#define OK TCL_OK
static Obj *string(Interp *i,const char *p,int n) {(void)i;return Tcl_NewStringObj(p,n);}
static void retain(Obj *o) {Tcl_IncrRefCount(o);}
static void release(Interp *i,Obj *o) {(void)i;Tcl_DecrRefCount(o);}
static int vector(Interp *i,int n,Obj **v) {return Tcl_EvalObjv(i,n,v,TCL_EVAL_GLOBAL);}
static int eval(Interp *i,const char *s) {return Tcl_Eval(i,s);}
static Obj *result(Interp *i) {return Tcl_GetObjResult(i);}
static const char *bytes(Obj *o,Count *n) {return Tcl_GetStringFromObj(o,n);}
static Obj *variable(Interp *i,const char *name) {return Tcl_GetVar2Ex(i,name,NULL,TCL_GLOBAL_ONLY);}
static Interp *create(void) {Interp *i=Tcl_CreateInterp();if(Tcl_Init(i)!=TCL_OK)exit(3);return i;}
static void destroy(Interp *i) {Tcl_DeleteInterp(i);}
#endif
struct Name {const char *label;unsigned char bytes[64];int length;};
static const struct Name names[]={
 {"utf8", {0x75,0x74,0x66,0x2d,0x38}, 5},
 {"ascii", {0x61,0x73,0x63,0x69,0x69}, 5},
 {"latin1", {0x69,0x73,0x6f,0x38,0x38,0x35,0x39,0x2d,0x31}, 9},
 {"empty", {0}, 0},
 {"binary", {0x62,0x69,0x6e,0x61,0x72,0x79}, 6},
 {"upper-utf8", {0x55,0x54,0x46,0x2d,0x38}, 5},
 {"unknown-ascii", {0x52,0x32,0x32,0x38,0x36,0x5f,0x6d,0x69,0x73,0x73,0x69,0x6e,0x67,0x5f,0x65,0x6e,0x63,0x6f,0x64,0x69,0x6e,0x67}, 22},
 {"utf8-raw-zero-tail", {0x75,0x74,0x66,0x2d,0x38,0x00,0x54,0x41,0x49,0x4c}, 10},
 {"empty-raw-zero-tail", {0x00,0x54,0x41,0x49,0x4c}, 5},
 {"unknown-raw-zero-tail", {0x52,0x32,0x32,0x38,0x36,0x5f,0x6d,0x69,0x73,0x73,0x69,0x6e,0x67,0x5f,0x65,0x6e,0x63,0x6f,0x64,0x69,0x6e,0x67,0x00,0x54,0x41,0x49,0x4c}, 27},
 {"raw-invalid-ff", {0xff}, 1},
 {"raw-invalid-c0af", {0xc0,0xaf}, 2},
 {"native-modified-zero", {0x75,0x74,0x66,0x2d,0x38,0xc0,0x80,0x54,0x41,0x49,0x4c}, 11},
};
static void hexobj(Obj *o) {Count n;if(!o){fputs("ABSENT",stdout);return;}const unsigned char *p=(const unsigned char*)bytes(o,&n);for(Count k=0;k<n;k++)printf("%02x",p[k]);}
static void report(Interp *i,const char *tag,int code) {printf("%s|%d|",tag,code);hexobj(result(i));putchar('\n');}
static void run(const char *label,int worker,const struct Name *name,const char *existing,const char *missing) {
 Interp *i=create();
#ifndef JIM_PROBE
 if(Tcl_SetSystemEncoding(i,"utf-8")!=TCL_OK)exit(4);
#endif
 if(eval(i,"set ::errorCode {R2286 ENCODING SEEDED}; set ::R2286_ENCODING_LOADED NO")!=OK)exit(4);
 printf("CASE|%s|%d\n",label,worker);
 if(eval(i,"info patchlevel")==OK)report(i,"VERSION",OK);else report(i,"VERSION",1);
 int availability=eval(i,"list [llength [info commands encoding]] [llength [info commands source]]");report(i,"AVAILABILITY",availability);
 int before=eval(i,"encoding system");report(i,"SYSTEM_BEFORE",before);
 if(eval(i,"set ::errorCode {R2286 ENCODING SEEDED}")!=OK)exit(4);
 Obj *v[4];int n=0;
 if(worker==0) {v[n++]=string(i,"encoding",-1);v[n++]=string(i,"system",-1);}
 else {v[n++]=string(i,"source",-1);if(name)v[n++]=string(i,"-encoding",-1);}
 if(name)v[n++]=string(i,(const char*)name->bytes,name->length);
 if(worker!=0)v[n++]=string(i,worker==2?missing:existing,-1);
 for(int k=0;k<n;k++)retain(v[k]);
 int code=vector(i,n,v);Obj *out=result(i);retain(out);Obj *ec=variable(i,"::errorCode");if(ec)retain(ec);Obj *effect=variable(i,"::R2286_ENCODING_LOADED");if(effect)retain(effect);
 printf("RETURN|%d|",code);hexobj(out);putchar('\n');fputs("ERROR_CODE|",stdout);hexobj(ec);putchar('\n');fputs("SOURCE_EFFECT|",stdout);hexobj(effect);putchar('\n');
 int after=eval(i,"encoding system");report(i,"SYSTEM_AFTER",after);
 if(effect)release(i,effect);if(ec)release(i,ec);release(i,out);for(int k=0;k<n;k++)release(i,v[k]);destroy(i);
}
int main(int argc,char **argv) {
 if(argc!=4)return 2;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#endif
 run("directory-utf8",1,&names[0],argv[3],argv[2]);
 run("directory-unknown",1,&names[6],argv[3],argv[2]);
 run("directory-binary",1,&names[4],argv[3],argv[2]);
 run("plain-directory",1,NULL,argv[3],argv[2]);
 run("existing-utf8",1,&names[0],argv[1],argv[2]);
 run("existing-unknown",1,&names[6],argv[1],argv[2]);
 run("missing-utf8",2,&names[0],argv[1],argv[2]);
 run("missing-unknown",2,&names[6],argv[1],argv[2]);
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
