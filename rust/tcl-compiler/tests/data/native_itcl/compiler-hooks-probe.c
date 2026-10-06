// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

#include <tclInt.h>
#include <stdio.h>
#include <string.h>
static void hooks(Tcl_Interp *interp, const char *name) {
    Tcl_Command token=Tcl_FindCommand(interp,name,NULL,TCL_GLOBAL_ONLY);
    Command *command=(Command *)token;
    printf("%s|%d|%d\n",name,token!=NULL,command ? command->compileProc!=NULL : -1);
}
int main(int argc,char **argv) {
    if (argc != 4) {fprintf(stderr,"usage: %s TCL_LIBRARY ITCL_LIBRARY ITCL_SO\n",argv[0]);return 2;}
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *interp=Tcl_CreateInterp();
    Tcl_SetVar2(interp,"env","TCL_LIBRARY",argv[1],TCL_GLOBAL_ONLY);
    Tcl_SetVar2(interp,"env","ITCL_LIBRARY",argv[2],TCL_GLOBAL_ONLY);
    Tcl_Obj *load[] = {Tcl_NewStringObj("load",-1),Tcl_NewStringObj(argv[3],-1),Tcl_NewStringObj("Itcl",-1)};
    for (int i=0;i<3;i++) Tcl_IncrRefCount(load[i]);
    int loaded = Tcl_Init(interp)==TCL_OK && Tcl_EvalObjv(interp,3,load,TCL_EVAL_GLOBAL)==TCL_OK;
    for (int i=0;i<3;i++) Tcl_DecrRefCount(load[i]);
    if (!loaded || Tcl_Eval(interp,"itcl::class C {proc ping {} {return static}; method status {} {return instance}}; C named")!=TCL_OK) {
        fprintf(stderr,"%s\n",Tcl_GetStringResult(interp));return 1;
    }
    hooks(interp,"::itcl::class"); hooks(interp,"::itcl::parser::method");
    hooks(interp,"::itcl::parser::proc"); hooks(interp,"::itcl::clazz");
    hooks(interp,"::C"); hooks(interp,"::C::ping"); hooks(interp,"::named");
    Tcl_DeleteInterp(interp);Tcl_Finalize();return 0;
}
