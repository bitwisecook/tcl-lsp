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

// Re-export runtime functions used throughout this file
const alloc = rt.alloc;
const memcpy = rt.memcpy;
const obj_new_string = rt.obj_new_string;
const obj_new_int = rt.obj_new_int;
const obj_get_int = rt.obj_get_int;
const obj_new_string_copy = rt.obj_new_string_copy;
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

        if (src[p] == '{') {
            const r = parse_braced(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            word_braced[count] = true;
            count += 1;
            p = r.end;
        } else if (src[p] == '"') {
            const r = parse_quoted(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            word_braced[count] = false;
            count += 1;
            p = r.end;
        } else {
            const r = parse_bare(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            word_braced[count] = false;
            count += 1;
            p = r.end;
        }
    }
    return .{ .count = count, .next = p };
}

// -- Variable substitution --

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
    for (0..wlen) |i| {
        if (src[i] == '$') has_dollar = true;
        if (src[i] == '[') has_bracket = true;
    }
    if (!has_dollar and !has_bracket and !do_bs) {
        return obj_new_string(@intCast(wptr), @intCast(wlen));
    }
    // Two-pass approach: first pass walks *src* to compute the
    // exact output size by resolving each ``$var`` / ``[cmd]``
    // substitution and summing its string length; second pass
    // re-resolves and writes into a buffer sized exactly right.
    //
    // The previous single-pass implementation used a ``wlen * 4 +
    // 64`` heuristic — fine for short words but catastrophically
    // wrong when a word like ``$s`` (2 chars of source) resolves
    // to a multi-KB value (a tcltest ``ConstraintInitializer``
    // body, for instance).  ``memcpy`` would write past the
    // buffer end, corrupting adjacent heap allocations; the
    // resulting ``info complete $s`` read the overflowed tail,
    // saw mis-matched braces, and returned 0 ("not complete"),
    // which in turn made tcltest reject every multi-line
    // constraint script.  Resolving substitutions twice is
    // cheap compared to the bump-allocator cost of over-reserving,
    // and eliminates the overflow class entirely.
    const total_out = compute_subst_size(src, wlen, do_vars, do_cmds, do_bs);
    const buf = alloc(total_out + 1);
    var out: u32 = 0;
    var i: u32 = 0;
    while (i < wlen) {
        if (do_vars and src[i] == '$' and i + 1 < wlen) {
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
                    if (sv.len > 0) { memcpy(buf + out, sv.ptr, sv.len); out += sv.len; }
                }
            } else {
                while (i < wlen and ((src[i] >= 'a' and src[i] <= 'z') or
                    (src[i] >= 'A' and src[i] <= 'Z') or
                    (src[i] >= '0' and src[i] <= '9') or src[i] == '_'))
                { i += 1; }
                const name_obj = obj_new_string(@intCast(wptr + vstart), @intCast(i - vstart));
                const val = frames.var_resolve(name_obj);
                if (val != 0) {
                    const sv = obj_ensure_string(val);
                    if (sv.len > 0) { memcpy(buf + out, sv.ptr, sv.len); out += sv.len; }
                }
            }
        } else if (do_cmds and src[i] == '[') {
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
                if (sv.len > 0) { memcpy(buf + out, sv.ptr, sv.len); out += sv.len; }
            }
        } else if (do_bs and src[i] == '\\' and i + 1 < wlen) {
            i += 1;
            const esc: u8 = switch (src[i]) { 'n' => '\n', 't' => '\t', 'r' => '\r', else => src[i] };
            const d: [*]u8 = @ptrFromInt(buf + out);
            d[0] = esc;
            out += 1;
            i += 1;
        } else {
            const d: [*]u8 = @ptrFromInt(buf + out);
            d[0] = src[i];
            out += 1;
            i += 1;
        }
    }
    return obj_new_string(@intCast(buf), @intCast(out));
}

