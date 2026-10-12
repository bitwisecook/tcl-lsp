#include <stdio.h>
#include <string.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Obj Obj;typedef Jim_Interp Interp;typedef int Size;
static int fail(Interp*i,int n,Obj*const*v){(void)n;if(strcmp(Jim_String(v[1]),"inner")==0)return Jim_Eval(i,"error INNER");Jim_SetResultString(i,"FAIL",4);return JIM_ERR;}
#define HOLD(o) Jim_IncrRefCount(o)
#define DROP(i,o) Jim_DecrRefCount(i,o)
#else
#include "tcl.h"
typedef Tcl_Obj Obj;typedef Tcl_Interp Interp;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
static int fail(void*d,Interp*i,Size n,Obj*const*v){(void)d;(void)n;if(strcmp(Tcl_GetString(v[1]),"inner")==0)return Tcl_Eval(i,"error INNER");Tcl_SetObjResult(i,Tcl_NewStringObj("FAIL",4));return TCL_ERROR;}
#define HOLD(o) Tcl_IncrRefCount(o)
#define DROP(i,o) Tcl_DecrRefCount(o)
#endif
int main(int argc,char**argv){(void)argc;
#ifndef USE_JIM
Tcl_FindExecutable(argv[0]);
#endif
for(int mode=0;mode<2;mode++)for(int shape=0;shape<4;shape++){
#ifdef USE_JIM
Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_CreateCommand(i,"fail",fail,NULL,NULL);Obj*head=Jim_NewStringObj(i,"fail",4),*kind=Jim_NewStringObj(i,mode?"inner":"direct",-1),*value=shape==0?Jim_NewIntObj(i,17):Jim_NewStringObj(i,(shape==1||shape==3)?"A\0B":"#hash",(shape==1||shape==3)?3:5);
#else
Interp*i=Tcl_CreateInterp();
#if TCL_MAJOR_VERSION >= 9
Tcl_CreateObjCommand2(i,"fail",fail,NULL,NULL);
#else
Tcl_CreateObjCommand(i,"fail",fail,NULL,NULL);
#endif
Obj*head=Tcl_NewStringObj("fail",4),*kind=Tcl_NewStringObj(mode?"inner":"direct",-1),*value=shape==0?Tcl_NewLongObj(17):Tcl_NewStringObj((shape==1||shape==3)?"A\0B":"#hash",(shape==1||shape==3)?3:5);
#endif
HOLD(head);HOLD(kind);HOLD(value);int before=value->bytes!=NULL;
#ifdef USE_JIM
Obj*tail=Jim_NewStringObj(i,"TAIL",4);
#else
Obj*tail=Tcl_NewStringObj("TAIL",4);
#endif
HOLD(tail);Obj*words[]={head,kind,value,tail};
#ifdef USE_JIM
int code=Jim_EvalObjVector(i,shape==3?4:3,words);Obj*info=Jim_GetGlobalVariableStr(i,"errorInfo",0);
#else
int code=Tcl_EvalObjv(i,shape==3?4:3,words,TCL_EVAL_DIRECT);Obj*info=Tcl_GetVar2Ex(i,"errorInfo",NULL,TCL_GLOBAL_ONLY);
#endif
printf("%d\t%d\tbefore=%d\tafter=%d\tcode=%d\tinfo=",mode,shape,before,value->bytes!=NULL,code);Size len=0;const char*s="";
if(info){
#ifdef USE_JIM
s=Jim_GetString(info,&len);
#else
s=Tcl_GetStringFromObj(info,&len);
#endif
}for(Size j=0;j<len;j++)printf("%02x",(unsigned char)s[j]);puts("");DROP(i,tail);DROP(i,value);DROP(i,kind);DROP(i,head);
#ifdef USE_JIM
Jim_FreeInterp(i);
#else
Tcl_DeleteInterp(i);
#endif
}
#ifndef USE_JIM
Tcl_Finalize();
#endif
return 0;}
