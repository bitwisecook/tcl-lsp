// Tcl substitution engine — $var, [cmd], and \escape expansion.
//
// Extracted from tcl_interp.zig so that both the interpreter's word
// expander (subst_word) and the ``subst`` command handler can share
// one canonical implementation without importing the entire interpreter.

const rt     = @import("../tcl_runtime.zig");
const frames = @import("../interp/tcl_frames.zig");
const obj_mod = @import("../value/tcl_obj.zig");

const alloc            = rt.alloc;
const memcpy           = rt.memcpy;
const read_i32         = obj_mod.read_i32;
const write_i32        = obj_mod.write_i32;
const obj_new_string   = rt.obj_new_string;
const obj_ensure_string = rt.obj_ensure_string;

/// Expand ``$var`` / ``[cmd]`` / ``\x`` in a raw byte span, with each
/// substitution kind independently enabled.  ``do_vars`` / ``do_cmds``
/// / ``do_bs`` correspond to the ``-novariables`` / ``-nocommands``
/// / ``-nobackslashes`` flags of the ``subst`` command.
///
/// Uses a single-pass "pieces" strategy: each resolved substitution is
/// recorded as a (ptr, len) pair in a bump-allocated scratch buffer;
/// all pieces are concatenated in one final memcpy.  This avoids
/// double-evaluating side-effecting commands (``[incr x]`` must fire
/// exactly once) and avoids the resize hazard that two-pass approaches
/// suffer when a variable changes between the sizing and copying passes.
pub fn subst_flagged(
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
            const interp = @import("../interp/tcl_interp.zig");
            const result = interp.eval_script(wptr + cs, ce - cs);
            if (result != 0) {
                const sv = obj_ensure_string(result);
                push_piece(pieces_buf, &n_pieces, &total_out, sv.ptr, sv.len);
            }
            lit_start = i;
        } else if (do_bs and src[i] == '\\' and i + 1 < wlen) {
            flush_lit(pieces_buf, &n_pieces, &total_out, lit_start, &lit_run, wptr);
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
