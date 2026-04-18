/*
 * tcl_reg_shim.c — ASCII-only implementations of the four
 * character-class predicates ``regcustom.h`` (our override) binds
 * ``iscalnum`` / ``iscalpha`` / ``iscdigit`` / ``iscspace`` to.
 *
 * Upstream Tcl calls ``Tcl_UniCharIsAlnum`` / ``...Alpha`` /
 * ``...Digit`` / ``...Space`` which use large Unicode tables.
 * Dragging those in would add tens of KB of wasm for
 * codepoint-class data we don't currently need — tcltest's
 * init-path patterns and counter.test only use ASCII characters.
 * ASCII-only coverage is correct for ASCII input and wrong (false
 * negatives) for non-ASCII, which is acceptable as a starting
 * point.  Swapping these for a full ``Tcl_UniCharIsXxx`` port is
 * a mechanical follow-up — the engine never sees the difference.
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "tclInt.h" /* Tcl_UniChar, Tcl_DString */

/*
 * wasi-libc's startup object (``__main_void.o``) refers to ``main``
 * so that ``_start`` can invoke it.  We build ``tcl_runtime.wasm``
 * as a WASI reactor — no ``_start``, Zig sets ``entry = .disabled``
 * — but the link graph still pulls in the startup object's
 * ``main`` symbol.  Provide a trivial stub so the link closes.
 * Never invoked: reactors don't run startup; ``__main_void.o`` is
 * dead code in the final binary past DCE.
 */
int main(void)
{
    return 0;
}

int TclWasmRegIsAlnum(int c)
{
    if (c < 0 || c > 0x7F) {
        return 0;
    }
    return (c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z') ||
           (c >= 'a' && c <= 'z');
}

int TclWasmRegIsAlpha(int c)
{
    if (c < 0 || c > 0x7F) {
        return 0;
    }
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
}

int TclWasmRegIsDigit(int c)
{
    if (c < 0 || c > 0x7F) {
        return 0;
    }
    return c >= '0' && c <= '9';
}

int TclWasmRegIsSpace(int c)
{
    /*
     * Matches POSIX ``[[:space:]]`` on the ASCII range:
     * space, tab, newline, vertical tab, form feed, carriage
     * return.  Tcl's ``\s`` escape resolves here.
     */
    return c == ' ' || c == '\t' || c == '\n' || c == '\v' ||
           c == '\f' || c == '\r';
}

/*
 * ------------------------------------------------------------------
 * Unicode case mapping — ASCII-only.
 *
 * The engine uses these to expand ``(?i)``/``-nocase`` ranges (see
 * ``regc_locale.c``'s ``range()``) by adding the lower/upper/title
 * variant of each character in the range.  For ASCII input, title
 * case coincides with upper case, so we re-use the upper mapping.
 * Non-ASCII codepoints pass through unchanged — which is wrong in
 * general but leaves those characters to match only in their
 * original case, a conservative failure mode that doesn't affect
 * tcltest's ASCII-only patterns.
 * ------------------------------------------------------------------
 */

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
    return Tcl_UniCharToUpper(ch);
}

/*
 * ------------------------------------------------------------------
 * Tcl_DString — minimal append-only dynamic string.
 *
 * ``regc_locale.c`` uses this to build the UTF-8 form of a
 * multi-character collating-element name (e.g. ``<nul>``) and
 * compare it against a static table.  Upstream's
 * ``Tcl_DString`` embeds a 200-byte static buffer plus heap
 * growth; we skip the static buffer and always heap-allocate for
 * simplicity.  Non-optimal for tiny strings but correct and
 * compact.
 * ------------------------------------------------------------------
 */

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
 * UTF-8 encoding of a single codepoint.  Writes 1–4 bytes to
 * ``out`` and returns the byte count.  Invalid codepoints
 * (surrogates, >U+10FFFF) are encoded as U+FFFD.
 */
static int encode_utf8(int cp, char *out)
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
    /*
     * Each UniChar encodes to at most 4 UTF-8 bytes.  Grow the
     * heap block up front to avoid repeated reallocs.  The
     * upstream API appends; callers pass an already-initialised
     * (possibly non-empty) DString.
     */
    int need = dsPtr->length + uniLength * 4 + 1;
    if (need > dsPtr->spaceAvl) {
        int new_size = dsPtr->spaceAvl == 0 ? 64 : dsPtr->spaceAvl * 2;
        while (new_size < need) {
            new_size *= 2;
        }
        char *new_buf = (char *)realloc(dsPtr->string, (size_t)new_size);
        if (new_buf == NULL) {
            return dsPtr->string; /* best-effort; caller may crash */
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
