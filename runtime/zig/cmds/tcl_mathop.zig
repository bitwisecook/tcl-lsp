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

const std    = @import("std");
const obj    = @import("../valtypes/tcl_obj.zig");
const reg    = @import("../dispatch/tcl_cmd_registry.zig");
const stubs  = @import("../stubs/tcl_stubs.zig");
const list   = @import("../valtypes/tcl_list.zig");

const obj_new_int    = obj.obj_new_int;
const obj_new_float  = obj.obj_new_float;
const obj_get_int    = obj.obj_get_int;
const obj_get_float  = obj.obj_get_float;
const obj_ensure_str = obj.obj_ensure_string;
const TYPE_FLOAT     = obj.TYPE_FLOAT;
const TYPE_STRING    = obj.TYPE_STRING;
const TYPE_INLINE_STRING = obj.TYPE_INLINE_STRING;

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

fn op_add(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(0);
    if (any_float(args)) {
        var sum: f64 = 0;
        for (args) |a| sum += obj_get_float(a);
        return obj_new_float(sum);
    }
    var sum: i64 = 0;
    for (args) |a| sum +%= obj_get_int(a);
    return obj_new_int(sum);
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
    if (args.len == 1) return obj_new_int(-%obj_get_int(args[0]));
    var acc: i64 = obj_get_int(args[0]);
    for (args[1..]) |a| acc -%= obj_get_int(a);
    return obj_new_int(acc);
}

fn op_mul(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(1);
    if (any_float(args)) {
        var prod: f64 = 1;
        for (args) |a| prod *= obj_get_float(a);
        return obj_new_float(prod);
    }
    var prod: i64 = 1;
    for (args) |a| prod *%= obj_get_int(a);
    return obj_new_int(prod);
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
        acc = @divFloor(acc, v);
    }
    return obj_new_int(acc);
}

fn op_mod(args: []const i32) i32 {
    if (!require_arity(args, "%", 2, 2)) return obj_new_int(0);
    const b = obj_get_int(args[1]);
    if (b == 0) {
        stubs.raise("divide by zero");
        return obj_new_int(0);
    }
    return obj_new_int(@mod(obj_get_int(args[0]), b));
}

fn ipow(base: i64, exp_in: i64) i64 {
    if (exp_in < 0) return 0;
    var result: i64 = 1;
    var b: i64 = base;
    var e: i64 = exp_in;
    while (e > 0) : (e >>= 1) {
        if ((e & 1) != 0) result *%= b;
        b *%= b;
    }
    return result;
}

fn op_pow(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(1);
    if (args.len == 1) {
        if (is_float(args[0])) return obj_new_float(obj_get_float(args[0]));
        return obj_new_int(obj_get_int(args[0]));
    }
    if (any_float(args)) {
        // Right-associative: a ** b ** c = a ** (b ** c).
        var acc: f64 = obj_get_float(args[args.len - 1]);
        var i: usize = args.len - 1;
        while (i > 0) {
            i -= 1;
            acc = std.math.pow(f64, obj_get_float(args[i]), acc);
        }
        return obj_new_float(acc);
    }
    var acc: i64 = obj_get_int(args[args.len - 1]);
    var i: usize = args.len - 1;
    while (i > 0) {
        i -= 1;
        acc = ipow(obj_get_int(args[i]), acc);
    }
    return obj_new_int(acc);
}

// -- bitwise / shift ---------------------------------------------------------

fn op_band(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(-1);
    var acc: i64 = obj_get_int(args[0]);
    for (args[1..]) |a| acc &= obj_get_int(a);
    return obj_new_int(acc);
}

fn op_bor(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(0);
    var acc: i64 = obj_get_int(args[0]);
    for (args[1..]) |a| acc |= obj_get_int(a);
    return obj_new_int(acc);
}

fn op_bxor(args: []const i32) i32 {
    if (args.len == 0) return obj_new_int(0);
    var acc: i64 = obj_get_int(args[0]);
    for (args[1..]) |a| acc ^= obj_get_int(a);
    return obj_new_int(acc);
}

