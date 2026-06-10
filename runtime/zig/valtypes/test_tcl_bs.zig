// Unit tests for ``valtypes/tcl_bs.zig`` — the Tcl backslash-escape
// decoder.  Walks every escape variant from the Tcl 9 spec
// (``\xNN``, ``\uNNNN``, ``\UNNNNNNNN``, octal ``\NNN``, the named
// single-byte escapes, and ``\<whitespace>`` folding incl.
// ``\<newline>`` continuation), plus the verbatim fall-through for
// unknown escapes.

const std = @import("std");
const testing = std.testing;

const bs = @import("tcl_bs.zig");

// ---- helpers --------------------------------------------------------

/// Run ``decode_into`` over ``src`` and copy the result into ``out``.
/// Returns the number of bytes written.  ``out`` must be at least
/// ``src.len`` bytes — the decoder never expands.
fn decode(src: []const u8, out: []u8) u32 {
    return bs.decode_into(@intFromPtr(src.ptr), @intCast(src.len), @intFromPtr(out.ptr));
}

/// Convenience: invoke ``consume_bs_escape`` directly with the byte
/// AFTER the leading backslash (the cursor convention the function
/// itself uses).  The wrapper unpacks the anonymous return struct
/// into a named one so test bodies can use a stable identifier.
const EscapeRes = struct { next_si: u32, written: u32 };
fn one(src: []const u8, out: []u8) EscapeRes {
    const r = bs.consume_bs_escape(src.ptr, 0, @intCast(src.len), out.ptr);
    return .{ .next_si = r.next_si, .written = r.written };
}

// ---- encode_utf8 ----------------------------------------------------

test "encode_utf8 — 1/2/3/4-byte forms" {
    var buf: [4]u8 = undefined;

    // ASCII (1 byte)
    try testing.expectEqual(@as(u32, 1), bs.encode_utf8(&buf, 'A'));
    try testing.expectEqual(@as(u8, 'A'), buf[0]);

    // U+00E9 — é (2 bytes: 0xC3 0xA9)
    try testing.expectEqual(@as(u32, 2), bs.encode_utf8(&buf, 0x00E9));
    try testing.expectEqual(@as(u8, 0xC3), buf[0]);
    try testing.expectEqual(@as(u8, 0xA9), buf[1]);

    // U+20AC — € (3 bytes: 0xE2 0x82 0xAC)
    try testing.expectEqual(@as(u32, 3), bs.encode_utf8(&buf, 0x20AC));
    try testing.expectEqual(@as(u8, 0xE2), buf[0]);
    try testing.expectEqual(@as(u8, 0x82), buf[1]);
    try testing.expectEqual(@as(u8, 0xAC), buf[2]);

    // U+1F600 — 😀 (4 bytes: 0xF0 0x9F 0x98 0x80)
    try testing.expectEqual(@as(u32, 4), bs.encode_utf8(&buf, 0x1F600));
    try testing.expectEqual(@as(u8, 0xF0), buf[0]);
    try testing.expectEqual(@as(u8, 0x9F), buf[1]);
    try testing.expectEqual(@as(u8, 0x98), buf[2]);
    try testing.expectEqual(@as(u8, 0x80), buf[3]);
}

// ---- named single-byte escapes -------------------------------------

test "consume_bs_escape — named single-byte escapes" {
    var out: [4]u8 = undefined;

    const cases = [_]struct { in: []const u8, b: u8 }{
        .{ .in = "n", .b = '\n' },
        .{ .in = "t", .b = '\t' },
        .{ .in = "r", .b = '\r' },
        .{ .in = "a", .b = 0x07 },
        .{ .in = "b", .b = 0x08 },
        .{ .in = "f", .b = 0x0C },
        .{ .in = "v", .b = 0x0B },
    };
    for (cases) |c| {
        const r = one(c.in, &out);
        try testing.expectEqual(@as(u32, 1), r.next_si);
        try testing.expectEqual(@as(u32, 1), r.written);
        try testing.expectEqual(c.b, out[0]);
    }
}

// ---- \xNN -----------------------------------------------------------

