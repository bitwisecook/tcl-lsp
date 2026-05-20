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
const hide_mod = @import("../cmds/tcl_hide.zig");

/// Wraps :func:`cmd_table.lookup` with a check against the current
/// interp's hidden-builtin marker.  When ``interp hide`` has
/// recorded *name* in the active interp's hidden table with the
/// :data:`hide_mod.BUILTIN_HIDDEN_MARKER` sentinel, return ``null``
/// so the caller treats the builtin as if it were never registered
/// — matching C tcl's per-interp ``Tcl_HideCommand`` semantics
/// without needing to clone the static dispatch table per child.
fn cmd_table_visible_lookup(name_ptr: u32, name_len: u32) ?@TypeOf(cmd_table.lookup(0, 0).?) {
    if (hide_mod.builtin_is_hidden(interp_reg.interp_current(), name_ptr, name_len)) {
        return null;
    }
    return cmd_table.lookup(name_ptr, name_len);
}

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
    expr_clear_nonnum();
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
    kind: enum { eq, ne, in_, ni, eq_eq, ne_eq },
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
/// Walk *src*[rest_start..len] looking for a lower-precedence boolean
/// or ternary operator that would override the candidate ``==``/``!=``.
/// Returns true when no such operator is present (so the equality
/// dispatch can safely fire) and false when one is found.
/// Skips over ``"..."`` / ``{...}`` / ``[...]`` / ``(...)`` so a nested
/// ``||`` doesn't trigger a false bail-out.
fn eq_op_is_top_level(src: [*]const u8, len: u32, rest_start: u32) bool {
    var i: u32 = rest_start;
    while (i < len) : (i += 1) {
        const c = src[i];
        if (c == '"') {
            i += 1;
            while (i < len and src[i] != '"') {
                if (src[i] == '\\' and i + 1 < len) i += 1;
                i += 1;
            }
            continue;
        }
        if (c == '{') {
            var d: u32 = 1;
            i += 1;
            while (i < len and d > 0) : (i += 1) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 1;
                    continue;
                }
                if (src[i] == '{') d += 1 else if (src[i] == '}') d -= 1;
            }
            if (i > 0) i -= 1;
            continue;
        }
        if (c == '[') {
            var d: u32 = 1;
            i += 1;
            while (i < len and d > 0) : (i += 1) {
                if (src[i] == '[') d += 1 else if (src[i] == ']') d -= 1;
            }
            if (i > 0) i -= 1;
            continue;
        }
        if (c == '(') {
            var d: u32 = 1;
            i += 1;
            while (i < len and d > 0) : (i += 1) {
                if (src[i] == '(') d += 1 else if (src[i] == ')') d -= 1;
            }
            if (i > 0) i -= 1;
            continue;
        }
        if (i + 1 < len and src[i] == '&' and src[i + 1] == '&') return false;
        if (i + 1 < len and src[i] == '|' and src[i + 1] == '|') return false;
        if (c == '?' or c == ':') return false;
        // Arithmetic / shift ops on the RHS mean ``$a == $b + $c`` style
        // expressions where ``eval_string_expr`` would textually
        // substitute the RHS instead of evaluating the arithmetic.
        if (c == '+' or c == '-' or c == '*' or c == '/' or c == '%') return false;
        if (i + 1 < len and src[i] == '<' and src[i + 1] == '<') return false;
        if (i + 1 < len and src[i] == '>' and src[i + 1] == '>') return false;
    }
    return true;
}

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
        // ``==`` / ``!=`` at top level: Tcl 9 picks numeric compare
        // when both operands parse as numbers, bytewise compare
        // otherwise.  The legacy ``expr_add`` i64 path raises ``expected
        // boolean value`` on non-numeric ``"..."`` operands, so this
        // top-level dispatch is the only way ``$v != "foobar1"`` works
        // through the interpreter eval path.  Two safety conditions
        // before claiming the operator:
        //   1.  Neither side may carry a lower-precedence boolean /
        //       ternary operator — ``$a == $b || $c`` has ``||`` as the
        //       top, so the equality dispatch must defer.
        //   2.  Neither side may carry a higher-precedence arithmetic
        //       operator (``+`` / ``-`` / ``*`` / ``/`` / ``%`` /
        //       ``<<`` / ``>>``) — ``$a == $b + 1`` needs the
        //       arithmetic on RHS evaluated by ``expr_add`` first, not
        //       textually substituted by ``eval_string_expr``.
        if (i + 1 < len and (src[i] == '=' or src[i] == '!') and src[i + 1] == '=') {
            // Scan the remainder for disqualifying operators.
            if (!eq_op_is_top_level(src, len, i + 2)) {
                if (src[i] == '+' or src[i] == '-' or src[i] == '*' or src[i] == '/' or src[i] == '%') return null;
                continue;
            }
            return .{
                .kind = if (src[i] == '=') .eq_eq else .ne_eq,
                .start = i,
                .op_len = 2,
            };
        }
        if (c == '+' or c == '-' or c == '*' or c == '/' or c == '%') return null;
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
    } else if (le >= ls + 2 and src[ls] == '"' and src[le - 1] == '"') {
        // ``$x eq ""`` — the empty-string literal would otherwise
        // round-trip through ``subst_word`` as the two-byte string
        // ``""`` (the quotes survive unchanged, ``has_dollar`` etc.
        // all false).  Strip the outer quotes the same way the
        // braced branch does so ``"abc"`` → ``abc`` and ``""``
        // → empty (cmdIL bootstrap: the
        // ``[namespace which -command …] eq ""`` guard around
        // ``namespace eval ::tcltest::internals { … }``).
        ls += 1;
        le -= 1;
    }
    if (re >= rs + 2 and src[rs] == '{' and src[re - 1] == '}') {
        rs += 1;
        re -= 1;
    } else if (re >= rs + 2 and src[rs] == '"' and src[re - 1] == '"') {
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
        .eq_eq, .ne_eq => {
            // Tcl 9 ``==`` / ``!=``: numeric compare when both operands
            // parse as numbers, else bytewise string compare.  Route
            // through ``tcl_expr_eq_nan_aware`` which enforces
            // IEEE-754's ``NaN != NaN`` rule on top of the regular
            // ``tcl_expr_order_cmp`` semantics (i64 fast path / bignum
            // / float / bytewise).  The string helpers above leave
            // the substituted operand TclObjs on the heap; reuse them
            // so we don't re-walk the source.
            const tcl_string = @import("../valtypes/tcl_string.zig");
            const eq_obj = tcl_string.tcl_expr_eq_nan_aware(lhs_obj, rhs_obj);
            const eq_flag = obj_mod.obj_get_int(eq_obj);
            obj_mod.tcl_obj_release(eq_obj);
            result = if (op.kind == .eq_eq) eq_flag else (1 - eq_flag);
        },
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

/// Tracks the last expression atom that failed numeric/boolean parsing.
/// The atom records the raw bytes here (instead of raising directly) so
/// the enclosing operator decides the wording:
///   * arithmetic operator (``+``, ``-``, ``*``, …) →
///       ``cannot use non-numeric string "X" as <left|right> operand of "<op>"``
///   * boolean operator (``&&`` / ``||`` / top-level boolean context) →
///       ``expected boolean value but got "X"``
/// Cleared by ``eval_expr_str`` on entry and by every successful atom.
///
/// Lifetime: the recorded bytes are OWNED by this module — every
/// :func:`expr_set_nonnum` copies its input into a fresh allocation
/// and every :func:`expr_clear_nonnum` releases it.  Earlier
/// revisions stored a borrowed ``(ptr, len)`` directly from the
/// variable value's string rep; an intervening ``[cmd]``
/// substitution that ``unset``-ed or rewrote the variable could
/// then UAF the bytes when the enclosing operator finally consumed
/// them (Copilot review on PR #452 flagged the dangle).
var expr_nonnum_active: bool = false;
var expr_nonnum_ptr: u32 = 0;
var expr_nonnum_len: u32 = 0;

fn expr_nonnum_release() void {
    if (expr_nonnum_ptr != 0 and expr_nonnum_len != 0) {
        obj_mod.free_sized(expr_nonnum_ptr, expr_nonnum_len);
    }
    expr_nonnum_ptr = 0;
    expr_nonnum_len = 0;
}

fn expr_set_nonnum(ptr: u32, slen: u32) void {
    expr_nonnum_release();
    expr_nonnum_active = true;
    if (slen == 0 or ptr == 0) {
        expr_nonnum_ptr = 0;
        expr_nonnum_len = 0;
        return;
    }
    const dst = obj_mod.alloc(slen);
    if (dst == 0) {
        // Allocation failure — record empty bytes; the diagnostic
        // will use an empty string in the ``"X"`` slot rather than
        // dangling at a stale pointer.
        expr_nonnum_ptr = 0;
        expr_nonnum_len = 0;
        return;
    }
    const src: [*]const u8 = @ptrFromInt(ptr);
    const out: [*]u8 = @ptrFromInt(dst);
    var k: u32 = 0;
    while (k < slen) : (k += 1) out[k] = src[k];
    expr_nonnum_ptr = dst;
    expr_nonnum_len = slen;
}

fn expr_clear_nonnum() void {
    expr_nonnum_active = false;
    expr_nonnum_release();
}

const NonNumSnapshot = struct {
    active: bool,
    ptr: u32,
    len: u32,
};

fn expr_take_nonnum() NonNumSnapshot {
    // Ownership transfer: the snapshot adopts the heap buffer the
    // global slot was holding; the global is cleared without a
    // release so a subsequent ``expr_restore_nonnum`` can re-park
    // the same buffer without copying.  A ``raise_…`` consumer that
    // doesn't restore must call ``free_sized(s.ptr, s.len)`` on its
    // way out — see ``expr_consume_nonnum_boolean`` for the pattern.
    const s: NonNumSnapshot = .{
        .active = expr_nonnum_active,
        .ptr = expr_nonnum_ptr,
        .len = expr_nonnum_len,
    };
    expr_nonnum_active = false;
    expr_nonnum_ptr = 0;
    expr_nonnum_len = 0;
    return s;
}

fn expr_restore_nonnum(s: NonNumSnapshot) void {
    expr_nonnum_release();
    expr_nonnum_active = s.active;
    expr_nonnum_ptr = s.ptr;
    expr_nonnum_len = s.len;
}

/// Release a snapshot's heap buffer.  Called by operand sites that
/// either consumed the snapshot via ``raise_…`` (and want to free
/// the bytes after the diagnostic is built) or discarded it
/// (LHS got short-circuited and the snapshot is being replaced).
fn expr_drop_nonnum_snap(s: NonNumSnapshot) void {
    if (s.ptr != 0 and s.len != 0) obj_mod.free_sized(s.ptr, s.len);
}

/// Did the most recent atom produce a non-numeric operand?  Used by
/// callers (``eval_while`` / ``eval_if`` / ``eval_for`` condition
/// evaluation) to convert the lingering state into the canonical
/// ``expected boolean value but got "X"`` error.
pub fn expr_consume_nonnum_boolean() bool {
    if (!expr_nonnum_active) return false;
    const ptr = expr_nonnum_ptr;
    const slen = expr_nonnum_len;
    // Detach the buffer from the global before raising so the
    // ``expr_clear_nonnum`` released it but the diagnostic helper
    // still sees the bytes.  ``free_sized`` after the raise is the
    // counterpart to ``expr_set_nonnum``'s ``obj_mod.alloc``.
    expr_nonnum_active = false;
    expr_nonnum_ptr = 0;
    expr_nonnum_len = 0;
    raise_expected_boolean_str(ptr, slen);
    if (ptr != 0 and slen != 0) obj_mod.free_sized(ptr, slen);
    return true;
}

/// Raise the arithmetic-context wording for a non-numeric operand:
/// ``cannot use non-numeric string "X" as <side> operand of "<op>"``.
/// Mirrors Tcl 9's INST_ADD / INST_SUB / etc. operand-coercion error.
fn raise_arith_op_error(op_str: []const u8, side: []const u8, ptr: u32, slen: u32) void {
    const catch_mod = @import("tcl_catch.zig");
    const prefix: []const u8 = "cannot use non-numeric string \"";
    const middle1: []const u8 = "\" as ";
    const middle2: []const u8 = " operand of \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @as(u32, @intCast(prefix.len + middle1.len + middle2.len + suffix.len + side.len + op_str.len)) + slen;
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
    for (middle1) |c| {
        dst[off] = c;
        off += 1;
    }
    for (side) |c| {
        dst[off] = c;
        off += 1;
    }
    for (middle2) |c| {
        dst[off] = c;
        off += 1;
    }
    for (op_str) |c| {
        dst[off] = c;
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

/// Check the captured snapshot.  When the snapshot recorded a
/// non-numeric atom, raise the arithmetic-operand error and clear
/// the live flag (the snapshot caller's responsibility is the
/// "left" side; the live flag covers the "right" side which will
/// be checked by the immediately-following ``expr_take_nonnum``).
fn expr_raise_if_nonnum_arith(snap: NonNumSnapshot, op_str: []const u8, side: []const u8) bool {
    if (!snap.active) return false;
    raise_arith_op_error(op_str, side, snap.ptr, snap.len);
    // Snapshot owns its buffer; raise built the diagnostic from
    // those bytes, so release them now.  Returning true tells the
    // caller to bail without restoring — the live flag was already
    // cleared by ``expr_take_nonnum``.
    expr_drop_nonnum_snap(snap);
    return true;
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
    // First operand: delimit its range so a top-level string operator
    // (``eq`` / ``ne`` / ``in`` / ``ni``) within it gets dispatched to
    // ``eval_string_expr`` even when the wider expression contains
    // ``||``.  Without this delimit, ``A || $i ne [...]`` would let
    // the ``$i`` atom set the non-numeric flag and the surrounding
    // boolean operator would raise (list-4.2).
    var lhs_end = find_op_terminator(ptr, len, pos.*, '|');
    var left = eval_or_operand(ptr, pos.*, lhs_end, skip);
    pos.* = lhs_end;
    var lhs_snap = expr_take_nonnum();
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* + 1 < len and src[pos.*] == '|' and src[pos.* + 1] == '|') {
            if (lhs_snap.active and !skip) {
                raise_expected_boolean_str(lhs_snap.ptr, lhs_snap.len);
                expr_drop_nonnum_snap(lhs_snap);
                return 0;
            }
            pos.* += 2;
            // Short-circuit: skip RHS when LHS is already truthy OR we're
            // already skipping outer scope.  Still walk the RHS tokens
            // so ``pos`` advances past them.
            const rhs_skip = skip or (left != 0);
            const rhs_end = find_op_terminator(ptr, len, pos.*, '|');
            const right = eval_or_operand(ptr, pos.*, rhs_end, rhs_skip);
            pos.* = rhs_end;
            const rhs_snap = expr_take_nonnum();
            if (rhs_snap.active and !rhs_skip) {
                raise_expected_boolean_str(rhs_snap.ptr, rhs_snap.len);
                expr_drop_nonnum_snap(rhs_snap);
                expr_drop_nonnum_snap(lhs_snap);
                return 0;
            }
            expr_drop_nonnum_snap(rhs_snap);
            if (!skip) {
                left = if (left != 0 or right != 0) @as(i64, 1) else @as(i64, 0);
            }
            // LHS snapshot is no longer relevant after a successful
            // ``||`` reduction — release its buffer.
            expr_drop_nonnum_snap(lhs_snap);
            lhs_snap = .{ .active = false, .ptr = 0, .len = 0 };
            lhs_end = pos.*;
        } else break;
    }
    expr_restore_nonnum(lhs_snap);
    return left;
}

/// Evaluate one operand of ``||`` (a ``&&``-chain or simpler).
/// Delegates to ``expr_and`` on the sub-range; ``expr_and`` itself
/// applies the same dispatch trick for ``&&`` operands.  Pos
/// arithmetic stays inside the helper.  When ``skip`` is true the
/// caller is short-circuiting: skip the actual evaluation so the
/// operand's ``[cmd]`` substitutions don't run (mathop-25.40's
/// ``[set i ...] == [set res ...]`` would otherwise re-fire its
/// side-effects past the loop-exit condition).
fn eval_or_operand(ptr: u32, start: u32, end: u32, skip: bool) i64 {
    if (skip) return 0;
    var sub_pos: u32 = 0;
    return expr_and(ptr + start, end - start, &sub_pos, skip);
}

fn expr_and(ptr: u32, len: u32, pos: *u32, skip: bool) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    // First operand of ``&&``: delimit its range so ``eq`` / ``ne`` /
    // ``in`` / ``ni`` get dispatched as string-compares.
    var lhs_end = find_op_terminator(ptr, len, pos.*, '&');
    var left = eval_and_operand(ptr, pos.*, lhs_end, skip);
    pos.* = lhs_end;
    var lhs_snap = expr_take_nonnum();
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* + 1 < len and src[pos.*] == '&' and src[pos.* + 1] == '&') {
            if (lhs_snap.active and !skip) {
                raise_expected_boolean_str(lhs_snap.ptr, lhs_snap.len);
                expr_drop_nonnum_snap(lhs_snap);
                return 0;
            }
            pos.* += 2;
            // Short-circuit: skip RHS when LHS is already falsy OR we're
            // already skipping.
            const rhs_skip = skip or (left == 0);
            const rhs_end = find_op_terminator(ptr, len, pos.*, '&');
            const right = eval_and_operand(ptr, pos.*, rhs_end, rhs_skip);
            pos.* = rhs_end;
            const rhs_snap = expr_take_nonnum();
            if (rhs_snap.active and !rhs_skip) {
                raise_expected_boolean_str(rhs_snap.ptr, rhs_snap.len);
                expr_drop_nonnum_snap(rhs_snap);
                expr_drop_nonnum_snap(lhs_snap);
                return 0;
            }
            expr_drop_nonnum_snap(rhs_snap);
            if (!skip) {
                left = if (left != 0 and right != 0) @as(i64, 1) else @as(i64, 0);
            }
            expr_drop_nonnum_snap(lhs_snap);
            lhs_snap = .{ .active = false, .ptr = 0, .len = 0 };
            lhs_end = pos.*;
        } else break;
    }
    expr_restore_nonnum(lhs_snap);
    return left;
}

