// Abstract-list demo commands ported from upstream
// ``tclTestABSList.c``.
//
// Tcl 9 introduced an "abstract list" Tcl_ObjType — lists whose
// elements are produced lazily by a callback.  Our runtime stores
// lists as canonical strings, so we approximate the behaviour by
// materialising the list eagerly:
//
//   * ``lstring str`` — return *str* as a list of single-character
//     elements (one byte per element; multi-byte UTF-8 sequences
//     are kept atomic if they're a valid single code point).
//   * ``lgen length cmd ?args ...?`` — return a list of *length*
//     elements where element i is ``cmd i ?args ...?`` evaluated.
//   * ``value:at: list index`` — abstract-list ``LindexProc``
//     hook; delegates to ``lindex`` builtin.

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

fn eval_lstring(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: lstring string");
    const view = obj_view(words[1]);
    const tcl_list = @import("../valtypes/tcl_list.zig");
    var lst: i32 = obj.obj_new_string(0, 0);
    var i: u32 = 0;
    while (i < view.len) {
        const seq_len = std.unicode.utf8ByteSequenceLength(view[i]) catch 1;
        const advance = @min(@as(u32, seq_len), @as(u32, @intCast(view.len)) - i);
        const elem = obj.obj_new_string_copy(@intCast(@intFromPtr(&view[i])), advance);
        lst = tcl_list.tcl_cmd_lappend(lst, elem);
        i += advance;
    }
    return result_mod.ok(lst);
}

fn eval_lgen(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) return err_msg("wrong # args: lgen length cmd ?args ...?");
    const length_i = obj.obj_get_int(words[1]);
    if (length_i < 0) return err_msg("lgen: negative length");
    const length: u32 = @intCast(length_i);
    if (length > 4096) return err_msg("lgen: length too large");

    const tcl_list = @import("../valtypes/tcl_list.zig");
    var lst: i32 = obj.obj_new_string(0, 0);

    var i: u32 = 0;
    while (i < length) : (i += 1) {
        // Compose: ``<cmd> <args ...> <i>``
        var script_buf: [512]u8 = undefined;
        var off: u32 = 0;
        var w: u32 = 2;
        while (w < words.len) : (w += 1) {
            const v = obj_view(words[w]);
            if (off + v.len + 1 > script_buf.len) return err_msg("lgen: composed script too long");
            for (v, 0..) |c, k| script_buf[off + k] = c;
            off += @intCast(v.len);
            script_buf[off] = ' ';
            off += 1;
        }
        const idx_text_buf = std.fmt.bufPrint(script_buf[off..], "{d}", .{i}) catch return err_msg("lgen: format error");
        off += @intCast(idx_text_buf.len);
        const r = interp.eval_script(@intCast(@intFromPtr(&script_buf)), off);
        lst = tcl_list.tcl_cmd_lappend(lst, r);
    }
    return result_mod.ok(lst);
}

fn eval_value_at(words: []const i32) result_mod.InterpResult {
    if (words.len != 3) return err_msg("wrong # args: value:at: list index");
    const tcl_list = @import("../valtypes/tcl_list.zig");
    const got = tcl_list.tcl_cmd_list_index(words[1], words[2]);
    if (got == 0) return result_mod.ok(obj.obj_new_string(0, 0));
    return result_mod.ok(got);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "lstring", .arity_min = 1, .arity_max = 1, .handler = &eval_lstring },
    .{ .name = "lgen", .arity_min = 2, .arity_max = null, .handler = &eval_lgen },
    .{ .name = "value:at:", .arity_min = 2, .arity_max = 2, .handler = &eval_value_at },
};
