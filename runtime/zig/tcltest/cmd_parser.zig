// Parser-introspection test commands ported from upstream
// ``tclTest.c`` — they probe ``Tcl_ParseCommand`` /
// ``Tcl_ParseExpr`` / ``Tcl_ParseVar`` / ``Tcl_ParseVarName``
// and emit a pretty-printed token tree.
//
// Our runtime doesn't expose the upstream parse-tree shape, so the
// port emits a best-effort ``{type text}`` form: a single token with
// type ``simple`` and the input text.  This satisfies test scripts
// that just check the parser doesn't crash; tests that pattern-match
// the upstream tree won't pass without a full parser-port (a much
// larger surface).

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const result_mod = @import("../interp/tcl_result.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
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

fn emit_simple(text: []const u8) i32 {
    var buf: [512]u8 = undefined;
    const slice = std.fmt.bufPrint(&buf, "- {s} simple {s} 1 simple {{{s}}} text {s} 1", .{ "simple", text, text, text }) catch text;
    return build_msg(slice);
}

fn eval_testparser(words: []const i32) result_mod.InterpResult {
    if (words.len != 3) return err_msg("wrong # args: testparser script length");
    const view = obj_view(words[1]);
    return result_mod.ok(emit_simple(view));
}

fn eval_testparsevar(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testparsevar varname");
    // ``testparsevar`` parses a $var form.  We strip a leading $ if
    // present, look up the variable through the runtime's globals
    // path, and return ``{value remaining}`` — the upstream shape.
    const view = obj_view(words[1]);
    if (view.len == 0) return err_msg("testparsevar: empty input");
    const tcl_ns = @import("../interp/tcl_ns.zig");
    var name_view = view;
    if (name_view[0] == '$') name_view = name_view[1..];
    const name_obj = obj.obj_new_string_copy(@intCast(@intFromPtr(name_view.ptr)), @intCast(name_view.len));
    const value = tcl_ns.global_get(name_obj);
    const tcl_list = @import("../valtypes/tcl_list.zig");
    var out: i32 = obj.obj_new_string(0, 0);
    out = tcl_list.tcl_cmd_lappend(out, if (value == 0) obj.obj_new_string(0, 0) else value);
    out = tcl_list.tcl_cmd_lappend(out, obj.obj_new_string(0, 0));
    return result_mod.ok(out);
}

fn eval_testparsevarname(words: []const i32) result_mod.InterpResult {
    if (words.len != 4) return err_msg("wrong # args: testparsevarname script length append");
    const view = obj_view(words[1]);
    return result_mod.ok(emit_simple(view));
}

fn eval_testexprparser(words: []const i32) result_mod.InterpResult {
    if (words.len != 3) return err_msg("wrong # args: testexprparser expr length");
    const view = obj_view(words[1]);
    return result_mod.ok(emit_simple(view));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "testparser", .arity_min = 2, .arity_max = 2, .handler = &eval_testparser },
    .{ .name = "testparsevar", .arity_min = 1, .arity_max = 1, .handler = &eval_testparsevar },
    .{ .name = "testparsevarname", .arity_min = 3, .arity_max = 3, .handler = &eval_testparsevarname },
    .{ .name = "testexprparser", .arity_min = 2, .arity_max = 2, .handler = &eval_testexprparser },
};
