// Bignum-aware runtime expression evaluator for the ``expr``
// command and any other path that evaluates a Tcl expression
// outside the AOT-compiled WASM frame.
//
// The legacy ``eval_expr_str`` in :file:`tcl_interp.zig` is a
// minimal recursive-descent evaluator that returns an ``i64`` and
// supports only ``+ - * / % == != < > <= >=``.  That is enough for
// loop / if conditions but causes ``[expr {1 << 70}]`` to silently
// return ``1`` (the ``<<`` operator is unrecognised, parsing stops
// at it, the leading ``1`` becomes the result).  Worse, even for
// recognised operators the i64 return clamps any overflow to
// ``i64::MIN`` / ``i64::MAX``.
//
// This module adds a parallel evaluator that returns a TclObj
// (preserving TYPE_BIGNUM through every step) and supports the
// full Tcl 9.0 operator set the runtime needs:
//
//   precedence (lowest → highest):
//     ?:         right associative
//     || or
//     && and
//     |          bitwise or
//     ^          bitwise xor
//     &          bitwise and
//     == != eq ne in ni
//     < <= > >= lt le gt ge
//     << >>
//     + -
//     * / %
//     **         right associative
//     unary - + ! ~
//     atom: literal | $var | [cmd] | (expr) | func(args) | "string"
//
// Hand-rolled rather than reusing the AOT expr parser because:
//   * The AOT parser produces an AST then lowers to WASM bytecode.
//     Translating an AST back to a runtime evaluator is more code
//     than just walking the source bytes once.
//   * The AOT parser depends on Python codegen modules that aren't
//     reachable from Zig.
//   * The tcl_arith helpers already do bignum-aware arithmetic;
//     a thin recursive-descent on top of them gets us the full
//     semantic for free.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const arith = @import("../valtypes/tcl_arith.zig");
const string = @import("../valtypes/tcl_string.zig");
const list_mod = @import("../valtypes/tcl_list.zig");

const obj_new_int = obj.obj_new_int;
const obj_new_string = obj.obj_new_string;

// Forward-declare the tcl_interp functions we lean on.  They live
// in the parent module; importing here would create a cycle, so
// we ``@import`` lazily inside the functions that need them.

const State = struct {
    src: [*]const u8,
    len: u32,
    pos: u32,
    /// When ``true``, leaf parsers walk the source text but produce
    /// ``obj_new_int(0)`` instead of evaluating side-effecting nodes
    /// (variable reads, command substitutions, function calls), and
    /// every ``apply_*`` helper short-circuits to ``obj_new_int(0)``.
    /// Used by ``parse_ternary`` / ``parse_or`` / ``parse_and`` to
    /// skip the unselected branch of ``a ? b : c`` / ``a || b`` /
    /// ``a && b`` per Tcl's short-circuit semantics — the legacy
    /// "always evaluate" path mis-fired ``expr {1 || (1/0)}``,
    /// ``expr {0 && [error boom]}``, and ``expr {1 ? 42 : (1/0)}``.
    skip: bool = false,
};

// Helpers to bridge raw memory pointers (usize on wasm32) into the
// runtime's TclObj signatures.  ``obj_new_string`` takes ``i32`` but
// treats the value as a bit pattern (the wasm linear-memory address);
// ``@intCast`` would panic for addresses >= 2 GiB, so we cast through
// u32 and bit-reinterpret.  ``subst_flagged`` already takes u32, so a
// plain ``@intCast`` covers it.

inline fn ptr_as_i32(p: usize) i32 {
    return @bitCast(@as(u32, @intCast(p)));
}

inline fn ptr_as_u32(p: usize) u32 {
    return @intCast(p);
}

inline fn len_as_i32(n: u32) i32 {
    return @bitCast(n);
}

/// Apply a binary runtime op to *left* and *right*, releasing both
/// inputs and returning the freshly-allocated result.  All
/// ``tcl_arith_*`` helpers produce a new TclObj rather than mutating
/// either operand, so each parser-level loop must drop the
/// intermediates it consumed — without this discipline the deep
/// expression trees in upstream tests (e.g. ``hello_world`` from
/// ``compExpr-old.test``) leak hundreds of TclObjs per loop iteration
/// and trip the wasi-libc allocator's u32 overflow check.
///
/// In skip mode (short-circuit unselected branch), *op* is not
/// invoked — both operands are dummy ``obj_new_int(0)`` placeholders
/// and the helper would either NPE on a null TclObj or trigger an
/// arithmetic side effect we explicitly want to suppress (e.g.
/// ``expr {1 || (1/0)}``).
inline fn apply_binary(
    s: *State,
    op: *const fn (a: i32, b: i32) callconv(.c) i32,
    left: i32,
    right: i32,
) i32 {
    if (s.skip) {
        obj.tcl_obj_release(left);
        obj.tcl_obj_release(right);
        return obj_new_int(0);
    }
    const r = op(left, right);
    obj.tcl_obj_release(left);
    obj.tcl_obj_release(right);
    return r;
}

