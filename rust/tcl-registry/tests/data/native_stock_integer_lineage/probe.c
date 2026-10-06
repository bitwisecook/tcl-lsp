#include "tcl.h"
#include <stdio.h>
#include <string.h>
static void snapshot(const char *label,Tcl_Obj *v) {
 printf("%s:%s:%d:",label,v->typePtr?v->typePtr->name:"none",v->bytes!=NULL);
 if(v->bytes) {int i;for(i=0;i<v->length;i++)printf("%02x",(unsigned char)v->bytes[i]);}
}
static Tcl_Obj *make_input(int kind,Tcl_Interp *ip,int *precode) {
 Tcl_Obj *v; double d=0;
#if TCL_MAJOR_VERSION >= 9
 Tcl_Size n=0;
#else
 int n=0;
#endif
 switch(kind) {
 case 0:v=Tcl_NewStringObj("0",1);break;
 case 1:v=Tcl_NewStringObj("1",1);break;
 case 2:v=Tcl_NewStringObj("18446744073709551616",20);break;
 case 3:v=Tcl_NewStringObj("-0",2);break;
 case 4:v=Tcl_NewDoubleObj(1.0);break;
 case 5:v=Tcl_NewIntObj(1);break;
 case 6:v=Tcl_NewStringObj("1",1);break;
 default:v=Tcl_NewStringObj("0",1);break;
 }
 Tcl_IncrRefCount(v); *precode=0;
 if(kind==0||kind==1||kind==2||kind==3||kind==7)*precode=Tcl_GetDoubleFromObj(ip,v,&d);
 if(kind==6)*precode=Tcl_ListObjLength(ip,v,&n);
 if(kind==7) {Tcl_Obj *w=Tcl_DuplicateObj(v);Tcl_IncrRefCount(w);Tcl_DecrRefCount(v);v=w;}
 return v;
}
int main(int argc,char **argv) {
 const char *names[]={"raw0-double","raw1-double","rawbig-double","rawminus0-double","double1-constructor","int1-constructor","raw1-list","raw0-double-duplicate"};
 int k,r;Tcl_FindExecutable(argv[0]);
 for(k=0;k<8;k++)for(r=0;r<3;r++) {
  Tcl_Interp *ip=Tcl_CreateInterp();int pre,code;Tcl_Obj *v=make_input(k,ip,&pre);
  printf("%s\t%s\tpre=%d\t",names[k],r==0?"observe":r==1?"expr":"incr",pre);snapshot("before",v);
  if(r==0)code=0;
  else {Tcl_SetVar2Ex(ip,"x",NULL,v,TCL_GLOBAL_ONLY);code=Tcl_EvalEx(ip,r==1?"expr {$x+1}":"incr x",-1,TCL_EVAL_GLOBAL);}
  printf("\tcode=%d\t",code);snapshot("original-after",v);
  if(r!=0){Tcl_Obj *cell=Tcl_GetVar2Ex(ip,"x",NULL,TCL_GLOBAL_ONLY);printf("\t");if(cell)snapshot("cell-after",cell);}
  printf("\tresult=");{const char *s=Tcl_GetStringResult(ip);int i;for(i=0;s[i];i++)printf("%02x",(unsigned char)s[i]);}
  printf("\n");Tcl_DecrRefCount(v);Tcl_DeleteInterp(ip);
 }
 Tcl_Finalize();return 0;
}
