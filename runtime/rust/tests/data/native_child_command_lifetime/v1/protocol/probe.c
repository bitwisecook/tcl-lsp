#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) { Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);if(Jim_InitStaticExtensions(i)!=JIM_OK)return NULL;return i; }
static Obj *make(Interp*i,const unsigned char*p,long n) {return Jim_NewStringObj(i,(const char*)p,(int)n);}
static void hold(Obj*o) {Jim_IncrRefCount(o);}
static void release(Interp*i,Obj*o) {Jim_DecrRefCount(i,o);}
static Obj *result(Interp*i) {return Jim_GetResult(i);}
static const char *bytes(Obj*o,long*n) {int count;const char*p=Jim_GetString(o,&count);*n=count;return p;}
static int evaluate(Interp*i,const unsigned char*p,long n) {Obj*o=make(i,p,n);hold(o);int code=Jim_EvalObj(i,o);release(i,o);return code;}
static void destroy(Interp*i) {Jim_FreeInterp(i);}
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static Interp *fresh(void) {Interp*i=Tcl_CreateInterp();if(Tcl_Init(i)!=TCL_OK)return NULL;return i;}
static Obj *make(Interp*i,const unsigned char*p,long n) {(void)i;return Tcl_NewStringObj((const char*)p,(Count)n);}
static void hold(Obj*o) {Tcl_IncrRefCount(o);}
static void release(Interp*i,Obj*o) {(void)i;Tcl_DecrRefCount(o);}
static Obj *result(Interp*i) {return Tcl_GetObjResult(i);}
static const char *bytes(Obj*o,long*n) {Count count;const char*p=Tcl_GetStringFromObj(o,&count);*n=(long)count;return p;}
static int evaluate(Interp*i,const unsigned char*p,long n) {return Tcl_EvalEx(i,(const char*)p,(Count)n,0);}
static void destroy(Interp*i) {Tcl_DeleteInterp(i);}
#endif
static const char *case_name,*phase;
static void hex(const char*p,long n) {for(long k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);}
static void object(const char*field,Obj*o) {
 if(!o){printf("VALUE|%s|%s|%s|missing\n",case_name,phase,field);return;}
 const char*type=o->typePtr?o->typePtr->name:"none";int resident=o->bytes!=NULL;long n;const char*p=bytes(o,&n);
 printf("VALUE|%s|%s|%s|%s|%d|%ld|",case_name,phase,field,type,resident,n);hex(p,n);puts("");
}
static void completion(Interp*i,int code) {
 Obj*r=result(i);hold(r);printf("CODE|%s|%s|%d\n",case_name,phase,code);object("result",r);
#ifdef JIM_PROBE
 printf("RETURN_FIELDS|%s|%s|%d|%d|%d\n",case_name,phase,i->returnCode,i->returnLevel,i->break_level);
#else
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
 Obj*options=Tcl_GetReturnOptions(i,code);hold(options);object("return-options",options);release(i,options);
#else
 printf("OPTIONS_API_UNAVAILABLE|%s|%s\n",case_name,phase);
#endif
#endif
 release(i,r);
}
typedef struct {const char*id;const unsigned char*source;long length;} Case;
#include "cases.h"
#ifndef JIM_PROBE
typedef struct {Interp*parent;Interp*original;int callback_calls;} Active;
static int replace_active(void*data,Interp*i,Count argc,Obj*const*argv) {
 (void)argc;(void)argv;Active*active=(Active*)data;active->callback_calls++;
 printf("ACTIVE|%s|callback-before|parent-selected|%d|original-deleted|%d\n",case_name,i==active->parent,Tcl_InterpDeleted(active->original));
 const char*recreate="interp delete kid; interp create kid";
 int code=evaluate(i,(const unsigned char*)recreate,(long)strlen(recreate));
 Interp*replacement=Tcl_GetSlave(active->parent,"kid");
 printf("ACTIVE|%s|callback-after|code|%d|original-deleted|%d|replacement-present|%d|different-allocation|%d\n",case_name,code,Tcl_InterpDeleted(active->original),replacement!=NULL,replacement!=NULL&&replacement!=active->original);
 if(code==TCL_OK)Tcl_SetObjResult(i,Tcl_NewStringObj("REPLACED",8));
 return code;
}
static void active_api(void) {
 case_name="active-original-allocation";Interp*i=fresh();if(!i)exit(2);
 phase="setup";const char*setup="interp create kid; kid eval {set before OLD}";int code=evaluate(i,(const unsigned char*)setup,(long)strlen(setup));completion(i,code);if(code){destroy(i);return;}
 Active active={i,Tcl_GetSlave(i,"kid"),0};if(!active.original){destroy(i);return;}
 Tcl_Preserve(active.original);
#if TCL_MAJOR_VERSION >= 9
 Tcl_CreateObjCommand2(i,"__replace",replace_active,&active,NULL);
#else
 Tcl_CreateObjCommand(i,"__replace",replace_active,&active,NULL);
#endif
 phase="alias-setup";const char*alias="interp alias kid swap {} __replace";code=evaluate(i,(const unsigned char*)alias,(long)strlen(alias));completion(i,code);
 if(code==TCL_OK){phase="active-evaluation";const char*script="kid eval {set replacement [swap]; list $before $replacement}";code=evaluate(i,(const unsigned char*)script,(long)strlen(script));completion(i,code);}
 phase="after-return";const char*after="list [interp exists kid] [kid eval {info exists before}]";code=evaluate(i,(const unsigned char*)after,(long)strlen(after));completion(i,code);
 printf("ACTIVE|%s|after-return|callback-count|%d|original-deleted|%d\n",case_name,active.callback_calls,Tcl_InterpDeleted(active.original));
 Tcl_Release(active.original);destroy(i);
}
#endif
int main(int argc,char**argv) {
#ifndef JIM_PROBE
 (void)argc;Tcl_FindExecutable(argv[0]);
#else
 (void)argc;(void)argv;
#endif
 case_name="startup";phase="version";Interp*i=fresh();if(!i)return 2;completion(i,evaluate(i,(const unsigned char*)"info patchlevel",15));destroy(i);
 for(size_t k=0;k<sizeof(cases)/sizeof(cases[0]);k++){
  case_name=cases[k].id;phase="direct";i=fresh();if(!i)return 2;Obj*source=make(i,cases[k].source,cases[k].length);hold(source);object("original-source",source);completion(i,evaluate(i,cases[k].source,cases[k].length));release(i,source);destroy(i);
 }
#ifdef JIM_PROBE
 printf("API_NOT_TESTED|active-original-allocation|C-public-child-interpreter-API-only\n");
#else
 active_api();
#endif
 return 0;
}
