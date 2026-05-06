// ``set``, ``incr``, ``unset`` — variable read/write commands.

const rt = @import("../tcl_runtime.zig");
const result_mod = @import("../interp/tcl_result.zig");
const frames = @import("../interp/tcl_frames.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const tcl_array = @import("../valtypes/tcl_array.zig");
const tcl_ns = @import("../interp/tcl_ns.zig");

const obj_ensure_string = rt.obj_ensure_string;
const obj_new_string = rt.obj_new_string;

fn eval_set(words: []const i32) result_mod.InterpResult {
    // Reference Tcl: ``set varName ?newValue?`` takes 1 or 2 args.
    // Out-of-range arities raise ``wrong # args`` (parse-9.2).
    if (words.len < 2 or words.len > 3) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const msg_text: []const u8 = "wrong # args: should be \"set varName ?newValue?\"";
        const buf = rt.alloc(@intCast(msg_text.len));
        if (buf == 0) {
            catch_mod.tcl_cmd_error(0);
            return result_mod.from_globals(0);
        }
        const dst: [*]u8 = @ptrFromInt(buf);
        for (msg_text, 0..) |b, k| dst[k] = b;
        const msg = rt.obj_new_string_take(buf, @intCast(msg_text.len), @intCast(msg_text.len));
        catch_mod.tcl_cmd_error(msg);
        return result_mod.from_globals(0);
    }
    if (words.len == 3) {
        _ = frames.var_set(words[1], words[2]);
        return result_mod.from_globals(words[2]);
    }
    // Read form: ``set varName``.  When the variable doesn't exist,
    // raise ``can't read "<name>": no such variable`` per Tcl 9
    // semantics (set-1.13).  ``var_resolve`` returns 0 silently for
    // missing slots — we re-raise via ``var_unset_error``.
    const v = frames.var_resolve(words[1]);
    if (v == 0) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        catch_mod.var_unset_error(words[1]);
        return result_mod.from_globals(0);
    }
    return result_mod.from_globals(v);
}

fn eval_incr(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(0);
    const amt_obj = if (words.len >= 3) words[2] else rt.obj_new_int(1);
    const cur = frames.var_resolve(words[1]);
    const result = rt.tcl_incr(cur, amt_obj);
    _ = frames.var_set(words[1], result);
    return result_mod.from_globals(result);
}

fn eval_unset(words: []const i32) result_mod.InterpResult {
    var i: u32 = 1;
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len == 0) continue;
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        if (wp[0] == '-') continue;
        // Clear the array table before nulling the variable so
        // ``info exists arr`` returns 0 after ``unset arr``.  Phase
        // 1: route through ``frame_resolve_array_name`` so a
        // proc-local ``unset arr`` evicts the
        // ``::__local::<depth>::arr`` directory entry, not just a
        // global ``arr``.
        const resolved_arr = frames.frame_resolve_array_name(words[i]);
        const tcl_obj_mod = @import("../valtypes/tcl_obj.zig");
        defer if (resolved_arr != words[i]) tcl_obj_mod.tcl_obj_release(resolved_arr);
        _ = tcl_array.array_unset(resolved_arr);
        // Namespace-qualified names (containing ``::``) always live in
        // the global table, even without a leading ``::`` prefix.
        var is_global: bool = (w.len >= 2 and wp[0] == ':' and wp[1] == ':');
        if (!is_global) {
            for (0..w.len - 1) |k| {
                if (wp[k] == ':' and wp[k + 1] == ':') {
                    is_global = true;
                    break;
                }
            }
        }
        if (is_global) {
            _ = tcl_ns.global_set(words[i], 0);
        } else {
            _ = frames.var_set(words[i], 0);
        }
    }
    return result_mod.from_globals(obj_new_string(0, 0));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "set", .arity_min = 1, .arity_max = 2, .handler = &eval_set },
    .{ .name = "incr", .arity_min = 1, .arity_max = 2, .handler = &eval_incr },
    .{ .name = "unset", .arity_min = 1, .arity_max = null, .handler = &eval_unset },
};
