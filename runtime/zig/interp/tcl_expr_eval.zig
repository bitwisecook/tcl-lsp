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

const obj_new_int = obj.obj_new_int;
const obj_new_string = obj.obj_new_string;

// Forward-declare the tcl_interp functions we lean on.  They live
// in the parent module; importing here would create a cycle, so
// we ``@import`` lazily inside the functions that need them.

const State = struct {
    src: [*]const u8,
    len: u32,
    pos: u32,
};

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
        const true_branch = parse_ternary(s);
        skip_ws(s);
        if (s.pos < s.len and s.src[s.pos] == ':') s.pos += 1;
        const false_branch = parse_ternary(s);
        if (truthy(cond)) return true_branch;
        return false_branch;
    }
    return cond;
}

fn parse_or(s: *State) i32 {
    var left = parse_and(s);
    while (true) {
        skip_ws(s);
        if (match2(s, '|', '|')) {
            const right = parse_and(s);
            left = obj_new_int(if (truthy(left) or truthy(right)) @as(i64, 1) else 0);
        } else break;
    }
    return left;
}

fn parse_and(s: *State) i32 {
    var left = parse_bit_or(s);
    while (true) {
        skip_ws(s);
        if (match2(s, '&', '&')) {
            const right = parse_bit_or(s);
            left = obj_new_int(if (truthy(left) and truthy(right)) @as(i64, 1) else 0);
        } else break;
    }
    return left;
}

fn parse_bit_or(s: *State) i32 {
    var left = parse_bit_xor(s);
    while (true) {
        skip_ws(s);
        // Reject ``||`` which is the logical-or token at the higher level.
        if (s.pos < s.len and s.src[s.pos] == '|' and (s.pos + 1 >= s.len or s.src[s.pos + 1] != '|')) {
            s.pos += 1;
            const right = parse_bit_xor(s);
            left = arith.tcl_arith_bor(left, right);
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
            left = arith.tcl_arith_bxor(left, right);
        } else break;
    }
    return left;
}

fn parse_bit_and(s: *State) i32 {
    var left = parse_eq_cmp(s);
    while (true) {
        skip_ws(s);
        // Reject ``&&`` which is the logical-and token at the higher level.
        if (s.pos < s.len and s.src[s.pos] == '&' and (s.pos + 1 >= s.len or s.src[s.pos + 1] != '&')) {
            s.pos += 1;
            const right = parse_eq_cmp(s);
            left = arith.tcl_arith_band(left, right);
        } else break;
    }
    return left;
}

fn parse_eq_cmp(s: *State) i32 {
    var left = parse_order_cmp(s);
    while (true) {
        skip_ws(s);
        if (match2(s, '=', '=')) {
            const right = parse_order_cmp(s);
            const cmp = string.tcl_expr_order_cmp(left, right);
            left = obj_new_int(if (obj.obj_get_int(cmp) == 0) @as(i64, 1) else 0);
        } else if (match2(s, '!', '=')) {
            const right = parse_order_cmp(s);
            const cmp = string.tcl_expr_order_cmp(left, right);
            left = obj_new_int(if (obj.obj_get_int(cmp) != 0) @as(i64, 1) else 0);
        } else break;
    }
    return left;
}

fn parse_order_cmp(s: *State) i32 {
    var left = parse_shift(s);
    while (true) {
        skip_ws(s);
        // Order matters: check the 2-char tokens (<= >=) before the
        // 1-char (< > <<) so we don't misparse ``<=`` as ``<`` then
        // a stray ``=``.  Also reject ``<<`` / ``>>`` which are the
        // shift tokens at the higher level.
        if (match2(s, '<', '=')) {
            const right = parse_shift(s);
            const cmp = string.tcl_expr_order_cmp(left, right);
            left = obj_new_int(if (obj.obj_get_int(cmp) <= 0) @as(i64, 1) else 0);
        } else if (match2(s, '>', '=')) {
            const right = parse_shift(s);
            const cmp = string.tcl_expr_order_cmp(left, right);
            left = obj_new_int(if (obj.obj_get_int(cmp) >= 0) @as(i64, 1) else 0);
        } else if (s.pos < s.len and s.src[s.pos] == '<' and (s.pos + 1 >= s.len or s.src[s.pos + 1] != '<')) {
            s.pos += 1;
            const right = parse_shift(s);
            const cmp = string.tcl_expr_order_cmp(left, right);
            left = obj_new_int(if (obj.obj_get_int(cmp) < 0) @as(i64, 1) else 0);
        } else if (s.pos < s.len and s.src[s.pos] == '>' and (s.pos + 1 >= s.len or s.src[s.pos + 1] != '>')) {
            s.pos += 1;
            const right = parse_shift(s);
            const cmp = string.tcl_expr_order_cmp(left, right);
            left = obj_new_int(if (obj.obj_get_int(cmp) > 0) @as(i64, 1) else 0);
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
            left = arith.tcl_arith_lshift(left, right);
        } else if (match2(s, '>', '>')) {
            const right = parse_add(s);
            left = arith.tcl_arith_rshift(left, right);
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
            left = arith.tcl_arith_add(left, right);
        } else if (s.pos < s.len and s.src[s.pos] == '-') {
            s.pos += 1;
            const right = parse_mul(s);
            left = arith.tcl_arith_sub(left, right);
        } else break;
    }
    return left;
}

