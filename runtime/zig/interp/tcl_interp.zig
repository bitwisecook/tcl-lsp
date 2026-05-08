// Minimal Tcl interpreter — tokeniser, expression evaluator, and eval loop.
//
// Provides tcl_eval(script) for compiled WASM code to fall back to
// when it encounters constructs that can't be statically compiled.
// Shares all runtime functions from tcl_runtime.zig — no duplication.
//
// Design: parse one command at a time, split into words, look up the
// command in a static dispatch table, call the handler.  Expressions
// are evaluated via a simple recursive-descent parser.

const rt = @import("../tcl_runtime.zig");
const procs = @import("tcl_procs.zig");
const frames = @import("tcl_frames.zig");
const info = @import("../cmds/tcl_cmd_info.zig");

const obj_mod = @import("../valtypes/tcl_obj.zig");
const arena = @import("../valtypes/tcl_arena.zig");
const result_mod = @import("tcl_result.zig");

// Re-export runtime functions used throughout this file
const alloc = rt.alloc;
const memcpy = rt.memcpy;
const read_i32 = obj_mod.read_i32;
const write_i32 = obj_mod.write_i32;
const obj_new_string = rt.obj_new_string;
const obj_new_int = rt.obj_new_int;
const obj_get_int = rt.obj_get_int;
const obj_new_string_copy = rt.obj_new_string_copy;
const obj_new_string_take = rt.obj_new_string_take;
const copy_unbraced_elem = rt.copy_unbraced_elem;
const obj_ensure_string = rt.obj_ensure_string;
const list_count_elements = rt.list_count_elements;
const list_element_at = rt.list_element_at;

// Convenience: check if any signal flag is set (error, return, break, continue, yield)
/// Phase 8 follow-up: re-entry counter for ``eval_script`` so
/// nested invocations (command substitutions, ``eval`` bodies,
/// ``subst`` partial brackets, ``uplevel``) don't clobber the
/// outer frame's recorded line number.  Only ``eval_script_depth
/// == 1`` (the outermost call) drives ``frame_set_line``; nested
/// calls leave the slot alone so an outer compiled stamp survives
/// while a sub-eval runs to completion.  ``info frame -line``
/// reads what the outer stamp wrote, matching reference Tcl's
/// notion of "the currently-executing command in the proc body".
var eval_script_depth: u32 = 0;

/// Phase 8 follow-up: count 1-based line number of byte offset
/// ``pos`` within the script buffer at ``src``.  Each ``\n`` bumps
/// the count by one; the first line is 1.  ``\r\n`` and lone ``\r``
/// don't bump separately — Tcl's parser itself only treats ``\n``
/// as a command terminator.  Linear scan, O(pos) per call; eval_script
/// invokes this once per parsed command, so the cumulative cost is
/// O(N²) worst-case for huge scripts.  The cached-slab replay path
/// stores the per-command line in the slab record so cached bodies
/// pay O(1).
fn count_lines_to(src: [*]const u8, pos: u32) u32 {
    var line: u32 = 1;
    var i: u32 = 0;
    while (i < pos) : (i += 1) {
        if (src[i] == '\n') line += 1;
    }
    return line;
}

fn has_signal() bool {
    const tcl_catch = @import("tcl_catch.zig");
    return result_mod.has_signal(result_mod.snapshot(0)) or
        tcl_catch.state.yield_flag != 0;
}

// -- Tokeniser --
// Splits a Tcl script into commands, each command into words.
// Handles: braces {}, quotes "", $var substitution, [cmd] substitution,
// backslash escapes, semicolons, newlines.
//
// The actual parsers live in ``tcl_parse.zig`` so both the interpreter
// and any future Tcl-Parse-tree consumer see one canonical tokeniser.
// ``eval_script`` consumes the Token-tree API (``parse.ParseCommand``);
// the legacy flat-array helpers (``parse_command`` / ``parse_braced``
// / …) are still exported from ``tcl_parse.zig`` for callers that
// want them, but nothing inside ``tcl_interp.zig`` uses them directly
// any more — so no local aliases here.

const parse = @import("../parse/tcl_parse.zig");
const MAX_WORDS: u32 = parse.MAX_WORDS;
const interp_impl = @import("../cmds/tcl_cmd_interp.zig");
const cmd_table = @import("../dispatch/tcl_cmd_table.zig");
const interp_reg = @import("tcl_interp_registry.zig");

// -- Variable substitution --

// ``encode_utf8`` lives in tcl_obj.zig as a shared helper used by both
// the interpreter's ``subst_flagged`` and the list-element decoder
// ``copy_unbraced_elem``.  Kept there so changes to the UTF-8 tables
// only happen in one place.

/// Expand ``$var`` / ``[cmd]`` / ``\x`` in a word.  Always performs
/// all three substitutions — this is the tokenizer's word-expansion
/// path called after parse_command.  Braced words skip this entirely
/// (see eval_script); the ``subst`` command uses :func:`subst_flagged`
/// for its flag-controlled variant.
fn subst_word(wptr: u32, wlen: u32) i32 {
    return subst_flagged(wptr, wlen, true, true, true);
}

/// Same as :func:`subst_word` but each substitution kind is
/// individually enabled.  Extracted to tcl_subst.zig; aliased here so
/// the rest of this file can call it unchanged.
const tcl_subst = @import("../parse/tcl_subst.zig");
const subst_flagged = tcl_subst.subst_flagged;

// -- Expression evaluator --
// Recursive-descent: +, -, *, /, %, ==, !=, <, >, <=, >=, unary -, (), $var, [cmd]

pub fn eval_expr_str(ptr: u32, len: u32) i64 {
    // Tcl's ``eq`` / ``ne`` / ``in`` / ``ni`` operators take string
    // operands, not numeric — but the legacy ``expr_*`` chain below is
    // a pure-numeric evaluator that loses the string form once it
    // reads an atom.  Pre-scan the expression for a top-level string
    // operator and dispatch through ``eval_string_expr`` when we see
    // one; that helper substitutes both sides via ``subst_word`` and
    // does a real string compare.  Without this dispatch, ``expr
    // {$token ne {}}`` parses ``$token`` as 0 (string ``"v1"`` is not
    // a number), then sees the unknown ``ne`` operator, returns 0,
    // and ``catch {$token ne {}}`` always evaluates false.  See the
    // ``::tcltest::SubstArguments`` smoke trace in the cmdAH triage
    // for a worked example.
    if (find_top_string_op(ptr, len)) |op| {
        return eval_string_expr(ptr, len, op);
    }
    var pos: u32 = 0;
    return expr_or(ptr, len, &pos, false);
}

/// One of the four Tcl string-comparison / list-membership operators
/// in ``expr``.  The kind is enough to dispatch the comparison; the
/// position lets the caller split the expression text into LHS / RHS
/// substrings.
const StringOp = struct {
    kind: enum { eq, ne, in_, ni },
    /// First byte of the operator keyword in the expression text.
    start: u32,
    /// Byte length of the keyword (2 for all four operators).
    op_len: u32,
};

/// Walk *ptr*[0..*len*] looking for a top-level ``eq`` / ``ne`` / ``in``
/// / ``ni`` operator.  Skips over ``"..."``, ``{...}``, ``[...]``,
/// ``(...)`` so nested constructs don't trip the match, and bails out
/// at the first higher- or equal-precedence boolean / ternary operator
/// (``&&``, ``||``, ``?``, ``:``) so chained expressions like
/// ``expr {$a eq $b && $c}`` or ``expr {$a eq $b ? 1 : 0}`` are
/// recognised as numeric expressions and dispatched through the
/// regular ``expr_or`` pipeline (which itself recurses back into
/// ``eval_expr_str`` for each side, picking up the string-op fix
/// where it actually applies).
fn find_top_string_op(ptr: u32, len: u32) ?StringOp {
    if (len < 4) return null;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    while (i < len) : (i += 1) {
        const c = src[i];
        // Skip nested constructs so an inner ``eq`` doesn't get picked.
        if (c == '"') {
            i += 1;
            while (i < len and src[i] != '"') {
                if (src[i] == '\\' and i + 1 < len) i += 1;
                i += 1;
            }
            continue;
        }
        if (c == '{') {
            var depth: u32 = 1;
            i += 1;
            while (i < len and depth > 0) : (i += 1) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 1;
                    continue;
                }
                if (src[i] == '{') depth += 1 else if (src[i] == '}') depth -= 1;
            }
            // ``i`` now points one past the closing brace; the loop
            // ``i += 1`` will advance once more, so step back.
            if (i > 0) i -= 1;
            continue;
        }
        if (c == '[') {
            var depth: u32 = 1;
            i += 1;
            while (i < len and depth > 0) : (i += 1) {
                if (src[i] == '[') depth += 1 else if (src[i] == ']') depth -= 1;
            }
            if (i > 0) i -= 1;
            continue;
        }
        if (c == '(') {
            var depth: u32 = 1;
            i += 1;
            while (i < len and depth > 0) : (i += 1) {
                if (src[i] == '(') depth += 1 else if (src[i] == ')') depth -= 1;
            }
            if (i > 0) i -= 1;
            continue;
        }
        // Bail out at higher- or equal-precedence boolean / ternary
        // operators so the string-op shortcut only fires for atom-
        // level ``$a eq $b`` shapes.  Inside a ``&&`` or ``?:``
        // expression the regular numeric ``expr_or`` chain handles
        // each operand and recurses back here for each subexpression.
        if (i + 1 < len and src[i] == '&' and src[i + 1] == '&') return null;
        if (i + 1 < len and src[i] == '|' and src[i + 1] == '|') return null;
        if (c == '?' or c == ':') return null;
        // String operator must be surrounded by whitespace on both
        // sides — otherwise a variable name like ``$equal`` would
        // self-match its ``eq`` substring.  Two-char keyword + at
        // least one trailing byte (the RHS start).
        if (c != ' ' and c != '\t') continue;
        if (i + 3 >= len) break;
        const ch1 = src[i + 1];
        const ch2 = src[i + 2];
        const ch3 = src[i + 3];
        if (ch3 != ' ' and ch3 != '\t') continue;
        if (ch1 == 'e' and ch2 == 'q') {
            return .{ .kind = .eq, .start = i + 1, .op_len = 2 };
        }
        if (ch1 == 'n' and ch2 == 'e') {
            return .{ .kind = .ne, .start = i + 1, .op_len = 2 };
        }
        if (ch1 == 'i' and ch2 == 'n') {
            return .{ .kind = .in_, .start = i + 1, .op_len = 2 };
        }
        if (ch1 == 'n' and ch2 == 'i') {
            return .{ .kind = .ni, .start = i + 1, .op_len = 2 };
        }
    }
    return null;
}

/// Evaluate a ``$LHS op $RHS`` expression where *op* is one of the
/// four string-domain operators.  Substitutes each side via
/// ``subst_word`` (so ``$var`` and ``[cmd]`` resolve to their string
/// values), then compares.  ``in`` / ``ni`` treat the RHS as a Tcl
/// list and check membership.
fn eval_string_expr(ptr: u32, len: u32, op: StringOp) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    // Trim surrounding whitespace from each side.
    var lhs_start: u32 = 0;
    var lhs_end: u32 = op.start;
    while (lhs_start < lhs_end and (src[lhs_start] == ' ' or src[lhs_start] == '\t')) lhs_start += 1;
    while (lhs_end > lhs_start and (src[lhs_end - 1] == ' ' or src[lhs_end - 1] == '\t')) lhs_end -= 1;
    var rhs_start: u32 = op.start + op.op_len;
    var rhs_end: u32 = len;
    while (rhs_start < rhs_end and (src[rhs_start] == ' ' or src[rhs_start] == '\t')) rhs_start += 1;
    while (rhs_end > rhs_start and (src[rhs_end - 1] == ' ' or src[rhs_end - 1] == '\t')) rhs_end -= 1;
    // ``subst_word`` resolves ``$var`` / ``[cmd]`` / backslash escapes
    // but doesn't strip outer ``{...}`` braces — those are a syntactic
    // hint that the contents are literal.  Strip them on each side
    // before subst so ``{}`` evaluates to the empty string and
    // ``{abc}`` evaluates to ``abc``.  Without the strip,
    // ``$token ne {}`` compared the (substituted) ``$token`` against
    // the literal two-char string ``{}`` and never matched the empty
    // string for which it stands.
    var ls = lhs_start;
    var le = lhs_end;
    var rs = rhs_start;
    var re = rhs_end;
    if (le >= ls + 2 and src[ls] == '{' and src[le - 1] == '}') {
        ls += 1;
        le -= 1;
    }
    if (re >= rs + 2 and src[rs] == '{' and src[re - 1] == '}') {
        rs += 1;
        re -= 1;
    }
    const lhs_obj = subst_word(ptr + ls, le - ls);
    const rhs_obj = subst_word(ptr + rs, re - rs);
    const lhs_raw = obj_ensure_string(lhs_obj);
    const rhs_raw = obj_ensure_string(rhs_obj);
    const lhs_s: StringSpan = .{ .ptr = lhs_raw.ptr, .len = lhs_raw.len };
    const rhs_s: StringSpan = .{ .ptr = rhs_raw.ptr, .len = rhs_raw.len };
    var result: i64 = 0;
    switch (op.kind) {
        .eq => result = if (string_bytes_equal(lhs_s, rhs_s)) @as(i64, 1) else @as(i64, 0),
        .ne => result = if (string_bytes_equal(lhs_s, rhs_s)) @as(i64, 0) else @as(i64, 1),
        .in_ => result = if (list_contains_string(rhs_s, lhs_s)) @as(i64, 1) else @as(i64, 0),
        .ni => result = if (list_contains_string(rhs_s, lhs_s)) @as(i64, 0) else @as(i64, 1),
    }
    obj_mod.tcl_obj_release(lhs_obj);
    obj_mod.tcl_obj_release(rhs_obj);
    return result;
}

const StringSpan = struct { ptr: u32, len: u32 };

fn string_bytes_equal(a: StringSpan, b: StringSpan) bool {
    if (a.len != b.len) return false;
    if (a.len == 0) return true;
    const ap: [*]const u8 = @ptrFromInt(a.ptr);
    const bp: [*]const u8 = @ptrFromInt(b.ptr);
    var i: u32 = 0;
    while (i < a.len) : (i += 1) {
        if (ap[i] != bp[i]) return false;
    }
    return true;
}

fn list_contains_string(list: StringSpan, needle: StringSpan) bool {
    const n = rt.list_count_elements(list.ptr, list.len);
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        const elem = rt.list_element_at(list.ptr, list.len, idx);
        // Decode the element's actual byte content (handle braced
        // quoting) before comparing.
        if (elem.braced) {
            const span: StringSpan = .{ .ptr = list.ptr + elem.start, .len = elem.len };
            if (string_bytes_equal(span, needle)) return true;
        } else {
            // Unbraced: backslash sequences need decoding.  Allocate
            // a tiny scratch buffer and use ``copy_unbraced_elem``
            // (the canonical decoder used by ``Tcl_SplitList``).
            const scratch = obj_mod.alloc(elem.len + 1);
            if (scratch == 0) return false;
            const real_len = rt.copy_unbraced_elem(scratch, list.ptr + elem.start, elem.len);
            const decoded: StringSpan = .{ .ptr = scratch, .len = real_len };
            const matched = string_bytes_equal(decoded, needle);
            obj_mod.free_sized(scratch, elem.len + 1);
            if (matched) return true;
        }
    }
    return false;
}

/// Raise ``expected boolean value but got "<text>"`` from the
/// legacy expr atom evaluator.  Used by the quoted-string branch
/// (``"YES"`` / ``"abc"`` etc.) when the operand isn't a numeric
/// or boolean literal — reference Tcl 9 surfaces this exact text
/// at the if / while / && / || / ?: callsites (expr-old-28.12 ..
/// 28.14).  Compiled-context tasks already raise this through
/// ``tcl_expr_eval.raise_expected_boolean``; this version is the
/// runtime fallback for the interpreter.
fn raise_expected_boolean_str(ptr: u32, slen: u32) void {
    const catch_mod = @import("tcl_catch.zig");
    if (@import("tcl_result.zig").snapshot(0).code == .ERROR) return;
    const prefix: []const u8 = "expected boolean value but got \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @as(u32, @intCast(prefix.len)) + slen + @as(u32, @intCast(suffix.len));
    const buf = obj_mod.alloc(total);
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
    if (slen > 0) {
        const sp: [*]const u8 = @ptrFromInt(ptr);
        for (0..slen) |i| {
            dst[off] = sp[i];
            off += 1;
        }
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

/// Short-circuit evaluation: when *skip* is true the expression walks the
/// tokens to advance ``pos`` but does NOT run ``[cmd]`` substitutions —
/// so ``{[info exists x] && [use $x]}`` only runs ``[use $x]`` when the
/// first operand is true.  Atoms like ``$var`` are read-only so they
/// evaluate normally even in skip mode (the alternative — routing
/// reads through a no-op path — buys nothing because variable lookup
/// has no observable side effect).
fn expr_or(ptr: u32, len: u32, pos: *u32, skip: bool) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var left = expr_and(ptr, len, pos, skip);
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* + 1 < len and src[pos.*] == '|' and src[pos.* + 1] == '|') {
            pos.* += 2;
            // Short-circuit: skip RHS when LHS is already truthy OR we're
            // already skipping outer scope.  Still walk the RHS tokens
            // so ``pos`` advances past them.
            const rhs_skip = skip or (left != 0);
            const right = expr_and(ptr, len, pos, rhs_skip);
            if (!skip) {
                left = if (left != 0 or right != 0) @as(i64, 1) else @as(i64, 0);
            }
        } else break;
    }
    return left;
}

fn expr_and(ptr: u32, len: u32, pos: *u32, skip: bool) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var left = expr_add(ptr, len, pos, skip);
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* + 1 < len and src[pos.*] == '&' and src[pos.* + 1] == '&') {
            pos.* += 2;
            // Short-circuit: skip RHS when LHS is already falsy OR we're
            // already skipping.
            const rhs_skip = skip or (left == 0);
            const right = expr_add(ptr, len, pos, rhs_skip);
            if (!skip) {
                left = if (left != 0 and right != 0) @as(i64, 1) else @as(i64, 0);
            }
        } else break;
    }
    return left;
}

fn expr_skip_ws(src: [*]const u8, len: u32, pos: *u32) void {
    while (pos.* < len and (src[pos.*] == ' ' or src[pos.*] == '\t')) pos.* += 1;
}

fn expr_add(ptr: u32, len: u32, pos: *u32, skip: bool) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var left = expr_mul(ptr, len, pos, skip);
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* >= len) break;
        if (src[pos.*] == '+') {
            pos.* += 1;
            left = left + expr_mul(ptr, len, pos, skip);
        } else if (src[pos.*] == '-') {
            pos.* += 1;
            left = left - expr_mul(ptr, len, pos, skip);
        } else if (pos.* + 1 < len and src[pos.*] == '=' and src[pos.* + 1] == '=') {
            pos.* += 2;
            left = if (left == expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0);
        } else if (pos.* + 1 < len and src[pos.*] == '!' and src[pos.* + 1] == '=') {
            pos.* += 2;
            left = if (left != expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0);
        } else if (pos.* + 1 < len and src[pos.*] == '<' and src[pos.* + 1] == '=') {
            pos.* += 2;
            left = if (left <= expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0);
        } else if (pos.* + 1 < len and src[pos.*] == '>' and src[pos.* + 1] == '=') {
            pos.* += 2;
            left = if (left >= expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0);
        } else if (src[pos.*] == '<') {
            pos.* += 1;
            left = if (left < expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0);
        } else if (src[pos.*] == '>') {
            pos.* += 1;
            left = if (left > expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0);
        } else break;
    }
    return left;
}

