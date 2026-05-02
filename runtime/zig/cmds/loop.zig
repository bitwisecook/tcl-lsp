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

fn eval_switch(words: []const i32) i32 {
    const interp = @import("../interp/tcl_interp.zig");
    return interp.eval_switch(words);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "if", .arity_min = 2, .arity_max = null, .handler = &eval_if },
    .{ .name = "while", .arity_min = 2, .arity_max = 2, .handler = &eval_while },
    .{ .name = "for", .arity_min = 4, .arity_max = 4, .handler = &eval_for },
    .{ .name = "foreach", .arity_min = 3, .arity_max = null, .handler = &eval_foreach },
    // ``switch`` is also handled inline by the codegen (IRSwitch),
    // so this entry only fires when the interpreter evaluates a
    // dynamic ``switch`` string (e.g. test bodies that build
    // pattern/body lists at runtime, or a ``switch`` reached via
    // ``eval``).  Removed from the ``STUB_TRAP`` table now.
    .{ .name = "switch", .arity_min = 2, .arity_max = null, .handler = &eval_switch },
};
