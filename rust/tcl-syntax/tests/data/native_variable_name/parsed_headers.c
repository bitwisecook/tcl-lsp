#include "tcl.h"
#include "tclInt.h"
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
static void observe(const char *name,const char *phase,Tcl_Obj *o) {
 printf("%s|%s|%s|%d|%d|%d|",name,phase,o->typePtr?o->typePtr->name:"none",o->refCount,o->typePtr&&o->typePtr->freeIntRepProc!=NULL,o->bytes!=NULL);
 if(o->bytes){for(Len k=0;k<o->length;k++)printf("%02x",(unsigned char)o->bytes[k]);}puts("");
}
static void run(Tcl_Interp *i,const char *name,Tcl_Obj *o) {
 Tcl_IncrRefCount(o);observe(name,"before",o); Tcl_ObjGetVar2(i,o,NULL,0);observe(name,"missing",o);
 Tcl_Obj *v=Tcl_NewStringObj("VALUE",-1);Tcl_IncrRefCount(v);Tcl_ObjSetVar2(i,o,NULL,v,0);observe(name,"stored",o);
 Tcl_ObjGetVar2(i,o,NULL,0);observe(name,"read",o); Tcl_Obj *copy=Tcl_DuplicateObj(o);Tcl_IncrRefCount(copy);observe(name,"duplicate",copy);Tcl_DecrRefCount(copy);
 Tcl_EvalEx(i,"unset -nocomplain missing 42 arr",-1,0);Tcl_ObjGetVar2(i,o,NULL,0);observe(name,"after-unset-missing",o);
 Tcl_DecrRefCount(v);Tcl_DecrRefCount(o);Tcl_ResetResult(i);
}
int main(void){
 Tcl_Interp *i=Tcl_CreateInterp();
 run(i,"string",Tcl_NewStringObj("missing",-1));
 run(i,"int",Tcl_NewIntObj(42));
 Tcl_Obj *part=Tcl_NewStringObj("missing",-1);run(i,"list",Tcl_NewListObj(1,&part));
 run(i,"bytearray",Tcl_NewByteArrayObj((unsigned char*)"missing",7));
 const char array[]={'a','r','r','(','k',0,'z',')'};run(i,"array",Tcl_NewStringObj(array,sizeof(array)));
 Tcl_DeleteInterp(i);return 0;
}
