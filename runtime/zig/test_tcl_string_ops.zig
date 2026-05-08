// Unit tests for ``valtypes/tcl_string.zig`` — the string-manipulation
// primitives (``string_compare/length/index/range/map/match/trim/equal
// /first/last/repeat/reverse/toupper/tolower/totitle/replace`` plus
// ``string_is_*`` predicates and ``tcl_cmd_split/join/concat``).
// Wave 3.
//
// Same test pattern as ``test_tcl_dict.zig`` / ``test_tcl_list_ops.zig``:
// build inputs from ``obj_new_string`` / ``obj_new_int``, call the
// exported function, compare the result's string bytes.  ``cmds/string.zig``
// is a thin dispatcher over these helpers, so this file covers the
// substance of every ``string`` builtin.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const str = @import("valtypes/tcl_string.zig");

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

// ---- compare / equal ------------------------------------------------

test "string_compare — bytewise lexicographic" {
    try testing.expectEqual(@as(i64, 0), intResult(str.string_compare(s("abc"), s("abc"))));
    try testing.expectEqual(@as(i64, -1), intResult(str.string_compare(s("abc"), s("abd"))));
    try testing.expectEqual(@as(i64, 1), intResult(str.string_compare(s("abd"), s("abc"))));
    // Prefix is less than its extension.
    try testing.expectEqual(@as(i64, -1), intResult(str.string_compare(s("abc"), s("abcd"))));
    // Empty edges.
    try testing.expectEqual(@as(i64, 0), intResult(str.string_compare(s(""), s(""))));
    try testing.expectEqual(@as(i64, -1), intResult(str.string_compare(s(""), s("x"))));
}

test "string_equal — 1 if equal, 0 otherwise" {
    try testing.expectEqual(@as(i64, 1), intResult(str.string_equal(s("hello"), s("hello"))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_equal(s("hello"), s("world"))));
    try testing.expectEqual(@as(i64, 1), intResult(str.string_equal(s(""), s(""))));
}

test "tcl_expr_order_cmp — numeric path when both parse as ints" {
    try testing.expectEqual(@as(i64, -1), intResult(str.tcl_expr_order_cmp(s("9"), s("10"))));
    try testing.expectEqual(@as(i64, 1), intResult(str.tcl_expr_order_cmp(s("10"), s("9"))));
    // Pure-string fallback when at least one operand is non-numeric.
    try testing.expectEqual(@as(i64, -1), intResult(str.tcl_expr_order_cmp(s("9"), s("abc"))));
}

// ---- length / index / range ----------------------------------------

test "string_length — byte length" {
    try testing.expectEqual(@as(i64, 0), intResult(str.string_length(s(""))));
    try testing.expectEqual(@as(i64, 5), intResult(str.string_length(s("hello"))));
    try testing.expectEqual(@as(i64, 11), intResult(str.string_length(s("hello world"))));
}

test "string_index — byte at position; supports \"end\"" {
    try testing.expectEqualStrings("h", bytes(str.string_index(s("hello"), i(0))));
    try testing.expectEqualStrings("o", bytes(str.string_index(s("hello"), i(4))));
    try testing.expectEqualStrings("o", bytes(str.string_index(s("hello"), s("end"))));
    try testing.expectEqualStrings("e", bytes(str.string_index(s("hello"), s("end-3"))));
    // Out of range → empty.
    try testing.expectEqualStrings("", bytes(str.string_index(s("hello"), i(99))));
    try testing.expectEqualStrings("", bytes(str.string_index(s("hello"), i(-1))));
}

test "string_range — substring [first..last] inclusive" {
    try testing.expectEqualStrings("ell", bytes(str.string_range(s("hello"), i(1), i(3))));
    try testing.expectEqualStrings("hello", bytes(str.string_range(s("hello"), i(0), s("end"))));
    try testing.expectEqualStrings("o", bytes(str.string_range(s("hello"), s("end"), s("end"))));
    // first > last → empty.
    try testing.expectEqualStrings("", bytes(str.string_range(s("hello"), i(3), i(1))));
}

