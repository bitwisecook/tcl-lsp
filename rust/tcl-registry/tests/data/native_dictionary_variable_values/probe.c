#ifdef USE_JIM
#include <jim.h>
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
#define HOLD(o) Jim_IncrRefCount(o)
#define DROP(i,o) Jim_DecrRefCount(i,o)
#define NEW_STRING(i,p,n) Jim_NewStringObj(i,p,n)
#define NEW_LIST(i) Jim_NewListObj(i,NULL,0)
#define NEW_DOUBLE(i) Jim_NewDoubleObj(i,1.5)
#define RESULT(i) Jim_GetResult(i)
#define GET_BYTES(o,n) Jim_GetString(o,n)
#define EVAL_WORD(i,o) Jim_EvalObj(i,o)
#define EVAL_ARGV(i,n,v) Jim_EvalObjVector(i,n,v)
static int install(Interp*i,const char*n,Obj*v){return Jim_SetVariableStr(i,n,v);}
#else
#include <tcl.h>
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
#define HOLD(o) Tcl_IncrRefCount(o)
#define DROP(i,o) Tcl_DecrRefCount(o)
#define NEW_STRING(i,p,n) Tcl_NewStringObj(p,n)
#define NEW_LIST(i) Tcl_NewListObj(0,NULL)
#define NEW_DOUBLE(i) Tcl_NewDoubleObj(1.5)
#define RESULT(i) Tcl_GetObjResult(i)
#define GET_BYTES(o,n) Tcl_GetStringFromObj(o,n)
#define EVAL_WORD(i,o) Tcl_EvalObjEx(i,o,0)
#define EVAL_ARGV(i,n,v) Tcl_EvalObjv(i,n,v,0)
static int install(Interp*i,const char*n,Obj*v){return Tcl_SetVar2Ex(i,n,NULL,v,0)?0:1;}
#endif
#include <stdio.h>
#include <string.h>
#ifdef USE_JIM
typedef int Count;
#endif

static void dump(Obj*o){Count n;const unsigned char*p=(const unsigned char*)GET_BYTES(o,&n);for(Count k=0;k<n;k++)printf("%02x",p[k]);}
int main(int argc,char**argv){
(void)argc;
#ifdef USE_JIM
 Interp*i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); Jim_InitStaticExtensions(i);
 Jim_Eval(i,"info patchlevel");
#else
 Tcl_FindExecutable(argv[0]); Interp*i=Tcl_CreateInterp(); if(Tcl_Init(i)!=0)return 2; Tcl_Eval(i,"info patchlevel");
#endif
 printf("{\"row\":\"startup\",\"patchlevel_hex\":\"");dump(RESULT(i));printf("\"}\n");
 static const char* labels[]={"simple","duplicate-last","raw-zero","modified-zero","surrogate","colon","odd","missing","mutated"};
 static const char roots[][32]={"k FIRST","k FIRST k LAST",{'k',0,'t',' ','R','A','W'}, {'k',(char)0xc0,(char)0x80,'t',' ','M','O','D'}, {'k',(char)0xed,(char)0xa0,(char)0x80,' ','S','U','R'},"::k COLON","k","k VALUE","k FIRST"};
 static const int lengths[]={7,14,7,8,8,9,1,7,7};
 static const char keys[][8]={"k","k",{'k',0,'t'},{'k',(char)0xc0,(char)0x80,'t'},{'k',(char)0xed,(char)0xa0,(char)0x80},"::k","k","other","k"};
 static const int klengths[]={1,1,3,4,4,3,1,5,1};
 for(int k=0;k<9;k++){
  Obj*root=NEW_STRING(i,roots[k],lengths[k]);HOLD(root); Obj*key=NEW_STRING(i,keys[k],klengths[k]);HOLD(key);
  if(install(i,"a",root)!=0||install(i,"index",key)!=0)return 3;
  const char*script=k==8?"set a {k LAST}; set a($index)":"set a($index)";
  Obj*source=NEW_STRING(i,script,(int)strlen(script));HOLD(source);int code=EVAL_WORD(i,source);
  printf("{\"case\":\"%s\",\"code\":%d,\"result_hex\":\"",labels[k],code);dump(RESULT(i));printf("\"}\n");
  DROP(i,source);DROP(i,key);DROP(i,root);
 }
#ifdef USE_JIM
 Jim_FreeInterp(i);
#else
 Tcl_DeleteInterp(i);Tcl_Finalize();
#endif
 return 0;
}
