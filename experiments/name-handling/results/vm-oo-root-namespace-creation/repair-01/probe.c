#include <stdio.h>
#include <string.h>
#include "tcl.h"
#include "tclOO.h"

static int exists(Tcl_Interp *interp, const char *name, int expected) {
    Tcl_Obj *words[] = {Tcl_NewStringObj("namespace", -1),
        Tcl_NewStringObj("exists", -1), Tcl_NewStringObj(name, -1)};
    for (int i = 0; i < 3; ++i) Tcl_IncrRefCount(words[i]);
    int code = Tcl_EvalObjv(interp, 3, words, TCL_EVAL_GLOBAL);
    int observed = -1;
    int converted = Tcl_GetBooleanFromObj(interp, Tcl_GetObjResult(interp), &observed);
    for (int i = 0; i < 3; ++i) Tcl_DecrRefCount(words[i]);
    printf("EXISTS|%s|code=%d|converted=%d|observed=%d|expected=%d\n",
        name, code, converted, observed, expected);
    return code != TCL_OK || converted != TCL_OK || observed != expected;
}
int main(int argc, char **argv) {
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *interp = Tcl_CreateInterp();
    int failed = Tcl_Init(interp) != TCL_OK;
    if (failed) { fprintf(stderr, "%s\n", Tcl_GetStringResult(interp)); return 2; }
    printf("VERSION|%s\n", Tcl_GetVar(interp, "tcl_patchLevel", TCL_GLOBAL_ONLY));
    const char *roots[] = {"::oo::object", "::oo::class", "::oo::configurable"};
    for (int i = 0; i < 3; ++i) {
        Tcl_Obj *name = Tcl_NewStringObj(roots[i], -1);
        Tcl_IncrRefCount(name);
        Tcl_Object object = Tcl_GetObjectFromObj(interp, name);
        Tcl_Namespace *owned = object ? Tcl_GetObjectNamespace(object) : NULL;
        Tcl_Namespace *found = owned ? Tcl_FindNamespace(interp, owned->fullName, NULL, TCL_GLOBAL_ONLY) : NULL;
        int same = owned != NULL && found == owned;
        printf("ORIGINAL_ROOT|%s|object=%d|owned=%d|found_same=%d\n",
            roots[i], object != NULL, owned != NULL, same);
        failed |= !same;
        if (owned) failed |= exists(interp, owned->fullName, 1);
        Tcl_DecrRefCount(name);
    }
    const char *missing = "::missing_original_oo_private_namespace";
    int absent = Tcl_FindNamespace(interp, missing, NULL, TCL_GLOBAL_ONLY) == NULL;
    printf("ABSENT|find_absent=%d\n", absent);
    failed |= !absent;
    failed |= exists(interp, missing, 0);
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return failed ? 1 : 0;
}
