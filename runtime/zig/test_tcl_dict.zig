// Unit tests for ``valtypes/tcl_dict.zig`` — exercises the
// ``dict_create / get / set / unset / exists / keys / values /
// size`` API surface that ``cmds/dict.zig`` is a thin dispatcher
// over.  Wave 3.
//
// Tests stay above the ``Tcl_DictObj`` ABI: every input is built
// from ``dict_create`` + ``dict_set`` so we never depend on the
// internal list-rep encoding.  Round-trip assertions
// (``dict_get k == v``) are deliberately preferred over snapshot
// comparisons of the dict's serialised string, because the
// canonical form (brace vs. backslash) is part of the
// implementation freedom we want to leave the dict module.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const dict = @import("valtypes/tcl_dict.zig");

// ---- helpers --------------------------------------------------------

fn s(v: []const u8) i32 {
    return obj.obj_new_string(@bitCast(@intFromPtr(v.ptr)), @bitCast(v.len));
}

fn bytes(o: i32) []const u8 {
    const r = obj.obj_ensure_string(o);
    if (r.len == 0) return &.{};
    const p: [*]const u8 = @ptrFromInt(r.ptr);
    return p[0..r.len];
}

fn dictSize(d: i32) i64 {
    return obj.obj_get_int(dict.dict_size(d));
}

fn dictExists(d: i32, k: []const u8) bool {
    return obj.obj_get_int(dict.dict_exists(d, s(k))) != 0;
}

// ---- create / size --------------------------------------------------

test "dict_create produces an empty dict" {
    const d = dict.dict_create();
    try testing.expectEqual(@as(usize, 0), bytes(d).len);
    try testing.expectEqual(@as(i64, 0), dictSize(d));
}

test "dict_size is len/2 for raw string-rep input" {
    // dict semantically is "k1 v1 k2 v2"; size walks list_count
    // and divides by 2.
    const d = s("a 1 b 2 c 3");
    try testing.expectEqual(@as(i64, 3), dictSize(d));
}

// ---- set / get round-trip ------------------------------------------

test "dict_set on empty dict, dict_get recovers value" {
    const empty = dict.dict_create();
    const d = dict.dict_set(empty, s("name"), s("alice"));
    try testing.expectEqualStrings("alice", bytes(dict.dict_get(d, s("name"))));
}

test "dict_set new key appends; old keys preserved" {
    var d = dict.dict_create();
    d = dict.dict_set(d, s("name"), s("alice"));
    d = dict.dict_set(d, s("age"), s("42"));
    d = dict.dict_set(d, s("city"), s("NYC"));

    try testing.expectEqualStrings("alice", bytes(dict.dict_get(d, s("name"))));
    try testing.expectEqualStrings("42", bytes(dict.dict_get(d, s("age"))));
    try testing.expectEqualStrings("NYC", bytes(dict.dict_get(d, s("city"))));
    try testing.expectEqual(@as(i64, 3), dictSize(d));
}

test "dict_set on existing key replaces value (size unchanged)" {
    var d = dict.dict_create();
    d = dict.dict_set(d, s("k"), s("first"));
    d = dict.dict_set(d, s("k"), s("second"));

    try testing.expectEqualStrings("second", bytes(dict.dict_get(d, s("k"))));
    try testing.expectEqual(@as(i64, 1), dictSize(d));
}

test "dict_get on missing key returns empty TclObj" {
    var d = dict.dict_create();
    d = dict.dict_set(d, s("present"), s("yes"));
    try testing.expectEqual(@as(usize, 0), bytes(dict.dict_get(d, s("absent"))).len);
}

// ---- exists ---------------------------------------------------------

test "dict_exists reports present / absent keys" {
    var d = dict.dict_create();
    d = dict.dict_set(d, s("alpha"), s("1"));
    d = dict.dict_set(d, s("beta"), s("2"));

    try testing.expect(dictExists(d, "alpha"));
    try testing.expect(dictExists(d, "beta"));
    try testing.expect(!dictExists(d, "gamma"));
    try testing.expect(!dictExists(d, ""));
}

test "dict_exists on empty dict is always false" {
    const d = dict.dict_create();
    try testing.expect(!dictExists(d, "anything"));
}

// ---- unset ----------------------------------------------------------

test "dict_unset removes the named key" {
    var d = dict.dict_create();
    d = dict.dict_set(d, s("a"), s("1"));
    d = dict.dict_set(d, s("b"), s("2"));
    d = dict.dict_set(d, s("c"), s("3"));

    d = dict.dict_unset(d, s("b"));
    try testing.expectEqual(@as(i64, 2), dictSize(d));
    try testing.expect(dictExists(d, "a"));
    try testing.expect(!dictExists(d, "b"));
    try testing.expect(dictExists(d, "c"));
    // Surviving values intact.
    try testing.expectEqualStrings("1", bytes(dict.dict_get(d, s("a"))));
    try testing.expectEqualStrings("3", bytes(dict.dict_get(d, s("c"))));
}

test "dict_unset of missing key is a no-op" {
    var d = dict.dict_create();
    d = dict.dict_set(d, s("only"), s("value"));

    const before_size = dictSize(d);
    d = dict.dict_unset(d, s("missing"));
    try testing.expectEqual(before_size, dictSize(d));
    try testing.expect(dictExists(d, "only"));
}

// ---- keys / values --------------------------------------------------

test "dict_keys returns a list of all keys (insertion order)" {
    var d = dict.dict_create();
    d = dict.dict_set(d, s("alpha"), s("1"));
    d = dict.dict_set(d, s("beta"), s("2"));
    d = dict.dict_set(d, s("gamma"), s("3"));

    try testing.expectEqualStrings("alpha beta gamma", bytes(dict.dict_keys(d)));
}

test "dict_values returns a list of all values (insertion order)" {
    var d = dict.dict_create();
    d = dict.dict_set(d, s("alpha"), s("one"));
    d = dict.dict_set(d, s("beta"), s("two"));

    try testing.expectEqualStrings("one two", bytes(dict.dict_values(d)));
}

test "dict_keys / dict_values on empty dict are empty" {
    const d = dict.dict_create();
    try testing.expectEqual(@as(usize, 0), bytes(dict.dict_keys(d)).len);
    try testing.expectEqual(@as(usize, 0), bytes(dict.dict_values(d)).len);
}

// ---- multi-word values ---------------------------------------------

test "dict_set + dict_get with whitespace-bearing value" {
    // Whitespace forces brace-quoting in the underlying list rep.
    // ``dict_get`` must strip those braces and return the literal.
    var d = dict.dict_create();
    d = dict.dict_set(d, s("greeting"), s("hello world"));
    try testing.expectEqualStrings("hello world", bytes(dict.dict_get(d, s("greeting"))));
}

test "dict_set replace path keeps order of unrelated keys" {
    var d = dict.dict_create();
    d = dict.dict_set(d, s("a"), s("1"));
    d = dict.dict_set(d, s("b"), s("2"));
    d = dict.dict_set(d, s("c"), s("3"));
    // Replace middle key.
    d = dict.dict_set(d, s("b"), s("two"));

    try testing.expectEqualStrings("a b c", bytes(dict.dict_keys(d)));
    try testing.expectEqualStrings("two", bytes(dict.dict_get(d, s("b"))));
}
