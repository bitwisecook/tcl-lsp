/* Expr number preparation and runtime scalar/grouped list indices are separate. */
#include <stdio.h>
#include "tcl.h"
static const char *cases[]={"0x20"," 2 ","2","1.5","9999999999999999999999999","end-1"};
static Tcl_Obj *held;
static int make(ClientData data,Tcl_Interp *interp,int objc,Tcl_Obj *const objv[]){int at,group;(void)data;(void)objc;
Tcl_GetIntFromObj(interp,objv[1],&at);Tcl_GetIntFromObj(interp,objv[2],&group);held=Tcl_NewStringObj(cases[at],-1);Tcl_IncrRefCount(held);
if(group==2){double number;Tcl_Obj *parsed=Tcl_DuplicateObj(held);Tcl_IncrRefCount(parsed);
if(Tcl_GetDoubleFromObj(NULL,parsed,&number)==TCL_OK){Tcl_Obj *cached=Tcl_NewDoubleObj(number);Tcl_IncrRefCount(cached);
/* Public native extension fields: retain original string while supplying a genuine Double cache. */
held->typePtr=cached->typePtr;held->internalRep=cached->internalRep;Tcl_DecrRefCount(cached);}
Tcl_DecrRefCount(parsed);}
if(group==1){
#if TCL_MAJOR_VERSION >= 9
 Tcl_Size len;
#else
 int len;
#endif
 (void)Tcl_ListObjLength(NULL,held,&len);}
Tcl_SetObjResult(interp,held);return TCL_OK;}
int main(int argc,char **argv){int at,mode;(void)argc;Tcl_FindExecutable(argv[0]);
for(mode=0;mode<7;mode++)for(at=0;at<6;at++){Tcl_Interp *interp=Tcl_CreateInterp();char script[300];int code;
int stage=mode>=4?mode-4:mode;const char *tail=stage==0?"expr {$i < 3}":stage==1?"lrange {a b c} $i $i":"lindex {a b c} $i";
Tcl_CreateObjCommand(interp,"make",make,NULL,NULL);snprintf(script,sizeof(script),"set i [make %d %d]; %s",at,mode>=4?2:mode==3,tail);code=Tcl_Eval(interp,script);
printf("mode=%d case=%d code=%d cache=%s\n",mode,at,code,held->typePtr?held->typePtr->name:"string");
Tcl_DecrRefCount(held);Tcl_DeleteInterp(interp);}Tcl_Finalize();return 0;}
