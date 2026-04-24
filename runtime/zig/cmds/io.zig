// ``puts``, ``append``, ``format`` — I/O and string output commands.
// ``scan`` moved to cmds/scan.zig for full multi-varname support.

const rt       = @import("../tcl_runtime.zig");
const frames   = @import("../interp/tcl_frames.zig");
const fmt_mod  = @import("../valtypes/tcl_format.zig");
const reg      = @import("../dispatch/tcl_cmd_registry.zig");

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

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "puts",   .handler = &eval_puts },
    .{ .name = "append", .handler = &eval_append },
    .{ .name = "format", .handler = &eval_format },
};