// ---- string_match (glob) -------------------------------------------

test "string_match — literal / wildcard / question-mark" {
    try testing.expectEqual(@as(i64, 1), intResult(str.string_match(s("hello"), s("hello"))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_match(s("hello"), s("world"))));
    // ``*`` matches any run, including empty.
    try testing.expectEqual(@as(i64, 1), intResult(str.string_match(s("h*o"), s("hello"))));
    try testing.expectEqual(@as(i64, 1), intResult(str.string_match(s("*"), s(""))));
    try testing.expectEqual(@as(i64, 1), intResult(str.string_match(s("**"), s("anything"))));
    // ``?`` matches exactly one byte.
    try testing.expectEqual(@as(i64, 1), intResult(str.string_match(s("h?llo"), s("hello"))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_match(s("h?llo"), s("hllo"))));
}

test "string_match — empty pattern only matches empty value" {
    try testing.expectEqual(@as(i64, 1), intResult(str.string_match(s(""), s(""))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_match(s(""), s("x"))));
}

// ---- string_map -----------------------------------------------------

test "string_map — apply key/value pairs" {
    try testing.expectEqualStrings(
        "henno",
        bytes(str.string_map(s("l n"), s("hello"))),
    );
    // Multiple pairs.
    try testing.expectEqualStrings(
        "HELLO",
        bytes(str.string_map(s("h H e E l L o O"), s("hello"))),
    );
}

test "string_map — non-overlapping replacements only" {
    // ``e`` is replaced with ``X`` once; the resulting ``X`` is NOT
    // re-scanned (matches reference Tcl).
    try testing.expectEqualStrings(
        "hXllo",
        bytes(str.string_map(s("e X"), s("hello"))),
    );
}

test "string_map_nocase — case-insensitive key match" {
    // Mapping ``e -> X`` matches ``E`` too because of -nocase.
    try testing.expectEqualStrings(
        "hXllo",
        bytes(str.string_map_nocase(s("E X"), s("hello"))),
    );
}

// ---- trim variants --------------------------------------------------

test "string_trim — default whitespace" {
    try testing.expectEqualStrings("hello", bytes(str.string_trim(s("  hello  "), 0)));
    try testing.expectEqualStrings("hello", bytes(str.string_trim(s("\t\n hello \r"), 0)));
}

test "string_trim — custom char set" {
    try testing.expectEqualStrings("hello", bytes(str.string_trim(s("xxhelloxx"), s("x"))));
    try testing.expectEqualStrings("hello", bytes(str.string_trim(s("xyhelloxy"), s("xy"))));
}

test "string_trimleft / string_trimright" {
    try testing.expectEqualStrings("hello  ", bytes(str.string_trimleft(s("  hello  "), 0)));
    try testing.expectEqualStrings("  hello", bytes(str.string_trimright(s("  hello  "), 0)));
}

// ---- first / last ---------------------------------------------------

test "string_first — first byte index of needle" {
    try testing.expectEqual(@as(i64, 2), intResult(str.string_first(s("ll"), s("hello"))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_first(s("h"), s("hello"))));
    try testing.expectEqual(@as(i64, -1), intResult(str.string_first(s("xyz"), s("hello"))));
    // Empty needle → -1 (Tcl convention).
    try testing.expectEqual(@as(i64, -1), intResult(str.string_first(s(""), s("hello"))));
}

test "string_last — last byte index of needle" {
    // ``hello world`` — 'o' at indices 4 and 7; last is 7.
    try testing.expectEqual(@as(i64, 7), intResult(str.string_last(s("o"), s("hello world"))));
    try testing.expectEqual(@as(i64, 4), intResult(str.string_last(s("o"), s("hello"))));
    try testing.expectEqual(@as(i64, -1), intResult(str.string_last(s("xyz"), s("hello"))));
}

