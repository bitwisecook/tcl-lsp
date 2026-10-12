/* SPDX-License-Identifier: AGPL-3.0-or-later */
#include "jim.h"
#include <stdio.h>
#include <string.h>
static const char *scripts[]={"concat {*}{A B} $c","concat {*}{A B} C","concat {*}{} $c","concat {*}{ {A B} {} } $c","concat {*}{A\\ B C} $c","concat {*}$a $c","concat {*}{\"} $c","concat {*}{A B} {*}$a $c"};
static void hex(const char*p,int n){int j;for(j=0;j<n;j++)printf("%02x",(unsigned char)p[j]);}
int main(void){int id;for(id=0;id<8;id++){Jim_Interp *ip=Jim_CreateInterp();Jim_Obj *def[4],*call[3],*r;int j,code;Jim_RegisterCoreCommands(ip);
 def[0]=Jim_NewStringObj(ip,"proc",-1);def[1]=Jim_NewStringObj(ip,"p",-1);def[2]=Jim_NewStringObj(ip,"a c",-1);def[3]=Jim_NewStringObj(ip,scripts[id],-1);for(j=0;j<4;j++)Jim_IncrRefCount(def[j]);code=Jim_EvalObjVector(ip,4,def);
 call[0]=Jim_NewStringObj(ip,"p",-1);call[1]=Jim_NewStringObj(ip,"D E",-1);call[2]=Jim_NewStringObj(ip,"C",-1);for(j=0;j<3;j++)Jim_IncrRefCount(call[j]);if(code==JIM_OK)code=Jim_EvalObjVector(ip,3,call);r=Jim_GetResult(ip);printf("RESULT\t%d\t%d\t%s\t%d\t%d\t",id,code,r->typePtr?r->typePtr->name:"NULL",r->bytes!=NULL,r->refCount);hex(Jim_String(r),Jim_Length(r));printf("\nBODY\t%d\t%s\t%d\t%d\n",id,def[3]->typePtr?def[3]->typePtr->name:"NULL",def[3]->bytes!=NULL,def[3]->refCount);
 for(j=0;j<3;j++)Jim_DecrRefCount(ip,call[j]);for(j=0;j<4;j++)Jim_DecrRefCount(ip,def[j]);Jim_FreeInterp(ip);}return 0;}
