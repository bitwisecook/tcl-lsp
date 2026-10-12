#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include "tclInt.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size ObsLen;
#else
typedef int ObsLen;
#endif
static void hex(const unsigned char *p,ObsLen n){ObsLen k;for(k=0;k<n;k++)printf("%02x",p[k]);}
static const char *spellings[]={"s-1-a","s-01-a","s-+1-a","s- 1-a","s--1-a","s-4294967297-a","s-18446744073709551617-a","s-1-other","s-1-a\0tail","BAD","s-1","s--a","s-1-a\xff"};
int main(int argc,char **argv){int k;Tcl_FindExecutable(argv[0]);
for(k=0;k<13;k++){Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CmdInfo info;Tcl_Obj *words[4],*h,*result,*code;ObsLen n;int c;
Tcl_Eval(ip,"array set a {k00 V k01 V k02 V}; array startsearch a");
h=Tcl_NewStringObj(spellings[k],k==8?10:-1);Tcl_IncrRefCount(h);
words[0]=Tcl_NewStringObj("array",-1);words[1]=Tcl_NewStringObj("anymore",-1);words[2]=Tcl_NewStringObj("a",-1);words[3]=h;
Tcl_IncrRefCount(words[0]);Tcl_IncrRefCount(words[1]);Tcl_IncrRefCount(words[2]);
Tcl_ResetResult(ip);Tcl_SetObjErrorCode(ip,Tcl_NewStringObj("SEEDED ARRAY",-1));
Tcl_GetCommandInfo(ip,"array",&info);c=info.objProc(info.objClientData,ip,4,words);
result=Tcl_GetObjResult(ip);printf("%d\t%d\t%s\t",k,c,h->typePtr?h->typePtr->name:"none");
if(h->typePtr&&!strcmp(h->typePtr->name,"array search"))printf("%d\t%lu\t",(int)(intptr_t)h->internalRep.twoPtrValue.ptr1,(unsigned long)(uintptr_t)h->internalRep.twoPtrValue.ptr2);else printf("-\t-\t");
{const char *b=Tcl_GetStringFromObj(result,&n);hex((const unsigned char *)b,n);}printf("\t");
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
{const char *b=((Interp *)ip)->errorCode;if(b)hex((const unsigned char *)b,(ObsLen)strlen(b));}
#else
code=((Interp *)ip)->errorCode;if(code){const char *b=Tcl_GetStringFromObj(code,&n);hex((const unsigned char *)b,n);}
#endif
printf("\n");
Tcl_DecrRefCount(words[0]);Tcl_DecrRefCount(words[1]);Tcl_DecrRefCount(words[2]);Tcl_DecrRefCount(h);Tcl_DeleteInterp(ip);
}return 0;}
