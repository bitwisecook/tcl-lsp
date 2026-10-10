#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK)return NULL; return i; }
static Obj *make(Interp*i,const unsigned char*p,long n) { return Jim_NewStringObj(i,(const char*)p,(int)n); }
static void hold(Obj*o) { Jim_IncrRefCount(o); }
static void release(Interp*i,Obj*o) { Jim_DecrRefCount(i,o); }
static Obj *result(Interp*i) { return Jim_GetResult(i); }
static const char *bytes(Obj*o,long*n) { int count;const char*p=Jim_GetString(o,&count);*n=count;return p; }
static int evaluate(Interp*i,Obj*script) { return Jim_EvalObj(i,script); }
static int invoke(Interp*i,int n,Obj**v) {for(int k=0;k<n;k++)hold(v[k]);int code=Jim_EvalObjVector(i,n,v);for(int k=0;k<n;k++)release(i,v[k]);return code;}
static Obj *variable(Interp*i,const char*name) { return Jim_GetVariableStr(i,name,JIM_NONE); }
static void destroy(Interp*i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static Interp *fresh(void) { Interp*i=Tcl_CreateInterp();if(Tcl_Init(i)!=TCL_OK)return NULL;return i; }
static Obj *make(Interp*i,const unsigned char*p,long n) { (void)i;return Tcl_NewStringObj((const char*)p,(Count)n); }
static void hold(Obj*o) { Tcl_IncrRefCount(o); }
static void release(Interp*i,Obj*o) { (void)i;Tcl_DecrRefCount(o); }
static Obj *result(Interp*i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj*o,long*n) { Count count;const char*p=Tcl_GetStringFromObj(o,&count);*n=(long)count;return p; }
static int evaluate(Interp*i,Obj*script) { long n;const char*p=bytes(script,&n);return Tcl_EvalEx(i,p,(Count)n,0); }
static int invoke(Interp*i,int n,Obj**v) {for(int k=0;k<n;k++)hold(v[k]);int code=Tcl_EvalObjv(i,n,v,0);for(int k=0;k<n;k++)release(i,v[k]);return code;}
static Obj *variable(Interp*i,const char*name) {return Tcl_GetVar2Ex(i,name,NULL,TCL_GLOBAL_ONLY);}
static void destroy(Interp*i) { Tcl_DeleteInterp(i); }
#endif
static const char *case_name,*variant_name,*channel_name,*operation_name;
static void hex(const char*p,long n) { for(long k=0;k<n;k++)printf("%02x",(unsigned char)p[k]); }
static void object(const char*field,Obj*o) {
 if(!o){printf("VALUE|%s|%s|%s|%s|%s|missing\n",case_name,variant_name,channel_name,operation_name,field);return;}
 const char*type=o->typePtr?o->typePtr->name:"none";int resident=o->bytes!=NULL;long n;const char*p=bytes(o,&n);
 printf("VALUE|%s|%s|%s|%s|%s|%s|%d|%ld|",case_name,variant_name,channel_name,operation_name,field,type,resident,n);hex(p,n);puts("");
}
static void completion(Interp*i,int code) {
 Obj*value=result(i);hold(value);
 printf("CODE|%s|%s|%s|%s|%d\n",case_name,variant_name,channel_name,operation_name,code);
 object("result",value);
#ifdef JIM_PROBE
 printf("RETURN_FIELDS|%s|%s|%s|%s|%d|%d|%d\n",case_name,variant_name,channel_name,operation_name,i->returnCode,i->returnLevel,i->break_level);
#else
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
 Obj*options=Tcl_GetReturnOptions(i,code);hold(options);object("return-options",options);release(i,options);
#else
 printf("OPTIONS_API_UNAVAILABLE|%s|%s|%s|%s\n",case_name,variant_name,channel_name,operation_name);
#endif
#endif
 object("errorCode",variable(i,"errorCode")); object("errorInfo",variable(i,"errorInfo"));
 object("seen",variable(i,"seen"));object("ticks",variable(i,"ticks"));release(i,value);
}
#ifdef JIM_PROBE
static int collector(Interp*i,int argc,Jim_Obj*const*argv) {
#else
static int collector(void*data,Interp*i,Count argc,Obj*const*argv) {
 (void)data;
#endif
 printf("ARGC|%s|%s|%s|%ld\n",case_name,variant_name,channel_name,(long)argc);
 for(long k=1;k<(long)argc;k++) {char field[32];snprintf(field,sizeof(field),"argv%ld",k);object(field,argv[k]);
#ifndef JIM_PROBE
 Count count;const Tcl_UniChar*u=Tcl_GetUnicodeFromObj(argv[k],&count);
 printf("UNITS|%s|%s|%s|%ld|%ld|",case_name,variant_name,channel_name,k,(long)count);
 for(long n=0;n<(long)count;n++)printf("%x%s",(unsigned)u[n],n+1<(long)count?",":"");puts("");
#endif
 }
#ifdef JIM_PROBE
 Jim_SetEmptyResult(i);return JIM_OK;
#else
 Tcl_ResetResult(i);return TCL_OK;
#endif
}
static Obj *input(Interp*i,const unsigned char*p,long n,const char*directory,const char*suffix) {
#ifdef JIM_PROBE
 (void)directory;(void)suffix;
#else
 if(strcmp(channel_name,"document-readchars")==0) {
  char path[4096];snprintf(path,sizeof(path),"%s/%s/%s.%s",directory,case_name,variant_name,suffix);
  Tcl_Channel channel=Tcl_OpenFileChannel(i,path,"r",0);
  if(!channel||Tcl_SetChannelOption(i,channel,"-encoding","utf-8")||Tcl_SetChannelOption(i,channel,"-translation","auto"))return NULL;
  const char*settings[]={"-encoding","-translation"};
  for(int k=0;k<2;k++){Tcl_DString value;Tcl_DStringInit(&value);int code=Tcl_GetChannelOption(i,channel,settings[k],&value);printf("CHANNEL_OPTION|%s|%s|%s|%s|%s|%d|",case_name,variant_name,channel_name,suffix,settings[k],code);hex(Tcl_DStringValue(&value),Tcl_DStringLength(&value));puts("");Tcl_DStringFree(&value);if(code)return NULL;}
  Obj*o=Tcl_NewObj();hold(o);Count read=Tcl_ReadChars(channel,o,-1,0);
  printf("READCHARS|%s|%s|%s|%s|%ld\n",case_name,variant_name,channel_name,suffix,(long)read);
  if(read<0||Tcl_Close(i,channel)!=TCL_OK){release(i,o);return NULL;}return o;
 }
#endif
 Obj*o=make(i,p,n);hold(o);return o;
}
typedef struct {const char*id;const unsigned char*original;long original_length;const unsigned char*original_argv;long original_argv_length;const unsigned char*candidate;long candidate_length;const unsigned char*candidate_argv;long candidate_argv_length;} Case;
#include "cases.h"
int main(int argc,char**argv) {
 if(argc!=2){fputs("usage: probe input-directory\n",stderr);return 2;}
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#endif
 case_name="startup";variant_name="provider";channel_name="counted-native";operation_name="version";
 Interp*i=fresh();if(!i)return 2;Obj*version=make(i,(const unsigned char*)"info patchlevel",15);hold(version);completion(i,evaluate(i,version));release(i,version);destroy(i);
 for(size_t c=0;c<sizeof(cases)/sizeof(cases[0]);c++)for(int variant=0;variant<2;variant++)for(int channel=0;channel<2;channel++) {
  case_name=cases[c].id;variant_name=variant?"candidate":"original";channel_name=channel?"document-readchars":"counted-native";
#ifdef JIM_PROBE
  if(channel){printf("CHANNEL_NOT_TESTED|%s|%s|document-readchars|C-public-ReadChars-only\n",case_name,variant_name);continue;}
#endif
  const unsigned char*p=variant?cases[c].candidate:cases[c].original;long length=variant?cases[c].candidate_length:cases[c].original_length;
  const unsigned char*a=variant?cases[c].candidate_argv:cases[c].original_argv;long an=variant?cases[c].candidate_argv_length:cases[c].original_argv_length;
  operation_name="direct";i=fresh();if(!i)return 2;Obj*script=input(i,p,length,argv[1],"source");if(!script)return 3;object("source",script);completion(i,evaluate(i,script));release(i,script);destroy(i);
  operation_name="argv-collector";i=fresh();if(!i)return 2;
#ifdef JIM_PROBE
  Jim_CreateCommand(i,"__argv",collector,NULL,NULL);
#else
  Tcl_CreateObjCommand(i,"__argv",collector,NULL,NULL);
#endif
  script=input(i,a,an,argv[1],"argv");if(!script)return 3;object("source",script);completion(i,evaluate(i,script));release(i,script);destroy(i);
  operation_name="catch-object-context";i=fresh();if(!i)return 2;script=input(i,p,length,argv[1],"source");if(!script)return 3;
#if !defined(JIM_PROBE) && TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
  printf("OPTIONS_API_UNAVAILABLE|%s|%s|%s|catch-object-context\n",case_name,variant_name,channel_name);
#else
  Obj*v[]={make(i,(const unsigned char*)"catch",5),script,make(i,(const unsigned char*)"message",7),make(i,(const unsigned char*)"options",7)};
  completion(i,invoke(i,4,v));object("caught-result",variable(i,"message"));object("caught-options",variable(i,"options"));
#endif
  release(i,script);destroy(i);
 }
 return 0;
}