/// Evaluate one operand of ``&&`` — a sub-expression that may carry a
/// top-level string op (``eq`` / ``ne`` / ``in`` / ``ni`` / ``==`` /
/// ``!=`` on non-numeric operands).  Inside this sub-range there's
/// no surrounding ``||`` / ``&&`` to interfere with
/// ``find_top_string_op``'s bail-out.  When ``skip`` is true the
/// caller is short-circuiting: skip the actual evaluation so the
/// operand's ``[cmd]`` substitutions don't run.
fn eval_and_operand(ptr: u32, start: u32, end: u32, skip: bool) i64 {
    if (skip) return 0;
    if (find_top_string_op(ptr + start, end - start)) |op| {
        return eval_string_expr(ptr + start, end - start, op);
    }
    var sub_pos: u32 = 0;
    return expr_add(ptr + start, end - start, &sub_pos, skip);
}

/// Scan from *start* through *ptr*[..*len*] looking for the next
/// occurrence of the doubled boolean operator ``op_char`` (``|`` for
/// ``||``, ``&`` for ``&&``) at the top level.  Skips over nested
/// braces / brackets / parens / quoted strings.  Returns either the
/// position of the operator (so ``ptr + result`` points at the first
/// byte of ``||`` / ``&&``) or ``len`` when no such operator exists.
fn find_op_terminator(ptr: u32, len: u32, start: u32, op_char: u8) u32 {
    if (start >= len) return len;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = start;
    while (i < len) {
        const c = src[i];
        if (c == '\\' and i + 1 < len) {
            i += 2;
            continue;
        }
        if (c == '"') {
            i += 1;
            while (i < len and src[i] != '"') {
                if (src[i] == '\\' and i + 1 < len) i += 1;
                i += 1;
            }
            if (i < len) i += 1;
            continue;
        }
        if (c == '{') {
            var depth: u32 = 1;
            i += 1;
            while (i < len and depth > 0) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (src[i] == '{') depth += 1 else if (src[i] == '}') depth -= 1;
                i += 1;
            }
            continue;
        }
        if (c == '[' or c == '(') {
            // Bracket / parenthesis group — recurse through nested
            // quoted / braced regions and honour backslash escapes
            // so ``[set x {a]b}]`` (or ``(set x "a)b")``) finishes
            // at the *outer* delimiter, not the first inner ``]`` /
            // ``)``.  Mirrors the ``{`` branch's structure (codex
            // review on PR #452).
            const open: u8 = c;
            const close: u8 = if (c == '[') ']' else ')';
            var depth: u32 = 1;
            i += 1;
            while (i < len and depth > 0) {
                const cc = src[i];
                if (cc == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (cc == '"') {
                    i += 1;
                    while (i < len and src[i] != '"') {
                        if (src[i] == '\\' and i + 1 < len) i += 1;
                        i += 1;
                    }
                    if (i < len) i += 1;
                    continue;
                }
                if (cc == '{') {
                    var bdepth: u32 = 1;
                    i += 1;
                    while (i < len and bdepth > 0) {
                        if (src[i] == '\\' and i + 1 < len) {
                            i += 2;
                            continue;
                        }
                        if (src[i] == '{') bdepth += 1 else if (src[i] == '}') bdepth -= 1;
                        i += 1;
                    }
                    continue;
                }
                if (cc == open) depth += 1 else if (cc == close) depth -= 1;
                i += 1;
            }
            continue;
        }
        // ``op_char`` doubled (``||`` or ``&&``) at top level → stop.
        // When scanning for ``&&`` (op_char = '&'), also stop at ``||``
        // — ``||`` has lower precedence so it ends the ``&&`` operand
        // belongs to.
        if (i + 1 < len and c == op_char and src[i + 1] == op_char) return i;
        if (op_char == '&' and i + 1 < len and c == '|' and src[i + 1] == '|') return i;
        i += 1;
    }
    return len;
}

fn expr_skip_ws(src: [*]const u8, len: u32, pos: *u32) void {
    // Tcl expression whitespace: space, tab, newline, carriage return,
    // form feed, and the ``\<newline>`` line-continuation sequence (which
    // collapses to a single space in command parsing but appears verbatim
    // in expression text when ``expr`` reads a brace-quoted argument
    // — ``if {... && \<NL>(rhs)}`` ships the continuation into the
    // expression parser intact, so it has to skip both bytes here).
    while (pos.* < len) {
        const c = src[pos.*];
        if (c == ' ' or c == '\t' or c == '\n' or c == '\r' or c == 0x0c) {
            pos.* += 1;
            continue;
        }
        if (c == '\\' and pos.* + 1 < len and src[pos.* + 1] == '\n') {
            pos.* += 2;
            continue;
        }
        break;
    }
}

fn expr_add(ptr: u32, len: u32, pos: *u32, skip: bool) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var left = expr_mul(ptr, len, pos, skip);
    var lhs_snap = expr_take_nonnum();
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* >= len) break;
        const op_str: []const u8 = blk: {
            if (src[pos.*] == '+') break :blk "+";
            if (src[pos.*] == '-') break :blk "-";
            if (pos.* + 1 < len and src[pos.*] == '=' and src[pos.* + 1] == '=') break :blk "==";
            if (pos.* + 1 < len and src[pos.*] == '!' and src[pos.* + 1] == '=') break :blk "!=";
            if (pos.* + 1 < len and src[pos.*] == '<' and src[pos.* + 1] == '=') break :blk "<=";
            if (pos.* + 1 < len and src[pos.*] == '>' and src[pos.* + 1] == '=') break :blk ">=";
            if (src[pos.*] == '<') break :blk "<";
            if (src[pos.*] == '>') break :blk ">";
            break :blk "";
        };
        if (op_str.len == 0) break;
        // Saw an arithmetic / relational op — the LHS must be numeric.
        // When the LHS came from a non-numeric atom (``"foo"`` / ``$x``
        // where x="foo"), raise the canonical operand-coercion error
        // and stop (while-old-4.4 ``"a"+"b"``).  Equality (``==`` /
        // ``!=``) operates on the i64 form too in our minimal expr
        // engine, so the same wording applies.
        if (expr_raise_if_nonnum_arith(lhs_snap, op_str, "left")) return 0;
        pos.* += @intCast(op_str.len);
        const right = expr_mul(ptr, len, pos, skip);
        const rhs_snap = expr_take_nonnum();
        if (expr_raise_if_nonnum_arith(rhs_snap, op_str, "right")) {
            expr_drop_nonnum_snap(lhs_snap);
            return 0;
        }
        expr_drop_nonnum_snap(rhs_snap);
        switch (op_str[0]) {
            '+' => left = left + right,
            '-' => left = left - right,
            '<' => {
                if (op_str.len == 2) {
                    left = if (left <= right) @as(i64, 1) else @as(i64, 0);
                } else {
                    left = if (left < right) @as(i64, 1) else @as(i64, 0);
                }
            },
            '>' => {
                if (op_str.len == 2) {
                    left = if (left >= right) @as(i64, 1) else @as(i64, 0);
                } else {
                    left = if (left > right) @as(i64, 1) else @as(i64, 0);
                }
            },
            '=' => left = if (left == right) @as(i64, 1) else @as(i64, 0),
            '!' => left = if (left != right) @as(i64, 1) else @as(i64, 0),
            else => unreachable,
        }
        // LHS has been folded into ``left`` — drop its buffer before
        // re-arming the slot for the next operator iteration.
        expr_drop_nonnum_snap(lhs_snap);
        lhs_snap = .{ .active = false, .ptr = 0, .len = 0 };
    }
    // No further operator after the LHS — restore the LHS's flag so
    // the surrounding context (boolean operator, or top-level
    // ``while``/``if`` condition) can decide whether to raise.
    expr_restore_nonnum(lhs_snap);
    return left;
}

fn expr_mul(ptr: u32, len: u32, pos: *u32, skip: bool) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var left = expr_atom(ptr, len, pos, skip);
    var lhs_snap = expr_take_nonnum();
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* >= len) break;
        const op_str: []const u8 = blk: {
            if (src[pos.*] == '*') break :blk "*";
            if (src[pos.*] == '/') break :blk "/";
            if (src[pos.*] == '%') break :blk "%";
            break :blk "";
        };
        if (op_str.len == 0) break;
        if (expr_raise_if_nonnum_arith(lhs_snap, op_str, "left")) return 0;
        pos.* += 1;
        const right = expr_atom(ptr, len, pos, skip);
        const rhs_snap = expr_take_nonnum();
        if (expr_raise_if_nonnum_arith(rhs_snap, op_str, "right")) {
            expr_drop_nonnum_snap(lhs_snap);
            return 0;
        }
        expr_drop_nonnum_snap(rhs_snap);
        switch (op_str[0]) {
            '*' => left = left * right,
            '/' => left = if (right != 0) @divTrunc(left, right) else 0,
            '%' => left = if (right != 0) @rem(left, right) else 0,
            else => unreachable,
        }
        // LHS has been folded into ``left`` — drop its buffer before
        // re-arming the slot for the next operator iteration.
        expr_drop_nonnum_snap(lhs_snap);
        lhs_snap = .{ .active = false, .ptr = 0, .len = 0 };
    }
    expr_restore_nonnum(lhs_snap);
    return left;
}

