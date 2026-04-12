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
            count += 1;
            p = r.end;
        } else if (src[p] == '"') {
            const r = parse_quoted(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            count += 1;
            p = r.end;
        } else {
            const r = parse_bare(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            count += 1;
            p = r.end;
        }
    }
    return .{ .count = count, .next = p };
}

// -- Variable substitution --

fn subst_word(wptr: u32, wlen: u32) i32 {
    if (wlen == 0) return obj_new_string(0, 0);
    const src: [*]const u8 = @ptrFromInt(wptr);
    var has_dollar = false;
    var has_bracket = false;
    for (0..wlen) |i| {
        if (src[i] == '$') has_dollar = true;
        if (src[i] == '[') has_bracket = true;
    }
    if (!has_dollar and !has_bracket) {
        return obj_new_string(@intCast(wptr), @intCast(wlen));
    }
    const buf = alloc(wlen * 4 + 64);
    var out: u32 = 0;
    var i: u32 = 0;
    while (i < wlen) {
        if (src[i] == '$' and i + 1 < wlen) {
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
        } else if (src[i] == '[') {
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
        } else if (src[i] == '\\' and i + 1 < wlen) {
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
    if (str_eq(cmd, cmd_s.len, "puts")) { if (words.len >= 2) return rt.puts(words[words.len - 1]); return 0; }
    if (str_eq(cmd, cmd_s.len, "expr")) {
        if (words.len >= 2) { const es = obj_ensure_string(words[1]); return obj_new_int(eval_expr_str(es.ptr, es.len)); }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "return")) {
        rt.return_flag.* = 1;
        rt.return_val.* = if (words.len >= 2) words[1] else 0;
        return rt.return_val.*;
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
    if (str_eq(cmd, cmd_s.len, "error")) { if (words.len >= 2) rt.@"error"(words[1]); return 0; }
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
            const result = rt.append(cur, words[2]);
            _ = frames.var_set(words[1], result);
            return result;
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "llength")) { if (words.len >= 2) return rt.list_length(words[1]); return 0; }
    if (str_eq(cmd, cmd_s.len, "lindex")) { if (words.len >= 3) return rt.list_index(words[1], words[2]); return 0; }
    if (str_eq(cmd, cmd_s.len, "lappend")) {
        if (words.len >= 3) {
            const cur = frames.var_resolve(words[1]);
            const result = rt.lappend(cur, words[2]);
            _ = frames.var_set(words[1], result);
            return result;
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "string")) return eval_string_cmd(words);
    if (str_eq(cmd, cmd_s.len, "dict")) return eval_dict_cmd(words);
    if (str_eq(cmd, cmd_s.len, "info")) {
        if (words.len >= 3) return info.info_dispatch(words[1], words[2]);
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "split")) {
        if (words.len >= 3) return rt.split(words[1], words[2]);
        if (words.len >= 2) return rt.split(words[1], obj_new_string(0, 0));
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "join")) {
        if (words.len >= 3) return rt.join(words[1], words[2]);
        if (words.len >= 2) {
            // Default separator is a space
            const sp = alloc(1);
            const d: [*]u8 = @ptrFromInt(sp);
            d[0] = ' ';
            return rt.join(words[1], obj_new_string(@intCast(sp), 1));
        }
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "list")) {
        if (words.len >= 3) return rt.tcl_list(words[1], words[2]);
        if (words.len >= 2) return words[1];
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "concat")) {
        if (words.len >= 3) return rt.concat(words[1], words[2]);
        if (words.len >= 2) return words[1];
        return obj_new_string(0, 0);
    }
    if (str_eq(cmd, cmd_s.len, "lsort")) { if (words.len >= 2) return rt.list_sort(words[words.len - 1]); return obj_new_string(0, 0); }
    if (str_eq(cmd, cmd_s.len, "lsearch")) { if (words.len >= 3) return rt.list_search(words[1], words[2]); return obj_new_int(-1); }
    if (str_eq(cmd, cmd_s.len, "lrange")) { if (words.len >= 4) return rt.list_range(words[1], words[2], words[3]); return obj_new_string(0, 0); }
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
        rt.@"error"(words[0]);
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

    while (pos < script_len) {
        const cmd = parse_command(src, pos, script_len, &wp, &wl);
        pos = cmd.next;
        if (cmd.count == 0) continue;

        var word_objs: [MAX_WORDS]i32 = undefined;
        var i: u32 = 0;
        while (i < cmd.count) : (i += 1) {
            word_objs[i] = subst_word(wp[i], wl[i]);
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