test "consume_bs_escape — \\xNN with 1 and 2 hex digits" {
    var out: [4]u8 = undefined;

    // \x4A — full two-digit form
    var r = one("x4A", &out);
    try testing.expectEqual(@as(u32, 3), r.next_si);
    try testing.expectEqual(@as(u32, 1), r.written);
    try testing.expectEqual(@as(u8, 0x4A), out[0]);

    // \x9 followed by a non-hex byte — only one digit is consumed.
    r = one("x9z", &out);
    try testing.expectEqual(@as(u32, 2), r.next_si);
    try testing.expectEqual(@as(u32, 1), r.written);
    try testing.expectEqual(@as(u8, 0x09), out[0]);

    // \xFFExtra — only the first two hex digits are taken; Tcl 9
    // emits the codepoint U+00FF as 2 UTF-8 bytes (0xC3 0xBF).
    r = one("xFFExtra", &out);
    try testing.expectEqual(@as(u32, 3), r.next_si);
    try testing.expectEqual(@as(u32, 2), r.written);
    try testing.expectEqual(@as(u8, 0xC3), out[0]);
    try testing.expectEqual(@as(u8, 0xBF), out[1]);

    // \x with no following hex digit at all → emits the literal
    // ``x`` byte (Tcl 9: unknown-escape rule, subst-3.1) and only
    // consumes the ``x``.
    r = one("xz", &out);
    try testing.expectEqual(@as(u32, 1), r.next_si);
    try testing.expectEqual(@as(u32, 1), r.written);
    try testing.expectEqual(@as(u8, 'x'), out[0]);
}

// ---- \uNNNN ---------------------------------------------------------

test "consume_bs_escape — \\uNNNN encodes as UTF-8" {
    var out: [4]u8 = undefined;

    // A → 'A' (1 byte)
    var r = one("u0041", &out);
    try testing.expectEqual(@as(u32, 5), r.next_si);
    try testing.expectEqual(@as(u32, 1), r.written);
    try testing.expectEqual(@as(u8, 'A'), out[0]);

    // é → é (2 bytes 0xC3 0xA9)
    r = one("u00E9", &out);
    try testing.expectEqual(@as(u32, 5), r.next_si);
    try testing.expectEqual(@as(u32, 2), r.written);
    try testing.expectEqual(@as(u8, 0xC3), out[0]);
    try testing.expectEqual(@as(u8, 0xA9), out[1]);

    // € → € (3 bytes)
    r = one("u20AC", &out);
    try testing.expectEqual(@as(u32, 5), r.next_si);
    try testing.expectEqual(@as(u32, 3), r.written);
    try testing.expectEqual(@as(u8, 0xE2), out[0]);

    // \uA — short form (1 hex digit) followed by non-hex.
    r = one("uAz", &out);
    try testing.expectEqual(@as(u32, 2), r.next_si);
    try testing.expectEqual(@as(u32, 1), r.written);
    try testing.expectEqual(@as(u8, 0x0A), out[0]);
}

// ---- \UNNNNNNNN -----------------------------------------------------

test "consume_bs_escape — \\UNNNNNNNN encodes as UTF-8 (incl. astral)" {
    var out: [4]u8 = undefined;

    // \U0001F600 → 😀 (4 bytes 0xF0 0x9F 0x98 0x80)
    var r = one("U0001F600", &out);
    try testing.expectEqual(@as(u32, 9), r.next_si);
    try testing.expectEqual(@as(u32, 4), r.written);
    try testing.expectEqual(@as(u8, 0xF0), out[0]);
    try testing.expectEqual(@as(u8, 0x9F), out[1]);
    try testing.expectEqual(@as(u8, 0x98), out[2]);
    try testing.expectEqual(@as(u8, 0x80), out[3]);

    // \U41 (short form, only 2 digits) followed by non-hex.
    r = one("U41z", &out);
    try testing.expectEqual(@as(u32, 3), r.next_si);
    try testing.expectEqual(@as(u32, 1), r.written);
    try testing.expectEqual(@as(u8, 'A'), out[0]);
}

// ---- octal \NNN -----------------------------------------------------

test "consume_bs_escape — octal \\NNN" {
    var out: [4]u8 = undefined;

    // \101 → 'A' (octal 0o101 == 0x41)
    var r = one("101", &out);
    try testing.expectEqual(@as(u32, 3), r.next_si);
    try testing.expectEqual(@as(u32, 1), r.written);
    try testing.expectEqual(@as(u8, 'A'), out[0]);

    // \77 — only two digits available before non-octal.
    r = one("778", &out);
    try testing.expectEqual(@as(u32, 2), r.next_si);
    try testing.expectEqual(@as(u8, 0o77), out[0]);

    // \7 — single digit form.
    r = one("7", &out);
    try testing.expectEqual(@as(u32, 1), r.next_si);
    try testing.expectEqual(@as(u8, 7), out[0]);

    // \377 — max byte value as codepoint U+00FF; Tcl 9 emits 2 UTF-8
    // bytes (0xC3 0xBF).
    r = one("377", &out);
    try testing.expectEqual(@as(u32, 3), r.next_si);
    try testing.expectEqual(@as(u32, 2), r.written);
    try testing.expectEqual(@as(u8, 0xC3), out[0]);
    try testing.expectEqual(@as(u8, 0xBF), out[1]);

    // ``8`` and ``9`` are NOT octal digits — Tcl treats ``\8`` and
    // ``\9`` as unknown escapes and emits the literal byte (cf.
    // ``subst "\\8"`` → ``"8"`` under tclsh 9.0).  The decoder must
    // therefore advance past the digit and emit it verbatim.
    r = one("8", &out);
    try testing.expectEqual(@as(u32, 1), r.next_si);
    try testing.expectEqual(@as(u32, 1), r.written);
    try testing.expectEqual(@as(u8, '8'), out[0]);

    r = one("9", &out);
    try testing.expectEqual(@as(u32, 1), r.next_si);
    try testing.expectEqual(@as(u32, 1), r.written);
    try testing.expectEqual(@as(u8, '9'), out[0]);
}

