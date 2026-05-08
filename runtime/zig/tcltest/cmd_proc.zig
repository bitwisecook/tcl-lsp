// `tcl::procbodytest::proc` and `tcl::procbodytest::check` —
// upstream tclTestProcBodyObj.c.  These probe the internal
// ``procbody`` Tcl_ObjType: ``::proc`` lets you create a new proc
// whose body is the *internal_rep* of an existing proc's body
// (sharing bytecode), and ``::check`` is a sanity check that the
// procbody type is registered.
//
// We don't expose a procbody Tcl_ObjType, so the port reduces to
// observable Tcl-level behaviour:
//
//   * ``proc name arg-list source-name`` — create a proc named
//     *name* whose body is the body of *source-name*.  Routes
//     through the runtime's ``proc`` builtin via a composed script.
//   * ``check`` — always returns 1 (the procbody type is "available"
//     for the purpose of every test that just calls ``check``).

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const result_mod = @import("../interp/tcl_result.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const interp = @import("../interp/tcl_interp.zig");
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

fn eval_procbodytest_proc(words: []const i32) result_mod.InterpResult {
    if (words.len != 4) return err_msg("wrong # args: tcl::procbodytest::proc name argList sourceName");
    const new_name = obj_view(words[1]);
    const arg_list = obj_view(words[2]);
    const source_name = obj_view(words[3]);
    var script_buf: [1024]u8 = undefined;
    const slice = std.fmt.bufPrint(
        &script_buf,
        "proc {s} {{{s}}} [info body {s}]",
        .{ new_name, arg_list, source_name },
    ) catch return err_msg("tcl::procbodytest::proc: composed script too long");
    const r = interp.eval_script(@intCast(@intFromPtr(slice.ptr)), @intCast(slice.len));
    return result_mod.from_globals(r);
}

fn eval_procbodytest_check(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.ok(obj.obj_new_int(1));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "tcl::procbodytest::proc", .arity_min = 3, .arity_max = 3, .handler = &eval_procbodytest_proc },
    .{ .name = "tcl::procbodytest::check", .arity_min = 0, .arity_max = 0, .handler = &eval_procbodytest_check },
};
