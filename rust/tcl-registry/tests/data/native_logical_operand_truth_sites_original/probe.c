/* Public primitive/expression and reached logical operand sites on held originals. */
#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
typedef int Count;
typedef jim_wide Wide;
static Interp *fresh(void) {
    Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);
    if(Jim_InitStaticExtensions(i)!=JIM_OK) { Jim_FreeInterp(i);return NULL; }
    return i;
}
static void retain(Obj *o) { Jim_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { Jim_DecrRefCount(i,o); }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
static Obj *string(Interp *i,const char *s,Count n) { return Jim_NewStringObj(i,s,n); }
static Obj *integer(Interp *i,Wide w) { return Jim_NewIntObj(i,w); }
static Obj *double_obj(Interp *i,double d) { return Jim_NewDoubleObj(i,d); }
static int install(Interp *i,Obj *o) { return Jim_SetVariableStr(i,"x",o); }
static Obj *variable(Interp *i) { return Jim_GetVariableStr(i,"x",JIM_NONE); }
static Obj *logical_partner(Interp *i,int value) { return Jim_NewIntObj(i,(Wide)value); }
static int install_y(Interp *i,Obj *o) { return Jim_SetVariableStr(i,"y",o); }
static Obj *variable_y(Interp *i) { return Jim_GetVariableStr(i,"y",JIM_NONE); }
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static const char *bytes(Obj *o,Count *n) { return Jim_GetString(o,n); }
static int eval(Interp *i,const char *s,Count n) {
    Obj *source=string(i,s,n);retain(source);
    int code=Jim_EvalObj(i,source);release(i,source);return code;
}
static int eval_original_object(Interp *i,Obj *source) { return Jim_EvalObj(i,source); }
static int primitive(Interp *i,Obj *o,int *b) { return Jim_GetBoolean(i,o,b); }
static int expression(Interp *i,Obj *o,int *b) { return Jim_GetBoolFromExpr(i,o,b); }
static void seed(Interp *i) {
    Jim_SetVariableStrWithStr(i,"errorCode","SEEDED CODE");
    Jim_SetResultString(i,"SEEDED RESULT",-1);
}
static Obj *error_code(Interp *i,int code) { (void)code;return Jim_GetVariableStr(i,"errorCode",JIM_NONE); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
typedef Tcl_WideInt Wide;
static Interp *fresh(void) {
    Interp *i=Tcl_CreateInterp();
    if(Tcl_Init(i)!=TCL_OK) { Tcl_DeleteInterp(i);return NULL; }
    return i;
}
static void retain(Obj *o) { Tcl_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { (void)i;Tcl_DecrRefCount(o); }
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
static Obj *string(Interp *i,const char *s,Count n) { (void)i;return Tcl_NewStringObj(s,n); }
static Obj *integer(Interp *i,Wide w) { (void)i;return Tcl_NewWideIntObj(w); }
static Obj *double_obj(Interp *i,double d) { (void)i;return Tcl_NewDoubleObj(d); }
static int install(Interp *i,Obj *o) { return Tcl_SetVar2Ex(i,"x",NULL,o,TCL_GLOBAL_ONLY)?TCL_OK:TCL_ERROR; }
static Obj *variable(Interp *i) { return Tcl_GetVar2Ex(i,"x",NULL,TCL_GLOBAL_ONLY); }
static Obj *logical_partner(Interp *i,int value) { (void)i;return Tcl_NewIntObj(value); }
static int install_y(Interp *i,Obj *o) { return Tcl_SetVar2Ex(i,"y",NULL,o,TCL_GLOBAL_ONLY)?TCL_OK:TCL_ERROR; }
static Obj *variable_y(Interp *i) { return Tcl_GetVar2Ex(i,"y",NULL,TCL_GLOBAL_ONLY); }
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj *o,Count *n) { return Tcl_GetStringFromObj(o,n); }
static int eval(Interp *i,const char *s,Count n) { return Tcl_EvalEx(i,s,n,0); }
static int eval_original_object(Interp *i,Obj *source) { return Tcl_EvalObjEx(i,source,0); }
static int primitive(Interp *i,Obj *o,int *b) { return Tcl_GetBooleanFromObj(i,o,b); }
static int expression(Interp *i,Obj *o,int *b) { return Tcl_ExprBooleanObj(i,o,b); }
static void seed(Interp *i) {
    Tcl_SetVar(i,"errorCode","SEEDED CODE",TCL_GLOBAL_ONLY);
    Tcl_SetErrorCode(i,"SEEDED","CODE",NULL);
    Tcl_SetObjResult(i,Tcl_NewStringObj("SEEDED RESULT",-1));
}
static Obj *error_code(Interp *i,int code) {
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
    Obj *opts=Tcl_GetReturnOptions(i,code),*key=Tcl_NewStringObj("-errorcode",-1),*ec=NULL;
    retain(opts);retain(key);
    if(Tcl_DictObjGet(NULL,opts,key,&ec)!=TCL_OK)ec=NULL;
    if(ec)retain(ec);release(i,key);release(i,opts);return ec;
#else
    (void)code;Obj *ec=Tcl_GetVar2Ex(i,"errorCode",NULL,TCL_GLOBAL_ONLY);
    if(ec)retain(ec);return ec;
#endif
}
#endif
static void hex_object(Obj *o) {
    if(!o) { printf("MISSING");return; }
    Count n;const char *s=bytes(o,&n);
    for(Count at=0;at<n;at++)printf("%02x",(unsigned char)s[at]);
}
static const char *type(Obj *o) { return o && o->typePtr?o->typePtr->name:"NULL"; }
static const char *string_cases[]={
    "17","0","-0","-0.0","4294967296","NaN","bad","on","true","false"
};
static const int64_t wide_cases[]={
    INT64_C(2147483648),INT64_C(4294967295),INT64_C(4294967296),
    INT64_C(4294967297),-INT64_C(4294967296),-INT64_C(4294967295),INT64_MAX,INT64_MIN
};
enum { CASE_COUNT=36, WORKER_COUNT=18 };
static Obj *make(Interp *i,int at) {
    if(at<10)return string(i,string_cases[at],-1);
    if(at==10 || at==11) { char s[3]={(char)(at==10?'1':'0'),0,'X'};return string(i,s,3); }
    if(at>=12 && at<=16) {
        const int values[]={17,0,-17,INT_MAX,INT_MIN};
#ifdef JIM_PROBE
        return Jim_NewIntObj(i,(Wide)values[at-12]);
#else
        return Tcl_NewIntObj(values[at-12]);
#endif
    }
    if(at>=17 && at<=24)return integer(i,(Wide)wide_cases[at-17]);
    if(at>=25 && at<=31) {
        const double values[]={17.0,0.0,-0.0,NAN,4294967296.0,INFINITY,-INFINITY};
        return double_obj(i,values[at-25]);
    }
    if(at==32)return string(i,"1.0",3);
    if(at==33)return string(i,"08",2);
    if(at==34)return string(i,"9223372036854775808",19);
    return string(i,"",0);
}
static const char *scripts[]={
    "$x\n",
    "expr {!!$x}\n",
    "expr {$x ? 1 : 0}\n",
    "if {$x} {list 1} else {list 0}\n",
    "set iterations 0\nwhile {$x} {incr iterations; break}\nset iterations\n",
    "expr {$x && $y}\n",
    "expr {$y && $x}\n",
    "expr {$x || $y}\n",
    "expr {$y || $x}\n"
};
static int metadata(void) {
    Interp *i=fresh();if(!i)return 3;
#ifdef JIM_PROBE
    const char *source="list [info patchlevel] [info version]";
#else
    const char *source="list [info patchlevel] [info tclversion]";
#endif
    int code=eval(i,source,(Count)strlen(source));
    printf("VERSION\t%d\t",code);hex_object(result(i));putchar('\n');
    printf("ABI\tint=%zu\tlong=%zu\twide=%zu\tdouble=%zu\tptr=%zu\tCHAR_BIT=%d\tINT_MIN=%d\tINT_MAX=%d\tLONG_MIN=%ld\tLONG_MAX=%ld\n",
        sizeof(int),sizeof(long),sizeof(Wide),sizeof(double),sizeof(void *),CHAR_BIT,INT_MIN,INT_MAX,LONG_MIN,LONG_MAX);
#ifdef JIM_PROBE
    printf("BUILD\tJIM_GITVERSION=%s\tJIM_VERSION=%d\tJIM_ABI_VERSION=%d\tJIM_UTF8=%d\n",JIM_GITVERSION,JIM_VERSION,JIM_ABI_VERSION,JIM_UTF8);
#else
    int major,minor,patch,release;Tcl_GetVersion(&major,&minor,&patch,&release);
    printf("BUILD\tHEADER=%s\tGetVersion=%d.%d.%d/%d\tTcl_UniChar=%zu\n",TCL_PATCH_LEVEL,major,minor,patch,release,sizeof(Tcl_UniChar));
#endif
    destroy(i);return code==0?0:4;
}
int main(int argc,char **argv) {
    (void)argc;
#ifndef JIM_PROBE
    Tcl_FindExecutable(argv[0]);
#else
    (void)argv;
#endif
    int status=metadata();if(status)return status;
    for(int at=0;at<CASE_COUNT;at++)for(int worker=0;worker<WORKER_COUNT;worker++) {
        Interp *i=fresh();if(!i)return 3;
        Obj *o=make(i,at);if(!o)return 5;retain(o);
        const char *constructed_type=type(o);int constructed_resident=o->bytes!=NULL;
        int installed=install(i,o);
        if(installed!=0) {
            printf("SETUP_FAILURE\tcase=%d\tworker=%d\tcode=%d\tresult=",at,worker,installed);
            hex_object(result(i));putchar('\n');release(i,o);destroy(i);return 6;
        }
        int source_worker=worker>=10?worker-8:worker;
        Obj *script_source=NULL;
        if(worker>=10) {
            const char *source=scripts[source_worker-1];
            script_source=string(i,source,(Count)strlen(source));if(!script_source)return 5;retain(script_source);
        }
        Obj *y=NULL;
        if(source_worker>=6) {
            /* AND y=1 and OR y=0 ensure x is reached in either operand position. */
            y=logical_partner(i,source_worker<8?1:0);if(!y)return 5;retain(y);
            int y_installed=install_y(i,y);
            if(y_installed!=0) {
                printf("SETUP_FAILURE_Y\tcase=%d\tworker=%d\tcode=%d\tresult=",at,worker,y_installed);
                hex_object(result(i));putchar('\n');release(i,y);if(script_source)release(i,script_source);release(i,o);destroy(i);return 7;
            }
        }
        seed(i);
        Obj *before_variable=variable(i);const char *before_type=type(o);char *before_bytes=o->bytes;
        printf("BEFORE\tcase=%d\tworker=%d\tconstructed=%s\tconstructed_string=%d\tcache=%s\tstring=%d\tvariable_same=%d\trefcount=%lld",
            at,worker,constructed_type,constructed_resident,before_type,before_bytes!=NULL,before_variable==o,(long long)o->refCount);
        if(y)printf("\ty_value=%d\ty_cache=%s\ty_string=%d\ty_variable_same=%d\ty_refcount=%lld",
            source_worker<8?1:0,type(y),y->bytes!=NULL,variable_y(i)==y,(long long)y->refCount);
        if(script_source)printf("\tscript_cache=%s\tscript_string=%d\tscript_refcount=%lld",
            type(script_source),script_source->bytes!=NULL,(long long)script_source->refCount);
        putchar('\n');fflush(stdout);
        int output=777,code;errno=0;
        if(worker==0)code=primitive(i,o,&output);
        else if(worker==1) {
            Obj *expression_source=string(i,scripts[0],(Count)strlen(scripts[0]));retain(expression_source);
            code=expression(i,expression_source,&output);release(i,expression_source);
        } else if(script_source)code=eval_original_object(i,script_source);
        else code=eval(i,scripts[worker-1],(Count)strlen(scripts[worker-1]));
        int reached_errno=errno;const char *after_type=type(o);char *after_bytes=o->bytes;
        Obj *after_variable=variable(i);
        printf("ROW\tcase=%d\tworker=%d\tcode=%d\tout=%d\terrno=%d\tcache=%s\tstring=%d\tstring_same=%d\tvariable_same=%d\tvariable_cache=%s",
            at,worker,code,output,reached_errno,after_type,after_bytes!=NULL,before_bytes && before_bytes==after_bytes,after_variable==o,type(after_variable));
        if(y)printf("\ty_cache=%s\ty_string=%d\ty_variable_same=%d\ty_refcount=%lld",
            type(y),y->bytes!=NULL,variable_y(i)==y,(long long)y->refCount);
        if(script_source)printf("\tscript_cache=%s\tscript_string=%d\tscript_refcount=%lld",
            type(script_source),script_source->bytes!=NULL,(long long)script_source->refCount);
        printf("\tresult=");hex_object(result(i));printf("\terror_code=");Obj *ec=error_code(i,code);hex_object(ec);
#ifndef JIM_PROBE
        if(ec)release(i,ec);
#endif
        printf("\tinput_after_observers=");hex_object(o);putchar('\n');fflush(stdout);
        if(y)release(i,y);if(script_source)release(i,script_source);release(i,o);destroy(i);
    }
    return 0;
}
