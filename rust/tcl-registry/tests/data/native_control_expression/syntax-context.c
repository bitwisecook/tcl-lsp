#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION>=9
typedef Tcl_Size ProbeSize;
#else
typedef int ProbeSize;
#endif
static const char *bodies[]={"expr {1+2}","expr {$x+(1+2)}","expr {0x10+010}","expr {false || (1+2)}","expr {$x ? (1+2) : (1/0)}","expr {$x ? (1/0) : (1+2)}","expr {abs(-4)}","catch {expr {1/0}} result; return $result"};
static void state(Tcl_Obj*o){printf("%s,%d,%d",o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,o->refCount);if(o->bytes){putchar(',');for(int k=0;k<o->length;k++)printf("%02x",(unsigned char)o->bytes[k]);}}
int main(void){
#if TCL_MAJOR_VERSION>8 || TCL_MINOR_VERSION>=6
printf("OPCODES\t%d\t%d\n",INST_SYNTAX,INST_RETURN_IMM);
#endif
for(int k=0;k<8;k++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj*def[4]={Tcl_NewStringObj("proc",-1),Tcl_NewStringObj("p",-1),Tcl_NewStringObj("x",-1),Tcl_NewStringObj(bodies[k],-1)};for(int j=0;j<4;j++)Tcl_IncrRefCount(def[j]);int dc=Tcl_EvalObjv(i,4,def,0);Tcl_Obj*call[2]={Tcl_NewStringObj("p",-1),Tcl_NewStringObj("3",-1)};for(int j=0;j<2;j++)Tcl_IncrRefCount(call[j]);int c=Tcl_EvalObjv(i,2,call,0);printf("CASE\t%d\t%d\t%d\t",k,dc,c);state(Tcl_GetObjResult(i));putchar('\n');Proc*p=TclFindProc((Interp*)i,"p");if(p&&p->bodyPtr->typePtr&&!strcmp(p->bodyPtr->typePtr->name,"bytecode")){ByteCode*b=(ByteCode*)p->bodyPtr->internalRep.twoPtrValue.ptr1;for(int j=0;j<b->numLitObjects;j++){printf("L\t%d\t%d\t",k,j);state(b->objArrayPtr[j]);putchar('\n');}}
#if TCL_MAJOR_VERSION>8 || TCL_MINOR_VERSION>=6
{Interp*ip=(Interp*)i;Tcl_Obj**items=NULL;ProbeSize length=0;Tcl_ListObjGetElements(NULL,ip->innerContext,&length,&items);for(int q=0;q<length;q++){int lit=-1;if(p&&p->bodyPtr->typePtr&&!strcmp(p->bodyPtr->typePtr->name,"bytecode")){ByteCode*b=(ByteCode*)p->bodyPtr->internalRep.twoPtrValue.ptr1;for(int z=0;z<b->numLitObjects;z++)if(b->objArrayPtr[z]==items[q])lit=z;}printf("CTX\t%d\t%d\t%d\t%d\t",k,q,lit,items[q]==Tcl_GetObjResult(i));state(items[q]);putchar('\n');}if(length){printf("INST\t%d\t%ld\t",k,items[0]->internalRep.longValue);const char*name=Tcl_GetString(items[0]);while(*name)printf("%02x",(unsigned char)*name++);putchar('\n');}}
#endif
for(int j=0;j<2;j++)Tcl_DecrRefCount(call[j]);for(int j=0;j<4;j++)Tcl_DecrRefCount(def[j]);Tcl_DeleteInterp(i);}return 0;}
