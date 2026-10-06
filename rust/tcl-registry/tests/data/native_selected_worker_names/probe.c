#include "tclInt.h"
#include <stdio.h>
int main(int argc, char **argv) {
 (void)argc; Tcl_FindExecutable(argv[0]); Tcl_Interp *interp=Tcl_CreateInterp();
 if (Tcl_Init(interp)!=TCL_OK) return 1;
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
 if (Tcl_Eval(interp,"namespace eval a: {proc w args {return selected}; namespace ensemble create -command ::E -map {member w}}; namespace eval a {proc w args {return collision}}")!=TCL_OK) return 2;
 Tcl_Obj *name=Tcl_NewStringObj("::E",-1); Tcl_IncrRefCount(name);
 Tcl_Command ensemble=Tcl_GetCommandFromObj(interp,name);
 if (Tcl_SetEnsembleFlags(interp,ensemble,TCL_ENSEMBLE_PREFIX|ENSEMBLE_COMPILE)!=TCL_OK) return 3;
 Tcl_DecrRefCount(name);
 if (Tcl_Eval(interp,"proc f {} {E mem VALUE}; list [f] [::a:::w VALUE] [namespace eval a: {namespace current}]")!=TCL_OK) return 4;
 printf("compiled-collision\t%s\n",Tcl_GetStringResult(interp));
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 6
 Tcl_HashEntry *namespaceEntry=Tcl_FindHashEntry(&((Interp *)interp)->globalNsPtr->childTable,"a:");
 Namespace *ns=namespaceEntry ? Tcl_GetHashValue(namespaceEntry) : NULL;
 Tcl_HashEntry *commandEntry=ns ? Tcl_FindHashEntry(&ns->cmdTable,"w") : NULL;
 Tcl_Command selected=commandEntry ? (Tcl_Command)Tcl_GetHashValue(commandEntry) : NULL;
 if (!selected) return 5;
 Tcl_Obj *target=Tcl_NewStringObj("::a:::w",-1);
 Tcl_IncrRefCount(target); TclSetCmdNameObj(interp,target,(Command *)selected);
 Tcl_Obj *prefix=Tcl_NewListObj(1,&target); Tcl_IncrRefCount(prefix);
 Tcl_Obj *mapping=Tcl_NewDictObj(); Tcl_IncrRefCount(mapping);
 Tcl_DictObjPut(interp,mapping,Tcl_NewStringObj("member",-1),prefix);
 if (Tcl_SetEnsembleMappingDict(interp,ensemble,mapping)!=TCL_OK) return 6;
 Tcl_DecrRefCount(mapping); Tcl_DecrRefCount(prefix); Tcl_DecrRefCount(target);
 if (Tcl_Eval(interp,"proc g {} {E mem VALUE}; list [g] [::a:::w VALUE]")!=TCL_OK) return 7;
 printf("cached-selected\t%s\n",Tcl_GetStringResult(interp));
#endif
#else
 puts("unsupported\tC8.4 has no namespace ensemble compiler");
#endif
 Tcl_DeleteInterp(interp); Tcl_Finalize(); return 0;
}
