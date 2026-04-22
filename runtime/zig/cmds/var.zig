// ``set``, ``incr``, ``unset`` — variable read/write commands.

const rt     = @import("../tcl_runtime.zig");
const frames = @import("../tcl_frames.zig");
const reg    = @import("../tcl_cmd_registry.zig");

const obj_ensure_string = rt.obj_ensure_string;
const obj_new_string    = rt.obj_new_string;

fn eval_set(words: []const i32) i32 {
    if (words.len >= 3) { _ = frames.var_set(words[1], words[2]); return words[2]; }
    if (words.len >= 2) return frames.var_resolve(words[1]);
    return 0;
}

fn eval_incr(words: []const i32) i32 {
    if (words.len < 2) return 0;
    const amt_obj = if (words.len >= 3) words[2] else rt.obj_new_int(1);
    const cur = frames.var_resolve(words[1]);
    const result = rt.tcl_incr(cur, amt_obj);
    _ = frames.var_set(words[1], result);
    return result;
}

fn eval_unset(words: []const i32) i32 {
    var i: u32 = 1;
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        if (w.len >= 1 and wp[0] == '-') continue;
        _ = frames.var_set(words[i], 0);
    }
    return obj_new_string(0, 0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "set",   .handler = &eval_set },
    .{ .name = "incr",  .handler = &eval_incr },
    .{ .name = "unset", .handler = &eval_unset },
};
