#include <stdio.h>
#include <stddef.h>
#include <string.h>
#include "tcl.h"
#include "tclInt.h"
static const char*type(Tcl_Obj*o){return !o?"null":o->typePtr?o->typePtr->name:"string";}
static Var*cell(Interp*i,const char*n){
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
Tcl_HashEntry*h=Tcl_FindHashEntry(&i->globalNsPtr->varTable,n);return h?Tcl_GetHashValue(h):NULL;
#else
Tcl_Obj*k=Tcl_NewStringObj(n,-1);Tcl_IncrRefCount(k);Tcl_HashEntry*h=Tcl_FindHashEntry(&i->globalNsPtr->varTable.table,(const char*)k);Tcl_DecrRefCount(k);return h?(Var*)((char*)h - offsetof(VarInHash,entry)):NULL;
#endif
}
static void report(Tcl_Interp*ip,const char*label,Tcl_Obj*info,Tcl_Obj*code){Interp*i=(Interp*)ip;
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
Tcl_Obj*pi=NULL,*pc=NULL;
#else
Tcl_Obj*pi=i->errorInfo,*pc=i->errorCode;
#endif
printf("%s\tprivateinfo=%s\tprivatecode=%s\tinfo_same=%d\tcode_same=%d\tinfo_refs=%d\tcode_refs=%d",label,type(pi),type(pc),pi==info,pc==code,info?info->refCount:-1,code?code->refCount:-1);
for(int k=0;k<2;k++){Var*v=cell(i,k?"errorCode":"errorInfo");Tcl_Obj*o=v&&!TclIsVarUndefined(v)&&!TclIsVarArray(v)&&!TclIsVarLink(v)?v->value.objPtr:NULL;printf("\tglobal%d=%s\tglobal%d_same=%d",k,type(o),k,o==(k?code:info));}puts("");}
static char*observe(ClientData data,Tcl_Interp*ip,const char*n1,const char*n2,int f){(void)data;(void)n2;(void)f;Interp*i=(Interp*)ip;
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
(void)i;printf("trace-%s\tlegacy=-1\n",n1);
#else
const char*key=n1;while(*key==':')key++;Tcl_Obj*o=strcmp(key,"errorCode")==0?i->errorCode:i->errorInfo;Var*v=cell(i,key);printf("trace-%s\tprivate=%s\tprivate_refs=%d\tglobal_same=%d\n",n1,type(o),o?o->refCount:-1,v&&!TclIsVarUndefined(v)&&v->value.objPtr==o);
#endif
return NULL;}
static int observe_obj(ClientData data,Tcl_Interp*ip,int objc,Tcl_Obj*const objv[]){ (void)objc;(void)objv; observe(data,ip,Tcl_GetString(objv[1]),NULL,0);return TCL_OK;}
int main(int argc,char**argv){(void)argc;Tcl_FindExecutable(argv[0]);Tcl_Interp*ip=Tcl_CreateInterp();Tcl_SetErrorCode(ip,"NONE",NULL);report(ip,"default-code",NULL,NULL);Tcl_ResetResult(ip);
unsigned char bytes[]={0xff,0,0x61};Tcl_Obj*info=Tcl_NewByteArrayObj(bytes,3),*code=Tcl_NewByteArrayObj(bytes,3);Tcl_IncrRefCount(info);Tcl_IncrRefCount(code);Tcl_Obj*args[]={Tcl_NewStringObj("error",-1),Tcl_NewStringObj("message",-1),info,code};Tcl_IncrRefCount(args[0]);Tcl_IncrRefCount(args[1]);Tcl_CreateObjCommand(ip,"observe",observe_obj,NULL,NULL); Tcl_EvalEx(ip,"trace add variable ::errorCode write observe; trace add variable ::errorInfo write observe",-1,0);Tcl_CmdInfo cmd;Tcl_GetCommandInfo(ip,"error",&cmd);int rc=cmd.objProc(cmd.objClientData,ip,4,args);printf("error_code=%d\n",rc);report(ip,"explicit-error",info,code);Tcl_ResetResult(ip);report(ip,"after-reset",info,code);Tcl_DecrRefCount(args[0]);Tcl_DecrRefCount(args[1]);Tcl_DecrRefCount(info);Tcl_DecrRefCount(code);Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;}
