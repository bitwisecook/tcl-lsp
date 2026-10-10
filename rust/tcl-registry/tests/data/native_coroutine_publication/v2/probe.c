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

struct Case {const char *name;const char *value;int length;};
static const struct Case cases[]={
 {"TEXT","c",1},{"EMPTY","",0},{"ROOT_EMPTY","::",2},
 {"TRAIL_REL","Q::",3},{"TRAIL_ABS","::Q::",5},
 {"RELATIVE","Q::c",4},{"ABSOLUTE","::Q::c",6},
 {"MISSING_REL","Missing::c",10},{"MISSING_ABS","::Missing::c",12},
 {"RAW_ZERO","c\000::Missing::tail",17},{"ENCODED_ZERO","c\300\200z",4},
 {"OPAQUE_FF","c\377z",3},{"SURROGATE_D800","c\355\240\200z",5},
 {"QUAL_RAW_ZERO","::N::c\000::Missing::tail",22},
 {"QUAL_ENCODED_ZERO","Q::c\300\200z",7}
};
static const char *case_label;
static Obj *original_name;
#ifdef JIM_PROBE
static int driver(Interp *i,int n,Obj *const *v){
#else
#if TCL_MAJOR_VERSION>=9
#define CB_COUNT Tcl_Size
#define create_command Tcl_CreateObjCommand2
#else
#define CB_COUNT int
#define create_command Tcl_CreateObjCommand
#endif
static int driver(void *data,Interp *i,CB_COUNT n,Obj *const *v){(void)data;
#endif
 if(n!=2)return 1;
 printf("%s_DRIVER_ORIGINAL|0|%d|\n",case_label,v[1]==original_name);
 Obj *call[]={string(i,"coroutine",9),v[1],string(i,"yield",5),string(i,"READY",5)};
 int c=invoke(i,4,call);row(i,case_label,c,NULL);options(i,case_label,c);
 char label[160];snprintf(label,sizeof(label),"%s_INVENTORY",case_label);
 row(i,label,source(i,"info commands"),NULL);
 if(c==0){Obj *resume[]={v[1],string(i,"RESUMED",7)};snprintf(label,sizeof(label),"%s_RESUME",case_label);int rc=invoke(i,2,resume);row(i,label,rc,NULL);options(i,label,rc);}
 else printf("%s_RESUME_NOT_ATTEMPTED|%d|0|initial-command-failed\n",case_label,c);
 Obj *report[]={string(i,"coroutine",9),v[1],string(i,"info",4),string(i,"coroutine",9)};
 snprintf(label,sizeof(label),"%s_TOKEN_FULLNAME",case_label);int report_code=invoke(i,4,report);row(i,label,report_code,NULL);options(i,label,report_code);
 snprintf(label,sizeof(label),"%s_MISSING_EXISTS",case_label);
 row(i,label,source(i,"namespace exists ::Missing"),NULL);
 return 0;
}
#ifndef JIM_PROBE
static Obj *callback_result;
static Obj *transported_argument;
static int error_b;
static int collector(void *data,Interp *i,CB_COUNT n,Obj *const *v){
 const char *tag=(const char *)data;char label[192];
 snprintf(label,sizeof(label),"%s_CALLBACK_%s",case_label,tag);
 if(n<2)return TCL_ERROR;
 if(callback_result)release(i,callback_result);
 callback_result=v[n-1];hold(callback_result);
 Tcl_SetObjResult(i,callback_result);row(i,label,0,transported_argument);
 if(n>=3){snprintf(label,sizeof(label),"%s_KIND_%s",case_label,tag);long size;const char *p=bytes(v[n-2],&size);printf("%s|0|0|",label);hex(p,size);puts("");}
 if(error_b && strcmp(tag,"B")==0){Tcl_SetObjResult(i,string(i,"B ERROR",7));return TCL_ERROR;}
 return TCL_OK;
}
#endif
static void transports(void){
#if !defined(JIM_PROBE) && TCL_MAJOR_VERSION>=9 && TCL_MINOR_VERSION>=1
 const struct Case *values[]={&cases[9],&cases[10],&cases[11],&cases[12]};
 for(int kind=0;kind<2;kind++)for(unsigned k=0;k<sizeof(values)/sizeof(values[0]);k++){
  const struct Case *item=values[k];Interp *i=fresh();if(!i)return;
  char label[192];snprintf(label,sizeof(label),"TRANSPORT_%s_%s",kind?"YIELDTO":"YIELD",item->name);case_label=label;
  create_command(i,"collector\377",collector,(void *)"COLLECT",NULL);
  callback_result=NULL;transported_argument=string(i,item->value,item->length);hold(transported_argument);error_b=0;
  Obj *start[]={string(i,"coroutine",9),string(i,"c",1),string(i,kind?"yieldto":"yield",kind?7:5),string(i,kind?"collector\377":"READY",kind?10:5),string(i,"READY",5)};
  int code=invoke(i,kind?5:4,start);char phase[224];snprintf(phase,sizeof(phase),"%s_START",label);row(i,phase,code,NULL);
  Obj *opaque=string(i,"r\377",2);hold(opaque);Obj *rename[]={string(i,"rename",6),string(i,"c",1),opaque};code=invoke(i,3,rename);snprintf(phase,sizeof(phase),"%s_RENAME",label);row(i,phase,code,NULL);
  Obj *probe[]={string(i,"coroprobe",9),opaque,string(i,"collector\377",10),transported_argument};code=invoke(i,4,probe);snprintf(phase,sizeof(phase),"%s_PROBE",label);row(i,phase,code,transported_argument);
  Obj *inject[]={string(i,"coroinject",10),opaque,string(i,"collector\377",10),transported_argument};code=invoke(i,4,inject);snprintf(phase,sizeof(phase),"%s_INJECT",label);row(i,phase,code,transported_argument);
  Obj *type[]={string(i,"::tcl::unsupported::corotype",28),opaque};code=invoke(i,2,type);snprintf(phase,sizeof(phase),"%s_TYPE",label);row(i,phase,code,NULL);
  Obj *resume[]={opaque,transported_argument};code=invoke(i,2,resume);snprintf(phase,sizeof(phase),"%s_RESUME",label);row(i,phase,code,transported_argument);snprintf(phase,sizeof(phase),"%s_RESUME_CALLBACK_OBJECT",label);row(i,phase,code,callback_result);
  if(callback_result){release(i,callback_result);callback_result=NULL;}release(i,opaque);release(i,transported_argument);transported_argument=NULL;destroy(i);
 }
 for(int kind=0;kind<2;kind++)for(int fail=0;fail<2;fail++){
  Interp *i=fresh();if(!i)return;char label[192];snprintf(label,sizeof(label),"ORDER_%s_%s",kind?"YIELDTO":"YIELD",fail?"B_ERROR":"SUCCESS");case_label=label;callback_result=NULL;transported_argument=NULL;error_b=fail;
  create_command(i,"A",collector,(void *)"A",NULL);create_command(i,"B",collector,(void *)"B",NULL);create_command(i,"collector\377",collector,(void *)"START",NULL);
  Obj *start[]={string(i,"coroutine",9),string(i,"c",1),string(i,kind?"yieldto":"yield",kind?7:5),string(i,kind?"collector\377":"READY",kind?10:5),string(i,"READY",5)};int code=invoke(i,kind?5:4,start);char phase[224];snprintf(phase,sizeof(phase),"%s_START",label);row(i,phase,code,NULL);
  for(int which=0;which<2;which++){Obj *inject[]={string(i,"coroinject",10),string(i,"c",1),string(i,which?"B":"A",1),string(i,which?"PB":"PA",2)};code=invoke(i,4,inject);snprintf(phase,sizeof(phase),"%s_QUEUE_%s",label,which?"B":"A");row(i,phase,code,NULL);}
  Obj *resume[]={string(i,"c",1),string(i,"RESUMED",7)};code=invoke(i,2,resume);snprintf(phase,sizeof(phase),"%s_RESUME",label);row(i,phase,code,callback_result);options(i,phase,code);
  if(callback_result){release(i,callback_result);callback_result=NULL;}destroy(i);
 }
#else
 puts("TRANSPORT_NOT_TESTED|0|0|no-C9.1-coroprobe-coroinject-provider");
 puts("ORDER_NOT_TESTED|0|0|no-C9.1-coroprobe-coroinject-provider");
#endif
}
int main(int argc,char **argv){(void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 Interp *i=fresh();if(!i)return 2;row(i,"VERSION",source(i,"info patchlevel"),NULL);destroy(i);
 for(int current=0;current<2;current++)for(unsigned k=0;k<sizeof(cases)/sizeof(cases[0]);k++){
  const struct Case *item=&cases[k];char label[160];snprintf(label,sizeof(label),"PUB_%s_%s",current?"N":"GLOBAL",item->name);case_label=label;
  i=fresh();if(!i)return 2;int setup=source(i,"namespace eval ::N {}; namespace eval ::N::Q {}; namespace eval ::Q {}");row(i,"SETUP",setup,NULL);if(setup!=0){destroy(i);return 3;}
#ifdef JIM_PROBE
 Jim_CreateCommand(i,"publicationDriver",driver,NULL,NULL);
#else
 create_command(i,"publicationDriver",driver,NULL,NULL);
#endif
  original_name=string(i,item->value,item->length);hold(original_name);printf("%s_INPUT|0|0|",label);hex(item->value,item->length);puts("");
  Obj *script_items[]={string(i,"publicationDriver",17),original_name};
#ifdef JIM_PROBE
  Obj *script=Jim_NewListObj(i,script_items,2);
#else
  Obj *script=Tcl_NewListObj(2,script_items);
#endif
  Obj *enter[]={string(i,"namespace",9),string(i,"eval",4),string(i,current?"::N":"::",current?3:2),script};int c=invoke(i,4,enter);char outer[192];snprintf(outer,sizeof(outer),"%s_OUTER",label);row(i,outer,c,NULL);release(i,original_name);original_name=NULL;destroy(i);
 }
 transports();
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