fn parse_mul(s: *State) i32 {
    var left = parse_pow(s);
    while (true) {
        skip_ws(s);
        // Reject ``**`` which is the power token at the higher level.
        if (s.pos < s.len and s.src[s.pos] == '*' and (s.pos + 1 >= s.len or s.src[s.pos + 1] != '*')) {
            s.pos += 1;
            const right = parse_pow(s);
            left = arith.tcl_arith_mul(left, right);
        } else if (s.pos < s.len and s.src[s.pos] == '/') {
            s.pos += 1;
            const right = parse_pow(s);
            left = arith.tcl_arith_div(left, right);
        } else if (s.pos < s.len and s.src[s.pos] == '%') {
            s.pos += 1;
            const right = parse_pow(s);
            left = arith.tcl_arith_mod(left, right);
        } else break;
    }
    return left;
}

fn parse_pow(s: *State) i32 {
    // ``**`` is right-associative — recurse into ``parse_pow`` for
    // the right operand so ``2 ** 3 ** 4`` parses as
    // ``2 ** (3 ** 4)``.
    const left = parse_unary(s);
    skip_ws(s);
    if (match2(s, '*', '*')) {
        const right = parse_pow(s);
        return arith.tcl_arith_pow(left, right);
    }
    return left;
}

fn parse_unary(s: *State) i32 {
    skip_ws(s);
    if (s.pos >= s.len) return obj_new_int(0);
    const c = s.src[s.pos];
    if (c == '-') {
        s.pos += 1;
        const operand = parse_unary(s);
        return arith.tcl_arith_neg(operand);
    }
    if (c == '+') {
        s.pos += 1;
        return parse_unary(s);
    }
    if (c == '!') {
        s.pos += 1;
        const operand = parse_unary(s);
        return obj_new_int(if (truthy(operand)) @as(i64, 0) else 1);
    }
    if (c == '~') {
        s.pos += 1;
        const operand = parse_unary(s);
        return arith.tcl_arith_bnot(operand);
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
    const slice_ptr = @intFromPtr(s.src) + start;
    const slice_len = s.pos - start;
    const subst = @import("../parse/tcl_subst.zig");
    return subst.subst_flagged(@intCast(slice_ptr), slice_len, true, false, false);
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
    const inner_ptr = @intFromPtr(s.src) + start;
    const interp_mod = @import("tcl_interp.zig");
    const script_obj = obj_new_string(@intCast(inner_ptr), @intCast(inner_len));
    return interp_mod.tcl_eval(script_obj);
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
    const inner_ptr = @intFromPtr(s.src) + start;
    const subst = @import("../parse/tcl_subst.zig");
    return subst.subst_flagged(@intCast(inner_ptr), inner_len, true, true, true);
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
    return obj_new_string(@intCast(inner_ptr), @intCast(inner_len));
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
    return obj_new_string(@intCast(sptr), @intCast(slen));
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
            while (argc < args.len) : (argc += 1) {
                args[argc] = parse_ternary(s);
                skip_ws(s);
                if (s.pos < s.len and s.src[s.pos] == ',') {
                    s.pos += 1;
                    skip_ws(s);
                } else break;
            }
            argc += 1;
        }
        if (s.pos < s.len and s.src[s.pos] == ')') s.pos += 1;
        return dispatch_math_func(name, args[0..argc]);
    }
    // Bare keyword: ``true`` / ``false`` / ``yes`` / ``no`` / ``on`` / ``off``.
    if (parse_bool_keyword(name)) |v| return obj_new_int(v);
    // Otherwise treat as a string literal — matches Tcl's handling
    // of bare words in expressions when they don't match a known
    // function name.
    const sptr = @intFromPtr(s.src) + start;
    return obj_new_string(@intCast(sptr), @intCast(name_len));
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
    }
    if (args.len == 2 and std.mem.eql(u8, name, "pow")) {
        return arith.tcl_arith_pow(args[0], args[1]);
    }
    return obj_new_int(0);
}

fn truthy(o: i32) bool {
    return obj.obj_get_int(o) != 0;
}
