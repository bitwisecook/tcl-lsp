// ``regsub`` — regular expression substitution command.

const regex_mod = @import("../valtypes/tcl_regex.zig");
const reg       = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_regsub(words: []const i32) i32 {
    return regex_mod.eval_regsub_cmd(words);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "regsub", .handler = &eval_regsub },
};
