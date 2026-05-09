// ``::tcl::mathop`` ensemble — prefix-form arithmetic / comparison /
// logical operators.
//
// Tcl exposes ``+``, ``-``, ``*``, ``==`` etc. as both expression
// operators *and* normal commands under ``::tcl::mathop``.  The
// expression-compiled forms are handled at codegen time
// (``core/compiler/codegen/...``); this module covers the
// **command-form** invocations like::
//
//     ::tcl::mathop::== $a $b $c
//     [namespace import ::tcl::mathop::*; + 1 2 3]
//
// which the upstream Tcl 9 ``clock.test`` / various tcllib tests rely
// on for chained comparisons (``[== a b c]``) and variadic accumulators
// (``[+ 1 2 3 4]``).
//
// Reference: ``tmp/tcl9.0.3/generic/tclMathOp.c`` (the C ensemble) and
// ``tmp/tcl9.0.3/doc/mathop.n`` (the user-facing semantics).
//
// Variadic semantics summary (matches ``mathop.n``):
//
//   +     0 args = 0;  N args = sum (left-assoc)
//   -     1 arg  = negate;  N args = a - b - c - ...
//   *     0 args = 1;  N args = product
//   /     1 arg  = 1/x;  N args = a / b / c / ...
//   %     exactly 2: a mod b
//   **    0 args = 1;  N args = right-assoc (a**(b**c))
//   ==/<=/...  variadic chain: 1 if all consecutive pairs hold
//   eq/ne pairwise string compare; eq is variadic chain
//   in/ni list membership (exactly 2 args)
//   !     1 arg: logical NOT
//   &&    0 args = 1; N args = AND
//   ||    0 args = 0; N args = OR
//   &/|/^ bitwise; identity element drives 0-arg case (-1/0/0)
//   <</>> exactly 2 args
//   ~     exactly 1 arg, bitwise NOT
//   min/max  variadic
//   @     exactly 2: list index
//
// Errors (``stubs.raise``) are emitted for arity violations and for
// divide-by-zero, matching ``tclMathOp.c``'s ``Tcl_WrongNumArgs`` /
// ``DIVZERO`` paths.

