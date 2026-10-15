#include <stdio.h>
#include <string.h>
#include "tcl.h"
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
#include "tclInt.h"
#endif
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
struct Observation { const char *label; const char *target; Tcl_Namespace *original_parent; int callbacks; Tcl_Interp *interp; };
static void row(const char *kind,const char *label,int code,Tcl_Obj *value) {
 Count length;const char *bytes=Tcl_GetStringFromObj(value,&length);
 printf("%s|%s|%d|",kind,label,code);for(Count n=0;n<length;n++)printf("%02x",(unsigned char)bytes[n]);puts("");
}
static void boolean(const char *kind,const char *label,int value) { row(kind,label,TCL_OK,Tcl_NewIntObj(value)); }
static void script(Tcl_Interp *interp,const char *label,const char *kind,const char *source) {
 int code=Tcl_Eval(interp,source);row(kind,label,code,Tcl_GetObjResult(interp));
}
static int during(void *data,Tcl_Interp *interp,int argc,Tcl_Obj *const argv[]) {
 (void)argc;(void)argv;struct Observation *item=data;item->callbacks++;
 script(interp,item->label,"CALLER","namespace current");
 Tcl_Obj *args[]={Tcl_NewStringObj("namespace",9),Tcl_NewStringObj("eval",4),Tcl_NewStringObj(item->target,-1),Tcl_NewStringObj("variable value HELD; proc p {} {return CHILD}; namespace current",-1)};
 for(int n=0;n<4;n++)Tcl_IncrRefCount(args[n]);
 int code=Tcl_EvalObjv(interp,4,args,0);row("CREATE",item->label,code,Tcl_GetObjResult(interp));
 if(code!=TCL_OK){Tcl_Obj *tuple=Tcl_GetVar2Ex(interp,"errorCode",NULL,TCL_GLOBAL_ONLY);if(tuple)row("ERROR_CODE",item->label,code,tuple);}
 for(int n=0;n<4;n++)Tcl_DecrRefCount(args[n]);
 Tcl_Namespace *parent=Tcl_FindNamespace(interp,"::retiring",NULL,0);
 Tcl_Namespace *child=Tcl_FindNamespace(interp,item->target,NULL,0);
 boolean("PARENT_API_PRESENT",item->label,parent!=NULL);
 boolean("CHILD_API_PRESENT",item->label,child!=NULL);
 int same=0;for(Tcl_Namespace *scan=child;scan;scan=scan->parentPtr)if(scan==item->original_parent)same=1;
 boolean("ORIGINAL_ANCESTOR",item->label,same);
 char source[1024];
 snprintf(source,sizeof(source),"list [namespace exists ::retiring] [namespace exists {%s}] [info exists {%s::value}] [info commands {%s::p}]",item->target,item->target,item->target);
 script(interp,item->label,"PUBLIC_LOOKUPS",source);
 snprintf(source,sizeof(source),"list [set {%s::value}] [%s::p]",item->target,item->target);
 script(interp,item->label,"CONTENTS",source);
 snprintf(source,sizeof(source),"lsort [namespace children {%s}]",item->target);
 script(interp,item->label,"CHILDREN",source);
 snprintf(source,sizeof(source),"namespace eval {%s} {namespace current}",item->target);
 script(interp,item->label,"REENTER",source);
 boolean("SAME_CHILD",item->label,Tcl_FindNamespace(interp,item->target,NULL,0)==child);
 return TCL_OK;
}
static int sentinel(void *data,Tcl_Interp *interp,int argc,Tcl_Obj *const argv[]) {
 (void)data;(void)interp;(void)argc;(void)argv;return TCL_OK;
}
static void command_deleted(void *data) {
 struct Observation *item=data;
 (void)during(item,item->interp,0,NULL);
}
int main(int argc,char **argv) {
 (void)argc;Tcl_FindExecutable(argv[0]);
 const char *labels[]={"MISSING_CHILD","MISSING_PARENT","DEEP_CHILD","EXISTING_CHILD"};
 const char *targets[]={"::retiring::New","::retiring::New::","::retiring::New::Leaf","::retiring::Existing"};
 for(unsigned n=0;n<8;n++){
  unsigned index=n%4;int command_window=n>=4;
  Tcl_Interp *interp=Tcl_CreateInterp();if(Tcl_Init(interp)!=TCL_OK)return 2;
  if(n==0)script(interp,"VERSION","VERSION","info patchlevel");
  char label[128];snprintf(label,sizeof(label),"%s_%s",command_window?"COMMAND":"UNSET",labels[index]);
  struct Observation item={label,targets[index],NULL,0,interp};
  Tcl_CreateObjCommand(interp,"during",during,&item,NULL);
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
  const char *setup=command_window ? "namespace eval retiring {namespace eval Existing {variable before BEFORE}; variable sentinel HELD}" : "namespace eval retiring {namespace eval Existing {variable before BEFORE}; variable sentinel HELD;trace variable sentinel u ::during}";
#else
  const char *setup=command_window ? "namespace eval retiring {namespace eval Existing {variable before BEFORE}; variable sentinel HELD}" : "namespace eval retiring {namespace eval Existing {variable before BEFORE}; variable sentinel HELD;trace add variable sentinel unset ::during}";
#endif
  int code=Tcl_Eval(interp,setup);if(code!=TCL_OK){row("SETUP",item.label,code,Tcl_GetObjResult(interp));return 3;}
  if(command_window) {
   Tcl_CreateObjCommand(interp,"::retiring::sentinel_command",sentinel,&item,command_deleted);
  }
  item.original_parent=Tcl_FindNamespace(interp,"::retiring",NULL,0);
  script(interp,item.label,"DELETE","namespace delete ::retiring");
  boolean("CALLBACK_ONCE",item.label,item.callbacks==1);
  char source[1024];snprintf(source,sizeof(source),"list [namespace exists ::retiring] [namespace exists {%s}] [info exists {%s::value}] [info commands {%s::p}]",item.target,item.target,item.target);
  script(interp,item.label,"FINAL",source);
  Tcl_DeleteInterp(interp);
 }
 Tcl_Finalize();return 0;
}
