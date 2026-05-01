// ``info``, ``trace`` — introspection commands.

const rt        = @import("../tcl_runtime.zig");
const info      = @import("./tcl_cmd_info.zig");
const trace_mod = @import("../interp/tcl_trace.zig");
const reg       = @import("../dispatch/tcl_cmd_registry.zig");

const str_eq            = @import("../valtypes/tcl_chars.zig").str_eq;
const obj_new_string    = rt.obj_new_string;
const obj_ensure_string = rt.obj_ensure_string;

fn eval_info(words: []const i32) i32 {
    if (words.len >= 2) {
        const sub_s = obj_ensure_string(words[1]);
        const sub_p: [*]const u8 = @ptrFromInt(sub_s.ptr);
        if (str_eq(sub_p, sub_s.len, "default")) {
            if (words.len < 5) return obj_new_string(0, 0);
            return info.info_default(words[2], words[3], words[4]);
        }
        if (str_eq(sub_p, sub_s.len, "commands") and words.len == 2) {
            return info.info_commands(0);
        }
        if (str_eq(sub_p, sub_s.len, "procs") and words.len == 2) {
            return info.info_procs(0);
        }
        if (words.len == 2) return info.info_dispatch(words[1], 0);
    }
    if (words.len >= 3) return info.info_dispatch(words[1], words[2]);
    return obj_new_string(0, 0);
}

fn eval_trace(words: []const i32) i32 {
    const sub     = if (words.len >= 2) words[1] else 0;
    const arg_obj = if (words.len >= 3) words[2] else 0;
    return trace_mod.tcl_cmd_trace_cmd(sub, arg_obj);
}

fn eval_pid(words: []const i32) i32 {
    _ = words;
    return rt.obj_new_int(12345);
}

/// Stub for ``tcl::build-info`` — returns a hard-coded string for the
/// queries Tcl test suites actually use (``patchlevel``, ``version``,
/// ``commit``).  The real C Tcl reads these from compile-time defines
/// in ``tclConfig.sh``; for our WASM runtime, returning sensible
/// strings is enough to keep tests like ``format.test`` running.
fn eval_tcl_build_info(words: []const i32) i32 {
    if (words.len < 2) {
        return obj_new_string_lit("9.0.3");
    }
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "patchlevel")) return obj_new_string_lit("9.0.3");
    if (str_eq(sp, sub.len, "version")) return obj_new_string_lit("9.0");
    if (str_eq(sp, sub.len, "commit")) return obj_new_string_lit("0000000000000000000000000000000000000000");
    if (str_eq(sp, sub.len, "branch")) return obj_new_string_lit("core-9-0-3");
    if (str_eq(sp, sub.len, "compiler")) return obj_new_string_lit("zig-wasm32");
    // Unknown sub-key — return empty rather than trapping; matches
    // reference Tcl which returns "" for unknown build-info keys.
    return obj_new_string(0, 0);
}

/// Helper: allocate a TclObj wrapping a Zig string literal.  The
/// literal lives in the wasm data segment, so its bytes are stable
/// for the lifetime of the module — we point ``OBJ_STR_PTR`` at
/// them with ``OBJ_STR_CAP == 0`` (not owned, not freeable).
fn obj_new_string_lit(comptime s: []const u8) i32 {
    return obj_new_string(@bitCast(@intFromPtr(s.ptr)), @bitCast(s.len));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "info", .arity_min = 1, .arity_max = null, .handler = &eval_info },
    .{ .name = "trace", .arity_min = 1, .arity_max = null, .handler = &eval_trace },
    .{ .name = "pid", .arity_min = 0, .arity_max = 1, .handler = &eval_pid },
    .{ .name = "tcl::build-info", .arity_min = 0, .arity_max = 1, .handler = &eval_tcl_build_info },
    // Also register without the leading ``tcl::`` — some callers
    // import it bare.
    .{ .name = "::tcl::build-info", .arity_min = 0, .arity_max = 1, .handler = &eval_tcl_build_info },
};

// ``info <sub>`` sub-commands — mirrors
// ``core/commands/registry/tcl/info.py``.  Cross-checked against
// ``generic/tclCmdIL.c`` (most handlers enforce ``objc != N`` or
// ``objc < A || objc > B`` directly; the ``TclCompileBasic*ArgCmd``
// entries in ``infoImplMap`` imply the arity for the remainder).
pub const info_subcommands: []const reg.SubEntry = &.{
    .{ .name = "args", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "body", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "class", .arity_min = 2, .arity_max = null, .handler = &eval_info },
    .{ .name = "cmdcount", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "cmdtype", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "commands", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "complete", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "constant", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "consts", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "coroutine", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "default", .arity_min = 3, .arity_max = 3, .handler = &eval_info },
    .{ .name = "errorstack", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "exists", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "frame", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "functions", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "globals", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "hostname", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "level", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "library", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "loaded", .arity_min = 0, .arity_max = 2, .handler = &eval_info },
    .{ .name = "locals", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "nameofexecutable", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "object", .arity_min = 2, .arity_max = null, .handler = &eval_info },
    .{ .name = "patchlevel", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "procs", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "script", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "sharedlibextension", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "tclversion", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "vars", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
};

// ``trace <sub>`` sub-commands — mirrors
// ``core/commands/registry/tcl/trace.py``.
pub const trace_subcommands: []const reg.SubEntry = &.{
    .{ .name = "add", .arity_min = 4, .arity_max = 4, .handler = &eval_trace },
    .{ .name = "info", .arity_min = 2, .arity_max = 2, .handler = &eval_trace },
    .{ .name = "remove", .arity_min = 4, .arity_max = 4, .handler = &eval_trace },
    .{ .name = "variable", .arity_min = 3, .arity_max = 3, .handler = &eval_trace },
    .{ .name = "vdelete", .arity_min = 3, .arity_max = 3, .handler = &eval_trace },
    .{ .name = "vinfo", .arity_min = 1, .arity_max = 1, .handler = &eval_trace },
};
