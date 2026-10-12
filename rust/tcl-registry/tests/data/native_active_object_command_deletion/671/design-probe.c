/* Active public ObjCommand deletion: prepared source only, not compiled or run. */
#include <limits.h>
#include <stdio.h>
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
static Obj *new_word(Interp *i) { return Jim_NewStringObj(i,"active_retiring",-1); }
static void retain(Obj *o) { Jim_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { Jim_DecrRefCount(i,o); }
static int invoke(Interp *i,Obj *o) { return Jim_EvalObjVector(i,1,&o); }
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static const char *bytes(Obj *o,Count *n) { return Jim_GetString(o,n); }
static void set_result(Interp *i,const char *s) { Jim_SetResultString(i,s,-1); }
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
static Obj *new_word(Interp *i) { (void)i; return Tcl_NewStringObj("active_retiring",-1); }
static void retain(Obj *o) { Tcl_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { (void)i; Tcl_DecrRefCount(o); }
static int invoke(Interp *i,Obj *o) { return Tcl_EvalObjv(i,1,&o,TCL_EVAL_GLOBAL); }
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj *o,Count *n) { return Tcl_GetStringFromObj(o,n); }
static void set_result(Interp *i,const char *s) { Tcl_SetObjResult(i,Tcl_NewStringObj(s,-1)); }
#endif

typedef struct State State;
typedef struct Record { State *state; int kind; } Record;
struct State {
    Interp *interp;
    int mode,sequence,recreated,calls[2],deletes[2],create_status[2];
    Record records[2];
#ifndef JIM_PROBE
    Tcl_Command original_token;
#endif
};
static int create(State *s,int kind);
static void event(State *s,const char *label,int code) {
    printf("EVENT\tcase=%d\tsequence=%d\tlabel=%s\tstatus=%d\toriginal_calls=%d\treplacement_calls=%d\toriginal_deletes=%d\treplacement_deletes=%d\n",
        s->mode,++s->sequence,label,code,s->calls[0],s->calls[1],s->deletes[0],s->deletes[1]);
    fflush(stdout);
}
static void deleted_record(Record *r) {
    State *s=r->state; s->deletes[r->kind]++;
    event(s,r->kind==0?"DELETE_ORIGINAL":"DELETE_REPLACEMENT",0);
    if(r->kind==0 && s->mode==1 && !s->recreated) {
        s->recreated=1;
        int code=create(s,1);
        event(s,"RECREATE_RETURN",code);
    }
}
static int run_command(Record *r,Interp *i,Obj *const *argv) {
    State *s=r->state; s->calls[r->kind]++;
    if(r->kind==1) {
        event(s,"REPLACEMENT_INVOKE",0);
        set_result(i,"REPLACEMENT");
        return 0;
    }
    event(s,"INVOKE_ENTER",0);
#ifdef JIM_PROBE
    int code=Jim_DeleteCommand(i,argv[0]);
#else
    (void)argv;
    int code=Tcl_DeleteCommandFromToken(i,s->original_token);
#endif
    event(s,"DELETE_API_RETURN",code);
    set_result(i,"ORIGINAL_RETURN");
    /* No observer work follows this event before the actual callback returns. */
    event(s,"INVOKE_RETURN",0);
    return 0;
}
#ifdef JIM_PROBE
static int command(Interp *i,int argc,Obj *const *argv) {
    (void)argc;
    return run_command(Jim_CmdPrivData(i),i,argv);
}
static void deleted(Interp *i,void *data) { (void)i; deleted_record(data); }
static int create(State *s,int kind) {
    int code=Jim_CreateCommand(s->interp,"active_retiring",command,&s->records[kind],deleted);
    s->create_status[kind]=code; return code;
}
#else
/* Actual Tcl_CreateObjCommand legacy int-count registration, also supported by Tcl9. */
static int command(ClientData data,Interp *i,int argc,Obj *const *argv) {
    (void)argc;
    return run_command(data,i,argv);
}
static void deleted(ClientData data) { deleted_record(data); }
static int create(State *s,int kind) {
    Tcl_Command token=Tcl_CreateObjCommand(s->interp,"active_retiring",command,&s->records[kind],deleted);
    if(kind==0)s->original_token=token;
    int code=token?TCL_OK:TCL_ERROR;
    s->create_status[kind]=code; return code;
}
#endif
static void output(State *s,const char *stage,int code) {
    Count n=0;const unsigned char *p=(const unsigned char *)bytes(result(s->interp),&n);
    printf("RESULT\tcase=%d\tsequence=%d\tstage=%s\tstatus=%d\thex=",s->mode,++s->sequence,stage,code);
    for(Count j=0;j<n;j++)printf("%02x",p[j]);
    putchar('\n');fflush(stdout);
}
int main(void) {
    for(int mode=0;mode<2;mode++) {
        State s;memset(&s,0,sizeof(s));s.mode=mode;s.interp=fresh();if(!s.interp)return 4;
        /* Independent stack client records live through full interpreter teardown.
         * Delete callbacks observe invocation, not client-data deallocation. */
        for(int kind=0;kind<2;kind++) {
            s.records[kind].state=&s;s.records[kind].kind=kind;s.create_status[kind]=-777;
        }
        event(&s,"CREATE_ORIGINAL_RETURN",create(&s,0));
        Obj *word=new_word(s.interp);retain(word);
        int code=invoke(s.interp,word);output(&s,"first-public-invocation-returned",code);
        event(&s,"BEFORE_SECOND_INVOKE",0);
        code=invoke(s.interp,word);output(&s,"second-public-invocation-returned",code);
        release(s.interp,word);event(&s,"CALLER_WORD_RELEASED",0);
        destroy(s.interp);event(&s,"INTERPRETER_DESTROYED",0);
    }
    return 0;
}
