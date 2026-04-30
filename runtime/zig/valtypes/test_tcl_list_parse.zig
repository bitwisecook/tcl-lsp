// Unit tests for ``valtypes/tcl_list_parse.zig`` — the Tcl list
// scanner (``TclFindElement`` port).
//
// Focuses on adversarial inputs: nested braces, ``\{`` / ``\}``
// escape pairs, empty elements, leading/trailing whitespace, and
// unterminated braces.  Reference oracle is ``Tcl_ListObjLength`` /
// ``Tcl_ListObjIndex`` from upstream Tcl 9.0 (``generic/tclListObj.c``,
// which delegates to ``TclFindElement``).

const std = @import("std");
const testing = std.testing;

const lp = @import("tcl_list_parse.zig");

// ---- helpers --------------------------------------------------------

fn count(src: []const u8) i64 {
    return lp.count_elements(@intFromPtr(src.ptr), @intCast(src.len));
}

fn at(src: []const u8, idx: i64) lp.Element {
    return lp.element_at(@intFromPtr(src.ptr), @intCast(src.len), idx);
}

/// Materialise the nth element as a slice into ``src`` (does NOT
/// decode backslash escapes — caller should use ``copy_unbraced_elem``
/// when the element is unbraced and they want literal bytes).
fn slice(src: []const u8, idx: i64) []const u8 {
    const elem = at(src, idx);
    return src[elem.start..][0..elem.len];
}

fn validBraces(src: []const u8) bool {
    return lp.validate_list_braces(@intFromPtr(src.ptr), @intCast(src.len));
}

// ---- count_elements -------------------------------------------------

test "count_elements — empty / whitespace-only" {
    try testing.expectEqual(@as(i64, 0), count(""));
    try testing.expectEqual(@as(i64, 0), count("   \t\n"));
}

test "count_elements — single bareword and quoted braces" {
    try testing.expectEqual(@as(i64, 1), count("a"));
    try testing.expectEqual(@as(i64, 1), count("{a b c}"));
    try testing.expectEqual(@as(i64, 1), count("{}"));
}

test "count_elements — multiple barewords" {
    try testing.expectEqual(@as(i64, 3), count("a b c"));
    try testing.expectEqual(@as(i64, 3), count("  a   b\tc  "));
    try testing.expectEqual(@as(i64, 5), count("a b c d e"));
}

test "count_elements — nested braces count as one element" {
    try testing.expectEqual(@as(i64, 1), count("{a {b c} d}"));
    try testing.expectEqual(@as(i64, 2), count("{a {b c}} {d e}"));
    try testing.expectEqual(@as(i64, 3), count("{a {b c}} d {e f g}"));
}

test "count_elements — escaped braces don't change nesting depth" {
    // ``\{`` and ``\}`` inside an unbraced word are escape pairs, not
    // brace tokens — they belong to the word but don't open/close.
    try testing.expectEqual(@as(i64, 2), count("a\\{b c"));
    try testing.expectEqual(@as(i64, 2), count("a\\}b c"));
    // Inside a braced word, ``\{`` / ``\}`` shouldn't unbalance the
    // outer braces.
    try testing.expectEqual(@as(i64, 1), count("{a\\{b\\}c}"));
}

test "count_elements — backslash-anything in unbraced word" {
    // ``\ `` inside a word: backslash + space — the backslash escapes
    // the space so the word continues past it.
    try testing.expectEqual(@as(i64, 1), count("a\\ b"));
    try testing.expectEqual(@as(i64, 2), count("a\\ b c"));
}

// ---- element_at -----------------------------------------------------

test "element_at — out-of-range returns empty unbraced" {
    const e = at("a b c", 99);
    try testing.expectEqual(@as(u32, 0), e.start);
    try testing.expectEqual(@as(u32, 0), e.len);
    try testing.expect(!e.braced);
}

test "element_at — bareword spans" {
    try testing.expectEqualStrings("a", slice("a b c", 0));
    try testing.expectEqualStrings("b", slice("a b c", 1));
    try testing.expectEqualStrings("c", slice("a b c", 2));
}

test "element_at — braced flag and inner span" {
    const e0 = at("{hello world} foo", 0);
    try testing.expect(e0.braced);
    try testing.expectEqualStrings("hello world", "{hello world} foo"[e0.start..][0..e0.len]);

    const e1 = at("{hello world} foo", 1);
    try testing.expect(!e1.braced);
    try testing.expectEqualStrings("foo", "{hello world} foo"[e1.start..][0..e1.len]);
}

test "element_at — empty braced element {}" {
    const e = at("a {} c", 1);
    try testing.expect(e.braced);
    try testing.expectEqual(@as(u32, 0), e.len);
}

test "element_at — nested braces preserved verbatim in span" {
    const src = "{a {b c} d} x";
    try testing.expectEqualStrings("a {b c} d", slice(src, 0));
    try testing.expectEqualStrings("x", slice(src, 1));
}

test "element_at — escaped braces in unbraced word" {
    const src = "a\\{b\\}c next";
    try testing.expectEqualStrings("a\\{b\\}c", slice(src, 0));
    try testing.expectEqualStrings("next", slice(src, 1));
}

test "element_at — escaped braces inside braced word" {
    const src = "{outer \\{ inner \\} done} tail";
    try testing.expect(at(src, 0).braced);
    try testing.expectEqualStrings("outer \\{ inner \\} done", slice(src, 0));
    try testing.expectEqualStrings("tail", slice(src, 1));
}

test "element_at — unterminated brace doesn't underflow" {
    // Single ``{`` with no body — ``element_at`` must not panic and
    // must return some span without u32-underflowing the length.
    // The exact span is implementation-defined but the call must
    // complete without trapping.
    _ = at("{", 0);
    _ = at("{abc", 0);
    _ = at("{a {b", 0);
    // count_elements is similarly tolerant.
    _ = count("{");
    _ = count("{abc");
}

// ---- validate_list_braces ------------------------------------------

test "validate_list_braces — well-formed inputs accepted" {
    try testing.expect(validBraces(""));
    try testing.expect(validBraces("a b c"));
    try testing.expect(validBraces("{a}"));
    try testing.expect(validBraces("{a {b c} d}"));
    try testing.expect(validBraces("{a\\{b\\}c}"));
}

test "validate_list_braces — unmatched braces rejected" {
    try testing.expect(!validBraces("{"));
    try testing.expect(!validBraces("{a"));
    try testing.expect(!validBraces("{a {b}"));
}

// ---- copy_unbraced_elem --------------------------------------------

test "copy_unbraced_elem — backslash escapes decoded" {
    var out: [32]u8 = undefined;
    const src = "a\\nb\\tc";
    const n = lp.copy_unbraced_elem(@intFromPtr(out[0..].ptr), @intFromPtr(src.ptr), @intCast(src.len));
    try testing.expectEqualStrings("a\nb\tc", out[0..n]);
}
