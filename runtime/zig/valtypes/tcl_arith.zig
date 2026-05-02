// Float-aware arithmetic for Tcl expr evaluation.
//
// Each function takes two TclObj i32 pointers and returns a TclObj i32.
// If either operand is a float (TYPE_FLOAT or a string containing '.'/e),
// the result is TYPE_FLOAT; otherwise TYPE_INT.
//
// Integer semantics are preserved: ``tcl_arith_div(7, 2)`` returns 3
// (integer division), while ``tcl_arith_div(7.0, 2)`` returns 3.5.
//
// **Divide-by-zero / mod-by-zero raise a Tcl error** (PR #237 second-
// pass review).  Real Tcl raises ``divide by zero`` with errorCode
// ``ARITH DIVZERO {divide by zero}``; we mirror that via
// ``stubs.raise``.  Earlier versions silently returned 0 to keep the
// tcllib counter::init path running, but that had the unacceptable
// cost of every other legitimate divide-by-zero in user code silently
// producing 0 — exactly the surprise that breaks ports.  Counter
// tests that depended on the silent-zero behaviour need their own
// initialisation-order fix downstream rather than the arithmetic
// layer covering for them.

const std = @import("std");
const obj = @import("tcl_obj.zig");
const bignum = @import("tcl_bignum.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

const TYPE_INT = obj.TYPE_INT;
const TYPE_FLOAT = obj.TYPE_FLOAT;
const TYPE_STRING = obj.TYPE_STRING;
const TYPE_BIGNUM = obj.TYPE_BIGNUM;

fn is_float(o: i32) bool {
    if (o == 0) return false;
    const tag = obj.obj_type(o);
    if (tag == TYPE_FLOAT) return true;
    if (tag == TYPE_STRING) {
        const s = obj.obj_ensure_string(o);
        if (s.len == 0) return false;
        const p: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| {
            const c = p[i];
            if (c == '.' or c == 'e' or c == 'E') return true;
        }
    }
    return false;
}

/// True iff the operand is already represented as a bignum (TYPE_BIGNUM)
/// or a string literal that exceeds the i64 range and so demands i128
/// arithmetic to compute correctly.  Used by the arithmetic helpers to
/// decide between the fast i64-with-wrap path and the i128 promotion
/// path.
fn is_bignum(o: i32) bool {
    if (o == 0) return false;
    const tag = obj.obj_type(o);
    if (tag == TYPE_BIGNUM) return true;
    if (tag == TYPE_STRING) {
        // Avoid the i128 parse on every operand by first probing the
        // i64 parser; only literals that *don't* fit in i64 need the
        // bignum path.
        const s = obj.obj_ensure_string(o);
        if (s.len == 0) return false;
        if (obj.try_parse_int(s.ptr, s.len) != null) return false;
        return bignum.parse_i128(s.ptr, s.len) != null;
    }
    return false;
}

pub export fn tcl_arith_add(a: i32, b: i32) i32 {
    if (is_float(a) or is_float(b))
        return obj.obj_new_float(obj.obj_get_float(a) + obj.obj_get_float(b));
    if (is_bignum(a) or is_bignum(b)) {
        const r = bignum.add_overflow(obj.obj_get_bignum(a), obj.obj_get_bignum(b)) orelse {
            // i128 boundary — saturate to wrap rather than trap.
            // C Tcl 9.0 produces the mathematically correct value via
            // libtommath; Stage 1 documents this saturation in
            // ``tcl_bignum.zig`` and leaves Stage 2 to lift the cap.
            return obj.obj_new_bignum(std.math.maxInt(i128));
        };
        return obj.obj_new_bignum(r);
    }
    // Detect i64 overflow and promote to bignum so wrap-around
    // doesn't silently corrupt mathematically meaningful sums.
    const ai = obj.obj_get_int(a);
    const bi = obj.obj_get_int(b);
    const r = @addWithOverflow(ai, bi);
    if (r[1] == 0) return obj.obj_new_int(r[0]);
    return obj.obj_new_bignum(@as(i128, ai) + @as(i128, bi));
}

pub export fn tcl_arith_sub(a: i32, b: i32) i32 {
    if (is_float(a) or is_float(b))
        return obj.obj_new_float(obj.obj_get_float(a) - obj.obj_get_float(b));
    if (is_bignum(a) or is_bignum(b)) {
        const r = bignum.sub_overflow(obj.obj_get_bignum(a), obj.obj_get_bignum(b)) orelse {
            return obj.obj_new_bignum(std.math.minInt(i128));
        };
        return obj.obj_new_bignum(r);
    }
    const ai = obj.obj_get_int(a);
    const bi = obj.obj_get_int(b);
    const r = @subWithOverflow(ai, bi);
    if (r[1] == 0) return obj.obj_new_int(r[0]);
    return obj.obj_new_bignum(@as(i128, ai) - @as(i128, bi));
}

pub export fn tcl_arith_mul(a: i32, b: i32) i32 {
    if (is_float(a) or is_float(b))
        return obj.obj_new_float(obj.obj_get_float(a) * obj.obj_get_float(b));
    if (is_bignum(a) or is_bignum(b)) {
        const r = bignum.mul_overflow(obj.obj_get_bignum(a), obj.obj_get_bignum(b)) orelse {
            // Pick the saturation sign based on operand signs so the
            // sign of the result is at least directionally correct.
            const av = obj.obj_get_bignum(a);
            const bv = obj.obj_get_bignum(b);
            const negative = (av < 0) != (bv < 0);
            return obj.obj_new_bignum(if (negative) std.math.minInt(i128) else std.math.maxInt(i128));
        };
        return obj.obj_new_bignum(r);
    }
    const ai = obj.obj_get_int(a);
    const bi = obj.obj_get_int(b);
    const r = @mulWithOverflow(ai, bi);
    if (r[1] == 0) return obj.obj_new_int(r[0]);
    return obj.obj_new_bignum(@as(i128, ai) * @as(i128, bi));
}

pub export fn tcl_arith_div(a: i32, b: i32) i32 {
    if (is_float(a) or is_float(b)) {
        const bf = obj.obj_get_float(b);
        if (bf == 0.0) {
            stubs.raise("divide by zero");
            return obj.obj_new_int(0);
        }
        return obj.obj_new_float(obj.obj_get_float(a) / bf);
    }
    if (is_bignum(a) or is_bignum(b)) {
        const bv = obj.obj_get_bignum(b);
        if (bv == 0) {
            stubs.raise("divide by zero");
            return obj.obj_new_int(0);
        }
        const av = obj.obj_get_bignum(a);
        // ``i128::MIN / -1`` overflows the signed range.  Promote-
        // saturate at the i128 boundary; mirrors the wrap of i64
        // ``min/-1`` we already accept for the i64-only case
        // (matches ``expr-34.13``'s ``int($min / -1) == 2147483648``
        // pattern at the next size up).
        if (av == std.math.minInt(i128) and bv == -1) {
            return obj.obj_new_bignum(std.math.maxInt(i128));
        }
        return obj.obj_new_bignum(@divTrunc(av, bv));
    }
    const bi = obj.obj_get_int(b);
    if (bi == 0) {
        stubs.raise("divide by zero");
        return obj.obj_new_int(0);
    }
    const ai = obj.obj_get_int(a);
    if (ai == std.math.minInt(i64) and bi == -1) {
        return obj.obj_new_bignum(-@as(i128, ai));
    }
    return obj.obj_new_int(@divTrunc(ai, bi));
}

pub export fn tcl_arith_mod(a: i32, b: i32) i32 {
    // Tcl's ``%`` follows Python-like "result has same sign as
    // divisor" semantics — see ``tclExecute.c`` INST_MOD which does
    // ``remainder = w1 % w2;  if (remainder != 0 && (remainder ^ w2) < 0)
    // remainder += w2;``.  Zig's ``@rem`` truncates toward zero (C's
    // ``%``), so we need the sign-fixup or ``-1 % (1 << 63)`` returns
    // ``-1`` instead of the upstream-correct ``9223372036854775807``
    // (the regression covered by ``expr-32.4`` and ``expr-32.6`` and
    // the ``Bug 1585704`` cluster).
    if (is_float(a) or is_float(b)) {
        const bf = obj.obj_get_float(b);
        if (bf == 0.0) {
            stubs.raise("divide by zero");
            return obj.obj_new_int(0);
        }
        var r = @rem(obj.obj_get_float(a), bf);
        if (r != 0.0 and ((r < 0) != (bf < 0))) r += bf;
        return obj.obj_new_float(r);
    }
    if (is_bignum(a) or is_bignum(b)) {
        const bv = obj.obj_get_bignum(b);
        if (bv == 0) {
            stubs.raise("divide by zero");
            return obj.obj_new_int(0);
        }
        const av = obj.obj_get_bignum(a);
        var r = @rem(av, bv);
        if (r != 0 and ((r < 0) != (bv < 0))) r += bv;
        return obj.obj_new_bignum(r);
    }
    const bi = obj.obj_get_int(b);
    if (bi == 0) {
        stubs.raise("divide by zero");
        return obj.obj_new_int(0);
    }
    var r = @rem(obj.obj_get_int(a), bi);
    if (r != 0 and ((r < 0) != (bi < 0))) r += bi;
    return obj.obj_new_int(r);
}

/// double(x) — coerce to float.  Used in ``expr {$n / double($d)}``.
pub export fn tcl_math_double(a: i32) i32 {
    if (a == 0) return obj.obj_new_float(0.0);
    if (obj.obj_type(a) == TYPE_FLOAT) return a;
    return obj.obj_new_float(obj.obj_get_float(a));
}

/// int(x) — truncate to integer.
pub export fn tcl_math_int(a: i32) i32 {
    if (a == 0) return obj.obj_new_int(0);
    return obj.obj_new_int(obj.obj_get_int(a));
}

/// round(x) — round to nearest integer (returns float TclObj or int).
pub export fn tcl_math_round(a: i32) i32 {
    const f = obj.obj_get_float(a);
    return obj.obj_new_int(@intFromFloat(@round(f)));
}

/// log(x) — natural logarithm.
pub export fn tcl_math_log(a: i32) i32 {
    const f = obj.obj_get_float(a);
    if (f <= 0.0) return obj.obj_new_float(0.0);
    return obj.obj_new_float(@log(f));
}

/// sqrt(x) — square root.
pub export fn tcl_math_sqrt(a: i32) i32 {
    const f = obj.obj_get_float(a);
    if (f < 0.0) return obj.obj_new_float(0.0);
    return obj.obj_new_float(@sqrt(f));
}

/// exp(x) — e^x.
pub export fn tcl_math_exp(a: i32) i32 {
    return obj.obj_new_float(@exp(obj.obj_get_float(a)));
}

/// log10(x) — base-10 logarithm.
pub export fn tcl_math_log10(a: i32) i32 {
    const f = obj.obj_get_float(a);
    if (f <= 0.0) return obj.obj_new_float(0.0);
    return obj.obj_new_float(@log10(f));
}

/// sin(x).
pub export fn tcl_math_sin(a: i32) i32 {
    return obj.obj_new_float(@sin(obj.obj_get_float(a)));
}

/// cos(x).
pub export fn tcl_math_cos(a: i32) i32 {
    return obj.obj_new_float(@cos(obj.obj_get_float(a)));
}

/// fabs(x) — absolute value as float.
pub export fn tcl_math_fabs(a: i32) i32 {
    return obj.obj_new_float(@abs(obj.obj_get_float(a)));
}

// ----------------------------------------------------------------------
// Bitwise / shift helpers — issues #260, #261, #262
//
// Tcl 9.0 rejects floating-point operands in bitwise (``&`` ``|`` ``^``
// ``~``) and shift (``<<`` ``>>``) operators with errors of the form:
//
//   cannot use floating-point value "X" as operand of "OP"
//   cannot use floating-point value "X" as left operand of "OP"
//   cannot use floating-point value "X" as right operand of "OP"
//
// Shift counts must additionally be non-negative; a negative count
// raises ``negative shift argument``.  The Python VM enforces both
// rules in ``vm/machine.py::_bitwise_binary``; the WASM expression
// emitter previously inlined ``i64.shl`` / ``i64.and`` / ``i64.or`` /
// ``i64.xor`` directly, which silently truncated floats and accepted
// negative shift counts (WASM masks the count by 63).  These helpers
// are called from ``core/compiler/codegen/wasm/_emitter/_expressions.py``
// to recover the missing domain checks.

/// Build ``cannot use floating-point value "X" as <position> operand of "<op>"``
/// and route it through the Tcl error path.
fn raise_float_in_bitwise(o: i32, op_sym: []const u8, position: []const u8) void {
    // Preserve the first error in a chain: if a prior helper in the
    // same statement (e.g. a missing-variable read on the other
    // operand) already set ``error_flag``, don't overwrite the
    // pending diagnostic with a follow-on ``cannot use floating-
    // point value`` error — match reference Tcl's "first error
    // wins" semantics for a single command.
    if (@import("../interp/tcl_catch.zig").error_flag != 0) return;
    const s = obj.obj_ensure_string(o);
    const prefix: []const u8 = "cannot use floating-point value \"";
    const middle: []const u8 = "\" as ";
    const between: []const u8 = " operand of \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + s.len + middle.len + position.len + between.len + op_sym.len + suffix.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    if (s.len > 0) {
        const sp: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| {
            buf[off] = sp[i];
            off += 1;
        }
    }
    for (middle) |c| {
        buf[off] = c;
        off += 1;
    }
    for (position) |c| {
        buf[off] = c;
        off += 1;
    }
    for (between) |c| {
        buf[off] = c;
        off += 1;
    }
    for (op_sym) |c| {
        buf[off] = c;
        off += 1;
    }
    for (suffix) |c| {
        buf[off] = c;
        off += 1;
    }
    // Issue #317: ``obj_new_string_take`` so the error TclObj owns
    // the message buffer; the older ``obj_new_string`` left
    // ``OBJ_STR_CAP = 0`` and the buf was leaked on release inside
    // a ``catch``.  Outside of ``catch`` ``tcl_cmd_error`` traps
    // the process so the leak doesn't accumulate, but io.test
    // exercises the catched path heavily through tcltest.
    const msg = obj.obj_new_string_take(buf_addr, total, total);
    @import("../interp/tcl_catch.zig").tcl_cmd_error(msg);
}

