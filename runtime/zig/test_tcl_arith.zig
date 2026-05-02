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
    return obj.obj_new_string(@bitCast(@intFromPtr(s.ptr)), @bitCast(s.len));
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

test "tcl_arith_add — int overflow promotes to bignum (Tcl 9.0 semantics)" {
    const max = std.math.maxInt(i64);
    // Reference Tcl 9.0 promotes i64 overflow to libtommath bignum.
    // Our Stage 1 implementation routes through i128 — the result
    // obj should report the next-higher value, *not* wrap to
    // ``i64::MIN`` like a plain ``+%``.  See ``valtypes/tcl_bignum.zig``.
    const r = arith.tcl_arith_add(intObj(max), intObj(1));
    try testing.expectEqual(obj.TYPE_BIGNUM, obj.obj_type(r));
    const big: i128 = @as(i128, max) + 1;
    try testing.expectEqual(big, obj.obj_get_bignum(r));
}

// ---- sub ------------------------------------------------------------

test "tcl_arith_sub — int - int = int" {
    try expectInt(arith.tcl_arith_sub(intObj(10), intObj(3)), 7);
    try expectInt(arith.tcl_arith_sub(intObj(0), intObj(5)), -5);
}

test "tcl_arith_sub — float - int = float" {
    try expectFloatClose(arith.tcl_arith_sub(floatObj(2.5), intObj(1)), 1.5, 1e-12);
}

