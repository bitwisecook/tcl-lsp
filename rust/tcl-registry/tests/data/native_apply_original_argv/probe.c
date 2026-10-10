#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp; typedef Jim_Obj Obj;
static Interp *fresh(void){Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);return Jim_InitStaticExtensions(i)==JIM_OK?i:NULL;}
static Obj *string(Interp *i,const char *p,int n){return Jim_NewStringObj(i,p,n);}
static void hold(Obj *o){Jim_IncrRefCount(o);}
static void release(Interp *i,Obj *o){Jim_DecrRefCount(i,o);}
static int invoke(Interp *i,int n,Obj **v){for(int k=0;k<n;k++)hold(v[k]);int c=Jim_EvalObjVector(i,n,v);for(int k=0;k<n;k++)release(i,v[k]);return c;}
static int source(Interp *i,const char *p){return Jim_Eval(i,p);}
static Obj *result(Interp *i){return Jim_GetResult(i);}
static const char *bytes(Obj *o,long *n){int size;const char *p=Jim_GetString(o,&size);*n=size;return p;}
static void destroy(Interp *i){Jim_FreeInterp(i);}
#else
#include "tcl.h"
typedef Tcl_Interp Interp; typedef Tcl_Obj Obj;
static Interp *fresh(void){Interp *i=Tcl_CreateInterp();return Tcl_Init(i)==TCL_OK?i:NULL;}
static Obj *string(Interp *i,const char *p,int n){(void)i;return Tcl_NewStringObj(p,n);}
static void hold(Obj *o){Tcl_IncrRefCount(o);}
static void release(Interp *i,Obj *o){(void)i;Tcl_DecrRefCount(o);}
static int invoke(Interp *i,int n,Obj **v){for(int k=0;k<n;k++)hold(v[k]);int c=Tcl_EvalObjv(i,n,v,0);for(int k=0;k<n;k++)release(i,v[k]);return c;}
static int source(Interp *i,const char *p){return Tcl_Eval(i,p);}
static Obj *result(Interp *i){return Tcl_GetObjResult(i);}
static const char *bytes(Obj *o,long *n){
#if TCL_MAJOR_VERSION >= 9
Tcl_Size size;
#else
int size;
#endif
const char *p=Tcl_GetStringFromObj(o,&size);*n=(long)size;return p;}
static void destroy(Interp *i){Tcl_DeleteInterp(i);}
#endif
static void hex(const char *p,long n){for(long k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);}
/* Original result/argument pointer correspondence is sampled before its getter. */
static void row(Interp *i,const char *label,int code,Obj *argument){int same=argument && result(i)==argument;long n;const char *p=bytes(result(i),&n);printf("%s|%d|%d|",label,code,same);hex(p,n);puts("");}
static void options(Interp *i,const char *label,int code){
#if !defined(JIM_PROBE) && !(TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4)
Obj *o=Tcl_GetReturnOptions(i,code);hold(o);long n;const char *p=bytes(o,&n);printf("%s_OPTIONS|%d|",label,code);hex(p,n);puts("");release(i,o);
#else
(void)i;printf("%s_OPTIONS_NOT_TESTED|%d|no-selected-C-options-api\n",label,code);
#endif
}
struct Case{const char *name;const char *value;int length;};
static const struct Case cases[]={
{"TEXT","VALUE",5},{"EMPTY","",0},{"RAW_ZERO","A\000B",3},{"ENCODED_ZERO","A\300\200B",4},{"OPAQUE_FF","A\377B",3},{"SURROGATE_D800","A\355\240\200B",5},{"SUPPLEMENTARY","A\360\237\230\200B",6}
};
int main(int argc,char **argv){(void)argc;
#ifndef JIM_PROBE
Tcl_FindExecutable(argv[0]);
#else
(void)argv;
#endif
Interp *i=fresh();if(!i)return 2;row(i,"VERSION",source(i,"info patchlevel"),NULL);destroy(i);
for(unsigned k=0;k<sizeof(cases)/sizeof(cases[0]);k++){
const struct Case *item=&cases[k];char label[128];
i=fresh();if(!i)return 2;Obj *arg=string(i,item->value,item->length);hold(arg);
printf("INPUT_%s|0|",item->name);hex(item->value,item->length);puts("");
Obj *call[]={string(i,"apply",5),string(i,"{x} {return $x}",15),arg};
snprintf(label,sizeof(label),"APPLY_%s",item->name);int c=invoke(i,3,call);row(i,label,c,arg);options(i,label,c);release(i,arg);destroy(i);
i=fresh();if(!i)return 2;arg=string(i,item->value,item->length);hold(arg);
const char *lambda="{x} {yield READY; return $x}";
Obj *coro[]={string(i,"coroutine",9),string(i,"c",1),string(i,"apply",5),string(i,lambda,(int)strlen(lambda)),arg};
c=invoke(i,5,coro);snprintf(label,sizeof(label),"COROUTINE_START_%s",item->name);row(i,label,c,arg);options(i,label,c);
if(c==0){Obj *resume[]={string(i,"c",1)};c=invoke(i,1,resume);snprintf(label,sizeof(label),"COROUTINE_RETURN_%s",item->name);row(i,label,c,arg);options(i,label,c);}
else printf("COROUTINE_RETURN_%s_NOT_ATTEMPTED|%d|initial-command-failed\n",item->name,c);
release(i,arg);destroy(i);
}
i=fresh();if(!i)return 2;int c=source(i,"proc apply args {return SHADOW}; coroutine c apply {} VALUE");row(i,"SHADOWED_APPLY_COROUTINE",c,NULL);options(i,"SHADOWED_APPLY_COROUTINE",c);destroy(i);
#ifndef JIM_PROBE
Tcl_Finalize();
#endif
return 0;}
