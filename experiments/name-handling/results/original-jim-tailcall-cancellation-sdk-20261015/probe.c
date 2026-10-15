#include <stdio.h>
#include "jim.h"
static const char *label;
static Jim_Interp *actual;
static Jim_Obj *payload;
static int deleted, invoked;
static Jim_Interp *fresh(void) { Jim_Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); return Jim_InitStaticExtensions(i)==JIM_OK ? i:NULL; }
static void row(const char *kind,int code,Jim_Obj *object) { int length; const char *bytes=Jim_GetString(object,&length); printf("%s|%s|%d|",kind,label,code); for(int n=0;n<length;n++)printf("%02x",(unsigned char)bytes[n]); puts(""); }
static void destroy(Jim_Interp *interp,void *data) {
 (void)data; deleted++;
 Jim_Obj *pending=interp->framePtr->tailcallObj;
 printf("DELETE|%s|%d|%d\n",label,deleted,invoked);
 printf("PENDING_LIST_AT_DELETE|%s|%d|%d\n",label,pending!=NULL,pending ? pending->internalRep.listValue.len:0);
 printf("PENDING_MEMBER_AT_DELETE|%s|%d|%d\n",label,pending && pending->internalRep.listValue.len==2 && pending->internalRep.listValue.ele[1]==payload,payload->refCount);
}
static int selected(Jim_Interp *interp,int argc,Jim_Obj *const *argv) { invoked++; printf("INVOKE|%s|%d|%d\n",label,deleted,invoked); if(argc!=2 || argv[1]!=payload)return JIM_ERR; Jim_SetResultString(interp,"VALUE",-1); return JIM_OK; }
static int execute(int cancelled) {
 actual=fresh(); if(!actual)return 2; label=cancelled ? "PENDING_CANCELLED":"SELECTED_EXECUTED"; deleted=invoked=0;
 Jim_CreateCommand(actual,"selected",selected,NULL,destroy);
 Jim_Obj *ns=Jim_NewIntObj(actual,42); Jim_IncrRefCount(ns);
 const char *script=cancelled ? "defer {rename ::selected {}; error CANCELLED}; tailcall $target $payload":"defer {rename ::selected {}}; tailcall $target $payload";
 Jim_Obj *parts[]={Jim_NewStringObj(actual,"target payload",-1),Jim_NewStringObj(actual,script,-1),ns};
 Jim_Obj *lambda=Jim_NewListObj(actual,parts,3); Jim_IncrRefCount(lambda);
 Jim_Obj *head=Jim_NewStringObj(actual,"apply",-1); Jim_IncrRefCount(head);
 Jim_Obj *target=Jim_NewStringObj(actual,"::selected",-1); Jim_IncrRefCount(target);
 payload=Jim_NewIntObj(actual,84); Jim_IncrRefCount(payload);
 Jim_Obj *argv[]={head,lambda,target,payload};
 printf("PAYLOAD_BEFORE|%s|%d|%s\n",label,payload->bytes!=NULL,payload->typePtr->name);
 int code=Jim_EvalObjVector(actual,4,argv);
 printf("PAYLOAD_AFTER|%s|%d|%s\n",label,payload->bytes!=NULL,payload->typePtr->name);
 printf("COUNTS|%s|%d|%d\n",label,deleted,invoked); row("RESULT",code,Jim_GetResult(actual));
 Jim_DecrRefCount(actual,head);Jim_DecrRefCount(actual,lambda);Jim_DecrRefCount(actual,target);Jim_DecrRefCount(actual,payload);Jim_DecrRefCount(actual,ns);Jim_FreeInterp(actual);return 0;
}
int main(void) { actual=fresh();if(!actual)return 2;label="VERSION";if(Jim_Eval(actual,"info patchlevel")!=JIM_OK)return 3;row("VERSION",0,Jim_GetResult(actual));Jim_FreeInterp(actual);int code=execute(0);return code ? code:execute(1); }
