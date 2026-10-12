#include <stdio.h>
#include <string.h>
#include "tcl.h"
#include "tclInt.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size L;
#else
typedef int L;
#endif
static Tcl_Interp *active; static int hook_calls, old_b_deleted, new_b_deleted;
static void old_b(void *p) { (void)p; old_b_deleted++; }
static void new_b(void *p) { (void)p; new_b_deleted++; }
static int hook(void *p,Tcl_Interp *i,int n,Tcl_Obj *const *v) {(void)p;(void)i;(void)n;(void)v;return TCL_OK;}
static void retire(void *p) {
  Tcl_Namespace *b=Tcl_FindNamespace(active,"::B",NULL,0); hook_calls++;
  if(b)Tcl_DeleteNamespace(b);
  if(p)Tcl_CreateNamespace(active,"::B",NULL,new_b);
}
static void one(const char *label,int scenario,int rawkind) {
  Tcl_Interp *i=Tcl_CreateInterp(); active=i;hook_calls=old_b_deleted=new_b_deleted=0;
  Tcl_CreateNamespace(i,"::A",NULL,NULL); Tcl_CreateNamespace(i,"::A::q",NULL,NULL);
  Tcl_CreateNamespace(i,"::B",NULL,old_b);
  if(scenario>=2)Tcl_CreateObjCommand(i,"::A::hook",hook,NULL,retire);
  if(scenario==3) {Tcl_DeleteCommand(i,"::A::hook");hook_calls=old_b_deleted=new_b_deleted=0;Tcl_CreateNamespace(i,"::B",NULL,old_b);Tcl_CreateObjCommand(i,"::A::hook",hook,(void *)1,retire);}
  const char raw[]={'\x3a','\x3a','A','\0','z'};
  Tcl_Obj *a=rawkind==1?Tcl_NewStringObj(raw,5):rawkind==2?Tcl_NewByteArrayObj((const unsigned char *)raw,5):Tcl_NewStringObj("::A",-1);
  Tcl_Obj *v[5]={Tcl_NewStringObj("namespace",-1),Tcl_NewStringObj("delete",-1),a,NULL,NULL};int n;
  if(scenario==0){v[3]=Tcl_NewStringObj("::missing",-1);n=4;}
  else if(scenario==1){v[3]=Tcl_NewStringObj("::A::q",-1);v[4]=Tcl_NewStringObj("::A",-1);n=5;}
  else{v[3]=Tcl_NewStringObj("::B",-1);n=4;}
  for(int k=0;k<n;k++)Tcl_IncrRefCount(v[k]);
  int code=Tcl_EvalObjv(i,n,v,0);Tcl_Obj *r=Tcl_GetObjResult(i);L len;const unsigned char *bytes=(const unsigned char *)Tcl_GetStringFromObj(r,&len);
  printf("{\"case\":\"%s\",\"code\":%d,\"result\":\"",label,code);for(L k=0;k<len;k++)printf("%02x",bytes[k]);
  printf("\",\"A\":%d,\"child\":%d,\"B\":%d,\"hook\":%d,\"oldBdeleted\":%d,\"newBdeleted\":%d}\n",Tcl_FindNamespace(i,"::A",NULL,0)!=NULL,Tcl_FindNamespace(i,"::A::q",NULL,0)!=NULL,Tcl_FindNamespace(i,"::B",NULL,0)!=NULL,hook_calls,old_b_deleted,new_b_deleted);
  for(int k=0;k<n;k++)Tcl_DecrRefCount(v[k]); Tcl_DeleteInterp(i);
}
#if TCL_MAJOR_VERSION >= 9
#define TRACE "trace add variable ::A::x unset dependent"
#else
#define TRACE "trace variable ::A::x u dependent"
#endif
static void trace_case(int recreate) {
  Tcl_Interp *i=Tcl_CreateInterp();
  const char *setup=recreate ?
    "namespace eval A {}; namespace eval B {}; set ::calls 0; set ::A::x 1; proc dependent {a b c} {incr ::calls; namespace delete ::B; namespace eval ::B {set fresh 1}}; " TRACE :
    "namespace eval A {}; namespace eval B {}; set ::calls 0; set ::A::x 1; proc dependent {a b c} {incr ::calls; namespace delete ::B}; " TRACE;
  if(Tcl_Eval(i,setup)!=TCL_OK){fprintf(stderr,"setup trace failed %s\n",Tcl_GetStringResult(i));return;}
  Tcl_Obj *v[4]={Tcl_NewStringObj("namespace",-1),Tcl_NewStringObj("delete",-1),Tcl_NewStringObj("::A",-1),Tcl_NewStringObj("::B",-1)};
  for(int k=0;k<4;k++)Tcl_IncrRefCount(v[k]);
  int code=Tcl_EvalObjv(i,4,v,0);L len;const unsigned char *bytes=(const unsigned char *)Tcl_GetStringFromObj(Tcl_GetObjResult(i),&len);
  printf("{\"case\":\"trace-%s-later\",\"code\":%d,\"result\":\"",recreate?"recreates":"deletes",code);for(L k=0;k<len;k++)printf("%02x",bytes[k]);
  int a=Tcl_FindNamespace(i,"::A",NULL,0)!=NULL,b=Tcl_FindNamespace(i,"::B",NULL,0)!=NULL;
  const char *calls=Tcl_GetVar(i,"::calls",0);
  printf("\",\"A\":%d,\"B\":%d,\"calls\":%s}\n",a,b,calls?calls:"-1");
  for(int k=0;k<4;k++)Tcl_DecrRefCount(v[k]);Tcl_DeleteInterp(i);
}
int main(void) {
  one("invalid-later",0,0);one("dependent-child-duplicate",1,0);
  one("callback-deletes-later",2,0);one("callback-recreates-later",3,0);
  one("raw-nul-invalid-later",0,1);one("pure-bytearray-nul-invalid-first",0,2);
  trace_case(0);trace_case(1);
  return 0;
}
