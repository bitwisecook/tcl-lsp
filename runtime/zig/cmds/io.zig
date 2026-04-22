// ``puts``, ``append``, ``format``, ``scan`` — I/O and string output commands.

const rt       = @import("../tcl_runtime.zig");
const frames   = @import("../tcl_frames.zig");
const fmt_mod  = @import("../tcl_format.zig");
const fmt_stubs = @import("../tcl_fmt_stubs.zig");
const reg      = @import("../tcl_cmd_registry.zig");

const obj_new_int  = rt.obj_new_int;

fn eval_puts(words: []const i32) i32 {
    if (words.len >= 2) return rt.tcl_cmd_puts(words[words.len - 1]);
    return 0;
}

fn eval_append(words: []const i32) i32 {
    if (words.len >= 2) {
        var result = frames.var_resolve(words[1]);
        var wi: u32 = 2;
        while (wi < words.len) : (wi += 1) {
            result = rt.tcl_cmd_append(result, words[wi]);
        }
        _ = frames.var_set(words[1], result);
        return result;
    }
    return 0;
}

fn eval_format(words: []const i32) i32 {
    const fmt  = if (words.len >= 2) words[1] else 0;
    const a1   = if (words.len >= 3) words[2] else 0;
    const a2   = if (words.len >= 4) words[3] else 0;
    const a3   = if (words.len >= 5) words[4] else 0;
    return fmt_mod.tcl_cmd_format(fmt, a1, a2, a3);
}

fn eval_scan(words: []const i32) i32 {
    if (words.len >= 3) {
        const val = fmt_stubs.tcl_cmd_scan(words[1], words[2]);
        if (words.len >= 4) {
            _ = frames.var_set(words[3], val);
            return obj_new_int(1);
        }
        return val;
    }
    return obj_new_int(-1);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "puts",   .handler = &eval_puts },
    .{ .name = "append", .handler = &eval_append },
    .{ .name = "format", .handler = &eval_format },
    .{ .name = "scan",   .handler = &eval_scan },
};