const std = @import("std");
const result_mod = @import("../interp/tcl_result.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const bignum = @import("../valtypes/tcl_bignum.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const stubs = @import("../stubs/tcl_stubs.zig");
const list = @import("../valtypes/tcl_list.zig");

const obj_new_int = obj.obj_new_int;
const obj_new_float = obj.obj_new_float;
const obj_get_int = obj.obj_get_int;
const obj_get_float = obj.obj_get_float;
const obj_ensure_str = obj.obj_ensure_string;
const TYPE_FLOAT = obj.TYPE_FLOAT;
const TYPE_BIGNUM = obj.TYPE_BIGNUM;
const TYPE_STRING = obj.TYPE_STRING;
const TYPE_INLINE_STRING = obj.TYPE_INLINE_STRING;

/// Detect bignum-shaped operand: TYPE_BIGNUM directly, or a string
/// literal that exceeds the i64 range.  Mirrors :func:`tcl_arith.is_bignum`.
fn is_bignum(o: i32) bool {
    if (o == 0) return false;
    const tag = obj.obj_type(o);
    if (tag == TYPE_BIGNUM) return true;
    if (tag == TYPE_STRING or tag == TYPE_INLINE_STRING) {
        const s = obj_ensure_str(o);
        if (s.len == 0) return false;
        if (obj.try_parse_int(s.ptr, s.len) != null) return false;
        if (bignum.parse_i128(s.ptr, s.len) != null) return true;
        const m = bignum.alloc_from_string(s.ptr, s.len) orelse return false;
        bignum.destroy(m);
        return true;
    }
    return false;
}

fn any_bignum(args: []const i32) bool {
    for (args) |a| if (is_bignum(a)) return true;
    return false;
}

/// Detect float-valued operand using the same heuristics as
/// ``tcl_arith.zig`` — a TYPE_FLOAT obj or a string literal that
/// contains ``.`` / ``e`` / ``E``.
fn is_float(o: i32) bool {
    if (o == 0) return false;
    const tag = obj.obj_type(o);
    if (tag == TYPE_FLOAT) return true;
    if (tag == TYPE_STRING or tag == TYPE_INLINE_STRING) {
        const s = obj_ensure_str(o);
        if (s.len == 0) return false;
        const p: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| {
            const c = p[i];
            if (c == '.' or c == 'e' or c == 'E') return true;
        }
    }
    return false;
}

fn any_float(args: []const i32) bool {
    for (args) |a| if (is_float(a)) return true;
    return false;
}

/// Tail-strip ``::``-prefixed namespace qualifiers so the dispatcher
/// can match a bare op-name against the per-operator table regardless
/// of whether the caller said ``+`` or ``::tcl::mathop::+``.
fn op_tail(name: []const u8) []const u8 {
    var i: usize = name.len;
    while (i > 1) {
        i -= 1;
        if (name[i] == ':' and name[i - 1] == ':') {
            return name[i + 1 ..];
        }
    }
    return name;
}

fn op_name(words: []const i32) []const u8 {
    if (words.len == 0) return "";
    const s = obj_ensure_str(words[0]);
    if (s.ptr == 0) return "";
    const p: [*]const u8 = @ptrFromInt(s.ptr);
    return op_tail(p[0..s.len]);
}

/// Arity check helper — emits a ``wrong # args`` Tcl error and returns
/// false on miss; callers return ``obj_new_int(0)`` to satisfy the
/// signature.
fn require_arity(args: []const i32, comptime opname: []const u8, comptime min: usize, comptime max: ?usize) bool {
    if (args.len < min) {
        stubs.raise("wrong # args: " ++ opname ++ " requires more arguments");
        return false;
    }
    if (max) |m| {
        if (args.len > m) {
            stubs.raise("wrong # args: " ++ opname ++ " takes too many arguments");
            return false;
        }
    }
    return true;
}

// -- arithmetic --------------------------------------------------------------

/// Bignum-aware variadic accumulator.  Promotes the running
/// accumulator to a ``*BigInt`` so each step's result keeps full
/// precision; the i64 fast paths above stay for the common case
/// where every operand fits.  Returns ``null`` on OOM.
const AccOp = enum { add, sub, mul };

fn bignum_accumulate(args: []const i32, op: AccOp, init_value: i128) ?*bignum.BigInt {
    var acc = bignum.alloc_from_int(init_value) orelse return null;
    for (args) |a| {
        const ap = obj.obj_promote_to_bignum(a);
        defer if (ap.owned) bignum.destroy(ap.m);
        if (ap.m == null) {
            bignum.destroy(acc);
            return null;
        }
        const new_acc = switch (op) {
            .add => bignum.alloc_add(acc, ap.m.?),
            .sub => bignum.alloc_sub(acc, ap.m.?),
            .mul => bignum.alloc_mul(acc, ap.m.?),
        } orelse {
            bignum.destroy(acc);
            return null;
        };
        bignum.destroy(acc);
        acc = new_acc;
    }
    return acc;
}

fn op_add(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(0);
    if (any_float(args)) {
        var sum: f64 = 0;
        for (args) |a| sum += obj_get_float(a);
        return obj_new_float(sum);
    }
    if (any_bignum(args)) {
        const r = bignum_accumulate(args, .add, 0) orelse return obj_new_int(0);
        return obj.obj_new_bignum_take(r);
    }
    var sum: i64 = 0;
    var overflowed = false;
    for (args) |a| {
        const r = @addWithOverflow(sum, obj_get_int(a));
        if (r[1] != 0) {
            overflowed = true;
            break;
        }
        sum = r[0];
    }
    if (!overflowed) return obj_new_int(sum);
    // Promote to bignum on i64 overflow.
    const r = bignum_accumulate(args, .add, 0) orelse return obj_new_int(0);
    return obj.obj_new_bignum_take(r);
}

fn op_sub(args: []const i32) i32 {
    if (args.len == 0) {
        stubs.raise("wrong # args: should be \"- ?arg ...?\"");
        return obj_new_int(0);
    }
    if (any_float(args)) {
        if (args.len == 1) return obj_new_float(-obj_get_float(args[0]));
        var acc: f64 = obj_get_float(args[0]);
        for (args[1..]) |a| acc -= obj_get_float(a);
        return obj_new_float(acc);
    }
    if (any_bignum(args)) {
        // Unary: ``- x`` = ``0 - x``.
        if (args.len == 1) {
            const ap = obj.obj_promote_to_bignum(args[0]);
            defer if (ap.owned) bignum.destroy(ap.m);
            if (ap.m == null) return obj_new_int(0);
            const r = bignum.alloc_neg(ap.m.?) orelse return obj_new_int(0);
            return obj.obj_new_bignum_take(r);
        }
        // Variadic: ``a - b - c`` = ``((a - b) - c)``.  Promote ``a``
        // to BigInt as the seed accumulator.
        const ap0 = obj.obj_promote_to_bignum(args[0]);
        defer if (ap0.owned) bignum.destroy(ap0.m);
        if (ap0.m == null) return obj_new_int(0);
        const seed = ap0.m.?.clone() catch return obj_new_int(0);
        const seed_heap = bignum.allocator.create(bignum.BigInt) catch {
            var s = seed;
            s.deinit();
            return obj_new_int(0);
        };
        seed_heap.* = seed;
        var acc = seed_heap;
        for (args[1..]) |a| {
            const ap = obj.obj_promote_to_bignum(a);
            defer if (ap.owned) bignum.destroy(ap.m);
            if (ap.m == null) {
                bignum.destroy(acc);
                return obj_new_int(0);
            }
            const new_acc = bignum.alloc_sub(acc, ap.m.?) orelse {
                bignum.destroy(acc);
                return obj_new_int(0);
            };
            bignum.destroy(acc);
            acc = new_acc;
        }
        return obj.obj_new_bignum_take(acc);
    }
    if (args.len == 1) {
        const v = obj_get_int(args[0]);
        if (v == std.math.minInt(i64)) {
            return obj.obj_new_bignum(-@as(i128, v));
        }
        return obj_new_int(-v);
    }
    var acc: i64 = obj_get_int(args[0]);
    var overflowed = false;
    for (args[1..]) |a| {
        const r = @subWithOverflow(acc, obj_get_int(a));
        if (r[1] != 0) {
            overflowed = true;
            break;
        }
        acc = r[0];
    }
    if (!overflowed) return obj_new_int(acc);
    const ap0 = obj.obj_promote_to_bignum(args[0]);
    defer if (ap0.owned) bignum.destroy(ap0.m);
    if (ap0.m == null) return obj_new_int(0);
    const seed = ap0.m.?.clone() catch return obj_new_int(0);
    const seed_heap = bignum.allocator.create(bignum.BigInt) catch {
        var s = seed;
        s.deinit();
        return obj_new_int(0);
    };
    seed_heap.* = seed;
    var bacc = seed_heap;
    for (args[1..]) |a| {
        const ap = obj.obj_promote_to_bignum(a);
        defer if (ap.owned) bignum.destroy(ap.m);
        if (ap.m == null) {
            bignum.destroy(bacc);
            return obj_new_int(0);
        }
        const new_acc = bignum.alloc_sub(bacc, ap.m.?) orelse {
            bignum.destroy(bacc);
            return obj_new_int(0);
        };
        bignum.destroy(bacc);
        bacc = new_acc;
    }
    return obj.obj_new_bignum_take(bacc);
}

fn op_mul(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(1);
    if (any_float(args)) {
        var prod: f64 = 1;
        for (args) |a| prod *= obj_get_float(a);
        return obj_new_float(prod);
    }
    if (any_bignum(args)) {
        const r = bignum_accumulate(args, .mul, 1) orelse return obj_new_int(0);
        return obj.obj_new_bignum_take(r);
    }
    var prod: i64 = 1;
    var overflowed = false;
    for (args) |a| {
        const r = @mulWithOverflow(prod, obj_get_int(a));
        if (r[1] != 0) {
            overflowed = true;
            break;
        }
        prod = r[0];
    }
    if (!overflowed) return obj_new_int(prod);
    const r = bignum_accumulate(args, .mul, 1) orelse return obj_new_int(0);
    return obj.obj_new_bignum_take(r);
}

fn op_div(args: []const i32) i32 {
    if (args.len == 0) {
        stubs.raise("wrong # args: should be \"/ arg ?arg ...?\"");
        return obj_new_int(0);
    }
    // Unary ``/`` is ALWAYS the floating reciprocal regardless of the
    // operand's source type.  ``mathop.n``: "With one argument, the
    // result is the reciprocal of that value (i.e. 1.0/x)."  An int
    // input still produces a float — ``[/ 5]`` is ``0.2``, not ``0``.
    if (args.len == 1) {
        const v = obj_get_float(args[0]);
        if (v == 0.0) {
            stubs.raise("divide by zero");
            return obj_new_int(0);
        }
        return obj_new_float(1.0 / v);
    }
    if (any_float(args)) {
        var acc: f64 = obj_get_float(args[0]);
        for (args[1..]) |a| {
            const v = obj_get_float(a);
            if (v == 0.0) {
                stubs.raise("divide by zero");
                return obj_new_int(0);
            }
            acc /= v;
        }
        return obj_new_float(acc);
    }
    var acc: i64 = obj_get_int(args[0]);
    for (args[1..]) |a| {
        const v = obj_get_int(a);
        if (v == 0) {
            stubs.raise("divide by zero");
            return obj_new_int(0);
        }
        // ``@divTrunc`` (toward zero) matches ``tcl_arith_div`` and
        // the rest of the runtime.  Earlier ``@divFloor`` produced
        // ``-3`` for ``[/ 5 -2]`` instead of the Tcl-correct ``-2``
        // (Copilot review).
        // Guard the one overflow case: minInt(i64) / -1 = 2^63 exceeds i64.
        if (acc == std.math.minInt(i64) and v == -1) {
            return obj.obj_new_bignum(@divTrunc(@as(i128, acc), @as(i128, v)));
        }
        acc = @divTrunc(acc, v);
    }
    return obj_new_int(acc);
}

fn op_mod(args: []const i32) i32 {
    if (!require_arity(args, "%", 2, 2)) return obj_new_int(0);
    if (any_bignum(args)) {
        const ap = obj.obj_promote_to_bignum(args[0]);
        defer if (ap.owned) bignum.destroy(ap.m);
        const bp = obj.obj_promote_to_bignum(args[1]);
        defer if (bp.owned) bignum.destroy(bp.m);
        if (ap.m == null or bp.m == null) return obj_new_int(0);
        if (bp.m.?.eqlZero()) {
            stubs.raise("divide by zero");
            return obj_new_int(0);
        }
        const r = bignum.alloc_mod_floor(ap.m.?, bp.m.?) orelse return obj_new_int(0);
        return obj.obj_new_bignum_take(r);
    }
    const b = obj_get_int(args[1]);
    if (b == 0) {
        stubs.raise("divide by zero");
        return obj_new_int(0);
    }
    // Tcl-correct mod: result has divisor's sign.  Same fixup
    // ``tcl_arith_mod`` does — earlier ``@rem``-only path returned
    // ``-1`` for ``[% -1 (1<<63)]`` rather than the upstream-correct
    // ``9223372036854775807``.
    var r = @rem(obj_get_int(args[0]), b);
    if (r != 0 and ((r < 0) != (b < 0))) r += b;
    return obj_new_int(r);
}

fn ipow(base: i64, exp_in: i64) i64 {
    // ``ipow`` is only called for non-negative exponents — the
    // ``op_pow`` caller routes negative exponents to the float
    // pathway since integer ``a**-n`` is ``1/(a**n)`` which is
    // fractional and can't be represented in i64.
    var result: i64 = 1;
    var b: i64 = base;
    var e: i64 = exp_in;
    while (e > 0) : (e >>= 1) {
        if ((e & 1) != 0) result *%= b;
        b *%= b;
    }
    return result;
}

fn any_negative_int(args: []const i32) bool {
    for (args) |a| {
        if (!is_float(a) and obj_get_int(a) < 0) return true;
    }
    return false;
}

fn op_pow(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(1);
    if (args.len == 1) {
        if (is_float(args[0])) return obj_new_float(obj_get_float(args[0]));
        return obj_new_int(obj_get_int(args[0]));
    }
    // Negative exponents force the float pathway — integer ``a ** -n``
    // is fractional (``1 / a**n``) and the documented Tcl semantics
    // promote to a float.  ``ipow`` is integer-only, so check the
    // exponents (every arg past the leftmost) before dispatching
    // (Copilot review).
    const has_neg_exp = blk: {
        for (args[1..]) |a| {
            if (!is_float(a) and obj_get_int(a) < 0) break :blk true;
        }
        break :blk false;
    };
    if (any_float(args) or has_neg_exp) {
        // Right-associative: a ** b ** c = a ** (b ** c).
        var acc: f64 = obj_get_float(args[args.len - 1]);
        var i: usize = args.len - 1;
        while (i > 0) {
            i -= 1;
            acc = std.math.pow(f64, obj_get_float(args[i]), acc);
        }
        return obj_new_float(acc);
    }
    // Bignum-aware right-associative chain: a ** b ** c =
    // a ** (b ** c).  Each step promotes through tcl_arith_pow's
    // helper so e.g. ``[** 2 64]`` produces the full bignum.
    if (any_bignum(args)) {
        const tcl_arith = @import("../valtypes/tcl_arith.zig");
        var acc = args[args.len - 1];
        var i: usize = args.len - 1;
        while (i > 0) {
            i -= 1;
            acc = tcl_arith.tcl_arith_pow(args[i], acc);
        }
        return acc;
    }
    // i64 fast path with overflow promotion: if any step overflows,
    // recompute the whole chain in bignum.
    var acc: i64 = obj_get_int(args[args.len - 1]);
    var i: usize = args.len - 1;
    var overflowed = false;
    while (i > 0 and !overflowed) {
        i -= 1;
        const base = obj_get_int(args[i]);
        // Use ipow with overflow detection: walk the loop with
        // @mulWithOverflow rather than ``*%``.
        var result: i64 = 1;
        var b: i64 = base;
        var e: i64 = acc;
        while (e > 0) : (e >>= 1) {
            if ((e & 1) != 0) {
                const m = @mulWithOverflow(result, b);
                if (m[1] != 0) {
                    overflowed = true;
                    break;
                }
                result = m[0];
            }
            if (e > 1) {
                const m = @mulWithOverflow(b, b);
                if (m[1] != 0) {
                    overflowed = true;
                    break;
                }
                b = m[0];
            }
        }
        if (overflowed) break;
        acc = result;
    }
    if (!overflowed) return obj_new_int(acc);
    // Re-run as bignum.
    const tcl_arith = @import("../valtypes/tcl_arith.zig");
    var bacc = args[args.len - 1];
    var bi: usize = args.len - 1;
    while (bi > 0) {
        bi -= 1;
        bacc = tcl_arith.tcl_arith_pow(args[bi], bacc);
    }
    return bacc;
}

// -- bitwise / shift ---------------------------------------------------------

const BitOp = enum { band, bor, bxor };

fn bignum_bitwise_chain(args: []const i32, op: BitOp, init_value: i128) ?*bignum.BigInt {
    var acc = bignum.alloc_from_int(init_value) orelse return null;
    for (args) |a| {
        const ap = obj.obj_promote_to_bignum(a);
        defer if (ap.owned) bignum.destroy(ap.m);
        if (ap.m == null) {
            bignum.destroy(acc);
            return null;
        }
        const new_acc = bignum.alloc_zero() orelse {
            bignum.destroy(acc);
            return null;
        };
        const res = switch (op) {
            .band => new_acc.bitAnd(acc, ap.m.?),
            .bor => new_acc.bitOr(acc, ap.m.?),
            .bxor => new_acc.bitXor(acc, ap.m.?),
        };
        res catch {
            bignum.destroy(new_acc);
            bignum.destroy(acc);
            return null;
        };
        bignum.destroy(acc);
        acc = new_acc;
    }
    return acc;
}

fn op_band(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(-1);
    if (any_bignum(args)) {
        // Identity for AND is all-ones; using -1 as i128 gives -1
        // (all-ones in two's complement) which the bignum AND
        // happily masks to the operand's full magnitude.
        const seed = args[0];
        const ap0 = obj.obj_promote_to_bignum(seed);
        defer if (ap0.owned) bignum.destroy(ap0.m);
        if (ap0.m == null) return obj_new_int(0);
        const initial = ap0.m.?.clone() catch return obj_new_int(0);
        const init_heap = bignum.allocator.create(bignum.BigInt) catch {
            var s = initial;
            s.deinit();
            return obj_new_int(0);
        };
        init_heap.* = initial;
        var acc = init_heap;
        for (args[1..]) |a| {
            const ap = obj.obj_promote_to_bignum(a);
            defer if (ap.owned) bignum.destroy(ap.m);
            if (ap.m == null) {
                bignum.destroy(acc);
                return obj_new_int(0);
            }
            const new_acc = bignum.alloc_zero() orelse {
                bignum.destroy(acc);
                return obj_new_int(0);
            };
            new_acc.bitAnd(acc, ap.m.?) catch {
                bignum.destroy(new_acc);
                bignum.destroy(acc);
                return obj_new_int(0);
            };
            bignum.destroy(acc);
            acc = new_acc;
        }
        return obj.obj_new_bignum_take(acc);
    }
    var acc: i64 = obj_get_int(args[0]);
    for (args[1..]) |a| acc &= obj_get_int(a);
    return obj_new_int(acc);
}

fn op_bor(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(0);
    if (any_bignum(args)) {
        const r = bignum_bitwise_chain(args, .bor, 0) orelse return obj_new_int(0);
        return obj.obj_new_bignum_take(r);
    }
    var acc: i64 = obj_get_int(args[0]);
    for (args[1..]) |a| acc |= obj_get_int(a);
    return obj_new_int(acc);
}

fn op_bxor(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(0);
    if (any_bignum(args)) {
        const r = bignum_bitwise_chain(args, .bxor, 0) orelse return obj_new_int(0);
        return obj.obj_new_bignum_take(r);
    }
    var acc: i64 = obj_get_int(args[0]);
    for (args[1..]) |a| acc ^= obj_get_int(a);
    return obj_new_int(acc);
}

fn op_bnot(args: []const i32) i32 {
    if (!require_arity(args, "~", 1, 1)) return obj_new_int(0);
    if (is_bignum(args[0])) {
        // ``~x`` = ``-x - 1`` in two's complement.  Same identity as
        // ``tcl_arith.zig::tcl_arith_bnot``.
        const ap = obj.obj_promote_to_bignum(args[0]);
        defer if (ap.owned) bignum.destroy(ap.m);
        if (ap.m == null) return obj_new_int(0);
        const neg = bignum.alloc_neg(ap.m.?) orelse return obj_new_int(0);
        defer bignum.destroy(neg);
        const one = bignum.alloc_from_int(1) orelse return obj_new_int(0);
        defer bignum.destroy(one);
        const r = bignum.alloc_sub(neg, one) orelse return obj_new_int(0);
        return obj.obj_new_bignum_take(r);
    }
    return obj_new_int(~obj_get_int(args[0]));
}

fn op_lshift(args: []const i32) i32 {
    if (!require_arity(args, "<<", 2, 2)) return obj_new_int(0);
    // ``<<`` shift count > INT_MAX (or any bignum-shaped count) is
    // rejected with ``integer value too large to represent`` — matches
    // ``tclExecute.c`` INST_LSHIFT (line ~8138).  We can't materialise
    // a 2^31-bit bignum in finite time, so the check has to come
    // before ``bignum.alloc_shl``.
    if (is_bignum(args[1])) {
        if (lshift_count_is_negative_bignum(args[1])) {
            stubs.raise("negative shift argument");
        } else {
            stubs.raise("integer value too large to represent");
        }
        return obj_new_int(0);
    }
    const b = obj_get_int(args[1]);
    if (b < 0) {
        stubs.raise("negative shift argument");
        return obj_new_int(0);
    }
    if (b > std.math.maxInt(i32)) {
        stubs.raise("integer value too large to represent");
        return obj_new_int(0);
    }
    if (is_bignum(args[0]) or b >= 63) {
        const ap = obj.obj_promote_to_bignum(args[0]);
        defer if (ap.owned) bignum.destroy(ap.m);
        if (ap.m == null) return obj_new_int(0);
        if (ap.m.?.eqlZero()) return obj_new_int(0);
        const count: u64 = @bitCast(b);
        const r = bignum.alloc_shl(ap.m.?, count) orelse return obj_new_int(0);
        return obj.obj_new_bignum_take(r);
    }
    const a = obj_get_int(args[0]);
    const sh: u6 = @intCast(b);
    const widened = @as(i128, a) << sh;
    if (widened >= std.math.minInt(i64) and widened <= std.math.maxInt(i64)) {
        return obj_new_int(@intCast(widened));
    }
    return obj.obj_new_bignum(widened);
}

fn op_rshift(args: []const i32) i32 {
    if (!require_arity(args, ">>", 2, 2)) return obj_new_int(0);
    // ``>>`` mirrors INST_RSHIFT (tclExecute.c ~8169): a shift count
    // larger than INT_MAX (or a bignum count) collapses to 0 / -1
    // depending on the sign of the operand — never feed an absurd
    // count to ``Managed.shiftRight``.
    if (is_bignum(args[1])) {
        if (lshift_count_is_negative_bignum(args[1])) {
            stubs.raise("negative shift argument");
            return obj_new_int(0);
        }
        return rshift_force_sign(args[0]);
    }
    const b = obj_get_int(args[1]);
    if (b < 0) {
        stubs.raise("negative shift argument");
        return obj_new_int(0);
    }
    if (b > std.math.maxInt(i32)) {
        return rshift_force_sign(args[0]);
    }
    if (is_bignum(args[0])) {
        const ap = obj.obj_promote_to_bignum(args[0]);
        defer if (ap.owned) bignum.destroy(ap.m);
        if (ap.m == null) return obj_new_int(0);
        const r = bignum.alloc_zero() orelse return obj_new_int(0);
        const count: u64 = @bitCast(b);
        // On wasm32, usize == u32; guard @intCast for large shift amounts —
        // no bignum can have > 2^32 bits, so the result is trivially 0/-1.
        if (count > std.math.maxInt(usize)) {
            bignum.destroy(r);
            return obj_new_int(if (ap.m.?.isPositive()) 0 else -1);
        }
        r.shiftRight(ap.m.?, @intCast(count)) catch {
            bignum.destroy(r);
            return obj_new_int(0);
        };
        return obj.obj_new_bignum_take(r);
    }
    const a = obj_get_int(args[0]);
    if (b >= 64) return obj_new_int(if (a < 0) -1 else 0);
    const sh: u6 = @intCast(b);
    return obj_new_int(a >> sh);
}

/// Returns true if *o* parses as a bignum whose value is negative.
/// Used by the shift operators to distinguish ``negative shift
/// argument`` from ``integer value too large to represent`` when the
/// shift count is too large for an i64.
fn lshift_count_is_negative_bignum(o: i32) bool {
    const ap = obj.obj_promote_to_bignum(o);
    defer if (ap.owned) bignum.destroy(ap.m);
    if (ap.m == null) return false;
    return !ap.m.?.isPositive() and !ap.m.?.eqlZero();
}

/// Right-shift by an amount that's beyond what ``mp_div_2d`` can
/// represent: result is 0 for non-negative operand, -1 for negative.
fn rshift_force_sign(o: i32) i32 {
    if (is_bignum(o)) {
        const ap = obj.obj_promote_to_bignum(o);
        defer if (ap.owned) bignum.destroy(ap.m);
        if (ap.m == null) return obj_new_int(0);
        return obj_new_int(if (ap.m.?.isPositive()) 0 else -1);
    }
    const v = obj_get_int(o);
    return obj_new_int(if (v < 0) -1 else 0);
}

// -- numeric comparison (chain) ----------------------------------------------

const CmpKind = enum { eq, ne, lt, le, gt, ge };

/// Inspect *o*'s string representation: returns whether the value is
/// usable as a number (TYPE_INT / TYPE_FLOAT, or TYPE_STRING that
/// parses cleanly through ``try_parse_int`` / ``try_parse_float``).
/// Used by the comparison ops to decide whether to compare
/// numerically or fall back to lexical-string semantics.
fn is_numeric(o: i32) bool {
    if (o == 0) return true; // null / empty obj — defaults to 0 numerically
    const tag = obj.obj_type(o);
    if (tag == obj.TYPE_INT or tag == obj.TYPE_FLOAT or tag == obj.TYPE_BIGNUM) return true;
    const s = obj_ensure_str(o);
    if (s.len == 0) return true;
    if (obj.try_parse_int(s.ptr, s.len) != null) return true;
    if (obj.try_parse_float(s.ptr, s.len) != null) return true;
    // String literal that exceeds i64 → still numeric for ``mathop``
    // dispatch.  Without this branch ``[< 99 (1<<200)]`` falls to
    // bytewise compare and returns the lexicographic answer.
    if (bignum.parse_i128(s.ptr, s.len) != null) return true;
    const m = bignum.alloc_from_string(s.ptr, s.len) orelse return false;
    bignum.destroy(m);
    return true;
}

/// Lexical-string comparison for the ``a < b`` family.  Returns
/// ``-1`` / ``0`` / ``1`` per ``memcmp`` semantics with shorter-
/// string-first tie-breaking.
fn str_cmp_lex(a: i32, b: i32) i32 {
    const sa = obj_ensure_str(a);
    const sb = obj_ensure_str(b);
    const pa: [*]const u8 = if (sa.ptr == 0) undefined else @ptrFromInt(sa.ptr);
    const pb: [*]const u8 = if (sb.ptr == 0) undefined else @ptrFromInt(sb.ptr);
    var i: u32 = 0;
    while (i < sa.len and i < sb.len) : (i += 1) {
        if (pa[i] != pb[i]) return if (pa[i] < pb[i]) -1 else 1;
    }
    if (sa.len < sb.len) return -1;
    if (sa.len > sb.len) return 1;
    return 0;
}

fn cmp_pair_num(a: i32, b: i32, k: CmpKind) bool {
    // Tcl's mathop comparison ops use *numeric* semantics when both
    // operands are numbers (or parse cleanly as numbers), otherwise
    // fall back to *lexical-string* comparison — ``::tcl::mathop::==
    // a b`` is false (string ``"a" != "b"``), not true (collapsed to
    // the integer-coerce of ``0 == 0``).  Codex P1 review caught the
    // missing fallback; ``tclMathOp.c::ChainedRelOpCmd`` does the
    // same numeric-vs-string dispatch.
    if (!is_numeric(a) or !is_numeric(b)) {
        const c = str_cmp_lex(a, b);
        return switch (k) {
            .eq => c == 0,
            .ne => c != 0,
            .lt => c < 0,
            .le => c <= 0,
            .gt => c > 0,
            .ge => c >= 0,
        };
    }
    if (is_float(a) or is_float(b)) {
        const af = obj_get_float(a);
        const bf = obj_get_float(b);
        return switch (k) {
            .eq => af == bf,
            .ne => af != bf,
            .lt => af < bf,
            .le => af <= bf,
            .gt => af > bf,
            .ge => af >= bf,
        };
    }
    // Bignum-aware integer compare.  Stage 1 truncated bignum
    // operands via ``obj_get_int``, miscomparing e.g.
    // ``[< 99 (1<<70)]`` as false (both truncated to 99 vs ~i64::MAX).
    // Routing through ``Managed.order`` gives the correct answer for
    // arbitrary magnitude.
    if (is_bignum(a) or is_bignum(b)) {
        const ap = obj.obj_promote_to_bignum(a);
        defer if (ap.owned) bignum.destroy(ap.m);
        const bp = obj.obj_promote_to_bignum(b);
        defer if (bp.owned) bignum.destroy(bp.m);
        if (ap.m == null or bp.m == null) return false;
        const ord = ap.m.?.order(bp.m.?.*);
        return switch (k) {
            .eq => ord == .eq,
            .ne => ord != .eq,
            .lt => ord == .lt,
            .le => ord != .gt,
            .gt => ord == .gt,
            .ge => ord != .lt,
        };
    }
    const ai = obj_get_int(a);
    const bi = obj_get_int(b);
    return switch (k) {
        .eq => ai == bi,
        .ne => ai != bi,
        .lt => ai < bi,
        .le => ai <= bi,
        .gt => ai > bi,
        .ge => ai >= bi,
    };
}

fn op_chain_num(args: []const i32, k: CmpKind) i32 {
    // Tcl 9 behaviour:
    //   0 / 1 args → 1 (vacuously true)
    //   N args (N ≥ 2) → 1 iff every consecutive pair holds.
    // ``!=`` is the documented exception — it's strictly binary (the
    // chain form would be ambiguous for ``[!= 1 2 1]``: pairwise true
    // but the set isn't all-distinct).  ``mathop.n``: "!= a b — exactly
    // two args".  We enforce that in :func:`op_neq`.
    if (args.len < 2) return obj_new_int(1);
    var i: usize = 0;
    while (i + 1 < args.len) : (i += 1) {
        if (!cmp_pair_num(args[i], args[i + 1], k)) return obj_new_int(0);
    }
    return obj_new_int(1);
}

fn op_eq_num(args: []const i32) i32 {
    return op_chain_num(args, .eq);
}
fn op_ne_num(args: []const i32) i32 {
    if (!require_arity(args, "!=", 2, 2)) return obj_new_int(0);
    return obj_new_int(if (cmp_pair_num(args[0], args[1], .ne)) 1 else 0);
}
fn op_lt_num(args: []const i32) i32 {
    return op_chain_num(args, .lt);
}
fn op_le_num(args: []const i32) i32 {
    return op_chain_num(args, .le);
}
fn op_gt_num(args: []const i32) i32 {
    return op_chain_num(args, .gt);
}
fn op_ge_num(args: []const i32) i32 {
    return op_chain_num(args, .ge);
}

// -- string compare ----------------------------------------------------------

fn slice_eq(a: i32, b: i32) bool {
    const sa = obj_ensure_str(a);
    const sb = obj_ensure_str(b);
    if (sa.len != sb.len) return false;
    if (sa.len == 0) return true;
    const pa: [*]const u8 = @ptrFromInt(sa.ptr);
    const pb: [*]const u8 = @ptrFromInt(sb.ptr);
    for (0..sa.len) |i| {
        if (pa[i] != pb[i]) return false;
    }
    return true;
}

// Returns negative/zero/positive like C strcmp.
fn slice_cmp(a: i32, b: i32) i32 {
    const sa = obj_ensure_str(a);
    const sb = obj_ensure_str(b);
    const pa: [*]const u8 = if (sa.ptr != 0) @ptrFromInt(sa.ptr) else &[_]u8{};
    const pb: [*]const u8 = if (sb.ptr != 0) @ptrFromInt(sb.ptr) else &[_]u8{};
    const min_len = @min(sa.len, sb.len);
    for (0..min_len) |i| {
        if (pa[i] < pb[i]) return -1;
        if (pa[i] > pb[i]) return 1;
    }
    if (sa.len < sb.len) return -1;
    if (sa.len > sb.len) return 1;
    return 0;
}

const StrCmpKind = enum { lt, le, gt, ge };

fn op_chain_str(args: []const i32, kind: StrCmpKind) i32 {
    if (args.len <= 1) return obj_new_int(1);
    var i: usize = 0;
    while (i + 1 < args.len) : (i += 1) {
        const c = slice_cmp(args[i], args[i + 1]);
        const ok = switch (kind) {
            .lt => c < 0,
            .le => c <= 0,
            .gt => c > 0,
            .ge => c >= 0,
        };
        if (!ok) return obj_new_int(0);
    }
    return obj_new_int(1);
}

fn op_eq_str(args: []const i32) i32 {
    // Variadic chain: all-equal-as-strings.
    if (args.len < 2) return obj_new_int(1);
    var i: usize = 0;
    while (i + 1 < args.len) : (i += 1) {
        if (!slice_eq(args[i], args[i + 1])) return obj_new_int(0);
    }
    return obj_new_int(1);
}

fn op_ne_str(args: []const i32) i32 {
    if (!require_arity(args, "ne", 2, 2)) return obj_new_int(0);
    return obj_new_int(if (slice_eq(args[0], args[1])) 0 else 1);
}

// -- list membership (in / ni) ----------------------------------------------

fn op_in(args: []const i32) i32 {
    if (!require_arity(args, "in", 2, 2)) return obj_new_int(0);
    return list.tcl_cmd_list_contains(args[1], args[0]);
}

fn op_ni(args: []const i32) i32 {
    if (!require_arity(args, "ni", 2, 2)) return obj_new_int(0);
    return obj_new_int(if (obj_get_int(list.tcl_cmd_list_contains(args[1], args[0])) == 0) 1 else 0);
}

// -- logical -----------------------------------------------------------------

fn truthy(o: i32) bool {
    if (o == 0) return false;
    const tag = obj.obj_type(o);
    // Try the boolean-keyword path FIRST for string-shaped objs.
    // ``is_float`` (used downstream by the numeric branch) classifies
    // any string containing ``e`` / ``E`` as float, which would
    // collapse ``"true"`` to ``obj_get_float() == 0.0`` and return
    // false.  Boolean keywords have to win before the float heuristic
    // sees them (Copilot review).
    if (tag == TYPE_STRING or tag == TYPE_INLINE_STRING) {
        const s = obj_ensure_str(o);
        if (s.len == 0) return false;
        if (obj.try_parse_bool(s.ptr, s.len)) |v| return v != 0;
    }
    if (is_float(o)) return obj_get_float(o) != 0.0;
    return obj_get_int(o) != 0;
}

fn op_not(args: []const i32) i32 {
    if (!require_arity(args, "!", 1, 1)) return obj_new_int(0);
    return obj_new_int(if (truthy(args[0])) 0 else 1);
}

fn op_and(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(1);
    for (args) |a| {
        if (!truthy(a)) return obj_new_int(0);
    }
    return obj_new_int(1);
}

fn op_or(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(0);
    for (args) |a| {
        if (truthy(a)) return obj_new_int(1);
    }
    return obj_new_int(0);
}

// -- min / max ---------------------------------------------------------------

fn op_min(args: []const i32) i32 {
    if (args.len == 0) {
        stubs.raise("wrong # args: should be \"min arg ?arg ...?\"");
        return obj_new_int(0);
    }
    if (any_float(args)) {
        var best: f64 = obj_get_float(args[0]);
        for (args[1..]) |a| {
            const v = obj_get_float(a);
            if (v < best) best = v;
        }
        return obj_new_float(best);
    }
    var best: i64 = obj_get_int(args[0]);
    for (args[1..]) |a| {
        const v = obj_get_int(a);
        if (v < best) best = v;
    }
    return obj_new_int(best);
}

fn op_max(args: []const i32) i32 {
    if (args.len == 0) {
        stubs.raise("wrong # args: should be \"max arg ?arg ...?\"");
        return obj_new_int(0);
    }
    if (any_float(args)) {
        var best: f64 = obj_get_float(args[0]);
        for (args[1..]) |a| {
            const v = obj_get_float(a);
            if (v > best) best = v;
        }
        return obj_new_float(best);
    }
    var best: i64 = obj_get_int(args[0]);
    for (args[1..]) |a| {
        const v = obj_get_int(a);
        if (v > best) best = v;
    }
    return obj_new_int(best);
}

// -- list index ``@`` --------------------------------------------------------

fn op_at(args: []const i32) i32 {
    if (!require_arity(args, "@", 2, 2)) return obj_new_int(0);
    return list.tcl_cmd_list_index(args[1], @intCast(obj_get_int(args[0])));
}

// -- dispatcher --------------------------------------------------------------

/// Single entry point for every registered mathop spelling — examines
/// the trailing operator name (``foo::bar::==`` → ``==``) and dispatches
/// to the per-op handler.  ``words[1..]`` is the operand list.
fn eval(words: []const i32) result_mod.InterpResult {
    const op = op_name(words);
    const rest = words[1..];
    // Stable order: arithmetic > comparison > bitwise > logical >
    // misc.  Linear scan is fine — there are ~25 ops and the
    // frequently-hit ones (``==``, ``+``, ``-``) are checked first.
    if (std.mem.eql(u8, op, "+")) return result_mod.from_globals(op_add(rest));
    if (std.mem.eql(u8, op, "-")) return result_mod.from_globals(op_sub(rest));
    if (std.mem.eql(u8, op, "*")) return result_mod.from_globals(op_mul(rest));
    if (std.mem.eql(u8, op, "/")) return result_mod.from_globals(op_div(rest));
    if (std.mem.eql(u8, op, "%")) return result_mod.from_globals(op_mod(rest));
    if (std.mem.eql(u8, op, "**")) return result_mod.from_globals(op_pow(rest));
    if (std.mem.eql(u8, op, "==")) return result_mod.from_globals(op_eq_num(rest));
    if (std.mem.eql(u8, op, "!=")) return result_mod.from_globals(op_ne_num(rest));
    if (std.mem.eql(u8, op, "<")) return result_mod.from_globals(op_lt_num(rest));
    if (std.mem.eql(u8, op, ">")) return result_mod.from_globals(op_gt_num(rest));
    if (std.mem.eql(u8, op, "<=")) return result_mod.from_globals(op_le_num(rest));
    if (std.mem.eql(u8, op, ">=")) return result_mod.from_globals(op_ge_num(rest));
    if (std.mem.eql(u8, op, "eq")) return result_mod.from_globals(op_eq_str(rest));
    if (std.mem.eql(u8, op, "ne")) return result_mod.from_globals(op_ne_str(rest));
    if (std.mem.eql(u8, op, "lt")) return result_mod.from_globals(op_chain_str(rest, .lt));
    if (std.mem.eql(u8, op, "le")) return result_mod.from_globals(op_chain_str(rest, .le));
    if (std.mem.eql(u8, op, "gt")) return result_mod.from_globals(op_chain_str(rest, .gt));
    if (std.mem.eql(u8, op, "ge")) return result_mod.from_globals(op_chain_str(rest, .ge));
    if (std.mem.eql(u8, op, "in")) return result_mod.from_globals(op_in(rest));
    if (std.mem.eql(u8, op, "ni")) return result_mod.from_globals(op_ni(rest));
    if (std.mem.eql(u8, op, "&")) return result_mod.from_globals(op_band(rest));
    if (std.mem.eql(u8, op, "|")) return result_mod.from_globals(op_bor(rest));
    if (std.mem.eql(u8, op, "^")) return result_mod.from_globals(op_bxor(rest));
    if (std.mem.eql(u8, op, "~")) return result_mod.from_globals(op_bnot(rest));
    if (std.mem.eql(u8, op, "<<")) return result_mod.from_globals(op_lshift(rest));
    if (std.mem.eql(u8, op, ">>")) return result_mod.from_globals(op_rshift(rest));
    if (std.mem.eql(u8, op, "!")) return result_mod.from_globals(op_not(rest));
    if (std.mem.eql(u8, op, "&&")) return result_mod.from_globals(op_and(rest));
    if (std.mem.eql(u8, op, "||")) return result_mod.from_globals(op_or(rest));
    if (std.mem.eql(u8, op, "min")) return result_mod.from_globals(op_min(rest));
    if (std.mem.eql(u8, op, "max")) return result_mod.from_globals(op_max(rest));
    if (std.mem.eql(u8, op, "@")) return result_mod.from_globals(op_at(rest));
    return result_mod.from_globals(obj_new_int(0));
}

// Each operator gets two registered spellings: the fully-qualified
// ``::tcl::mathop::OP`` form (what tcltest typically writes) and the
// half-qualified ``tcl::mathop::OP`` form (used inside
// ``namespace eval ::tcl`` blocks).  The bare ``+`` / ``==`` / etc.
// names are *not* registered here — they're handled by the
// expression compiler at codegen time, and registering them as
// commands would shadow ``[catch {puts -}]`` style inputs.
pub const registrations = [_]reg.CmdEntry{
    .{ .name = "::tcl::mathop::+", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::-", .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::*", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::/", .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::%", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "::tcl::mathop::**", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::==", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::!=", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "::tcl::mathop::<", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::>", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::<=", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::>=", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::eq", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::ne", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "::tcl::mathop::lt", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::le", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::gt", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::ge", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::in", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "::tcl::mathop::ni", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "::tcl::mathop::&", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::|", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::^", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::~", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "::tcl::mathop::<<", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "::tcl::mathop::>>", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "::tcl::mathop::!", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "::tcl::mathop::&&", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::||", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::min", .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::max", .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::@", .arity_min = 2, .arity_max = 2, .handler = &eval },
    // Half-qualified spellings (inside ``namespace eval ::tcl``).
    .{ .name = "tcl::mathop::+", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::-", .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::*", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::/", .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::%", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "tcl::mathop::**", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::==", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::!=", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "tcl::mathop::<", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::>", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::<=", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::>=", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::eq", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::ne", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "tcl::mathop::lt", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::le", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::gt", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::ge", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::in", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "tcl::mathop::ni", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "tcl::mathop::&", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::|", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::^", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::~", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "tcl::mathop::<<", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "tcl::mathop::>>", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "tcl::mathop::!", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "tcl::mathop::&&", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::||", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::min", .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::max", .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::@", .arity_min = 2, .arity_max = 2, .handler = &eval },
};
