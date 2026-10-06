#include "tclInt.h"
#include <stdio.h>
static void state(int id,const char *window,Tcl_Obj *o,ListStore *origin){
    ListRep r; ListObjGetRep(o,&r);
    printf("S|%d|%s|%d|%d|%lld|%lld|%lld|%zu|%d|%lld|%lld|%zu|%d\n",id,window,o->refCount,o->bytes!=NULL,
        (long long)r.storePtr->firstUsed,(long long)r.storePtr->numUsed,(long long)r.storePtr->numAllocated,r.storePtr->refCount,
        r.spanPtr!=NULL,r.spanPtr?(long long)r.spanPtr->spanStart:-1,r.spanPtr?(long long)r.spanPtr->spanLength:-1,
        r.spanPtr?r.spanPtr->refCount:0,r.storePtr==origin);
}
int main(int argc,char **argv){
    Tcl_FindExecutable(argv[0]); Tcl_Interp *ip=Tcl_CreateInterp(); int id=0;
    const int n[]={100,101,202,203,300,301};
    for(int ni=0;ni<6;ni++)for(int excess=0;excess<2;excess++)for(int sharing=0;sharing<3;sharing++)for(int shape=0;shape<4;shape++,id++){
        int used=n[ni]; int cap=excess?used*3:used;
        Tcl_Obj *original=Tcl_NewListObj(cap,NULL);Tcl_IncrRefCount(original);
        for(int i=0;i<used;i++) if(Tcl_ListObjAppendElement(ip,original,Tcl_NewIntObj(i))!=TCL_OK)return 2;
        Tcl_Obj *duplicate=NULL;
        if(sharing==1)Tcl_IncrRefCount(original);
        if(sharing==2){duplicate=Tcl_DuplicateObj(original);Tcl_IncrRefCount(duplicate);}
        ListRep before;ListObjGetRep(original,&before);ListStore *origin=before.storePtr;
        int first=shape==0?0:shape==1?0:shape==2?used/4:used/3;
        int last=shape==0?used-1:shape==1?100:shape==2?3*used/4-1:used/3+99;
        if(last>=used)last=used-1;
        printf("C|%d|%d|%d|%d|%d|%d\n",id,used,cap,sharing,first,last);
        state(id,"before",original,origin);
        Tcl_Obj *result=TclListObjRange(ip,original,first,last);if(!result)return 3;
        printf("R|%d|%d\n",id,result==original);
        /* Real public result publication owns the returned object. No child observer pins. */
        Tcl_SetObjResult(ip,result);
        state(id,"after-original",original,origin);state(id,"result",result,origin);
        Tcl_ResetResult(ip);if(duplicate)Tcl_DecrRefCount(duplicate);
        if(sharing==1)Tcl_DecrRefCount(original);Tcl_DecrRefCount(original);
    }
    Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;
}
