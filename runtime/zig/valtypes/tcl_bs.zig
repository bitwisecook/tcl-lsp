// Tcl backslash-escape decoder.
//
// Reference Tcl (9.0, `generic/tclParse.c`) exposes ``TclParseBackslash``
// as the canonical single-escape decoder and ``Tcl_UtfBackslash`` /
// ``TclCopyAndCollapse`` as higher-level loops that iterate it.  This
// module mirrors that split: :func:`consume_bs_escape` decodes ONE
// ``\x`` sequence; :func:`decode_into` loops across a whole span —
// consolidating the two hand-rolled copies of that loop that used to
// live in `tcl_obj.copy_unbraced_elem` (list-parse path) and
// `tcl_interp.subst_flagged` (script-word path).
//
// Both ``\xNN``, ``\uNNNN``, ``\UNNNNNNNN``, octal ``\NNN``, and
// ``\<whitespace>`` folding match reference Tcl byte-for-byte.
// Unknown escapes ``\<byte>`` emit the byte verbatim, matching Tcl.

/// UTF-8-encode a codepoint into *d*, returning the byte count.  Used
/// by the backslash-escape decoder for ``\uNNNN`` / ``\UNNNNNNNN``.
pub fn encode_utf8(d: [*]u8, cp: u32) u32 {
    if (cp < 0x80) {
        d[0] = @intCast(cp);
        return 1;
    } else if (cp < 0x800) {
        d[0] = @intCast(0xC0 | (cp >> 6));
        d[1] = @intCast(0x80 | (cp & 0x3F));
        return 2;
    } else if (cp < 0x10000) {
        d[0] = @intCast(0xE0 | (cp >> 12));
        d[1] = @intCast(0x80 | ((cp >> 6) & 0x3F));
        d[2] = @intCast(0x80 | (cp & 0x3F));
        return 3;
    } else {
        d[0] = @intCast(0xF0 | (cp >> 18));
        d[1] = @intCast(0x80 | ((cp >> 12) & 0x3F));
        d[2] = @intCast(0x80 | ((cp >> 6) & 0x3F));
        d[3] = @intCast(0x80 | (cp & 0x3F));
        return 4;
    }
}

