#include <stdio.h>
#include "tclInt.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static const char *type(Tcl_Obj*o){return o->typePtr?o->typePtr->name:"none";}
static int flag(Tcl_Obj*o){if(o->typePtr!=Tcl_GetObjType("list"))return -1;
#if TCL_MAJOR_VERSION>=9
return (ListObjStorePtr(o)->flags&LISTSTORE_CANONICAL)!=0;
#elif TCL_MINOR_VERSION>=5
return ListRepPtr(o)->canonicalFlag;
#else
return -1;
#endif
}
static void row(int m,const char*stage,Tcl_Obj*o){printf("%d\t%s\t%s\t%d\t%d\n",m,stage,type(o),o->bytes!=NULL,flag(o));}
int main(int ac,char**av){Tcl_Interp*i;int m;(void)ac;Tcl_FindExecutable(av[0]);i=Tcl_CreateInterp();for(m=0;m<6;m++){Tcl_Obj*a=Tcl_NewStringObj("a",1),*b=Tcl_NewStringObj("b",1),*c=Tcl_NewStringObj("c",1),*v[2]={a,b};Tcl_Obj*r=Tcl_NewListObj(2,v),*copy=NULL;Tcl_IncrRefCount(r);if(m>=2&&m<5)Tcl_GetString(r);if(m==3||m==5){copy=Tcl_DuplicateObj(r);Tcl_IncrRefCount(copy);}if(m==4){Tcl_Obj*s=Tcl_NewStringObj("a   b",5);Count n;Tcl_IncrRefCount(s);Tcl_DecrRefCount(r);r=s;Tcl_ListObjLength(i,r,&n);}row(m,"before",copy?copy:r);if(m==0||m==5)Tcl_GetString(copy?copy:r);else Tcl_ListObjAppendElement(i,copy?copy:r,c);row(m,"after",copy?copy:r);if(copy)Tcl_DecrRefCount(copy);Tcl_DecrRefCount(r);}Tcl_DeleteInterp(i);return 0;}
