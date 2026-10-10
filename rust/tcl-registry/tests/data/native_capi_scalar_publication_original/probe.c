/* Exact public native scalar getter/ABI capture; no implementation predictions. */
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
    Interp *i=Jim_CreateInterp();
    Jim_RegisterCoreCommands(i);
    if(Jim_InitStaticExtensions(i)!=JIM_OK) { Jim_FreeInterp(i); return NULL; }
    return i;
}
static void release(Interp *i,Obj *o) { Jim_DecrRefCount(i,o); }
static void retain(Obj *o) { Jim_IncrRefCount(o); }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
static Obj *string(Interp *i,const char *s,Count n) { return Jim_NewStringObj(i,s,n); }
static const char *bytes(Obj *o,Count *n) { return Jim_GetString(o,n); }
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static void seed(Interp *i) {
    Jim_SetVariableStrWithStr(i,"errorCode","SEEDED CODE");
    Jim_SetResultString(i,"SEEDED RESULT",-1);
}
static Obj *error_code(Interp *i,int code) {
    (void)code;
    return Jim_GetVariableStr(i,"errorCode",JIM_NONE);
}
static void version(Interp *i) {
    int code=Jim_Eval(i,"list [info patchlevel] [info version]");
    Count n; const char *s=bytes(result(i),&n);
    printf("VERSION\t%d\t",code);
    for(Count at=0;at<n;at++)printf("%02x",(unsigned char)s[at]);
    printf("\tJIM_VERSION=%d\tJIM_ABI_VERSION=%d\n",JIM_VERSION,JIM_ABI_VERSION);
}
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
    if(Tcl_Init(i)!=TCL_OK) { Tcl_DeleteInterp(i); return NULL; }
    return i;
}
static void release(Interp *i,Obj *o) { (void)i; Tcl_DecrRefCount(o); }
static void retain(Obj *o) { Tcl_IncrRefCount(o); }
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
static Obj *string(Interp *i,const char *s,Count n) { (void)i; return Tcl_NewStringObj(s,n); }
static const char *bytes(Obj *o,Count *n) { return Tcl_GetStringFromObj(o,n); }
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static void seed(Interp *i) {
    Tcl_SetVar(i,"errorCode","SEEDED CODE",TCL_GLOBAL_ONLY);
    Tcl_SetErrorCode(i,"SEEDED","CODE",NULL);
    Tcl_SetObjResult(i,Tcl_NewStringObj("SEEDED RESULT",-1));
}
static Obj *error_code(Interp *i,int code) {
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
    Obj *opts=Tcl_GetReturnOptions(i,code),*key=Tcl_NewStringObj("-errorcode",-1),*ec=NULL;
    retain(opts); retain(key);
    if(Tcl_DictObjGet(NULL,opts,key,&ec)!=TCL_OK)ec=NULL;
    if(ec)retain(ec);
    release(i,key); release(i,opts);
    return ec;
#else
    (void)code;
    Obj *ec=Tcl_GetVar2Ex(i,"errorCode",NULL,TCL_GLOBAL_ONLY);
    if(ec)retain(ec);
    return ec;
#endif
}
static void version(Interp *i) {
    int major,minor,patch,type;
    Tcl_GetVersion(&major,&minor,&patch,&type);
    int code=Tcl_Eval(i,"list [info patchlevel] [info tclversion]");
    Count n; const char *s=bytes(result(i),&n);
    printf("VERSION\t%d\t",code);
    for(Count at=0;at<n;at++)printf("%02x",(unsigned char)s[at]);
    printf("\tGetVersion=%d.%d.%d/%d\tHEADER=%s\n",major,minor,patch,type,TCL_PATCH_LEVEL);
}
#endif

