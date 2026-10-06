#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
static const char *bodies[] = {
 "variable v", "variable {}", "variable :::", "variable {a)}",
 "variable v $val w [set x 3]", "variable v 1 {a)} 2",
 "variable ${ns}::v", "variable ${ns}v", "variable ${ns}::[set x 1]",
 "variable ${ns}::v 1 ${ns}v 2", "variable {*}{v 1}", "variable",
 "global v", "global {}", "global :::", "global {a)}",
 "global ${ns}::v", "global ${ns}v", "global v {a)}", "global {*}{v w}", NULL
};
static void hex(const char *bytes, int length) {
 for (int n=0; n<length; n++) printf("%02x", (unsigned char)bytes[n]);
}
int main(int argc, char **argv) {
 Tcl_FindExecutable(argv[0]);
 puts("case\thook\tvariables\tnsupvars\tlocals");
 for (int index=0; bodies[index]; index++) {
  Tcl_Interp *interp = Tcl_CreateInterp();
  Tcl_Obj *definition = Tcl_NewStringObj("namespace eval N {}; proc p {ns val} {",-1);
  Tcl_IncrRefCount(definition);
  Tcl_AppendToObj(definition,bodies[index],-1);
  Tcl_AppendToObj(definition,"}; catch {p ::N 7}",-1);
  if(Tcl_EvalObjEx(interp,definition,0)!=TCL_OK) return 2;
  Proc *proc = TclFindProc((Interp *)interp,"p");
  if(!proc) return 3;
  int compiled=proc->bodyPtr->typePtr && !strcmp(proc->bodyPtr->typePtr->name,"bytecode");
  ByteCode *code=compiled?(ByteCode *)proc->bodyPtr->internalRep.twoPtrValue.ptr1:NULL;
  int variables=compiled?0:-1, nsupvars=compiled?0:-1;
  for(unsigned char *pc=code?code->codeStart:NULL; code && pc < code->codeStart+code->numCodeBytes; ) {
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
   if(*pc == INST_VARIABLE) variables++;
   if(*pc == INST_NSUPVAR) nsupvars++;
#endif
   pc += tclInstructionTable[*pc].numBytes;
  }
  Command *command=(Command *)Tcl_FindCommand(interp,index<12?"variable":"global",NULL,TCL_GLOBAL_ONLY);
  printf("%d\t%d\t%d\t%d\t",index,command?command->compileProc!=NULL:-1,variables,nsupvars);
  for(CompiledLocal *local=proc->firstLocalPtr;local;local=local->nextPtr) {
   if(local != proc->firstLocalPtr) putchar(',');
   hex(local->name,(int)local->nameLength);
  }
  putchar('\n');
  Tcl_DecrRefCount(definition);
  Tcl_DeleteInterp(interp);
 }
 Tcl_Finalize();
 return 0;
}
