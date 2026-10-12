#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Obj Obj; typedef Jim_Interp Interp; typedef int Len;
#define NEW(i,s,n) Jim_NewStringObj(i,s,n)
#define INC(o) Jim_IncrRefCount(o)
#define DEC(i,o) Jim_DecrRefCount(i,o)
#define GET(o,n) Jim_GetString(o,n)
#define RESULT(i) Jim_GetResult(i)
#define LIST(i,n,v) Jim_NewListObj(i,v,n)
#define VAR(i,n) Jim_GetVariableStr(i,n,0)
#define EVAL(i,n,v) Jim_EvalObjVector(i,n,v)
#define SCRIPT(i,o) Jim_EvalObj(i,o)
static Interp *create(void){Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK){fprintf(stderr,"Jim static extensions failed: %s\n",Jim_String(Jim_GetResult(i)));exit(2);}return i;}
#define DESTROY(i) Jim_FreeInterp(i)
#else
#include "tcl.h"
typedef Tcl_Obj Obj; typedef Tcl_Interp Interp;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
#define NEW(i,s,n) Tcl_NewStringObj(s,n)
#define INC(o) Tcl_IncrRefCount(o)
#define DEC(i,o) Tcl_DecrRefCount(o)
#define GET(o,n) Tcl_GetStringFromObj(o,n)
#define RESULT(i) Tcl_GetObjResult(i)
#define LIST(i,n,v) Tcl_NewListObj(n,v)
#define VAR(i,n) Tcl_GetVar2Ex(i,n,NULL,TCL_GLOBAL_ONLY)
#define EVAL(i,n,v) Tcl_EvalObjv(i,n,v,TCL_EVAL_GLOBAL)
#define SCRIPT(i,o) Tcl_EvalObjEx(i,o,TCL_EVAL_GLOBAL)
static Interp *create(void){return Tcl_CreateInterp();}
#define DESTROY(i) Tcl_DeleteInterp(i)
#endif
static const char *caseid,*pathid; static Interp *active;
static void hexobj(Obj *o){Len n=0; const unsigned char *s; if(!o){printf("null");return;} s=(const unsigned char *)GET(o,&n);putchar('"'); for(Len j=0;j<n;j++)printf("%02x",s[j]);putchar('"');}
static void report(const char *op,int code){printf("{\"case\":\"%s\",\"path\":\"%s\",\"op\":\"%s\",\"code\":%d,\"result\":",caseid,pathid,op,code);hexobj(RESULT(active));printf(",\"result_type\":\"%s\",\"error_code\":",RESULT(active)->typePtr?RESULT(active)->typePtr->name:"string");hexobj(VAR(active,"errorCode"));printf(",\"error_info\":");hexobj(VAR(active,"errorInfo"));printf(",\"return_options\":");
#if !defined(USE_JIM) && (TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5)
Obj*options=Tcl_GetReturnOptions(active,code);INC(options);hexobj(options);DEC(active,options);
#else
printf("null");
#endif
puts("}");}
static Obj *word(const char*s){return NEW(active,s,-1);}
static int call(const char*op,int n,Obj**v){for(int j=0;j<n;j++)INC(v[j]);int c=EVAL(active,n,v);report(op,c);for(int j=0;j<n;j++)DEC(active,v[j]);return c;}
#define C1(op,a) do{Obj*v[]={a};call(op,1,v);}while(0)
#define C2(op,a,b) do{Obj*v[]={a,b};call(op,2,v);}while(0)
#define C3(op,a,b,c) do{Obj*v[]={a,b,c};call(op,3,v);}while(0)
#define C4(op,a,b,c,d) do{Obj*v[]={a,b,c,d};call(op,4,v);}while(0)
#define C6(op,a,b,c,d,e,f) do{Obj*v[]={a,b,c,d,e,f};call(op,6,v);}while(0)
static void objrow(const char *op,Obj*o){printf("{\"case\":\"%s\",\"path\":\"%s\",\"op\":\"%s\",\"object_type_before\":\"%s\",\"bytes\":",caseid,pathid,op,o->typePtr?o->typePtr->name:"string");hexobj(o);printf(",\"object_type_after\":\"%s\"}\n",o->typePtr?o->typePtr->name:"string");}
#ifdef USE_JIM
static int tracecmd(Interp*i,int n,Obj*const*v){for(int j=1;j<n;j++)objrow(j==1?"trace-name1":j==2?"trace-name2":"trace-op",v[j]);Jim_SetResultString(i,"",0);return JIM_OK;}
#else
static int tracecmd(ClientData d,Interp*i,int n,Obj*const*v){(void)d;for(int j=1;j<n;j++)objrow(j==1?"trace-name1":j==2?"trace-name2":"trace-op",v[j]);Tcl_ResetResult(i);return TCL_OK;}
#endif
static void installtrace(void){
#ifdef USE_JIM
Jim_CreateCommand(active,"capture",tracecmd,NULL,NULL);
#else
Tcl_CreateObjCommand(active,"capture",tracecmd,NULL,NULL);
#endif
}
static Obj *makename(const unsigned char*s,int n,int path){
#ifndef USE_JIM
if(path==1)return Tcl_NewByteArrayObj(s,n);
#endif
return NEW(active,(const char*)s,n);}


