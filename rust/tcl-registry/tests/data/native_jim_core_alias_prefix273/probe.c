#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
static void version(Jim_Interp *i) {
    int code=Jim_Eval(i,"info patchlevel");
    int n; const unsigned char *s=(const unsigned char *)Jim_GetString(Jim_GetResult(i),&n);
    printf("VERSION|%d|",code); for(int k=0;k<n;k++)printf("%02x",s[k]); puts("");
}
int main(int argc,char **argv) {
    (void)argv; if(argc!=1)return 2;
    Jim_Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); version(i);
    Jim_Obj *head=Jim_NewStringObj(i,"alias",5);
    Jim_Obj *name=Jim_NewStringObj(i,"::r2286_original_prefix_alias",29);
    Jim_Obj *target=Jim_NewStringObj(i,"list",4);
    Jim_Obj *member=Jim_NewIntObj(i,17);
    Jim_IncrRefCount(head); Jim_IncrRefCount(name); Jim_IncrRefCount(target); Jim_IncrRefCount(member);
    printf("BEFORE|%s|%d\n",member->typePtr?member->typePtr->name:"none",member->bytes!=NULL);
    Jim_Obj *definition[4]={head,name,target,member};
    int code=Jim_EvalObjVector(i,4,definition);
    printf("DEFINITION|%d|%d|%s|%d\n",code,Jim_GetResult(i)==name,member->typePtr?member->typePtr->name:"none",member->bytes!=NULL);
    Jim_Obj *info=Jim_NewStringObj(i,"info",4); Jim_IncrRefCount(info);
    Jim_Obj *query[3]={info,head,name}; code=Jim_EvalObjVector(i,3,query);
    Jim_Obj *prefix=Jim_GetResult(i); Jim_IncrRefCount(prefix);
    Jim_Obj *stored_target=Jim_ListGetIndex(i,prefix,0);
    Jim_Obj *stored_member=Jim_ListGetIndex(i,prefix,1);
    printf("QUERY|%d|%s|%d|%d|%s|%d\n",code,prefix->typePtr?prefix->typePtr->name:"none",stored_target==target,stored_member==member,member->typePtr?member->typePtr->name:"none",member->bytes!=NULL);
    Jim_Obj *call[1]={name}; code=Jim_EvalObjVector(i,1,call);
    Jim_Obj *result=Jim_GetResult(i);
    printf("CALL|%d|%s|%d|%s|%d\n",code,result->typePtr?result->typePtr->name:"none",Jim_ListGetIndex(i,result,0)==member,member->typePtr?member->typePtr->name:"none",member->bytes!=NULL);
    Jim_DecrRefCount(i,prefix); Jim_DecrRefCount(i,info);
    Jim_DecrRefCount(i,head); Jim_DecrRefCount(i,name); Jim_DecrRefCount(i,target); Jim_DecrRefCount(i,member);
    Jim_FreeInterp(i); return 0;
}
#else
#include "tcl.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
int main(int argc,char **argv) {
    if(argc!=1)return 2; Tcl_FindExecutable(argv[0]);
    Tcl_Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK)return 3;
    int code=Tcl_EvalEx(i,"info patchlevel",15,0); Count n;
    const unsigned char *s=(const unsigned char *)Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n);
    printf("VERSION|%d|",code);for(Count k=0;k<n;k++)printf("%02x",s[k]);puts("");
    puts("CORE_ALIAS_PREFIX|NOT_APPLICABLE"); Tcl_DeleteInterp(i);return 0;
}
#endif
