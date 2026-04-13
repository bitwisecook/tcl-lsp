/*
 * regcustom.h — WASM runtime override of Tcl's regex environment
 * customisation header.  Upstream ``regcustom.h`` (at the root of
 * Tcl's ``generic/``) exists precisely so embedders can replace it
 * — the comment at the top of every engine file says not to touch
 * anything else.  ``scripts/fetch_tcl_regex.sh`` omits the upstream
 * copy from its fetch list; ``build.zig`` adds
 * ``runtime/zig/regex_include`` to the compiler's ``-I`` path so
 * ``regguts.h``'s ``#include "regcustom.h"`` resolves here.
 *
 * Differences from upstream:
 *
 *   - ``MALLOC``/``FREE``/``REALLOC`` bind to wasi-libc's ``malloc``
 *     / ``free`` / ``realloc`` directly rather than Tcl's
 *     ``Tcl_AttemptAlloc`` family.  Our WASM target links wasi-libc
 *     automatically and the engine only needs a standard heap.
 *
 *   - ``AllocVars`` resolves to a file-static ``struct vars``
 *     instance instead of ``Tcl_GetThreadData``.  The runtime is
 *     single-threaded (WASM has no threads in the reactor model we
 *     use) and upstream's own comment notes the engine is never
 *     reentered from the same thread, so one static instance is
 *     correct.
 *
 *   - The four ``iscXXX`` character predicates call local helpers
 *     (``TclWasmRegIsAlnum`` etc.) defined in ``tcl_reg_shim.c``.
 *     They cover ASCII exactly (which is all tcltest's patterns
 *     need); Unicode extensions can be added by extending those
 *     helpers without touching the engine.
 *
 * Everything else — type aliases, ``CHR`` / ``CHRBITS`` / ``CHR_MAX``,
 * the name renames of the exec/compile entry points, the
 * ``__REG_NOFRONT`` / ``__REG_NOCHAR`` flags — matches upstream
 * verbatim so the engine sources work unchanged.
 */

/* Henry Spencer's original boilerplate retained for attribution. */
/*
 * Copyright (c) 1998, 1999 Henry Spencer.  All rights reserved.
 * Redistribution permitted under the BSD-style terms in the
 * header of the corresponding upstream file at
 * https://github.com/tcltk/tcl/blob/core-9-0-3/generic/regcustom.h
 */

#ifndef _REG_CUSTOM_H_
#define _REG_CUSTOM_H_

#include "tclInt.h" /* Tcl_UniChar, MODULE_SCOPE */
#include "regex.h"  /* regex_t, REG_OKAY etc. — regguts.h/engine need them */
#include <stdio.h>  /* FILE (used in un-gated ``static dump(..., FILE*)`` prototypes) */
#include <stdlib.h> /* malloc / free / realloc */
#include <string.h> /* strlen for regfronts.c */
#include <limits.h> /* CHAR_BIT for regguts.h's UBITS macro */

/*
 * Memory — wasi-libc's heap rather than Tcl_Attempt* wrappers.
 */
#define MALLOC(n) malloc(n)
#define FREE(p) free(p)
#define REALLOC(p, n) realloc(p, n)

/*
 * Splice point normally stamped by regfwd from regex.h's
 * ``--- begin --- / --- end ---`` chunk.  Kept verbatim so the
 * downstream ``regex.h`` file (which expects these macros to be
 * pre-defined by some header chain) sees them.
 */
#ifdef __REG_WIDE_T
#undef __REG_WIDE_T
#endif
#ifdef __REG_WIDE_COMPILE
#undef __REG_WIDE_COMPILE
#endif
#ifdef __REG_WIDE_EXEC
#undef __REG_WIDE_EXEC
#endif
#ifdef __REG_NOFRONT
#undef __REG_NOFRONT
#endif
#ifdef __REG_NOCHAR
#undef __REG_NOCHAR
#endif
#define __REG_WIDE_T Tcl_UniChar
#define __REG_WIDE_COMPILE TclReComp
#define __REG_WIDE_EXEC TclReExec
#define __REG_NOFRONT /* Don't want regcomp() and regexec() */
#define __REG_NOCHAR  /* Or the char versions */
#define regfree TclReFree
#define regerror TclReError

/*
 * Internal character type and related.
 */
typedef Tcl_UniChar chr;  /* The character type. */
typedef int pchr;         /* What it promotes to. */
typedef unsigned uchr;    /* Unsigned type that will hold a chr. */
typedef int celt;         /* Type to hold chr, or NOCELT. */
#define NOCELT (-1)
#define CHR(c) ((unsigned char)(c))
#define DIGITVAL(c) ((c) - '0')
#define CHRBITS 32
#define CHR_MIN 0x00000000
#define CHR_MAX 0x10FFFF

/*
 * Character classification — ASCII-only for now.  Implementations
 * live in ``tcl_reg_shim.c`` so extending them to full Unicode
 * doesn't require editing this header.  tcltest's init patterns
 * only use ASCII (``[a-z]``, ``\w``, ``\s``, ``\d``), so ASCII is
 * sufficient to unblock the tcllib runner.
 */
extern int TclWasmRegIsAlnum(int c);
extern int TclWasmRegIsAlpha(int c);
extern int TclWasmRegIsDigit(int c);
extern int TclWasmRegIsSpace(int c);

#define iscalnum(x) TclWasmRegIsAlnum(x)
#define iscalpha(x) TclWasmRegIsAlpha(x)
#define iscdigit(x) TclWasmRegIsDigit(x)
#define iscspace(x) TclWasmRegIsSpace(x)

/*
 * External entry-point rename so the engine exports
 * ``TclReComp``/``TclReExec`` rather than the generic ``compile`` /
 * ``exec`` (which would collide with stdlib names in the wasi-libc
 * link).  Upstream does the same.
 */
#define compile TclReComp
#define exec TclReExec

/*
 * Debug is off unless explicitly requested at build time.
 */
#if 0
#define REG_DEBUG /* */
#endif

/*
 * Workspace allocation — single static instance.  Upstream uses
 * thread-local storage via ``Tcl_GetThreadData``; we're
 * single-threaded so a file-static suffices.  Upstream's own
 * comment acknowledges the engine is never reentered from the
 * same thread, so a static instance matches the intended
 * semantics exactly.
 */
#define AllocVars(vPtr)   \
    static struct vars _TclWasmRegVars; \
    struct vars *vPtr = &_TclWasmRegVars

#endif /* _REG_CUSTOM_H_ */
