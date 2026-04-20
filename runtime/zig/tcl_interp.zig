// Minimal Tcl interpreter — tokeniser, expression evaluator, and eval loop.
//
// Provides tcl_eval(script) for compiled WASM code to fall back to
// when it encounters constructs that can't be statically compiled.
// Shares all runtime functions from tcl_runtime.zig — no duplication.
//
// Design: parse one command at a time, split into words, look up the
// command in a static dispatch table, call the handler.  Expressions
// are evaluated via a simple recursive-descent parser.

const rt = @import("tcl_runtime.zig");
const procs = @import("tcl_procs.zig");
const frames = @import("tcl_frames.zig");
const info = @import("tcl_cmd_info.zig");

const obj_mod = @import("tcl_obj.zig");

// Re-export runtime functions used throughout this file
const alloc = rt.alloc;
const memcpy = rt.memcpy;
const read_i32 = obj_mod.read_i32;
const write_i32 = obj_mod.write_i32;
const obj_new_string = rt.obj_new_string;
const obj_new_int = rt.obj_new_int;
const obj_get_int = rt.obj_get_int;
const obj_new_string_copy = rt.obj_new_string_copy;
const copy_unbraced_elem = rt.copy_unbraced_elem;
const obj_ensure_string = rt.obj_ensure_string;
const list_count_elements = rt.list_count_elements;
const list_element_at = rt.list_element_at;

// Convenience: check if any signal flag is set (error, return, break, continue)
fn has_signal() bool {
    return rt.error_flag.* != 0 or rt.return_flag.* != 0 or
        rt.break_flag.* != 0 or rt.continue_flag.* != 0;
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

const parse = @import("tcl_parse.zig");
const MAX_WORDS: u32 = parse.MAX_WORDS;

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
/// individually enabled.  Called from the ``subst`` command
/// handler with ``-novariables`` / ``-nocommands`` /
/// ``-nobackslashes`` toggling the flags.
fn subst_flagged(
    wptr: u32,
    wlen: u32,
    do_vars: bool,
    do_cmds: bool,
    do_bs: bool,
) i32 {
    if (wlen == 0) return obj_new_string(0, 0);
    const src: [*]const u8 = @ptrFromInt(wptr);
    var has_dollar = false;
    var has_bracket = false;
    var has_backslash = false;
    for (0..wlen) |i| {
        if (src[i] == '$') has_dollar = true;
        if (src[i] == '[') has_bracket = true;
        if (src[i] == '\\') has_backslash = true;
    }
    if (!has_dollar and !has_bracket and (!do_bs or !has_backslash)) {
        return obj_new_string(@intCast(wptr), @intCast(wlen));
    }
    // Single-pass substitution into a pre-recorded "pieces" array.
    //
    // Earlier revisions used two passes — one to sum sizes and one
    // to write — but that required calling ``eval_script`` /
    // ``var_resolve`` twice per substitution.  That changes Tcl
    // semantics for observable side effects (``subst {[incr x]}``
    // incremented twice) and — worse — reopened the same overflow
    // class the two-pass approach was meant to close: if a first
    // pass read ``$x`` and got a short value, then ``[set x
    // longer]`` changed it, the allocated buffer would be
    // undersized for the second pass's re-read.
    //
    // The fix: resolve each ``$var`` / ``[cmd]`` exactly once,
    // record each resulting (ptr, len) span (plus literal runs and
    // backslash-escape replacements) in a scratch ``pieces``
    // array, then sum the lens and memcpy into a tight buffer.
    // Each piece occupies 8 bytes: (ptr: u32, len: u32).  The
    // upper bound on piece count is ``wlen`` — one piece per
    // source byte in the pathological case of alternating ``$a``
    // single-char vars.  We allocate ``wlen`` pieces up front from
    // the bump arena; the overshoot is cheap and there is no
    // growth path to get wrong.
    const pieces_buf = alloc(wlen * 8);
    var n_pieces: u32 = 0;
    var total_out: u32 = 0;
    var lit_start: u32 = 0;
    var lit_run: u32 = 0;
    const flush_lit = struct {
        fn go(pb: u32, np: *u32, to: *u32, start: u32, run: *u32, base: u32) void {
            if (run.* == 0) return;
            const slot = pb + np.* * 8;
            write_i32(slot, @bitCast(base + start));
            write_i32(slot + 4, @bitCast(run.*));
            np.* += 1;
            to.* += run.*;
            run.* = 0;
        }
    }.go;
    const push_piece = struct {
        fn go(pb: u32, np: *u32, to: *u32, ptr: u32, len: u32) void {
            if (len == 0) return;
            const slot = pb + np.* * 8;
            write_i32(slot, @bitCast(ptr));
            write_i32(slot + 4, @bitCast(len));
            np.* += 1;
            to.* += len;
        }
    }.go;
    var i: u32 = 0;
    while (i < wlen) {
        if (do_vars and src[i] == '$' and i + 1 < wlen) {
            flush_lit(pieces_buf, &n_pieces, &total_out, lit_start, &lit_run, wptr);
            i += 1;
            const vstart = i;
            if (src[i] == '{') {
                i += 1;
                const vs = i;
                while (i < wlen and src[i] != '}') i += 1;
                const ve = i;
                if (i < wlen) i += 1;
                const name_obj = obj_new_string(@intCast(wptr + vs), @intCast(ve - vs));
                const val = frames.var_resolve(name_obj);
                if (val != 0) {
                    const sv = obj_ensure_string(val);
                    push_piece(pieces_buf, &n_pieces, &total_out, sv.ptr, sv.len);
                }
            } else {
                while (i < wlen and ((src[i] >= 'a' and src[i] <= 'z') or
                    (src[i] >= 'A' and src[i] <= 'Z') or
                    (src[i] >= '0' and src[i] <= '9') or src[i] == '_'))
                {
                    i += 1;
                }
                const name_obj = obj_new_string(@intCast(wptr + vstart), @intCast(i - vstart));
                const val = frames.var_resolve(name_obj);
                if (val != 0) {
                    const sv = obj_ensure_string(val);
                    push_piece(pieces_buf, &n_pieces, &total_out, sv.ptr, sv.len);
                }
            }
            lit_start = i;
        } else if (do_cmds and src[i] == '[') {
            flush_lit(pieces_buf, &n_pieces, &total_out, lit_start, &lit_run, wptr);
            i += 1;
            const cs = i;
            var depth: u32 = 1;
            while (i < wlen and depth > 0) {
                if (src[i] == '[') depth += 1 else if (src[i] == ']') depth -= 1;
                if (depth > 0) i += 1 else i += 1;
            }
            const ce = i - 1;
            const result = eval_script(wptr + cs, ce - cs);
            if (result != 0) {
                const sv = obj_ensure_string(result);
                push_piece(pieces_buf, &n_pieces, &total_out, sv.ptr, sv.len);
            }
            lit_start = i;
        } else if (do_bs and src[i] == '\\' and i + 1 < wlen) {
            flush_lit(pieces_buf, &n_pieces, &total_out, lit_start, &lit_run, wptr);
            // Shared escape-decoder handles the full Tcl backslash
            // table (\\n \\t \\r \\a \\b \\f \\v, \\xNN, \\uNNNN,
            // \\UNNNNNNNN, octal \\NNN, \\<whitespace> folding).
            // Allocate 4 bytes upfront (UTF-8 max for \\uXXXX) in the
            // bump arena so the push_piece recording has a stable src.
            const esc_ptr = alloc(4);
            const r = obj_mod.consume_bs_escape(src, i + 1, wlen, @ptrFromInt(esc_ptr));
            i = r.next_si;
            push_piece(pieces_buf, &n_pieces, &total_out, esc_ptr, r.written);
            lit_start = i;
        } else {
            lit_run += 1;
            i += 1;
        }
    }
    flush_lit(pieces_buf, &n_pieces, &total_out, lit_start, &lit_run, wptr);

    const buf = alloc(total_out + 1);
    var out: u32 = 0;
    var pi: u32 = 0;
    while (pi < n_pieces) : (pi += 1) {
        const slot = pieces_buf + pi * 8;
        const p: u32 = @bitCast(read_i32(slot));
        const l: u32 = @bitCast(read_i32(slot + 4));
        if (l > 0) {
            memcpy(buf + out, p, l);
            out += l;
        }
    }
    return obj_new_string(@intCast(buf), @intCast(out));
}

// -- Expression evaluator --
// Recursive-descent: +, -, *, /, %, ==, !=, <, >, <=, >=, unary -, (), $var, [cmd]

fn eval_expr_str(ptr: u32, len: u32) i64 {
    var pos: u32 = 0;
    return expr_or(ptr, len, &pos, false);
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
        if (src[pos.*] == '+') { pos.* += 1; left = left + expr_mul(ptr, len, pos, skip); }
        else if (src[pos.*] == '-') { pos.* += 1; left = left - expr_mul(ptr, len, pos, skip); }
        else if (pos.* + 1 < len and src[pos.*] == '=' and src[pos.* + 1] == '=') { pos.* += 2; left = if (left == expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0); }
        else if (pos.* + 1 < len and src[pos.*] == '!' and src[pos.* + 1] == '=') { pos.* += 2; left = if (left != expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0); }
        else if (pos.* + 1 < len and src[pos.*] == '<' and src[pos.* + 1] == '=') { pos.* += 2; left = if (left <= expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0); }
        else if (pos.* + 1 < len and src[pos.*] == '>' and src[pos.* + 1] == '=') { pos.* += 2; left = if (left >= expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0); }
        else if (src[pos.*] == '<') { pos.* += 1; left = if (left < expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0); }
        else if (src[pos.*] == '>') { pos.* += 1; left = if (left > expr_mul(ptr, len, pos, skip)) @as(i64, 1) else @as(i64, 0); }
        else break;
    }
    return left;
}

fn expr_mul(ptr: u32, len: u32, pos: *u32, skip: bool) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var left = expr_atom(ptr, len, pos, skip);
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* >= len) break;
        if (src[pos.*] == '*') { pos.* += 1; left = left * expr_atom(ptr, len, pos, skip); }
        else if (src[pos.*] == '/') { pos.* += 1; const r = expr_atom(ptr, len, pos, skip); left = if (r != 0) @divTrunc(left, r) else 0; }
        else if (src[pos.*] == '%') { pos.* += 1; const r = expr_atom(ptr, len, pos, skip); left = if (r != 0) @rem(left, r) else 0; }
        else break;
    }
    return left;
}

