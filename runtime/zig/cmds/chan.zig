// ``encoding``, ``fconfigure`` — channel/encoding commands.

const rt  = @import("../tcl_runtime.zig");
const enc = @import("../tcl_encoding.zig");
const chan = @import("../tcl_chan.zig");
const reg = @import("../tcl_cmd_registry.zig");

const alloc          = rt.alloc;
const obj_new_string = rt.obj_new_string;

fn eval_encoding(words: []const i32) i32 {
    const sub  = if (words.len >= 2) words[1] else 0;
    const arg1 = if (words.len >= 3) words[2] else 0;
    const arg2 = if (words.len >= 4) words[3] else 0;
    return enc.tcl_cmd_encoding(sub, arg1, arg2);
}

fn eval_fconfigure(words: []const i32) i32 {
    if (words.len < 2) return chan.tcl_cmd_fconfigure(0, 0);
    const fd = words[1];
    if (words.len < 3) return chan.tcl_cmd_fconfigure(fd, 0);
    var acc = words[2];
    var i: u32 = 3;
    while (i < words.len) : (i += 1) {
        const sp_ptr: u32 = alloc(1);
        const d: [*]u8 = @ptrFromInt(sp_ptr);
        d[0] = ' ';
        const sep = obj_new_string(@intCast(sp_ptr), 1);
        acc = rt.tcl_cmd_concat(acc, sep);
        acc = rt.tcl_cmd_concat(acc, words[i]);
    }
    return chan.tcl_cmd_fconfigure(fd, acc);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "encoding",   .handler = &eval_encoding },
    .{ .name = "fconfigure", .handler = &eval_fconfigure },
};
