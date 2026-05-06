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
/// or a string literal that exceeds the i64 range and so demands wider-
/// than-i64 arithmetic to compute correctly.  Used by the arithmetic
/// helpers to decide between the fast i64-with-promotion path and the
/// Managed (arbitrary-precision) path.
fn is_bignum(o: i32) bool {
    if (o == 0) return false;
    const tag = obj.obj_type(o);
    if (tag == TYPE_BIGNUM) return true;
    if (tag == TYPE_STRING) {
        // Avoid the bignum parse on every operand by first probing the
        // i64 parser; only literals that *don't* fit in i64 need the
        // bignum path.
        const s = obj.obj_ensure_string(o);
        if (s.len == 0) return false;
        if (obj.try_parse_int(s.ptr, s.len) != null) return false;
        // i128 parse covers values up to ~38 decimal digits — past that
        // we still want the bignum path, so probe alloc_from_string and
        // immediately destroy.
        if (bignum.parse_i128(s.ptr, s.len) != null) return true;
        const m = bignum.alloc_from_string(s.ptr, s.len) orelse return false;
        bignum.destroy(m);
        return true;
    }
    return false;
}

/// Borrowed-or-owned BigInt promotion result.  Returned by
/// :func:`promote_to_bignum` so callers can destroy the BigInt
/// only when they own it.
const PromotedBigInt = struct { m: *bignum.BigInt, owned: bool };

/// Promote an arithmetic operand to a ``*bignum.BigInt``.  Returns
/// ``null`` on OOM.  Callers must :func:`bignum.destroy` the
/// result iff ``owned`` is true.
fn promote_to_bignum(o: i32) ?PromotedBigInt {
    const r = obj.obj_promote_to_bignum(o);
    if (r.m) |m| return PromotedBigInt{ .m = m, .owned = r.owned };
    return null;
}

fn release_promoted(p: PromotedBigInt) void {
    if (p.owned) bignum.destroy(p.m);
}

/// Generic Managed-backed binary op.  Promotes both operands to
/// BigInt, runs the op, and wraps the result in a TYPE_BIGNUM
/// TclObj (auto-collapsing to TYPE_INT when the result fits i64).
/// Returns ``null`` on OOM.
fn managed_binop(
    a: i32,
    b: i32,
    op: *const fn (a: *const bignum.BigInt, b: *const bignum.BigInt) ?*bignum.BigInt,
) ?i32 {
    const ap = promote_to_bignum(a) orelse return null;
    defer release_promoted(ap);
    const bp = promote_to_bignum(b) orelse return null;
    defer release_promoted(bp);
    const r = op(ap.m, bp.m) orelse return null;
    return obj.obj_new_bignum_take(r);
}

pub export fn tcl_arith_add(a: i32, b: i32) i32 {
    if (!check_numeric_binary(a, b, "+")) return obj.obj_new_int(0);
    if (is_float(a) or is_float(b))
        return obj.obj_new_float(obj.obj_get_float(a) + obj.obj_get_float(b));
    // Bignum operand → Managed path (arbitrary precision).  The
    // Stage 1 ``add_overflow`` saturated at i128, which silently
    // miscompiled e.g. ``(1 << 126) + (1 << 126)`` (2^127, fits
    // i128 by 1 bit) and any larger combination.  Stage 2 routes
    // the path through ``std.math.big.int.Managed`` for unbounded
    // precision matching C Tcl's libtommath.
    if (is_bignum(a) or is_bignum(b)) {
        return managed_binop(a, b, bignum.alloc_add) orelse return obj.obj_new_int(0);
    }
    // Detect i64 overflow and promote to bignum so wrap-around
    // doesn't silently corrupt mathematically meaningful sums.
    // i64 + i64 always fits in i65, well within i128 — no need
    // for the Managed allocation here, just the i128 box (which
    // auto-collapses if the result fits i64).
    const ai = obj.obj_get_int(a);
    const bi = obj.obj_get_int(b);
    const r = @addWithOverflow(ai, bi);
    if (r[1] == 0) return obj.obj_new_int(r[0]);
    return obj.obj_new_bignum(@as(i128, ai) + @as(i128, bi));
}

