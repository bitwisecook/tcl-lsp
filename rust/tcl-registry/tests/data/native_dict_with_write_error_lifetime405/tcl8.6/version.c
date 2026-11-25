#include <stdio.h>
#include "tcl.h"
int main(void) {
    Tcl_FindExecutable("with-lifetime405");
    Tcl_Interp *interp = Tcl_CreateInterp();
    int code = Tcl_Eval(interp, "info patchlevel");
    printf("%d", code);
    putchar(9);
    printf("%s", Tcl_GetStringResult(interp));
    putchar(10);
    Tcl_DeleteInterp(interp);
    return code;
}
