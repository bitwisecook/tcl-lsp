// ``if``, ``while``, ``for``, ``foreach`` — loop and conditional commands.
// All implementations live in tcl_interp.zig (pub fn); this file registers them.

const reg = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_if(words: []const i32) i32 {
    const interp = @import("../interp/tcl_interp.zig");
    return interp.eval_if(words);
}

fn eval_while(words: []const i32) i32 {
    const interp = @import("../interp/tcl_interp.zig");
    return interp.eval_while(words);
}

fn eval_for(words: []const i32) i32 {
    const interp = @import("../interp/tcl_interp.zig");
    return interp.eval_for(words);
}

fn eval_foreach(words: []const i32) i32 {
    const interp = @import("../interp/tcl_interp.zig");
    return interp.eval_foreach(words);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "if",      .handler = &eval_if },
    .{ .name = "while",   .handler = &eval_while },
    .{ .name = "for",     .handler = &eval_for },
    .{ .name = "foreach", .handler = &eval_foreach },
};
