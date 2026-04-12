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

const io = @import("tcl_io.zig");

/// Most recently registered source site ID.  Zero means "no site set"
/// — the error paths fall back to their raw message without a prefix.
pub var current_site_id: u32 = 0;

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
    // Reuse the integer formatter; strip its trailing newline by
    // using the no-newline variant.
    const buf = io.itoa_no_nl(@as(i64, @intCast(current_site_id)));
    io.fd_write_all(fd, buf.ptr, buf.len);
    io.fd_write_all(fd, " ", 1);
    return true;
}
