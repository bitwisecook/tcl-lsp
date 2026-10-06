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


static void probe(const char*id,const char*src){caseid=id;pathid="fixed-grammar";active=create();Obj*o=word(src);INC(o);int code=SCRIPT(active,o);report("native-result",code);DEC(active,o);DESTROY(active);}
int main(void){setvbuf(stdout,NULL,_IONBF,0);
probe("empty-bind","try {error MSG} on error {{} o} {list [info exists {}] [set {}]}");
probe("empty-options-bind","try {error MSG} on error {m {}} {list [info exists {}] [set m]}");
probe("malformed-unmatched-vars","set broken [format %c 123]; set side BEFORE; catch {try {set side AFTER} on error $broken {set x y}} m; list $side $m");
probe("malformed-matching-vars","set broken [format %c 123]; set side BEFORE; catch {try {set side AFTER; error MSG} on error $broken {set x y}} m; list $side $m");
probe("malformed-later-handler","set broken [format %c 123]; set side BEFORE; catch {try {set side AFTER; error MSG} on error {m} {set selected YES} trap $broken {x} {set selected NO}} m; list $side $m");
probe("multi-on-code","try {error MSG} on {ok error} {m} {set m}");
probe("on-abbreviation","try {error MSG} o error {m} {set m}");
probe("dash-fallthrough","try {error MSG} on error {m} - on ok {m} {set m}");
probe("trap-ok","try {set v VALUE} trap {NONE} {m} {list HIT $m}");
probe("malformed-handler-type","set side BEFORE; catch {try {set side AFTER} wat code {m} script} m; list $side $m");
probe("finally-after-error","try {error MSG} finally {set cleanup YES}");
probe("empty-trap-pattern","try {error MSG} trap {} {m} {set m}");
probe("bind-array","try {error MSG} on error {a(k) o} {list $a(k) [info exists a(k)]}");
probe("missing-namespace-binding","try {error MSG} on error {absent::x o} {set o} finally {set cleanup YES}");
probe("noerror-option","try -noerror {error MSG} on error {m} {set m} finally {set cleanup YES}");
return 0;}
