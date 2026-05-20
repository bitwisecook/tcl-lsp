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
const tcl_arith = @import("../valtypes/tcl_arith.zig");
const tcl_expr_eval = @import("../interp/tcl_expr_eval.zig");

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
    // The exact ``should be "<op> ..."`` operand list isn't checked by
    // the test suite — only the ``wrong # args: should be * TCL
    // WRONGARGS`` glob (mathop-20.2 / 20.5 / 21.6 / 24.8).  The
    // ``wrong # args:`` prefix drives ``detect_error_code`` to stamp
    // ``TCL WRONGARGS``.
    if (args.len < min) {
        stubs.raise("wrong # args: should be \"" ++ opname ++ " ...\"");
        return false;
    }
    if (max) |m| {
        if (args.len > m) {
            stubs.raise("wrong # args: should be \"" ++ opname ++ " ...\"");
            return false;
        }
    }
    return true;
}

// Fold helpers for the arithmetic / bitwise operators.  They delegate
// each pairwise step to the validated ``tcl_arith_*`` binary ops so
// the command form inherits the expr path's operand-domain checks,
// bignum precision, and error wording instead of re-implementing them.
const BinFn = *const fn (i32, i32) callconv(.c) i32;

fn err_pending() bool {
    return result_mod.snapshot(0).code == .ERROR;
}

/// Left-fold *args* through *f*, seeding with *identity_val* so a
/// single-argument call still validates (and normalises) its operand
/// and so ``args[0]`` is the left operand of the first real pairwise
/// step (correct ``left``/``right`` error wording).  Bails on the
/// first raised error.  Returns a fresh +1-owned result.
fn fold_left(args: []const i32, f: BinFn, identity_val: i64) i32 {
    if (args.len == 0) return obj_new_int(identity_val);
    const id = obj_new_int(identity_val);
    var acc = f(args[0], id);
    obj.tcl_obj_release(id);
    if (err_pending()) return acc;
    var i: usize = 1;
    while (i < args.len) : (i += 1) {
        const next = f(acc, args[i]);
        obj.tcl_obj_release(acc);
        acc = next;
        if (err_pending()) break;
    }
    return acc;
}

/// Left-fold without an identity seed — used by ``-`` / ``/`` where
/// the multi-arg form is ``a - b - c`` / ``a / b / c`` and there is
/// no useful identity to prepend.  Caller guarantees ``args.len >= 2``.
fn fold_noseed(args: []const i32, f: BinFn) i32 {
    var acc = f(args[0], args[1]);
    if (err_pending()) return acc;
    var i: usize = 2;
    while (i < args.len) : (i += 1) {
        const next = f(acc, args[i]);
        obj.tcl_obj_release(acc);
        acc = next;
        if (err_pending()) break;
    }
    return acc;
}

// -- arithmetic --------------------------------------------------------------
//
// The variadic operators fold over the validated ``tcl_arith_*``
// binary ops (the same ones the expression compiler targets) so the
// command form inherits operand-domain checks, bignum precision, and
// the exact Tcl error wording instead of re-implementing them.

fn op_add(args: []const i32) i32 {
    return fold_left(args, tcl_arith.tcl_arith_add, 0);
}

fn op_sub(args: []const i32) i32 {
    if (args.len == 0) {
        stubs.raise("wrong # args: should be \"- arg ?arg ...?\"");
        return obj_new_int(0);
    }
    if (args.len == 1) return tcl_arith.tcl_arith_neg(args[0]);
    return fold_noseed(args, tcl_arith.tcl_arith_sub);
}

fn op_mul(args: []const i32) i32 {
    return fold_left(args, tcl_arith.tcl_arith_mul, 1);
}

fn op_div(args: []const i32) i32 {
    if (args.len == 0) {
        stubs.raise("wrong # args: should be \"/ arg ?arg ...?\"");
        return obj_new_int(0);
    }
    // Unary ``/`` is the floating reciprocal ``1.0 / x`` regardless of
    // the operand type (``[/ 5]`` is ``0.2``).  Routing through
    // ``tcl_arith_div(1.0, x)`` reuses the operand validation so a
    // non-numeric ``x`` reports ``... as right operand of "/"``.
    if (args.len == 1) {
        const one = obj_new_float(1.0);
        const r = tcl_arith.tcl_arith_div(one, args[0]);
        obj.tcl_obj_release(one);
        return r;
    }
    return fold_noseed(args, tcl_arith.tcl_arith_div);
}