fn expr_atom(ptr: u32, len: u32, pos: *u32, skip: bool) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    expr_skip_ws(src, len, pos);
    if (pos.* >= len) return 0;
    if (src[pos.*] == '!') { pos.* += 1; return if (expr_atom(ptr, len, pos, skip) != 0) @as(i64, 0) else @as(i64, 1); }
    if (src[pos.*] == '~') { pos.* += 1; return ~expr_atom(ptr, len, pos, skip); }
    if (src[pos.*] == '-') { pos.* += 1; return -expr_atom(ptr, len, pos, skip); }
    if (src[pos.*] == '(') {
        pos.* += 1;
        const val = expr_or(ptr, len, pos, skip);
        expr_skip_ws(src, len, pos);
        if (pos.* < len and src[pos.*] == ')') pos.* += 1;
        return val;
    }
    // String literal "...": advance past closing quote and parse the
    // content as an integer if possible.  In Tcl expressions ``"5"``
    // evaluates to 5 and participates in numeric context; falling back
    // to 0 silently made any ``{"$x" == "5"}`` comparison compare
    // against 0 regardless of value.  (String-vs-string comparison
    // operators go through dedicated runtime helpers, not this atom
    // path — this return value only surfaces when the literal is
    // used in a numeric slot.)
    if (src[pos.*] == '"') {
        pos.* += 1;
        const content_start = pos.*;
        while (pos.* < len and src[pos.*] != '"') {
            if (src[pos.*] == '\\' and pos.* + 1 < len) pos.* += 1;
            pos.* += 1;
        }
        const content_end = pos.*;
        if (pos.* < len) pos.* += 1; // closing quote
        if (obj_mod.try_parse_int(ptr + content_start, content_end - content_start)) |v| {
            return v;
        }
        return 0;
    }
    if (src[pos.*] == '$') {
        pos.* += 1;
        const vs = pos.*;
        while (pos.* < len and ((src[pos.*] >= 'a' and src[pos.*] <= 'z') or
            (src[pos.*] >= 'A' and src[pos.*] <= 'Z') or
            (src[pos.*] >= '0' and src[pos.*] <= '9') or src[pos.*] == '_'))
        { pos.* += 1; }
        const name = obj_new_string(@intCast(ptr + vs), @intCast(pos.* - vs));
        const val = frames.var_resolve(name);
        if (val != 0) return obj_get_int(val);
        return 0;
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
        if (result != 0) return obj_get_int(result);
        return 0;
    }
    var negative = false;
    if (src[pos.*] == '+') pos.* += 1;
    if (pos.* < len and src[pos.*] == '-') { negative = true; pos.* += 1; }
    var val: i64 = 0;
    // Hex literal: 0x...
    if (pos.* + 1 < len and src[pos.*] == '0' and (src[pos.* + 1] == 'x' or src[pos.* + 1] == 'X')) {
        pos.* += 2;
        while (pos.* < len) {
            const c = src[pos.*];
            if (c >= '0' and c <= '9') { val = val * 16 + @as(i64, c - '0'); pos.* += 1; }
            else if (c >= 'a' and c <= 'f') { val = val * 16 + @as(i64, c - 'a' + 10); pos.* += 1; }
            else if (c >= 'A' and c <= 'F') { val = val * 16 + @as(i64, c - 'A' + 10); pos.* += 1; }
            else break;
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

fn eval_command(words: []const i32) i32 {
    if (words.len == 0) return 0;
    const cmd_s = obj_ensure_string(words[0]);
    if (cmd_s.len == 0) return 0;
    const cmd: [*]const u8 = @ptrFromInt(cmd_s.ptr);

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

    if (str_eq(cmd, cmd_s.len, "set")) {
        if (words.len >= 3) { _ = frames.var_set(words[1], words[2]); return words[2]; }
        else if (words.len >= 2) { return frames.var_resolve(words[1]); }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "puts")) { if (words.len >= 2) return rt.tcl_cmd_puts(words[words.len - 1]); return 0; }
    if (str_eq(cmd, cmd_s.len, "expr")) {
        if (words.len >= 2) { const es = obj_ensure_string(words[1]); return obj_new_int(eval_expr_str(es.ptr, es.len)); }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "return")) {
        // ``return ?-code code? ?-level level? ?-errorinfo info?
        // ?-errorcode code? ?result?``.  Walk the switches and
        // capture the final positional arg (if any) as the result.
        // ``-code error`` raises an error via ``@"error"`` so
        // surrounding ``catch`` sees a 1/error code.
        var is_error = false;
        var result_obj: i32 = 0;
        var wi: u32 = 1;
        while (wi < words.len) : (wi += 1) {
            const w = obj_ensure_string(words[wi]);
            if (w.len >= 1) {
                const wp: [*]const u8 = @ptrFromInt(w.ptr);
                if (wp[0] == '-') {
                    // Recognise ``-code <code>``.  Other switches
                    // (-level, -errorinfo, -errorcode) are accepted
                    // and their value skipped.
                    if (str_eq(wp, w.len, "-code") and wi + 1 < words.len) {
                        const code = obj_ensure_string(words[wi + 1]);
                        if (code.len >= 1) {
                            const cp: [*]const u8 = @ptrFromInt(code.ptr);
                            if (str_eq(cp, code.len, "error")) {
                                is_error = true;
                            }
                        }
                        wi += 1;
                        continue;
                    }
                    if ((str_eq(wp, w.len, "-level") or
                        str_eq(wp, w.len, "-errorinfo") or
                        str_eq(wp, w.len, "-errorcode") or
                        str_eq(wp, w.len, "-options")) and wi + 1 < words.len)
                    {
                        wi += 1;
                        continue;
                    }
                }
            }
            result_obj = words[wi];
        }
        if (is_error) {
            const catch_mod = @import("tcl_catch.zig");
            catch_mod.tcl_cmd_error(result_obj);
            return 0;
        }
        rt.return_flag.* = 1;
        rt.return_val.* = result_obj;
        return result_obj;
    }
    if (str_eq(cmd, cmd_s.len, "break")) { rt.break_flag.* = 1; return 0; }
    if (str_eq(cmd, cmd_s.len, "continue")) { rt.continue_flag.* = 1; return 0; }
    if (str_eq(cmd, cmd_s.len, "incr")) {
        if (words.len >= 2) {
            const amt_obj = if (words.len >= 3) words[2] else obj_new_int(1);
            const cur = frames.var_resolve(words[1]);
            const result = rt.tcl_incr(cur, amt_obj);
            _ = frames.var_set(words[1], result);
            return result;
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "if")) return eval_if(words);
    if (str_eq(cmd, cmd_s.len, "while")) return eval_while(words);
    if (str_eq(cmd, cmd_s.len, "for")) return eval_for(words);
    if (str_eq(cmd, cmd_s.len, "foreach")) return eval_foreach(words);
    if (str_eq(cmd, cmd_s.len, "proc")) {
        if (words.len >= 4) {
            // Resolve the registration name with respect to the
            // current namespace (set by compiled procs before calling
            // tcl_eval).  Unqualified names get the namespace prefix;
            // ``::``-qualified names pass through verbatim so callers
            // that explicitly name-space their defs still work.
            const qname = qualify_name(words[1]);
            _ = procs.proc_register(qname, words[2], words[3]);
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "eval")) {
        // ``eval script ?args?``: concatenate the args with spaces
        // and evaluate the result as a script in the current scope.
        // Single-arg form passes the script through directly; the
        // multi-arg form concatenates with a single-space separator
        // matching Tcl semantics.
        if (words.len == 2) {
            const s = obj_ensure_string(words[1]);
            return eval_script(s.ptr, s.len);
        }
        if (words.len >= 3) {
            // Build concat'd script on the bump allocator.  Size it
            // conservatively (sum of arg lengths + separators).
            var total: u32 = 0;
            var k: u32 = 1;
            while (k < words.len) : (k += 1) {
                total += @as(u32, @intCast(obj_ensure_string(words[k]).len)) + 1;
            }
            if (total == 0) return 0;
            const buf = alloc(total);
            var off: u32 = 0;
            k = 1;
            while (k < words.len) : (k += 1) {
                const s = obj_ensure_string(words[k]);
                if (s.len > 0) {
                    memcpy(buf + off, s.ptr, s.len);
                    off += s.len;
                }
                if (k + 1 < words.len) {
                    const d: [*]u8 = @ptrFromInt(buf + off);
                    d[0] = ' ';
                    off += 1;
                }
            }
            return eval_script(buf, off);
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "error")) { if (words.len >= 2) rt.tcl_cmd_error(words[1]); return 0; }
    // ``regexp`` — dispatch to the Tcl regex engine wrapper.
    // Handles the switches the 2-arg compiled-path export can't
    // (``-nocase``, ``--``); capture vars and ``-all`` / ``-indices``
    // / ``-inline`` are not supported yet and are silently
    // ignored (the match result is still returned correctly —
    // the ignored vars just don't get set, which is observable
    // but doesn't silently return a wrong match result).
    if (str_eq(cmd, cmd_s.len, "regexp")) {
        const regex_mod = @import("tcl_regex.zig");
        return regex_mod.eval_regexp_cmd(words);
    }
    if (str_eq(cmd, cmd_s.len, "catch")) {
        if (words.len >= 2) {
            rt.catch_enter();
            const body_s = obj_ensure_string(words[1]);
            const body_result = eval_script(body_s.ptr, body_s.len);
            rt.catch_set_ok_result(body_result);
            const catch_val = rt.catch_result();
            const code = rt.catch_leave();
            if (words.len >= 3) {
                _ = frames.var_set(words[2], catch_val);
            }
            return code;
        }
        return obj_new_int(0);
    }
    if (str_eq(cmd, cmd_s.len, "append")) {
        if (words.len >= 2) {
            var result = frames.var_resolve(words[1]);
            var wi: u32 = 2;
            while (wi < words.len) : (wi += 1) {
                result = rt.tcl_cmd_append(result, words[wi]);
            }
            _ = frames.var_set(words[1], result);
            return result;
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "llength")) { if (words.len >= 2) return rt.tcl_cmd_list_length(words[1]); return 0; }
    if (str_eq(cmd, cmd_s.len, "lindex")) { if (words.len >= 3) return rt.tcl_cmd_list_index(words[1], words[2]); return 0; }
    if (str_eq(cmd, cmd_s.len, "lappend")) {
        if (words.len >= 2) {
            var result = frames.var_resolve(words[1]);
            var wi2: u32 = 2;
            while (wi2 < words.len) : (wi2 += 1) {
                result = rt.tcl_cmd_lappend(result, words[wi2]);
            }
            _ = frames.var_set(words[1], result);
            return result;
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "lset")) {
        // ``lset varName ?index ...? newValue``.  Shapes:
        //   2 words:  ``lset v``         — error (handled by tcltest
        //                                 with a normal "wrong # args"
        //                                 that we don't synthesise yet).
        //   3 words:  ``lset v newval``  — no indices; replace the
        //                                 whole variable.
        //   4 words:  ``lset v idx nv``  — single index.
        //   N words:  ``lset v i1 i2 … nv`` — multiple indices.  Build a
        //                                 combined indices list on the
        //                                 fly via ``tcl_list`` pairs.
        if (words.len >= 3) {
            const current = frames.var_resolve(words[1]);
            const newval = words[words.len - 1];
            const indices: i32 = if (words.len == 3)
                obj_new_string(0, 0)
            else if (words.len == 4)
                words[2]
            else blk: {
                var acc: i32 = rt.tcl_list(words[2], words[3]);
                var wi3: u32 = 4;
                while (wi3 + 1 < words.len) : (wi3 += 1) {
                    acc = rt.tcl_list(acc, words[wi3]);
                }
                break :blk acc;
            };
            const result = rt.tcl_cmd_list_set(current, indices, newval);
            _ = frames.var_set(words[1], result);
            return result;
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "linsert")) {
        // ``linsert list index value1 ?value2 …?``.  The runtime's
        // fixed-arity ``tcl_cmd_list_insert`` takes a single value, so
        // for the multi-value form we chain calls with an index-ordering
        // strategy that mirrors the compiler's ``_emit_cmd_runtime``:
        // forward iteration for ``end`` / ``end-N`` (position re-
        // resolves against the growing list) and reverse iteration for
        // numeric indices (same index means same insertion point).
        if (words.len >= 4) {
            const list_arg = words[1];
            const idx_arg = words[2];
            const idx_s = obj_ensure_string(idx_arg);
            var forward = false;
            if (idx_s.len >= 3) {
                const p: [*]const u8 = @ptrFromInt(idx_s.ptr);
                if (p[0] == 'e' and p[1] == 'n' and p[2] == 'd') forward = true;
            }
            var result: i32 = list_arg;
            if (forward) {
                var wi4: u32 = 3;
                while (wi4 < words.len) : (wi4 += 1) {
                    result = rt.tcl_cmd_list_insert(result, idx_arg, words[wi4]);
                }
            } else {
                var wi4: u32 = words.len;
                while (wi4 > 3) {
                    wi4 -= 1;
                    result = rt.tcl_cmd_list_insert(result, idx_arg, words[wi4]);
                }
            }
            return result;
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "lreplace")) {
        // ``lreplace list first last ?value1 …?``.  Multi-value shape
        // chains the base call with ``tcl_cmd_list_insert`` per
        // additional value, same index-ordering strategy as linsert.
        if (words.len >= 4) {
            const list_arg = words[1];
            const first_arg = words[2];
            const last_arg = words[3];
            if (words.len == 4) {
                // ``lreplace list first last`` — delete the range.
                return rt.tcl_cmd_list_replace(list_arg, first_arg, last_arg, 0);
            }
            const idx_s = obj_ensure_string(first_arg);
            var forward = false;
            if (idx_s.len >= 3) {
                const p: [*]const u8 = @ptrFromInt(idx_s.ptr);
                if (p[0] == 'e' and p[1] == 'n' and p[2] == 'd') forward = true;
            }
            if (forward) {
                var result = rt.tcl_cmd_list_replace(list_arg, first_arg, last_arg, words[4]);
                var wi5: u32 = 5;
                while (wi5 < words.len) : (wi5 += 1) {
                    result = rt.tcl_cmd_list_insert(result, first_arg, words[wi5]);
                }
                return result;
            } else {
                var result = rt.tcl_cmd_list_replace(list_arg, first_arg, last_arg, words[words.len - 1]);
                var wi5: u32 = words.len - 1;
                while (wi5 > 4) {
                    wi5 -= 1;
                    result = rt.tcl_cmd_list_insert(result, first_arg, words[wi5]);
                }
                return result;
            }
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "string")) return eval_string_cmd(words);
    if (str_eq(cmd, cmd_s.len, "dict")) return eval_dict_cmd(words);
    if (str_eq(cmd, cmd_s.len, "array")) return eval_array_cmd(words);
    if (str_eq(cmd, cmd_s.len, "subst")) {
        // ``subst ?-nobackslashes? ?-nocommands? ?-novariables? string``
        // — walk the switches, then subst the final arg with the
        // appropriate flags.
        var do_vars = true;
        var do_cmds = true;
        var do_bs = true;
        var wi: u32 = 1;
        while (wi < words.len) : (wi += 1) {
            const a = obj_ensure_string(words[wi]);
            const ap: [*]const u8 = @ptrFromInt(a.ptr);
            if (str_eq(ap, a.len, "-nobackslashes")) {
                do_bs = false;
            } else if (str_eq(ap, a.len, "-nocommands")) {
                do_cmds = false;
            } else if (str_eq(ap, a.len, "-novariables")) {
                do_vars = false;
            } else {
                // First non-switch arg is the string to subst.
                break;
            }
        }
        if (wi >= words.len) return obj_new_string(0, 0);
        const s = obj_ensure_string(words[wi]);
        return subst_flagged(s.ptr, s.len, do_vars, do_cmds, do_bs);
    }
    if (str_eq(cmd, cmd_s.len, "auto_load")) {
        // Without the Tcl stdlib there's nothing to auto-load;
        // return 0 ("not auto-loaded") so callers that check
        // the return value see the expected "proc not in index"
        // signal rather than a trap.  ``auto_reset`` /
        // ``auto_mkindex`` / ``auto_import`` / ``auto_execok`` /
        // ``auto_qualify`` similarly return empty strings.
        return obj_new_int(0);
    }
    if (str_eq(cmd, cmd_s.len, "auto_reset") or
        str_eq(cmd, cmd_s.len, "auto_mkindex") or
        str_eq(cmd, cmd_s.len, "auto_import") or
        str_eq(cmd, cmd_s.len, "auto_execok") or
        str_eq(cmd, cmd_s.len, "auto_qualify"))
    {
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "format")) {
        // ``format fmt ?arg1? ?arg2? ?arg3?`` — dispatch to the
        // real UTF-8 impl in tcl_format.zig so interpreter-path
        // callers (Tcl-source proc bodies walked by eval_script)
        // produce the same result as compiled dispatch.
        const fmt_mod = @import("tcl_format.zig");
        const fmt = if (words.len >= 2) words[1] else 0;
        const a1 = if (words.len >= 3) words[2] else 0;
        const a2 = if (words.len >= 4) words[3] else 0;
        const a3 = if (words.len >= 5) words[4] else 0;
        return fmt_mod.tcl_cmd_format(fmt, a1, a2, a3);
    }
    if (str_eq(cmd, cmd_s.len, "scan")) {
        // ``scan str fmt ?varName?``
        // 2-arg: return matched value; 3-arg: store in varName, return match count.
        const fmt_stubs = @import("tcl_fmt_stubs.zig");
        if (words.len >= 3) {
            const val = fmt_stubs.tcl_cmd_scan(words[1], words[2]);
            if (words.len >= 4) {
                _ = frames.var_set(words[3], val);
                return obj_new_int(1);
            }
            return val;
        }
        return obj_new_int(-1);
    }
    if (str_eq(cmd, cmd_s.len, "pwd")) {
        const fs_mod = @import("tcl_fs.zig");
        return fs_mod.tcl_cmd_pwd();
    }
    if (str_eq(cmd, cmd_s.len, "file")) {
        const fs_mod = @import("tcl_fs.zig");
        const sub = if (words.len >= 2) words[1] else 0;
        const a1 = if (words.len >= 3) words[2] else 0;
        const a2 = if (words.len >= 4) words[3] else 0;
        return fs_mod.tcl_cmd_file(sub, a1, a2);
    }
    if (str_eq(cmd, cmd_s.len, "cd")) {
        const fs_mod = @import("tcl_fs.zig");
        return fs_mod.tcl_cmd_cd(if (words.len >= 2) words[1] else 0);
    }
    if (str_eq(cmd, cmd_s.len, "trace")) {
        // ``trace add`` / ``trace remove`` pass through; other
        // subcommands trap via the real impl.
        const trace_mod = @import("tcl_trace.zig");
        const sub = if (words.len >= 2) words[1] else 0;
        const arg_obj = if (words.len >= 3) words[2] else 0;
        return trace_mod.tcl_cmd_trace_cmd(sub, arg_obj);
    }
    if (str_eq(cmd, cmd_s.len, "unset")) {
        // ``unset ?-nocomplain? ?--? var ?var ...?`` — clear each
        // variable.  We approximate by setting to the null TclObj
        // (matches what ``info exists`` checks for) and ignore the
        // ``-nocomplain`` / ``--`` switches; an unknown variable
        // isn't an error under either branch.
        var i: u32 = 1;
        while (i < words.len) : (i += 1) {
            const w = obj_ensure_string(words[i]);
            const wp: [*]const u8 = @ptrFromInt(w.ptr);
            // Skip option switches.
            if (w.len >= 1 and wp[0] == '-') continue;
            _ = frames.var_set(words[i], 0);
        }
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "variable")) {
        // ``variable name ?value? ?name value …?`` — declare a
        // namespace-scoped variable in the current namespace and
        // create a frame-local VAR_LINK-style alias to it.  Matches
        // C Tcl's ``Tcl_VariableObjCmd``: the var lives in the
        // namespace's ``var_table``, and within the active proc
        // body the local name reads / writes through the alias.
        var i: u32 = 1;
        while (i < words.len) : (i += 1) {
            const name_obj = words[i];
            const sn = obj_ensure_string(name_obj);
            // Resolve to (target_ns, simple_name) — find-or-creates
            // intermediates so ``variable ::deep::ns::v`` works
            // before ``::deep::ns`` exists.
            const r = tcl_ns.ns_resolve_qualified_creating(
                tcl_ns.ns_current(),
                sn.ptr,
                sn.len,
            );
            if (r.target_ns == 0 or r.simple_len == 0) continue;
            const var_ptr = tcl_ns.ns_var_create(r.target_ns, r.simple_ptr, r.simple_len);
            // Alias the local *simple* name to the ns var so the
            // proc body can refer to it unqualified.  C Tcl uses
            // the trailing component for this; we do the same.
            const local_name = obj_new_string(@bitCast(r.simple_ptr), @bitCast(r.simple_len));
            frames.frame_alias_ns_var(local_name, var_ptr);
            // Optional initialiser.
            if (i + 1 < words.len) {
                tcl_ns.var_set_scalar(var_ptr, @bitCast(words[i + 1]));
                i += 1;
            }
        }
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "encoding")) {
        // Route ``encoding <sub> ?arg1? ?arg2?`` through the real
        // UTF-8 implementation in tcl_encoding.zig.  When the
        // interpreter is walking a fallback script (e.g. the body
        // of tcltest::bytestring) and hits an encoding command, we
        // want the same pass-through semantics compiled code gets.
        const enc = @import("tcl_encoding.zig");
        const sub = if (words.len >= 2) words[1] else 0;
        const arg1 = if (words.len >= 3) words[2] else 0;
        const arg2 = if (words.len >= 4) words[3] else 0;
        return enc.tcl_cmd_encoding(sub, arg1, arg2);
    }
    if (str_eq(cmd, cmd_s.len, "fconfigure")) {
        // Fconfigure pass-through: packs the remaining words into
        // a space-joined TclObj and calls the real impl.  Walking
        // Tcl-registered procs that call fconfigure hit this path.
        const chan = @import("tcl_chan.zig");
        if (words.len < 2) return chan.tcl_cmd_fconfigure(0, 0);
        const fd = words[1];
        if (words.len < 3) return chan.tcl_cmd_fconfigure(fd, 0);
        // Concatenate words[2..] with a single space separator via
        // the existing ``concat`` runtime helper.
        var acc = words[2];
        var i: u32 = 3;
        while (i < words.len) : (i += 1) {
            const sp_ptr: u32 = alloc(1);
            const d: [*]u8 = @ptrFromInt(sp_ptr);
            d[0] = ' ';
            const sep = obj_new_string(@intCast(sp_ptr), 1);
            acc = rt.tcl_cmd_concat(acc, sep);
            acc = rt.tcl_cmd_concat(acc, words[i]);
        }
        return chan.tcl_cmd_fconfigure(fd, acc);
    }
    if (str_eq(cmd, cmd_s.len, "info")) {
        if (words.len >= 3) return info.info_dispatch(words[1], words[2]);
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "split")) {
        if (words.len >= 3) return rt.tcl_cmd_split(words[1], words[2]);
        if (words.len >= 2) return rt.tcl_cmd_split(words[1], obj_new_string(0, 0));
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "join")) {
        if (words.len >= 3) return rt.tcl_cmd_join(words[1], words[2]);
        if (words.len >= 2) {
            // Default separator is a space
            const sp = alloc(1);
            const d: [*]u8 = @ptrFromInt(sp);
            d[0] = ' ';
            return rt.tcl_cmd_join(words[1], obj_new_string(@intCast(sp), 1));
        }
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "list")) {
        // ``list`` — build a properly-quoted Tcl list from all arguments.
        // Uses list_elem_quote for each element so that values containing
        // braces, backslashes, or spaces are correctly represented.
        if (words.len <= 1) return obj_new_string(0, 0);
        // Allocate worst-case buffer: each element may double in size
        // (backslash-escaping) plus 2 for braces, plus separators.
        var max_total: u32 = 0;
        var ei: u32 = 1;
        while (ei < words.len) : (ei += 1) {
            const s = obj_ensure_string(words[ei]);
            max_total += s.len * 2 + 2;
            if (ei > 1) max_total += 1; // separator space
        }
        const buf = obj_mod.alloc(max_total + 4);
        var off: u32 = 0;
        ei = 1;
        while (ei < words.len) : (ei += 1) {
            if (ei > 1) {
                const d: [*]u8 = @ptrFromInt(buf + off);
                d[0] = ' ';
                off += 1;
            }
            const s = obj_ensure_string(words[ei]);
            // ei starts at 1 (words[0] is the command name); ei==1 is
            // element 0 of the output list and gets hash-quoting.
            if (ei == 1) {
                off = obj_mod.list_elem_quote(buf, off, s.ptr, s.len);
            } else {
                off = obj_mod.list_elem_quote_nth(buf, off, s.ptr, s.len);
            }
        }
        return obj_new_string(@bitCast(buf), @bitCast(off));
    }
    if (str_eq(cmd, cmd_s.len, "concat")) {
        if (words.len <= 1) return obj_new_string(0, 0);
        var acc = words[1];
        var ci: usize = 2;
        while (ci < words.len) : (ci += 1) {
            acc = rt.tcl_cmd_concat(acc, words[ci]);
        }
        return acc;
    }
    if (str_eq(cmd, cmd_s.len, "lsort")) { if (words.len >= 2) return rt.tcl_cmd_list_sort(words[words.len - 1]); return obj_new_string(0, 0); }
    if (str_eq(cmd, cmd_s.len, "lsearch")) { if (words.len >= 3) return rt.tcl_cmd_list_search(words[1], words[2]); return obj_new_int(-1); }
    if (str_eq(cmd, cmd_s.len, "lrange")) { if (words.len >= 4) return rt.tcl_cmd_list_range(words[1], words[2], words[3]); return obj_new_string(0, 0); }
    if (str_eq(cmd, cmd_s.len, "global")) {
        // Register each listed name as a global alias in the current frame.
        // Subsequent reads/writes of the local name pass through to globals,
        // so the proc sees up-to-date values and mutations propagate.
        var gi: u32 = 1;
        while (gi < words.len) : (gi += 1) {
            frames.frame_alias_global(words[gi]);
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "upvar")) return eval_upvar(words);
    if (str_eq(cmd, cmd_s.len, "uplevel")) return eval_uplevel(words);
    if (str_eq(cmd, cmd_s.len, "package") or
        str_eq(cmd, cmd_s.len, "variable"))
    { return 0; }
    if (str_eq(cmd, cmd_s.len, "rename")) return eval_rename(words);
    if (str_eq(cmd, cmd_s.len, "interp")) return eval_interp(words);
    // ``namespace`` sub-command dispatch.  P4.1 adds ``export`` and
    // makes ``eval`` actually switch ``current_ns`` for the body so
    // ``namespace eval ::ctx { namespace export foo }`` records the
    // pattern on ``::ctx`` rather than on root.
    if (str_eq(cmd, cmd_s.len, "namespace")) {
        if (words.len >= 2) {
            const sub = obj_ensure_string(words[1]);
            if (sub.len == 4 and sub.ptr != 0) {
                const sp: [*]const u8 = @ptrFromInt(sub.ptr);
                if (sp[0] == 'e' and sp[1] == 'v' and sp[2] == 'a' and sp[3] == 'l') {
                    if (words.len < 4) return 0;
                    // Resolve / create the target ns and switch
                    // ``current_ns`` for the duration of the body.
                    const ns_obj_s = obj_ensure_string(words[2]);
                    const target_ns = tcl_ns.ns_create_from_fqn(ns_obj_s.ptr, ns_obj_s.len);
                    const saved_ns = tcl_ns.current_ns;
                    tcl_ns.current_ns = target_ns;
                    defer tcl_ns.current_ns = saved_ns;
                    // Concatenate body args with single spaces
                    // (matches Tcl semantics for >1 body part).
                    if (words.len == 4) {
                        const bs = obj_ensure_string(words[3]);
                        if (bs.len > 0) return eval_script(bs.ptr, bs.len);
                        return 0;
                    }
                    var total: u32 = 0;
                    var wi3: u32 = 3;
                    while (wi3 < words.len) : (wi3 += 1) {
                        const ws = obj_ensure_string(words[wi3]);
                        total += ws.len;
                        if (wi3 + 1 < words.len) total += 1;
                    }
                    const buf = alloc(total);
                    var off: u32 = 0;
                    wi3 = 3;
                    while (wi3 < words.len) : (wi3 += 1) {
                        const ws = obj_ensure_string(words[wi3]);
                        memcpy(buf + off, ws.ptr, ws.len);
                        off += ws.len;
                        if (wi3 + 1 < words.len) {
                            const d: [*]u8 = @ptrFromInt(buf + off);
                            d[0] = ' ';
                            off += 1;
                        }
                    }
                    return eval_script(buf, total);
                }
            }
            // ``namespace export ?-clear? pat1 pat2 …`` — append
            // each pattern to the current ns's export list.
            // ``-clear`` (a documented but rarely-used flag) would
            // wipe before appending; not implemented in P4.1.
            if (sub.len == 6 and sub.ptr != 0) {
                const sp6: [*]const u8 = @ptrFromInt(sub.ptr);
                if (sp6[0] == 'e' and sp6[1] == 'x' and sp6[2] == 'p' and sp6[3] == 'o' and sp6[4] == 'r' and sp6[5] == 't') {
                    var pi: u32 = 2;
                    while (pi < words.len) : (pi += 1) {
                        const ps = obj_ensure_string(words[pi]);
                        // Skip the ``-clear`` flag rather than
                        // recording it as a pattern (we don't
                        // implement the wipe but we mustn't
                        // pollute the pattern list either).
                        if (ps.len == 6 and ps.ptr != 0) {
                            const psp: [*]const u8 = @ptrFromInt(ps.ptr);
                            if (psp[0] == '-' and psp[1] == 'c' and psp[2] == 'l' and psp[3] == 'e' and psp[4] == 'a' and psp[5] == 'r') continue;
                        }
                        tcl_ns.ns_export(tcl_ns.ns_current(), ps.ptr, ps.len);
                    }
                    return 0;
                }
            }
            // ``namespace import ?-force? ::src::pat …`` — for each
            // pattern, walk the source ns's exports and create
            // redirect commands in the current ns's cmd_table.
            // ``-force`` (overwrite shadowed imports) is recognised
            // and ignored — our redirect insert path already
            // overwrites any existing entry under the same name.
            if (sub.len == 6 and sub.ptr != 0) {
                const sp6: [*]const u8 = @ptrFromInt(sub.ptr);
                if (sp6[0] == 'i' and sp6[1] == 'm' and sp6[2] == 'p' and sp6[3] == 'o' and sp6[4] == 'r' and sp6[5] == 't') {
                    var ii: u32 = 2;
                    while (ii < words.len) : (ii += 1) {
                        const is = obj_ensure_string(words[ii]);
                        if (is.len == 6 and is.ptr != 0) {
                            const isp: [*]const u8 = @ptrFromInt(is.ptr);
                            if (isp[0] == '-' and isp[1] == 'f' and isp[2] == 'o' and isp[3] == 'r' and isp[4] == 'c' and isp[5] == 'e') continue;
                        }
                        const created = tcl_ns.ns_import(tcl_ns.ns_current(), is.ptr, is.len);
                        // Each redirect counts as a real command
                        // for the proc-first dispatch fast path —
                        // bump the procs counter so ``proc_lookup``
                        // doesn't early-return 0 when the importing
                        // module has only imports (no own procs).
                        var bk: u32 = 0;
                        while (bk < created) : (bk += 1) procs.proc_count_bump();
                    }
                    return 0;
                }
                // ``namespace forget pat …`` — deactivate matching
                // redirects in the current ns.  Patterns are
                // ``string match`` globs against the simple name in
                // the importing ns (matches Tcl's behaviour for the
                // common single-component form; ``::src::pat``
                // qualified forms are treated the same as ``pat``
                // for now since our forget walks only ``current_ns``).
                if (sp6[0] == 'f' and sp6[1] == 'o' and sp6[2] == 'r' and sp6[3] == 'g' and sp6[4] == 'e' and sp6[5] == 't') {
                    var fi: u32 = 2;
                    var any_forgotten: u32 = 0;
                    while (fi < words.len) : (fi += 1) {
                        const fs = obj_ensure_string(words[fi]);
                        any_forgotten += tcl_ns.ns_forget(tcl_ns.ns_current(), fs.ptr, fs.len);
                    }
                    // Invalidate the proc-lookup LRU — cached
                    // entries might point at sources whose redirect
                    // has just been deactivated, and the cache key
                    // doesn't track that.
                    if (any_forgotten > 0) procs.lru_invalidate_all();
                    return 0;
                }
            }
            // ``namespace path { ::ns1 ::ns2 … }`` — set the current
            // ns's command resolution path.  Argument is a Tcl list
            // (single ``words[2]``) of namespace names.  Each name is
            // resolved to a ``*Namespace`` (find-only — missing
            // namespaces are silently skipped, matching our pattern
            // for namespace-tree gaps).  P5.2 will start consulting
            // the path in ``ns_find_command``; for now this just
            // records it.
            if (sub.len == 4 and sub.ptr != 0) {
                const sp4: [*]const u8 = @ptrFromInt(sub.ptr);
                if (sp4[0] == 'p' and sp4[1] == 'a' and sp4[2] == 't' and sp4[3] == 'h') {
                    if (words.len < 3) {
                        // ``namespace path`` with no args queries
                        // current — not implemented here, return empty.
                        return 0;
                    }
                    const ls = obj_ensure_string(words[2]);
                    const count = obj_mod.list_count_elements(ls.ptr, ls.len);
                    if (count == 0) {
                        // Empty list clears the path.
                        tcl_ns.ns_set_path(tcl_ns.ns_current(), 0, 0);
                        return 0;
                    }
                    // Allocate a packed u32 array for the resolved
                    // targets.  Writing via ``write_i32`` sidesteps
                    // the ``[*]u32`` alignment cast Zig requires for
                    // pointer arithmetic on bump-allocated memory.
                    const targets_buf = alloc(@intCast(count * 4));
                    var li: i64 = 0;
                    while (li < count) : (li += 1) {
                        const elt = obj_mod.list_element_at(ls.ptr, ls.len, li);
                        const r = tcl_ns.ns_resolve_qualified(tcl_ns.ns_current(), elt.start, elt.len);
                        // ``namespace path`` should resolve to an
                        // existing leaf ns.  ``ns_resolve_qualified``
                        // for ``::tcltest`` yields target=root,
                        // simple="tcltest" (because there's no
                        // trailing component to make it a "this whole
                        // path is a ns" lookup).  Combine the two:
                        // if simple_len > 0, descend one more level.
                        var resolved: u32 = r.target_ns;
                        if (r.simple_len > 0 and r.target_ns != 0) {
                            const child = tcl_ns.ns_lookup(r.target_ns, r.simple_ptr, r.simple_len);
                            resolved = child;
                        }
                        obj_mod.write_i32(targets_buf + @as(u32, @intCast(li)) * 4, @bitCast(resolved));
                    }
                    tcl_ns.ns_set_path(tcl_ns.ns_current(), targets_buf, @intCast(count));
                    procs.lru_invalidate_all();
                    return 0;
                }
            }
        }
        return 0;
    }
    // -- Proc dispatch: check registry before erroring --
    return eval_proc_call(words);
}

