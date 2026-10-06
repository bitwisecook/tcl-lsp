#include "tclInt.h"
#include <stdio.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
static void hex(const char *p,Size n){for(Size i=0;i<n;i++)printf("%02x",(unsigned char)p[i]);}
int main(int argc,char **argv){(void)argc;Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();
const char *inputs[]={"0","end","end-1","0 1","bad","{","2147483647","-1","1+1"};
Tcl_EvalEx(ip,"proc p {x i} {lindex $x $i}",-1,TCL_EVAL_GLOBAL);
for(int n=0;n<sizeof(inputs)/sizeof(*inputs);n++){
Tcl_Obj *v[]={Tcl_NewStringObj("p",-1),Tcl_NewStringObj("{{A B} C} D",-1),Tcl_NewStringObj(inputs[n],-1)};
for(int k=0;k<3;k++)Tcl_IncrRefCount(v[k]);int rc=Tcl_EvalObjv(ip,3,v,TCL_EVAL_GLOBAL);
Tcl_Obj *r=Tcl_GetObjResult(ip);
printf("%d|%d|%s|%d|%d|%s|%d|%d|",n,rc,v[2]->typePtr?v[2]->typePtr->name:"none",v[2]->bytes!=NULL,v[2]->refCount,r->typePtr?r->typePtr->name:"none",r->bytes!=NULL,r->refCount);
Size len;const char *text=Tcl_GetStringFromObj(r,&len);hex(text,len);puts("");for(int k=0;k<3;k++)Tcl_DecrRefCount(v[k]);
}
Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;}