fn op_mod(args: []const i32) i32 {
    if (!require_arity(args, "%", 2, 2)) return obj_new_int(0);
    return tcl_arith.tcl_arith_mod(args[0], args[1]);
}

fn op_pow(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(1);
    if (args.len == 1) {
        // ``** x`` is ``x`` (validated numeric); ``tcl_arith_pow(x, 1)``
        // returns the operand retained after the numeric check.
        const one = obj_new_int(1);
        const r = tcl_arith.tcl_arith_pow(args[0], one);
        obj.tcl_obj_release(one);
        return r;
    }
    // ``**`` is right-associative: ``a ** b ** c`` = ``a ** (b ** c)``.
    // Each step delegates to ``tcl_arith_pow`` (integer / float /
    // bignum / negative-exponent / domain-error semantics).
    var acc = args[args.len - 1];
    obj.tcl_obj_retain(acc);
    var i: usize = args.len - 1;
    while (i > 0) {
        i -= 1;
        const next = tcl_arith.tcl_arith_pow(args[i], acc);
        obj.tcl_obj_release(acc);
        acc = next;
        if (err_pending()) break;
    }
    return acc;
}

// -- bitwise / shift ---------------------------------------------------------
//
// All four delegate to the validated ``tcl_arith_*`` helpers, which
// enforce the integer-operand domain (rejecting floats / non-numeric
// strings with the canonical wording), carry bignum precision, and
// handle the large- / negative-shift edge cases.

fn op_band(args: []const i32) i32 {
    return fold_left(args, tcl_arith.tcl_arith_band, -1);
}

fn op_bor(args: []const i32) i32 {
    return fold_left(args, tcl_arith.tcl_arith_bor, 0);
}

fn op_bxor(args: []const i32) i32 {
    return fold_left(args, tcl_arith.tcl_arith_bxor, 0);
}

fn op_bnot(args: []const i32) i32 {
    if (!require_arity(args, "~", 1, 1)) return obj_new_int(0);
    return tcl_arith.tcl_arith_bnot(args[0]);
}

fn op_lshift(args: []const i32) i32 {
    if (!require_arity(args, "<<", 2, 2)) return obj_new_int(0);
    return tcl_arith.tcl_arith_lshift(args[0], args[1]);
}

fn op_rshift(args: []const i32) i32 {
    if (!require_arity(args, ">>", 2, 2)) return obj_new_int(0);
    return tcl_arith.tcl_arith_rshift(args[0], args[1]);
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

/// Validate that *o* is a well-formed list before ``in`` / ``ni``
/// membership testing.  An unbalanced brace raises ``unmatched open
/// brace in list`` (errorCode ``TCL VALUE LIST BRACE`` via
/// ``detect_error_code``) rather than silently treating the malformed
/// string as an empty / partial list (mathop-24.3).
fn validate_membership_list(o: i32) bool {
    const lp = @import("../valtypes/tcl_list_parse.zig");
    const s = obj_ensure_str(o);
    if (s.ptr == 0 or s.len == 0) return true;
    if (!lp.validate_list_braces(s.ptr, s.len)) {
        stubs.raise("unmatched open brace in list");
        return false;
    }
    return true;
}

fn op_in(args: []const i32) i32 {
    if (!require_arity(args, "in", 2, 2)) return obj_new_int(0);
    if (!validate_membership_list(args[1])) return obj_new_int(0);
    return list.tcl_cmd_list_contains(args[1], args[0]);
}

fn op_ni(args: []const i32) i32 {
    if (!require_arity(args, "ni", 2, 2)) return obj_new_int(0);
    if (!validate_membership_list(args[1])) return obj_new_int(0);
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
    // Delegate to the expr logical-NOT so a non-numeric / non-boolean
    // operand raises ``cannot use non-numeric string "x" as operand of
    // "!"`` (mathop-21.5) instead of silently coercing to false.
    return tcl_expr_eval.tcl_expr_lnot(args[0]);
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
