#include <tcl.h>
#include <stdio.h>
#include <string.h>
#include <math.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
static void bytes(Tcl_Obj *o){Len n;const unsigned char *s=(const unsigned char*)Tcl_GetStringFromObj(o,&n);putchar('"');for(Len j=0;j<n;j++)printf("%02x",s[j]);putchar('"');}
int main(void){Tcl_Interp*i=Tcl_CreateInterp();const char*words[]={"1\0X","2147483648","4294967295","4294967296","-4294967295","9223372036854775808","1.0","NaN","08","0x1","bad"};for(int n=0;n<14;n++){Tcl_Obj*o;if(n<11)o=Tcl_NewStringObj(words[n],n==0?3:-1);else if(n==11)o=Tcl_NewDoubleObj(1.0);else if(n==12)o=Tcl_NewWideIntObj(1);else o=Tcl_NewDoubleObj(NAN);Tcl_IncrRefCount(o);Tcl_SetErrorCode(i,"SEEDED","CODE",NULL);int value=777;int code=Tcl_GetIntFromObj(i,o,&value);printf("{\"case\":%d,\"code\":%d,\"value\":%d,\"cache\":\"%s\",\"message\":",n,code,value,o->typePtr?o->typePtr->name:"string");bytes(Tcl_GetObjResult(i));printf(",\"error_code\":");
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
Tcl_Obj *opts=Tcl_GetReturnOptions(i,code),*ec=NULL,*k=Tcl_NewStringObj("-errorcode",-1);Tcl_IncrRefCount(opts);Tcl_IncrRefCount(k);Tcl_DictObjGet(NULL,opts,k,&ec);if(ec)bytes(ec);else printf("null");Tcl_DecrRefCount(k);Tcl_DecrRefCount(opts);
#else
Tcl_Obj *ec=Tcl_GetVar2Ex(i,"errorCode",NULL,TCL_GLOBAL_ONLY);if(ec)bytes(ec);else printf("null");
#endif
printf("}\n");Tcl_DecrRefCount(o);Tcl_ResetResult(i);}Tcl_DeleteInterp(i);return 0;}
