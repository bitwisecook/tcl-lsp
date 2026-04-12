/*
 * tclInt.h — minimal shim.
 *
 * Upstream Tcl's ``regex.h`` does ``#include "tclInt.h"`` for the
 * ``Tcl_UniChar`` typedef.  Providing a stub here lets us link
 * Tcl's regex engine into the WASM runtime without dragging in the
 * full Tcl internal header (thousands of lines of interp/obj/cmd
 * APIs the engine doesn't actually use).
 *
 * ``regex.h`` is the only file in the engine that pulls this in —
 * every other reference goes through ``regcustom.h``.
 */
#ifndef _TCL_INT_STUB_H_
#define _TCL_INT_STUB_H_

#include <stdint.h>

/*
 * Tcl 9.0 chooses the ``Tcl_UniChar`` integer size from
 * ``TCL_UTF_MAX``: a value of 4 gives a full-Unicode 32-bit
 * codepoint (``typedef int Tcl_UniChar``).  We always target
 * Tcl 9's wide form — 21-bit codepoints stored in 32 bits —
 * so ``int32_t`` is the right match regardless of host ``int``
 * width.  ``regcustom.h`` (our override) pins ``CHRBITS 32``
 * and ``CHR_MAX 0x10FFFF`` to agree with this.
 */
#ifndef _TCL_UNICHAR_TYPEDEF_
#define _TCL_UNICHAR_TYPEDEF_
typedef int32_t Tcl_UniChar;
#endif

/*
 * ``MODULE_SCOPE`` in upstream Tcl carries hidden-visibility
 * attributes for shared-library builds.  The WASM runtime is a
 * single-module reactor; we don't control symbol visibility from
 * here, so degrade the attribute to plain ``extern`` for function
 * prototypes.  Matches the definition used by other embedders of
 * Tcl's regex engine that don't ship the full Tcl core.
 */
#ifndef MODULE_SCOPE
#define MODULE_SCOPE extern
#endif

/*
 * ``TCL_Z_MODIFIER`` is Tcl's printf modifier for ``size_t`` —
 * ``"z"`` on POSIX / C99, ``"I"`` on Windows.  Only used in the
 * engine's debug ``fprintf`` paths (REG_FTRACE / REG_MTRACE,
 * normally off) and tree-dump paths (also normally off), but the
 * definitions need to be well-formed C regardless.  We always use
 * the C99 ``"z"`` form — wasi-libc follows C99.
 */
#ifndef TCL_Z_MODIFIER
#define TCL_Z_MODIFIER "z"
#endif

/*
 * Tcl API functions the regex engine (specifically ``regc_locale.c``)
 * reaches out to for named-collating-element lookup and
 * case-independent range expansion.  Upstream they live in
 * ``tclInt.h`` / ``tcl.h`` backed by Tcl's Unicode tables (tens of
 * KB of ``.c`` data).  Our ``tcl_reg_shim.c`` provides ASCII-only
 * implementations — tcltest init and counter.test only use ASCII
 * patterns, so this is sufficient to unblock the runner.  Swapping
 * in full-Unicode versions later is mechanical.
 *
 * ``Tcl_DString`` is exposed as a plain struct rather than the
 * upstream public type so the engine's ``Tcl_DStringInit(&ds);
 * np = Tcl_UniCharToUtfDString(...);`` usage compiles unchanged.
 * The fields mirror upstream's public layout (string pointer plus
 * length/spaceAvl bookkeeping) but we omit the 200-byte embedded
 * ``staticSpace`` — we always heap-allocate.
 */

typedef struct Tcl_DString {
    char *string;
    int length;
    int spaceAvl;
} Tcl_DString;

MODULE_SCOPE void Tcl_DStringInit(Tcl_DString *dsPtr);
MODULE_SCOPE void Tcl_DStringFree(Tcl_DString *dsPtr);
MODULE_SCOPE char *Tcl_UniCharToUtfDString(
    const Tcl_UniChar *uniStr, int uniLength, Tcl_DString *dsPtr);

MODULE_SCOPE Tcl_UniChar Tcl_UniCharToLower(int ch);
MODULE_SCOPE Tcl_UniChar Tcl_UniCharToUpper(int ch);
MODULE_SCOPE Tcl_UniChar Tcl_UniCharToTitle(int ch);

#endif /* _TCL_INT_STUB_H_ */
