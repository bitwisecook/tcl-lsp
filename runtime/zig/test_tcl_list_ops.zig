// Unit tests for ``valtypes/tcl_list.zig`` — the list-manipulation
// primitives (``tcl_cmd_list_index/range/sort/search/contains/reverse
// /insert/repeat``, ``tcl_list``, ``list_tail``).  Wave 3.
//
// These are the same functions ``cmds/list.zig`` dispatches to, so
// covering them here exercises the substance of every list builtin
// (``llength`` / ``lindex`` / ``lrange`` / ``lsort`` / ``lsearch``
// / ``lreverse`` / ``linsert`` / ``lrepeat`` / ``list`` / ``concat``)
// without needing an interpreter to set up call frames.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const list = @import("valtypes/tcl_list.zig");

// ---- helpers --------------------------------------------------------

fn s(v: []const u8) i32 {
    return obj.obj_new_string(@bitCast(@intFromPtr(v.ptr)), @bitCast(v.len));
}

fn i(v: i64) i32 {
    return obj.obj_new_int(v);
}

fn bytes(o: i32) []const u8 {
    const r = obj.obj_ensure_string(o);
    if (r.len == 0) return &.{};
    const p: [*]const u8 = @ptrFromInt(r.ptr);
    return p[0..r.len];
}

fn intResult(o: i32) i64 {
    return obj.obj_get_int(o);
}

// ---- length ---------------------------------------------------------

test "tcl_cmd_list_length on empty / single / multi" {
    try testing.expectEqual(@as(i64, 0), intResult(list.tcl_cmd_list_length(s(""))));
    try testing.expectEqual(@as(i64, 1), intResult(list.tcl_cmd_list_length(s("solo"))));
    try testing.expectEqual(@as(i64, 3), intResult(list.tcl_cmd_list_length(s("a b c"))));
    // Nested braces count as one element.
    try testing.expectEqual(@as(i64, 2), intResult(list.tcl_cmd_list_length(s("{a b c} d"))));
}

// ---- index ---------------------------------------------------------

test "tcl_cmd_list_index — 0-based, unbraced and braced elements" {
    const l = s("alpha beta gamma");
    try testing.expectEqualStrings("alpha", bytes(list.tcl_cmd_list_index(l, i(0))));
    try testing.expectEqualStrings("beta", bytes(list.tcl_cmd_list_index(l, i(1))));
    try testing.expectEqualStrings("gamma", bytes(list.tcl_cmd_list_index(l, i(2))));
}

test "tcl_cmd_list_index — out-of-range is empty" {
    const l = s("a b");
    try testing.expectEqual(@as(usize, 0), bytes(list.tcl_cmd_list_index(l, i(99))).len);
    try testing.expectEqual(@as(usize, 0), bytes(list.tcl_cmd_list_index(l, i(-1))).len);
}

test "tcl_cmd_list_index — \"end\" / \"end-N\"" {
    const l = s("a b c d e");
    try testing.expectEqualStrings("e", bytes(list.tcl_cmd_list_index(l, s("end"))));
    try testing.expectEqualStrings("d", bytes(list.tcl_cmd_list_index(l, s("end-1"))));
    try testing.expectEqualStrings("a", bytes(list.tcl_cmd_list_index(l, s("end-4"))));
}

test "tcl_cmd_list_index — braced element is unwrapped" {
    const l = s("{a b} c");
    try testing.expectEqualStrings("a b", bytes(list.tcl_cmd_list_index(l, i(0))));
    try testing.expectEqualStrings("c", bytes(list.tcl_cmd_list_index(l, i(1))));
}

// ---- range ----------------------------------------------------------

test "tcl_cmd_list_range — full / partial / empty windows" {
    const l = s("a b c d e");
    try testing.expectEqualStrings("a b c d e", bytes(list.tcl_cmd_list_range(l, i(0), s("end"))));
    try testing.expectEqualStrings("b c d", bytes(list.tcl_cmd_list_range(l, i(1), i(3))));
    try testing.expectEqualStrings("e", bytes(list.tcl_cmd_list_range(l, s("end"), s("end"))));
    // first > last → empty.
    try testing.expectEqual(@as(usize, 0), bytes(list.tcl_cmd_list_range(l, i(3), i(1))).len);
}

