/*
 * ============================================================================
 * SPIKE -- throwaway proof-of-concept. NOT the final design. Representative
 * subset of Tcl's bundled libtommath C API, enough to compile a bignum
 * extension. See docs/design/runtime/c-extension-abi.md.
 * ============================================================================
 *
 * tclTomMath.h -- authored, API-compatible subset of the mp_* bignum API.
 *
 * In real Tcl these names are #define'd to TclBN_* and routed through a
 * dedicated stub table; here they are declared directly (our ABI is direct
 * linking). Raw mp_* arithmetic is the one thing that genuinely needs this
 * sibling header -- merely passing bignums in/out of Tcl_Objs uses
 * Tcl_NewBignumObj / Tcl_GetBignumFromObj in tcl.h.
 */
#ifndef SPIKE_TCLTOMMATH_H
#define SPIKE_TCLTOMMATH_H

#include "tcl.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef uint32_t mp_digit;
typedef enum { MP_ZPOS = 0, MP_NEG = 1 } mp_sign;
typedef enum { MP_OKAY = 0, MP_ERR = -1, MP_MEM = -2, MP_VAL = -3 } mp_err;

typedef struct {
    int used, alloc;
    mp_sign sign;
    mp_digit *dp;
} mp_int;

extern mp_err mp_init(mp_int *a);
extern void mp_clear(mp_int *a);
extern void mp_set(mp_int *a, uint32_t b);
extern mp_err mp_add(const mp_int *a, const mp_int *b, mp_int *c);
extern mp_err mp_sub(const mp_int *a, const mp_int *b, mp_int *c);
extern mp_err mp_mul(const mp_int *a, const mp_int *b, mp_int *c);

#ifdef __cplusplus
}
#endif

#endif /* SPIKE_TCLTOMMATH_H */
