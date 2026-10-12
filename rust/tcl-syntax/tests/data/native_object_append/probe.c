#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Obj Obj; typedef Jim_Interp ProbeInterp; typedef int Len;
#define NEW(i,s,n) Jim_NewStringObj(i,s,n)
#define INC(o) Jim_IncrRefCount(o)
#define DEC(i,o) Jim_DecrRefCount(i,o)
#define GET(o,n) Jim_GetString(o,n)
#define RESULT(i) Jim_GetResult(i)
#define LIST(i,n,v) Jim_NewListObj(i,v,n)
#define VAR(i,n) Jim_GetVariableStr(i,n,0)
#define EVAL(i,n,v) Jim_EvalObjVector(i,n,v)
#define SCRIPT(i,o) Jim_EvalObj(i,o)
static ProbeInterp *create(void){ProbeInterp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK){fprintf(stderr,"Jim static extensions failed: %s\n",Jim_String(Jim_GetResult(i)));exit(2);}return i;}
#define DESTROY(i) Jim_FreeInterp(i)
#else
#include "tcl.h"
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
#include "tclInt.h"
#endif
typedef Tcl_Obj Obj; typedef Tcl_Interp ProbeInterp;
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
static ProbeInterp *create(void){return Tcl_CreateInterp();}
#define DESTROY(i) Tcl_DeleteInterp(i)
#endif
static const char *caseid,*pathid; static ProbeInterp *active;
static void hexobj(Obj *o){Len n=0; const unsigned char *s; if(!o){printf("null");return;} s=(const unsigned char *)GET(o,&n);putchar('"'); for(Len j=0;j<n;j++)printf("%02x",s[j]);putchar('"');}
static Obj *observer_global_storage(const char *name) {
#if !defined(USE_JIM) && TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
    Var *variable = (Var *)Tcl_FindNamespaceVar(active, name, NULL, TCL_GLOBAL_ONLY);
    while (variable && TclIsVarLink(variable)) variable = variable->value.linkPtr;
    return variable && TclIsVarScalar(variable) && !TclIsVarUndefined(variable)
        ? variable->value.objPtr : NULL;
#elif defined(USE_JIM)
    return VAR(active, name);
#else
    (void)name;
    return NULL;
#endif
}
static void report(const char *op,int code) {
    Obj *saved_result = RESULT(active), *options = NULL;
    Obj *saved_code = NULL, *saved_info = NULL;
    INC(saved_result);
#if !defined(USE_JIM) && (TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5)
    options = Tcl_GetReturnOptions(active,code);
    INC(options);
    Obj *code_key = NEW(active,"-errorcode",10), *info_key = NEW(active,"-errorinfo",10);
    INC(code_key); INC(info_key);
    (void)Tcl_DictObjGet(NULL,options,code_key,&saved_code);
    (void)Tcl_DictObjGet(NULL,options,info_key,&saved_info);
    DEC(active,code_key); DEC(active,info_key);
#else
    saved_code = observer_global_storage("errorCode");
    saved_info = observer_global_storage("errorInfo");
#endif
    if (saved_code) INC(saved_code);
    if (saved_info) INC(saved_info);
    printf("{\"case\":\"%s\",\"path\":\"%s\",\"op\":\"%s\",\"code\":%d,\"result\":",caseid,pathid,op,code);
    hexobj(saved_result);
    printf(",\"result_type\":\"%s\",\"error_code\":",saved_result->typePtr?saved_result->typePtr->name:"string");
    hexobj(saved_code); printf(",\"error_info\":");hexobj(saved_info);
    printf(",\"return_options\":");hexobj(options);puts("}");
    if (saved_code) DEC(active,saved_code);
    if (saved_info) DEC(active,saved_info);
    if (options) DEC(active,options);
    DEC(active,saved_result);
}

static Obj *word(const char*s){return NEW(active,s,-1);}
static int call(const char*op,int n,Obj**v){for(int j=0;j<n;j++)INC(v[j]);int c=EVAL(active,n,v);report(op,c);for(int j=0;j<n;j++)DEC(active,v[j]);return c;}
#define C1(op,a) do{Obj*v[]={a};call(op,1,v);}while(0)
#define C2(op,a,b) do{Obj*v[]={a,b};call(op,2,v);}while(0)
#define C3(op,a,b,c) do{Obj*v[]={a,b,c};call(op,3,v);}while(0)
#define C4(op,a,b,c,d) do{Obj*v[]={a,b,c,d};call(op,4,v);}while(0)
#define C6(op,a,b,c,d,e,f) do{Obj*v[]={a,b,c,d,e,f};call(op,6,v);}while(0)
static void objrow(const char *op,Obj*o){printf("{\"case\":\"%s\",\"path\":\"%s\",\"op\":\"%s\",\"object_type_before\":\"%s\",\"bytes\":",caseid,pathid,op,o->typePtr?o->typePtr->name:"string");hexobj(o);printf(",\"object_type_after\":\"%s\"}\n",o->typePtr?o->typePtr->name:"string");}
#ifdef USE_JIM
static int tracecmd(ProbeInterp*i,int n,Obj*const*v){for(int j=1;j<n;j++)objrow(j==1?"trace-name1":j==2?"trace-name2":"trace-op",v[j]);Jim_SetResultString(i,"",0);return JIM_OK;}
#else
static int tracecmd(ClientData d,ProbeInterp*i,int n,Obj*const*v){(void)d;for(int j=1;j<n;j++)objrow(j==1?"trace-name1":j==2?"trace-name2":"trace-op",v[j]);Tcl_ResetResult(i);return TCL_OK;}
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


