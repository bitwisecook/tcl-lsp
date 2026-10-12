/* Pinned Jim primitive getter failures, preserving original raw-string bytes. */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "jim.h"
static const char *cases[]={"x","k 1","1\0X","\xff\0after","NaN","1e400","1e-400","18446744073709551616","true","","aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"};
static const int lengths[]={1,3,3,7,3,5,6,20,4,0,72};
static Jim_Obj *retained;
static int getter(Jim_Interp *interp,int argc,Jim_Obj *const *argv) {
    int kind=atoi(Jim_String(argv[1])),at=atoi(Jim_String(argv[2]));
    jim_wide wide;double value;int boolean,code;(void)argc;
    Jim_SetVariableStr(interp,"errorCode",Jim_NewStringObj(interp,"PROBE BEFORE",-1));
    retained=Jim_NewStringObj(interp,cases[at],lengths[at]);Jim_IncrRefCount(retained);
    if(kind==0)code=Jim_GetWide(interp,retained,&wide);
    else if(kind==1)code=Jim_GetDouble(interp,retained,&value);
    else code=Jim_GetBoolean(interp,retained,&boolean);
    return code;
}
static void hex(const char *bytes,int len){int i;for(i=0;i<len;i++)printf("%02x",(unsigned char)bytes[i]);}
int main(void) {
    int kind,at;
    for(kind=0;kind<3;kind++)for(at=0;at<11;at++) {
        Jim_Interp *interp=Jim_CreateInterp();char script[80];int code,len;const char *bytes;
        Jim_CreateCommand(interp,"get",getter,NULL,NULL);
        snprintf(script,sizeof(script),"get %d %d",kind,at);code=Jim_Eval(interp,script);
        bytes=Jim_GetString(Jim_GetResult(interp),&len);
        printf("getter=%d storage=0 case=%d code=%d cache=%s message=",kind,at,code,retained->typePtr?retained->typePtr->name:"string");hex(bytes,len);
        printf(" errorCode=");bytes=Jim_GetString(Jim_GetVariableStr(interp,"errorCode",JIM_NONE),&len);hex(bytes,len);
        printf(" input=");bytes=Jim_GetString(retained,&len);hex(bytes,len);putchar('\n');
        Jim_DecrRefCount(interp,retained);Jim_FreeInterp(interp);
    }
    return 0;
}
