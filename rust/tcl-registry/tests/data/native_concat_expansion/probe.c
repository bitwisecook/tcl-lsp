/* SPDX-License-Identifier: AGPL-3.0-or-later */
#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
static const char *scripts[]={"concat {*}{A B} $c","concat {*}{A B} C","concat {*}{} $c","concat {*}{ {A B} {} } $c","concat {*}{A\\ B C} $c","concat {*}$a $c","concat {*}{\"} $c","concat {*}{A B} {*}$a $c"};
static void hex(const char *p,int n){int j;for(j=0;j<n;j++)printf("%02x",(unsigned char)p[j]);}
int main(int argc,char **argv){int id;(void)argc;Tcl_FindExecutable(argv[0]);for(id=0;id<8;id++){
 Tcl_Interp *ip=Tcl_CreateInterp();Tcl_Parse parse;Tcl_Obj *def[4],*call[3],*r;Proc *p;ByteCode *bc;int code,j,offset;
 printf("SOURCE\t%d\t",id);hex(scripts[id],(int)strlen(scripts[id]));putchar('\n');
 code=Tcl_ParseCommand(ip,scripts[id],-1,0,&parse);printf("PARSE\t%d\t%d\t%d\t",id,code,parse.numWords);hex(Tcl_GetStringResult(ip),(int)strlen(Tcl_GetStringResult(ip)));putchar('\n');
 if(code==TCL_OK)for(j=0;j<parse.numTokens;j++){Tcl_Token *t=&parse.tokenPtr[j];printf("TOKEN\t%d\t%d\t%d\t%ld\t%ld\t%d\t",id,j,t->type,(long)(t->start-scripts[id]),(long)t->size,t->numComponents);hex(t->start,(int)t->size);putchar('\n');}
 Tcl_FreeParse(&parse);Tcl_ResetResult(ip);
 def[0]=Tcl_NewStringObj("proc",-1);def[1]=Tcl_NewStringObj("p",-1);def[2]=Tcl_NewStringObj("a c",-1);def[3]=Tcl_NewStringObj(scripts[id],-1);for(j=0;j<4;j++)Tcl_IncrRefCount(def[j]);code=Tcl_EvalObjv(ip,4,def,0);
 call[0]=Tcl_NewStringObj("p",-1);call[1]=Tcl_NewStringObj("D E",-1);call[2]=Tcl_NewStringObj("C",-1);for(j=0;j<3;j++)Tcl_IncrRefCount(call[j]);if(code==TCL_OK)code=Tcl_EvalObjv(ip,3,call,0);
 r=Tcl_GetObjResult(ip);printf("RESULT\t%d\t%d\t%s\t%d\t%d\t",id,code,r->typePtr?r->typePtr->name:"NULL",r->bytes!=NULL,r->refCount);hex(Tcl_GetString(r),(int)strlen(Tcl_GetString(r)));putchar('\n');
 p=TclFindProc((Interp*)ip,"p");if(p&&p->bodyPtr->typePtr&&!strcmp(p->bodyPtr->typePtr->name,"bytecode")){bc=(ByteCode*)p->bodyPtr->internalRep.twoPtrValue.ptr1;
 for(offset=0;offset<bc->numCodeBytes;){unsigned char op=bc->codeStart[offset];printf("OP\t%d\t%d\t%s\t",id,offset,tclInstructionTable[op].name);hex((char*)bc->codeStart+offset,tclInstructionTable[op].numBytes);putchar('\n');offset+=tclInstructionTable[op].numBytes;}
 for(j=0;j<bc->numLitObjects;j++){Tcl_Obj *o=bc->objArrayPtr[j];printf("LITERAL\t%d\t%d\t%s\t%d\t%d\t",id,j,o->typePtr?o->typePtr->name:"NULL",o->bytes!=NULL,o->refCount);hex(Tcl_GetString(o),(int)strlen(Tcl_GetString(o)));putchar('\n');}}
 for(j=0;j<3;j++)Tcl_DecrRefCount(call[j]);for(j=0;j<4;j++)Tcl_DecrRefCount(def[j]);Tcl_DeleteInterp(ip);
 }Tcl_Finalize();return 0;}
