#include "tclInt.h"
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
static void header(const char *stage,Tcl_Obj *o){
 Size count=-999;
 if(o->typePtr&&!strcmp(o->typePtr->name,"string"))memcpy(&count,o->internalRep.otherValuePtr,sizeof(count));
 printf("%s|%s|%d|%d|%lld\n",stage,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,o->refCount,(long long)count);
}
static void result(Tcl_Interp *i){Tcl_Obj *r=Tcl_GetObjResult(i);Size n;header("BEFORE-STRING",r);const unsigned char *s=(const unsigned char*)Tcl_GetStringFromObj(r,&n);printf("HEX|");for(Size k=0;k<n;k++)printf("%02x",s[k]);puts("");header("AFTER-STRING",r);printf("CHARS|%lld\n",(long long)Tcl_GetCharLength(r));header("AFTER-LENGTH",r);}
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *i=Tcl_CreateInterp();Tcl_EvalEx(i,"info patchlevel",-1,0);printf("VERSION|%s\n",Tcl_GetStringResult(i));
const char *scripts[]={"proc bad {args} {error BOOM}; set first 0; trace add variable first write bad; scan {1 2} {%d %d} first output"};
for(int k=0;k<1;k++){Tcl_ResetResult(i);printf("CASE|%d\n",k);printf("CODE|%d\n",Tcl_EvalEx(i,scripts[k],-1,0));result(i);Tcl_Obj *v=Tcl_GetVar2Ex(i,"output",NULL,0);printf("OUTPUT|%s\n",v?Tcl_GetString(v):"UNSET");}
Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
