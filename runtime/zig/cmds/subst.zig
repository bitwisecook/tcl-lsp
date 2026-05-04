// ``subst``, ``expr`` — substitution and expression evaluation commands.

const rt        = @import("../tcl_runtime.zig");
const tcl_subst = @import("../parse/tcl_subst.zig");
const reg       = @import("../dispatch/tcl_cmd_registry.zig");

const str_eq            = @import("../valtypes/tcl_chars.zig").str_eq;
const obj_new_string    = rt.obj_new_string;
const obj_new_int       = rt.obj_new_int;
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

fn eval_subst(words: []const i32) i32 {
    var do_vars = true;
    var do_cmds = true;
    var do_bs   = true;
    var wi: u32 = 1;
    while (wi < words.len) : (wi += 1) {
        const a  = obj_ensure_string(words[wi]);
        if (a.ptr == 0 or a.len == 0) break;
        const ap: [*]const u8 = @ptrFromInt(a.ptr);
        if (a.len < 2 or ap[0] != '-') break;
        // Disambiguate by prefix: ``-no`` alone is ambiguous (matches
        // all three), ``-nob`` → ``-nobackslashes``, ``-noc`` →
        // ``-nocommands``, ``-nov`` → ``-novariables``.  Real Tcl
        // raises an "ambiguous option" error for the bare ``-no``;
        // for now we treat anything not uniquely matching as a
        // non-option and break out of the flag loop.
        if (is_prefix(ap, a.len, "-nobackslashes") and
            !is_prefix(ap, a.len, "-nocommands") and
            !is_prefix(ap, a.len, "-novariables"))
        {
            do_bs = false;
        } else if (is_prefix(ap, a.len, "-nocommands") and
                   !is_prefix(ap, a.len, "-nobackslashes") and
                   !is_prefix(ap, a.len, "-novariables"))
        {
            do_cmds = false;
        } else if (is_prefix(ap, a.len, "-novariables") and
                   !is_prefix(ap, a.len, "-nobackslashes") and
                   !is_prefix(ap, a.len, "-nocommands"))
        {
            do_vars = false;
        } else {
            break;
        }
    }
    if (wi >= words.len) return obj_new_string(0, 0);
    const s = obj_ensure_string(words[wi]);
    // ``from_subst_cmd=true`` activates Tcl_SubstObj exception handling:
    // ``[break]`` / ``[continue]`` / ``[return]`` inside ``[...]`` are
    // folded into the subst result rather than propagating up.
    return tcl_subst.subst_flagged_full(s.ptr, s.len, do_vars, do_cmds, do_bs, true);
}

fn eval_expr(words: []const i32) i32 {
    if (words.len >= 2) {
        // Use the bignum-aware evaluator so the runtime ``expr``
        // command produces the same result as the AOT-compiled
        // ``expr {...}`` body.  Returns a TclObj directly — no i64
        // round-trip — so TYPE_BIGNUM results survive intact for
        // downstream callers (``puts`` / ``set`` / mathop).  Closes
        // the gap where ``[expr {1 << 70}]`` returned ``1`` (the
        // legacy ``eval_expr_str`` recursive descent didn't even
        // recognise ``<<``).
        const expr_eval = @import("../interp/tcl_expr_eval.zig");
        const es = obj_ensure_string(words[1]);
        return expr_eval.eval(es.ptr, es.len);
    }
    return 0;
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "subst", .arity_min = 1, .arity_max = null, .handler = &eval_subst },
    .{ .name = "expr", .arity_min = 1, .arity_max = null, .handler = &eval_expr },
};