#ifndef USE_JIM
extern int TclParseBackslash(const char*,Len,Len*,char*);
#endif
static void run(const char*id,const unsigned char*bytes,int length){caseid=id;pathid="resident-string";active=create();Obj*input=NEW(active,(const char*)bytes,length);INC(input);objrow("input",input);
#ifndef USE_JIM
if(length && bytes[0]=='\\'){char encoded[16]={0};Len consumed=0;int size=TclParseBackslash((const char*)bytes,length,&consumed,encoded);Obj*out=NEW(active,encoded,size);INC(out);printf("{\"case\":\"%s\",\"path\":\"resident-string\",\"op\":\"native-first-escape\",\"consumed\":%lld,\"bytes\":",caseid,(long long)consumed);hexobj(out);puts("}");DEC(active,out);}
#endif
C4("subst",word("subst"),word("-nocommands"),word("-novariables"),input);unsigned char*script=malloc(length+20);memcpy(script,"set x \"",7);memcpy(script+7,bytes,length);memcpy(script+7+length,"\"; set x",8);Obj*source=NEW(active,(const char*)script,length+15);INC(source);objrow("raw-script-input",source);int c=SCRIPT(active,source);report("raw-script-result",c);DEC(active,source);
#ifndef USE_JIM
source=Tcl_NewByteArrayObj(script,length+15);INC(source);objrow("bytearray-script-input",source);c=SCRIPT(active,source);report("bytearray-script-result",c);DEC(active,source);
#endif
free(script);DEC(active,input);DESTROY(active);}
int main(int argc,char**argv){(void)argc;
#ifndef USE_JIM
Tcl_FindExecutable(argv[0]);
#else
(void)argv;
#endif
run("xzero",(const unsigned char[]){92,120,48,48},4);
run("octalzero",(const unsigned char[]){92,48,48,48},4);
run("octal400",(const unsigned char[]){92,52,48,48},4);
run("uzero",(const unsigned char[]){92,117,48,48,48,48},6);
run("usurrogate",(const unsigned char[]){92,117,100,56,48,48},6);
run("wide-astral",(const unsigned char[]){92,85,48,48,48,49,102,54,48,48},10);
run("wide-max",(const unsigned char[]){92,85,48,48,49,48,102,102,102,102},10);
run("wide-over",(const unsigned char[]){92,85,48,48,49,49,48,48,48,48},10);
run("wide-long",(const unsigned char[]){92,85,102,102,102,102,102,102,102,102},10);
run("wide-cap-suffix",(const unsigned char[]){92,85,49,49,48,48,48,48,88},9);
run("xlong",(const unsigned char[]){92,120,49,50,51,52,53,54,55,56,57,97,98,99,100,101,102,88},18);
run("jim-braced-max",(const unsigned char[]){92,117,123,49,102,102,102,102,102,125},10);
run("jim-braced-over",(const unsigned char[]){92,117,123,50,48,48,48,48,48,125},10);
run("raw-nul",(const unsigned char[]){0},1);
run("raw-ff",(const unsigned char[]){255},1);
run("raw-utf8-ff",(const unsigned char[]){195,191},2);
run("raw-surrogate",(const unsigned char[]){237,160,128},3);
run("raw-astral",(const unsigned char[]){240,159,152,128},4);
run("escaped-nul",(const unsigned char[]){92,0},2);
run("escaped-ff",(const unsigned char[]){92,255},2);
run("escaped-80",(const unsigned char[]){92,128},2);
run("escaped-utf8-ff",(const unsigned char[]){92,195,191},3);
run("escaped-surrogate",(const unsigned char[]){92,237,160,128},4);
run("escaped-astral",(const unsigned char[]){92,240,159,152,128},5);
run("escaped-incomplete",(const unsigned char[]){92,226,130},3);
run("escaped-invalid-lead",(const unsigned char[]){92,193,129},3);
run("escaped-modified-nul",(const unsigned char[]){92,192,128},3);
run("escaped-modified-overlong",(const unsigned char[]){92,192,129},3);
run("raw-surrogate-pair",(const unsigned char[]){237,160,189,237,184,128},6);
run("escaped-surrogate-pair",(const unsigned char[]){92,237,160,189,237,184,128},7);
run("escaped-nul-tail",(const unsigned char[]){92,0,122},3);
run("u-surrogate-pair",(const unsigned char[]){92,117,100,56,51,100,92,117,100,101,48,48},12);
run("u-low-surrogate",(const unsigned char[]){92,117,100,99,48,48},6);
run("wide-surrogate",(const unsigned char[]){92,85,48,48,48,48,100,56,48,48},10);
run("escaped-truncated-astral",(const unsigned char[]){92,240,159,152},4);
run("escaped-truncated-two",(const unsigned char[]){92,195},2);
run("unknown-ascii",(const unsigned char[]){92,81},2);
run("empty-hex",(const unsigned char[]){92,120},2);
#ifndef USE_JIM
Tcl_Finalize();
#endif
return 0;}
