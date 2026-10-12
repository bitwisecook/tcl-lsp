/* Original object-vector alias observations. No String presentation in raw rows. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <limits.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
typedef int Count;
#define INC(o) Jim_IncrRefCount(o)
#define DEC(i,o) Jim_DecrRefCount((i),(o))
#define ALLOC(n) Jim_Alloc(n)
static Interp *fresh(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK){Jim_FreeInterp(i);return NULL;}return i; }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static const char *bytes(Obj *o,Count *n) { return Jim_GetString(o,n); }
static Obj *word(Interp *i,const char *p,int n) { Obj *o=Jim_NewStringObj(i,p,n);INC(o);return o; }
static int evalv(Interp *i,int n,Obj **v) { return Jim_EvalObjVector(i,n,v); }
static int source(Interp *i,const char *p) { return Jim_Eval(i,p); }
static void set_text(Interp *i,const char *p) { Jim_SetResultString(i,p,-1); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
#define INC(o) Tcl_IncrRefCount(o)
#define DEC(i,o) Tcl_DecrRefCount(o)
#define ALLOC(n) Tcl_Alloc(n)
static Interp *fresh(void) { Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK){Tcl_DeleteInterp(i);return NULL;}return i; }
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj *o,Count *n) { return Tcl_GetStringFromObj(o,n); }
static Obj *word(Interp *i,const char *p,int n) { (void)i;Obj *o=Tcl_NewStringObj(p,n);INC(o);return o; }
static int evalv(Interp *i,int n,Obj **v) { return Tcl_EvalObjv(i,n,v,0); }
static int source(Interp *i,const char *p) { return Tcl_EvalEx(i,p,-1,0); }
static void set_text(Interp *i,const char *p) { Tcl_SetObjResult(i,Tcl_NewStringObj(p,-1)); }
#endif

typedef struct State State;
typedef struct Record { State *state; int role; const unsigned char *payload; int length; } Record;
struct State { Interp *interp,*invocation; int id,sequence,calls; int updates[4],frees[4],duplicates[4]; Obj *held[4],*identity[4]; Record record[4]; };
static const unsigned char prefix_one[]={'a','l','p','h','a',' ',0,' ','x'};
static const unsigned char prefix_two[]={'b','e','t','a',' ','y'};
static void hex(const char *p,Count n) { for(Count k=0;k<n;k++)printf("%02x",(unsigned char)p[k]); }
static void row(State *s,const char *stage,int code) {
    printf("ROW|%d|%s|code=%d|sequence=%d|calls=%d",s->id,stage,code,s->sequence,s->calls);
    for(int k=0;k<4;k++) { Obj *o=s->held[k]; printf("|r%d_update=%d|r%d_free_rep=%d|r%d_dup=%d|r%d_held=%d|r%d_refs=%ld|r%d_string=%d|r%d_type=%s",k,s->updates[k],k,s->frees[k],k,s->duplicates[k],k,o!=NULL,k,o?(long)o->refCount:-1,k,o&&o->bytes!=NULL,k,o&&o->typePtr?o->typePtr->name:"NONE"); }
    puts("");fflush(stdout);
}
static void free_record(Record *r) { r->state->frees[r->role]++;printf("EVENT|%d|%d|free_rep|role=%d\n",r->state->id,++r->state->sequence,r->role);fflush(stdout); }
static void update_record(Obj *o,Record *r) {
    State *s=r->state;s->updates[r->role]++;
    printf("EVENT|%d|%d|update|role=%d|refs=%ld|prior_string=%d|payload=",s->id,++s->sequence,r->role,(long)o->refCount,o->bytes!=NULL);hex((const char *)r->payload,r->length);puts("");fflush(stdout);
    o->bytes=ALLOC((size_t)r->length+1);memcpy(o->bytes,r->payload,(size_t)r->length);o->bytes[r->length]=0;o->length=(Count)r->length;
}
#ifdef JIM_PROBE
static void free_rep(Interp *i,Obj *o) { (void)i;free_record(o->internalRep.ptr); }
static void update_rep(Obj *o) { update_record(o,o->internalRep.ptr); }
static void dup_rep(Interp *i,Obj *a,Obj *b) { (void)i;Record *r=a->internalRep.ptr;r->state->duplicates[r->role]++;b->internalRep=a->internalRep;b->typePtr=a->typePtr;printf("EVENT|%d|%d|duplicate|role=%d\n",r->state->id,++r->state->sequence,r->role); }
static const Jim_ObjType original_type={"originalAliasOperand",free_rep,dup_rep,update_rep,JIM_TYPE_NONE};
#else
static void free_rep(Obj *o) { free_record(o->internalRep.twoPtrValue.ptr1); }
static void update_rep(Obj *o) { update_record(o,o->internalRep.twoPtrValue.ptr1); }
static void dup_rep(Obj *a,Obj *b) { Record *r=a->internalRep.twoPtrValue.ptr1;r->state->duplicates[r->role]++;b->internalRep=a->internalRep;b->typePtr=a->typePtr;printf("EVENT|%d|%d|duplicate|role=%d\n",r->state->id,++r->state->sequence,r->role); }
static const Tcl_ObjType original_type={"originalAliasOperand",free_rep,dup_rep,update_rep,NULL};
#endif
static Obj *original(State *s,int role,const unsigned char *p,int n) {
    Record *r=&s->record[role];r->state=s;r->role=role;r->payload=p;r->length=n;
#ifdef JIM_PROBE
    Obj *o=Jim_NewObj(s->interp);o->bytes=NULL;o->length=0;o->internalRep.ptr=r;
#else
    Obj *o=Tcl_NewObj();Tcl_InvalidateStringRep(o);o->internalRep.twoPtrValue.ptr1=r;o->internalRep.twoPtrValue.ptr2=NULL;
#endif
    o->typePtr=&original_type;INC(o);s->held[role]=s->identity[role]=o;return o;
}
static void release(State *s,int role) { Obj *o=s->held[role];s->held[role]=NULL;if(o)DEC(s->interp,o); }
static int list(Interp *i,Obj *o,Count *n,Obj ***v) {
#ifdef JIM_PROBE
    *n=Jim_ListLength(i,o);*v=o->internalRep.listValue.ele;return JIM_OK;
#else
    return Tcl_ListObjGetElements(i,o,n,v);
#endif
}
static void list_row(State *s,const char *stage,Obj *o,Obj *other) {
    Count n=0;Obj **v=NULL;int rc=list(s->interp,o,&n,&v);
    printf("LIST|%d|%s|code=%d|header=%p|same_other=%d|string=%d|refs=%ld|count=%lld",s->id,stage,rc,(void *)o,o==other,o->bytes!=NULL,(long)o->refCount,(long long)n);
    if(rc==0)for(Count k=0;k<n;k++)printf("|m%lld_header=%p|m%lld_original=%d|m%lld_string=%d|m%lld_refs=%ld",(long long)k,(void *)v[k],(long long)k,k<3&&v[k]==s->identity[k+1],(long long)k,v[k]->bytes!=NULL,(long long)k,(long)v[k]->refCount);
    puts("");row(s,stage,rc);
}
#ifdef JIM_PROBE
static int capture(Interp *i,int argc,Obj *const *argv) {
    State *s=Jim_CmdPrivData(i);
#else
static int capture(ClientData data,Interp *i,int argc,Obj *const *argv) {
    State *s=data;
#endif
    s->calls++;printf("CALL|%d|argc=%d|head_original=%d|p1_original=%d|p2_original=%d\n",s->id,argc,argc>0&&argv[0]==s->identity[1],argc>1&&argv[1]==s->identity[2],argc>2&&argv[2]==s->identity[3]);row(s,"capture-before-getters",0);
    /* The reached command intentionally reads p2 before p1. */
    for(int k=2;k>=1;k--)if(k<argc){Count n=0;const char *p=bytes(argv[k],&n);printf("READ|%d|arg=%d|length=%lld|hex=",s->id,k,(long long)n);hex(p,n);puts("");}
    row(s,"capture-after-getters",0);set_text(i,"CAPTURED");return 0;
}
static int create_alias(State *s,int child) {
#ifdef JIM_PROBE
    (void)child;Obj *v[]={word(s->interp,"alias",-1),s->held[0],s->held[1],s->held[2],s->held[3]};int rc=evalv(s->interp,5,v);DEC(s->interp,v[0]);return rc;
#else
    Obj *v[]={word(s->interp,"interp",-1),word(s->interp,"alias",-1),word(s->interp,child?"child":"",-1),s->held[0],word(s->interp,"",-1),s->held[1],s->held[2],s->held[3]};int rc=evalv(s->interp,8,v);DEC(s->interp,v[0]);DEC(s->interp,v[1]);DEC(s->interp,v[2]);DEC(s->interp,v[4]);return rc;
#endif
}
static int query(State *s,int child,Obj **out) {
#ifdef JIM_PROBE
    (void)child;Obj *v[]={word(s->interp,"info",-1),word(s->interp,"alias",-1),word(s->interp,"aliasProbe",-1)};int rc=evalv(s->interp,3,v);for(int k=0;k<3;k++)DEC(s->interp,v[k]);
#else
    Obj *v[]={word(s->interp,"interp",-1),word(s->interp,"alias",-1),word(s->interp,child?"child":"",-1),word(s->interp,"aliasProbe",-1)};int rc=evalv(s->interp,4,v);for(int k=0;k<4;k++)DEC(s->interp,v[k]);
#endif
    *out=NULL;if(rc==0){*out=result(s->interp);INC(*out);}return rc;
}
static int remove_alias(State *s,int child) {
#ifdef JIM_PROBE
    (void)child;Obj *v[]={word(s->interp,"rename",-1),word(s->interp,"aliasProbe",-1),word(s->interp,"",-1)};int rc=evalv(s->interp,3,v);for(int k=0;k<3;k++)DEC(s->interp,v[k]);return rc;
#else
    Obj *v[]={word(s->interp,"interp",-1),word(s->interp,"alias",-1),word(s->interp,child?"child":"",-1),word(s->interp,"aliasProbe",-1),word(s->interp,"",-1)};int rc=evalv(s->interp,5,v);for(int k=0;k<5;k++)DEC(s->interp,v[k]);return rc;
#endif
}
static int invoke_alias(State *s) { Obj *v[]={word(s->invocation,"aliasProbe",-1),word(s->invocation,"tail",-1)};int rc=evalv(s->invocation,2,v);for(int k=0;k<2;k++)DEC(s->invocation,v[k]);return rc; }
static int metadata(void) {
    Interp *i=fresh();if(!i)return 3;
#ifdef JIM_PROBE
    int rc=source(i,"list [info version] [info commands interp] [info commands alias]");printf("BUILD|JimVersion=%d|Git=%s|ABI=%d\n",JIM_VERSION,JIM_GITVERSION,JIM_ABI_VERSION);
#else
    int rc=source(i,"list [info patchlevel] [info commands interp]");int a,b,c,d;Tcl_GetVersion(&a,&b,&c,&d);printf("BUILD|header=%s|linked=%d.%d.%d/%d\n",TCL_PATCH_LEVEL,a,b,c,d);
#endif
    Count n=0;const char *p=bytes(result(i),&n);printf("VERSION|code=%d|hex=",rc);hex(p,n);printf("|int=%zu|long=%zu|ptr=%zu|Count=%zu|CHAR_BIT=%d\n",sizeof(int),sizeof(long),sizeof(void *),sizeof(Count),CHAR_BIT);
#ifdef JIM_PROBE
    rc=source(i,"interp alias {} aliasProbe {} capture");p=bytes(result(i),&n);printf("API|C-interp-alias-spelling|code=%d|hex=",rc);hex(p,n);puts("");
    puts("API|Jim-child-pointer|unavailable=No-selected-public-Tcl_GetSlave-or-C-alias-query-recipe;core-alias-is-measured-independently");
#endif
    destroy(i);return 0;
}
static int case_run(int id) {
    State s;memset(&s,0,sizeof(s));s.id=id;s.interp=fresh();if(!s.interp)return 3;s.invocation=s.interp;
    int child=id==6;
#ifdef JIM_PROBE
    if(child){printf("UNAVAILABLE|6|cross-interp-original-header-query|reason=Jim-interp-is-child-command-API-with-String-result-crossing-not-C-interp-alias-query\n");destroy(s.interp);puts("CLOSED_CASE|6");return 0;}
    Jim_CreateCommand(s.interp,"capture",capture,&s,NULL);
#else
    Tcl_CreateObjCommand(s.interp,"capture",capture,&s,NULL);
    if(child){int rc=source(s.interp,"interp create child");row(&s,"child-create",rc);if(rc!=0){destroy(s.interp);puts("CLOSED_CASE|6");return 0;}s.invocation=Tcl_GetSlave(s.interp,"child");if(!s.invocation){destroy(s.interp);return 4;}}
#endif
    original(&s,0,(const unsigned char *)"aliasProbe",10);original(&s,1,(const unsigned char *)"capture",7);original(&s,2,prefix_one,sizeof(prefix_one));original(&s,3,prefix_two,sizeof(prefix_two));
    row(&s,"before-capture",0);int rc=create_alias(&s,child);row(&s,"after-capture",rc);
    Obj *q1=NULL,*q2=NULL;
    if(rc==0&&(id==1||id==2||id==4||id==6)){
        rc=query(&s,child,&q1);row(&s,"query-one-returned-unpresented",rc);if(q1)list_row(&s,"query-one-raw",q1,NULL);
        rc=query(&s,child,&q2);row(&s,"query-two-returned-unpresented",rc);if(q2)list_row(&s,"query-two-raw",q2,q1);
        if(id==2&&q1){Count n=0;const char *p=bytes(q1,&n);printf("PRESENT|2|length=%lld|hex=",(long long)n);hex(p,n);puts("");row(&s,"query-one-presented",0);if(q2)list_row(&s,"query-two-after-other-presentation",q2,q1);}
    }
    if(rc==0&&(id==1||id==2||id==3||id==6)){rc=invoke_alias(&s);row(&s,"first-invocation",rc);rc=invoke_alias(&s);row(&s,"second-invocation",rc);}
    /* Discard any incidental result owner before testing capture/query owners. */
    set_text(s.interp,"RESULT_CLEARED");if(s.invocation!=s.interp)set_text(s.invocation,"RESULT_CLEARED");
    for(int k=0;k<4;k++)release(&s,k);row(&s,"caller-original-holds-released",0);
    int deleted=remove_alias(&s,child);row(&s,"alias-deleted",deleted);
    if(q1)list_row(&s,"query-one-held-after-delete",q1,q2);
    if(q2)list_row(&s,"query-two-held-after-delete",q2,q1);
    if(q1){DEC(s.interp,q1);q1=NULL;}row(&s,"query-one-released",0);
    if(q2){DEC(s.interp,q2);q2=NULL;}row(&s,"query-two-released",0);
    destroy(s.interp);row(&s,"interpreter-destroyed",0);
    printf("RELEASE|%d|updates=%d,%d,%d,%d|free_rep=%d,%d,%d,%d|duplicates=%d,%d,%d,%d\n",id,s.updates[0],s.updates[1],s.updates[2],s.updates[3],s.frees[0],s.frees[1],s.frees[2],s.frees[3],s.duplicates[0],s.duplicates[1],s.duplicates[2],s.duplicates[3]);printf("CLOSED_CASE|%d\n",id);return 0;
}
int main(int argc,char **argv) {
    (void)argc;
#ifndef JIM_PROBE
    Tcl_FindExecutable(argv[0]);
#else
    (void)argv;
#endif
    if(metadata())return 3;for(int id=1;id<=6;id++){int rc=case_run(id);if(rc)return rc;}puts("CLOSED_ORIGINAL_ALIAS_OBJECTS");return 0;
}
