// ``subst``, ``expr`` — substitution and expression evaluation commands.

const rt        = @import("../tcl_runtime.zig");
const tcl_subst = @import("../parse/tcl_subst.zig");
const reg       = @import("../dispatch/tcl_cmd_registry.zig");

const str_eq            = @import("../valtypes/tcl_chars.zig").str_eq;
const obj_new_string    = rt.obj_new_string;
const obj_new_int       = rt.obj_new_int;
const obj_ensure_string = rt.obj_ensure_string;

fn eval_subst(words: []const i32) i32 {
    var do_vars = true;
    var do_cmds = true;
    var do_bs   = true;
    var wi: u32 = 1;
    while (wi < words.len) : (wi += 1) {
        const a  = obj_ensure_string(words[wi]);
        const ap: [*]const u8 = @ptrFromInt(a.ptr);
        if (str_eq(ap, a.len, "-nobackslashes")) {
            do_bs = false;
        } else if (str_eq(ap, a.len, "-nocommands")) {
            do_cmds = false;
        } else if (str_eq(ap, a.len, "-novariables")) {
            do_vars = false;
        } else {
            break;
        }
    }
    if (wi >= words.len) return obj_new_string(0, 0);
    const s = obj_ensure_string(words[wi]);
    return tcl_subst.subst_flagged(s.ptr, s.len, do_vars, do_cmds, do_bs);
}

fn eval_expr(words: []const i32) i32 {
    if (words.len >= 2) {
        const interp = @import("../interp/tcl_interp.zig");
        const es = obj_ensure_string(words[1]);
        return obj_new_int(interp.eval_expr_str(es.ptr, es.len));
    }
    return 0;
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "subst", .handler = &eval_subst },
    .{ .name = "expr",  .handler = &eval_expr },
};