pub export fn tcl_arith_sub(a: i32, b: i32) i32 {
    if (!check_numeric_binary(a, b, "-")) return obj.obj_new_int(0);
    if (is_float(a) or is_float(b))
        return obj.obj_new_float(obj.obj_get_float(a) - obj.obj_get_float(b));
    if (is_bignum(a) or is_bignum(b)) {
        return managed_binop(a, b, bignum.alloc_sub) orelse return obj.obj_new_int(0);
    }
    const ai = obj.obj_get_int(a);
    const bi = obj.obj_get_int(b);
    const r = @subWithOverflow(ai, bi);
    if (r[1] == 0) return obj.obj_new_int(r[0]);
    return obj.obj_new_bignum(@as(i128, ai) - @as(i128, bi));
}

pub export fn tcl_arith_mul(a: i32, b: i32) i32 {
    if (!check_numeric_binary(a, b, "*")) return obj.obj_new_int(0);
    if (is_float(a) or is_float(b))
        return obj.obj_new_float(obj.obj_get_float(a) * obj.obj_get_float(b));
    if (is_bignum(a) or is_bignum(b)) {
        return managed_binop(a, b, bignum.alloc_mul) orelse return obj.obj_new_int(0);
    }
    const ai = obj.obj_get_int(a);
    const bi = obj.obj_get_int(b);
    const r = @mulWithOverflow(ai, bi);
    if (r[1] == 0) return obj.obj_new_int(r[0]);
    // i64 * i64 fits in i128 (max product magnitude is 2^126), so
    // the i128 promotion is enough — no Managed allocation needed.
    return obj.obj_new_bignum(@as(i128, ai) * @as(i128, bi));
}

