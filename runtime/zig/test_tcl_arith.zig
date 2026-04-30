// Unit tests for ``valtypes/tcl_arith.zig`` — float-aware arithmetic
// for ``expr`` evaluation.  These functions take TclObj IDs and
// return a new TclObj; the type-promotion rules (int + int = int,
// any float → float) need to match ``Tcl_ExprObj`` behaviour from
// upstream Tcl 9.0 (``generic/tclExecute.c``).
//
// Wave 2 of the WASM-runtime test harness.  Divide-by-zero is
// covered by the integration suite (``stubs.raise`` traps when
// called outside a ``catch`` context) so we deliberately don't
// exercise it here — those tests need the full interpreter to set
// up an enclosing ``catch`` frame.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const arith = @import("valtypes/tcl_arith.zig");

// ---- helpers --------------------------------------------------------

fn intObj(v: i64) i32 {
    return obj.obj_new_int(v);
}

fn floatObj(v: f64) i32 {
    return obj.obj_new_float(v);
}

fn stringObj(s: []const u8) i32 {
    return obj.obj_new_string(@intCast(@intFromPtr(s.ptr)), @intCast(s.len));
}

fn expectInt(o: i32, expected: i64) !void {
    try testing.expectEqual(expected, obj.obj_get_int(o));
}

fn expectFloatClose(o: i32, expected: f64, eps: f64) !void {
    const got = obj.obj_get_float(o);
    if (@abs(got - expected) > eps) {
        std.debug.print("expected {d} got {d}\n", .{ expected, got });
        return error.TestExpectedClose;
    }
}

// ---- add ------------------------------------------------------------

test "tcl_arith_add — int + int = int" {
    try expectInt(arith.tcl_arith_add(intObj(2), intObj(3)), 5);
    try expectInt(arith.tcl_arith_add(intObj(-7), intObj(7)), 0);
    try expectInt(arith.tcl_arith_add(intObj(0), intObj(0)), 0);
}

test "tcl_arith_add — int + float = float" {
    try expectFloatClose(arith.tcl_arith_add(intObj(2), floatObj(0.5)), 2.5, 1e-12);
    try expectFloatClose(arith.tcl_arith_add(floatObj(1.25), intObj(3)), 4.25, 1e-12);
}

test "tcl_arith_add — float + float = float" {
    try expectFloatClose(arith.tcl_arith_add(floatObj(0.1), floatObj(0.2)), 0.3, 1e-12);
}

test "tcl_arith_add — int overflow wraps (Tcl int is 64-bit two's-complement)" {
    const max = std.math.maxInt(i64);
    // Overflow is silent + two's-complement (the ``+%`` operator);
    // matches reference Tcl behaviour for 64-bit int expressions.
    try expectInt(arith.tcl_arith_add(intObj(max), intObj(1)), std.math.minInt(i64));
}

// ---- sub ------------------------------------------------------------

test "tcl_arith_sub — int - int = int" {
    try expectInt(arith.tcl_arith_sub(intObj(10), intObj(3)), 7);
    try expectInt(arith.tcl_arith_sub(intObj(0), intObj(5)), -5);
}

test "tcl_arith_sub — float - int = float" {
    try expectFloatClose(arith.tcl_arith_sub(floatObj(2.5), intObj(1)), 1.5, 1e-12);
}

test "tcl_arith_sub — wraps on underflow" {
    const min = std.math.minInt(i64);
    try expectInt(arith.tcl_arith_sub(intObj(min), intObj(1)), std.math.maxInt(i64));
}

// ---- mul ------------------------------------------------------------

test "tcl_arith_mul — int * int = int" {
    try expectInt(arith.tcl_arith_mul(intObj(3), intObj(4)), 12);
    try expectInt(arith.tcl_arith_mul(intObj(-3), intObj(4)), -12);
    try expectInt(arith.tcl_arith_mul(intObj(0), intObj(123456)), 0);
}

test "tcl_arith_mul — float promotion" {
    try expectFloatClose(arith.tcl_arith_mul(intObj(3), floatObj(2.5)), 7.5, 1e-12);
}

// ---- div ------------------------------------------------------------

test "tcl_arith_div — int / int = trunc-int" {
    try expectInt(arith.tcl_arith_div(intObj(7), intObj(2)), 3);
    try expectInt(arith.tcl_arith_div(intObj(-7), intObj(2)), -3); // truncation, NOT floor
    try expectInt(arith.tcl_arith_div(intObj(7), intObj(-2)), -3);
}

