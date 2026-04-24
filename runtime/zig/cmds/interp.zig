// ``rename``, ``interp`` — interpreter management commands.

const interp_impl = @import("./tcl_cmd_interp.zig");
const reg         = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_rename(words: []const i32) i32 {
    return interp_impl.eval_rename(words);
}

fn eval_interp(words: []const i32) i32 {
    const interp = @import("../interp/tcl_interp.zig");
    return interp.eval_interp(words);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "rename", .arity_min = 2, .arity_max = 2, .handler = &eval_rename },
    .{ .name = "interp", .arity_min = 1, .arity_max = null, .handler = &eval_interp },
};
