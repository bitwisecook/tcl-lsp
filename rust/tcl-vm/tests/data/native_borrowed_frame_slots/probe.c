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

static Obj *first_name,*second_name;
#ifdef USE_JIM
static int namea(ProbeInterp*i,int n,Obj*const*v){(void)n;(void)v;Jim_SetResult(i,first_name);return JIM_OK;}
static int nameb(ProbeInterp*i,int n,Obj*const*v){(void)n;(void)v;Jim_SetResult(i,second_name);return JIM_OK;}
#else
static int namea(ClientData d,ProbeInterp*i,int n,Obj*const*v){(void)d;(void)n;(void)v;Tcl_SetObjResult(i,first_name);return TCL_OK;}
static int nameb(ClientData d,ProbeInterp*i,int n,Obj*const*v){(void)d;(void)n;(void)v;Tcl_SetObjResult(i,second_name);return TCL_OK;}
#endif

static Obj *scriptb(ProbeInterp*i,int write){Len len;const char*name=GET(second_name,&len);char*buf=malloc(2*len+40);int n=0;memcpy(buf+n,"set {",5);n+=5;memcpy(buf+n,name,len);n+=len;memcpy(buf+n,write?"} ALTER;set {":"};set {",write?13:7);n+=write?13:7;memcpy(buf+n,name,len);n+=len;buf[n++]='}';Obj*o=NEW(i,buf,n);free(buf);return o;}
#ifdef USE_JIM
static int sourceb(ProbeInterp*i,int n,Obj*const*v){(void)v;Jim_SetResult(i,scriptb(i,n>1));return JIM_OK;}
#else
static int sourceb(ClientData d,ProbeInterp*i,int n,Obj*const*v){(void)d;(void)v;Tcl_SetObjResult(i,scriptb(i,n>1));return TCL_OK;}
#endif
static void run_pair(const char *id,const unsigned char *a,int na,const unsigned char *b,int nb,int duplicate){
 caseid=id;pathid=duplicate?"two-original-formals":"one-original-formal";active=create();
 first_name=NEW(active,(const char*)a,na);INC(first_name);second_name=NEW(active,(const char*)b,nb);INC(second_name);
#ifdef USE_JIM
 Jim_CreateCommand(active,"originalBScript",sourceb,NULL,NULL);Jim_CreateCommand(active,"originalA",namea,NULL,NULL);Jim_CreateCommand(active,"originalB",nameb,NULL,NULL);
#else
 Tcl_CreateObjCommand(active,"originalBScript",sourceb,NULL,NULL);Tcl_CreateObjCommand(active,"originalA",namea,NULL,NULL);Tcl_CreateObjCommand(active,"originalB",nameb,NULL,NULL);
#endif
 Obj *formalv[]={first_name,second_name};Obj *params=LIST(active,duplicate?2:1,formalv);INC(params);
 char buffer[]="set [originalB] DYNAMIC;set output {};foreach how {eval uplevel} {if {[string equal $how eval]} {set c [catch {eval [originalBScript]} r]} else {set c [catch {uplevel 0 [originalBScript]} r]};lappend output $how $c $r};lappend output dynamicA [set [originalA]] dynamicB [set [originalB]];set c [catch {eval [originalBScript write]} r];lappend output write $c $r afterA [set [originalA]] afterB [set [originalB]];set output";int len=sizeof(buffer)-1;
 Obj *body=NEW(active,buffer,len);INC(body);
 C4("original-definition",word("proc"),word("p"),params,body);
 if(duplicate)C3("compiled-and-dynamic-invocation",word("p"),word("ONE"),word("TWO"));else C2("compiled-and-dynamic-invocation",word("p"),word("ONE"));
 DEC(active,body);DEC(active,params);DEC(active,first_name);DEC(active,second_name);DESTROY(active);
}
int main(void){setvbuf(stdout,NULL,_IONBF,0);
static const unsigned char raw_a[]={107,0,97},raw_b[]={107,0,98},raw_bb[]={107,0,98,98};
static const unsigned char mod_a[]={107,0xc0,0x80,97},mod_b[]={107,0xc0,0x80,98};
static const unsigned char qual_a[]={107,0,58,58,97},qual_b[]={107,0,58,58,98};
static const unsigned char before_a[]={107,58,58,0,97},before_b[]={107,58,58,0,98};
static const unsigned char opaque_a[]={107,0,97,40,113,41},opaque_b[]={107,0,98,40,113,41};
static const unsigned char ordinary_a[]={97,40,113,41},ordinary_b[]={97,40,114,41};
static const unsigned char ff_a[]={107,0xff,97},ff_b[]={107,0xff,98};
static const unsigned char ascii[]={120};
for(int duplicate=0;duplicate<2;duplicate++){
run_pair("ordinary-duplicate",ascii,1,ascii,1,duplicate);
run_pair("nul-equal-length",raw_a,3,raw_b,3,duplicate);
run_pair("nul-different-length",raw_a,3,raw_bb,4,duplicate);
run_pair("modified-nul",mod_a,4,mod_b,4,duplicate);
run_pair("qualification-after-nul",qual_a,5,qual_b,5,duplicate);
run_pair("qualification-before-nul",before_a,5,before_b,5,duplicate);
run_pair("opaque-array-after-nul",opaque_a,6,opaque_b,6,duplicate);
run_pair("opaque-array-ordinary",ordinary_a,4,ordinary_b,4,duplicate);
run_pair("raw-invalid-byte",ff_a,3,ff_b,3,duplicate);
run_pair("identical-duplicate",raw_a,3,raw_a,3,duplicate);
}return 0;}
