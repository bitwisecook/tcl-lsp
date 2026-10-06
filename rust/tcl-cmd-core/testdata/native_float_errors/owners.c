// SPDX-License-Identifier: AGPL-3.0-or-later
#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tclInt.h"
extern void TclExprFloatError(Tcl_Interp *, double);

static void bytes(const char *s) {
    if (s) while (*s) printf("%02x", (unsigned char)*s++);
}

/* C8.4's producer stores the actual global variable; later C owns an
 * independent Interp errorCode. This getter walks no public read/trace path. */
static Tcl_Obj *selected_code(Tcl_Interp *interp) {
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
    Tcl_HashEntry *entry = Tcl_FindHashEntry(&((Interp *)interp)->globalNsPtr->varTable,
                                          "errorCode");
    Var *variable = entry ? (Var *)Tcl_GetHashValue(entry) : NULL;
    return variable && !TclIsVarUndefined(variable) ? variable->value.objPtr : NULL;
#else
    return ((Interp *)interp)->errorCode;
#endif
}

/* Copy only report bytes, without retaining a native object owner.
 * The public lookup can replace/free the exact original error-code object. */
static char *code_bytes(Tcl_Interp *interp) {
    Tcl_Obj *code = selected_code(interp);
    if (!code) return NULL;
    const char *value = Tcl_GetString(code);
    size_t length = strlen(value);
    char *copy = (char *)malloc(length + 1);
    if (!copy) exit(2);
    memcpy(copy, value, length + 1);
    return copy;
}

int main(void) {
    const int errors[] = {0, EDOM, ERANGE, EINVAL, ERANGE, 0};
    const double values[] = {INFINITY, INFINITY, 0.0, 1.0, NAN, 1.0};
    for (int row = 0; row < 6; row++) {
        Tcl_Interp *interp = Tcl_CreateInterp();
        errno = errors[row];
        TclExprFloatError(interp, values[row]);
        int observed = errno;
        char *before = code_bytes(interp);
        printf("DIRECT\t%d\t%d\t%d\t%d\t", row, observed,
               observed == EDOM, observed == ERANGE);
        bytes(Tcl_GetStringResult(interp));
        putchar('\t');
        /* Keep the original public diagnostic observation in its original
         * column. This read can itself replace the private error-code owner. */
        bytes(Tcl_GetVar(interp, "errorCode", TCL_GLOBAL_ONLY));
        putchar('\t');
        bytes(before);
        putchar('\t');
        char *after = code_bytes(interp);
        bytes(after);
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
        printf("\tnamespace-cell");
#else
        printf("\tprivate-interp");
#endif
        printf("\tbefore-public-getter/after-public-getter\n");
        free(before);
        free(after);
        Tcl_DeleteInterp(interp);
    }
    return 0;
}
