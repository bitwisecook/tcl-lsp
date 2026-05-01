// Unit tests for ``valtypes/tcl_format.zig`` — the ``format``
// command's sprintf-flavoured spec parser and per-conversion
// emitters.  Wave 4 of the WASM-runtime test harness (#267).
//
// ``tcl_cmd_format`` takes a fmt TclObj and three optional argument
// TclObj handles (``0`` for missing slots).  The resulting TclObj
// wraps the formatted bytes; tests compare the bytes directly with
// ``expectEqualStrings``.
//
// Error-path coverage uses ``runtime_test_fixture.with_catch`` —
// unknown conversions raise ``unsupported command: format %?`` via
// ``stubs.unsupported_sub`` so the test asserts against that exact
// message.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const fmt = @import("valtypes/tcl_format.zig");
const fixture = @import("runtime_test_fixture.zig");

// ---- helpers --------------------------------------------------------

fn s(v: []const u8) i32 {
    return obj.obj_new_string(@bitCast(@intFromPtr(v.ptr)), @bitCast(v.len));
}

fn i(v: i64) i32 {
    return obj.obj_new_int(v);
}

fn f(v: f64) i32 {
    return obj.obj_new_float(v);
}

fn bytes(o: i32) []const u8 {
    const r = obj.obj_ensure_string(o);
    if (r.len == 0) return &.{};
    const p: [*]const u8 = @ptrFromInt(r.ptr);
    return p[0..r.len];
}

fn fmt0(spec: []const u8) []const u8 {
    return bytes(fmt.tcl_cmd_format(s(spec), 0, 0, 0));
}

fn fmt1(spec: []const u8, a: i32) []const u8 {
    return bytes(fmt.tcl_cmd_format(s(spec), a, 0, 0));
}

fn fmt2(spec: []const u8, a: i32, b: i32) []const u8 {
    return bytes(fmt.tcl_cmd_format(s(spec), a, b, 0));
}

fn fmt3(spec: []const u8, a: i32, b: i32, c: i32) []const u8 {
    return bytes(fmt.tcl_cmd_format(s(spec), a, b, c));
}

// ---- plain conversions ---------------------------------------------

test "tcl_cmd_format — %% emits a literal percent" {
    try testing.expectEqualStrings("%", fmt0("%%"));
    try testing.expectEqualStrings("100%", fmt1("%d%%", i(100)));
}

test "tcl_cmd_format — %s string conversion" {
    try testing.expectEqualStrings("hello", fmt1("%s", s("hello")));
    try testing.expectEqualStrings("(hi)", fmt1("(%s)", s("hi")));
    try testing.expectEqualStrings("", fmt1("%s", s("")));
    // Missing arg → empty string (slot is 0).
    try testing.expectEqualStrings("", fmt0("%s"));
}

test "tcl_cmd_format — %d decimal integer" {
    try testing.expectEqualStrings("42", fmt1("%d", i(42)));
    try testing.expectEqualStrings("-7", fmt1("%d", i(-7)));
    try testing.expectEqualStrings("0", fmt1("%d", i(0)));
    // ``%i`` is the standard alias for ``%d``.
    try testing.expectEqualStrings("42", fmt1("%i", i(42)));
}

test "tcl_cmd_format — %x / %X hex (lower / upper)" {
    try testing.expectEqualStrings("ff", fmt1("%x", i(255)));
    try testing.expectEqualStrings("FF", fmt1("%X", i(255)));
    try testing.expectEqualStrings("0", fmt1("%x", i(0)));
    try testing.expectEqualStrings("deadbeef", fmt1("%x", i(0xdeadbeef)));
}

test "tcl_cmd_format — %o octal" {
    try testing.expectEqualStrings("10", fmt1("%o", i(8)));
    try testing.expectEqualStrings("777", fmt1("%o", i(0o777)));
    try testing.expectEqualStrings("0", fmt1("%o", i(0)));
}

test "tcl_cmd_format — %c char from code point (ASCII)" {
    try testing.expectEqualStrings("A", fmt1("%c", i(65)));
    try testing.expectEqualStrings("z", fmt1("%c", i(122)));
    // Out-of-range (negative or >127) drops silently.
    try testing.expectEqualStrings("", fmt1("%c", i(200)));
    try testing.expectEqualStrings("", fmt1("%c", i(-1)));
}

// ---- width + precision ---------------------------------------------

test "tcl_cmd_format — %Nd right-align with spaces" {
    try testing.expectEqualStrings("   42", fmt1("%5d", i(42)));
    try testing.expectEqualStrings("   -7", fmt1("%5d", i(-7)));
    // Width less than digit count → no padding, no truncation.
    try testing.expectEqualStrings("12345", fmt1("%2d", i(12345)));
}

test "tcl_cmd_format — %-Nd left-align with spaces" {
    try testing.expectEqualStrings("42   ", fmt1("%-5d", i(42)));
    try testing.expectEqualStrings("-7   ", fmt1("%-5d", i(-7)));
}