/// Build ``cannot use floating-point value "X" as operand of "<op>"``
/// (unary form — no left/right qualifier).
fn raise_float_in_unary_bitwise(o: i32, op_sym: []const u8) void {
    // Preserve the first error in a chain — see the binary form for
    // the rationale.
    if (@import("../interp/tcl_catch.zig").error_flag != 0) return;
    const s = obj.obj_ensure_string(o);
    const prefix: []const u8 = "cannot use floating-point value \"";
    const middle: []const u8 = "\" as operand of \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + s.len + middle.len + op_sym.len + suffix.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    if (s.len > 0) {
        const sp: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| {
            buf[off] = sp[i];
            off += 1;
        }
    }
    for (middle) |c| {
        buf[off] = c;
        off += 1;
    }
    for (op_sym) |c| {
        buf[off] = c;
        off += 1;
    }
    for (suffix) |c| {
        buf[off] = c;
        off += 1;
    }
    // Issue #317: ``obj_new_string_take`` so the error TclObj owns
    // the message buffer; the older ``obj_new_string`` left
    // ``OBJ_STR_CAP = 0`` and the buf was leaked on release inside
    // a ``catch``.  Outside of ``catch`` ``tcl_cmd_error`` traps
    // the process so the leak doesn't accumulate, but io.test
    // exercises the catched path heavily through tcltest.
    const msg = obj.obj_new_string_take(buf_addr, total, total);
    @import("../interp/tcl_catch.zig").tcl_cmd_error(msg);
}