fn expr_mul(ptr: u32, len: u32, pos: *u32, skip: bool) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var left = expr_atom(ptr, len, pos, skip);
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* >= len) break;
        if (src[pos.*] == '*') {
            pos.* += 1;
            left = left * expr_atom(ptr, len, pos, skip);
        } else if (src[pos.*] == '/') {
            pos.* += 1;
            const r = expr_atom(ptr, len, pos, skip);
            left = if (r != 0) @divTrunc(left, r) else 0;
        } else if (src[pos.*] == '%') {
            pos.* += 1;
            const r = expr_atom(ptr, len, pos, skip);
            left = if (r != 0) @rem(left, r) else 0;
        } else break;
    }
    return left;
}

fn expr_atom(ptr: u32, len: u32, pos: *u32, skip: bool) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    expr_skip_ws(src, len, pos);
    if (pos.* >= len) return 0;
    if (src[pos.*] == '!') {
        pos.* += 1;
        return if (expr_atom(ptr, len, pos, skip) != 0) @as(i64, 0) else @as(i64, 1);
    }
    if (src[pos.*] == '~') {
        pos.* += 1;
        return ~expr_atom(ptr, len, pos, skip);
    }
    if (src[pos.*] == '-') {
        pos.* += 1;
        return -expr_atom(ptr, len, pos, skip);
    }
    if (src[pos.*] == '(') {
        pos.* += 1;
        const val = expr_or(ptr, len, pos, skip);
        expr_skip_ws(src, len, pos);
        if (pos.* < len and src[pos.*] == ')') pos.* += 1;
        return val;
    }
    // String literal "...": advance past closing quote and parse the
    // content as an integer / float / boolean.  In Tcl expressions
    // ``"5"`` evaluates to 5 and ``"YES"`` evaluates to 1 (true).
    // Falling back to 0 silently made ``if {"YES"}`` test false
    // (expr-old-28.6 / 28.8 / 28.10).  When the string isn't any of
    // those forms the operand is invalid in the surrounding numeric
    // / boolean context — raise ``expected boolean value but got
    // "X"`` (expr-old-28.12 / 28.13 / 28.14).  String-equality
    // operators (``eq`` / ``ne`` / ``in`` / ``ni``) bypass this
    // atom path via :func:`find_top_string_op` so this return value
    // only surfaces when the literal is used in a numeric / boolean
    // slot.
    if (src[pos.*] == '"') {
        pos.* += 1;
        const content_start = pos.*;
        while (pos.* < len and src[pos.*] != '"') {
            if (src[pos.*] == '\\' and pos.* + 1 < len) pos.* += 1;
            pos.* += 1;
        }
        const content_end = pos.*;
        if (pos.* < len) pos.* += 1; // closing quote
        const clen: u32 = content_end - content_start;
        if (obj_mod.try_parse_int(ptr + content_start, clen)) |v| {
            return v;
        }
        if (obj_mod.try_parse_float(ptr + content_start, clen)) |fv| {
            return if (fv != 0.0) @as(i64, 1) else @as(i64, 0);
        }
        if (obj_mod.try_parse_bool(ptr + content_start, clen)) |bv| {
            return bv;
        }
        // Not a recognised numeric or boolean literal — raise the
        // canonical ``expected boolean value but got "X"`` so callers
        // (``if`` / ``while`` / ``&&`` / ``||`` / ``?:``) report the
        // same surface as reference Tcl.
        raise_expected_boolean_str(ptr + content_start, clen);
        return 0;
    }
    if (src[pos.*] == '$') {
        pos.* += 1;
        const vs = pos.*;
        while (pos.* < len and ((src[pos.*] >= 'a' and src[pos.*] <= 'z') or
            (src[pos.*] >= 'A' and src[pos.*] <= 'Z') or
            (src[pos.*] >= '0' and src[pos.*] <= '9') or src[pos.*] == '_'))
        {
            pos.* += 1;
        }
        const name = obj_new_string(@bitCast(ptr + vs), @bitCast(pos.* - vs));
        // Issue #303 — release the per-atom name temp.
        // ``var_resolve`` returns a borrowed handle to the variable
        // storage; the lookup-only name TclObj is ours to free.
        // Without the release, every ``$var`` reference inside an
        // expression leaks one obj header per iteration.
        const val = frames.var_resolve(name);
        const v: i64 = if (val != 0) obj_get_int(val) else 0;
        obj_mod.tcl_obj_release(name);
        return v;
    }
    if (src[pos.*] == '[') {
        pos.* += 1;
        const cs = pos.*;
        var depth: u32 = 1;
        while (pos.* < len and depth > 0) {
            if (src[pos.*] == '[') depth += 1 else if (src[pos.*] == ']') depth -= 1;
            if (depth > 0) pos.* += 1 else pos.* += 1;
        }
        // Short-circuit: when the enclosing ``||`` or ``&&`` already knows
        // the result, skip the command substitution body entirely — the
        // tokens are consumed above, but ``eval_script`` would run
        // side-effecting commands that Tcl's short-circuit semantics
        // require us to avoid.
        if (skip) return 0;
        const result = eval_script(ptr + cs, pos.* - 1 - cs);
        // Codex review on PR #319 (issue #303): do NOT release
        // ``result`` here — ``eval_script`` can return a *borrowed*
        // handle into a variable's live storage (``[set x]`` read
        // mode goes through ``frames.var_resolve`` and forwards
        // that handle directly), and a release would decrement the
        // var slot's live refcount and free the value still stored
        // there.  The intermediate-result discipline can only run
        // at sites that are guaranteed to see +1-owned returns.
        // The narrow ``$var`` ``name`` release a few lines up is
        // safe because that name TclObj is freshly allocated by
        // this function, not borrowed.
        if (result != 0) return obj_get_int(result);
        return 0;
    }
    var negative = false;
    if (src[pos.*] == '+') pos.* += 1;
    if (pos.* < len and src[pos.*] == '-') {
        negative = true;
        pos.* += 1;
    }
    var val: i64 = 0;
    // Hex literal: 0x...
    if (pos.* + 1 < len and src[pos.*] == '0' and (src[pos.* + 1] == 'x' or src[pos.* + 1] == 'X')) {
        pos.* += 2;
        while (pos.* < len) {
            const c = src[pos.*];
            if (c >= '0' and c <= '9') {
                val = val * 16 + @as(i64, c - '0');
                pos.* += 1;
            } else if (c >= 'a' and c <= 'f') {
                val = val * 16 + @as(i64, c - 'a' + 10);
                pos.* += 1;
            } else if (c >= 'A' and c <= 'F') {
                val = val * 16 + @as(i64, c - 'A' + 10);
                pos.* += 1;
            } else break;
        }
        return if (negative) -val else val;
    }
    // Octal literal: 0o...
    if (pos.* + 1 < len and src[pos.*] == '0' and (src[pos.* + 1] == 'o' or src[pos.* + 1] == 'O')) {
        pos.* += 2;
        while (pos.* < len and src[pos.*] >= '0' and src[pos.*] <= '7') {
            val = val * 8 + @as(i64, src[pos.*] - '0');
            pos.* += 1;
        }
        return if (negative) -val else val;
    }
    // Decimal integer
    while (pos.* < len and src[pos.*] >= '0' and src[pos.*] <= '9') {
        val = val * 10 + @as(i64, src[pos.*] - '0');
        pos.* += 1;
    }
    return if (negative) -val else val;
}

// -- Command dispatch --

/// Per-interp counter of commands dispatched.  Mirrors C Tcl's
/// ``Interp.cmdCount`` (see ``tclCmdIL.c``).  Read by
/// ``info cmdcount``; incremented for every dispatched command —
/// builtin, proc, alias, or stub-trap path.  Compiled procs that
/// bypass the dispatcher entirely (the AOT fast path) are NOT
/// counted, matching the historical contract that ``info cmdcount``
/// reports interpreter-visible commands; if a future test exposes
/// the discrepancy, increment in the compiled-proc prologue too.
pub var cmd_count: i64 = 0;

fn eval_command(words: []const i32) i32 {
    if (words.len == 0) return 0;
    cmd_count +%= 1;
    const cmd_s = obj_ensure_string(words[0]);
    if (cmd_s.len == 0) return 0;

    // Fast path: probe the proc registry first.  Tcl semantics say a
    // user-defined proc shadows a built-in, and for dispatch-heavy
    // test bundles (tcltest calling ::tcltest::preserveCore,
    // ::tcltest::temporaryDirectory, etc. — and the test body itself
    // calling ``test`` which resolves to ``::tcltest::test``) the proc
    // path wins 10× more often than a builtin would.  ``proc_lookup``
    // is O(1) via the MRU cache + open-addressed hash and short-
    // circuits on ``proc_buf == 0``, so the miss cost on a bundle
    // with no procs is a single ``i32`` load.  The builtin chain
    // below still runs on miss so nothing else changes.
    if (procs.proc_buf_nonzero()) {
        const bucket = procs.proc_lookup(words[0]);
        if (bucket != 0) return eval_proc_call_bucket(words, bucket);
    }

    // Registered builtin dispatch — all builtin commands are in per-module
    // files under cmds/ and assembled in tcl_cmd_table.zig.  Handlers now
    // return :type:`InterpResult`; the legacy globals are still set as
    // a side effect of ``tcl_cmd_error`` etc., so we extract ``.value``
    // for the i32-returning ``eval_command`` contract and let the
    // surrounding ``snapshot``-based inspection observe the signal.
    if (cmd_table.lookup(cmd_s.ptr, cmd_s.len)) |handler| return handler(words).value;

    // ``::concat``, ``::expr``, etc. — fully-qualified names for
    // root-namespace builtins.  Strip the leading ``::`` and retry
    // the builtin table.  tcltest's ``Eval`` uses this form
    // (``uplevel 1 ::concat $body``) and without the strip it
    // surfaces as ``unknown command: ::concat`` mid-test.
    if (cmd_s.len >= 2) {
        const cmd_p: [*]const u8 = @ptrFromInt(cmd_s.ptr);
        if (cmd_p[0] == ':' and cmd_p[1] == ':') {
            if (cmd_table.lookup(cmd_s.ptr + 2, cmd_s.len - 2)) |handler| {
                return handler(words).value;
            }
            // Ensemble FQ rewrites — the CFG canonicalises
            // ``dict for`` to ``::tcl::dict::for`` (and friends).
            // Detect the ``::tcl::dict::`` / ``::tcl::string::`` /
            // ``::tcl::info::`` prefixes and re-dispatch as the
            // ensemble form by synthesising a ``words[]`` slice
            // with the subcommand spliced in.
            if (try_ensemble_rewrite(cmd_s, words)) |result| return result;
        } else if (cmd_s.len >= 5 and cmd_p[0] == 't' and cmd_p[1] == 'c' and
            cmd_p[2] == 'l' and cmd_p[3] == ':' and cmd_p[4] == ':')
        {
            // Relative-qualified ``tcl::ENSEMBLE::SUBCMD`` from the
            // global namespace — Tcl resolves this to
            // ``::tcl::ENSEMBLE::SUBCMD``.  Upstream string-31.x and
            // string-24.x test bodies invoke ``tcl::string::insert``
            // / ``tcl::string::reverse`` directly, which would
            // otherwise miss the FQ rewrite above.  Re-dispatch via
            // the same ensemble route.
            if (try_ensemble_rewrite_relative(cmd_s, words)) |result| return result;
        }
    }

    // -- Proc dispatch: check registry before erroring --
    return eval_proc_call(words);
}

/// Recognise ``::tcl::ENSEMBLE::SUBCMD`` (the canonical FQ rewrite
/// form the CFG emits for compiled ensembles like ``dict for`` →
/// ``::tcl::dict::for``) and re-dispatch as the BUILTIN form
/// ``ENSEMBLE SUBCMD ...``.
///
/// Returns ``null`` when *cmd_s* doesn't match a known ensemble, or
/// the handler's i32 result when it does.  The synthesised words[]
/// slice has ``ENSEMBLE`` at slot 0 and ``SUBCMD`` at slot 1; the
/// caller's args[1..] follow at slot 2.
fn try_ensemble_rewrite(cmd_s: anytype, words: []const i32) ?i32 {
    if (cmd_s.len < 11) return null; // ``::tcl::X::Y`` is 11 minimum
    const p: [*]const u8 = @ptrFromInt(cmd_s.ptr);
    // ``::tcl::`` prefix
    const prefix = "::tcl::";
    inline for (prefix, 0..) |c, i| {
        if (p[i] != c) return null;
    }
    // Find the second ``::`` to split ENSEMBLE from SUBCMD.
    var i: u32 = prefix.len;
    var ens_end: u32 = 0;
    while (i + 1 < cmd_s.len) : (i += 1) {
        if (p[i] == ':' and p[i + 1] == ':') {
            ens_end = i;
            break;
        }
    }
    if (ens_end == 0 or ens_end + 2 >= cmd_s.len) return null;
    const ens_ptr = cmd_s.ptr + prefix.len;
    const ens_len: u32 = ens_end - prefix.len;
    const sub_ptr = cmd_s.ptr + ens_end + 2;
    const sub_len: u32 = cmd_s.len - ens_end - 2;

    // Confirm the ENSEMBLE name resolves to a real BUILTIN (else this
    // could swallow an unrelated ``::tcl::pkg::path``-style name).
    const handler = cmd_table.lookup(ens_ptr, ens_len) orelse return null;

    // Build a synthesised words[] with the ensemble at [0], subcmd at
    // [1], and the caller's args[1..] at [2..].  Use the bump
    // allocator — the slice lives only for the handler call.
    const total: u32 = @as(u32, @intCast(words.len)) + 1;
    const buf = obj_mod.alloc(total * 4);
    const slot: [*]i32 = @ptrFromInt(buf);
    slot[0] = obj_mod.obj_new_string(@bitCast(ens_ptr), @bitCast(ens_len));
    slot[1] = obj_mod.obj_new_string(@bitCast(sub_ptr), @bitCast(sub_len));
    var k: u32 = 1;
    while (k < words.len) : (k += 1) {
        slot[k + 1] = words[k];
    }
    return handler(slot[0..total]).value;
}

/// Same as :fn:`try_ensemble_rewrite` but for the relative-qualified
/// form ``tcl::ENSEMBLE::SUBCMD`` (no leading ``::``).  Tcl resolves
/// such a name from the global namespace as if it were
/// ``::tcl::ENSEMBLE::SUBCMD``; this helper is the runtime hook so
/// upstream tests that invoke ``tcl::string::insert`` /
/// ``tcl::string::reverse`` directly land on the same dispatch path
/// as the ``::``-prefixed form.
fn try_ensemble_rewrite_relative(cmd_s: anytype, words: []const i32) ?i32 {
    if (cmd_s.len < 9) return null; // ``tcl::X::Y`` is 9 minimum
    const p: [*]const u8 = @ptrFromInt(cmd_s.ptr);
    const prefix = "tcl::";
    inline for (prefix, 0..) |c, i| {
        if (p[i] != c) return null;
    }
    var i: u32 = prefix.len;
    var ens_end: u32 = 0;
    while (i + 1 < cmd_s.len) : (i += 1) {
        if (p[i] == ':' and p[i + 1] == ':') {
            ens_end = i;
            break;
        }
    }
    if (ens_end == 0 or ens_end + 2 >= cmd_s.len) return null;
    const ens_ptr = cmd_s.ptr + prefix.len;
    const ens_len: u32 = ens_end - prefix.len;
    const sub_ptr = cmd_s.ptr + ens_end + 2;
    const sub_len: u32 = cmd_s.len - ens_end - 2;

    const handler = cmd_table.lookup(ens_ptr, ens_len) orelse return null;

    const total: u32 = @as(u32, @intCast(words.len)) + 1;
    const buf = obj_mod.alloc(total * 4);
    const slot: [*]i32 = @ptrFromInt(buf);
    slot[0] = obj_mod.obj_new_string(@bitCast(ens_ptr), @bitCast(ens_len));
    slot[1] = obj_mod.obj_new_string(@bitCast(sub_ptr), @bitCast(sub_len));
    var k: u32 = 1;
    while (k < words.len) : (k += 1) {
        slot[k + 1] = words[k];
    }
    return handler(slot[0..total]).value;
}

const str_eq = @import("../valtypes/tcl_chars.zig").str_eq;

const tcl_ns = @import("tcl_ns.zig");
const alias_mod = @import("../cmds/tcl_alias.zig");

// Namespace context for eval-fallback calls.  Storage lives in
// ``tcl_ns.current_ns`` (moved there in P2.1 so ``tcl_procs.zig``
// can read it without circular-importing ``tcl_interp.zig``).
// Zero means "no namespace context active" — equivalent to the
// root ns for resolution but treated as "leave names unqualified"
// by ``qualify_name`` so legacy callers that never call ``ns_set``
// keep their old behaviour.

/// Set the current namespace.  ``name_ptr`` / ``name_len`` are a
/// fully-qualified name like ``::tcltest`` that the runtime walks
/// (find-or-create) to obtain the corresponding ``*Namespace``
/// handle.
///
/// Returns the previously-active handle packed into the low 32 bits
/// of an i64 — keeping the i64 ABI matching the compiler-side
/// import declaration (``[I32, I32] -> I64`` in
/// ``codegen/wasm/_imports.py``).  The high 32 bits are unused
/// (always 0) to leave room for future flags without another ABI
/// flip.
pub export fn ns_set(name_ptr: i32, name_len: i32) i64 {
    const saved: u32 = tcl_ns.current_ns;
    const ns = tcl_ns.ns_create_from_fqn(@bitCast(name_ptr), @bitCast(name_len));
    tcl_ns.current_ns = ns;
    // Record the caller's namespace on the current proc frame so a
    // later ``uplevel`` can restore it.  Only stamps when the slot
    // hasn't been set yet — eval-fallback regions inside a compiled
    // proc body re-emit ``ns_set`` to push the proc's own namespace
    // for the duration of the fallback, and that secondary call
    // would otherwise clobber the proc-prologue's caller-ns record
    // and leave ``uplevel 1`` shifting to the wrong namespace.
    //
    // Canonicalise ``saved == 0`` (uninitialised ``current_ns`` —
    // top-level callers) to the root namespace handle so the
    // ``if_unset`` slot test below considers the slot *filled*.
    // Without this, a top-level caller's prologue would leave
    // ``frame_ns[depth-1] == 0``, then a body-side eval-fallback's
    // ``ns_set`` (saved == proc's own ns) would stamp the slot
    // with the *proc's* namespace and ``uplevel 1`` would shift
    // back to the proc's namespace instead of the caller's root.
    const stamp = if (saved != 0) saved else tcl_ns.ns_root();
    frames.frame_set_ns_if_unset(stamp);
    return @as(i64, saved);
}

/// Restore a saved namespace context, unwinding an ``ns_set`` pair.
pub export fn ns_restore(saved: i64) void {
    tcl_ns.current_ns = @intCast(saved & 0xFFFFFFFF);
}

