// ``eval``, ``uplevel`` — script evaluation commands.

const rt = @import("../tcl_runtime.zig");
const result_mod = @import("../interp/tcl_result.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

const alloc = rt.alloc;
const memcpy = rt.memcpy;
const obj_ensure_string = rt.obj_ensure_string;

fn eval_eval(words: []const i32) result_mod.InterpResult {
    const interp = @import("../interp/tcl_interp.zig");
    if (words.len < 2) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"eval arg ?arg ...?\"");
        return result_mod.from_globals(0);
    }
    // ``eval`` is a script-eval dispatch — counts as one level
    // (matches C Tcl's ``TclNREvalObjEx`` ``numLevels++``).
    if (!interp.recursion_check_enter()) return result_mod.from_globals(0);
    defer interp.recursion_check_leave();
    if (words.len == 2) {
        const s = obj_ensure_string(words[1]);
        return result_mod.from_globals(interp.eval_script(s.ptr, s.len));
    }
    if (words.len >= 3) {
        var total: u32 = 0;
        var k: u32 = 1;
        while (k < words.len) : (k += 1) {
            total += @as(u32, @intCast(obj_ensure_string(words[k]).len)) + 1;
        }
        if (total == 0) return result_mod.from_globals(0);
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
        // Reclaim the joined-script scratch after eval_script
        // returns — the bytes only need to live during evaluation
        // (parser copies tokens into its own structures).  Without
        // this, every multi-arg ``eval`` leaks ``total`` bytes,
        // which io.test's tcltest helpers exercised on every test
        // body and contributed to the leak driving #303.
        const result = interp.eval_script(buf, off);
        obj.free_sized(buf, total);
        return result_mod.from_globals(result);
    }
    return result_mod.from_globals(0);
}

fn eval_uplevel(words: []const i32) result_mod.InterpResult {
    const interp = @import("../interp/tcl_interp.zig");
    // ``uplevel`` is a script-eval dispatch.
    if (!interp.recursion_check_enter()) return result_mod.from_globals(0);
    defer interp.recursion_check_leave();
    return result_mod.from_globals(interp.eval_uplevel(words));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "eval", .arity_min = 1, .arity_max = null, .handler = &eval_eval },
    .{ .name = "uplevel", .arity_min = 1, .arity_max = null, .handler = &eval_uplevel },
};
