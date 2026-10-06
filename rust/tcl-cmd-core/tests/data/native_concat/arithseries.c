/* SPDX-License-Identifier: AGPL-3.0-or-later */
#include "tclInt.h"
#include <stdio.h>
#include <stdbool.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef struct {Tcl_Size len;Tcl_Obj **elements;
#if TCL_MINOR_VERSION >= 1
bool isDouble;
#else
int isDouble;
#endif
Tcl_Size refs;} ActualSeries;
static void state(Tcl_Obj *o) {
 if(o->typePtr && !strcmp(o->typePtr->name,"arithseries")) {ActualSeries *a=(ActualSeries *)o->internalRep.twoPtrValue.ptr1;printf("arithseries,%d,%d,%ld,%ld",o->bytes!=NULL,a->elements!=NULL,(long)o->refCount,(long)a->refs);}
 else printf("%s,%d,-1,%ld,-1",o->typePtr?o->typePtr->name:"NULL",o->bytes!=NULL,(long)o->refCount);
}
static Tcl_Obj *series(Tcl_Interp *ip) {
 Tcl_Obj *v[4],*r; int k;v[0]=Tcl_NewStringObj("lseq",4);v[1]=Tcl_NewIntObj(1);v[2]=Tcl_NewIntObj(3);v[3]=Tcl_NewIntObj(1);
 for(k=0;k<4;k++)Tcl_IncrRefCount(v[k]);if(Tcl_EvalObjv(ip,4,v,0)!=TCL_OK)Tcl_Panic("actual lseq failed");r=Tcl_GetObjResult(ip);Tcl_IncrRefCount(r);Tcl_ResetResult(ip);for(k=0;k<4;k++)Tcl_DecrRefCount(v[k]);return r;
}
static void run(Tcl_Interp *ip,int id) {
 Tcl_Obj *a=series(ip),*r=NULL,*v[2],*word=NULL,*child=NULL;int n=1;
 printf("A\t%d\tbefore\t",id);state(a);putchar('\n');
 if(id==0) {Tcl_ListObjIndex(NULL,a,0,&child);printf("A\t%d\tindex\t%s,%d,%ld\n",id,child->typePtr->name,child->bytes!=NULL,(long)child->refCount);Tcl_GetString(child);Tcl_IncrRefCount(child);Tcl_DecrRefCount(child);}
 else {if(id==1){v[0]=a;}else {word=Tcl_NewStringObj("a",1);v[0]=Tcl_NewListObj(1,&word);Tcl_IncrRefCount(v[0]);v[1]=a;n=2;}r=Tcl_ConcatObj(n,v);Tcl_IncrRefCount(r);printf("A\t%d\tresult\t",id);state(r);putchar('\n');}
 printf("A\t%d\tafter\t",id);state(a);putchar('\n');
 if(r)Tcl_DecrRefCount(r);if(id==2)Tcl_DecrRefCount(v[0]);Tcl_DecrRefCount(a);
}
#endif
int main(int argc,char **argv){Tcl_Interp *ip;Tcl_CmdInfo info;(void)argc;Tcl_FindExecutable(argv[0]);ip=Tcl_CreateInterp();printf("SUPPORTED\t%d\n",Tcl_GetCommandInfo(ip,"lseq",&info));
#if TCL_MAJOR_VERSION >= 9
run(ip,0);run(ip,1);run(ip,2);
#endif
Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;}
