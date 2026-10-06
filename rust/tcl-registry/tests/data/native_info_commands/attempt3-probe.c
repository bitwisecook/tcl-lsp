#include "tcl.h"
#include <stdio.h>
static const unsigned char *patterns[]={(void*)"raw*",(void*)"raw?",(void*)"raw[\xff]",(void*)"raw\xff",(void*)"raw\xc3\xbf",(void*)"N::raw*",(void*)"N::raw?",(void*)"N::raw[\xff]",(void*)"::raw?",(void*)"raw\0::N::*"};
static const int lengths[]={4,4,6,4,5,9,9,11,6,10};
int main(int argc,char **argv) {
 Tcl_FindExecutable(argv[0]); Tcl_Interp *ip=Tcl_CreateInterp();
 if(Tcl_Eval(ip,"namespace eval N {}")!=TCL_OK)return 2;
 const char *names[]={"raw\xff","raw\xc3\xbf","N::raw\xff","N::raw\xc3\xbf"};
 for(int n=0;n<4;n++){Tcl_Obj *v[]={Tcl_NewStringObj("proc",-1),Tcl_NewStringObj(names[n],-1),Tcl_NewObj(),Tcl_NewObj()};for(int j=0;j<4;j++)Tcl_IncrRefCount(v[j]);int rc=Tcl_EvalObjv(ip,4,v,0);for(int j=0;j<4;j++)Tcl_DecrRefCount(v[j]);if(rc!=TCL_OK)return 3;}
 for(int kind=0;kind<2;kind++)for(int n=0;n<10;n++){
  Tcl_Obj *p=kind?Tcl_NewByteArrayObj(patterns[n],lengths[n]):Tcl_NewStringObj((void*)patterns[n],lengths[n]);
  Tcl_Obj *v[]={Tcl_NewStringObj("info",-1),Tcl_NewStringObj("commands",-1),p};for(int j=0;j<3;j++)Tcl_IncrRefCount(v[j]);
  int rc=Tcl_EvalObjv(ip,3,v,0);
#if TCL_MAJOR_VERSION>=9
  Tcl_Size count,size;
#else
  int count,size;
#endif
  Tcl_Obj **members;
  printf("%d|%d|%d|",kind,n,rc);
  if(Tcl_ListObjGetElements(ip,Tcl_GetObjResult(ip),&count,&members)!=TCL_OK)return 4;
  printf("%d",(int)count);for(int k=0;k<count;k++){const unsigned char *b=(void*)Tcl_GetStringFromObj(members[k],&size);printf("|");for(int j=0;j<size;j++)printf("%02x",b[j]);}puts("");
  for(int j=0;j<3;j++)Tcl_DecrRefCount(v[j]);
 }
 Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;
}