/// Decode one Tcl backslash escape starting at ``src[si]``.  ``si`` is
/// the INDEX OF THE FIRST BYTE AFTER THE BACKSLASH — the caller has
/// already consumed the ``\`` and verified the sequence is non-empty.
///
/// Writes up to 4 UTF-8 bytes to ``out`` and returns ``(next_si, written)``
/// so the caller can advance its input cursor and output offset.
///
/// Handles the full Tcl table (same as ``subst`` in a double-quoted /
/// bare context): ``\n \t \r \a \b \f \v``, ``\xNN``, ``\uNNNN``,
/// ``\UNNNNNNNN``, octal ``\NNN``, and ``\<whitespace>`` folding (with
/// ``\<newline>`` additionally eating following spaces / tabs).
pub fn consume_bs_escape(
    src: [*]const u8,
    si_in: u32,
    len: u32,
    out: [*]u8,
) struct { next_si: u32, written: u32 } {
    var si = si_in;
    const ch = src[si];
    switch (ch) {
        'n' => { out[0] = '\n'; return .{ .next_si = si + 1, .written = 1 }; },
        't' => { out[0] = '\t'; return .{ .next_si = si + 1, .written = 1 }; },
        'r' => { out[0] = '\r'; return .{ .next_si = si + 1, .written = 1 }; },
        'a' => { out[0] = 0x07; return .{ .next_si = si + 1, .written = 1 }; },
        'b' => { out[0] = 0x08; return .{ .next_si = si + 1, .written = 1 }; },
        'f' => { out[0] = 0x0C; return .{ .next_si = si + 1, .written = 1 }; },
        'v' => { out[0] = 0x0B; return .{ .next_si = si + 1, .written = 1 }; },
        'x' => {
            // ``\xNN`` — 1 or 2 hex digits → Unicode codepoint U+00NN
            // emitted as UTF-8 (Tcl 9 semantics).  When no hex digit
            // follows, the backslash is dropped and ``x`` is emitted
            // literally (matching the unknown-escape rule, subst-3.1).
            si += 1;
            var val: u32 = 0;
            var ndig: u32 = 0;
            while (ndig < 2 and si < len) {
                const c = src[si];
                if (c >= '0' and c <= '9') { val = val * 16 + @as(u32, c - '0'); si += 1; ndig += 1; }
                else if (c >= 'a' and c <= 'f') { val = val * 16 + @as(u32, c - 'a' + 10); si += 1; ndig += 1; }
                else if (c >= 'A' and c <= 'F') { val = val * 16 + @as(u32, c - 'A' + 10); si += 1; ndig += 1; }
                else break;
            }
            if (ndig == 0) {
                // No hex digits — emit the literal ``x`` byte.
                out[0] = 'x';
                return .{ .next_si = si, .written = 1 };
            }
            return .{ .next_si = si, .written = encode_utf8(out, val) };
        },
        'u' => {
            // ``\uNNNN`` — up to 4 hex digits → UTF-8.
            si += 1;
            var cp: u32 = 0;
            var ndig: u32 = 0;
            while (ndig < 4 and si < len) {
                const c = src[si];
                if (c >= '0' and c <= '9') { cp = cp * 16 + @as(u32, c - '0'); si += 1; ndig += 1; }
                else if (c >= 'a' and c <= 'f') { cp = cp * 16 + @as(u32, c - 'a' + 10); si += 1; ndig += 1; }
                else if (c >= 'A' and c <= 'F') { cp = cp * 16 + @as(u32, c - 'A' + 10); si += 1; ndig += 1; }
                else break;
            }
            return .{ .next_si = si, .written = encode_utf8(out, cp) };
        },
        'U' => {
            // ``\UNNNNNNNN`` — up to 8 hex digits → UTF-8.
            si += 1;
            var cp: u32 = 0;
            var ndig: u32 = 0;
            while (ndig < 8 and si < len) {
                const c = src[si];
                if (c >= '0' and c <= '9') { cp = cp * 16 + @as(u32, c - '0'); si += 1; ndig += 1; }
                else if (c >= 'a' and c <= 'f') { cp = cp * 16 + @as(u32, c - 'a' + 10); si += 1; ndig += 1; }
                else if (c >= 'A' and c <= 'F') { cp = cp * 16 + @as(u32, c - 'A' + 10); si += 1; ndig += 1; }
                else break;
            }
            return .{ .next_si = si, .written = encode_utf8(out, cp) };
        },
        '0'...'7' => {
            // ``\NNN`` — up to 3 octal digits → Unicode codepoint
            // emitted as UTF-8 (Tcl 9 semantics; subst-3.2).
            // ``\8`` / ``\9`` are NOT octal: real Tcl treats them as
            // unknown escapes and emits the literal byte (``subst
            // "\\8"`` → ``"8"``), so they fall through to the ``else``
            // arm below rather than being accepted by this case.
            var val: u32 = 0;
            var ndig: u32 = 0;
            while (ndig < 3 and si < len and src[si] >= '0' and src[si] <= '7') {
                val = val * 8 + @as(u32, src[si] - '0');
                si += 1; ndig += 1;
            }
            return .{ .next_si = si, .written = encode_utf8(out, val) };
        },
        ' ', '\n', '\t', '\r' => {
            // ``\<whitespace>`` → single space; ``\<newline>``
            // additionally eats following spaces / tabs.
            out[0] = ' ';
            si += 1;
            if (ch == '\n') {
                while (si < len and (src[si] == ' ' or src[si] == '\t')) si += 1;
            }
            return .{ .next_si = si, .written = 1 };
        },
        else => {
            // Unknown escape — emit the byte verbatim (Tcl behaviour).
            out[0] = ch;
            return .{ .next_si = si + 1, .written = 1 };
        },
    }
}

/// Decode an entire byte span through the backslash rules, writing the
/// substituted output to ``dst``.  Returns the number of bytes written.
/// The caller owns ``dst`` and must have reserved at least ``src_len``
/// bytes (decode never expands — every ``\<byte>`` pair produces at
/// most one UTF-8 codepoint, at most 4 output bytes, and consumes at
/// least two source bytes).
///
/// Used by :func:`tcl_list_parse.copy_unbraced_elem` to materialise a
/// list element's value with escapes resolved, and by ``subst_flagged``
/// in the interpreter to substitute a script word's backslash escapes.
pub fn decode_into(src_ptr: u32, src_len: u32, dst: u32) u32 {
    const src: [*]const u8 = @ptrFromInt(src_ptr);
    const out: [*]u8 = @ptrFromInt(dst);
    var si: u32 = 0;
    var di: u32 = 0;
    while (si < src_len) {
        if (src[si] == '\\' and si + 1 < src_len) {
            const r = consume_bs_escape(src, si + 1, src_len, @ptrFromInt(dst + di));
            di += r.written;
            si = r.next_si;
        } else {
            out[di] = src[si];
            di += 1;
            si += 1;
        }
    }
    return di;
}
