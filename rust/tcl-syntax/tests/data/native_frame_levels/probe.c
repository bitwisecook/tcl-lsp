#include <tcl.h>
#include <stdio.h>
#include <math.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
static void hex(Tcl_Obj *o){Len n;const unsigned char*s=(const unsigned char*)Tcl_GetStringFromObj(o,&n);for(Len k=0;k<n;k++)printf("%02x",s[k]);}
static int probe(ClientData cd,Tcl_Interp*i,int count,Tcl_Obj*const words[]){
(void)cd;(void)count;(void)words;
const char*texts[]={"1","1\0X","#1","#1\0X","-1","+1","01","1.0","NaN","2147483648","4294967295","0","bad"};
for(int k=0;k<16;k++){
Tcl_Obj*o=k<13?Tcl_NewStringObj(texts[k],(k==1?3:k==3?4:-1)):(k==13?Tcl_NewWideIntObj(1):Tcl_NewDoubleObj(k==14?1.0:NAN));
Tcl_IncrRefCount(o);const char*before=o->typePtr?o->typePtr->name:"string";
Tcl_Obj*argv[]={Tcl_NewStringObj("uplevel",-1),o,Tcl_NewStringObj("info level",-1)};
Tcl_IncrRefCount(argv[0]);Tcl_IncrRefCount(argv[2]);int code=Tcl_EvalObjv(i,3,argv,0);
printf("{\"case\":%d,\"before\":\"%s\",\"after\":\"%s\",\"code\":%d,\"result\":\"",k,before,o->typePtr?o->typePtr->name:"string",code);hex(Tcl_GetObjResult(i));printf("\"}\n");
Tcl_DecrRefCount(argv[0]);Tcl_DecrRefCount(argv[2]);Tcl_DecrRefCount(o);Tcl_ResetResult(i);
}
return TCL_OK;}
int main(){Tcl_Interp*i=Tcl_CreateInterp();Tcl_CreateObjCommand(i,"probe",probe,NULL,NULL);int code=Tcl_Eval(i,"proc outer {} {inner}; proc inner {} {probe}; outer");if(code!=TCL_OK){fprintf(stderr,"%s\n",Tcl_GetStringResult(i));return 1;}Tcl_DeleteInterp(i);return 0;}
