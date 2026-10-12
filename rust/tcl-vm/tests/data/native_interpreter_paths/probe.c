#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) {Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);if(Jim_InitStaticExtensions(i)!=JIM_OK)return NULL;return i;}
static Obj *string(Interp *i,const char*p,int n){return Jim_NewStringObj(i,p,n);}
static int eval(Interp*i,const char*p){return Jim_Eval(i,p);}
static int invoke(Interp*i,int n,Obj**v){for(int k=0;k<n;k++)Jim_IncrRefCount(v[k]);int c=Jim_EvalObjVector(i,n,v);for(int k=0;k<n;k++)Jim_DecrRefCount(i,v[k]);return c;}
static void row(Interp*i,const char*l,int c){int n;const char*p=Jim_GetString(Jim_GetResult(i),&n);printf("%s|%d|",l,c);for(int k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);puts("");}
static void destroy(Interp*i){Jim_FreeInterp(i);}
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
static Interp *fresh(void){Interp*i=Tcl_CreateInterp();if(Tcl_Init(i)!=TCL_OK)return NULL;return i;}
static Obj *string(Interp*i,const char*p,int n){(void)i;return Tcl_NewStringObj(p,n);}
static int eval(Interp*i,const char*p){return Tcl_Eval(i,p);}
static int invoke(Interp*i,int n,Obj**v){for(int k=0;k<n;k++)Tcl_IncrRefCount(v[k]);int c=Tcl_EvalObjv(i,n,v,0);for(int k=0;k<n;k++)Tcl_DecrRefCount(v[k]);return c;}
static void row(Interp*i,const char*l,int c){
#if TCL_MAJOR_VERSION >= 9
Tcl_Size n;
#else
int n;
#endif
const char*p=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n);printf("%s|%d|",l,c);for(long k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);puts("");}
static void destroy(Interp*i){Tcl_DeleteInterp(i);}
#endif


static void object_row(Interp*i,const char*label,Obj*object){
#ifdef JIM_PROBE
int length;const char*bytes=Jim_GetString(object,&length);
#else
#if TCL_MAJOR_VERSION >= 9
Tcl_Size length;
#else
int length;
#endif
const char*bytes=Tcl_GetStringFromObj(object,&length);
#endif
printf("%s|0|",label);for(long n=0;n<length;n++)printf("%02x",(unsigned char)bytes[n]);puts("");}
static void inspect_list(Interp*i,const char*label){
#ifndef JIM_PROBE
Obj*element=NULL;char sublabel[160];
#if TCL_MAJOR_VERSION >= 9
Tcl_Size length=0;
#else
int length=0;
#endif
int code=Tcl_ListObjLength(i,Tcl_GetObjResult(i),&length);
printf("%s_COUNT|%d|%ld\n",label,code,(long)length);
for(long n=0;code==TCL_OK&&n<length;n++){code=Tcl_ListObjIndex(i,Tcl_GetObjResult(i),n,&element);if(code||!element)break;snprintf(sublabel,sizeof(sublabel),"%s_LEAF_%ld",label,n);object_row(i,sublabel,element);}
#endif
}
static int call(Interp*i,const char*sub,Obj*path,const char*tail){
Obj*v[4]={string(i,"interp",6),string(i,sub,(int)strlen(sub)),path,NULL};
if(tail[0])v[3]=string(i,tail,(int)strlen(tail));
return invoke(i,tail[0]?4:3,v);
}
static void one(const char*label,const char*name,int length,const char*lookup,int lookupLength){
Interp*i=fresh();if(!i)return;
Obj*path=string(i,name,length);
#ifdef JIM_PROBE
Jim_IncrRefCount(path);
#else
Tcl_IncrRefCount(path);
#endif
char selected[150];int code=call(i,"create",path,"");
snprintf(selected,sizeof(selected),"%s_CREATE",label);row(i,selected,code);
#ifndef JIM_PROBE
printf("%s_CREATE_SAME|%d|%d\n",label,code,Tcl_GetObjResult(i)==path);
#endif
Obj*address=string(i,lookup,lookupLength);
#ifdef JIM_PROBE
Jim_IncrRefCount(address);
#else
Tcl_IncrRefCount(address);
#endif
code=call(i,"exists",path,"");snprintf(selected,sizeof(selected),"%s_EXISTS_ORIGINAL",label);row(i,selected,code);
code=call(i,"exists",address,"");snprintf(selected,sizeof(selected),"%s_EXISTS_ADDRESS",label);row(i,selected,code);
Obj*v[]={string(i,"interp",6),string(i,"children",8)};code=invoke(i,2,v);
snprintf(selected,sizeof(selected),"%s_CHILDREN",label);if(!code)inspect_list(i,selected);row(i,selected,code);
code=call(i,"eval",address,"list CHILD");snprintf(selected,sizeof(selected),"%s_EVAL",label);row(i,selected,code);
code=call(i,"delete",address,"");snprintf(selected,sizeof(selected),"%s_DELETE",label);row(i,selected,code);
#ifdef JIM_PROBE
Jim_DecrRefCount(i,address);Jim_DecrRefCount(i,path);
#else
Tcl_DecrRefCount(address);Tcl_DecrRefCount(path);
#endif
destroy(i);
}
int main(void){Interp*i=fresh();if(!i)return 2;
#ifdef JIM_PROBE
int code=eval(i,"info patchlevel");row(i,"VERSION",code);
code=eval(i,"interp create named");row(i,"NAMED_PATH_UNAVAILABLE",code);destroy(i);return 0;
#else
int code=Tcl_EvalEx(i,"info patchlevel",-1,0);row(i,"VERSION",code);destroy(i);
one("RAW_ZERO","k\0tail",6,"k\0other",7);
one("ENCODED_ZERO","k\xc0\x80tail",7,"k\xc0\x80tail",7);
one("OPAQUE_FF","k\xff",2,"k\xff",2);
one("SURROGATE","k\xed\xa0\x80",4,"k\xed\xa0\x80",4);
one("SINGLETON_BRACED","{with space}",12,"{{with space}}",14);
one("EMPTY","",0,"{}",2);
one("MALFORMED","{",1,"{",1);
return 0;
#endif
}
