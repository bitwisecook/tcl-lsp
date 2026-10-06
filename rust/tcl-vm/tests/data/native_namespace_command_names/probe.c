// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <tcl.h>
#if TCL_MAJOR_VERSION < 9
typedef int Tcl_Size;
#endif
#include <stdio.h>
#include <string.h>
static int empty(ClientData data,Tcl_Interp *ip,int objc,Tcl_Obj *const objv[]) {return TCL_OK;}
static void hex(const char *p,Tcl_Size n){Tcl_Size k;for(k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);}
int main(int argc,char **argv){Tcl_Interp *ip;int kind,row,code;Tcl_Size n;Tcl_Obj *words[4],*original;const char opaque[]={'o','p','a','q','u','e',(char)0xff,0};const char *inputs[]={opaque,"opaque\xff\0ignored","absent\0suffix"};int lengths[]={7,15,13};Tcl_FindExecutable(argv[0]);ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,opaque,empty,NULL,NULL);for(kind=0;kind<2;kind++)for(row=0;row<3;row++){original=Tcl_NewStringObj(inputs[row],lengths[row]);Tcl_IncrRefCount(original);words[0]=Tcl_NewStringObj("namespace",-1);words[1]=Tcl_NewStringObj(kind?"origin":"which",-1);words[2]=kind?original:Tcl_NewStringObj("-command",-1);words[3]=original;Tcl_IncrRefCount(words[0]);Tcl_IncrRefCount(words[1]);if(!kind)Tcl_IncrRefCount(words[2]);code=Tcl_EvalObjv(ip,kind?3:4,words,0);printf("%s\t%d\t%d\t",kind?"origin":"which",row,code);{const char *p=Tcl_GetStringFromObj(Tcl_GetObjResult(ip),&n);hex(p,n);}printf("\t%s\t",original->typePtr?original->typePtr->name:"none");{const char *p=Tcl_GetStringFromObj(original,&n);hex(p,n);}puts("");Tcl_DecrRefCount(words[0]);Tcl_DecrRefCount(words[1]);if(!kind)Tcl_DecrRefCount(words[2]);Tcl_DecrRefCount(original);Tcl_ResetResult(ip);}Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;}
