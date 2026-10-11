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
static unsigned char *filename(const char *directory,const unsigned char *key,size_t keylen,int absent,int extent,size_t *outlen) {
 size_t length=strlen(directory),suffix=extent==1?5:extent==2?6:0;
 const char *stem=absent?"/absent-":"/key-";size_t stemlen=strlen(stem);
 if(length+stemlen+keylen+4+suffix>2147483600u)exit(5);
 *outlen=length+stemlen+keylen+4+suffix;
 unsigned char *path=malloc(*outlen+1);if(!path)exit(6);
 memcpy(path,directory,length);memcpy(path+length,stem,stemlen);memcpy(path+length+stemlen,key,keylen);memcpy(path+length+stemlen+keylen,".tcl",4);
 size_t end=length+stemlen+keylen+4;
 if(extent==1){path[end]=0;memcpy(path+end+1,"TAIL",4);}
 if(extent==2){path[end]=0xc0;path[end+1]=0x80;memcpy(path+end+2,"TAIL",4);}
 path[*outlen]=0;return path;
}
static void run(const char *directory,const char *codec,const char *label,const unsigned char *key,size_t keylen,int absent,int extent) {
 Interp *i=create();size_t pathlen;unsigned char *path=filename(directory,key,keylen,absent,extent,&pathlen);
 printf("CASE|%s/%s|1\n",codec,label);fputs("INPUT_PATH|",stdout);hexbytes(path,pathlen);putchar('\n');
 report(i,"VERSION",eval(i,"info patchlevel"));report(i,"ENCODING_AVAILABILITY",eval(i,"llength [info commands encoding]"));
#ifndef JIM_PROBE
 int selected=Tcl_SetSystemEncoding(i,codec);report(i,"SELECT_ENCODING",selected);
 if(selected!=OK){destroy(i);free(path);return;}
 report(i,"CURRENT_ENCODING",eval(i,"encoding system"));
#else
 fputs("SELECT_ENCODING|UNAVAILABLE\n",stdout);
#endif
 if(eval(i,"set ::errorCode {R2286 CODEC SEEDED}; set ::R2286_FILENAME_EFFECT NO")!=OK)exit(4);
 Obj *v[2]={string(i,"source",-1),string(i,(const char*)path,(int)pathlen)};retain(v[0]);retain(v[1]);
 int code=vector(i,2,v);Obj *out=result(i);retain(out);Obj *ec=variable(i,"::errorCode");if(ec)retain(ec);Obj *effect=variable(i,"::R2286_FILENAME_EFFECT");if(effect)retain(effect);
 printf("RETURN|%d|",code);hexobj(out);putchar('\n');fputs("ERROR_CODE|",stdout);hexobj(ec);putchar('\n');fputs("SOURCE_EFFECT|",stdout);hexobj(effect);putchar('\n');fputs("PATH_AFTER|",stdout);hexobj(v[1]);putchar('\n');
 if(effect)release(i,effect);if(ec)release(i,ec);release(i,out);release(i,v[1]);release(i,v[0]);destroy(i);free(path);
}
static void codec(const char *directory,const char *encoding) {
 static const unsigned char ascii[]="ASCII",e9[]={0xc3,0xa9},invalid[]={0xff},euro[]={0xe2,0x82,0xac};
 run(directory,encoding,"ascii-existing",ascii,sizeof(ascii)-1,0,0);
 run(directory,encoding,"ascii-absent",ascii,sizeof(ascii)-1,1,0);
 run(directory,encoding,"unicode-e9-existing",e9,sizeof(e9),0,0);
 run(directory,encoding,"raw-invalid-ff-existing",invalid,sizeof(invalid),0,0);
 run(directory,encoding,"unicode-euro-existing",euro,sizeof(euro),0,0);
 run(directory,encoding,"raw-zero-tail-existing",ascii,sizeof(ascii)-1,0,1);
 run(directory,encoding,"modified-zero-tail-existing",ascii,sizeof(ascii)-1,0,2);
 run(directory,encoding,"unicode-e9-absent",e9,sizeof(e9),1,0);
}
int main(int argc,char **argv) {
 if(argc!=2)return 2;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);codec(argv[1],"utf-8");codec(argv[1],"iso8859-1");codec(argv[1],"identity");
 static const unsigned char ascii[]="ASCII";run(argv[1],"R2286_NO_SUCH_ENCODING","explicit-unknown",ascii,sizeof(ascii)-1,0,0);Tcl_Finalize();
#else
 codec(argv[1],"Jim-no-encoding-interface");
#endif
 return 0;
}
