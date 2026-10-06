#include "tclInt.h"
#include <stdio.h>
#include <string.h>
typedef struct{Tcl_Size len;Tcl_Obj**elements;int dbl;Tcl_Size refs;}Series;
static Tcl_Obj*root;static int c;
static void snapshot(Tcl_Interp*i,const char*w){Tcl_Obj*v=Tcl_GetVar2Ex(i,"v",NULL,TCL_GLOBAL_ONLY);Series*r=root->typePtr&&strcmp(root->typePtr->name,"arithseries")==0?root->internalRep.twoPtrValue.ptr1:NULL;printf("S\t%d\t%s\t%s\t%d\t%d\t%ld\t%d\t%s\t%d\t%d\n",c,w,root->typePtr?root->typePtr->name:"none",root->bytes!=NULL,root->refCount,r?(long)r->refs:-1,r&&r->elements!=NULL,v&&v->typePtr?v->typePtr->name:"none",v?v->bytes!=NULL:0,v?v->refCount:-1);}
static char*watch(ClientData d,Tcl_Interp*i,const char*a,const char*b,int f){snapshot(i,"write");return NULL;}
static int body(ClientData d,Tcl_Interp*i,int n,Tcl_Obj*const*a){snapshot(i,"body");Tcl_ResetResult(i);return TCL_BREAK;}
int main(int argc,char**argv){Tcl_FindExecutable(argv[0]);const char*s[]={"lseq 5","lseq 0","lseq 100000001","lseq 0 to 0.5 by 0.1"};for(c=0;c<4;c++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_CreateObjCommand(i,"stop",body,NULL,NULL);Tcl_TraceVar(i,"v",TCL_TRACE_WRITES|TCL_GLOBAL_ONLY,watch,NULL);int code=Tcl_Eval(i,s[c]);root=Tcl_GetObjResult(i);Tcl_IncrRefCount(root);Tcl_ResetResult(i);Tcl_Obj*a[]={Tcl_NewStringObj("foreach",-1),Tcl_NewStringObj("v",-1),root,Tcl_NewStringObj("stop",-1)};for(int n=0;n<4;n++)if(n!=2)Tcl_IncrRefCount(a[n]);snapshot(i,"before");code=Tcl_EvalObjv(i,4,a,TCL_EVAL_GLOBAL);snapshot(i,"after");printf("R\t%d\t%d\t%s\n",c,code,Tcl_GetStringResult(i));for(int n=0;n<4;n++)Tcl_DecrRefCount(a[n]);Tcl_DeleteInterp(i);}Tcl_Finalize();}
