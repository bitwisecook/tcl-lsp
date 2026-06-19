/*
 * regex_shim.c — the small set of host functions Tcl's regex engine
 * reaches for, provided standalone so the engine links without the full
 * Tcl core.  Adapted from the Zig runtime's `tcl_reg_shim.c` (the
 * behavioural oracle), minus the wasm-reactor `main` stub.
 *
 * Two groups:
 *   1. The four character-class predicates `regcustom.h` binds
 *      `iscalnum`/`iscalpha`/`iscdigit`/`iscspace` to — ASCII-only.
 *   2. `Tcl_UniCharToLower/Upper/Title` + the `Tcl_DString` /
 *      `Tcl_UniCharToUtfDString` trio `regc_locale.c` uses to build the
 *      UTF-8 form of named collating elements — also ASCII-only.
 *
 * ASCII coverage is correct for ASCII input and a conservative false-negative
 * for non-ASCII (those codepoints match only in their original case).  The
 * Tcl 9 library's load-time patterns and the compute-only `.test` slices we
 * target first are ASCII; a full `Tcl_UniCharIsXxx` port is a mechanical
 * follow-up that the engine never observes.
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "tclInt.h" /* Tcl_UniChar, Tcl_DString */

int TclShimRegIsAlnum(int c)
{
    if (c < 0 || c > 0x7F) {
        return 0;
    }
    return (c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z') ||
           (c >= 'a' && c <= 'z');
}

int TclShimRegIsAlpha(int c)
{
    if (c < 0 || c > 0x7F) {
        return 0;
    }
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
}

int TclShimRegIsDigit(int c)
{
    if (c < 0 || c > 0x7F) {
        return 0;
    }
    return c >= '0' && c <= '9';
}

int TclShimRegIsSpace(int c)
{
    /* POSIX [[:space:]] on ASCII: space, tab, NL, VT, FF, CR. */
    return c == ' ' || c == '\t' || c == '\n' || c == '\v' ||
           c == '\f' || c == '\r';
}

Tcl_UniChar Tcl_UniCharToLower(int ch)
{
    if (ch >= 'A' && ch <= 'Z') {
        return (Tcl_UniChar)(ch + ('a' - 'A'));
    }
    return (Tcl_UniChar)ch;
}

Tcl_UniChar Tcl_UniCharToUpper(int ch)
{
    if (ch >= 'a' && ch <= 'z') {
        return (Tcl_UniChar)(ch - ('a' - 'A'));
    }
    return (Tcl_UniChar)ch;
}

Tcl_UniChar Tcl_UniCharToTitle(int ch)
{
    /* For ASCII, title case coincides with upper case. */
    return Tcl_UniCharToUpper(ch);
}

void Tcl_DStringInit(Tcl_DString *dsPtr)
{
    dsPtr->string = NULL;
    dsPtr->length = 0;
    dsPtr->spaceAvl = 0;
}

void Tcl_DStringFree(Tcl_DString *dsPtr)
{
    if (dsPtr->string != NULL) {
        free(dsPtr->string);
    }
    dsPtr->string = NULL;
    dsPtr->length = 0;
    dsPtr->spaceAvl = 0;
}

/*
 * UTF-8 encoding of a single codepoint.  Writes 1–4 bytes to `out` and
 * returns the byte count.  Invalid codepoints encode as U+FFFD.
 */
static size_t encode_utf8(int cp, char *out)
{
    if (cp < 0 || cp > 0x10FFFF || (cp >= 0xD800 && cp <= 0xDFFF)) {
        cp = 0xFFFD;
    }
    if (cp < 0x80) {
        out[0] = (char)cp;
        return 1;
    } else if (cp < 0x800) {
        out[0] = (char)(0xC0 | (cp >> 6));
        out[1] = (char)(0x80 | (cp & 0x3F));
        return 2;
    } else if (cp < 0x10000) {
        out[0] = (char)(0xE0 | (cp >> 12));
        out[1] = (char)(0x80 | ((cp >> 6) & 0x3F));
        out[2] = (char)(0x80 | (cp & 0x3F));
        return 3;
    } else {
        out[0] = (char)(0xF0 | (cp >> 18));
        out[1] = (char)(0x80 | ((cp >> 12) & 0x3F));
        out[2] = (char)(0x80 | ((cp >> 6) & 0x3F));
        out[3] = (char)(0x80 | (cp & 0x3F));
        return 4;
    }
}

char *Tcl_UniCharToUtfDString(
    const Tcl_UniChar *uniStr, int uniLength, Tcl_DString *dsPtr)
{
    /* Each UniChar encodes to at most 4 UTF-8 bytes; grow up front. */
    size_t need = dsPtr->length + (size_t)uniLength * 4 + 1;
    if (need > dsPtr->spaceAvl) {
        size_t new_size = dsPtr->spaceAvl == 0 ? 64 : dsPtr->spaceAvl * 2;
        while (new_size < need) {
            new_size *= 2;
        }
        char *new_buf = (char *)realloc(dsPtr->string, new_size);
        if (new_buf == NULL) {
            return dsPtr->string; /* best-effort */
        }
        dsPtr->string = new_buf;
        dsPtr->spaceAvl = new_size;
    }
    for (int i = 0; i < uniLength; i++) {
        dsPtr->length += encode_utf8(
            (int)uniStr[i], dsPtr->string + dsPtr->length);
    }
    dsPtr->string[dsPtr->length] = '\0';
    return dsPtr->string;
}