const str_eq = @import("tcl_chars.zig").str_eq;

const tcl_ns = @import("tcl_ns.zig");
const alias_mod = @import("tcl_alias.zig");

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
fn qualify_name(name: i32) i32 {
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
    return obj_mod.obj_new_string(@intCast(buf_addr), @intCast(total));
}

// -- upvar / uplevel helpers --

/// Parse an unsigned integer from a byte slice.  Stops at first non-digit.
fn parse_uint_bytes(ptr: [*]const u8, len: u32) u32 {
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
fn concat_words(ws: []const i32) i32 {
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
fn eval_upvar(words: []const i32) i32 {
    if (words.len < 3) return 0;

    // Determine whether words[1] is a level specifier.
    // A leading '#' or a digit sequence marks it as a level.
    const w1 = obj_ensure_string(words[1]);
    const w1p: [*]const u8 = @ptrFromInt(w1.ptr);

    var pairs_start: u32 = 1;
    var is_global: bool = false;
    // abs_target_depth: absolute 1-indexed depth of the target frame.
    // Default: one level up from current (upvar 1).
    var abs_target: i32 = @as(i32, @intCast(frames.frame_depth)) - 1;

    if (w1.len > 0) {
        if (w1p[0] == '#') {
            // Absolute level: #0 = global, #N = abs frame N
            pairs_start = 2;
            const level = parse_uint_bytes(w1p + 1, w1.len - 1);
            if (level == 0) {
                is_global = true;
            } else {
                abs_target = @intCast(level);
            }
        } else if (w1p[0] >= '0' and w1p[0] <= '9') {
            // Relative level
            pairs_start = 2;
            const rel = parse_uint_bytes(w1p, w1.len);
            abs_target = @as(i32, @intCast(frames.frame_depth)) - @as(i32, @intCast(rel));
        }
        // else: not a level spec; default level 1 applies (pairs_start = 1)
    }

    var i = pairs_start;
    while (i + 1 < words.len) : (i += 2) {
        const other_var = words[i];     // name in the target frame
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
fn eval_uplevel(words: []const i32) i32 {
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
    const saved = frames.frame_depth_stash(shift);
    const body_s = obj_ensure_string(body_obj);
    const result = eval_script(body_s.ptr, body_s.len);
    frames.frame_depth_restore(saved);
    return result;
}

// -- Control flow --

fn eval_if(words: []const i32) i32 {
    var i: u32 = 1;
    while (i < words.len) {
        const kw = obj_ensure_string(words[i]);
        const kp: [*]const u8 = @ptrFromInt(kw.ptr);
        if (str_eq(kp, kw.len, "else")) {
            if (i + 1 < words.len) { const bs = obj_ensure_string(words[i + 1]); return eval_script(bs.ptr, bs.len); }
            return 0;
        }
        const cond_s = obj_ensure_string(words[i]);
        if (eval_expr_str(cond_s.ptr, cond_s.len) != 0) {
            if (i + 1 < words.len) { const bs = obj_ensure_string(words[i + 1]); return eval_script(bs.ptr, bs.len); }
            return 0;
        }
        i += 2;
        if (i < words.len) { const nk = obj_ensure_string(words[i]); const np: [*]const u8 = @ptrFromInt(nk.ptr); if (str_eq(np, nk.len, "elseif")) i += 1; }
    }
    return 0;
}

fn eval_while(words: []const i32) i32 {
    if (words.len < 3) return 0;
    const cond_s = obj_ensure_string(words[1]);
    const body_s = obj_ensure_string(words[2]);
    var result: i32 = 0;
    while (true) {
        if (eval_expr_str(cond_s.ptr, cond_s.len) == 0) break;
        result = eval_script(body_s.ptr, body_s.len);
        if (rt.break_flag.* != 0) { rt.break_flag.* = 0; break; }
        if (rt.continue_flag.* != 0) { rt.continue_flag.* = 0; continue; }
        if (rt.error_flag.* != 0 or rt.return_flag.* != 0) return result;
    }
    return result;
}

fn eval_for(words: []const i32) i32 {
    if (words.len < 5) return 0;
    const init_s = obj_ensure_string(words[1]);
    const cond_s = obj_ensure_string(words[2]);
    const next_s = obj_ensure_string(words[3]);
    const body_s = obj_ensure_string(words[4]);
    _ = eval_script(init_s.ptr, init_s.len);
    if (has_signal()) return 0;
    var result: i32 = 0;
    while (true) {
        if (eval_expr_str(cond_s.ptr, cond_s.len) == 0) break;
        result = eval_script(body_s.ptr, body_s.len);
        if (rt.break_flag.* != 0) { rt.break_flag.* = 0; break; }
        if (rt.continue_flag.* != 0) { rt.continue_flag.* = 0; }
        if (rt.error_flag.* != 0 or rt.return_flag.* != 0) return result;
        _ = eval_script(next_s.ptr, next_s.len);
    }
    return result;
}

fn eval_foreach(words: []const i32) i32 {
    if (words.len < 4) return 0;
    const var_name = words[1];
    const list_s = obj_ensure_string(words[2]);
    const body_s = obj_ensure_string(words[3]);
    const n = list_count_elements(list_s.ptr, list_s.len);
    var result: i32 = 0;
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        const elem = list_element_at(list_s.ptr, list_s.len, idx);
        const elem_val = if (elem.braced)
            obj_new_string_copy(list_s.ptr + elem.start, elem.len)
        else blk: {
            const buf = alloc(elem.len);
            const out_len = copy_unbraced_elem(buf, list_s.ptr + elem.start, elem.len);
            break :blk obj_new_string(@intCast(buf), @intCast(out_len));
        };
        _ = frames.var_set(var_name, elem_val);
        result = eval_script(body_s.ptr, body_s.len);
        if (rt.break_flag.* != 0) { rt.break_flag.* = 0; break; }
        if (rt.continue_flag.* != 0) { rt.continue_flag.* = 0; continue; }
        if (rt.error_flag.* != 0 or rt.return_flag.* != 0) return result;
    }
    return result;
}

// -- Proc dispatch (picol-style) --
// Look up the command name in the proc registry. If found:
//   1. Push a new call frame
//   2. Bind arguments to parameter names as local variables
//   3. Evaluate the body
//   4. Pop the frame
//   5. Absorb RETURN signal (convert to OK)
// If not found, raise an error.

/// Alias dispatch trampoline.  An ``interp alias`` redirect Command
/// has ``CMD_ALIAS`` set in its flags and stores an
/// :type:`tcl_alias.AliasRec` in ``params_obj``.  On dispatch we:
///
///   1. Build a new argv: ``[target_name, prefix_args..., words[1..]]``.
///   2. Resolve the target command by name (the alias tracks
///      rename / deletion of its target automatically because
///      resolution is by-string, not by-pointer).
///   3. Recurse through ``eval_proc_call_bucket`` — this preserves
///      all the compiled-proc / host-bridge paths the target might
///      take.
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
        const msg = rt.obj_new_string_copy(
            @intFromPtr("alias argv exceeds MAX_WORDS".ptr),
            28,
        );
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

    // Resolve the target.  Use the current-ns context so an alias
    // whose target is unqualified (rare, but valid) falls through
    // the normal resolution chain.  On miss we do NOT fall through
    // to ``eval_proc_call``'s stub dispatch — alias targets are
    // user-defined by construction; a missing target is a clear
    // "unknown command" diagnostic.
    const target_bucket = procs.proc_lookup(new_words[0]);
    if (target_bucket == 0) {
        const catch_mod = @import("tcl_catch.zig");
        catch_mod.error_unknown_command(new_words[0]);
        return 0;
    }
    return eval_proc_call_bucket(new_words[0..total], target_bucket);
}

fn eval_proc_call(words: []const i32) i32 {
    const bucket = procs.proc_lookup(words[0]);
    if (bucket == 0) {
        // Before declaring the command unknown, consult the stub
        // dispatch table — Tcl 8.4–9.0 core commands we haven't
        // implemented (encoding, fconfigure, regexp, trace, …) are
        // routed here from compiled-code fallbacks and produce
        // "unsupported command: <name>" rather than the generic
        // "unknown command: <name>" message.  Keeping the
        // dispatch-before-error pattern means user-defined procs
        // still win when they shadow a core command.
        const stub_dispatch = @import("tcl_cmd_dispatch.zig");
        const cmd_s = obj_ensure_string(words[0]);
        if (stub_dispatch.try_stub(@as([*]const u8, @ptrFromInt(cmd_s.ptr)), cmd_s.len)) {
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
    // Compiled proc (func_idx != 0 is a marker set by
    // ``proc_register_compiled``) — dispatch via the host bridge
    // because pure WASM can't call across modules.  The bridge
    // looks up the proc's compiled WASM function by its
    // *registered* (fully-qualified) name — taken from the
    // bucket, not ``words[0]`` — and invokes it with the
    // unpacked argv.
    const func_idx = procs.proc_get_func_idx(bucket);
    if (func_idx != 0) {
        const dispatch_mod = @import("tcl_dispatch.zig");
        return dispatch_mod.dispatch(bucket, words);
    }
    const body_obj = procs.proc_get_body(bucket);
    const params_obj = procs.proc_get_params(bucket);
    const n_params: u32 = @intCast(procs.proc_get_n_params(bucket));

    // Push frame
    _ = frames.frame_push();

    // Bind parameters: walk the params list, assign each from argv.
    // If the last parameter is named "args", it collects all remaining
    // arguments as a Tcl list (standard Tcl variadic proc convention).
    if (params_obj != 0 and n_params > 0) {
        const ps = obj_ensure_string(params_obj);
        var pi: u32 = 0;
        while (pi < n_params) : (pi += 1) {
            const param_elem = list_element_at(ps.ptr, ps.len, @intCast(pi));
            const param_name_ptr = ps.ptr + param_elem.start;
            const param_name_len = param_elem.len;
            const param_name = obj_new_string_copy(param_name_ptr, param_name_len);
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
                    // No remaining args: set to empty list
                    _ = frames.local_set(param_name, obj_new_string(0, 0));
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
                    const buf = alloc(total + 4);
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
                    _ = frames.local_set(param_name, obj_new_string(@bitCast(buf), @bitCast(off)));
                }
                break;
            } else {
                const arg_val = if (arg_idx < words.len) words[arg_idx] else obj_new_string(0, 0);
                _ = frames.local_set(param_name, arg_val);
            }
        }
    }

    // Evaluate body
    const body_s = obj_ensure_string(body_obj);
    const result = eval_script(body_s.ptr, body_s.len);

    // Pop frame
    frames.frame_pop();

    // Absorb return signal (like picol: PICOL_RETURN → PICOL_OK)
    if (rt.return_flag.* != 0) {
        rt.return_flag.* = 0;
        return rt.return_val.*;
    }
    // A break/continue that survived to the proc boundary is a Tcl error
    // ("invoked \"break\" outside of a loop"); clear the flags and raise
    // an error so the signal cannot short-circuit outer eval_script
    // frames in the caller.  The error message uses words[0] (the proc
    // name) because synthesising a fresh string object from a static
    // literal is awkward in the WASM heap model.
    if (rt.break_flag.* != 0 or rt.continue_flag.* != 0) {
        rt.break_flag.* = 0;
        rt.continue_flag.* = 0;
        rt.tcl_cmd_error(words[0]);
    }
    return result;
}

// -- ``rename`` built-in -------------------------------------------------------
//
// ``rename oldName newName``.  ``newName == ""`` deletes ``oldName``.
// Semantics live in ``tcl_rename.zig``; this wrapper parses argv,
// resolves the ``(old_ns, old_simple)`` / ``(new_ns, new_simple)``
// pairs via the qualified-name walker, and formats the user-visible
// error messages.

const rename_mod = @import("tcl_rename.zig");

/// Build an error message like ``can't rename "foo": command doesn't
/// exist`` and route it through the standard error trap.  The
/// per-case templates come from ``tclsh 9.0`` verbatim so tcltest's
/// error-string matchers behave identically.
fn rename_error(template_prefix: []const u8, name_ptr: u32, name_len: u32, template_suffix: []const u8) void {
    const total: u32 = @intCast(template_prefix.len + name_len + template_suffix.len);
    const buf = alloc(total);
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (template_prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (name_len > 0) {
        const src: [*]const u8 = @ptrFromInt(name_ptr);
        for (0..name_len) |k| dst[off + k] = src[k];
        off += name_len;
    }
    for (template_suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = obj_new_string(@bitCast(buf), @bitCast(off));
    const catch_mod = @import("tcl_catch.zig");
    catch_mod.tcl_cmd_error(msg);
}

fn eval_rename(words: []const i32) i32 {
    if (words.len < 3) {
        const catch_mod = @import("tcl_catch.zig");
        const msg = rt.obj_new_string_copy(
            @intFromPtr("wrong # args: should be \"rename oldName newName\"".ptr),
            50,
        );
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const old_s = obj_ensure_string(words[1]);
    const new_s = obj_ensure_string(words[2]);

    const cxt = tcl_ns.ns_current();
    const old_r = tcl_ns.ns_resolve_qualified(cxt, old_s.ptr, old_s.len);
    // Resolve old name: prefer the primary target, fall back to alt
    // (search-from-root when the primary context misses).
    var old_ns: u32 = 0;
    if (old_r.target_ns != 0 and
        tcl_ns.ns_cmd_find(old_r.target_ns, old_r.simple_ptr, old_r.simple_len) != 0)
    {
        old_ns = old_r.target_ns;
    } else if (old_r.alt_ns != 0 and
        tcl_ns.ns_cmd_find(old_r.alt_ns, old_r.simple_ptr, old_r.simple_len) != 0)
    {
        old_ns = old_r.alt_ns;
    } else {
        rename_error("can't rename \"", old_s.ptr, old_s.len, "\": command doesn't exist");
        return 0;
    }

    // Deletion form: new name is empty.
    if (new_s.len == 0) {
        const r = rename_mod.rename_command(
            old_ns,
            old_r.simple_ptr,
            old_r.simple_len,
            0,
            0,
            0,
        );
        switch (r) {
            .ok => return 0,
            .not_found => {
                rename_error("can't rename \"", old_s.ptr, old_s.len, "\": command doesn't exist");
                return 0;
            },
            .builtin_protected => {
                rename_error("can't rename \"", old_s.ptr, old_s.len, "\": built-in command");
                return 0;
            },
            .target_exists => return 0, // unreachable on delete form
        }
    }

    // Move form: resolve new name, materialising missing intermediate
    // namespaces the way ``proc`` does for its registration path.
    const new_r = tcl_ns.ns_resolve_qualified_creating(cxt, new_s.ptr, new_s.len);
    if (new_r.target_ns == 0 or new_r.simple_len == 0) {
        rename_error("can't rename to \"", new_s.ptr, new_s.len, "\": invalid name");
        return 0;
    }
    const r = rename_mod.rename_command(
        old_ns,
        old_r.simple_ptr,
        old_r.simple_len,
        new_r.target_ns,
        new_r.simple_ptr,
        new_r.simple_len,
    );
    switch (r) {
        .ok => return 0,
        .not_found => {
            rename_error("can't rename \"", old_s.ptr, old_s.len, "\": command doesn't exist");
            return 0;
        },
        .target_exists => {
            rename_error("can't rename to \"", new_s.ptr, new_s.len, "\": command already exists");
            return 0;
        },
        .builtin_protected => {
            rename_error("can't rename \"", old_s.ptr, old_s.len, "\": built-in command");
            return 0;
        },
    }
}

// -- ``interp`` built-in -------------------------------------------------------
//
// Currently only ``interp alias`` is implemented, in its
// single-interp form: ``interp alias {} newName {} target ?arg …?``
// creates / queries / deletes an alias in the (only) interpreter.
// Child-interp aliases and the other ``interp`` sub-commands (eval,
// hide, expose, create, delete, …) remain trapping stubs via
// :mod:`tcl_env_stubs`.

fn eval_interp(words: []const i32) i32 {
    if (words.len < 2) {
        const catch_mod = @import("tcl_catch.zig");
        const msg = rt.obj_new_string_copy(
            @intFromPtr("wrong # args: should be \"interp subcommand ?arg ...?\"".ptr),
            55,
        );
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const sub = obj_ensure_string(words[1]);
    if (!str_eq(@ptrFromInt(sub.ptr), sub.len, "alias") and
        !str_eq(@ptrFromInt(sub.ptr), sub.len, "aliases"))
    {
        // Fall back to the existing trapping stub for unsupported
        // ``interp`` subcommands.
        const stubs = @import("tcl_stubs.zig");
        stubs.unsupported("interp");
        return 0;
    }

    // ``interp aliases {}``: list every alias in the (only) interp.
    // We traverse the namespace tree and collect every Command with
    // CMD_ALIAS set, emitting its simple name.
    if (str_eq(@ptrFromInt(sub.ptr), sub.len, "aliases")) {
        return interp_aliases_list();
    }

    // ``interp alias {}`` → 4-arg query / delete / create shapes:
    //
    //   interp alias {} newName                    (query)
    //   interp alias {} newName {}                 (delete)
    //   interp alias {} newName {} target ?arg…?   (create)
    //
    // The ``{}`` placeholders are the target interp path.  With
    // single-interp we only honour the empty-list form.  Any other
    // value is silently treated as "this interp" — good enough for
    // tcltest which always passes ``{}``.
    if (words.len < 4) {
        const catch_mod = @import("tcl_catch.zig");
        const msg = rt.obj_new_string_copy(
            @intFromPtr("wrong # args: should be \"interp alias path ?arg ...?\"".ptr),
            55,
        );
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    // words[2] = target interp path (we require empty or a single
    // ``{}``).  words[3] = alias name in that interp.  words[4] (if
    // present) = second interp path.  words[5+] = target + prefix.
    const new_name = obj_ensure_string(words[3]);
    if (words.len == 4) {
        // Query form: ``interp alias {} newName``.
        return interp_alias_query(new_name.ptr, new_name.len);
    }
    // words[4] = source interp path (must also be empty for us).
    if (words.len == 5) {
        const src_path = obj_ensure_string(words[4]);
        if (src_path.len == 0) {
            // ``interp alias {} newName {}``: delete form.
            return interp_alias_delete(new_name.ptr, new_name.len);
        }
    }
    // words[5+] = target cmd + prefix args.
    if (words.len < 6) {
        const catch_mod = @import("tcl_catch.zig");
        const msg = rt.obj_new_string_copy(
            @intFromPtr("wrong # args: should be \"interp alias path srcCmd path targetCmd ?arg ...?\"".ptr),
            74,
        );
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const target_name = obj_ensure_string(words[5]);
    // Pack prefix args into a bump-allocated u32 array.
    const n_prefix: u32 = @as(u32, @intCast(words.len)) - 6;
    var prefix_buf: u32 = 0;
    if (n_prefix > 0) {
        prefix_buf = alloc(n_prefix * 4);
        var i: u32 = 0;
        while (i < n_prefix) : (i += 1) {
            write_i32(prefix_buf + i * 4, words[6 + i]);
        }
    }
    return interp_alias_create(
        new_name.ptr,
        new_name.len,
        target_name.ptr,
        target_name.len,
        n_prefix,
        prefix_buf,
    );
}

fn interp_alias_create(
    new_name_ptr: u32,
    new_name_len: u32,
    target_name_ptr: u32,
    target_name_len: u32,
    n_prefix: u32,
    prefix_buf: u32,
) i32 {
    // Resolve the alias-home ns + simple name.  Creating missing
    // intermediates keeps parity with ``proc`` defines via FQN.
    const cxt = tcl_ns.ns_current();
    const r = tcl_ns.ns_resolve_qualified_creating(cxt, new_name_ptr, new_name_len);
    if (r.target_ns == 0 or r.simple_len == 0) return 0;

    // If an alias / command already lives under this name, replace
    // it.  This matches C Tcl where ``interp alias {} foo {} bar``
    // overwrites any previous ``foo`` (proc, alias, or otherwise).
    // The previous Command stays in linear memory — leaked per the
    // bump-allocator contract.
    const cmd = alias_mod.alias_alloc(
        r.simple_ptr,
        r.simple_len,
        target_name_ptr,
        target_name_len,
        n_prefix,
        prefix_buf,
    );
    _ = tcl_ns.ns_cmd_put(r.target_ns, r.simple_ptr, r.simple_len, cmd);
    // Bump the proc counter so ``proc_buf_nonzero`` fires for
    // bundles whose only commands are aliases.
    procs.proc_count_bump();
    return words_obj_new_string_dup(new_name_ptr, new_name_len);
}

fn interp_alias_query(new_name_ptr: u32, new_name_len: u32) i32 {
    const cxt = tcl_ns.ns_current();
    const cmd = tcl_ns.ns_find_command(cxt, new_name_ptr, new_name_len);
    if (!alias_mod.is_alias(cmd)) return 0;
    return alias_mod.alias_describe(cmd);
}

fn interp_alias_delete(new_name_ptr: u32, new_name_len: u32) i32 {
    const cxt = tcl_ns.ns_current();
    const r = tcl_ns.ns_resolve_qualified(cxt, new_name_ptr, new_name_len);
    const host_ns: u32 = if (r.target_ns != 0 and
        tcl_ns.ns_cmd_find(r.target_ns, r.simple_ptr, r.simple_len) != 0)
        r.target_ns
    else if (r.alt_ns != 0)
        r.alt_ns
    else
        return 0;
    const cmd = tcl_ns.ns_cmd_find(host_ns, r.simple_ptr, r.simple_len);
    if (alias_mod.is_alias(cmd)) {
        alias_mod.alias_clear(cmd);
    }
    _ = tcl_ns.ns_cmd_clear(host_ns, r.simple_ptr, r.simple_len);
    procs.lru_invalidate_all();
    return 0;
}

/// Return a TclObj wrapping a fresh string copy of the given bytes.
/// Tiny helper to avoid pulling in ``obj_new_string_copy``'s ABI
/// naming at every callsite.
fn words_obj_new_string_dup(ptr: u32, len: u32) i32 {
    const buf = alloc(len);
    if (len > 0) memcpy(buf, ptr, len);
    return obj_new_string(@bitCast(buf), @bitCast(len));
}

/// ``interp aliases {}`` — list every registered alias.  Walks every
/// namespace in the tree, visiting each one's ``cmd_table`` once and
/// emitting commands flagged ``CMD_ALIAS``.  Output is a Tcl list of
/// simple alias names (not FQNs) — matches ``tclsh``'s default.
fn interp_aliases_list() i32 {
    // Accumulator: sum string lengths (plus separators) to size the
    // output buffer.  We walk the tree twice: once to size, once to
    // fill.  Single-pass grown allocation would require a realloc
    // path the bump allocator doesn't support.
    const root = tcl_ns.ns_root();
    var ctx: AliasListCtx = .{ .total = 0, .count = 0, .buf = 0, .off = 0 };
    walk_ns_for_aliases(root, &ctx);
    if (ctx.total == 0) return obj_new_string(0, 0);

    ctx.buf = alloc(ctx.total);
    ctx.off = 0;
    ctx.count = 0;
    // Second pass fills the buffer.  ``fill = true``.
    walk_ns_for_aliases_fill(root, &ctx);
    return obj_new_string(@bitCast(ctx.buf), @bitCast(ctx.off));
}

const AliasListCtx = struct {
    total: u32,
    count: u32,
    buf: u32,
    off: u32,
};

fn walk_ns_for_aliases(ns: u32, ctx: *AliasListCtx) void {
    const n: *const tcl_ns.Namespace = @ptrFromInt(ns);
    if (n.cmd_table.buf != 0) {
        var i: u32 = 0;
        const bucket_size: u32 = 16;
        while (i < n.cmd_table.cap) : (i += 1) {
            const bucket = n.cmd_table.buf + i * bucket_size;
            const name_ptr: u32 = @bitCast(read_i32(bucket));
            if (name_ptr == 0) continue;
            const name_len: u32 = @bitCast(read_i32(bucket + 4));
            const cmd: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
            if (cmd == 0) continue;
            if (!alias_mod.is_alias(cmd)) continue;
            // Reserve space: leading sep if not first, then the name.
            if (ctx.count > 0) ctx.total += 1;
            ctx.total += name_len;
            ctx.count += 1;
        }
    }
    if (n.child_table.buf != 0) {
        var i: u32 = 0;
        const bucket_size: u32 = 16;
        while (i < n.child_table.cap) : (i += 1) {
            const bucket = n.child_table.buf + i * bucket_size;
            const name_ptr: u32 = @bitCast(read_i32(bucket));
            if (name_ptr == 0) continue;
            const child: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
            if (child != 0) walk_ns_for_aliases(child, ctx);
        }
    }
}

fn walk_ns_for_aliases_fill(ns: u32, ctx: *AliasListCtx) void {
    const n: *const tcl_ns.Namespace = @ptrFromInt(ns);
    if (n.cmd_table.buf != 0) {
        var i: u32 = 0;
        const bucket_size: u32 = 16;
        while (i < n.cmd_table.cap) : (i += 1) {
            const bucket = n.cmd_table.buf + i * bucket_size;
            const name_ptr: u32 = @bitCast(read_i32(bucket));
            if (name_ptr == 0) continue;
            const name_len: u32 = @bitCast(read_i32(bucket + 4));
            const cmd: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
            if (cmd == 0) continue;
            if (!alias_mod.is_alias(cmd)) continue;
            if (ctx.count > 0) {
                const d: [*]u8 = @ptrFromInt(ctx.buf + ctx.off);
                d[0] = ' ';
                ctx.off += 1;
            }
            const src: [*]const u8 = @ptrFromInt(name_ptr);
            const d: [*]u8 = @ptrFromInt(ctx.buf + ctx.off);
            for (0..name_len) |k| d[k] = src[k];
            ctx.off += name_len;
            ctx.count += 1;
        }
    }
    if (n.child_table.buf != 0) {
        var i: u32 = 0;
        const bucket_size: u32 = 16;
        while (i < n.child_table.cap) : (i += 1) {
            const bucket = n.child_table.buf + i * bucket_size;
            const name_ptr: u32 = @bitCast(read_i32(bucket));
            if (name_ptr == 0) continue;
            const child: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
            if (child != 0) walk_ns_for_aliases_fill(child, ctx);
        }
    }
}

fn eval_string_cmd(words: []const i32) i32 {
    if (words.len < 3) return 0;
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "length")) return rt.string_length(words[2]);
    if (str_eq(sp, sub.len, "index") and words.len >= 4) return rt.string_index(words[2], words[3]);
    if (str_eq(sp, sub.len, "range") and words.len >= 5) return rt.string_range(words[2], words[3], words[4]);
    if (str_eq(sp, sub.len, "compare") and words.len >= 4) return rt.string_compare(words[2], words[3]);
    if (str_eq(sp, sub.len, "equal") and words.len >= 4) return rt.string_equal(words[2], words[3]);
    if (str_eq(sp, sub.len, "match") and words.len >= 4) return rt.string_match(words[2], words[3]);
    if (str_eq(sp, sub.len, "map") and words.len >= 4) return rt.string_map(words[2], words[3]);
    if (str_eq(sp, sub.len, "trim")) {
        const chars = if (words.len >= 4) words[3] else 0;
        return rt.string_trim(words[2], chars);
    }
    if (str_eq(sp, sub.len, "trimleft")) {
        const chars = if (words.len >= 4) words[3] else 0;
        return rt.string_trimleft(words[2], chars);
    }
    if (str_eq(sp, sub.len, "trimright")) {
        const chars = if (words.len >= 4) words[3] else 0;
        return rt.string_trimright(words[2], chars);
    }
    if (str_eq(sp, sub.len, "first") and words.len >= 4) return rt.string_first(words[2], words[3]);
    if (str_eq(sp, sub.len, "last") and words.len >= 4) return rt.string_last(words[2], words[3]);
    if (str_eq(sp, sub.len, "toupper")) return rt.string_toupper(words[2]);
    if (str_eq(sp, sub.len, "tolower")) return rt.string_tolower(words[2]);
    if (str_eq(sp, sub.len, "reverse")) return rt.string_reverse(words[2]);
    if (str_eq(sp, sub.len, "repeat") and words.len >= 4) return rt.string_repeat(words[2], words[3]);
    if (str_eq(sp, sub.len, "replace") and words.len >= 6) return rt.string_replace(words[2], words[3], words[4], words[5]);
    if (str_eq(sp, sub.len, "is")) {
        // ``string is class ?-strict? ?-failindex var? str``
        // Find the class name (words[2]) and the final string arg.
        // Skip any -strict / -failindex flags and their args.
        if (words.len < 4) return obj_new_int(1); // empty string: non-strict default is 1
        const cls = obj_ensure_string(words[2]);
        const clsp: [*]const u8 = @ptrFromInt(cls.ptr);
        var str_idx: u32 = 3;
        while (str_idx + 1 < words.len) {
            const a = obj_ensure_string(words[str_idx]);
            const ap: [*]const u8 = @ptrFromInt(a.ptr);
            if (a.len > 0 and ap[0] == '-') {
                // -strict: no extra arg; -failindex: consumes next arg
                if (str_eq(ap, a.len, "-failindex")) str_idx += 1;
                str_idx += 1;
            } else break;
        }
        if (str_idx >= words.len) return obj_new_int(1);
        const sv = obj_ensure_string(words[str_idx]);
        if (sv.len == 0) {
            // non-strict: empty is 1 for all; strict: 0
            return obj_new_int(1);
        }
        const svp: [*]const u8 = @ptrFromInt(sv.ptr);
        if (str_eq(clsp, cls.len, "print")) {
            // printable: 0x20-0x7E ASCII, or any multibyte UTF-8
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) continue; // multibyte UTF-8 — treat as printable
                if (b < 0x20 or b == 0x7F) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "alpha")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                if (!((b >= 'a' and b <= 'z') or (b >= 'A' and b <= 'Z'))) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "digit")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                if (svp[i] < '0' or svp[i] > '9') return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "alnum")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                if (!((b >= 'a' and b <= 'z') or (b >= 'A' and b <= 'Z') or (b >= '0' and b <= '9'))) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "space") or str_eq(clsp, cls.len, "whitespace")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b != ' ' and b != '\t' and b != '\n' and b != '\r' and b != 0x0C and b != 0x0B) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "integer")) {
            var i: u32 = 0;
            while (i < sv.len and (svp[i] == ' ' or svp[i] == '\t')) i += 1;
            if (i < sv.len and (svp[i] == '+' or svp[i] == '-')) i += 1;
            if (i < sv.len and svp[i] == '0' and i + 1 < sv.len and (svp[i+1] == 'x' or svp[i+1] == 'X')) {
                i += 2;
                if (i >= sv.len) return obj_new_int(0);
                while (i < sv.len) : (i += 1) {
                    const b = svp[i];
                    if (!((b >= '0' and b <= '9') or (b >= 'a' and b <= 'f') or (b >= 'A' and b <= 'F'))) return obj_new_int(0);
                }
                return obj_new_int(1);
            }
            if (i >= sv.len) return obj_new_int(0);
            while (i < sv.len) : (i += 1) {
                if (svp[i] < '0' or svp[i] > '9') return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "boolean")) {
            if (str_eq(svp, sv.len, "1") or str_eq(svp, sv.len, "0") or
                str_eq(svp, sv.len, "true") or str_eq(svp, sv.len, "false") or
                str_eq(svp, sv.len, "yes") or str_eq(svp, sv.len, "no") or
                str_eq(svp, sv.len, "on") or str_eq(svp, sv.len, "off") or
                str_eq(svp, sv.len, "True") or str_eq(svp, sv.len, "False") or
                str_eq(svp, sv.len, "TRUE") or str_eq(svp, sv.len, "FALSE")) return obj_new_int(1);
            return obj_new_int(0);
        }
        if (str_eq(clsp, cls.len, "ascii")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                if (svp[i] > 0x7F) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "control")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) return obj_new_int(0);
                if (b >= 0x20 and b != 0x7F) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "graph")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                if (b <= 0x20 or b == 0x7F) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "lower")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                if (b < 'a' or b > 'z') return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "upper")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                if (b < 'A' or b > 'Z') return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "punct")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                const is_punct = (b >= '!' and b <= '/') or (b >= ':' and b <= '@') or
                    (b >= '[' and b <= '`') or (b >= '{' and b <= '~');
                if (!is_punct) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "xdigit")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (!((b >= '0' and b <= '9') or (b >= 'a' and b <= 'f') or (b >= 'A' and b <= 'F'))) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "double") or str_eq(clsp, cls.len, "float")) {
            // Very basic: try to parse as number with optional decimal/exponent
            var i: u32 = 0;
            while (i < sv.len and (svp[i] == ' ' or svp[i] == '\t')) i += 1;
            if (i < sv.len and (svp[i] == '+' or svp[i] == '-')) i += 1;
            var has_digit = false;
            while (i < sv.len and svp[i] >= '0' and svp[i] <= '9') { i += 1; has_digit = true; }
            if (i < sv.len and svp[i] == '.') {
                i += 1;
                while (i < sv.len and svp[i] >= '0' and svp[i] <= '9') { i += 1; has_digit = true; }
            }
            if (!has_digit) return obj_new_int(0);
            if (i < sv.len and (svp[i] == 'e' or svp[i] == 'E')) {
                i += 1;
                if (i < sv.len and (svp[i] == '+' or svp[i] == '-')) i += 1;
                if (i >= sv.len or svp[i] < '0' or svp[i] > '9') return obj_new_int(0);
                while (i < sv.len and svp[i] >= '0' and svp[i] <= '9') i += 1;
            }
            if (i != sv.len) return obj_new_int(0);
            return obj_new_int(1);
        }
        // Unknown class — return 0
        return obj_new_int(0);
    }
    return 0;
}

