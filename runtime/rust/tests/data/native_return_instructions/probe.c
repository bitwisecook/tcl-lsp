#include "tclInt.h"
#include <stdio.h>
#include <string.h>
static void put(Tcl_Obj *dict,const char *key,Tcl_Obj *value){Tcl_DictObjPut(NULL,dict,Tcl_NewStringObj(key,-1),value);}
int main(void){
 Tcl_FindExecutable("original-return-instructions");
 Tcl_Interp *interp=Tcl_CreateInterp();Interp *ip=(Interp*)interp;
 Tcl_Obj *member=Tcl_NewStringObj("ORIGINAL",-1);Tcl_IncrRefCount(member);
 Tcl_Obj *code=Tcl_NewListObj(1,&member);Tcl_IncrRefCount(code);
 Tcl_Obj *options=Tcl_NewDictObj();Tcl_IncrRefCount(options);put(options,"-errorcode",code);
 Tcl_Obj *result=Tcl_GetObjResult(interp);
 int outcome=TclProcessReturn(interp,TCL_ERROR,2,options);
 printf("immediate\t%d\t%d\t%d\t%d\t%d\t%d\t%d\n",outcome,options->refCount,ip->returnOpts==options,ip->errorCode==code,code->bytes==NULL,ip->returnLevel,ip->returnCode);
 outcome=TclProcessReturn(interp,TCL_ERROR,0,options);
 printf("syntax\t%d\t%d\t%d\t%d\t%d\t%d\n",outcome,options->refCount,ip->returnOpts==options,ip->errorCode==code,code->bytes==NULL,Tcl_GetObjResult(interp)==result);
 Tcl_Obj *line=Tcl_NewStringObj("7",1);Tcl_IncrRefCount(line);Tcl_Obj *lineopts=Tcl_NewDictObj();Tcl_IncrRefCount(lineopts);put(lineopts,"-errorcode",code);put(lineopts,"-errorline",line);
 outcome=TclProcessReturn(interp,TCL_ERROR,0,lineopts);
 printf("line\t%d\t%d\t%s\t%d\n",outcome,ip->errorLine,line->typePtr?line->typePtr->name:"none",line->bytes!=NULL);
 Tcl_Obj *bad=Tcl_NewStringObj("bad",3);Tcl_IncrRefCount(bad);
 Tcl_Obj *failed=Tcl_NewDictObj();Tcl_IncrRefCount(failed);put(failed,"-errorline",bad);
 outcome=TclProcessReturn(interp,TCL_ERROR,0,failed);
 printf("line-failure\t%d\t%d\t%s\t%d\t%d\n",outcome,ip->errorLine,bad->typePtr?bad->typePtr->name:"none",Tcl_GetObjResult(interp)==result,ip->returnOpts==failed);
#if TCL_MAJOR_VERSION>8 || TCL_MINOR_VERSION>=6
 Tcl_Obj *old=ip->errorStack;Tcl_IncrRefCount(old);
 Tcl_Obj *stackopts=Tcl_NewDictObj();Tcl_IncrRefCount(stackopts);put(stackopts,"-errorstack",old);
 outcome=TclProcessReturn(interp,TCL_ERROR,0,stackopts);
 printf("stack-alias\t%d\t%d\t%d\t%d\n",outcome,ip->errorStack!=old,old->refCount,ip->resetErrorStack);
 Tcl_DecrRefCount(stackopts);Tcl_DecrRefCount(old);
#endif
 Tcl_ResetResult(interp);Tcl_SetObjResult(interp,Tcl_NewStringObj("SYNTAX",-1));Tcl_SetErrorCode(interp,"NONE",(char*)NULL);
 Tcl_Obj *message=Tcl_GetObjResult(interp);Tcl_IncrRefCount(message);Tcl_Obj *captured=Tcl_GetReturnOptions(interp,TCL_ERROR);Tcl_IncrRefCount(captured);
#if TCL_MAJOR_VERSION>8 || TCL_MINOR_VERSION>=6
 TclNoErrorStack(interp,captured);
#endif
 Tcl_Obj *capturedCode,*capturedInfo,*control,*level,*stack;
 Tcl_DictObjGet(NULL,captured,Tcl_NewStringObj("-errorcode",-1),&capturedCode);
 Tcl_DictObjGet(NULL,captured,Tcl_NewStringObj("-errorinfo",-1),&capturedInfo);
 Tcl_DictObjGet(NULL,captured,Tcl_NewStringObj("-code",-1),&control);
 Tcl_DictObjGet(NULL,captured,Tcl_NewStringObj("-level",-1),&level);
 Tcl_DictObjGet(NULL,captured,Tcl_NewStringObj("-errorstack",-1),&stack);
 printf("capture\t%d\t%d\t%s\t%d\t%s\t%d\t%d\t%d\n",capturedInfo==message,capturedCode==ip->errorCode,control->typePtr?control->typePtr->name:"none",control->bytes==NULL,level->typePtr?level->typePtr->name:"none",level->bytes==NULL,stack==NULL,capturedCode->bytes==NULL);
 Tcl_ResetResult(interp);printf("capture-reset\t%d\t%d\t%d\t%d\n",ip->errorInfo==NULL,ip->errorCode==NULL,capturedInfo==message,message->refCount);
 Tcl_DecrRefCount(captured);Tcl_DecrRefCount(message);
 Tcl_DecrRefCount(options);Tcl_DecrRefCount(lineopts);Tcl_DecrRefCount(failed);Tcl_DecrRefCount(line);Tcl_DecrRefCount(bad);Tcl_DecrRefCount(code);Tcl_DecrRefCount(member);
 Tcl_DeleteInterp(interp);Tcl_Finalize();return 0;
}
