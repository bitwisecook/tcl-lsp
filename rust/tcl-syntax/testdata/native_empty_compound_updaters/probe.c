#include <stdio.h>
#ifdef USE_JIM
#include "jim.h"
int main(){Jim_Interp*i=Jim_CreateInterp();Jim_Obj*o;int c,n;for(c=0;c<2;c++){o=c?Jim_NewDictObj(i,NULL,0):Jim_NewListObj(i,NULL,0);Jim_IncrRefCount(o);printf("%d\t%s\t%d\t",c,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL);(void)Jim_GetString(o,&n);printf("%s\t%d\t%d\n",o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,n);Jim_DecrRefCount(i,o);}Jim_FreeInterp(i);return 0;}
#else
#include <tcl.h>
#if TCL_MAJOR_VERSION == 9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
int main(int n,char**v){Tcl_Interp*i;Tcl_Obj*o,*e;int c;Len len;(void)n;Tcl_FindExecutable(v[0]);i=Tcl_CreateInterp();e=Tcl_NewObj();Tcl_IncrRefCount(e);for(c=0;c<2;c++){
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
if(c){printf("1\tunavailable\n");continue;}
#endif
o=Tcl_NewObj();Tcl_IncrRefCount(o);if(!c){const Tcl_ObjType*t=Tcl_GetObjType("list");(void)t->setFromAnyProc(i,o);}else{
#if TCL_MAJOR_VERSION != 8 || TCL_MINOR_VERSION != 4
(void)Tcl_DictObjSize(i,o,&len);
#endif
}printf("%d\t%s\t%d\t",c,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL);Tcl_InvalidateStringRep(o);(void)Tcl_GetStringFromObj(o,&len);printf("%s\t%d\t%d\t%d\n",o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,(int)len,o->bytes==e->bytes);Tcl_DecrRefCount(o);}Tcl_DecrRefCount(e);Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
#endif
