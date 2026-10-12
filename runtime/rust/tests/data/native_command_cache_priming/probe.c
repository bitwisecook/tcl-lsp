#include <stdio.h>
#include <tcl.h>
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
extern Tcl_Command Tcl_GetCommandFromObj(Tcl_Interp *, Tcl_Obj *);
#endif
extern void TclSetCmdNameObj(Tcl_Interp *, Tcl_Obj *, Tcl_Command);
static int callback(ClientData d,Tcl_Interp*i,int n,Tcl_Obj*const v[]){(void)d;(void)i;(void)n;(void)v;return TCL_OK;}
static const char *type(Tcl_Obj *v){return v->typePtr?v->typePtr->name:"none";}
static Tcl_Command cached(Tcl_Obj *v){if(!v->typePtr||!v->internalRep.twoPtrValue.ptr1)return NULL;return *(Tcl_Command *)v->internalRep.twoPtrValue.ptr1;}
int main(int n,char**v){Tcl_Interp*i;Tcl_Command a,b;int c;(void)n;Tcl_FindExecutable(v[0]);i=Tcl_CreateInterp();a=Tcl_CreateObjCommand(i,"A",callback,NULL,NULL);b=Tcl_CreateObjCommand(i,"B",callback,NULL,NULL);
for(c=0;c<5;c++){Tcl_Obj*o=c==2?Tcl_NewIntObj(7):Tcl_NewStringObj(c==1?"MISSING":"A",-1);const char*original;Tcl_IncrRefCount(o);if(c<3)(void)Tcl_GetCommandFromObj(i,o);else TclSetCmdNameObj(i,o,a);original=o->bytes;printf("%d\t%s\t%d\t",c,type(o),o->bytes!=NULL);TclSetCmdNameObj(i,o,c==3?a:b);printf("%s\t%d\t%d\t%d\n",type(o),cached(o)==a?1:cached(o)==b?2:0,o->bytes!=NULL,o->bytes==original);Tcl_DecrRefCount(o);Tcl_ResetResult(i);}Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
