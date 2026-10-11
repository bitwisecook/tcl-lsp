/* Original TclOO operand updater stages through public object-vector APIs. */
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
    char path[4096]; char source[4096]; size_t n;
    int size=snprintf(path,sizeof(path),"%s/%s",source_root,name);
    if(size<0 || size>=(int)sizeof(path)) return -701;
    FILE *f=fopen(path,"rb"); if(!f) return -702;
    n=fread(source,1,sizeof(source),f);
    if(ferror(f) || !feof(f) || fclose(f)) return -703;
    return eval_source(i,source,(int)n);
}
static void hex_result(Interp *i) {
    Count n=0; const unsigned char *p=(const unsigned char *)bytes(result(i),&n);
    for(Count j=0;j<n;j++) printf("%02x",p[j]);
}
static int metadata(void) {
    Interp *i=fresh(); if(!i)return 3;
    #ifdef JIM_PROBE
    int code=eval_file(i,"jim-version-probe.tcl");
#else
    int code=eval_file(i,"version-probe.tcl");
#endif
    printf("VERSION\tstatus=%d\thex=",code); hex_result(i); putchar('\n');
#ifdef JIM_PROBE
    printf("BUILD\tJIM_GITVERSION=%s\tJIM_VERSION=%d\tJIM_ABI_VERSION=%d\tJIM_UTF8=%d\n",JIM_GITVERSION,JIM_VERSION,JIM_ABI_VERSION,JIM_UTF8);
#else
    int a,b,c,d; Tcl_GetVersion(&a,&b,&c,&d);
    printf("BUILD\tHEADER=%s\tGetVersion=%d.%d.%d/%d\n",TCL_PATCH_LEVEL,a,b,c,d);
#endif
    printf("ABI\tint=%zu\tlong=%zu\tptr=%zu\tCHAR_BIT=%d\n",sizeof(int),sizeof(long),sizeof(void *),CHAR_BIT);
    destroy(i); return code;
}
#ifndef JIM_PROBE
typedef struct State State;
typedef struct Record { State *state; int role; const char *payload; } Record;
struct State {
    Interp *interp; int case_id,sequence,updates[4],frees[4],duplicates[4];
    int report_mode,capture_calls; Obj *held[4]; Record records[4];
};
static void row(State *s,const char *stage,int code) {
    printf("ROW\tcase=%d\tstage=%s\tstatus=%d\tsequence=%d\tcapture_calls=%d",s->case_id,stage,code,s->sequence,s->capture_calls);
    for(int k=0;k<4;k++) {
        Obj *o=s->held[k];
        printf("\tr%d_update=%d\tr%d_free=%d\tr%d_duplicate=%d\tr%d_refs=%d\tr%d_string=%d",
            k,s->updates[k],k,s->frees[k],k,s->duplicates[k],k,o?o->refCount:-1,k,o&&o->bytes!=NULL);
    }
    putchar('\n'); fflush(stdout);
}
static void free_rep(Obj *o) {
    Record *r=o->internalRep.twoPtrValue.ptr1; r->state->frees[r->role]++;
    printf("EVENT\tcase=%d\tsequence=%d\tkind=free\trole=%d\n",r->state->case_id,++r->state->sequence,r->role); fflush(stdout);
}
static void duplicate_rep(Obj *from,Obj *to);
static void update_rep(Obj *o) {
    Record *r=o->internalRep.twoPtrValue.ptr1; size_t n=strlen(r->payload);
    r->state->updates[r->role]++;
    printf("EVENT\tcase=%d\tsequence=%d\tkind=update\trole=%d\n",r->state->case_id,++r->state->sequence,r->role); fflush(stdout);
    o->bytes=Tcl_Alloc((unsigned)n+1); memcpy(o->bytes,r->payload,n+1); o->length=(Count)n;
}
static Tcl_ObjType original_type={"originalOoOperandStage",free_rep,duplicate_rep,update_rep,NULL};
static void duplicate_rep(Obj *from,Obj *to) {
    Record *r=from->internalRep.twoPtrValue.ptr1; r->state->duplicates[r->role]++;
    printf("EVENT\tcase=%d\tsequence=%d\tkind=duplicate\trole=%d\n",r->state->case_id,++r->state->sequence,r->role); fflush(stdout);
    to->internalRep=from->internalRep; to->typePtr=&original_type;
}
static Obj *held(State *s,int role,const char *payload) {
    Record *r=&s->records[role]; r->state=s; r->role=role; r->payload=payload;
    Obj *o=Tcl_NewObj(); Tcl_IncrRefCount(o); Tcl_InvalidateStringRep(o);
    o->typePtr=&original_type; o->internalRep.twoPtrValue.ptr1=r; o->internalRep.twoPtrValue.ptr2=NULL;
    s->held[role]=o; return o;
}
static Obj *word(const char *s) { Obj *o=Tcl_NewStringObj(s,-1); Tcl_IncrRefCount(o); return o; }
static int invoke(State *s,int count,Obj **argv) { return Tcl_EvalObjv(s->interp,count,argv,0); }
static void release_words(int count,Obj **argv) { for(int k=0;k<count;k++) Tcl_DecrRefCount(argv[k]); }
static int capture(ClientData data,Interp *i,int argc,Obj *const *argv) {
    State *s=data; s->capture_calls++;
    printf("CALL\tcase=%d\tkind=forward\targc=%d\toriginal_member=%d\n",s->case_id,argc,argc>1&&argv[1]==s->held[0]);
    row(s,"capture-before-read",TCL_OK);
    for(int k=1;k<argc;k++) { Count n; (void)bytes(argv[k],&n); }
    row(s,"capture-after-read",TCL_OK);
    Tcl_SetObjResult(i,Tcl_NewStringObj("CAPTURED",-1)); return TCL_OK;
}
static int observer(ClientData data,Interp *i,int argc,Obj *const *argv) {
    State *s=data; (void)argc; (void)argv; row(s,"definition-entered",TCL_OK);
    if(!s->report_mode) { Tcl_SetObjResult(i,Tcl_NewStringObj("NO_REPORT",-1)); return TCL_OK; }
    Obj *words[3]={word("info"),word("level"),word("0")};
    int rc=Tcl_EvalObjv(i,3,words,0); release_words(3,words);
    row(s,"definition-info-level-returned",rc);
    if(rc==TCL_OK) {
        Count n=0; Obj **members=NULL;
        int list_rc=Tcl_ListObjGetElements(i,result(i),&n,&members);
        int found=0; if(list_rc==TCL_OK) for(Count k=0;k<n;k++) if(members[k]==s->held[0])found++;
        printf("REPORT\tcase=%d\tstatus=%d\tcount=%lld\toriginal_member=%d\n",s->case_id,list_rc,(long long)n,found);
        row(s,"definition-info-list-accessed",list_rc);
    }
    if(s->report_mode==2) {
        printf("PRESENTATION\tcase=%d\thex=",s->case_id); hex_result(i); putchar('\n');
        row(s,"definition-info-presented",rc);
    }
    return rc;
}
static int forward_case(State *s) {
    Obj *prefix=held(s,0,"prefix-data");
    Tcl_CreateObjCommand(s->interp,"capture",capture,s,NULL);
    Obj *words[6]={word("oo::define"),word("C"),word("forward"),word("f"),word("capture"),prefix};
    row(s,"initial",0); int rc=invoke(s,6,words); row(s,"forward-defined",rc); release_words(5,words);
    if(rc!=TCL_OK)return rc;
    if(s->case_id==1) { rc=eval_file(s->interp,"invoke-forward.tcl"); row(s,"forward-invoked-without-report",rc); return rc; }
    Obj *info[5]={word("info"),word("class"),word("forward"),word("C"),word("f")};
    rc=invoke(s,5,info); row(s,"info-forward-returned",rc); release_words(5,info);
    if(rc==TCL_OK) {
        Count n=0; Obj **members=NULL; int lr=Tcl_ListObjGetElements(s->interp,result(s->interp),&n,&members);
        printf("PREFIX\tcase=%d\tstatus=%d\tcount=%lld\toriginal_member=%d\n",s->case_id,lr,(long long)n,lr==TCL_OK&&n>1&&members[1]==prefix);
        row(s,"info-forward-list-accessed",lr);
    }
    printf("PRESENTATION\tcase=%d\thex=",s->case_id); hex_result(s->interp); putchar('\n'); row(s,"info-forward-presented",rc);
    rc=eval_file(s->interp,"invoke-forward.tcl"); row(s,"forward-invoked",rc); return rc;
}
static int frame_case(State *s) {
    s->report_mode=s->case_id-2;
    Obj *payload=held(s,0,"frame-data");
    Tcl_CreateObjCommand(s->interp,"::oo::define::observer",observer,s,NULL);
    Obj *words[4]={word("oo::define"),word("C"),word("observer"),payload};
    row(s,"initial",0); int rc=invoke(s,4,words); row(s,"definition-returned",rc); release_words(3,words); return rc;
}
static int method_case(State *s) {
    int option=s->case_id>=7;
    Obj *name=held(s,0,"m"); Obj *params=held(s,1,s->case_id==6?"{broken":"");
    Obj *body=held(s,2,"return BODY");
    Obj *flag=option?held(s,3,s->case_id==8?"-bad":"-export"):NULL;
    Obj *words[7]={word("oo::define"),word("C"),word("method"),name,NULL,NULL,NULL};
    int count=option?7:6;
    if(option) { words[4]=flag; words[5]=params; words[6]=body; } else { words[4]=params;words[5]=body; }
    row(s,"initial",0); int rc=invoke(s,count,words); row(s,"method-returned",rc); release_words(3,words); return rc;
}
static int supported_cases(void) {
    for(int id=0;id<9;id++) {
        State s; memset(&s,0,sizeof(s)); s.case_id=id; s.interp=fresh(); if(!s.interp)return 4;
        int setup=eval_file(s.interp,"setup.tcl"); printf("CASE\tid=%d\tsetup=%d\n",id,setup);
        if(setup!=TCL_OK) { destroy(s.interp); return 5; }
        int rc=id<=1?forward_case(&s):id<=4?frame_case(&s):method_case(&s);
        printf("RESULT\tcase=%d\tstatus=%d\thex=",id,rc); hex_result(s.interp); putchar('\n'); row(&s,"result-presented",rc);
        destroy(s.interp); row(&s,"interpreter-destroyed-held-originals",0);
        for(int k=0;k<4;k++) if(s.held[k])Tcl_DecrRefCount(s.held[k]);
        printf("RELEASE\tcase=%d\tupdates=%d,%d,%d,%d\tfrees=%d,%d,%d,%d\tduplicates=%d,%d,%d,%d\n",id,s.updates[0],s.updates[1],s.updates[2],s.updates[3],s.frees[0],s.frees[1],s.frees[2],s.frees[3],s.duplicates[0],s.duplicates[1],s.duplicates[2],s.duplicates[3]);
    }
    return 0;
}
#endif
int main(int argc,char **argv) {
    if(argc!=2)return 2; source_root=argv[1];
    int status=metadata(); if(status)return status;
    Interp *i=fresh(); if(!i)return 3;
    status=eval_file(i,"availability.tcl");
    printf("AVAILABILITY\tstatus=%d\thex=",status); hex_result(i); putchar('\n');
    Count n=0; const char *p=bytes(result(i),&n); int available=status==0&&n==5&&memcmp(p,"1 1 1",5)==0;
    destroy(i); if(status)return status;
#ifdef JIM_PROBE
    printf("UNAVAILABLE\tcommand_available=%d\treason=public-TclOO-object-API-not-a-Jim-API\n",available); return 0;
#else
    if(!available) { printf("UNAVAILABLE\tcommand_available=0\treason=original-initialized-provider-has-no-TclOO\n"); return 0; }
    return supported_cases();
#endif
}
