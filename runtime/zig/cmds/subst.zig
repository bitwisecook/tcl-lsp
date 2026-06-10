// ``subst``, ``expr`` — substitution and expression evaluation commands.

const rt = @import("../tcl_runtime.zig");
const result_mod = @import("../interp/tcl_result.zig");
const tcl_subst = @import("../parse/tcl_subst.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

const str_eq = @import("../valtypes/tcl_chars.zig").str_eq;
const obj_new_string = rt.obj_new_string;
const obj_new_int = rt.obj_new_int;
const obj_ensure_string = rt.obj_ensure_string;

/// Return ``true`` iff ``arg`` is a non-empty prefix of ``opt``.
/// Mirrors Tcl's ``Tcl_GetIndexFromObj`` prefix-matching for subst's
/// option list — ``-nov`` matches ``-novariables``, ``-noc`` matches
/// ``-nocommands``, etc. (subst-7.7).
fn is_prefix(arg: [*]const u8, arg_len: u32, opt: []const u8) bool {
    if (arg_len == 0 or arg_len > opt.len) return false;
    for (0..arg_len) |i| {
        if (arg[i] != opt[i]) return false;
    }
    return true;
}

fn eval_subst(words: []const i32) result_mod.InterpResult {
    var do_vars = true;
    var do_cmds = true;
    var do_bs = true;
    var wi: u32 = 1;
    while (wi < words.len) : (wi += 1) {
        const a = obj_ensure_string(words[wi]);
        if (a.ptr == 0 or a.len == 0) break;
        const ap: [*]const u8 = @ptrFromInt(a.ptr);
        if (a.len < 2 or ap[0] != '-') break;
        // Disambiguate by prefix: ``-no`` alone is ambiguous (matches
        // all three), ``-nob`` → ``-nobackslashes``, ``-noc`` →
        // ``-nocommands``, ``-nov`` → ``-novariables``.  Reference
        // Tcl raises ``ambiguous option "<prefix>"`` for an
        // ambiguous prefix — surface that error so user mistakes
        // aren't silently treated as "not an option" (subst-7.x).
        const m_bs = is_prefix(ap, a.len, "-nobackslashes");
        const m_c = is_prefix(ap, a.len, "-nocommands");
        const m_v = is_prefix(ap, a.len, "-novariables");
        const matches = @as(u32, @intFromBool(m_bs)) +
            @as(u32, @intFromBool(m_c)) +
            @as(u32, @intFromBool(m_v));
        if (matches == 0) {
            // Not a known option prefix — bail out of option scan;
            // the word is the script.
            break;
        }
        if (matches > 1) {
            // Ambiguous prefix.  Build ``ambiguous option "<arg>":
            // must be -nobackslashes, -nocommands, or -novariables``
            // and route through the standard error path.  Codex
            // review.
            const catch_mod = @import("../interp/tcl_catch.zig");
            const obj_mod = @import("../valtypes/tcl_obj.zig");
            const prefix: []const u8 = "ambiguous option \"";
            const middle: []const u8 = "\": must be -nobackslashes, -nocommands, or -novariables";
            const total: u32 = @intCast(prefix.len + a.len + middle.len);
            const buf = obj_mod.alloc(total);
            if (buf == 0) return result_mod.from_globals(0);
            const dst: [*]u8 = @ptrFromInt(buf);
            var off: u32 = 0;
            for (prefix) |c| {
                dst[off] = c;
                off += 1;
            }
            for (0..a.len) |i| {
                dst[off] = ap[i];
                off += 1;
            }
            for (middle) |c| {
                dst[off] = c;
                off += 1;
            }
            const msg = obj_mod.obj_new_string_take(buf, total, total);
            catch_mod.tcl_cmd_error(msg);
            return result_mod.from_globals(0);
        }
        if (m_bs) do_bs = false else if (m_c) do_cmds = false else do_vars = false;
    }
    if (wi >= words.len) return result_mod.from_globals(obj_new_string(0, 0));
    const s = obj_ensure_string(words[wi]);
    // ``from_subst_cmd=true`` activates Tcl_SubstObj exception handling:
    // ``[break]`` / ``[continue]`` / ``[return]`` inside ``[...]`` are
    // folded into the subst result rather than propagating up.
    return result_mod.from_globals(tcl_subst.subst_flagged_full(s.ptr, s.len, do_vars, do_cmds, do_bs, true));
}

fn eval_expr(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        // ``expr`` with no arguments raises ``wrong # args: should be
        // "expr arg ?arg ...?"`` (expr-old-26.20).  The legacy
        // silent-zero fallback let ``[catch expr]`` return 0 with an
        // empty message instead of the canonical arity error.
        const catch_mod = @import("../interp/tcl_catch.zig");
        const obj_mod = @import("../valtypes/tcl_obj.zig");
        const msg_text: []const u8 =
            "wrong # args: should be \"expr arg ?arg ...?\"";
        const buf = obj_mod.alloc(@intCast(msg_text.len));
        if (buf == 0) {
            catch_mod.tcl_cmd_error(0);
            return result_mod.from_globals(0);
        }
        const d: [*]u8 = @ptrFromInt(buf);
        for (msg_text, 0..) |b, k| d[k] = b;
        const msg = obj_mod.obj_new_string_take(
            buf,
            @intCast(msg_text.len),
            @intCast(msg_text.len),
        );
        catch_mod.tcl_cmd_error(msg);
        return result_mod.from_globals(0);
    }
    // Tcl ``expr`` concatenates all arguments with spaces and
    // evaluates the joined string as a single expression.
    // ``expr 20 - 5 +10 -7`` → ``20 - 5 +10 -7`` → 18.  The
    // single-arg case takes the simple fast path; anything else
    // builds a join buffer first.
    const expr_eval = @import("../interp/tcl_expr_eval.zig");
    if (words.len == 2) {
        const es = obj_ensure_string(words[1]);
        return result_mod.from_globals(expr_eval.eval_top(es.ptr, es.len));
    }
    var total: u32 = 0;
    var wi: u32 = 1;
    while (wi < words.len) : (wi += 1) {
        const ws = obj_ensure_string(words[wi]);
        total += ws.len;
        if (wi + 1 < words.len) total += 1; // separator space
    }
    if (total == 0) return result_mod.from_globals(expr_eval.eval(0, 0));
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        // Out-of-memory while building the join buffer.  Surface as
        // a generic error and bail rather than dereferencing a null
        // address inside ``rt.memcpy`` below.
        @import("../interp/tcl_catch.zig").tcl_cmd_error(obj_new_string(0, 0));
        return result_mod.from_globals(0);
    }
    var off: u32 = 0;
    wi = 1;
    while (wi < words.len) : (wi += 1) {
        const ws = obj_ensure_string(words[wi]);
        rt.memcpy(buf + off, ws.ptr, ws.len);
        off += ws.len;
        if (wi + 1 < words.len) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
    }
    const r = expr_eval.eval_top(buf, total);
    obj_mod.free_sized(buf, total);
    return result_mod.from_globals(r);
}