fn check_int_binary(a: i32, b: i32, op_sym: []const u8) bool {
    if (is_float(a)) {
        raise_float_in_bitwise(a, op_sym, "left");
        return false;
    }
    if (is_float(b)) {
        raise_float_in_bitwise(b, op_sym, "right");
        return false;
    }
    return true;
}

pub export fn tcl_arith_lshift(a: i32, b: i32) i32 {
    if (!check_int_binary(a, b, "<<")) return obj.obj_new_int(0);
    const bi = obj.obj_get_int(b);
    if (bi < 0) {
        stubs.raise("negative shift argument");
        return obj.obj_new_int(0);
    }
    // Promote to i128 when the operand is already a bignum, when the
    // shift count alone would overflow i64, or when the shifted value
    // would step past i64::MAX / under i64::MIN.  This is the path
    // that turns ``1 << 63`` from the silent two's-complement wrap
    // ``-9223372036854775808`` into the mathematically correct
    // bignum ``9223372036854775808`` — the regression covered by
    // upstream ``expr-32.{3..9}`` and the ``Bug 1585704`` cluster.
    if (is_bignum(a) or bi >= 63) {
        const av = obj.obj_get_bignum(a);
        if (av == 0) return obj.obj_new_int(0);
        const count: u32 = if (bi >= std.math.maxInt(u32)) std.math.maxInt(u32) else @intCast(bi);
        const r = bignum.shl_overflow(av, count) orelse {
            return obj.obj_new_bignum(if (av < 0) std.math.minInt(i128) else std.math.maxInt(i128));
        };
        return obj.obj_new_bignum(r);
    }
    const ai = obj.obj_get_int(a);
    // For 0 <= bi < 63 the i64 result is well-defined when no
    // overflow occurs; if it does overflow, promote to i128.
    const shift: u6 = @intCast(bi);
    const widened = @as(i128, ai) << shift;
    if (widened >= std.math.minInt(i64) and widened <= std.math.maxInt(i64)) {
        return obj.obj_new_int(@intCast(widened));
    }
    return obj.obj_new_bignum(widened);
}