test "tcl_arith_sub — underflow promotes to bignum" {
    const min = std.math.minInt(i64);
    const r = arith.tcl_arith_sub(intObj(min), intObj(1));
    try testing.expectEqual(obj.TYPE_BIGNUM, obj.obj_type(r));
    const big: i128 = @as(i128, min) - 1;
    try testing.expectEqual(big, obj.obj_get_bignum(r));
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

// ---- bignum promotion -----------------------------------------------
//
// Stage 1 (i128) bignum-overflow tests.  See
// ``valtypes/tcl_bignum.zig`` for the underlying parse / format /
// arithmetic-with-overflow helpers.  These tests exercise the wiring
// in ``tcl_arith.zig`` that promotes i64 overflow to TYPE_BIGNUM and
// renders the result through ``obj_ensure_string`` so ``puts`` / list
// formatting / tcltest's ``-result`` matcher all observe the correct
// digits — the regression caught by upstream ``expr-32.{3..9}`` and
// the "Bug 1585704" cluster (``1 << 63``, ``1 % (1 << 63)`` etc.).

fn bignumObj(value: i128) i32 {
    return obj.obj_new_bignum(value);
}

fn expectBignum(o: i32, expected: i128) !void {
    try testing.expectEqual(obj.TYPE_BIGNUM, obj.obj_type(o));
    try testing.expectEqual(expected, obj.obj_get_bignum(o));
}

test "tcl_arith_add — bignum + i64 stays bignum" {
    const big: i128 = @as(i128, std.math.maxInt(i64)) + 1;
    const r = arith.tcl_arith_add(bignumObj(big), intObj(1));
    try expectBignum(r, big + 1);
}

test "tcl_arith_add — bignum result that fits i64 collapses back to int" {
    const big: i128 = @as(i128, std.math.maxInt(i64)) + 1;
    const r = arith.tcl_arith_add(bignumObj(big), intObj(-1));
    try testing.expectEqual(obj.TYPE_INT, obj.obj_type(r));
    try expectInt(r, std.math.maxInt(i64));
}

test "tcl_arith_mul — i64 * i64 promoting to bignum" {
    // ``2^32 * 2^32`` overflows i64 (would wrap to 0) but fits in
    // i128.  Reference Tcl 9.0 produces ``18446744073709551616``
    // through the libtommath path; we produce the same value via
    // i128 promotion.
    const a: i64 = @as(i64, 1) << 32;
    const r = arith.tcl_arith_mul(intObj(a), intObj(a));
    try expectBignum(r, @as(i128, a) * @as(i128, a));
}

test "tcl_arith_lshift — 1 << 63 promotes to bignum (Bug 1585704)" {
    const r = arith.tcl_arith_lshift(intObj(1), intObj(63));
    try expectBignum(r, @as(i128, 1) << 63);
}

test "tcl_arith_lshift — 1 << 100 promotes to bignum" {
    const r = arith.tcl_arith_lshift(intObj(1), intObj(100));
    try expectBignum(r, @as(i128, 1) << 100);
}

test "tcl_arith_lshift — 0 << huge stays int 0" {
    const r = arith.tcl_arith_lshift(intObj(0), intObj(1000));
    try testing.expectEqual(obj.TYPE_INT, obj.obj_type(r));
    try expectInt(r, 0);
}

test "tcl_arith_mod — i % (1 << 63) returns i (expr-32.7-9 regression)" {
    // ``1 % (1 << 63)`` in Tcl 9.0: shift produces bignum
    // ``9223372036854775808``, mod returns ``1``.  Pre-bignum we
    // wrapped the shift and got ``1 % -9223372036854775808 = 1``
    // by coincidence — but ``0 % (1<<63)`` returned ``0`` only by
    // luck and ``0 % -(1+(1<<63))`` failed.  Lock the bignum path.
    const lhs = arith.tcl_arith_lshift(intObj(1), intObj(63));
    try testing.expectEqual(obj.TYPE_BIGNUM, obj.obj_type(lhs));
    const r = arith.tcl_arith_mod(intObj(1), lhs);
    // Result fits in i64 so it collapses back to TYPE_INT.
    try testing.expectEqual(obj.TYPE_INT, obj.obj_type(r));
    try expectInt(r, 1);
}

test "tcl_arith_mod — 0 % (1 << 63) = 0 (expr-32.7)" {
    const lhs = arith.tcl_arith_lshift(intObj(1), intObj(63));
    const r = arith.tcl_arith_mod(intObj(0), lhs);
    try expectInt(r, 0);
}

test "tcl_arith_neg — -(i64::MIN) promotes to bignum" {
    const r = arith.tcl_arith_neg(intObj(std.math.minInt(i64)));
    try expectBignum(r, -@as(i128, std.math.minInt(i64)));
}

test "tcl_arith_neg — bignum value negates as bignum" {
    const big: i128 = @as(i128, std.math.maxInt(i64)) + 1;
    const r = arith.tcl_arith_neg(bignumObj(big));
    // ``-(maxInt(i64) + 1)`` = ``minInt(i64)`` which fits in i64;
    // collapses back to TYPE_INT.
    try testing.expectEqual(obj.TYPE_INT, obj.obj_type(r));
    try expectInt(r, std.math.minInt(i64));
}

test "tcl_arith_div — i64::MIN / -1 promotes to bignum" {
    const r = arith.tcl_arith_div(intObj(std.math.minInt(i64)), intObj(-1));
    try expectBignum(r, -@as(i128, std.math.minInt(i64)));
}

test "obj_ensure_string — bignum renders to decimal string" {
    const big: i128 = @as(i128, 1) << 63;
    const o = bignumObj(big);
    const s = obj.obj_ensure_string(o);
    const sl: [*]const u8 = @ptrFromInt(s.ptr);
    try testing.expectEqualStrings("9223372036854775808", sl[0..s.len]);
}

test "obj_ensure_string — caches the rendered buffer (str_ptr persists)" {
    const big: i128 = @as(i128, 1) << 100;
    const o = bignumObj(big);
    const s1 = obj.obj_ensure_string(o);
    const s2 = obj.obj_ensure_string(o);
    // Second call should return the same cached buffer pointer
    // (TYPE_BIGNUM uses the same OBJ_STR_PTR/_LEN/_CAP slots as
    // TYPE_INT for caching, so re-rendering must not re-allocate).
    try testing.expectEqual(s1.ptr, s2.ptr);
    try testing.expectEqual(s1.len, s2.len);
}

test "obj_get_int on bignum truncates to i64 low bits" {
    // Pick a value whose i64 truncation is well-defined and
    // distinct from the full i128 magnitude.  ``(1 << 65) | 0xff`` —
    // the high bits live above the i64 boundary; the low 64 bits
    // are ``0xff``.
    const big: i128 = (@as(i128, 1) << 65) | 0xff;
    const o = bignumObj(big);
    try expectInt(o, 0xff);
}

test "obj_get_float on bignum converts (with the usual precision loss)" {
    const big: i128 = @as(i128, 1) << 63;
    const o = bignumObj(big);
    const f = obj.obj_get_float(o);
    try expectFloatClose(o, @as(f64, @floatFromInt(big)), 1.0);
    _ = f;
}

test "is_bignum — string literal exceeding i64 range parses as bignum" {
    // Operand is a TYPE_STRING whose digits exceed i64 — the
    // arithmetic helper must promote via the i128 path even when
    // neither operand was already TYPE_BIGNUM.  Mirrors how a
    // literal in a Tcl source ``expr 9223372036854775808 + 1``
    // arrives at the runtime as a string.
    const big = "9223372036854775808";
    const r = arith.tcl_arith_add(stringObj(big), intObj(1));
    try expectBignum(r, @as(i128, std.math.maxInt(i64)) + 2);
}
