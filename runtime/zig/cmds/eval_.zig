// ``eval``, ``uplevel`` — script evaluation commands.

const rt  = @import("../tcl_runtime.zig");
const reg = @import("../tcl_cmd_registry.zig");

const alloc             = rt.alloc;
const memcpy            = rt.memcpy;
const obj_ensure_string = rt.obj_ensure_string;

fn eval_eval(words: []const i32) i32 {
    const interp = @import("../tcl_interp.zig");
    if (words.len == 2) {
        const s = obj_ensure_string(words[1]);
        return interp.eval_script(s.ptr, s.len);
    }
    if (words.len >= 3) {
        var total: u32 = 0;
        var k: u32 = 1;
        while (k < words.len) : (k += 1) {
            total += @as(u32, @intCast(obj_ensure_string(words[k]).len)) + 1;
        }
        if (total == 0) return 0;
        const buf = alloc(total);
        var off: u32 = 0;
        k = 1;
        while (k < words.len) : (k += 1) {
            const s = obj_ensure_string(words[k]);
            if (s.len > 0) {
                memcpy(buf + off, s.ptr, s.len);
                off += s.len;
            }
            if (k + 1 < words.len) {
                const d: [*]u8 = @ptrFromInt(buf + off);
                d[0] = ' ';
                off += 1;
            }
        }
        return interp.eval_script(buf, off);
    }
    return 0;
}

fn eval_uplevel(words: []const i32) i32 {
    const interp = @import("../tcl_interp.zig");
    return interp.eval_uplevel(words);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "eval",    .handler = &eval_eval },
    .{ .name = "uplevel", .handler = &eval_uplevel },
};
