#include <stdio.h>
#include <string.h>
#include "tcl.h"
typedef struct PkgFilesProbe { void *names; Tcl_HashTable table; } PkgFilesProbe;
static Tcl_Obj *header(Tcl_Interp *i) {
    PkgFilesProbe *files = (PkgFilesProbe *)Tcl_GetAssocData(i,"tclPkgFiles",NULL);
    Tcl_HashEntry *entry = files ? Tcl_FindHashEntry(&files->table,"probe") : NULL;
    return entry ? (Tcl_Obj *)Tcl_GetHashValue(entry) : NULL;
}
int main(int argc,char **argv) {
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *i = Tcl_CreateInterp();
    int code=Tcl_EvalEx(i,"package ifneeded probe 1 {source /workspace/.proofs/2286-packages-dialects/package-native-completion/leaf.tcl; package provide probe 1}; package require probe",-1,TCL_EVAL_GLOBAL);
    Tcl_Obj *files=header(i); printf("created\t%d\t%d\t%s\n",code,files ? files->refCount:-1,files&&files->typePtr?files->typePtr->name:"none");
    code=Tcl_EvalEx(i,"package files probe",-1,TCL_EVAL_GLOBAL);
    printf("query\t%d\t%d\t%d\n",code,Tcl_GetObjResult(i)==files,files->refCount);
    code=Tcl_EvalEx(i,"set held [package files probe]",-1,TCL_EVAL_GLOBAL);
    printf("held\t%d\t%d\t%d\n",code,Tcl_GetObjResult(i)==files,files->refCount);
    code=Tcl_EvalEx(i,"package forget probe",-1,TCL_EVAL_GLOBAL);
    printf("forgotten\t%d\t%d\t%d\n",code,header(i)==NULL,files->refCount);
    code=Tcl_EvalEx(i,"set held",-1,TCL_EVAL_GLOBAL);
    printf("retained\t%d\t%d\t%d\n",code,Tcl_GetObjResult(i)==files,files->refCount);
    Tcl_DeleteInterp(i); Tcl_Finalize(); return 0;
}
