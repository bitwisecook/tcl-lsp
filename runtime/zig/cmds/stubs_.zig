// ``auto_load``, ``auto_reset``, ``auto_mkindex``, ``auto_import``,
// ``auto_execok``, ``auto_qualify``, ``package`` — stub commands.
//
// Without the Tcl stdlib there is nothing to auto-load.  ``auto_load``
// returns 0 ("not found") so callers that check the return value see
// the expected "proc not in index" signal.  The other auto_* and
// package commands return an empty string.

const rt  = @import("../tcl_runtime.zig");
const reg = @import("../tcl_cmd_registry.zig");

fn eval_auto_load(words: []const i32) i32 {
    _ = words;
    return rt.obj_new_int(0);
}

fn eval_auto_noop(words: []const i32) i32 {
    _ = words;
    return rt.obj_new_string(0, 0);
}

fn eval_package(words: []const i32) i32 {
    _ = words;
    return 0;
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "auto_load",    .handler = &eval_auto_load },
    .{ .name = "auto_reset",   .handler = &eval_auto_noop },
    .{ .name = "auto_mkindex", .handler = &eval_auto_noop },
    .{ .name = "auto_import",  .handler = &eval_auto_noop },
    .{ .name = "auto_execok",  .handler = &eval_auto_noop },
    .{ .name = "auto_qualify", .handler = &eval_auto_noop },
    .{ .name = "package",      .handler = &eval_package },
};
