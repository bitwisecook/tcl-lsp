// Source-location diagnostic for trap messages.
//
// The codegen decorates every potentially-trapping call site (eval
// fallback, unsupported-command trap, unknown-command dispatch) with
// a call to ``diag_set(site_id)`` that stashes a compact integer ID
// into a single-slot register.  When a trap path fires (``@"error"``
// in tcl_catch.zig, unknown-command in tcl_interp.zig) the stashed
// ID is written to stderr as a prefix, so the output is
//
//     tcl trap: site=1234 unknown command: encoding
//
// which a companion tool (tcl-trap-resolve skill) decodes against the
// ``<module>.wasm.map.json`` sidecar emitted by the codegen into a
// human-readable location like ``at tcltest.tcl:3347:13 in encoding``.
//
// The register is a single slot, not a stack — on deeper traces the
// most recently set site wins, which is usually the one the user
// cares about.  Unset (site_id == 0) suppresses the prefix so unit
// tests that don't wire diag_set still see clean error output.
//
// Keeping all the rich source data out-of-band (in the sidecar map
// file) means the compiled WASM pays only 4 bytes + a single call
// per trap-able site, regardless of how long the original source
// line is.  That matters when tcltest.tcl alone has hundreds of
// fallback sites.

const io = @import("../io/tcl_io.zig");

/// Most recently registered source site ID.  Zero means "no site set"
/// — the error paths fall back to their raw message without a prefix.
pub var current_site_id: u32 = 0;

/// The script + offset currently being walked by the interpreter's
/// ``eval_script`` loop.  Set by the interpreter before each command
/// iteration; read by the trap paths to print a short snippet of
/// source context on error.  All three zero means "no eval context"
/// (e.g. when the trap fires from directly-compiled code).
pub var current_eval_ptr: u32 = 0;
pub var current_eval_len: u32 = 0;
pub var current_eval_pos: u32 = 0;

/// Exported: update the eval-script context.  Called from
/// :func:`tcl_interp.eval_script` on every iteration.
pub export fn diag_set_eval_ctx(script_ptr: i32, script_len: i32, pos: i32) void {
    current_eval_ptr = @intCast(script_ptr);
    current_eval_len = @intCast(script_len);
    current_eval_pos = @intCast(pos);
}

/// Exported: record the current source site ID.  The codegen emits a
/// call to this immediately before any call that might trap.  The
/// argument is an opaque integer handle; the sidecar ``.wasm.map.json``
/// resolves it to a ``(file, line, col, command, args)`` tuple.
pub export fn diag_set(site_id: i32) void {
    current_site_id = @intCast(site_id);
}

/// Write ``site=<id> `` to ``fd`` iff a site is registered.  Called by
/// the trap paths before they print the message body.  Returns true
/// if a prefix was written so the caller can arrange spacing.
pub fn write_prefix(fd: i32) bool {
    if (current_site_id == 0) return false;
    io.fd_write_all(fd, "site=", 5);
    // ``itoa`` renders digits without a trailing newline; the caller
    // appends any separator (a space here, matching the trailing
    // ``" "`` below).
    const buf = io.itoa(@as(i64, @intCast(current_site_id)));
    io.fd_write_all(fd, buf.ptr, buf.len);
    io.fd_write_all(fd, " ", 1);
    return true;
}

/// Append the current eval-script context (if set) to ``fd`` as a
/// ``\n  in eval-script at offset N: <snippet>\n`` line.  Called by
/// the trap paths after the main error message so a user can see
/// which command inside a ``tcl_eval`` fallback fired the trap,
/// even when the outer diag site only points at the fallback's
/// creation site (e.g. the outer ``namespace eval { ... }``).
pub fn write_eval_ctx(fd: i32) void {
    if (current_eval_ptr == 0 or current_eval_len == 0) return;
    io.fd_write_all(fd, "  in eval-script at offset ", 27);
    const off = io.itoa(@as(i64, @intCast(current_eval_pos)));
    io.fd_write_all(fd, off.ptr, off.len);
    io.fd_write_all(fd, ": ", 2);
    // Print a 60-char (or rest-of-script) snippet starting at the
    // current position — enough context for a reader to find the
    // command in the original source.  Newlines in the snippet are
    // echoed literally; we trust the reader to cope with a
    // multi-line window.
    var span: u32 = current_eval_len - current_eval_pos;
    if (span > 60) span = 60;
    if (span > 0) {
        io.fd_write_all(fd, @ptrFromInt(current_eval_ptr + current_eval_pos), span);
    }
    io.fd_write_all(fd, "\n", 1);
}