/// Same as :func:`apply_binary` for unary ops.
inline fn apply_unary(
    s: *State,
    op: *const fn (a: i32) callconv(.c) i32,
    operand: i32,
) i32 {
    if (s.skip) {
        obj.tcl_obj_release(operand);
        return obj_new_int(0);
    }
    const r = op(operand);
    obj.tcl_obj_release(operand);
    return r;
}

/// True iff the alphabetic identifier ``name`` matches a Tcl
/// operator-keyword and the *next* character would cleanly end the
/// keyword (i.e. not the leading char of a longer identifier).  The
/// runtime evaluator needs this to disambiguate ``eq``/``ne``/``in``/
/// ``ni``/``lt``/``le``/``gt``/``ge`` from a same-prefixed bareword
/// or function name (``int(x)`` shouldn't trigger the ``in`` operator
/// branch even though it starts with ``in``).
fn match_kw_op(s: *State, kw: []const u8) bool {
    if (s.pos + kw.len > s.len) return false;
    for (kw, 0..) |c, i| {
        if (s.src[s.pos + i] != c) return false;
    }
    if (s.pos + kw.len == s.len) return true;
    const next = s.src[s.pos + kw.len];
    if ((next >= 'a' and next <= 'z') or (next >= 'A' and next <= 'Z') or
        (next >= '0' and next <= '9') or next == '_')
    {
        return false;
    }
    return true;
}

fn skip_ws(s: *State) void {
    while (s.pos < s.len) {
        const c = s.src[s.pos];
        if (c == ' ' or c == '\t' or c == '\n' or c == '\r') {
            s.pos += 1;
        } else break;
    }
}

fn peek(s: *State, n: u32) ?u8 {
    if (s.pos + n >= s.len) return null;
    return s.src[s.pos + n];
}

fn match2(s: *State, a: u8, b: u8) bool {
    if (s.pos + 1 < s.len and s.src[s.pos] == a and s.src[s.pos + 1] == b) {
        s.pos += 2;
        return true;
    }
    return false;
}

fn match1(s: *State, a: u8) bool {
    if (s.pos < s.len and s.src[s.pos] == a) {
        s.pos += 1;
        return true;
    }
    return false;
}

/// Public entry point.  Returns a TclObj holding the expression's
/// value.  On parse error or empty input, returns a TclObj with
/// integer 0 — matching the legacy ``eval_expr_str`` behaviour for
/// non-fatal cases.
pub fn eval(ptr: u32, len: u32) i32 {
    if (len == 0) return obj_new_int(0);
    var s = State{ .src = @ptrFromInt(ptr), .len = len, .pos = 0 };
    skip_ws(&s);
    return parse_ternary(&s);
}

fn parse_ternary(s: *State) i32 {
    const cond = parse_or(s);
    skip_ws(s);
    if (s.pos < s.len and s.src[s.pos] == '?') {
        s.pos += 1;
        // Tcl 9 ``?:`` short-circuits — only the selected branch
        // runs, the other is parsed (so ``s.pos`` advances past it)
        // but no side effect fires.  Without the skip flag,
        // ``expr {1 ? 42 : (1/0)}`` errored on the unselected
        // ``1/0``.  In dry mode the cond was a dummy 0, so we
        // recurse with skip set on both branches — the outer
        // parse_ternary already chose its result before the inner
        // ones were parsed.
        const taken_true = if (s.skip) false else truthy(cond);
        obj.tcl_obj_release(cond);
        const saved_skip = s.skip;
        s.skip = saved_skip or !taken_true;
        const true_branch = parse_ternary(s);
        s.skip = saved_skip;
        skip_ws(s);
        if (s.pos < s.len and s.src[s.pos] == ':') s.pos += 1;
        s.skip = saved_skip or taken_true;
        const false_branch = parse_ternary(s);
        s.skip = saved_skip;
        if (taken_true) {
            obj.tcl_obj_release(false_branch);
            return true_branch;
        }
        obj.tcl_obj_release(true_branch);
        return false_branch;
    }
    return cond;
}