fn eval_array_cmd(words: []const i32) i32 {
    if (words.len < 3) return 0;
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    const array_mod = @import("tcl_array.zig");
    if (str_eq(sp, sub.len, "get")) {
        if (words.len >= 4) return array_mod.array_get(words[2], words[3]);
        return array_mod.array_get(words[2], obj_new_string(0, 0));
    }
    if (str_eq(sp, sub.len, "set") and words.len >= 4) {
        // ``array set arr pairlist`` — the payload is a Tcl list
        // of ``{k v k v …}`` pairs.  We always route through
        // ``array_set_list`` because even a single-pair invocation
        // (``array set a {key value}``) is just a 2-element list.
        // The previous shape ``array_set(words[2], words[3], 0)``
        // stored the whole payload under one key with a null
        // value — that silently broke tcltest's
        // ``ArrayDefault numTests [list Total 0 …]``
        // initialisation (``incr numTests(Total)`` then ran on
        // an uninitialised element).
        return array_mod.array_set_list(words[2], words[3]);
    }
    if (str_eq(sp, sub.len, "exists")) return array_mod.array_exists(words[2]);
    if (str_eq(sp, sub.len, "names")) {
        // ``array names arr ?pattern? ?mode?`` — we handle the
        // first two positions; ``mode`` (``-exact`` / ``-glob`` /
        // ``-regexp``) beyond glob isn't wired yet.
        const pat: i32 = if (words.len >= 4) words[3] else 0;
        return array_mod.array_names(words[2], pat);
    }
    if (str_eq(sp, sub.len, "size")) return array_mod.array_size(words[2]);
    if (str_eq(sp, sub.len, "unset")) {
        if (words.len >= 4) return array_mod.array_unset_element(words[2], words[3]);
        return array_mod.array_unset(words[2]);
    }
    // Other subcommands (statistics, startsearch, …) not yet wired —
    // fall through to the stub dispatch which raises the exception.
    const stubs_mod = @import("tcl_stubs.zig");
    const sub_slice: []const u8 = (@as([*]const u8, @ptrFromInt(sub.ptr)))[0..sub.len];
    stubs_mod.unsupported_sub("array", sub_slice);
    return 0;
}

