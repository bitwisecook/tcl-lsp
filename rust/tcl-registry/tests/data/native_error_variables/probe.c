#include <stdio.h>
#include <string.h>
#include <stddef.h>
#include "tcl.h"
#include "tclInt.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
static void hex(Tcl_Obj *o){if(!o){printf("null");return;}Len n;const char*s=Tcl_GetStringFromObj(o,&n);putchar('"');for(Len k=0;k<n;k++)printf("%02x",(unsigned char)s[k]);putchar('"');}
static Var* cell(Interp*i,const char*name){
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
 Tcl_HashEntry*h=Tcl_FindHashEntry(&i->globalNsPtr->varTable,name);return h?Tcl_GetHashValue(h):NULL;
#else
 Tcl_Obj*key=Tcl_NewStringObj(name,-1);Tcl_IncrRefCount(key);Tcl_HashEntry*h=Tcl_FindHashEntry(&i->globalNsPtr->varTable.table,(const char*)key);Tcl_DecrRefCount(key);return h?(Var*)((char*)h - offsetof(VarInHash,entry)):NULL;
#endif
}
static void report(Tcl_Interp*ip,const char*label,int code,Tcl_Obj*read){Interp*i=(Interp*)ip;printf("%s\tcode=%d\tread=",label,code);hex(read);
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
 printf("\tlegacy=-1\tprivateinfo=null\tprivatecode=null");
#else
 printf("\tlegacy=%d\tprivateinfo=",!!(i->flags&ERR_LEGACY_COPY));hex(i->errorInfo);printf("\tprivatecode=");hex(i->errorCode);
#endif
 for(unsigned k=0;k<2;k++){const char*name=k?"errorCode":"errorInfo";Var*v=cell(i,name);printf("\t%s_present=%d\t%s_defined=%d\t%s_value=",name,v!=NULL,name,v&&!TclIsVarUndefined(v),name);hex(v&&!TclIsVarUndefined(v)&&!TclIsVarArray(v)&&!TclIsVarLink(v)?v->value.objPtr:NULL);
#if !(TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4)
 printf("\t%s_same=%d",name,v&&!TclIsVarUndefined(v)&&!TclIsVarArray(v)&&v->value.objPtr==(k?i->errorCode:i->errorInfo));
#endif
 }
 puts("");}
static void seed(Tcl_Interp*ip,int flags,int values){
#if !(TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4)
 Interp*i=(Interp*)ip;if(i->errorInfo)Tcl_DecrRefCount(i->errorInfo);if(i->errorCode)Tcl_DecrRefCount(i->errorCode);i->errorInfo=values?Tcl_NewStringObj("PRIVATE INFO",-1):NULL;i->errorCode=values?Tcl_NewStringObj("PRIVATE CODE",-1):NULL;if(i->errorInfo)Tcl_IncrRefCount(i->errorInfo);if(i->errorCode)Tcl_IncrRefCount(i->errorCode);if(flags)i->flags|=ERR_LEGACY_COPY;else i->flags&=~ERR_LEGACY_COPY;
#endif
}
int main(int argc,char**argv){(void)argc;Tcl_FindExecutable(argv[0]);Tcl_Interp*ip=Tcl_CreateInterp();report(ip,"constructor",0,NULL);Tcl_Obj*o=Tcl_GetVar2Ex(ip,"errorInfo",NULL,TCL_GLOBAL_ONLY);report(ip,"fresh-read",o?0:1,o);Tcl_SetVar(ip,"errorInfo","GUEST INFO",TCL_GLOBAL_ONLY);Tcl_SetVar(ip,"errorCode","GUEST CODE",TCL_GLOBAL_ONLY);seed(ip,0,1);o=Tcl_GetVar2Ex(ip,"errorInfo",NULL,TCL_GLOBAL_ONLY);report(ip,"clear-flag-read",o?0:1,o);seed(ip,1,1);o=Tcl_GetVar2Ex(ip,"errorInfo",NULL,TCL_GLOBAL_ONLY);report(ip,"set-flag-read",o?0:1,o);int code=Tcl_UnsetVar(ip,"errorInfo",TCL_GLOBAL_ONLY);report(ip,"unset-defined",code,NULL);o=Tcl_GetVar2Ex(ip,"errorInfo",NULL,TCL_GLOBAL_ONLY);report(ip,"read-after-unset",o?0:1,o);code=Tcl_UnsetVar(ip,"errorCode",TCL_GLOBAL_ONLY);report(ip,"unset-code-defined",code,NULL);code=Tcl_UnsetVar(ip,"errorCode",TCL_GLOBAL_ONLY);report(ip,"unset-code-undefined",code,NULL);seed(ip,1,0);o=Tcl_GetVar2Ex(ip,"errorCode",NULL,TCL_GLOBAL_ONLY);report(ip,"null-private-read",o?0:1,o);seed(ip,1,1);Tcl_ResetResult(ip);report(ip,"reset-legacy",0,NULL);Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;}