test "tcl_cmd_list_range — clamps over-range last to len-1" {
    const l = s("a b c");
    try testing.expectEqualStrings("a b c", bytes(list.tcl_cmd_list_range(l, i(0), i(99))));
}

// ---- sort -----------------------------------------------------------

test "tcl_cmd_list_sort — ASCII string ordering" {
    try testing.expectEqualStrings("a b c d", bytes(list.tcl_cmd_list_sort(s("d c a b"))));
    try testing.expectEqualStrings("alpha beta gamma", bytes(list.tcl_cmd_list_sort(s("gamma alpha beta"))));
}

test "tcl_cmd_list_sort — empty / single / already-sorted" {
    try testing.expectEqualStrings("", bytes(list.tcl_cmd_list_sort(s(""))));
    try testing.expectEqualStrings("solo", bytes(list.tcl_cmd_list_sort(s("solo"))));
    try testing.expectEqualStrings("a b c", bytes(list.tcl_cmd_list_sort(s("a b c"))));
}

test "tcl_cmd_list_sort — duplicates preserved (stable enough for equal keys)" {
    try testing.expectEqualStrings("a a b b c", bytes(list.tcl_cmd_list_sort(s("c a b a b"))));
}

test "tcl_cmd_list_sort — braced elements with internal whitespace preserve grouping" {
    // tcltest stores compound constraint names like
    // ``"testexprparser && !ieeeFloatingPoint"`` as array keys; the
    // ``[lsort [array names ...]]`` over those keys must keep each
    // element a single list element.  Without re-bracing the sorted
    // output, ``foreach`` over the result iterates 7 fragments instead
    // of 3 and ``[info exists arr($constraint)]`` then traps.
    try testing.expectEqualStrings(
        "{a b c} {d e f}",
        bytes(list.tcl_cmd_list_sort(s("{d e f} {a b c}"))),
    );
    try testing.expectEqualStrings(
        "testexprparser {testexprparser && !ieeeFloatingPoint} {testexprparser && ieeeFloatingPoint}",
        bytes(list.tcl_cmd_list_sort(s(
            "{testexprparser && !ieeeFloatingPoint} testexprparser {testexprparser && ieeeFloatingPoint}",
        ))),
    );
}

// ---- search / contains ---------------------------------------------

test "tcl_cmd_list_search — exact match returns index" {
    const l = s("alpha beta gamma");
    try testing.expectEqual(@as(i64, 0), intResult(list.tcl_cmd_list_search(l, s("alpha"))));
    try testing.expectEqual(@as(i64, 2), intResult(list.tcl_cmd_list_search(l, s("gamma"))));
    try testing.expectEqual(@as(i64, -1), intResult(list.tcl_cmd_list_search(l, s("missing"))));
}

test "tcl_cmd_list_search — glob pattern matches first occurrence" {
    const l = s("apple banana avocado");
    // ``a*`` matches anything starting with 'a' — first hit is "apple".
    try testing.expectEqual(@as(i64, 0), intResult(list.tcl_cmd_list_search(l, s("a*"))));
    // ``*na`` matches "banana" only (avocado ends in 'do').
    try testing.expectEqual(@as(i64, 1), intResult(list.tcl_cmd_list_search(l, s("*na"))));
}

test "tcl_cmd_list_contains — exact match boolean" {
    const l = s("a b c");
    try testing.expectEqual(@as(i64, 1), intResult(list.tcl_cmd_list_contains(l, s("a"))));
    try testing.expectEqual(@as(i64, 1), intResult(list.tcl_cmd_list_contains(l, s("c"))));
    try testing.expectEqual(@as(i64, 0), intResult(list.tcl_cmd_list_contains(l, s("missing"))));
    // Glob meta-chars in ``contains`` are LITERAL (NOT a glob).
    try testing.expectEqual(@as(i64, 0), intResult(list.tcl_cmd_list_contains(l, s("a*"))));
}

