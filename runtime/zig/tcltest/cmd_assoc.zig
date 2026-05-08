// `testsetassocdata` / `testgetassocdata` / `testdelassocdata` —
// upstream's per-interp assoc-data dictionary keyed by string.
//
// Our runtime doesn't surface a per-interp assoc-data slot table,
// so we keep one private to the tcltest extension here.  The keys
// and values are short ASCII strings; capacity is bounded.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const result_mod = @import("../interp/tcl_result.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

const MAX_ASSOC_ENTRIES: u32 = 64;
const MAX_KEY_LEN: u32 = 64;
const MAX_VALUE_LEN: u32 = 256;

const Entry = struct {
    key_buf: [MAX_KEY_LEN]u8 = std.mem.zeroes([MAX_KEY_LEN]u8),
    key_len: u32 = 0,
    value_buf: [MAX_VALUE_LEN]u8 = std.mem.zeroes([MAX_VALUE_LEN]u8),
    value_len: u32 = 0,
    used: bool = false,
};

var entries: [MAX_ASSOC_ENTRIES]Entry = .{Entry{}} ** MAX_ASSOC_ENTRIES;

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

fn lookup(key: []const u8) ?u32 {
    for (0..entries.len) |i| {
        if (entries[i].used and entries[i].key_len == key.len and
            std.mem.eql(u8, entries[i].key_buf[0..entries[i].key_len], key))
            return @intCast(i);
    }
    return null;
}

fn allocate(key: []const u8) ?u32 {
    if (lookup(key)) |i| return i;
    for (0..entries.len) |i| {
        if (!entries[i].used) return @intCast(i);
    }
    return null;
}

fn eval_testsetassocdata(words: []const i32) result_mod.InterpResult {
    if (words.len != 3) return err_msg("wrong # args: testsetassocdata data_key data_item");
    const k = obj_view(words[1]);
    const v = obj_view(words[2]);
    if (k.len > MAX_KEY_LEN or v.len > MAX_VALUE_LEN) return err_msg("assoc-data: key or value too long");
    const idx = allocate(k) orelse return err_msg("assoc-data: registry full");
    var e = &entries[idx];
    for (k, 0..) |c, i| e.key_buf[i] = c;
    e.key_len = @intCast(k.len);
    for (v, 0..) |c, i| e.value_buf[i] = c;
    e.value_len = @intCast(v.len);
    e.used = true;
    return result_mod.from_globals(0);
}

fn eval_testgetassocdata(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testgetassocdata data_key");
    const k = obj_view(words[1]);
    if (lookup(k)) |idx| {
        const e = entries[idx];
        return result_mod.ok(obj.obj_new_string_copy(
            @intCast(@intFromPtr(&e.value_buf)),
            @intCast(e.value_len),
        ));
    }
    return result_mod.ok(obj.obj_new_string(0, 0));
}

fn eval_testdelassocdata(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testdelassocdata data_key");
    const k = obj_view(words[1]);
    if (lookup(k)) |idx| entries[idx].used = false;
    return result_mod.from_globals(0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "testsetassocdata", .arity_min = 2, .arity_max = 2, .handler = &eval_testsetassocdata },
    .{ .name = "testgetassocdata", .arity_min = 1, .arity_max = 1, .handler = &eval_testgetassocdata },
    .{ .name = "testdelassocdata", .arity_min = 1, .arity_max = 1, .handler = &eval_testdelassocdata },
};