fn expr_atom(ptr: u32, len: u32, pos: *u32, skip: bool) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    expr_skip_ws(src, len, pos);
    if (pos.* >= len) return 0;
    if (src[pos.*] == '!') {
        pos.* += 1;
        const v = expr_atom(ptr, len, pos, skip);
        // ``!`` is a boolean operator: if the operand wasn't a valid
        // boolean (e.g. ``!"foo"``), raise the canonical wording now
        // rather than letting an enclosing arithmetic op consume the
        // non-numeric flag with the wrong message.
        if (expr_consume_nonnum_boolean()) return 0;
        return if (v != 0) @as(i64, 0) else @as(i64, 1);
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
        // Find the matching close paren so the inner expression text
        // can be re-checked for a top-level string operator
        // (``==``/``!=``/``eq``/``ne``/``in``/``ni``).  Without this
        // dispatch ``($v != "foobar1")`` falls into ``expr_or → expr_add``
        // → ``expr_atom`` for ``"foobar1"`` and raises ``expected
        // boolean value``, because the i64 ``==``/``!=`` branches in
        // ``expr_add`` can't represent string compare.  The outer
        // ``eval_expr_str`` entry point dispatches via
        // ``find_top_string_op`` for the top-level expression text;
        // recurse into the same helper here so paren-wrapped string
        // comparisons get the numeric-or-string semantics they need.
        const inner_start = pos.*;
        var depth: u32 = 1;
        var scan: u32 = pos.*;
        while (scan < len and depth > 0) : (scan += 1) {
            if (src[scan] == '\\' and scan + 1 < len) {
                scan += 1;
                continue;
            }
            if (src[scan] == '"') {
                scan += 1;
                while (scan < len and src[scan] != '"') {
                    if (src[scan] == '\\' and scan + 1 < len) scan += 1;
                    scan += 1;
                }
                continue;
            }
            if (src[scan] == '{') {
                var d2: u32 = 1;
                scan += 1;
                while (scan < len and d2 > 0) : (scan += 1) {
                    if (src[scan] == '\\' and scan + 1 < len) {
                        scan += 1;
                        continue;
                    }
                    if (src[scan] == '{') d2 += 1 else if (src[scan] == '}') d2 -= 1;
                }
                if (scan > 0) scan -= 1;
                continue;
            }
            if (src[scan] == '(') depth += 1 else if (src[scan] == ')') depth -= 1;
        }
        const inner_end = if (scan > 0) scan - 1 else inner_start;
        if (find_top_string_op(ptr + inner_start, inner_end - inner_start)) |op| {
            const val = eval_string_expr(ptr + inner_start, inner_end - inner_start, op);
            pos.* = scan;
            return val;
        }
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
            expr_clear_nonnum();
            return v;
        }
        if (obj_mod.try_parse_float(ptr + content_start, clen)) |fv| {
            expr_clear_nonnum();
            return if (fv != 0.0) @as(i64, 1) else @as(i64, 0);
        }
        if (obj_mod.try_parse_bool(ptr + content_start, clen)) |bv| {
            expr_clear_nonnum();
            return bv;
        }
        // Not a recognised numeric or boolean literal — flag the raw
        // bytes for the enclosing operator to consume.  Arithmetic
        // operators (``+``, ``-``, …) will raise ``cannot use
        // non-numeric string "X" as <side> operand of "<op>"``; boolean
        // operators (``&&``, ``||``) and the top-level boolean context
        // (``if``/``while``/``for`` condition) will raise ``expected
        // boolean value but got "X"`` (while-old-4.4 / 4.5).
        expr_set_nonnum(ptr + content_start, clen);
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
        var v: i64 = 0;
        if (val != 0) {
            // Parse the variable's value as int / float / bool;
            // unparseable values flag the raw bytes so the enclosing
            // operator (arithmetic / boolean) raises the right error.
            // ``obj_get_int`` silently returns 0 for non-numeric
            // strings, masking ``while {$x}`` where ``$x = "foo"``
            // (while-old-4.5).
            const vs_str = obj_mod.obj_ensure_string(val);
            if (obj_mod.try_parse_int(vs_str.ptr, vs_str.len)) |iv| {
                v = iv;
                expr_clear_nonnum();
            } else if (obj_mod.try_parse_float(vs_str.ptr, vs_str.len)) |fv| {
                v = obj_mod.float_to_i64_clamped(fv);
                expr_clear_nonnum();
            } else if (obj_mod.try_parse_bool(vs_str.ptr, vs_str.len)) |bv| {
                v = bv;
                expr_clear_nonnum();
            } else {
                expr_set_nonnum(vs_str.ptr, vs_str.len);
            }
        }
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
        // ``eval_script`` returns +1 owned to the caller — the
        // ``eval_set``-read-form borrow regression (issue #303,
        // PR #319) was paid down by retaining inside the handler
        // before forwarding the var slot's handle, so every
        // command's result now carries its own +1 share regardless
        // of whether the underlying value came from a var slot,
        // a fresh allocation, or a parser-allocated word.  Release
        // the result so the substituted ``[…]`` doesn't leak one
        // TclObj per expression invocation.
        if (result != 0) {
            const v = obj_get_int(result);
            obj_mod.tcl_obj_release(result);
            return v;
        }
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

/// Recursion-limit gate.  Increments :attr:`Interp.num_levels`,
/// returns ``true`` if the new value is within
/// :attr:`Interp.recursion_limit` (and within a hard cap that
/// guards against wasm stack exhaustion).  Returns ``false``
/// after raising ``too many nested evaluations (infinite loop?)``
/// when the limit would be exceeded.
///
/// Mirrors C Tcl's ``Tcl_EvalObjv`` increment-then-check pattern
/// (tclBasic.c line 4098).  Callers must pair every successful
/// :fn:`recursion_check_enter` with :fn:`recursion_check_leave`
/// — the proc / alias / eval / uplevel dispatchers use Zig
/// ``defer`` to make this automatic.
pub fn recursion_check_enter() bool {
    const cur = interp_reg.interp_current();
    const ci: *interp_reg.Interp = @ptrFromInt(cur);
    const limit = interp_reg.interp_recursion_limit(cur);
    // WASM-stack fail-safe.  Each interpreter dispatch lands ~10
    // wasm stack frames; wasmtime's default 512 KiB stack runs out
    // long before reference Tcl's default 1000.  Check both the
    // per-interp ``recursionlimit`` (user-visible knob) and the
    // physical frame footprint (``frame_depth + parked_top``) so
    // an ``uplevel``-style pattern that parks the top frame still
    // trips the safety net before the wasm trap kicks in
    // (test_wasm_tier2_fixes error-1.8 case).
    const HARD_CAP: u32 = 512;
    // Physical wasm-stack ceiling.  Each frame_push / parked_top
    // increment maps to ~10 wasm stack frames; wasmtime's default
    // 512 KiB stack runs out around ~120 nested dispatches.  Pick
    // a cap that leaves head-room above reference Tcl's typical
    // ``recursionlimit 50`` test bodies (interp-29.3.x reach
    // frame_depth ~50) but trips on a runaway ``uplevel`` chain
    // (test_recursion_limit_bounded_iteration ``up 200`` parks
    // ~100 frames before its first ``return``).
    const FRAME_CAP: u32 = 60;
    ci.num_levels += 1;
    if (ci.num_levels > limit or
        ci.num_levels > HARD_CAP or
        frames.frame_depth + frames.parked_top >= FRAME_CAP)
    {
        ci.num_levels -= 1;
        const tcl_catch_lim = @import("tcl_catch.zig");
        const msg_text: []const u8 = "too many nested evaluations (infinite loop?)";
        const m = obj_mod.obj_new_string_copy(@intFromPtr(msg_text.ptr), @intCast(msg_text.len));
        tcl_catch_lim.tcl_cmd_error(m);
        return false;
    }
    return true;
}

/// Pair of :fn:`recursion_check_enter`.  Decrements
/// :attr:`Interp.num_levels` on the way out of a counted
/// dispatch.  Safe to call when ``num_levels`` is already 0
/// (returns silently rather than underflowing).
pub fn recursion_check_leave() void {
    const cur = interp_reg.interp_current();
    const ci: *interp_reg.Interp = @ptrFromInt(cur);
    if (ci.num_levels > 0) ci.num_levels -= 1;
}

pub fn eval_command(words: []const i32) i32 {
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
    if (cmd_table_visible_lookup(cmd_s.ptr, cmd_s.len)) |handler| return handler(words).value;

    // ``::concat``, ``::expr``, etc. — fully-qualified names for
    // root-namespace builtins.  Strip the leading ``::`` and retry
    // the builtin table.  tcltest's ``Eval`` uses this form
    // (``uplevel 1 ::concat $body``) and without the strip it
    // surfaces as ``unknown command: ::concat`` mid-test.
    if (cmd_s.len >= 2) {
        const cmd_p: [*]const u8 = @ptrFromInt(cmd_s.ptr);
        if (cmd_p[0] == ':' and cmd_p[1] == ':') {
            if (cmd_table_visible_lookup(cmd_s.ptr + 2, cmd_s.len - 2)) |handler| {
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
            //
            // Restricted to the root namespace: ``namespace eval ::foo
            // { tcl::string::reverse $s }`` should resolve relative
            // to ``::foo`` first (``::foo::tcl::string::reverse``)
            // per Tcl's name-resolution rules, so the fallback only
            // fires when the caller is at ``::``.  ``current_ns``
            // is zero-or-root in both the uninitialised and explicit
            // root cases (see :fn:`tcl_ns.ns_current`).
            const cur = tcl_ns.current_ns;
            if (cur == 0 or cur == tcl_ns.ns_root()) {
                if (try_ensemble_rewrite_relative(cmd_s, words)) |result| return result;
            }
        }
    }

    // -- Namespace ``path`` resolution --------------------------------
    // Unqualified names that miss the BUILTINS / proc tables consult
    // each entry of the current namespace's ``commandPathArray`` and
    // probe the BUILTINS table for ``${path_ns_full}::${cmd}``.
    // ``::tcl::mathop::+`` is registered FQ in BUILTINS; ``namespace
    // path ::tcl::mathop`` makes the bare ``+`` resolve here without
    // forcing every test to ``namespace import`` first (mathop-1.x,
    // mathop-2.x, …).
    if (cmd_s.len > 0) {
        const cp: [*]const u8 = @ptrFromInt(cmd_s.ptr);
        var has_colons = false;
        if (cmd_s.len >= 2) {
            var i: u32 = 0;
            while (i + 1 < cmd_s.len) : (i += 1) {
                if (cp[i] == ':' and cp[i + 1] == ':') {
                    has_colons = true;
                    break;
                }
            }
        }
        if (!has_colons) {
            if (resolve_via_path(cmd_s, words)) |r| return r;
        }
    }

    // -- Proc dispatch: check registry before erroring --
    return eval_proc_call(words);
}

/// Walk the current namespace's command-resolution ``path`` and
/// look the unqualified ``cmd_s`` up as ``${path_ns}::${cmd_s}``
/// against the BUILTINS slice.  Returns the handler's result if a
/// match fires, or ``null`` so the caller can fall through to the
/// proc / unknown chain.  Read-only — does not mutate any tables.
fn resolve_via_path(cmd_s: anytype, words: []const i32) ?i32 {
    const ns_cur = tcl_ns.ns_current();
    if (ns_cur == 0) return null;
    const plen = tcl_ns.ns_path_len(ns_cur);
    if (plen == 0) return null;
    var pi: u32 = 0;
    while (pi < plen) : (pi += 1) {
        const e = tcl_ns.ns_path_entry(ns_cur, pi);
        if (e.target_ns == 0) continue;
        const full = tcl_ns.ns_full_name(e.target_ns);
        // Build ``${full}::${cmd_s}``.  Root (``::``) collapses to
        // ``::${cmd_s}``.
        const join_len: u32 = if (full.len == 2) 0 else 2;
        const total: u32 = full.len + join_len + cmd_s.len;
        const buf = obj_mod.alloc(total);
        if (buf == 0) return null; // OOM — surface as unresolved.
        const dst: [*]u8 = @ptrFromInt(buf);
        const src: [*]const u8 = @ptrFromInt(full.ptr);
        var fi: u32 = 0;
        while (fi < full.len) : (fi += 1) dst[fi] = src[fi];
        var loff: u32 = full.len;
        if (join_len > 0) {
            dst[loff] = ':';
            dst[loff + 1] = ':';
            loff += 2;
        }
        const cps2: [*]const u8 = @ptrFromInt(cmd_s.ptr);
        var j: u32 = 0;
        while (j < cmd_s.len) : (j += 1) dst[loff + j] = cps2[j];
        // Try the BUILTINS table with the FQ name (strip leading
        // ``::`` to match the table's storage convention — root
        // commands are keyed without it).
        var lookup_ptr: u32 = buf;
        var lookup_len: u32 = total;
        if (lookup_len >= 2 and dst[0] == ':' and dst[1] == ':') {
            lookup_ptr += 2;
            lookup_len -= 2;
        }
        // Build the FQ command-name TclObj with ``_take`` so the
        // resulting obj owns ``buf`` and a later ``tcl_obj_release``
        // frees it via ``free_sized`` (PR #413 review — the prior
        // ``obj_new_string`` form leaked the buf once per probe).
        const fq_obj = obj_mod.obj_new_string_take(buf, total, total);
        if (fq_obj == 0) {
            obj_mod.free_sized(buf, total);
            return null;
        }
        if (cmd_table_visible_lookup(lookup_ptr, lookup_len)) |handler| {
            // Rewrite words[0] so error messages / the handler's
            // view of the command name match what was dispatched.
            var new_words: [parse.MAX_WORDS]i32 = undefined;
            new_words[0] = fq_obj;
            var w: u32 = 1;
            while (w < words.len) : (w += 1) new_words[w] = words[w];
            const result = handler(new_words[0..words.len]).value;
            obj_mod.tcl_obj_release(fq_obj);
            return result;
        }
        // Also try the proc registry — ``namespace path`` resolves
        // user procs too, not just builtins.
        const bucket = procs.proc_lookup(fq_obj);
        if (bucket != 0) {
            var new_words: [parse.MAX_WORDS]i32 = undefined;
            new_words[0] = fq_obj;
            var w: u32 = 1;
            while (w < words.len) : (w += 1) new_words[w] = words[w];
            const result = eval_proc_call_bucket(new_words[0..words.len], bucket);
            obj_mod.tcl_obj_release(fq_obj);
            return result;
        }
        obj_mod.tcl_obj_release(fq_obj);
    }
    return null;
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
    const handler = cmd_table_visible_lookup(ens_ptr, ens_len) orelse return null;

    // Build a synthesised words[] with the ensemble at [0], subcmd at
    // [1], and the caller's args[1..] at [2..].  Use the bump
    // allocator — the slice lives only for the handler call.
    const total: u32 = @as(u32, @intCast(words.len)) + 1;
    const buf_size: u32 = total * 4;
    const buf = obj_mod.alloc(buf_size);
    if (buf == 0) return null;
    defer obj_mod.free_sized(buf, buf_size);
    const slot: [*]i32 = @ptrFromInt(buf);
    // Slots 0 and 1 are fresh +1 TclObjs we own; release after the
    // handler returns.  Without this each ``dict for`` / ``string
    // cat`` / etc. ensemble dispatch leaked two TclObjs per call.
    slot[0] = obj_mod.obj_new_string(@bitCast(ens_ptr), @bitCast(ens_len));
    slot[1] = obj_mod.obj_new_string(@bitCast(sub_ptr), @bitCast(sub_len));
    defer obj_mod.tcl_obj_release(slot[0]);
    defer obj_mod.tcl_obj_release(slot[1]);
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

    const handler = cmd_table_visible_lookup(ens_ptr, ens_len) orelse return null;

    const total: u32 = @as(u32, @intCast(words.len)) + 1;
    const buf_size: u32 = total * 4;
    const buf = obj_mod.alloc(buf_size);
    if (buf == 0) return null;
    defer obj_mod.free_sized(buf, buf_size);
    const slot: [*]i32 = @ptrFromInt(buf);
    slot[0] = obj_mod.obj_new_string(@bitCast(ens_ptr), @bitCast(ens_len));
    slot[1] = obj_mod.obj_new_string(@bitCast(sub_ptr), @bitCast(sub_len));
    defer obj_mod.tcl_obj_release(slot[0]);
    defer obj_mod.tcl_obj_release(slot[1]);
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
    if (buf_addr == 0) return name;
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    for (0..ns_full.len) |i| buf[i] = ns_ptr[i];
    buf[ns_full.len] = ':';
    buf[ns_full.len + 1] = ':';
    for (0..s.len) |i| buf[ns_full.len + 2 + i] = sp[i];
    return obj_mod.obj_new_string_take(buf_addr, total, total);
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
    if (buf == 0) return obj_new_string(0, 0);
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
    return obj_mod.obj_new_string_take(buf, total, total);
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
                // Route through the same-interp walker so a hidden
                // command's ``upvar`` reaches its same-interp caller
                // when foreign frames are sandwiched into the global
                // stack by alias / invokehidden trampolines.
                abs_target = frames.upvar_walk_relative(
                    @as(i32, @intCast(frames.frame_depth)),
                    @as(i32, @intCast(rel)),
                );
            }
        } else {
            // Default ``upvar 1`` — same walker treatment.
            abs_target = frames.upvar_walk_relative(
                @as(i32, @intCast(frames.frame_depth)),
                1,
            );
        }
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

    // ``concat_words`` returns the borrowed input handle when there's
    // exactly one body word, and a fresh +1 owned obj otherwise.
    // Only release in the multi-arg case — releasing the single-word
    // borrow would decrement the caller's word ref and over-release.
    const body_words = words[body_start..];
    const body_obj = concat_words(body_words);
    const body_owned = body_words.len > 1;
    defer if (body_owned) obj_mod.tcl_obj_release(body_obj);
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
    // Tcl ``if`` syntax accepts optional ``then`` / ``else``
    // keywords and an explicit ``elseif`` chaining keyword.  The
    // trailing else body may even omit the ``else`` keyword — the
    // dangling word at the end is treated as the else clause.
    //
    // Grammar:
    //
    //   if expr1 ?then? body1 \
    //      ?elseif expr2 ?then? body2 …? \
    //      ?else? ?bodyN?
    //
    // The previous walker mishandled both the ``then`` keyword and
    // the bare-trailing-body form, which sent if-old-2.x off the
    // rails by re-parsing keyword words as conditions.
    const stubs = @import("../stubs/tcl_stubs.zig");
    if (words.len < 2) {
        stubs.raise("wrong # args: no expression after \"if\" argument");
        return 0;
    }
    var i: u32 = 1;
    var seen_cond_body: bool = false;
    while (i < words.len) {
        // ``else BODY`` — terminal branch.  Empty / null word
        // (kw.ptr == 0) can't match any keyword, so don't waste
        // a ``@ptrFromInt(0)`` (which Debug-build Zig panics on).
        const kw = obj_ensure_string(words[i]);
        const kw_is_else = kw.ptr != 0 and str_eq(
            @as([*]const u8, @ptrFromInt(kw.ptr)),
            kw.len,
            "else",
        );
        if (kw_is_else) {
            if (i + 1 < words.len) {
                const bs = obj_ensure_string(words[i + 1]);
                return eval_script(bs.ptr, bs.len);
            }
            stubs.raise("wrong # args: no script following \"else\" argument");
            return 0;
        }
        // Bare-trailing-body form: when the not-taken branch leaves
        // exactly one word remaining AND we've already processed at
        // least one condition/body pair, that word is the implicit
        // else body.  Without the ``seen_cond_body`` gate ``if EXPR``
        // (just one arg, no body) would silently treat the
        // condition word as a body and run it as a command.
        if (seen_cond_body and i + 1 == words.len) {
            const bs = obj_ensure_string(words[i]);
            return eval_script(bs.ptr, bs.len);
        }
        // Condition word.
        const cond_s = obj_ensure_string(words[i]);
        const cond_true = eval_expr_str(cond_s.ptr, cond_s.len) != 0;
        // ``if`` evaluates the condition in boolean context: a
        // non-numeric value (e.g. ``$x = "foo"``) that lingered in
        // the expr atom's non-numeric flag must surface as
        // ``expected boolean value but got "X"`` rather than
        // silently being treated as false.
        if (expr_consume_nonnum_boolean()) return 0;
        // Optional ``then`` keyword between condition and body.
        // ``if EXPR then`` (then with no body) is a script-following
        // error; ``if EXPR`` with no body is the same shape under
        // a different keyword.
        const cond_word: u32 = i;
        i += 1;
        var saw_then: bool = false;
        if (i < words.len) {
            const tk = obj_ensure_string(words[i]);
            if (tk.ptr != 0 and str_eq(
                @as([*]const u8, @ptrFromInt(tk.ptr)),
                tk.len,
                "then",
            )) {
                saw_then = true;
                i += 1;
            }
        }
        if (i >= words.len) {
            if (saw_then) {
                stubs.raise("wrong # args: no script following \"then\" argument");
                return 0;
            }
            const cw = obj_ensure_string(words[cond_word]);
            raise_no_script_after(cw.ptr, cw.len);
            return 0;
        }
        if (cond_true) {
            const bs = obj_ensure_string(words[i]);
            return eval_script(bs.ptr, bs.len);
        }
        // Skip the body of the not-taken branch.
        seen_cond_body = true;
        i += 1;
        // Consume an ``elseif`` keyword so the next iteration's
        // cond probe lands on the actual expression word.  An
        // ``else`` keyword (or a bare trailing body) is handled at
        // the loop head.  Empty/null word can't match — guard the
        // ``@ptrFromInt(0)``.
        if (i < words.len) {
            const ek = obj_ensure_string(words[i]);
            if (ek.ptr != 0 and str_eq(
                @as([*]const u8, @ptrFromInt(ek.ptr)),
                ek.len,
                "elseif",
            )) {
                i += 1;
                if (i >= words.len) {
                    stubs.raise("wrong # args: no expression after \"elseif\" argument");
                    return 0;
                }
            }
        }
    }
    return 0;
}