// ---- repeat / reverse ----------------------------------------------

test "string_repeat — N copies" {
    try testing.expectEqualStrings("abcabcabc", bytes(str.string_repeat(s("abc"), i(3))));
    try testing.expectEqualStrings("", bytes(str.string_repeat(s("abc"), i(0))));
    try testing.expectEqualStrings("", bytes(str.string_repeat(s("abc"), i(-1))));
    try testing.expectEqualStrings("", bytes(str.string_repeat(s(""), i(5))));
}

test "string_reverse" {
    try testing.expectEqualStrings("olleh", bytes(str.string_reverse(s("hello"))));
    try testing.expectEqualStrings("a", bytes(str.string_reverse(s("a"))));
    // The implementation returns the input TclObj for length≤1.
    try testing.expectEqualStrings("", bytes(str.string_reverse(s(""))));
}

// ---- case conversion -----------------------------------------------

test "string_toupper / string_tolower" {
    try testing.expectEqualStrings("HELLO", bytes(str.string_toupper(s("hello"))));
    try testing.expectEqualStrings("hello", bytes(str.string_tolower(s("HELLO"))));
    // Mixed in / non-alphabetic preserved.
    try testing.expectEqualStrings("HELLO 123!", bytes(str.string_toupper(s("Hello 123!"))));
}

test "string_totitle — first alpha upper, rest lower" {
    try testing.expectEqualStrings("Hello", bytes(str.string_totitle(s("hello"))));
    try testing.expectEqualStrings("Hello", bytes(str.string_totitle(s("HELLO"))));
    // Non-alpha leading bytes are passed through; first alpha gets upper.
    try testing.expectEqualStrings("123 Abc", bytes(str.string_totitle(s("123 abc"))));
}

// ---- replace --------------------------------------------------------

test "string_replace — substitute range with new bytes" {
    try testing.expectEqualStrings(
        "heXXo",
        bytes(str.string_replace(s("hello"), i(2), i(3), s("XX"))),
    );
    // Empty replacement → deletion.
    try testing.expectEqualStrings(
        "heo",
        bytes(str.string_replace(s("hello"), i(2), i(3), s(""))),
    );
}

// ---- insert ---------------------------------------------------------

test "string_insert — start, middle, end (integer indices)" {
    try testing.expectEqualStrings("_0123", bytes(str.string_insert(s("0123"), i(0), s("_"))));
    try testing.expectEqualStrings("01_23", bytes(str.string_insert(s("0123"), i(2), s("_"))));
    try testing.expectEqualStrings("0123_", bytes(str.string_insert(s("0123"), i(4), s("_"))));
}

test "string_insert — end / end-N (insertion-point semantics)" {
    // ``end`` resolves to ``length`` (one past last char), unlike
    // ``string index`` where ``end`` is ``length-1``.  Matches
    // ``StringInsertCmd`` in upstream ``tclCmdMZ.c``.
    try testing.expectEqualStrings("0123_", bytes(str.string_insert(s("0123"), s("end"), s("_"))));
    try testing.expectEqualStrings("01_23", bytes(str.string_insert(s("0123"), s("end-2"), s("_"))));
    try testing.expectEqualStrings("_0123", bytes(str.string_insert(s("0123"), s("end-4"), s("_"))));
}

test "string_insert — clamps out-of-range indices" {
    // Negative indices clamp to 0 (prepend).
    try testing.expectEqualStrings("_0123", bytes(str.string_insert(s("0123"), i(-1), s("_"))));
    // Indices past the end clamp to length (append).
    try testing.expectEqualStrings("0123_", bytes(str.string_insert(s("0123"), i(5), s("_"))));
}

test "string_insert — empty operands" {
    try testing.expectEqualStrings("_", bytes(str.string_insert(s(""), i(0), s("_"))));
    try testing.expectEqualStrings("0123", bytes(str.string_insert(s("0123"), i(0), s(""))));
    try testing.expectEqualStrings("", bytes(str.string_insert(s(""), i(0), s(""))));
}

