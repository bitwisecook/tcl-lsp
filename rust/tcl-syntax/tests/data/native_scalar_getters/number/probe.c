
#include <stdio.h>
#include <string.h>
#ifdef USE_JIM
#include "jim.h"
int main(void){puts("{\"capability\":\"CNumberBignumUnavailable\"}");return 0;}
#else
#include "tclInt.h"
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
int main(void){puts("{\"capability\":\"CNumberBignumUnavailable\"}");return 0;}
#else
#include "tclTomMath.h"
#if TCL_MAJOR_VERSION>=9
#define NUMBER Tcl_GetNumberFromObj
#else
#define NUMBER TclGetNumberFromObj
#endif
static void hex(Tcl_Obj *o){int n;
#if TCL_MAJOR_VERSION>=9
Tcl_Size length;const unsigned char*s=(void*)Tcl_GetStringFromObj(o,&length);n=(int)length;
#else
const unsigned char*s=(void*)Tcl_GetStringFromObj(o,&n);
#endif
putchar('"');for(int j=0;j<n;j++)printf("%02x",s[j]);putchar('"');}
static Tcl_Obj *make(int shape){
static const char *text[]={"1","1.5","NaN","08","184467440737095516160000","true","", " 2 ","1\0x","1\xc0\x80x"};
if(shape<10){int n=shape==8?3:shape==9?4:(int)strlen(text[shape]);return Tcl_NewStringObj(text[shape],n);}
if(shape==10)return Tcl_NewDoubleObj(1.5);
if(shape==11){Tcl_Obj*o=Tcl_NewStringObj("NaN",3);double d;Tcl_GetDoubleFromObj(NULL,o,&d);return o;}
if(shape==12)return Tcl_NewWideIntObj(7);
if(shape==13){Tcl_Obj*o=Tcl_NewStringObj("true",4);int b;Tcl_GetBooleanFromObj(NULL,o,&b);return o;}
Tcl_Obj*o=Tcl_NewStringObj("184467440737095516160000",24);mp_int big;
Tcl_GetBignumFromObj(NULL,o,&big);mp_clear(&big);
if(shape==15){Tcl_InvalidateStringRep(o);o->bytes=ckalloc(1);o->bytes[0]=0;o->length=0;}
return o;}
int main(void){setvbuf(stdout,NULL,_IONBF,0);
for(int shape=0;shape<16;shape++)for(int stage=0;stage<3;stage++){
Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj*o=make(shape);Tcl_IncrRefCount(o);Tcl_SetObjResult(i,Tcl_NewStringObj("SEED",4));Tcl_SetErrorCode(i,"SENTINEL",NULL);
printf("{\"shape\":%d,\"stage\":%d,\"before_type\":\"%s\",\"before_resident\":%d,\"before_length\":%d",shape,stage,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,(int)(o->bytes?o->length:-1));
int code,type=-1;void*number=NULL;
if(stage==0)code=NUMBER(NULL,o,&number,&type);
else if(stage==1){mp_int big;code=Tcl_GetBignumFromObj(i,o,&big);if(code==TCL_OK)mp_clear(&big);}
else {Tcl_Obj*a=Tcl_NewIntObj(1);Tcl_IncrRefCount(a);code=TclIncrObj(i,o,a);Tcl_DecrRefCount(a);}
printf(",\"code\":%d,\"number_type\":%d,\"after_type\":\"%s\",\"after_resident\":%d,\"after_length\":%d,\"result\":",code,type,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,(int)(o->bytes?o->length:-1));
Tcl_Obj*result=Tcl_GetObjResult(i);Tcl_IncrRefCount(result);Tcl_Obj*options=Tcl_GetReturnOptions(i,code);Tcl_IncrRefCount(options);hex(result);printf(",\"options\":");hex(options);printf(",\"object_after_window\":");hex(o);puts("}");Tcl_DecrRefCount(options);Tcl_DecrRefCount(result);Tcl_DecrRefCount(o);Tcl_DeleteInterp(i);
}return 0;}
#endif
#endif