fn parse_or(s: *State) i32 {
    var left = parse_and(s);
    while (true) {
        skip_ws(s);
        if (match2(s, '|', '|')) {
            // Short-circuit: if ``left`` is already truthy we still
            // parse the RHS (so ``s.pos`` advances) but suppress
            // every side effect via ``s.skip``.  ``expr {1 || (1/0)}``
            // without the gate raised divide-by-zero on the dummy
            // RHS evaluation; with the gate it returns 1 cleanly.
            const left_truthy = if (s.skip) false else truthy(left);
            const saved_skip = s.skip;
            s.skip = saved_skip or left_truthy;
            const right = parse_and(s);
            s.skip = saved_skip;
            const result_val: i64 = if (saved_skip) 0 else if (left_truthy or truthy(right)) 1 else 0;
            obj.tcl_obj_release(left);
            obj.tcl_obj_release(right);
            left = obj_new_int(result_val);
        } else break;
    }
    return left;
}

fn parse_and(s: *State) i32 {
    var left = parse_bit_or(s);
    while (true) {
        skip_ws(s);
        if (match2(s, '&', '&')) {
            // Short-circuit: if ``left`` is falsy we suppress the
            // RHS via ``s.skip`` so ``expr {0 && [error boom]}``
            // doesn't trigger ``boom``.
            const left_truthy = if (s.skip) false else truthy(left);
            const saved_skip = s.skip;
            s.skip = saved_skip or !left_truthy;
            const right = parse_bit_or(s);
            s.skip = saved_skip;
            const result_val: i64 =
                if (saved_skip) 0 else if (left_truthy and truthy(right)) 1 else 0;
            obj.tcl_obj_release(left);
            obj.tcl_obj_release(right);
            left = obj_new_int(result_val);
        } else break;
    }
    return left;
}

fn parse_bit_or(s: *State) i32 {
    var left = parse_bit_xor(s);
    while (true) {
        skip_ws(s);
        if (s.pos < s.len and s.src[s.pos] == '|' and (s.pos + 1 >= s.len or s.src[s.pos + 1] != '|')) {
            s.pos += 1;
            const right = parse_bit_xor(s);
            left = apply_binary(s, arith.tcl_arith_bor, left, right);
        } else break;
    }
    return left;
}

fn parse_bit_xor(s: *State) i32 {
    var left = parse_bit_and(s);
    while (true) {
        skip_ws(s);
        if (s.pos < s.len and s.src[s.pos] == '^') {
            s.pos += 1;
            const right = parse_bit_and(s);
            left = apply_binary(s, arith.tcl_arith_bxor, left, right);
        } else break;
    }
    return left;
}

fn parse_bit_and(s: *State) i32 {
    var left = parse_eq_cmp(s);
    while (true) {
        skip_ws(s);
        if (s.pos < s.len and s.src[s.pos] == '&' and (s.pos + 1 >= s.len or s.src[s.pos + 1] != '&')) {
            s.pos += 1;
            const right = parse_eq_cmp(s);
            left = apply_binary(s, arith.tcl_arith_band, left, right);
        } else break;
    }
    return left;
}

/// Map a ``tcl_expr_order_cmp`` result (-1 / 0 / +1 wrapped in a
/// TclObj) to an int for the sign-test in ``==`` / ``!=`` / ``<`` /
/// ``<=`` / ``>`` / ``>=``.  Releases the cmp obj before returning.
inline fn drain_cmp(cmp: i32) i64 {
    const v = obj.obj_get_int(cmp);
    obj.tcl_obj_release(cmp);
    return v;
}

/// Compare *left* and *right* via the bignum-aware order helper,
/// release both inputs, and produce a 0/1 boolean TclObj based on
/// the cmp result vs the requested *threshold*.  In skip mode the
/// helper is bypassed and the result is always 0 (the operands were
/// dummy zeros from a short-circuited branch).
const CmpThreshold = enum { lt, le, eq, ne, ge, gt };

inline fn apply_order_cmp(s: *State, left: i32, right: i32, threshold: CmpThreshold) i32 {
    if (s.skip) {
        obj.tcl_obj_release(left);
        obj.tcl_obj_release(right);
        return obj_new_int(0);
    }
    const cmp = string.tcl_expr_order_cmp(left, right);
    obj.tcl_obj_release(left);
    obj.tcl_obj_release(right);
    const v = drain_cmp(cmp);
    const matched: bool = switch (threshold) {
        .lt => v < 0,
        .le => v <= 0,
        .eq => v == 0,
        .ne => v != 0,
        .ge => v >= 0,
        .gt => v > 0,
    };
    return obj_new_int(if (matched) @as(i64, 1) else 0);
}

