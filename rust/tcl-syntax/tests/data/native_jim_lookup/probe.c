#include <stdio.h>
#include <string.h>
#include "jim.h"
static Jim_Obj *held;
static void report(Jim_Interp *i,const char *label,Jim_Obj *o,int ok) {
    const char *type=o->typePtr?o->typePtr->name:"none";
    printf("%s\tok=%d\ttype=%s\trefs=%d\tframe=%lu\tproc=%lu",label,ok,type,o->refCount,i->framePtr->id,i->procEpoch);
    if(!strcmp(type,"command"))printf("\tcacheproc=%lu\tnsrefs=%d\tcmdlive=%d",o->internalRep.cmdValue.procEpoch,o->internalRep.cmdValue.nsObj->refCount,o->internalRep.cmdValue.procEpoch==i->procEpoch ? (o->internalRep.cmdValue.cmdPtr->inUse!=0) : -1);
    if(!strcmp(type,"variable"))printf("\tcacheframe=%lu\tglobal=%d",o->internalRep.varValue.callFrameId,o->internalRep.varValue.global);
    printf("\n");
}
static int active_command(Jim_Interp *i,int ac,Jim_Obj *const*av) {
 (void)ac;Jim_Cmd *before=Jim_GetCommand(i,av[0],JIM_NONE);
 Jim_DeleteCommand(i,av[0]);Jim_Cmd *after=Jim_GetCommand(i,av[0],JIM_NONE);
 report(i,"cmd-deleted-active-original-hit",av[0],after==before);
 Jim_Obj *fresh=Jim_NewStringObj(i,"active",6);Jim_IncrRefCount(fresh);
 report(i,"cmd-deleted-active-fresh-miss",fresh,Jim_GetCommand(i,fresh,JIM_NONE)!=NULL);Jim_DecrRefCount(i,fresh);
 return JIM_OK;
}
static int command(Jim_Interp *i,int ac,Jim_Obj *const*av) {(void)ac;(void)av;Jim_SetResultString(i,"OK",-1);return JIM_OK;}
static int inside(Jim_Interp *i,int ac,Jim_Obj *const*av) {(void)ac;(void)av;report(i,"var-local-frame-miss",held,Jim_GetVariable(i,held,JIM_NONE)!=NULL);return JIM_OK;}
static Jim_Obj *obj(Jim_Interp*i,const char*s,int n){Jim_Obj*o=Jim_NewStringObj(i,s,n);Jim_IncrRefCount(o);return o;}
int main(void) {
 Jim_Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_InitStaticExtensions(i);
 Jim_Obj*n=obj(i,"probe",5);report(i,"cmd-miss-fresh",n,Jim_GetCommand(i,n,JIM_NONE)!=NULL);
 Jim_CreateCommand(i,"probe",command,NULL,NULL);report(i,"cmd-first",n,Jim_GetCommand(i,n,JIM_NONE)!=NULL);
 Jim_Obj*d=Jim_DuplicateObj(i,n);Jim_IncrRefCount(d);report(i,"cmd-duplicate",d,1);Jim_DecrRefCount(i,d);
 Jim_CreateCommand(i,"probe",command,NULL,NULL);report(i,"cmd-replace-before-lookup",n,1);report(i,"cmd-replace-lookup",n,Jim_GetCommand(i,n,JIM_NONE)!=NULL);
 Jim_Eval(i,"rename probe renamed");report(i,"cmd-rename-before-lookup",n,1);report(i,"cmd-rename-old-miss",n,Jim_GetCommand(i,n,JIM_NONE)!=NULL);
 Jim_Eval(i,"rename renamed {} ");report(i,"cmd-retired-miss",n,Jim_GetCommand(i,n,JIM_NONE)!=NULL);Jim_DecrRefCount(i,n);
 held=obj(i,"x",1);report(i,"var-miss-fresh",held,Jim_GetVariable(i,held,JIM_NONE)!=NULL);
 Jim_Obj*v=obj(i,"V",1);report(i,"var-created",held,Jim_SetVariable(i,held,v)==JIM_OK);report(i,"var-read",held,Jim_GetVariable(i,held,JIM_NONE)==v);
 d=Jim_DuplicateObj(i,held);Jim_IncrRefCount(d);report(i,"var-duplicate",d,Jim_GetVariable(i,d,JIM_NONE)==v);Jim_DecrRefCount(i,d);
 Jim_CreateCommand(i,"inside",inside,NULL,NULL);Jim_Eval(i,"proc p {} {inside}; p");report(i,"var-root-again",held,Jim_GetVariable(i,held,JIM_NONE)==v);
 n=obj(i,"other",5);Jim_SetVariable(i,n,v);Jim_UnsetVariable(i,n,JIM_NONE);report(i,"var-unrelated-unset-before-lookup",held,1);report(i,"var-unrelated-unset-lookup",held,Jim_GetVariable(i,held,JIM_NONE)==v);Jim_DecrRefCount(i,n);
 Jim_UnsetVariable(i,held,JIM_NONE);report(i,"var-unset-before-miss",held,1);report(i,"var-unset-miss",held,Jim_GetVariable(i,held,JIM_NONE)!=NULL);report(i,"var-recreate",held,Jim_SetVariable(i,held,v)==JIM_OK);Jim_DecrRefCount(i,held);
 n=obj(i,"::absolute",10);report(i,"var-absolute-create",n,Jim_SetVariable(i,n,v)==JIM_OK);report(i,"var-absolute-read",n,Jim_GetVariable(i,n,JIM_NONE)==v);Jim_DecrRefCount(i,n);
 n=obj(i,"key\0tail",8);report(i,"var-counted-nul-create",n,Jim_SetVariable(i,n,v)==JIM_OK);report(i,"var-counted-nul-read",n,Jim_GetVariable(i,n,JIM_NONE)==v);Jim_DecrRefCount(i,n);
 n=obj(i,"k\xff",2);report(i,"var-opaque-create",n,Jim_SetVariable(i,n,v)==JIM_OK);report(i,"var-opaque-read",n,Jim_GetVariable(i,n,JIM_NONE)==v);Jim_DecrRefCount(i,n);
 Jim_CreateCommand(i,"active",active_command,NULL,NULL);n=obj(i,"active",6);Jim_GetCommand(i,n,JIM_NONE);Jim_EvalObjVector(i,1,&n);report(i,"cmd-after-active-retirement",n,Jim_GetCommand(i,n,JIM_NONE)!=NULL);Jim_DecrRefCount(i,n);
 Jim_DecrRefCount(i,v);Jim_FreeInterp(i);return 0;
}
