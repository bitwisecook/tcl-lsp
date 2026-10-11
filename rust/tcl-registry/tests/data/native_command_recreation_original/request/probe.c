/* Original public command replacement and a once-only recreating delete callback. */
#include <inttypes.h>
#include <limits.h>
#include <stdint.h>
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
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static const char *bytes(Obj *o,Count *n) { return Jim_GetString(o,n); }
static int eval(Interp *i,const char *s) {
    Obj *o=Jim_NewStringObj(i,s,(int)strlen(s)); Jim_IncrRefCount(o);
    int rc=Jim_EvalObj(i,o); Jim_DecrRefCount(i,o); return rc;
}
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
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj *o,Count *n) { return Tcl_GetStringFromObj(o,n); }
static int eval(Interp *i,const char *s) {
    Obj *o=Tcl_NewStringObj(s,(Count)strlen(s)); Tcl_IncrRefCount(o);
    int rc=Tcl_EvalObjEx(i,o,0); Tcl_DecrRefCount(o); return rc;
}
static void set_result(Interp *i,const char *s) { Tcl_SetObjResult(i,Tcl_NewStringObj(s,-1)); }
#endif

typedef struct State State;
typedef struct Record { State *state; int kind; } Record;
struct State {
    Interp *interp;
    int mode,sequence,recreated,create_status[3],deletes[3],calls[3];
    uintptr_t tokens[3];
    Record records[3];
};
static int create(State *s,int kind);
static void event(Record *r) {
    State *s=r->state; s->deletes[r->kind]++;
    printf("EVENT\tcase=%d\tsequence=%d\tdelete=%d\told=%d\tmiddle=%d\tfinal=%d\n",
        s->mode,++s->sequence,r->kind,s->deletes[0],s->deletes[1],s->deletes[2]);
    fflush(stdout);
    if(r->kind==0 && s->mode==1 && !s->recreated) {
        s->recreated=1;
        int code=create(s,1);
        printf("RECREATE\tcase=%d\tsequence=%d\tstatus=%d\tmiddle_token=%" PRIxPTR "\n",
            s->mode,++s->sequence,code,s->tokens[1]);
        fflush(stdout);
    }
}
#ifdef JIM_PROBE
static int command(Interp *i,int argc,Obj *const *argv) {
    (void)argc; (void)argv;
    Record *r=Jim_CmdPrivData(i); r->state->calls[r->kind]++;
    set_result(i,r->kind==0?"OLD":r->kind==1?"MIDDLE":"FINAL"); return JIM_OK;
}
static void deleted(Interp *i,void *data) { (void)i; event(data); }
static int create(State *s,int kind) {
    int code=Jim_CreateCommand(s->interp,"retiring",command,&s->records[kind],deleted);
    s->create_status[kind]=code; return code;
}
static int delete_public(Interp *i) {
    Obj *name=Jim_NewStringObj(i,"retiring",8); Jim_IncrRefCount(name);
    int code=Jim_DeleteCommand(i,name); Jim_DecrRefCount(i,name); return code;
}
#else
static int command(ClientData data,Interp *i,int argc,Obj *const *argv) {
    (void)argc; (void)argv;
    Record *r=data; r->state->calls[r->kind]++;
    set_result(i,r->kind==0?"OLD":r->kind==1?"MIDDLE":"FINAL"); return TCL_OK;
}
static void deleted(ClientData data) { event(data); }
static int create(State *s,int kind) {
    Tcl_Command token=Tcl_CreateObjCommand(s->interp,"retiring",command,&s->records[kind],deleted);
    s->tokens[kind]=(uintptr_t)token;
    int code=token?TCL_OK:TCL_ERROR; s->create_status[kind]=code; return code;
}
static int delete_public(Interp *i) { return Tcl_DeleteCommand(i,"retiring"); }
#endif
static void hex_result(Interp *i) {
    Count n=0; const unsigned char *p=(const unsigned char *)bytes(result(i),&n);
    for(Count j=0;j<n;j++) printf("%02x",p[j]);
}
static void window(const char *stage,State *s,int code) {
    printf("ROW\tcase=%d\tstage=%s\tstatus=%d\told_delete=%d\tmiddle_delete=%d\tfinal_delete=%d\told_call=%d\tmiddle_call=%d\tfinal_call=%d\trecreated=%d\n",
        s->mode,stage,code,s->deletes[0],s->deletes[1],s->deletes[2],s->calls[0],s->calls[1],s->calls[2],s->recreated);
    fflush(stdout);
}
static int metadata(void) {
    Interp *i=fresh(); if(!i)return 3;
#ifdef JIM_PROBE
    int code=eval(i,"list [info patchlevel] [info version]");
    printf("VERSION\t%d\t",code); hex_result(i); putchar('\n');
    printf("BUILD\tJIM_GITVERSION=%s\tJIM_VERSION=%d\tJIM_ABI_VERSION=%d\tJIM_UTF8=%d\n",JIM_GITVERSION,JIM_VERSION,JIM_ABI_VERSION,JIM_UTF8);
#else
    int code=eval(i,"list [info patchlevel] [info tclversion]");
    printf("VERSION\t%d\t",code); hex_result(i); putchar('\n');
    int a,b,c,d; Tcl_GetVersion(&a,&b,&c,&d);
    printf("BUILD\tHEADER=%s\tGetVersion=%d.%d.%d/%d\n",TCL_PATCH_LEVEL,a,b,c,d);
#endif
    printf("ABI\tint=%zu\tlong=%zu\tptr=%zu\tCHAR_BIT=%d\n",sizeof(int),sizeof(long),sizeof(void *),CHAR_BIT);
    destroy(i); return code;
}
int main(void) {
    int status=metadata(); if(status)return status;
    for(int mode=0;mode<2;mode++) {
        State s; memset(&s,0,sizeof(s)); s.mode=mode; s.interp=fresh(); if(!s.interp)return 4;
        for(int k=0;k<3;k++) { s.records[k].state=&s; s.records[k].kind=k; s.create_status[k]=-777; }
        int original=create(&s,0); window("original-created",&s,original);
        int final=create(&s,2); window("replacement-returned",&s,final);
        printf("TOKENS\tcase=%d\told=%" PRIxPTR "\tmiddle=%" PRIxPTR "\tfinal=%" PRIxPTR "\n",mode,s.tokens[0],s.tokens[1],s.tokens[2]);
        int invoked=eval(s.interp,"retiring"); window("invocation-returned",&s,invoked);
        printf("RESULT\tcase=%d\tstatus=%d\thex=",mode,invoked); hex_result(s.interp); putchar('\n');
        int removed=delete_public(s.interp); window("public-delete-returned",&s,removed);
        destroy(s.interp); window("interpreter-destroyed",&s,0);
    }
    return 0;
}
