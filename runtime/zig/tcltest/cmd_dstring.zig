// `testdstring` — exercises the upstream Tcl_DString abstraction
// (a growable byte buffer with append/element/sublist support).
//
// Upstream stashes a single ``Tcl_DString dstring`` at file scope
// (``tclTest.c:612``); each sub-command reads or mutates it.  We
// keep the same shape with a static byte buffer + length here.
//
// Sub-commands implemented (PORTABLE):
//   * ``testdstring append str count`` — append count bytes of str
//   * ``testdstring element item``      — append item as a list element
//   * ``testdstring end``                — end the current sublist
//   * ``testdstring free``               — clear the buffer
//   * ``testdstring get``                — return the current contents
//   * ``testdstring length``             — current byte length
//   * ``testdstring result``             — copy contents to interp result
//   * ``testdstring toobj``              — same; obj rep
//   * ``testdstring trunc count``        — truncate to count bytes
//   * ``testdstring start``              — start a new sublist
//   * ``testdstring gresult <type>``     — exercises Tcl_AppendResult /
//     Tcl_DStringGetResult round-trip; we approximate with the same
//     ``staticsmall`` / ``staticlarge`` text upstream emits.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const result_mod = @import("../interp/tcl_result.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

const DSTRING_CAP: u32 = 65536;
var dstring_buf: [DSTRING_CAP]u8 = undefined;
var dstring_len: u32 = 0;
var dstring_sublist_start: i32 = -1; // byte offset where the current sublist began

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

fn obj_eq_lit(handle: i32, lit: []const u8) bool {
    return std.mem.eql(u8, obj_view(handle), lit);
}

fn d_append(bytes: []const u8) bool {
    if (dstring_len + bytes.len > DSTRING_CAP) return false;
    for (bytes, 0..) |c, i| dstring_buf[dstring_len + i] = c;
    dstring_len += @intCast(bytes.len);
    return true;
}

fn d_clear() void {
    dstring_len = 0;
    dstring_sublist_start = -1;
}

fn d_view() []const u8 {
    return dstring_buf[0..dstring_len];
}

/// Quote ``s`` for use as a list element.  Conservative: we wrap in
/// braces if any whitespace, ``{``, ``}``, ``\``, ``"``, or ``[`` is
/// present.  Empty strings become ``{}``.
fn list_element_quote(s: []const u8, dst: *[DSTRING_CAP]u8, off: u32) ?u32 {
    var needs_quoting = s.len == 0;
    for (s) |c| {
        switch (c) {
            ' ', '\t', '\n', '\r', '{', '}', '\\', '"', '[', ']' => {
                needs_quoting = true;
                break;
            },
            else => {},
        }
    }
    if (!needs_quoting) {
        if (off + s.len > DSTRING_CAP) return null;
        for (s, 0..) |c, i| dst[off + i] = c;
        return off + @as(u32, @intCast(s.len));
    }
    if (off + s.len + 2 > DSTRING_CAP) return null;
    dst[off] = '{';
    for (s, 0..) |c, i| dst[off + 1 + i] = c;
    dst[off + 1 + s.len] = '}';
    return off + @as(u32, @intCast(s.len)) + 2;
}

