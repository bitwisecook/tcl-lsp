// ``proc`` — procedure definition command.

const procs = @import("../tcl_procs.zig");
const reg   = @import("../tcl_cmd_registry.zig");

fn eval_proc(words: []const i32) i32 {
    if (words.len >= 4) {
        const interp = @import("../tcl_interp.zig");
        const qname = interp.qualify_name(words[1]);
        _ = procs.proc_register(qname, words[2], words[3]);
    }
    return 0;
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "proc", .handler = &eval_proc },
};
