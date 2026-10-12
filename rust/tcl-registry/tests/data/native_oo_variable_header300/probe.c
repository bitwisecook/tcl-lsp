#include <stdio.h>
#include <string.h>
#ifdef USE_JIM
#include "jim.h"
int main(int argc, char **argv) {
    (void)argc; (void)argv;
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
    if (Jim_Eval(interp, "info patchlevel") != JIM_OK) return 3;
    int length; const char *version = Jim_GetString(Jim_GetResult(interp), &length);
    printf("VERSION|"); for (int n = 0; n < length; ++n) printf("%02x", (unsigned char)version[n]);
    printf("\nNOT_APPLICABLE|stock-TclOO-unavailable\n");
    Jim_FreeInterp(interp); return 0;
}
#else
#include "tcl.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
int main(int argc, char **argv) {
    (void)argc; Tcl_FindExecutable(argv[0]);
    Tcl_Interp *interp = Tcl_CreateInterp();
    if (Tcl_Init(interp) != TCL_OK) return 2;
    if (Tcl_Eval(interp, "info patchlevel") != TCL_OK) return 3;
    Count length; const char *version = Tcl_GetStringFromObj(Tcl_GetObjResult(interp), &length);
    printf("VERSION|"); for (Count n = 0; n < length; ++n) printf("%02x", (unsigned char)version[n]); puts("");
    if (Tcl_Eval(interp, "oo::class create C") != TCL_OK) {
        puts("NOT_APPLICABLE|stock-TclOO-unavailable"); Tcl_DeleteInterp(interp); return 0;
    }
    const char original_bytes[] = {'a', 0, 'z'};
    Tcl_Obj *original = Tcl_NewStringObj(original_bytes, 3);
    Tcl_Obj *declaration[] = {Tcl_NewStringObj("oo::define", -1), Tcl_NewStringObj("C", -1), Tcl_NewStringObj("variable", -1), original};
    for (int n = 0; n < 4; ++n) Tcl_IncrRefCount(declaration[n]);
    int declare_code = Tcl_EvalObjv(interp, 4, declaration, 0);
    printf("DECLARE|%d\n", declare_code);
    Tcl_Obj *query[] = {Tcl_NewStringObj("info", -1), Tcl_NewStringObj("class", -1), Tcl_NewStringObj("variables", -1), Tcl_NewStringObj("C", -1)};
    for (int n = 0; n < 4; ++n) Tcl_IncrRefCount(query[n]);
    int query_code = Tcl_EvalObjv(interp, 4, query, 0);
    Count count = 0; Tcl_Obj **members = NULL;
    int list_code = query_code == TCL_OK ? Tcl_ListObjGetElements(interp, Tcl_GetObjResult(interp), &count, &members) : TCL_ERROR;
    int same = list_code == TCL_OK && count == 1 && members[0] == original;
    printf("QUERY|%d|%d|%lld|%d|", query_code, list_code, (long long)count, same);
    if (list_code == TCL_OK && count == 1) {
        const char *bytes = Tcl_GetStringFromObj(members[0], &length);
        for (Count n = 0; n < length; ++n) printf("%02x", (unsigned char)bytes[n]);
    }
    puts("");
    for (int n = 0; n < 4; ++n) Tcl_DecrRefCount(query[n]);
    for (int n = 0; n < 4; ++n) Tcl_DecrRefCount(declaration[n]);
    Tcl_DeleteInterp(interp); Tcl_Finalize(); return 0;
}
#endif