fn eval_testdstring(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return err_msg("wrong # args: testdstring option ?args?");

    if (obj_eq_lit(words[1], "append")) {
        if (words.len != 4) return err_msg("wrong # args: testdstring append str count");
        const view = obj_view(words[2]);
        const count_i = obj.obj_get_int(words[3]);
        const count: u32 = if (count_i < 0)
            @intCast(view.len)
        else
            @min(@as(u32, @intCast(count_i)), @as(u32, @intCast(view.len)));
        if (!d_append(view[0..count])) return err_msg("dstring overflow");
        return result_mod.from_globals(0);
    }
    if (obj_eq_lit(words[1], "element")) {
        if (words.len != 3) return err_msg("wrong # args: testdstring element item");
        const view = obj_view(words[2]);
        if (dstring_len > 0 and dstring_buf[dstring_len - 1] != '{') {
            if (!d_append(" ")) return err_msg("dstring overflow");
        }
        const new_off = list_element_quote(view, &dstring_buf, dstring_len) orelse
            return err_msg("dstring overflow");
        dstring_len = new_off;
        return result_mod.from_globals(0);
    }
    if (obj_eq_lit(words[1], "end")) {
        if (words.len != 2) return err_msg("wrong # args: testdstring end");
        if (dstring_sublist_start >= 0) {
            if (!d_append("}")) return err_msg("dstring overflow");
            dstring_sublist_start = -1;
        }
        return result_mod.from_globals(0);
    }
    if (obj_eq_lit(words[1], "start")) {
        if (words.len != 2) return err_msg("wrong # args: testdstring start");
        if (dstring_len > 0 and dstring_buf[dstring_len - 1] != '{') {
            if (!d_append(" ")) return err_msg("dstring overflow");
        }
        if (!d_append("{")) return err_msg("dstring overflow");
        dstring_sublist_start = @intCast(dstring_len);
        return result_mod.from_globals(0);
    }
    if (obj_eq_lit(words[1], "free")) {
        if (words.len != 2) return err_msg("wrong # args: testdstring free");
        d_clear();
        return result_mod.from_globals(0);
    }
    if (obj_eq_lit(words[1], "length")) {
        if (words.len != 2) return err_msg("wrong # args: testdstring length");
        return result_mod.ok(obj.obj_new_int(dstring_len));
    }
    if (obj_eq_lit(words[1], "get") or obj_eq_lit(words[1], "result") or obj_eq_lit(words[1], "toobj")) {
        if (words.len != 2) return err_msg("wrong # args: testdstring get/result/toobj");
        const view = d_view();
        const out = obj.obj_new_string_copy(@intCast(@intFromPtr(view.ptr)), @intCast(view.len));
        if (obj_eq_lit(words[1], "result") or obj_eq_lit(words[1], "toobj")) d_clear();
        return result_mod.ok(out);
    }
    if (obj_eq_lit(words[1], "trunc")) {
        if (words.len != 3) return err_msg("wrong # args: testdstring trunc count");
        const c = obj.obj_get_int(words[2]);
        const new_len: u32 = if (c < 0) 0 else @min(@as(u32, @intCast(c)), dstring_len);
        dstring_len = new_len;
        return result_mod.from_globals(0);
    }
    if (obj_eq_lit(words[1], "gresult")) {
        if (words.len != 3) return err_msg("wrong # args: testdstring gresult <type>");
        if (obj_eq_lit(words[2], "staticsmall")) return result_mod.ok(build_msg("short"));
        if (obj_eq_lit(words[2], "staticlarge")) {
            return result_mod.ok(build_msg(
                "first0 first1 first2 first3 first4 first5 first6 first7 first8 first9\n" ++
                "second0 second1 second2 second3 second4 second5 second6 second7 second8 second9\n" ++
                "third0 third1 third2 third3 third4 third5 third6 third7 third8 third9\n" ++
                "fourth0 fourth1 fourth2 fourth3 fourth4 fourth5 fourth6 fourth7 fourth8 fourth9\n" ++
                "fifth0 fifth1 fifth2 fifth3 fifth4 fifth5 fifth6 fifth7 fifth8 fifth9\n" ++
                "sixth0 sixth1 sixth2 sixth3 sixth4 sixth5 sixth6 sixth7 sixth8 sixth9\n" ++
                "seventh0 seventh1 seventh2 seventh3 seventh4 seventh5 seventh6 seventh7 seventh8 seventh9\n",
            ));
        }
        if (obj_eq_lit(words[2], "free")) {
            return result_mod.ok(build_msg("This is a malloc-ed string"));
        }
        if (obj_eq_lit(words[2], "special")) {
            return result_mod.ok(build_msg("This is a specially-allocated string"));
        }
        return err_msg("bad gresult option: must be staticsmall, staticlarge, free, or special");
    }
    return err_msg("bad option: must be append, element, end, free, get, gresult, length, result, start, toobj, or trunc");
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "testdstring", .arity_min = 1, .arity_max = null, .handler = &eval_testdstring },
};
