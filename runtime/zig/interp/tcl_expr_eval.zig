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
    // Tcl 9 ``expr`` accepts ``3eq2`` as ``3 eq 2`` — after a
    // keyword operator, only an alpha or underscore continues
    // the identifier.  Digits and punctuation are valid
    // boundaries (they cannot extend ``eq`` into a longer
    // identifier).  The earlier rule lumped digits in with
    // alpha and rejected ``3eq2`` outright.
    const next = s.src[s.pos + kw.len];
    if ((next >= 'a' and next <= 'z') or (next >= 'A' and next <= 'Z') or next == '_') {
        return false;
    }
    return true;
}

fn skip_ws(s: *State) void {
    while (s.pos < s.len) {
        const c = s.src[s.pos];
        if (c == ' ' or c == '\t' or c == '\n' or c == '\r') {
            s.pos += 1;
        } else if (c == '#') {
            // TIP 582: ``#`` starts a line comment that runs to the
            // next newline (or end of input).  Skipping the comment
            // here lets ``expr {1 # comment\n + 2}`` and the
            // ``max(1,# comment\n2)`` family of cases parse cleanly
            // without modifying every parse_* call site.  The
            // comment terminator is the literal ``\n`` byte; ``\r``
            // alone doesn't end the comment because Tcl's source
            // line-end is the newline byte.
            while (s.pos < s.len and s.src[s.pos] != '\n') s.pos += 1;
            // Loop iteration will pick up the trailing whitespace.
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

/// Top-level expression evaluator entry — runs ``eval`` and then
/// shimmers the result toward its canonical numeric form when the
/// source was a single token (var read, quoted string, braced
/// literal).  Distinct from :func:`eval` because nested ``eval``
/// invocations (cond / branch / sub-expression) must NOT canonicalise
/// — comparison and string-op operands need their original string
/// repr (``$v1 < "0x12"`` falls back to string compare which sees
/// ``"0x12"``, not the canonicalised ``18``; expr-8.29).
pub fn eval_top(ptr: u32, len: u32) i32 {
    if (len == 0) return obj_new_int(0);
    var s = State{ .src = @ptrFromInt(ptr), .len = len, .pos = 0 };
    skip_ws(&s);
    const result = parse_ternary(&s);
    skip_ws(&s);
    // Reject ``NaN`` as the whole-expression result — reference Tcl 9
    // emits ``domain error: argument not in valid range`` for
    // ``expr nan`` (expr-old-36.7).  We let nested operands carry
    // NaN through (so ``isnan(NaN)`` / ``isunordered(NaN, $v)``
    // work) and only check at this top-level entry.
    if (result != 0 and !obj.is_immediate(result)) {
        const tag = obj.read_i32(@as(u32, @bitCast(result)) + obj.OBJ_TYPE_TAG);
        if (tag == obj.TYPE_FLOAT) {
            const fv: f64 = @bitCast(obj.read_i64(@as(u32, @bitCast(result)) + obj.OBJ_INT_CACHE));
            if (std.math.isNan(fv)) {
                @import("../stubs/tcl_stubs.zig").raise(
                    "domain error: argument not in valid range",
                );
                obj.tcl_obj_release(result);
                return obj_new_int(0);
            }
        }
    }
    if (s.pos == s.len and result != 0 and !obj.is_immediate(result)) {
        const tag = obj.read_i32(@as(u32, @bitCast(result)) + obj.OBJ_TYPE_TAG);
        if (tag != obj.TYPE_INT and tag != obj.TYPE_FLOAT and tag != obj.TYPE_BIGNUM) {
            const rs = obj.obj_ensure_string(result);
            if (rs.len > 0) {
                if (try_canonicalise_string(rs.ptr, rs.len)) |canon| {
                    obj.tcl_obj_release(result);
                    return canon;
                }
            }
        }
    }
    return result;
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
        var taken_true: bool = false;
        if (!s.skip) {
            if (truthy_strict(cond)) |b| {
                taken_true = b;
            } else {
                raise_expected_boolean(cond);
                obj.tcl_obj_release(cond);
                return obj_new_int(0);
            }
        }
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
            // Tcl 9 also requires both operands be valid booleans
            // — ``expr {"a" || "b"}`` raises ``expected boolean
            // value but got "a"`` rather than silently returning 0.
            var left_truthy: bool = false;
            if (!s.skip) {
                if (truthy_strict(left)) |b| {
                    left_truthy = b;
                } else {
                    raise_expected_boolean(left);
                    obj.tcl_obj_release(left);
                    return obj_new_int(0);
                }
            }
            const saved_skip = s.skip;
            s.skip = saved_skip or left_truthy;
            const right = parse_and(s);
            s.skip = saved_skip;
            var result_val: i64 = 0;
            if (!saved_skip) {
                if (left_truthy) {
                    result_val = 1;
                } else if (truthy_strict(right)) |b| {
                    result_val = if (b) 1 else 0;
                } else {
                    raise_expected_boolean(right);
                    obj.tcl_obj_release(left);
                    obj.tcl_obj_release(right);
                    return obj_new_int(0);
                }
            }
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
            // doesn't trigger ``boom``.  Strict-boolean check
            // mirrors ``||``.
            var left_truthy: bool = false;
            if (!s.skip) {
                if (truthy_strict(left)) |b| {
                    left_truthy = b;
                } else {
                    raise_expected_boolean(left);
                    obj.tcl_obj_release(left);
                    return obj_new_int(0);
                }
            }
            const saved_skip = s.skip;
            s.skip = saved_skip or !left_truthy;
            const right = parse_bit_or(s);
            s.skip = saved_skip;
            var result_val: i64 = 0;
            if (!saved_skip) {
                if (!left_truthy) {
                    result_val = 0;
                } else if (truthy_strict(right)) |b| {
                    result_val = if (b) 1 else 0;
                } else {
                    raise_expected_boolean(right);
                    obj.tcl_obj_release(left);
                    obj.tcl_obj_release(right);
                    return obj_new_int(0);
                }
            }
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
        // Tcl 9 unary ``+`` validates non-numeric strings — ``expr
        // {+"a"}`` raises ``cannot use non-numeric string "a" as
        // operand of "+"`` (expr-old-5.2).  Route through
        // ``tcl_arith_pos`` so the runtime fast-path test bundle
        // (which uses this evaluator) observes the same surface as
        // the AOT codegen path.
        return apply_unary(s, arith.tcl_arith_pos, parse_unary(s));
    }
    if (c == '!') {
        s.pos += 1;
        const operand = parse_unary(s);
        if (s.skip) {
            obj.tcl_obj_release(operand);
            return obj_new_int(0);
        }
        // Tcl 9 ``!`` requires a boolean operand.  Unlike ``||`` /
        // ``&&`` which raise ``expected boolean value but got
        // "X"``, the unary form uses ``cannot use non-numeric
        // string "X" as operand of "!"`` (expr-4.10 / 21.13–22).
        var b: bool = false;
        if (truthy_strict(operand)) |bv| {
            b = bv;
        } else {
            raise_non_numeric_unary(operand, "!");
            obj.tcl_obj_release(operand);
            return obj_new_int(0);
        }
        const result = obj_new_int(if (b) @as(i64, 0) else 1);
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
        // Bareword name scanner — matches the canonical version in
        // ``subst_flagged_full``: identifier chars plus the ``::``
        // namespace separator only.  A lone ``:`` terminates the
        // name (so ``$x:foo`` resolves ``$x`` and leaves ``:foo``
        // literal); accepting a single ``:`` would silently absorb
        // it into the variable name and surface as a misleading
        // ``can't read "x:foo"`` later.
        while (s.pos < s.len) {
            const ch = s.src[s.pos];
            if ((ch >= 'a' and ch <= 'z') or (ch >= 'A' and ch <= 'Z') or
                (ch >= '0' and ch <= '9') or ch == '_')
            {
                s.pos += 1;
            } else if (ch == ':' and s.pos + 1 < s.len and s.src[s.pos + 1] == ':') {
                s.pos += 2;
            } else break;
        }
        // Array element form ``$arr(idx)``: extend the slice through
        // the matching ``)`` so the downstream ``subst_flagged`` call
        // sees the full ``$arr(idx)`` reference and dispatches to its
        // array-lookup path.  Truncating the slice at ``$arr`` would
        // route the lookup through ``var_resolve`` as if the user
        // wrote ``$arr`` standalone, which then traps with
        // ``can't read "arr": no such variable``.  Mirrors the
        // array-form branch in ``subst_flagged_full``
        // (parse/tcl_subst.zig): scan to the first ``)`` with no
        // escape handling — both end-positions must agree so the
        // recursive substitution of the index span sees the same
        // bytes the expression parser advanced over.
        if (s.pos < s.len and s.src[s.pos] == '(') {
            s.pos += 1;
            while (s.pos < s.len and s.src[s.pos] != ')') s.pos += 1;
            if (s.pos < s.len) s.pos += 1; // consume ')'
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

/// Try to convert *s* to a canonical numeric obj, returning ``null``
/// when the string isn't a valid Tcl integer / hex / octal / binary /
/// float literal in its entirety.  Distinct from the expression
/// evaluator's eager parser (which would consume ``0o289`` as ``0o2``
/// and silently drop the trailing ``89``).
fn try_canonicalise_string(ptr: u32, len: u32) ?i32 {
    if (len == 0) return null;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    while (i < len and obj.is_space(src[i])) i += 1;
    if (i >= len) return null;
    const sign_start = i;
    if (src[i] == '+' or src[i] == '-') i += 1;
    if (i >= len) return null;
    // Base prefix?
    if (i + 1 < len and src[i] == '0' and (src[i + 1] == 'x' or src[i + 1] == 'X' or
        src[i + 1] == 'o' or src[i + 1] == 'O' or src[i + 1] == 'b' or src[i + 1] == 'B'))
    {
        const base_char = src[i + 1];
        const digit_start = i + 2;
        var j = digit_start;
        while (j < len and !obj.is_space(src[j])) : (j += 1) {
            const ch = src[j];
            const ok = switch (base_char) {
                'x', 'X' => (ch >= '0' and ch <= '9') or (ch >= 'a' and ch <= 'f') or (ch >= 'A' and ch <= 'F'),
                'o', 'O' => ch >= '0' and ch <= '7',
                'b', 'B' => ch == '0' or ch == '1',
                else => unreachable,
            };
            if (!ok) return null;
        }
        if (j == digit_start) return null;
        // Trailing whitespace allowed.
        var k = j;
        while (k < len and obj.is_space(src[k])) k += 1;
        if (k != len) return null;
        // Re-use eval() now that we know the string is fully valid:
        // it handles bignum overflow and emits the canonical decimal
        // form.  ``sign_start`` covers the optional ``+``/``-`` prefix.
        const slice_ptr: u32 = @intCast(@intFromPtr(src) + sign_start);
        const slice_len: u32 = j - sign_start;
        return eval(slice_ptr, slice_len);
    }
    // Plain decimal int form (covers leading zeros like ``0005`` per
    // Tcl 9 — which canonicalises to ``5`` decimal, not octal).
    if (obj.try_parse_int(ptr, len)) |iv| return obj_new_int(iv);
    // Floating point.
    if (obj.try_parse_float(ptr, len)) |fv| return obj.obj_new_float(fv);
    return null;
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
    const result = subst.subst_flagged(ptr_as_u32(inner_ptr), inner_len, true, true, true);
    // Tcl 9 ``expr`` shimmers a quoted operand to its numeric
    // form when the string parses as an integer or float — so
    // ``expr "0005"`` returns ``5`` (canonical int), not ``"0005"``.
    // Without this, an ``expr "0005"`` whose result is consumed
    // verbatim by ``return`` / ``set`` keeps the leading zeroes
    // and downstream comparisons against ``5`` mismatch.
    if (result != 0) {
        const rs = obj.obj_ensure_string(result);
        if (rs.len > 0) {
            if (obj.try_parse_int(rs.ptr, rs.len)) |iv| {
                obj.tcl_obj_release(result);
                return obj_new_int(iv);
            }
            if (obj.try_parse_float(rs.ptr, rs.len)) |fv| {
                obj.tcl_obj_release(result);
                return obj.obj_new_float(fv);
            }
        }
    }
    return result;
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
    // Tcl 9 ``expr {{X}}`` shimmers the brace-quoted operand
    // toward its numeric form when the inner text parses as a
    // number.  ``expr {{-0x1234}}`` returns ``-4660`` rather
    // than the literal string ``"-0x1234"``.  Use the same
    // detector ``finalize_num`` runs on bare numeric literals
    // (base-prefix, decimal, float).
    if (inner_len > 0) {
        const sptr = inner_ptr;
        const slen = inner_len;
        const src: [*]const u8 = @ptrFromInt(sptr);
        // Trim leading minus/plus to inspect base prefix.
        var off: u32 = 0;
        var sign_neg = false;
        if (slen > 0 and (src[0] == '-' or src[0] == '+')) {
            sign_neg = src[0] == '-';
            off = 1;
        }
        if (off + 2 <= slen and src[off] == '0') {
            const c = src[off + 1];
            if (c == 'x' or c == 'X' or c == 'o' or c == 'O' or c == 'b' or c == 'B') {
                // Re-use finalize_num's logic by funnelling the
                // entire (possibly signed) source through
                // ``try_parse_int`` first via a synthetic buffer.
                // For simplicity, inline the base-prefix loop
                // since ``try_parse_int`` only handles decimal.
                var v: u64 = 0;
                var base: u8 = 10;
                if (c == 'x' or c == 'X') base = 16 else if (c == 'o' or c == 'O') base = 8 else base = 2;
                var i: u32 = off + 2;
                var ok = i < slen;
                while (i < slen) : (i += 1) {
                    const ch = src[i];
                    var d: u32 = 16;
                    if (ch >= '0' and ch <= '9') d = ch - '0' else if (ch >= 'a' and ch <= 'f') d = (ch - 'a') + 10 else if (ch >= 'A' and ch <= 'F') d = (ch - 'A') + 10 else {
                        ok = false;
                        break;
                    }
                    if (d >= @as(u32, base)) {
                        ok = false;
                        break;
                    }
                    const m = @mulWithOverflow(v, @as(u64, base));
                    if (m[1] != 0) {
                        ok = false;
                        break;
                    }
                    const a = @addWithOverflow(m[0], @as(u64, d));
                    if (a[1] != 0) {
                        ok = false;
                        break;
                    }
                    v = a[0];
                }
                if (ok and v <= @as(u64, 9223372036854775807)) {
                    const iv: i64 = @intCast(v);
                    return obj_new_int(if (sign_neg) -iv else iv);
                }
            }
        }
        if (obj.try_parse_int(@bitCast(sptr), @bitCast(slen))) |iv| {
            return obj_new_int(iv);
        }
        if (obj.try_parse_float(@bitCast(sptr), @bitCast(slen))) |fv| {
            return obj.obj_new_float(fv);
        }
        // Inner contains line continuations (``\<newline>``) — Tcl 9
        // expr-braced operands collapse those to whitespace before
        // parsing the inner numeric token (compExpr-2.5).  Stamp out
        // a scratch copy with the continuation removed and retry
        // ``try_parse_int`` / ``try_parse_float`` against the cleaned
        // bytes.  Skip the copy when no backslash-newline is present
        // — the common case (no continuation) keeps the original
        // hot-path trim-free.
        var has_cont: bool = false;
        var ci: u32 = 0;
        while (ci + 1 < slen) : (ci += 1) {
            if (src[ci] == '\\' and src[ci + 1] == '\n') {
                has_cont = true;
                break;
            }
        }
        if (has_cont) {
            const buf_addr = obj.alloc(slen);
            if (buf_addr != 0) {
                const buf: [*]u8 = @ptrFromInt(buf_addr);
                var bi: u32 = 0;
                var di: u32 = 0;
                while (bi < slen) : (bi += 1) {
                    if (bi + 1 < slen and src[bi] == '\\' and src[bi + 1] == '\n') {
                        // Replace ``\<newline>[whitespace*]`` with a single
                        // space — matches ``Tcl_Backslash`` behaviour.
                        buf[di] = ' ';
                        di += 1;
                        bi += 1; // outer for loop also advances
                        // Skip subsequent whitespace per Tcl's line-
                        // continuation semantics.
                        while (bi + 1 < slen and (src[bi + 1] == ' ' or src[bi + 1] == '\t')) bi += 1;
                        continue;
                    }
                    buf[di] = src[bi];
                    di += 1;
                }
                const cleaned_ptr: u32 = buf_addr;
                if (obj.try_parse_int(cleaned_ptr, di)) |iv| {
                    obj.free_sized(buf_addr, slen);
                    return obj_new_int(iv);
                }
                if (obj.try_parse_float(cleaned_ptr, di)) |fv| {
                    obj.free_sized(buf_addr, slen);
                    return obj.obj_new_float(fv);
                }
                obj.free_sized(buf_addr, slen);
            }
        }
    }
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
            // ``e``/``E`` only starts a float exponent when at
            // least one digit follows (with an optional sign).
            // ``expr 3eq2`` (string-equal operator) must NOT
            // consume the ``e`` as exponent — without this guard
            // ``parse_number`` greedily ate ``3e`` and the test
            // got the literal ``"3e"`` back instead of comparing
            // ``3 eq 2``.
            var look = s.pos + 1;
            if (look < s.len and (s.src[look] == '+' or s.src[look] == '-')) look += 1;
            if (look >= s.len or s.src[look] < '0' or s.src[look] > '9') break;
            has_exp = true;
            s.pos = look;
        } else break;
    }
    return finalize_num(s, start);
}

fn finalize_num(s: *State, start: u32) i32 {
    const slen = s.pos - start;
    const sptr = @intFromPtr(s.src) + start;
    // Tcl 9 ``expr`` canonicalises numeric literals: ``0005``
    // collapses to ``5``, ``0x1234`` to ``4660``, ``1.5e2`` to
    // ``150.0``.  Without this normalisation the returned obj
    // keeps the source bytes and a top-level ``expr 0005`` prints
    // the literal spelling instead of the canonical decimal
    // form.  Detect base-prefixed forms first (``0x`` / ``0o`` /
    // ``0b``) so hex / octal / binary literals collapse to
    // their decimal int form; otherwise fall back to the
    // ``try_parse_int`` / ``try_parse_float`` decimal path.
    const src: [*]const u8 = @ptrFromInt(sptr);
    if (slen >= 2 and src[0] == '0') {
        const c = src[1];
        if (c == 'x' or c == 'X') {
            var v: u64 = 0;
            var i: u32 = 2;
            var ok = i < slen;
            while (i < slen) : (i += 1) {
                const ch = src[i];
                const d: u8 = if (ch >= '0' and ch <= '9')
                    ch - '0'
                else if (ch >= 'a' and ch <= 'f')
                    (ch - 'a') + 10
                else if (ch >= 'A' and ch <= 'F')
                    (ch - 'A') + 10
                else blk: {
                    ok = false;
                    break :blk 0;
                };
                if (!ok) break;
                const m = @mulWithOverflow(v, @as(u64, 16));
                if (m[1] != 0) {
                    ok = false;
                    break;
                }
                const a = @addWithOverflow(m[0], @as(u64, d));
                if (a[1] != 0) {
                    ok = false;
                    break;
                }
                v = a[0];
            }
            if (ok and v <= @as(u64, @intCast(@as(i64, 9223372036854775807)))) {
                return obj_new_int(@intCast(v));
            }
        } else if (c == 'o' or c == 'O') {
            var v: u64 = 0;
            var i: u32 = 2;
            var ok = i < slen;
            while (i < slen) : (i += 1) {
                const ch = src[i];
                if (ch < '0' or ch > '7') {
                    ok = false;
                    break;
                }
                const m = @mulWithOverflow(v, @as(u64, 8));
                if (m[1] != 0) {
                    ok = false;
                    break;
                }
                const a = @addWithOverflow(m[0], @as(u64, ch - '0'));
                if (a[1] != 0) {
                    ok = false;
                    break;
                }
                v = a[0];
            }
            // ``@intCast(v)`` panics under Zig safety checks when ``v``
            // exceeds ``i64`` max, so the i64-bounds check here is
            // load-bearing — without it, ``0o7777777777777777777777`` and
            // friends trap rather than falling through to the bignum path.
            if (ok and v <= @as(u64, @intCast(@as(i64, 9223372036854775807)))) {
                return obj_new_int(@intCast(v));
            }
        } else if (c == 'b' or c == 'B') {
            var v: u64 = 0;
            var i: u32 = 2;
            var ok = i < slen;
            while (i < slen) : (i += 1) {
                const ch = src[i];
                if (ch != '0' and ch != '1') {
                    ok = false;
                    break;
                }
                const m = @mulWithOverflow(v, @as(u64, 2));
                if (m[1] != 0) {
                    ok = false;
                    break;
                }
                v = m[0] + @as(u64, ch - '0');
            }
            // Same i64-bounds gate as the hex path — guard against
            // ``@intCast`` panicking for values >= 2^63.
            if (ok and v <= @as(u64, @intCast(@as(i64, 9223372036854775807)))) {
                return obj_new_int(@intCast(v));
            }
        }
    }
    if (obj.try_parse_int(@bitCast(sptr), @bitCast(slen))) |iv| {
        return obj_new_int(iv);
    }
    if (obj.try_parse_float(@bitCast(sptr), @bitCast(slen))) |fv| {
        return obj.obj_new_float(fv);
    }
    // Bignum path — base-prefixed integer literals that overflow i64
    // (``0b1[64 zeros]`` = 2^64, ``0xff…ff`` past 16 digits) and plain
    // decimal literals beyond i64 / i128 range get the full Managed
    // parser, which collapses to the canonical decimal string repr.
    // Without this, ``expr-43.11`` returned ``0b11…1`` (the source
    // bytes) instead of ``18446744073709551615``.
    const bn = @import("../valtypes/tcl_bignum.zig");
    if (bn.alloc_from_string(@bitCast(sptr), @bitCast(slen))) |m| {
        return obj.obj_new_bignum_take(m);
    }
    // Truly unparseable — fall back to the source bytes.
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
    // Bare boolean keyword (``true`` / ``false`` / ``yes`` / ``no``
    // / ``on`` / ``off``) — Tcl 9 returns the SOURCE spelling
    // verbatim when the keyword is the whole expression
    // (``expr false`` → ``false``, not ``0``).  Coercion to int
    // only happens when the value flows into an arithmetic /
    // logical operator.  ``parse_bool_keyword`` is still useful
    // for identifying the operand class but we no longer use its
    // numeric form for the bare-keyword case.
    if (s.skip) return obj_new_int(0);
    // Tcl 9 ``NaN`` (case-insensitive) is a valid float keyword in
    // expr — it produces a NaN-valued ``TYPE_FLOAT`` operand.
    // Arithmetic operators that reject non-finite operands raise
    // their own diagnostic (``cannot use non-numeric floating-point
    // value …``), so we shouldn't pre-emptively block parsing here.
    // ``isunordered(NaN, $v)`` and ``isnan(NaN)`` both depend on the
    // bareword returning a real NaN.
    if (is_nan_keyword(name)) {
        return obj.obj_new_float(std.math.nan(f64));
    }
    // ``Inf`` / ``Infinity`` are likewise valid float keywords.  The
    // expression-level numeric path picks them up via
    // ``try_parse_float`` once the bareword reaches the operator
    // dispatch, but eagerly returning a TYPE_FLOAT here means
    // ``isfinite(Inf)`` / ``isnan(Inf)`` get a real numeric operand
    // rather than a TYPE_STRING ``"Inf"`` they'd have to re-parse.
    if (is_inf_keyword(name)) {
        return obj.obj_new_float(std.math.inf(f64));
    }
    const sptr = @intFromPtr(s.src) + start;
    return obj_new_string(ptr_as_i32(sptr), len_as_i32(name_len));
}

fn is_inf_keyword(name: []const u8) bool {
    if (name.len != 3 and name.len != 8) return false;
    var buf: [8]u8 = undefined;
    for (0..name.len) |i| {
        const c = name[i];
        buf[i] = if (c >= 'A' and c <= 'Z') c + 32 else c;
    }
    return std.mem.eql(u8, buf[0..name.len], "inf") or std.mem.eql(u8, buf[0..name.len], "infinity");
}

fn is_nan_keyword(name: []const u8) bool {
    if (name.len != 3) return false;
    var buf: [3]u8 = undefined;
    for (0..3) |i| {
        const c = name[i];
        buf[i] = if (c >= 'A' and c <= 'Z') c + 32 else c;
    }
    return std.mem.eql(u8, buf[0..3], "nan");
}

fn parse_bool_keyword(name: []const u8) ?i64 {
    if (std.mem.eql(u8, name, "true") or std.mem.eql(u8, name, "yes") or std.mem.eql(u8, name, "on"))
        return 1;
    if (std.mem.eql(u8, name, "false") or std.mem.eql(u8, name, "no") or std.mem.eql(u8, name, "off"))
        return 0;
    return null;
}

fn dispatch_math_func(name: []const u8, args: []const i32) i32 {
    // Variable-arity math functions — checked before the per-arity
    // chains so e.g. ``min(1)`` and ``max(1,2,3,4)`` both reach the
    // same handler.  Reference Tcl allows the empty form (``min()`` →
    // Inf, ``max()`` → -Inf) so the harness's deg-test cases that
    // exercise zero-arg dispatch don't trap.
    // Zero-arity ``rand()`` — uniform float in [0, 1).
    if (std.mem.eql(u8, name, "rand")) {
        if (args.len != 0) return raise_too_many_args(name);
        return arith.tcl_math_rand();
    }
    if (std.mem.eql(u8, name, "srand")) {
        if (args.len != 1) {
            if (args.len > 1) return raise_too_many_args(name);
            return raise_too_few_args(name);
        }
        return arith.tcl_math_srand(args[0]);
    }
    if (std.mem.eql(u8, name, "min")) {
        if (args.len == 0) return obj.obj_new_float(std.math.inf(f64));
        var best = args[0];
        obj.tcl_obj_retain(best);
        var i: usize = 1;
        while (i < args.len) : (i += 1) {
            if (arith.compare_for_minmax(args[i], best) < 0) {
                obj.tcl_obj_release(best);
                best = args[i];
                obj.tcl_obj_retain(best);
            }
        }
        return best;
    }
    if (std.mem.eql(u8, name, "max")) {
        if (args.len == 0) return obj.obj_new_float(-std.math.inf(f64));
        var best = args[0];
        obj.tcl_obj_retain(best);
        var i: usize = 1;
        while (i < args.len) : (i += 1) {
            if (arith.compare_for_minmax(args[i], best) > 0) {
                obj.tcl_obj_release(best);
                best = args[i];
                obj.tcl_obj_retain(best);
            }
        }
        return best;
    }
    if (args.len == 1) {
        if (std.mem.eql(u8, name, "int") or std.mem.eql(u8, name, "entier"))
            return arith.tcl_math_int(args[0]);
        if (std.mem.eql(u8, name, "wide"))
            return arith.tcl_math_wide(args[0]);
        if (std.mem.eql(u8, name, "double") or std.mem.eql(u8, name, "float"))
            return arith.tcl_math_double(args[0]);
        if (std.mem.eql(u8, name, "round")) return arith.tcl_math_round(args[0]);
        if (std.mem.eql(u8, name, "abs") or std.mem.eql(u8, name, "fabs"))
            return arith.tcl_math_fabs(args[0]);
        if (std.mem.eql(u8, name, "log")) return arith.tcl_math_log(args[0]);
        if (std.mem.eql(u8, name, "log10")) return arith.tcl_math_log10(args[0]);
        if (std.mem.eql(u8, name, "sqrt")) return arith.tcl_math_sqrt(args[0]);
        if (std.mem.eql(u8, name, "isqrt")) return arith.tcl_math_isqrt(args[0]);
        if (std.mem.eql(u8, name, "bool")) return arith.tcl_math_bool(args[0]);
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
        if (std.mem.eql(u8, name, "isfinite")) return arith.tcl_math_isfinite(args[0]);
        if (std.mem.eql(u8, name, "isinf")) return arith.tcl_math_isinf(args[0]);
        if (std.mem.eql(u8, name, "isnan")) return arith.tcl_math_isnan(args[0]);
        if (std.mem.eql(u8, name, "isnormal")) return arith.tcl_math_isnormal(args[0]);
        if (std.mem.eql(u8, name, "issubnormal")) return arith.tcl_math_issubnormal(args[0]);
        if (std.mem.eql(u8, name, "fpclassify")) return arith.tcl_math_fpclassify(args[0]);
    }
    if (args.len == 2) {
        if (std.mem.eql(u8, name, "pow")) return arith.tcl_arith_pow(args[0], args[1]);
        if (std.mem.eql(u8, name, "isunordered")) return arith.tcl_math_isunordered(args[0], args[1]);
        if (std.mem.eql(u8, name, "atan2")) return arith.tcl_math_atan2(args[0], args[1]);
        if (std.mem.eql(u8, name, "fmod")) return arith.tcl_math_fmod(args[0], args[1]);
        if (std.mem.eql(u8, name, "hypot")) return arith.tcl_math_hypot(args[0], args[1]);
    }
    // Reference Tcl 9 raises a specific surface diagnostic for unknown
    // function names (expr-50.x) and arg-count mismatches on known
    // functions (expr-47.1).  Falling through silently as
    // ``obj_new_int(0)`` would mask user errors.
    if (std.mem.eql(u8, name, "isqrt") or std.mem.eql(u8, name, "bool") or
        std.mem.eql(u8, name, "abs") or std.mem.eql(u8, name, "fabs") or
        std.mem.eql(u8, name, "int") or std.mem.eql(u8, name, "entier") or
        std.mem.eql(u8, name, "wide") or std.mem.eql(u8, name, "double") or
        std.mem.eql(u8, name, "float") or std.mem.eql(u8, name, "round") or
        std.mem.eql(u8, name, "log") or std.mem.eql(u8, name, "log10") or
        std.mem.eql(u8, name, "sqrt") or std.mem.eql(u8, name, "exp") or
        std.mem.eql(u8, name, "sin") or std.mem.eql(u8, name, "cos") or
        std.mem.eql(u8, name, "tan") or std.mem.eql(u8, name, "asin") or
        std.mem.eql(u8, name, "acos") or std.mem.eql(u8, name, "atan") or
        std.mem.eql(u8, name, "sinh") or std.mem.eql(u8, name, "cosh") or
        std.mem.eql(u8, name, "tanh") or std.mem.eql(u8, name, "floor") or
        std.mem.eql(u8, name, "ceil"))
    {
        if (args.len > 1) return raise_too_many_args(name);
        if (args.len < 1) return raise_too_few_args(name);
    }
    if (std.mem.eql(u8, name, "pow") or std.mem.eql(u8, name, "atan2") or
        std.mem.eql(u8, name, "fmod") or std.mem.eql(u8, name, "hypot"))
    {
        if (args.len > 2) return raise_too_many_args(name);
        if (args.len < 2) return raise_too_few_args(name);
    }
    // User-defined math function fallback.  Tcl 9 lets a script
    // install ``proc ::tcl::mathfunc::NAME {args} {...}`` and use
    // ``NAME(...)`` in an expression — ``brodnik.test`` does this
    // to define ``log2`` for its MSB-correctness loop.  Look up the
    // fully-qualified name in the proc registry; on hit, invoke it
    // via ``eval_command`` with the existing ``args`` slice prepended
    // by the command-name obj.  On miss, fall through to the
    // standard "unknown math function" diagnostic.
    if (try_user_math_func(name, args)) |result| return result;
    return raise_unknown_func(name);
}

fn try_user_math_func(name: []const u8, args: []const i32) ?i32 {
    // Reference Tcl's expr mathfunc dispatcher resolves NAME by
    // building ``::tcl::mathfunc::NAME`` and routing through
    // ``Tcl_GetCommandFromObj``, which itself walks the
    // namespacePath / commandPathArray relative to the caller's
    // namespace.  brodnik.test exercises the relative lookup: it
    // sits inside ``::tcl::test::brodnik`` and defines
    // ``namespace eval tcl { namespace eval mathfunc { proc log2
    // ... } }`` — so the proc lands at
    // ``::tcl::test::brodnik::tcl::mathfunc::log2`` rather than the
    // global ``::tcl::mathfunc::log2``.  Our resolver tries the
    // global FQN first (the common case for user-installed
    // helpers) then the current-namespace relative FQN (for
    // namespace-scoped helpers like brodnik's) before giving up.
    const procs = @import("tcl_procs.zig");
    if (!procs.proc_buf_nonzero()) return null;

    // First attempt: ``::tcl::mathfunc::NAME`` — the canonical
    // global mathfunc namespace.
    if (try_dispatch_mathfunc(name, args, .{ .with_current_ns = false })) |r| return r;

    // Second attempt: ``<current_ns>::tcl::mathfunc::NAME`` when
    // we're inside a non-root namespace.  Matches reference Tcl's
    // path-walk semantics for relative mathfunc lookup.
    if (try_dispatch_mathfunc(name, args, .{ .with_current_ns = true })) |r| return r;

    return null;
}

const MathFuncLookup = struct {
    with_current_ns: bool,
};

fn try_dispatch_mathfunc(name: []const u8, args: []const i32, opts: MathFuncLookup) ?i32 {
    const procs = @import("tcl_procs.zig");
    const tcl_ns = @import("tcl_ns.zig");

    // Build the candidate FQN.
    var prefix_bytes: [256]u8 = undefined;
    var prefix_len: u32 = 0;
    if (opts.with_current_ns) {
        // Only meaningful when we're inside a real, non-root ns.
        if (tcl_ns.current_ns == 0 or tcl_ns.current_ns == tcl_ns.ns_root()) return null;
        const cur_ptr = tcl_ns.current_ns_full_ptr();
        const cur_len = tcl_ns.current_ns_full_len();
        if (cur_len < 2 or cur_len + 17 + name.len > prefix_bytes.len) return null;
        const cur_p: [*]const u8 = @ptrFromInt(cur_ptr);
        // Skip the leading ``::`` on the current-ns full name so we
        // don't end up with ``::::ns::tcl::mathfunc``.
        const cur_skip: u32 = if (cur_len >= 2 and cur_p[0] == ':' and cur_p[1] == ':') 2 else 0;
        // ``::`` + current_ns_tail + ``::tcl::mathfunc::``
        prefix_bytes[0] = ':';
        prefix_bytes[1] = ':';
        prefix_len = 2;
        var k: u32 = 0;
        while (k < cur_len - cur_skip) : (k += 1) {
            prefix_bytes[prefix_len + k] = cur_p[cur_skip + k];
        }
        prefix_len += cur_len - cur_skip;
        const tcl_inner = "::tcl::mathfunc::";
        for (tcl_inner, 0..) |c, i| prefix_bytes[prefix_len + i] = c;
        prefix_len += @intCast(tcl_inner.len);
    } else {
        const tcl_global = "::tcl::mathfunc::";
        for (tcl_global, 0..) |c, i| prefix_bytes[i] = c;
        prefix_len = @intCast(tcl_global.len);
    }

    if (prefix_len + name.len > prefix_bytes.len) return null;
    for (name, 0..) |c, i| prefix_bytes[prefix_len + i] = c;
    const total_len: u32 = prefix_len + @as(u32, @intCast(name.len));
    const cmd_obj = obj.obj_new_string_copy(@intFromPtr(&prefix_bytes), total_len);
    if (cmd_obj == 0) return null;

    if (procs.proc_lookup(cmd_obj) == 0) {
        obj.tcl_obj_release(cmd_obj);
        return null;
    }
    const parse_mod = @import("../parse/tcl_parse.zig");
    if (args.len + 1 > parse_mod.MAX_WORDS) {
        obj.tcl_obj_release(cmd_obj);
        return null;
    }
    var new_words: [parse_mod.MAX_WORDS]i32 = undefined;
    new_words[0] = cmd_obj;
    var i: usize = 0;
    while (i < args.len) : (i += 1) new_words[1 + i] = args[i];
    const interp_mod = @import("tcl_interp.zig");
    const result = interp_mod.eval_command(new_words[0 .. args.len + 1]);
    obj.tcl_obj_release(cmd_obj);
    return result;
}

fn raise_math_func_error(prefix: []const u8, name: []const u8, fallback: []const u8) i32 {
    const stubs = @import("../stubs/tcl_stubs.zig");
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + name.len + suffix.len);
    const buf_addr: u32 = obj.alloc(total);
    if (buf_addr == 0) {
        // OOM on the formatted-message buffer — fall back to the
        // static prefix so the diagnostic still surfaces instead of
        // writing through a null pointer.
        stubs.raise(fallback);
        return obj_new_int(0);
    }
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    @memcpy(buf[0..prefix.len], prefix);
    @memcpy(buf[prefix.len .. prefix.len + name.len], name);
    @memcpy(buf[prefix.len + name.len ..][0..suffix.len], suffix);
    stubs.raise(buf[0..total]);
    obj.free_sized(buf_addr, total);
    return obj_new_int(0);
}

fn raise_too_many_args(name: []const u8) i32 {
    return raise_math_func_error(
        "too many arguments for math function \"",
        name,
        "too many arguments for math function",
    );
}

fn raise_too_few_args(name: []const u8) i32 {
    return raise_math_func_error(
        "too few arguments for math function \"",
        name,
        "too few arguments for math function",
    );
}

fn raise_unknown_func(name: []const u8) i32 {
    return raise_math_func_error(
        "unknown math function \"",
        name,
        "unknown math function",
    );
}

fn truthy(o: i32) bool {
    return obj.obj_get_int(o) != 0;
}

/// Strict ``truthy`` that returns null when *o*'s string repr
/// isn't a valid Tcl boolean (a number, or the keywords ``yes`` /
/// ``no`` / ``true`` / ``false`` / ``on`` / ``off``).  Used by
/// the logical operators (``&&``, ``||``, ``!``) and the
/// conditional (``?:``) which raise ``expected boolean value
/// but got "X"`` for non-boolean operands.  Numeric types
/// (TYPE_INT / TYPE_FLOAT / TYPE_BIGNUM) are always boolean;
/// strings dispatch through ``try_parse_int`` / ``try_parse_float`` /
/// ``try_parse_bool`` so ``"0"`` / ``"1.5"`` / ``"true"`` all
/// answer correctly.
fn truthy_strict(o: i32) ?bool {
    // Empty / null operand: Tcl 9 ``expr {"" && 1}`` raises
    // ``expected boolean value but got ""`` rather than treating
    // ``""`` as ``false``.  Return ``null`` so callers route
    // through ``raise_expected_boolean`` / ``raise_non_numeric_unary``.
    if (o == 0) return null;
    // Tagged-immediate small ints and TYPE_INT / TYPE_FLOAT /
    // TYPE_BIGNUM all answer ``boolean`` directly via
    // ``obj_get_int``'s numeric-domain.  String / inline-string
    // forms route through the parse helpers so ``"true"`` /
    // ``"0"`` / ``"1.5"`` all work.
    const handle: u32 = @bitCast(o);
    const is_immediate = (handle & obj.IMMEDIATE_TAG_BIT) != 0;
    if (!is_immediate) {
        const tag = obj.read_i32(handle + obj.OBJ_TYPE_TAG);
        if (tag == obj.TYPE_INT or tag == obj.TYPE_FLOAT or tag == obj.TYPE_BIGNUM) {
            return obj.obj_get_int(o) != 0;
        }
    } else {
        return obj.obj_get_int(o) != 0;
    }
    const s = obj.obj_ensure_string(o);
    if (s.len == 0) return null;
    if (obj.try_parse_int(s.ptr, s.len)) |iv| return iv != 0;
    if (obj.try_parse_float(s.ptr, s.len)) |fv| return fv != 0.0;
    if (obj.try_parse_bool(s.ptr, s.len)) |bv| return bv != 0;
    // Integer literal beyond i64 (``! 10000000000000000000000000``,
    // mathop-3.7 / 3.17) — a non-zero bignum is truthy.
    const bignum = @import("../valtypes/tcl_bignum.zig");
    if (bignum.parse_i128(s.ptr, s.len)) |iv| return iv != 0;
    if (bignum.alloc_from_string(s.ptr, s.len)) |m| {
        defer bignum.destroy(m);
        return !m.eqlZero();
    }
    return null;
}

/// Raise ``cannot use non-numeric string "<text>" as operand of
/// "<op>"`` — used by the unary ``!`` and the AOT logical-not
/// helper.  The wording differs from the binary logical
/// operators which use ``expected boolean value but got "X"``.
fn raise_non_numeric_unary(o: i32, op_sym: []const u8) void {
    const catch_mod = @import("tcl_catch.zig");
    if (@import("tcl_result.zig").snapshot(0).code == .ERROR) return;
    const s = obj.obj_ensure_string(o);
    const prefix: []const u8 = "cannot use non-numeric string \"";
    const middle: []const u8 = "\" as operand of \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + s.len + middle.len + op_sym.len + suffix.len);
    const buf = obj.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(0);
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: usize = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (s.len > 0) {
        const sp: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| {
            dst[off] = sp[i];
            off += 1;
        }
    }
    for (middle) |c| {
        dst[off] = c;
        off += 1;
    }
    for (op_sym) |c| {
        dst[off] = c;
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = obj.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

/// Strict logical-NOT runtime helper exported for the AOT
/// ``UnaryOp.NOT`` codegen.  Validates *operand* as a boolean,
/// raises ``cannot use non-numeric string "X" as operand of
/// "!"`` for non-boolean strings, and returns the boxed
/// negation as a TclObj.  ``operand == 0`` (null TclObj) is
/// treated as ``false`` so the caller can chain through the
/// existing tagged-immediate handle path.
pub export fn tcl_expr_lnot(operand: i32) i32 {
    // Preserve the first error in a chain — when the operand
    // expression already raised (e.g. a missing-variable read
    // upstream), don't wrap the prior error message into a
    // ``cannot use non-numeric string`` diagnostic.  Without
    // this guard, ``info exists arr($x) && !$arr($x)`` whose
    // first arm correctly returned 0 but left a partially
    // populated TclObj on the stack lit up as the operand of
    // ``!`` and produced the bizarre nested-quote diagnostic
    // ``cannot use non-numeric string "can't read ..." as
    // operand of "!"``.
    const result_mod = @import("tcl_result.zig");
    if (result_mod.snapshot(0).code == .ERROR) return obj_new_int(0);
    if (truthy_strict(operand)) |b| {
        return obj_new_int(if (b) @as(i64, 0) else 1);
    }
    raise_non_numeric_unary(operand, "!");
    return obj_new_int(0);
}

/// Raise ``expected boolean value but got "<text>"`` and
/// return ``false``.  Caller must short-circuit the surrounding
/// op after the call.
fn raise_expected_boolean(o: i32) void {
    const catch_mod = @import("tcl_catch.zig");
    const s = obj.obj_ensure_string(o);
    const prefix: []const u8 = "expected boolean value but got \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @as(u32, @intCast(prefix.len)) + s.len + @as(u32, @intCast(suffix.len));
    const buf = obj.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(0);
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (s.len > 0) {
        const sp: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| {
            dst[off] = sp[i];
            off += 1;
        }
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = obj.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}
