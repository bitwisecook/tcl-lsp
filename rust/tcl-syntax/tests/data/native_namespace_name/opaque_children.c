#include "tcl.h"
#include "tclInt.h"
#include <stdio.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
static void run(Tcl_Interp *i,const char *label,const char *parent,const char *pattern) {
 Tcl_Obj *v[4];int n=3;v[0]=Tcl_NewStringObj("namespace",-1);v[1]=Tcl_NewStringObj("children",-1);v[2]=Tcl_NewStringObj(parent,-1);if(pattern){v[3]=Tcl_NewStringObj(pattern,-1);n=4;}
 for(int k=0;k<n;k++)Tcl_IncrRefCount(v[k]);int code=Tcl_EvalObjv(i,n,v,0);Tcl_Obj *r=Tcl_GetObjResult(i);const char *type=r->typePtr?r->typePtr->name:"none";Len len;const unsigned char *s=(const unsigned char*)Tcl_GetStringFromObj(r,&len);printf("%s|%d|%s|",label,code,type);for(Len k=0;k<len;k++)printf("%02x",s[k]);puts("");for(int k=0;k<n;k++)Tcl_DecrRefCount(v[k]);
}
int main(void) {Tcl_Interp *i=Tcl_CreateInterp();const char parent[]="::raw\xff";const char child[]="::raw\xff::child\xfd";Tcl_CreateNamespace(i,parent,NULL,NULL);Tcl_CreateNamespace(i,child,NULL,NULL);run(i,"enumerate",parent,NULL);run(i,"exact",parent,child);run(i,"star",parent,"*");run(i,"negative",parent,"*other*");Tcl_DeleteInterp(i);return 0;}
