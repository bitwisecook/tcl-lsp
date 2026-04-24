// Fallback stub dispatch for Tcl 8.4–9.0 core commands that aren't
// wired up in the interpreter's main BUILTINS table and aren't
// shadowed by a user-defined proc.  Consulted last by
// :func:`tcl_interp.eval_proc_call` when both proc-lookup and the
// BUILTINS table miss.
//
// Philosophy: the interpreter must produce a meaningful result (or a
// clear trap) for *any* Tcl core command, without ever reaching
// ``unknown command: <name>``.  A user-defined proc that shadows a
// core command is handled by the proc registry (consulted in
// ``eval_proc_call`` *before* this dispatch table), so shadowing
// still works.  Commands fall into three groups:
//
//   (a) **Real** — the command has a runtime implementation and we
//       dispatch to it (``encoding``, ``fconfigure``, …).  Those
//       live in BUILTINS (``cmds/*.zig``) and never reach this file.
//
//   (b) **Stub** — the command has no WASM-available implementation
//       (``regexp``, ``file``, ``exec``, …); :func:`try_stub` traps
//       with ``unsupported command: X`` via :func:`tcl_stubs.unsupported`
//       so the caller sees a precise diagnostic.  Names live in
//       :data:`STUB_TRAP`.
//
//   (c) **Already handled in eval_command** — the hot-path builtins
//       (``set``, ``puts``, ``expr``, ``if``, ``while``, ``proc``,
//       ``string``, ``dict``, ``info``, ``array``, …) are checked by
//       BUILTINS before we're ever called.  A handful of commands
//       (``format``, ``unknown``) appear in :data:`STUB_FALLTHROUGH`
//       so this file documents that their absence from a trap is
//       deliberate — they're served elsewhere or should fall through
//       to the generic "unknown command" error path.

const stubs = @import("../stubs/tcl_stubs.zig");

/// Command names that trap with ``unsupported command: X`` when
/// reached.  Kept as a data table instead of an if-chain so adding a
/// new stub is one line.  Order doesn't matter — the lookup is a
/// linear scan.
const STUB_TRAP: []const []const u8 = &.{
    // I/O + channel — fconfigure has a real impl dispatched by BUILTINS.
    "open", "close", "read",     "gets",      "eof",     "flush",
    "fblocked", "tell", "seek",  "chan",      "fcopy",   "fileevent",
    "socket",
    // Filesystem / process — file/pwd/cd have real impls in BUILTINS.
    "glob", "exec", "source", "load", "unload",
    // Format / pattern matching — scan/binary are in BUILTINS.
    "regexp",
    // Time / event loop.  ``after``, ``vwait``, ``update``, ``clock``
    // are all handled by BUILTINS (``cmds/stubs_.zig``) before we
    // reach this table, so they are intentionally absent.
    "coroutine", "yield", "yieldto", "yieldmeta",
    // Environment / metadata — ``package`` is in BUILTINS
    // (cmds/stubs_.zig); ``trace`` / ``namespace`` / ``subst`` /
    // ``auto_*`` are in BUILTINS with real implementations.
    "interp", "rename",
    // Object system (TclOO, 8.6+).
    "oo::class", "oo::define", "oo::object", "oo::copy",
    "self", "my", "next",
    // Misc that tcltest specifically references.
    "zlib", "registry",
};

// Commands deliberately absent from STUB_TRAP (documentation only, no
// code references this list — it's here for a reader grep'ing for a
// command name):
//
//   * ``format`` — real runtime impl in tcl_format.zig, dispatched
//     through BUILTINS before we're reached.
//   * ``unknown`` — user-overridable hook; returning false lets the
//     stock ``unknown command`` error fire, which is the correct
//     behaviour when no override is installed.

fn eql(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    for (a, b) |x, y| if (x != y) return false;
    return true;
}

/// Emit ``unsupported command: <name>`` and return true so the
/// dispatch caller treats the command as handled.
fn trap(name: []const u8) bool {
    stubs.unsupported(name);
    return true;
}

/// Consult the fallback dispatch table.  Returns true when the command
/// name matched :data:`STUB_TRAP` (trap fired) — the caller should
/// treat the command as handled.  Returns false for any name not in
/// the list, letting the caller emit the generic ``unknown command``
/// error.
pub fn try_stub(cmd: [*]const u8, cmd_len: u32) bool {
    const name = cmd[0..cmd_len];
    for (STUB_TRAP) |n| {
        if (eql(name, n)) return trap(n);
    }
    return false;
}