/// If *name* (a TclObj) is unqualified (no leading ``::``) and a
/// non-root current namespace context is active, return a fresh
/// TclObj holding ``<ns_full_name>::<name>``.  Otherwise return
/// *name* unchanged.  Used by the interpreter's ``proc`` /
/// ``variable`` handlers to namespace-qualify dynamically
/// constructed names.
pub fn qualify_name(name: i32) i32 {
    if (tcl_ns.current_ns == 0) return name;
    // Root has full name ``::``; ``::name`` is the same as ``name``
    // for resolution purposes, so don't bother prefixing — keeps
    // the output stable for callers that previously got an
    // unqualified name back when no ns was active.
    const ns_full = tcl_ns.ns_full_name(tcl_ns.current_ns);
    if (ns_full.len == 2) return name;
    const s = obj_ensure_string(name);
    if (s.len == 0) return name;
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    // Already qualified with ``::`` — leave alone.
    if (s.len >= 2 and sp[0] == ':' and sp[1] == ':') return name;
    // Build ``<ns_full>::<name>`` in the bump allocator.
    const ns_ptr: [*]const u8 = @ptrFromInt(ns_full.ptr);
    const total: u32 = ns_full.len + 2 + s.len;
    const buf_addr: u32 = obj_mod.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    for (0..ns_full.len) |i| buf[i] = ns_ptr[i];
    buf[ns_full.len] = ':';
    buf[ns_full.len + 1] = ':';
    for (0..s.len) |i| buf[ns_full.len + 2 + i] = sp[i];
    return obj_mod.obj_new_string(@bitCast(buf_addr), @bitCast(total));
}

// -- upvar / uplevel helpers --

/// Parse an unsigned integer from a byte slice.  Stops at first non-digit.
pub fn parse_uint_bytes(ptr: [*]const u8, len: u32) u32 {
    var result: u32 = 0;
    var i: u32 = 0;
    while (i < len) : (i += 1) {
        const c = ptr[i];
        if (c < '0' or c > '9') break;
        result = result * 10 + (c - '0');
    }
    return result;
}

/// Concatenate a slice of TclObj words with single spaces into one TclObj.
pub fn concat_words(ws: []const i32) i32 {
    if (ws.len == 0) return obj_new_string(0, 0);
    if (ws.len == 1) return ws[0];
    // Calculate total byte length including spaces between words.
    var total: u32 = 0;
    for (ws) |w| {
        const s = obj_ensure_string(w);
        total += s.len;
    }
    total += @as(u32, @intCast(ws.len)) - 1; // spaces
    const buf = alloc(total);
    var off: u32 = 0;
    for (ws, 0..) |w, wi| {
        const s = obj_ensure_string(w);
        memcpy(buf + off, s.ptr, s.len);
        off += s.len;
        if (wi + 1 < ws.len) {
            const bp: [*]u8 = @ptrFromInt(buf + off);
            bp[0] = ' ';
            off += 1;
        }
    }
    return obj_new_string(@bitCast(buf), @bitCast(total));
}

/// ``upvar ?level? otherVar myVar ?otherVar myVar ...?``
///
/// Supported levels:
///   ``#0``  — global alias (optionally different local/target name)
///   ``N``   — relative: N frames above current (default 1)
///
/// Limitation: if the target frame belongs to a compiled proc that has not
/// yet synced its WASM locals into the frame hash table (i.e., it never hit
/// an eval-fallback), the aliased variable will read as 0/unset.  This is
/// acceptable until full shadow-stack or pre-call-sync is implemented.
pub fn eval_upvar(words: []const i32) i32 {
    if (words.len < 3) return 0;

    // Determine whether words[1] is a level specifier.  Mirrors
    // ``Tcl_UpvarObjCmd`` (tclVar.c): the first argument is taken as
    // a level only when it both LOOKS like a level (leading ``#`` or
    // digit) AND the remaining word count has the right parity for
    // ``otherVar localVar`` pairs.  Without the parity check,
    // ``upvar 0 x`` (where ``0`` is the local variable's name and
    // ``x`` is the alias target) is mis-parsed as level=0 with no
    // pair, leaving the alias unmade — see upvar-8.2.1.
    const w1 = obj_ensure_string(words[1]);

    var pairs_start: u32 = 1;
    var is_global: bool = false;
    // abs_target_depth: absolute 1-indexed depth of the target frame.
    // Default: one level up from current (upvar 1).
    var abs_target: i32 = @as(i32, @intCast(frames.frame_depth)) - 1;

    if (w1.len > 0) {
        const w1p: [*]const u8 = @ptrFromInt(w1.ptr);
        const looks_like_level =
            (w1p[0] == '#') or (w1p[0] >= '0' and w1p[0] <= '9');
        // After a level word there must be an even number of
        // remaining words for the otherVar/localVar pairs.  If not,
        // ``words[1]`` is a name, not a level.
        const remaining_after_level = words.len - 2;
        const treat_as_level = looks_like_level and (remaining_after_level % 2 == 0);
        if (treat_as_level) {
            pairs_start = 2;
            if (w1p[0] == '#') {
                const level = parse_uint_bytes(w1p + 1, w1.len - 1);
                if (level == 0) {
                    is_global = true;
                } else {
                    abs_target = @intCast(level);
                }
            } else {
                const rel = parse_uint_bytes(w1p, w1.len);
                abs_target = @as(i32, @intCast(frames.frame_depth)) - @as(i32, @intCast(rel));
            }
        }
        // else: not a level spec; default level 1 applies
    }

    var i = pairs_start;
    while (i + 1 < words.len) : (i += 2) {
        const other_var = words[i]; // name in the target frame
        const local_var = words[i + 1]; // alias name in the current frame
        if (is_global or abs_target <= 0) {
            // #0 or level underflow → global alias
            frames.frame_alias_named(local_var, other_var);
        } else {
            frames.frame_alias_frame_var(local_var, abs_target, other_var);
        }
    }
    return 0;
}

/// ``uplevel ?level? body ?body ...?``
///
/// Evaluates the body in the caller's frame by temporarily adjusting
/// frame_depth.  Multiple body words are joined with spaces (Tcl semantics).
///
/// Level defaults to 1 (one frame up).  ``#0`` means the global frame.
pub fn eval_uplevel(words: []const i32) i32 {
    if (words.len < 2) return 0;

    const w1 = obj_ensure_string(words[1]);
    const w1p: [*]const u8 = @ptrFromInt(w1.ptr);

    var body_start: u32 = 1;
    var shift: i32 = 1; // frames to shift down (default: uplevel 1)

    if (w1.len > 0) {
        if (w1p[0] == '#') {
            // ``#N`` is an ABSOLUTE target level — shift by
            // (frame_depth - N) so the target frame becomes the
            // active one.  Clamp to ``frame_depth`` when ``N`` is
            // deeper than the current stack (treats as #0).
            body_start = 2;
            const level = parse_uint_bytes(w1p + 1, w1.len - 1);
            if (level >= frames.frame_depth) {
                shift = @intCast(frames.frame_depth);
            } else {
                shift = @intCast(frames.frame_depth - level);
            }
        } else if (w1p[0] >= '0' and w1p[0] <= '9') {
            body_start = 2;
            shift = @intCast(parse_uint_bytes(w1p, w1.len));
        }
        // else: not a level spec; body_start stays 1, shift stays 1
    }

    if (body_start >= words.len) return 0;

    const body_obj = concat_words(words[body_start..]);
    // ``frame_depth_stash`` / ``frame_depth_restore`` save and
    // restore both the frame depth and the namespace context,
    // re-entering the target frame's recorded caller-ns for the
    // duration of the eval.  See :file:`tcl_frames.zig`.
    const saved = frames.frame_depth_stash(shift);
    const body_s = obj_ensure_string(body_obj);
    const result = eval_script(body_s.ptr, body_s.len);
    frames.frame_depth_restore(saved);
    return result;
}

// -- Control flow --

pub fn eval_if(words: []const i32) i32 {
    var i: u32 = 1;
    while (i < words.len) {
        const kw = obj_ensure_string(words[i]);
        const kp: [*]const u8 = @ptrFromInt(kw.ptr);
        if (str_eq(kp, kw.len, "else")) {
            if (i + 1 < words.len) {
                const bs = obj_ensure_string(words[i + 1]);
                return eval_script(bs.ptr, bs.len);
            }
            return 0;
        }
        const cond_s = obj_ensure_string(words[i]);
        if (eval_expr_str(cond_s.ptr, cond_s.len) != 0) {
            if (i + 1 < words.len) {
                const bs = obj_ensure_string(words[i + 1]);
                return eval_script(bs.ptr, bs.len);
            }
            return 0;
        }
        i += 2;
        if (i < words.len) {
            const nk = obj_ensure_string(words[i]);
            const np: [*]const u8 = @ptrFromInt(nk.ptr);
            if (str_eq(np, nk.len, "elseif")) i += 1;
        }
    }
    return 0;
}

pub fn eval_while(words: []const i32) i32 {
    if (words.len < 3) return 0;
    const cond_s = obj_ensure_string(words[1]);
    const body_s = obj_ensure_string(words[2]);
    var result: i32 = 0;
    while (true) {
        if (eval_expr_str(cond_s.ptr, cond_s.len) == 0) break;
        // Release the previous iteration's body result before
        // overwriting (issue #303 — tight ``while`` loops on the
        // interpreter path were leaking one TclObj per pass and
        // pushing io.test past the wasm32 4 GiB linear-memory
        // ceiling).
        if (result != 0) obj_mod.tcl_obj_release(result);
        result = eval_script(body_s.ptr, body_s.len);
        const ir = result_mod.snapshot(result);
        switch (ir.code) {
            .OK => {},
            .BREAK => {
                result_mod.consume(.BREAK);
                break;
            },
            .CONTINUE => {
                result_mod.consume(.CONTINUE);
                continue;
            },
            .ERROR, .RETURN => return result,
        }
    }
    return result;
}

pub fn eval_for(words: []const i32) i32 {
    if (words.len < 5) return 0;
    const init_s = obj_ensure_string(words[1]);
    const cond_s = obj_ensure_string(words[2]);
    const next_s = obj_ensure_string(words[3]);
    const body_s = obj_ensure_string(words[4]);
    // ``init`` and ``next`` clauses are evaluated for their side
    // effects only — release any TclObj they produce so a tight
    // ``for {set i 0} {$i < N} {incr i} {…}`` loop doesn't leak
    // ``N`` per-iteration result objs.  Same handling for the
    // body result on each subsequent overwrite (issue #303 root
    // cause for the io.test 2 GiB cliff).
    const init_res = eval_script(init_s.ptr, init_s.len);
    if (init_res != 0) obj_mod.tcl_obj_release(init_res);
    if (has_signal()) return 0;
    var result: i32 = 0;
    while (true) {
        if (eval_expr_str(cond_s.ptr, cond_s.len) == 0) break;
        if (result != 0) obj_mod.tcl_obj_release(result);
        result = eval_script(body_s.ptr, body_s.len);
        const ir = result_mod.snapshot(result);
        switch (ir.code) {
            .OK => {},
            // ``for`` runs the next-clause even after ``continue``;
            // ``while`` doesn't have one, hence the divergence above.
            .CONTINUE => result_mod.consume(.CONTINUE),
            .BREAK => {
                result_mod.consume(.BREAK);
                break;
            },
            .ERROR, .RETURN => return result,
        }
        const next_res = eval_script(next_s.ptr, next_s.len);
        if (next_res != 0) obj_mod.tcl_obj_release(next_res);
    }
    return result;
}

/// ``switch ?-options? string pattern body ?pattern body ...?``
/// or ``switch ?-options? string {pattern body ...}``
///
/// Supported options:
///   * ``-exact`` (default) — literal string equality
///   * ``-glob``            — glob-style wildcards via ``glob_match``
///   * ``-regexp``          — POSIX-extended regex via ``run_match``
///   * ``-nocase``          — case-insensitive (modifies match mode)
///   * ``--``               — end of options
///
/// Pattern handling:
///   * Body of ``-`` means "fall through to next non-``-`` body"
///   * Pattern ``default`` (only valid in last position) matches if
///     no earlier pattern did
///
/// Not implemented (will trap):
///   * ``-matchvar`` / ``-indexvar`` — regex capture binding
pub fn eval_switch(words: []const i32) i32 {
    const stubs = @import("../stubs/tcl_stubs.zig");
    if (words.len < 3) {
        stubs.raise("wrong # args: should be \"switch ?options? string ?pattern body ...? ?default body?\"");
        return 0;
    }
    // Parse options.
    const Mode = enum { exact, glob, regexp };
    var mode: Mode = .exact;
    var nocase: bool = false;
    var i: u32 = 1;
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len < 1) break;
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        if (wp[0] != '-') break;
        if (str_eq(wp, w.len, "-exact")) {
            mode = .exact;
            continue;
        }
        if (str_eq(wp, w.len, "-glob")) {
            mode = .glob;
            continue;
        }
        if (str_eq(wp, w.len, "-regexp")) {
            mode = .regexp;
            continue;
        }
        if (str_eq(wp, w.len, "-nocase")) {
            nocase = true;
            continue;
        }
        if (str_eq(wp, w.len, "--")) {
            i += 1;
            break;
        }
        if (str_eq(wp, w.len, "-matchvar") or str_eq(wp, w.len, "-indexvar")) {
            stubs.raise("switch: -matchvar / -indexvar not yet supported");
            return 0;
        }
        // Unknown option — produce reference Tcl's error
        const prefix: []const u8 = "bad option \"";
        const suffix: []const u8 = "\": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --";
        const total: u32 = @intCast(prefix.len + w.len + suffix.len);
        const buf = obj_mod.alloc(total);
        const bp: [*]u8 = @ptrFromInt(buf);
        var off: usize = 0;
        for (prefix) |c| {
            bp[off] = c;
            off += 1;
        }
        if (w.len > 0) {
            const sp: [*]const u8 = @ptrFromInt(w.ptr);
            for (0..w.len) |k| {
                bp[off] = sp[k];
                off += 1;
            }
        }
        for (suffix) |c| {
            bp[off] = c;
            off += 1;
        }
        const msg = obj_mod.obj_new_string(@bitCast(buf), @bitCast(total));
        const catch_mod = @import("tcl_catch.zig");
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }

    if (i >= words.len) {
        stubs.raise("wrong # args: should be \"switch ?options? string ?pattern body ...? ?default body?\"");
        return 0;
    }
    const subject = words[i];
    i += 1;
    const subject_s = obj_ensure_string(subject);

    // Two shapes for the pattern/body list:
    //   (A) words[i..] are alternating pattern/body pairs.
    //   (B) words[i] is a single list with even number of elements.
    // Per Tcl 9 spec the single-arg form is detected by exactly one
    // remaining word and that word being a list of >= 2 elements.
    var pairs_in_list: bool = false;
    if (i + 1 == words.len) {
        const probe = obj_ensure_string(words[i]);
        const n_probe = list_count_elements(probe.ptr, probe.len);
        if (n_probe >= 2 and @rem(n_probe, 2) == 0) pairs_in_list = true;
    }

    // Helper: match one pattern against subject, accounting for mode/nocase.
    const PatRunner = struct {
        fn run(p_ptr: u32, p_len: u32, s_ptr: u32, s_len: u32, mode_v: Mode, no_case: bool) bool {
            const tcl_string = @import("../valtypes/tcl_string.zig");
            const tcl_chars = @import("../valtypes/tcl_chars.zig");
            switch (mode_v) {
                .exact => {
                    if (no_case) {
                        if (p_len != s_len) return false;
                        const pp: [*]const u8 = @ptrFromInt(p_ptr);
                        const ss: [*]const u8 = @ptrFromInt(s_ptr);
                        var k: u32 = 0;
                        while (k < p_len) : (k += 1) {
                            const a: u8 = if (pp[k] >= 'A' and pp[k] <= 'Z') pp[k] | 0x20 else pp[k];
                            const b: u8 = if (ss[k] >= 'A' and ss[k] <= 'Z') ss[k] | 0x20 else ss[k];
                            if (a != b) return false;
                        }
                        return true;
                    }
                    if (p_len != s_len) return false;
                    if (p_len == 0) return true;
                    const pp: [*]const u8 = @ptrFromInt(p_ptr);
                    const ss: [*]const u8 = @ptrFromInt(s_ptr);
                    return tcl_chars.mem_eq(pp, p_len, ss, s_len);
                },
                .glob => {
                    if (no_case) {
                        // Lowercase both before glob_match.
                        const obj_alloc = @import("../valtypes/tcl_obj.zig");
                        const pbuf = obj_alloc.alloc(p_len);
                        const sbuf = obj_alloc.alloc(s_len);
                        const pd: [*]u8 = @ptrFromInt(pbuf);
                        const sd: [*]u8 = @ptrFromInt(sbuf);
                        const ps: [*]const u8 = @ptrFromInt(p_ptr);
                        const ss: [*]const u8 = @ptrFromInt(s_ptr);
                        var k: u32 = 0;
                        while (k < p_len) : (k += 1) pd[k] = if (ps[k] >= 'A' and ps[k] <= 'Z') ps[k] | 0x20 else ps[k];
                        k = 0;
                        while (k < s_len) : (k += 1) sd[k] = if (ss[k] >= 'A' and ss[k] <= 'Z') ss[k] | 0x20 else ss[k];
                        return tcl_string.glob_match(pbuf, p_len, sbuf, s_len);
                    }
                    return tcl_string.glob_match(p_ptr, p_len, s_ptr, s_len);
                },
                .regexp => {
                    const tcl_regex = @import("../valtypes/tcl_regex.zig");
                    return tcl_regex.run_match_pub(p_ptr, p_len, s_ptr, s_len, no_case);
                },
            }
        }
    };

    // Iterate pattern/body pairs.  We do two passes when ``-`` body
    // appears: the first pass picks the matching pattern, the second
    // walks forward to find the first non-``-`` body.
    var match_idx: i64 = -1; // index of matching pair (0-based pair index)
    var n_pairs: i64 = 0;
    if (pairs_in_list) {
        const list_s = obj_ensure_string(words[i]);
        const total_elems = list_count_elements(list_s.ptr, list_s.len);
        n_pairs = @divTrunc(total_elems, 2);
        var pi: i64 = 0;
        while (pi < n_pairs) : (pi += 1) {
            const pat_e = list_element_at(list_s.ptr, list_s.len, pi * 2);
            const is_default = pat_e.len == 7 and blk: {
                const sp: [*]const u8 = @ptrFromInt(list_s.ptr + pat_e.start);
                break :blk sp[0] == 'd' and sp[1] == 'e' and sp[2] == 'f' and sp[3] == 'a' and
                    sp[4] == 'u' and sp[5] == 'l' and sp[6] == 't';
            };
            if (is_default and pi == n_pairs - 1) {
                if (match_idx == -1) match_idx = pi;
                break;
            }
            if (PatRunner.run(list_s.ptr + pat_e.start, pat_e.len, subject_s.ptr, subject_s.len, mode, nocase)) {
                match_idx = pi;
                break;
            }
        }
        if (match_idx == -1) return obj_new_string(0, 0);
        // Walk forward for non-``-`` body
        var bi: i64 = match_idx;
        while (bi < n_pairs) : (bi += 1) {
            const body_e = list_element_at(list_s.ptr, list_s.len, bi * 2 + 1);
            if (body_e.len == 1) {
                const bp: [*]const u8 = @ptrFromInt(list_s.ptr + body_e.start);
                if (bp[0] == '-') continue;
            }
            return eval_script(list_s.ptr + body_e.start, body_e.len);
        }
        return obj_mod.obj_new_string(0, 0);
    }

    // Inline form: words[i..] are alternating pattern/body
    if (@rem(@as(u32, @intCast(words.len)) - i, 2) != 0) {
        stubs.raise("extra switch pattern with no body");
        return 0;
    }
    n_pairs = @intCast((words.len - i) / 2);
    var pi: i64 = 0;
    while (pi < n_pairs) : (pi += 1) {
        const pat = words[i + @as(u32, @intCast(pi)) * 2];
        const pat_s = obj_ensure_string(pat);
        const is_default = pat_s.len == 7 and blk: {
            const sp: [*]const u8 = @ptrFromInt(pat_s.ptr);
            break :blk sp[0] == 'd' and sp[1] == 'e' and sp[2] == 'f' and sp[3] == 'a' and
                sp[4] == 'u' and sp[5] == 'l' and sp[6] == 't';
        };
        if (is_default and pi == n_pairs - 1) {
            if (match_idx == -1) match_idx = pi;
            break;
        }
        if (PatRunner.run(pat_s.ptr, pat_s.len, subject_s.ptr, subject_s.len, mode, nocase)) {
            match_idx = pi;
            break;
        }
    }
    if (match_idx == -1) return obj_new_string(0, 0);
    var bi: i64 = match_idx;
    while (bi < n_pairs) : (bi += 1) {
        const body = words[i + @as(u32, @intCast(bi)) * 2 + 1];
        const body_s = obj_ensure_string(body);
        if (body_s.len == 1) {
            const bp: [*]const u8 = @ptrFromInt(body_s.ptr);
            if (bp[0] == '-') continue;
        }
        return eval_script(body_s.ptr, body_s.len);
    }
    return obj_mod.obj_new_string(0, 0);
}

