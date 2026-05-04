// Tcl list-element encoder.
//
// Ports the reference Tcl 9.0 ``TclScanElement`` / ``TclConvertElement``
// pair (``generic/tclUtil.c``, COMPAT=1).  Together they quote any raw
// byte span as a single Tcl list element that round-trips through
// ``TclFindElement`` — the ``Tcl_Merge`` / ``UpdateStringOfList``
// pipeline in reference Tcl uses the same pair.
//
// Layering: this module is the output side of the list-string
// contract.  The input side lives in ``tcl_list_parse.zig``.  Both
// share ``tcl_chars.zig`` for whitespace classification.
//
// A single ``u8`` flags byte carries the decision from scan to
// convert, matching the reference C enum ``ConvertFlags``.  The same
// byte is also how callers request ``DONT_USE_BRACES`` /
// ``DONT_QUOTE_HASH`` — the flag bits round-trip verbatim through
// :func:`scan_element` into :func:`convert_element`.

const chars = @import("tcl_chars.zig");
const is_scan_space = chars.is_scan_space;

// List-element conversion flags — mirror ``tcl.h`` and ``tclUtil.c``
// ``enum ConvertFlags``.  Returned by :func:`scan_element` and consumed
// by :func:`convert_element`.
pub const FLAG_CONVERT_NONE: u8 = 0;
pub const FLAG_DONT_USE_BRACES: u8 = 1; // TCL_DONT_USE_BRACES
pub const FLAG_CONVERT_BRACE: u8 = 2;
pub const FLAG_CONVERT_ESCAPE: u8 = 4;
pub const FLAG_DONT_QUOTE_HASH: u8 = 8; // TCL_DONT_QUOTE_HASH
pub const FLAG_CONVERT_MASK: u8 = FLAG_CONVERT_BRACE | FLAG_CONVERT_ESCAPE;

/// Minimal local memcpy — avoids importing the whole ``tcl_obj`` layer
/// for what is a three-line byte copy; keeps this module free of
/// TclObj / allocator dependencies.
fn memcpy(dst: u32, src: u32, len: u32) void {
    if (len == 0 or src == 0 or dst == 0) return;
    const d: [*]u8 = @ptrFromInt(dst);
    const s: [*]const u8 = @ptrFromInt(src);
    for (0..len) |i| d[i] = s[i];
}

/// Port of ``TclScanElement`` (tclUtil.c, Tcl 9.0, COMPAT=1).
///
/// Classifies *src* and returns the conversion mode needed for
/// :func:`convert_element` to emit a valid list element.  The caller
/// may pass :data:`FLAG_DONT_QUOTE_HASH` to suppress the leading-``#``
/// quoting rule (used for all but the first element when a list object
/// re-renders its string representation).
///
/// Returns only the chosen ``CONVERT_*`` flag bits OR'd with any of the
/// ``DONT_*`` bits the caller supplied.  The byte-count tracking used
/// by the C version for buffer sizing is omitted: callers in this
/// runtime allocate worst-case buffers.
pub fn scan_element(src_ptr: u32, len: u32, flag_in: u8) u8 {
    if (len == 0) {
        return (flag_in & FLAG_DONT_QUOTE_HASH) | FLAG_CONVERT_BRACE;
    }
    const src: [*]const u8 = @ptrFromInt(src_ptr);
    var forbid_none: bool = false;
    var require_escape: bool = false;
    // COMPAT preferences:
    var prefer_escape: bool = false;
    var prefer_brace: bool = false;
    var nesting: i32 = 0;

    // Leading-{ or leading-" forces some form of quoting.
    if (src[0] == '{' or src[0] == '"') {
        forbid_none = true;
        prefer_brace = true;
    }
    // Leading-# forces brace-preference unless the caller opts out.
    if (src[0] == '#' and (flag_in & FLAG_DONT_QUOTE_HASH) == 0) {
        prefer_brace = true;
    }

    var i: u32 = 0;
    while (i < len) : (i += 1) {
        const ch = src[i];
        switch (ch) {
            '{' => {
                nesting += 1;
            },
            '}' => {
                nesting -= 1;
                if (nesting < 0) require_escape = true;
            },
            ']', '"' => {
                forbid_none = true;
                prefer_escape = true;
            },
            '[', '$', ';' => {
                forbid_none = true;
                prefer_brace = true;
            },
            '\\' => {
                if (i + 1 >= len) {
                    // Trailing ``\`` — cannot brace-quote, would escape the close.
                    require_escape = true;
                } else if (src[i + 1] == '\n') {
                    // ``\<newline>`` collapses to space via subst; brace form
                    // is forbidden (would be re-parsed as literal).
                    require_escape = true;
                    i += 1;
                } else if (src[i + 1] == '{' or src[i + 1] == '}' or src[i + 1] == '\\') {
                    // ``\{`` / ``\}`` / ``\\`` — consume as a pair, do NOT
                    // credit the inner brace toward nesting.
                    i += 1;
                }
                forbid_none = true;
                prefer_brace = true;
            },
            else => {
                if (is_scan_space(ch)) {
                    forbid_none = true;
                    prefer_brace = true;
                }
            },
        }
    }
    if (nesting > 0) require_escape = true;

    const out_hash = flag_in & FLAG_DONT_QUOTE_HASH;
    if (require_escape) return out_hash | FLAG_CONVERT_ESCAPE;
    if (forbid_none) {
        if (prefer_escape and !prefer_brace) {
            // COMPAT "mask" mode — escape every special char EXCEPT braces.
            return out_hash | FLAG_CONVERT_MASK;
        }
        return out_hash | FLAG_CONVERT_BRACE;
    }
    return out_hash | FLAG_CONVERT_NONE;
}

