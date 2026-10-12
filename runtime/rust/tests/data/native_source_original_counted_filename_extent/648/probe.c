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

static void hexbytes(const unsigned char *p,size_t n) {for(size_t k=0;k<n;k++)printf("%02x",p[k]);}
static void hexobj(Obj *o) {Count n;if(!o){fputs("ABSENT",stdout);return;}const unsigned char *p=(const unsigned char*)bytes(o,&n);for(Count k=0;k<n;k++)printf("%02x",p[k]);}
static void report(Interp *i,const char *tag,int code) {printf("%s|%d|",tag,code);hexobj(result(i));putchar('\n');}
static void run(const char *label,const char *original_path,int extent) {
 size_t length=strlen(original_path),suffix=extent==1?5:extent==2?6:0;
 if(length>2147483600u)exit(5);
 unsigned char *path=malloc(length+suffix+1);if(!path)exit(6);
 memcpy(path,original_path,length);
 if(extent==1){path[length]=0;memcpy(path+length+1,"TAIL",4);}
 if(extent==2){path[length]=0xc0;path[length+1]=0x80;memcpy(path+length+2,"TAIL",4);}
 path[length+suffix]=0;
 Interp *i=create();
#ifndef JIM_PROBE
 if(Tcl_SetSystemEncoding(i,"utf-8")!=TCL_OK)exit(4);
#endif
 if(eval(i,"set ::errorCode {R2286 FILENAME SEEDED}; set ::R2286_ENCODING_LOADED NO")!=OK)exit(4);
 printf("CASE|%s|1\n",label);
 fputs("INPUT_PATH|",stdout);hexbytes(path,length+suffix);putchar('\n');
 int version=eval(i,"info patchlevel");report(i,"VERSION",version);
 int available=eval(i,"llength [info commands source]");report(i,"AVAILABILITY",available);
 if(eval(i,"set ::errorCode {R2286 FILENAME SEEDED}")!=OK)exit(4);
 Obj *v[2]={string(i,"source",-1),string(i,(const char*)path,(int)(length+suffix))};
 retain(v[0]);retain(v[1]);
 int code=vector(i,2,v);Obj *out=result(i);retain(out);Obj *ec=variable(i,"::errorCode");if(ec)retain(ec);Obj *effect=variable(i,"::R2286_ENCODING_LOADED");if(effect)retain(effect);
 printf("RETURN|%d|",code);hexobj(out);putchar('\n');fputs("ERROR_CODE|",stdout);hexobj(ec);putchar('\n');fputs("SOURCE_EFFECT|",stdout);hexobj(effect);putchar('\n');
 fputs("PATH_AFTER|",stdout);hexobj(v[1]);putchar('\n');
 if(effect)release(i,effect);if(ec)release(i,ec);release(i,out);release(i,v[1]);release(i,v[0]);destroy(i);free(path);
}
int main(int argc,char **argv) {
 if(argc!=3)return 2;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#endif
 run("plain-existing",argv[1],0);run("plain-absent",argv[2],0);
 run("raw-zero-tail-existing",argv[1],1);run("raw-zero-tail-absent",argv[2],1);
 run("modified-zero-tail-existing",argv[1],2);run("modified-zero-tail-absent",argv[2],2);
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
