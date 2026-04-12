// Common helper for all per-area Tcl command stubs.
//
// A "stub" here is an export that identifies a core Tcl 8.4–9.0
// command we haven't implemented.  Rather than silently returning
// 0 / -1 (the previous pattern in ``tcl_catch.zig`` — which the
// project style rules explicitly forbid) each stub raises a
// ``unsupported command: <name>`` error through the standard error
// path.  Inside a ``catch`` this sets ``error_flag`` + ``error_msg``;
// outside a catch it writes
//
//     tcl trap: site=<id> unsupported command: <name>
//
// to stderr (the ``site=<id>`` prefix comes from the diag machinery
// added in the previous commit) and traps.
//
// Having the stubs centralised here means every area-specific file
// (``tcl_io_stubs.zig``, ``tcl_fmt_stubs.zig``, etc.) only needs to
// declare the export signature and call ``unsupported``.  The format
// of the diagnostic is identical for every stub.

const obj = @import("tcl_obj.zig");
const catch_mod = @import("tcl_catch.zig");

/// Build ``unsupported command: <name>`` and route it through
/// :func:`tcl_catch.@"error"`.  When called outside a catch the
/// runtime writes the message to stderr and traps; inside a catch
/// it sets ``error_flag`` so the caller can observe a normal error
/// return instead of a hard trap.
pub fn unsupported(name: []const u8) void {
    const prefix: []const u8 = "unsupported command: ";
    const total: u32 = @intCast(prefix.len + name.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    for (prefix, 0..) |c, i| buf[i] = c;
    for (name, 0..) |c, i| buf[prefix.len + i] = c;
    const msg = obj.obj_new_string(@intCast(buf_addr), @intCast(total));
    catch_mod.@"error"(msg);
}

/// Same shape as ``unsupported`` but for a subcommand.  Produces
/// ``unsupported command: <cmd> <sub>`` so the user can see
/// which specific form (``clock format`` vs ``clock scan``) is
/// missing.
pub fn unsupported_sub(cmd: []const u8, sub: []const u8) void {
    const prefix: []const u8 = "unsupported command: ";
    const sep: []const u8 = " ";
    const total: u32 = @intCast(prefix.len + cmd.len + sep.len + sub.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    for (cmd) |c| {
        buf[off] = c;
        off += 1;
    }
    for (sep) |c| {
        buf[off] = c;
        off += 1;
    }
    for (sub) |c| {
        buf[off] = c;
        off += 1;
    }
    const msg = obj.obj_new_string(@intCast(buf_addr), @intCast(total));
    catch_mod.@"error"(msg);
}