test "tcl_arith_div — float operand promotes the result" {
    try expectFloatClose(arith.tcl_arith_div(floatObj(7.0), intObj(2)), 3.5, 1e-12);
    try expectFloatClose(arith.tcl_arith_div(intObj(7), floatObj(2.0)), 3.5, 1e-12);
}

// ---- mod ------------------------------------------------------------

test "tcl_arith_mod — int % int" {
    try expectInt(arith.tcl_arith_mod(intObj(10), intObj(3)), 1);
    try expectInt(arith.tcl_arith_mod(intObj(0), intObj(5)), 0);
}

test "tcl_arith_mod — float promotes via @rem semantics" {
    try expectFloatClose(arith.tcl_arith_mod(floatObj(5.5), floatObj(2.0)), 1.5, 1e-12);
}

// ---- string operands triggering float promotion --------------------

test "is_float — string with '.' triggers float coercion" {
    // ``"1.5"`` plus ``"2"`` should produce ``3.5`` as a float.
    try expectFloatClose(arith.tcl_arith_add(stringObj("1.5"), stringObj("2")), 3.5, 1e-12);
}

test "is_float — string with 'e' notation triggers float coercion" {
    try expectFloatClose(arith.tcl_arith_add(stringObj("1e2"), stringObj("0")), 100.0, 1e-9);
}

test "is_float — empty string is NOT float" {
    // Empty strings parse as int 0; result stays in int domain.
    const result = arith.tcl_arith_add(stringObj(""), intObj(5));
    try expectInt(result, 5);
}

// ---- math functions -------------------------------------------------

test "tcl_math_double — int → float" {
    try expectFloatClose(arith.tcl_math_double(intObj(7)), 7.0, 0);
}

test "tcl_math_double — float passes through unchanged" {
    const f = floatObj(3.14);
    try testing.expectEqual(f, arith.tcl_math_double(f));
}

test "tcl_math_int — float → trunc int" {
    // ``obj_get_int`` truncates the float operand.
    try expectInt(arith.tcl_math_int(floatObj(7.9)), 7);
    try expectInt(arith.tcl_math_int(floatObj(-7.9)), -7);
}

test "tcl_math_round — banker's-style nearest int" {
    try expectInt(arith.tcl_math_round(floatObj(2.4)), 2);
    try expectInt(arith.tcl_math_round(floatObj(2.6)), 3);
    try expectInt(arith.tcl_math_round(floatObj(-2.6)), -3);
}

test "tcl_math_sqrt + tcl_math_log + tcl_math_exp" {
    try expectFloatClose(arith.tcl_math_sqrt(floatObj(16.0)), 4.0, 1e-12);
    try expectFloatClose(arith.tcl_math_log(floatObj(1.0)), 0.0, 1e-12);
    try expectFloatClose(arith.tcl_math_exp(floatObj(0.0)), 1.0, 1e-12);
    try expectFloatClose(arith.tcl_math_log10(floatObj(1000.0)), 3.0, 1e-12);
}

test "tcl_math_sqrt + tcl_math_log clamp negative inputs to 0" {
    // Module's defensive return of 0.0 instead of NaN/-Inf.  Pinned
    // because changing it would silently shift behaviour for any
    // ``expr`` user that depends on the current contract.
    try expectFloatClose(arith.tcl_math_sqrt(floatObj(-1.0)), 0.0, 0);
    try expectFloatClose(arith.tcl_math_log(floatObj(0.0)), 0.0, 0);
    try expectFloatClose(arith.tcl_math_log(floatObj(-1.0)), 0.0, 0);
    try expectFloatClose(arith.tcl_math_log10(floatObj(0.0)), 0.0, 0);
}

test "tcl_math_sin + tcl_math_cos at canonical angles" {
    try expectFloatClose(arith.tcl_math_sin(floatObj(0.0)), 0.0, 1e-12);
    try expectFloatClose(arith.tcl_math_cos(floatObj(0.0)), 1.0, 1e-12);
    const pi = std.math.pi;
    try expectFloatClose(arith.tcl_math_sin(floatObj(pi)), 0.0, 1e-9);
    try expectFloatClose(arith.tcl_math_cos(floatObj(pi)), -1.0, 1e-9);
}

test "tcl_math_fabs" {
    try expectFloatClose(arith.tcl_math_fabs(floatObj(-3.5)), 3.5, 0);
    try expectFloatClose(arith.tcl_math_fabs(floatObj(3.5)), 3.5, 0);
    try expectFloatClose(arith.tcl_math_fabs(intObj(-7)), 7.0, 0);
}
