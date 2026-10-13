#include <limits.h>
#include <stddef.h>
#include <tcl.h>
int Aliasabi_Init(Tcl_Interp *interp) {
    Tcl_Obj *values[3];
    if (Tcl_InitStubs(interp, TCL_VERSION, 0) == NULL) return TCL_ERROR;
    values[0] = Tcl_NewIntObj(CHAR_MIN);
    values[1] = Tcl_NewIntObj((int)sizeof(unsigned int));
    values[2] = Tcl_NewIntObj((int)sizeof(size_t));
    Tcl_SetObjResult(interp, Tcl_NewListObj(3, values));
    return TCL_OK;
}
