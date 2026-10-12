#include <stdio.h>
#include "tcl.h"
static Tcl_Obj *held;
static const char *before;
static int before_string;
static int make(ClientData data,Tcl_Interp *i,int n,Tcl_Obj *const v[]){int mode;(void)data;(void)n;Tcl_GetIntFromObj(i,v[1],&mode);
switch(mode){case 0:held=Tcl_NewStringObj("",0);break;case 1:held=Tcl_NewStringObj("0",1);break;case 2:held=Tcl_NewStringObj("{",1);break;case 3:held=Tcl_NewIntObj(0);break;case 4:held=Tcl_NewIntObj(2);break;case 5:held=Tcl_NewDoubleObj(0.0);break;case 6:held=Tcl_NewDoubleObj(2.0);break;case 7:held=Tcl_NewListObj(0,NULL);break;case 8:{Tcl_Obj *a=Tcl_NewStringObj("2",1);held=Tcl_NewListObj(1,&a);break;}case 9:held=Tcl_NewByteArrayObj(NULL,0);break;case 10:held=Tcl_NewByteArrayObj((const unsigned char*)"2",1);break;case 14:held=Tcl_NewIntObj(2);(void)Tcl_GetString(held);break;case 15:held=Tcl_NewDoubleObj(2.0);(void)Tcl_GetString(held);break;case 16:held=Tcl_NewStringObj("true",4);{int b;(void)Tcl_GetBooleanFromObj(NULL,held,&b);}break;default:
#if TCL_MAJOR_VERSION >= 9 || TCL_MINOR_VERSION >= 5
held=Tcl_NewDictObj();if(mode==12)Tcl_DictObjPut(NULL,held,Tcl_NewStringObj("2",1),Tcl_NewStringObj("3",1));
#else
held=Tcl_NewListObj(0,NULL);
#endif
if(mode==13)(void)Tcl_GetString(held);break;}
Tcl_IncrRefCount(held);before=held->typePtr?held->typePtr->name:"string";before_string=held->bytes!=NULL;Tcl_SetObjResult(i,held);return TCL_OK;}
int main(int argc,char**argv){int mode,route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Tcl_Interp*i=Tcl_CreateInterp();char source[160];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Tcl_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Tcl_GetStringResult(i);for(;*s;s++)printf("%02x",*s);}puts("");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}
