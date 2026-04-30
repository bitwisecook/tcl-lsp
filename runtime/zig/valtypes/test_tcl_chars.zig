// Unit tests for ``valtypes/tcl_chars.zig``.
//
// First wave of the WASM-runtime test harness — see
// docs ``runtime/zig/build.zig``'s ``test`` step.  These tests
// pin down the byte-classifier and span-comparator predicates so
// changes to the parsing modules can't silently shift what counts
// as whitespace / a bareword character / a hex digit.

const std = @import("std");
const testing = std.testing;

const chars = @import("tcl_chars.zig");

test "is_space matches the list-parser whitespace set" {
    try testing.expect(chars.is_space(' '));
    try testing.expect(chars.is_space('\t'));
    try testing.expect(chars.is_space('\n'));
    try testing.expect(chars.is_space('\r'));

    // Vertical tab / form feed are NOT list separators (see the
    // module doc-comment) — pinning this prevents an accidental
    // tokenisation regression.
    try testing.expect(!chars.is_space(0x0B));
    try testing.expect(!chars.is_space(0x0C));
    try testing.expect(!chars.is_space(0));
    try testing.expect(!chars.is_space('a'));
    try testing.expect(!chars.is_space('0'));
}

test "is_scan_space includes \\v and \\f beyond is_space" {
    try testing.expect(chars.is_scan_space(' '));
    try testing.expect(chars.is_scan_space('\t'));
    try testing.expect(chars.is_scan_space('\n'));
    try testing.expect(chars.is_scan_space('\r'));
    try testing.expect(chars.is_scan_space(0x0B));
    try testing.expect(chars.is_scan_space(0x0C));

    try testing.expect(!chars.is_scan_space(0));
    try testing.expect(!chars.is_scan_space('a'));
    try testing.expect(!chars.is_scan_space(0x7F));
}

test "is_digit covers 0-9 only" {
    var i: u8 = 0;
    while (true) : (i += 1) {
        const expected = (i >= '0' and i <= '9');
        try testing.expectEqual(expected, chars.is_digit(i));
        if (i == 0xFF) break;
    }
}

test "is_hex_digit covers 0-9, a-f, A-F" {
    var i: u8 = 0;
    while (true) : (i += 1) {
        const expected = (i >= '0' and i <= '9') or
            (i >= 'a' and i <= 'f') or
            (i >= 'A' and i <= 'F');
        try testing.expectEqual(expected, chars.is_hex_digit(i));
        if (i == 0xFF) break;
    }

    // Spot-check edge cases that have historically tripped up
    // hand-rolled hex predicates: 'g' / 'G' (just past 'f'/'F') and
    // ':' / '/' (the bytes immediately after / before '0'..'9').
    try testing.expect(!chars.is_hex_digit('g'));
    try testing.expect(!chars.is_hex_digit('G'));
    try testing.expect(!chars.is_hex_digit('/'));
    try testing.expect(!chars.is_hex_digit(':'));
}

test "is_octal_digit covers 0-7" {
    try testing.expect(chars.is_octal_digit('0'));
    try testing.expect(chars.is_octal_digit('7'));
    try testing.expect(!chars.is_octal_digit('8'));
    try testing.expect(!chars.is_octal_digit('9'));
    try testing.expect(!chars.is_octal_digit('/'));
    try testing.expect(!chars.is_octal_digit('a'));
}

test "is_bareword accepts alnum + underscore + colon" {
    // Alpha
    try testing.expect(chars.is_bareword('a'));
    try testing.expect(chars.is_bareword('z'));
    try testing.expect(chars.is_bareword('A'));
    try testing.expect(chars.is_bareword('Z'));
    // Digit
    try testing.expect(chars.is_bareword('0'));
    try testing.expect(chars.is_bareword('9'));
    // Specials
    try testing.expect(chars.is_bareword('_'));
    try testing.expect(chars.is_bareword(':'));

    // Common non-bareword characters from real Tcl source.
    try testing.expect(!chars.is_bareword('-'));
    try testing.expect(!chars.is_bareword('.'));
    try testing.expect(!chars.is_bareword('$'));
    try testing.expect(!chars.is_bareword('{'));
    try testing.expect(!chars.is_bareword('['));
    try testing.expect(!chars.is_bareword(' '));
    try testing.expect(!chars.is_bareword('\n'));
    try testing.expect(!chars.is_bareword(0));
}

test "str_eq compares against comptime keyword" {
    const s = "lappend";
    try testing.expect(chars.str_eq(s.ptr, s.len, "lappend"));
    try testing.expect(!chars.str_eq(s.ptr, s.len, "lappenx"));
    // Length mismatch short-circuits even when prefix matches.
    try testing.expect(!chars.str_eq(s.ptr, s.len, "lappend!"));
    try testing.expect(!chars.str_eq(s.ptr, s.len, "lappen"));

    const empty: []const u8 = "";
    try testing.expect(chars.str_eq(empty.ptr, empty.len, ""));
}

test "mem_eq compares two runtime spans" {
    const a = "abc";
    const b = "abc";
    const c = "abd";
    try testing.expect(chars.mem_eq(a.ptr, a.len, b.ptr, b.len));
    try testing.expect(!chars.mem_eq(a.ptr, a.len, c.ptr, c.len));
    try testing.expect(!chars.mem_eq(a.ptr, a.len, a.ptr, a.len - 1));
}

test "slice_eq handles empty and unequal lengths" {
    try testing.expect(chars.slice_eq("", ""));
    try testing.expect(chars.slice_eq("foo", "foo"));
    try testing.expect(!chars.slice_eq("foo", "bar"));
    try testing.expect(!chars.slice_eq("foo", "foobar"));
    try testing.expect(!chars.slice_eq("foobar", "foo"));
}

test "str_cmp returns memcmp-style ordering" {
    // ``str_cmp`` takes ``u32`` raw addresses; cast via @intFromPtr.
    const a = "abc";
    const b = "abd";
    const c = "abc";
    const d = "abcd";
    const e = "";

    try testing.expectEqual(@as(i32, 0), chars.str_cmp(@intFromPtr(a.ptr), a.len, @intFromPtr(c.ptr), c.len));
    try testing.expectEqual(@as(i32, -1), chars.str_cmp(@intFromPtr(a.ptr), a.len, @intFromPtr(b.ptr), b.len));
    try testing.expectEqual(@as(i32, 1), chars.str_cmp(@intFromPtr(b.ptr), b.len, @intFromPtr(a.ptr), a.len));
    // Prefix-shorter sorts before its extension.
    try testing.expectEqual(@as(i32, -1), chars.str_cmp(@intFromPtr(a.ptr), a.len, @intFromPtr(d.ptr), d.len));
    try testing.expectEqual(@as(i32, 1), chars.str_cmp(@intFromPtr(d.ptr), d.len, @intFromPtr(a.ptr), a.len));
    // Empty vs empty is equal; empty vs anything is less.
    try testing.expectEqual(@as(i32, 0), chars.str_cmp(@intFromPtr(e.ptr), e.len, @intFromPtr(e.ptr), e.len));
    try testing.expectEqual(@as(i32, -1), chars.str_cmp(@intFromPtr(e.ptr), e.len, @intFromPtr(a.ptr), a.len));
    try testing.expectEqual(@as(i32, 1), chars.str_cmp(@intFromPtr(a.ptr), a.len, @intFromPtr(e.ptr), e.len));
}
