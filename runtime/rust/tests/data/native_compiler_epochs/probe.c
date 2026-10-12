#include "tclInt.h"
#include <stdio.h>
#include <string.h>

static void snapshot(Tcl_Interp *interp, const char *event, int result) {
    Interp *i = (Interp *)interp;
    Namespace *n = (Namespace *)Tcl_FindNamespace(interp, "::N", NULL, 0);
    Command *set = (Command *)Tcl_FindCommand(interp, "::set", NULL, TCL_GLOBAL_ONLY);
    printf("%s\t%d\t%d\t%d\t%d\t%d\t%d\n", event, result, i->compileEpoch,
        i->globalNsPtr->resolverEpoch, n ? n->resolverEpoch : -1,
        n ? n->cmdRefEpoch : -1, set ? set->compileProc != NULL : -1);
    fflush(stdout);
}
static int reached(Tcl_Interp *interp, const char *event, const char *script) {
    int code = Tcl_EvalEx(interp, script, (int)strlen(script), TCL_EVAL_GLOBAL);
    snapshot(interp, event, code);
    if (code != TCL_OK) {
        fprintf(stderr, "%s: %s\n", event, Tcl_GetStringResult(interp));
        return 1;
    }
    return 0;
}
int main(int argc, char **argv) {
    int failed = 0;
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *interp = Tcl_CreateInterp();
    puts("event\tcode\tcompiler\tglobal_resolver\tN_resolver\tN_command_reference\tset_compiler");
    snapshot(interp, "initial", TCL_OK);
    failed |= reached(interp, "namespace_create", "namespace eval N {}");
    failed |= reached(interp, "plain_create", "proc p {} {return P}");
    failed |= reached(interp, "plain_rename", "rename p q");
    failed |= reached(interp, "plain_delete", "rename q {}");
    failed |= reached(interp, "compiled_shadow", "proc N::set {} {return LOCAL}");
    failed |= reached(interp, "shadow_delete", "rename N::set {}");
    failed |= reached(interp, "compiled_rename", "rename set stamp_saved_set");
    failed |= reached(interp, "replacement_plain", "proc set {args} {return REPLACEMENT}");
    failed |= reached(interp, "replacement_delete", "rename set {}");
    failed |= reached(interp, "compiled_restore", "rename stamp_saved_set set");
    failed |= reached(interp, "compiled_hide", "interp hide {} set");
    failed |= reached(interp, "compiled_expose", "interp expose {} set");
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
    failed |= reached(interp, "namespace_path", "namespace eval M {}; namespace eval N {namespace path ::M}");
    failed |= reached(interp, "same_path", "namespace eval N {namespace path ::M}");
    failed |= reached(interp, "clear_path", "namespace eval N {namespace path {}}");
#endif
    failed |= reached(interp, "namespace_delete", "namespace delete N");
    failed |= reached(interp, "namespace_recreate", "namespace eval N {}");
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return failed;
}