fn op_bnot(args: []const i32) i32 {
    if (!require_arity(args, "~", 1, 1)) return obj_new_int(0);
    return obj_new_int(~obj_get_int(args[0]));
}

fn op_lshift(args: []const i32) i32 {
    if (!require_arity(args, "<<", 2, 2)) return obj_new_int(0);
    const a = obj_get_int(args[0]);
    const b = obj_get_int(args[1]);
    if (b < 0) {
        stubs.raise("negative shift argument");
        return obj_new_int(0);
    }
    if (b >= 64) return obj_new_int(0);
    const sh: u6 = @intCast(b);
    return obj_new_int(a << sh);
}

fn op_rshift(args: []const i32) i32 {
    if (!require_arity(args, ">>", 2, 2)) return obj_new_int(0);
    const a = obj_get_int(args[0]);
    const b = obj_get_int(args[1]);
    if (b < 0) {
        stubs.raise("negative shift argument");
        return obj_new_int(0);
    }
    if (b >= 64) return obj_new_int(if (a < 0) -1 else 0);
    const sh: u6 = @intCast(b);
    return obj_new_int(a >> sh);
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
    if (tag == obj.TYPE_INT or tag == obj.TYPE_FLOAT) return true;
    const s = obj_ensure_str(o);
    if (s.len == 0) return true;
    if (obj.try_parse_int(s.ptr, s.len) != null) return true;
    if (obj.try_parse_float(s.ptr, s.len) != null) return true;
    return false;
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

fn op_eq_num(args: []const i32) i32 { return op_chain_num(args, .eq); }
fn op_ne_num(args: []const i32) i32 {
    if (!require_arity(args, "!=", 2, 2)) return obj_new_int(0);
    return obj_new_int(if (cmp_pair_num(args[0], args[1], .ne)) 1 else 0);
}
fn op_lt_num(args: []const i32) i32 { return op_chain_num(args, .lt); }
fn op_le_num(args: []const i32) i32 { return op_chain_num(args, .le); }
fn op_gt_num(args: []const i32) i32 { return op_chain_num(args, .gt); }
fn op_ge_num(args: []const i32) i32 { return op_chain_num(args, .ge); }

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
    if (is_float(o)) return obj_get_float(o) != 0.0;
    const tag = obj.obj_type(o);
    if (tag == TYPE_STRING or tag == TYPE_INLINE_STRING) {
        const s = obj_ensure_str(o);
        if (s.len == 0) return false;
        const p: [*]const u8 = @ptrFromInt(s.ptr);
        // ``true`` / ``false`` / ``yes`` / ``no`` accepted (case-folded).
        if (s.len == 4 and (p[0] == 't' or p[0] == 'T') and (p[1] == 'r' or p[1] == 'R')) return true;
        if (s.len == 5 and (p[0] == 'f' or p[0] == 'F')) return false;
        if (s.len == 3 and (p[0] == 'y' or p[0] == 'Y')) return true;
        if (s.len == 2 and (p[0] == 'n' or p[0] == 'N') and (p[1] == 'o' or p[1] == 'O')) return false;
    }
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
fn eval(words: []const i32) i32 {
    const op = op_name(words);
    const rest = words[1..];
    // Stable order: arithmetic > comparison > bitwise > logical >
    // misc.  Linear scan is fine — there are ~25 ops and the
    // frequently-hit ones (``==``, ``+``, ``-``) are checked first.
    if (std.mem.eql(u8, op, "+")) return op_add(rest);
    if (std.mem.eql(u8, op, "-")) return op_sub(rest);
    if (std.mem.eql(u8, op, "*")) return op_mul(rest);
    if (std.mem.eql(u8, op, "/")) return op_div(rest);
    if (std.mem.eql(u8, op, "%")) return op_mod(rest);
    if (std.mem.eql(u8, op, "**")) return op_pow(rest);
    if (std.mem.eql(u8, op, "==")) return op_eq_num(rest);
    if (std.mem.eql(u8, op, "!=")) return op_ne_num(rest);
    if (std.mem.eql(u8, op, "<")) return op_lt_num(rest);
    if (std.mem.eql(u8, op, ">")) return op_gt_num(rest);
    if (std.mem.eql(u8, op, "<=")) return op_le_num(rest);
    if (std.mem.eql(u8, op, ">=")) return op_ge_num(rest);
    if (std.mem.eql(u8, op, "eq")) return op_eq_str(rest);
    if (std.mem.eql(u8, op, "ne")) return op_ne_str(rest);
    if (std.mem.eql(u8, op, "in")) return op_in(rest);
    if (std.mem.eql(u8, op, "ni")) return op_ni(rest);
    if (std.mem.eql(u8, op, "&")) return op_band(rest);
    if (std.mem.eql(u8, op, "|")) return op_bor(rest);
    if (std.mem.eql(u8, op, "^")) return op_bxor(rest);
    if (std.mem.eql(u8, op, "~")) return op_bnot(rest);
    if (std.mem.eql(u8, op, "<<")) return op_lshift(rest);
    if (std.mem.eql(u8, op, ">>")) return op_rshift(rest);
    if (std.mem.eql(u8, op, "!")) return op_not(rest);
    if (std.mem.eql(u8, op, "&&")) return op_and(rest);
    if (std.mem.eql(u8, op, "||")) return op_or(rest);
    if (std.mem.eql(u8, op, "min")) return op_min(rest);
    if (std.mem.eql(u8, op, "max")) return op_max(rest);
    if (std.mem.eql(u8, op, "@")) return op_at(rest);
    return obj_new_int(0);
}

// Each operator gets two registered spellings: the fully-qualified
// ``::tcl::mathop::OP`` form (what tcltest typically writes) and the
// half-qualified ``tcl::mathop::OP`` form (used inside
// ``namespace eval ::tcl`` blocks).  The bare ``+`` / ``==`` / etc.
// names are *not* registered here — they're handled by the
// expression compiler at codegen time, and registering them as
// commands would shadow ``[catch {puts -}]`` style inputs.
pub const registrations = [_]reg.CmdEntry{
    .{ .name = "::tcl::mathop::+",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::-",  .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::*",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::/",  .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::%",  .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "::tcl::mathop::**", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::==", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::!=", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "::tcl::mathop::<",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::>",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::<=", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::>=", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::eq", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::ne", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "::tcl::mathop::in", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "::tcl::mathop::ni", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "::tcl::mathop::&",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::|",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::^",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::~",  .arity_min = 1, .arity_max = 1,    .handler = &eval },
    .{ .name = "::tcl::mathop::<<", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "::tcl::mathop::>>", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "::tcl::mathop::!",  .arity_min = 1, .arity_max = 1,    .handler = &eval },
    .{ .name = "::tcl::mathop::&&", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::||", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::min",.arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::max",.arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "::tcl::mathop::@",  .arity_min = 2, .arity_max = 2,    .handler = &eval },
    // Half-qualified spellings (inside ``namespace eval ::tcl``).
    .{ .name = "tcl::mathop::+",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::-",  .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::*",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::/",  .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::%",  .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "tcl::mathop::**", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::==", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::!=", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "tcl::mathop::<",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::>",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::<=", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::>=", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::eq", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::ne", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "tcl::mathop::in", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "tcl::mathop::ni", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "tcl::mathop::&",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::|",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::^",  .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::~",  .arity_min = 1, .arity_max = 1,    .handler = &eval },
    .{ .name = "tcl::mathop::<<", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "tcl::mathop::>>", .arity_min = 2, .arity_max = 2,    .handler = &eval },
    .{ .name = "tcl::mathop::!",  .arity_min = 1, .arity_max = 1,    .handler = &eval },
    .{ .name = "tcl::mathop::&&", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::||", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::min",.arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::max",.arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "tcl::mathop::@",  .arity_min = 2, .arity_max = 2,    .handler = &eval },
};