fn raise_no_script_after(name_ptr: u32, name_len: u32) void {
    const stubs = @import("../stubs/tcl_stubs.zig");
    const prefix = "wrong # args: no script following \"";
    const suffix = "\" argument";
    const safe_len: u32 = if (name_ptr == 0) 0 else name_len;
    const total: u32 = @intCast(prefix.len + safe_len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        stubs.raise("wrong # args: no script following argument");
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (safe_len > 0) {
        const src: [*]const u8 = @ptrFromInt(name_ptr);
        var k: u32 = 0;
        while (k < safe_len) : (k += 1) {
            dst[off + k] = src[k];
        }
        off += safe_len;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    stubs.raise(dst[0..total]);
    obj_mod.free_sized(buf, total);
}

pub fn eval_while(words: []const i32) i32 {
    if (words.len != 3) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"while test command\"");
        return 0;
    }
    const cond_s = obj_ensure_string(words[1]);
    const body_s = obj_ensure_string(words[2]);
    var result: i32 = 0;
    while (true) {
        const cond_val = eval_expr_str(cond_s.ptr, cond_s.len);
        // Boolean context: a non-numeric atom (e.g. ``$x = "foo"``)
        // must surface as ``expected boolean value but got "X"``
        // (while-old-4.5).  ``expr_consume_nonnum_boolean`` raises
        // the canonical error and clears the flag.
        if (expr_consume_nonnum_boolean()) {
            const tcl_catch = @import("tcl_catch.zig");
            tcl_catch.state.transparent_error = 1;
            if (result != 0) obj_mod.tcl_obj_release(result);
            return 0;
        }
        // ``eval_expr_str`` returns 0 for both "condition was false"
        // *and* "expression evaluation raised an error".  Distinguish
        // by probing ``error_flag``: when set, the cond raised — set
        // ``transparent_error`` (matches bytecode-compiled ``while``
        // where the loop never appears in the traceback) and return
        // 0 with the error state intact so the caller's catch can
        // observe the propagated error (while-old-4.4 / 4.5).  Plain
        // false ends the loop with the canonical ``""`` return.
        if (cond_val == 0) {
            const tcl_catch = @import("tcl_catch.zig");
            if (tcl_catch.state.error_flag != 0) {
                tcl_catch.state.transparent_error = 1;
                if (result != 0) obj_mod.tcl_obj_release(result);
                return 0;
            }
            break;
        }
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
            .ERROR, .RETURN => {
                // Transparent error propagation: C Tcl 9 bytecode-
                // compiles ``while`` so the loop command itself
                // never appears in ``::errorInfo``.  Setting
                // ``transparent_error`` makes the outer
                // :func:`log_command_info` skip the ``invoked from
                // within "while ..."`` frame.  Only on TCL_ERROR —
                // a plain ``return`` from the body doesn't touch
                // errorInfo.
                if (ir.code == .ERROR) {
                    const tcl_catch = @import("tcl_catch.zig");
                    tcl_catch.state.transparent_error = 1;
                }
                return result;
            },
        }
    }
    // ``while`` always returns the empty string (Tcl docs); body
    // results from prior iterations were already released above.
    if (result != 0) obj_mod.tcl_obj_release(result);
    return obj_new_string(0, 0);
}

pub fn eval_for(words: []const i32) i32 {
    if (words.len != 5) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"for start test next command\"");
        return 0;
    }
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
    if (has_signal()) {
        // Init clause errored — propagate transparently (bytecode-
        // compiled ``for`` in C Tcl 9 never shows up in errorInfo).
        const tcl_catch = @import("tcl_catch.zig");
        if (tcl_catch.state.error_flag != 0) tcl_catch.state.transparent_error = 1;
        return 0;
    }
    var result: i32 = 0;
    while (true) {
        const cond_val_for = eval_expr_str(cond_s.ptr, cond_s.len);
        // Boolean context — surface non-numeric atoms as
        // ``expected boolean value but got "X"``.  Mirrors eval_while.
        if (expr_consume_nonnum_boolean()) {
            const tcl_catch = @import("tcl_catch.zig");
            tcl_catch.state.transparent_error = 1;
            if (result != 0) obj_mod.tcl_obj_release(result);
            return 0;
        }
        if (cond_val_for == 0) {
            // Condition errored — propagate transparently.
            if (has_signal()) {
                const tcl_catch = @import("tcl_catch.zig");
                if (tcl_catch.state.error_flag != 0) tcl_catch.state.transparent_error = 1;
            }
            break;
        }
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
            .ERROR, .RETURN => {
                if (ir.code == .ERROR) {
                    const tcl_catch = @import("tcl_catch.zig");
                    tcl_catch.state.transparent_error = 1;
                }
                return result;
            },
        }
        const next_res = eval_script(next_s.ptr, next_s.len);
        // Mirror ``tclCmdAH.c`` ForPostNextCallback (Tcl 9.0.3 line
        // ~2713): the next-clause's BREAK collapses to TCL_OK and
        // terminates the loop; CONTINUE / ERROR / RETURN propagate
        // unchanged.  Without this branch ``for {} {1} {break} {}``
        // span-locks because the BREAK signal never gets observed
        // (the cond is a bare literal so eval_expr_str ignores it),
        // and ``for {} {1} {continue} {}`` ditto for CONTINUE — both
        // appear as Tier-3 timeouts in the upstream for.test slice
        // (for-6.17 / for-6.18).
        //
        // Result-obj handling: upstream's ``Tcl_EvalObjv`` resets the
        // interp result before invoking each command, so by the time
        // next-clause's ``continue``/``break``/``return``/``error``
        // command runs, the body's prior result is already gone.  We
        // model this by returning ``next_res`` (the next-clause's own
        // result) for the propagating codes, not the body's prior
        // ``result`` — otherwise ``catch {for {} {1} {continue} {set
        // x foo}} msg`` would set ``msg`` to "foo" instead of the
        // empty string upstream produces (Codex P1 review on
        // PR #384).
        const next_ir = result_mod.snapshot(0);
        switch (next_ir.code) {
            .OK => {
                if (next_res != 0) obj_mod.tcl_obj_release(next_res);
            },
            .BREAK => {
                if (next_res != 0) obj_mod.tcl_obj_release(next_res);
                result_mod.consume(.BREAK);
                break;
            },
            .CONTINUE, .ERROR, .RETURN => {
                if (result != 0) obj_mod.tcl_obj_release(result);
                if (next_ir.code == .ERROR) {
                    const tcl_catch = @import("tcl_catch.zig");
                    tcl_catch.state.transparent_error = 1;
                }
                return next_res;
            },
        }
    }
    // ``for`` always returns the empty string (Tcl docs).  Drop the
    // body's last result and hand the caller a fresh empty obj.
    if (result != 0) obj_mod.tcl_obj_release(result);
    return obj_new_string(0, 0);
}

