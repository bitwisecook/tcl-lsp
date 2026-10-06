#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
int main(int argc,char **argv) {
    Tcl_Interp *i; Tcl_Obj *donor,*source; ByteCode *first,*second; int code,j;
    const char *script=argc>1?argv[1]:"x";
    Tcl_FindExecutable(argv[0]); i=Tcl_CreateInterp();
    donor=Tcl_NewStringObj(script,(int)strlen(script)); Tcl_IncrRefCount(donor);
    code=Tcl_ConvertToType(i,donor,&tclByteCodeType);
    if(code!=TCL_OK){printf("donor-code=%d\n",code);return 2;}
#if TCL_MAJOR_VERSION >= 9
    first=(ByteCode*)TclFetchInternalRep(donor,&tclByteCodeType)->twoPtrValue.ptr1;
#else
    first=(ByteCode*)donor->internalRep.twoPtrValue.ptr1;
#endif
    if(!first->numLitObjects){printf("no-literals\n");return 3;}
    source=first->objArrayPtr[0]; Tcl_IncrRefCount(source);
    if(strcmp(Tcl_GetString(source),script)){printf("not-matching-source\n");return 4;}
    printf("before\ttype=%s\trefs=%d\n",source->typePtr?source->typePtr->name:"none",source->refCount);
    code=Tcl_ConvertToType(i,source,&tclByteCodeType);
#if TCL_MAJOR_VERSION >= 9
    second=(ByteCode*)TclFetchInternalRep(source,&tclByteCodeType)->twoPtrValue.ptr1;
#else
    second=(ByteCode*)source->internalRep.twoPtrValue.ptr1;
#endif
    printf("after\tcode=%d\ttype=%s\trefs=%d\tcount=%d",code,source->typePtr?source->typePtr->name:"none",source->refCount,(int)second->numLitObjects);
    for(j=0;j<second->numLitObjects;j++)printf("\tself%d=%d\tlit%d_type=%s",j,second->objArrayPtr[j]==source,j,second->objArrayPtr[j]->typePtr?second->objArrayPtr[j]->typePtr->name:"none");
    puts("");fflush(stdout);
    /* Older actual self-cycles have no cleanup-safe embedding receipt. Keep
       this native process bounded without pretending to free that cycle. */
    return 0;
}