#define C5(op,a,b,c,d,e) do{Obj*v[]={a,b,c,d,e};call(op,5,v);}while(0)
static void trace_registration(const char *op,Obj *name){
#if !defined(USE_JIM) && TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
C5(op,word("trace"),word("variable"),name,word("w"),word("capture"));
#else
C6(op,word("trace"),word("add"),word("variable"),name,word("write"),word("capture"));
#endif
}


static const char *kinds[]={"missing","empty-string","text-string","integer","double","empty-list","typed-list","pure-empty-ba","pure-ba","resident-ba"};
static Obj *makevalue(int kind){Obj*v;static const unsigned char raw[]={0xff,0};switch(kind){case 1:return word("");case 2:return word("TEXT");case 3:
#ifdef USE_JIM
return Jim_NewIntObj(active,5);
#else
return Tcl_NewIntObj(5);
#endif
case 4:
#ifdef USE_JIM
return Jim_NewDoubleObj(active,1.5);
#else
return Tcl_NewDoubleObj(1.5);
#endif
case 5:return LIST(active,0,NULL);case 6:{Obj *words[]={word("alpha"),word("beta gamma")};return LIST(active,2,words);}case 7:case 8:case 9:
#ifndef USE_JIM
v=Tcl_NewByteArrayObj(raw,kind==7?0:sizeof(raw));if(kind==9){Len n;GET(v,&n);}return v;
#else
return NULL;
#endif
default:return NULL;}}
static void physical(const char *phase,Obj*source){Obj*dest=VAR(active,"v");printf("{\"case\":\"%s\",\"path\":\"%s\",\"op\":\"%s\",\"source_type\":\"%s\",\"source_resident\":%d,\"dest_type\":\"%s\",\"dest_resident\":%d}\n",caseid,pathid,phase,source->typePtr?source->typePtr->name:"none",source->bytes!=NULL,dest?(dest->typePtr?dest->typePtr->name:"none"):"missing",dest&&dest->bytes!=NULL);}
static int quiet(const char*op,int n,Obj**v){for(int j=0;j<n;j++)INC(v[j]);int code=EVAL(active,n,v);printf("{\"case\":\"%s\",\"path\":\"%s\",\"op\":\"%s\",\"code\":%d}\n",caseid,pathid,op,code);for(int j=0;j<n;j++)DEC(active,v[j]);return code;}
static void probe(int destkind,int sourcekind,int mode){char id[160];snprintf(id,sizeof(id),"dest-%s-source-%s",kinds[destkind],kinds[sourcekind]);caseid=id;pathid=mode==0?"ordinary":mode==1?"shared-destination":"self-alias";active=create();if(destkind){Obj*dest=makevalue(destkind);INC(dest);Obj*set[]={word("set"),word("v"),dest};quiet("destination-set",3,set);if(mode==1){Obj*share[]={word("set"),word("alias"),dest};quiet("share-destination",3,share);}DEC(active,dest);}Obj*source=mode==2?VAR(active,"v"):makevalue(sourcekind);if(!source){DESTROY(active);return;}INC(source);physical("physical-before",source);Obj*append[]={word("append"),word("v"),source};for(int j=0;j<3;j++)INC(append[j]);int code=EVAL(active,3,append);physical("physical-after-native-append",source);report("append-result",code);for(int j=0;j<3;j++)DEC(active,append[j]);C2("variable-readback",word("set"),word("v"));if(mode==1)C2("shared-readback",word("set"),word("alias"));objrow("source-representation",source);DEC(active,source);DESTROY(active);}
int main(void){setvbuf(stdout,NULL,_IONBF,0);for(int dest=0;dest<
#ifdef USE_JIM
7
#else
10
#endif
;dest++)for(int source=1;source<
#ifdef USE_JIM
7
#else
10
#endif
;source++)for(int mode=0;mode<2;mode++)probe(dest,source,mode);for(int dest=1;dest<
#ifdef USE_JIM
7
#else
10
#endif
;dest++)probe(dest,dest,2);return 0;}
