#include "tcl.h"
#include "tclInt.h"
#include <stdio.h>
#include <string.h>
static const char *kind(Tcl_Obj*o){return o->typePtr?o->typePtr->name:"none";}
static void observe(Tcl_Interp*i,int mode,const char*phase,Tcl_Obj*name,Tcl_Obj*fresh,Var*before){
 Var *v=(Var*)Tcl_FindNamespaceVar(i,"k",NULL,0);Tcl_Obj*key=NULL;
#if TCL_MAJOR_VERSION>=9 || TCL_MINOR_VERSION>=5
 if(v){VarInHash*h=(VarInHash*)v;key=h->entry.key.objPtr;}
#endif
 printf("alias|%d|%s|%s|%d|%d|%d|%d|%d\n",mode,phase,kind(name),name->refCount,fresh?fresh->refCount:-1,v!=NULL,v==before,key==name);
}
static int call(Tcl_Interp*i,int n,Tcl_Obj**a){int c=Tcl_EvalObjv(i,n,a,0);printf("call|%d\n",c);return c;}
typedef struct State {int mode;Tcl_Obj*name,*fresh,*value;Var*before;} State;
static int worker(ClientData data,Tcl_Interp*i,int objc,Tcl_Obj*const objv[]){(void)objc;(void)objv;State*s=(State*)data;
 Tcl_Obj*global=Tcl_NewStringObj("::k",3);Tcl_IncrRefCount(global);Tcl_Obj*alias=Tcl_NewStringObj("a",1);Tcl_IncrRefCount(alias);
 Tcl_Obj*args[]={Tcl_NewStringObj("upvar",-1),Tcl_NewStringObj("#0",-1),s->name,alias};Tcl_IncrRefCount(args[0]);Tcl_IncrRefCount(args[1]);call(i,4,args);observe(i,s->mode,"linked",s->name,NULL,s->before);
 Tcl_Obj*unset=Tcl_NewStringObj("unset",-1);Tcl_IncrRefCount(unset);Tcl_Obj*ua[]={unset,global};call(i,2,ua);observe(i,s->mode,"unset-target",s->name,NULL,s->before);
 s->fresh=Tcl_NewStringObj("k",1);Tcl_IncrRefCount(s->fresh);Tcl_ObjSetVar2(i,s->mode?alias:global,NULL,s->value,0);observe(i,s->mode,"recreated",s->name,s->fresh,s->before);
 call(i,2,ua);observe(i,s->mode,"unset-recreated-target",s->name,s->fresh,s->before);Tcl_Obj*ub[]={unset,alias};call(i,2,ub);observe(i,s->mode,"unset-alias",s->name,s->fresh,s->before);
 Tcl_DecrRefCount(global);Tcl_DecrRefCount(args[0]);Tcl_DecrRefCount(args[1]);Tcl_DecrRefCount(unset);Tcl_DecrRefCount(alias);Tcl_ResetResult(i);return TCL_OK;
}
int main(void){for(int mode=0;mode<2;mode++){
 Tcl_Interp*i=Tcl_CreateInterp();State s={mode,Tcl_NewStringObj("k",1),NULL,Tcl_NewStringObj("ONE",3),NULL};Tcl_IncrRefCount(s.name);Tcl_IncrRefCount(s.value);Tcl_ObjSetVar2(i,s.name,NULL,s.value,0);s.before=(Var*)Tcl_FindNamespaceVar(i,"k",NULL,0);observe(i,mode,"created",s.name,NULL,s.before);
 Tcl_CreateObjCommand(i,"worker",worker,&s,NULL);printf("procedure|%d\n",Tcl_EvalEx(i,"proc run {} {worker}; run",-1,0));observe(i,mode,"frame-popped",s.name,s.fresh,s.before);Tcl_ObjGetVar2(i,s.fresh,NULL,0);observe(i,mode,"fresh-read",s.name,s.fresh,s.before);
 Tcl_DecrRefCount(s.fresh);Tcl_DecrRefCount(s.value);Tcl_DecrRefCount(s.name);Tcl_DeleteInterp(i);
 }
 Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj*name=Tcl_NewStringObj("arr(k)",-1);Tcl_IncrRefCount(name);Tcl_Obj*v=Tcl_NewStringObj("VALUE",-1);Tcl_IncrRefCount(v);Tcl_ObjSetVar2(i,name,NULL,v,0);
#if TCL_MAJOR_VERSION>=9
 const Tcl_ObjInternalRep*ir=Tcl_FetchInternalRep(name,name->typePtr);Tcl_Obj*root=(Tcl_Obj*)ir->twoPtrValue.ptr1,*element=(Tcl_Obj*)ir->twoPtrValue.ptr2;printf("parts|stored|%d|%d\n",root->refCount,element->refCount);
#else
 Tcl_Obj*root=(Tcl_Obj*)name->internalRep.twoPtrValue.ptr1;printf("parts|stored|%d|-1\n",root->refCount);
#endif
 Tcl_Obj*copy=Tcl_DuplicateObj(name);Tcl_IncrRefCount(copy);
#if TCL_MAJOR_VERSION>=9
 printf("parts|duplicated|%d|%d\n",root->refCount,element->refCount);
#else
 printf("parts|duplicated|%d|-1\n",root->refCount);
#endif
 Tcl_DecrRefCount(copy);Tcl_EvalEx(i,"unset arr",-1,0);
#if TCL_MAJOR_VERSION>=9
 printf("parts|unset|%d|%d\n",root->refCount,element->refCount);
#else
 printf("parts|unset|%d|-1\n",root->refCount);
#endif
 Tcl_DecrRefCount(name);Tcl_DecrRefCount(v);Tcl_DeleteInterp(i);return 0;
}