pub fn eval_foreach(words: []const i32) i32 {
    // foreach varList1 list1 ?varList2 list2 ...? body
    // varList may be a list of multiple var names (e.g. {a b}).
    // words[0]=foreach, words[1..len-2]=(varlist,list) pairs (even count), words[len-1]=body
    const stubs = @import("../stubs/tcl_stubs.zig");
    const wrong_args_msg = "wrong # args: should be \"foreach varList list ?varList list ...? command\"";
    if (words.len < 4) {
        stubs.raise(wrong_args_msg);
        return 0;
    }
    const pair_words = words.len - 2; // words[1..words.len-1]
    if (pair_words % 2 != 0) {
        stubs.raise(wrong_args_msg);
        return 0;
    }
    const n_pairs = pair_words / 2;
    const MAX_PAIRS = 63; // (MAX_WORDS-2)/2
    if (n_pairs > MAX_PAIRS) return 0;
    const body_s = obj_ensure_string(words[words.len - 1]);

    // For each pair, compute the step (number of vars in the varlist) and the
    // list length.  Iteration count = max(ceil(list_len / step)) across pairs.
    var list_lens: [MAX_PAIRS]i64 = [_]i64{0} ** MAX_PAIRS;
    var steps: [MAX_PAIRS]i64 = [_]i64{1} ** MAX_PAIRS;
    var n: i64 = 0;
    {
        var p: u32 = 0;
        while (p < n_pairs) : (p += 1) {
            const varlist_obj = words[1 + p * 2];
            const vls = obj_ensure_string(varlist_obj);
            // Validate varlist syntax.
            {
                const lp = @import("../valtypes/tcl_list_parse.zig");
                if (lp.check_list_syntax(vls.ptr, vls.len) != 0) return 0;
            }
            const count = list_count_elements(vls.ptr, vls.len);
            if (count == 0) {
                stubs.raise("foreach varlist is empty");
                return 0;
            }
            const step: i64 = count;
            steps[p] = step;

            const ls = obj_ensure_string(words[2 + p * 2]);
            // Validate list syntax (e.g. {a}{b} is illegal in Tcl).
            {
                const lp = @import("../valtypes/tcl_list_parse.zig");
                if (lp.check_list_syntax(ls.ptr, ls.len) != 0) return 0;
            }
            const len = list_count_elements(ls.ptr, ls.len);
            list_lens[p] = len;
            // Iteration count for this pair: ceil(len / step)
            const iters: i64 = @divTrunc(len + step - 1, step);
            if (iters > n) n = iters;
        }
    }
    var result: i32 = 0;
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        // Bind each variable in each varlist for this iteration step.
        var p: u32 = 0;
        while (p < n_pairs) : (p += 1) {
            const varlist_obj = words[1 + p * 2];
            const list_obj = words[2 + p * 2];
            const vls = obj_ensure_string(varlist_obj);
            const ls = obj_ensure_string(list_obj);
            const step = steps[p];
            var j: i64 = 0;
            while (j < step) : (j += 1) {
                const elem_idx = idx * step + j;
                // Get the variable name for slot j in this varlist
                const var_elem = list_element_at(vls.ptr, vls.len, j);
                const src_vname = vls.ptr + var_elem.start;
                const var_name_obj: i32 = if (var_elem.braced or src_vname == 0)
                    obj_new_string_copy(src_vname, var_elem.len)
                else inner: {
                    const buf = alloc(var_elem.len + 1);
                    if (buf == 0) break :inner obj_new_string(0, 0);
                    const out_len = copy_unbraced_elem(buf, src_vname, var_elem.len);
                    break :inner obj_new_string_take(buf, out_len, var_elem.len + 1);
                };
                // Get the value from the list at element index elem_idx
                const elem_val: i32 = if (elem_idx < list_lens[p]) blk: {
                    const elem = list_element_at(ls.ptr, ls.len, elem_idx);
                    const src_elem = ls.ptr + elem.start;
                    break :blk if (elem.braced or src_elem == 0)
                        obj_new_string_copy(src_elem, elem.len)
                    else inner: {
                        const buf = alloc(elem.len + 1);
                        if (buf == 0) break :inner obj_new_string(0, 0);
                        const out_len = copy_unbraced_elem(buf, src_elem, elem.len);
                        break :inner obj_new_string_take(buf, out_len, elem.len + 1);
                    };
                } else obj_new_string(0, 0); // past end of list → ""
                _ = frames.var_set(var_name_obj, elem_val);
                obj_mod.tcl_obj_release(elem_val);
                obj_mod.tcl_obj_release(var_name_obj);
            }
        }
        // Release the prior body result before overwriting (issue #303).
        if (result != 0) obj_mod.tcl_obj_release(result);
        result = eval_script(body_s.ptr, body_s.len);
        const ir = result_mod.snapshot(result);
        switch (ir.code) {
            .OK => {},
            .CONTINUE => {
                result_mod.consume(.CONTINUE);
                continue;
            },
            .BREAK => {
                result_mod.consume(.BREAK);
                break;
            },
            .ERROR, .RETURN => return result,
        }
    }
    // foreach returns "" on normal completion — discard last body result.
    if (result != 0) obj_mod.tcl_obj_release(result);
    return obj_new_string(0, 0);
}

// -- Proc dispatch (picol-style) --
// Look up the command name in the proc registry. If found:
//   1. Push a new call frame
//   2. Bind arguments to parameter names as local variables
//   3. Evaluate the body
//   4. Pop the frame
//   5. Absorb RETURN signal (convert to OK)
// If not found, raise an error.

/// Child-as-command dispatch.  When ``interp create name`` runs we
/// register a ``CMD_INTERP_CHILD`` Command in the parent's ns under
/// the child's simple name.  Calls to that name route here: we
/// treat ``name`` as an ``interp`` subcommand dispatcher —
/// ``name eval script`` → ``interp eval name script``, etc.
///
/// Matches the per-child ``interpCmd`` handler in
/// ``tmp/tcl9.0.3/generic/tclInterp.c`` (``ChildObjCmd``).  We ship
/// the ``eval`` shape; the rest surface as clean diagnostics so
/// callers get a readable error rather than a stub-dispatch trap.
/// Child-as-command bad-option error string.  Matches
/// ``tmp/tcl9.0.3/generic/tclInterp.c``'s per-child
/// ``childCmdOptions`` table verbatim (less the ones we don't
/// ship — ``bgerror``, ``cancel``, ``limit``, ``marktrusted``,
/// ``recursionlimit``).
const CHILD_SUBCOMMAND_LIST: []const u8 = "alias, aliases, bgerror, cancel, eval, expose, hide, hidden, issafe, invokehidden, limit, marktrusted, recursionlimit";

