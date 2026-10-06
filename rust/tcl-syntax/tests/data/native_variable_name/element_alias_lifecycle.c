#include <stdio.h>
#include <string.h>
#include <stdint.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Obj Obj;
typedef Jim_Interp ProbeInterp;
#define NEW(i,s) Jim_NewStringObj(i,s,-1)
#define INC(o) Jim_IncrRefCount(o)
#define DEC(i,o) Jim_DecrRefCount(i,o)
static ProbeInterp *create(void) { ProbeInterp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK) return NULL; return i; }
#define DESTROY(i) Jim_FreeInterp(i)
static int writevar(ProbeInterp*i,Obj*n,Obj*v) { return Jim_SetVariable(i,n,v); }
static int removevar(ProbeInterp*i,Obj*n) { return Jim_UnsetVariable(i,n,0); }
static Obj *readvar(ProbeInterp*i,Obj*n) { return Jim_GetVariable(i,n,0); }
static int linkvar(ProbeInterp*i,Obj*n,Obj*t) { return Jim_SetVariableLink(i,n,t,i->topFramePtr); }
#else
#include "tcl.h"
#include "tclInt.h"
typedef Tcl_Obj Obj;
typedef Tcl_Interp ProbeInterp;
#define NEW(i,s) Tcl_NewStringObj(s,-1)
#define INC(o) Tcl_IncrRefCount(o)
#define DEC(i,o) Tcl_DecrRefCount(o)
static ProbeInterp *create(void) { return Tcl_CreateInterp(); }
#define DESTROY(i) Tcl_DeleteInterp(i)
static int writevar(ProbeInterp*i,Obj*n,Obj*v) { return Tcl_ObjSetVar2(i,n,NULL,v,0)?0:1; }
static int removevar(ProbeInterp*i,Obj*n) { return Tcl_UnsetVar(i,Tcl_GetString(n),TCL_GLOBAL_ONLY); }
static Obj *readvar(ProbeInterp*i,Obj*n) { return Tcl_ObjGetVar2(i,n,NULL,0); }
static int linkvar(ProbeInterp*i,Obj*n,Obj*t) { return Tcl_UpVar(i,"#0",Tcl_GetString(t),Tcl_GetString(n),0); }
static Var *currentelement(ProbeInterp*i) {
 Var *array=(Var*)Tcl_FindNamespaceVar(i,"arr",NULL,0);
 if(!array || !TclIsVarArray(array) || TclIsVarUndefined(array)) return NULL;
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
 Tcl_HashTable *table=array->value.tablePtr;
#else
 Tcl_HashTable *table=&array->value.tablePtr->table;
#endif
#if TCL_MAJOR_VERSION>=9 || TCL_MINOR_VERSION>=5
 Obj *key=Tcl_NewStringObj("k",1); Tcl_IncrRefCount(key);
 Tcl_HashEntry *entry=Tcl_FindHashEntry(table,(char*)key); Tcl_DecrRefCount(key);
#else
 Tcl_HashEntry *entry=Tcl_FindHashEntry(table,"k");
#endif
 return entry?(Var*)Tcl_GetHashValue(entry):NULL;
}
#endif
static void observe(ProbeInterp*i,int whole,const char*phase,Obj*name,Obj*root,Obj*key,void*old,int aliases) {
 int same=-1,present=-1,defined=-1,dead=-1,refs=-1;
#ifndef USE_JIM
 Var *current=currentelement(i);same=current==(Var*)old;present=current!=NULL;
 if(aliases>0||strcmp(phase,"created")==0) {
  Var *original=(Var*)old;defined=!TclIsVarUndefined(original);
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
  dead=original->hPtr==NULL;refs=original->refCount;
#else
  dead=!!TclIsVarDeadHash(original);refs=((VarInHash*)original)->refCount;
#endif
 }
#endif
 printf("window|%s|%s|%d|%d|%d|%d|%d|%d|%d|%d\n",whole?"whole":"element",phase,name->refCount,root?root->refCount:-1,key?key->refCount:-1,present,same,defined,dead,refs);
}
static void value(const char *label,Obj*result) {
 printf("value|%s|",label);
 if(!result) printf("null");
 else {
#ifdef USE_JIM
  int length;const unsigned char*s=(const unsigned char*)Jim_GetString(result,&length);
#else
#if TCL_MAJOR_VERSION>=9
  Tcl_Size length;
#else
  int length;
#endif
  const unsigned char*s=(const unsigned char*)Tcl_GetStringFromObj(result,&length);
#endif
  for(int j=0;j<length;j++)printf("%02x",s[j]);
 }
 puts("");
}

