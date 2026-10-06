#include "tcl.h"
#include "tclInt.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#if TCL_MAJOR_VERSION >= 9
#define SIZE Tcl_Size
#else
#define SIZE int
#endif
static int mode;
static char *trace(ClientData data,Tcl_Interp *interp,const char *name1,const char *name2,int flags){(void)data;(void)interp;(void)name1;(void)name2;(void)flags;return NULL;}
static int probe(ClientData data,Tcl_Interp *interp,SIZE objc,Tcl_Obj *const objv[]){
 Tcl_Obj *name=objv[1],*value,*result; Var *array,*before,*after; int code,resident;const char *kind; SIZE len;
 (void)data;(void)objc;
 value=Tcl_ObjGetVar2(interp,name,NULL,TCL_LEAVE_ERR_MSG);if(!value)return TCL_ERROR;
 before=TclObjLookupVar(interp,name,NULL,0,"read",0,0,&array);
 if(mode==2||mode==3)Tcl_TraceVar2(interp,"x",NULL,TCL_TRACE_READS|TCL_TRACE_WRITES,trace,NULL);
 if(mode==4)Tcl_UnsetVar(interp,"x",0);
 Tcl_InvalidateStringRep(name);
 if(mode==1||mode==3){value=Tcl_NewStringObj("NEXT",4);result=Tcl_ObjSetVar2(interp,name,NULL,value,TCL_LEAVE_ERR_MSG);}
 else result=Tcl_ObjGetVar2(interp,name,NULL,TCL_LEAVE_ERR_MSG);
 code=result?TCL_OK:TCL_ERROR;resident=name->bytes!=NULL;kind=name->typePtr?name->typePtr->name:"none";
 /* Lookup of the selected actual cell precedes byte/result observation. */
 after=TclObjLookupVar(interp,name,NULL,0,"read",0,0,&array);
 printf("%d\t%d\t%d\t%s\t%d\t",mode,code,resident,kind,before==after);
 result=Tcl_GetObjResult(interp);{const char *bytes=Tcl_GetStringFromObj(result,&len);SIZE k;for(k=0;k<len;k++)printf("%02x",(unsigned char)bytes[k]);}printf("\n");fflush(stdout);
 return code;
}
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);(void)argc;for(mode=atoi(argv[1]);mode==atoi(argv[1]);mode++){Tcl_Interp *interp=Tcl_CreateInterp();
#if TCL_MAJOR_VERSION >= 9
 Tcl_CreateObjCommand2(interp,"probe",probe,NULL,NULL);
#else
 Tcl_CreateObjCommand(interp,"probe",probe,NULL,NULL);
#endif
 Tcl_EvalEx(interp,"proc p {} {set x VALUE;probe x};p",-1,0);Tcl_DeleteInterp(interp);
}Tcl_Finalize();return 0;}
