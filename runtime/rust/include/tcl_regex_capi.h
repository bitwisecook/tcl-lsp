// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

/*
 * tcl_regex_capi.h — C interface to the pure-Rust Tcl 9 ARE engine.
 *
 * These are the Tcl regex engine entry points (`TclReComp` / `TclReExec` /
 * `TclReFree` / `TclReError`), implemented in Rust by `runtime/rust`'s
 * `regex_capi` module over the `tcl-regex` crate. A C Tcl build or extension
 * that used to link the C Henry-Spencer engine can link this instead,
 * unchanged: the symbol names, the `regex_t` / `regmatch_t` layout, the
 * `REG_*` flag and error values, and the `(size_t)-1` non-participation
 * sentinel all match `generic/regex.h`.
 *
 * `chr` is a 32-bit codepoint (the engine's `CHRBITS == 32` convention).
 */
#ifndef TCL_REGEX_CAPI_H
#define TCL_REGEX_CAPI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef uint32_t tclre_chr; /* wide char = full Unicode codepoint */

typedef struct {
    size_t rm_so; /* start of substring ((size_t)-1 = did not participate) */
    size_t rm_eo; /* end of substring (exclusive) */
} regmatch_t;

typedef struct {
    regmatch_t rm_extend; /* REG_EXPECT extent (unused by this engine) */
} rm_detail_t;

typedef struct {
    int re_magic;
    long re_info;
    size_t re_nsub;        /* number of capturing subexpressions */
    const tclre_chr *re_endp;
    void *re_guts;         /* opaque: the compiled engine state */
    void *re_fns;
} regex_t;

/* Compile flags (subset; values match generic/regex.h). */
#define REG_BASIC 000000
#define REG_EXTENDED 000001
#define REG_ADVF 000002
#define REG_ADVANCED 000003
#define REG_QUOTE 000004
#define REG_ICASE 000010
#define REG_NOSUB 000020
#define REG_EXPANDED 000040
#define REG_NLSTOP 000100
#define REG_NLANCH 000200
#define REG_NEWLINE 000300

/* Execution flags. */
#define REG_NOTBOL 0001
#define REG_NOTEOL 0002

/* Result codes. */
#define REG_OKAY 0
#define REG_NOMATCH 1

/*
 * Compile `string` (a `len`-codepoint pattern) under `flags` into `*preg`.
 * Returns REG_OKAY or a REG_* error code.
 */
int TclReComp(regex_t *preg, const tclre_chr *string, size_t len, int flags);

/*
 * Match `*preg` against `string` (`len` codepoints). Fills up to `nmatch`
 * entries of `pmatch` (group 0 is the whole match). `details` is accepted for
 * ABI compatibility and ignored. Returns REG_OKAY or REG_NOMATCH.
 */
int TclReExec(regex_t *preg, const tclre_chr *string, size_t len,
              rm_detail_t *details, size_t nmatch, regmatch_t pmatch[],
              int flags);

/* Release the engine state held in `preg->re_guts`. */
void TclReFree(regex_t *preg);

/*
 * Write the message for `errcode` into `errbuf` (NUL-terminated, truncated to
 * `errbuf_size`). Returns the full message length + 1.
 */
size_t TclReError(int errcode, char *errbuf, size_t errbuf_size);

#ifdef __cplusplus
}
#endif

#endif /* TCL_REGEX_CAPI_H */
