#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
static const char *bodies[] = {
 "variable v [list VALUE] ${ns}v [list LATE]",
 "variable v [list VALUE] {a)} [list LATE]",
 "global [list ::N]::v ${ns}v",
 "global [list ::N]::v {a)}",
 "variable [list ::N]::v [list VALUE] ${ns}v [list LATE]", NULL
};
static void hex(const char *bytes, int length) {
 for (int n=0; n<length; n++) printf("%02x", (unsigned char)bytes[n]);
}
int main(int argc, char **argv) {
 Tcl_FindExecutable(argv[0]);
 puts("case\thook\tvariables\tnsupvars\tlocals\tliteral_count\tliterals");
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
#ifdef INST_VARIABLE
   if(*pc == INST_VARIABLE) variables++;
#endif
#ifdef INST_NSUPVAR
   if(*pc == INST_NSUPVAR) nsupvars++;
#endif
   pc += tclInstructionTable[*pc].numBytes;
  }
  Command *command=(Command *)Tcl_FindCommand(interp,(index==2 || index==3)?"global":"variable",NULL,TCL_GLOBAL_ONLY);
  printf("%d\t%d\t%d\t%d\t",index,command?command->compileProc!=NULL:-1,variables,nsupvars);
  for(CompiledLocal *local=proc->firstLocalPtr;local;local=local->nextPtr) {
   if(local != proc->firstLocalPtr) putchar(',');
   hex(local->name,(int)local->nameLength);
  }
  printf("\t%d\t", code ? code->numLitObjects : -1);
  if(code) for(int literal=0; literal<code->numLitObjects; literal++) {
   Tcl_Obj *value=code->objArrayPtr[literal];
   if(literal) putchar(',');
   const char *type=value->typePtr?value->typePtr->name:"none";
#if TCL_MAJOR_VERSION >= 9
   Tcl_Size length;
#else
   int length;
#endif
   const char *bytes=Tcl_GetStringFromObj(value,&length);
   printf("%s:",type);hex(bytes,(int)length);
  }
  putchar('\n');
  Tcl_DecrRefCount(definition);
  Tcl_DeleteInterp(interp);
 }
 Tcl_Finalize();
 return 0;
}