pub export fn tcl_arith_rshift(a: i32, b: i32) i32 {
    if (!check_int_binary(a, b, ">>")) return obj.obj_new_int(0);
    const bi = obj.obj_get_int(b);
    if (bi < 0) {
        stubs.raise("negative shift argument");
        return obj.obj_new_int(0);
    }
    const ai = obj.obj_get_int(a);
    if (bi >= 64) {
        // Arithmetic shift by 64+ produces 0 for non-negative ``a``
        // and -1 for negative ``a``.  Match that exactly rather than
        // letting WASM mask the count.
        return obj.obj_new_int(if (ai < 0) -1 else 0);
    }
    const shift: u6 = @intCast(bi);
    return obj.obj_new_int(ai >> shift);
}

pub export fn tcl_arith_band(a: i32, b: i32) i32 {
    if (!check_int_binary(a, b, "&")) return obj.obj_new_int(0);
    return obj.obj_new_int(obj.obj_get_int(a) & obj.obj_get_int(b));
}

pub export fn tcl_arith_bor(a: i32, b: i32) i32 {
    if (!check_int_binary(a, b, "|")) return obj.obj_new_int(0);
    return obj.obj_new_int(obj.obj_get_int(a) | obj.obj_get_int(b));
}

pub export fn tcl_arith_bxor(a: i32, b: i32) i32 {
    if (!check_int_binary(a, b, "^")) return obj.obj_new_int(0);
    return obj.obj_new_int(obj.obj_get_int(a) ^ obj.obj_get_int(b));
}

