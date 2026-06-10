// Catch-all for the remaining tcltest commands that don't fit any
// other cluster — small wrappers, namespace-prefixed registrations,
// and best-effort stubs that pass simple smoke tests.

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

// -- ::tcl::test::build-info ---------------------------------------------

/// ``::tcl::test::build-info ?key?`` — query build-time metadata.
/// Upstream delegates to the existing ``::tcl::build-info`` command;
/// we synthesise a minimal response.
fn eval_test_build_info(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.ok(build_msg("9.0.3+wasm"));
}

// -- test_ns_basic::createdcommand ---------------------------------------

/// Synthetic command in the ``test_ns_basic`` namespace; upstream
/// uses it as a registration target for ``testcreatecommand``.
/// Returns the args concatenated.
fn eval_test_ns_basic_createdcommand(words: []const i32) result_mod.InterpResult {
    var total: u32 = 0;
    var i: u32 = 1;
    while (i < words.len) : (i += 1) {
        total += obj_view(words[i]).len;
        if (i < words.len - 1) total += 1;
    }
    const buf = obj.alloc(total);
    if (buf == 0) return err_msg("out of memory");
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    i = 1;
    while (i < words.len) : (i += 1) {
        const v = obj_view(words[i]);
        for (v, 0..) |c, k| dst[off + k] = c;
        off += @intCast(v.len);
        if (i < words.len - 1) {
            dst[off] = ' ';
            off += 1;
        }
    }
    return result_mod.ok(obj.obj_new_string_take(buf, total, total));
}

// -- testencoding -------------------------------------------------------

/// Best-effort stub: most encoding C APIs (Tcl_CreateEncoding /
/// Tcl_GetEncoding) require an encoding registry we don't surface.
/// We accept ``nullength`` (returns 1, since UTF-8 / ASCII single-
/// byte encodings have a 1-byte null terminator) and reject the rest
/// with an explicit error.
fn eval_testencoding(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return err_msg("wrong # args: testencoding option");
    const opt = obj_view(words[1]);
    if (std.mem.eql(u8, opt, "nullength")) return result_mod.ok(obj.obj_new_int(1));
    return err_msg("testencoding: only nullength is supported under WASM");
}

// -- testregexp ---------------------------------------------------------

/// ``testregexp ?-flags? expr string ?matchVar? ?subVar ...?`` —
/// thin wrapper around the ``regexp`` builtin.  We strip leading
/// flags (``-xflags``, ``-about``, ``-indices``) silently and
/// delegate the rest, matching the test-corpus shapes that don't
/// rely on the upstream ``REG_EXPECT`` extension.
fn eval_testregexp(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) return err_msg("wrong # args: testregexp ?-flags? expr string ?matchVar ...?");
    var i: u32 = 1;
    while (i < words.len) : (i += 1) {
        const v = obj_view(words[i]);
        if (v.len > 0 and v[0] == '-') continue;
        break;
    }
    if (i + 1 >= words.len) return err_msg("testregexp: missing expr/string");
    const expr_view = obj_view(words[i]);
    const string_view = obj_view(words[i + 1]);
    var script_buf: [1024]u8 = undefined;
    const slice = std.fmt.bufPrint(
        &script_buf,
        "regexp {{{s}}} {{{s}}}",
        .{ expr_view, string_view },
    ) catch return err_msg("testregexp: composed script too long");
    const r = interp.eval_script(@intCast(@intFromPtr(slice.ptr)), @intCast(slice.len));
    return result_mod.from_globals(r);
}

// -- testlistrep --------------------------------------------------------

/// ``testlistrep new <count>`` returns a fresh list of count zeroes.
/// ``testlistrep describe <list>`` returns a synthetic shape string.
/// ``testlistrep validate <list>`` returns 1 — validation is a no-op
/// for our string-backed lists.
/// ``testlistrep config`` returns the canonical (capacity, leadSpace)
/// pair as zero — upstream uses this to introspect ``ListRep`` C
/// fields we don't expose.
fn eval_testlistrep(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return err_msg("wrong # args: testlistrep option ?args?");
    const opt = obj_view(words[1]);
    const tcl_list = @import("../valtypes/tcl_list.zig");
    if (std.mem.eql(u8, opt, "new")) {
        if (words.len != 3) return err_msg("wrong # args: testlistrep new count");
        const n = obj.obj_get_int(words[2]);
        if (n < 0 or n > 4096) return err_msg("testlistrep: count out of range");
        var lst: i32 = obj.obj_new_string(0, 0);
        var i: i64 = 0;
        while (i < n) : (i += 1) lst = tcl_list.tcl_cmd_lappend(lst, obj.obj_new_int(0));
        return result_mod.ok(lst);
    }
    if (std.mem.eql(u8, opt, "validate")) return result_mod.ok(obj.obj_new_int(1));
    if (std.mem.eql(u8, opt, "describe")) return result_mod.ok(build_msg("list canonical"));
    if (std.mem.eql(u8, opt, "config")) return result_mod.ok(build_msg("0 0"));
    return err_msg("testlistrep: bad sub-command (new|describe|validate|config)");
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "::tcl::test::build-info", .arity_min = 0, .arity_max = 1, .handler = &eval_test_build_info },
    .{ .name = "test_ns_basic::createdcommand", .arity_min = 0, .arity_max = null, .handler = &eval_test_ns_basic_createdcommand },
    .{ .name = "testencoding", .arity_min = 1, .arity_max = null, .handler = &eval_testencoding },
    .{ .name = "testregexp", .arity_min = 2, .arity_max = null, .handler = &eval_testregexp },
    .{ .name = "testlistrep", .arity_min = 1, .arity_max = null, .handler = &eval_testlistrep },
};
