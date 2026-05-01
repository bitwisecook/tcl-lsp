// ``exit`` — terminate the WASM instance via WASI ``proc_exit``.
//
// Gated behind ``CAP_EXIT`` (see :module:`interp/tcl_caps.zig`).
// Default sandboxed builds refuse the call with a Tcl-catchable
// permission-denied error so a hostile script cannot end the
// embedder's run.  When the host has granted ``CAP_EXIT`` the
// handler parses the optional return code and dispatches to
// ``proc_exit``, which wasmtime surfaces to the embedder as an
// ``Exit`` trap (``wasmtime.ExitTrap`` in the Python binding).
//
// Form:
//   ``exit ?returnCode?``    — returnCode defaults to 0; non-integer
//                              code raises a regular Tcl error rather
//                              than passing junk through to WASI.
//
// We import :data:`std.os.wasi.proc_exit` rather than wiring our own
// extern; ``std.os.wasi`` resolves to the same
// ``wasi_snapshot_preview1.proc_exit`` import the runtime already
// uses for ``fd_write`` from :module:`io/tcl_io.zig`, so no new
// host-import contract is added.

const std = @import("std");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");
const caps = @import("../interp/tcl_caps.zig");

fn eval_exit(words: []const i32) i32 {
    if (!caps.check(caps.CAP_EXIT, "exit", "EXIT")) return 0;
    var code: i32 = 0;
    if (words.len >= 2) {
        const s = obj.obj_ensure_string(words[1]);
        const parsed = obj.try_parse_int(s.ptr, s.len);
        if (parsed == null) {
            stubs.raise("exit: returnCode must be an integer");
            return 0;
        }
        // POSIX exit codes wrap into 8 bits at the kernel boundary;
        // pass the i32 through verbatim and let wasmtime / the OS
        // do the truncation so the embedder sees what the script
        // asked for, including negative values.
        code = @intCast(parsed.? & 0xffffffff);
    }
    std.os.wasi.proc_exit(@bitCast(code));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "exit", .arity_min = 0, .arity_max = 1, .handler = &eval_exit },
};

/// Single-arg compiled-codegen export.  The runtime-import contract
/// declared in ``core/commands/registry/tcl/exit.py`` (added in this
/// patch) routes ``exit`` / ``exit code`` through this entry point
/// so the compiled fast path matches the BUILTIN dispatch.  Words
/// outside ``exit`` / ``exit code`` fall through the codegen to the
/// eval fallback, which dispatches via :data:`registrations` above.
pub export fn tcl_cmd_exit(code_obj: i32) i32 {
    if (!caps.check(caps.CAP_EXIT, "exit", "EXIT")) return 0;
    var code: i32 = 0;
    if (code_obj != 0) {
        const s = obj.obj_ensure_string(code_obj);
        if (s.len != 0) {
            const parsed = obj.try_parse_int(s.ptr, s.len);
            if (parsed == null) {
                stubs.raise("exit: returnCode must be an integer");
                return 0;
            }
            code = @intCast(parsed.? & 0xffffffff);
        }
    }
    std.os.wasi.proc_exit(@bitCast(code));
}