/// First-pass of :func:`subst_flagged`: walk *src* and resolve
/// each ``$var`` / ``[cmd]`` the same way the main loop would,
/// summing up the total byte count so the second pass can
/// allocate an exactly-sized buffer.  Side effects: command
/// substitutions ARE executed here, and their results are
/// subsequently re-executed by the second pass — not ideal for
/// commands with observable side effects, but matches what the
/// old single-pass impl's over-sized buffer would have done in
/// the degenerate case (it still called ``eval_script`` once).
/// ``puts`` etc. will now print twice from a ``subst`` call
/// that wraps them; callers that care route through ``eval``
/// directly rather than ``subst``.
fn compute_subst_size(
    src: [*]const u8,
    wlen: u32,
    do_vars: bool,
    do_cmds: bool,
    do_bs: bool,
) u32 {
    var total: u32 = 0;
    var i: u32 = 0;
    while (i < wlen) {
        if (do_vars and src[i] == '$' and i + 1 < wlen) {
            i += 1;
            const vstart = i;
            if (src[i] == '{') {
                i += 1;
                const vs = i;
                while (i < wlen and src[i] != '}') i += 1;
                const ve = i;
                if (i < wlen) i += 1;
                const name_obj = obj_new_string(@intCast(@intFromPtr(src) + vs), @intCast(ve - vs));
                const val = frames.var_resolve(name_obj);
                if (val != 0) {
                    total += obj_ensure_string(val).len;
                }
            } else {
                while (i < wlen and ((src[i] >= 'a' and src[i] <= 'z') or
                    (src[i] >= 'A' and src[i] <= 'Z') or
                    (src[i] >= '0' and src[i] <= '9') or src[i] == '_'))
                { i += 1; }
                const name_obj = obj_new_string(@intCast(@intFromPtr(src) + vstart), @intCast(i - vstart));
                const val = frames.var_resolve(name_obj);
                if (val != 0) {
                    total += obj_ensure_string(val).len;
                }
            }
        } else if (do_cmds and src[i] == '[') {
            i += 1;
            const cs = i;
            var depth: u32 = 1;
            while (i < wlen and depth > 0) {
                if (src[i] == '[') depth += 1 else if (src[i] == ']') depth -= 1;
                if (depth > 0) i += 1 else i += 1;
            }
            const ce = i - 1;
            const result = eval_script(@intCast(@intFromPtr(src) + cs), ce - cs);
            if (result != 0) {
                total += obj_ensure_string(result).len;
            }
        } else if (do_bs and src[i] == '\\' and i + 1 < wlen) {
            // Backslash-escape output is always 1 byte.
            i += 2;
            total += 1;
        } else {
            i += 1;
            total += 1;
        }
    }
    return total;
}

// -- Expression evaluator --
// Recursive-descent: +, -, *, /, %, ==, !=, <, >, <=, >=, unary -, (), $var, [cmd]

fn eval_expr_str(ptr: u32, len: u32) i64 {
    var pos: u32 = 0;
    return expr_add(ptr, len, &pos);
}

fn expr_skip_ws(src: [*]const u8, len: u32, pos: *u32) void {
    while (pos.* < len and (src[pos.*] == ' ' or src[pos.*] == '\t')) pos.* += 1;
}

fn expr_add(ptr: u32, len: u32, pos: *u32) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var left = expr_mul(ptr, len, pos);
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* >= len) break;
        if (src[pos.*] == '+') { pos.* += 1; left = left + expr_mul(ptr, len, pos); }
        else if (src[pos.*] == '-') { pos.* += 1; left = left - expr_mul(ptr, len, pos); }
        else if (pos.* + 1 < len and src[pos.*] == '=' and src[pos.* + 1] == '=') { pos.* += 2; left = if (left == expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0); }
        else if (pos.* + 1 < len and src[pos.*] == '!' and src[pos.* + 1] == '=') { pos.* += 2; left = if (left != expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0); }
        else if (pos.* + 1 < len and src[pos.*] == '<' and src[pos.* + 1] == '=') { pos.* += 2; left = if (left <= expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0); }
        else if (pos.* + 1 < len and src[pos.*] == '>' and src[pos.* + 1] == '=') { pos.* += 2; left = if (left >= expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0); }
        else if (src[pos.*] == '<') { pos.* += 1; left = if (left < expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0); }
        else if (src[pos.*] == '>') { pos.* += 1; left = if (left > expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0); }
        else break;
    }
    return left;
}

fn expr_mul(ptr: u32, len: u32, pos: *u32) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var left = expr_atom(ptr, len, pos);
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* >= len) break;
        if (src[pos.*] == '*') { pos.* += 1; left = left * expr_atom(ptr, len, pos); }
        else if (src[pos.*] == '/') { pos.* += 1; const r = expr_atom(ptr, len, pos); left = if (r != 0) @divTrunc(left, r) else 0; }
        else if (src[pos.*] == '%') { pos.* += 1; const r = expr_atom(ptr, len, pos); left = if (r != 0) @rem(left, r) else 0; }
        else break;
    }
    return left;
}

