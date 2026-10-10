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
static int set(Interp*i,const char*name,int n,const char*value){Obj*v[]={string(i,"set",3),string(i,name,n),string(i,value,(int)strlen(value))};return invoke(i,3,v);}
static int get(Interp*i,const char*name,int n){Obj*v[]={string(i,"set",3),string(i,name,n)};return invoke(i,2,v);}
int main(int argc,char**argv){(void)argc;
#ifndef JIM_PROBE
Tcl_FindExecutable(argv[0]);
#else
(void)argv;
#endif
Interp*i=fresh();if(!i)return 2;row(i,"VERSION",eval(i,"info patchlevel"));destroy(i);
const char op0[]="-novariables\0tail",op1[]="-novariables\xc0\x80tail",op2[]="-novariables\xff";const char*options[]={op0,op1,op2};int lengths[]={sizeof(op0)-1,sizeof(op1)-1,sizeof(op2)-1};const char*labels[]={"SUBST_OPTION_RAW_ZERO","SUBST_OPTION_ENCODED_ZERO","SUBST_OPTION_FF"};
for(int k=0;k<3;k++){i=fresh();if(!i)return 2;set(i,"x",1,"VALUE");Obj*v[]={string(i,"subst",5),string(i,options[k],lengths[k]),string(i,"$x",2)};row(i,labels[k],invoke(i,3,v));destroy(i);}
const char*positive[]={"-backslashes","-commands","-variables"};for(int k=0;k<3;k++){i=fresh();if(!i)return 2;set(i,"x",1,"VALUE");Obj*v[]={string(i,"subst",5),string(i,positive[k],(int)strlen(positive[k])),string(i,"$x",2)};char label[80];snprintf(label,sizeof(label),"SUBST_POSITIVE_%d",k);row(i,label,invoke(i,3,v));destroy(i);}i=fresh();if(!i)return 2;set(i,"x",1,"VALUE");Obj*mixed[]={string(i,"subst",5),string(i,"-variables",10),string(i,"-nocommands",11),string(i,"$x",2)};row(i,"SUBST_MIXED",invoke(i,4,mixed));destroy(i);
const char name0[]="r\xff",name1[]="r\0tail",name2[]="r\xc0\x80tail";const char*names[]={name0,name1,name2};int sizes[]={sizeof(name0)-1,sizeof(name1)-1,sizeof(name2)-1};const char*cases[]={"CATCH_NAME_FF","CATCH_NAME_RAW_ZERO","CATCH_NAME_ENCODED_ZERO"};
for(int k=0;k<3;k++){i=fresh();if(!i)return 2;set(i,"r",1,"PLAIN");set(i,"r\xef\xbf\xbd",4,"REPLACEMENT");Obj*v[]={string(i,"catch",5),string(i,"error BOOM",10),string(i,names[k],sizes[k])};row(i,cases[k],invoke(i,3,v));char label[80];snprintf(label,sizeof(label),"%s_PRIMARY",cases[k]);row(i,label,get(i,names[k],sizes[k]));snprintf(label,sizeof(label),"%s_PLAIN",cases[k]);row(i,label,get(i,"r",1));snprintf(label,sizeof(label),"%s_REPLACEMENT",cases[k]);row(i,label,get(i,"r\xef\xbf\xbd",4));destroy(i);}
#ifndef JIM_PROBE
Tcl_Finalize();
#endif
return 0;}
