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
static const char*type(Obj*o){return o->typePtr?o->typePtr->name:"none";}
static void hex(Obj*o){Count n;const char*p=GET_BYTES(o,&n);for(Count k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);}
static Obj*input(Interp*i,int kind){
 static const char rawzero[]={'A',0,'B'};
 static const char opaque[]={(char)0xff,(char)0xed,(char)0xa0,(char)0x80};
 static const unsigned char binary[]={0,255};
 switch(kind){
  case 0:return NEW_STRING(i,"A",1);
  case 1:
#ifdef USE_JIM
   return NULL;
#else
   {Tcl_UniChar units[]={0,0xd800};return Tcl_NewUnicodeObj(units,2);}
#endif
  case 2:return NEW_STRING(i,rawzero,3);
  case 3:
#ifdef USE_JIM
   return NULL;
#else
   return Tcl_NewByteArrayObj(binary,2);
#endif
  case 4:return NEW_STRING(i,opaque,4);
  case 5:return NEW_LIST(i);
  case 6:return NEW_DOUBLE(i);
  case 7:return NEW_STRING(i,"",0);
 }
 return NULL;
}
int main(int argc,char**argv){
 (void)argc;
#ifdef USE_JIM
 Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);
#else
 Tcl_FindExecutable(argv[0]);Interp*i=Tcl_CreateInterp();if(Tcl_Init(i)!=0)return 2;
#endif
 Obj*version=NEW_STRING(i,"info patchlevel",-1);HOLD(version);if(EVAL_WORD(i,version)!=0)return 3;
 printf("{\"row\":\"startup\",\"patchlevel_hex\":\"");hex(RESULT(i));puts("\"}");DROP(i,version);
 for(int kind=0;kind<8;kind++)for(int mode=0;mode<2;mode++){
  Obj*x=input(i,kind);if(!x){printf("{\"kind\":%d,\"mode\":%d,\"input_available\":false}\n",kind,mode);continue;}HOLD(x);
  Obj*y=input(i,kind==3?3:kind==1?7:0);if(!y)return 4;HOLD(y);
  if(install(i,"x",x)!=0||install(i,"y",y)!=0)return 5;
  const char*text=mode==0?"set result \"${x}${y}\"":"${x}${y}";
  Obj*source=NEW_STRING(i,text,-1);HOLD(source);Obj*head=NEW_STRING(i,"subst",5);HOLD(head);
  for(int repetition=0;repetition<2;repetition++){
   const char*xt=type(x),*yt=type(y),*st=type(source);int xr=x->bytes!=NULL,yr=y->bytes!=NULL,sr=source->bytes!=NULL;
   Obj*v[]={head,source};int code=mode==0?EVAL_WORD(i,source):EVAL_ARGV(i,2,v);Obj*r=RESULT(i);
   const char*rt=type(r);int rr=r->bytes!=NULL;const char*xa=type(x),*ya=type(y),*sa=type(source);int xra=x->bytes!=NULL,yra=y->bytes!=NULL,sra=source->bytes!=NULL;
   printf("{\"kind\":%d,\"mode\":%d,\"repetition\":%d,\"input_available\":true,\"code\":%d,\"x_before_type\":\"%s\",\"x_before_resident\":%d,\"y_before_type\":\"%s\",\"y_before_resident\":%d,\"source_before_type\":\"%s\",\"source_before_resident\":%d,\"result_before_type\":\"%s\",\"result_before_resident\":%d,\"same_x\":%d,\"same_y\":%d,\"x_after_type\":\"%s\",\"x_after_resident\":%d,\"y_after_type\":\"%s\",\"y_after_resident\":%d,\"source_after_type\":\"%s\",\"source_after_resident\":%d,\"result_hex\":\"",kind,mode,repetition,code,xt,xr,yt,yr,st,sr,rt,rr,r==x,r==y,xa,xra,ya,yra,sa,sra);hex(r);puts("\"}");
  }
  DROP(i,head);DROP(i,source);DROP(i,x);DROP(i,y);
 }
#ifdef USE_JIM
 Jim_FreeInterp(i);
#else
 Tcl_DeleteInterp(i);Tcl_Finalize();
#endif
 return 0;
}
