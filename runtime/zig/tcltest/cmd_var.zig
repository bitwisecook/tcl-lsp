// `testupvar` and `testgetvarfullname` — upstream's introspection
// hooks into Tcl_UpVar2 / Tcl_GetVariableFullName.
//
// Both reduce to small wrappers around the runtime's namespace +
// frame-alias machinery.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const result_mod = @import("../interp/tcl_result.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const interp = @import("../interp/tcl_interp.zig");
const tcl_ns = @import("../interp/tcl_ns.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

fn build_msg(buf: []const u8) i32 {
    const dst = obj.alloc(@intCast(buf.len));
    if (dst == 0) return 0;
    const dst_p: [*]u8 = @ptrFromInt(dst);
    for (buf, 0..) |c, i| dst_p[i] = c;
    return obj.obj_new_string_take(dst, @intCast(buf.len), @intCast(buf.len));
}

fn err_msg(buf: []const u8) result_mod.InterpResult {
    catch_mod.tcl_cmd_error(build_msg(buf));
    return result_mod.from_globals(0);
}

fn obj_view(handle: i32) []const u8 {
    const s = obj.obj_ensure_string(handle);
    const ptr: [*]const u8 = @ptrFromInt(s.ptr);
    return ptr[0..s.len];
}

/// ``testupvar level name ?name2? dest global`` — installs an alias
/// in the current frame for *name* (or array element ``name(name2)``)
/// from *level* frames up.  We delegate to the runtime's
/// ``upvar`` builtin by composing a script.
fn eval_testupvar(words: []const i32) result_mod.InterpResult {
    if (words.len != 5 and words.len != 6) {
        return err_msg("wrong # args: testupvar level name ?name2? dest global");
    }
    // Compose: ``upvar #LEVEL name dest`` (or with name2 the array form).
    const level = obj_view(words[1]);
    const name = obj_view(words[2]);
    const name2 = if (words.len == 6) obj_view(words[3]) else "";
    const dest = if (words.len == 6) obj_view(words[4]) else obj_view(words[3]);

    var script_buf: [512]u8 = undefined;
    const slice = if (words.len == 5)
        std.fmt.bufPrint(&script_buf, "upvar {s} {s} {s}", .{ level, name, dest }) catch return err_msg("testupvar: composed script too long")
    else if (name2.len == 0)
        std.fmt.bufPrint(&script_buf, "upvar {s} {s} {s}", .{ level, name, dest }) catch return err_msg("testupvar: composed script too long")
    else
        std.fmt.bufPrint(&script_buf, "upvar {s} {s}({s}) {s}", .{ level, name, name2, dest }) catch return err_msg("testupvar: composed script too long");
    const r = interp.eval_script(@intCast(@intFromPtr(slice.ptr)), @intCast(slice.len));
    return result_mod.from_globals(r);
}

/// ``testgetvarfullname name flags`` — emit ``::ns::name`` form.
/// ``flags`` matches upstream's ``TCL_GLOBAL_ONLY`` / ``TCL_NAMESPACE_ONLY``
/// hints; we ignore the flag and always resolve through the
/// current namespace.  Pure namespace-introspection.
fn eval_testgetvarfullname(words: []const i32) result_mod.InterpResult {
    if (words.len != 3) return err_msg("wrong # args: testgetvarfullname name flags");
    const name = obj_view(words[1]);
    // Best-effort: if name already starts with "::" return verbatim,
    // else prefix with the current namespace.
    if (name.len >= 2 and name[0] == ':' and name[1] == ':') {
        return result_mod.ok(obj.obj_new_string_copy(@intCast(@intFromPtr(name.ptr)), @intCast(name.len)));
    }
    // Read current namespace via the runtime export.
    const cur_ns_p = tcl_ns.current_ns_full_ptr();
    const cur_ns_l = tcl_ns.current_ns_full_len();
    const cur: [*]const u8 = @ptrFromInt(@as(u32, @bitCast(cur_ns_p)));
    const cur_view = cur[0..@intCast(cur_ns_l)];
    var buf: [256]u8 = undefined;
    const slice = if (cur_view.len == 0 or std.mem.eql(u8, cur_view, "::"))
        std.fmt.bufPrint(&buf, "::{s}", .{name}) catch return err_msg("testgetvarfullname: name too long")
    else
        std.fmt.bufPrint(&buf, "{s}::{s}", .{ cur_view, name }) catch return err_msg("testgetvarfullname: name too long");
    return result_mod.ok(obj.obj_new_string_copy(@intCast(@intFromPtr(slice.ptr)), @intCast(slice.len)));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "testupvar", .arity_min = 4, .arity_max = 5, .handler = &eval_testupvar },
    .{ .name = "testgetvarfullname", .arity_min = 2, .arity_max = 2, .handler = &eval_testgetvarfullname },
};
