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
//   - chan   — top-level ensemble, not yet routed
//   - fileevent, socket — needs an event loop
//
// ``puts`` lives in tcl_io.zig; ``flush`` / ``open`` / ``close`` /
// ``read`` / ``gets`` / ``eof`` / ``fblocked`` / ``tell`` / ``seek``
// / ``fcopy`` live in tcl_chan.zig (real WASI-backed implementations).

const stubs = @import("tcl_stubs.zig");
const chan = @import("../io/tcl_chan.zig");

/// ``flush ?channelId?`` — drain the channel's per-write buffer
/// to its underlying fd.  With no argument (``fd == 0``) the call
/// is a no-op so the codegen's "flush stdout after a top-level
/// puts" emission doesn't trap.  Without a real flush path,
/// ``-buffering full`` writes would never reach the host stream
/// before ``close`` ran.
pub export fn tcl_cmd_flush(fd: i32) i32 {
    return chan.flush_chan_id(fd);
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
