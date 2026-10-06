#include <stdio.h>
#include "tcl.h"
int main(int argc,char **argv) {
    Tcl_FindExecutable(argv[0]); Tcl_Interp *i=Tcl_CreateInterp(); int parsed;
    int code=Tcl_EvalEx(i,"package provide probe 12; package provide probe",-1,TCL_EVAL_GLOBAL);
    Tcl_Obj *first=Tcl_GetObjResult(i); Tcl_IncrRefCount(first);
    printf("created\t%d\t%s\t%d\n",code,first->typePtr?first->typePtr->name:"none",first->refCount);
    code=Tcl_GetIntFromObj(i,first,&parsed);
    printf("converted\t%d\t%d\t%s\n",code,parsed,first->typePtr?first->typePtr->name:"none");
    code=Tcl_EvalEx(i,"package provide probe",-1,TCL_EVAL_GLOBAL); Tcl_Obj *next=Tcl_GetObjResult(i);
    printf("queried\t%d\t%d\t%s\t%d\n",code,next==first,next->typePtr?next->typePtr->name:"none",first->refCount);
    code=Tcl_EvalEx(i,"package require probe",-1,TCL_EVAL_GLOBAL);next=Tcl_GetObjResult(i);
    printf("required\t%d\t%d\t%s\t%d\n",code,next==first,next->typePtr?next->typePtr->name:"none",first->refCount);
    code=Tcl_EvalEx(i,"package present probe",-1,TCL_EVAL_GLOBAL);next=Tcl_GetObjResult(i);
    printf("present\t%d\t%d\t%s\t%d\n",code,next==first,next->typePtr?next->typePtr->name:"none",first->refCount);
    code=Tcl_EvalEx(i,"package forget probe",-1,TCL_EVAL_GLOBAL);
    printf("forgotten\t%d\t%d\t%s\n",code,first->refCount,first->typePtr?first->typePtr->name:"none");
    Tcl_DecrRefCount(first);Tcl_DeleteInterp(i);Tcl_Finalize();return 0;
}