/// Same shape as :func:`apply_order_cmp` but for the lexical
/// ``lt`` / ``le`` / ``gt`` / ``ge`` keyword operators backed by
/// ``string_compare``.
inline fn apply_string_cmp(s: *State, left: i32, right: i32, threshold: CmpThreshold) i32 {
    if (s.skip) {
        obj.tcl_obj_release(left);
        obj.tcl_obj_release(right);
        return obj_new_int(0);
    }
    const cmp_obj = string.string_compare(left, right);
    obj.tcl_obj_release(left);
    obj.tcl_obj_release(right);
    const v = obj.obj_get_int(cmp_obj);
    obj.tcl_obj_release(cmp_obj);
    const matched: bool = switch (threshold) {
        .lt => v < 0,
        .le => v <= 0,
        .eq => v == 0,
        .ne => v != 0,
        .ge => v >= 0,
        .gt => v > 0,
    };
    return obj_new_int(if (matched) @as(i64, 1) else 0);
}

/// ``ne`` keyword — string-equal with negated boolean result.
inline fn apply_string_ne(s: *State, left: i32, right: i32) i32 {
    if (s.skip) {
        obj.tcl_obj_release(left);
        obj.tcl_obj_release(right);
        return obj_new_int(0);
    }
    const eq_obj = string.string_equal(left, right);
    obj.tcl_obj_release(left);
    obj.tcl_obj_release(right);
    const v = obj.obj_get_int(eq_obj);
    obj.tcl_obj_release(eq_obj);
    return obj_new_int(if (v == 0) @as(i64, 1) else 0);
}

/// ``in`` keyword — list-membership.  ``tcl_cmd_list_contains``
/// already returns a 0/1 TclObj; just gate the call on skip mode.
/// Argument order is reversed from the source-level ``$value in
/// $list``: the helper takes ``(list, value)``.
inline fn apply_list_in(s: *State, value: i32, list: i32) i32 {
    if (s.skip) {
        obj.tcl_obj_release(value);
        obj.tcl_obj_release(list);
        return obj_new_int(0);
    }
    const r = list_mod.tcl_cmd_list_contains(list, value);
    obj.tcl_obj_release(value);
    obj.tcl_obj_release(list);
    return r;
}

inline fn apply_list_ni(s: *State, value: i32, list: i32) i32 {
    if (s.skip) {
        obj.tcl_obj_release(value);
        obj.tcl_obj_release(list);
        return obj_new_int(0);
    }
    const in_obj = list_mod.tcl_cmd_list_contains(list, value);
    obj.tcl_obj_release(value);
    obj.tcl_obj_release(list);
    const v = obj.obj_get_int(in_obj);
    obj.tcl_obj_release(in_obj);
    return obj_new_int(if (v == 0) @as(i64, 1) else 0);
}

fn parse_eq_cmp(s: *State) i32 {
    var left = parse_order_cmp(s);
    while (true) {
        skip_ws(s);
        if (match2(s, '=', '=')) {
            const right = parse_order_cmp(s);
            left = apply_order_cmp(s, left, right, .eq);
        } else if (match2(s, '!', '=')) {
            const right = parse_order_cmp(s);
            left = apply_order_cmp(s, left, right, .ne);
        } else if (match_kw_op(s, "eq")) {
            // ``eq`` — string equality (canonical-byte-equal).  The
            // ``string_equal`` helper byte-compares the canonical
            // string form of both operands; for canonical decimal
            // ints / bignums it reduces to numeric equality.
            s.pos += 2;
            const right = parse_order_cmp(s);
            left = apply_binary(s, string.string_equal, left, right);
        } else if (match_kw_op(s, "ne")) {
            s.pos += 2;
            const right = parse_order_cmp(s);
            left = apply_string_ne(s, left, right);
        } else if (match_kw_op(s, "in")) {
            // ``in`` — list-membership.  ``list.tcl_cmd_list_contains``
            // takes ``(list, value)`` (note the order).
            s.pos += 2;
            const right = parse_order_cmp(s);
            left = apply_list_in(s, left, right);
        } else if (match_kw_op(s, "ni")) {
            s.pos += 2;
            const right = parse_order_cmp(s);
            left = apply_list_ni(s, left, right);
        } else break;
    }
    return left;
}

