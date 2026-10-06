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
int main(int argc,char**argv){(void)argc;Tcl_FindExecutable(argv[0]);Tcl_Interp*ip=Tcl_CreateInterp();
unsigned char bytes[]={0xff,0,0x61};Tcl_Obj*original=Tcl_NewByteArrayObj(bytes,3);Tcl_IncrRefCount(original);Tcl_SetObjResult(ip,original);
Tcl_AddObjErrorInfo(ip,"",0);report(ip,"empty-append",original,NULL);
Tcl_AddObjErrorInfo(ip," context",8);report(ip,"nonempty-append",original,NULL);
Tcl_ResetResult(ip);report(ip,"append-reset",original,NULL);Tcl_DecrRefCount(original);Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;}
