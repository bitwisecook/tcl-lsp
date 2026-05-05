// ``regsub`` — regular expression substitution command.

const regex_mod = @import("../valtypes/tcl_regex.zig");
const result_mod = @import("../interp/tcl_result.zig");
const reg       = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_regsub(words: []const i32) result_mod.InterpResult {
    return result_mod.from_globals(regex_mod.eval_regsub_cmd(words));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "regsub", .arity_min = 3, .arity_max = 4, .handler = &eval_regsub },
};
