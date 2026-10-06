#include "tclInt.h"
#include <stdio.h>
static const char *current;
static int observer(ClientData data,Tcl_Interp *ip,int argc,Tcl_Obj *const argv[]){
 (void)data;if(argc!=4)return TCL_ERROR;
 const char *name=Tcl_GetString(argv[1]),*op=Tcl_GetString(argv[3]);
 Tcl_Obj *v=Tcl_GetVar2Ex(ip,name,NULL,TCL_GLOBAL_ONLY);
 printf("trace|%s|%s|%s|%s|%d|%d|",current,name,op,v&&v->typePtr?v->typePtr->name:"none",v?v->bytes!=NULL:0,v?v->refCount:0);
 if(v){
#if TCL_MAJOR_VERSION >= 9
 Tcl_Size len;
#else
 int len;
#endif
 const unsigned char *b=(void*)Tcl_GetStringFromObj(v,&len);for(int i=0;i<len;i++)printf("%02x",b[i]);
 }puts("");return TCL_OK;
}
int main(int argc,char **argv){(void)argc;Tcl_FindExecutable(argv[0]);
 const char *names[]={"conflict","no-code","NONE","structured","structured-getter"};
 const char *scripts[]={"package provide probe 1;catch {package provide probe 2} m;list $m $::errorCode","catch {error FAIL} m;list $m $::errorCode","catch {error FAIL {} NONE} m;list $m $::errorCode","catch {error FAIL {} {CUSTOM DETAIL}} m;list $m $::errorCode","catch {lindex {} BAD} m;list $m $::errorCode"};
 for(int n=0;n<5;n++){current=names[n];Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"observe",observer,NULL,NULL);
 #if TCL_MAJOR_VERSION >= 9
 if(Tcl_Eval(ip,"trace add variable ::errorInfo {read write} observe;trace add variable ::errorCode {read write} observe")!=TCL_OK)return 2;
#else
 if(Tcl_Eval(ip,"trace variable ::errorInfo rw observe;trace variable ::errorCode rw observe")!=TCL_OK)return 2;
#endif
 int rc=Tcl_Eval(ip,scripts[n]);Tcl_Obj *r=Tcl_GetObjResult(ip);printf("result|%s|%d|%s|%d|%d|",current,rc,r->typePtr?r->typePtr->name:"none",r->bytes!=NULL,r->refCount);
#if TCL_MAJOR_VERSION >= 9
 Tcl_Size len;
#else
 int len;
#endif
 const unsigned char *b=(void*)Tcl_GetStringFromObj(r,&len);for(int i=0;i<len;i++)printf("%02x",b[i]);puts("");Tcl_DeleteInterp(ip);}
 Tcl_Finalize();return 0;}
