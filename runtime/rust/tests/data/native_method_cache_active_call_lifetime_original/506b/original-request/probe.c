/* Original public TclOO MethodType, idle cache and active-call lifetimes. */
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
typedef struct MethodData {
    State *state; int role,owns_token; Obj *token;
} MethodData;
typedef struct TokenData { State *state; int role; } TokenData;
struct State {
    Interp *interp; int case_id,sequence,active_action;
    int method_calls[2],method_deletes[2],token_updates[2],token_frees[2],token_duplicates[2];
    Obj *tokens[2],*selector,*duplicate,*object_name,*method_name;
    MethodData data[2]; TokenData token_data[2]; Tcl_Class cls;
};
static void row(State *s,const char *stage,int code) {
    printf("ROW\tcase=%d\tstage=%s\tstatus=%d\tsequence=%d",s->case_id,stage,code,s->sequence);
    for(int k=0;k<2;k++) {
        Obj *o=s->tokens[k];
        printf("\td%d_calls=%d\td%d_deletes=%d\td%d_owns_token=%d\tt%d_update=%d\tt%d_free=%d\tt%d_duplicate=%d\tt%d_refs=%lld\tt%d_string=%d",
            k,s->method_calls[k],k,s->method_deletes[k],k,s->data[k].owns_token,
            k,s->token_updates[k],k,s->token_frees[k],k,s->token_duplicates[k],
            k,o?(long long)o->refCount:-1,k,o&&o->bytes!=NULL);
    }
    printf("\tselector_refs=%lld\tselector_type=%s\tduplicate_refs=%lld\tduplicate_type=%s\n",
        s->selector?(long long)s->selector->refCount:-1,
        s->selector&&s->selector->typePtr?s->selector->typePtr->name:"NONE",
        s->duplicate?(long long)s->duplicate->refCount:-1,
        s->duplicate&&s->duplicate->typePtr?s->duplicate->typePtr->name:"NONE");
    fflush(stdout);
}
static void token_free(Obj *o) {
    TokenData *r=o->internalRep.twoPtrValue.ptr1;
    r->state->token_frees[r->role]++;
    printf("EVENT\tcase=%d\tsequence=%d\tkind=token-free\trole=%d\n",r->state->case_id,++r->state->sequence,r->role); fflush(stdout);
}
static void token_duplicate(Obj *from,Obj *to);
static void token_update(Obj *o) {
    TokenData *r=o->internalRep.twoPtrValue.ptr1; const char *value=r->role?"TOKEN1":"TOKEN0";
    r->state->token_updates[r->role]++;
    printf("EVENT\tcase=%d\tsequence=%d\tkind=token-update\trole=%d\n",r->state->case_id,++r->state->sequence,r->role); fflush(stdout);
    o->bytes=Tcl_Alloc(7); memcpy(o->bytes,value,7); o->length=6;
}
static Tcl_ObjType token_type={"originalMethodClientToken",token_free,token_duplicate,token_update,NULL};
static void token_duplicate(Obj *from,Obj *to) {
    TokenData *r=from->internalRep.twoPtrValue.ptr1;
    r->state->token_duplicates[r->role]++;
    printf("EVENT\tcase=%d\tsequence=%d\tkind=token-duplicate\trole=%d\n",r->state->case_id,++r->state->sequence,r->role); fflush(stdout);
    to->internalRep=from->internalRep; to->typePtr=&token_type;
}
static Obj *held_word(const char *value) { Obj *o=Tcl_NewStringObj(value,-1); Tcl_IncrRefCount(o); return o; }
static void held_token(State *s,int role) {
    Obj *o=Tcl_NewObj(); Tcl_IncrRefCount(o); Tcl_InvalidateStringRep(o);
    TokenData *r=&s->token_data[role]; r->state=s;r->role=role;
    o->typePtr=&token_type;o->internalRep.twoPtrValue.ptr1=r;o->internalRep.twoPtrValue.ptr2=NULL;
    s->tokens[role]=o;s->data[role].state=s;s->data[role].role=role;s->data[role].token=o;
}
static void method_delete(void *opaque) {
    MethodData *d=opaque; State *s=d->state;
    s->method_deletes[d->role]++;
    printf("EVENT\tcase=%d\tsequence=%d\tkind=method-delete\trole=%d\ttoken_refs_before=%lld\n",s->case_id,++s->sequence,d->role,(long long)d->token->refCount); fflush(stdout);
    if(d->owns_token) { d->owns_token=0; Tcl_DecrRefCount(d->token); }
    row(s,"inside-method-delete-after-owned-token-release",TCL_OK);
}
static int invoke_original(State *s) {
    Obj *argv[2]={s->object_name,s->selector};
    return Tcl_EvalObjv(s->interp,2,argv,0);
}
static Tcl_Method install(State *s,int role);
static int method_call(void *opaque,Interp *i,Tcl_ObjectContext context,Count objc,Obj *const *objv) {
    MethodData *d=opaque;State *s=d->state;(void)context;(void)objc;(void)objv;
    s->method_calls[d->role]++;
    row(s,"inside-method-before-action",TCL_OK);
    if(d->role==0 && s->active_action) {
        s->active_action=0;
        if(s->case_id==1) {
            int code=eval_file(i,"delete-method.tcl");row(s,"inside-method-after-delete",code);
            if(code!=TCL_OK)return code;
        } else if(s->case_id==4) {
            Tcl_Method replacement=install(s,1);
            printf("METHOD\tcase=%d\tstage=active-replacement\trole=1\tavailable=%d\taddress=%p\n",s->case_id,replacement!=NULL,(void *)replacement);
            row(s,"inside-method-after-replacement",replacement?TCL_OK:TCL_ERROR);
            if(!replacement)return TCL_ERROR;
            int code=invoke_original(s);row(s,"inside-method-after-nested-new-invocation",code);
            if(code!=TCL_OK)return code;
        }
    }
    Tcl_SetObjResult(i,Tcl_NewStringObj(d->role?"DATA1":"DATA0",-1));
    row(s,"inside-method-before-return",TCL_OK);return TCL_OK;
}
#if TCL_MAJOR_VERSION >= 9
static Tcl_MethodType2 method_type={TCL_OO_METHOD_VERSION_2,"originalMethodLifetime",method_call,method_delete,NULL};
#else
static Tcl_MethodType method_type={TCL_OO_METHOD_VERSION_CURRENT,"originalMethodLifetime",method_call,method_delete,NULL};
#endif
static Tcl_Method install(State *s,int role) {
    MethodData *d=&s->data[role];Tcl_IncrRefCount(d->token);d->owns_token=1;
#if TCL_MAJOR_VERSION >= 9
    Tcl_Method m=Tcl_NewMethod2(s->interp,s->cls,s->method_name,TCL_OO_METHOD_PUBLIC,&method_type,d);
#else
    Tcl_Method m=Tcl_NewMethod(s->interp,s->cls,s->method_name,1,&method_type,d);
#endif
    if(!m) { d->owns_token=0;Tcl_DecrRefCount(d->token); }
    return m;
}
static int observer(void *opaque,Interp *i,Count objc,Obj *const *objv) {
    State *s=opaque; (void)i;
    if(objc!=2)return TCL_ERROR;
    Count length;const char *stage=bytes(objv[1],&length);
    printf("CALL\tcase=%d\tkind=script-observer\tstage=%.*s\n",s->case_id,(int)length,stage);
    row(s,stage,TCL_OK);return TCL_OK;
}
static void presentation(State *s,const char *stage,int code) {
    printf("RESULT\tcase=%d\tstage=%s\tstatus=%d\thex=",s->case_id,stage,code);hex_result(s->interp);putchar('\n');fflush(stdout);
}
static void release_selector(State *s,int duplicate) {
    Obj **slot=duplicate?&s->duplicate:&s->selector;Obj *o=*slot;*slot=NULL;
    if(o)Tcl_DecrRefCount(o);
    row(s,duplicate?"after-held-duplicate-release":"after-held-selector-release",TCL_OK);
}
static int one_case(int id) {
    State s;memset(&s,0,sizeof(s));s.case_id=id;s.interp=fresh();if(!s.interp)return 10;
    printf("CASE\tcase=%d\tentry=public-object-vector\n",id);
    int code=eval_file(s.interp,"setup.tcl");row(&s,"after-setup",code);
    if(code!=TCL_OK) { presentation(&s,"setup",code);destroy(s.interp);return 11; }
    s.object_name=held_word("::MethodObject");s.method_name=held_word("m");s.selector=held_word("m");
    held_token(&s,0);held_token(&s,1);
    Obj *class_name=held_word("::MethodBase");Tcl_Object object=Tcl_GetObjectFromObj(s.interp,class_name);
    s.cls=object?Tcl_GetObjectAsClass(object):NULL;Tcl_DecrRefCount(class_name);
    if(!s.cls) { destroy(s.interp);return 12; }
#if TCL_MAJOR_VERSION >= 9
    Tcl_CreateObjCommand2(s.interp,"::methodObserve",observer,&s,NULL);
    printf("API\tcase=%d\tmethod=Tcl_NewMethod2\tmethodTypeVersion=%d\tcountBytes=%zu\n",id,TCL_OO_METHOD_VERSION_2,sizeof(Count));
#else
    Tcl_CreateObjCommand(s.interp,"::methodObserve",observer,&s,NULL);
    printf("API\tcase=%d\tmethod=Tcl_NewMethod\tmethodTypeVersion=%d\tcountBytes=%zu\n",id,TCL_OO_METHOD_VERSION_CURRENT,sizeof(Count));
#endif
    Tcl_Method method=install(&s,0);
    printf("METHOD\tcase=%d\tstage=initial\trole=0\tavailable=%d\taddress=%p\n",id,method!=NULL,(void *)method);
    row(&s,"after-initial-install",method?TCL_OK:TCL_ERROR);
    if(!method) { destroy(s.interp);return 13; }
    code=invoke_original(&s);row(&s,"after-prime-invocation",code);presentation(&s,"prime-invocation",code);
    s.duplicate=Tcl_DuplicateObj(s.selector);Tcl_IncrRefCount(s.duplicate);row(&s,"after-selector-duplicate",TCL_OK);
    if(id==0) {
        code=eval_file(s.interp,"delete-method.tcl");row(&s,"after-idle-delete",code);presentation(&s,"idle-delete",code);
        release_selector(&s,1);release_selector(&s,0);
    } else if(id==1 || id==4) {
        s.active_action=1;code=invoke_original(&s);row(&s,"after-active-invocation",code);presentation(&s,"active-invocation",code);
    } else if(id==2) {
        code=eval_file(s.interp,"active-next.tcl");row(&s,"after-derived-script-install",code);
        code=invoke_original(&s);row(&s,"after-active-next-invocation",code);presentation(&s,"active-next-invocation",code);
    } else if(id==3) {
        method=install(&s,1);row(&s,"after-idle-replacement",method?TCL_OK:TCL_ERROR);
        printf("METHOD\tcase=%d\tstage=idle-replacement\trole=1\tavailable=%d\taddress=%p\n",id,method!=NULL,(void *)method);
        code=invoke_original(&s);row(&s,"after-new-invocation",code);presentation(&s,"new-invocation",code);
    } else if(id==5) {
        release_selector(&s,1);release_selector(&s,0);
        code=eval_file(s.interp,"delete-method.tcl");row(&s,"after-delete-without-held-cache",code);
    } else if(id==6) {
        code=eval_file(s.interp,"destroy-class.tcl");row(&s,"after-class-destroy-with-held-cache",code);presentation(&s,"class-destroy",code);
    }
    row(&s,"before-interpreter-destroy",TCL_OK);destroy(s.interp);s.interp=NULL;
    row(&s,"after-interpreter-destroy",TCL_OK);
    release_selector(&s,1);release_selector(&s,0);
    Tcl_DecrRefCount(s.method_name);Tcl_DecrRefCount(s.object_name);s.method_name=NULL;s.object_name=NULL;
    for(int k=0;k<2;k++) {
        Obj *token=s.tokens[k];row(&s,k?"before-token1-caller-release":"before-token0-caller-release",TCL_OK);
        s.tokens[k]=NULL;Tcl_DecrRefCount(token);row(&s,k?"after-token1-caller-release":"after-token0-caller-release",TCL_OK);
    }
    return 0;
}
#endif
int main(int argc,char **argv) {
    if(argc!=2)return 2;source_root=argv[1];
#ifndef JIM_PROBE
    Tcl_FindExecutable(argv[0]);
#endif
    int code=metadata();if(code)return code;
#ifdef PROBE_HAVE_OO
    for(int k=0;k<8;k++) { code=one_case(k);if(code)return code; }
#else
    printf("UNAVAILABLE\tworker=public-TclOO-MethodType\treason=not-in-this-initialized-core-SDK\n");
#endif
    return 0;
}
