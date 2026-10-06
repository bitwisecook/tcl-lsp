#include "tcl.h"
#include <stdio.h>
static void obs(const char *phase,Tcl_Obj *o) {
 printf("%s|%s|%d|%d\n",phase,o->typePtr?o->typePtr->name:"none",o->typePtr&&o->typePtr->freeIntRepProc!=NULL,o->bytes!=NULL);
}
int main(void) {
 Tcl_Interp *i=Tcl_CreateInterp();
 if(Tcl_EvalEx(i,"array set a {k v}; array startsearch a",-1,0)!=TCL_OK)return 2;
 Tcl_Obj *h=Tcl_GetObjResult(i);Tcl_IncrRefCount(h);Tcl_ResetResult(i);
 Tcl_Obj *av[4]={Tcl_NewStringObj("array",-1),Tcl_NewStringObj("anymore",-1),Tcl_NewStringObj("a",-1),h};
 for(int n=0;n<3;n++)Tcl_IncrRefCount(av[n]);
 if(Tcl_EvalObjv(i,4,av,0)!=TCL_OK)return 3;
 for(int n=0;n<3;n++)Tcl_DecrRefCount(av[n]);Tcl_ResetResult(i);
 obs("converted",h);
 Tcl_Obj *copy=Tcl_DuplicateObj(h);Tcl_IncrRefCount(copy);obs("duplicate",copy);
 Tcl_ObjGetVar2(i,h,NULL,0);obs("missing",h);
 Tcl_ObjGetVar2(i,copy,NULL,0);obs("duplicate-missing",copy);
 Tcl_DecrRefCount(copy);Tcl_DecrRefCount(h);Tcl_DeleteInterp(i);return 0;
}
