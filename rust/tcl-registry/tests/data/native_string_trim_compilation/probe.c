#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
static void header(Tcl_Obj *o) { printf("%s,%d,%d", o->typePtr ? o->typePtr->name : "none",o->bytes != NULL,o->refCount); }
int main(int argc,char **argv) {
 Tcl_FindExecutable(argv[0]); Tcl_Interp *ip=Tcl_CreateInterp();
 const char *members[]={"trim","trimleft","trimright"};
 const unsigned char *input[]={(const unsigned char*)" x ",(const unsigned char*)"x",(const unsigned char*)"",(const unsigned char*)"\0x\0",(const unsigned char*)"\xffx\xff",(const unsigned char*)"--x--"};
 const int sizes[]={3,1,0,3,3,5};
 for(int mode=0;mode<2;mode++)for(int member=0;member<3;member++)for(int n=0;n<6;n++) {
  char source[256]; snprintf(source,sizeof(source),"proc p {s c} {string %s $s%s}",members[member], n==5?" $c":"");
  if(Tcl_EvalEx(ip,source,-1,TCL_EVAL_GLOBAL)!=TCL_OK)return 2;
  Tcl_Obj *subject=Tcl_NewByteArrayObj(input[n],sizes[n]),*chars=Tcl_NewStringObj("-",1);
  Tcl_Obj *v[4]={Tcl_NewStringObj(mode?"p":"string",-1),mode?subject:Tcl_NewStringObj(members[member],-1),mode?chars:subject,chars};
  int count=mode?3:(n==5?4:3);for(int i=0;i<count;i++)Tcl_IncrRefCount(v[i]);
  int rc=Tcl_EvalObjv(ip,count,v,TCL_EVAL_GLOBAL);Tcl_Obj *r=Tcl_GetObjResult(ip);
  printf("window|%d|%s|%d|%d|",mode,members[member],n,rc);header(r);printf("|%d|",r==subject);header(subject);printf("|");header(chars);printf("|");
#if TCL_MAJOR_VERSION >= 9
  Tcl_Size length;
#else
  int length;
#endif
  const unsigned char *bytes=(const unsigned char*)Tcl_GetStringFromObj(r,&length);for(int i=0;i<length;i++)printf("%02x",bytes[i]);puts("");
  if(mode) { Proc *proc=TclFindProc((Interp*)ip,"p"); ByteCode *code;
#ifdef ByteCodeGetInternalRep
   ByteCodeGetInternalRep(proc->bodyPtr,&tclByteCodeType,code);
#else
   code=(ByteCode*)proc->bodyPtr->internalRep.otherValuePtr;
#endif
   printf("code|%s|%d",members[member],n);for(int off=0;off<code->numCodeBytes;) {const InstructionDesc *d=&tclInstructionTable[code->codeStart[off]];printf("|%s",d->name);off+=d->numBytes;}puts("");
  }
  for(int i=0;i<count;i++)Tcl_DecrRefCount(v[i]);
 }
 Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;
}
