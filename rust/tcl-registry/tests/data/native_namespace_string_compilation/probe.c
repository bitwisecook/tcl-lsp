#include <stdio.h>
#include <string.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
typedef int Size;
static Obj *string(Interp *i,const char *p,int n){return Jim_NewStringObj(i,p,n);}
static void hold(Obj *o){Jim_IncrRefCount(o);}
static void release(Interp *i,Obj *o){Jim_DecrRefCount(i,o);}
static int invoke(Interp *i,int n,Obj **v){return Jim_EvalObjVector(i,n,v);}
static Obj *result(Interp *i){return Jim_GetResult(i);}
static const char *bytes(Obj *o,Size *n){return Jim_GetString(o,n);}
static int setup(Interp *i,const char *s){return Jim_Eval(i,s);}
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
static Obj *string(Interp *i,const char *p,int n){(void)i;return Tcl_NewStringObj(p,n);}
static void hold(Obj *o){Tcl_IncrRefCount(o);}
static void release(Interp *i,Obj *o){(void)i;Tcl_DecrRefCount(o);}
static int invoke(Interp *i,int n,Obj **v){return Tcl_EvalObjv(i,n,v,TCL_EVAL_DIRECT);}
static Obj *result(Interp *i){return Tcl_GetObjResult(i);}
static const char *bytes(Obj *o,Size *n){return Tcl_GetStringFromObj(o,n);}
static int setup(Interp *i,const char *s){return Tcl_EvalEx(i,s,-1,TCL_EVAL_DIRECT);}
#endif
static void hex(const char *p,Size n){for(Size k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);}
static const char *inputs[]={"a::tail", "no_separator", "a:::tail", "a::", "a::\xed\xa0\x80", "a::\xed\xa0\x81", "a::\xff", "a\0::tail", "", ":"};
static const int lengths[]={7,12,8,3,6,6,4,8,0,1};
int main(void){
#ifdef USE_JIM
 Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);if(Jim_InitStaticExtensions(i)!=JIM_OK)return 2;
#else
 Tcl_FindExecutable("namespace-tail-counted-control");Interp *i=Tcl_CreateInterp();if(Tcl_Init(i)!=TCL_OK)return 2;
#endif
 Obj *version[2]={string(i,"info",-1),string(i,"patchlevel",-1)};for(int k=0;k<2;k++)hold(version[k]);int version_code=invoke(i,2,version);Size version_length;const char *version_bytes=bytes(result(i),&version_length);printf("V|%d|%lld|",version_code,(long long)version_length);hex(version_bytes,version_length);puts("");for(int k=0;k<2;k++)release(i,version[k]);
 if(setup(i,"proc originalTail {value} {namespace tail $value}")!=0)return 3;
 for(int form=0;form<2;form++)for(int c=0;c<10;c++){
  Obj *input=string(i,inputs[c],lengths[c]);Obj *v[3]={string(i,form?"originalTail":"namespace",-1),form?input:string(i,"tail",-1),input};int count=form?2:3;
  for(int k=0;k<count;k++)hold(v[k]);int code=invoke(i,count,v);Obj *r=result(i);hold(r);
  printf("R|%d|%d|%d|%s|%d|",form,c,code,r->typePtr?r->typePtr->name:"NULL",r==input);Size n;const char *p=bytes(r,&n);printf("%lld|",(long long)n);hex(p,n);puts("");
  printf("I|%d|%d|%s|",form,c,input->typePtr?input->typePtr->name:"NULL");p=bytes(input,&n);printf("%lld|",(long long)n);hex(p,n);puts("");
  release(i,r);for(int k=0;k<count;k++)release(i,v[k]);
 }
#ifndef USE_JIM
 Obj *d[3]={string(i,"tcl::unsupported::disassemble",-1),string(i,"proc",-1),string(i,"originalTail",-1)};for(int k=0;k<3;k++)hold(d[k]);int code=invoke(i,3,d);Size n;const char *p=bytes(result(i),&n);printf("D|%d|%lld|",code,(long long)n);hex(p,n);puts("");for(int k=0;k<3;k++)release(i,d[k]);Tcl_DeleteInterp(i);Tcl_Finalize();
#else
 puts("D|unsupported|Jim-has-no-C-Tail-compiler");Jim_FreeInterp(i);
#endif
 return 0;
}
