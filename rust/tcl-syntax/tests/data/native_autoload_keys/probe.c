// SPDX-License-Identifier: AGPL-3.0-or-later
#include <tcl.h>
#include <stdio.h>
#include <stdlib.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
int main(int argc,char **argv) {
 const unsigned char *cases[]={(unsigned char *)"::a\xff::b",(unsigned char *)"a\0::b",(unsigned char *)"::a\0x",(unsigned char *)"a\xff",(unsigned char *)"a\xc0\x80::b"};
 const int lengths[]={7,5,5,2,6};
 Tcl_FindExecutable(argv[0]); Tcl_Interp *i=Tcl_CreateInterp();
 Tcl_SetVar(i,"tcl_library",argv[2],TCL_GLOBAL_ONLY);
 if (Tcl_Eval(i,argv[1])!=TCL_OK) {fprintf(stderr,"init: %s",Tcl_GetStringResult(i));return 2;}
 for (int n=0;n<5;n++) {
 Tcl_Obj *v[]={Tcl_NewStringObj("auto_qualify",-1),Tcl_NewStringObj((char *)cases[n],lengths[n]),Tcl_NewStringObj("::n",-1)};
 for(int k=0;k<3;k++)Tcl_IncrRefCount(v[k]);
 int code=Tcl_EvalObjv(i,3,v,TCL_EVAL_DIRECT);printf("%d code%d",n,code);
 Tcl_Obj **parts;Count count;
 if(code==0&&Tcl_ListObjGetElements(i,Tcl_GetObjResult(i),&count,&parts)==0)for(Count x=0;x<count;x++){
 Count bytes;unsigned char *s=(unsigned char *)Tcl_GetStringFromObj(parts[x],&bytes);printf(" ");for(Count at=0;at<bytes;at++)printf("%02x",s[at]);}
 printf("\n");for(int k=0;k<3;k++)Tcl_DecrRefCount(v[k]);
 }
 Tcl_DeleteInterp(i);Tcl_Finalize();return 0;
}