test "tcl_cmd_format — %0Nd zero-pad on the left" {
    try testing.expectEqualStrings("00042", fmt1("%05d", i(42)));
    // Sign sits before the zero pad.
    try testing.expectEqualStrings("-0042", fmt1("%05d", i(-42)));
}

test "tcl_cmd_format — %+d show sign for positive" {
    try testing.expectEqualStrings("+42", fmt1("%+d", i(42)));
    try testing.expectEqualStrings("-42", fmt1("%+d", i(-42)));
}

test "tcl_cmd_format — %.Ns string precision truncates" {
    try testing.expectEqualStrings("hel", fmt1("%.3s", s("hello")));
    // Precision >= length → no truncation.
    try testing.expectEqualStrings("hello", fmt1("%.10s", s("hello")));
    // Precision 0 → empty.
    try testing.expectEqualStrings("", fmt1("%.0s", s("hello")));
}

test "tcl_cmd_format — %WIDTH.PRECs string width + precision" {
    // Truncate to 3 chars then right-align in width 6 → "   hel".
    try testing.expectEqualStrings("   hel", fmt1("%6.3s", s("hello")));
    // Left-align variant.
    try testing.expectEqualStrings("hel   ", fmt1("%-6.3s", s("hello")));
}

test "tcl_cmd_format — multiple conversions in one fmt" {
    try testing.expectEqualStrings(
        "(3,4)",
        fmt2("(%d,%d)", i(3), i(4)),
    );
    try testing.expectEqualStrings(
        "x=10 name=tcl",
        fmt2("x=%d name=%s", i(10), s("tcl")),
    );
}

// ---- positional args -----------------------------------------------

test "tcl_cmd_format — %N$s explicit positional ordering" {
    // Reverse the args via positional indexing.
    try testing.expectEqualStrings(
        "second first",
        fmt2("%2$s %1$s", s("first"), s("second")),
    );
}

test "tcl_cmd_format — %N$d positional with int" {
    try testing.expectEqualStrings(
        "20 10",
        fmt2("%2$d %1$d", i(10), i(20)),
    );
}

test "tcl_cmd_format — out-of-range positional → empty slot" {
    // ``%4$s`` indexes past the 3-arg dispatch; the implementation
    // falls through to the 0 / empty-string sentinel rather than
    // raising.
    try testing.expectEqualStrings(
        "[]",
        fmt1("[%4$s]", s("only")),
    );
}

// ---- float formats -------------------------------------------------

test "tcl_cmd_format — %f decimal float at default precision (6)" {
    // Default precision is 6 fractional digits.
    try testing.expectEqualStrings("3.140000", fmt1("%f", f(3.14)));
    try testing.expectEqualStrings("0.000000", fmt1("%f", f(0.0)));
    try testing.expectEqualStrings("-1.500000", fmt1("%f", f(-1.5)));
}

test "tcl_cmd_format — %.Nf precision controls fractional digits" {
    try testing.expectEqualStrings("3.14", fmt1("%.2f", f(3.14)));
    try testing.expectEqualStrings("3.1", fmt1("%.1f", f(3.14)));
    // Precision 0 trims the decimal point.
    try testing.expectEqualStrings("3", fmt1("%.0f", f(3.0)));
}

test "tcl_cmd_format — %g shortest decimal representation" {
    // ``%g`` defers to std.fmt's ``{d}`` formatter; exact digit
    // count depends on Zig's float printer but the result must
    // contain the value.
    const out = fmt1("%g", f(1.5));
    try testing.expect(std.mem.indexOf(u8, out, "1.5") != null);
}

// ---- empty / edge inputs -------------------------------------------

test "tcl_cmd_format — empty fmt yields empty result" {
    try testing.expectEqualStrings("", fmt0(""));
}

test "tcl_cmd_format — fmt with no conversions passes through verbatim" {
    try testing.expectEqualStrings("hello world", fmt0("hello world"));
}

test "tcl_cmd_format — trailing %% with no conversion is dropped" {
    // ``%`` at the very end falls out of the parse loop without
    // emitting anything (no spec, no conversion byte).
    try testing.expectEqualStrings("abc", fmt0("abc%"));
}

// ---- error paths (need #266 fixture) -------------------------------

const UnknownVerbCtx = struct {
    var captured: ?i32 = null;
    fn body() void {
        captured = fmt.tcl_cmd_format(s("%z"), i(1), 0, 0);
    }
};

test "tcl_cmd_format — unknown conversion raises via stubs.unsupported_sub" {
    UnknownVerbCtx.captured = null;
    const msg = fixture.with_catch(&UnknownVerbCtx.body);
    try testing.expect(msg != null);
    try testing.expectEqualStrings("unsupported command: format %z", msg.?);
}

const UnknownVerbCapsCtx = struct {
    fn body() void {
        _ = fmt.tcl_cmd_format(s("%Q"), i(1), 0, 0);
    }
};

test "tcl_cmd_format — unknown uppercase conversion raises with verb" {
    const msg = fixture.with_catch(&UnknownVerbCapsCtx.body);
    try testing.expect(msg != null);
    try testing.expectEqualStrings("unsupported command: format %Q", msg.?);
}
