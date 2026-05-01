// I/O + channel stubs for Tcl 8.4–9.0 core commands we haven't
// implemented.  Each stub raises ``unsupported command: <name>``
// through :func:`tcl_stubs.unsupported`.
//
// **Reachability** — these ``pub export fn tcl_cmd_X`` symbols are
// direct WASM imports on the Python codegen side.  Each export name
// is referenced from the ``WasmRuntimeImport`` field on a
// ``CommandSpec`` under ``core/commands/registry/tcl/``.  The
// compiled WASM emits direct calls to these exports, so deleting
// any one of them breaks module instantiation for scripts that use
// that command.  Keep them in lock-step with the specs; the parity
// gate (``scripts/check_wasm_command_parity.py``) enforces this.
//
// Coverage today (everything else has a real impl elsewhere):
//   - flush  — synchronous wasi-libc writes; nothing to flush
//   - chan   — top-level ensemble, not yet routed
//   - fileevent, socket — needs an event loop
//
// ``puts`` lives in tcl_io.zig; ``open`` / ``close`` / ``read`` /
// ``gets`` / ``eof`` / ``fblocked`` / ``tell`` / ``seek`` / ``fcopy``
// live in tcl_chan.zig (real WASI-backed implementations).

const stubs = @import("tcl_stubs.zig");

/// ``flush`` — WASI's ``fd_write`` is synchronous and unbuffered
/// from our side (we don't maintain a per-channel write buffer),
/// so ``flush`` has nothing to do beyond succeed.  Returns empty
/// string, matching Tcl semantics.  Without this ``cleanupTests``
/// traps at ``flush [outputChannel]`` during its per-file wrap-up.
pub export fn tcl_cmd_flush(fd: i32) i32 {
    _ = fd;
    return 0;
}

pub export fn tcl_cmd_chan(sub: i32, arg: i32) i32 {
    _ = sub;
    _ = arg;
    stubs.unsupported("chan");
    return 0;
}

pub export fn tcl_cmd_fileevent(fd: i32, mode: i32) i32 {
    _ = fd;
    _ = mode;
    stubs.unsupported("fileevent");
    return 0;
}

pub export fn tcl_cmd_socket(host: i32, port: i32) i32 {
    _ = host;
    _ = port;
    stubs.unsupported("socket");
    return 0;
}
