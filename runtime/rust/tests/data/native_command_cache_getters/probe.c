#include <stdio.h>
#include <string.h>
#include <tcl.h>
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
extern Tcl_Command Tcl_GetCommandFromObj(Tcl_Interp *, Tcl_Obj *);
#endif
static int callback(ClientData d,Tcl_Interp *i,int n,Tcl_Obj *const v[]) {(void)d;(void)i;(void)n;(void)v;return TCL_OK;}
static const char *type(Tcl_Obj *v){return v->typePtr?v->typePtr->name:"none";}
int main(int argc,char **argv){const char *names[]={"1","1.0","true","1 2","1 x","bad"};Tcl_Interp *i;int s,c;Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();(void)argc;
for(s=0;s<6;s++)for(c=0;c<6;c++){Tcl_Obj *v=Tcl_NewStringObj(names[c],-1);const char *original;int rc=0;long long result=0;Tcl_IncrRefCount(v);Tcl_CreateObjCommand(i,names[c],callback,NULL,NULL);original=v->bytes;if(!Tcl_GetCommandFromObj(i,v)){return 2;}printf("%d\t%d\t%s\t",s,c,type(v));
switch(s){case 0:{Tcl_WideInt n=0;rc=Tcl_GetWideIntFromObj(i,v,&n);result=(long long)n;break;}case 1:{double n=0;rc=Tcl_GetDoubleFromObj(i,v,&n);result=(long long)n;break;}case 2:{int n=0;rc=Tcl_GetBooleanFromObj(i,v,&n);result=n;break;}case 3:{
#if TCL_MAJOR_VERSION >= 9
Tcl_Size n=0;
#else
int n=0;
#endif
rc=Tcl_ListObjLength(i,v,&n);result=(long long)n;break;}case 4:{
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
#if TCL_MAJOR_VERSION >= 9
Tcl_Size n=0;
#else
int n=0;
#endif
rc=Tcl_DictObjSize(i,v,&n);result=(long long)n;
#else
rc=-1;
#endif
break;}case 5:result=(long long)Tcl_GetCharLength(v);break;}
printf("%d\t%lld\t%s\t%d\t%d\n",rc,result,type(v),v->bytes!=NULL,v->bytes==original);Tcl_DecrRefCount(v);Tcl_ResetResult(i);}Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
