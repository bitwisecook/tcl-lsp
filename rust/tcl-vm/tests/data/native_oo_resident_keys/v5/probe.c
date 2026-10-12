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
static void resident(Interp*i,const char*label){int code=eval(i,"info object vars c");if(!code)inspect_list(i,label);row(i,label,code);
#ifndef JIM_PROBE
char selected[180];snprintf(selected,sizeof(selected),"%s_EVAL_EX",label);code=Tcl_EvalEx(i,"info object vars c",-1,0);if(!code)inspect_list(i,selected);row(i,selected,code);
Obj*v[]={string(i,"info",4),string(i,"object",6),string(i,"vars",4),string(i,"c",1)};snprintf(selected,sizeof(selected),"%s_OBJECT_VECTOR",label);code=invoke(i,4,v);if(!code)inspect_list(i,selected);row(i,selected,code);
#endif
}
static void namespace_read(Interp*i,const char*label,const char*name,int nameLength){
#ifndef JIM_PROBE
Obj*v[]={string(i,"info",4),string(i,"object",6),string(i,"namespace",9),string(i,"c",1)};
if(invoke(i,4,v))return;
#if TCL_MAJOR_VERSION >= 9
Tcl_Size n;
#else
int n;
#endif
const char*namespaceName=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n);char qualified[1024];
memcpy(qualified,namespaceName,n);qualified[n++]=':';qualified[n++]=':';memcpy(qualified+n,name,nameLength);n+=nameLength;qualified[n]=0;
Obj*key=string(i,qualified,n);Tcl_IncrRefCount(key);
Obj*value=Tcl_ObjGetVar2(i,key,NULL,TCL_GLOBAL_ONLY|TCL_LEAVE_ERR_MSG);
char sublabel[160];snprintf(sublabel,sizeof(sublabel),"%s_COUNTED_GET",label);if(value)object_row(i,sublabel,value);else row(i,sublabel,TCL_ERROR);
value=Tcl_GetVar2Ex(i,qualified,NULL,TCL_GLOBAL_ONLY|TCL_LEAVE_ERR_MSG);
snprintf(sublabel,sizeof(sublabel),"%s_CSTRING_GET",label);if(value)object_row(i,sublabel,value);else row(i,sublabel,TCL_ERROR);
Tcl_DecrRefCount(key);
#endif
}

