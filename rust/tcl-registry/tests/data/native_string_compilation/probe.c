#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
static Tcl_Obj *fresh_pattern;
static int fresh_cmd(ClientData c,Tcl_Interp *ip,int objc,Tcl_Obj *const objv[]) {fresh_pattern=Tcl_NewStringObj("*",-1);Tcl_SetObjResult(ip,fresh_pattern);return TCL_OK;}
int main(int argc,char **argv) {
 Tcl_FindExecutable(argv[0]); Tcl_Interp *ip=Tcl_CreateInterp();
 const char *bodies[]={"string match * $subject","string match A $subject","string match {]} $subject","string match -n A $subject","string match - A $subject","string match -nocaseX A $subject","string match $flag A $subject","string match $pattern $subject","string match \\-n A $subject","string match {*}\"A\" $subject","string match {*}{* X}","string match A"};
 for(int n=0;n<12;n++) {
  char source[1024]; snprintf(source,sizeof(source),"proc p {pattern subject flag} {%s}",bodies[n]);
  int rc=Tcl_EvalEx(ip,source,-1,TCL_EVAL_GLOBAL); if(rc){printf("setup|%d|%d\n",n,rc);continue;}
  Tcl_Obj *v[4]={Tcl_NewStringObj("p",-1),Tcl_NewStringObj("*",-1),Tcl_NewStringObj("A",-1),Tcl_NewStringObj("-n",-1)};
  for(int i=0;i<4;i++)Tcl_IncrRefCount(v[i]);
  rc=Tcl_EvalObjv(ip,4,v,TCL_EVAL_GLOBAL);Tcl_Obj *r=Tcl_GetObjResult(ip);
  int shared=r->refCount, resident=r->bytes!=NULL, same=r==v[1]; const char *type=r->typePtr?r->typePtr->name:"none";
  printf("result|%d|%d|%s|%d|%d|%d|%s\n",n,rc,type,resident,shared,same,Tcl_GetStringResult(ip));
  Proc *proc=TclFindProc((Interp*)ip,"p"); ByteCode *code;
#ifdef ByteCodeGetInternalRep
  ByteCodeGetInternalRep(proc->bodyPtr, &tclByteCodeType, code);
#else
  code=(ByteCode*)proc->bodyPtr->internalRep.otherValuePtr;
#endif
  printf("code|%d",n);if(proc->bodyPtr->typePtr && !strcmp(proc->bodyPtr->typePtr->name,"bytecode") && code)for(int off=0;off<code->numCodeBytes;){unsigned int op=code->codeStart[off]; const InstructionDesc *d=&tclInstructionTable[op];printf("|%s",d->name);if(!d->numBytes)break;off+=d->numBytes;}puts("");
  for(int i=0;i<4;i++)Tcl_DecrRefCount(v[i]);
 }
 Tcl_CreateObjCommand(ip,"freshpattern",fresh_cmd,NULL,NULL);
 Tcl_EvalEx(ip,"proc freshmatch {} {string match [freshpattern] A}",-1,TCL_EVAL_GLOBAL);
 int rc=Tcl_EvalEx(ip,"freshmatch",-1,TCL_EVAL_GLOBAL); Tcl_Obj *r=Tcl_GetObjResult(ip);
 int resident=r->bytes!=NULL, refs=r->refCount, same=r==fresh_pattern; const char *type=r->typePtr?r->typePtr->name:"none";
 printf("fresh|%d|%s|%d|%d|%d|%s\n",rc,type,resident,refs,same,Tcl_GetStringResult(ip));
 Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;
}
