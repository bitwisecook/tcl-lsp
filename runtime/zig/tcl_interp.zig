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

const MAX_WORDS: u32 = 32;

fn skip_space(src: [*]const u8, pos: u32, len: u32) u32 {
    var p = pos;
    while (p < len and (src[p] == ' ' or src[p] == '\t')) p += 1;
    return p;
}

fn parse_braced(src: [*]const u8, pos: u32, len: u32) struct { end: u32, start: u32, wlen: u32 } {
    var p = pos + 1;
    const start = p;
    var depth: u32 = 1;
    while (p < len and depth > 0) {
        // Tcl brace parsing: ``\<anychar>`` inside a braced word
        // consumes two bytes without affecting the brace depth —
        // so ``\{`` / ``\}`` are NOT depth-changing sequences,
        // matching Tcl's TclParseBraces.  Without this, a test body
        // like ``{lappend x \{\  abc}`` would see the ``\{`` bump
        // depth past the closing ``}`` and consume the rest of the
        // script into the single word.
        if (src[p] == '\\' and p + 1 < len) {
            p += 2;
            continue;
        }
        if (src[p] == '{') depth += 1 else if (src[p] == '}') depth -= 1;
        if (depth > 0) p += 1 else p += 1;
    }
    return .{ .end = p, .start = start, .wlen = p - 1 - start };
}

fn parse_quoted(src: [*]const u8, pos: u32, len: u32) struct { end: u32, start: u32, wlen: u32 } {
    var p = pos + 1;
    const start = p;
    while (p < len and src[p] != '"') {
        if (src[p] == '\\' and p + 1 < len) p += 2 else p += 1;
    }
    const wlen = p - start;
    if (p < len) p += 1;
    return .{ .end = p, .start = start, .wlen = wlen };
}

fn parse_bare(src: [*]const u8, pos: u32, len: u32) struct { end: u32, start: u32, wlen: u32 } {
    const start = pos;
    var p = pos;
    // Scan until a top-level terminator.  Crucially, nested ``[...]``
    // command substitutions and ``${...}`` variable references must
    // be kept inside the same word — splitting on the space inside
    // ``[clock seconds]`` would truncate the inner command when
    // subst_word later runs it through eval_script (the observed
    // "unknown command: cloc" off-by-one).
    while (p < len and src[p] != ' ' and src[p] != '\t' and
        src[p] != '\n' and src[p] != ';' and src[p] != '\r')
    {
        if (src[p] == '\\' and p + 1 < len) {
            p += 2;
        } else if (src[p] == '[') {
            // Skip a balanced ``[...]`` subscript in one gulp so
            // whitespace inside a command substitution does not
            // terminate the outer word.
            var depth: u32 = 1;
            p += 1;
            while (p < len and depth > 0) {
                if (src[p] == '\\' and p + 1 < len) {
                    p += 2;
                    continue;
                }
                if (src[p] == '[') depth += 1;
                if (src[p] == ']') depth -= 1;
                p += 1;
            }
        } else if (src[p] == '$' and p + 1 < len and src[p + 1] == '{') {
            // ``${...}`` keeps its braces together — normal $name
            // refs terminate on the first non-identifier char which
            // is handled by the outer loop already.
            p += 2;
            while (p < len and src[p] != '}') p += 1;
            if (p < len) p += 1;
        } else {
            p += 1;
        }
    }
    return .{ .end = p, .start = start, .wlen = p - start };
}

