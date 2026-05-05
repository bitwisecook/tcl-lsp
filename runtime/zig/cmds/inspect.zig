// ``info``, ``trace`` — introspection commands.

const rt        = @import("../tcl_runtime.zig");
const info      = @import("./tcl_cmd_info.zig");
const trace_mod = @import("../interp/tcl_trace.zig");
const var_trace = @import("../interp/tcl_var_trace.zig");
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
    if (words.len < 2) {
        return obj_new_string(0, 0);
    }
    const sub_s = obj_ensure_string(words[1]);
    const sub_p: [*]const u8 = @ptrFromInt(sub_s.ptr);
    // ``trace add variable NAME OPS CMD`` — phase 6 implementation.
    if (str_eq(sub_p, sub_s.len, "add") and words.len >= 6) {
        const kind_s = obj_ensure_string(words[2]);
        const kind_p: [*]const u8 = @ptrFromInt(kind_s.ptr);
        if (str_eq(kind_p, kind_s.len, "variable")) {
            const name = words[3];
            const ops = parse_ops(words[4]);
            var_trace.add(name, ops, words[5]);
            return obj_new_string(0, 0);
        }
        // execution / command tracing — pass-through NOP.
        return obj_new_string(0, 0);
    }
    // ``trace remove variable NAME OPS CMD``.
    if (str_eq(sub_p, sub_s.len, "remove") and words.len >= 6) {
        const kind_s = obj_ensure_string(words[2]);
        const kind_p: [*]const u8 = @ptrFromInt(kind_s.ptr);
        if (str_eq(kind_p, kind_s.len, "variable")) {
            const name = words[3];
            const ops = parse_ops(words[4]);
            _ = var_trace.remove(name, ops, words[5]);
            return obj_new_string(0, 0);
        }
        return obj_new_string(0, 0);
    }
    // ``trace info variable NAME``.
    if (str_eq(sub_p, sub_s.len, "info") and words.len >= 4) {
        const kind_s = obj_ensure_string(words[2]);
        const kind_p: [*]const u8 = @ptrFromInt(kind_s.ptr);
        if (str_eq(kind_p, kind_s.len, "variable")) {
            return var_trace.info(words[3]);
        }
        return obj_new_string(0, 0);
    }
    // Legacy forms: ``trace variable NAME OPS CMD`` /
    // ``trace vdelete NAME OPS CMD`` / ``trace vinfo NAME``.
    if (str_eq(sub_p, sub_s.len, "variable") and words.len >= 5) {
        var_trace.add(words[2], parse_ops(words[3]), words[4]);
        return obj_new_string(0, 0);
    }
    if (str_eq(sub_p, sub_s.len, "vdelete") and words.len >= 5) {
        _ = var_trace.remove(words[2], parse_ops(words[3]), words[4]);
        return obj_new_string(0, 0);
    }
    if (str_eq(sub_p, sub_s.len, "vinfo") and words.len >= 3) {
        return var_trace.info(words[2]);
    }
    // Anything else: defer to the legacy NOP module so we don't trap
    // tests that rely on pass-through behaviour.
    const sub     = if (words.len >= 2) words[1] else 0;
    const arg_obj = if (words.len >= 3) words[2] else 0;
    return trace_mod.tcl_cmd_trace_cmd(sub, arg_obj);
}

/// Parse the ops list ``{read write unset array}`` (any subset, in
/// any order — Tcl is lenient) into the OP_* bitmask.
fn parse_ops(ops_obj: i32) u32 {
    if (ops_obj == 0) return 0;
    const so = obj_ensure_string(ops_obj);
    const n = rt.list_count_elements(so.ptr, so.len);
    if (n <= 0) return 0;
    var mask: u32 = 0;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        const e = rt.list_element_at(so.ptr, so.len, i);
        const ep: [*]const u8 = @ptrFromInt(so.ptr + e.start);
        if (str_eq(ep, e.len, "read")) mask |= var_trace.OP_READ;
        if (str_eq(ep, e.len, "write")) mask |= var_trace.OP_WRITE;
        if (str_eq(ep, e.len, "unset")) mask |= var_trace.OP_UNSET;
        if (str_eq(ep, e.len, "array")) mask |= var_trace.OP_ARRAY;
    }
    return mask;
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
