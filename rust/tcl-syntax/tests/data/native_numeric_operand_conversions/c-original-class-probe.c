#include <stdio.h>
#include "tcl.h"
static Tcl_Obj *held;
static const char *before;
static int make(ClientData data,Tcl_Interp *interp,int objc,Tcl_Obj *const objv[]){int mode;(void)data;(void)objc;Tcl_GetIntFromObj(interp,objv[1],&mode);
switch(mode){case 0:held=Tcl_NewStringObj("2",-1);break;case 1:held=Tcl_NewWideIntObj(2);break;case 2:{Tcl_Obj *v=Tcl_NewStringObj("2",-1);held=Tcl_NewListObj(1,&v);break;}case 3:held=Tcl_NewByteArrayObj((const unsigned char*)"2",1);break;case 4:{Tcl_Obj *v[2]={Tcl_NewStringObj("2",-1),Tcl_NewStringObj("3",-1)};held=Tcl_NewListObj(2,v);Tcl_Eval(interp,"dict size {}");
#if TCL_MAJOR_VERSION >= 9 || TCL_MINOR_VERSION >= 5
{
#if TCL_MAJOR_VERSION >= 9
Tcl_Size size;
#else
int size;
#endif
Tcl_DictObjSize(NULL,held,&size);}
#endif
break;}default:held=Tcl_NewDoubleObj(2.0);break;}
Tcl_IncrRefCount(held);before=held->typePtr?held->typePtr->name:"string";Tcl_SetObjResult(interp,held);return TCL_OK;}
int main(int argc,char**argv){int mode;(void)argc;Tcl_FindExecutable(argv[0]);for(mode=0;mode<6;mode++){Tcl_Interp *i=Tcl_CreateInterp();char script[80];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);snprintf(script,sizeof(script),"set i [make %d]; expr {$i < 3}",mode);code=Tcl_Eval(i,script);printf("mode=%d before=%s code=%d after=%s\n",mode,before,code,held->typePtr?held->typePtr->name:"string");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}