static const char *strings[]={
    "","17","2147483648","4294967295","4294967296","-4294967295",
    "-2147483649","9223372036854775807","9223372036854775808",
    "18446744073709551615","18446744073709551616","-18446744073709551615",
    "-9223372036854775809","08","0o10","true","bad","NaN","1.0"
};
enum { STRING_CASES=19, CASE_COUNT=29 };
static Obj *make(Interp *i,int at) {
    const char zero[]={'1',0,'X'};
    if(at<STRING_CASES)return string(i,strings[at],-1);
    switch(at) {
        case 19:return string(i,zero,3);
#ifdef JIM_PROBE
        case 20:return Jim_NewIntObj(i,17);
        case 21:return Jim_NewIntObj(i,(long)17);
        case 22:return Jim_NewIntObj(i,(Wide)17);
        case 23:return Jim_NewDoubleObj(i,17.0);
        case 24:return Jim_NewDoubleObj(i,NAN);
        case 25:return NULL; /* No claimed public Jim Boolean-object constructor. */
        case 26:return Jim_NewIntObj(i,(Wide)4294967296LL);
        case 27:return NULL; /* No claimed C byte-array constructor analogue. */
#else
        case 20:return Tcl_NewIntObj(17);
        case 21:return Tcl_NewLongObj((long)17);
        case 22:return Tcl_NewWideIntObj((Wide)17);
        case 23:return Tcl_NewDoubleObj(17.0);
        case 24:return Tcl_NewDoubleObj(NAN);
        case 25:return Tcl_NewBooleanObj(17);
        case 26:return Tcl_NewWideIntObj((Wide)4294967296LL);
        case 27:return Tcl_NewByteArrayObj((const unsigned char *)zero,3);
#endif
        default: {
            /* Fifty ASCII scalars plus a supplementary scalar, original counted UTF-8. */
            char text[55]; memset(text,'a',50);
            text[50]=(char)0xf0;text[51]=(char)0x9f;text[52]=(char)0x98;text[53]=(char)0x80;
            text[54]='Z';return string(i,text,55);
        }
    }
}
static void hex_object(Obj *o) {
    if(!o) { printf("MISSING"); return; }
    Count n; const char *s=bytes(o,&n);
    for(Count at=0;at<n;at++)printf("%02x",(unsigned char)s[at]);
}
static const char *type(Obj *o) { return o->typePtr?o->typePtr->name:"NULL"; }
static int get(Interp *i,Obj *o,int kind,int *iv,long *lv,Wide *wv,double *dv) {
#ifdef JIM_PROBE
    if(kind==0)return -999; /* Jim has no public GetIntFromObj analogue. */
    if(kind==1)return Jim_GetLong(i,o,lv);
    if(kind==2)return Jim_GetWide(i,o,wv);
    if(kind==3)return Jim_GetDouble(i,o,dv);
    return Jim_GetBoolean(i,o,iv);
#else
    if(kind==0)return Tcl_GetIntFromObj(i,o,iv);
    if(kind==1)return Tcl_GetLongFromObj(i,o,lv);
    if(kind==2)return Tcl_GetWideIntFromObj(i,o,wv);
    if(kind==3)return Tcl_GetDoubleFromObj(i,o,dv);
    return Tcl_GetBooleanFromObj(i,o,iv);
#endif
}
int main(int argc,char **argv) {
    if(argc!=2 || (strcmp(argv[1],"live") && strcmp(argv[1],"null")))return 2;
    int null_mode=!strcmp(argv[1],"null");
#ifdef JIM_PROBE
    if(null_mode) { puts("NOT_APPLICABLE\tJim public getter failures require a live interpreter");return 0; }
#else
    Tcl_FindExecutable(argv[0]);
#endif
    Interp *metadata=fresh();if(!metadata)return 3;version(metadata);destroy(metadata);
    printf("ABI\tint=%zu\tlong=%zu\twide=%zu\tdouble=%zu\tptr=%zu\tCHAR_BIT=%d\tLONG_MIN=%ld\tLONG_MAX=%ld\tULONG_MAX=%lu\n",
        sizeof(int),sizeof(long),sizeof(Wide),sizeof(double),sizeof(void *),CHAR_BIT,LONG_MIN,LONG_MAX,ULONG_MAX);
#ifndef JIM_PROBE
    printf("CHAR_UNITS\tTcl_UniChar=%zu\n",sizeof(Tcl_UniChar));
#ifdef TCL_WIDE_INT_IS_LONG
    puts("BUILD_MACRO\tTCL_WIDE_INT_IS_LONG=1");
#else
    puts("BUILD_MACRO\tTCL_WIDE_INT_IS_LONG=0");
#endif
#else
    printf("BUILD_MACRO\tJIM_UTF8=%d\tJIM_GITVERSION=%s\n",JIM_UTF8,JIM_GITVERSION);
#endif
    for(int at=0;at<CASE_COUNT;at++)for(int kind=0;kind<5;kind++) {
#ifdef JIM_PROBE
        if(kind==0 || at==25 || at==27) { printf("API_UNAVAILABLE\tcase=%d\tgetter=%d\n",at,kind);continue; }
#endif
        Interp *i=fresh();if(!i)return 3;
        Obj *o=make(i,at);if(!o)return 4;retain(o);seed(i);
        const char *before_type=type(o);char *before_bytes=o->bytes;
        int iv=777;long lv=777;Wide wv=777;double dv=777.25;
        errno=0;int code=get(null_mode?NULL:i,o,kind,&iv,&lv,&wv,&dv);int getter_errno=errno;
        union { double value;uint64_t bits; } double_bits;double_bits.value=dv;
        printf("ROW\tcase=%d\tgetter=%d\tmode=%s\tcode=%d\tint=%d\tlong=%ld\twide=%" PRId64 "\tdouble_bits=%016" PRIx64 "\terrno=%d\tbefore=%s\tafter=%s\tstring_before=%d\tstring_after=%d\tstring_same=%d\tresult=",
            at,kind,argv[1],code,iv,lv,(int64_t)wv,double_bits.bits,getter_errno,before_type,type(o),before_bytes!=NULL,o->bytes!=NULL,before_bytes && before_bytes==o->bytes);
        hex_object(result(i));printf("\terror_code=");
        Obj *ec=error_code(i,code);hex_object(ec);
#ifndef JIM_PROBE
        if(ec)release(i,ec);
#endif
        printf("\tinput_after_observers=");hex_object(o);putchar('\n');fflush(stdout);
        release(i,o);destroy(i);
    }
    return 0;
}
