// Minimal ``trace`` pass-through.
//
// Tcl's ``trace`` command installs read/write/unset hooks on variables
// and command-execution hooks on procs.  Implementing real traces
// requires hooking every ``var_set`` / ``var_resolve`` / ``unset``
// path to check an opaque callback list, which we don't have.
//
// ``tcltest`` and ``tcllib`` lean on traces mostly for *lazy
// configuration* — e.g. reading a testConstraints array element
// triggers a lazy initialiser that populates it.  Without the trace
// installed, the variable is simply its initial value (often empty
// or false), which keeps the harness loading without crashing.
//
// So this module treats ``trace add`` / ``trace remove`` as benign
// pass-throughs (the callback is silently dropped) and ``trace
// info`` as an error — the caller asked for state we don't keep.

const obj = @import("../value/tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;

fn eq(a: [*]const u8, alen: u32, literal: []const u8) bool {
    if (alen != literal.len) return false;
    for (0..literal.len) |i| if (a[i] != literal[i]) return false;
    return true;
}

/// ``trace <sub> ?args…?`` — pass-through add/remove, unsupported
/// for info / variable (legacy) / execution queries.
pub export fn tcl_cmd_trace_cmd(sub: i32, arg: i32) i32 {
    _ = arg;
    if (sub == 0) {
        stubs.unsupported("trace (missing subcommand)");
        return 0;
    }
    const s = obj_ensure_string(sub);
    if (s.len == 0) {
        stubs.unsupported("trace (empty subcommand)");
        return 0;
    }
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    // Accepted (pass-through, NOP).
    if (eq(sp, s.len, "add") or eq(sp, s.len, "remove")) {
        return obj_new_string(0, 0);
    }
    // Legacy forms also accepted quietly.
    if (eq(sp, s.len, "variable") or
        eq(sp, s.len, "vdelete") or
        eq(sp, s.len, "vinfo"))
    {
        return obj_new_string(0, 0);
    }
    // Info / query — we have nothing useful to return.
    if (eq(sp, s.len, "info")) {
        stubs.unsupported_sub("trace", "info");
        return 0;
    }
    stubs.unsupported_sub("trace", "<unknown-subcmd>");
    return 0;
}