static int define(Interp*i,const char*member,const char*name,int n,const char*params,const char*body,int bodyLen){
Obj*v[]={string(i,"oo::define",10),string(i,"C",1),string(i,member,(int)strlen(member)),string(i,name,n),string(i,params,(int)strlen(params)),string(i,body,bodyLen)};return invoke(i,6,v);}
static int method(Interp*i,const char*name,const char*argument,int n){Obj*v[]={string(i,"c",1),string(i,name,(int)strlen(name)),NULL};if(argument)v[2]=string(i,argument,n);return invoke(i,argument?3:2,v);}
int main(int argc,char**argv){(void)argc;
#ifndef JIM_PROBE
Tcl_FindExecutable(argv[0]);
#else
(void)argv;
#endif
Interp*i=fresh();if(!i)return 2;row(i,"VERSION",eval(i,"info patchlevel"));int availability=eval(i,"info commands ::oo::class");
#ifdef JIM_PROBE
int available;Jim_GetString(Jim_GetResult(i),&available);
#else
#if TCL_MAJOR_VERSION >= 9
Tcl_Size available;
#else
int available;
#endif
Tcl_GetStringFromObj(Tcl_GetObjResult(i),&available);
#endif
if(availability||!available){row(i,"OO_UNAVAILABLE",eval(i,"set status UNAVAILABLE"));destroy(i);return 0;}destroy(i);
const char plain[]="k",zero[]="k\0tail",qualified[]="k\0::tail",surrogate[]="k\xed\xa0\x80",opaque[]="k\xff";
const char*names[]={plain,zero,qualified,surrogate,opaque,zero};
int lengths[]={sizeof(plain)-1,sizeof(zero)-1,sizeof(qualified)-1,sizeof(surrogate)-1,sizeof(opaque)-1,sizeof(zero)-1};
const char*cases[]={"PLAIN","RAW_ZERO","QUALIFIED_AFTER_ZERO","SURROGATE","OPAQUE_FF","PREFIX_DECLARATION_NEGATIVE"};
for(int c=0;c<6;c++){
i=fresh();if(!i)return 2;if(eval(i,"oo::class create C {};C create c"))return 3;const char*declared=c==5?plain:names[c];int declaredLength=c==5?1:lengths[c];Obj*registration[]={string(i,"oo::define",10),string(i,"C",1),string(i,"variable",8),string(i,declared,declaredLength)};char label[120];
#ifdef JIM_PROBE
Jim_IncrRefCount(registration[3]);
#else
Tcl_IncrRefCount(registration[3]);
#endif
snprintf(label,sizeof(label),"%s_DECLARE",cases[c]);int code=invoke(i,4,registration);row(i,label,code);if(code){destroy(i);continue;}snprintf(label,sizeof(label),"%s_DECLARATION_BEFORE_BODY",cases[c]);object_row(i,label,registration[3]);
char script[1024];int offset=0;const char*prefix="set {";memcpy(script+offset,prefix,strlen(prefix));offset+=(int)strlen(prefix);memcpy(script+offset,names[c],lengths[c]);offset+=lengths[c];const char*suffix="} STATIC;return STATIC";memcpy(script+offset,suffix,strlen(suffix));offset+=(int)strlen(suffix);code=define(i,"method","writer",6,"",script,offset);if(code)return 4;
offset=0;prefix="return ${";memcpy(script+offset,prefix,strlen(prefix));offset+=(int)strlen(prefix);memcpy(script+offset,names[c],lengths[c]);offset+=lengths[c];script[offset++]='}';code=define(i,"method","reader",6,"",script,offset);if(code)return 4;
code=define(i,"method","observe",7,"","return [list [info locals] [info vars]]",(int)strlen("return [list [info locals] [info vars]]"));if(code)return 4;
const char*dynamic="set $target DYNAMIC;return [list [info locals] [info vars]]";code=define(i,"method","dynamic",7,"target",dynamic,(int)strlen(dynamic));if(code)return 4;dynamic="set $target";code=define(i,"method","dynamicread",11,"target",dynamic,(int)strlen(dynamic));if(code)return 4;
snprintf(label,sizeof(label),"%s_STATIC_WRITE",cases[c]);row(i,label,method(i,"writer",NULL,0));snprintf(label,sizeof(label),"%s_BEFORE_STATIC_READ",cases[c]);resident(i,label);snprintf(label,sizeof(label),"%s_NAMESPACE_BEFORE_STATIC_READ",cases[c]);namespace_read(i,label,names[c],lengths[c]);snprintf(label,sizeof(label),"%s_OBSERVE_FRAME",cases[c]);row(i,label,method(i,"observe",NULL,0));snprintf(label,sizeof(label),"%s_AFTER_FRAME_OBSERVER",cases[c]);resident(i,label);snprintf(label,sizeof(label),"%s_DECLARATION_BEFORE_READ",cases[c]);object_row(i,label,registration[3]);snprintf(label,sizeof(label),"%s_STATIC_READ",cases[c]);row(i,label,method(i,"reader",NULL,0));snprintf(label,sizeof(label),"%s_OBJECT_VARS_AFTER_STATIC",cases[c]);resident(i,label);snprintf(label,sizeof(label),"%s_NAMESPACE_AFTER_STATIC_READ",cases[c]);namespace_read(i,label,names[c],lengths[c]);snprintf(label,sizeof(label),"%s_DECLARATION_AFTER_READ",cases[c]);object_row(i,label,registration[3]);snprintf(label,sizeof(label),"%s_DYNAMIC_WRITE",cases[c]);row(i,label,method(i,"dynamic",names[c],lengths[c]));snprintf(label,sizeof(label),"%s_STATIC_READ_AFTER_DYNAMIC",cases[c]);row(i,label,method(i,"reader",NULL,0));snprintf(label,sizeof(label),"%s_DYNAMIC_READ",cases[c]);row(i,label,method(i,"dynamicread",names[c],lengths[c]));snprintf(label,sizeof(label),"%s_OBJECT_VARS_AFTER_DYNAMIC",cases[c]);row(i,label,eval(i,"info object vars c"));
#ifdef JIM_PROBE
Jim_DecrRefCount(i,registration[3]);
#else
Tcl_DecrRefCount(registration[3]);
#endif
destroy(i);
}
#ifndef JIM_PROBE
Tcl_Finalize();
#endif
return 0;}
