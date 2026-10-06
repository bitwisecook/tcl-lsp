#include "tcl.h"
#include "tclInt.h"
#include <stdio.h>
#include <string.h>
static Tcl_Obj *keyK,*keyJ;
static Var *oldK,*oldJ;
static int sequence;
static Var *element(Tcl_Interp *i,const char *key) {
 Var *a=(Var*)Tcl_FindNamespaceVar(i,"arr",NULL,0);
 if(!a || TclIsVarUndefined(a)||!TclIsVarArray(a))return NULL;
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
 Tcl_HashEntry *h=Tcl_FindHashEntry(a->value.tablePtr,key);
#else
 Tcl_Obj *obj=Tcl_NewStringObj(key,-1);Tcl_IncrRefCount(obj);
 Tcl_HashEntry *h=Tcl_FindHashEntry(&a->value.tablePtr->table,(char*)obj);Tcl_DecrRefCount(obj);
#endif
 return h?(Var*)Tcl_GetHashValue(h):NULL;
}
static void observe(Tcl_Interp *i,const char *phase) {
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
 int kd=oldK->hPtr==NULL,jd=oldJ->hPtr==NULL,kr=oldK->refCount,jr=oldJ->refCount;
#else
 int kd=!!TclIsVarDeadHash(oldK),jd=!!TclIsVarDeadHash(oldJ),kr=((VarInHash*)oldK)->refCount,jr=((VarInHash*)oldJ)->refCount;
#endif
 printf("window|%s|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d\n",phase,keyK->refCount,keyJ->refCount,kd,jd,!TclIsVarUndefined(oldK),!TclIsVarUndefined(oldJ),kr,jr,element(i,"k")!=NULL,element(i,"j")!=NULL);
}
static char *trace(ClientData d,Tcl_Interp*i,const char*n,const char*e,int flags) {
 (void)d;(void)n;(void)flags;
 char phase[64];
 if(e) snprintf(phase,sizeof phase,"element%d-%s",++sequence,e);else strcpy(phase,"root");
 observe(i,phase);return NULL;
}
int main(void) {
 Tcl_FindExecutable("probe");Tcl_Interp*i=Tcl_CreateInterp();
 Tcl_Obj *root=Tcl_NewStringObj("arr",-1),*value=Tcl_NewStringObj("VALUE",-1);
 keyK=Tcl_NewStringObj("k",-1);keyJ=Tcl_NewStringObj("j",-1);
 Tcl_IncrRefCount(root);Tcl_IncrRefCount(value);Tcl_IncrRefCount(keyK);Tcl_IncrRefCount(keyJ);
 if(!Tcl_ObjSetVar2(i,root,keyK,value,0)||!Tcl_ObjSetVar2(i,root,keyJ,value,0))return 2;
 oldK=element(i,"k");oldJ=element(i,"j");
 if(Tcl_UpVar(i,"#0","arr(k)","ak",TCL_GLOBAL_ONLY)||Tcl_UpVar(i,"#0","arr(j)","aj",TCL_GLOBAL_ONLY))return 3;
 if(Tcl_TraceVar2(i,"arr",NULL,TCL_TRACE_UNSETS,trace,NULL)||Tcl_TraceVar2(i,"arr","k",TCL_TRACE_UNSETS,trace,NULL)||Tcl_TraceVar2(i,"arr","j",TCL_TRACE_UNSETS,trace,NULL))return 4;
 observe(i,"before");int code=Tcl_UnsetVar(i,"arr",TCL_GLOBAL_ONLY);printf("unset|%d\n",code);observe(i,"after");
 Tcl_DeleteInterp(i);Tcl_DecrRefCount(root);Tcl_DecrRefCount(value);Tcl_DecrRefCount(keyK);Tcl_DecrRefCount(keyJ);Tcl_Finalize();return code;
}
