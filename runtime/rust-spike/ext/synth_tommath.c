/*
 * ============================================================================
 * SPIKE -- throwaway proof-of-concept, NOT a real extension. Synthetic bignum
 * extension that compile-checks tclTomMath.h (raw mp_* arithmetic) plus the
 * public Tcl_NewBignumObj in tcl.h. Never run.
 * ============================================================================
 */
#undef STATIC_BUILD
#include "tcl.h"
#include "tclTomMath.h"

DLLEXPORT int Synthbn_Init(Tcl_Interp *interp) {
    if (Tcl_InitStubs(interp, "9.0-", 0) == NULL) {
        return TCL_ERROR;
    }
    mp_int a, b, c;
    mp_init(&a);
    mp_init(&b);
    mp_init(&c);
    mp_set(&a, 40);
    mp_set(&b, 2);
    mp_add(&a, &b, &c);
    Tcl_SetObjResult(interp, Tcl_NewBignumObj(&c));
    mp_clear(&a);
    mp_clear(&b);
    mp_clear(&c);
    Tcl_PkgProvide(interp, "synthbn", "1.0");
    return TCL_OK;
}