// ---- string_is_* predicates ----------------------------------------

test "string_is_integer" {
    try testing.expectEqual(@as(i64, 1), intResult(str.string_is_integer(s("123"))));
    try testing.expectEqual(@as(i64, 1), intResult(str.string_is_integer(s("-7"))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_is_integer(s("12.5"))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_is_integer(s("abc"))));
    // Empty string is NOT integer (matches the implementation).
    try testing.expectEqual(@as(i64, 0), intResult(str.string_is_integer(s(""))));
    // Bignum-shaped literals (Stage 2): values > i64 still register
    // as integer because the runtime supports arbitrary precision.
    try testing.expectEqual(@as(i64, 1), intResult(str.string_is_integer(s("18446744073709551616"))));
    try testing.expectEqual(@as(i64, 1), intResult(str.string_is_integer(s("99999999999999999999999"))));
    try testing.expectEqual(@as(i64, 1), intResult(str.string_is_integer(s("-99999999999999999999999"))));
}

test "string_is_wideinteger" {
    // ``string is wideinteger`` accepts the same set as
    // ``string is integer`` once bignum support landed — the
    // distinction collapsed when arbitrary precision became the
    // runtime default (matches Tcl 9.0 semantics).
    try testing.expectEqual(@as(i64, 1), intResult(str.string_is_wideinteger(s("99"))));
    try testing.expectEqual(@as(i64, 1), intResult(str.string_is_wideinteger(s("18446744073709551616"))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_is_wideinteger(s("12.5"))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_is_wideinteger(s("abc"))));
}

test "string_is_alpha / digit / space" {
    try testing.expectEqual(@as(i64, 1), intResult(str.string_is_alpha(s("hello"))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_is_alpha(s("hello1"))));

    try testing.expectEqual(@as(i64, 1), intResult(str.string_is_digit(s("12345"))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_is_digit(s("12a"))));

    try testing.expectEqual(@as(i64, 1), intResult(str.string_is_space(s("   \t\n"))));
    try testing.expectEqual(@as(i64, 0), intResult(str.string_is_space(s(" x "))));
}

// ---- split / join / concat -----------------------------------------

test "tcl_cmd_split — single-char separator" {
    try testing.expectEqualStrings("a b c", bytes(str.tcl_cmd_split(s("a,b,c"), s(","))));
    try testing.expectEqualStrings("a b c d", bytes(str.tcl_cmd_split(s("a:b:c:d"), s(":"))));
}

test "tcl_cmd_split — empty separator splits each byte" {
    try testing.expectEqualStrings("a b c", bytes(str.tcl_cmd_split(s("abc"), s(""))));
}

test "tcl_cmd_split — multi-char separator splits on any" {
    try testing.expectEqualStrings("a b c d", bytes(str.tcl_cmd_split(s("a,b;c,d"), s(",;"))));
}

test "tcl_cmd_join — default space separator" {
    // separator == 0 means "use default space".
    try testing.expectEqualStrings("a b c", bytes(str.tcl_cmd_join(s("a b c"), 0)));
    // Custom separator.
    try testing.expectEqualStrings("a-b-c", bytes(str.tcl_cmd_join(s("a b c"), s("-"))));
}

test "tcl_cmd_concat — trims and joins with single space" {
    try testing.expectEqualStrings("a b", bytes(str.tcl_cmd_concat(s("a"), s("b"))));
    try testing.expectEqualStrings("a b", bytes(str.tcl_cmd_concat(s("  a  "), s("  b  "))));
    // Empty after trim → other side returned alone.
    try testing.expectEqualStrings("a", bytes(str.tcl_cmd_concat(s("a"), s("   "))));
    try testing.expectEqualStrings("b", bytes(str.tcl_cmd_concat(s("   "), s("b"))));
    try testing.expectEqualStrings("", bytes(str.tcl_cmd_concat(s(""), s(""))));
}
