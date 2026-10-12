#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
static Jim_Cmd *original;
static Jim_Obj *original_ns;
static Jim_Cmd *lookup(Jim_Interp *i, const char *s) {
    Jim_Obj *o=Jim_NewStringObj(i,s,-1); Jim_IncrRefCount(o);
    Jim_Cmd *c=Jim_GetCommand(i,o,0); Jim_DecrRefCount(i,o); return c;
}
static int observe(Jim_Interp *i,int argc,Jim_Obj *const *argv) {
    printf("active\t%d\t%d\n",i->framePtr->nsObj==original_ns,original->u.proc.nsObj==original_ns);
    return JIM_OK;
}
#else
#include "tclInt.h"
static Tcl_Command original;
static Proc *original_proc;
static Tcl_Namespace *original_ns;
static int observe(ClientData data,Tcl_Interp *i,int argc,Tcl_Obj *const *argv) {
    printf("active\t%d\t%d\n",((Interp*)i)->varFramePtr->nsPtr==(Namespace*)original_ns,original_proc->cmdPtr->nsPtr==(Namespace*)original_ns);
    return TCL_OK;
}
#endif
static void hex(const char *s,int n){for(int j=0;j<n;j++)printf("%02x",(unsigned char)s[j]);}
int main(int argc,char **argv) {
    const char *setup="namespace eval A {proc p {move destination} {set before [namespace current]; if {$move} {rename ::A::p $destination; observe}; list $before [namespace current]}}; namespace eval B {}";
    const char *destinations[]={"::B::q","q","B::q","q","::q","B::q"};
    for(int k=0;k<6;k++) {
        char command[256];if(k<3)sprintf(command,"::A::p 1 %s",destinations[k]);else sprintf(command,"rename ::A::p %s",destinations[k]);
#ifdef JIM_PROBE
        Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_InitStaticExtensions(i);
        Jim_CreateCommand(i,"observe",observe,NULL,NULL);
        int code=Jim_Eval(i,setup);if(code){fprintf(stderr,"setup: %s\n",Jim_String(Jim_GetResult(i)));return 2;}
        original=lookup(i,"::A::p");if(!original)return 3;original_ns=original->u.proc.nsObj;Jim_IncrRefCount(original_ns);
        code=Jim_Eval(i,command);int n;const char *s=Jim_GetString(Jim_GetResult(i),&n);
        printf("first\t%d\t%d\t",k,code);hex(s,n);puts("");
        printf("namespace-holder\t%d\t%d\n",k,original->u.proc.nsObj==original_ns);
        char name[128];name[0]=0;Jim_HashTableIterator *iterator=Jim_GetHashTableIterator(&i->commands);Jim_HashEntry *entry;
        while((entry=Jim_NextHashEntry(iterator)))if(Jim_GetHashEntryVal(entry)==original){snprintf(name,sizeof(name),"%s",Jim_String((Jim_Obj*)Jim_GetHashEntryKey(entry)));break;}
        Jim_FreeHashTableIterator(iterator);if(!name[0])return 4;
        Jim_Cmd *after=lookup(i,name);printf("identity\t%d\t%d\n",k,after==original);
        printf("name\t%d\t",k);hex(name,strlen(name));puts("");
        sprintf(command,"%s 0 unused",name);code=Jim_Eval(i,command);s=Jim_GetString(Jim_GetResult(i),&n);
        printf("future\t%d\t%d\t",k,code);hex(s,n);puts("");Jim_DecrRefCount(i,original_ns);Jim_FreeInterp(i);
#else
        Tcl_FindExecutable(argv[0]);Tcl_Interp *i=Tcl_CreateInterp();Tcl_CreateObjCommand(i,"observe",observe,NULL,NULL);
        int code=Tcl_Eval(i,setup);if(code){fprintf(stderr,"setup: %s\n",Tcl_GetStringResult(i));return 2;}
        original=Tcl_FindCommand(i,"::A::p",NULL,0);Tcl_CmdInfo info;Tcl_GetCommandInfoFromToken(original,&info);
#if TCL_MAJOR_VERSION == 9 && TCL_MINOR_VERSION >= 1
        original_proc=(Proc*)info.objClientData2;
#else
        original_proc=(Proc*)info.objClientData;
#endif
        original_ns=Tcl_FindNamespace(i,"::A",NULL,0);
        code=Tcl_Eval(i,command);
#if TCL_MAJOR_VERSION >= 9
        Tcl_Size n;
#else
        int n;
#endif
        const char *s=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n);
        printf("first\t%d\t%d\t",k,code);hex(s,n);puts("");
        printf("namespace-holder\t%d\t%d\n",k,original_proc->cmdPtr->nsPtr==(Namespace*)original_ns);
        Tcl_Obj *nameObj=Tcl_NewObj();Tcl_IncrRefCount(nameObj);Tcl_GetCommandFullName(i,original,nameObj);
        char name[128];snprintf(name,sizeof(name),"%s",Tcl_GetString(nameObj));Tcl_DecrRefCount(nameObj);
        Tcl_Command after=Tcl_FindCommand(i,name,NULL,0);if(!after)return 4;Tcl_GetCommandInfoFromToken(after,&info);
#if TCL_MAJOR_VERSION == 9 && TCL_MINOR_VERSION >= 1
        printf("identity\t%d\t%d\t%d\n",k,after==original,info.objClientData2==(ClientData)original_proc);
#else
        printf("identity\t%d\t%d\t%d\n",k,after==original,info.objClientData==(ClientData)original_proc);
#endif
        printf("name\t%d\t",k);hex(name,strlen(name));puts("");
        sprintf(command,"%s 0 unused",name);code=Tcl_Eval(i,command);s=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n);
        printf("future\t%d\t%d\t",k,code);hex(s,n);puts("");Tcl_DeleteInterp(i);
#endif
    }
#ifndef JIM_PROBE
    Tcl_Finalize();
#endif
    return 0;
}
