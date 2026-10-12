#include "tclInt.h"
#include <stdio.h>
static void report(Tcl_Interp *interp, const char *event, int code) {
    Interp *i=(Interp *)interp;
    Command *cmd=(Command *)Tcl_FindCommand(interp,"::info",NULL,TCL_GLOBAL_ONLY);
    printf("%s\t%d\t%d\t%d\n",event,code,i->compileEpoch,cmd ? cmd->compileProc != NULL : -1);
}
int main(int argc,char **argv) {
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *interp=Tcl_CreateInterp();
    puts("event\tcode\tcompiler\tinfo_hook");
    report(interp,"initial",TCL_OK);
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
    report(interp,"prefix_false",Tcl_Eval(interp,"namespace ensemble configure ::info -prefixes 0"));
    report(interp,"same_prefix",Tcl_Eval(interp,"namespace ensemble configure ::info -prefixes 0"));
    report(interp,"bad_option",Tcl_Eval(interp,"namespace ensemble configure ::info -NOT_AN_OPTION 1"));
    report(interp,"prefix_true",Tcl_Eval(interp,"namespace ensemble configure ::info -prefixes 1"));
#endif
    Tcl_DeleteInterp(interp);Tcl_Finalize();return 0;
}
