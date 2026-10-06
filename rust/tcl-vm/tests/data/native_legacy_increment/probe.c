#include <stdio.h>
#include <string.h>
#include <limits.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Obj Obj;typedef Jim_Interp Interp;typedef int Size;
#define HOLD(o) Jim_IncrRefCount(o)
#define DROP(i,o) Jim_DecrRefCount(i,o)
static Obj*newstr(Interp*i,const char*b,int n){return Jim_NewStringObj(i,b,n);}
static Obj*newnum(Interp*i,int wide,long long n){(void)wide;return Jim_NewIntObj(i,n);}
static Obj*newdouble(Interp*i,double n){return Jim_NewDoubleObj(i,n);}
static Interp*create(void){Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);return i;}
static void reset(Interp*i){Jim_SetResultString(i,"",0);}
static void put(Interp*i,Obj*n,Obj*v){Jim_SetVariable(i,n,v);}
static Obj*get(Interp*i,Obj*n){return Jim_GetVariable(i,n,0);}
static int run(Interp*i,Obj**v){return Jim_EvalObjVector(i,3,v);}
static Obj*result(Interp*i){return Jim_GetResult(i);}
static const char*str(Obj*o,Size*n){return Jim_GetString(o,n);}
static void finish(Interp*i){Jim_FreeInterp(i);}
#else
#include "tcl.h"
typedef Tcl_Obj Obj;typedef Tcl_Interp Interp;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
#define HOLD(o) Tcl_IncrRefCount(o)
#define DROP(i,o) Tcl_DecrRefCount(o)
static Obj*newstr(Interp*i,const char*b,int n){(void)i;return Tcl_NewStringObj(b,n);}
static Obj*newnum(Interp*i,int wide,long long n){(void)i;return wide?Tcl_NewWideIntObj(n):Tcl_NewLongObj(n);}
static Obj*newdouble(Interp*i,double n){(void)i;return Tcl_NewDoubleObj(n);}
static Interp*create(void){return Tcl_CreateInterp();}
static void reset(Interp*i){Tcl_ResetResult(i);}
static void put(Interp*i,Obj*n,Obj*v){Tcl_ObjSetVar2(i,n,NULL,v,0);}
static Obj*get(Interp*i,Obj*n){return Tcl_ObjGetVar2(i,n,NULL,0);}
static int run(Interp*i,Obj**v){return Tcl_EvalObjv(i,3,v,TCL_EVAL_DIRECT);}
static Obj*result(Interp*i){return Tcl_GetObjResult(i);}
static const char*str(Obj*o,Size*n){return Tcl_GetStringFromObj(o,n);}
static void finish(Interp*i){Tcl_DeleteInterp(i);}
#endif
static const char*type(Obj*o){return !o?"missing":o->typePtr?o->typePtr->name:"none";}
static void state(const char*label,Obj*o){printf("\t%s=%s,%d,%d",label,type(o),o?o->bytes!=NULL:0,o?o->refCount:0);if(o&&o->typePtr&&(strcmp(type(o),"int")==0||strcmp(type(o),"wideInt")==0))printf(",%lld",(long long)o->internalRep.wideValue);}
static Obj*make(Interp*i,int shape){switch(shape){case 0:return newstr(i,"4",1);case 1:return newnum(i,0,4);case 2:return newnum(i,1,4);case 3:return newdouble(i,4.0);case 4:return newstr(i,"1.5",3);case 5:return newnum(i,1,LLONG_MAX);case 6:return newstr(i,"BAD",3);case 7:return newstr(i,"1 + 2",5);case 8:return newdouble(i,1.0);case 9:return newdouble(i,1.5);case 10:return newnum(i,0,1);case 11:return newstr(i,"1\0BAD",5);default:return NULL;}}
int main(int argc,char**argv){(void)argc;
#ifndef USE_JIM
Tcl_FindExecutable(argv[0]);
#endif
int pairs[][2]={{0,10},{1,10},{2,10},{3,10},{4,10},{5,10},{0,8},{0,9},{0,7},{0,6},{6,6},{12,6},{12,10},{0,11},{6,10}};
for(int c=0;c<15;c++)for(int shared=0;shared<2;shared++){
 Interp*i=create();Obj*n=newstr(i,"x",1),*head=newstr(i,"incr",4),*current=make(i,pairs[c][0]),*amount=make(i,pairs[c][1]);HOLD(n);HOLD(head);HOLD(amount);if(current){put(i,n,current);if(shared)HOLD(current);}reset(i);
 printf("%d\t%d",c,shared);state("before-current",current);state("before-amount",amount);Obj*objv[]={head,n,amount};int code=run(i,objv);Obj*res=result(i); /* No string observer before physical snapshots. */
 Obj*after=get(i,n);state("after-current",after);state("after-amount",amount);if(shared)state("original-current",current);printf("\tidentity=%d\tcode=%d\tresult=",after&&after==current,code);Size len;const unsigned char*s=(const unsigned char*)str(res,&len);for(Size j=0;j<len;j++)printf("%02x",s[j]);puts("");
 if(current&&shared)DROP(i,current);DROP(i,amount);DROP(i,head);DROP(i,n);finish(i);
}
#ifndef USE_JIM
Tcl_Finalize();
#endif
return 0;}