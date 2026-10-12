#include <errno.h>
#include <math.h>
#include <stdio.h>
#ifdef JIM
#include "jim.h"
#else
#include "tcl.h"
extern void TclExprFloatError(Tcl_Interp *, double);
#endif
static int seed_errno;
static double seed_value;
static void bytes(const char *s) { if (!s) return; while (*s) printf("%02x", (unsigned char)*s++); }
#ifdef JIM
static int seed(Jim_Interp *i, int argc, Jim_Obj *const *argv) {
    (void)argc; (void)argv;
    Jim_SetResult(i, Jim_NewDoubleObj(i, seed_value));
    errno = seed_errno;
    return JIM_OK;
}
#else
static int seed(ClientData data, Tcl_Interp *i, int argc, Tcl_Obj *const argv[]) {
    (void)data; (void)argc; (void)argv;
    Tcl_SetObjResult(i, Tcl_NewDoubleObj(seed_value));
    errno = seed_errno;
    return TCL_OK;
}
#endif
int main(void) {
    const int errors[] = {0, EDOM, ERANGE, EINVAL, EDOM, ERANGE};
    const double values[] = {INFINITY, INFINITY, INFINITY, NAN, 1.0, 0.0};
    for (int row = 0; row < 6; row++) {
#ifdef JIM
        Jim_Interp *i = Jim_CreateInterp();
        Jim_RegisterCoreCommands(i);
        Jim_CreateCommand(i, "seed", seed, NULL, NULL);
        Jim_Eval(i, "proc p {} {expr {[seed]}}");
#else
        Tcl_Interp *i = Tcl_CreateInterp();
        Tcl_CreateObjCommand(i, "seed", seed, NULL, NULL);
        Tcl_Eval(i, "proc p {} {expr {[seed]}}");
#endif
        seed_errno = errors[row]; seed_value = values[row];
#ifdef JIM
        int code = Jim_Eval(i, "p");
        int observed_errno = errno;
        printf("ENTER\t%d\t%d\t%d\t%d\t%d\t", row, code, observed_errno, observed_errno == EDOM, observed_errno == ERANGE);
        bytes(Jim_GetString(Jim_GetResult(i), NULL));
        puts("");
        Jim_FreeInterp(i);
#else
        int code = Tcl_Eval(i, "p");
        int observed_errno = errno;
        printf("ENTER\t%d\t%d\t%d\t%d\t%d\t", row, code, observed_errno, observed_errno == EDOM, observed_errno == ERANGE);
        bytes(Tcl_GetStringResult(i));
        printf("\t"); bytes(Tcl_GetVar(i, "errorCode", TCL_GLOBAL_ONLY)); puts("");
        Tcl_DeleteInterp(i);
#endif
    }
#ifndef JIM
    const int direct_errors[] = {0, EDOM, ERANGE, EINVAL, ERANGE, 0};
    const double direct_values[] = {INFINITY, INFINITY, 0.0, 1.0, NAN, 1.0};
    for (int row = 0; row < 6; row++) {
        Tcl_Interp *i = Tcl_CreateInterp();
        errno = direct_errors[row];
        TclExprFloatError(i, direct_values[row]);
        int observed_errno = errno;
        printf("DIRECT\t%d\t%d\t%d\t%d\t", row, observed_errno, observed_errno == EDOM, observed_errno == ERANGE);
        bytes(Tcl_GetStringResult(i)); printf("\t"); bytes(Tcl_GetVar(i, "errorCode", TCL_GLOBAL_ONLY)); puts("");
        Tcl_DeleteInterp(i);
    }
#endif
    return 0;
}
