/* Original TclOO rename operand, exact declaration and callback stages. */
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
typedef int Count;
static Interp *fresh(void) {
    Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i);
    if(Jim_InitStaticExtensions(i)!=JIM_OK) { Jim_FreeInterp(i); return NULL; }
    return i;
}
static void destroy(Interp *i) { Jim_FreeInterp(i); }
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static const char *bytes(Obj *o,Count *n) { return Jim_GetString(o,n); }
static int eval_source(Interp *i,const char *s,int n) {
    Obj *o=Jim_NewStringObj(i,s,n); Jim_IncrRefCount(o);
    int rc=Jim_EvalObj(i,o); Jim_DecrRefCount(i,o); return rc;
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
#if TCL_MAJOR_VERSION > 8 || (TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION >= 6)
#include "tclOO.h"
#define PROBE_HAVE_OO 1
#endif
static Interp *fresh(void) {
    Interp *i=Tcl_CreateInterp();
    if(Tcl_Init(i)!=TCL_OK) { Tcl_DeleteInterp(i); return NULL; }
    return i;
}
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj *o,Count *n) { return Tcl_GetStringFromObj(o,n); }
static int eval_source(Interp *i,const char *s,int n) {
    Obj *o=Tcl_NewStringObj(s,n); Tcl_IncrRefCount(o);
    int rc=Tcl_EvalObjEx(i,o,0); Tcl_DecrRefCount(o); return rc;
}
#endif
static const char *source_root;
static int eval_file(Interp *i,const char *name) {
    char path[4096],source[4096]; size_t n;
    int size=snprintf(path,sizeof(path),"%s/%s",source_root,name);
    if(size<0 || size>=(int)sizeof(path)) return -701;
    FILE *f=fopen(path,"rb"); if(!f)return -702;
    n=fread(source,1,sizeof(source),f);
    int failed=ferror(f)||!feof(f); int closed=fclose(f);
    if(failed || closed)return -703;
    return eval_source(i,source,(int)n);
}
static void hex_result(Interp *i) {
    Count n=0; const unsigned char *p=(const unsigned char *)bytes(result(i),&n);
    for(Count j=0;j<n;j++)printf("%02x",p[j]);
}
static int metadata(void) {
    Interp *i=fresh(); if(!i)return 3;
#ifdef JIM_PROBE
    int code=eval_file(i,"jim-version-probe.tcl");
    printf("VERSION\tstatus=%d\thex=",code); hex_result(i); putchar('\n');
    printf("BUILD\tJIM_GITVERSION=%s\tJIM_VERSION=%d\tJIM_ABI_VERSION=%d\tJIM_UTF8=%d\n",JIM_GITVERSION,JIM_VERSION,JIM_ABI_VERSION,JIM_UTF8);
#else
    int code=eval_file(i,"version-probe.tcl");
    printf("VERSION\tstatus=%d\thex=",code); hex_result(i); putchar('\n');
    int a,b,c,d; Tcl_GetVersion(&a,&b,&c,&d);
    printf("BUILD\tHEADER=%s\tGetVersion=%d.%d.%d/%d\n",TCL_PATCH_LEVEL,a,b,c,d);
#endif
    printf("ABI\tint=%zu\tlong=%zu\tptr=%zu\tCHAR_BIT=%d\n",sizeof(int),sizeof(long),sizeof(void *),CHAR_BIT);
    code=eval_file(i,"availability.tcl");
    printf("AVAILABILITY\tstatus=%d\thex=",code); hex_result(i); putchar('\n');
    destroy(i); return code;
}
#ifdef PROBE_HAVE_OO
typedef struct State State;
typedef struct NameData {State *state;int role;const char *payload;Count length;} NameData;
typedef struct MethodData {State *state;int role,owns_token;Obj *token;} MethodData;
struct State {
    Interp *interp;int case_id,scenario,object_scope,channel,sequence;
    const char *phase;Obj *held[4];NameData names[4];MethodData data[2];
    int updates[4],frees[4],duplicates[4],calls[2],deletes[2];
    Tcl_Class cls;Tcl_Object object;
};
static void row(State *s,const char *stage,int code) {
    printf("ROW\tcase=%d\tphase=%s\tstage=%s\tstatus=%d\tsequence=%d",s->case_id,s->phase,stage,code,s->sequence);
    for(int k=0;k<4;k++) {
        Obj *o=s->held[k];
        printf("\tn%d_updates=%d\tn%d_frees=%d\tn%d_duplicates=%d\tn%d_refs=%lld\tn%d_string=%d",k,s->updates[k],k,s->frees[k],k,s->duplicates[k],k,o?(long long)o->refCount:-1,k,o&&o->bytes!=NULL);
    }
    for(int k=0;k<2;k++)printf("\td%d_calls=%d\td%d_deletes=%d\td%d_owns_token=%d",k,s->calls[k],k,s->deletes[k],k,s->data[k].owns_token);
    putchar('\n');fflush(stdout);
}
static void name_free(Obj *o) {
    NameData *d=o->internalRep.twoPtrValue.ptr1;State *s=d->state;s->frees[d->role]++;
    printf("EVENT\tcase=%d\tphase=%s\tsequence=%d\tkind=name-free\trole=%d\trefs=%lld\n",s->case_id,s->phase,++s->sequence,d->role,(long long)o->refCount);fflush(stdout);
}
static void name_duplicate(Obj *source,Obj *copy);
static void name_update(Obj *o) {
    NameData *d=o->internalRep.twoPtrValue.ptr1;State *s=d->state;s->updates[d->role]++;
    printf("EVENT\tcase=%d\tphase=%s\tsequence=%d\tkind=name-update\trole=%d\n",s->case_id,s->phase,++s->sequence,d->role);fflush(stdout);
    row(s,"inside-name-update-before-string",TCL_OK);
    o->bytes=Tcl_Alloc((unsigned)d->length+1);memcpy(o->bytes,d->payload,(size_t)d->length);o->bytes[d->length]=0;o->length=d->length;
}
static Tcl_ObjType name_type={"originalRenameName",name_free,name_duplicate,name_update,NULL};
static void name_duplicate(Obj *source,Obj *copy) {
    NameData *d=source->internalRep.twoPtrValue.ptr1;d->state->duplicates[d->role]++;
    printf("EVENT\tcase=%d\tphase=%s\tsequence=%d\tkind=name-duplicate\trole=%d\n",d->state->case_id,d->state->phase,++d->state->sequence,d->role);fflush(stdout);
    copy->internalRep=source->internalRep;copy->typePtr=&name_type;
}
static Obj *word(const char *value,Count length) {Obj *o=Tcl_NewStringObj(value,length);Tcl_IncrRefCount(o);return o;}
static Obj *original(State *s,int role,const char *value,Count length) {
    Obj *o=Tcl_NewObj();Tcl_IncrRefCount(o);Tcl_InvalidateStringRep(o);
    NameData *d=&s->names[role];d->state=s;d->role=role;d->payload=value;d->length=length;
    o->typePtr=&name_type;o->internalRep.twoPtrValue.ptr1=d;o->internalRep.twoPtrValue.ptr2=NULL;s->held[role]=o;return o;
}
static void method_delete(void *opaque) {
    MethodData *d=opaque;State *s=d->state;s->deletes[d->role]++;
    printf("EVENT\tcase=%d\tphase=%s\tsequence=%d\tkind=method-delete\trole=%d\n",s->case_id,s->phase,++s->sequence,d->role);fflush(stdout);
    if(d->owns_token){d->owns_token=0;Tcl_DecrRefCount(d->token);}row(s,"inside-method-delete-after-token-release",TCL_OK);
}
static int method_call(void *opaque,Interp *i,Tcl_ObjectContext context,Count objc,Obj *const *objv) {
    MethodData *d=opaque;State *s=d->state;(void)context;(void)objc;(void)objv;s->calls[d->role]++;
    row(s,"inside-selected-method-call",TCL_OK);Tcl_SetObjResult(i,Tcl_NewStringObj(d->role?"DEST":"FROM",-1));return TCL_OK;
}
#if TCL_MAJOR_VERSION >= 9
static Tcl_MethodType2 method_type={TCL_OO_METHOD_VERSION_2,"originalRenameMethod",method_call,method_delete,NULL};
#else
static Tcl_MethodType method_type={TCL_OO_METHOD_VERSION_CURRENT,"originalRenameMethod",method_call,method_delete,NULL};
#endif
static int install(State *s,int role,const char *name,Count length) {
    Obj *n=word(name,length);MethodData *d=&s->data[role];d->state=s;d->role=role;d->token=s->held[role+2];Tcl_IncrRefCount(d->token);d->owns_token=1;
#if TCL_MAJOR_VERSION >= 9
    Tcl_Method m=s->object_scope?Tcl_NewInstanceMethod2(s->interp,s->object,n,TCL_OO_METHOD_PUBLIC,&method_type,d):Tcl_NewMethod2(s->interp,s->cls,n,TCL_OO_METHOD_PUBLIC,&method_type,d);
#else
    Tcl_Method m=s->object_scope?Tcl_NewInstanceMethod(s->interp,s->object,n,1,&method_type,d):Tcl_NewMethod(s->interp,s->cls,n,1,&method_type,d);
#endif
    printf("METHOD\tcase=%d\tphase=%s\trole=%d\tavailable=%d\taddress=%p\n",s->case_id,s->phase,role,m!=NULL,(void *)m);fflush(stdout);
    Tcl_DecrRefCount(n);if(!m){d->owns_token=0;Tcl_DecrRefCount(d->token);}return m?TCL_OK:TCL_ERROR;
}
static void print_hex(Obj *value) {Count length;const unsigned char *p=(const unsigned char *)bytes(value,&length);for(Count k=0;k<length;k++)printf("%02x",p[k]);}
static void presentation(State *s,const char *stage,int code) {
    printf("RESULT\tcase=%d\tphase=%s\tstage=%s\tstatus=%d\thex=",s->case_id,s->phase,stage,code);hex_result(s->interp);putchar('\n');
    Obj *options=Tcl_GetReturnOptions(s->interp,code);Tcl_IncrRefCount(options);Obj *key=word("-errorcode",-1),*stamp=NULL;
    int rc=Tcl_DictObjGet(NULL,options,key,&stamp);printf("ERRORCODE\tcase=%d\tphase=%s\tstage=%s\tlookup_status=%d\tpresent=%d\thex=",s->case_id,s->phase,stage,rc,stamp!=NULL);if(stamp)print_hex(stamp);putchar('\n');
    Tcl_DecrRefCount(key);Tcl_DecrRefCount(options);fflush(stdout);
}
static int rename_observer(void *opaque,Interp *i,Count objc,Obj *const *objv) {
    State *s=opaque;(void)objv;if(objc!=1)return TCL_ERROR;
    const char *worker=s->object_scope?"::oo::objdefine::renamemethod":"::oo::define::renamemethod";
    Obj *head=word(worker,-1);Obj *arguments[3]={head,s->held[0],s->held[1]};
    Tcl_SetObjResult(i,Tcl_NewStringObj("BEFORE",-1));Tcl_SetErrorCode(i,"SEEDED","CODE",NULL);
    s->phase=s->channel?"stock-worker":"public-vector";row(s,"before-rename-worker",TCL_OK);
    int code;
    if(s->channel) {
        Tcl_CmdInfo info;int found=Tcl_GetCommandInfo(i,worker,&info);
        printf("API\tcase=%d\tphase=%s\tworker=%s\tfound=%d\tisNativeObjectProc=%d\tcountBytes=%zu\n",s->case_id,s->phase,worker,found,found?info.isNativeObjectProc:-1,sizeof(Count));fflush(stdout);
        if(!found)code=-711;
#if TCL_MAJOR_VERSION >= 9
        else if(info.isNativeObjectProc!=2 || !info.objProc2)code=-712;
        else code=info.objProc2(info.objClientData2,i,3,arguments);
#else
        else if(info.isNativeObjectProc!=1 || !info.objProc)code=-712;
        else code=info.objProc(info.objClientData,i,3,arguments);
#endif
    } else code=Tcl_EvalObjv(i,3,arguments,0);
    row(s,"after-rename-worker-before-presentation",code);
    s->phase="worker-result-presentation";presentation(s,"inner-rename",code);row(s,"after-inner-presentation",code);
    Tcl_DecrRefCount(head);s->phase="outer-definition-return";return code;
}
static int invoke_definition(State *s) {
    Obj *arguments[3]={word(s->object_scope?"::oo::objdefine":"::oo::define",-1),word(s->object_scope?"::RenameObject":"::RenameClass",-1),word("::renameObserve",-1)};
    int code=Tcl_EvalObjv(s->interp,3,arguments,0);for(int k=0;k<3;k++)Tcl_DecrRefCount(arguments[k]);return code;
}
static void invoke_name(State *s,int role) {
    s->phase=role?"destination-invocation":"source-invocation";Obj *head=word("::RenameObject",-1);Obj *arguments[2]={head,s->held[role]};
    int code=Tcl_EvalObjv(s->interp,2,arguments,0);row(s,role?"after-destination-invocation":"after-source-invocation",code);
    s->phase="invocation-result-presentation";presentation(s,role?"destination-invocation":"source-invocation",code);Tcl_DecrRefCount(head);
}
static int one_case(int id) {
    State s;memset(&s,0,sizeof(s));s.case_id=id;s.scenario=id%8;s.object_scope=(id/8)%2;s.channel=id/16;s.phase="setup";s.interp=fresh();if(!s.interp)return 10;
    printf("CASE\tcase=%d\tscenario=%d\tobject_scope=%d\tchannel=%s\n",id,s.scenario,s.object_scope,s.channel?"public-CmdInfo-selected-stock-worker":"public-EvalObjv");
    int code=eval_file(s.interp,"setup.tcl");if(code!=TCL_OK){presentation(&s,"setup",code);destroy(s.interp);return 11;}
    Obj *name=word("::RenameObject",-1);s.object=Tcl_GetObjectFromObj(s.interp,name);Tcl_DecrRefCount(name);
    name=word("::RenameClass",-1);Tcl_Object c=Tcl_GetObjectFromObj(s.interp,name);s.cls=c?Tcl_GetObjectAsClass(c):NULL;Tcl_DecrRefCount(name);if(!s.object||!s.cls){destroy(s.interp);return 12;}
    const char *from="from",*to="moved",*declared="from",*occupied="occupied";Count fl=4,tl=5,dl=4,ol=8;
    switch(s.scenario){
    case 0:from="missing";fl=7;to="unused";tl=6;break;
    case 1:to="from";tl=4;break;
    case 2:to="occupied";tl=8;break;
    case 4:to="Upper";tl=5;break;
    case 5:from=declared="f\0x";fl=dl=3;to="t\0y";tl=3;break;
    case 6:to=occupied="o\0x";tl=ol=3;break;
    case 7:from=declared="::from";fl=dl=6;to="::moved";tl=7;break;
    }
    original(&s,0,from,fl);original(&s,1,to,tl);original(&s,2,"CLIENT0",7);original(&s,3,"CLIENT1",7);
    if(install(&s,0,declared,dl)!=TCL_OK || install(&s,1,occupied,ol)!=TCL_OK){destroy(s.interp);return 13;}
#if TCL_MAJOR_VERSION >= 9
    Tcl_CreateObjCommand2(s.interp,"::renameObserve",rename_observer,&s,NULL);
#else
    Tcl_CreateObjCommand(s.interp,"::renameObserve",rename_observer,&s,NULL);
#endif
    row(&s,"after-original-install",TCL_OK);code=invoke_definition(&s);row(&s,"after-outer-definition-return",code);
    s.phase="outer-result-presentation";presentation(&s,"outer-rename",code);
    invoke_name(&s,0);invoke_name(&s,1);
    s.phase="interpreter-destroy";row(&s,"before-interpreter-destroy",TCL_OK);destroy(s.interp);s.interp=NULL;row(&s,"after-interpreter-destroy",TCL_OK);
    s.phase="caller-release";for(int k=0;k<4;k++){Obj *o=s.held[k];s.held[k]=NULL;Tcl_DecrRefCount(o);row(&s,"after-original-caller-release",TCL_OK);}return 0;
}
#endif
int main(int argc,char **argv) {
    if(argc!=2)return 2;source_root=argv[1];
#ifndef JIM_PROBE
    Tcl_FindExecutable(argv[0]);
#endif
    int code=metadata();if(code)return code;
#ifdef PROBE_HAVE_OO
    for(int k=0;k<32;k++){code=one_case(k);if(code)return code;}
#else
    printf("UNAVAILABLE\tworker=original-TclOO-method-rename\treason=not-in-this-initialized-core-SDK\n");
#endif
    return 0;
}