fn parse_command(
    src: [*]const u8,
    pos: u32,
    len: u32,
    word_ptrs: *[MAX_WORDS]u32,
    word_lens: *[MAX_WORDS]u32,
    word_braced: *[MAX_WORDS]bool,
    word_expand: *[MAX_WORDS]bool,
) struct { count: u32, next: u32 } {
    var p = pos;
    var count: u32 = 0;

    while (p < len and (src[p] == ' ' or src[p] == '\t' or src[p] == '\n' or src[p] == '\r' or src[p] == ';')) p += 1;

    if (p < len and src[p] == '#') {
        while (p < len and src[p] != '\n') p += 1;
        if (p < len) p += 1;
        return .{ .count = 0, .next = p };
    }

    while (p < len and count < MAX_WORDS) {
        p = skip_space(src, p, len);
        if (p >= len or src[p] == '\n' or src[p] == ';' or src[p] == '\r') {
            if (p < len) p += 1;
            break;
        }
        if (src[p] == '#' and count == 0) {
            while (p < len and src[p] != '\n') p += 1;
            if (p < len) p += 1;
            break;
        }

        // Detect ``{*}`` argument-expansion prefix (Tcl 8.5+).
        // The three-character sequence ``{*}`` immediately before a
        // word signals that the word should be evaluated and then
        // split as a Tcl list, with each element inserted as a
        // separate argument.  Strip the prefix here and record the
        // expansion flag; the actual splitting happens in eval_script.
        var expand = false;
        if (src[p] == '{' and p + 2 < len and src[p + 1] == '*' and src[p + 2] == '}') {
            expand = true;
            p += 3;
            // Skip any whitespace between {*} and the word (rare but
            // valid in Tcl: ``cmd {*} $args`` is the same as
            // ``cmd {*}$args``).
            p = skip_space(src, p, len);
            if (p >= len or src[p] == '\n' or src[p] == ';') {
                // bare {*} with nothing following — treat as empty expansion
                word_ptrs[count] = 0;
                word_lens[count] = 0;
                word_braced[count] = false;
                word_expand[count] = true;
                count += 1;
                break;
            }
        }

        if (src[p] == '{') {
            const r = parse_braced(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            word_braced[count] = true;
            word_expand[count] = expand;
            count += 1;
            p = r.end;
        } else if (src[p] == '"') {
            const r = parse_quoted(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            word_braced[count] = false;
            word_expand[count] = expand;
            count += 1;
            p = r.end;
        } else {
            const r = parse_bare(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            word_braced[count] = false;
            word_expand[count] = expand;
            count += 1;
            p = r.end;
        }
    }
    return .{ .count = count, .next = p };
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
        // ``variable name ?value? ?name value …?`` — declare +
        // optionally initialise a namespace variable.  We don't
        // track namespace scopes in the interpreter, so treat it
        // identically to ``set`` when a value is given, else a NOP.
        var i: u32 = 1;
        while (i < words.len) : (i += 2) {
            if (i + 1 < words.len) {
                _ = frames.var_set(words[i], words[i + 1]);
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
        str_eq(cmd, cmd_s.len, "variable") or
        str_eq(cmd, cmd_s.len, "rename"))
    { return 0; }
    // ``namespace eval <ns> <body>`` — execute the body script.
    // We ignore the namespace argument (no namespace tracking in the
    // interpreter's flat model) and just run the body so that commands
    // like ``upvar 0 src local`` inside namespace-eval blocks take
    // effect.  Other namespace sub-commands (import, exists, …) are
    // silently treated as no-ops.
    if (str_eq(cmd, cmd_s.len, "namespace")) {
        if (words.len >= 3) {
            const sub = obj_ensure_string(words[1]);
            if (sub.len == 4 and sub.ptr != 0) {
                const sp: [*]const u8 = @ptrFromInt(sub.ptr);
                if (sp[0] == 'e' and sp[1] == 'v' and sp[2] == 'a' and sp[3] == 'l') {
                    // namespace eval <ns> script ?arg? ... — concatenate body args
                    // with single spaces (matches Tcl semantics).
                    if (words.len == 4) {
                        const bs = obj_ensure_string(words[3]);
                        if (bs.len > 0) return eval_script(bs.ptr, bs.len);
                        return 0;
                    }
                    if (words.len > 4) {
                        var total: u32 = 0;
                        var wi3: u32 = 3;
                        while (wi3 < words.len) : (wi3 += 1) {
                            const ws = obj_ensure_string(words[wi3]);
                            total += ws.len;
                            if (wi3 + 1 < words.len) total += 1; // space
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
            }
        }
        return 0;
    }
    // -- Proc dispatch: check registry before erroring --
    return eval_proc_call(words);
}

const str_eq = @import("tcl_chars.zig").str_eq;

/// Namespace context for eval-fallback calls.  Compiled procs set
/// this before calling :func:`tcl_eval` (via :func:`ns_set`) so
/// commands like ``proc $varName body`` inside the fallback
/// register in the enclosing namespace instead of the global scope.
/// Zero means "no namespace context" — unqualified names stay
/// unqualified.
var current_ns_ptr: u32 = 0;
var current_ns_len: u32 = 0;

/// Set the current namespace (pointer + length into UTF-8 bytes).
/// Returns a packed save value the caller should pass back to
/// ``ns_restore`` to unwind — supports nesting without a heap stack.
pub export fn ns_set(name_ptr: i32, name_len: i32) i64 {
    const saved: i64 = (@as(i64, current_ns_ptr) << 32) | @as(i64, current_ns_len);
    current_ns_ptr = @intCast(name_ptr);
    current_ns_len = @intCast(name_len);
    return saved;
}

/// Restore a saved namespace context, unwinding an ``ns_set`` pair.
pub export fn ns_restore(saved: i64) void {
    current_ns_ptr = @intCast((saved >> 32) & 0xFFFFFFFF);
    current_ns_len = @intCast(saved & 0xFFFFFFFF);
}

/// If *name* (a TclObj) is unqualified (no leading ``::``) and a
/// current namespace context is active, return a fresh TclObj
/// holding ``<ns>::<name>``.  Otherwise return *name* unchanged.
/// Used by the interpreter's ``proc`` / ``variable`` handlers to
/// namespace-qualify dynamically constructed names.
fn qualify_name(name: i32) i32 {
    if (current_ns_ptr == 0 or current_ns_len == 0) return name;
    const s = obj_ensure_string(name);
    if (s.len == 0) return name;
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    // Already qualified with ``::`` — leave alone.
    if (s.len >= 2 and sp[0] == ':' and sp[1] == ':') return name;
    // Build ``<ns>::<name>`` in the bump allocator.
    const ns_ptr: [*]const u8 = @ptrFromInt(current_ns_ptr);
    const total: u32 = current_ns_len + 2 + s.len;
    const buf_addr: u32 = obj_mod.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    for (0..current_ns_len) |i| buf[i] = ns_ptr[i];
    buf[current_ns_len] = ':';
    buf[current_ns_len + 1] = ':';
    for (0..s.len) |i| buf[current_ns_len + 2 + i] = sp[i];
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
    const src: [*]const u8 = @ptrFromInt(script_ptr);
    var pos: u32 = 0;
    var result: i32 = 0;
    var wp: [MAX_WORDS]u32 = undefined;
    var wl: [MAX_WORDS]u32 = undefined;
    // ``wb[i] == true`` means word i was parsed as ``{braced}``
    // — subst_word must skip ``$var`` / ``[cmd]`` substitution
    // because braces protect their contents in Tcl.  Without this
    // flag a braced word's ``$option`` would be resolved at
    // command-dispatch time rather than preserved literally, which
    // is wrong for e.g. ``proc foo args {body-with-$var}`` where
    // the body must stay unsubstituted until the proc runs.
    var wb: [MAX_WORDS]bool = undefined;
    // ``we[i] == true`` means word i was prefixed with ``{*}`` and
    // should be split as a Tcl list, expanding its elements into
    // individual arguments at the call site.
    var we: [MAX_WORDS]bool = undefined;

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

    while (pos < script_len) {
        // Publish the current command's position so any trap that
        // fires during dispatch includes a useful source snippet.
        diag.current_eval_ptr = script_ptr;
        diag.current_eval_len = script_len;
        diag.current_eval_pos = pos;

        const cmd = parse_command(src, pos, script_len, &wp, &wl, &wb, &we);
        pos = cmd.next;
        if (cmd.count == 0) continue;

        // Build the evaluated word list.  When no word has the
        // expansion flag set we use the fast path (fixed-size array
        // indexed directly).  When expansion is needed we copy into
        // a larger buffer so each ``{*}`` word can contribute
        // multiple elements without an allocation.
        var has_expand = false;
        var i: u32 = 0;
        while (i < cmd.count) : (i += 1) {
            if (we[i]) { has_expand = true; break; }
        }

        if (!has_expand) {
            // Fast path: no expansion.
            var word_objs: [MAX_WORDS]i32 = undefined;
            i = 0;
            while (i < cmd.count) : (i += 1) {
                if (wb[i]) {
                    // Braced word — preserve the content literally.
                    word_objs[i] = obj_new_string(@intCast(wp[i]), @intCast(wl[i]));
                } else {
                    word_objs[i] = subst_word(wp[i], wl[i]);
                }
            }
            result = eval_command(word_objs[0..cmd.count]);
        } else {
            // Slow path: at least one {*} expansion.
            var expanded: [MAX_EXPANDED_WORDS]i32 = undefined;
            var ecount: u32 = 0;
            i = 0;
            while (i < cmd.count) : (i += 1) {
                const word_obj: i32 = if (wb[i])
                    obj_new_string(@intCast(wp[i]), @intCast(wl[i]))
                else
                    subst_word(wp[i], wl[i]);

                if (we[i]) {
                    // Expansion: split word_obj as a Tcl list and
                    // insert each element as a separate argument.
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
            result = eval_command(expanded[0..ecount]);
        }
        if (has_signal()) return result;
    }
    return result;
}

// Exported: evaluate a Tcl script string.
pub export fn tcl_eval(script: i32) i32 {
    const s = obj_ensure_string(script);
    if (s.len == 0) return 0;
    return eval_script(s.ptr, s.len);
}