fn parse_order_cmp(s: *State) i32 {
    var left = parse_shift(s);
    while (true) {
        skip_ws(s);
        if (match2(s, '<', '=')) {
            const right = parse_shift(s);
            left = apply_order_cmp(s, left, right, .le);
        } else if (match2(s, '>', '=')) {
            const right = parse_shift(s);
            left = apply_order_cmp(s, left, right, .ge);
        } else if (s.pos < s.len and s.src[s.pos] == '<' and (s.pos + 1 >= s.len or s.src[s.pos + 1] != '<')) {
            s.pos += 1;
            const right = parse_shift(s);
            left = apply_order_cmp(s, left, right, .lt);
        } else if (s.pos < s.len and s.src[s.pos] == '>' and (s.pos + 1 >= s.len or s.src[s.pos + 1] != '>')) {
            s.pos += 1;
            const right = parse_shift(s);
            left = apply_order_cmp(s, left, right, .gt);
        } else if (match_kw_op(s, "lt")) {
            // ``lt`` / ``le`` / ``gt`` / ``ge`` — string-order
            // comparison.  Always lexicographic (Tcl 9 semantic),
            // regardless of whether the operands look numeric.
            s.pos += 2;
            const right = parse_shift(s);
            left = apply_string_cmp(s, left, right, .lt);
        } else if (match_kw_op(s, "le")) {
            s.pos += 2;
            const right = parse_shift(s);
            left = apply_string_cmp(s, left, right, .le);
        } else if (match_kw_op(s, "gt")) {
            s.pos += 2;
            const right = parse_shift(s);
            left = apply_string_cmp(s, left, right, .gt);
        } else if (match_kw_op(s, "ge")) {
            s.pos += 2;
            const right = parse_shift(s);
            left = apply_string_cmp(s, left, right, .ge);
        } else break;
    }
    return left;
}

fn parse_shift(s: *State) i32 {
    var left = parse_add(s);
    while (true) {
        skip_ws(s);
        if (match2(s, '<', '<')) {
            const right = parse_add(s);
            left = apply_binary(s, arith.tcl_arith_lshift, left, right);
        } else if (match2(s, '>', '>')) {
            const right = parse_add(s);
            left = apply_binary(s, arith.tcl_arith_rshift, left, right);
        } else break;
    }
    return left;
}

fn parse_add(s: *State) i32 {
    var left = parse_mul(s);
    while (true) {
        skip_ws(s);
        if (s.pos < s.len and s.src[s.pos] == '+') {
            s.pos += 1;
            const right = parse_mul(s);
            left = apply_binary(s, arith.tcl_arith_add, left, right);
        } else if (s.pos < s.len and s.src[s.pos] == '-') {
            s.pos += 1;
            const right = parse_mul(s);
            left = apply_binary(s, arith.tcl_arith_sub, left, right);
        } else break;
    }
    return left;
}

fn parse_mul(s: *State) i32 {
    var left = parse_pow(s);
    while (true) {
        skip_ws(s);
        if (s.pos < s.len and s.src[s.pos] == '*' and (s.pos + 1 >= s.len or s.src[s.pos + 1] != '*')) {
            s.pos += 1;
            const right = parse_pow(s);
            left = apply_binary(s, arith.tcl_arith_mul, left, right);
        } else if (s.pos < s.len and s.src[s.pos] == '/') {
            s.pos += 1;
            const right = parse_pow(s);
            left = apply_binary(s, arith.tcl_arith_div, left, right);
        } else if (s.pos < s.len and s.src[s.pos] == '%') {
            s.pos += 1;
            const right = parse_pow(s);
            left = apply_binary(s, arith.tcl_arith_mod, left, right);
        } else break;
    }
    return left;
}

fn parse_pow(s: *State) i32 {
    const left = parse_unary(s);
    skip_ws(s);
    if (match2(s, '*', '*')) {
        const right = parse_pow(s);
        return apply_binary(s, arith.tcl_arith_pow, left, right);
    }
    return left;
}

fn parse_unary(s: *State) i32 {
    skip_ws(s);
    if (s.pos >= s.len) return obj_new_int(0);
    const c = s.src[s.pos];
    if (c == '-') {
        s.pos += 1;
        return apply_unary(s, arith.tcl_arith_neg, parse_unary(s));
    }
    if (c == '+') {
        s.pos += 1;
        return parse_unary(s);
    }
    if (c == '!') {
        s.pos += 1;
        const operand = parse_unary(s);
        if (s.skip) {
            obj.tcl_obj_release(operand);
            return obj_new_int(0);
        }
        const result = obj_new_int(if (truthy(operand)) @as(i64, 0) else 1);
        obj.tcl_obj_release(operand);
        return result;
    }
    if (c == '~') {
        s.pos += 1;
        return apply_unary(s, arith.tcl_arith_bnot, parse_unary(s));
    }
    return parse_atom(s);
}

