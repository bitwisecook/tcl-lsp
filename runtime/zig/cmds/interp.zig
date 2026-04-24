// ``rename``, ``interp`` — interpreter management commands.

const interp_impl = @import("../commands/tcl_cmd_interp.zig");
const reg         = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_rename(words: []const i32) i32 {
    return interp_impl.eval_rename(words);
}

fn eval_interp(words: []const i32) i32 {
    const interp = @import("../interp/tcl_interp.zig");
    return interp.eval_interp(words);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "rename", .handler = &eval_rename },
    .{ .name = "interp", .handler = &eval_interp },
};
