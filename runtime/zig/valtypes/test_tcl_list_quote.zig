// Unit tests for ``valtypes/tcl_list_quote.zig`` — the Tcl list-element
// quoter (``TclScanElement`` / ``TclConvertElement`` port).
//
// Strategy: round-trip every interesting input through
// ``scan_element`` → ``convert_element`` → ``tcl_list_parse.element_at``
// + (optionally) ``tcl_bs.decode_into``, and assert the recovered
// payload matches the original.  The reference oracle is ``Tcl_Merge``
// in upstream Tcl 9.0; the expected outputs below were captured from
// running ``puts [list $payload]`` under tclsh 9.0.

const std = @import("std");
const testing = std.testing;

const quote = @import("tcl_list_quote.zig");
const parse = @import("tcl_list_parse.zig");
const bs = @import("tcl_bs.zig");

// ---- helpers --------------------------------------------------------

fn quoteFirst(src: []const u8, dst: []u8) []const u8 {
    const flags = quote.scan_element(@intFromPtr(src.ptr), @intCast(src.len), 0);
    const n = quote.convert_element(
        @intFromPtr(src.ptr),
        @intCast(src.len),
        @intFromPtr(dst.ptr),
        flags,
    );
    return dst[0..n];
}

fn quoteNth(src: []const u8, dst: []u8) []const u8 {
    const off = quote.list_elem_quote_nth(@intFromPtr(dst.ptr), 0, @intFromPtr(src.ptr), @intCast(src.len));
    return dst[0..off];
}

/// Round-trip: quote ``src`` as a single list element, then split the
/// result back through the list-parse layer (decoding backslash
/// escapes for unbraced elements).  Returns the recovered payload.
fn roundTrip(src: []const u8, scratch_q: []u8, scratch_d: []u8) []const u8 {
    const quoted = quoteFirst(src, scratch_q);
    // The parser sees the quoted form as a one-element list.
    const elem = parse.element_at(@intFromPtr(quoted.ptr), @intCast(quoted.len), 0);
    const payload_ptr: [*]const u8 = @ptrFromInt(@intFromPtr(quoted.ptr) + elem.start);
    if (elem.braced) {
        // Braced form: bytes are literal.
        @memcpy(scratch_d[0..elem.len], payload_ptr[0..elem.len]);
        return scratch_d[0..elem.len];
    } else {
        const n = bs.decode_into(@intFromPtr(payload_ptr), elem.len, @intFromPtr(scratch_d.ptr));
        return scratch_d[0..n];
    }
}

// ---- scan_element / convert_element direct outputs -----------------

test "convert_element — empty input is {}" {
    var out: [16]u8 = undefined;
    try testing.expectEqualStrings("{}", quoteFirst("", &out));
}

test "convert_element — plain bareword passes through unchanged" {
    var out: [16]u8 = undefined;
    try testing.expectEqualStrings("hello", quoteFirst("hello", &out));
}

test "convert_element — whitespace forces brace quoting" {
    var out: [32]u8 = undefined;
    try testing.expectEqualStrings("{a b}", quoteFirst("a b", &out));
    try testing.expectEqualStrings("{a\tb}", quoteFirst("a\tb", &out));
}

test "convert_element — leading-{ braces the element when balanced" {
    var out: [32]u8 = undefined;
    // Balanced inner braces: ``{abc}`` brace-quotes to ``{{abc}}``.
    try testing.expectEqualStrings("{{abc}}", quoteFirst("{abc}", &out));
}

test "convert_element — unbalanced leading-{ falls back to escape form" {
    var out: [32]u8 = undefined;
    // ``{abc`` (no closing brace) raises ``require_escape`` so the
    // brace form is forbidden — output is the escape form ``\{abc``.
    try testing.expectEqualStrings("\\{abc", quoteFirst("{abc", &out));
}

