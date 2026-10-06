#include <stdio.h>
#include <string.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Obj Obj;
typedef Jim_Interp EngineInterp;
#define RET_ERR JIM_ERR
#define RET_OK JIM_OK
#define SIZE int
static Obj *str(EngineInterp *i,const char *s,int n){return Jim_NewStringObj(i,s,n);}
static int fail(EngineInterp *i,int c,Obj *const *v){(void)c;(void)v;Jim_SetResultString(i,"FAILED",-1);return JIM_ERR;}
static int nested(EngineInterp *i,int c,Obj *const *v){(void)c;(void)v;return Jim_Eval(i,"error CHILD");}
static int logged(EngineInterp *i,int c,Obj *const *v){return fail(i,c,v);}
static int pass(EngineInterp *i,int c,Obj *const *v){(void)i;(void)c;(void)v;return JIM_OK;}
#else
#include "tcl.h"
typedef Tcl_Obj Obj;
typedef Tcl_Interp EngineInterp;
#define RET_ERR TCL_ERROR
#define RET_OK TCL_OK
#if TCL_MAJOR_VERSION >= 9
#define SIZE Tcl_Size
#else
#define SIZE int
#endif
static Obj *str(EngineInterp *i,const char *s,int n){(void)i;return Tcl_NewStringObj(s,n);}
static int fail(ClientData d,EngineInterp *i,SIZE c,Obj *const v[]){(void)d;(void)c;(void)v;Tcl_SetObjResult(i,Tcl_NewStringObj("FAILED",-1));return TCL_ERROR;}
static int nested(ClientData d,EngineInterp *i,SIZE c,Obj *const v[]){(void)d;(void)c;(void)v;return Tcl_EvalEx(i,"error CHILD",-1,0);}
static int logged(ClientData d,EngineInterp *i,SIZE c,Obj *const v[]){int result=fail(d,i,c,v);Tcl_LogCommandInfo(i,"retained_source","retained_source",15);return result;}
static int pass(ClientData d,EngineInterp *i,SIZE c,Obj *const v[]){(void)d;(void)i;(void)c;(void)v;return TCL_OK;}
#endif
static void hex(const char *p,int n){int k;for(k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);}
int main(void){int k;const char *heads[]={"return","break","continue","error","list"};
for(k=0;k<10;k++){
 EngineInterp *i;Obj *member,*arg,*head,*v[2],*r;int code,has;const char *type,*rb;SIZE n;char script[96];
#ifdef USE_JIM
 i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_CreateCommand(i,"fail_original",fail,NULL,NULL);Jim_CreateCommand(i,"nested_original",nested,NULL,NULL);Jim_CreateCommand(i,"pass_original",pass,NULL,NULL);Jim_CreateCommand(i,"logged_original",logged,NULL,NULL);
 member=str(i,"A B",3);arg=Jim_NewListObj(i,&member,1);head=str(i,heads[k%5],-1);Jim_IncrRefCount(arg);Jim_IncrRefCount(head);Jim_SetVariableStr(i,"arg",arg);v[0]=head;v[1]=arg;
 if(k<5)code=Jim_EvalObjVector(i,2,v);else{sprintf(script,"%s $arg",heads[k%5]);code=Jim_Eval(i,script);}r=Jim_GetResult(i);has=arg->bytes!=NULL;type=r->typePtr?r->typePtr->name:"none";
#else
 i=Tcl_CreateInterp();
#if TCL_MAJOR_VERSION >= 9
 Tcl_CreateObjCommand2(i,"fail_original",fail,NULL,NULL);Tcl_CreateObjCommand2(i,"nested_original",nested,NULL,NULL);Tcl_CreateObjCommand2(i,"pass_original",pass,NULL,NULL);Tcl_CreateObjCommand2(i,"logged_original",logged,NULL,NULL);
#else
 Tcl_CreateObjCommand(i,"fail_original",fail,NULL,NULL);Tcl_CreateObjCommand(i,"nested_original",nested,NULL,NULL);Tcl_CreateObjCommand(i,"pass_original",pass,NULL,NULL);Tcl_CreateObjCommand(i,"logged_original",logged,NULL,NULL);
#endif
 member=str(i,"A B",3);arg=Tcl_NewListObj(1,&member);head=str(i,heads[k%5],-1);Tcl_IncrRefCount(arg);Tcl_IncrRefCount(head);Tcl_SetVar2Ex(i,"arg",NULL,arg,0);v[0]=head;v[1]=arg;
 if(k<5)code=Tcl_EvalObjv(i,2,v,0);else{sprintf(script,"%s $arg",heads[k%5]);code=Tcl_EvalEx(i,script,-1,0);}r=Tcl_GetObjResult(i);has=arg->bytes!=NULL;type=r->typePtr?r->typePtr->name:"none";
#endif
 printf("%d\t%d\t%d\t%s\t",k,code,has,type);
#ifdef USE_JIM
 rb=Jim_GetString(r,&n);hex(rb,(int)n);printf("\n");Jim_DecrRefCount(i,head);Jim_DecrRefCount(i,arg);Jim_FreeInterp(i);
#else
 rb=Tcl_GetStringFromObj(r,&n);hex(rb,(int)n);printf("\n");Tcl_DecrRefCount(head);Tcl_DecrRefCount(arg);Tcl_DeleteInterp(i);
#endif
}return 0;}