pub export fn tcl_arith_div(a: i32, b: i32) i32 {
    if (!check_numeric_binary(a, b, "/")) return obj.obj_new_int(0);
    if (is_float(a) or is_float(b)) {
        const bf = obj.obj_get_float(b);
        if (bf == 0.0) {
            stubs.raise("divide by zero");
            return obj.obj_new_int(0);
        }
        return obj.obj_new_float(obj.obj_get_float(a) / bf);
    }
    if (is_bignum(a) or is_bignum(b)) {
        const ap = promote_to_bignum(a) orelse return obj.obj_new_int(0);
        defer release_promoted(ap);
        const bp = promote_to_bignum(b) orelse return obj.obj_new_int(0);
        defer release_promoted(bp);
        if (bp.m.eqlZero()) {
            stubs.raise("divide by zero");
            return obj.obj_new_int(0);
        }
        const r = bignum.alloc_div_trunc(ap.m, bp.m) orelse return obj.obj_new_int(0);
        return obj.obj_new_bignum_take(r);
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
    // Tcl 9 ``/`` uses floored division (rounds toward negative
    // infinity).  ``-1 / 2`` is ``-1`` in Tcl (and Python), not
    // ``0`` like C / ``@divTrunc``.  Compute via @divTrunc and
    // adjust when the remainder is non-zero and the operand
    // signs differ.
    var q = @divTrunc(ai, bi);
    const r = ai - q * bi;
    if (r != 0 and ((r < 0) != (bi < 0))) q -= 1;
    return obj.obj_new_int(q);
}

pub export fn tcl_arith_mod(a: i32, b: i32) i32 {
    if (!check_numeric_binary(a, b, "%")) return obj.obj_new_int(0);
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
        const ap = promote_to_bignum(a) orelse return obj.obj_new_int(0);
        defer release_promoted(ap);
        const bp = promote_to_bignum(b) orelse return obj.obj_new_int(0);
        defer release_promoted(bp);
        if (bp.m.eqlZero()) {
            stubs.raise("divide by zero");
            return obj.obj_new_int(0);
        }
        const r = bignum.alloc_mod_floor(ap.m, bp.m) orelse return obj.obj_new_int(0);
        return obj.obj_new_bignum_take(r);
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

/// Convert an f64 magnitude that exceeds the i128 range to a
/// TYPE_BIGNUM TclObj holding the exact integer part of *fval*.
/// Used by :func:`tcl_math_int` for inputs like ``1.0e30`` which
/// the i128 fast path can't represent.
///
/// Implementation: format the float in fixed-point notation via
/// ``std.fmt.bufPrint("{d:.0}", ...)`` (rounded to no fractional
/// digits, but Zig formats f64 conservatively so we get the exact
/// IEEE-754 integer view), then feed the resulting digit string
/// to ``bignum.alloc_from_string``.  Matches upstream Tcl's
/// ``int(1.0e30) = 1000000000000000019884624838656`` (the exact
/// double value, not the mathematical ``1e30``).
fn float_to_bignum_obj(fval: f64) i32 {
    // Truncate toward zero before formatting so fractional bits
    // don't appear in the digit string ``alloc_from_string``
    // would reject.
    const trunc = @trunc(fval);
    var buf: [80]u8 = undefined;
    const slice = std.fmt.bufPrint(&buf, "{d:.0}", .{trunc}) catch {
        return obj.obj_new_int(0);
    };
    const m = bignum.alloc_from_string(@intFromPtr(slice.ptr), @intCast(slice.len)) orelse {
        return obj.obj_new_int(0);
    };
    return obj.obj_new_bignum_take(m);
}

/// double(x) — coerce to float.  Used in ``expr {$n / double($d)}``.
///
/// Refcount contract: caller passes ownership of ``a`` and expects a
/// fresh +1-ref result.  When the operand is already a float we
/// must NOT return the same handle without bumping its refcount,
/// otherwise the caller's release-of-arg + release-of-result pair
/// frees the same TclObj twice (Copilot review #326).
pub export fn tcl_math_double(a: i32) i32 {
    if (a == 0) return obj.obj_new_float(0.0);
    if (obj.obj_type(a) == TYPE_FLOAT) {
        obj.tcl_obj_retain(a);
        return a;
    }
    // ``double(x)`` for a non-numeric string operand should raise
    // ``expected floating-point number but got "x"`` — matching
    // reference Tcl's ``ExprDoubleFunc``.  Without the validation
    // ``obj_get_float`` falls through to ``return 0.0`` and the
    // caller silently sees ``0.0`` for any non-numeric input,
    // which made ``::tcl::OptCheckType x float`` succeed when it
    // should error (opt-7.1 expected ``[catch ... 1]`` for the
    // ``double(x)`` case but got ``0`` from the silent zero).
    const tag = obj.obj_type(a);
    if (tag != TYPE_INT and tag != TYPE_FLOAT and tag != TYPE_BIGNUM) {
        const sa = obj.obj_ensure_string(a);
        if (obj.try_parse_float(sa.ptr, sa.len) == null and
            obj.try_parse_int(sa.ptr, sa.len) == null and
            bignum.parse_i128(sa.ptr, sa.len) == null and
            !bignum.string_needs_bignum(sa.ptr, sa.len))
        {
            const prefix: []const u8 = "expected floating-point number but got \"";
            const suffix: []const u8 = "\"";
            const total: u32 = @as(u32, @intCast(prefix.len)) + sa.len +
                @as(u32, @intCast(suffix.len));
            const buf = obj.alloc(total);
            const d: [*]u8 = @ptrFromInt(buf);
            var off: u32 = 0;
            for (prefix) |b| {
                d[off] = b;
                off += 1;
            }
            if (sa.len > 0) {
                const sp: [*]const u8 = @ptrFromInt(sa.ptr);
                for (0..sa.len) |k| {
                    d[off] = sp[k];
                    off += 1;
                }
            }
            for (suffix) |b| {
                d[off] = b;
                off += 1;
            }
            const msg = obj.obj_new_string_take(buf, total, total);
            const tcl_catch = @import("../interp/tcl_catch.zig");
            tcl_catch.tcl_cmd_error(msg);
            return obj.obj_new_float(0.0);
        }
    }
    return obj.obj_new_float(obj.obj_get_float(a));
}

/// int(x) — truncate to integer.  Bignum-aware: ``int(1 << 200)``
/// preserves the full 200-bit value rather than truncating to the
/// low 64 bits.  ``wide()`` and ``entier()`` route through the same
/// helper from the WASM emitter, so all three preserve precision.
pub export fn tcl_math_int(a: i32) i32 {
    if (a == 0) return obj.obj_new_int(0);
    // Already an integer-shaped operand → return as-is (bignum stays
    // bignum, int stays int).  ``int($x)`` on a string operand still
    // routes through obj_get_int below; on a TYPE_FLOAT operand it
    // truncates toward zero.  Retain on the same-handle return so
    // the caller's release-of-arg pair doesn't double-free the
    // shared TclObj (Copilot review #326).
    const tag = obj.obj_type(a);
    if (tag == TYPE_INT or tag == TYPE_BIGNUM) {
        obj.tcl_obj_retain(a);
        return a;
    }
    if (tag == TYPE_FLOAT) {
        // Tcl's ``int(3.9)`` = 3, ``int(-3.9)`` = -3 — truncation toward
        // zero.  Float magnitudes that exceed i64 land in TYPE_BIGNUM
        // via the f64→i128 widening + auto-collapse path; values that
        // exceed i128 (e.g. ``1e30``) need the Managed setFloat path.
        const fval = obj.obj_get_float(a);
        if (std.math.isNan(fval) or std.math.isInf(fval)) {
            stubs.raise("integer value too large to represent");
            return obj.obj_new_int(0);
        }
        // Fast path for values within i128 range — saves the Managed
        // alloc / setFloat overhead for the common ``int(3.14)`` etc.
        if (fval >= @as(f64, @floatFromInt(std.math.minInt(i128))) and
            fval <= @as(f64, @floatFromInt(std.math.maxInt(i128))))
        {
            return obj.obj_new_bignum(@as(i128, @intFromFloat(fval)));
        }
        // Wide-magnitude float → render to scientific notation,
        // expand the mantissa according to the exponent, then parse
        // as BigInt.  Matches Tcl's ``int(1e30)`` =
        // ``1000000000000000019884624838656`` (the exact integer
        // value of the IEEE-754 representation, not the
        // mathematical ``1e30``).
        return float_to_bignum_obj(fval);
    }
    // String / bool / list / dict etc. — try the i64 / bignum parse
    // chain via obj_promote_to_bignum, then reuse obj_new_bignum_take's
    // auto-collapse to drop back to TYPE_INT when it fits.
    const ap = obj.obj_promote_to_bignum(a);
    if (ap.m) |m| {
        if (ap.owned) return obj.obj_new_bignum_take(m);
        // Borrowed bignum — caller doesn't own.  Clone into a fresh
        // BigInt so the obj layer can safely destroy it on release.
        const cloned = m.clone() catch return obj.obj_new_int(0);
        const heap = bignum.allocator.create(bignum.BigInt) catch {
            var c = cloned;
            c.deinit();
            return obj.obj_new_int(0);
        };
        heap.* = cloned;
        return obj.obj_new_bignum_take(heap);
    }
    return obj.obj_new_int(0);
}

/// pow(x, y) — Tcl's ``**`` operator and ``pow()``-as-int math
/// function (with both args integral).  Bignum-aware: ``2 ** 100``
/// produces a TYPE_BIGNUM with the full 30-digit value rather than
/// the i64 wrap of the inline loop in the WASM emitter.
///
/// Negative exponent semantics match upstream INST_EXPON:
///   * ``a == 0, b < 0`` → "exponentiation of zero by negative power"
///   * ``a == 1`` → 1
///   * ``a == -1`` → ``b`` even ? 1 : -1
///   * otherwise (``|a| >= 2, b < 0``) → 0 (truncation)
///
/// Float operands route to ``f64`` ``std.math.pow`` so
/// ``expr {2.0 ** 0.5}`` keeps working — the integer path applies
/// only when both operands are integer-shaped.
pub export fn tcl_arith_pow(a: i32, b: i32) i32 {
    if (is_float(a) or is_float(b)) {
        return obj.obj_new_float(std.math.pow(f64, obj.obj_get_float(a), obj.obj_get_float(b)));
    }
    // Integer base / integer exponent.  We need the exponent as an
    // i64 to detect the negative-exponent corner cases; even a
    // bignum exponent that fits a u32 maxes out at ~4 billion which
    // is plenty for any well-formed Tcl program (a 4-billion-digit
    // result wouldn't fit memory anyway).
    const bi = obj.obj_get_int(b);
    if (bi < 0) {
        const ai = obj.obj_get_int(a);
        if (is_bignum(a)) {
            // |a| >= 2 (anything bigger than ±1 has |a| >= 2 in
            // bignum land too, since 0/-1/1 collapse to TYPE_INT).
            return obj.obj_new_int(0);
        }
        if (ai == 0) {
            stubs.raise("exponentiation of zero by negative power");
            return obj.obj_new_int(0);
        }
        if (ai == 1) return obj.obj_new_int(1);
        if (ai == -1) return obj.obj_new_int(if (@rem(bi, 2) == 0) 1 else -1);
        return obj.obj_new_int(0);
    }
    if (bi == 0) return obj.obj_new_int(1);
    if (bi == 1) {
        // Same-handle return: retain so the caller's
        // release-operand + release-result pair doesn't double-
        // free.  apply_binary in the runtime expr evaluator and
        // the AOT emitter helpers both follow that contract.
        obj.tcl_obj_retain(a);
        return a;
    }

    // For TYPE_INT operands, try i64 multiplication first and
    // promote on overflow.  Empirically the vast majority of Tcl
    // ``**`` calls fit i64 (e.g. ``$i ** 2`` in a loop).
    if (!is_bignum(a) and bi <= 64) {
        const ai = obj.obj_get_int(a);
        var acc: i64 = 1;
        var i: i64 = 0;
        while (i < bi) : (i += 1) {
            const r = @mulWithOverflow(acc, ai);
            if (r[1] != 0) {
                // Overflow → fall through to bignum path.
                break;
            }
            acc = r[0];
        }
        if (i == bi) return obj.obj_new_int(acc);
    }

    // Bignum path.  Promote base to BigInt, take exponent as u32.
    const ap = promote_to_bignum(a) orelse return obj.obj_new_int(0);
    defer release_promoted(ap);
    if (bi >= (1 << 28)) {
        // Reference Tcl refuses exponents >= 2^28 = 268435456 — match
        // INST_EXPON's "exponent too large" check (tclExecute.c).
        stubs.raise("exponent too large");
        return obj.obj_new_int(0);
    }
    const exp_u32: u32 = @intCast(bi);
    const r = bignum.alloc_pow(ap.m, exp_u32) orelse return obj.obj_new_int(0);
    return obj.obj_new_bignum_take(r);
}

/// round(x) — round to nearest integer.  Bignum-aware: huge floats
/// (e.g. ``round(1.0e30)``) are rendered through the same fixed-
/// point format-and-parse path as ``int()`` so the exact IEEE-754
/// integer view survives.  In-range floats take the i128 fast path.
pub export fn tcl_math_round(a: i32) i32 {
    const f = obj.obj_get_float(a);
    if (std.math.isNan(f) or std.math.isInf(f)) {
        stubs.raise("integer value too large to represent");
        return obj.obj_new_int(0);
    }
    const r = @round(f);
    if (r >= @as(f64, @floatFromInt(std.math.minInt(i128))) and
        r <= @as(f64, @floatFromInt(std.math.maxInt(i128))))
    {
        return obj.obj_new_bignum(@as(i128, @intFromFloat(r)));
    }
    return float_to_bignum_obj(r);
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

/// tan(x).
pub export fn tcl_math_tan(a: i32) i32 {
    return obj.obj_new_float(@tan(obj.obj_get_float(a)));
}

/// asin(x) — arc sine.  Domain is ``-1 ≤ x ≤ 1``; outside the
/// domain we follow C / IEEE-754 semantics and emit a NaN
/// (``std.math.asin`` returns NaN for |x| > 1).
pub export fn tcl_math_asin(a: i32) i32 {
    return obj.obj_new_float(std.math.asin(obj.obj_get_float(a)));
}

/// acos(x) — arc cosine.  Same domain handling as :func:`tcl_math_asin`.
pub export fn tcl_math_acos(a: i32) i32 {
    return obj.obj_new_float(std.math.acos(obj.obj_get_float(a)));
}

/// atan(x) — arc tangent (single-arg form).  ``atan2(y, x)`` is a
/// separate two-arg helper (:func:`tcl_math_atan2`).
pub export fn tcl_math_atan(a: i32) i32 {
    return obj.obj_new_float(std.math.atan(obj.obj_get_float(a)));
}

/// atan2(y, x) — two-arg arctangent, returning the angle in
/// ``[-π, π]``.
pub export fn tcl_math_atan2(y: i32, x: i32) i32 {
    return obj.obj_new_float(std.math.atan2(obj.obj_get_float(y), obj.obj_get_float(x)));
}

/// sinh(x) / cosh(x) / tanh(x) — hyperbolic trig.
pub export fn tcl_math_sinh(a: i32) i32 {
    return obj.obj_new_float(std.math.sinh(obj.obj_get_float(a)));
}

pub export fn tcl_math_cosh(a: i32) i32 {
    return obj.obj_new_float(std.math.cosh(obj.obj_get_float(a)));
}

pub export fn tcl_math_tanh(a: i32) i32 {
    return obj.obj_new_float(std.math.tanh(obj.obj_get_float(a)));
}

/// floor(x) — round toward -∞.  Returns a float per Tcl semantics
/// (``expr {floor(2.7)}`` is ``2.0``, not the integer ``2``).
pub export fn tcl_math_floor(a: i32) i32 {
    return obj.obj_new_float(@floor(obj.obj_get_float(a)));
}

/// ceil(x) — round toward +∞.  Float result per Tcl semantics.
pub export fn tcl_math_ceil(a: i32) i32 {
    return obj.obj_new_float(@ceil(obj.obj_get_float(a)));
}

/// fmod(x, y) — IEEE-754 remainder of ``x / y`` with the sign of
/// ``x``.  Distinct from Tcl's ``%`` operator (``expr {x % y}`` is
/// integer-only and uses divisor sign).
pub export fn tcl_math_fmod(x: i32, y: i32) i32 {
    return obj.obj_new_float(@rem(obj.obj_get_float(x), obj.obj_get_float(y)));
}

/// hypot(x, y) — ``sqrt(x*x + y*y)``, computed without intermediate
/// overflow.
pub export fn tcl_math_hypot(x: i32, y: i32) i32 {
    return obj.obj_new_float(std.math.hypot(obj.obj_get_float(x), obj.obj_get_float(y)));
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
    if (@import("../interp/tcl_result.zig").snapshot(0).code == .ERROR) return;
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
    if (@import("../interp/tcl_result.zig").snapshot(0).code == .ERROR) return;
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

/// Return true iff *o* parses as a number (int / float / bignum
/// keyword like ``Inf``).  Used by the arithmetic helpers to
/// raise ``cannot use non-numeric string "X" as Y operand of
/// "OP"`` rather than silently treating non-numeric operands as
/// 0 (the legacy ``obj_get_int`` / ``obj_get_float`` fallback).
fn obj_is_numeric(o: i32) bool {
    if (o == 0) return false;
    const tag = obj.obj_type(o);
    if (tag == TYPE_INT or tag == TYPE_FLOAT or is_bignum(o)) return true;
    const s = obj.obj_ensure_string(o);
    if (s.len == 0) return false;
    if (obj.try_parse_int(s.ptr, s.len) != null) return true;
    if (obj.try_parse_float(s.ptr, s.len) != null) return true;
    return false;
}

/// Return true when *o*'s string repr is a recognised
/// non-numeric IEEE-754 keyword (``NaN`` / ``Inf`` / ``Infinity``,
/// case-insensitive, with optional leading sign).  Tcl 9 routes
/// these through a separate diagnostic
/// (``cannot use non-numeric floating-point value "X"``) so the
/// reader can tell the difference between an unrecognised
/// string and a recognised-but-unrepresentable float.
fn is_ieee_keyword_string(o: i32) bool {
    const s = obj.obj_ensure_string(o);
    if (s.len == 0 or s.len > 9) return false;
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    var off: u32 = 0;
    if (sp[0] == '+' or sp[0] == '-') off = 1;
    const remain = s.len - off;
    if (remain != 3 and remain != 8) return false;
    var buf: [9]u8 = undefined;
    for (0..remain) |i| {
        const c = sp[off + i];
        buf[i] = if (c >= 'A' and c <= 'Z') c + 32 else c;
    }
    const lc = buf[0..remain];
    return std.mem.eql(u8, lc, "nan") or std.mem.eql(u8, lc, "inf") or std.mem.eql(u8, lc, "infinity");
}

/// Build ``cannot use non-numeric string "X" as <position>
/// operand of "<op>"`` and route it through the Tcl error path.
/// IEEE-754 keyword operands (``NaN`` / ``Inf``) get the
/// ``non-numeric floating-point value "X"`` wording instead —
/// reference Tcl 9 distinguishes the two cases (expr-22.1 / 22.3).
fn raise_non_numeric(o: i32, op_sym: []const u8, position: []const u8) void {
    if (@import("../interp/tcl_result.zig").snapshot(0).code == .ERROR) return;
    const s = obj.obj_ensure_string(o);
    const ieee_kw = is_ieee_keyword_string(o);
    const prefix: []const u8 = if (ieee_kw)
        "cannot use non-numeric floating-point value \""
    else
        "cannot use non-numeric string \"";
    const middle: []const u8 = "\" as ";
    const between: []const u8 = " operand of \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + s.len + middle.len + position.len + between.len + op_sym.len + suffix.len);
    const buf_addr: u32 = obj.alloc(total);
    if (buf_addr == 0) {
        stubs.raise("non-numeric operand");
        return;
    }
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
    const msg = obj.obj_new_string_take(buf_addr, total, total);
    @import("../interp/tcl_catch.zig").tcl_cmd_error(msg);
}

/// Validate both operands are numeric for an arithmetic op.
/// Returns true when the call should proceed; raises the
/// ``cannot use non-numeric string`` error and returns false
/// when either operand is a non-numeric string.
fn check_numeric_binary(a: i32, b: i32, op_sym: []const u8) bool {
    if (!obj_is_numeric(a)) {
        raise_non_numeric(a, op_sym, "left");
        return false;
    }
    if (!obj_is_numeric(b)) {
        raise_non_numeric(b, op_sym, "right");
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
    // Bignum operand or wide shift count → Managed (arbitrary-precision)
    // path so ``1 << 1000`` and ``(2^200) << 50`` round-trip exactly.
    // Stage 1's i128 path saturated past ``1 << 127``; Stage 2 routes
    // through ``std.math.big.int.Managed`` for unbounded precision.
    if (is_bignum(a) or bi >= 63) {
        const ap = promote_to_bignum(a) orelse return obj.obj_new_int(0);
        defer release_promoted(ap);
        if (ap.m.eqlZero()) return obj.obj_new_int(0);
        const count: u64 = @bitCast(bi);
        const r = bignum.alloc_shl(ap.m, count) orelse return obj.obj_new_int(0);
        return obj.obj_new_bignum_take(r);
    }
    const ai = obj.obj_get_int(a);
    // For 0 <= bi < 63 the i64 result is well-defined when no
    // overflow occurs; if it does overflow, promote to i128 (which
    // auto-collapses if the result fits i64).
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
    // Bignum operand → Managed shiftRight so e.g. ``(1 << 200) >> 100``
    // recovers ``1 << 100`` instead of truncating to 0 (the bottom 64
    // bits of any large power-of-two are zero).
    if (is_bignum(a)) {
        const ap = promote_to_bignum(a) orelse return obj.obj_new_int(0);
        defer release_promoted(ap);
        // Managed.shiftRight takes the shift count as usize; a bignum
        // operand with shift >= ``bit_len(a)`` produces 0 / -1 per the
        // arithmetic-shift convention, which Managed handles natively.
        const shift: u64 = @bitCast(bi);
        const r = bignum.alloc_zero() orelse return obj.obj_new_int(0);
        // On wasm32, usize == u32; guard @intCast for large shift amounts —
        // no bignum can have > 2^32 bits, so the result is trivially 0/-1.
        if (shift > std.math.maxInt(usize)) {
            bignum.destroy(r);
            return obj.obj_new_int(if (ap.m.isPositive()) 0 else -1);
        }
        r.shiftRight(ap.m, @intCast(shift)) catch {
            bignum.destroy(r);
            return obj.obj_new_int(0);
        };
        return obj.obj_new_bignum_take(r);
    }
    const ai = obj.obj_get_int(a);
    if (bi >= 64) {
        return obj.obj_new_int(if (ai < 0) -1 else 0);
    }
    const shift: u6 = @intCast(bi);
    return obj.obj_new_int(ai >> shift);
}

/// Bignum-aware bitwise binary op.  Routes through ``Managed.bitAnd /
/// bitOr / bitXor`` for arbitrary precision; the i64 fast path stays
/// for the common small-int case so we don't pay the heap allocation.
fn bitwise_managed(
    a: i32,
    b: i32,
    op: enum { band, bor, bxor },
) i32 {
    const ap = promote_to_bignum(a) orelse return obj.obj_new_int(0);
    defer release_promoted(ap);
    const bp = promote_to_bignum(b) orelse return obj.obj_new_int(0);
    defer release_promoted(bp);
    const r = bignum.alloc_zero() orelse return obj.obj_new_int(0);
    const res = switch (op) {
        .band => r.bitAnd(ap.m, bp.m),
        .bor => r.bitOr(ap.m, bp.m),
        .bxor => r.bitXor(ap.m, bp.m),
    };
    res catch {
        bignum.destroy(r);
        return obj.obj_new_int(0);
    };
    return obj.obj_new_bignum_take(r);
}

pub export fn tcl_arith_band(a: i32, b: i32) i32 {
    if (!check_numeric_binary(a, b, "&")) return obj.obj_new_int(0);
    if (!check_int_binary(a, b, "&")) return obj.obj_new_int(0);
    if (is_bignum(a) or is_bignum(b)) return bitwise_managed(a, b, .band);
    return obj.obj_new_int(obj.obj_get_int(a) & obj.obj_get_int(b));
}

pub export fn tcl_arith_bor(a: i32, b: i32) i32 {
    if (!check_numeric_binary(a, b, "|")) return obj.obj_new_int(0);
    if (!check_int_binary(a, b, "|")) return obj.obj_new_int(0);
    if (is_bignum(a) or is_bignum(b)) return bitwise_managed(a, b, .bor);
    return obj.obj_new_int(obj.obj_get_int(a) | obj.obj_get_int(b));
}

pub export fn tcl_arith_bxor(a: i32, b: i32) i32 {
    if (!check_numeric_binary(a, b, "^")) return obj.obj_new_int(0);
    if (!check_int_binary(a, b, "^")) return obj.obj_new_int(0);
    if (is_bignum(a) or is_bignum(b)) return bitwise_managed(a, b, .bxor);
    return obj.obj_new_int(obj.obj_get_int(a) ^ obj.obj_get_int(b));
}

pub export fn tcl_arith_bnot(a: i32) i32 {
    if (is_float(a)) {
        raise_float_in_unary_bitwise(a, "~");
        return obj.obj_new_int(0);
    }
    if (is_bignum(a)) {
        // ``~x`` = ``-x - 1`` in two's complement; Managed has no
        // direct bitNot but the identity gives a clean implementation
        // that matches Tcl's "result has divisor's sign" semantic
        // (here the "divisor" is implicitly ``-1``).
        const ap = promote_to_bignum(a) orelse return obj.obj_new_int(0);
        defer release_promoted(ap);
        const neg = bignum.alloc_neg(ap.m) orelse return obj.obj_new_int(0);
        defer bignum.destroy(neg);
        const one = bignum.alloc_from_int(1) orelse return obj.obj_new_int(0);
        defer bignum.destroy(one);
        const r = bignum.alloc_sub(neg, one) orelse return obj.obj_new_int(0);
        return obj.obj_new_bignum_take(r);
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
        const ap = promote_to_bignum(a) orelse return obj.obj_new_int(0);
        defer release_promoted(ap);
        const r = bignum.alloc_neg(ap.m) orelse return obj.obj_new_int(0);
        return obj.obj_new_bignum_take(r);
    }
    const ai = obj.obj_get_int(a);
    if (ai == std.math.minInt(i64)) {
        // Promote: ``-(-2^63)`` = ``2^63`` doesn't fit in i64.
        return obj.obj_new_bignum(-@as(i128, ai));
    }
    return obj.obj_new_int(-ai);
}