// ---- reverse --------------------------------------------------------

test "tcl_cmd_list_reverse" {
    try testing.expectEqualStrings("c b a", bytes(list.tcl_cmd_list_reverse(s("a b c"))));
    try testing.expectEqualStrings("solo", bytes(list.tcl_cmd_list_reverse(s("solo"))));
    // Empty — same i32 returned (≤1 element shortcut).
    try testing.expectEqualStrings("", bytes(list.tcl_cmd_list_reverse(s(""))));
}

test "tcl_cmd_list_reverse — keeps brace grouping for nested lists" {
    // Without ``append_list_element``'s brace-restore, ``{a b}``
    // would flatten into separate elements ``a b``.  Pin it.
    try testing.expectEqualStrings("c {a b}", bytes(list.tcl_cmd_list_reverse(s("{a b} c"))));
}

// ---- insert ---------------------------------------------------------

test "tcl_cmd_list_insert — front / middle / end" {
    try testing.expectEqualStrings("X a b c", bytes(list.tcl_cmd_list_insert(s("a b c"), i(0), s("X"))));
    try testing.expectEqualStrings("a X b c", bytes(list.tcl_cmd_list_insert(s("a b c"), i(1), s("X"))));
    try testing.expectEqualStrings("a b c X", bytes(list.tcl_cmd_list_insert(s("a b c"), i(3), s("X"))));
}

test "tcl_cmd_list_insert — clamps negative / over-range index" {
    // Negative → front, beyond-end → end.
    try testing.expectEqualStrings("X a", bytes(list.tcl_cmd_list_insert(s("a"), i(-5), s("X"))));
    try testing.expectEqualStrings("a X", bytes(list.tcl_cmd_list_insert(s("a"), i(99), s("X"))));
}

test "tcl_cmd_list_insert — into empty list" {
    try testing.expectEqualStrings("X", bytes(list.tcl_cmd_list_insert(s(""), i(0), s("X"))));
}

// ---- repeat ---------------------------------------------------------

test "tcl_cmd_list_repeat" {
    try testing.expectEqualStrings("foo foo foo", bytes(list.tcl_cmd_list_repeat(i(3), s("foo"))));
    try testing.expectEqualStrings("foo", bytes(list.tcl_cmd_list_repeat(i(1), s("foo"))));
    // Zero / negative count → empty.
    try testing.expectEqualStrings("", bytes(list.tcl_cmd_list_repeat(i(0), s("foo"))));
    try testing.expectEqualStrings("", bytes(list.tcl_cmd_list_repeat(i(-1), s("foo"))));
}

test "tcl_cmd_list_repeat — value with whitespace gets brace-quoted" {
    try testing.expectEqualStrings("{a b} {a b}", bytes(list.tcl_cmd_list_repeat(i(2), s("a b"))));
}

// ---- list (build a 2-element list) ---------------------------------

test "tcl_list — joins two elements canonically" {
    try testing.expectEqualStrings("a b", bytes(list.tcl_list(s("a"), s("b"))));
    // Whitespace in the second value triggers brace-quoting.
    try testing.expectEqualStrings("a {b c}", bytes(list.tcl_list(s("a"), s("b c"))));
}

test "tcl_list — empty first arg gives a single-element list" {
    try testing.expectEqualStrings("solo", bytes(list.tcl_list(s(""), s("solo"))));
}

// ---- list_tail ------------------------------------------------------

test "list_tail — drops the first N elements" {
    const l = s("a b c d e");
    try testing.expectEqualStrings("c d e", bytes(list.list_tail(l, i(2))));
    try testing.expectEqualStrings("a b c d e", bytes(list.list_tail(l, i(0))));
    // start >= total → empty
    try testing.expectEqualStrings("", bytes(list.list_tail(l, i(99))));
    try testing.expectEqualStrings("", bytes(list.list_tail(l, i(-1))));
}
