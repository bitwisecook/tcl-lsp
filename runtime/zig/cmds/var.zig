// ``set``, ``incr``, ``unset`` — variable read/write commands.

const rt          = @import("../tcl_runtime.zig");
const frames      = @import("../interp/tcl_frames.zig");
const reg         = @import("../dispatch/tcl_cmd_registry.zig");
const tcl_array   = @import("../valtypes/tcl_array.zig");
const tcl_ns      = @import("../interp/tcl_ns.zig");

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
        if (w.len == 0) continue;
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        if (wp[0] == '-') continue;
        // Clear the array table before nulling the variable so that
        // ``info exists`` on an upvar alias finds no stale array
        // entries after an ``unset`` (Tcl semantics: unset removes
        // both the scalar slot and any associated array).
        _ = tcl_array.array_unset(words[i]);
        // Namespace-qualified names (containing ``::``) always live in
        // the global table, even without a leading ``::`` prefix.
        var is_global: bool = (w.len >= 2 and wp[0] == ':' and wp[1] == ':');
        if (!is_global) {
            for (0..w.len - 1) |k| {
                if (wp[k] == ':' and wp[k + 1] == ':') { is_global = true; break; }
            }
        }
        if (is_global) {
            _ = tcl_ns.global_set(words[i], 0);
        } else {
            _ = frames.var_set(words[i], 0);
        }
    }
    return obj_new_string(0, 0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "set", .arity_min = 1, .arity_max = 2, .handler = &eval_set },
    .{ .name = "incr", .arity_min = 1, .arity_max = 2, .handler = &eval_incr },
    .{ .name = "unset", .arity_min = 1, .arity_max = null, .handler = &eval_unset },
};
