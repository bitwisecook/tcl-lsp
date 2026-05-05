// ``regexp`` — regular expression match / capture command.
//
// ``regsub`` lives in its own module (``regsub.zig``) so the BUILTINS
// table only sees it once.  Both modules delegate to the canonical
// impl in ``valtypes/tcl_regex.zig``.

const regex_mod = @import("../valtypes/tcl_regex.zig");
const result_mod = @import("../interp/tcl_result.zig");
const reg       = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_regexp(words: []const i32) result_mod.InterpResult {
    return result_mod.from_globals(regex_mod.eval_regexp_cmd(words));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "regexp", .arity_min = 1, .arity_max = null, .handler = &eval_regexp },
};
