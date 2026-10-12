#include <tcl.h>
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
#define SIZE Tcl_Size
#else
#define SIZE int
#endif
static int aliases, nested_aliases;
static Tcl_Obj *expected;
static int callback(ClientData cd,Tcl_Interp *ip,int objc,Tcl_Obj *const objv[]) {
 int mode=(int)(long)cd, code=TCL_OK, number=0; Tcl_WideInt wide=0;
 if (objc!=3) return TCL_ERROR;
 aliases=objv[1]==objv[2];
 if (mode==4) {
  Tcl_Obj *call[3]={Tcl_NewStringObj("leaf",-1),objv[1],objv[2]};
  Tcl_IncrRefCount(call[0]); expected=objv[1];
  code=Tcl_EvalObjv(ip,3,call,TCL_EVAL_DIRECT); Tcl_DecrRefCount(call[0]);
 } else if (mode==0) code=Tcl_GetWideIntFromObj(ip,objv[1],&wide);
 else if (mode==1) code=Tcl_GetIntFromObj(ip,objv[1],&number);
 else if (mode==2 || mode==3) (void)Tcl_GetString(objv[1]);
 else { nested_aliases=objv[1]==expected && objv[2]==expected; code=Tcl_GetWideIntFromObj(ip,objv[1],&wide); }
 return code;
}
static void observe(Tcl_Interp *ip,const char *label,int mode,Tcl_Obj *value) {
 const char *before=value->typePtr?value->typePtr->name:"none";
 int resident=value->bytes!=NULL;
 Tcl_Obj *name=Tcl_NewStringObj("outer",-1),*call[3]={name,value,value};
 Tcl_IncrRefCount(name); Tcl_IncrRefCount(value); Tcl_IncrRefCount(value);
 Tcl_CreateObjCommand(ip,"outer",callback,(ClientData)(long)mode,NULL);
 aliases=0; nested_aliases=0;
 int code=Tcl_EvalObjv(ip,3,call,TCL_EVAL_DIRECT);
 printf("{\"case\":\"%s\",\"code\":%d,\"alias\":%d,\"nested_alias\":%d,\"before\":\"%s\",\"resident_before\":%d,\"after\":\"%s\",\"resident_after\":%d,\"resident_hex\":\"",label,code,aliases,nested_aliases,before,resident,value->typePtr?value->typePtr->name:"none",value->bytes!=NULL);
 if(value->bytes) for(int i=0;i<value->length;i++) printf("%02x",(unsigned char)value->bytes[i]);
 puts("\"}");
 Tcl_DecrRefCount(value); Tcl_DecrRefCount(value); Tcl_DecrRefCount(name);
}
int main(int argc,char **argv) {
 Tcl_FindExecutable(argv[0]); Tcl_Interp *ip=Tcl_CreateInterp();
 Tcl_CreateObjCommand(ip,"leaf",callback,(ClientData)5,NULL);
 observe(ip,"wide-success",0,Tcl_NewStringObj("0x10",-1));
 observe(ip,"int-failure",1,Tcl_NewStringObj("1.5",-1));
 observe(ip,"int-magnitude-failure",1,Tcl_NewStringObj("18446744073709551616",-1));
 observe(ip,"integer-string",2,Tcl_NewWideIntObj(9007199254740993LL));
 {unsigned char data[]={0,255};observe(ip,"binary-string",3,Tcl_NewByteArrayObj(data,2));}
 observe(ip,"nested-wide",4,Tcl_NewStringObj("0x10",-1));
 Tcl_DeleteInterp(ip); Tcl_Finalize(); return 0;
}
