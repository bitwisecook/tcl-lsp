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
    var code: u32 = 0;
    if (words.len >= 2) {
        const s = obj.obj_ensure_string(words[1]);
        const parsed = obj.try_parse_int(s.ptr, s.len);
        if (parsed == null) {
            stubs.raise("exit: returnCode must be an integer");
            return 0;
        }
        code = exit_code_from_i64(parsed.?);
    }
    std.os.wasi.proc_exit(code);
}

/// Map a Tcl integer return code (i64) to the WASI ``exitcode_t``
/// (u32 — see ``zig/lib/std/os/wasi.zig``).  Tcl accepts the full
/// signed-i64 range syntactically; POSIX / WASI truncate to 32 bits
/// at the kernel boundary, so we replicate that truncation
/// explicitly via :func:`@truncate` on the raw two's-complement
/// pattern.  This preserves the documented semantics for negative
/// values (``exit -1`` → ``0xFFFFFFFF``, which the kernel typically
/// renders as exit status 255) and out-of-i32-range positive values
/// (``exit 4294967295`` → ``0xFFFFFFFF``) without tripping
/// ``@intCast``'s safety check.
fn exit_code_from_i64(v: i64) u32 {
    return @truncate(@as(u64, @bitCast(v)));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "exit", .arity_min = 0, .arity_max = 1, .handler = &eval_exit },
};

/// Single-arg export for embedders that want the same ``exit`` /
/// ``exit code`` behaviour without going through the Tcl word
/// dispatcher.  ``core/commands/registry/tcl/exit.py`` deliberately
/// has no ``wasm_runtime_import``; the standard eval-fallback path
/// dispatches via :data:`registrations` above.  This export exists
/// purely so embedders that have already parsed the return code can
/// reach the capability gate + ``proc_exit`` pair directly.
pub export fn tcl_cmd_exit(code_obj: i32) i32 {
    if (!caps.check(caps.CAP_EXIT, "exit", "EXIT")) return 0;
    var code: u32 = 0;
    if (code_obj != 0) {
        const s = obj.obj_ensure_string(code_obj);
        if (s.len != 0) {
            const parsed = obj.try_parse_int(s.ptr, s.len);
            if (parsed == null) {
                stubs.raise("exit: returnCode must be an integer");
                return 0;
            }
            code = exit_code_from_i64(parsed.?);
        }
    }
    std.os.wasi.proc_exit(code);
}
