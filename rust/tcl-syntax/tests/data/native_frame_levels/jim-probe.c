#include "jim.h"
#include <stdio.h>
#include <math.h>
static void hex(Jim_Interp*i,Jim_Obj*o){int n;const unsigned char*s=(const unsigned char*)Jim_GetString(o,&n);for(int k=0;k<n;k++)printf("%02x",s[k]);(void)i;}
static int probe(Jim_Interp*i,int argc,Jim_Obj*const*argv){(void)argc;(void)argv;
const char *texts[]={"1","1\0X","#1","#1\0X","-1","+1","01","1.0","NaN","2147483648","4294967295","0","bad","#1 ","#9223372036854775808","#-0x8000000000000000","#-0","#0o1"};
for(int k=0;k<21;k++){
Jim_Obj*o=k<18?Jim_NewStringObj(i,texts[k],k==1?3:k==3?4:-1):(k==18?Jim_NewIntObj(i,1):Jim_NewDoubleObj(i,k==19?1.0:NAN));
Jim_IncrRefCount(o);const char*before=o->typePtr?o->typePtr->name:"string";
Jim_Obj*words[]={Jim_NewStringObj(i,"uplevel",-1),o,Jim_NewStringObj(i,"info level",-1)};
Jim_IncrRefCount(words[0]);Jim_IncrRefCount(words[2]);int code=Jim_EvalObjVector(i,3,words);
printf("{\"case\":%d,\"before\":\"%s\",\"after\":\"%s\",\"code\":%d,\"result\":\"",k,before,o->typePtr?o->typePtr->name:"string",code);hex(i,Jim_GetResult(i));printf("\"}\n");
Jim_DecrRefCount(i,words[0]);Jim_DecrRefCount(i,words[2]);Jim_DecrRefCount(i,o);Jim_SetResultString(i,"",0);
}
return JIM_OK;}
int main(void){Jim_Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_CreateCommand(i,"probe",probe,NULL,NULL);int code=Jim_Eval(i,"proc outer {} {inner}; proc inner {} {probe}; outer");if(code!=JIM_OK){fprintf(stderr,"%s\n",Jim_String(Jim_GetResult(i)));return 1;}Jim_FreeInterp(i);return 0;}
