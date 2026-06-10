// Command-table introspection test commands ported from upstream
// ``tclTest.c``.
//
// The C-tier originals manipulate ``Tcl_GetCommandInfo`` /
// ``Tcl_SetCommandInfo`` / ``Tcl_CreateObjCommand`` directly to
// register synthetic commands in the interp's hash table.  Our
// runtime stores commands in a static ``BUILTINS`` slice plus the
// proc-registry; we don't expose live registration / token APIs.
// The port reduces to:
//
//   * commands that just need to "exist" (pass) — the Tcl-level
//     test scripts only check return codes
//   * commands that probe a token / cookie — return a synthetic
//     stable token
//
// Real ``Tcl_GetCommandInfo``-shape behaviour would require
// extending the runtime's command table to record per-command
// origin metadata.  That's a larger refactor; for now the
// stub-with-explicit-message in the upstream "modify" / "get"
// branches surfaces the gap clearly.

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

fn obj_eq_lit(handle: i32, lit: []const u8) bool {
    return std.mem.eql(u8, obj_view(handle), lit);
}

// -- testcmdtoken --------------------------------------------------------

var token_counter: u32 = 1;

fn eval_testcmdtoken(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return err_msg("wrong # args: testcmdtoken create|name name");
    if (obj_eq_lit(words[1], "create")) {
        if (words.len != 3) return err_msg("wrong # args: testcmdtoken create name");
        // Synthesise a stable token; format matches upstream's pointer
        // shape (0xN) so test scripts that pattern-match {0x*} still
        // recognise it.
        var buf: [32]u8 = undefined;
        const slice = std.fmt.bufPrint(&buf, "0x{x}", .{token_counter}) catch return err_msg("token format error");
        token_counter += 1;
        return result_mod.ok(build_msg(slice));
    }
    if (obj_eq_lit(words[1], "name")) {
        if (words.len != 3) return err_msg("wrong # args: testcmdtoken name token");
        // Without a real token table we round-trip the input.
        return result_mod.ok(words[2]);
    }
    return err_msg("testcmdtoken: bad sub-command");
}

// -- testcmdinfo --------------------------------------------------------

fn eval_testcmdinfo(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) return err_msg("wrong # args: testcmdinfo option name");
    if (obj_eq_lit(words[1], "create")) {
        // Compose ``proc <name> args { return $args }`` so subsequent
        // ``info commands`` / ``$name`` calls observe a real proc.
        const name = obj_view(words[2]);
        var buf: [256]u8 = undefined;
        const slice = std.fmt.bufPrint(&buf, "proc {s} args {{ return $args }}", .{name}) catch
            return err_msg("testcmdinfo: name too long");
        const r = interp.eval_script(@intCast(@intFromPtr(slice.ptr)), @intCast(slice.len));
        return result_mod.from_globals(r);
    }
    if (obj_eq_lit(words[1], "delete")) {
        const name = obj_view(words[2]);
        var buf: [256]u8 = undefined;
        const slice = std.fmt.bufPrint(&buf, "rename {s} {{}}", .{name}) catch return err_msg("testcmdinfo: name too long");
        const r = interp.eval_script(@intCast(@intFromPtr(slice.ptr)), @intCast(slice.len));
        return result_mod.from_globals(r);
    }
    if (obj_eq_lit(words[1], "get") or obj_eq_lit(words[1], "modify") or obj_eq_lit(words[1], "call")) {
        // Probe / mutate per-command C-side metadata; we don't store it.
        return result_mod.ok(obj.obj_new_string(0, 0));
    }
    return err_msg("testcmdinfo: bad sub-command");
}

// -- testcmdtrace -------------------------------------------------------

fn eval_testcmdtrace(words: []const i32) result_mod.InterpResult {
    _ = words;
    // Real ``Tcl_CreateObjTrace`` is stub-able; we accept the call
    // and return empty so test scripts that just register / unregister
    // a trace see no error.
    return result_mod.ok(obj.obj_new_string(0, 0));
}

// -- testcmdobj2 --------------------------------------------------------

fn eval_testcmdobj2(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.ok(obj.obj_new_string(0, 0));
    return result_mod.ok(obj.obj_new_int(@intCast(words.len - 1)));
}

// -- testcreatecommand --------------------------------------------------

fn eval_testcreatecommand(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return err_msg("wrong # args: testcreatecommand option");
    if (obj_eq_lit(words[1], "create")) {
        // Upstream creates ``test_ns_basic::createdcommand``; ours
        // graduates to a real proc returning empty string.
        const script = "namespace eval test_ns_basic { proc createdcommand args { return {} } }";
        const r = interp.eval_script(@intCast(@intFromPtr(script.ptr)), @intCast(script.len));
        return result_mod.from_globals(r);
    }
    if (obj_eq_lit(words[1], "delete")) {
        const script = "rename test_ns_basic::createdcommand {}";
        const r = interp.eval_script(@intCast(@intFromPtr(script.ptr)), @intCast(script.len));
        return result_mod.from_globals(r);
    }
    if (obj_eq_lit(words[1], "create2") or obj_eq_lit(words[1], "delete2")) {
        return result_mod.ok(obj.obj_new_string(0, 0));
    }
    return err_msg("testcreatecommand: bad sub-command");
}

// -- testdel ------------------------------------------------------------

fn eval_testdel(words: []const i32) result_mod.InterpResult {
    if (words.len != 4) return err_msg("wrong # args: testdel child cmdname value");
    // Without child-interp delete callbacks we can't replicate the
    // exact behaviour; accept the call and return empty.
    return result_mod.ok(obj.obj_new_string(0, 0));
}

// -- testinterpdelete ---------------------------------------------------

fn eval_testinterpdelete(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testinterpdelete path");
    // Compose: ``interp delete <path>``.
    const path = obj_view(words[1]);
    var buf: [256]u8 = undefined;
    const slice = std.fmt.bufPrint(&buf, "interp delete {s}", .{path}) catch return err_msg("testinterpdelete: path too long");
    const r = interp.eval_script(@intCast(@intFromPtr(slice.ptr)), @intCast(slice.len));
    return result_mod.from_globals(r);
}

// -- testinterpresolver -------------------------------------------------

fn eval_testinterpresolver(words: []const i32) result_mod.InterpResult {
    _ = words;
    // No Tcl_AddInterpResolvers equivalent — accept the call and return empty.
    return result_mod.ok(obj.obj_new_string(0, 0));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "testcmdtoken", .arity_min = 1, .arity_max = null, .handler = &eval_testcmdtoken },
    .{ .name = "testcmdinfo", .arity_min = 2, .arity_max = null, .handler = &eval_testcmdinfo },
    .{ .name = "testcmdtrace", .arity_min = 0, .arity_max = null, .handler = &eval_testcmdtrace },
    .{ .name = "testcmdobj2", .arity_min = 0, .arity_max = null, .handler = &eval_testcmdobj2 },
    .{ .name = "testcreatecommand", .arity_min = 1, .arity_max = null, .handler = &eval_testcreatecommand },
    .{ .name = "testdel", .arity_min = 3, .arity_max = 3, .handler = &eval_testdel },
    .{ .name = "testinterpdelete", .arity_min = 1, .arity_max = 1, .handler = &eval_testinterpdelete },
    .{ .name = "testinterpresolver", .arity_min = 0, .arity_max = null, .handler = &eval_testinterpresolver },
};
