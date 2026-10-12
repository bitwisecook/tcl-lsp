#include "tclInt.h"
#include <stdio.h>

static Command *visible(Tcl_Interp *interp, const char *name) {
    return (Command *)Tcl_FindCommand(interp, name, NULL, TCL_GLOBAL_ONLY);
}

static void report(Tcl_Interp *interp, const char *event, int code) {
    Command *source = visible(interp, "::N::e");
    Command *old = visible(interp, "::old");
    Command *fresh = visible(interp, "::fresh");
    int epoch = ((Interp *)interp)->compileEpoch;
    int source_hook = source ? source->compileProc != NULL : -1;
    int old_hook = old ? old->compileProc != NULL : -1;
    int fresh_hook = fresh ? fresh->compileProc != NULL : -1;
    int old_same = source && old ? source->compileProc == old->compileProc : -1;
    int fresh_same = source && fresh ? source->compileProc == fresh->compileProc : -1;
    printf("%s\t%d\t%d\t%d\t%d\t%d\t%d\t%d\n", event, code, epoch,
        source_hook, old_hook, fresh_hook, old_same, fresh_same);
}

int main(int argc, char **argv) {
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *interp = Tcl_CreateInterp();
    int code;
    puts("event\tcode\tcompiler\tsource_hook\told_hook\tfresh_hook\told_same\tfresh_same");
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
    code = Tcl_Eval(interp,
        "namespace eval N {namespace export e; namespace ensemble create -command e -map {x ::list}};"
        "namespace import ::N::e; rename e old");
    report(interp, "import_absent", code);
    code = Tcl_SetEnsembleFlags(interp, (Tcl_Command)visible(interp, "::N::e"),
        TCL_ENSEMBLE_PREFIX | ENSEMBLE_COMPILE);
    report(interp, "attach_source", code);
    code = Tcl_Eval(interp, "namespace import ::N::e; rename e fresh");
    report(interp, "import_present", code);
    code = Tcl_SetEnsembleFlags(interp, (Tcl_Command)visible(interp, "::N::e"),
        TCL_ENSEMBLE_PREFIX);
    report(interp, "detach_source", code);
    code = Tcl_Eval(interp, "namespace ensemble configure ::N::e -prefixes 0");
    report(interp, "configure_unhooked", code);
    code = Tcl_SetEnsembleFlags(interp, (Tcl_Command)visible(interp, "::N::e"),
        TCL_ENSEMBLE_PREFIX | ENSEMBLE_COMPILE);
    report(interp, "reattach_source", code);
    code = Tcl_Eval(interp, "namespace ensemble configure ::N::e -prefixes 0");
    report(interp, "configure_hooked", code);
#else
    code = Tcl_Eval(interp,
        "namespace eval N {namespace export e; proc e {} {return SOURCE}};"
        "namespace import ::N::e; rename e old");
    report(interp, "import_absent", code);
#endif
    code = Tcl_Eval(interp, "rename old moved; rename moved old");
    report(interp, "rename_old", code);
    code = Tcl_HideCommand(interp, "old", "hiddenOld");
    report(interp, "hide_old", code);
    code = Tcl_ExposeCommand(interp, "hiddenOld", "old");
    report(interp, "expose_old", code);
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
    code = Tcl_HideCommand(interp, "fresh", "hiddenFresh");
    report(interp, "hide_fresh", code);
    code = Tcl_ExposeCommand(interp, "hiddenFresh", "fresh");
    report(interp, "expose_fresh", code);
#endif
    code = Tcl_Eval(interp, "rename ::N::e {}");
    report(interp, "delete_source", code);
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return 0;
}
