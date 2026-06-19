/*
 * regcustom.h — Rust-runtime override of Tcl's regex environment header.
 *
 * Upstream `regcustom.h` exists precisely so embedders can replace it (the
 * banner on every engine file says not to touch anything else).  Our
 * `build.rs` copies the engine sources into `$OUT_DIR` *omitting* the
 * upstream copy and puts `regex_shim/` first on the `-I` path, so
 * `regguts.h`'s `#include "regcustom.h"` resolves here.
 *
 * Adapted from the Zig runtime's `runtime/zig/regex_include/regcustom.h`
 * (the behavioural oracle), retargeted from wasm32 to the native host:
 *
 *   - MALLOC/FREE/REALLOC bind to the C library heap directly (not Tcl's
 *     `Tcl_AttemptAlloc` family).
 *
 *   - `AllocVars` resolves to a `_Thread_local` static `struct vars`
 *     instead of `Tcl_GetThreadData`.  Upstream uses thread-local storage
 *     because the engine is never reentered *on the same thread*; the Rust
 *     test runner executes tests across threads, so a *file*-static (as the
 *     single-threaded wasm build could use) would race.  `_Thread_local`
 *     gives the same per-thread-singleton semantics as upstream.
 *
 *   - The `iscXXX` predicates and the `Tcl_UniChar*` case/encode helpers
 *     call ASCII-only shims in `regex_shim.c`.
 *
 * Everything else — type aliases, CHR/CHRBITS/CHR_MAX, the exec/compile
 * entry-point renames, `__REG_NOFRONT`/`__REG_NOCHAR` — matches upstream
 * verbatim so the engine sources work unchanged.
 */

/*
 * Copyright (c) 1998, 1999 Henry Spencer.  All rights reserved.
 * Redistribution permitted under the BSD-style terms in the header of the
 * corresponding upstream file (Tcl `core-9-0-3`, `generic/regcustom.h`).
 */

#ifndef _REG_CUSTOM_H_
#define _REG_CUSTOM_H_

#include "tclInt.h" /* Tcl_UniChar, MODULE_SCOPE */
#include "regex.h"  /* regex_t, REG_OKAY etc. */
#include <stdio.h>  /* FILE (un-gated static dump(..., FILE*) prototypes) */
#include <stdlib.h> /* malloc / free / realloc */
#include <string.h> /* strlen for regfronts.c */
#include <limits.h> /* CHAR_BIT for regguts.h's UBITS macro */

/* Memory — the C library heap rather than Tcl_Attempt* wrappers. */
#define MALLOC(n) malloc(n)
#define FREE(p) free(p)
#define REALLOC(p, n) realloc(p, n)

/*
 * Splice point normally stamped by regfwd from regex.h's
 * `--- begin --- / --- end ---` chunk.  Kept verbatim.
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

/* Internal character type and related. */
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
 * Character classification — ASCII-only (implementations in
 * `regex_shim.c`).  The library's load-time patterns only use ASCII.
 */
extern int TclShimRegIsAlnum(int c);
extern int TclShimRegIsAlpha(int c);
extern int TclShimRegIsDigit(int c);
extern int TclShimRegIsSpace(int c);

#define iscalnum(x) TclShimRegIsAlnum(x)
#define iscalpha(x) TclShimRegIsAlpha(x)
#define iscdigit(x) TclShimRegIsDigit(x)
#define iscspace(x) TclShimRegIsSpace(x)

/*
 * Export the wide entry points as `TclReComp`/`TclReExec` rather than the
 * generic `compile`/`exec` (which would collide with libc names).
 */
#define compile TclReComp
#define exec TclReExec

/* Debug off unless explicitly requested. */
#if 0
#define REG_DEBUG /* */
#endif

/*
 * Workspace allocation — a per-thread singleton.  Upstream uses
 * `Tcl_GetThreadData`; `_Thread_local` gives the same semantics (the engine
 * is never reentered on the same thread) while staying race-free under the
 * multi-threaded `cargo test` runner.
 */
#define AllocVars(vPtr)                          \
    static _Thread_local struct vars _TclRegVars; \
    struct vars *vPtr = &_TclRegVars

#endif /* _REG_CUSTOM_H_ */
