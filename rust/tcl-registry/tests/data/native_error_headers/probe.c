#include "tclInt.h"
#include <stdio.h>
static const char *type(Tcl_Obj *o) {return !o?"absent":o->typePtr?o->typePtr->name:"none";}
int main(int argc,char **argv){Tcl_Interp *i;Interp *p;Tcl_Obj *h,*v;int code;
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
Tcl_InterpState s;
#endif
Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();p=(Interp*)i;
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
printf("initial-options\t%s\n",type(p->returnOpts));
code=Tcl_EvalEx(i,"return -custom {original value} -level 0 X",-1,0);h=p->returnOpts;
if(!h){fprintf(stderr,"no private options %d\n",code);return 2;}
{Tcl_Obj *key=Tcl_NewStringObj("-custom",-1);Tcl_IncrRefCount(key);Tcl_DictObjGet(NULL,h,key,&v);Tcl_DecrRefCount(key);}
printf("options-before\t%s\t%d\t%s\t%d\n",type(h),h->refCount,type(v),v->refCount);
s=Tcl_SaveInterpState(i,code);
printf("options-saved\t%d\t%d\n",h->refCount,v->refCount);
Tcl_EvalEx(i,"return -other changed -level 0 Y",-1,0);
printf("options-mutated\t%d\t%d\t%d\n",p->returnOpts!=h,h->refCount,v->refCount);
Tcl_RestoreInterpState(i,s);
printf("options-restored\t%d\t%d\t%d\n",p->returnOpts==h,h->refCount,v->refCount);
#else
printf("initial-options\tabsent-field\n");
#endif
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 6
h=p->errorStack;printf("initial-stack\t%s\t%d\n",type(h),h->refCount);
code=Tcl_EvalEx(i,"error FIRST",-1,0);h=p->errorStack;{long n;Tcl_Obj **parts;
#if TCL_MAJOR_VERSION < 9
int oldn;Tcl_ListObjGetElements(NULL,h,&oldn,&parts);n=oldn;
#else
Tcl_Size newn;Tcl_ListObjGetElements(NULL,h,&newn,&parts);n=newn;
#endif
v=parts[1];printf("stack-before\t%s\t%d\t%ld\t%s\t%d\n",type(h),h->refCount,(long)n,type(v),v->refCount);
s=Tcl_SaveInterpState(i,code);printf("stack-saved\t%d\t%d\n",h->refCount,v->refCount);
Tcl_EvalEx(i,"error SECOND",-1,0);printf("stack-mutated\t%d\t%d\t%d\n",p->errorStack!=h,h->refCount,v->refCount);
Tcl_RestoreInterpState(i,s);printf("stack-restored\t%d\t%d\t%d\n",p->errorStack==h,h->refCount,v->refCount);}
#else
printf("initial-stack\tabsent-field\n");
#endif
Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