fn eval_dict_cmd(words: []const i32) i32 {
    if (words.len < 3) return 0;
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "get") and words.len >= 4) return rt.dict_get(words[2], words[3]);
    if (str_eq(sp, sub.len, "set") and words.len >= 5) {
        const cur = frames.var_resolve(words[2]);
        const result = rt.dict_set(cur, words[3], words[4]);
        _ = frames.var_set(words[2], result);
        return result;
    }
    if (str_eq(sp, sub.len, "exists") and words.len >= 4) return rt.dict_exists(words[2], words[3]);
    if (str_eq(sp, sub.len, "keys")) return rt.dict_keys(words[2]);
    if (str_eq(sp, sub.len, "values")) return rt.dict_values(words[2]);
    if (str_eq(sp, sub.len, "size")) return rt.dict_size(words[2]);
    if (str_eq(sp, sub.len, "create")) return rt.dict_create();
    return 0;
}

// -- Main eval entry point --

// Maximum number of words after {*} expansion.  The parse limit is
// MAX_WORDS per command, but each {*}$var can expand to many elements.
// 128 is generous enough for realistic Tcl calls while staying cheap
// on the WASM stack.
const MAX_EXPANDED_WORDS: u32 = 128;

pub fn eval_script(script_ptr: u32, script_len: u32) i32 {
    if (script_len == 0) return 0;

    // Save any outer eval context so nested eval_script invocations
    // (e.g. a command-substitution inside a word) can restore it
    // when they return.  Without this the outermost frame's trap
    // context would be replaced by the innermost — and the reader
    // would lose the "which fallback fired this?" line.
    const diag = @import("tcl_diag.zig");
    const saved_ptr = diag.current_eval_ptr;
    const saved_len = diag.current_eval_len;
    const saved_pos = diag.current_eval_pos;
    defer {
        diag.current_eval_ptr = saved_ptr;
        diag.current_eval_len = saved_len;
        diag.current_eval_pos = saved_pos;
    }

    // P9.2: fast path — if this body was pre-parsed by
    // ``parse_cache.build_for_body`` (called from ``proc_register``
    // in P9.3), replay the cached command list without re-parsing.
    const parse_cache = @import("parse_cache.zig");
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

    while (pos < script_len) {
        diag.current_eval_ptr = script_ptr;
        diag.current_eval_len = script_len;
        diag.current_eval_pos = pos;

        const cmd = parse.ParseCommand(src, pos, script_len, &tok_buf, tok_buf.len);
        pos = cmd.next;
        if (cmd.n_words == 0) continue;

        result = execute_parsed_command(cmd.src_ptr, cmd.tokens_ptr, cmd.tokens_len);
        if (has_signal()) return result;
    }
    return result;
}

