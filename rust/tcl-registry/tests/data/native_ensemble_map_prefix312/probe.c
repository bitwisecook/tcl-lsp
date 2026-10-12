#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
typedef int Count;
static Obj *string(Interp *i,const char *s,Count n){return Jim_NewStringObj(i,s,n);}
static Obj *list(Interp *i,Count n,Obj **v){return Jim_NewListObj(i,v,n);}
static void hold(Obj *o){Jim_IncrRefCount(o);}
static void release(Interp *i,Obj *o){Jim_DecrRefCount(i,o);}
static int vector(Interp *i,Count n,Obj **v){return Jim_EvalObjVector(i,n,v);}
static int source(Interp *i,const char *s){return Jim_Eval(i,s);}
static Obj *result(Interp *i){return Jim_GetResult(i);}
static const char *bytes(Obj *o,Count *n){return Jim_GetString(o,n);}
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static Obj *string(Interp *i,const char *s,Count n){(void)i;return Tcl_NewStringObj(s,n);}
static Obj *list(Interp *i,Count n,Obj **v){(void)i;return Tcl_NewListObj(n,v);}
static void hold(Obj *o){Tcl_IncrRefCount(o);}
static void release(Interp *i,Obj *o){(void)i;Tcl_DecrRefCount(o);}
static int vector(Interp *i,Count n,Obj **v){return Tcl_EvalObjv(i,n,v,0);}
static int source(Interp *i,const char *s){return Tcl_EvalEx(i,s,(Count)strlen(s),0);}
static Obj *result(Interp *i){return Tcl_GetObjResult(i);}
static const char *bytes(Obj *o,Count *n){return Tcl_GetStringFromObj(o,n);}
#endif
static void hex(const char *s,Count n){for(Count k=0;k<n;k++)printf("%02x",(unsigned char)s[k]);}
static void report(Interp *i,const char *tag,int code){Count n;const char *s=bytes(result(i),&n);printf("%s|%d|",tag,code);hex(s,n);puts("");fflush(stdout);}
static int selected;
static int original_vector(Interp *i){
 static const char relative[]={'p',0,'t','a','i','l'};
 static const char rooted[]={':',':','r','2','2','8','6','_','m','a','p','_','N',':',':','p',0,'t','a','i','l'};
 int absolute=(selected==1 || selected==3);Obj *head=string(i,absolute?rooted:relative,absolute?(Count)sizeof rooted:(Count)sizeof relative);hold(head);
 Count n;const char *s=bytes(head,&n);printf("INPUT|%lld|",(long long)n);hex(s,n);puts("");
 Obj *members[2]={head,string(i,"FIRST",5)};Obj *prefix=list(i,2,members);hold(prefix);
 Obj *pairs[2]={string(i,"go",2),prefix};Obj *map=list(i,2,pairs);hold(map);
 Obj *words[7]={string(i,"namespace",9),string(i,"ensemble",8),string(i,selected>=2&&selected<4?"configure":"create",-1),string(i,selected>=2&&selected<4?"::r2286_map_E":"-command",-1),NULL,NULL,NULL};
 Count count;
 if(selected>=2&&selected<4){words[4]=string(i,"-map",4);words[5]=map;count=6;}
 else{words[4]=string(i,"::r2286_map_E",-1);words[5]=string(i,"-map",4);words[6]=map;count=7;}
 for(Count k=0;k<count;k++)hold(words[k]);int code=vector(i,count,words);for(Count k=0;k<count;k++)release(i,words[k]);
 release(i,map);release(i,prefix);release(i,head);return code;
}
#ifdef JIM_PROBE
static int callback(Interp *i,int n,Obj *const *v){(void)n;(void)v;return original_vector(i);}
#else
static int callback(void *data,Interp *i,int n,Obj *const *v){(void)data;(void)n;(void)v;return original_vector(i);}
#endif
int main(int argc,char **argv){
 if(argc!=2)return 2;selected=atoi(argv[1]);if(selected<0||selected>4)return 2;
#ifdef JIM_PROBE
 Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);if(Jim_InitStaticExtensions(i)!=JIM_OK)return 3;
 Jim_CreateCommand(i,"::r2286_original_map_callback",callback,NULL,NULL);
#else
 Tcl_FindExecutable(argv[0]);Interp *i=Tcl_CreateInterp();if(Tcl_Init(i)!=TCL_OK)return 3;
 Tcl_CreateObjCommand(i,"::r2286_original_map_callback",callback,NULL,NULL);
#endif
 report(i,"VERSION",source(i,"info patchlevel"));
 report(i,"SETUP",source(i,"proc ::p args {list GLOBAL $args}; namespace eval ::r2286_map_N {proc p args {list N $args}}; namespace eval ::r2286_map_Q {proc p args {list Q $args}}"));
 if(selected>=2&&selected<4)report(i,"INITIAL",source(i,"namespace eval ::r2286_map_N {namespace ensemble create -command ::r2286_map_E -map {go {::r2286_map_N::p INITIAL}}}"));
 int code=source(i,selected==4?"::r2286_original_map_callback":selected>=2?"namespace eval ::r2286_map_Q {::r2286_original_map_callback}":"namespace eval ::r2286_map_N {::r2286_original_map_callback}");report(i,"OPERATION",code);
 Obj *query[5]={string(i,"namespace",9),string(i,"ensemble",8),string(i,"configure",9),string(i,"::r2286_map_E",-1),string(i,"-map",4)};
 for(int k=0;k<5;k++)hold(query[k]);code=vector(i,5,query);Obj *map=result(i);hold(map);
 if(code==0){
  Obj *prefix=NULL,*head=NULL;Obj *key=string(i,"go",2);hold(key);
#ifdef JIM_PROBE
  int found=Jim_DictKey(i,map,key,&prefix,JIM_NONE);if(found==0&&prefix)head=Jim_ListGetIndex(i,prefix,0);
#elif TCL_MAJOR_VERSION>8 || TCL_MINOR_VERSION>=5
  int found=Tcl_DictObjGet(i,map,key,&prefix);if(found==0&&prefix)Tcl_ListObjIndex(i,prefix,0,&head);
#endif
  if(head){Count n;const char *s=bytes(head,&n);printf("STORED_HEAD|%lld|",(long long)n);hex(s,n);puts("");}
  else puts("STORED_HEAD|MISSING");release(i,key);
 }
 report(i,"MAP_QUERY",code);release(i,map);for(int k=0;k<5;k++)release(i,query[k]);
 Obj *call[2]={string(i,"::r2286_map_E",-1),string(i,"go",2)};for(int k=0;k<2;k++)hold(call[k]);report(i,"CALL",vector(i,2,call));for(int k=0;k<2;k++)release(i,call[k]);
#ifdef JIM_PROBE
 Jim_FreeInterp(i);
#else
 Tcl_DeleteInterp(i);
#endif
 return 0;
}