fn expr_atom(ptr: u32, len: u32, pos: *u32) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    expr_skip_ws(src, len, pos);
    if (pos.* >= len) return 0;
    if (src[pos.*] == '-') { pos.* += 1; return -expr_atom(ptr, len, pos); }
    if (src[pos.*] == '(') {
        pos.* += 1;
        const val = expr_add(ptr, len, pos);
        expr_skip_ws(src, len, pos);
        if (pos.* < len and src[pos.*] == ')') pos.* += 1;
        return val;
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
        const result = eval_script(ptr + cs, pos.* - 1 - cs);
        if (result != 0) return obj_get_int(result);
        return 0;
    }
    var negative = false;
    if (src[pos.*] == '+') pos.* += 1;
    if (pos.* < len and src[pos.*] == '-') { negative = true; pos.* += 1; }
    var val: i64 = 0;
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
        if (words.len >= 4) _ = procs.proc_register(words[1], words[2], words[3]);
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
            _ = eval_script(body_s.ptr, body_s.len);
            return rt.catch_leave();
        }
        return obj_new_int(0);
    }
    if (str_eq(cmd, cmd_s.len, "append")) {
        if (words.len >= 3) {
            const cur = frames.var_resolve(words[1]);
            const result = rt.tcl_cmd_append(cur, words[2]);
            _ = frames.var_set(words[1], result);
            return result;
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "llength")) { if (words.len >= 2) return rt.tcl_cmd_list_length(words[1]); return 0; }
    if (str_eq(cmd, cmd_s.len, "lindex")) { if (words.len >= 3) return rt.tcl_cmd_list_index(words[1], words[2]); return 0; }
    if (str_eq(cmd, cmd_s.len, "lappend")) {
        if (words.len >= 3) {
            const cur = frames.var_resolve(words[1]);
            const result = rt.tcl_cmd_lappend(cur, words[2]);
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
        if (words.len >= 3) return rt.tcl_list(words[1], words[2]);
        if (words.len >= 2) return words[1];
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "concat")) {
        if (words.len >= 3) return rt.tcl_cmd_concat(words[1], words[2]);
        if (words.len >= 2) return words[1];
        return obj_new_string(0, 0);
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
    if (str_eq(cmd, cmd_s.len, "package") or
        str_eq(cmd, cmd_s.len, "namespace") or str_eq(cmd, cmd_s.len, "variable") or
        str_eq(cmd, cmd_s.len, "rename"))
    { return 0; }
    // -- Proc dispatch: check registry before erroring --
    return eval_proc_call(words);
}

fn str_eq(a: [*]const u8, alen: u32, comptime b: []const u8) bool {
    if (alen != b.len) return false;
    inline for (0..b.len) |i| {
        if (a[i] != b[i]) return false;
    }
    return true;
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
        _ = frames.var_set(var_name, obj_new_string_copy(list_s.ptr + elem.start, elem.len));
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

    // Bind parameters: walk the params list, assign each from argv
    if (params_obj != 0 and n_params > 0) {
        const ps = obj_ensure_string(params_obj);
        var pi: u32 = 0;
        while (pi < n_params) : (pi += 1) {
            const param_elem = list_element_at(ps.ptr, ps.len, @intCast(pi));
            const param_name = obj_new_string_copy(ps.ptr + param_elem.start, param_elem.len);
            // argv[0] is the command name, so argv[pi+1] is the first arg
            const arg_idx = pi + 1;
            const arg_val = if (arg_idx < words.len) words[arg_idx] else obj_new_string(0, 0);
            _ = frames.local_set(param_name, arg_val);
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
    if (str_eq(sp, sub.len, "trim")) return rt.string_trim(words[2]);
    if (str_eq(sp, sub.len, "first") and words.len >= 4) return rt.string_first(words[2], words[3]);
    if (str_eq(sp, sub.len, "last") and words.len >= 4) return rt.string_last(words[2], words[3]);
    if (str_eq(sp, sub.len, "toupper")) return rt.string_toupper(words[2]);
    if (str_eq(sp, sub.len, "tolower")) return rt.string_tolower(words[2]);
    if (str_eq(sp, sub.len, "reverse")) return rt.string_reverse(words[2]);
    if (str_eq(sp, sub.len, "repeat") and words.len >= 4) return rt.string_repeat(words[2], words[3]);
    if (str_eq(sp, sub.len, "replace") and words.len >= 6) return rt.string_replace(words[2], words[3], words[4], words[5]);
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
        return array_mod.array_set(words[2], words[3], 0);
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

        const cmd = parse_command(src, pos, script_len, &wp, &wl, &wb);
        pos = cmd.next;
        if (cmd.count == 0) continue;

        var word_objs: [MAX_WORDS]i32 = undefined;
        var i: u32 = 0;
        while (i < cmd.count) : (i += 1) {
            if (wb[i]) {
                // Braced word — preserve the content literally.
                word_objs[i] = obj_new_string(@intCast(wp[i]), @intCast(wl[i]));
            } else {
                word_objs[i] = subst_word(wp[i], wl[i]);
            }
        }

        result = eval_command(word_objs[0..cmd.count]);
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