fn parse_atom(s: *State) i32 {
    skip_ws(s);
    if (s.pos >= s.len) return obj_new_int(0);
    const c = s.src[s.pos];
    if (c == '(') {
        s.pos += 1;
        const v = parse_ternary(s);
        skip_ws(s);
        if (s.pos < s.len and s.src[s.pos] == ')') s.pos += 1;
        return v;
    }
    if (c == '$') return parse_var(s);
    if (c == '[') return parse_cmd_subst(s);
    if (c == '"') return parse_quoted(s);
    if (c == '{') return parse_braced(s);
    if ((c >= '0' and c <= '9') or c == '.') return parse_number(s);
    if ((c >= 'a' and c <= 'z') or (c >= 'A' and c <= 'Z') or c == '_') {
        return parse_func_or_word(s);
    }
    return obj_new_int(0);
}

fn parse_var(s: *State) i32 {
    // Consume ``$`` and either ``${name}`` or ``$name``.  Defers
    // the actual lookup to the ``ns_subst_word`` machinery via a
    // tiny temporary script — keeps parser scope tight.
    const start = s.pos;
    s.pos += 1; // skip $
    if (s.pos < s.len and s.src[s.pos] == '{') {
        s.pos += 1;
        while (s.pos < s.len and s.src[s.pos] != '}') s.pos += 1;
        if (s.pos < s.len) s.pos += 1;
    } else {
        while (s.pos < s.len) {
            const ch = s.src[s.pos];
            if ((ch >= 'a' and ch <= 'z') or (ch >= 'A' and ch <= 'Z') or
                (ch >= '0' and ch <= '9') or ch == '_' or ch == ':')
            {
                s.pos += 1;
            } else break;
        }
    }
    // Skip mode: source position has advanced past the variable
    // reference, but we suppress the actual lookup.  Without this
    // ``expr {[info exists x] ? $x : 0}`` would still raise
    // ``can't read "x"`` even when ``x`` is unset.
    if (s.skip) return obj_new_int(0);
    const slice_ptr = @intFromPtr(s.src) + start;
    const slice_len = s.pos - start;
    const subst = @import("../parse/tcl_subst.zig");
    return subst.subst_flagged(ptr_as_u32(slice_ptr), slice_len, true, false, false);
}

fn parse_cmd_subst(s: *State) i32 {
    // ``[cmd ...]`` — find the matching close bracket, evaluate
    // the inner script via ``tcl_eval``, return its result obj.
    // Bracket-depth counter handles nested ``[...]`` substitutions.
    s.pos += 1; // skip [
    const start = s.pos;
    var depth: u32 = 1;
    while (s.pos < s.len and depth > 0) : (s.pos += 1) {
        const ch = s.src[s.pos];
        if (ch == '\\' and s.pos + 1 < s.len) {
            s.pos += 1;
            continue;
        }
        if (ch == '[') depth += 1;
        if (ch == ']') {
            depth -= 1;
            if (depth == 0) break;
        }
    }
    const inner_len = s.pos - start;
    if (s.pos < s.len) s.pos += 1; // skip closing ]
    // Skip mode: short-circuited branch — don't run the embedded
    // command.  ``expr {0 && [puts hi]}`` must not call ``puts``.
    if (s.skip) return obj_new_int(0);
    const inner_ptr = @intFromPtr(s.src) + start;
    const interp_mod = @import("tcl_interp.zig");
    const script_obj = obj_new_string(ptr_as_i32(inner_ptr), len_as_i32(inner_len));
    const result = interp_mod.tcl_eval(script_obj);
    // Release the +1 ref from ``obj_new_string`` — ``tcl_eval``
    // retains/releases internally so the script-obj's original ref
    // would otherwise leak per substitution (Copilot review #326).
    obj.tcl_obj_release(script_obj);
    return result;
}

