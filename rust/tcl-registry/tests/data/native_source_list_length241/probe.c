#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
static Interp *fresh(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK)return NULL; return i; }
static int evaluate(Interp *i,const char *s,size_t n) { Jim_Obj *o=Jim_NewStringObj(i,s,(int)n); Jim_IncrRefCount(o); int code=Jim_EvalObj(i,o); Jim_DecrRefCount(i,o); return code; }
static void report(Interp *i,const char *tag,int code) { int n; const unsigned char *s=(const unsigned char *)Jim_GetString(Jim_GetResult(i),&n); printf("%s|%d|",tag,code); for(int k=0;k<n;k++)printf("%02x",s[k]); puts(""); fflush(stdout); }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static Interp *fresh(void) { Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK)return NULL; return i; }
static int evaluate(Interp *i,const char *s,size_t n) { return Tcl_EvalEx(i,s,(Count)n,0); }
static void report(Interp *i,const char *tag,int code) { Count n; const unsigned char *s=(const unsigned char *)Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n); printf("%s|%d|",tag,code); for(Count k=0;k<n;k++)printf("%02x",s[k]); puts(""); fflush(stdout); }
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
#endif

static void original_operands(Interp *i) {
 const char *names[2]={"first","second"};
 for(int k=0;k<2;k++) {
#ifdef JIM_PROBE
  Jim_Obj *o=Jim_GetGlobalVariableStr(i,names[k],JIM_ERRMSG);
#else
  Tcl_Obj *o=Tcl_GetVar2Ex(i,names[k],NULL,TCL_GLOBAL_ONLY|TCL_LEAVE_ERR_MSG);
#endif
  if(!o) { printf("OPERAND|%s|MISSING\n",names[k]); continue; }
  printf("OPERAND|%s|%s\n",names[k],o->typePtr?o->typePtr->name:"none");
 }
}
static int original_source_length(void) {
#ifdef JIM_PROBE
 Interp *i=fresh(); if(!i)return 3;
 Jim_Obj *value=Jim_NewStringObj(i,"A  B",4);
 Jim_Obj *filename=Jim_NewStringObj(i,"source-check.tcl",16);
 Jim_IncrRefCount(value); Jim_IncrRefCount(filename);
 Jim_SetSourceInfo(i,value,filename,17);
 printf("SOURCE_BEFORE|%s|%d\n",value->typePtr?value->typePtr->name:"none",value->bytes!=NULL);
 int length=Jim_ListLength(i,value);
 int n; const unsigned char *bytes=(const unsigned char *)Jim_GetString(value,&n);
 printf("SOURCE_AFTER|%s|%d|%d|",value->typePtr?value->typePtr->name:"none",length,value->bytes!=NULL);
 for(int k=0;k<n;k++)printf("%02x",bytes[k]); puts("");
 Jim_Obj *member=Jim_ListGetIndex(i,value,0);
 if(!member)return 4;
 int line; Jim_Obj *sourcefile=Jim_GetSourceInfo(i,member,&line);
 bytes=(const unsigned char *)Jim_GetString(sourcefile,&n);
 printf("SOURCE_CHILD|%s|%d|",member->typePtr?member->typePtr->name:"none",line);
 for(int k=0;k<n;k++)printf("%02x",bytes[k]); puts("");
 Jim_DecrRefCount(i,value); Jim_DecrRefCount(i,filename); destroy(i);
#else
 puts("SOURCE_LENGTH|NOT_APPLICABLE");
#endif
 return 0;
}
int main(int argc,char **argv) {
 if(argc!=1)return 2;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#endif
 Interp *i=fresh(); if(!i)return 3;
 report(i,"VERSION",evaluate(i,"info patchlevel",15));
 static const char *sources[3]={
  "concat word {}",
  "set first {A  B}; set second C; concat $first $second",
  "llength $first; llength $second; concat $first $second"
 };
 static const char *tags[3]={"ORIGINAL_0","ORIGINAL_1","ORIGINAL_2"};
 for(int k=0;k<3;k++) {
  if(k==2)original_operands(i);
  report(i,tags[k],evaluate(i,sources[k],strlen(sources[k])));
 }
 destroy(i);
 return original_source_length();
}