/// fpclassify — IEEE-754 float classification command (TIP 519).
/// ``fpclassify floatValue`` returns one of ``zero`` / ``subnormal`` /
/// ``normal`` / ``infinite`` / ``nan``.
fn cmd_fpclassify(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const obj_mod = @import("../valtypes/tcl_obj.zig");
        const msg_text: []const u8 = "wrong # args: should be \"fpclassify floatValue\"";
        const buf = obj_mod.alloc(@intCast(msg_text.len));
        if (buf != 0) {
            const d: [*]u8 = @ptrFromInt(buf);
            for (msg_text, 0..) |b, k| d[k] = b;
            const msg = obj_mod.obj_new_string_take(buf, @intCast(msg_text.len), @intCast(msg_text.len));
            catch_mod.tcl_cmd_error(msg);
        }
        return result_mod.from_globals(0);
    }
    const arith = @import("../valtypes/tcl_arith.zig");
    return result_mod.from_globals(arith.tcl_math_fpclassify(words[1]));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "subst", .arity_min = 1, .arity_max = null, .handler = &eval_subst },
    .{ .name = "expr", .arity_min = 1, .arity_max = null, .handler = &eval_expr },
    .{ .name = "fpclassify", .arity_min = 1, .arity_max = 1, .handler = &cmd_fpclassify },
};