// ---- whitespace folding --------------------------------------------

test "consume_bs_escape — \\<whitespace> folds to a single space" {
    var out: [4]u8 = undefined;

    inline for (.{ ' ', '\t', '\r' }) |ws| {
        const src = [_]u8{ws};
        const r = one(&src, &out);
        try testing.expectEqual(@as(u32, 1), r.next_si);
        try testing.expectEqual(@as(u32, 1), r.written);
        try testing.expectEqual(@as(u8, ' '), out[0]);
    }
}

test "consume_bs_escape — \\<newline> additionally eats following spaces/tabs" {
    var out: [4]u8 = undefined;

    // Backslash-newline plus three spaces and a tab — all eaten.
    const r = one("\n   \t" ++ "x", &out);
    try testing.expectEqual(@as(u32, 5), r.next_si);
    try testing.expectEqual(@as(u32, 1), r.written);
    try testing.expectEqual(@as(u8, ' '), out[0]);

    // Backslash-newline at end of input — just consume the newline.
    const r2 = one("\n", &out);
    try testing.expectEqual(@as(u32, 1), r2.next_si);
    try testing.expectEqual(@as(u8, ' '), out[0]);
}

// ---- unknown escape -------------------------------------------------

test "consume_bs_escape — unknown escape emits the byte verbatim" {
    var out: [4]u8 = undefined;

    var r = one("z", &out);
    try testing.expectEqual(@as(u32, 1), r.next_si);
    try testing.expectEqual(@as(u32, 1), r.written);
    try testing.expectEqual(@as(u8, 'z'), out[0]);

    // Backslash itself
    r = one("\\", &out);
    try testing.expectEqual(@as(u32, 1), r.next_si);
    try testing.expectEqual(@as(u8, '\\'), out[0]);

    // Brace
    r = one("{", &out);
    try testing.expectEqual(@as(u32, 1), r.next_si);
    try testing.expectEqual(@as(u8, '{'), out[0]);
}

// ---- decode_into — whole-span loop ---------------------------------

test "decode_into — pass-through of plain text" {
    var out: [32]u8 = undefined;
    const n = decode("abc 123", &out);
    try testing.expectEqual(@as(u32, 7), n);
    try testing.expectEqualStrings("abc 123", out[0..n]);
}

test "decode_into — mixed escapes" {
    var out: [64]u8 = undefined;
    const src = "a\\nb\\tc\\x4Ad\\u00E9";
    // a + \n + b + \t + c + 'J' + d + 'é' (two bytes)
    const n = decode(src, &out);
    try testing.expectEqualSlices(u8, "a\nb\tcJd\xC3\xA9", out[0..n]);
}

test "decode_into — line continuation collapses to space and eats indent" {
    var out: [64]u8 = undefined;
    const src = "foo\\\n   bar";
    const n = decode(src, &out);
    try testing.expectEqualStrings("foo bar", out[0..n]);
}

test "decode_into — trailing lone backslash is preserved" {
    var out: [16]u8 = undefined;
    // ``si + 1 < src_len`` guard means a final ``\`` with nothing
    // after it is emitted as a literal backslash byte.
    const src = "abc\\";
    const n = decode(src, &out);
    try testing.expectEqualStrings("abc\\", out[0..n]);
}

test "decode_into — empty input produces empty output" {
    var out: [4]u8 = undefined;
    const n = decode("", &out);
    try testing.expectEqual(@as(u32, 0), n);
}

test "decode_into — \\\\ collapses to one backslash" {
    var out: [16]u8 = undefined;
    const n = decode("a\\\\b", &out);
    try testing.expectEqualStrings("a\\b", out[0..n]);
}