/// Render ``wrong # args: should be "<child><suffix>"`` — the
/// per-child arity-error wording used throughout ``ChildObjCmd``
/// in tclInterp.c.  ``suffix`` supplies the post-child-name
/// portion (``" alias aliasName ?targetName? ?arg ...?"``,
/// ``" aliases"`` etc.).
fn emit_child_arity_error(
    child_name_ptr: u32,
    child_name_len: u32,
    suffix: []const u8,
) void {
    const catch_mod = @import("tcl_catch.zig");
    const prefix: []const u8 = "wrong # args: should be \"";
    const tail: []const u8 = "\"";
    const total: u32 = @as(u32, @intCast(prefix.len)) +
        child_name_len +
        @as(u32, @intCast(suffix.len)) +
        @as(u32, @intCast(tail.len));
    const buf = alloc(total);
    const d: [*]u8 = @ptrFromInt(buf);
    for (prefix, 0..) |b, k| d[k] = b;
    if (child_name_len > 0) {
        const np: [*]const u8 = @ptrFromInt(child_name_ptr);
        for (0..child_name_len) |k| d[prefix.len + k] = np[k];
    }
    for (suffix, 0..) |b, k| d[prefix.len + child_name_len + k] = b;
    for (tail, 0..) |b, k| {
        d[prefix.len + child_name_len + suffix.len + k] = b;
    }
    // Issue #317: see ``stubs.unsupported`` for the ownership
    // rationale.
    const msg = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

fn emit_child_bad_option(name_ptr: u32, name_len: u32) void {
    const catch_mod = @import("tcl_catch.zig");
    const prefix: []const u8 = "bad option \"";
    const infix: []const u8 = "\": must be ";
    const total: u32 = @as(u32, @intCast(prefix.len)) +
        name_len +
        @as(u32, @intCast(infix.len)) +
        @as(u32, @intCast(CHILD_SUBCOMMAND_LIST.len));
    const buf = alloc(total);
    const d: [*]u8 = @ptrFromInt(buf);
    for (prefix, 0..) |b, k| d[k] = b;
    if (name_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(name_ptr);
        for (0..name_len) |k| d[prefix.len + k] = sp[k];
    }
    for (infix, 0..) |b, k| d[prefix.len + name_len + k] = b;
    for (CHILD_SUBCOMMAND_LIST, 0..) |b, k| {
        d[prefix.len + name_len + infix.len + k] = b;
    }
    // Issue #317: see ``stubs.unsupported``.
    const msg = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

/// Dispatch ``<child> subcmd ?args ...?``.  The per-child
/// subcommand set is the same shape as the ``interp`` built-in's
/// path-taking subcommands (``eval`` / ``alias`` / ``aliases`` /
/// ``hide`` / ``expose`` / ``hidden`` / ``invokehidden`` /
/// ``issafe``) with the child's identity supplied implicitly
/// rather than as an explicit path argument.
///
/// We implement this as an argv-rewrite: the caller's
/// ``[<child>, subcmd, args...]`` is transformed into
/// ``["interp", subcmd, <child_path>, args...]`` and handed to
/// the corresponding top-level ``interp`` subcommand handler.  The
/// ``<child_path>`` is a TclObj wrapping the child's simple name
/// in the current interp, which ``resolve_interp_path`` walks
/// through.  ``alias`` needs a slightly different rewrite (the
/// parent-path ``{}`` is injected between ``newName`` and
/// ``target`` on creation) so it's routed through the
/// per-subcommand helpers directly.
///
/// Mirrors ``ChildObjCmd`` (``tmp/tcl9.0.3/generic/tclInterp.c``).
fn dispatch_interp_child(words: []const i32, bucket: i32) i32 {
    const child: u32 = interp_reg.cmd_child_interp(@as(u32, @bitCast(bucket)));
    if (interp_reg.is_deleted(child)) {
        const catch_mod = @import("tcl_catch.zig");
        catch_mod.error_unknown_command(words[0]);
        return 0;
    }

    // Extract the caller-visible child name (taken from argv[0]
    // rather than the Interp's ``name_*`` slot so errors mirror
    // whatever spelling the user invoked — ``interp create foo``
    // and a later ``rename foo bar`` would read as ``bar alias ...``).
    const child_name = obj_ensure_string(words[0]);

    if (words.len < 2) {
        emit_child_arity_error(child_name.ptr, child_name.len, " cmd ?arg ...?");
        return 0;
    }
    const sub = obj_ensure_string(words[1]);
    if (sub.len == 0) return 0;
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);

    // ``alias`` diverges from the other subcommands because its
    // argv shape on the per-child form omits the explicit parent
    // path: ``<child> alias newName ?target ?arg ...??`` implicitly
    // sets ``parentPath = {}`` (current interp).  Route directly to
    // the same helpers the top-level form uses so we don't have to
    // fake up an argv with a synthetic ``{}`` slot.
    if (str_eq(sp, sub.len, "alias")) {
        if (words.len < 3) {
            emit_child_arity_error(
                child_name.ptr,
                child_name.len,
                " alias aliasName ?targetName? ?arg ...?",
            );
            return 0;
        }
        const new_name = obj_ensure_string(words[2]);
        if (words.len == 3) {
            return interp_impl.interp_alias_query(child, new_name.ptr, new_name.len);
        }
        const arg3 = obj_ensure_string(words[3]);
        if (arg3.len == 0 and words.len == 4) {
            return interp_impl.interp_alias_delete(child, new_name.ptr, new_name.len);
        }
        // Create: ``<child> alias newName target ?args...?``.  Parent
        // interp = current (the interp hosting this child Command).
        const target_name = obj_ensure_string(words[3]);
        const n_prefix: u32 = @as(u32, @intCast(words.len)) - 4;
        var prefix_buf: u32 = 0;
        // S6.3 v1: route the prefix-args staging buffer through
        // the arena.  ``alias_alloc`` heap-copies the handles into
        // its own buffer before returning, so the caller's buffer
        // is pure scratch — pre-arena code leaked it on every
        // ``interp alias`` call.  ``arena_restore`` runs AFTER
        // ``interp_alias_create`` returns its value, so the
        // alias_alloc's reads see valid bytes.
        const arena_saved = arena.arena_save();
        defer arena.arena_restore(arena_saved);
        var prefix_alloc: arena.Allocation = .{ .addr = 0, .size = 0, .from_arena = true };
        if (n_prefix > 0) {
            prefix_alloc = arena.arena_alloc_or_libc(n_prefix * 4);
            prefix_buf = prefix_alloc.addr;
            var j: u32 = 0;
            while (j < n_prefix) : (j += 1) {
                write_i32(prefix_buf + j * 4, words[4 + j]);
            }
        }
        const result = interp_impl.interp_alias_create(
            child,
            interp_reg.interp_current(),
            new_name.ptr,
            new_name.len,
            target_name.ptr,
            target_name.len,
            n_prefix,
            prefix_buf,
        );
        arena.arena_free(prefix_alloc);
        return result;
    }

    // The remaining subcommands route through the top-level
    // ``interp`` dispatcher with a synthesised ``path`` slot.  Build
    // ``["interp", subcmd, <child_path>, words[2..]]``.
    const c: *interp_reg.Interp = @ptrFromInt(child);
    const child_path_obj = rt.obj_new_string(@bitCast(c.name_ptr), @bitCast(c.name_len));

    const new_len: u32 = @as(u32, @intCast(words.len)) + 1;
    if (new_len > parse.MAX_WORDS) {
        const catch_mod = @import("tcl_catch.zig");
        const err_text = "<interp> subcommand argv exceeds MAX_WORDS";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    var new_words: [parse.MAX_WORDS]i32 = undefined;
    const interp_str: []const u8 = "interp";
    new_words[0] = rt.obj_new_string_copy(@intFromPtr(interp_str.ptr), interp_str.len);
    new_words[1] = words[1]; // subcmd (re-use caller's TclObj)
    new_words[2] = child_path_obj;
    var j: u32 = 3;
    var k: u32 = 2;
    while (k < words.len) : (k += 1) {
        new_words[j] = words[k];
        j += 1;
    }

    if (str_eq(sp, sub.len, "eval")) return eval_interp_eval(new_words[0..new_len]);
    if (str_eq(sp, sub.len, "aliases")) {
        // ``<child> aliases`` takes no arguments.  Matches
        // ``ChildObjCmd``'s ``CHILD_ALIASES`` branch in tclInterp.c.
        if (words.len != 2) {
            emit_child_arity_error(child_name.ptr, child_name.len, " aliases");
            return 0;
        }
        return interp_impl.interp_aliases_list_for(child);
    }
    if (str_eq(sp, sub.len, "hide")) return interp_impl.eval_interp_hide(new_words[0..new_len]);
    if (str_eq(sp, sub.len, "expose")) return interp_impl.eval_interp_expose(new_words[0..new_len]);
    if (str_eq(sp, sub.len, "hidden")) return interp_impl.eval_interp_hidden(new_words[0..new_len]);
    if (str_eq(sp, sub.len, "invokehidden")) return eval_interp_invokehidden(new_words[0..new_len]);
    if (str_eq(sp, sub.len, "issafe")) return interp_impl.eval_interp_issafe(new_words[0..new_len]);

    // Unknown subcommand — match tclsh's per-child wording.
    emit_child_bad_option(sub.ptr, sub.len);
    return 0;
}

/// Alias dispatch trampoline.  An ``interp alias`` redirect Command
/// has ``CMD_ALIAS`` set in its flags and stores an
/// :type:`tcl_alias.AliasRec` in ``params_obj``.  On dispatch we:
///
///   1. Build a new argv: ``[target_name, prefix_args..., words[1..]]``.
///   2. Resolve the stored target name at call time.  Resolution is
///      anchored at the global namespace (``TCL_EVAL_INVOKE``-style)
///      so a bare target name binds to ``::<target>`` regardless of
///      which namespace the alias is invoked from.
///   3. Recurse through ``eval_proc_call_bucket`` — this preserves
///      all the compiled-proc / host-bridge paths the target might
///      take.
///
/// Deletion / rename behaviour: because we resolve by string each
/// call, an alias *lazily* observes deletion of its target — the
/// first dispatch after the target is gone produces "unknown
/// command: <target>".  Rename of the target, however, is NOT
/// tracked — the alias keeps looking up the old stored name, which
/// no longer resolves once the Command has moved to the new name.
/// This matches C Tcl semantics where ``rename target new`` breaks
/// any alias still pointing at ``target`` until some other path
/// repoints it.
///
/// Error surface: missing target produces ``unknown command: <target>``
/// at dispatch time, matching C Tcl's behaviour where an alias to a
/// since-deleted command fails only when invoked, not at delete time.
///
/// Argv cap: ``tcl_parse.MAX_WORDS`` bounds the words array the
/// interpreter is allowed to construct.  If the prefix + caller argv
/// would exceed it, we error out rather than truncating.
fn dispatch_alias(words: []const i32, bucket: i32) i32 {
    const rec = alias_mod.alias_rec(@bitCast(bucket));
    if (rec.target_name_len == 0) {
        // Cleared / deleted alias.  Raise ``unknown command: <self>``
        // matching the behaviour of calling an undefined command.
        const catch_mod = @import("tcl_catch.zig");
        catch_mod.error_unknown_command(words[0]);
        return 0;
    }

    // Total argv length = target name + prefix + (caller argv minus
    // words[0]).  Guard against MAX_WORDS overflow; Tcl traps with
    // "too many nested evaluations" but we get a cleaner signal by
    // raising an explicit error here.
    const caller_tail: u32 = if (words.len > 1) @as(u32, @intCast(words.len - 1)) else 0;
    const total: u32 = 1 + rec.n_prefix + caller_tail;
    if (total > parse.MAX_WORDS) {
        const catch_mod = @import("tcl_catch.zig");
        const err_text = "alias argv exceeds MAX_WORDS";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }

    var new_words: [parse.MAX_WORDS]i32 = undefined;
    // Slot 0: the target command name as a fresh TclObj.
    new_words[0] = rt.obj_new_string(
        @bitCast(rec.target_name_ptr),
        @bitCast(rec.target_name_len),
    );
    // Slots 1..1+n_prefix: the frozen prefix args.  The AliasRec
    // stores u32 TclObj handles but the interpreter runs on i32
    // handles; @bitCast is the reinterpretation we want.
    var i: u32 = 0;
    while (i < rec.n_prefix) : (i += 1) {
        new_words[1 + i] = read_i32(rec.prefix_args_addr + i * 4);
    }
    // Slots 1+n_prefix..total: the caller's argv past the command
    // name.
    i = 0;
    while (i < caller_tail) : (i += 1) {
        new_words[1 + rec.n_prefix + i] = words[1 + i];
    }

    // Resolve the target in the *global* namespace.  Tcl's
    // ``interp alias`` dispatches targets as if the call originated
    // from the root interpreter scope (``tclInterp.c``'s
    // ``AliasObjCmd`` passes ``TCL_EVAL_INVOKE``, which bypasses
    // the caller's ``namespace path`` and anchors command lookup at
    // the global ns).  Without this, an unqualified target name
    // would bind to whatever command shadowed it in the caller's
    // namespace — e.g. ``::foo`` vs ``::ns::foo`` when the alias
    // is invoked from ``::ns``.
    //
    // We implement the anchor by temporarily swapping ``current_ns``
    // to root for the duration of the lookup + recursion, then
    // restoring it.  ``proc_lookup``'s LRU is keyed on the current
    // ns so this doesn't poison the caller's cache.  On miss we do
    // NOT fall through to ``eval_proc_call``'s stub dispatch —
    // alias targets are user-defined by construction; a missing
    // target is a clear "unknown command" diagnostic.
    //
    // Cross-interp aliases: the parent interp is stashed on the
    // ``AliasRec.parent_interp`` slot at create time.  When it's
    // set and names a different interp, we enter that interp
    // before the lookup so the target resolves against the
    // parent's cmd_table, then leave on return.  (Previously this
    // lived in the Command's ``OFF_IMPORT_REF_HEAD`` slot, which
    // clashed with ``namespace import`` back-reference tracking
    // — moved into ``AliasRec`` so the two metadata axes stay
    // independent.)
    const parent_interp: u32 = rec.parent_interp;
    // If the stashed parent interp was torn down by a prior
    // ``interp delete``, produce a clean "unknown command"
    // diagnostic instead of walking a zeroed Interp.  Matches the
    // behaviour of an alias whose target command was renamed out
    // from under it — both surface at dispatch time.
    if (parent_interp != 0 and interp_reg.is_deleted(parent_interp)) {
        const catch_mod = @import("tcl_catch.zig");
        catch_mod.error_unknown_command(new_words[0]);
        return 0;
    }
    const do_cross_swap = parent_interp != 0 and parent_interp != interp_reg.interp_current();
    const save = if (do_cross_swap) interp_reg.enter(parent_interp) else interp_reg.EnterSave{
        .prev_interp = interp_reg.current_interp,
        .prev_root_addr = tcl_ns.root_addr,
        .prev_current_ns = tcl_ns.current_ns,
    };
    tcl_ns.current_ns = tcl_ns.ns_root();
    // Try the user-proc registry first — same priority as
    // ``eval_command``.  On miss, fall through to the builtin cmd_table
    // so an alias target like ``try`` / ``puts`` / ``string`` (Tcl
    // commands implemented in the runtime, not as procs) dispatches
    // correctly.  Without this, the proc-lookup-only path above
    // surfaced ``unknown command: try`` for ``interp alias {} run
    // {} try`` callers — string.test relied on this to alias ``run``
    // onto ``try`` for its compiled / non-compiled variants.
    const target_bucket = procs.proc_lookup(new_words[0]);
    if (target_bucket != 0) {
        const result = eval_proc_call_bucket(new_words[0..total], target_bucket);
        interp_reg.leave(save);
        return result;
    }
    const target_s = obj_ensure_string(new_words[0]);
    if (target_s.len > 0) {
        if (cmd_table.lookup(target_s.ptr, target_s.len)) |handler| {
            const result = handler(new_words[0..total]).value;
            interp_reg.leave(save);
            return result;
        }
        // ``::cmd`` qualified — strip and retry, mirrors eval_command's
        // builtin lookup branch.
        if (target_s.len >= 2) {
            const tp: [*]const u8 = @ptrFromInt(target_s.ptr);
            if (tp[0] == ':' and tp[1] == ':') {
                if (cmd_table.lookup(target_s.ptr + 2, target_s.len - 2)) |handler| {
                    const result = handler(new_words[0..total]).value;
                    interp_reg.leave(save);
                    return result;
                }
            }
        }
    }
    // Auto-index fallback before declaring the alias target unknown
    // (parse-8.12).  We're still in the entered-parent-interp scope
    // and ns_root, so the lookup picks up ``::auto_index`` exactly as
    // the test expects.
    const ai_loaded = try_auto_index_load(new_words[0]);
    if (ai_loaded != 0) {
        defer obj_mod.tcl_obj_release(ai_loaded);
        const ai_bucket = procs.proc_lookup(ai_loaded);
        if (ai_bucket != 0) {
            // Swap in the loaded FQ name as the dispatched command
            // name so ``info level 0`` etc see the canonical form.
            new_words[0] = ai_loaded;
            obj_mod.tcl_obj_retain(ai_loaded);
            const result = eval_proc_call_bucket(new_words[0..total], ai_bucket);
            interp_reg.leave(save);
            return result;
        }
    }
    interp_reg.leave(save);
    const catch_mod = @import("tcl_catch.zig");
    catch_mod.error_unknown_command(new_words[0]);
    return 0;
}

fn eval_proc_call(words: []const i32) i32 {
    const bucket = procs.proc_lookup(words[0]);
    if (bucket == 0) {
        // Before declaring the command unknown, try the ``auto_index``
        // auto-load mechanism (parse-8.12).  Tcl's stdlib ``unknown``
        // proc consults ``::auto_index($cmd)`` when a command isn't
        // found and evaluates the value (typically a ``proc`` definition)
        // to load it.  We replicate that lookup at the C level so the
        // test passes without shipping init.tcl.
        //
        // If the autoload script itself errors, the error is the
        // user's actual diagnostic (e.g. a syntax error in the
        // loader script body); surface it by short-circuiting the
        // unknown-command fallback.  Codex review: previously this
        // path ignored the loader's error and overwrote it with
        // ``invalid command name`` in ``error_unknown_command``
        // below, hiding the real failure.
        const loaded_name = try_auto_index_load(words[0]);
        if (result_mod.snapshot(0).code == .ERROR) {
            // Loader script raised; ``error_msg`` already holds the
            // user-facing message.  Drop any retained ``loaded_name``
            // (the loader-attempt path returned a name token only
            // when the script ran cleanly) and propagate.
            if (loaded_name != 0) obj_mod.tcl_obj_release(loaded_name);
            return 0;
        }
        if (loaded_name != 0) {
            defer obj_mod.tcl_obj_release(loaded_name);
            const bucket2 = procs.proc_lookup(loaded_name);
            if (bucket2 != 0) {
                // Replace words[0] with the loaded FQ name so the
                // dispatched call sees the canonical command name.
                var new_words: [parse.MAX_WORDS]i32 = undefined;
                new_words[0] = loaded_name;
                obj_mod.tcl_obj_retain(loaded_name);
                var k: u32 = 1;
                while (k < words.len) : (k += 1) new_words[k] = words[k];
                const result = eval_proc_call_bucket(new_words[0..words.len], bucket2);
                obj_mod.tcl_obj_release(loaded_name);
                return result;
            }
            // Loaded but no proc registered — fall through.
        }
        // Before declaring the command unknown, consult the stub
        // dispatch table — Tcl 8.4–9.0 core commands we haven't
        // implemented (encoding, fconfigure, regexp, trace, …) are
        // routed here from compiled-code fallbacks and produce
        // "unsupported command: <name>" rather than the generic
        // "unknown command: <name>" message.  Keeping the
        // dispatch-before-error pattern means user-defined procs
        // still win when they shadow a core command.
        const stub_dispatch = @import("../dispatch/tcl_stub_fallback.zig");
        const cmd_s = obj_ensure_string(words[0]);
        if (cmd_s.len > 0 and stub_dispatch.try_stub(@as([*]const u8, @ptrFromInt(cmd_s.ptr)), cmd_s.len)) {
            return 0;
        }
        // Unknown command — build a "unknown command: <name>"
        // message so the stderr/error_msg output identifies the
        // missing proc rather than emitting a bare command name.
        const catch_mod = @import("tcl_catch.zig");
        catch_mod.error_unknown_command(words[0]);
        return 0;
    }
    return eval_proc_call_bucket(words, bucket);
}

/// Implement the C-level half of Tcl's ``unknown`` / ``auto_load``
/// machinery: look up ``::auto_index(<calling-ns>::$cmd)`` first
/// (mirroring ``auto_load $cmd $namespace``'s namespace probe), then
/// fall back to ``::auto_index($cmd)``.  Evaluates the matching
/// entry (conventionally a ``proc`` definition).
///
/// Returns the FQ name TclObj that was registered (so the caller can
/// dispatch via that name) or 0 if no auto_index entry matched or
/// evaluation failed.  ``current_ns`` at call time supplies the
/// namespace for the namespaced probe — for ``interp alias``
/// dispatches that's the global root (TCL_EVAL_INVOKE), so only the
/// bare entry can match (parse-8.12).
fn try_auto_index_load(cmd_obj: i32) i32 {
    if (cmd_obj == 0) return 0;
    const cmd_s = obj_ensure_string(cmd_obj);
    if (cmd_s.ptr == 0 or cmd_s.len == 0) return 0;
    const tcl_array = @import("../valtypes/tcl_array.zig");
    const arr_name = obj_mod.obj_new_string_copy(@intFromPtr("::auto_index".ptr), 12);
    defer obj_mod.tcl_obj_release(arr_name);
    const cmd_p: [*]const u8 = @ptrFromInt(cmd_s.ptr);
    const cmd_is_fq = cmd_s.len >= 2 and cmd_p[0] == ':' and cmd_p[1] == ':';

    // 1. Namespaced probe — ``auto_index(<current_ns_full>::<cmd>)``.
    //    Skipped when caller is at the global root (so the alias-
    //    dispatch path falls through to the bare-name probe).
    if (!cmd_is_fq) {
        const ns_full_ptr = tcl_ns.current_ns_full_ptr();
        const ns_full_len = tcl_ns.current_ns_full_len();
        if (ns_full_len > 2) { // ">2" excludes the bare "::" root
            const total: u32 = ns_full_len + 2 + cmd_s.len;
            const buf = obj_mod.alloc(total);
            if (buf != 0) {
                const dst: [*]u8 = @ptrFromInt(buf);
                const nsp: [*]const u8 = @ptrFromInt(ns_full_ptr);
                var off: u32 = 0;
                for (0..ns_full_len) |k| {
                    dst[off] = nsp[k];
                    off += 1;
                }
                dst[off] = ':';
                off += 1;
                dst[off] = ':';
                off += 1;
                for (0..cmd_s.len) |k| {
                    dst[off] = cmd_p[k];
                    off += 1;
                }
                const fq_key = obj_mod.obj_new_string_take(buf, total, total);
                const entry = tcl_array.array_get(arr_name, fq_key);
                if (entry != 0) {
                    const es = obj_ensure_string(entry);
                    if (es.len > 0) {
                        const r = eval_script(es.ptr, es.len);
                        if (r != 0) obj_mod.tcl_obj_release(r);
                        if (result_mod.snapshot(0).code != .ERROR) return fq_key;
                    }
                }
                obj_mod.tcl_obj_release(fq_key);
            }
        }
    }
    // 2. Bare-name probe.
    const entry2 = tcl_array.array_get(arr_name, cmd_obj);
    if (entry2 != 0) {
        const es = obj_ensure_string(entry2);
        if (es.len > 0) {
            const r = eval_script(es.ptr, es.len);
            if (r != 0) obj_mod.tcl_obj_release(r);
            if (result_mod.snapshot(0).code != .ERROR) {
                obj_mod.tcl_obj_retain(cmd_obj);
                return cmd_obj;
            }
        }
    }
    return 0;
}

/// Build a Tcl list TclObj from every word in ``words`` (the proc
/// name at ``words[0]`` plus its call arguments).  Used by
/// :func:`eval_proc_call_bucket` to stash the invocation argv so
/// ``info level 0`` inside the body can read the real list.
///
/// Uses :func:`obj.list_elem_quote` / :func:`obj.list_elem_quote_nth`
/// so each word is properly escaped for list-element placement
/// — matching the shape :func:`tcl_dispatch.build_args_list`
/// produces for the ``args`` tail slot.
fn build_invocation_list(words: []const i32) i32 {
    if (words.len == 0) return obj_new_string(0, 0);
    // Worst-case expansion per element: 2x + 2 (full escape mode)
    // plus one separator byte per gap.
    var total: u32 = 0;
    var i: u32 = 0;
    while (i < words.len) : (i += 1) {
        const s = obj_ensure_string(words[i]);
        total += s.len * 2 + 2;
        if (i > 0) total += 1;
    }
    if (total == 0) return obj_new_string(0, 0);
    const buf = obj_mod.alloc(total);
    var off: u32 = 0;
    i = 0;
    while (i < words.len) : (i += 1) {
        if (i > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const s = obj_ensure_string(words[i]);
        if (i == 0) {
            off = obj_mod.list_elem_quote(buf, off, s.ptr, s.len);
        } else {
            off = obj_mod.list_elem_quote_nth(buf, off, s.ptr, s.len);
        }
    }
    // Issue #317: ``obj_new_string_take`` so the resulting list
    // owns its buffer; the older borrowing form leaked the buf
    // every time the eval-fallback proc dispatcher pushed an
    // argv list.  Combined with the un-released argv ref at the
    // caller (see ``eval_proc_call_bucket``), this was the
    // dominant io.test residual — every ``::tcltest::test`` call
    // (578 invocations in the failing trace) leaked one TclObj
    // header and one quoted-args buffer.
    return obj_new_string_take(buf, off, total);
}

/// Internal: dispatch once the proc bucket is already resolved.
/// Shared between ``eval_proc_call`` (legacy path) and the proc-first
/// fast path in ``eval_command``.
fn eval_proc_call_bucket(words: []const i32, bucket: i32) i32 {
    // ``interp alias`` redirect Commands carry the CMD_ALIAS flag bit
    // and an ``AliasRec`` in their params_obj slot.  Route them
    // through the alias trampoline BEFORE the generic proc body
    // path, which would otherwise mistake the AliasRec pointer for
    // a params TclObj.  Unlike CMD_IMPORTED (which proc_lookup
    // unwraps), aliases are visible to the dispatcher so the
    // trampoline can prepend the frozen prefix args.
    const cmd_flags: u32 = @bitCast(read_i32(@as(u32, @bitCast(bucket)) + procs.OFF_FLAGS));
    if ((cmd_flags & procs.CMD_ALIAS) != 0) {
        return dispatch_alias(words, bucket);
    }
    // ``CMD_INTERP_CHILD`` Commands carry the child's ``Interp*`` in
    // their ``params_obj`` slot.  ``name subcommand ?arg...?`` routes
    // to the child-subcommand dispatcher — currently handles ``eval``
    // (the only shape tcltest reaches for); everything else surfaces
    // as a clean diagnostic rather than a stub-dispatch miss.
    if ((cmd_flags & procs.CMD_INTERP_CHILD) != 0) {
        return dispatch_interp_child(words, bucket);
    }
    // ``coroutine NAME`` redirect commands carry CMD_COROUTINE and
    // a ``*Coro`` in ``params_obj``.  Resuming the coroutine is a
    // direct call into the segment driver — no proc body to push.
    if ((cmd_flags & procs.CMD_COROUTINE) != 0) {
        const tcl_coro = @import("../sched/tcl_coro.zig");
        const rec_addr: u32 = @bitCast(read_i32(@as(u32, @bitCast(bucket)) + procs.OFF_PARAMS_OBJ));
        const c: *tcl_coro.Coro = @ptrFromInt(rec_addr);
        return tcl_coro.resume_one(c);
    }
    // ``CMD_BUILTIN_FORWARD`` Commands wrap a hardcoded BUILTIN's
    // handler — set by ``rename BUILTIN newName`` so ``newName``
    // dispatches the original handler with the caller's words[].
    if ((cmd_flags & procs.CMD_BUILTIN_FORWARD) != 0) {
        const handler_addr: u32 = @bitCast(read_i32(@as(u32, @bitCast(bucket)) + procs.OFF_PARAMS_OBJ));
        const reg = @import("../dispatch/tcl_cmd_registry.zig");
        const handler: reg.HandlerFn = @ptrFromInt(handler_addr);
        return handler(words).value;
    }
    // ``CMD_BUILTIN_MASKED`` Commands are tombstones — set by
    // ``rename BUILTIN ""`` so a subsequent ``[BUILTIN ...]`` call
    // surfaces ``invalid command name "BUILTIN"`` instead of
    // dispatching through the BUILTIN cmd_table.  Matches reference
    // Tcl's ``TclEvalObjvInternal`` error verbatim — rename.test
    // 1.2 / 2.1 grep for it explicitly.
    if ((cmd_flags & procs.CMD_BUILTIN_MASKED) != 0) {
        const catch_mod = @import("tcl_catch.zig");
        catch_mod.error_invalid_command_name(words[0]);
        return 0;
    }
    // Compiled proc (func_idx != 0 is a marker set by
    // ``proc_register_compiled``) — dispatch via the host bridge
    // because pure WASM can't call across modules.  The bridge
    // looks up the proc's compiled WASM function by its
    // *registered* (fully-qualified) name — taken from the
    // bucket, not ``words[0]`` — and invokes it with the
    // unpacked argv.
    const func_idx = procs.proc_get_func_idx(bucket);
    if (func_idx != 0) {
        const dispatch_mod = @import("../dispatch/tcl_dispatch.zig");
        // Forward the invoked word (``words[0]``) through the
        // pending-argv0 slot so the compiled callee's prologue
        // reports the caller's source-level command name via
        // ``info level 0`` — tcltest's renamed / imported
        // entry points rely on this, and the host bridge
        // otherwise loses the information (it resolves by the
        // proc's compile-time export name, not the invoked
        // word).  The slot is consumed on entry by
        // ``frame_take_pending_argv0`` so it can't leak into
        // subsequent calls.
        if (words.len > 0) {
            frames.frame_set_pending_argv0(words[0]);
        }
        var result = dispatch_mod.dispatch(bucket, words);
        // Absorb ``return`` at the proc-dispatch boundary, mirroring
        // the interpreted-proc path below.  Without this, a compiled
        // proc whose body ends in ``return X`` (via the eval-fallback
        // path that sets ``return_flag``) leaks TCL_RETURN into the
        // script-level eval, and a surrounding ``catch {...}`` reports
        // code 2 where code 0 is correct — opt-3.x saw this after
        // ``catch_leave`` grew TCL_RETURN handling.  Real Tcl's proc
        // dispatch unconditionally converts body-level ``return`` to
        // a normal return value at the dispatch boundary.
        const ir_compiled = result_mod.snapshot(result);
        if (ir_compiled.code == .RETURN) {
            const tcl_catch = @import("tcl_catch.zig");
            const rv = ir_compiled.value;
            // ``return -code return`` / ``-level N`` propagates past
            // the immediate proc — keep the flag set and decrement
            // the extra-level counter so the next proc up also exits.
            if (ir_compiled.return_level > 0) {
                tcl_catch.state.return_level -= 1;
                // Leave return_flag/return_val set; produce ``rv`` as
                // the result so the parent dispatcher sees the value
                // when it absorbs.
                if (result == 0 and rv != 0) result = rv;
            } else {
                result_mod.consume(.RETURN);
                // The dispatch-bridge result the WASM-side flow_take_return
                // already absorbed will be non-zero; only fall back to the
                // recorded ``return_val`` when the bridge handed us 0.
                if (result == 0 and rv != 0) {
                    result = rv;
                } else if (rv != 0 and rv != result) {
                    obj_mod.tcl_obj_release(rv);
                }
            }
        }
        // ``return -code break`` / ``return -code continue`` from
        // a compiled callee leave the signal flag set; clear it
        // here so we don't re-route on the next dispatch cycle.
        // The break_flag / continue_flag they paired with stay
        // set, so an enclosing compiled while loop's
        // ``flow_consume_*`` probe catches the unwind.
        _ = result_mod.take_signal_break_continue();
        return result;
    }
    const body_obj = procs.proc_get_body(bucket);
    const params_obj = procs.proc_get_params(bucket);
    const n_params: u32 = @intCast(procs.proc_get_n_params(bucket));

    // Push frame
    _ = frames.frame_push();
    // Phase 8: stamp call-site metadata for ``info frame``.
    frames.frame_set_type(.PROC);
    frames.frame_set_script(body_obj);
    {
        const name_ptr: u32 = @bitCast(procs.proc_get_name_ptr(bucket));
        const name_len: u32 = @bitCast(procs.proc_get_name_len(bucket));
        if (name_len > 0) {
            const fq = obj_mod.obj_new_string_copy(name_ptr, name_len);
            frames.frame_set_proc_name(fq);
            obj_mod.tcl_obj_release(fq);
        }
    }
    // Tcl semantics: ``break`` / ``continue`` inside a proc body
    // are local to that body's enclosing loop, NOT the caller's
    // loop.  Save and clear any pending caller-scope flow signal
    // before running the body, restore on the way out only when
    // the body itself didn't raise a flow signal of its own.
    // Without this, a caller that has just set ``break_flag``
    // (e.g. tcltest's test #N+1 picking up a leftover from #N's
    // body=``break``) sees the post-body cleanup at the bottom
    // of this function fire unconditionally with ``words[0]`` as
    // the error message, blaming the wrong proc — surfaces as
    // ``tcl trap: site=N <inner-proc-name>`` traps deep inside
    // tcltest's ``[preserveCore]`` / ``[temporaryDirectory]``
    // checks and aborts entire test files.
    const bc_snap = result_mod.break_continue_save_and_clear();
    // Stash the invocation argv (proc name + all call args) so
    // ``info level 0`` inside the body returns the real list
    // rather than the placeholder emitted by legacy callers.
    // Issue #317: ``frame_set_argv`` retains the list for the
    // frame's argv slot; without dropping our creator-side ref
    // here the list TclObj sits at rc=2 once the frame pops, the
    // frame's release brings it to 1, and the orphan rc keeps
    // the obj (and its quoted-args buffer) alive forever.  Each
    // proc call leaked one TclObj + one buf — the dominant
    // io.test residual.
    const argv = build_invocation_list(words);
    frames.frame_set_argv(argv);
    obj_mod.tcl_obj_release(argv);

    // Stamp the proc's namespace onto ``current_ns`` for the
    // duration of the body so unqualified calls inside the body
    // resolve via the right ns tree.  Mirrors the compiled-proc
    // prologue's ``tcl_ns_set`` emission.  The proc's qualified
    // name lives in its Command bucket; the enclosing ns is the
    // prefix up to the last ``::``.
    const saved_proc_ns: u32 = tcl_ns.current_ns;
    defer tcl_ns.current_ns = saved_proc_ns;
    {
        const name_ptr: u32 = @bitCast(procs.proc_get_name_ptr(bucket));
        const name_len: u32 = @bitCast(procs.proc_get_name_len(bucket));
        if (name_len >= 2) {
            const nsrc: [*]const u8 = @ptrFromInt(name_ptr);
            // Find the LAST ``::`` so ``::ns::sub::foo`` maps to
            // ``::ns::sub``.  Loop from the end.
            var j: u32 = name_len;
            var found: u32 = 0;
            while (j >= 2) : (j -= 1) {
                if (nsrc[j - 2] == ':' and nsrc[j - 1] == ':') {
                    found = j - 2;
                    break;
                }
            }
            if (found >= 2) {
                // ns prefix is name[0..found], which is ``::ns…``
                // without the trailing ``::``.
                tcl_ns.current_ns = tcl_ns.ns_create_from_fqn(
                    @bitCast(name_ptr),
                    @bitCast(found),
                );
            } else if (found == 0 and name_len >= 2 and nsrc[0] == ':' and nsrc[1] == ':') {
                // Root-level proc (``::foo``) — ns_current stays
                // at root; nothing to push.
            }
        }
    }
    // Record the *caller's* namespace on the new frame so a later
    // ``uplevel`` from inside this body can re-enter the namespace
    // that was active immediately before this proc was invoked.
    // Without this, an uplevel'd script body resolves variables
    // against the *callee*'s namespace (typically ``::tcltest``)
    // instead of the caller's (e.g. ``::tcl::test::io``), so
    // unqualified array element refs miss the outer namespace's
    // array — the symptom upstream io.test / ioCmd.test exhibited
    // when ``::tcltest::RunTest`` upleveled the test body.
    frames.frame_set_ns(saved_proc_ns);

    // Bind parameters: walk the params list, assign each from argv.
    // If the last parameter is named "args", it collects all remaining
    // arguments as a Tcl list (standard Tcl variadic proc convention).
    if (params_obj != 0 and n_params > 0) {
        const ps = obj_ensure_string(params_obj);
        var pi: u32 = 0;
        while (pi < n_params) : (pi += 1) {
            const param_elem = list_element_at(ps.ptr, ps.len, @intCast(pi));
            // Each parameter is either a bare name (``a``) or a
            // 2-element list of ``{name default}``.  Detect the latter
            // by re-parsing the post-brace span as a sub-list and
            // checking the count.  Without this, an interpreted proc
            // body whose params include defaults (e.g. tcltest's
            // ``proc ConstraintInitializer {constraint {script ""}}``
            // sourced at runtime) treated ``script ""`` as a single
            // literal name — the first arg-less call wired up a local
            // named ``script "" `` (with embedded space and quotes)
            // and the body's ``$script`` reference then errored
            // "no such variable".  In tcltest's SafeFetch path that
            // error is caught silently and the read trace returns
            // ``""``, which PR #341's strict-boolean ``!`` then
            // trapped on (string.test regression).  Reference Tcl
            // (``TclCreateProc``) parses the sub-list at proc-define
            // time; doing it on every call keeps the binding cheap
            // and matches the existing eval-fallback shape — the
            // compiled-codegen path already does its own pre-parse
            // via ``tcl_proc_register_compiled``.
            var param_name_ptr = ps.ptr + param_elem.start;
            var param_name_len = param_elem.len;
            var param_default: i32 = 0;
            var has_default: bool = false;
            const sub_n = list_count_elements(param_name_ptr, param_name_len);
            if (sub_n == 2) {
                const name_elem = list_element_at(param_name_ptr, param_name_len, 0);
                const default_elem = list_element_at(param_name_ptr, param_name_len, 1);
                const default_ptr = param_name_ptr + default_elem.start;
                const default_len = default_elem.len;
                // ``obj_new_string_copy`` already returns rc=1 — the
                // earlier extra ``tcl_obj_retain`` here pumped rc=2
                // and the matching ``defer release`` only dropped it
                // to 1, leaking one ref per defaulted parameter per
                // call (Codex review on PR #343).  ``local_set``
                // below retains independently when it stores the
                // value in a frame slot, so the creator-side defer
                // release alone is enough to free ``param_default``
                // when the slot release fires at frame teardown
                // (or immediately when the caller-supplied value
                // overrides our default and we never store it).
                param_default = obj_new_string_copy(default_ptr, default_len);
                has_default = true;
                // Narrow the name span before allocating ``param_name``.
                const new_name_ptr = param_name_ptr + name_elem.start;
                const new_name_len = name_elem.len;
                param_name_ptr = new_name_ptr;
                param_name_len = new_name_len;
            }
            const param_name = obj_new_string_copy(param_name_ptr, param_name_len);
            defer if (has_default) obj_mod.tcl_obj_release(param_default);
            // Issue #317: ``local_set`` only reads ``param_name``'s byte
            // span (the hash table copies the bytes into its own
            // bucket); the TclObj itself is not retained anywhere
            // afterwards.  Without this defer, every proc invocation
            // leaked one TclObj per parameter — the dominant io.test
            // residual leak source after #303 closed the eval-loop
            // intermediate-result paths.  The same fix applies in
            // ``eval_apply`` below.
            defer obj_mod.tcl_obj_release(param_name);
            const param_name_s: [*]const u8 = @ptrFromInt(param_name_ptr);
            // argv[0] is the command name, so argv[pi+1] is the first arg
            const arg_idx = pi + 1;
            // Check if this is the special "args" parameter (last param only)
            const is_args_param = (pi == n_params - 1) and
                (param_name_len == 4) and
                param_name_s[0] == 'a' and param_name_s[1] == 'r' and
                param_name_s[2] == 'g' and param_name_s[3] == 's';
            if (is_args_param) {
                // Collect all remaining arguments into a list
                if (arg_idx >= words.len) {
                    // No remaining args: set to empty list.  Drop the
                    // creator-side ref after ``local_set`` retains so
                    // the frame slot is the only owner — otherwise the
                    // empty TclObj leaks at rc=1 once the frame pops
                    // (issue #317).
                    const empty = obj_new_string(0, 0);
                    _ = frames.local_set(param_name, empty);
                    obj_mod.tcl_obj_release(empty);
                } else if (arg_idx + 1 == words.len) {
                    // Exactly one remaining arg: use it directly as a list
                    _ = frames.local_set(param_name, words[arg_idx]);
                } else {
                    // Multiple remaining args: build a list
                    var total: u32 = 0;
                    var ai: u32 = arg_idx;
                    while (ai < words.len) : (ai += 1) {
                        const sv = obj_ensure_string(words[ai]);
                        total += sv.len * 2 + 4; // generous quoting estimate
                        if (ai > arg_idx) total += 1;
                    }
                    const args_alloc_size: u32 = total + 4;
                    const buf = alloc(args_alloc_size);
                    var off: u32 = 0;
                    ai = arg_idx;
                    while (ai < words.len) : (ai += 1) {
                        if (ai > arg_idx) {
                            const d: [*]u8 = @ptrFromInt(buf + off);
                            d[0] = ' ';
                            off += 1;
                        }
                        const sv = obj_ensure_string(words[ai]);
                        // ai starts at ``arg_idx`` (first element of the
                        // ``args`` list) and increases; only that first
                        // element gets leading-# quoting.
                        if (ai == arg_idx) {
                            off = obj_mod.list_elem_quote(buf, off, sv.ptr, sv.len);
                        } else {
                            off = obj_mod.list_elem_quote_nth(buf, off, sv.ptr, sv.len);
                        }
                    }
                    // Issue #317: ``obj_new_string_take`` lets the
                    // resulting TclObj own ``buf`` (so its release
                    // frees the backing slab via ``free_sized``); the
                    // matching ``tcl_obj_release`` after ``local_set``
                    // drops our creator-side ref so the frame slot is
                    // the only owner.
                    const args_obj = obj_new_string_take(buf, off, args_alloc_size);
                    _ = frames.local_set(param_name, args_obj);
                    obj_mod.tcl_obj_release(args_obj);
                }
                break;
            } else {
                if (arg_idx < words.len) {
                    // Caller-owned TclObj — no creator-side ref to drop.
                    _ = frames.local_set(param_name, words[arg_idx]);
                } else if (has_default) {
                    // Use the parsed default value when the caller
                    // omitted this positional arg.
                    _ = frames.local_set(param_name, param_default);
                } else {
                    // No default and no caller-supplied value —
                    // reference Tcl raises ``wrong # args`` here, but
                    // legacy tests (and our own existing baselines)
                    // rely on the lenient empty-default behaviour, so
                    // keep that for the no-default case.
                    const empty = obj_new_string(0, 0);
                    _ = frames.local_set(param_name, empty);
                    obj_mod.tcl_obj_release(empty);
                }
            }
        }
    }

    // Evaluate body
    const body_s = obj_ensure_string(body_obj);
    const result = eval_script(body_s.ptr, body_s.len);

    // Pop frame
    frames.frame_pop();

    // Absorb return signal (like picol: PICOL_RETURN → PICOL_OK)
    const ir_proc = result_mod.snapshot(result);
    if (ir_proc.code == .RETURN) {
        const tcl_catch = @import("tcl_catch.zig");
        const rv = ir_proc.value;
        // ``return -code return`` / ``-level N`` propagates past the
        // immediate proc — keep the flag set, decrement the extra-
        // level counter, and let the next proc up absorb.
        if (ir_proc.return_level > 0) {
            tcl_catch.state.return_level -= 1;
            return rv;
        }
        // MM-B.5: ``return_val`` was retained by ``eval_return``.
        // Hand its reference to the caller (no transfer-side
        // retain needed) and clear the slot so a recursive
        // ``return`` doesn't see a stale pointer in its release
        // path.  Caller's standard "holds 1" contract is met.
        result_mod.consume(.RETURN);
        return rv;
    }
    // Body-raised break/continue without an enclosing loop is a Tcl
    // error per the docs (``invoked "break"/"continue" outside of a
    // loop``).  Convert it here.  The opposite case — caller leaked
    // its own pending break/continue into us — restores the caller's
    // signal so an outer compiled loop still observes it.
    const body_break = ir_proc.code == .BREAK;
    const body_continue = ir_proc.code == .CONTINUE;
    const sig = result_mod.take_signal_break_continue();
    result_mod.break_continue_restore(bc_snap);
    // ``return -code break`` / ``return -code continue`` route the
    // signal to the caller's enclosing loop instead of triggering
    // the "outside of a loop" error.  Re-stamp the caller's flag so
    // an enclosing while/foreach catches the loop-control exit.
    if (sig.was_break) {
        result_mod.set_break();
        return result;
    }
    if (sig.was_continue) {
        result_mod.set_continue();
        return result;
    }
    if (body_break) {
        const msg_text = "invoked \"break\" outside of a loop";
        const msg = rt.obj_new_string_copy(@intFromPtr(msg_text.ptr), msg_text.len);
        rt.tcl_cmd_error(msg);
    } else if (body_continue) {
        const msg_text = "invoked \"continue\" outside of a loop";
        const msg = rt.obj_new_string_copy(@intFromPtr(msg_text.ptr), msg_text.len);
        rt.tcl_cmd_error(msg);
    }
    return result;
}

pub fn eval_interp(words: []const i32) i32 {
    if (words.len < 2) {
        const catch_mod = @import("tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp cmd ?arg ...?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "hide")) return interp_impl.eval_interp_hide(words);
    if (str_eq(sp, sub.len, "expose")) return interp_impl.eval_interp_expose(words);
    if (str_eq(sp, sub.len, "hidden")) return interp_impl.eval_interp_hidden(words);
    if (str_eq(sp, sub.len, "invokehidden")) return eval_interp_invokehidden(words);
    if (str_eq(sp, sub.len, "create")) return interp_impl.eval_interp_create(words);
    if (str_eq(sp, sub.len, "eval")) return eval_interp_eval(words);
    if (str_eq(sp, sub.len, "exists")) return interp_impl.eval_interp_exists(words);
    if (str_eq(sp, sub.len, "slaves") or str_eq(sp, sub.len, "children")) {
        return interp_impl.eval_interp_slaves(words);
    }
    if (str_eq(sp, sub.len, "delete")) return interp_impl.eval_interp_delete(words);
    if (str_eq(sp, sub.len, "target")) return interp_impl.eval_interp_target(words);
    if (str_eq(sp, sub.len, "issafe") or str_eq(sp, sub.len, "safe")) {
        return interp_impl.eval_interp_issafe(words);
    }
    // ``interp share interpA channel interpB`` / ``interp transfer
    // interpA channel interpB`` — channel ownership manipulation
    // between interpreters.  Our runtime exposes ``stdin`` /
    // ``stdout`` / ``stderr`` as pre-shared global channels and has no
    // user-creatable channels (``open`` and ``socket`` traps as
    // unsupported), so there is no per-interp channel table to update.
    // Accept the call as a no-op so cmdAH-31's interp-sharing tests
    // proceed past the inter-test setup steps.  Real behaviour can
    // land alongside a real channel-isolation table later.
    if (str_eq(sp, sub.len, "share") or str_eq(sp, sub.len, "transfer")) {
        return 0;
    }
    // ``interp share-variable LOCAL_PATH LOCAL_NAME TARGET_PATH
    // ?TARGET_NAME?`` — Phase 9 cross-interp variable link.  Aliases
    // ``LOCAL_NAME`` in the LOCAL_PATH interp to ``TARGET_NAME``
    // (defaults to ``LOCAL_NAME``) in TARGET_PATH.  Subsequent
    // reads / writes of the local name route through the target's
    // resolution.  This is a custom extension on top of the C Tcl
    // surface — ``interp share`` proper only handles channels.
    if (str_eq(sp, sub.len, "share-variable")) {
        if (words.len < 5 or words.len > 6) {
            const catch_mod = @import("tcl_catch.zig");
            const err_text = "wrong # args: should be \"interp share-variable localPath localName targetPath ?targetName?\"";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        }
        const local_interp = interp_impl.resolve_interp_path(words[2]);
        if (local_interp == 0) return 0;
        const target_interp = interp_impl.resolve_interp_path(words[4]);
        if (target_interp == 0) return 0;
        const target_name = if (words.len == 6) words[5] else words[3];
        const xlinks = @import("tcl_xlinks.zig");
        if (!xlinks.install(local_interp, words[3], target_interp, target_name)) {
            const catch_mod = @import("tcl_catch.zig");
            const err_text = "interp share-variable: out of memory";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        }
        return obj_new_string(0, 0);
    }
    // ``interp unshare-variable LOCAL_PATH LOCAL_NAME`` — remove
    // a previously-installed cross-interp variable link.  No-op
    // when no link exists (matches reference Tcl's tolerance for
    // the common "remove if present" pattern).
    if (str_eq(sp, sub.len, "unshare-variable")) {
        if (words.len != 4) {
            const catch_mod = @import("tcl_catch.zig");
            const err_text = "wrong # args: should be \"interp unshare-variable localPath localName\"";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        }
        const local_interp = interp_impl.resolve_interp_path(words[2]);
        if (local_interp == 0) return 0;
        const xlinks = @import("tcl_xlinks.zig");
        _ = xlinks.remove(local_interp, words[3]);
        return obj_new_string(0, 0);
    }
    if (!str_eq(sp, sub.len, "alias") and !str_eq(sp, sub.len, "aliases")) {
        // Unrecognised subcommand: raise tclsh's ``bad option "X"``
        // error rather than the stub-dispatch trap.  Matches
        // ``InterpObjCmd`` in tclInterp.c.
        interp_impl.emit_bad_option(sub.ptr, sub.len);
        return 0;
    }

    // ``interp aliases ?path?``: list every alias in the target
    // interp.  We traverse the namespace tree and collect every
    // Command with CMD_ALIAS set, emitting its simple name.  Empty
    // path (or no path) resolves to the current interp.
    if (str_eq(sp, sub.len, "aliases")) {
        if (words.len > 3) {
            const catch_mod = @import("tcl_catch.zig");
            const err_text = "wrong # args: should be \"interp aliases ?path?\"";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        }
        const target_interp: u32 = if (words.len == 3) blk: {
            const t = interp_impl.resolve_interp_path(words[2]);
            if (t == 0) return 0;
            break :blk t;
        } else interp_reg.interp_current();
        return interp_impl.interp_aliases_list_for(target_interp);
    }

    // ``interp alias childPath newName ?targetPath target ?arg…??``
    // shapes:
    //
    //   interp alias childPath newName                           (query)
    //   interp alias childPath newName {}                        (delete)
    //   interp alias childPath newName targetPath target ?arg…?  (create)
    //
    // Both paths are resolved against the current interp's child
    // tree.  Empty paths collapse to "this interp".
    if (words.len < 4) {
        const catch_mod = @import("tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp alias path ?arg ...?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const child_interp = interp_impl.resolve_interp_path(words[2]);
    if (child_interp == 0) return 0;
    const new_name = obj_ensure_string(words[3]);
    if (words.len == 4) {
        // Query form: ``interp alias childPath newName``.
        return interp_impl.interp_alias_query(child_interp, new_name.ptr, new_name.len);
    }
    // words[4] = parent-side interp path.
    if (words.len == 5) {
        const src_path = obj_ensure_string(words[4]);
        if (src_path.len == 0) {
            // Delete form: ``interp alias childPath newName {}``.
            return interp_impl.interp_alias_delete(child_interp, new_name.ptr, new_name.len);
        }
    }
    // words[5+] = target cmd + prefix args.
    if (words.len < 6) {
        const catch_mod = @import("tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp alias path srcCmd path targetCmd ?arg ...?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const parent_interp = interp_impl.resolve_interp_path(words[4]);
    if (parent_interp == 0) return 0;
    const target_name = obj_ensure_string(words[5]);
    // Pack prefix args into an arena-allocated u32 array.  See
    // the matching site higher up — alias_alloc heap-copies the
    // handles, so this buffer is pure scratch.
    const n_prefix: u32 = @as(u32, @intCast(words.len)) - 6;
    const arena_saved = arena.arena_save();
    defer arena.arena_restore(arena_saved);
    var prefix_buf: u32 = 0;
    var prefix_alloc: arena.Allocation = .{ .addr = 0, .size = 0, .from_arena = true };
    if (n_prefix > 0) {
        prefix_alloc = arena.arena_alloc_or_libc(n_prefix * 4);
        prefix_buf = prefix_alloc.addr;
        var i: u32 = 0;
        while (i < n_prefix) : (i += 1) {
            write_i32(prefix_buf + i * 4, words[6 + i]);
        }
    }
    const result = interp_impl.interp_alias_create(
        child_interp,
        parent_interp,
        new_name.ptr,
        new_name.len,
        target_name.ptr,
        target_name.len,
        n_prefix,
        prefix_buf,
    );
    arena.arena_free(prefix_alloc);
    return result;
}

fn eval_interp_invokehidden(words: []const i32) i32 {
    const catch_mod = @import("tcl_catch.zig");
    if (words.len < 4) {
        const err_text = "wrong # args: should be \"interp invokehidden path ?-global? ?-namespace ns? hiddenCmdName ?arg ...?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }

    // words[0] = "interp", words[1] = "invokehidden", words[2] = path.
    // Resolve the path up front so cross-interp invokehidden lands in
    // the resolved child's hidden table / namespace tree.
    const target_interp = interp_impl.resolve_interp_path(words[2]);
    if (target_interp == 0) return 0;
    var idx: u32 = 3;
    var use_global: bool = false;
    // ``-namespace NS`` is deferred: we remember the raw bytes and
    // resolve via ``ns_create_from_fqn`` *after* the ``enter`` swap
    // so the namespace lands in the target interp's ns tree rather
    // than the caller's.  Previously we resolved up-front, which for
    // cross-interp invokehidden would create ``NS`` in the wrong
    // interp and then execute against that foreign handle.
    var ns_name_ptr: u32 = 0;
    var ns_name_len: u32 = 0;
    var saw_namespace: bool = false;
    while (idx < words.len) : (idx += 1) {
        const arg = obj_ensure_string(words[idx]);
        const ap: [*]const u8 = @ptrFromInt(arg.ptr);
        if (str_eq(ap, arg.len, "-global")) {
            use_global = true;
            continue;
        }
        if (str_eq(ap, arg.len, "-namespace")) {
            if (idx + 1 >= words.len) {
                const err_text = "missing argument to -namespace";
                const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
                catch_mod.tcl_cmd_error(msg);
                return 0;
            }
            idx += 1;
            const ns_s = obj_ensure_string(words[idx]);
            ns_name_ptr = ns_s.ptr;
            ns_name_len = ns_s.len;
            saw_namespace = true;
            continue;
        }
        // First non-flag argument is the hidden command name.
        break;
    }
    if (idx >= words.len) {
        const err_text = "wrong # args: should be \"interp invokehidden path ?-global? ?-namespace ns? hiddenCmdName ?arg ...?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    if (use_global and saw_namespace) {
        const err_text = "cannot use -global option and -namespace option together";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }

    const hidden_name = obj_ensure_string(words[idx]);
    const cmd = interp_reg.hidden_find(target_interp, hidden_name.ptr, hidden_name.len);
    if (cmd == 0) {
        interp_impl.interp_hide_error("invalid hidden command name \"", hidden_name.ptr, hidden_name.len, "\"");
        return 0;
    }

    // Build the call-site argv: ``[hidden_name, caller_args...]``.
    // The hidden Command's ``name_ptr`` slot already holds the
    // hidden-name bytes (set by ``hide_command``); we re-wrap as a
    // TclObj so dispatch sees the same shape it'd see from an
    // ordinary ``cmd arg…`` call.
    const tail_count: u32 = @intCast(words.len - 1 - idx);
    const total: u32 = 1 + tail_count;
    if (total > parse.MAX_WORDS) {
        const err_text = "invokehidden argv exceeds MAX_WORDS";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    var new_words: [parse.MAX_WORDS]i32 = undefined;
    new_words[0] = rt.obj_new_string(@bitCast(hidden_name.ptr), @bitCast(hidden_name.len));
    var k: u32 = 0;
    while (k < tail_count) : (k += 1) {
        new_words[1 + k] = words[idx + 1 + k];
    }

    // Dispatch namespace selection.  Default (no flag) and
    // ``-global`` both anchor at the target interp's root; ``-namespace
    // ns`` anchors at the resolved namespace.  For cross-interp
    // invokehidden we swap into the target interp so ``ns_root()``
    // resolves against the child's namespace tree and the child's
    // procs / vars become reachable.  Save / restore in one pair.
    //
    // The ``-namespace`` resolution (find-or-create) runs *after*
    // ``enter`` so the ns lands in the target's tree.  Previously
    // it ran before and the handle pointed at a namespace in the
    // caller's tree — undefined behaviour for cross-interp calls.
    const swapped = target_interp != interp_reg.interp_current();
    const save = if (swapped) interp_reg.enter(target_interp) else interp_reg.EnterSave{
        .prev_interp = interp_reg.current_interp,
        .prev_root_addr = tcl_ns.root_addr,
        .prev_current_ns = tcl_ns.current_ns,
    };
    if (saw_namespace) {
        tcl_ns.current_ns = tcl_ns.ns_create_from_fqn(ns_name_ptr, ns_name_len);
    } else {
        // Both ``-global`` (explicit) and the default (no flag) land
        // at the target interp's root — matches the C Tcl behaviour
        // where ``InvokeHiddenObjCmd`` pushes the global frame when
        // neither flag is set.
        tcl_ns.current_ns = tcl_ns.ns_root();
    }
    const result = eval_proc_call_bucket(new_words[0..total], @bitCast(cmd));
    interp_reg.leave(save);
    return result;
}

fn eval_interp_eval(words: []const i32) i32 {
    if (words.len < 4) {
        const catch_mod = @import("tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp eval path arg ?arg ...?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const target_interp = interp_impl.resolve_interp_path(words[2]);
    if (target_interp == 0) return 0;

    // Concatenate words[3..] with single-space separator.
    var total: u32 = 0;
    var k: u32 = 3;
    while (k < words.len) : (k += 1) {
        total += @as(u32, @intCast(obj_ensure_string(words[k]).len));
        if (k + 1 < words.len) total += 1;
    }
    const script_ptr: u32 = if (total == 0) 0 else alloc(total);
    var off: u32 = 0;
    k = 3;
    while (k < words.len) : (k += 1) {
        const s = obj_ensure_string(words[k]);
        if (s.len > 0) {
            memcpy(script_ptr + off, s.ptr, s.len);
            off += s.len;
        }
        if (k + 1 < words.len) {
            const d: [*]u8 = @ptrFromInt(script_ptr + off);
            d[0] = ' ';
            off += 1;
        }
    }

    // Swap into the target interp, eval, restore.  Single-interp
    // callers that hand us ``path == {}`` resolve to the current
    // interp and skip the swap — same shape the alias / invokehidden
    // paths use.  Procs cached by ``proc_lookup`` are keyed on
    // (ns, ...) so the LRU doesn't cross-pollute between interps.
    const swapped = target_interp != interp_reg.interp_current();
    const save = if (swapped) interp_reg.enter(target_interp) else interp_reg.EnterSave{
        .prev_interp = interp_reg.current_interp,
        .prev_root_addr = tcl_ns.root_addr,
        .prev_current_ns = tcl_ns.current_ns,
    };
    const result = if (off > 0) eval_script(script_ptr, off) else 0;
    interp_reg.leave(save);
    return result;
}

// -- Main eval entry point --

// Maximum number of words after {*} expansion.  The parse limit is
// MAX_WORDS per command, but each {*}$var can expand to many elements.
// 128 is generous enough for realistic Tcl calls while staying cheap
// on the WASM stack.
const MAX_EXPANDED_WORDS: u32 = 128;

/// Public entry point into ``eval_command`` — lets cmds/flow.zig (tailcall)
/// dispatch a command by words slice without exposing the private function.
pub fn eval_call(words: []const i32) i32 {
    return eval_command(words);
}

/// ``apply`` — invoke an anonymous lambda: apply {paramList body ?ns?} ?arg ...?
/// Public so tcl_env_stubs.zig can call it from the 2-arg compiled export.
pub fn eval_apply(words: []const i32) i32 {
    if (words.len < 2) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"apply lambdaExpr ?arg ...?\"");
        return 0;
    }
    const tcl_ns_mod = @import("tcl_ns.zig");

    // Parse lambda tuple: {paramList body ?ns?}
    const lambda_s = obj_ensure_string(words[1]);
    const n_parts = list_count_elements(lambda_s.ptr, lambda_s.len);
    if (n_parts < 2) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("bad lambda expression: must have 2 or 3 elements");
        return 0;
    }

    const params_elem = list_element_at(lambda_s.ptr, lambda_s.len, 0);
    const body_elem = list_element_at(lambda_s.ptr, lambda_s.len, 1);

    const params_obj = if (params_elem.braced)
        obj_new_string_copy(lambda_s.ptr + params_elem.start, params_elem.len)
    else blk: {
        const alloc_size: u32 = params_elem.len + 4;
        const buf = alloc(alloc_size);
        const out_len = copy_unbraced_elem(buf, lambda_s.ptr + params_elem.start, params_elem.len);
        break :blk obj_new_string_take(buf, out_len, alloc_size);
    };
    // Issue #317: ``params_obj`` / ``body_obj`` are local copies of
    // the lambda's two list elements; they're never stored in a
    // frame slot or registered command, so without these defers
    // every ``apply`` call leaks both TclObjs (and, before
    // ``obj_new_string_take``, their backing bufs as well).
    defer obj_mod.tcl_obj_release(params_obj);
    const body_obj = if (body_elem.braced)
        obj_new_string_copy(lambda_s.ptr + body_elem.start, body_elem.len)
    else blk: {
        const alloc_size: u32 = body_elem.len + 4;
        const buf = alloc(alloc_size);
        const out_len = copy_unbraced_elem(buf, lambda_s.ptr + body_elem.start, body_elem.len);
        break :blk obj_new_string_take(buf, out_len, alloc_size);
    };
    defer obj_mod.tcl_obj_release(body_obj);

    // Optional namespace from third lambda element
    const saved_ns: u32 = tcl_ns_mod.current_ns;
    defer tcl_ns_mod.current_ns = saved_ns;
    if (n_parts >= 3) {
        const ns_elem = list_element_at(lambda_s.ptr, lambda_s.len, 2);
        if (ns_elem.len > 0) {
            const ns_ptr: u32 = lambda_s.ptr + ns_elem.start;
            tcl_ns_mod.current_ns = tcl_ns_mod.ns_create_from_fqn(
                @bitCast(ns_ptr),
                @bitCast(ns_elem.len),
            );
        }
    }

    _ = frames.frame_push();
    // Phase 8: ``apply`` is a proc-style invocation but with an
    // anonymous lambda — record the body as the script and stamp
    // ``apply`` as the proc name so ``info frame`` surfaces a
    // useful diagnostic.
    frames.frame_set_type(.PROC);
    frames.frame_set_script(body_obj);
    {
        const apply_lit: []const u8 = "apply";
        const fq = obj_new_string_copy(@intFromPtr(apply_lit.ptr), apply_lit.len);
        frames.frame_set_proc_name(fq);
        obj_mod.tcl_obj_release(fq);
    }

    // Bind parameters — same pattern as eval_proc_call_bucket but user
    // args start at words[2] (words[0]=apply, words[1]=lambda).
    // Issue #317: refcount discipline matches eval_proc_call_bucket —
    // see the comment block there for the per-branch leak rationale.
    const ps = obj_ensure_string(params_obj);
    const n_params: u32 = @intCast(list_count_elements(ps.ptr, ps.len));
    if (ps.len > 0 and n_params > 0) {
        var pi: u32 = 0;
        while (pi < n_params) : (pi += 1) {
            const param_elem = list_element_at(ps.ptr, ps.len, @intCast(pi));
            const param_name_ptr = ps.ptr + param_elem.start;
            const param_name_len = param_elem.len;
            const param_name = obj_new_string_copy(param_name_ptr, param_name_len);
            defer obj_mod.tcl_obj_release(param_name);
            const param_name_s: [*]const u8 = @ptrFromInt(param_name_ptr);
            const arg_idx = pi + 2; // skip words[0]=apply, words[1]=lambda
            const is_args_param = (pi == n_params - 1) and (param_name_len == 4) and
                param_name_s[0] == 'a' and param_name_s[1] == 'r' and
                param_name_s[2] == 'g' and param_name_s[3] == 's';
            if (is_args_param) {
                if (arg_idx >= words.len) {
                    const empty = obj_new_string(0, 0);
                    _ = frames.local_set(param_name, empty);
                    obj_mod.tcl_obj_release(empty);
                } else if (arg_idx + 1 == words.len) {
                    _ = frames.local_set(param_name, words[arg_idx]);
                } else {
                    var total: u32 = 0;
                    var ai: u32 = arg_idx;
                    while (ai < words.len) : (ai += 1) {
                        const sv = obj_ensure_string(words[ai]);
                        total += sv.len * 2 + 4;
                        if (ai > arg_idx) total += 1;
                    }
                    const args_alloc_size: u32 = total + 4;
                    const buf = alloc(args_alloc_size);
                    var off: u32 = 0;
                    ai = arg_idx;
                    while (ai < words.len) : (ai += 1) {
                        if (ai > arg_idx) {
                            const d: [*]u8 = @ptrFromInt(buf + off);
                            d[0] = ' ';
                            off += 1;
                        }
                        const sv = obj_ensure_string(words[ai]);
                        if (ai == arg_idx) {
                            off = obj_mod.list_elem_quote(buf, off, sv.ptr, sv.len);
                        } else {
                            off = obj_mod.list_elem_quote_nth(buf, off, sv.ptr, sv.len);
                        }
                    }
                    const args_obj = obj_new_string_take(buf, off, args_alloc_size);
                    _ = frames.local_set(param_name, args_obj);
                    obj_mod.tcl_obj_release(args_obj);
                }
                break;
            } else {
                if (arg_idx < words.len) {
                    _ = frames.local_set(param_name, words[arg_idx]);
                } else {
                    const empty = obj_new_string(0, 0);
                    _ = frames.local_set(param_name, empty);
                    obj_mod.tcl_obj_release(empty);
                }
            }
        }
    }

    const body_s = obj_ensure_string(body_obj);
    const result = eval_script(body_s.ptr, body_s.len);

    frames.frame_pop();

    const ir_apply = result_mod.snapshot(result);
    if (ir_apply.code == .RETURN) {
        const tcl_catch = @import("tcl_catch.zig");
        const rv = ir_apply.value;
        if (ir_apply.return_level > 0) {
            tcl_catch.state.return_level -= 1;
            return rv;
        }
        // MM-B.5: same as eval_proc_call_bucket.
        result_mod.consume(.RETURN);
        return rv;
    }
    if (ir_apply.code == .BREAK or ir_apply.code == .CONTINUE) {
        result_mod.consume(ir_apply.code);
        rt.tcl_cmd_error(words[1]);
    }
    return result;
}

pub fn eval_script(script_ptr: u32, script_len: u32) i32 {
    if (script_len == 0) return 0;

    // Save any outer eval context so nested eval_script invocations
    // (e.g. a command-substitution inside a word) can restore it
    // when they return.  Routes through ``diag.push_eval_ctx`` /
    // ``pop_eval_ctx`` so the saved chain is also visible to
    // ``Tcl_LogCommandInfo``-style multi-frame traceback construction
    // (parse-9.1).
    const diag = @import("../dispatch/tcl_diag.zig");
    const saved_ctx_depth = diag.push_eval_ctx();
    defer diag.pop_eval_ctx(saved_ctx_depth);

    // P9.2: fast path — if this body was pre-parsed by
    // ``parse_cache.build_for_body`` (called from ``proc_register``
    // in P9.3), replay the cached command list without re-parsing.
    const parse_cache = @import("../valtypes/parse_cache.zig");
    const slab = parse_cache.lookup(script_ptr, script_len);
    if (slab != 0) {
        return eval_cached_slab(slab, script_ptr, script_len);
    }

    // Cold path: no cache entry — parse + execute inline the way
    // we always have.  The cache stays empty for bodies that
    // weren't pre-parsed; non-proc scripts (``eval``, ``uplevel``,
    // command subs) flow through here every call.
    const src: [*]const u8 = @ptrFromInt(script_ptr);
    var pos: u32 = 0;
    var result: i32 = 0;
    // Token-tree scratch for each parsed command.  A command contributes
    // at most MAX_WORDS ``.WORD`` / ``.SIMPLE_WORD`` tokens plus one
    // ``.EXPAND_WORD`` marker per ``{*}`` — so 2 * MAX_WORDS is the
    // worst-case slot count.
    var tok_buf: [2 * MAX_WORDS]parse.Token = undefined;

    // Phase 8 follow-up: line counter for ``info frame -line``.
    // Bumps whenever the parse loop advances past a newline in the
    // source.  Each command's stamp uses :func:`count_lines_to` from
    // the script start to the command start, then we update the
    // frame's recorded line via ``frame_set_line`` so a body
    // running inside this command observes the correct line number.
    //
    // ``eval_script`` re-enters itself for command substitutions
    // (``[…]``), ``eval`` / ``uplevel`` bodies, and ``subst`` partial
    // bracket execution.  Only the OUTERMOST eval_script for the
    // current proc body should drive the recorded line — a nested
    // eval would otherwise clobber the outer compiled stamp's line
    // and leave ``info frame -line`` reading the inner script's
    // line instead of the outer call site's.  Track re-entry via
    // ``eval_script_depth``; non-zero means "we're inside someone
    // else's eval_script call", so leave the line alone.
    eval_script_depth += 1;
    defer eval_script_depth -= 1;
    // Update the recorded line only when (a) this is the outermost
    // ``eval_script`` invocation for the chain (no nested clobbering)
    // AND (b) the current frame hasn't claimed line ownership for
    // compiled-proc codegen.  In the latter case the prologue's
    // emitted ``frame_set_line`` calls are the single source of
    // truth — eval_script entries that resolve a ``[…]`` inside a
    // compiled body must not overwrite the outer stamp.
    const update_line = (eval_script_depth == 1) and !frames.frame_line_owned_by_codegen();
    while (pos < script_len) {
        diag.current_eval_ptr = script_ptr;
        diag.current_eval_len = script_len;
        diag.current_eval_pos = pos;

        const cmd = parse.ParseCommand(src, pos, script_len, &tok_buf, tok_buf.len);
        // Compute 1-based line number of this command's start.
        if (update_line) {
            const cmd_line = count_lines_to(src, cmd.command_start);
            frames.frame_set_line(cmd_line);
        }
        pos = cmd.next;
        if (cmd.extra_chars_after_close) {
            // Reference Tcl raises this at parse time; route through
            // ``tcl_cmd_error`` so a surrounding ``catch`` observes it
            // and ``::errorInfo`` is stamped (parse-18.19/20/21,
            // set-1.3, set-3.3).
            const msg_text: []const u8 = if (cmd.extra_chars_after_quote)
                "extra characters after close-quote"
            else
                "extra characters after close-brace";
            const buf = obj_mod.alloc(@intCast(msg_text.len));
            if (buf != 0) {
                const dst: [*]u8 = @ptrFromInt(buf);
                for (msg_text, 0..) |b, k| dst[k] = b;
                const tcl_catch = @import("tcl_catch.zig");
                const msg = obj_mod.obj_new_string_take(buf, @intCast(msg_text.len), @intCast(msg_text.len));
                tcl_catch.tcl_cmd_error(msg);
            }
            return result;
        }
        if (cmd.n_words == 0) continue;

        // Release the previous command's result obj before
        // overwriting (issue #303).  ``execute_parsed_command``
        // hands the caller a +1-refcount handle — without this
        // release, every script with N commands leaked N-1 result
        // TclObjs and pushed long-lived test bundles past the
        // wasm32 4 GiB linear-memory ceiling.
        if (result != 0) obj_mod.tcl_obj_release(result);
        result = execute_parsed_command(cmd.src_ptr, cmd.tokens_ptr, cmd.tokens_len);
        const ir = result_mod.snapshot(result);
        // ``Tcl_LogCommandInfo`` (parse-9.2 / parse-9.1): when an
        // error fires inside the executed command, append THIS
        // frame's command source to ``::errorInfo``.  Each frame
        // in the unwind contributes one ``while executing`` /
        // ``invoked from within`` line; the dedup happens inside
        // ``log_command_info`` (last_log_script/pos), and the
        // unwind stops when ``catch_leave`` clears
        // ``error_flag`` + ``last_log_*``.
        if (ir.code == .ERROR) {
            log_command_info(script_ptr, cmd.command_start, cmd.command_len);
        }
        // Yield is its own escape hatch — handled separately from the
        // main control-flow signal so a coroutine body that raises
        // ``error`` mid-iteration still propagates the error normally.
        if (result_mod.has_signal(ir)) return result;
        // Yield is its own escape hatch — not part of the
        // OK/ERROR/RETURN/BREAK/CONTINUE result code set, so
        // ``has_signal`` doesn't cover it.  Coroutine bodies that
        // yield mid-iteration unwind here.
        const tcl_catch_y = @import("tcl_catch.zig");
        if (tcl_catch_y.state.yield_flag != 0) return result;
    }
    return result;
}

/// Append a single ``while executing "<cmd>"`` (or, on subsequent
/// calls, ``invoked from within "<cmd>"``) frame to ``::errorInfo``.
/// Mirrors reference Tcl's ``Tcl_LogCommandInfo`` (tclBasic.c) — the
/// first call after a fresh error event uses ``while executing``;
/// every later call as the error unwinds through enclosing
/// ``eval_script`` frames uses ``invoked from within``.  ``catch``
/// stops the unwind by clearing ``error_flag``, so the trace
/// terminates at the catch boundary.
///
/// Per-call deduplication: if the same (script_ptr, cmd_start) was
/// already logged for this error event we skip — guards against an
/// inner eval_script that re-enters log_command_info before its
/// caller has unwound (e.g. recursive substitution paths).
fn log_command_info(script_ptr: u32, cmd_start: u32, cmd_len: u32) void {
    if (cmd_len == 0) return;
    const tcl_catch = @import("tcl_catch.zig");
    const cur = tcl_catch.state.error_msg;
    if (cur == 0) return;
    if (script_ptr == tcl_catch.state.last_log_script and cmd_start == tcl_catch.state.last_log_pos) return;
    const max_len: u32 = 150;
    const trunc_len: u32 = if (cmd_len > max_len) max_len else cmd_len;
    const need_ellipsis: bool = cmd_len > max_len;
    // Pick base text: ``::errorInfo`` when ``append_errinfo_frame``
    // appended a per-command frame (sentinel: last_log_script != 0
    // and last_log_pos == 0), else fall back to ``error_msg`` so a
    // stale errorInfo from a prior catch event doesn't leak text
    // into a new traceback (incr-2.30 / 2.31 / 2.32 / 2.33 want
    // the ``\n    (reading increment)`` frame the incr handler
    // appends).  The narrow sentinel match is the *only* path where
    // we read errorInfo back; every other ``log_command_info`` call
    // builds errorInfo from ``error_msg`` directly.
    const cur_s = blk: {
        const msg_s = obj_mod.obj_ensure_string(cur);
        if (tcl_catch.state.last_log_script != 0 and tcl_catch.state.last_log_pos == 0) {
            const ec_name_probe = obj_mod.obj_new_string_copy(@intFromPtr("::errorInfo".ptr), 11);
            defer obj_mod.tcl_obj_release(ec_name_probe);
            const ei_cur = tcl_ns.global_get(ec_name_probe);
            if (ei_cur != 0) {
                const ei_s = obj_mod.obj_ensure_string(ei_cur);
                if (ei_s.len > msg_s.len) {
                    const msg_p: [*]const u8 = @ptrFromInt(msg_s.ptr);
                    const ei_p: [*]const u8 = @ptrFromInt(ei_s.ptr);
                    var matches = true;
                    var k: u32 = 0;
                    while (k < msg_s.len) : (k += 1) {
                        if (msg_p[k] != ei_p[k]) {
                            matches = false;
                            break;
                        }
                    }
                    if (matches) break :blk ei_s;
                }
            }
        }
        break :blk msg_s;
    };
    // First frame after a fresh error → ``while executing``;
    // subsequent unwinding frames → ``invoked from within``.  The
    // ``last_log_script == 0`` sentinel is reset by ``tcl_cmd_error``
    // / ``catch_leave`` on each new error event.
    const prefix: []const u8 = if (tcl_catch.state.last_log_script == 0)
        "\n    while executing\n\""
    else
        "\n    invoked from within\n\"";
    const ellipsis = "...";
    const suffix = "\"";
    var total: u32 = cur_s.len + @as(u32, @intCast(prefix.len)) + trunc_len;
    if (need_ellipsis) total += @as(u32, @intCast(ellipsis.len));
    total += @as(u32, @intCast(suffix.len));
    const buf = obj_mod.alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    if (cur_s.len > 0) {
        const csp: [*]const u8 = @ptrFromInt(cur_s.ptr);
        for (0..cur_s.len) |k| {
            dst[off] = csp[k];
            off += 1;
        }
    }
    for (prefix) |b| {
        dst[off] = b;
        off += 1;
    }
    const cmdp: [*]const u8 = @ptrFromInt(script_ptr + cmd_start);
    for (0..trunc_len) |k| {
        dst[off] = cmdp[k];
        off += 1;
    }
    if (need_ellipsis) for (ellipsis) |b| {
        dst[off] = b;
        off += 1;
    };
    for (suffix) |b| {
        dst[off] = b;
        off += 1;
    }
    // Only update ``::errorInfo`` — ``error_msg`` (which becomes the
    // ``catch BODY msg`` variable) MUST stay just the original error
    // string per Tcl semantics.  The ``while executing`` frame
    // belongs in errorInfo only.
    const new_msg = obj_mod.obj_new_string_take(buf, total, total);
    const info_name = obj_mod.obj_new_string_copy(@intFromPtr("::errorInfo".ptr), 11);
    _ = tcl_ns.global_set(info_name, new_msg);
    obj_mod.tcl_obj_release(info_name);
    tcl_catch.state.last_log_script = script_ptr;
    tcl_catch.state.last_log_pos = cmd_start;
}

/// Replay pre-parsed command records from a parse-cache slab.
/// Exits on the first command that raises a signal (break /
/// continue / return / error) — same semantics as the cold path.
fn eval_cached_slab(slab: u32, body_ptr: u32, body_len: u32) i32 {
    const parse_cache = @import("../valtypes/parse_cache.zig");
    const diag = @import("../dispatch/tcl_diag.zig");
    const n_cmds = parse_cache.slab_n_commands(slab);
    var result: i32 = 0;
    var i: u32 = 0;
    const src: [*]const u8 = @ptrFromInt(body_ptr);
    // Phase 8 follow-up: same re-entry guard as the cold path so a
    // nested cached replay doesn't overwrite the outer frame's line.
    eval_script_depth += 1;
    defer eval_script_depth -= 1;
    // Same gate as the cold path — both checks must pass before we
    // can trust ``frame_set_line`` not to overwrite the outer
    // compiled stamp.
    const update_line = (eval_script_depth == 1) and !frames.frame_line_owned_by_codegen();
    while (i < n_cmds) : (i += 1) {
        const rec = parse_cache.command_record(slab, i);
        const tok_offset: u32 = @bitCast(obj_mod.read_i32(rec + parse_cache.OFF_CR_TOKENS_OFFSET));
        const tok_len: u32 = @bitCast(obj_mod.read_i32(rec + parse_cache.OFF_CR_TOKENS_LEN));
        const next_pos: u32 = @bitCast(obj_mod.read_i32(rec + parse_cache.OFF_CR_NEXT_POS));
        diag.current_eval_ptr = body_ptr;
        diag.current_eval_len = body_len;
        // The per-record ``next_pos`` captures the byte offset the
        // cold path would have advanced to after this command;
        // publishing it as ``current_eval_pos`` keeps trap
        // diagnostics pointing at the right source span for
        // errors triggered inside the dispatched command.
        const cmd_start: u32 = if (i == 0) 0 else @bitCast(obj_mod.read_i32(parse_cache.command_record(slab, i - 1) + parse_cache.OFF_CR_NEXT_POS));
        diag.current_eval_pos = cmd_start;
        // Phase 8 follow-up: stamp the frame's recorded line so a
        // command body's ``info frame`` reads the right line number.
        // O(cmd_start) per record; cached-body replay still wins on
        // parse cost which dominates for proc bodies.  Skipped when
        // we're a nested ``eval_script`` call — the outer stamp
        // survives.
        if (update_line) frames.frame_set_line(count_lines_to(src, cmd_start));
        const tokens_ptr = parse_cache.token_at(slab, tok_offset);
        // Release prior command result before overwriting — same
        // leak pattern as the cold path (issue #303).
        if (result != 0) obj_mod.tcl_obj_release(result);
        result = execute_parsed_command(body_ptr, tokens_ptr, tok_len);
        if (has_signal()) return result;
        _ = next_pos;
    }
    return result;
}

/// Execute the dispatch + eval_command body for a single parsed
/// command.  Factored out of ``eval_script`` so both the cold
/// (parse-on-demand) and warm (cached) paths share identical
/// command-execution semantics.  ``body_ptr`` is the base of the
/// original script bytes; ``parse.Token.start`` values are offsets
/// relative to it.
fn execute_parsed_command(body_ptr: u32, tokens_ptr: u32, tokens_len: u32) i32 {
    if (tokens_ptr == 0) return 0;
    const tokens: [*]parse.Token = @ptrFromInt(tokens_ptr);
    var has_expand = false;
    {
        var t: u32 = 0;
        while (t < tokens_len) : (t += 1) {
            if (tokens[t].kind == .EXPAND_WORD) {
                has_expand = true;
                break;
            }
        }
    }

    if (!has_expand) {
        // Fast path: no expansion.
        var word_objs: [MAX_WORDS]i32 = undefined;
        var wi: u32 = 0;
        var t: u32 = 0;
        while (t < tokens_len) : (t += 1) {
            const tok = tokens[t];
            if (tok.kind == .EXPAND_WORD) continue;
            const wptr_abs: u32 = body_ptr + tok.start;
            if (tok.braced) {
                // Phase 1.3 fix: copy braced-word bytes into an owned
                // buffer instead of borrowing from the source script.
                // See ``proc_register::ensure_owned`` for the matching
                // proc-table-side promotion.
                word_objs[wi] = obj_mod.obj_new_string_copy(wptr_abs, tok.len);
            } else {
                word_objs[wi] = subst_word(wptr_abs, tok.len);
            }
            wi += 1;
            // If a substitution raised break / continue / return /
            // error, abort word evaluation now and let the enclosing
            // eval_script loop observe the flag — a ``[continue]``
            // inside ``w[continue]`` must propagate as TCL_CONTINUE
            // (parse-17.1), not result in a phantom dispatch of ``w``.
            if (has_signal()) {
                var rj: u32 = 0;
                while (rj < wi) : (rj += 1) {
                    if (word_objs[rj] != 0) obj_mod.tcl_obj_release(word_objs[rj]);
                }
                return 0;
            }
        }
        // MM-B.4: release the per-word TclObjs after dispatch.
        // The dispatch result's refcount semantics are "+1 for the
        // caller" (caller releases when done).  But if dispatch
        // returned one of the words verbatim (e.g. ``return $x``,
        // or a builtin that promotes a word to its result), the
        // word-release loop below would decrement that refcount
        // alongside the others — leaving the returned handle with
        // rc=0 and queued for free.  Detect that case and retain
        // the result first so the word-release loop's decrement
        // exactly cancels the retain, leaving the caller with the
        // expected +1 ownership.  Without this scan, parseOld.test
        // tripped a use-after-free at site=146 with the obj's own
        // refcount field showing through as the trap message
        // bytes.  Buffer recycling under MM-B.4 is safe for
        // parse_cache thanks to the ``invalidate_for_buffer`` hook
        // in ``tcl_obj.release_now``.
        const result = eval_command(word_objs[0..wi]);
        var result_is_word = false;
        if (result != 0) {
            var sc: u32 = 0;
            while (sc < wi) : (sc += 1) {
                if (word_objs[sc] == result) {
                    result_is_word = true;
                    break;
                }
            }
        }
        if (result_is_word) obj_mod.tcl_obj_retain(result);
        var ri: u32 = 0;
        while (ri < wi) : (ri += 1) {
            if (word_objs[ri] != 0) obj_mod.tcl_obj_release(word_objs[ri]);
        }
        return result;
    }

    // Slow path: at least one {*} expansion.
    var expanded: [MAX_EXPANDED_WORDS]i32 = undefined;
    var ecount: u32 = 0;
    var pending_expand = false;
    var t: u32 = 0;
    while (t < tokens_len) : (t += 1) {
        const tok = tokens[t];
        if (tok.kind == .EXPAND_WORD) {
            pending_expand = true;
            continue;
        }
        const wptr_abs: u32 = body_ptr + tok.start;
        // Phase 1.3 fix: see the matching site above for the
        // borrow-vs-copy rationale.
        const word_obj: i32 = if (tok.braced)
            obj_mod.obj_new_string_copy(wptr_abs, tok.len)
        else
            subst_word(wptr_abs, tok.len);

        if (pending_expand) {
            pending_expand = false;
            const s = obj_ensure_string(word_obj);
            const n = list_count_elements(s.ptr, s.len);
            var j: i64 = 0;
            while (j < n) : (j += 1) {
                if (ecount >= MAX_EXPANDED_WORDS) break;
                const elem = list_element_at(s.ptr, s.len, j);
                if (elem.braced) {
                    expanded[ecount] = obj_new_string_copy(s.ptr + elem.start, elem.len);
                } else {
                    const buf = alloc(elem.len);
                    const out_len = copy_unbraced_elem(buf, s.ptr + elem.start, elem.len);
                    expanded[ecount] = obj_new_string_take(buf, out_len, elem.len);
                }
                ecount += 1;
            }
            // Release the source word obj — its bytes have been copied
            // into independently-owned ``expanded[]`` entries above.
            // Without this release, every ``{*}$args`` expansion leaked
            // the source list object, growing the heap monotonically
            // across long-running tcltest sweeps and exhausting the
            // 32-byte slab class.
            obj_mod.tcl_obj_release(word_obj);
        } else {
            if (ecount < MAX_EXPANDED_WORDS) {
                expanded[ecount] = word_obj;
                ecount += 1;
            }
        }
    }
    // MM-B.4 (slow path): release expanded[] elements after
    // dispatch, with the same alias-aware retain pattern as the
    // fast path.  Each ``{*}$args`` expansion element is a fresh
    // TclObj that owns its bytes — braced elements via
    // ``obj_new_string_copy`` (cap = rounded-up alloc), unbraced
    // via ``obj_new_string_take`` (cap matches the per-element
    // ``alloc(elem.len)`` that backed the backslash-decoded copy).
    // Issue #317: the older ``obj_new_string`` borrowing form left
    // ``cap = 0`` and leaked one buf per unbraced element on
    // release — observed under io.test as the residual heap
    // pressure that pushed the runtime past its 4 GiB ceiling.
    // In both cases an element's release is independent of any
    // peer; releasing one doesn't free a buffer another element
    // borrows.
    const result = eval_command(expanded[0..ecount]);
    var result_is_word = false;
    if (result != 0) {
        var sc: u32 = 0;
        while (sc < ecount) : (sc += 1) {
            if (expanded[sc] == result) {
                result_is_word = true;
                break;
            }
        }
    }
    if (result_is_word) obj_mod.tcl_obj_retain(result);
    var ri: u32 = 0;
    while (ri < ecount) : (ri += 1) {
        if (expanded[ri] != 0) obj_mod.tcl_obj_release(expanded[ri]);
    }
    return result;
}

// Exported: evaluate a Tcl script string.
//
// Drains the deferred-free queue at the outermost call boundary
// so a ``(ptr, len)`` borrowed across an ``alloc`` doesn't alias
// a freshly-recycled slab while the script body is still walking
// the bytes.  Nested calls (command substitution, ``eval``,
// ``[expr]`` bodies) skip the drain — the outer call cleans up
// for everyone when it unwinds.
var tcl_eval_depth: u32 = 0;
pub export fn tcl_eval(script: i32) i32 {
    const s = obj_ensure_string(script);
    if (s.len == 0) return 0;
    // Retain the script TclObj for the duration of eval_script so
    // parser-produced word TclObjs that BORROW from the script's
    // buffer (the common case for braced/quoted words) stay valid
    // through the eval — even when an intermediate site releases
    // its handle on the script.  Phase 1.3 fix.
    obj_mod.tcl_obj_retain(script);
    tcl_eval_depth += 1;
    const r = eval_script(s.ptr, s.len);
    tcl_eval_depth -= 1;
    obj_mod.tcl_obj_release(script);
    if (tcl_eval_depth == 0) {
        obj_mod.tcl_obj_drain_pending();
    }
    return r;
}
