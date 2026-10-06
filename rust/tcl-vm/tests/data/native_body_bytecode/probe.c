#include <stdio.h>
#include <tcl.h>
int main(void) {
 const char *sources[]={"{", "set x X; {", "return OK; {", "observe", ""};
 for(int n=0;n<5;n++) {
  Tcl_Interp *i=Tcl_CreateInterp(); Tcl_Obj *o=Tcl_NewStringObj(sources[n],-1); Tcl_IncrRefCount(o);
  printf("before\t%d\t%s\t%d\t%d\n",n,o->typePtr?o->typePtr->name:"none",o->refCount,o->bytes!=NULL);
  int code=Tcl_EvalObjEx(i,o,0);
  printf("after\t%d\t%d\t%s\t%d\t%d\n",n,code,o->typePtr?o->typePtr->name:"none",o->refCount,o->bytes!=NULL);
  Tcl_DecrRefCount(o); Tcl_DeleteInterp(i);
 }
 return 0;
}