pub export fn tcl_arith_bnot(a: i32) i32 {
    if (is_float(a)) {
        raise_float_in_unary_bitwise(a, "~");
        return obj.obj_new_int(0);
    }
    return obj.obj_new_int(~obj.obj_get_int(a));
}

/// Float-preserving unary negation.  Mirrors ``tcl_arith_sub(0, x)``:
/// int → int, any float → float.  Used by the WASM expression emitter
/// in object-context paths (``_emit_expr_obj``) so that ``-$x`` of a
/// float-string variable keeps its TYPE_FLOAT tag end-to-end and the
/// bitwise / shift domain checks (``tcl_arith_lshift`` /
/// ``tcl_arith_bnot``) observe the float on the operand chain.  Without
/// this helper the inline ``0 - x`` i64 path silently truncates to int
/// and the float check is bypassed (Codex review on PR #287).
///
/// Bignum-aware: ``-(1<<127)`` is i128::MIN whose magnitude exceeds
/// i128::MAX by 1, so the ``i64.MIN`` wrap-trick for ``-i64::MIN`` no
/// longer suffices once the operand can carry an i128 payload.  We
/// route through :func:`bignum.sub_overflow(0, av)` and saturate to
/// i128::MAX on the one boundary case.
pub export fn tcl_arith_neg(a: i32) i32 {
    if (is_float(a)) return obj.obj_new_float(-obj.obj_get_float(a));
    if (is_bignum(a)) {
        const av = obj.obj_get_bignum(a);
        const r = bignum.sub_overflow(0, av) orelse return obj.obj_new_bignum(std.math.maxInt(i128));
        return obj.obj_new_bignum(r);
    }
    const ai = obj.obj_get_int(a);
    if (ai == std.math.minInt(i64)) {
        // Promote: ``-(-2^63)`` = ``2^63`` doesn't fit in i64.
        return obj.obj_new_bignum(-@as(i128, ai));
    }
    return obj.obj_new_int(-ai);
}
