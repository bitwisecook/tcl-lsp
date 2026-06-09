/*
 * tclInt.h — minimal shim for linking Tcl 9's regex engine standalone.
 *
 * Upstream Tcl's `regex.h` does `#include "tclInt.h"` for the
 * `Tcl_UniChar` typedef.  Providing a stub here lets us compile Tcl's
 * Henry-Spencer regex engine (`regcomp.c`/`regexec.c` + the `#include`d
 * `regc_*.c`/`rege_dfa.c`) into the Rust runtime without dragging in the
 * full Tcl internal header.  This mirrors the Zig runtime's
 * `runtime/zig/regex_include/tclInt.h` (the behavioural oracle), retargeted
 * from wasm32 to the native host the Rust runtime's `build.rs` compiles for.
 *
 * `regex.h` is the only engine file that pulls this in — every other
 * reference goes through `regcustom.h`.
 */
#ifndef _TCL_INT_REGEX_SHIM_H_
#define _TCL_INT_REGEX_SHIM_H_

#include <stdint.h>
#include <stddef.h>

/*
 * Tcl 9.0 picks `Tcl_UniChar`'s width from `TCL_UTF_MAX`: the wide form
 * (`TCL_UTF_MAX == 4`) is a full-Unicode 32-bit codepoint.  We always
 * target the wide form — 21-bit codepoints in 32 bits — so `int32_t` is
 * the right match regardless of host `int` width.  `regcustom.h` (our
 * override) pins `CHRBITS 32` / `CHR_MAX 0x10FFFF` to agree.
 */
#ifndef _TCL_UNICHAR_TYPEDEF_
#define _TCL_UNICHAR_TYPEDEF_
typedef int32_t Tcl_UniChar;
#endif

/*
 * `MODULE_SCOPE` carries hidden-visibility attributes in a full Tcl
 * shared-library build; standalone we degrade it to plain `extern`.
 */
#ifndef MODULE_SCOPE
#define MODULE_SCOPE extern
#endif

/*
 * Tcl's printf modifier for `size_t` — only used in the engine's debug
 * `fprintf` paths (normally compiled out), but the definitions must be
 * well-formed C regardless.  The native host follows C99.
 */
#ifndef TCL_Z_MODIFIER
#define TCL_Z_MODIFIER "z"
#endif

/*
 * The handful of Tcl API functions `regc_locale.c` reaches for: named
 * collating-element lookup and case-independent range expansion.
 * Upstream backs these with Tcl's multi-KB Unicode tables; our
 * `regex_shim.c` provides ASCII-only implementations — sufficient for the
 * library's load-time patterns (`[a-z]`, `\w`, `\s`, `\d`) and the
 * compute-only `.test` slices we target first.  Swapping in full-Unicode
 * versions later is mechanical (the engine never sees the difference).
 *
 * `Tcl_DString` is exposed as a plain struct (not the public Tcl type) so
 * the engine's `Tcl_DStringInit(&ds); np = Tcl_UniCharToUtfDString(...)`
 * usage compiles unchanged.  The fields mirror the public layout (string
 * pointer + length/capacity bookkeeping); we omit the embedded static
 * buffer and always heap-allocate.
 */
typedef struct Tcl_DString {
    char *string;
    size_t length;
    size_t spaceAvl;
} Tcl_DString;

MODULE_SCOPE void Tcl_DStringInit(Tcl_DString *dsPtr);
MODULE_SCOPE void Tcl_DStringFree(Tcl_DString *dsPtr);
MODULE_SCOPE char *Tcl_UniCharToUtfDString(
    const Tcl_UniChar *uniStr, int uniLength, Tcl_DString *dsPtr);

MODULE_SCOPE Tcl_UniChar Tcl_UniCharToLower(int ch);
MODULE_SCOPE Tcl_UniChar Tcl_UniCharToUpper(int ch);
MODULE_SCOPE Tcl_UniChar Tcl_UniCharToTitle(int ch);

#endif /* _TCL_INT_REGEX_SHIM_H_ */
