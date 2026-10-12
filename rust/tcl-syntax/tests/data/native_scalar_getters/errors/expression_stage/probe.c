/* Seeded direct expression API versus interpreter propagation. */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "tcl.h"
#if TCL_MAJOR_VERSION >= 9
#define ProbeSize Tcl_Size
#else
#define ProbeSize int
#endif
static const char *cases[]={"x","1\0X","1.5\0X","NaN","08","true"};
static const int lengths[]={1,3,5,3,2,4};
static Tcl_Obj *inside_result;
static char *inside_code;
static char *inside_options_code;
static void hex(const char *bytes,ProbeSize len){ProbeSize at;for(at=0;at<len;at++)printf("%02x",(unsigned char)bytes[at]);}
static int probe(ClientData data,Tcl_Interp *interp,int objc,Tcl_Obj *const objv[]){
 int route=atoi(Tcl_GetString(objv[1])),at=atoi(Tcl_GetString(objv[2])),code,boolean;
 Tcl_Obj *result=NULL,*expression;const char *ec;(void)data;(void)objc;
 Tcl_SetVar2Ex(interp,"v",NULL,Tcl_NewStringObj(cases[at],lengths[at]),0);
 Tcl_SetErrorCode(interp,"PROBE","BEFORE",NULL);
 expression=Tcl_NewStringObj(route==1?"$v":"$v + 1",-1);Tcl_IncrRefCount(expression);
 if(route==0){code=Tcl_ExprObj(interp,expression,&result);if(code==TCL_OK)Tcl_SetObjResult(interp,result);}
 else if(route==1){code=Tcl_ExprBooleanObj(interp,expression,&boolean);if(code==TCL_OK)Tcl_SetObjResult(interp,Tcl_NewIntObj(boolean));}
 else code=Tcl_Eval(interp,"expr {$v + 1}");
 Tcl_DecrRefCount(expression);
 inside_result=Tcl_DuplicateObj(Tcl_GetObjResult(interp));Tcl_IncrRefCount(inside_result);
#if TCL_MAJOR_VERSION >= 9 || TCL_MINOR_VERSION >= 5
 {Tcl_Obj *options=Tcl_GetReturnOptions(interp,code),*value=NULL,*key=Tcl_NewStringObj("-errorcode",-1);Tcl_IncrRefCount(options);Tcl_IncrRefCount(key);(void)Tcl_DictObjGet(NULL,options,key,&value);ec=value?Tcl_GetString(value):"ABSENT";inside_options_code=malloc(strlen(ec)+1);strcpy(inside_options_code,ec);inside_code=malloc(strlen(ec)+1);strcpy(inside_code,ec);Tcl_DecrRefCount(key);Tcl_DecrRefCount(options);}
#else
 ec=Tcl_GetVar(interp,"errorCode",TCL_GLOBAL_ONLY);if(!ec)ec="ABSENT";
 inside_code=malloc(strlen(ec)+1);strcpy(inside_code,ec);
 inside_options_code=malloc(strlen(inside_code)+1);strcpy(inside_options_code,inside_code);
#endif
 return code;
}
int main(int argc,char **argv){int route,at;(void)argc;Tcl_FindExecutable(argv[0]);
 for(route=0;route<3;route++)for(at=0;at<6;at++){
  Tcl_Interp *interp=Tcl_CreateInterp();char script[64];int code;ProbeSize len;const char *bytes,*ec;
  Tcl_CreateObjCommand(interp,"probe",probe,NULL,NULL);snprintf(script,sizeof(script),"probe %d %d",route,at);code=Tcl_Eval(interp,script);
  printf("route=%d case=%d code=%d inside=",route,at,code);bytes=Tcl_GetStringFromObj(inside_result,&len);hex(bytes,len);
  printf(" insideCode=");hex(inside_code,(ProbeSize)strlen(inside_code));printf(" insideOptionsCode=");hex(inside_options_code,(ProbeSize)strlen(inside_options_code));printf(" result=");bytes=Tcl_GetStringFromObj(Tcl_GetObjResult(interp),&len);hex(bytes,len);
  ec=Tcl_GetVar(interp,"errorCode",TCL_GLOBAL_ONLY);if(!ec)ec="ABSENT";printf(" propagatedCode=");hex(ec,(ProbeSize)strlen(ec));puts("");
  free(inside_code);free(inside_options_code);Tcl_DecrRefCount(inside_result);Tcl_DeleteInterp(interp);
 }Tcl_Finalize();return 0;}
