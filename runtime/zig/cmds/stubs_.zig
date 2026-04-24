// ``auto_load``, ``auto_reset``, ``auto_mkindex``, ``auto_import``,
// ``auto_execok``, ``auto_qualify``, ``package``, ``after``,
// ``vwait``, ``update`` — stub commands.
//
// Without the Tcl stdlib there is nothing to auto-load.  ``auto_load``
// returns 0 ("not found") so callers that check the return value see
// the expected "proc not in index" signal.  The other auto_* and
// package commands return an empty string.
//
// ``after`` has no event-loop backend in WASM.  All forms silently
// succeed: ``after ms`` (sleep), ``after ms script`` (schedule),
// ``after idle script`` (idle-callback), ``after cancel id``, and
// ``after info`` all return an empty string rather than trapping.

const std   = @import("std");
const rt    = @import("../tcl_runtime.zig");
const reg   = @import("../tcl_cmd_registry.zig");
const clock = @import("../tcl_clock.zig");

fn eval_auto_load(words: []const i32) i32 {
    _ = words;
    return rt.obj_new_int(0);
}

fn eval_auto_noop(words: []const i32) i32 {
    _ = words;
    return rt.obj_new_string(0, 0);
}

fn eval_package(words: []const i32) i32 {
    _ = words;
    return 0;
}

fn eval_after(words: []const i32) i32 {
    _ = words;
    return rt.obj_new_string(0, 0);
}

// Interpreter-side ``clock`` dispatcher.
//
// The Tcl compiler emits ``_emit_eval_fallback("clock", args)`` for
// subcommands it can't compile directly (``format``, ``scan``, ``add``).
// The fallback invokes the interpreter, which sees ``clock`` as the
// command.  Without this entry the interpreter errors "unknown command:
// clock".
//
// Subcommand routing:
//   seconds / clicks / milliseconds → WASI-backed real values.
//   format / add                    → return the string "0" (stub).
//   scan                            → return integer 0 (stub).
//   All others                      → empty string.
//
// ``format`` / ``scan`` / ``add`` stubbing mirrors ``tcl_time_stubs.zig``
// (the WASM-export path used by compiled procs).  See that file's
// header for the deliberate divergence from real Tcl semantics.
fn eval_clock(words: []const i32) i32 {
    if (words.len < 2) return clock.clock_seconds();
    const sub = rt.obj_ensure_string(words[1]);
    const sp: []const u8 = if (sub.ptr == 0) "" else @as([*]const u8, @ptrFromInt(sub.ptr))[0..sub.len];
    if (std.mem.eql(u8, sp, "seconds")) return clock.clock_seconds();
    if (std.mem.eql(u8, sp, "clicks")) return clock.clock_clicks();
    if (std.mem.eql(u8, sp, "milliseconds")) return clock.clock_milliseconds();
    if (std.mem.eql(u8, sp, "scan")) return rt.obj_new_int(0);
    if (std.mem.eql(u8, sp, "format") or std.mem.eql(u8, sp, "add")) {
        // Return the string "0" so callers can parse it as an integer.
        const buf = rt.alloc(1);
        const p: [*]u8 = @ptrFromInt(buf);
        p[0] = '0';
        return rt.obj_new_string(@intCast(buf), 1);
    }
    return rt.obj_new_string(0, 0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "auto_load",    .handler = &eval_auto_load },
    .{ .name = "auto_reset",   .handler = &eval_auto_noop },
    .{ .name = "auto_mkindex", .handler = &eval_auto_noop },
    .{ .name = "auto_import",  .handler = &eval_auto_noop },
    .{ .name = "auto_execok",  .handler = &eval_auto_noop },
    .{ .name = "auto_qualify", .handler = &eval_auto_noop },
    .{ .name = "package",      .handler = &eval_package },
    .{ .name = "after",        .handler = &eval_after },
    .{ .name = "vwait",        .handler = &eval_auto_noop },
    .{ .name = "update",       .handler = &eval_auto_noop },
    .{ .name = "clock",        .handler = &eval_clock },
};