/// Replay pre-parsed command records from a parse-cache slab.
/// Exits on the first command that raises a signal (break /
/// continue / return / error) — same semantics as the cold path.
fn eval_cached_slab(slab: u32, body_ptr: u32, body_len: u32) i32 {
    const parse_cache = @import("parse_cache.zig");
    const diag = @import("tcl_diag.zig");
    const n_cmds = parse_cache.slab_n_commands(slab);
    var result: i32 = 0;
    var i: u32 = 0;
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
        diag.current_eval_pos = if (i == 0) 0 else @bitCast(obj_mod.read_i32(parse_cache.command_record(slab, i - 1) + parse_cache.OFF_CR_NEXT_POS));
        const tokens_ptr = parse_cache.token_at(slab, tok_offset);
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
                word_objs[wi] = obj_new_string(@intCast(wptr_abs), @intCast(tok.len));
            } else {
                word_objs[wi] = subst_word(wptr_abs, tok.len);
            }
            wi += 1;
        }
        return eval_command(word_objs[0..wi]);
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
        const word_obj: i32 = if (tok.braced)
            obj_new_string(@intCast(wptr_abs), @intCast(tok.len))
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
                    expanded[ecount] = obj_new_string(@intCast(buf), @intCast(out_len));
                }
                ecount += 1;
            }
        } else {
            if (ecount < MAX_EXPANDED_WORDS) {
                expanded[ecount] = word_obj;
                ecount += 1;
            }
        }
    }
    return eval_command(expanded[0..ecount]);
}

// Exported: evaluate a Tcl script string.
pub export fn tcl_eval(script: i32) i32 {
    const s = obj_ensure_string(script);
    if (s.len == 0) return 0;
    return eval_script(s.ptr, s.len);
}
