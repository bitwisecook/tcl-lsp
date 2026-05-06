// ``proc`` — procedure definition command.

const procs = @import("../interp/tcl_procs.zig");
const result_mod = @import("../interp/tcl_result.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_proc(words: []const i32) result_mod.InterpResult {
    if (words.len >= 4) {
        const interp = @import("../interp/tcl_interp.zig");
        const qname = interp.qualify_name(words[1]);
        _ = procs.proc_register(qname, words[2], words[3]);
    }
    return result_mod.from_globals(0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "proc", .arity_min = 3, .arity_max = 3, .handler = &eval_proc },
};
