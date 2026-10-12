#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static void hex(Tcl_Obj *o) {
    Count n; const unsigned char *p=(const unsigned char *)Tcl_GetStringFromObj(o,&n);
    for(Count j=0;j<n;j++) printf("%02x",p[j]);
}
static const char *type(Tcl_Obj *o) {return o->typePtr?o->typePtr->name:"none";}
static void report(Tcl_Interp *i,Tcl_Obj *o,const char *label,int source,int flags,int code) {
    Tcl_Obj *r=Tcl_GetObjResult(i);
    printf("R|%s|%d|%d|%d|%s|%d|%s|%d|",label,source,flags,code,type(o),o->bytes!=NULL,type(r),r->bytes!=NULL);
    hex(r);puts("");
#if TCL_MAJOR_VERSION >= 9 || TCL_MINOR_VERSION >= 6
    if(!strcmp(type(o),"substcode")) {
        ByteCode *bc=NULL;
#ifdef ByteCodeGetInternalRep
        ByteCodeGetInternalRep(o,o->typePtr,bc);
#else
        bc=(ByteCode *)o->internalRep.twoPtrValue.ptr1;
#endif
        if(!bc) {puts("HARNESS|missing-bytecode");return;}
        printf("C|%s|%d|%d|%d|%d|%d|%d|",label,source,flags,bc->compileEpoch,bc->nsEpoch,bc->localCachePtr!=NULL,bc->procPtr!=NULL);
        for(int off=0;off<bc->numCodeBytes;) {
            const InstructionDesc *d=&tclInstructionTable[bc->codeStart[off]];
            if(!d->numBytes) {puts("HARNESS|zero-opcode-width");return;}
            printf("%s,",d->name);off+=d->numBytes;
        }
        puts("");
        for(int j=0;j<bc->numLitObjects;j++) {
            Tcl_Obj *v=bc->objArrayPtr[j];
            printf("L|%s|%d|%d|%d|%s|%d|%d|",label,source,flags,j,type(v),v->bytes!=NULL,v->refCount);
            hex(v);puts("");
        }
    }
#endif
}
static void invoke(Tcl_Interp *i,Tcl_Obj *o,const char *label,int source,int flags) {
    printf("N|%s|%s\n",label,Tcl_GetCurrentNamespace(i)->fullName);
    Interp *ip=(Interp *)i;
    Namespace *actual=ip->varFramePtr->nsPtr;
    Tcl_Namespace *a=Tcl_FindNamespace(i,"::A",NULL,0),*b=Tcl_FindNamespace(i,"::B",NULL,0);
    printf("VF|%s|%d|%d|%d|",label,actual==(Namespace *)Tcl_GetGlobalNamespace(i),actual==(Namespace *)a,actual==(Namespace *)b);
#if TCL_MAJOR_VERSION >= 9 || TCL_MINOR_VERSION >= 6
    if(!strcmp(type(o),"substcode")) {
        ByteCode *bc=NULL;
#ifdef ByteCodeGetInternalRep
        ByteCodeGetInternalRep(o,o->typePtr,bc);
#else
        bc=(ByteCode *)o->internalRep.twoPtrValue.ptr1;
#endif
        printf("%d|%d|%d\n",bc->nsPtr==(Namespace *)Tcl_GetGlobalNamespace(i),bc->nsPtr==(Namespace *)a,bc->nsPtr==(Namespace *)b);
    } else puts("-1|-1|-1");
#else
    puts("-1|-1|-1");
#endif
    Tcl_Obj *v[5];int n=0;
    v[n++]=Tcl_NewStringObj("subst",-1);
    if(!(flags&TCL_SUBST_BACKSLASHES))v[n++]=Tcl_NewStringObj("-nobackslashes",-1);
    if(!(flags&TCL_SUBST_COMMANDS))v[n++]=Tcl_NewStringObj("-nocommands",-1);
    if(!(flags&TCL_SUBST_VARIABLES))v[n++]=Tcl_NewStringObj("-novariables",-1);
    v[n++]=o;
    for(int j=0;j<n;j++)Tcl_IncrRefCount(v[j]);
    int code;
    if(label[0]=='D') {
        Tcl_Obj *r=Tcl_SubstObj(i,o,flags);
        code=r?TCL_OK:TCL_ERROR;
        if(r) Tcl_SetObjResult(i,r);
    } else code=Tcl_EvalObjv(i,n,v,0);
    report(i,o,label,source,flags,code);
    for(int j=0;j<n;j++)Tcl_DecrRefCount(v[j]);
}
static int observe(ClientData data,Tcl_Interp *i,Count objc,Tcl_Obj *const objv[]) {
    if(objc!=2) {Tcl_WrongNumArgs(i,1,objv,"label");return TCL_ERROR;}
    const char *label=Tcl_GetString(objv[1]);
    Tcl_Obj *source=(Tcl_Obj *)data;
    int fresh=strstr(label,"FRESH")!=NULL;
    if(fresh) {source=Tcl_NewStringObj("$x",2);Tcl_IncrRefCount(source);}
    invoke(i,source,label,-2,7);
    if(fresh) Tcl_DecrRefCount(source);
    return TCL_OK;
}
static const char units[]={ 'p',0,'p','\\','u','0','0','0','0','$','x','[','s','e','t',' ','x',']','t','a','i','l'};
static const struct {const char *p;int n;} sources[]={
    {"${x}${y}",8},{units,sizeof(units)},{"$a([set k k])",13},{"[set x]p$x",10},
    {"pre[set x",9},{"pre${broken",11},{"[continue]p$x",13},{"[break]p$x",10},
    {"[return VALUE]p$x",sizeof("[return VALUE]p$x")-1},{"[return -code 7 OTHER]p$x",sizeof("[return -code 7 OTHER]p$x")-1}
};
int main(int argc,char **argv) {
    Tcl_FindExecutable(argv[0]);Tcl_Interp *i=Tcl_CreateInterp();
    printf("VERSION|%s\n",TCL_PATCH_LEVEL);
    for(int source=0;source<sizeof(sources)/sizeof(*sources);source++) {
        Tcl_Obj *o=Tcl_NewStringObj(sources[source].p,sources[source].n);Tcl_IncrRefCount(o);
        for(int flags=7;flags>=0;flags--) {
            Tcl_Eval(i,"set x X;set y Y;set a(k) K;set k k");
            invoke(i,o,"ROOT",source,flags);
            invoke(i,o,"ROOT_REPEAT",source,flags);
        }
        Tcl_DecrRefCount(o);
    }
    Tcl_Obj *o=Tcl_NewStringObj("$x",2);Tcl_IncrRefCount(o);
#if TCL_MAJOR_VERSION >= 9
    Tcl_CreateObjCommand2(i,"observeSubst",observe,(ClientData)o,NULL);
#else
    Tcl_CreateObjCommand(i,"observeSubst",observe,(ClientData)o,NULL);
#endif
    Tcl_Eval(i,"namespace eval A {set x A};namespace eval B {set x B};set x ROOT");
    invoke(i,o,"ROOT_CONTEXT",-1,7);
    Tcl_Eval(i,"namespace eval A {observeSubst A_SCRIPT_CONTEXT;observeSubst DIRECT_A_SCRIPT_CONTEXT;observeSubst DIRECT_FRESH_A_SCRIPT_CONTEXT};namespace eval B {observeSubst B_SCRIPT_CONTEXT;observeSubst DIRECT_B_SCRIPT_CONTEXT;observeSubst DIRECT_FRESH_B_SCRIPT_CONTEXT}");
    Tcl_CallFrame frameA,frameB;
    Tcl_Namespace *a=Tcl_FindNamespace(i,"::A",NULL,0),*b=Tcl_FindNamespace(i,"::B",NULL,0);
    Tcl_PushCallFrame(i,&frameA,a,0);invoke(i,o,"A_CONTEXT",-1,7);Tcl_PopCallFrame(i);
    Tcl_PushCallFrame(i,&frameB,b,0);invoke(i,o,"B_CONTEXT",-1,7);Tcl_PopCallFrame(i);
    invoke(i,o,"ROOT_RETURN",-1,7);
    Tcl_Eval(i,"rename set savedSet;interp alias {} set {} savedSet");
    invoke(i,o,"ROOT_NEW_COMMAND_EPOCH",-1,7);
    Tcl_DeleteCommand(i,"set");Tcl_Eval(i,"rename savedSet set");
#if TCL_MAJOR_VERSION >= 9
    Tcl_CreateObjCommand2(i,"observeSubst",observe,(ClientData)o,NULL);
#else
    Tcl_CreateObjCommand(i,"observeSubst",observe,(ClientData)o,NULL);
#endif
    Tcl_Eval(i,"proc P {x} {observeSubst P_FIRST;observeSubst DIRECT_P};proc Q {padding x} {observeSubst Q_FIRST;observeSubst DIRECT_Q}");
    Tcl_Eval(i,"P P_VALUE");Tcl_Eval(i,"P P_REPEAT");Tcl_Eval(i,"Q PAD Q_VALUE");Tcl_Eval(i,"P P_RETURN");
    Tcl_DeleteCommand(i,"observeSubst");
    Tcl_DecrRefCount(o);Tcl_DeleteInterp(i);Tcl_Finalize();(void)argc;return 0;
}
