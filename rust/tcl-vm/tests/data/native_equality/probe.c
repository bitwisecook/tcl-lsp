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



struct Pair {const char *name;const unsigned char *left;int leftn;const unsigned char *right;int rightn;};
static const unsigned char nulx[]={'a',0,'x'},nuly[]={'a',0,'y'},modnulx[]={'a',0xc0,0x80,'x'},rawff[]={'a',0xff,'x'},modff[]={'a',0xc3,0xbf,'x'},euroraw[]={'a',0x80,'x'},euroutf[]={'a',0xe2,0x82,0xac,'x'},eacute1[]={0xc3,0xa9,'x'},eacute2[]={0xc3,0xaa,'x'},astral1[]={0xf0,0x9f,0x98,0x80,'x'},astral2[]={0xf0,0x9f,0x98,0x81,'x'},invalid1[]={0xff,'a'},invalid2[]={0xff,'b'};
static const struct Pair pairs[]={
{"nul-tail",nulx,sizeof(nulx),nuly,sizeof(nuly)},
{"raw-modified-nul",nulx,sizeof(nulx),modnulx,sizeof(modnulx)},
{"raw-modified-ff",rawff,sizeof(rawff),modff,sizeof(modff)},
{"raw-cp1252-euro",euroraw,sizeof(euroraw),euroutf,sizeof(euroutf)},
{"utf8-two-byte",eacute1,sizeof(eacute1),eacute2,sizeof(eacute2)},
{"astral",astral1,sizeof(astral1),astral2,sizeof(astral2)},
{"invalid-leading",invalid1,sizeof(invalid1),invalid2,sizeof(invalid2)}};
static Obj*input(const unsigned char*s,int n,int rep){Obj *o;
#ifdef USE_JIM
o=NEW(active,(const char*)s,n);if(rep==1)(void)Jim_Utf8Length(active,o);
#else
if(rep>=2)o=Tcl_NewByteArrayObj(s,n);else o=NEW(active,(const char*)s,n);if(rep==1){Len len;(void)Tcl_GetUnicodeFromObj(o,&len);}if(rep==3){Len len;(void)GET(o,&len);}
#endif
return o;}

#if !defined(USE_JIM) && (TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 6)
extern int TclStringCmp(Obj*,Obj*,int,int,Len);
#endif
static void physical(const char*phase,Obj*l,Obj*r){printf("{\"case\":\"%s\",\"op\":\"%s\",\"left_type\":\"%s\",\"left_resident\":%d,\"right_type\":\"%s\",\"right_resident\":%d}\n",caseid,phase,l->typePtr?l->typePtr->name:"none",l->bytes!=NULL,r->typePtr?r->typePtr->name:"none",r->bytes!=NULL);}
#if defined(USE_JIM) || TCL_MAJOR_VERSION>8 || TCL_MINOR_VERSION>=6
static void equality(const struct Pair*pair,int lr,int rr,int same){char id[160];snprintf(id,sizeof(id),"%s-L%d-R%d-same%d",pair->name,lr,rr,same);caseid=id;active=create();Obj*l=input(pair->left,pair->leftn,lr);INC(l);Obj*r=same?l:input(pair->right,pair->rightn,rr);INC(r);physical("before",l,r);
#ifdef USE_JIM
int compared=Jim_StringCompareObj(active,l,r,0);
#else
int compared=TclStringCmp(l,r,1,0,-1);
#endif
printf("{\"case\":\"%s\",\"op\":\"equality\",\"equal\":%d}\n",caseid,compared==0);physical("after",l,r);DEC(active,l);DEC(active,r);DESTROY(active);}
#endif
int main(void){setvbuf(stdout,NULL,_IONBF,0);
#if !defined(USE_JIM) && TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION<6
puts("{\"op\":\"TclStringCmp-entry\",\"available\":false}");
#else
for(int p=0;p<sizeof(pairs)/sizeof(*pairs);p++)for(int l=0;l<
#ifdef USE_JIM
2
#else
4
#endif
;l++)for(int r=0;r<
#ifdef USE_JIM
2
#else
4
#endif
;r++)for(int same=0;same<2;same++)equality(&pairs[p],l,r,same);
#endif
return 0;}