/// Port of ``TclConvertElement`` (tclUtil.c, Tcl 9.0, COMPAT=1).  Writes
/// the list-element representation of ``src[0..len]`` to ``dst``.  The
/// ``flags`` argument must come from :func:`scan_element` (possibly with
/// ``FLAG_DONT_USE_BRACES`` / ``FLAG_DONT_QUOTE_HASH`` added by the caller).
///
/// Returns the number of bytes written.  ``dst`` must have capacity for
/// the worst case — callers should size for ``2 * len + 2``.
pub fn convert_element(src_ptr: u32, len_in: u32, dst_base: u32, flags_in: u8) u32 {
    const flags = flags_in;
    var conversion = flags & FLAG_CONVERT_MASK;
    // DONT_USE_BRACES + any BRACE bit → downgrade to ESCAPE.
    if ((flags & FLAG_DONT_USE_BRACES) != 0 and (conversion & FLAG_CONVERT_BRACE) != 0) {
        conversion = FLAG_CONVERT_ESCAPE;
    }

    // Empty string is always ``{}``.
    if (len_in == 0) {
        const d: [*]u8 = @ptrFromInt(dst_base);
        d[0] = '{'; d[1] = '}';
        return 2;
    }

    const src: [*]const u8 = @ptrFromInt(src_ptr);
    var p: u32 = 0;
    var s: u32 = 0;
    var len: u32 = len_in;

    // Leading-# handling: either escape ``\#`` or switch to brace mode.
    if (src[0] == '#' and (flags & FLAG_DONT_QUOTE_HASH) == 0) {
        if (conversion == FLAG_CONVERT_ESCAPE) {
            const d: [*]u8 = @ptrFromInt(dst_base + p);
            d[0] = '\\'; d[1] = '#';
            p += 2;
            s += 1;
            len -= 1;
        } else {
            conversion = FLAG_CONVERT_BRACE;
        }
    }

    if (conversion == FLAG_CONVERT_NONE) {
        memcpy(dst_base + p, src_ptr + s, len);
        return p + len;
    }

    if (conversion == FLAG_CONVERT_BRACE) {
        var d: [*]u8 = @ptrFromInt(dst_base + p);
        d[0] = '{';
        p += 1;
        memcpy(dst_base + p, src_ptr + s, len);
        p += len;
        d = @ptrFromInt(dst_base + p);
        d[0] = '}';
        return p + 1;
    }

    // CONVERT_ESCAPE or CONVERT_MASK.
    var k: u32 = 0;
    while (k < len) : (k += 1) {
        const ch = src[s + k];
        switch (ch) {
            ']', '[', '$', ';', ' ', '\\', '"' => {
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\';
                p += 1;
            },
            '{', '}' => {
                // In CONVERT_MASK, braces are NOT escaped.
                if (conversion == FLAG_CONVERT_ESCAPE) {
                    const d: [*]u8 = @ptrFromInt(dst_base + p);
                    d[0] = '\\';
                    p += 1;
                }
            },
            '\n' => {
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\'; d[1] = 'n';
                p += 2;
                continue;
            },
            '\t' => {
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\'; d[1] = 't';
                p += 2;
                continue;
            },
            '\r' => {
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\'; d[1] = 'r';
                p += 2;
                continue;
            },
            0x0B => { // \v
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\'; d[1] = 'v';
                p += 2;
                continue;
            },
            0x0C => { // \f
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\'; d[1] = 'f';
                p += 2;
                continue;
            },
            else => {},
        }
        const d: [*]u8 = @ptrFromInt(dst_base + p);
        d[0] = ch;
        p += 1;
    }
    return p;
}

/// Append *src* to *buf* at *off* as a canonical list element (flag=0 —
/// first-element mode).  Returns the new offset.  Worst-case expansion is
/// ``2 * len + 2`` bytes; callers must size their buffer accordingly.
pub fn list_elem_quote(buf: u32, off: u32, ptr: u32, len: u32) u32 {
    const flags = scan_element(ptr, len, 0);
    return off + convert_element(ptr, len, buf + off, flags);
}

/// Non-first-element variant: ``FLAG_DONT_QUOTE_HASH`` — a leading ``#``
/// is NOT braced / escaped.  Used by list-builders for every element
/// after index 0, matching ``UpdateStringOfList``.
pub fn list_elem_quote_nth(buf: u32, off: u32, ptr: u32, len: u32) u32 {
    const flags = scan_element(ptr, len, FLAG_DONT_QUOTE_HASH);
    return off + convert_element(ptr, len, buf + off, flags);
}
