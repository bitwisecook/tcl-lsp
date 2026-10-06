#include "tclInt.h"
#include <stdio.h>
#if TCL_MAJOR_VERSION >= 9
#define Count Tcl_Size
#else
#define Count int
#endif
static int canonical(Tcl_Obj *o) {
 if (!o->typePtr || strcmp(o->typePtr->name,"list")) return -1;
#if TCL_MAJOR_VERSION >= 9
 return !!(ListObjStorePtr(o)->flags & LISTSTORE_CANONICAL);
#elif TCL_MINOR_VERSION >= 5
 return ListRepPtr(o)->canonicalFlag;
#else
 return 0;
#endif
}
static int probe(ClientData data,Tcl_Interp *i,Count objc,Tcl_Obj *const objv[]) {
    const char *names[]={"v","d","n\0(k)"};
    int k;
    for(k=0;k<3;k++) {
        Tcl_Obj *element=Tcl_NewStringObj(names[k],k==2?5:1);
        Tcl_Obj *local=k==2?element:Tcl_NewListObj(1,&element);
        Tcl_Obj *a[4]; int j,code;
        Tcl_IncrRefCount(local);
        printf("before%d|%s|%d|%d\n",k,local->typePtr?local->typePtr->name:"NULL",local->refCount,canonical(local));
        a[0]=Tcl_NewStringObj("upvar",-1);a[1]=Tcl_NewStringObj("#0",-1);a[2]=Tcl_NewStringObj("x",-1);a[3]=local;
        for(j=0;j<3;j++)Tcl_IncrRefCount(a[j]);
        code=Tcl_EvalObjv(i,4,a,0);
        printf("after%d|%d|%s|%d|%d\n",k,code,local->typePtr?local->typePtr->name:"NULL",local->refCount,canonical(local));
        for(j=0;j<3;j++)Tcl_DecrRefCount(a[j]);
        Tcl_DecrRefCount(local);
        if(code!=TCL_OK)return code;
    }
    Tcl_ResetResult(i);return TCL_OK;
}
int main(int argc,char **argv) {
    Tcl_Interp *i;int code;
    Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();
#if TCL_MAJOR_VERSION >= 9
    Tcl_CreateObjCommand2(i,"probe",probe,NULL,NULL);
#else
    Tcl_CreateObjCommand(i,"probe",probe,NULL,NULL);
#endif
    code=Tcl_Eval(i,"set x X; proc p {} {probe; return $v}; p");
    printf("completion|%d|%s\n",code,Tcl_GetStringResult(i));
    Tcl_DeleteInterp(i);Tcl_Finalize();return code;
}
