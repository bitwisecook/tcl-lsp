/* Actual primitive getter failure after original storage materialization. */
#include <errno.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tcl.h"
#if TCL_MAJOR_VERSION >= 9
#define ProbeSize Tcl_Size
#else
#define ProbeSize int
#endif
static const char *cases[]={"x","k 1","1\0X","\xff\0after","NaN","1e400","1e-400","18446744073709551616","true","","aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","08","-09","08.0","0xG","1.5","1.5\0X","1.5                                                                      "};
static const int lengths[]={1,3,3,7,3,5,6,20,4,0,72,2,3,4,3,3,5,73};
static Tcl_Obj *retained;
static Tcl_Obj *primitive_result;
static int getter(ClientData data,Tcl_Interp *interp,int objc,Tcl_Obj *const objv[]) {
    int kind,storage,at,code,boolean; Tcl_WideInt wide; double value;
    (void)data;(void)objc;
    kind=atoi(Tcl_GetString(objv[1])); storage=atoi(Tcl_GetString(objv[2]));at=atoi(Tcl_GetString(objv[3]));
    retained=(storage==1)?Tcl_NewByteArrayObj((const unsigned char *)cases[at],lengths[at]):Tcl_NewStringObj(cases[at],lengths[at]);
    Tcl_IncrRefCount(retained);
    if(storage==2) (void)Tcl_GetDoubleFromObj(NULL,retained,&value);
    if(storage==3) (void)Tcl_GetBooleanFromObj(NULL,retained,&boolean);
    Tcl_SetErrorCode(interp,"PROBE","BEFORE",NULL);
    if(kind==0)code=Tcl_GetWideIntFromObj(interp,retained,&wide);
    else if(kind==1)code=Tcl_GetDoubleFromObj(interp,retained,&value);
    else code=Tcl_GetBooleanFromObj(interp,retained,&boolean);
    primitive_result=Tcl_DuplicateObj(Tcl_GetObjResult(interp));
    Tcl_IncrRefCount(primitive_result);
    return code;
}
static void hex(const char *bytes,ProbeSize len){ProbeSize i;for(i=0;i<len;i++)printf("%02x",(unsigned char)bytes[i]);}
int main(int argc,char **argv) {
    int kind,storage,at;(void)argc;Tcl_FindExecutable(argv[0]);
    for(kind=0;kind<3;kind++)for(storage=0;storage<4;storage++)for(at=0;at<18;at++) {
        Tcl_Interp *interp=Tcl_CreateInterp(); char script[80];int code;ProbeSize len;const char *bytes,*ec;
        Tcl_CreateObjCommand(interp,"get",getter,NULL,NULL);
        snprintf(script,sizeof(script),"get %d %d %d",kind,storage,at);code=Tcl_Eval(interp,script);
        bytes=Tcl_GetStringFromObj(Tcl_GetObjResult(interp),&len);
        printf("getter=%d storage=%d case=%d code=%d cache=%s message=",kind,storage,at,code,retained->typePtr?retained->typePtr->name:"string");hex(bytes,len);
        ec=Tcl_GetVar(interp,"errorCode",TCL_GLOBAL_ONLY);printf(" errorCode=");if(ec)hex(ec,(ProbeSize)strlen(ec));else hex("NONE",4);
        bytes=Tcl_GetStringFromObj(retained,&len);printf(" input=");hex(bytes,len);
        bytes=Tcl_GetStringFromObj(primitive_result,&len);printf(" primitive=");hex(bytes,len);putchar('\n');
        Tcl_DecrRefCount(primitive_result);
        Tcl_DecrRefCount(retained);Tcl_DeleteInterp(interp);
    }
    Tcl_Finalize();return 0;
}