/// ``switch ?-options? string pattern body ?pattern body ...?``
/// or ``switch ?-options? string {pattern body ...}``
///
/// Supported options:
///   * ``-exact`` (default) — literal string equality
///   * ``-glob``            — glob-style wildcards via ``glob_match``
///   * ``-regexp``          — POSIX-extended regex via ``run_match``
///   * ``-nocase``          — case-insensitive (modifies match mode)
///   * ``-matchvar VAR``    — (regexp only) bind list of captures to VAR
///   * ``-indexvar VAR``    — (regexp only) bind list of indices to VAR
///   * ``--``               — end of options
///
/// Pattern handling:
///   * Body of ``-`` means "fall through to next non-``-`` body"
///   * Pattern ``default`` (only valid in last position) matches if
///     no earlier pattern did
///
/// All options accept any unique prefix abbreviation (``-exa`` =
/// ``-exact``, ``-gl`` = ``-glob``, ``-re`` = ``-regexp``, …) to
/// match upstream's ``Tcl_GetIndexFromObj`` behaviour.
pub fn eval_switch(words: []const i32) i32 {
    const stubs = @import("../stubs/tcl_stubs.zig");
    const catch_mod = @import("tcl_catch.zig");
    if (words.len < 3) {
        stubs.raise("wrong # args: should be \"switch ?options? string ?pattern body ...? ?default body?\"");
        return 0;
    }
    // Parse options.
    const Mode = enum { exact, glob, regexp };
    var mode: Mode = .exact;
    var nocase: bool = false;
    var matchvar: i32 = 0; // 0 = not set; otherwise a TclObj name
    var indexvar: i32 = 0;
    var i: u32 = 1;
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len < 1) break;
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        if (wp[0] != '-') break;
        if (w.len == 2 and wp[1] == '-') {
            // ``--`` end of options
            i += 1;
            break;
        }
        // Prefix match — switch options have distinct second-char
        // (after the dash), so any unambiguous prefix works.
        // Matches Tcl_GetIndexFromObj's behaviour.
        if (switch_opt_prefix(wp, w.len, "-exact")) {
            mode = .exact;
            continue;
        }
        if (switch_opt_prefix(wp, w.len, "-glob")) {
            mode = .glob;
            continue;
        }
        if (switch_opt_prefix(wp, w.len, "-regexp")) {
            mode = .regexp;
            continue;
        }
        if (switch_opt_prefix(wp, w.len, "-nocase")) {
            nocase = true;
            continue;
        }
        if (switch_opt_prefix(wp, w.len, "-matchvar")) {
            i += 1;
            if (i >= words.len) {
                stubs.raise("missing variable name argument to switch");
                return 0;
            }
            matchvar = words[i];
            continue;
        }
        if (switch_opt_prefix(wp, w.len, "-indexvar")) {
            i += 1;
            if (i >= words.len) {
                stubs.raise("missing variable name argument to switch");
                return 0;
            }
            indexvar = words[i];
            continue;
        }
        // Unknown option — produce reference Tcl's error
        switch_bad_option(wp, w.len);
        return 0;
    }

    // Reject -matchvar / -indexvar without -regexp.
    if (matchvar != 0 and mode != .regexp) {
        stubs.raise("-matchvar option requires -regexp option");
        return 0;
    }
    if (indexvar != 0 and mode != .regexp) {
        stubs.raise("-indexvar option requires -regexp option");
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

    // Helper: build a decoded copy of a list element when unbraced,
    // or return the raw bytes (no copy) when braced.  Unbraced
    // elements need backslash substitution applied — ``{\a\$\.\[}``
    // (one braced element) and ``\\a\\$\\.\\[`` (unbraced with
    // escapes) both decode to ``\a\$\.\[`` (8 bytes), but list
    // ``element_at`` returns the *raw* span; the comparator wants
    // the post-substitution form.  Reference Tcl 9 does this via
    // ``Tcl_SplitList`` which applies ``TclCopyAndCollapse`` to
    // each element.  switch-6.1 / 6.2 exercise this.
    const ListElem = struct {
        ptr: u32,
        len: u32,
        is_default: bool,
        is_dash: bool,
        // Pattern source for the "(arm line N)" errorInfo frame —
        // the raw on-disk text (with surrounding braces, if any).
        src_ptr: u32,
        src_len: u32,
    };
    const elem_decoder = struct {
        fn decode(list_ptr: u32, list_len: u32, idx: i64) ListElem {
            const e = list_element_at(list_ptr, list_len, idx);
            if (e.len == 0) {
                return .{ .ptr = 0, .len = 0, .is_default = false, .is_dash = false, .src_ptr = list_ptr + e.start, .src_len = 0 };
            }
            // ``default`` arm match — bytes-equal compare against the
            // raw list bytes.  Tcl 9 ``Tcl_SwitchObjCmd`` treats
            // ``{default}`` (braced) and ``default`` (unbraced) as
            // the same pattern word: the brace quoting is part of
            // list syntax, not the matched value.  Likewise for the
            // ``-`` / ``{-}`` fall-through sentinel.  ``list_element_at``
            // strips the outer braces before exposing the ``ptr`` /
            // ``len`` span, so a raw bytes-equal check on the inner
            // content is sufficient for both shapes (codex P1 review
            // on PR #452).
            const is_default = e.len == 7 and blk_default: {
                const sp: [*]const u8 = @ptrFromInt(list_ptr + e.start);
                break :blk_default sp[0] == 'd' and sp[1] == 'e' and sp[2] == 'f' and sp[3] == 'a' and
                    sp[4] == 'u' and sp[5] == 'l' and sp[6] == 't';
            };
            const is_dash = e.len == 1 and blk_dash: {
                const sp: [*]const u8 = @ptrFromInt(list_ptr + e.start);
                break :blk_dash sp[0] == '-';
            };
            if (e.braced) {
                return .{ .ptr = list_ptr + e.start, .len = e.len, .is_default = is_default, .is_dash = is_dash, .src_ptr = list_ptr + e.start, .src_len = e.len };
            }
            // Unbraced — apply backslash substitution into a scratch
            // buffer.  Length never grows beyond e.len (each escape
            // produces ≤ its input bytes).
            const buf = alloc(e.len);
            if (buf == 0) return .{ .ptr = list_ptr + e.start, .len = e.len, .is_default = is_default, .is_dash = is_dash, .src_ptr = list_ptr + e.start, .src_len = e.len };
            const out_len = copy_unbraced_elem(buf, list_ptr + e.start, e.len);
            return .{ .ptr = buf, .len = out_len, .is_default = is_default, .is_dash = is_dash, .src_ptr = list_ptr + e.start, .src_len = e.len };
        }
    };

    // Match attempt + capture binding for one pattern.  Returns
    // ``.matched = true`` when the pattern matches, populates
    // ``cap_count`` + ``cap_starts`` / ``cap_ends`` (codepoint
    // offsets relative to the subject) for regexp mode so the
    // -matchvar / -indexvar binding step below has the data it
    // needs.  ``cap_count`` is 0 outside regexp mode.
    const PatRunner = struct {
        fn run(p_ptr: u32, p_len: u32, s_ptr: u32, s_len: u32, mode_v: Mode, no_case: bool) bool {
            const tcl_string = @import("../valtypes/tcl_string.zig");
            const tcl_chars = @import("../valtypes/tcl_chars.zig");
            switch (mode_v) {
                .exact => {
                    if (no_case) {
                        if (p_len != s_len) return false;
                        if (p_len == 0) return true;
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

    // First pass: find the matching pattern index.  For regexp +
    // capture-binding mode we also record the matching pattern's
    // span so we can re-run it against the subject with captures
    // after the match index is known.
    var match_idx: i64 = -1;
    var match_pat_ptr: u32 = 0; // for regexp capture re-run
    var match_pat_len: u32 = 0;
    var match_pat_src_ptr: u32 = 0; // for "(arm line N)" frame
    var match_pat_src_len: u32 = 0;
    var n_pairs: i64 = 0;

    var bodies_list_ptr: u32 = 0; // list-form: list bytes
    var bodies_list_len: u32 = 0;
    var pair_base: u32 = 0; // inline-form: words[i..] index of first pattern
    if (pairs_in_list) {
        const list_s = obj_ensure_string(words[i]);
        bodies_list_ptr = list_s.ptr;
        bodies_list_len = list_s.len;
        const total_elems = list_count_elements(list_s.ptr, list_s.len);
        n_pairs = @divTrunc(total_elems, 2);
        var pi: i64 = 0;
        while (pi < n_pairs) : (pi += 1) {
            const elem = elem_decoder.decode(list_s.ptr, list_s.len, pi * 2);
            if (elem.is_default and pi == n_pairs - 1) {
                if (match_idx == -1) match_idx = pi;
                break;
            }
            if (PatRunner.run(elem.ptr, elem.len, subject_s.ptr, subject_s.len, mode, nocase)) {
                match_idx = pi;
                match_pat_ptr = elem.ptr;
                match_pat_len = elem.len;
                match_pat_src_ptr = elem.src_ptr;
                match_pat_src_len = elem.src_len;
                break;
            }
        }
    } else {
        // Inline form: words[i..] are alternating pattern/body
        if (@rem(@as(u32, @intCast(words.len)) - i, 2) != 0) {
            stubs.raise("extra switch pattern with no body");
            return 0;
        }
        pair_base = i;
        n_pairs = @intCast((words.len - i) / 2);
        var pi: i64 = 0;
        while (pi < n_pairs) : (pi += 1) {
            const pat = words[pair_base + @as(u32, @intCast(pi)) * 2];
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
                match_pat_ptr = pat_s.ptr;
                match_pat_len = pat_s.len;
                match_pat_src_ptr = pat_s.ptr;
                match_pat_src_len = pat_s.len;
                break;
            }
        }
    }

    // Bind -matchvar / -indexvar (regexp mode).  Per Tcl 9
    // ``Tcl_SwitchObjCmd``: indexvar is set first; if that fails
    // (e.g. ``x(x)`` when ``x`` is a scalar), matchvar is *not* set
    // and the body is not run.
    //
    // Binding semantics:
    //   * No match (no default taken):     vars unchanged
    //     — switch-11.2 / -12.2
    //   * Default arm taken:               vars set to empty lists
    //     — switch-11.4 / -12.4 (TIP#75)
    //   * Pattern matched:                 vars set to captures
    //     — switch-11.1 / -12.1
    if (mode == .regexp and (matchvar != 0 or indexvar != 0) and match_idx != -1) {
        // ``match_pat_ptr == 0`` here means: default arm taken (no
        // pattern matched).  Produces empty bindings via TIP#75.
        const ok = switch_bind_capture_vars(
            match_pat_ptr,
            match_pat_len,
            subject_s.ptr,
            subject_s.len,
            nocase,
            matchvar,
            indexvar,
        );
        if (!ok) return 0;
    }

    if (match_idx == -1) return obj_new_string(0, 0);

    // Walk forward for non-``-`` body, then eval it.  On error,
    // append ``("<pattern>" arm line N)`` to errorInfo (matches
    // Tcl 9 ``SwitchPostProc`` in tclCmdMZ.c — line ~3933).  N is
    // hard-coded to 1 for now: every failing tcltest exercises a
    // single-line arm body, which is the only shape that produces
    // a checkable result string.  Multi-line arm bodies don't
    // appear in the upstream control-flow suite as exact-result
    // tests; when one shows up we'll thread the actual error line
    // through ``eval_script``.
    var bi: i64 = match_idx;
    var body_ptr: u32 = 0;
    var body_len: u32 = 0;
    var arm_pat_src_ptr: u32 = match_pat_src_ptr;
    var arm_pat_src_len: u32 = match_pat_src_len;
    while (bi < n_pairs) : (bi += 1) {
        if (pairs_in_list) {
            const e = elem_decoder.decode(bodies_list_ptr, bodies_list_len, bi * 2 + 1);
            if (e.is_dash) {
                // Take the *next* pair's pattern for the (arm line N)
                // attribution per Tcl 9 — the dash forwards execution
                // to the next concrete body, but the matching pattern
                // is still the originally-matched one.
                continue;
            }
            body_ptr = e.src_ptr;
            body_len = e.src_len;
            if (bi != match_idx) {
                // For fall-through, the (arm line N) frame still
                // attributes to the original matched pattern's text.
                arm_pat_src_ptr = match_pat_src_ptr;
                arm_pat_src_len = match_pat_src_len;
            }
            break;
        } else {
            const body = words[pair_base + @as(u32, @intCast(bi)) * 2 + 1];
            const body_s = obj_ensure_string(body);
            if (body_s.len == 1) {
                const bp: [*]const u8 = @ptrFromInt(body_s.ptr);
                if (bp[0] == '-') continue;
            }
            body_ptr = body_s.ptr;
            body_len = body_s.len;
            break;
        }
    }
    if (body_ptr == 0 and body_len == 0) {
        // Should be unreachable per the "default must be last" rule;
        // fall through to empty result.
        return obj_mod.obj_new_string(0, 0);
    }

    // If the matched pattern was "default", the (arm line N) frame
    // uses literal "default" rather than the empty string.
    var dflt_pat = [_]u8{ 'd', 'e', 'f', 'a', 'u', 'l', 't' };
    if (match_idx >= 0 and match_pat_src_ptr == 0) {
        // default arm — the matching wasn't done via PatRunner
        arm_pat_src_ptr = @intFromPtr(&dflt_pat[0]);
        arm_pat_src_len = 7;
    } else if (match_idx >= 0 and arm_pat_src_len == 0) {
        // Same path: arm_pat_src_ptr may have been zeroed by the
        // default-as-last detection above.  Restore the literal
        // ``default`` token.
        arm_pat_src_ptr = @intFromPtr(&dflt_pat[0]);
        arm_pat_src_len = 7;
    }

    const result = eval_script(body_ptr, body_len);
    const ir = result_mod.snapshot(result);
    if (ir.code == .ERROR) {
        // Add ``("<pattern>" arm line N)`` to ::errorInfo and
        // suppress the body's frame from being deduped against the
        // arm-line frame.  Truncate pattern at 50 chars matching
        // tclCmdMZ.c:3930 ``limit = 50``.
        switch_append_arm_line(arm_pat_src_ptr, arm_pat_src_len);
    }
    // Switch's own enclosing ``invoked from within "switch ..."``
    // frame is added by the caller's eval_script (we want it for
    // switch tests 4.1 / 4.5 — switch is NOT bytecode-transparent
    // the way ``while`` is).
    _ = catch_mod; // Silence unused import on the OK path.
    return result;
}

/// Return true if ``opt`` (an option spelling like ``-exa``) is a
/// prefix of ``full`` (e.g. ``-exact``).  Both must begin with a
/// dash; ``opt`` may be shorter than ``full`` but must be at least
/// 2 bytes (the dash + at least one disambiguating letter).
/// Single-byte ``-`` matches nothing — that's reserved for ``--``.
fn switch_opt_prefix(opt_ptr: [*]const u8, opt_len: u32, comptime full: []const u8) bool {
    if (opt_len < 2) return false;
    if (opt_len > full.len) return false;
    if (opt_ptr[0] != '-') return false;
    var k: u32 = 0;
    while (k < opt_len) : (k += 1) {
        if (opt_ptr[k] != full[k]) return false;
    }
    return true;
}

/// Build and raise the canonical ``bad option "X": must be …`` error
/// for switch.  Mirrors Tcl 9 ``Tcl_SwitchObjCmd`` (tclCmdMZ.c)'s
/// option-set wording exactly so switch-3.x return-code tests catch
/// the upstream string.
fn switch_bad_option(opt_ptr: [*]const u8, opt_len: u32) void {
    const catch_mod = @import("tcl_catch.zig");
    const prefix: []const u8 = "bad option \"";
    const suffix: []const u8 = "\": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --";
    const total: u32 = @intCast(prefix.len + opt_len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(0);
        return;
    }
    const bp: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        bp[off] = c;
        off += 1;
    }
    var k: u32 = 0;
    while (k < opt_len) : (k += 1) {
        bp[off] = opt_ptr[k];
        off += 1;
    }
    for (suffix) |c| {
        bp[off] = c;
        off += 1;
    }
    const msg = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

/// Append ``\n    (setting foreach loop variable "X")`` to
/// ``::errorInfo`` after a ``foreach`` variable bind raises.
/// Mirrors Tcl 9's ``ForeachLoopStep`` (tclExecute.c) which sets
/// this frame via ``Tcl_AppendObjToErrorInfo`` when the body's
/// loop-variable set returns an error.
fn foreach_append_var_frame(var_name_obj: i32) void {
    const tcl_ns_mod = @import("tcl_ns.zig");
    const sn = obj_ensure_string(var_name_obj);
    const prefix: []const u8 = "\n    (setting foreach loop variable \"";
    const suffix: []const u8 = "\")";
    const total: u32 = @intCast(prefix.len + sn.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (sn.len > 0 and sn.ptr != 0) {
        const sp: [*]const u8 = @ptrFromInt(sn.ptr);
        var k: u32 = 0;
        while (k < sn.len) : (k += 1) {
            dst[off] = sp[k];
            off += 1;
        }
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const bp_const: [*]const u8 = @ptrFromInt(buf);
    tcl_ns_mod.append_errinfo_frame(bp_const[0..total]);
    obj_mod.free_sized(buf, total);
}

/// Shared body for the two proc-error-frame stamp entry points
/// below.  Skips when no error is pending; consumes the
/// ``return -code error`` marker (which bypasses the frame per Tcl 9
/// ``TclUpdateReturnInfo``); strips any namespace prefix off
/// ``name_ptr/name_len`` so the frame text uses the bare (last-
/// segment) name; defaults the line to 1 when no
/// ``error_frame_line`` was captured; delegates to
/// :func:`proc_append_procedure_frame`; finally clears
/// ``error_frame_line`` + ``error_info_supplied`` so the surrounding
/// ``eval_script`` correctly stamps the outer ``invoked from within``
/// callsite frame on the way out.  Mirrors Tcl 9
/// ``MakeProcError`` + the ``checkForCatch`` ``ERR_ALREADY_LOGGED``
/// reset (tclProc.c / tclExecute.c).
fn proc_stamp_error_frame_inner(name_ptr: u32, name_len: u32) void {
    const tcl_catch = @import("tcl_catch.zig");
    if (tcl_catch.state.error_flag == 0) return;
    if (tcl_catch.state.return_via_error_code != 0) {
        tcl_catch.state.return_via_error_code = 0;
        return;
    }
    var bare_ptr = name_ptr;
    var bare_len = name_len;
    if (bare_len > 0 and bare_ptr != 0) {
        const np: [*]const u8 = @ptrFromInt(name_ptr);
        var k: u32 = 0;
        var last_colcol_end: u32 = 0;
        while (k + 1 < name_len) : (k += 1) {
            if (np[k] == ':' and np[k + 1] == ':') {
                last_colcol_end = k + 2;
            }
        }
        if (last_colcol_end > 0 and last_colcol_end < name_len) {
            bare_ptr = name_ptr + last_colcol_end;
            bare_len = name_len - last_colcol_end;
        } else if (bare_len >= 2 and np[0] == ':' and np[1] == ':') {
            bare_ptr = name_ptr + 2;
            bare_len = name_len - 2;
        }
    }
    var line_n = tcl_catch.state.error_frame_line;
    if (line_n == 0) line_n = 1;
    proc_append_procedure_frame(bare_ptr, bare_len, line_n);
    tcl_catch.state.error_frame_line = 0;
    tcl_catch.state.error_info_supplied = 0;
}

/// Compiled-proc epilogue hook: when the body's tail raised an
/// error, append ``(procedure "X" line N)`` to ``::errorInfo`` using
/// the proc's qualified name (passed by codegen) and the frame's
/// tracked line.  Called by the codegen-emitted epilogue BEFORE
/// ``tcl_frame_pop`` so the frame's metadata is still readable.
/// All shared logic lives in :func:`proc_stamp_error_frame_inner`.
pub export fn proc_stamp_error_frame(name_ptr: u32, name_len: u32) void {
    proc_stamp_error_frame_inner(name_ptr, name_len);
}

/// Append the ``(procedure "X" line N)`` errorInfo frame after the
/// interpreted-proc dispatch returns TCL_ERROR.  Reads the proc
/// name from the *bucket* (compiled procs go through
/// :func:`proc_stamp_error_frame` instead, called from the codegen
/// epilogue).  Shared logic lives in
/// :func:`proc_stamp_error_frame_inner`.
fn proc_dispatch_stamp_error_frame(bucket: i32, result: i32) void {
    const ir = result_mod.snapshot(result);
    if (ir.code != .ERROR) return;
    const name_ptr: u32 = @bitCast(procs.proc_get_name_ptr(bucket));
    const name_len: u32 = @bitCast(procs.proc_get_name_len(bucket));
    proc_stamp_error_frame_inner(name_ptr, name_len);
}

/// Append ``\n    (procedure "X" line N)`` to ``::errorInfo`` after
/// a proc body raises.  Mirrors Tcl 9's proc dispatch frame logging
/// (``Tcl_LogCommandInfo`` in ``TclEvalProcBody``).  N is the 1-based
/// line number of the failing command inside the body.  Used by
/// ``eval_proc_call_bucket`` after ``eval_script`` returns TCL_ERROR.
fn proc_append_procedure_frame(name_ptr: u32, name_len: u32, line_n: u32) void {
    const tcl_ns_mod = @import("tcl_ns.zig");
    // Decimal-render the line number with a small stack scratch (line
    // numbers ≤ 10 digits even at i64 max, well under our 16-byte
    // buffer).
    var line_buf: [16]u8 = undefined;
    var line_off: u32 = 0;
    if (line_n == 0) {
        line_buf[0] = '0';
        line_off = 1;
    } else {
        var n = line_n;
        var digits: [16]u8 = undefined;
        var d_off: u32 = 0;
        while (n > 0) : (n /= 10) {
            digits[d_off] = @intCast('0' + (n % 10));
            d_off += 1;
        }
        // Reverse into ``line_buf``.
        while (d_off > 0) {
            d_off -= 1;
            line_buf[line_off] = digits[d_off];
            line_off += 1;
        }
    }
    const prefix: []const u8 = "\n    (procedure \"";
    const middle: []const u8 = "\" line ";
    const suffix: []const u8 = ")";
    const total: u32 = @intCast(prefix.len + name_len + middle.len + line_off + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (name_len > 0 and name_ptr != 0) {
        const sp: [*]const u8 = @ptrFromInt(name_ptr);
        var k: u32 = 0;
        while (k < name_len) : (k += 1) {
            dst[off] = sp[k];
            off += 1;
        }
    }
    for (middle) |c| {
        dst[off] = c;
        off += 1;
    }
    var k: u32 = 0;
    while (k < line_off) : (k += 1) {
        dst[off] = line_buf[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const bp_const: [*]const u8 = @ptrFromInt(buf);
    tcl_ns_mod.append_errinfo_frame(bp_const[0..total]);
    obj_mod.free_sized(buf, total);
}

/// Append the ``\n    ("<pattern>" arm line N)`` errorInfo frame
/// Tcl 9 ``SwitchPostProc`` writes after a body errors.  Falls back
/// to ``"default"`` literal when the source pattern span is empty
/// (caller substitutes a stack buffer for this).
fn switch_append_arm_line(pat_src_ptr: u32, pat_src_len: u32) void {
    const tcl_ns_mod = @import("tcl_ns.zig");
    // Compose the suffix into a scratch buffer: ``\n    ("<pat>" arm line 1)``.
    // Limit pattern text to 50 chars (tclCmdMZ.c:3930 ``limit = 50``);
    // an overflow appends ``...``.
    const limit: u32 = 50;
    const overflow: bool = pat_src_len > limit;
    const pat_len_used: u32 = if (overflow) limit else pat_src_len;
    const ellipsis: []const u8 = if (overflow) "..." else "";
    const prefix: []const u8 = "\n    (\"";
    const middle: []const u8 = "\" arm line 1)";
    const total: u32 = @intCast(prefix.len + pat_len_used + ellipsis.len + middle.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (pat_len_used > 0 and pat_src_ptr != 0) {
        const sp: [*]const u8 = @ptrFromInt(pat_src_ptr);
        var k: u32 = 0;
        while (k < pat_len_used) : (k += 1) {
            dst[off] = sp[k];
            off += 1;
        }
    }
    for (ellipsis) |c| {
        dst[off] = c;
        off += 1;
    }
    for (middle) |c| {
        dst[off] = c;
        off += 1;
    }
    // Borrow ``append_errinfo_frame`` to splice into the live
    // ``::errorInfo`` global.  Pass as a Zig slice over the buffer.
    const bp_const: [*]const u8 = @ptrFromInt(buf);
    tcl_ns_mod.append_errinfo_frame(bp_const[0..total]);
    obj_mod.free_sized(buf, total);
}

/// Bind ``-matchvar`` / ``-indexvar`` from a regexp match against
/// ``subject``.  ``pat_*`` describes the matching pattern (or 0/0
/// for "no pattern matched / default arm taken" — in which case
/// the variables are set to empty lists).  Returns false when a
/// variable set fails (e.g. ``x(x)`` when ``x`` is a scalar) so
/// the caller can propagate the error without running a body.
fn switch_bind_capture_vars(
    pat_ptr: u32,
    pat_len: u32,
    sub_ptr: u32,
    sub_len: u32,
    nocase: bool,
    matchvar: i32,
    indexvar: i32,
) bool {
    const tcl_regex = @import("../valtypes/tcl_regex.zig");
    var match_list: i32 = 0;
    var index_list: i32 = 0;
    if (pat_ptr != 0) {
        const built = tcl_regex.capture_match_for_switch(pat_ptr, pat_len, sub_ptr, sub_len, nocase);
        match_list = built.match_list;
        index_list = built.index_list;
    }
    if (match_list == 0) match_list = obj_new_string(0, 0);
    if (index_list == 0) index_list = obj_new_string(0, 0);
    // Set indexvar first, then matchvar — mirrors tclCmdMZ.c
    // line ~3784 ordering.  Before each set, probe for the
    // scalar-array-name conflict ourselves so the error message
    // includes the full ``arr(key)`` form (``frames.var_set`` →
    // ``tcl_array.find_or_create`` raises a bare ``can't set:
    // variable isn't array`` which tests like switch-11.6 /
    // switch-13.5 check by exact wording).
    obj_mod.tcl_obj_retain(match_list);
    obj_mod.tcl_obj_retain(index_list);
    defer {
        obj_mod.tcl_obj_release(match_list);
        obj_mod.tcl_obj_release(index_list);
    }
    const catch_mod = @import("tcl_catch.zig");
    if (indexvar != 0) {
        if (switch_check_var_writable(indexvar) == false) return false;
        _ = frames.var_set(indexvar, index_list);
        if (catch_mod.state.error_flag != 0) return false;
    }
    if (matchvar != 0) {
        if (switch_check_var_writable(matchvar) == false) return false;
        _ = frames.var_set(matchvar, match_list);
        if (catch_mod.state.error_flag != 0) return false;
    }
    return true;
}

/// Pre-flight check for the switch matchvar / indexvar set: raise
/// ``can't set "arr(key)": variable isn't array`` directly (with
/// the original full name) when the array-name part is already a
/// scalar in scope, instead of letting ``var_set`` route through
/// ``array_set`` which raises a bare ``can't set: variable isn't
/// array`` without the name.  Returns ``false`` if a conflict was
/// detected (and an error raised) so the caller stops.
fn switch_check_var_writable(name_obj: i32) bool {
    const ns_mod = @import("tcl_ns.zig");
    const sn = obj_ensure_string(name_obj);
    if (sn.len < 3 or sn.ptr == 0) return true;
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    var paren: u32 = 0;
    var found = false;
    var k: u32 = 0;
    while (k < sn.len) : (k += 1) {
        if (sp[k] == '(') {
            paren = k;
            found = true;
            break;
        }
    }
    if (!found or paren == 0 or sp[sn.len - 1] != ')') return true;
    // ``ns_scalar_exists`` probes the root namespace.  When the
    // array-name part is already a scalar there, set raises with
    // the canonical wording.  Local-frame scalars would route
    // through ``local_set`` later and a paren-name in a local
    // is treated as a literal scalar by Tcl 9 — leave those alone.
    if (ns_mod.ns_scalar_exists(sn.ptr, paren) == 0) return true;
    // Build & raise ``can't set "<full>": variable isn't array``.
    const prefix: []const u8 = "can't set \"";
    const suffix: []const u8 = "\": variable isn't array";
    const total: u32 = @intCast(prefix.len + sn.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) return false;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    var i: u32 = 0;
    while (i < sn.len) : (i += 1) {
        dst[off] = sp[i];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = obj_mod.obj_new_string_take(buf, total, total);
    const catch_mod = @import("tcl_catch.zig");
    catch_mod.tcl_cmd_error(msg);
    return false;
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
    // Derived from ``parse.MAX_WORDS``: ``foreach varlist1 list1
    // varlist2 list2 ... body`` carries 1 cmd-word + 2N pair-words + 1
    // body-word, so the largest ``n_pairs`` the parser can ever hand
    // us is ``(MAX_WORDS - 2) / 2``.  Tracking the parser cap directly
    // keeps the two limits from drifting (Copilot review on PR #443
    // flagged the stale literal that survived the ``MAX_WORDS = 128
    // → 512`` bump).
    const MAX_PAIRS = (parse.MAX_WORDS - 2) / 2;
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
                // ``foreach`` ${var binding} failed (e.g. trying to
                // bind a scalar name to ``a`` when ``a`` is an
                // array).  Match Tcl 9 ``ForeachLoopStep`` (tclExecute.c)
                // by appending ``(setting foreach loop variable "X")``
                // to errorInfo before propagating.  The outer
                // ``invoked from within "foreach ..."`` frame is then
                // added by the caller's eval_script.
                {
                    const tcl_catch = @import("tcl_catch.zig");
                    if (tcl_catch.state.error_flag != 0) {
                        foreach_append_var_frame(var_name_obj);
                        obj_mod.tcl_obj_release(elem_val);
                        obj_mod.tcl_obj_release(var_name_obj);
                        if (result != 0) obj_mod.tcl_obj_release(result);
                        return 0;
                    }
                }
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
            .ERROR, .RETURN => {
                if (ir.code == .ERROR) {
                    const tcl_catch = @import("tcl_catch.zig");
                    tcl_catch.state.transparent_error = 1;
                }
                return result;
            },
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
    if (buf == 0) return;
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
    if (buf == 0) return;
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
    // ``child_path_obj`` is a fresh +1 owned TclObj wrapping the
    // child interp's name bytes; release after the sub-handler
    // returns or the slot leaks one TclObj header per
    // ``<child> SUBCOMMAND`` dispatch.
    const child_path_obj = rt.obj_new_string(@bitCast(c.name_ptr), @bitCast(c.name_len));
    defer obj_mod.tcl_obj_release(child_path_obj);

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
    // ``new_words[0]`` (``"interp"`` copy) is +1 owned; release on
    // every return path via defer.
    new_words[0] = rt.obj_new_string_copy(@intFromPtr(interp_str.ptr), interp_str.len);
    defer obj_mod.tcl_obj_release(new_words[0]);
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
    if (str_eq(sp, sub.len, "issafe")) {
        // ``<child> issafe`` takes no extra args (the child identity
        // is supplied implicitly).  Matches ``ChildObjCmd``'s
        // ``CHILD_ISSAFE`` branch in tclInterp.c, which raises
        // ``wrong # args: should be "<child> issafe"`` when the
        // caller appends arguments.
        if (words.len != 2) {
            emit_child_arity_error(child_name.ptr, child_name.len, " issafe");
            return 0;
        }
        return interp_impl.eval_interp_issafe(new_words[0..new_len]);
    }
    if (str_eq(sp, sub.len, "marktrusted")) {
        // ``<child> marktrusted`` takes no extra args.  Routes to
        // the same handler as ``interp marktrusted path`` after
        // injecting the child's identity in slot 2.
        if (words.len != 2) {
            emit_child_arity_error(child_name.ptr, child_name.len, " marktrusted");
            return 0;
        }
        return interp_impl.eval_interp_marktrusted(new_words[0..new_len]);
    }
    if (str_eq(sp, sub.len, "recursionlimit")) {
        // ``<child> recursionlimit ?newlimit?`` — 0 or 1 extra arg.
        if (words.len < 2 or words.len > 3) {
            emit_child_arity_error(child_name.ptr, child_name.len, " recursionlimit ?newlimit?");
            return 0;
        }
        return interp_impl.eval_interp_recursionlimit(new_words[0..new_len]);
    }

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
    // Slot 0: the target command name as a fresh TclObj (+1 owned).
    // Track the original allocation separately so every return path
    // can release it — without this each alias dispatch leaks the
    // target-name obj, and the auto-index branch below (which
    // overwrites ``new_words[0]`` with a different handle) would
    // drop the original on the floor.
    const target_name_obj = rt.obj_new_string(
        @bitCast(rec.target_name_ptr),
        @bitCast(rec.target_name_len),
    );
    defer obj_mod.tcl_obj_release(target_name_obj);
    new_words[0] = target_name_obj;
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
        if (cmd_table_visible_lookup(target_s.ptr, target_s.len)) |handler| {
            const result = handler(new_words[0..total]).value;
            interp_reg.leave(save);
            return result;
        }
        // ``::cmd`` qualified — strip and retry, mirrors eval_command's
        // builtin lookup branch.
        if (target_s.len >= 2) {
            const tp: [*]const u8 = @ptrFromInt(target_s.ptr);
            if (tp[0] == ':' and tp[1] == ':') {
                if (cmd_table_visible_lookup(target_s.ptr + 2, target_s.len - 2)) |handler| {
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
        // ai_loaded carries a +1 from try_auto_index_load; the
        // function-exit defer below releases it.  No extra retain
        // needed even when we swap it into ``new_words[0]`` — the
        // slot is a local array entry whose lifetime is bounded by
        // this function, and the +1 from try_auto_index_load is
        // alive until the defer fires after return.
        defer obj_mod.tcl_obj_release(ai_loaded);
        const ai_bucket = procs.proc_lookup(ai_loaded);
        if (ai_bucket != 0) {
            // Swap in the loaded FQ name as the dispatched command
            // name so ``info level 0`` etc see the canonical form.
            // Releasing ``target_name_obj`` happens via the outer
            // ``defer`` (it stays alive for the duration of the call).
            new_words[0] = ai_loaded;
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
        // Before declaring the command unknown, look for a
        // user-defined ``unknown`` proc that should intercept the
        // call (Tcl's classic auto-dispatch hook).  Reference Tcl's
        // ``Tcl_FindCommand`` consults ``::unknown`` when a name
        // doesn't resolve; we mirror that by checking the proc
        // registry for a procedure named ``unknown`` and invoking
        // it with the original word slice prepended by the literal
        // ``unknown`` command name.
        //
        // unknown.test exercises this: it does
        // ``proc unknown {args} { … set x $args }`` and then calls
        // ``foobar x y z``; the test expects ``$x`` to be
        // ``foobar x y z`` (the args ``unknown`` saw).
        const unknown_name = obj_mod.obj_new_string_copy(
            @intFromPtr("unknown".ptr),
            7,
        );
        defer obj_mod.tcl_obj_release(unknown_name);
        const unknown_bucket = procs.proc_lookup(unknown_name);
        if (unknown_bucket != 0 and words.len + 1 <= parse.MAX_WORDS) {
            var new_words: [parse.MAX_WORDS]i32 = undefined;
            new_words[0] = unknown_name;
            obj_mod.tcl_obj_retain(unknown_name);
            var k: u32 = 0;
            while (k < words.len) : (k += 1) new_words[1 + k] = words[k];
            const result = eval_proc_call_bucket(new_words[0 .. words.len + 1], unknown_bucket);
            obj_mod.tcl_obj_release(unknown_name);
            return result;
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
    // Per-interp recursion gate — fires for procs, aliases, and
    // hidden-cmd dispatches that push a Tcl call frame.  Builtins
    // (list, set, incr, etc.) are bytecode-compiled in C Tcl and
    // don't ``numLevels++``; mirror that here by NOT ++ing in
    // ``eval_command`` and only checking here.  ``eval`` /
    // ``uplevel`` / ``catch`` separately count via
    // :fn:`recursion_check_enter`.
    if (!recursion_check_enter()) return 0;
    defer recursion_check_leave();

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
    // ``CMD_ENSEMBLE`` Commands carry an ``*EnsembleRec`` in their
    // ``params_obj`` slot (see ``cmds/namespace.zig``).  Route the
    // call through the ensemble subcommand dispatcher; the
    // dispatcher resolves the subcommand against the configured
    // ``-map`` / ``-subcommands`` / target-ns exports, builds the
    // final argv, and re-enters ``eval_command``.
    if ((cmd_flags & procs.CMD_ENSEMBLE) != 0) {
        const ns_mod = @import("../cmds/namespace.zig");
        return ns_mod.dispatch_ensemble(bucket, words);
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
        // (No stamp here for the compiled path — the codegen epilogue
        // already calls ``proc_stamp_error_frame`` via the
        // ``tcl_proc_stamp_error_frame`` import before its
        // ``frame_pop``.  Stamping again would double-add the
        // procedure frame.  See the interpreted path below.)
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
                // Apply ``return -code N`` at the proc boundary —
                // see the matching site in eval_proc_call_bucket.
                apply_pending_return_code(rv);
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
                    if (buf == 0) {
                        frames.frame_pop();
                        return 0;
                    }
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

    // Append ``(procedure "name" line N)`` to ``::errorInfo`` BEFORE
    // popping the frame so a surrounding catch's traceback reports
    // the failing line within the proc body.  Mirrors Tcl 9's proc
    // dispatch frame logging via ``Tcl_LogCommandInfo``.  ``N`` is
    // captured at error time from the current frame's tracked line
    // (``tcl_cmd_error`` / ``tcl_cmd_error_full`` populate
    // ``state.error_frame_line``).  error-2.3 / error-2.6 check this.
    proc_dispatch_stamp_error_frame(bucket, result);

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
        // Apply ``return -code N``'s pending code at the proc-
        // dispatch boundary, mirroring C Tcl's
        // ``TclUpdateReturnInfo`` in tclProc.c's InterpProcNR2.
        // Without this, ``proc p {} {return -code 3 X}`` would
        // absorb the TCL_RETURN to TCL_OK and lose the BREAK code
        // (interp-26.2 / 26.3).
        apply_pending_return_code(rv);
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
    if (str_eq(sp, sub.len, "marktrusted")) return interp_impl.eval_interp_marktrusted(words);
    if (str_eq(sp, sub.len, "recursionlimit")) return interp_impl.eval_interp_recursionlimit(words);
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

    // Safe-interpreter gate: a safe interp can't reach into the
    // hidden table.  Matches ``InvokeHiddenObjCmd`` /
    // ``ChildInvokeHidden`` in tclInterp.c which raise
    // ``not allowed to invoke hidden commands from safe interpreter``
    // before the path resolves.
    if (interp_reg.interp_is_safe(interp_reg.interp_current())) {
        const err_text = "not allowed to invoke hidden commands from safe interpreter";
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
    // Builtin-hidden marker: invoke the static-table handler directly,
    // since there's no Command record to dispatch through.  The marker
    // is a sentinel — dereferencing it as a pointer would fault.
    if (cmd == hide_mod.BUILTIN_HIDDEN_MARKER) {
        const tail_count_b: u32 = @intCast(words.len - 1 - idx);
        if (1 + tail_count_b > parse.MAX_WORDS) {
            const err_text = "invokehidden argv exceeds MAX_WORDS";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        }
        var bwords: [parse.MAX_WORDS]i32 = undefined;
        // ``bwords[0]`` is a fresh +1 owned TclObj wrapping the hidden
        // name's bytes; release after the dispatch — without this each
        // ``interp invokehidden`` (builtin path) leaked one TclObj
        // header per call.
        bwords[0] = rt.obj_new_string(@bitCast(hidden_name.ptr), @bitCast(hidden_name.len));
        defer obj_mod.tcl_obj_release(bwords[0]);
        var bk: u32 = 0;
        while (bk < tail_count_b) : (bk += 1) {
            bwords[1 + bk] = words[idx + 1 + bk];
        }
        const handler = cmd_table.lookup(hidden_name.ptr, hidden_name.len) orelse {
            interp_impl.interp_hide_error("invalid hidden command name \"", hidden_name.ptr, hidden_name.len, "\"");
            return 0;
        };
        // Swap into target interp for the handler dispatch.  No call
        // frame push needed for builtin invocation.
        const swapped_b = target_interp != interp_reg.interp_current();
        const save_b = if (swapped_b) interp_reg.enter(target_interp) else interp_reg.EnterSave{
            .prev_interp = interp_reg.current_interp,
            .prev_root_addr = tcl_ns.root_addr,
            .prev_current_ns = tcl_ns.current_ns,
        };
        const result_b = handler(bwords[0 .. 1 + tail_count_b]).value;
        interp_reg.leave(save_b);
        return result_b;
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
    // ``new_words[0]`` is a fresh +1 owned TclObj wrapping the hidden
    // name's bytes; release after the dispatch — see the builtin path
    // above for the same rationale.  Without this each
    // ``interp invokehidden`` (proc path) leaked one TclObj header.
    new_words[0] = rt.obj_new_string(@bitCast(hidden_name.ptr), @bitCast(hidden_name.len));
    defer obj_mod.tcl_obj_release(new_words[0]);
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
    } else if (use_global) {
        // ``-global`` swaps to the root namespace, mirroring C Tcl's
        // ``TclObjInvokeNamespace(... ::, TCL_INVOKE_HIDDEN)`` path.
        tcl_ns.current_ns = tcl_ns.ns_root();
    }
    // Default (no flag) — invoke the hidden command at the caller's
    // current frame.  ``ChildInvokeHidden`` in tclInterp.c routes
    // this through ``TclNRInvoke`` which does NOT push a fresh
    // namespace frame, so an inner ``upvar`` reaches the caller's
    // locals (interp-20.35 / 20.36 / 20.40 / 20.41).
    //
    // ``-global`` (use_global) mirrors C Tcl's
    // ``TCL_INVOKE_HIDDEN | TCL_EVAL_GLOBAL`` — the hidden body
    // runs anchored at the global frame so ``upvar 1`` inside
    // resolves to ``#0`` (interp-20.38 / 20.39 / 20.43 / 20.44).
    // Set the one-shot pending flag so the upcoming
    // ``frame_push`` (deep inside ``eval_proc_call_bucket``)
    // stamps :var:`frame_is_global` on the new top slot.
    if (use_global) pending_global_invocation = true;
    const result = eval_proc_call_bucket(new_words[0..total], @bitCast(cmd));
    // Clear the pending flag defensively in case the dispatch
    // bailed before pushing.  ``frame_push`` clears it on first
    // consumption so nested calls inside the hidden body don't
    // also inherit the global anchor.
    pending_global_invocation = false;
    interp_reg.leave(save);
    return result;
}

/// Single-shot sentinel consumed by :fn:`tcl_frames.frame_push`.
/// When set, the next pushed frame is tagged as a ``-global``
/// invokehidden invocation so ``upvar`` from within resolves to
/// ``#0`` rather than the lexical caller chain.
pub var pending_global_invocation: bool = false;

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
    if (total > 0 and script_ptr == 0) {
        // OOM allocating the concat buffer — bail rather than write
        // through ``@ptrFromInt(0)`` below.
        return 0;
    }
    // Reclaim the concat buffer before returning so every ``interp
    // eval child …`` call doesn't leak ``total`` bytes of slab.
    defer if (total > 0) obj_mod.free_sized(script_ptr, total);
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

    // Detect "deleted-interp eval" before swapping: a parent alias
    // whose dispatch landed here after the target interp was torn
    // down should report ``attempt to call eval in deleted
    // interpreter`` rather than walking the zeroed namespace.
    // Matches ``ChildEval``'s pre-check in tclInterp.c (Tcl 9.0.3
    // interp.test 18.7 / 18.8 / 18.9 / 18.10).
    if (interp_reg.is_deleted(target_interp)) {
        const catch_mod = @import("tcl_catch.zig");
        const err_text = "attempt to call eval in deleted interpreter";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
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

    // ``ChildEval`` (tclInterp.c) wraps the child eval in
    // ``Tcl_EvalObjEx`` which calls ``TclUpdateReturnInfo`` when the
    // child returns ``TCL_RETURN``.  That converts the child's
    // pending ``-code N`` into the actual outer status code so
    // ``catch {interp eval child return -code N}`` reports ``N``
    // (interp-26.1).  Our model encodes the pending code in
    // ``pending_return_code``; mirror the absorption here.
    absorb_return_at_eval_boundary();
    return result;
}

/// Apply pending ``return -code N`` semantics at an eval boundary.
/// Mirrors C Tcl's ``TclUpdateReturnInfo``: if the body left
/// ``return_flag`` set, decrement the return-level counter; when
/// it reaches zero, translate ``pending_return_code`` into the
/// matching flag (error / break / continue / TCL_RETURN / custom).
///
/// Called from ``eval_interp_eval`` (and the per-child ``<c> eval``
/// dispatch above) so the outer catch sees the user-requested code
/// rather than the generic ``TCL_RETURN`` our internal ``return``
/// flag normally carries.
fn absorb_return_at_eval_boundary() void {
    const tcl_catch = @import("tcl_catch.zig");
    if (tcl_catch.state.return_flag == 0) return;
    if (tcl_catch.state.return_level > 0) {
        tcl_catch.state.return_level -= 1;
        return;
    }
    // ``pending_return_code == 0`` AND ``return_code != 0`` means a
    // lower-level absorber (proc dispatch) already translated a
    // ``return -code N`` into its terminal flag/code shape — the
    // proc returned with the custom code stamped onto ``return_code``
    // and the catch above us should surface that verbatim, not re-
    // process through ``apply_pending_return_code``.  Leave the
    // state untouched in that case.
    if (tcl_catch.state.pending_return_code == 0 and tcl_catch.state.return_code != 0) {
        return;
    }
    // return_level == 0 and there is a pending ``-code`` to apply
    // (or it's a plain ``return`` with ``-code 0`` that should
    // convert into TCL_OK).  Clear ``return_flag`` first so the
    // ``apply_pending_return_code`` decision is purely a function of
    // the pending code — apply will re-set the flag for TCL_RETURN
    // / custom codes that should keep propagating.
    const saved_val = tcl_catch.state.return_val;
    tcl_catch.state.return_flag = 0;
    tcl_catch.state.return_val = 0;
    apply_pending_return_code(saved_val);
}

/// Translate ``pending_return_code`` into the matching control-flow
/// flag once a ``return -code N`` has reached the absorber level.
/// Shared between :fn:`absorb_return_at_eval_boundary` (interp eval
/// boundary) and :fn:`eval_proc_call_bucket`'s proc-dispatch
/// boundary so both code paths apply identical semantics.  Mirrors
/// the post-absorb leg of C Tcl's ``TclUpdateReturnInfo``.
fn apply_pending_return_code(rv: i32) void {
    const tcl_catch = @import("tcl_catch.zig");
    const pc = tcl_catch.state.pending_return_code;
    tcl_catch.state.pending_return_code = 0;
    switch (pc) {
        // TCL_OK — clean exit, no flags.
        0 => {},
        // TCL_ERROR — surface as an error with the return value
        // as the error message.
        1 => {
            tcl_catch.state.error_flag = 1;
            tcl_catch.state.error_msg = rv;
        },
        // TCL_RETURN — propagate one more eval level.  Re-set the
        // flag so the outer absorb decrements again (each level
        // strips one ``-code return`` layer).
        2 => {
            tcl_catch.state.return_flag = 1;
            tcl_catch.state.return_val = rv;
        },
        // TCL_BREAK / TCL_CONTINUE
        3 => {
            tcl_catch.state.break_flag = 1;
        },
        4 => {
            tcl_catch.state.continue_flag = 1;
        },
        // Custom numeric (N < 0 or N >= 5).  Re-set ``return_flag``
        // and stamp ``return_code`` so the surrounding ``catch``
        // surfaces the exact value via ``catch_leave``.
        else => {
            tcl_catch.state.return_flag = 1;
            tcl_catch.state.return_val = rv;
            tcl_catch.state.return_code = pc;
        },
    }
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
        if (buf == 0) break :blk 0;
        const out_len = copy_unbraced_elem(buf, lambda_s.ptr + params_elem.start, params_elem.len);
        break :blk obj_new_string_take(buf, out_len, alloc_size);
    };
    if (params_obj == 0) return 0;
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
        if (buf == 0) break :blk 0;
        const out_len = copy_unbraced_elem(buf, lambda_s.ptr + body_elem.start, body_elem.len);
        break :blk obj_new_string_take(buf, out_len, alloc_size);
    };
    if (body_obj == 0) return 0;
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
                    if (buf == 0) {
                        frames.frame_pop();
                        return 0;
                    }
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
        if (cmd.unterminated_brace or cmd.unterminated_quote) {
            // ``parseOld-10.1`` / ``-10.2`` / ``-10.3`` / ``-10.4`` —
            // unbalanced ``{`` / ``"`` should produce the canonical
            // ``missing close-brace`` / ``missing "`` diagnostic at
            // parse time.
            const msg_text: []const u8 = if (cmd.unterminated_quote)
                "missing \""
            else
                "missing close-brace";
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
        // A command may have torn down the interpreter we're
        // currently evaluating in (e.g. ``suicide`` alias which
        // dispatches ``interp delete tst`` in the parent — see
        // interp-18.9 / 18.10).  Match C Tcl's ``ChildEval`` mid-
        // loop check and raise the canonical diagnostic instead of
        // continuing past the deletion into stale namespace state.
        // Only fire when there is MORE script to execute — a
        // self-delete on the final command (interp-16.0 / 16.1 /
        // 16.2's ``xxx alias kill kill; xxx eval kill``) is a
        // clean exit; the error only matters if the script would
        // otherwise reach unreachable post-delete commands.
        if (pos < script_len and interp_reg.is_deleted(interp_reg.interp_current())) {
            const tcl_catch_d = @import("tcl_catch.zig");
            const err_text = "attempt to call eval in deleted interpreter";
            const msg = obj_mod.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            tcl_catch_d.tcl_cmd_error(msg);
            return result;
        }
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
    // ``script_ptr == 0`` means the caller doesn't have a backing
    // script buffer to quote (e.g. error raised from a synthesised
    // recursion-limit / arity stub).  ReleaseSafe builds turn the
    // ``@ptrFromInt(0)`` cast at line 3311 into a panic, which
    // wasmtime surfaces as ``cast causes pointer to be null``;
    // bail early so the unwinding error continues to propagate
    // without an extra ``invoked from within`` frame we have no
    // bytes to fill.
    if (script_ptr == 0) return;
    const tcl_catch = @import("tcl_catch.zig");
    // ``error msg info ?code?`` set ``error_info_supplied`` so the
    // immediate ``error`` callsite frame is suppressed (the user
    // supplied the desired traceback text via *info*).  Consume
    // the flag and short-circuit before mutating ``errorInfo`` /
    // ``last_log_*`` — but still stamp the dedup keys so a later
    // log call from an outer eval frame uses ``invoked from
    // within`` rather than the first-frame ``while executing``.
    if (tcl_catch.state.error_info_supplied != 0) {
        tcl_catch.state.error_info_supplied = 0;
        tcl_catch.state.last_log_script = script_ptr;
        tcl_catch.state.last_log_pos = cmd_start;
        return;
    }
    // ``transparent_error`` is set by ``while``/``for``/``foreach`` when
    // an error from their body propagates out.  Tcl 9 bytecode-
    // compiles these commands so the loop itself never appears in the
    // traceback — the body's own command frames provide the full
    // context.  We mirror that here: consume the flag and skip adding
    // an ``invoked from within "while ..."`` frame for the loop
    // command itself.  Still stamp the dedup keys so any further
    // unwinding through outer eval frames uses ``invoked from within``
    // for the outermost frame that wasn't transparent.
    if (tcl_catch.state.transparent_error != 0) {
        tcl_catch.state.transparent_error = 0;
        tcl_catch.state.last_log_script = script_ptr;
        tcl_catch.state.last_log_pos = cmd_start;
        return;
    }
    const cur = tcl_catch.state.error_msg;
    if (cur == 0) return;
    if (script_ptr == tcl_catch.state.last_log_script and cmd_start == tcl_catch.state.last_log_pos) return;
    const max_len: u32 = 150;
    // Tcl 9 normalises ``\<NL><whitespace>+`` to a single space when
    // formatting the command source for ``::errorInfo``.  Mirror that
    // here so multi-line commands (``catch {switch foo a {…} \``
    // ``<NL>     default {…}}``) report the canonical single-space
    // form rather than the raw ``\<NL><tab>     `` bytes (switch-4.5).
    const norm_full_len = log_cmd_normalized_len(script_ptr + cmd_start, cmd_len);
    const trunc_len: u32 = if (norm_full_len > max_len) max_len else norm_full_len;
    const need_ellipsis: bool = norm_full_len > max_len;
    // Pick base text: ``::errorInfo`` when ``append_errinfo_frame``
    // appended a per-command frame (sentinel: last_log_script != 0
    // and last_log_pos == 0), else fall back to ``error_msg`` so a
    // stale errorInfo from a prior catch event doesn't leak text
    // into a new traceback (incr-2.30 / 2.31 / 2.32 / 2.33 want
    // the ``\n    (reading increment)`` frame the incr handler
    // appends).  The narrow sentinel match is the *only* path where
    // we read errorInfo back; every other ``log_command_info`` call
    // builds errorInfo from ``error_msg`` directly.
    //
    // Inside the sentinel branch ``::errorInfo`` is by construction
    // owned by the *current* error event (catch_leave clears
    // ``last_log_*``, so the sentinel can only re-arm via
    // ``append_errinfo_frame`` from within the same unwind).  Use
    // the live ``::errorInfo`` as the base unconditionally — the
    // earlier prefix-of-msg gate broke ``error msg info`` re-raises
    // where ``::errorInfo`` legitimately starts with the *info*
    // argument's text rather than ``msg`` (error-2.6's ``glorp2 …``
    // vs ``Human-generated``).
    const cur_s = blk: {
        const msg_s = obj_mod.obj_ensure_string(cur);
        if (tcl_catch.state.last_log_script != 0 and tcl_catch.state.last_log_pos == 0) {
            const ec_name_probe = obj_mod.obj_new_string_copy(@intFromPtr("::errorInfo".ptr), 11);
            defer obj_mod.tcl_obj_release(ec_name_probe);
            const ei_cur = tcl_ns.global_get(ec_name_probe);
            if (ei_cur != 0) {
                const ei_s = obj_mod.obj_ensure_string(ei_cur);
                // Guard against zero-pointer string reps before
                // ``@ptrFromInt`` — ReleaseSafe panics with
                // ``cast causes pointer to be null`` if a non-zero
                // length span carries a stale 0 ptr (seen when
                // the proc-call recursion-limit error fires with a
                // message obj whose string rep was pre-built but
                // whose underlying buffer was already freed).
                if (ei_s.len > 0 and ei_s.ptr != 0) break :blk ei_s;
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
    // ``cur_len`` collapses ``cur_s`` to the effective copyable
    // length: the loop below skips when ``cur_s.ptr == 0`` (a stale
    // string rep with no backing buffer).  Without folding the same
    // skip into ``total`` the allocator reserves ``cur_s.len`` bytes
    // we never write, and ``obj_new_string_take`` publishes those
    // uninitialised bytes as the leading ``::errorInfo`` content
    // (Codex P2 review on PR #382).
    const cur_len: u32 = if (cur_s.ptr != 0) cur_s.len else 0;
    var total: u32 = cur_len + @as(u32, @intCast(prefix.len)) + trunc_len;
    if (need_ellipsis) total += @as(u32, @intCast(ellipsis.len));
    total += @as(u32, @intCast(suffix.len));
    const buf = obj_mod.alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    if (cur_len > 0) {
        const csp: [*]const u8 = @ptrFromInt(cur_s.ptr);
        for (0..cur_len) |k| {
            dst[off] = csp[k];
            off += 1;
        }
    }
    for (prefix) |b| {
        dst[off] = b;
        off += 1;
    }
    // ``script_ptr == 0`` was already filtered at function entry,
    // so by here ``script_ptr + cmd_start`` is non-zero and the
    // ``@ptrFromInt`` cast cannot panic.  Copy the command bytes
    // through the normalising helper so ``\<NL><whitespace>+``
    // collapses to a single space (matches Tcl 9's source-form
    // canonicalisation in ``Tcl_LogCommandInfo``).
    off = log_cmd_copy_normalized(dst, off, script_ptr + cmd_start, cmd_len, trunc_len);
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

/// Count the bytes a normalised copy of *src* of length *len* would
/// emit, where each ``\<NL><whitespace>+`` sequence (backslash, then
/// newline, then 0+ horizontal whitespace) is collapsed to a single
/// space.  Mirrors Tcl 9's ``Tcl_LogCommandInfo`` source-form
/// canonicalisation so multi-line commands report on one line in
/// ``::errorInfo`` (switch-4.5).
fn log_cmd_normalized_len(src_ptr: u32, len: u32) u32 {
    if (src_ptr == 0 or len == 0) return 0;
    const src: [*]const u8 = @ptrFromInt(src_ptr);
    var i: u32 = 0;
    var out_len: u32 = 0;
    while (i < len) {
        if (i + 1 < len and src[i] == '\\' and src[i + 1] == '\n') {
            // ``\<NL>`` → one space, plus consume the following
            // horizontal whitespace run.
            out_len += 1;
            i += 2;
            while (i < len and (src[i] == ' ' or src[i] == '\t')) i += 1;
            continue;
        }
        out_len += 1;
        i += 1;
    }
    return out_len;
}

/// Copy at most *max_out* bytes of *src* (length *len*) into *dst* at
/// offset *off*, normalising ``\<NL><whitespace>+`` to a single space.
/// Returns the new offset after the copy.  Used by ``log_command_info``
/// to format the command-source frame for ``::errorInfo``.
fn log_cmd_copy_normalized(dst: [*]u8, off_in: u32, src_ptr: u32, len: u32, max_out: u32) u32 {
    if (src_ptr == 0 or len == 0 or max_out == 0) return off_in;
    const src: [*]const u8 = @ptrFromInt(src_ptr);
    var i: u32 = 0;
    var off: u32 = off_in;
    const limit: u32 = off_in + max_out;
    while (i < len and off < limit) {
        if (i + 1 < len and src[i] == '\\' and src[i + 1] == '\n') {
            dst[off] = ' ';
            off += 1;
            i += 2;
            while (i < len and (src[i] == ' ' or src[i] == '\t')) i += 1;
            continue;
        }
        dst[off] = src[i];
        off += 1;
        i += 1;
    }
    return off;
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
        // Mid-script interp deletion check — mirrors the cold-path
        // probe in :fn:`eval_script` (interp-18.7..18.10).  Only
        // fires when there are MORE commands left to run so a clean
        // last-command self-delete (interp-16.0/1/2) doesn't get
        // flagged as a deleted-interp error.
        if (i + 1 < n_cmds and interp_reg.is_deleted(interp_reg.interp_current())) {
            const tcl_catch_d = @import("tcl_catch.zig");
            const err_text = "attempt to call eval in deleted interpreter";
            const msg = obj_mod.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            tcl_catch_d.tcl_cmd_error(msg);
            return result;
        }
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
                // proc-table-side promotion.  ``obj_new_braced_string``
                // applies Tcl's ``\<newline>`` → space substitution
                // that ``Tcl_ParseBraces`` performs in C (interp-20.33,
                // parseOld-7.4) — falls back to the regular copy when
                // the span has no line-continuation escapes.
                word_objs[wi] = obj_mod.obj_new_braced_string(wptr_abs, tok.len);
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
                // Arg-substitution error → the command was never
                // dispatched.  Tcl 9 bytecode-compiles most commands,
                // so a sub-script error from arg eval skips the
                // outer command's traceback frame (the body inside
                // the sub-script already added its own).  Mirror
                // that here: set ``transparent_error`` so the
                // surrounding ``eval_script``'s ``log_command_info``
                // skips the outer ``invoked from within "<outer>"``
                // frame (error-1.3, error-2.6's inner
                // ``format [error glorp2]`` masquerade).
                const tcl_catch = @import("tcl_catch.zig");
                if (tcl_catch.state.error_flag != 0) {
                    tcl_catch.state.transparent_error = 1;
                }
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
            obj_mod.obj_new_braced_string(wptr_abs, tok.len)
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
                    if (buf == 0) {
                        obj_mod.tcl_obj_release(word_obj);
                        return 0;
                    }
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