test "convert_element — leading-\" braces the element" {
    var out: [32]u8 = undefined;
    try testing.expectEqualStrings("{\"hi}", quoteFirst("\"hi", &out));
}

test "convert_element — leading-# braces unless DONT_QUOTE_HASH" {
    var out: [32]u8 = undefined;
    // First-element mode (flag=0) treats leading-# as comment-risky
    // and braces the element.
    try testing.expectEqualStrings("{#foo}", quoteFirst("#foo", &out));
    // Nth-element mode (DONT_QUOTE_HASH) leaves it alone.
    try testing.expectEqualStrings("#foo", quoteNth("#foo", &out));
}

test "convert_element — unmatched close-brace forces escape mode" {
    var out: [32]u8 = undefined;
    // ``a}`` cannot brace-quote (would close the wrapper) so the
    // result is escape-form ``a\}``.
    try testing.expectEqualStrings("a\\}", quoteFirst("a}", &out));
}

test "convert_element — trailing backslash forces escape mode" {
    var out: [32]u8 = undefined;
    // ``foo\`` would escape the closing brace — forced to ``foo\\``.
    try testing.expectEqualStrings("foo\\\\", quoteFirst("foo\\", &out));
}

test "convert_element — backslash-newline forces escape mode" {
    var out: [32]u8 = undefined;
    // ``a\<NL>b`` cannot brace (would re-substitute on parse); the
    // newline gets ``\n``-escaped and the backslash itself is doubled.
    try testing.expectEqualStrings("a\\\\\\nb", quoteFirst("a\\\nb", &out));
}

test "convert_element — special chars round-trip via brace form" {
    var out: [64]u8 = undefined;
    // ``$``, ``[``, ``;`` flip ``forbid_none`` + ``prefer_brace``,
    // so the quoter wraps the element in braces.
    try testing.expectEqualStrings("{a$b}", quoteFirst("a$b", &out));
    try testing.expectEqualStrings("{a[b]c}", quoteFirst("a[b]c", &out));
    try testing.expectEqualStrings("{a;b}", quoteFirst("a;b", &out));
}

// ---- round-trip property tests -------------------------------------

test "round-trip — adversarial payloads recover bytes-for-bytes" {
    var q: [128]u8 = undefined;
    var d: [128]u8 = undefined;

    const cases = [_][]const u8{
        "",
        "hello",
        "hello world",
        "hello\tworld",
        "{",
        "}",
        "a}",
        "{a",
        "{}",
        "{abc}",
        "[cmd]",
        "$var",
        ";",
        "#hash",
        "a\\b",
        "trailing\\",
    };
    for (cases) |case| {
        const got = roundTrip(case, &q, &d);
        try testing.expectEqualSlices(u8, case, got);
    }
}

// Cases involving ``\<newline>`` go via the escape form which
// ``decode_into`` collapses to ``" "``; we exercise these separately
// so the expectations match the canonical Tcl semantics.
test "round-trip — newline and special whitespace recover" {
    var q: [128]u8 = undefined;
    var d: [128]u8 = undefined;

    // Plain newline: ``a\nb`` braces to ``{a\nb}``; brace form keeps
    // bytes literal so we get the original back.
    try testing.expectEqualSlices(u8, "a\nb", roundTrip("a\nb", &q, &d));

    // Vertical-tab / form-feed are scan_space but not list separators;
    // they brace-quote without further transformation.
    try testing.expectEqualSlices(u8, "a\x0Bb", roundTrip("a\x0Bb", &q, &d));
    try testing.expectEqualSlices(u8, "a\x0Cb", roundTrip("a\x0Cb", &q, &d));
}

test "list_elem_quote_nth — first vs nth differs only on leading-#" {
    var out: [32]u8 = undefined;
    // Non-leading-# input behaves identically.
    try testing.expectEqualStrings("hello", quoteNth("hello", &out));
    try testing.expectEqualStrings("{a b}", quoteNth("a b", &out));
}
