#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK)return NULL; return i; }
static Obj *string(Interp *i,const char *p,int n) { return Jim_NewStringObj(i,p,n); }
static void hold(Obj *o) { Jim_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { Jim_DecrRefCount(i,o); }
static int eval(Interp *i,const char *p) { return Jim_Eval(i,p); }
static int invoke(Interp *i,int n,Obj **v) { for(int k=0;k<n;k++)hold(v[k]); int c=Jim_EvalObjVector(i,n,v); for(int k=0;k<n;k++)release(i,v[k]); return c; }
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static const char *bytes(Obj *o,long *n) { int size; const char *p=Jim_GetString(o,&size); *n=size; return p; }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
static Interp *fresh(void) { Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK)return NULL; return i; }
static Obj *string(Interp *i,const char *p,int n) { (void)i; return Tcl_NewStringObj(p,n); }
static void hold(Obj *o) { Tcl_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { (void)i; Tcl_DecrRefCount(o); }
static int eval(Interp *i,const char *p) { return Tcl_Eval(i,p); }
static int invoke(Interp *i,int n,Obj **v) { for(int k=0;k<n;k++)hold(v[k]); int c=Tcl_EvalObjv(i,n,v,0); for(int k=0;k<n;k++)release(i,v[k]); return c; }
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj *o,long *n) {
#if TCL_MAJOR_VERSION >= 9
Tcl_Size size;
#else
int size;
#endif
const char *p=Tcl_GetStringFromObj(o,&size); *n=(long)size; return p; }
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
#endif
static void hex(const char *p,long n) { for(long k=0;k<n;k++)printf("%02x",(unsigned char)p[k]); }
/* Pointer equality is sampled before the only result string getter. */
static void row(Interp *i,const char *label,int code,Obj *argument) { int same=argument && result(i)==argument; long n; const char *p=bytes(result(i),&n); printf("%s|%d|%d|",label,code,same); hex(p,n); puts(""); }
static int define(Interp *i,const char *command,const char *parameter,int n,const char *body,int m) { Obj *v[]={string(i,"proc",4),string(i,command,(int)strlen(command)),string(i,parameter,n),string(i,body,m)}; return invoke(i,4,v); }
static int call(Interp *i,const char *command,Obj *argument) { Obj *v[]={string(i,command,(int)strlen(command)),argument}; return invoke(i,2,v); }
static int body_for(char *destination,const char *parameter,int n) { const char *head="return ${"; int offset=(int)strlen(head); memcpy(destination,head,offset); memcpy(destination+offset,parameter,n); offset+=n; destination[offset++]='}'; return offset; }
int main(int argc,char **argv) {
    (void)argc;
#ifndef JIM_PROBE
    Tcl_FindExecutable(argv[0]);
#else
    (void)argv;
#endif
    Interp *startup=fresh(); if(!startup)return 2; row(startup,"VERSION",eval(startup,"info patchlevel"),NULL); destroy(startup);
    const char ascii[]="longargument", unicode[]="longcaf\xc3\xa9", supplementary[]="longp\xf0\x9f\x98\x80", zero[]="long\0tail", encoded[]="long\xc0\x80tail", opaque[]="long\xff", surrogate[]="long\xed\xa0\x80";
    const char *parameters[]={ascii,unicode,supplementary,zero,encoded,opaque,surrogate};
    const int lengths[]={sizeof(ascii)-1,sizeof(unicode)-1,sizeof(supplementary)-1,sizeof(zero)-1,sizeof(encoded)-1,sizeof(opaque)-1,sizeof(surrogate)-1};
    const char *names[]={"ASCII","UNICODE_E9","UNICODE_SUPPLEMENTARY","RAW_ZERO","ENCODED_ZERO","OPAQUE_FF","SURROGATE"};
    const char value[]="VALUE", empty[]="", raw[]="A\0B", value_opaque[]="A\xff", value_surrogate[]="A\xed\xa0\x80", value_encoded[]="A\xc0\x80";
    const char *values[]={value,empty,raw,value_opaque,value_surrogate,value_encoded};
    const int value_lengths[]={sizeof(value)-1,0,sizeof(raw)-1,sizeof(value_opaque)-1,sizeof(value_surrogate)-1,sizeof(value_encoded)-1};
    const char *value_names[]={"TEXT","EMPTY","RAW_ZERO","OPAQUE_FF","SURROGATE","ENCODED_ZERO"};
    for(int p=0;p<7;p++) {
        Interp *i=fresh(); if(!i)return 2; char body[256],label[128]; int body_length=body_for(body,parameters[p],lengths[p]);
        printf("INPUT|%s|",names[p]); hex(parameters[p],lengths[p]); printf("|"); hex(body,body_length); puts("");
        int original=define(i,"original",parameters[p],lengths[p],body,body_length); snprintf(label,sizeof(label),"%s_DECLARE_ORIGINAL",names[p]); row(i,label,original,NULL);
        int renamed=define(i,"renamed","a",1,"return $a",9); snprintf(label,sizeof(label),"%s_DECLARE_RENAMED",names[p]); row(i,label,renamed,NULL);
        for(int v=0;v<6;v++) {
            Obj *argument=string(i,values[v],value_lengths[v]); hold(argument);
            printf("VALUE|%s_%s|",names[p],value_names[v]); hex(values[v],value_lengths[v]); puts("");
            if(!original) { snprintf(label,sizeof(label),"%s_%s_ORIGINAL",names[p],value_names[v]); row(i,label,call(i,"original",argument),argument); }
            if(!renamed) { snprintf(label,sizeof(label),"%s_%s_RENAMED",names[p],value_names[v]); row(i,label,call(i,"renamed",argument),argument); }
            release(i,argument);
        }
        destroy(i);
    }
    Interp *i=fresh(); if(!i)return 2; Obj *argument=string(i,"VALUE",5); hold(argument);
    if(eval(i,"proc original {longargument} {info locals}; proc renamed {a} {info locals}"))return 3;
    row(i,"LOCALS_ORIGINAL",call(i,"original",argument),argument); row(i,"LOCALS_RENAMED",call(i,"renamed",argument),argument);
    row(i,"ARGS_ORIGINAL",eval(i,"info args original"),NULL); row(i,"ARGS_RENAMED",eval(i,"info args renamed"),NULL);
    if(eval(i,"proc observe {name index op} {set ::seen $name}"))return 3;
#ifdef JIM_PROBE
    row(i,"TRACE_AVAILABILITY",eval(i,"info commands trace"),NULL);
#else
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
    const char *original="trace variable longargument r observe;return $longargument";
    const char *renamed="trace variable a r observe;return $a";
#else
    const char *original="trace add variable longargument read observe;return $longargument";
    const char *renamed="trace add variable a read observe;return $a";
#endif
    if(define(i,"original","longargument",12,original,(int)strlen(original)))return 3;
    if(define(i,"renamed","a",1,renamed,(int)strlen(renamed)))return 3;
    row(i,"TRACE_RETURN_ORIGINAL",call(i,"original",argument),argument); row(i,"TRACE_NAME_ORIGINAL",eval(i,"set ::seen"),NULL);
    row(i,"TRACE_RETURN_RENAMED",call(i,"renamed",argument),argument); row(i,"TRACE_NAME_RENAMED",eval(i,"set ::seen"),NULL);
#endif
    release(i,argument); destroy(i);
#ifndef JIM_PROBE
    Tcl_Finalize();
#endif
    return 0;
}