fn parse_quoted(s: *State) i32 {
    s.pos += 1; // skip opening "
    const start = s.pos;
    while (s.pos < s.len and s.src[s.pos] != '"') {
        if (s.src[s.pos] == '\\' and s.pos + 1 < s.len) s.pos += 1;
        s.pos += 1;
    }
    const inner_len = s.pos - start;
    if (s.pos < s.len) s.pos += 1; // skip closing "
    // Skip mode: a quoted string in a short-circuited branch may
    // contain ``$var`` / ``[cmd]`` substitutions whose evaluation
    // would surface the same kind of error we're trying to dodge.
    if (s.skip) return obj_new_int(0);
    const inner_ptr = @intFromPtr(s.src) + start;
    const subst = @import("../parse/tcl_subst.zig");
    return subst.subst_flagged(ptr_as_u32(inner_ptr), inner_len, true, true, true);
}

fn parse_braced(s: *State) i32 {
    s.pos += 1; // skip opening {
    const start = s.pos;
    var depth: u32 = 1;
    while (s.pos < s.len and depth > 0) {
        const ch = s.src[s.pos];
        if (ch == '\\' and s.pos + 1 < s.len) {
            s.pos += 2;
            continue;
        }
        if (ch == '{') depth += 1;
        if (ch == '}') {
            depth -= 1;
            if (depth == 0) break;
        }
        s.pos += 1;
    }
    const inner_len = s.pos - start;
    if (s.pos < s.len) s.pos += 1; // skip closing }
    const inner_ptr = @intFromPtr(s.src) + start;
    return obj_new_string(ptr_as_i32(inner_ptr), len_as_i32(inner_len));
}

fn parse_number(s: *State) i32 {
    const start = s.pos;
    var has_dot = false;
    var has_exp = false;
    // Detect base prefix for integer literals.
    if (s.src[s.pos] == '0' and s.pos + 1 < s.len) {
        const nxt = s.src[s.pos + 1];
        if (nxt == 'x' or nxt == 'X') {
            s.pos += 2;
            while (s.pos < s.len) {
                const ch = s.src[s.pos];
                if ((ch >= '0' and ch <= '9') or (ch >= 'a' and ch <= 'f') or (ch >= 'A' and ch <= 'F')) {
                    s.pos += 1;
                } else break;
            }
            return finalize_num(s, start);
        }
        if (nxt == 'o' or nxt == 'O') {
            s.pos += 2;
            while (s.pos < s.len and s.src[s.pos] >= '0' and s.src[s.pos] <= '7') s.pos += 1;
            return finalize_num(s, start);
        }
        if (nxt == 'b' or nxt == 'B') {
            s.pos += 2;
            while (s.pos < s.len and (s.src[s.pos] == '0' or s.src[s.pos] == '1')) s.pos += 1;
            return finalize_num(s, start);
        }
    }
    while (s.pos < s.len) {
        const ch = s.src[s.pos];
        if (ch >= '0' and ch <= '9') {
            s.pos += 1;
        } else if (ch == '.' and !has_dot and !has_exp) {
            has_dot = true;
            s.pos += 1;
        } else if ((ch == 'e' or ch == 'E') and !has_exp) {
            has_exp = true;
            s.pos += 1;
            if (s.pos < s.len and (s.src[s.pos] == '+' or s.src[s.pos] == '-')) s.pos += 1;
        } else break;
    }
    return finalize_num(s, start);
}

fn finalize_num(s: *State, start: u32) i32 {
    const slen = s.pos - start;
    const sptr = @intFromPtr(s.src) + start;
    return obj_new_string(ptr_as_i32(sptr), len_as_i32(slen));
}

fn parse_func_or_word(s: *State) i32 {
    const start = s.pos;
    while (s.pos < s.len) {
        const ch = s.src[s.pos];
        if ((ch >= 'a' and ch <= 'z') or (ch >= 'A' and ch <= 'Z') or
            (ch >= '0' and ch <= '9') or ch == '_' or ch == ':')
        {
            s.pos += 1;
        } else break;
    }
    const name_len = s.pos - start;
    const name: []const u8 = (s.src + start)[0..name_len];
    skip_ws(s);
    // ``func(args)`` form — function call.
    if (s.pos < s.len and s.src[s.pos] == '(') {
        s.pos += 1;
        var args: [4]i32 = .{ 0, 0, 0, 0 };
        var argc: usize = 0;
        skip_ws(s);
        if (s.pos < s.len and s.src[s.pos] != ')') {
            while (true) {
                const slot = parse_ternary(s);
                if (argc < args.len) {
                    args[argc] = slot;
                    argc += 1;
                } else {
                    // Overflow: too many args — release the slot
                    // immediately so it doesn't leak, and let the
                    // dispatcher error on the (clamped) arg count.
                    // Without this clamp ``pow(1,2,3,4,5)`` walked
                    // off ``args[4]`` and tripped the @intCast /
                    // bounds check in safe builds (Codex review
                    // #326).
                    obj.tcl_obj_release(slot);
                }
                skip_ws(s);
                if (s.pos < s.len and s.src[s.pos] == ',') {
                    s.pos += 1;
                    skip_ws(s);
                } else break;
            }
        }
        if (s.pos < s.len and s.src[s.pos] == ')') s.pos += 1;
        if (s.skip) {
            for (args[0..argc]) |arg| obj.tcl_obj_release(arg);
            return obj_new_int(0);
        }
        const result = dispatch_math_func(name, args[0..argc]);
        // Release the argument TclObjs we own — the math helpers
        // produce a fresh result rather than retaining their inputs.
        for (args[0..argc]) |arg| obj.tcl_obj_release(arg);
        return result;
    }
    // Bare keyword: ``true`` / ``false`` / ``yes`` / ``no`` / ``on`` / ``off``.
    if (parse_bool_keyword(name)) |v| return obj_new_int(v);
    // Otherwise treat as a string literal — matches Tcl's handling
    // of bare words in expressions when they don't match a known
    // function name.
    if (s.skip) return obj_new_int(0);
    const sptr = @intFromPtr(s.src) + start;
    return obj_new_string(ptr_as_i32(sptr), len_as_i32(name_len));
}