static int writeglobal(ProbeInterp*i,Obj*n,Obj*v) {
#ifdef USE_JIM
 Jim_CallFrame *frame=i->framePtr;i->framePtr=i->topFramePtr;int code=Jim_SetVariable(i,n,v);i->framePtr=frame;return code;
#else
 return Tcl_ObjSetVar2(i,n,NULL,v,TCL_GLOBAL_ONLY)?0:1;
#endif
}
static int removeglobal(ProbeInterp*i,Obj*n) {
#ifdef USE_JIM
 Jim_CallFrame *frame=i->framePtr;i->framePtr=i->topFramePtr;int code=Jim_UnsetVariable(i,n,0);i->framePtr=frame;return code;
#else
 return Tcl_UnsetVar(i,Tcl_GetString(n),TCL_GLOBAL_ONLY);
#endif
}
static Obj *readglobal(ProbeInterp*i,Obj*n) {
#ifdef USE_JIM
 Jim_CallFrame *frame=i->framePtr;i->framePtr=i->topFramePtr;Obj*v=Jim_GetVariable(i,n,0);i->framePtr=frame;return v;
#else
 return Tcl_ObjGetVar2(i,n,NULL,TCL_GLOBAL_ONLY);
#endif
}
typedef struct State {int whole;Obj*name,*rootname,*one,*two,*a,*b,*fresh,*root,*key;void*old;int key_pin;} State;
static State *active;
static int evaluate(ProbeInterp*i,const char*s) {
#ifdef USE_JIM
 return Jim_Eval(i,s);
#else
 return Tcl_EvalEx(i,s,-1,0);
#endif
}
#ifdef USE_JIM
static int inner(ProbeInterp*i,int objc,Obj*const argv[]) {
#else
static int inner(ClientData data,ProbeInterp*i,int objc,Obj*const argv[]) {
 (void)data;
#endif
 (void)objc;(void)argv;State*s=active;
 printf("call|link-b|%d\n",linkvar(i,s->b,s->name));
 observe(i,s->whole,"linked",s->name,s->root,s->key,s->old,2);
 printf("call|unset-target|%d\n",removeglobal(i,s->whole?s->rootname:s->name));
 observe(i,s->whole,"unset-target",s->name,s->root,s->key,s->old,2);value("alias-after-unset",readvar(i,s->b));
 printf("call|recreate|%d\n",writeglobal(i,s->fresh,s->two));
 observe(i,s->whole,"recreated",s->name,s->root,s->key,s->old,2);value("alias-after-recreate",readvar(i,s->b));value("fresh-after-recreate",readglobal(i,s->fresh));
 printf("call|unset-recreated|%d\n",removeglobal(i,s->fresh));
 observe(i,s->whole,"unset-recreated",s->name,s->root,s->key,s->old,2);
#ifdef USE_JIM
 Jim_SetResultString(i,"",0);
#else
 Tcl_ResetResult(i);
#endif
 return 0;
}
#ifdef USE_JIM
static int outer(ProbeInterp*i,int objc,Obj*const argv[]) {
#else
static int outer(ClientData data,ProbeInterp*i,int objc,Obj*const argv[]) {
 (void)data;
#endif
 (void)objc;(void)argv;State*s=active;
 printf("call|link-a|%d\n",linkvar(i,s->a,s->name));
 printf("call|inner|%d\n",evaluate(i,"probe_inner_proc"));
 observe(i,s->whole,"one-alias",s->name,s->root,s->key,s->old,1);
#ifdef USE_JIM
 Jim_SetResultString(i,"",0);
#else
 Tcl_ResetResult(i);
#endif
 return 0;
}
int main(void) {
 setvbuf(stdout,NULL,_IONBF,0);
 for(int whole=0;whole<2;whole++) {
  ProbeInterp*i=create();if(!i)return 2;
  State state={0};state.whole=whole;active=&state;
  state.name=NEW(i,"arr(k)");state.rootname=NEW(i,"arr");state.one=NEW(i,"ONE");state.two=NEW(i,"TWO");state.a=NEW(i,"a");state.b=NEW(i,"b");state.fresh=NEW(i,"arr(k)");
  Obj *owned[]={state.name,state.rootname,state.one,state.two,state.a,state.b,state.fresh};for(int j=0;j<7;j++)INC(owned[j]);
  printf("call|created|%d\n",writeglobal(i,state.name,state.one));
#ifdef USE_JIM
  state.root=state.name->internalRep.dictSubstValue.varNameObjPtr;state.key=state.name->internalRep.dictSubstValue.indexObjPtr;
#else
#if TCL_MAJOR_VERSION>=9
  const Tcl_ObjInternalRep*rep=Tcl_FetchInternalRep(state.name,state.name->typePtr);state.root=(Obj*)rep->twoPtrValue.ptr1;state.key=(Obj*)rep->twoPtrValue.ptr2;
#else
  state.root=(Obj*)state.name->internalRep.twoPtrValue.ptr1;
#endif
  state.old=currentelement(i);
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION>=5
  state.key=((VarInHash*)state.old)->entry.key.objPtr;INC(state.key);state.key_pin=1;
#endif
#endif
  printf("observer-key-pin|%d\n",state.key_pin);
  observe(i,whole,"created",state.name,state.root,state.key,state.old,0);
#ifdef USE_JIM
  Jim_CreateCommand(i,"probe_outer",outer,NULL,NULL);Jim_CreateCommand(i,"probe_inner",inner,NULL,NULL);
#else
  Tcl_CreateObjCommand(i,"probe_outer",outer,NULL,NULL);Tcl_CreateObjCommand(i,"probe_inner",inner,NULL,NULL);
#endif
  printf("call|procedure|%d\n",evaluate(i,"proc probe_outer_proc {} {probe_outer}; proc probe_inner_proc {} {probe_inner}; probe_outer_proc"));
  observe(i,whole,"no-alias",state.name,state.root,state.key,state.old,0);
  if(state.key_pin)DEC(i,state.key);
  for(int j=6;j>=0;j--)DEC(i,owned[j]);DESTROY(i);
 }
 return 0;
}
