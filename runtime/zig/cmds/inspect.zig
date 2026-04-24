// ``info``, ``trace`` — introspection commands.

const rt        = @import("../tcl_runtime.zig");
const info      = @import("../dispatch/tcl_cmd_info.zig");
const trace_mod = @import("../interp/tcl_trace.zig");
const reg       = @import("../dispatch/tcl_cmd_registry.zig");

const str_eq            = @import("../value/tcl_chars.zig").str_eq;
const obj_new_string    = rt.obj_new_string;
const obj_ensure_string = rt.obj_ensure_string;

fn eval_info(words: []const i32) i32 {
    if (words.len >= 2) {
        const sub_s = obj_ensure_string(words[1]);
        const sub_p: [*]const u8 = @ptrFromInt(sub_s.ptr);
        if (str_eq(sub_p, sub_s.len, "default")) {
            if (words.len < 5) return obj_new_string(0, 0);
            return info.info_default(words[2], words[3], words[4]);
        }
        if (str_eq(sub_p, sub_s.len, "commands") and words.len == 2) {
            return info.info_commands(0);
        }
        if (str_eq(sub_p, sub_s.len, "procs") and words.len == 2) {
            return info.info_procs(0);
        }
        if (words.len == 2) return info.info_dispatch(words[1], 0);
    }
    if (words.len >= 3) return info.info_dispatch(words[1], words[2]);
    return obj_new_string(0, 0);
}

fn eval_trace(words: []const i32) i32 {
    const sub     = if (words.len >= 2) words[1] else 0;
    const arg_obj = if (words.len >= 3) words[2] else 0;
    return trace_mod.tcl_cmd_trace_cmd(sub, arg_obj);
}

fn eval_pid(words: []const i32) i32 {
    _ = words;
    return rt.obj_new_int(12345);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "info",  .handler = &eval_info },
    .{ .name = "trace", .handler = &eval_trace },
    .{ .name = "pid",   .handler = &eval_pid },
};
