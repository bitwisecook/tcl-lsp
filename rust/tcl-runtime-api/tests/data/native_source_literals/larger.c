#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
static int width=2,sourceIndex=1;
static int worker(ClientData cd,Tcl_Interp *i,int objc,Tcl_Obj *const objv[]){return TCL_OK;}
static int compile(Tcl_Interp *i,Tcl_Parse *p,
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
Command *cmd,
#endif
CompileEnv *e){
 int j,index; char text[32];
 for(j=0;j<width;j++){
  if(j==sourceIndex)index=TclRegisterLiteral(e,(char *)e->source,e->numSrcBytes,0);
  else {sprintf(text,"extra-%d",j);index=TclRegisterLiteral(e,text,(int)strlen(text),0);}
  TclEmitPush(index,e);
  if(j+1<width){TclEmitOpcode(INST_POP,e);}
 }
 return TCL_OK;
}
int main(int argc,char **argv){
 Tcl_Interp *i;Tcl_Obj *donor,*source;ByteCode *first,*second;int code,j;
 if(argc>1)width=atoi(argv[1]);if(argc>2)sourceIndex=atoi(argv[2]);
 Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();
 ((Command *)Tcl_CreateObjCommand(i,"seed",worker,NULL,NULL))->compileProc=compile;
 donor=Tcl_NewStringObj("seed",4);Tcl_IncrRefCount(donor);
 code=Tcl_ConvertToType(i,donor,&tclByteCodeType);if(code!=TCL_OK)return 2;
 first=(ByteCode *)donor->internalRep.twoPtrValue.ptr1;
 source=first->objArrayPtr[sourceIndex];Tcl_IncrRefCount(source);
 printf("before\trefs=%d\ttype=%s\n",source->refCount,source->typePtr?source->typePtr->name:"none");
 code=Tcl_ConvertToType(i,source,&tclByteCodeType);if(code!=TCL_OK)return 3;
 second=(ByteCode *)source->internalRep.twoPtrValue.ptr1;
 printf("after\trefs=%d\tcount=%d",source->refCount,second->numLitObjects);
 for(j=0;j<second->numLitObjects;j++)printf("\tself%d=%d",j,second->objArrayPtr[j]==source);
 puts("");fflush(stdout);Tcl_DeleteInterp(i);
 printf("after-delete\trefs=%d\ttype=%s\n",source->refCount,source->typePtr?source->typePtr->name:"none");fflush(stdout);
 Tcl_DecrRefCount(source);Tcl_DecrRefCount(donor);puts("released-external-originals");return 0;
}
