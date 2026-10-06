#include "tcl.h"
#include <stdio.h>
#include <string.h>
static void hex(const char*s,int n){for(int k=0;k<n;k++)printf("%02x",(unsigned char)s[k]);}
int main(void){for(int k=0;k<7;k++){
Tcl_Interp*i=Tcl_CreateInterp();char name[80];int n;
if(k==0){memcpy(name,"p",1);n=1;}else if(k==1){memcpy(name,"p q",3);n=3;}else if(k==2){memcpy(name,"p\xff",2);n=2;}else if(k==3){memcpy(name,"p\0ignored",9);n=9;}else {memset(name,'p',49);name[49]=(char)0xc3;name[50]=(char)0xa9;memset(name+51,'q',9);n=60;}
const char*body=k==6?"set side READY\nset value \"":"set value \"";
Tcl_Obj*o[4]={Tcl_NewStringObj("proc",4),Tcl_NewStringObj(name,n),Tcl_NewStringObj("x",1),Tcl_NewStringObj(body,-1)};
for(int j=0;j<4;j++)Tcl_IncrRefCount(o[j]);int dc=Tcl_EvalObjv(i,4,o,TCL_EVAL_GLOBAL);
Tcl_Obj*a[2]={o[1],Tcl_NewStringObj("OK",2)};Tcl_IncrRefCount(a[1]);int ac=Tcl_EvalObjv(i,k==5?2:1,a,TCL_EVAL_GLOBAL);
Tcl_Obj*r=Tcl_GetObjResult(i);Tcl_IncrRefCount(r);int rn;const char*rs=Tcl_GetStringFromObj(r,&rn);
Tcl_Obj*info=Tcl_GetVar2Ex(i,"errorInfo",NULL,TCL_GLOBAL_ONLY);Tcl_Obj*code=Tcl_GetVar2Ex(i,"errorCode",NULL,TCL_GLOBAL_ONLY);int in,cn;const char*is=Tcl_GetStringFromObj(info,&in);const char*cs=Tcl_GetStringFromObj(code,&cn);
printf("%d\t%d\t%d\t",k,dc,ac);hex(name,n);putchar('\t');hex(body,strlen(body));putchar('\t');hex(rs,rn);putchar('\t');hex(is,in);putchar('\t');hex(cs,cn);putchar('\n');
Tcl_DecrRefCount(r);Tcl_DecrRefCount(a[1]);for(int j=0;j<4;j++)Tcl_DecrRefCount(o[j]);Tcl_DeleteInterp(i);
}return 0;}