fn parse_bool_keyword(name: []const u8) ?i64 {
    if (std.mem.eql(u8, name, "true") or std.mem.eql(u8, name, "yes") or std.mem.eql(u8, name, "on"))
        return 1;
    if (std.mem.eql(u8, name, "false") or std.mem.eql(u8, name, "no") or std.mem.eql(u8, name, "off"))
        return 0;
    return null;
}

fn dispatch_math_func(name: []const u8, args: []const i32) i32 {
    if (args.len == 1) {
        if (std.mem.eql(u8, name, "int") or std.mem.eql(u8, name, "wide") or std.mem.eql(u8, name, "entier"))
            return arith.tcl_math_int(args[0]);
        if (std.mem.eql(u8, name, "double") or std.mem.eql(u8, name, "float"))
            return arith.tcl_math_double(args[0]);
        if (std.mem.eql(u8, name, "round")) return arith.tcl_math_round(args[0]);
        if (std.mem.eql(u8, name, "abs") or std.mem.eql(u8, name, "fabs"))
            return arith.tcl_math_fabs(args[0]);
        if (std.mem.eql(u8, name, "log")) return arith.tcl_math_log(args[0]);
        if (std.mem.eql(u8, name, "log10")) return arith.tcl_math_log10(args[0]);
        if (std.mem.eql(u8, name, "sqrt")) return arith.tcl_math_sqrt(args[0]);
        if (std.mem.eql(u8, name, "exp")) return arith.tcl_math_exp(args[0]);
        if (std.mem.eql(u8, name, "sin")) return arith.tcl_math_sin(args[0]);
        if (std.mem.eql(u8, name, "cos")) return arith.tcl_math_cos(args[0]);
        if (std.mem.eql(u8, name, "tan")) return arith.tcl_math_tan(args[0]);
        if (std.mem.eql(u8, name, "asin")) return arith.tcl_math_asin(args[0]);
        if (std.mem.eql(u8, name, "acos")) return arith.tcl_math_acos(args[0]);
        if (std.mem.eql(u8, name, "atan")) return arith.tcl_math_atan(args[0]);
        if (std.mem.eql(u8, name, "sinh")) return arith.tcl_math_sinh(args[0]);
        if (std.mem.eql(u8, name, "cosh")) return arith.tcl_math_cosh(args[0]);
        if (std.mem.eql(u8, name, "tanh")) return arith.tcl_math_tanh(args[0]);
        if (std.mem.eql(u8, name, "floor")) return arith.tcl_math_floor(args[0]);
        if (std.mem.eql(u8, name, "ceil")) return arith.tcl_math_ceil(args[0]);
    }
    if (args.len == 2) {
        if (std.mem.eql(u8, name, "pow")) return arith.tcl_arith_pow(args[0], args[1]);
        if (std.mem.eql(u8, name, "atan2")) return arith.tcl_math_atan2(args[0], args[1]);
        if (std.mem.eql(u8, name, "fmod")) return arith.tcl_math_fmod(args[0], args[1]);
        if (std.mem.eql(u8, name, "hypot")) return arith.tcl_math_hypot(args[0], args[1]);
    }
    return obj_new_int(0);
}

fn truthy(o: i32) bool {
    return obj.obj_get_int(o) != 0;
}
