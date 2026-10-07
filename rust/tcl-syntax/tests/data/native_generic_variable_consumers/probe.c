#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
typedef int Count;
#define NEW(i,s,n) Jim_NewStringObj(i,s,n)
#define HOLD(i,o) Jim_IncrRefCount(o)
#define DROP(i,o) Jim_DecrRefCount(i,o)
#define RUN(i,n,v) Jim_EvalObjVector(i,n,v)
#define SOURCE(i,s) Jim_Eval(i,s)
#define RESULT(i) Jim_GetResult(i)
#define STRING(o,n) Jim_GetString(o,n)
#define SET(i,n,v) Jim_SetVariable(i,n,v)
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
#define NEW(i,s,n) Tcl_NewStringObj(s,n)
#define HOLD(i,o) Tcl_IncrRefCount(o)
#define DROP(i,o) Tcl_DecrRefCount(o)
#define RUN(i,n,v) Tcl_EvalObjv(i,n,v,TCL_EVAL_DIRECT)
#define SOURCE(i,s) Tcl_Eval(i,s)
#define RESULT(i) Tcl_GetObjResult(i)
#define STRING(o,n) Tcl_GetStringFromObj(o,n)
#define SET(i,n,v) Tcl_ObjSetVar2(i,n,NULL,v,TCL_LEAVE_ERR_MSG)
#endif

static void header(const char *label, Obj *obj) {
    printf("%s|type=%s|refs=%d|bytes=%d", label,
        obj->typePtr ? obj->typePtr->name : "none", obj->refCount, obj->bytes != NULL);
    if (obj->bytes) {
        printf("|resident=");
        for (Count j=0;j<obj->length;j++) printf("%02x",(unsigned char)obj->bytes[j]);
    }
    printf("\n");
}

int main(void) {
    const char *commands[] = {"lassign", "scan", "binary", "string", "info", "lset", "append", "lappend", "empty_lappend", "catch"};
    const char *names[] = {"v", "::N::v", "arr(k::part)", "\xff\0x", "e\xcc\x81"};
    Count lengths[] = {1,6,12,3,3};
    for (int command=0; command<10; command++) for (int name=0;name<5;name++) {
#ifdef JIM_PROBE
        Interp *interp=Jim_CreateInterp();
        Jim_RegisterCoreCommands(interp);
        Jim_InitStaticExtensions(interp);
#else
        Interp *interp=Tcl_CreateInterp();
#endif
        if(command==0 && name==0) {
            SOURCE(interp,"info patchlevel");
            Count n; const char *patchlevel=STRING(RESULT(interp),&n);
            printf("engine_patchlevel=%.*s\n",(int)n,patchlevel);
        }
        SOURCE(interp,"namespace eval N {}; proc sample {{x DEFAULT}} {}");
        Obj *original=NEW(interp,names[name],lengths[name]); HOLD(interp,original);
        Obj *argv[8]; int argc=0;
        const char *head=command==8?"lappend":commands[command];
        argv[argc++]=NEW(interp,head,-1);
        if (command==0) argv[argc++]=NEW(interp,"A B",-1);
        if (command==1) {argv[argc++]=NEW(interp,"17",-1);argv[argc++]=NEW(interp,"%d",-1);}
        if (command==2) {argv[argc++]=NEW(interp,"scan",-1);argv[argc++]=NEW(interp,"AB",-1);argv[argc++]=NEW(interp,"a*",-1);}
        if (command==3) {argv[argc++]=NEW(interp,"is",-1);argv[argc++]=NEW(interp,"alpha",-1);argv[argc++]=NEW(interp,"-failindex",-1);}
        if (command==4) {argv[argc++]=NEW(interp,"default",-1);argv[argc++]=NEW(interp,"sample",-1);argv[argc++]=NEW(interp,"x",-1);}
        if (command==9) argv[argc++]=NEW(interp,"set result RESULT",-1);
        argv[argc++]=original;
        if (command==3) argv[argc++]=NEW(interp,"a2",-1);
        if (command==5) {argv[argc++]=NEW(interp,"1",-1);argv[argc++]=NEW(interp,"C",-1);}
        if (command==6 || command==7) {argv[argc++]=NEW(interp,"B",-1);argv[argc++]=NEW(interp,"C",-1);}
        for (int j=0;j<argc;j++) if(argv[j]!=original) HOLD(interp,argv[j]);
        if (command>=5 && command<=8) {
            Obj *value=NEW(interp,command==5?"A B":command==8?"A  B":"A",-1);
            HOLD(interp,value); SET(interp,original,value); DROP(interp,value);
        }
        printf("case=%s/%d\n",commands[command],name);
        header("before",original);
        int code=RUN(interp,argc,argv);
        printf("code=%d\n",code);
        header("after",original);
        Count size; const char *result=STRING(RESULT(interp),&size);
        printf("result=");for(Count j=0;j<size;j++)printf("%02x",(unsigned char)result[j]);printf("\n");
        for (int j=0;j<argc;j++) if(argv[j]!=original) DROP(interp,argv[j]);
        DROP(interp,original);
#ifdef JIM_PROBE
        Jim_FreeInterp(interp);
#else
        Tcl_DeleteInterp(interp);
#endif
    }
    return 0;
}
