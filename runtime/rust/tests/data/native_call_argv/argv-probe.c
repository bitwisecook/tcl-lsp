#include "tclInt.h"
#include <stdio.h>
#include <string.h>
static Tcl_Obj *argument;
static int observe(ClientData cd,Tcl_Interp *i,int objc,Tcl_Obj *const objv[]){Interp *p=(Interp*)i;CallFrame *f=p->varFramePtr;int j;
printf("frame\tobjc=%d",f->objc);for(j=0;j<f->objc;j++)printf("\tsame%d=%d\ttype%d=%s\trefs%d=%d",j,f->objv[j]==argument,j,f->objv[j]->typePtr?f->objv[j]->typePtr->name:"none",j,f->objv[j]->refCount);puts("");return TCL_OK;}
int main(int argc,char **argv){Tcl_Interp*i;Interp*p;Tcl_Obj *words[3];int code;
Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();p=(Interp*)i;Tcl_CreateObjCommand(i,"observe",observe,NULL,NULL);
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 6
code=Tcl_EvalEx(i,"oo::class create Base {method m {x} {observe; error FAILURE}}; oo::class create Derived {superclass Base; method m {x} {next $x}}; Derived create o",-1,0);
if(code){fprintf(stderr,"setup: %s\n",Tcl_GetStringResult(i));return 2;}
argument=Tcl_NewListObj(0,NULL);Tcl_ListObjAppendElement(NULL,argument,Tcl_NewStringObj("ORIGINAL",-1));Tcl_IncrRefCount(argument);
words[0]=Tcl_NewStringObj("o",-1);words[1]=Tcl_NewStringObj("m",-1);words[2]=argument;Tcl_IncrRefCount(words[0]);Tcl_IncrRefCount(words[1]);
code=Tcl_EvalObjv(i,3,words,TCL_EVAL_DIRECT);printf("after\tcode=%d\targType=%s\targRefs=%d\n",code,argument->typePtr?argument->typePtr->name:"none",argument->refCount);
{Tcl_Obj **parts;long n,j,k;
#if TCL_MAJOR_VERSION >= 9
Tcl_Size size;Tcl_ListObjGetElements(NULL,p->errorStack,&size,&parts);n=size;
#else
int size;Tcl_ListObjGetElements(NULL,p->errorStack,&size,&parts);n=size;
#endif
for(j=0;j<n;j+=2){if(!strcmp(Tcl_GetString(parts[j]),"CALL")){Tcl_Obj **call;
#if TCL_MAJOR_VERSION >= 9
Tcl_Size count;Tcl_ListObjGetElements(NULL,parts[j+1],&count,&call);
#else
int count;Tcl_ListObjGetElements(NULL,parts[j+1],&count,&call);
#endif
printf("call\tcount=%ld",(long)count);for(k=0;k<count;k++)printf("\tsame%ld=%d\ttype%ld=%s\trefs%ld=%d",k,call[k]==argument,k,call[k]->typePtr?call[k]->typePtr->name:"none",k,call[k]->refCount);puts("");}}}
Tcl_DecrRefCount(words[0]);Tcl_DecrRefCount(words[1]);Tcl_DecrRefCount(argument);
#else
puts("oo\tabsent-native-core");
#endif
Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
