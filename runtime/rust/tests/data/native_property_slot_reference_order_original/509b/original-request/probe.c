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
#if TCL_MAJOR_VERSION >= 9 && TCL_MINOR_VERSION >= 1
#define PROBE_HAVE_PROPERTY 1
#endif
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
#ifdef PROBE_HAVE_PROPERTY
typedef struct State State;
typedef struct Member { State *state; int role; const char *payload; } Member;
struct State {
    Interp *interp; int case_id,sequence,updates[4],frees[4],duplicates[4];
    int class_scope,writable,hold_cache,old_caller_owned;
    const char *phase; Obj *held[4],*cache; Member members[4];
};
static void row(State *s,const char *stage,int code) {
    printf("ROW\tcase=%d\tphase=%s\tstage=%s\tstatus=%d\tsequence=%d\told_caller_owned=%d\tcache_refs=%lld\tcache_string=%d",s->case_id,s->phase,stage,code,s->sequence,s->old_caller_owned,s->cache?(long long)s->cache->refCount:-1,s->cache&&s->cache->bytes!=NULL);
    for(int k=0;k<4;k++) {
        Obj *o=s->held[k];
        printf("\tm%d_update=%d\tm%d_free=%d\tm%d_duplicate=%d\tm%d_refs=%lld\tm%d_string=%d",k,s->updates[k],k,s->frees[k],k,s->duplicates[k],k,o?(long long)o->refCount:-1,k,o&&o->bytes!=NULL);
    }
    putchar('\n');fflush(stdout);
}
static void free_rep(Obj *o) {
    Member *m=o->internalRep.twoPtrValue.ptr1;State *s=m->state;s->frees[m->role]++;
    printf("EVENT\tcase=%d\tphase=%s\tsequence=%d\tkind=free\trole=%d\trefs=%lld\n",s->case_id,s->phase,++s->sequence,m->role,(long long)o->refCount);fflush(stdout);
    if(s->held[m->role]==o && o->refCount<=0)s->held[m->role]=NULL;
    row(s,"inside-member-free",TCL_OK);
}
static void duplicate_rep(Obj *from,Obj *to);
static void update_rep(Obj *o) {
    Member *m=o->internalRep.twoPtrValue.ptr1;State *s=m->state;s->updates[m->role]++;
    printf("EVENT\tcase=%d\tphase=%s\tsequence=%d\tkind=update\trole=%d\n",s->case_id,s->phase,++s->sequence,m->role);fflush(stdout);
    row(s,"inside-member-update-before-string",TCL_OK);
    size_t n=strlen(m->payload);o->bytes=Tcl_Alloc((unsigned)n+1);memcpy(o->bytes,m->payload,n+1);o->length=(Count)n;
}
static Tcl_ObjType member_type={"originalPropertyMemberOrder",free_rep,duplicate_rep,update_rep,NULL};
static void duplicate_rep(Obj *from,Obj *to) {
    Member *m=from->internalRep.twoPtrValue.ptr1;State *s=m->state;s->duplicates[m->role]++;
    printf("EVENT\tcase=%d\tphase=%s\tsequence=%d\tkind=duplicate\trole=%d\n",s->case_id,s->phase,++s->sequence,m->role);fflush(stdout);
    to->internalRep=from->internalRep;to->typePtr=&member_type;
}
static Obj *word(const char *value) { Obj *o=Tcl_NewStringObj(value,-1);Tcl_IncrRefCount(o);return o; }
static Obj *member(State *s,int role,const char *payload) {
    Obj *o=Tcl_NewObj();Tcl_IncrRefCount(o);Tcl_InvalidateStringRep(o);
    Member *m=&s->members[role];m->state=s;m->role=role;m->payload=payload;
    o->typePtr=&member_type;o->internalRep.twoPtrValue.ptr1=m;o->internalRep.twoPtrValue.ptr2=NULL;s->held[role]=o;return o;
}
static const char *slot(State *s) {
    if(s->class_scope)return s->writable?"::oo::configuresupport::writableproperties":"::oo::configuresupport::readableproperties";
    return "::oo::configuresupport::objreadableproperties";
}
static int setter(void *opaque,Interp *i,Count objc,Obj *const *objv) {
    State *s=opaque;(void)objv;if(objc!=1)return TCL_ERROR;
    Obj *arguments[5]={word(slot(s)),word("-set"),NULL,NULL,NULL};
    Count count;
    if(!strcmp(s->phase,"initial")) { arguments[2]=s->held[0];count=3; }
    else { arguments[2]=s->held[1];arguments[3]=s->held[2];arguments[4]=s->held[3];count=5; }
    row(s,"before-public-slot-set",TCL_OK);
    int code=Tcl_EvalObjv(i,count,arguments,0);row(s,"after-public-slot-set",code);
    Tcl_DecrRefCount(arguments[0]);Tcl_DecrRefCount(arguments[1]);return code;
}
static int invoke_definition(State *s) {
    Obj *arguments[3]={word(s->class_scope?"::oo::define":"::oo::objdefine"),word(s->class_scope?"::PropertyClass":"::PropertyObject"),word("::originalPropertySet")};
    int code=Tcl_EvalObjv(s->interp,3,arguments,0);
    for(int k=0;k<3;k++)Tcl_DecrRefCount(arguments[k]);return code;
}
static int invoke_info(State *s) {
    Obj *arguments[6]={word("info"),word(s->class_scope?"class":"object"),word("properties"),word(s->class_scope?"::PropertyClass":"::PropertyObject"),word("-all"),word(s->writable?"-writable":"-readable")};
    int code=Tcl_EvalObjv(s->interp,6,arguments,0);
    for(int k=0;k<6;k++)Tcl_DecrRefCount(arguments[k]);return code;
}
static void raw_list(State *s,const char *stage) {
    Obj *list=result(s->interp);Count count=0;Obj **elements=NULL;
    int code=Tcl_ListObjGetElements(NULL,list,&count,&elements);
    printf("LIST\tcase=%d\tphase=%s\tstage=%s\tstatus=%d\tcount=%lld\theader_refs=%lld\theader_string=%d",s->case_id,s->phase,stage,code,(long long)count,(long long)list->refCount,list->bytes!=NULL);
    if(code==TCL_OK)for(Count k=0;k<count;k++) {
        int role=-1;for(int j=0;j<4;j++)if(elements[k]==s->held[j])role=j;
        printf("\tmember%lld_role=%d\tmember%lld_refs=%lld",(long long)k,role,(long long)k,(long long)elements[k]->refCount);
    }
    putchar('\n');fflush(stdout);
}
static void presentation(State *s,const char *stage,int code) {
    printf("RESULT\tcase=%d\tphase=%s\tstage=%s\tstatus=%d\thex=",s->case_id,s->phase,stage,code);hex_result(s->interp);putchar('\n');fflush(stdout);
}
static int one_case(int id) {
    State s;memset(&s,0,sizeof(s));s.case_id=id;s.class_scope=id<2||id>=4;s.writable=id>=4;s.hold_cache=id%2;s.phase="setup";
    s.interp=fresh();if(!s.interp)return 10;
    printf("CASE\tcase=%d\tclass=%d\twritable=%d\thold_cache=%d\tentry=public-slot-minus-set-in-original-definition-context\n",id,s.class_scope,s.writable,s.hold_cache);
    int code=eval_file(s.interp,"setup.tcl");row(&s,"after-setup",code);
    if(code!=TCL_OK) { presentation(&s,"setup",code);destroy(s.interp);return 11; }
    Tcl_CreateObjCommand2(s.interp,"::originalPropertySet",setter,&s,NULL);
    member(&s,0,"old");member(&s,1,"p");member(&s,2,"p");member(&s,3,"q");s.old_caller_owned=1;
    s.phase="initial";code=invoke_definition(&s);row(&s,"after-initial-definition",code);presentation(&s,"initial-definition",code);
    Tcl_ResetResult(s.interp);row(&s,"after-initial-result-reset",TCL_OK);
    s.phase="cache-prime";code=invoke_info(&s);row(&s,"after-info-all-before-presentation",code);raw_list(&s,"initial-info-all");
    if(code!=TCL_OK) { presentation(&s,"initial-info-all",code);destroy(s.interp);return 12; }
    if(s.hold_cache) { s.cache=result(s.interp);Tcl_IncrRefCount(s.cache); }
    Tcl_ResetResult(s.interp);row(&s,"after-result-reset-cache-selection",TCL_OK);
    Obj *old=s.held[0];s.old_caller_owned=0;Tcl_DecrRefCount(old);row(&s,"after-old-caller-reference-release",TCL_OK);
    s.phase="worker";code=invoke_definition(&s);row(&s,"after-replacement-definition",code);presentation(&s,"replacement-definition",code);
    Tcl_ResetResult(s.interp);row(&s,"after-replacement-result-reset",TCL_OK);
    s.phase="cache-release";Obj *cache=s.cache;s.cache=NULL;if(cache)Tcl_DecrRefCount(cache);row(&s,"after-held-old-cache-list-release",TCL_OK);
    s.phase="post-info";code=invoke_info(&s);row(&s,"after-new-info-before-presentation",code);raw_list(&s,"new-info-all");
    s.phase="presentation";presentation(&s,"new-info-all",code);row(&s,"after-new-info-presentation",TCL_OK);
    s.phase="interp-delete";destroy(s.interp);s.interp=NULL;row(&s,"after-interpreter-destroy",TCL_OK);
    s.phase="held-release";for(int k=1;k<4;k++) { Obj *o=s.held[k];s.held[k]=NULL;if(o)Tcl_DecrRefCount(o);row(&s,k==1?"after-first-p-release":k==2?"after-duplicate-p-release":"after-q-release",TCL_OK); }
    return 0;
}
#endif
int main(int argc,char **argv) {
    if(argc!=2)return 2;source_root=argv[1];
#ifndef JIM_PROBE
    Tcl_FindExecutable(argv[0]);
#endif
    int code=metadata();if(code)return code;
#ifdef PROBE_HAVE_PROPERTY
    for(int k=0;k<6;k++) { code=one_case(k);if(code)return code; }
#else
    printf("UNAVAILABLE\tworker=C91-original-property-storage\treason=not-in-this-initialized-core-SDK\n");
#endif
    return 0;
}
