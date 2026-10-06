#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
static const char *bodies[] = {
 "switch -exact -- miss x {set kept 1} x {set masked 2} default {set done 3}",
 "switch -exact -- miss {x {set kept 1} x {set masked 2} default {set done 3}}",
 "switch -glob -- miss {x {set kept 1} x {set masked 2} default {set done 3}}",
 "switch -- miss {x {set kept 1} y - x {set forced 2} default {set done 3}}",
 "switch -- miss {*}{x {set kept 1} x {set masked 2} default {set done 3}}",
 "switch -- [set visited miss] {x {set kept 1} x {set masked 2} default {set done 3}}", NULL
};
static void hex(const char *s,int n){for(int i=0;i<n;i++)printf("%02x",(unsigned char)s[i]);}
int main(int argc,char **argv){
 Tcl_FindExecutable(argv[0]); puts("case\tcode\tresult\tlocals");
 for(int i=0;bodies[i];i++){
  Tcl_Interp *interp=Tcl_CreateInterp();
  Tcl_Obj *definition=Tcl_NewStringObj("proc p {} {",-1); Tcl_IncrRefCount(definition);
  Tcl_AppendToObj(definition,bodies[i],-1); Tcl_AppendToObj(definition,"}",-1);
  if(Tcl_EvalObjEx(interp,definition,0)!=TCL_OK)return 2;
  int code=Tcl_Eval(interp,"p");
#if TCL_MAJOR_VERSION >= 9
  Tcl_Size length;
#else
  int length;
#endif
  const char *result=Tcl_GetStringFromObj(Tcl_GetObjResult(interp),&length);
  printf("%d\t%d\t",i,code);hex(result,(int)length);putchar('\t');
  Proc *proc=TclFindProc((Interp*)interp,"p"); if(!proc)return 3;
  for(CompiledLocal *local=proc->firstLocalPtr;local;local=local->nextPtr){
   if(local!=proc->firstLocalPtr)putchar(',');hex(local->name,(int)local->nameLength);
  }
  putchar('\n');Tcl_DecrRefCount(definition);Tcl_DeleteInterp(interp);
 }
 Tcl_Finalize();return 0;
}
