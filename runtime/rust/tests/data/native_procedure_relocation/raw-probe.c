#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
#else
#include "tclInt.h"
#endif
static void hex(const char *s,int n){for(int j=0;j<n;j++)printf("%02x",(unsigned char)s[j]);}
int main(int argc,char **argv){
 const char names[][16]={"::A::p\xff","::A::p\0tail","p\xff","::A::p\xff\0tail"};
 const char destinations[][16]={"::B::q\xff","::B::q\0tail","::B::q\xff","::B::q\xff\0tail"};
 const int lengths[]={7,11,2,12};
 const int destination_lengths[]={7,11,7,12};
 for(int k=0;k<4;k++){
#ifdef JIM_PROBE
  Jim_Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_InitStaticExtensions(i);
  if(Jim_Eval(i,"namespace eval A {};namespace eval B {}"))return 2;
  Jim_Obj*words[4]={Jim_NewStringObj(i,"proc",-1),Jim_NewStringObj(i,names[k],lengths[k]),Jim_NewStringObj(i,"",0),Jim_NewStringObj(i,"namespace current",-1)};
  for(int j=0;j<4;j++)Jim_IncrRefCount(words[j]);
  int code=Jim_EvalObjVector(i,4,words);if(code)return 3;
  Jim_Cmd*before=Jim_GetCommand(i,words[1],0);if(!before)return 4;
  Jim_Obj*rename[3]={Jim_NewStringObj(i,"rename",-1),words[1],Jim_NewStringObj(i,destinations[k],destination_lengths[k])};Jim_IncrRefCount(rename[0]);Jim_IncrRefCount(rename[2]);
  code=Jim_EvalObjVector(i,3,rename);Jim_Cmd*after=Jim_GetCommand(i,rename[2],0);
  printf("rename\t%d\t%d\t%d\n",k,code,after==before);
  code=Jim_EvalObjVector(i,1,&rename[2]);int n;const char*s=Jim_GetString(Jim_GetResult(i),&n);
  printf("future\t%d\t%d\t",k,code);hex(s,n);puts("");
  Jim_DecrRefCount(i,rename[0]);Jim_DecrRefCount(i,rename[2]);for(int j=0;j<4;j++)Jim_DecrRefCount(i,words[j]);Jim_FreeInterp(i);
#else
  Tcl_FindExecutable(argv[0]);Tcl_Interp*i=Tcl_CreateInterp();if(Tcl_Eval(i,"namespace eval A {};namespace eval B {}"))return 2;
  Tcl_Obj*words[4]={Tcl_NewStringObj("proc",-1),Tcl_NewStringObj(names[k],lengths[k]),Tcl_NewStringObj("",0),Tcl_NewStringObj("namespace current",-1)};
  for(int j=0;j<4;j++)Tcl_IncrRefCount(words[j]);
  int code=Tcl_EvalObjv(i,4,words,0);if(code)return 3;
  Tcl_Command before=Tcl_FindCommand(i,Tcl_GetString(words[1]),NULL,0);if(!before)return 4;
  Tcl_Obj*rename[3]={Tcl_NewStringObj("rename",-1),words[1],Tcl_NewStringObj(destinations[k],destination_lengths[k])};Tcl_IncrRefCount(rename[0]);Tcl_IncrRefCount(rename[2]);
  code=Tcl_EvalObjv(i,3,rename,0);Tcl_Command after=Tcl_FindCommand(i,Tcl_GetString(rename[2]),NULL,0);
  printf("rename\t%d\t%d\t%d\n",k,code,after==before);
  code=Tcl_EvalObjv(i,1,&rename[2],0);
#if TCL_MAJOR_VERSION >= 9
  Tcl_Size n;
#else
  int n;
#endif
  const char*s=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n);printf("future\t%d\t%d\t",k,code);hex(s,n);puts("");
  Tcl_DecrRefCount(rename[0]);Tcl_DecrRefCount(rename[2]);for(int j=0;j<4;j++)Tcl_DecrRefCount(words[j]);Tcl_DeleteInterp(i);
#endif
 }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
